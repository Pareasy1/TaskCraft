//! Versioned schema migrations (`PRAGMA user_version`).

use crate::model::Result;
use rusqlite::Connection;

pub const CURRENT_VERSION: i64 = 5;

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

/// Task status, labels and task↔label links.
const V2: &str = "
ALTER TABLE tasks ADD COLUMN status TEXT NOT NULL DEFAULT 'todo'
  CHECK(status IN ('todo','in_progress','done'));
UPDATE tasks SET status = 'done' WHERE completed_at IS NOT NULL;
CREATE TABLE labels(
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  color TEXT NOT NULL
);
CREATE TABLE task_labels(
  task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  label_id INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
  PRIMARY KEY(task_id, label_id)
);
";

/// Plans, buckets, prefs and task ordering.
const V3: &str = "
CREATE TABLE plans(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, color TEXT NOT NULL, sort INTEGER NOT NULL);
CREATE TABLE buckets(id INTEGER PRIMARY KEY, plan_id INTEGER NOT NULL REFERENCES plans(id) ON DELETE CASCADE, name TEXT NOT NULL, sort INTEGER NOT NULL);
CREATE TABLE prefs(key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT INTO plans(id, name, color, sort) VALUES (1, 'My tasks', '#3b82f6', 0);
INSERT INTO buckets(id, plan_id, name, sort) VALUES (1, 1, 'To do', 0);
INSERT INTO prefs VALUES ('default_plan_id', '1'), ('seed_names_pending', '1');
ALTER TABLE tasks ADD COLUMN plan_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE tasks ADD COLUMN bucket_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE tasks ADD COLUMN sort_order REAL NOT NULL DEFAULT 0;
CREATE INDEX tasks_bucket ON tasks(bucket_id, sort_order);
";

/// Subtasks, priority, start date and checklists.
const V4: &str = "
ALTER TABLE tasks ADD COLUMN parent_id INTEGER;
ALTER TABLE tasks ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('urgent','important','medium','low'));
ALTER TABLE tasks ADD COLUMN start_utc TEXT;
ALTER TABLE tasks ADD COLUMN start_has_time INTEGER NOT NULL DEFAULT 0;
CREATE TABLE checklist_items(id INTEGER PRIMARY KEY, task_id INTEGER NOT NULL, text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0, sort REAL NOT NULL);
CREATE INDEX tasks_parent ON tasks(parent_id, sort_order);
CREATE INDEX checklist_task ON checklist_items(task_id, sort);
";

/// Templates, task attachments and the delayed file-delete queue.
const V5: &str = "
CREATE TABLE templates(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, body TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE attachments(
  id INTEGER PRIMARY KEY,
  task_id INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('file','link')),
  name TEXT NOT NULL,
  stored TEXT,
  url TEXT,
  size INTEGER,
  created_at TEXT NOT NULL,
  sort REAL NOT NULL
);
CREATE INDEX attachments_task ON attachments(task_id, sort);
CREATE TABLE pending_file_deletes(stored TEXT PRIMARY KEY, delete_after_utc TEXT NOT NULL);
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
    if v < 2 {
        tx.execute_batch(V2)?;
    }
    if v < 3 {
        tx.execute_batch(V3)?;
        let ids: Vec<i64> = {
            let mut st = tx.prepare(
                "SELECT id FROM tasks ORDER BY (completed_at IS NOT NULL), (due_utc IS NULL), due_utc, id",
            )?;
            let rows = st.query_map([], |r| r.get(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        for (i, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE tasks SET sort_order = ?1 WHERE id = ?2",
                rusqlite::params![(i + 1) as f64, id],
            )?;
        }
    }
    if v < 4 {
        tx.execute_batch(V4)?;
    }
    if v < 5 {
        tx.execute_batch(V5)?;
    }
    tx.pragma_update(None, "user_version", CURRENT_VERSION)?;
    tx.commit()?;
    Ok(())
}
