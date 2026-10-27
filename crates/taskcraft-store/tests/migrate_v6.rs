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

const V4: &str = "
ALTER TABLE tasks ADD COLUMN parent_id INTEGER;
ALTER TABLE tasks ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('urgent','important','medium','low'));
ALTER TABLE tasks ADD COLUMN start_utc TEXT;
ALTER TABLE tasks ADD COLUMN start_has_time INTEGER NOT NULL DEFAULT 0;
CREATE TABLE checklist_items(id INTEGER PRIMARY KEY, task_id INTEGER NOT NULL, text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0, sort REAL NOT NULL);
CREATE INDEX tasks_parent ON tasks(parent_id, sort_order);
CREATE INDEX checklist_task ON checklist_items(task_id, sort);
";

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

fn v5_db(path: &std::path::Path) {
    let c = Connection::open(path).unwrap();
    c.pragma_update(None, "journal_mode", "WAL").unwrap();
    for s in [V1, V2, V3, V4, V5] {
        c.execute_batch(s).unwrap();
    }
    c.execute_batch(
        "INSERT INTO tasks(calendar_id, title, tz, created_at, updated_at, plan_id, bucket_id, sort_order, priority)
           VALUES (1, 'a', 'UTC', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z', 1, 1, 1.0, 'urgent');
         PRAGMA user_version = 5;",
    )
    .unwrap();
}

#[test]
fn v5_keeps_everything() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taskcraft.db");
    v5_db(&path);
    let s = Store::open(&path).unwrap();
    assert_eq!(s.schema_version().unwrap(), 6);
    let t = s.get_task(1).unwrap();
    assert_eq!(
        (t.title.as_str(), t.priority, t.blocked_by),
        ("a", Priority::Urgent, 0)
    );
    assert!(s.list_dependencies(None).unwrap().is_empty());
    assert!(dir.path().join("taskcraft.db.bak-v5").exists());
}

/// Run manually against a COPY of a real database:
/// `TASKCRAFT_MIGRATE_CHECK=/path/copy.db cargo test -p taskcraft-store --test migrate_v6 -- --ignored real_db_copy_v6 --nocapture`
#[test]
#[ignore]
fn real_db_copy_v6() {
    let path = std::env::var("TASKCRAFT_MIGRATE_CHECK").expect("set TASKCRAFT_MIGRATE_CHECK");
    let path = std::path::PathBuf::from(path);
    let (tasks, subs) = {
        let c = Connection::open(&path).unwrap();
        let n = |sql: &str| c.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap();
        (
            n("SELECT COUNT(*) FROM tasks"),
            n("SELECT COUNT(*) FROM tasks WHERE parent_id IS NOT NULL"),
        )
    };
    println!("before: tasks={tasks} subtasks={subs}");
    let s = Store::open(&path).unwrap();
    assert_eq!(s.schema_version().unwrap(), 6);
    drop(s);
    let c = Connection::open(&path).unwrap();
    let n = |sql: &str| c.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap();
    assert_eq!(n("SELECT COUNT(*) FROM tasks"), tasks);
    assert_eq!(
        n("SELECT COUNT(*) FROM tasks WHERE parent_id IS NOT NULL"),
        subs
    );
    assert_eq!(n("SELECT COUNT(*) FROM dependencies"), 0);
    println!("after: tasks={tasks} subtasks={subs}");
}
