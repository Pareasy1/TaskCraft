#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod logging;
mod settings;
mod state;
mod windows;

use tauri::Manager;

#[tauri::command]
fn open_main(app: tauri::AppHandle) -> Result<(), commands::CmdError> {
    windows::show_main(&app).map(|_| ()).map_err(|e| commands::CmdError::new("window", e.to_string()))
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), commands::CmdError> {
    windows::show_settings(&app).map_err(|e| commands::CmdError::new("window", e.to_string()))
}

fn main() {
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
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::parse_preview,
            commands::create_from_text,
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
            commands::list_range,
            commands::list_tasks,
            commands::search,
            commands::list_calendars,
            commands::upsert_calendar,
            commands::delete_calendar,
            commands::get_settings,
            commands::export_ics,
            commands::local_time_zone,
            open_main,
            open_settings,
        ])
        .setup(|app| {
            windows::show_main(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "tauri runtime error");
            std::process::exit(1);
        });
}
