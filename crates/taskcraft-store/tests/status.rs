use chrono::{DateTime, Utc};
use taskcraft_store::*;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn input(title: &str) -> TaskInput {
    TaskInput {
        calendar_id: 1,
        title: title.into(),
        tz: "Europe/Rome".into(),
        ..Default::default()
    }
}

const NOW: &str = "2026-10-20T10:00:00Z";

#[test]
fn in_progress_round_trip() {
    let s = Store::open_in_memory().unwrap();
    let id = s.insert_task(&input("write report")).unwrap();
    s.set_task_status(id, TaskStatus::InProgress, utc(NOW))
        .unwrap();
    let t = s.get_task(id).unwrap();
    assert_eq!(t.status, TaskStatus::InProgress);
    assert_eq!(t.completed_at, None);
}

#[test]
fn done_sets_completed_at_and_back() {
    let s = Store::open_in_memory().unwrap();
    let id = s.insert_task(&input("a")).unwrap();
    s.set_task_status(id, TaskStatus::Done, utc(NOW)).unwrap();
    let t = s.get_task(id).unwrap();
    assert_eq!(t.status, TaskStatus::Done);
    assert_eq!(t.completed_at, Some(utc(NOW)));
    s.set_task_status(id, TaskStatus::Todo, utc(NOW)).unwrap();
    let t = s.get_task(id).unwrap();
    assert_eq!(t.status, TaskStatus::Todo);
    assert_eq!(t.completed_at, None);
}

#[test]
fn checkbox_keeps_invariant() {
    let s = Store::open_in_memory().unwrap();
    let id = s.insert_task(&input("a")).unwrap();
    s.set_task_status(id, TaskStatus::InProgress, utc(NOW))
        .unwrap();
    s.set_task_completed(id, true, utc(NOW)).unwrap();
    assert_eq!(s.get_task(id).unwrap().status, TaskStatus::Done);
    s.set_task_completed(id, false, utc(NOW)).unwrap();
    assert_eq!(s.get_task(id).unwrap().status, TaskStatus::Todo);
}

#[test]
fn recurring_done_advances_and_resets() {
    let s = Store::open_in_memory().unwrap();
    let id = s
        .insert_task(&TaskInput {
            due: Some(utc("2026-10-20T07:00:00Z")), // 09:00 Rome (CEST)
            due_has_time: true,
            rrule: Some("FREQ=WEEKLY;INTERVAL=1".into()),
            status: TaskStatus::InProgress,
            ..input("standup notes")
        })
        .unwrap();
    s.set_task_status(id, TaskStatus::Done, utc(NOW)).unwrap();
    let t = s.get_task(id).unwrap();
    assert_eq!(t.due, Some(utc("2026-10-27T08:00:00Z")), "09:00 Rome (CET)");
    assert_eq!(t.completed_at, None);
    assert_eq!(t.status, TaskStatus::Todo);
}

#[test]
fn insert_with_done_status_sets_completed_at() {
    let s = Store::open_in_memory().unwrap();
    let id = s
        .insert_task(&TaskInput {
            status: TaskStatus::Done,
            ..input("a")
        })
        .unwrap();
    let t = s.get_task(id).unwrap();
    assert!(t.completed_at.is_some());
    // Updating with the same status keeps the original completion time.
    let first = t.completed_at;
    s.update_task(
        id,
        &TaskInput {
            status: TaskStatus::Done,
            ..input("b")
        },
    )
    .unwrap();
    assert_eq!(s.get_task(id).unwrap().completed_at, first);
    s.update_task(
        id,
        &TaskInput {
            status: TaskStatus::InProgress,
            ..input("b")
        },
    )
    .unwrap();
    assert_eq!(s.get_task(id).unwrap().completed_at, None);
}

#[test]
fn update_to_done_advances_recurring_task() {
    let s = Store::open_in_memory().unwrap();
    let base = TaskInput {
        due: Some(utc("2026-10-20T07:00:00Z")),
        due_has_time: true,
        rrule: Some("FREQ=WEEKLY;INTERVAL=1".into()),
        ..input("standup notes")
    };
    let id = s.insert_task(&base).unwrap();
    s.update_task(
        id,
        &TaskInput {
            status: TaskStatus::Done,
            notes: Some("x".into()),
            ..base.clone()
        },
    )
    .unwrap();
    let t = s.get_task(id).unwrap();
    assert_eq!(
        t.due,
        Some(utc("2026-10-27T08:00:00Z")),
        "series continues like the checkbox"
    );
    assert_eq!(t.status, TaskStatus::Todo);
    assert_eq!(t.completed_at, None);
    assert_eq!(t.notes.as_deref(), Some("x"), "other edits are kept");
}
