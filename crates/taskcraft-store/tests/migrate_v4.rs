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

const V2: &str = "
ALTER TABLE tasks ADD COLUMN status TEXT NOT NULL DEFAULT 'todo' CHECK(status IN ('todo','in_progress','done'));
CREATE TABLE labels(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, color TEXT NOT NULL);
CREATE TABLE task_labels(
  task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  label_id INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
  PRIMARY KEY(task_id, label_id)
);";

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

fn v3_db(path: &std::path::Path) {
    let c = Connection::open(path).unwrap();
    c.pragma_update(None, "journal_mode", "WAL").unwrap();
    c.execute_batch(V1).unwrap();
    c.execute_batch(V2).unwrap();
    c.execute_batch(V3).unwrap();
    c.execute_batch(
        "INSERT INTO tasks(calendar_id, title, tz, created_at, updated_at, plan_id, bucket_id, sort_order)
           VALUES (1, 'a', 'UTC', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z', 1, 1, 1.0);
         INSERT INTO tasks(calendar_id, title, tz, created_at, updated_at, plan_id, bucket_id, sort_order)
           VALUES (1, 'b', 'UTC', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z', 1, 1, 2.5);
         PRAGMA user_version = 3;",
    )
    .unwrap();
}

#[test]
fn v3_tasks_keep_everything() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taskcraft.db");
    v3_db(&path);
    let s = Store::open(&path).unwrap();
    assert_eq!(s.schema_version().unwrap(), 5);
    for (id, sort) in [(1, 1.0), (2, 2.5)] {
        let t = s.get_task(id).unwrap();
        assert_eq!(t.priority, Priority::Medium);
        assert_eq!(t.start, None);
        assert!(!t.start_has_time);
        assert_eq!(t.parent_id, None);
        assert_eq!((t.plan_id, t.bucket_id, t.sort_order), (1, 1, sort));
    }
    assert!(dir.path().join("taskcraft.db.bak-v3").exists());
}

/// Run manually against a copy of a real database:
/// `TASKCRAFT_MIGRATE_CHECK=/path/to/copy.db cargo test -p taskcraft-store --test migrate_v4 -- --ignored`
#[test]
#[ignore]
fn real_db_copy_v4() {
    let path = std::env::var("TASKCRAFT_MIGRATE_CHECK").expect("TASKCRAFT_MIGRATE_CHECK not set");
    let path = std::path::Path::new(&path);
    let snapshot = |p: &std::path::Path| -> Vec<(i64, Option<i64>, Option<i64>)> {
        let c = Connection::open(p).unwrap();
        let mut st = c
            .prepare("SELECT id, plan_id, bucket_id FROM tasks ORDER BY id")
            .unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    let before = snapshot(path);
    let store = Store::open(path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 5);
    drop(store);
    assert_eq!(snapshot(path), before, "task count/plan/bucket changed");
    let c = Connection::open(path).unwrap();
    let n: i64 = c
        .query_row(
            "SELECT count(*) FROM tasks WHERE priority = 'medium' AND parent_id IS NULL AND start_utc IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n as usize, before.len());
}
