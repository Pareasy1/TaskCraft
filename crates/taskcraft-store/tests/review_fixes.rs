//! Regression tests for the final review's findings.
use chrono::{DateTime, Duration, Utc};
use chrono_tz::Europe::Rome;
use taskcraft_store::*;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn ev(
    s: &Store,
    start: &str,
    end: &str,
    all_day: bool,
    rrule: &str,
    alerts: Vec<AlertSpecUtc>,
) -> i64 {
    s.insert_event(&EventInput {
        calendar_id: 1,
        title: "Series".into(),
        start: utc(start),
        end: utc(end),
        tz: "Europe/Rome".into(),
        all_day,
        location: None,
        attendees: vec![],
        notes: None,
        url: None,
        rrule: Some(rrule.into()),
        alerts,
    })
    .unwrap()
}

fn starts(s: &Store, from: &str, to: &str) -> Vec<DateTime<Utc>> {
    s.list_range(utc(from), utc(to))
        .unwrap()
        .into_iter()
        .map(|o| o.start)
        .collect()
}

// Finding 1
#[test]
fn delete_this_without_occurrence_is_rejected() {
    let s = Store::open_in_memory().unwrap();
    let id = ev(
        &s,
        "2026-10-20T08:00:00Z",
        "2026-10-20T09:00:00Z",
        false,
        "FREQ=DAILY;INTERVAL=1",
        vec![],
    );
    assert!(matches!(
        s.delete_event(id, None, Scope::This),
        Err(StoreError::Invalid(_))
    ));
    assert!(s.get_event(id).is_ok(), "series must survive");
}

// Finding 2
#[test]
fn move_all_keeps_exceptions_excluded() {
    let s = Store::open_in_memory().unwrap();
    // Weekly Tue 10:00 Rome = 08:00Z (CEST).
    let id = ev(
        &s,
        "2026-10-20T08:00:00Z",
        "2026-10-20T09:00:00Z",
        false,
        "FREQ=WEEKLY;INTERVAL=1",
        vec![],
    );
    s.delete_event(id, Some(utc("2026-10-20T08:00:00Z")), Scope::This)
        .unwrap();
    // Move the series 30 min later via the Oct 27 occurrence (09:00Z, CET).
    s.move_event(
        id,
        Some(utc("2026-10-27T09:00:00Z")),
        utc("2026-10-27T09:30:00Z"),
        utc("2026-10-27T10:30:00Z"),
        Scope::All,
    )
    .unwrap();
    assert!(
        starts(&s, "2026-10-19T00:00:00Z", "2026-10-21T00:00:00Z").is_empty(),
        "deleted occurrence came back"
    );
    assert_eq!(
        starts(&s, "2026-10-26T00:00:00Z", "2026-10-28T00:00:00Z"),
        vec![utc("2026-10-27T09:30:00Z")]
    );
}

// Finding 3
#[test]
fn excluding_an_occurrence_rearms_its_alert() {
    let s = Store::open_in_memory().unwrap();
    let id = ev(
        &s,
        "2026-10-20T08:00:00Z",
        "2026-10-20T09:00:00Z",
        false,
        "FREQ=DAILY;INTERVAL=1",
        vec![AlertSpecUtc::Before(10)],
    );
    s.delete_event(id, Some(utc("2026-10-20T08:00:00Z")), Scope::This)
        .unwrap();
    assert!(
        s.due_alerts(utc("2026-10-20T07:55:00Z"))
            .unwrap()
            .is_empty(),
        "alert for a deleted occurrence"
    );
    let due = s.due_alerts(utc("2026-10-21T07:50:00Z")).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].when, utc("2026-10-21T08:00:00Z"));
}

#[test]
fn moving_one_occurrence_does_not_fire_twice() {
    let s = Store::open_in_memory().unwrap();
    let id = ev(
        &s,
        "2026-10-20T08:00:00Z",
        "2026-10-20T09:00:00Z",
        false,
        "FREQ=DAILY;INTERVAL=1",
        vec![AlertSpecUtc::Before(10)],
    );
    s.move_event(
        id,
        Some(utc("2026-10-20T08:00:00Z")),
        utc("2026-10-20T12:00:00Z"),
        utc("2026-10-20T13:00:00Z"),
        Scope::This,
    )
    .unwrap();
    assert!(s
        .due_alerts(utc("2026-10-20T07:55:00Z"))
        .unwrap()
        .is_empty());
    let due = s.due_alerts(utc("2026-10-20T11:50:00Z")).unwrap();
    assert_eq!(due.len(), 1, "only the moved copy's alert");
    assert_eq!(due[0].when, utc("2026-10-20T12:00:00Z"));
}

