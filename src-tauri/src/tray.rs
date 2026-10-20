//! Menu-bar icon, quick-entry popover and global hotkey.

use crate::i18n::Lang;
use crate::windows;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_positioner::{Position, WindowExt};

pub const QUICK: &str = "quick";

fn quick_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window(QUICK) {
        return Ok(w);
    }
    let builder = WebviewWindowBuilder::new(app, QUICK, WebviewUrl::App("index.html".into()))
        .title("TaskCraft")
        .inner_size(360.0, 520.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .shadow(true);
    // macOS draws the popover with the system's translucent material; other
    // systems get an opaque window (the UI paints its own background there).
    #[cfg(target_os = "macos")]
    let builder = builder.transparent(true).effects(
        EffectsBuilder::new()
            .effect(Effect::Popover)
            .state(EffectState::Active)
            .radius(12.0)
            .build(),
    );
    let w = builder.build()?;
    let handle = w.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::Focused(false) = e {
            let _ = handle.hide();
        }
    });
    Ok(w)
}

/// Where the popover appears: under the menu-bar icon on macOS, by the clock
/// on Windows, centred on Linux (its tray icons don't report a position).
fn popover_position() -> Position {
    if cfg!(target_os = "macos") {
        Position::TrayBottomCenter
    } else if cfg!(target_os = "windows") {
        Position::BottomRight
    } else {
        Position::Center
    }
}

/// Shows the popover under the menu-bar icon, or hides it if visible.
pub fn toggle_quick(app: &AppHandle) {
    let Ok(w) = quick_window(app) else { return };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    let _ = w.move_window(popover_position());
    let _ = w.show();
    let _ = w.set_focus();
}

fn build_menu(app: &AppHandle, lang: Lang) -> tauri::Result<Menu<tauri::Wry>> {
    let text = crate::i18n::text(lang);
    let quick = MenuItem::with_id(app, "quick", text.quick_entry, true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", text.open, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", text.settings, true, Some("CmdOrCtrl+,"))?;
    let quit = MenuItem::with_id(app, "quit", text.quit, true, Some("CmdOrCtrl+Q"))?;
    Menu::with_items(
        app,
        &[
            &quick,
            &open,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

/// Rebuilds the menu-bar menu in `lang`.
pub fn set_menu_language(app: &AppHandle, lang: Lang) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        tray.set_menu(Some(build_menu(app, lang)?))?;
    }
    Ok(())
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, Lang::default())?;
    let icon = Image::from_bytes(include_bytes!("../icons/tray@2x.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("TaskCraft")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "quick" => toggle_quick(app),
            "open" => {
                let _ = windows::show_main(app);
            }
            "settings" => {
                let _ = windows::show_settings(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_quick(tray.app_handle());
            }
        })
        .build(app)?;
    // Create the popover ahead of time so the first open is instant.
    quick_window(app)?;
    Ok(())
}

/// Replaces the registered global hotkey. On failure the previous one stays.
pub fn register_hotkey(app: &AppHandle, new: &str, old: Option<&str>) -> Result<(), String> {
    let shortcut: Shortcut = new.parse().map_err(|e| format!("{e}"))?;
    let gs = app.global_shortcut();
    let previous: Option<Shortcut> = old.and_then(|o| o.parse().ok());
    if let Some(p) = previous {
        if p == shortcut && gs.is_registered(p) {
            return Ok(());
        }
        let _ = gs.unregister(p);
    }
    let handler =
        |app: &AppHandle, _s: &Shortcut, e: tauri_plugin_global_shortcut::ShortcutEvent| {
            if e.state() == ShortcutState::Pressed {
                toggle_quick(app);
            }
        };
    match gs.on_shortcut(shortcut, handler) {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Some(p) = previous {
                let _ = gs.on_shortcut(p, handler);
            }
            Err(e.to_string())
        }
    }
}
