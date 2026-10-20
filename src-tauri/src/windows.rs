//! Window creation helpers. Windows are created on demand and reused.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

fn existing(app: &AppHandle, label: &str) -> Option<WebviewWindow> {
    app.get_webview_window(label)
}

/// The main window's title bar: macOS overlays its traffic lights on the
/// app's own toolbar; other systems keep their native title bar.
trait MainChrome {
    fn build_main_chrome(self) -> tauri::Result<WebviewWindow>;
}

impl MainChrome for WebviewWindowBuilder<'_, tauri::Wry, AppHandle> {
    fn build_main_chrome(self) -> tauri::Result<WebviewWindow> {
        #[cfg(target_os = "macos")]
        let builder = self
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true);
        #[cfg(not(target_os = "macos"))]
        let builder = self;
        builder.build()
    }
}

/// Shows (creating if needed) the main calendar window.
pub fn show_main(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let w = match existing(app, "main") {
        Some(w) => w,
        None => WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
            .title("TaskCraft")
            .inner_size(1100.0, 720.0)
            .min_inner_size(800.0, 560.0)
            .build_main_chrome()?,
    };
    w.show()?;
    w.set_focus()?;
    Ok(w)
}

/// A request to open Settings, kept until the main window takes it (it may
/// still be loading when the request arrives).
#[derive(Default)]
pub struct PendingSettings(AtomicBool);

impl PendingSettings {
    pub fn request(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn take(&self) -> bool {
        self.0.swap(false, Ordering::SeqCst)
    }
}

/// Settings live in a dialog inside the main window: bring the window up and
/// ask it to open the dialog.
pub fn show_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(p) = app.try_state::<PendingSettings>() {
        p.request();
    }
    show_main(app)?;
    app.emit_to("main", "open-settings", ())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::PendingSettings;

    /// A request made before the main window is listening is kept until it asks, then cleared.
    #[test]
    fn pending_settings_is_taken_once() {
        let p = PendingSettings::default();
        assert!(!p.take());
        p.request();
        assert!(p.take());
        assert!(!p.take());
    }

    /// The main window hides the system title bar and uses
    /// `data-tauri-drag-region` instead; Tauri only honours it when the
    /// window may start dragging (and toggle zoom on double-click).
    #[test]
    fn main_window_can_be_dragged_by_its_toolbar() {
        let caps: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        let windows: Vec<&str> = caps["windows"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|w| w.as_str())
            .collect();
        let perms: Vec<&str> = caps["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p.as_str())
            .collect();
        assert!(windows.contains(&"main"));
        for p in [
            "core:window:allow-start-dragging",
            "core:window:allow-internal-toggle-maximize",
        ] {
            assert!(perms.contains(&p), "missing permission {p}");
        }
    }
}
