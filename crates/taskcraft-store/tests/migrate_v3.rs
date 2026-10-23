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

fn v2_db(path: &std::path::Path) {
    let c = Connection::open(path).unwrap();
    c.pragma_update(None, "journal_mode", "WAL").unwrap();
    c.execute_batch(V1).unwrap();
    c.execute_batch(V2).unwrap();
    c.execute_batch(
        "INSERT INTO tasks(calendar_id, title, due_utc, tz, created_at, updated_at)
           VALUES (1, 'due25', '2026-10-25T00:00:00Z', 'UTC', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');
         INSERT INTO tasks(calendar_id, title, due_utc, tz, created_at, updated_at)
           VALUES (1, 'due21', '2026-10-21T00:00:00Z', 'UTC', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');
         INSERT INTO tasks(calendar_id, title, tz, created_at, updated_at)
           VALUES (1, 'nodue', 'UTC', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');
         PRAGMA user_version = 2;",
    )
    .unwrap();
}

fn all_tasks(s: &Store) -> Vec<Task> {
    let mut v = vec![];
    for id in 1..=3 {
        v.push(s.get_task(id).unwrap());
    }
    v
}

#[test]
fn v2_tasks_land_in_seed_plan_in_list_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taskcraft.db");
    v2_db(&path);
    let s = Store::open(&path).unwrap();
    assert_eq!(s.schema_version().unwrap(), 4);
    let mut tasks = all_tasks(&s);
    assert!(tasks.iter().all(|t| t.plan_id == 1 && t.bucket_id == 1));
    tasks.sort_by(|a, b| a.sort_order.partial_cmp(&b.sort_order).unwrap());
    let titles: Vec<_> = tasks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, ["due21", "due25", "nodue"]);
    let orders: Vec<_> = tasks.iter().map(|t| t.sort_order).collect();
    assert_eq!(orders, [1.0, 2.0, 3.0]);
}

#[test]
fn fresh_db_has_seed_plan_and_bucket() {
    let s = Store::open_in_memory().unwrap();
    let plans = s.list_plans().unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].name, "My tasks");
    let buckets = s.list_buckets(1).unwrap();
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0].name, "To do");
    assert_eq!(s.default_plan_id().unwrap(), 1);
}

#[test]
fn migration_writes_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taskcraft.db");
    v2_db(&path);
    drop(Store::open(&path).unwrap());
    assert!(dir.path().join("taskcraft.db.bak-v2").exists());
}

#[test]
#[ignore]
fn real_db_copy() {
    let path = std::env::var("TASKCRAFT_MIGRATE_CHECK").expect("TASKCRAFT_MIGRATE_CHECK");
    let before: i64 = Connection::open(&path)
        .unwrap()
        .query_row("SELECT count(*) FROM tasks", [], |r| r.get(0))
        .unwrap();
    let s = Store::open(std::path::Path::new(&path)).unwrap();
    let after: i64 = Connection::open(&path)
        .unwrap()
        .query_row("SELECT count(*) FROM tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(s.schema_version().unwrap(), 4);
    let orphans: i64 = Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT count(*) FROM tasks t JOIN buckets b ON b.id = t.bucket_id WHERE b.plan_id != t.plan_id",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(orphans, 0);
    let missing: i64 = Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT count(*) FROM tasks t WHERE NOT EXISTS (SELECT 1 FROM buckets b WHERE b.id = t.bucket_id AND b.plan_id = t.plan_id)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(missing, 0);
}
