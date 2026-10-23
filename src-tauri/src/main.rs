#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod i18n;
mod logging;
mod scheduler;
mod settings;
mod smoke;
mod state;
mod tray;
mod windows;

use std::sync::Mutex;
use tauri::{Emitter, Manager, WindowEvent};

/// Item the main window should reveal when it next loads or receives focus.
#[derive(Default)]
pub struct PendingFocus(pub Mutex<Option<commands::ItemRef>>);

/// Shows the main window and asks it to reveal `item`.
pub fn reveal_item(app: &tauri::AppHandle, item: commands::ItemRef) {
    if let Some(p) = app.try_state::<PendingFocus>() {
        if let Ok(mut slot) = p.0.lock() {
            *slot = Some(item);
        }
    }
    if windows::show_main(app).is_ok() {
        let _ = app.emit_to("main", "focus-item", item);
    }
}

#[tauri::command]
fn open_item(app: tauri::AppHandle, item: commands::ItemRef) {
    reveal_item(&app, item);
}

#[tauri::command]
fn take_pending_settings(p: tauri::State<windows::PendingSettings>) -> bool {
    p.take()
}

#[tauri::command]
fn take_pending_focus(p: tauri::State<PendingFocus>) -> Option<commands::ItemRef> {
    p.0.lock().ok().and_then(|mut s| s.take())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn open_main(app: tauri::AppHandle) -> Result<(), commands::CmdError> {
    windows::show_main(&app)
        .map(|_| ())
        .map_err(|e| commands::CmdError::new("window", e.to_string()))
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), commands::CmdError> {
    windows::show_settings(&app).map_err(|e| commands::CmdError::new("window", e.to_string()))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--smoke") {
        let text = args.get(i + 1).map(String::as_str).unwrap_or("");
        match smoke::run(text, commands::local_now()) {
            Ok(json) => {
                println!("{json}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("smoke failed: {} ({})", e.message, e.code);
                std::process::exit(1);
            }
        }
    }

    let _log_guard = logging::init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting TaskCraft");

    let state = match state::AppState::open(&state::data_dir()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "cannot open database");
            eprintln!("TaskCraft: cannot open database: {e}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(state)
        .manage(PendingFocus::default())
        .manage(windows::PendingSettings::default())
        .manage(scheduler::LastFired::default())
        .invoke_handler(tauri::generate_handler![
            commands::parse_preview,
            commands::create_from_text,
            commands::list_subtasks,
            commands::reorder_subtasks,
            commands::set_task_parent,
            commands::complete_with_subtasks,
            commands::list_checklist,
            commands::add_checklist_item,
            commands::update_checklist_item,
            commands::reorder_checklist,
            commands::delete_checklist_item,
            commands::checklist_item_to_subtask,
            commands::get_event,
            commands::create_event,
            commands::update_event,
            commands::move_event,
            commands::delete_event,
            commands::get_task,
            commands::create_task,
            commands::update_task,
            commands::set_task_completed,
            commands::delete_task,
            commands::set_task_status,
            commands::list_labels,
            commands::update_label,
            commands::delete_label,
            commands::set_ui_language,
            commands::list_range,
            commands::list_tasks,
            commands::search,
            commands::list_calendars,
            commands::upsert_calendar,
            commands::delete_calendar,
            commands::get_settings,
            commands::set_settings,
            commands::export_ics,
            commands::local_time_zone,
            commands::list_plans,
            commands::create_plan,
            commands::update_plan,
            commands::reorder_plans,
            commands::delete_plan,
            commands::list_buckets,
            commands::create_bucket,
            commands::rename_bucket,
            commands::reorder_buckets,
            commands::delete_bucket,
            commands::move_task,
            commands::get_default_plan,
            commands::set_default_plan,
            open_main,
            open_settings,
            open_item,
            take_pending_focus,
            take_pending_settings,
            quit_app,
        ])
        .setup(|app| {
            let handle = app.handle();
            tray::setup_tray(handle)?;
            let hotkey = handle
                .state::<state::AppState>()
                .settings
                .read()
                .map(|s| s.hotkey.clone())
                .unwrap_or_default();
            if let Err(e) = tray::register_hotkey(handle, &hotkey, None) {
                tracing::warn!(%hotkey, error = %e, "could not register hotkey");
            }
            {
                use tauri_plugin_notification::{NotificationExt, PermissionState};
                let n = handle.notification();
                if !matches!(n.permission_state(), Ok(PermissionState::Granted)) {
                    let _ = n.request_permission();
                }
            }
            scheduler::spawn(handle);
            // Launched at login → stay in the menu bar only.
            if !std::env::args().any(|a| a == "--minimized") {
                windows::show_main(handle)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window keeps TaskCraft running in the menu bar.
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "tauri build error");
            std::process::exit(1);
        })
        .run(|_app, _event| {
            // Dock click, or a click on one of our notifications. macOS only:
            // other systems have no Dock "reopen" event.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                match scheduler::recent(_app) {
                    Some(item) => reveal_item(_app, item),
                    None => {
                        let _ = windows::show_main(_app);
                    }
                }
            }
        });
}
