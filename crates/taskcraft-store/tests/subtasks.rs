use chrono::{TimeZone, Utc};
use taskcraft_store::*;

fn store() -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let s = Store::open(&path).unwrap();
    (dir, path, s)
}

fn input(title: &str) -> TaskInput {
    TaskInput {
        calendar_id: 1,
        title: title.into(),
        tz: "UTC".into(),
        ..Default::default()
    }
}

fn child(title: &str, parent: i64) -> TaskInput {
    TaskInput {
        parent_id: Some(parent),
        ..input(title)
    }
}

fn is_rule(r: Result<i64>, want: Rule) {
    match r {
        Err(StoreError::Rule(got)) => assert_eq!(got, want),
        other => panic!("expected Rule({want:?}), got {other:?}"),
    }
}

#[test]
fn insert_subtask_copies_parent_placement() {
    let (_d, _p, s) = store();
    let plan = s.create_plan("Work", "#fff").unwrap();
    let bucket = s.list_buckets(plan).unwrap()[0].id;
    let parent = s
        .insert_task(&TaskInput {
            plan_id: Some(plan),
            bucket_id: Some(bucket),
            ..input("parent")
        })
        .unwrap();
    // Input plan/bucket are ignored for subtasks.
    let a = s
        .insert_task(&TaskInput {
            plan_id: Some(1),
            bucket_id: Some(1),
            ..child("a", parent)
        })
        .unwrap();
    let b = s.insert_task(&child("b", parent)).unwrap();
    let (ta, tb) = (s.get_task(a).unwrap(), s.get_task(b).unwrap());
    assert_eq!((ta.plan_id, ta.bucket_id), (plan, bucket));
    assert_eq!(ta.parent_id, Some(parent));
    assert_eq!(tb.sort_order, ta.sort_order + 1.0);
}

#[test]
fn subtask_of_subtask_rejected() {
    let (_d, _p, s) = store();
    let p = s.insert_task(&input("p")).unwrap();
    let c = s.insert_task(&child("c", p)).unwrap();
    is_rule(s.insert_task(&child("g", c)), Rule::ParentIsSubtask);
}

#[test]
fn parent_with_children_cannot_become_subtask() {
    let (_d, _p, s) = store();
    let p = s.insert_task(&input("p")).unwrap();
    s.insert_task(&child("c", p)).unwrap();
    let other = s.insert_task(&input("other")).unwrap();
    let r = s.update_task(p, &child("p", other)).map(|_| 0);
    is_rule(r, Rule::HasSubtasks);
}

#[test]
fn repeating_subtask_rejected() {
    let (_d, _p, s) = store();
    let p = s.insert_task(&input("p")).unwrap();
    let rep = TaskInput {
        rrule: Some("FREQ=DAILY".into()),
        due: Some(Utc.with_ymd_and_hms(2026, 10, 25, 0, 0, 0).unwrap()),
        ..child("c", p)
    };
    is_rule(s.insert_task(&rep), Rule::RepeatingSubtask);
    // A repeating parent is fine.
    let rp = s
        .insert_task(&TaskInput {
            rrule: Some("FREQ=DAILY".into()),
            due: Some(Utc.with_ymd_and_hms(2026, 10, 25, 0, 0, 0).unwrap()),
            ..input("rp")
        })
        .unwrap();
    s.insert_task(&child("ok", rp)).unwrap();
}

#[test]
fn self_parent_rejected() {
    let (_d, _p, s) = store();
    let p = s.insert_task(&input("p")).unwrap();
    let r = s.update_task(p, &child("p", p));
    assert!(matches!(r, Err(StoreError::Invalid(_))), "{r:?}");
    let r = s.insert_task(&child("x", 9999));
    assert!(matches!(r, Err(StoreError::NotFound)), "{r:?}");
}

#[test]
fn start_after_due_rejected() {
    let (_d, _p, s) = store();
    let due = Utc.with_ymd_and_hms(2026, 10, 25, 9, 0, 0).unwrap();
    let late = due + chrono::Duration::hours(1);
    let bad = TaskInput {
        due: Some(due),
        start: Some(late),
        ..input("x")
    };
    is_rule(s.insert_task(&bad), Rule::StartAfterDue);
    let ok = TaskInput {
        due: Some(due),
        start: Some(due),
        ..input("x")
    };
    s.insert_task(&ok).unwrap();
}

