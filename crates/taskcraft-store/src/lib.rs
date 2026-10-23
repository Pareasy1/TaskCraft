//! SQLite storage for TaskCraft: calendars, events, tasks and alerts.

mod alerts;
mod calendars;
mod events;
mod ics;
mod labels;
mod model;
mod plans;
mod query;
mod recur;
mod schema;
mod subtasks;
mod tasks;
mod time;

pub use alerts::DueAlert;
pub use model::*;
pub use query::{Hit, Occurrence, Scope, TaskFilter};
pub use time::{from_parsed, local_to_utc, to_utc};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::Connection;
use std::path::Path;

pub struct Store {
    pub(crate) conn: Connection,
}

/// Case-insensitive (full Unicode) comparison key for plan and label names;
/// SQLite's NOCASE only folds ASCII.
pub(crate) fn name_key(s: &str) -> String {
    s.to_lowercase()
}

pub(crate) fn fmt_ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(crate) fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_default()
}

impl Store {
    /// Opens (creating if needed) the database at `path`. If migrations are
    /// pending on an existing file, a consistent copy (including rows still
    /// in the `-wal` file) is first written to `<name>.bak-v<old version>`.
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let existed = path.exists() && std::fs::metadata(path)?.len() > 0;
        let mut conn = Connection::open(path)?;
        let v = schema::version(&conn)?;
        if existed && v < schema::CURRENT_VERSION {
            let mut bak = path.as_os_str().to_owned();
            bak.push(format!(".bak-v{v}"));
            // VACUUM INTO refuses to overwrite an existing file.
            let _ = std::fs::remove_file(&bak);
            conn.execute("VACUUM INTO ?1", [bak.to_string_lossy()])?;
        }
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(&mut conn)?;
        Ok(Store { conn })
    }

    pub fn open_in_memory() -> Result<Store> {
        let mut conn = Connection::open_in_memory()?;
        Self::init(&mut conn)?;
        Ok(Store { conn })
    }

    fn init(conn: &mut Connection) -> Result<()> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        schema::migrate(conn)
    }

    /// Runs `f` in a transaction, or inside the caller's transaction if one
    /// is already open (SQLite does not nest transactions).
    pub(crate) fn in_tx<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        if !self.conn.is_autocommit() {
            return f(&self.conn);
        }
        let tx = self.conn.unchecked_transaction()?;
        let v = f(&tx)?;
        tx.commit()?;
        Ok(v)
    }

    pub fn schema_version(&self) -> Result<i64> {
        schema::version(&self.conn)
    }
}
