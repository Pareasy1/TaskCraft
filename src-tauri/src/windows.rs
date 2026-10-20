//! Window creation helpers. Windows are created on demand and reused.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

fn existing(app: &AppHandle, label: &str) -> Option<WebviewWindow> {
    app.get_webview_window(label)
}

/// Shows (creating if needed) the main calendar window.
pub fn show_main(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let w = match existing(app, "main") {
        Some(w) => w,
        None => WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
            .title("TaskCraft")
            .inner_size(1100.0, 720.0)
            .min_inner_size(800.0, 560.0)
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .build()?,
    };
    w.show()?;
    w.set_focus()?;
    Ok(w)
}

fn settings_title(app: &AppHandle) -> &'static str {
    let lang = app
        .try_state::<crate::state::AppState>()
        .map(|s| *s.lang.read().unwrap_or_else(|e| e.into_inner()))
        .unwrap_or_default();
    crate::i18n::text(lang).settings_window
}

pub fn show_settings(app: &AppHandle) -> tauri::Result<()> {
    let w = match existing(app, "settings") {
        Some(w) => w,
        None => WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
            .title(settings_title(app))
            .inner_size(560.0, 620.0)
            .resizable(false)
            .build()?,
    };
    w.show()?;
    w.set_focus()?;
    Ok(())
}
