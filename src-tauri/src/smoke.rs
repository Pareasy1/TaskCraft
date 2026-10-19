//! `taskcraft --smoke "<sentence>"`: parse and insert into a throwaway
//! in-memory database, print the stored item as JSON. Used by scripts/smoke.sh
//! to check a release build without touching the user's data.

use crate::commands::{create_from_text_impl, CmdError};
use crate::settings::Settings;
use crate::state::AppState;
use chrono::DateTime;
use chrono_tz::Tz;
use std::sync::{Mutex, RwLock};
use taskcraft_store::{ItemKindDb, Store};

pub fn run(text: &str, now: DateTime<Tz>) -> Result<String, CmdError> {
    let store = Store::open_in_memory().map_err(CmdError::from)?;
    let st = AppState { store: Mutex::new(store), settings: RwLock::new(Settings::default()), settings_path: None };
    let r = create_from_text_impl(&st, text, now)?;
    let store = st.store.lock().map_err(|_| CmdError::new("internal", "lock"))?;
    let json = match r.kind {
        ItemKindDb::Event => serde_json::to_string(&store.get_event(r.id)?),
        ItemKindDb::Task => serde_json::to_string(&store.get_task(r.id)?),
    };
    json.map_err(|e| CmdError::new("json", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn smoke_creates_lunch() {
        let now = chrono_tz::Europe::Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
        let json: serde_json::Value = serde_json::from_str(&run("Lunch with Sam tomorrow at 1", now).unwrap()).unwrap();
        assert_eq!(json["title"], "Lunch");
        assert_eq!(json["start"], "2026-10-21T11:00:00Z");
        assert_eq!(json["attendees"][0], "Sam");
    }
}
