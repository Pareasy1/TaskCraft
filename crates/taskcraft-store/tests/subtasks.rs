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