#[test]
fn priority_and_start_round_trip() {
    let (_d, _p, s) = store();
    let start = Utc.with_ymd_and_hms(2026, 10, 24, 8, 30, 0).unwrap();
    let id = s
        .insert_task(&TaskInput {
            priority: Priority::Urgent,
            start: Some(start),
            start_has_time: true,
            ..input("x")
        })
        .unwrap();
    let t = s.get_task(id).unwrap();
    assert_eq!(t.priority, Priority::Urgent);
    assert_eq!(t.start, Some(start));
    assert!(t.start_has_time);
    s.update_task(
        id,
        &TaskInput {
            priority: Priority::Low,
            ..input("x")
        },
    )
    .unwrap();
    let t = s.get_task(id).unwrap();
    assert_eq!(t.priority, Priority::Low);
    assert_eq!(t.start, None);
}

#[test]
fn counts_on_task() {
    let (_d, path, s) = store();
    let p = s.insert_task(&input("p")).unwrap();
    let ids: Vec<i64> = (0..3)
        .map(|i| s.insert_task(&child(&format!("c{i}"), p)).unwrap())
        .collect();
    s.set_task_completed(ids[0], true, Utc::now()).unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute(
        &format!(
            "INSERT INTO checklist_items(task_id, text, done, sort) VALUES ({p}, 'x', 1, 1.0), ({p}, 'y', 0, 2.0)"
        ),
        [],
    )
    .unwrap();
    let t = s.get_task(p).unwrap();
    assert_eq!((t.subtask_open, t.subtask_total), (2, 3));
    assert_eq!((t.checklist_done, t.checklist_total), (1, 2));
    let sub = s.get_task(ids[1]).unwrap();
    assert_eq!((sub.subtask_open, sub.subtask_total), (0, 0));
    let listed = s
        .list_tasks(TaskFilter::All, Utc::now(), chrono_tz::Tz::UTC)
        .unwrap();
    let lp = listed.iter().find(|t| t.id == p).unwrap();
    assert_eq!(
        (
            lp.subtask_open,
            lp.subtask_total,
            lp.checklist_done,
            lp.checklist_total
        ),
        (2, 3, 1, 2)
    );
}

// ---- Task 2: behaviour ----

fn raw(path: &std::path::Path) -> rusqlite::Connection {
    rusqlite::Connection::open(path).unwrap()
}

fn count(path: &std::path::Path, sql: &str) -> i64 {
    raw(path).query_row(sql, [], |r| r.get(0)).unwrap()
}

fn ids(v: Vec<Task>) -> Vec<i64> {
    v.into_iter().map(|t| t.id).collect()
}

fn add_check(path: &std::path::Path, task: i64, done: i64) {
    raw(path)
        .execute(
            "INSERT INTO checklist_items(task_id, text, done, sort) VALUES (?1, 'x', ?2, 1.0)",
            rusqlite::params![task, done],
        )
        .unwrap();
}

/// A parent with two subtasks in a fresh plan; returns (plan, bucket, parent, [a, b]).
fn family(s: &Store) -> (i64, i64, i64, [i64; 2]) {
    let plan = s.create_plan("Fam", "#fff").unwrap();
    let bucket = s.list_buckets(plan).unwrap()[0].id;
    let p = s
        .insert_task(&TaskInput {
            plan_id: Some(plan),
            bucket_id: Some(bucket),
            ..input("p")
        })
        .unwrap();
    let a = s.insert_task(&child("a", p)).unwrap();
    let b = s.insert_task(&child("b", p)).unwrap();
    (plan, bucket, p, [a, b])
}

fn placed(s: &Store, id: i64) -> (i64, i64) {
    let t = s.get_task(id).unwrap();
    (t.plan_id, t.bucket_id)
}

#[test]
fn list_and_reorder_subtasks() {
    let (_d, _p, s) = store();
    let (_, _, p, [a, b]) = family(&s);
    assert_eq!(ids(s.list_subtasks(p).unwrap()), vec![a, b]);
    s.reorder_subtasks(p, &[b, a]).unwrap();
    assert_eq!(ids(s.list_subtasks(p).unwrap()), vec![b, a]);
    for bad in [vec![a], vec![a, b, 999], vec![a, a]] {
        assert!(matches!(
            s.reorder_subtasks(p, &bad),
            Err(StoreError::Invalid(_))
        ));
    }
    assert_eq!(ids(s.list_subtasks(p).unwrap()), vec![b, a]);
}

