use crate::settings::Settings;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use taskcraft_store::Store;

pub struct AppState {
    pub store: Mutex<Store>,
    pub settings: RwLock<Settings>,
    /// None in tests (settings are not persisted).
    pub settings_path: Option<PathBuf>,
    /// UI language, reported by the webview at startup.
    pub lang: RwLock<crate::i18n::Lang>,
}

impl AppState {
    pub fn open(data_dir: &std::path::Path) -> Result<AppState, taskcraft_store::StoreError> {
        let store = Store::open(&data_dir.join("taskcraft.db"))?;
        let settings_path = data_dir.join("settings.json");
        let settings = Settings::load(&settings_path);
        Ok(AppState {
            store: Mutex::new(store),
            settings: RwLock::new(settings),
            settings_path: Some(settings_path),
            lang: RwLock::default(),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> AppState {
        let store = Store::open_in_memory().expect("in-memory store");
        AppState {
            store: Mutex::new(store),
            settings: RwLock::new(Settings::default()),
            settings_path: None,
            lang: RwLock::default(),
        }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TaskCraft")
}

pub fn log_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Library/Logs/TaskCraft")
}
