use chrono::{DateTime, Utc};
use taskcraft_store::*;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn personal(s: &Store) -> i64 {
    s.calendars().unwrap()[0].id
}

fn event(
    s: &Store,
    title: &str,
    start: &str,
    rrule: Option<&str>,
    alerts: Vec<AlertSpecUtc>,
) -> i64 {
    let start = utc(start);
    s.insert_event(&EventInput {
        calendar_id: personal(s),
        title: title.into(),
        start,
        end: start + chrono::Duration::hours(1),
        tz: "Europe/Rome".into(),
        all_day: false,
        location: Some("Office".into()),
        attendees: vec![],
        notes: None,
        url: None,
        rrule: rrule.map(str::to_string),
        alerts,
    })
    .unwrap()
}

#[test]
fn before_offset_computes_fire_at() {
    let s = Store::open_in_memory().unwrap();
    let id = event(
        &s,
        "Call",
        "2026-10-20T15:00:00Z",
        None,
        vec![AlertSpecUtc::Before(10)],
    );
    assert!(s
        .due_alerts(utc("2026-10-20T14:49:59Z"))
        .unwrap()
        .is_empty());
    let due = s.due_alerts(utc("2026-10-20T14:50:00Z")).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!((due[0].owner, due[0].owner_id), (ItemKindDb::Event, id));
    assert_eq!(due[0].title, "Call");
    assert_eq!(due[0].when, utc("2026-10-20T15:00:00Z"));
    assert_eq!(due[0].location.as_deref(), Some("Office"));
}

#[test]
fn stale_alerts_skipped_recent_fire_once() {
    let s = Store::open_in_memory().unwrap();
    let now = utc("2026-10-20T20:00:00Z");
    event(
        &s,
        "Old",
        "2026-10-20T07:00:00Z",
        None,
        vec![AlertSpecUtc::At(utc("2026-10-20T07:00:00Z"))],
    ); // now-13h
    event(
        &s,
        "Recent",
        "2026-10-20T19:00:00Z",
        None,
        vec![AlertSpecUtc::At(utc("2026-10-20T19:00:00Z"))],
    ); // now-1h
    let due = s.due_alerts(now).unwrap();
    assert_eq!(
        due.iter().map(|d| d.title.as_str()).collect::<Vec<_>>(),
        ["Recent"]
    );
    s.mark_fired(due[0].alert_id, now).unwrap();
    assert!(s.due_alerts(now).unwrap().is_empty());
    assert!(s
        .due_alerts(now + chrono::Duration::hours(1))
        .unwrap()
        .is_empty());
}

#[test]
fn recurring_alert_rematerializes() {
    let s = Store::open_in_memory().unwrap();
    // Daily 09:00 Rome (07:00Z in CEST), alert 5 min before.
    event(
        &s,
        "Standup",
        "2026-10-20T07:00:00Z",
        Some("FREQ=DAILY;INTERVAL=1"),
        vec![AlertSpecUtc::Before(5)],
    );
    let day1 = utc("2026-10-20T06:55:00Z");
    let due = s.due_alerts(day1).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].when, utc("2026-10-20T07:00:00Z"));
    s.mark_fired(due[0].alert_id, day1).unwrap();
    assert!(s
        .due_alerts(utc("2026-10-21T06:54:59Z"))
        .unwrap()
        .is_empty());
    let due = s.due_alerts(utc("2026-10-21T06:55:00Z")).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].when, utc("2026-10-21T07:00:00Z"));
}

#[test]
fn recurring_stale_alert_catches_up_to_next_occurrence() {
    let s = Store::open_in_memory().unwrap();
    event(
        &s,
        "Standup",
        "2026-10-01T07:00:00Z",
        Some("FREQ=DAILY;INTERVAL=1"),
        vec![AlertSpecUtc::Before(5)],
    );
    // Weeks later: the old alert is stale, nothing fires now…
    let now = utc("2026-10-20T12:00:00Z");
    assert!(s.due_alerts(now).unwrap().is_empty());
    // …but the next morning's alert does.
    let due = s.due_alerts(utc("2026-10-21T06:55:00Z")).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].when, utc("2026-10-21T07:00:00Z"));
}

fn task(s: &Store, title: &str, due: &str, has_time: bool) -> i64 {
    s.insert_task(&TaskInput {
        calendar_id: personal(s),
        title: title.into(),
        due: Some(utc(due)),
        due_has_time: has_time,
        tz: "Europe/Rome".into(),
        rrule: None,
        notes: None,
        alerts: vec![],
        ..Default::default()
    })
    .unwrap()
}

#[test]
fn task_due_time_implicit_alert() {
    let s = Store::open_in_memory().unwrap();
    let id = task(&s, "call mom", "2026-10-20T16:00:00Z", true);
    let done = task(&s, "already done", "2026-10-20T16:00:00Z", true);
    task(&s, "no time", "2026-10-19T22:00:00Z", false);
    s.set_task_completed(done, true, utc("2026-10-20T10:00:00Z"))
        .unwrap();
    assert_eq!(s.get_task(id).unwrap().alerts, vec![]);
    let due = s.due_alerts(utc("2026-10-20T16:00:30Z")).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!((due[0].owner, due[0].owner_id), (ItemKindDb::Task, id));
}

#[test]
fn deleting_event_removes_its_alerts() {
    let s = Store::open_in_memory().unwrap();
    let id = event(
        &s,
        "Call",
        "2026-10-20T15:00:00Z",
        None,
        vec![AlertSpecUtc::Before(10)],
    );
    s.delete_event(id, None, Scope::All).unwrap();
    assert!(s
        .due_alerts(utc("2026-10-20T15:00:00Z"))
        .unwrap()
        .is_empty());
}

#[test]
fn editing_event_does_not_refire_alert() {
    let s = Store::open_in_memory().unwrap();
    let id = event(
        &s,
        "Call",
        "2026-10-20T15:00:00Z",
        None,
        vec![AlertSpecUtc::Before(10)],
    );
    let now = utc("2026-10-20T14:50:00Z");
    let due = s.due_alerts(now).unwrap();
    s.mark_fired(due[0].alert_id, now).unwrap();
    let mut e = s.get_event(id).unwrap().input;
    assert_eq!(
        e.alerts,
        vec![AlertSpecUtc::Before(10)],
        "fired alerts still belong to the event"
    );
    e.title = "Call (renamed)".into();
    s.update_event(id, &e).unwrap();
    assert!(s
        .due_alerts(utc("2026-10-20T14:55:00Z"))
        .unwrap()
        .is_empty());
}

#[test]
fn editing_recurring_event_keeps_next_alert() {
    let s = Store::open_in_memory().unwrap();
    let id = event(
        &s,
        "Standup",
        "2026-10-20T07:00:00Z",
        Some("FREQ=DAILY;INTERVAL=1"),
        vec![AlertSpecUtc::Before(5)],
    );
    let day1 = utc("2026-10-20T06:55:00Z");
    let due = s.due_alerts(day1).unwrap();
    s.mark_fired(due[0].alert_id, day1).unwrap();
    let mut e = s.get_event(id).unwrap().input;
    assert_eq!(e.alerts, vec![AlertSpecUtc::Before(5)]);
    e.title = "Daily standup".into();
    s.update_event(id, &e).unwrap();
    assert!(s
        .due_alerts(utc("2026-10-20T08:00:00Z"))
        .unwrap()
        .is_empty());
    let due = s.due_alerts(utc("2026-10-21T06:55:00Z")).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].when, utc("2026-10-21T07:00:00Z"));
}