#[test]
fn subtasks_follow_parent_everywhere() {
    let (_d, _p, s) = store();
    let (plan, bucket, p, kids) = family(&s);
    // move_task
    let b2 = s.create_bucket(plan, "B2").unwrap();
    s.move_task(p, b2, None, None).unwrap();
    for k in kids {
        assert_eq!(placed(&s, k), (plan, b2));
    }
    assert_eq!(ids(s.list_subtasks(p).unwrap()), kids.to_vec());
    // update_task plan change
    let plan2 = s.create_plan("Other", "#000").unwrap();
    let b_other = s.list_buckets(plan2).unwrap()[0].id;
    s.update_task(
        p,
        &TaskInput {
            plan_id: Some(plan2),
            ..input("p")
        },
    )
    .unwrap();
    for k in kids {
        assert_eq!(placed(&s, k), (plan2, b_other));
    }
    // delete_bucket MoveTo
    let b3 = s.create_bucket(plan2, "B3").unwrap();
    s.delete_bucket(b_other, DeleteMode::MoveTo(b3)).unwrap();
    for k in kids {
        assert_eq!(placed(&s, k), (plan2, b3));
    }
    assert_eq!(ids(s.list_subtasks(p).unwrap()), kids.to_vec());
    // delete_plan MoveTo
    s.delete_plan(plan2, DeleteMode::MoveTo(plan)).unwrap();
    let want = placed(&s, p);
    assert_eq!(want.0, plan);
    for k in kids {
        assert_eq!(placed(&s, k), want);
    }
    assert_eq!(ids(s.list_subtasks(p).unwrap()), kids.to_vec());
    let _ = bucket;
}

#[test]
fn move_subtask_rejected() {
    let (_d, _p, s) = store();
    let (_, bucket, _, [a, _]) = family(&s);
    let r = s.move_task(a, bucket, None, None);
    assert!(matches!(r, Err(StoreError::Invalid(_))), "{r:?}");
}

#[test]
fn promote_subtask_appends_to_bucket() {
    let (_d, _p, s) = store();
    let (plan, bucket, p, [a, _]) = family(&s);
    let other = s
        .insert_task(&TaskInput {
            plan_id: Some(plan),
            bucket_id: Some(bucket),
            ..input("other")
        })
        .unwrap();
    s.set_task_parent(a, None).unwrap();
    let (ta, to) = (s.get_task(a).unwrap(), s.get_task(other).unwrap());
    assert_eq!(ta.parent_id, None);
    assert_eq!((ta.plan_id, ta.bucket_id), (plan, bucket));
    assert!(ta.sort_order > to.sort_order);
    assert_eq!(s.get_task(p).unwrap().subtask_total, 1);
}

#[test]
fn update_clearing_parent_appends_to_bucket() {
    let (_d, _p, s) = store();
    let (plan, bucket, _p2, [a, _]) = family(&s);
    let other = s
        .insert_task(&TaskInput {
            plan_id: Some(plan),
            bucket_id: Some(bucket),
            ..input("other")
        })
        .unwrap();
    s.update_task(a, &input("a")).unwrap();
    let (ta, to) = (s.get_task(a).unwrap(), s.get_task(other).unwrap());
    assert_eq!(ta.parent_id, None);
    assert_eq!((ta.plan_id, ta.bucket_id), (plan, bucket));
    assert!(ta.sort_order > to.sort_order);
}

#[test]
fn demote_task_under_parent() {
    let (_d, _p, s) = store();
    let (_, _, p, [a, b]) = family(&s);
    let x = s.insert_task(&input("x")).unwrap();
    s.set_task_parent(x, Some(p)).unwrap();
    assert_eq!(ids(s.list_subtasks(p).unwrap()), vec![a, b, x]);
    assert_eq!(placed(&s, x), placed(&s, p));
    // Same rules as Task 1.
    let r = s.set_task_parent(p, Some(x));
    assert!(
        matches!(r, Err(StoreError::Rule(Rule::ParentIsSubtask))),
        "{r:?}"
    );
    let y = s.insert_task(&input("y")).unwrap();
    let r = s.set_task_parent(p, Some(y));
    assert!(
        matches!(r, Err(StoreError::Rule(Rule::HasSubtasks))),
        "{r:?}"
    );
}

