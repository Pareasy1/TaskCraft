use rusqlite::Connection;
use std::path::PathBuf;
use taskcraft_store::*;

struct Fx {
    _dir: tempfile::TempDir,
    path: PathBuf,
    s: Store,
}

fn fx() -> Fx {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let s = Store::open(&path).unwrap();
    Fx { _dir: dir, path, s }
}

impl Fx {
    fn raw(&self) -> Connection {
        Connection::open(&self.path).unwrap()
    }
    /// Inserts a task and places it directly via SQL.
    fn task(&self, title: &str, plan: i64, bucket: i64, sort: f64) -> i64 {
        let id = self
            .s
            .insert_task(&TaskInput {
                calendar_id: 1,
                title: title.into(),
                tz: "Europe/Rome".into(),
                ..Default::default()
            })
            .unwrap();
        self.raw()
            .execute(
                "UPDATE tasks SET plan_id=?1, bucket_id=?2, sort_order=?3 WHERE id=?4",
                rusqlite::params![plan, bucket, sort, id],
            )
            .unwrap();
        id
    }
    fn titles_in(&self, bucket: i64) -> Vec<String> {
        let c = self.raw();
        let mut st = c
            .prepare("SELECT title FROM tasks WHERE bucket_id=?1 ORDER BY sort_order, id")
            .unwrap();
        st.query_map([bucket], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }
    fn count(&self, sql: &str) -> i64 {
        self.raw().query_row(sql, [], |r| r.get(0)).unwrap()
    }
}

fn first_bucket(s: &Store, plan: i64) -> i64 {
    s.list_buckets(plan).unwrap()[0].id
}

#[test]
fn create_plan_has_one_bucket_and_goes_last() {
    let s = Store::open_in_memory().unwrap();
    let a = s.create_plan("  Casa ", "#ff0000").unwrap();
    let b = s.create_plan("Lavoro", "#00ff00").unwrap();
    let plans = s.list_plans().unwrap();
    assert_eq!(plans.iter().map(|p| p.id).collect::<Vec<_>>(), [1, a, b]);
    assert_eq!(plans[1].name, "Casa");
    let bs = s.list_buckets(b).unwrap();
    assert_eq!(bs.len(), 1);
    assert_eq!(bs[0].name, "To do");
}

#[test]
fn duplicate_plan_name_case_insensitive() {
    let s = Store::open_in_memory().unwrap();
    s.create_plan("Casa", "#fff").unwrap();
    assert!(matches!(
        s.create_plan("casa", "#fff"),
        Err(StoreError::Duplicate)
    ));
    let other = s.create_plan("Altro", "#fff").unwrap();
    assert!(matches!(
        s.update_plan(other, Some("CASA"), None),
        Err(StoreError::Duplicate)
    ));
    // renaming to its own name in different case is fine
    s.update_plan(other, Some("ALTRO"), Some("#000")).unwrap();
    let p = s
        .list_plans()
        .unwrap()
        .into_iter()
        .find(|p| p.id == other)
        .unwrap();
    assert_eq!((p.name.as_str(), p.color.as_str()), ("ALTRO", "#000"));
}

#[test]
fn empty_plan_name_rejected() {
    let s = Store::open_in_memory().unwrap();
    assert!(matches!(
        s.create_plan("  ", "#fff"),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.update_plan(1, Some(" "), None),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.create_bucket(1, " "),
        Err(StoreError::Invalid(_))
    ));
}

#[test]
fn cannot_delete_last_plan() {
    let s = Store::open_in_memory().unwrap();
    assert!(matches!(
        s.delete_plan(1, DeleteMode::DeleteTasks),
        Err(StoreError::LastPlan)
    ));
}

#[test]
fn cannot_delete_last_bucket() {
    let s = Store::open_in_memory().unwrap();
    assert!(matches!(
        s.delete_bucket(1, DeleteMode::DeleteTasks),
        Err(StoreError::LastBucket)
    ));
}

#[test]
fn delete_plan_move_to_keeps_tasks() {
    let f = fx();
    let b = f.s.create_plan("B", "#fff").unwrap();
    let bb = first_bucket(&f.s, b);
    f.task("a1", 1, 1, 5.0);
    f.task("b2", b, bb, 20.0);
    f.task("b1", b, bb, 10.0);
    f.s.delete_plan(b, DeleteMode::MoveTo(1)).unwrap();
    assert_eq!(f.titles_in(1), ["a1", "b1", "b2"]);
    assert_eq!(f.count("SELECT count(*) FROM tasks WHERE plan_id=1"), 3);
    assert_eq!(f.s.list_plans().unwrap().len(), 1);
    assert_eq!(f.count("SELECT count(*) FROM buckets"), 1);
    assert!(matches!(
        f.s.delete_plan(1, DeleteMode::MoveTo(1)),
        Err(StoreError::LastPlan)
    ));
}

#[test]
fn delete_plan_move_to_self_or_missing_rejected() {
    let s = Store::open_in_memory().unwrap();
    let b = s.create_plan("B", "#fff").unwrap();
    assert!(matches!(
        s.delete_plan(b, DeleteMode::MoveTo(b)),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.delete_plan(b, DeleteMode::MoveTo(99)),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.delete_plan(99, DeleteMode::DeleteTasks),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn delete_plan_delete_tasks_removes_tasks_and_alerts() {
    let f = fx();
    let b = f.s.create_plan("B", "#fff").unwrap();
    let bb = first_bucket(&f.s, b);
    let t =
        f.s.insert_task(&TaskInput {
            calendar_id: 1,
            title: "x".into(),
            tz: "Europe/Rome".into(),
            due: Some(chrono::Utc::now() + chrono::Duration::days(1)),
            alerts: vec![AlertSpecUtc::At(
                chrono::Utc::now() + chrono::Duration::hours(5),
            )],
            labels: vec!["home".into()],
            ..Default::default()
        })
        .unwrap();
    f.raw()
        .execute(
            "UPDATE tasks SET plan_id=?1, bucket_id=?2 WHERE id=?3",
            rusqlite::params![b, bb, t],
        )
        .unwrap();
    assert_eq!(f.count("SELECT count(*) FROM alerts"), 1);
    f.s.delete_plan(b, DeleteMode::DeleteTasks).unwrap();
    assert_eq!(f.count("SELECT count(*) FROM tasks"), 0);
    assert_eq!(f.count("SELECT count(*) FROM alerts"), 0);
    assert_eq!(f.count("SELECT count(*) FROM task_labels"), 0);
    assert_eq!(f.s.list_labels().unwrap().len(), 1);
}

#[test]
fn delete_default_plan_reassigns_default() {
    let s = Store::open_in_memory().unwrap();
    let b = s.create_plan("B", "#fff").unwrap();
    let c = s.create_plan("C", "#fff").unwrap();
    s.set_default_plan(b).unwrap();
    assert!(matches!(s.set_default_plan(99), Err(StoreError::NotFound)));
    s.reorder_plans(&[c, b, 1]).unwrap();
    s.delete_plan(b, DeleteMode::DeleteTasks).unwrap();
    assert_eq!(s.default_plan_id().unwrap(), c);
}

#[test]
fn delete_bucket_move_to_other_plan_rejected() {
    let s = Store::open_in_memory().unwrap();
    let b = s.create_plan("B", "#fff").unwrap();
    let other = first_bucket(&s, b);
    let extra = s.create_bucket(1, "Doing").unwrap();
    assert!(matches!(
        s.delete_bucket(extra, DeleteMode::MoveTo(other)),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.delete_bucket(extra, DeleteMode::MoveTo(extra)),
        Err(StoreError::Invalid(_))
    ));
}

#[test]
fn delete_bucket_move_to_appends_and_delete_tasks() {
    let f = fx();
    let b2 = f.s.create_bucket(1, "Doing").unwrap();
    let b3 = f.s.create_bucket(1, "Done").unwrap();
    f.task("a", 1, 1, 3.0);
    f.task("y", 1, b2, 9.0);
    f.task("x", 1, b2, 4.0);
    f.s.delete_bucket(b2, DeleteMode::MoveTo(1)).unwrap();
    assert_eq!(f.titles_in(1), ["a", "x", "y"]);
    f.task("z", 1, b3, 1.0);
    f.s.delete_bucket(b3, DeleteMode::DeleteTasks).unwrap();
    assert_eq!(f.count("SELECT count(*) FROM tasks WHERE title='z'"), 0);
    assert_eq!(f.s.list_buckets(1).unwrap().len(), 1);
}

#[test]
fn reorder_plans_and_buckets() {
    let s = Store::open_in_memory().unwrap();
    let b = s.create_plan("B", "#fff").unwrap();
    s.reorder_plans(&[b, 1]).unwrap();
    assert_eq!(
        s.list_plans()
            .unwrap()
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>(),
        [b, 1]
    );
    assert!(matches!(s.reorder_plans(&[1]), Err(StoreError::Invalid(_))));
    assert!(matches!(
        s.reorder_plans(&[1, 1, b]),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.reorder_plans(&[1, 77]),
        Err(StoreError::Invalid(_))
    ));

    let b2 = s.create_bucket(1, "Doing").unwrap();
    s.rename_bucket(b2, "In corso").unwrap();
    s.reorder_buckets(1, &[b2, 1]).unwrap();
    let bs = s.list_buckets(1).unwrap();
    assert_eq!(bs.iter().map(|x| x.id).collect::<Vec<_>>(), [b2, 1]);
    assert_eq!(bs[0].name, "In corso");
    assert!(matches!(
        s.reorder_buckets(1, &[b2]),
        Err(StoreError::Invalid(_))
    ));
    assert!(matches!(
        s.rename_bucket(99, "x"),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn localize_seed_renames_once() {
    let s = Store::open_in_memory().unwrap();
    s.localize_seed("Le mie attività", "Da fare").unwrap();
    s.localize_seed("Other", "Other").unwrap();
    assert_eq!(s.list_plans().unwrap()[0].name, "Le mie attività");
    assert_eq!(s.list_buckets(1).unwrap()[0].name, "Da fare");
    let p = s.create_plan("Nuovo", "#fff").unwrap();
    assert_eq!(s.list_buckets(p).unwrap()[0].name, "Da fare");
}

#[test]
fn localize_seed_keeps_user_renamed() {
    let s = Store::open_in_memory().unwrap();
    s.update_plan(1, Some("Lavoro"), None).unwrap();
    s.rename_bucket(1, "Mio").unwrap();
    s.localize_seed("Le mie attività", "Da fare").unwrap();
    assert_eq!(s.list_plans().unwrap()[0].name, "Lavoro");
    assert_eq!(s.list_buckets(1).unwrap()[0].name, "Mio");
}

#[test]
fn duplicate_plan_name_unicode_case() {
    let f = fx();
    let citta = f.s.create_plan("Città", "#fff").unwrap();
    assert!(matches!(
        f.s.create_plan("CITTÀ", "#fff"),
        Err(StoreError::Duplicate)
    ));
    let att = f.s.create_plan("Attività", "#fff").unwrap();
    assert!(matches!(
        f.s.update_plan(att, Some("città"), None),
        Err(StoreError::Duplicate)
    ));
    f.s.update_plan(citta, Some("CITTÀ"), None).unwrap();
}

#[test]
fn names_capped_at_60_chars() {
    let s = Store::open_in_memory().unwrap();
    let ok = "é".repeat(60);
    let long = "é".repeat(61);
    s.create_plan(&ok, "#fff").unwrap();
    assert!(matches!(
        s.create_plan(&long, "#fff"),
        Err(StoreError::Invalid(_))
    ));
    let b = first_bucket(&s, 1);
    s.rename_bucket(b, &ok).unwrap();
    assert!(matches!(
        s.rename_bucket(b, &long),
        Err(StoreError::Invalid(_))
    ));
}