// Finding 4
#[test]
fn recurring_all_day_stays_one_day_across_dst() {
    let s = Store::open_in_memory().unwrap();
    // Sunday 2026-10-25 is 25 hours long in Rome: 22:00Z Oct 24 → 23:00Z Oct 25.
    ev(
        &s,
        "2026-10-24T22:00:00Z",
        "2026-10-25T23:00:00Z",
        true,
        "FREQ=WEEKLY;INTERVAL=1",
        vec![],
    );
    let occ = s
        .list_range(utc("2026-10-31T00:00:00Z"), utc("2026-11-03T00:00:00Z"))
        .unwrap();
    assert_eq!(occ.len(), 1);
    // Nov 1 local midnight → Nov 2 local midnight (CET).
    assert_eq!(
        (occ[0].start, occ[0].end),
        (utc("2026-10-31T23:00:00Z"), utc("2026-11-01T23:00:00Z"))
    );
    // And across the spring change (2027-03-28 is 23 hours long).
    let occ = s
        .list_range(utc("2027-03-27T00:00:00Z"), utc("2027-03-30T00:00:00Z"))
        .unwrap();
    assert_eq!(
        (occ[0].start, occ[0].end),
        (utc("2027-03-27T23:00:00Z"), utc("2027-03-28T22:00:00Z"))
    );
}

// Finding 7
#[test]
fn completing_recurring_task_advances_due() {
    let s = Store::open_in_memory().unwrap();
    let id = s
        .insert_task(&TaskInput {
            calendar_id: 1,
            title: "trash".into(),
            due: Some(utc("2026-10-20T18:00:00Z")),
            due_has_time: true,
            tz: "Europe/Rome".into(),
            rrule: Some("FREQ=WEEKLY;INTERVAL=1".into()),
            notes: None,
            alerts: vec![],
        })
        .unwrap();
    s.set_task_completed(id, true, utc("2026-10-20T19:00:00Z"))
        .unwrap();
    let t = s.get_task(id).unwrap();
    assert_eq!(t.completed_at, None, "series continues");
    assert_eq!(
        t.due,
        Some(utc("2026-10-27T19:00:00Z")),
        "next Tuesday 20:00 Rome (CET)"
    );
    assert!(s
        .due_alerts(utc("2026-10-21T00:00:00Z"))
        .unwrap()
        .is_empty());
    assert_eq!(s.due_alerts(utc("2026-10-27T19:00:00Z")).unwrap().len(), 1);
    // A series that has ended completes normally.
    s.update_task(
        id,
        &TaskInput {
            rrule: Some("FREQ=WEEKLY;INTERVAL=1;UNTIL=20261028".into()),
            ..task_input(&s, id)
        },
    )
    .unwrap();
    s.set_task_completed(id, true, utc("2026-10-27T20:00:00Z"))
        .unwrap();
    assert!(s.get_task(id).unwrap().completed_at.is_some());
}

fn task_input(s: &Store, id: i64) -> TaskInput {
    let t = s.get_task(id).unwrap();
    TaskInput {
        calendar_id: t.calendar_id,
        title: t.title,
        due: t.due,
        due_has_time: t.due_has_time,
        tz: t.tz,
        rrule: t.rrule,
        notes: t.notes,
        alerts: t.alerts,
    }
}

// Finding 8
#[test]
fn ics_recurring_uses_local_time_zone() {
    let s = Store::open_in_memory().unwrap();
    let id = ev(
        &s,
        "2026-10-20T08:00:00Z",
        "2026-10-20T09:00:00Z",
        false,
        "FREQ=WEEKLY;INTERVAL=1;UNTIL=20261124",
        vec![],
    );
    s.delete_event(id, Some(utc("2026-10-27T09:00:00Z")), Scope::This)
        .unwrap();
    ev(
        &s,
        "2026-10-24T22:00:00Z",
        "2026-10-25T23:00:00Z",
        true,
        "FREQ=WEEKLY;INTERVAL=1",
        vec![],
    );
    let all_day_id = 2;
    s.delete_event(all_day_id, Some(utc("2026-10-31T23:00:00Z")), Scope::This)
        .unwrap();
    let ics = s.export_ics().unwrap().replace("\r\n ", "");
    for needle in [
        "DTSTART;TZID=Europe/Rome:20261020T100000",
        "DTEND;TZID=Europe/Rome:20261020T110000",
        "UNTIL=20261124T225959Z",
        "EXDATE;TZID=Europe/Rome:20261027T100000",
        "DTSTART;VALUE=DATE:20261025",
        "EXDATE;VALUE=DATE:20261101",
    ] {
        assert!(ics.contains(needle), "missing {needle}\n{ics}");
    }
    let _ = (Rome, Duration::zero());
}