#[test]
fn delete_parent_cascades() {
    let (_d, path, s) = store();
    let (_, _, p, [a, _]) = family(&s);
    add_check(&path, p, 0);
    add_check(&path, a, 1);
    let l = s
        .insert_task(&TaskInput {
            labels: vec!["home".into()],
            ..child("lab", p)
        })
        .unwrap();
    let _ = l;
    raw(&path)
        .execute(
            &format!("INSERT INTO alerts(owner_kind, owner_id, offset_min, fire_at_utc) VALUES ('task', {a}, 5, '2026-10-25T00:00:00Z')"),
            [],
        )
        .unwrap();
    s.delete_task(p).unwrap();
    assert_eq!(
        count(
            &path,
            "SELECT count(*) FROM tasks WHERE title IN ('p','a','b','lab')"
        ),
        0
    );
    assert_eq!(count(&path, "SELECT count(*) FROM checklist_items"), 0);
    assert_eq!(count(&path, "SELECT count(*) FROM alerts"), 0);
    assert_eq!(count(&path, "SELECT count(*) FROM task_labels"), 0);
    assert_eq!(s.list_labels().unwrap().len(), 1);
}

#[test]
fn delete_plan_delete_tasks_cascades() {
    let (_d, path, s) = store();
    let (plan, _, p, [a, _]) = family(&s);
    add_check(&path, p, 0);
    add_check(&path, a, 0);
    s.delete_plan(plan, DeleteMode::DeleteTasks).unwrap();
    assert_eq!(count(&path, "SELECT count(*) FROM tasks"), 0);
    assert_eq!(count(&path, "SELECT count(*) FROM checklist_items"), 0);
    // Same through delete_bucket.
    let (plan, bucket, p, [a, _]) = family(&s);
    let spare = s.create_bucket(plan, "spare").unwrap();
    let _ = spare;
    add_check(&path, a, 0);
    add_check(&path, p, 0);
    s.delete_bucket(bucket, DeleteMode::DeleteTasks).unwrap();
    assert_eq!(count(&path, "SELECT count(*) FROM tasks"), 0);
    assert_eq!(count(&path, "SELECT count(*) FROM checklist_items"), 0);
}

#[test]
fn repeat_reset() {
    let (_d, path, s) = store();
    let due = Utc.with_ymd_and_hms(2026, 10, 25, 9, 0, 0).unwrap();
    let p = s
        .insert_task(&TaskInput {
            rrule: Some("FREQ=DAILY".into()),
            due: Some(due),
            due_has_time: true,
            ..input("rp")
        })
        .unwrap();
    let a = s.insert_task(&child("a", p)).unwrap();
    let b = s.insert_task(&child("b", p)).unwrap();
    s.set_task_completed(a, true, Utc::now()).unwrap();
    add_check(&path, p, 1);
    add_check(&path, a, 1);
    s.set_task_completed(p, true, Utc::now()).unwrap();
    let t = s.get_task(p).unwrap();
    assert_eq!(t.due, Some(due + chrono::Duration::days(1)));
    for k in [a, b] {
        let k = s.get_task(k).unwrap();
        assert_eq!(k.status, TaskStatus::Todo);
        assert_eq!(k.completed_at, None);
    }
    assert_eq!(
        count(&path, "SELECT count(*) FROM checklist_items WHERE done = 1"),
        0
    );
    assert_eq!(t.subtask_open, 2);
}

#[test]
fn complete_with_subtasks_marks_all() {
    let (_d, _p, s) = store();
    let (_, _, p, [a, b]) = family(&s);
    s.complete_with_subtasks(p, Utc::now()).unwrap();
    for id in [p, a, b] {
        let t = s.get_task(id).unwrap();
        assert!(t.completed_at.is_some());
        assert_eq!(t.status, TaskStatus::Done);
    }
}

#[test]
fn complete_only_parent() {
    let (_d, _p, s) = store();
    let (_, _, p, [a, b]) = family(&s);
    s.set_task_completed(p, true, Utc::now()).unwrap();
    assert!(s.get_task(p).unwrap().completed_at.is_some());
    for id in [a, b] {
        assert!(s.get_task(id).unwrap().completed_at.is_none());
    }
}

#[test]
fn update_parent_keeps_children() {
    let (_d, path, s) = store();
    let (_, _, p, [a, b]) = family(&s);
    add_check(&path, p, 1);
    s.update_task(
        p,
        &TaskInput {
            status: TaskStatus::InProgress,
            ..input("renamed")
        },
    )
    .unwrap();
    assert_eq!(ids(s.list_subtasks(p).unwrap()), vec![a, b]);
    let t = s.get_task(p).unwrap();
    assert_eq!(
        (t.subtask_total, t.checklist_total, t.checklist_done),
        (2, 1, 1)
    );
}

