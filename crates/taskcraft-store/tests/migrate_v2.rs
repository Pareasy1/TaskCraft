use rusqlite::Connection;
use taskcraft_store::*;

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
INSERT INTO calendars(name, color, sort) VALUES ('Personal', '#3b82f6', 0), ('Work', '#f97316', 1);";

/// A V1 database in WAL mode with one open and one completed task; the
/// returned connection is kept open so its rows may still sit in the -wal file.
fn v1_db(path: &std::path::Path) -> Connection {
    let c = Connection::open(path).unwrap();
    c.pragma_update(None, "journal_mode", "WAL").unwrap();
    c.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    c.execute_batch(V1).unwrap();
    c.execute_batch(
        "INSERT INTO tasks(calendar_id, title, tz, created_at, updated_at)
           VALUES (1, 'open', 'Europe/Rome', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');
         INSERT INTO tasks(calendar_id, title, tz, completed_at, created_at, updated_at)
           VALUES (1, 'finished', 'Europe/Rome', '2026-10-02T00:00:00Z', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');
         PRAGMA user_version = 1;",
    )
    .unwrap();
    c
}

#[test]
fn v1_completed_task_becomes_done() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taskcraft.db");
    let _keep = v1_db(&path);
    let s = Store::open(&path).unwrap();
    assert_eq!(s.schema_version().unwrap(), 6);
    let open = s.get_task(1).unwrap();
    let done = s.get_task(2).unwrap();
    assert_eq!(open.status, TaskStatus::Todo);
    assert_eq!(done.status, TaskStatus::Done);
    assert!(open.labels.is_empty());
}

#[test]
fn backup_includes_wal_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taskcraft.db");
    let _keep = v1_db(&path);
    drop(Store::open(&path).unwrap());
    let bak = Connection::open(dir.path().join("taskcraft.db.bak-v1")).unwrap();
    let n: i64 = bak
        .query_row("SELECT count(*) FROM tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
}

#[test]
fn fresh_db_is_v6() {
    assert_eq!(
        Store::open_in_memory().unwrap().schema_version().unwrap(),
        6
    );
}
