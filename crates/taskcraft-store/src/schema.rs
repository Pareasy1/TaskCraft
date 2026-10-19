//! Versioned schema migrations (`PRAGMA user_version`).

use crate::model::Result;
use rusqlite::Connection;

pub const CURRENT_VERSION: i64 = 1;

const V1: &str = "
CREATE TABLE calendars(
  id INTEGER PRIMARY KEY,
  name TEXT UNIQUE NOT NULL,
  color TEXT NOT NULL,
  visible INTEGER NOT NULL DEFAULT 1,
  sort INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE events(
  id INTEGER PRIMARY KEY,
  calendar_id INTEGER NOT NULL REFERENCES calendars(id),
  title TEXT NOT NULL,
  start_utc TEXT NOT NULL,
  end_utc TEXT NOT NULL,
  tz TEXT NOT NULL,
  all_day INTEGER NOT NULL DEFAULT 0,
  location TEXT, attendees TEXT, notes TEXT, url TEXT, rrule TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX events_start ON events(start_utc);
CREATE TABLE tasks(
  id INTEGER PRIMARY KEY,
  calendar_id INTEGER NOT NULL REFERENCES calendars(id),
  title TEXT NOT NULL,
  due_utc TEXT,
  due_has_time INTEGER NOT NULL DEFAULT 0,
  tz TEXT NOT NULL,
  completed_at TEXT, rrule TEXT, notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE alerts(
  id INTEGER PRIMARY KEY,
  owner_kind TEXT NOT NULL CHECK(owner_kind IN ('event','task')),
  owner_id INTEGER NOT NULL,
  offset_min INTEGER,
  absolute_utc TEXT,
  occurrence_utc TEXT,
  fire_at_utc TEXT NOT NULL,
  fired_at TEXT
);
CREATE INDEX alerts_due ON alerts(fired_at, fire_at_utc);
INSERT INTO calendars(name, color, sort) VALUES ('Personal', '#3b82f6', 0), ('Work', '#f97316', 1);
";

pub fn version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

/// Applies pending migrations inside one transaction.
pub fn migrate(conn: &mut Connection) -> Result<()> {
    let v = version(conn)?;
    if v >= CURRENT_VERSION {
        return Ok(());
    }
    let tx = conn.transaction()?;
    if v < 1 {
        tx.execute_batch(V1)?;
    }
    tx.pragma_update(None, "user_version", CURRENT_VERSION)?;
    tx.commit()?;
    Ok(())
}