#[test]
fn today_excludes_not_started() {
    let (_d, path, s) = store();
    let now = Utc.with_ymd_and_hms(2026, 10, 25, 10, 0, 0).unwrap();
    let due = Utc.with_ymd_and_hms(2026, 10, 25, 18, 0, 0).unwrap();
    let mk = |title: &str, start| {
        s.insert_task(&TaskInput {
            due: Some(due),
            due_has_time: true,
            start: Some(start),
            start_has_time: true,
            ..input(title)
        })
        .unwrap()
    };
    let later = mk(
        "later",
        Utc.with_ymd_and_hms(2026, 10, 25, 1, 0, 0).unwrap(),
    );
    // The API forbids start after due; force it to cover the filter itself.
    raw(&path)
        .execute(
            &format!("UPDATE tasks SET start_utc = '2026-10-26T00:00:00Z' WHERE id = {later}"),
            [],
        )
        .unwrap();
    let started = mk(
        "started",
        Utc.with_ymd_and_hms(2026, 10, 25, 12, 0, 0).unwrap(),
    );
    let today = ids(s
        .list_tasks(TaskFilter::Today, now, chrono_tz::Tz::UTC)
        .unwrap());
    assert!(today.contains(&started));
    assert!(!today.contains(&later));
}

#[test]
fn plan_open_count_top_level_only() {
    let (_d, _p, s) = store();
    let (plan, _, _, _) = family(&s);
    let n = s
        .list_plans()
        .unwrap()
        .into_iter()
        .find(|p| p.id == plan)
        .unwrap()
        .open_count;
    assert_eq!(n, 1);
}

#[test]
fn complete_with_subtasks_on_repeating_parent() {
    let (_d, _p, s) = store();
    let due = Utc.with_ymd_and_hms(2026, 10, 25, 9, 0, 0).unwrap();
    let p = s
        .insert_task(&TaskInput {
            rrule: Some("FREQ=DAILY".into()),
            due: Some(due),
            due_has_time: true,
            ..input("rp")
        })
        .unwrap();
    let a = s.insert_task(&child("a", p)).unwrap();
    let b = s.insert_task(&child("b", p)).unwrap();
    s.set_task_completed(a, true, Utc::now()).unwrap();
    s.complete_with_subtasks(p, Utc::now()).unwrap();
    let t = s.get_task(p).unwrap();
    assert_eq!(t.due, Some(due + chrono::Duration::days(1)));
    assert!(t.completed_at.is_none());
    for k in [a, b] {
        let k = s.get_task(k).unwrap();
        assert_eq!(k.status, TaskStatus::Todo);
        assert_eq!(k.completed_at, None);
    }
}

#[test]
fn repeat_reset_rearms_future_subtask_alerts() {
    let (_d, path, s) = store();
    let now = Utc.with_ymd_and_hms(2026, 10, 23, 12, 0, 0).unwrap();
    let p = s
        .insert_task(&TaskInput {
            rrule: Some("FREQ=DAILY".into()),
            due: Some(Utc.with_ymd_and_hms(2026, 10, 25, 9, 0, 0).unwrap()),
            due_has_time: true,
            ..input("rp")
        })
        .unwrap();
    let past = now - chrono::Duration::hours(1);
    let a = s
        .insert_task(&TaskInput {
            due: Some(Utc.with_ymd_and_hms(2026, 10, 30, 10, 0, 0).unwrap()),
            due_has_time: true,
            alerts: vec![AlertSpecUtc::Before(10), AlertSpecUtc::At(past)],
            ..child("a", p)
        })
        .unwrap();
    let pending = |sql: &str| {
        count(&path, &format!("SELECT count(*) FROM alerts WHERE owner_kind = 'task' AND owner_id = {a} AND fired_at IS NULL {sql}"))
    };
    // Ticking the subtask: the next alert sweep marks all its alerts fired.
    s.set_task_completed(a, true, now).unwrap();
    s.due_alerts(now).unwrap();
    assert_eq!(pending(""), 0);
    // The repeating parent advances: the subtask's future alerts are pending again.
    s.set_task_completed(p, true, now).unwrap();
    assert_eq!(s.get_task(a).unwrap().completed_at, None);
    assert_eq!(
        pending(&format!(
            "AND fire_at_utc > '{}'",
            now.format("%Y-%m-%dT%H:%M:%S")
        )),
        2
    );
    assert_eq!(pending(""), 2, "the past alert stays fired");
    let due: Vec<_> = s
        .due_alerts(Utc.with_ymd_and_hms(2026, 10, 30, 9, 55, 0).unwrap())
        .unwrap();
    assert!(due.iter().any(|x| x.owner_id == a), "{due:?}");
}

// ---- Task 8b: in-progress subtasks ----

fn status_of(s: &Store, id: i64) -> TaskStatus {
    s.get_task(id).unwrap().status
}

#[test]
fn subtask_in_progress_starts_parent() {
    let (_d, _p, s) = store();
    // Via set_task_status.
    let p = s.insert_task(&input("p")).unwrap();
    let a = s.insert_task(&child("a", p)).unwrap();
    s.insert_task(&child("b", p)).unwrap();
    s.set_task_status(a, TaskStatus::InProgress, Utc::now()).unwrap();
    let pt = s.get_task(p).unwrap();
    assert_eq!(pt.status, TaskStatus::InProgress);
    assert_eq!(pt.completed_at, None);
    // Via update_task.
    let q = s.insert_task(&input("q")).unwrap();
    let c = s.insert_task(&child("c", q)).unwrap();
    s.update_task(
        c,
        &TaskInput {
            status: TaskStatus::InProgress,
            ..child("c", q)
        },
    )
    .unwrap();
    assert_eq!(status_of(&s, q), TaskStatus::InProgress);
    assert_eq!(s.get_task(q).unwrap().completed_at, None);
    // A subtask created already in progress starts its parent too.
    let r = s.insert_task(&input("r")).unwrap();
    s.insert_task(&TaskInput {
        status: TaskStatus::InProgress,
        ..child("d", r)
    })
    .unwrap();
    assert_eq!(status_of(&s, r), TaskStatus::InProgress);
}

#[test]
fn parent_already_done_or_in_progress_unchanged() {
    let (_d, _p, s) = store();
    let now = Utc::now();
    let done = s.insert_task(&input("done")).unwrap();
    let a = s.insert_task(&child("a", done)).unwrap();
    s.set_task_completed(done, true, now).unwrap();
    let before = s.get_task(done).unwrap();
    s.set_task_status(a, TaskStatus::InProgress, now).unwrap();
    let after = s.get_task(done).unwrap();
    assert_eq!(after.status, TaskStatus::Done);
    assert_eq!(after.completed_at, before.completed_at);

    let started = s.insert_task(&input("started")).unwrap();
    let b = s.insert_task(&child("b", started)).unwrap();
    s.set_task_status(started, TaskStatus::InProgress, now).unwrap();
    s.set_task_status(b, TaskStatus::InProgress, now).unwrap();
    assert_eq!(status_of(&s, started), TaskStatus::InProgress);

    // Other subtask statuses leave a To do parent alone.
    let idle = s.insert_task(&input("idle")).unwrap();
    let c = s.insert_task(&child("c", idle)).unwrap();
    s.set_task_status(c, TaskStatus::Todo, now).unwrap();
    s.set_task_completed(c, true, now).unwrap();
    assert_eq!(status_of(&s, idle), TaskStatus::Todo);
}

#[test]
fn top_level_task_unaffected() {
    let (_d, _p, s) = store();
    let a = s.insert_task(&input("a")).unwrap();
    let b = s.insert_task(&input("b")).unwrap();
    s.set_task_status(a, TaskStatus::InProgress, Utc::now()).unwrap();
    assert_eq!(status_of(&s, a), TaskStatus::InProgress);
    assert_eq!(status_of(&s, b), TaskStatus::Todo);
}

#[test]
fn subtask_in_progress_count() {
    let (_d, _p, s) = store();
    let now = Utc::now();
    let p = s.insert_task(&input("p")).unwrap();
    let ids: Vec<i64> = (0..4)
        .map(|i| s.insert_task(&child(&format!("c{i}"), p)).unwrap())
        .collect();
    s.set_task_status(ids[0], TaskStatus::InProgress, now).unwrap();
    s.set_task_status(ids[1], TaskStatus::InProgress, now).unwrap();
    s.set_task_completed(ids[2], true, now).unwrap();
    let t = s.get_task(p).unwrap();
    assert_eq!(
        (t.subtask_in_progress, t.subtask_open, t.subtask_total),
        (2, 3, 4)
    );
    let listed = s.list_tasks(TaskFilter::All, now, chrono_tz::Tz::UTC).unwrap();
    let lp = listed.iter().find(|t| t.id == p).unwrap();
    assert_eq!(lp.subtask_in_progress, 2);
    assert_eq!(s.get_task(ids[0]).unwrap().subtask_in_progress, 0);
}
