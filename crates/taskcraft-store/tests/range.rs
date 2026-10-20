use chrono::{DateTime, Utc};
use chrono_tz::Europe::Rome;
use taskcraft_store::*;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn cal(s: &Store, name: &str) -> i64 {
    s.calendars()
        .unwrap()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap()
        .id
}

fn ev(s: &Store, title: &str, start: &str, end: &str, all_day: bool, rrule: Option<&str>) -> i64 {
    s.insert_event(&EventInput {
        calendar_id: cal(s, "Personal"),
        title: title.into(),
        start: utc(start),
        end: utc(end),
        tz: "Europe/Rome".into(),
        all_day,
        location: None,
        attendees: vec![],
        notes: None,
        url: None,
        rrule: rrule.map(str::to_string),
        alerts: vec![],
    })
    .unwrap()
}

fn starts(occ: &[Occurrence]) -> Vec<DateTime<Utc>> {
    occ.iter().map(|o| o.start).collect()
}

#[test]
fn range_includes_overlapping_multiday() {
    let s = Store::open_in_memory().unwrap();
    // Oct 24–26 all-day in Rome
    ev(
        &s,
        "Trip",
        "2026-10-23T22:00:00Z",
        "2026-10-26T23:00:00Z",
        true,
        None,
    );
    let occ = s
        .list_range(utc("2026-10-24T22:00:00Z"), utc("2026-10-25T23:00:00Z"))
        .unwrap();
    assert_eq!(occ.len(), 1);
    assert_eq!(occ[0].title, "Trip");
    assert!(occ[0].all_day);
    // A range that ends exactly when the event starts does not include it.
    assert!(s
        .list_range(utc("2026-10-22T22:00:00Z"), utc("2026-10-23T22:00:00Z"))
        .unwrap()
        .is_empty());
}

#[test]
fn hidden_calendars_are_excluded() {
    let s = Store::open_in_memory().unwrap();
    ev(
        &s,
        "Lunch",
        "2026-10-21T11:00:00Z",
        "2026-10-21T12:00:00Z",
        false,
        None,
    );
    let p = cal(&s, "Personal");
    s.upsert_calendar(&CalendarInput {
        id: Some(p),
        name: "Personal".into(),
        color: "#000".into(),
        visible: false,
    })
    .unwrap();
    assert!(s
        .list_range(utc("2026-10-21T00:00:00Z"), utc("2026-10-22T00:00:00Z"))
        .unwrap()
        .is_empty());
}

#[test]
fn weekly_expansion_keeps_wall_clock_across_dst() {
    let s = Store::open_in_memory().unwrap();
    // Mondays 09:00 Rome from Oct 19 (CEST) — Oct 26 is CET.
    ev(
        &s,
        "Standup",
        "2026-10-19T07:00:00Z",
        "2026-10-19T07:30:00Z",
        false,
        Some("FREQ=WEEKLY;INTERVAL=1;BYDAY=MO"),
    );
    let occ = s
        .list_range(utc("2026-10-18T00:00:00Z"), utc("2026-10-28T00:00:00Z"))
        .unwrap();
    assert_eq!(
        starts(&occ),
        vec![utc("2026-10-19T07:00:00Z"), utc("2026-10-26T08:00:00Z")]
    );
    assert_eq!(occ[1].end, utc("2026-10-26T08:30:00Z"));
    assert!(occ.iter().all(|o| o.recurring));
}

#[test]
fn monthly_31st_skips_short_months() {
    let s = Store::open_in_memory().unwrap();
    // Oct 31 all-day Rome
    ev(
        &s,
        "Bills",
        "2026-10-30T23:00:00Z",
        "2026-10-31T23:00:00Z",
        true,
        Some("FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31"),
    );
    let occ = s
        .list_range(utc("2026-10-01T00:00:00Z"), utc("2027-02-15T00:00:00Z"))
        .unwrap();
    assert_eq!(
        starts(&occ),
        vec![
            utc("2026-10-30T23:00:00Z"),
            utc("2026-12-30T23:00:00Z"),
            utc("2027-01-30T23:00:00Z")
        ]
    );
}

#[test]
fn until_is_respected() {
    let s = Store::open_in_memory().unwrap();
    ev(
        &s,
        "Daily",
        "2026-10-20T08:00:00Z",
        "2026-10-20T08:30:00Z",
        false,
        Some("FREQ=DAILY;INTERVAL=1;UNTIL=20261022"),
    );
    let occ = s
        .list_range(utc("2026-10-19T00:00:00Z"), utc("2026-10-30T00:00:00Z"))
        .unwrap();
    assert_eq!(occ.len(), 3);
}

#[test]
fn move_this_occurrence_detaches() {
    let s = Store::open_in_memory().unwrap();
    let id = ev(
        &s,
        "Daily",
        "2026-10-20T08:00:00Z",
        "2026-10-20T08:30:00Z",
        false,
        Some("FREQ=DAILY;INTERVAL=1"),
    );
    let new_id = s
        .move_event(
            id,
            Some(utc("2026-10-21T08:00:00Z")),
            utc("2026-10-21T10:00:00Z"),
            utc("2026-10-21T10:30:00Z"),
            Scope::This,
        )
        .unwrap();
    assert_ne!(new_id, id);
    let occ = s
        .list_range(utc("2026-10-21T00:00:00Z"), utc("2026-10-22T00:00:00Z"))
        .unwrap();
    assert_eq!(starts(&occ), vec![utc("2026-10-21T10:00:00Z")]);
    assert_eq!(occ[0].event_id, new_id);
    assert!(!occ[0].recurring);
    // Other days unaffected.
    assert_eq!(
        s.list_range(utc("2026-10-22T00:00:00Z"), utc("2026-10-23T00:00:00Z"))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn move_all_shifts_series() {
    let s = Store::open_in_memory().unwrap();
    let id = ev(
        &s,
        "Daily",
        "2026-10-20T08:00:00Z",
        "2026-10-20T08:30:00Z",
        false,
        Some("FREQ=DAILY;INTERVAL=1"),
    );
    let same = s
        .move_event(
            id,
            Some(utc("2026-10-22T08:00:00Z")),
            utc("2026-10-22T09:00:00Z"),
            utc("2026-10-22T10:00:00Z"),
            Scope::All,
        )
        .unwrap();
    assert_eq!(same, id);
    let occ = s
        .list_range(utc("2026-10-20T00:00:00Z"), utc("2026-10-21T00:00:00Z"))
        .unwrap();
    assert_eq!(
        (occ[0].start, occ[0].end),
        (utc("2026-10-20T09:00:00Z"), utc("2026-10-20T10:00:00Z"))
    );
}

#[test]
fn move_plain_event() {
    let s = Store::open_in_memory().unwrap();
    let id = ev(
        &s,
        "Lunch",
        "2026-10-21T11:00:00Z",
        "2026-10-21T12:00:00Z",
        false,
        None,
    );
    s.move_event(
        id,
        None,
        utc("2026-10-22T11:00:00Z"),
        utc("2026-10-22T13:00:00Z"),
        Scope::All,
    )
    .unwrap();
    let e = s.get_event(id).unwrap();
    assert_eq!(
        (e.input.start, e.input.end),
        (utc("2026-10-22T11:00:00Z"), utc("2026-10-22T13:00:00Z"))
    );
}

#[test]
fn delete_this_and_all() {
    let s = Store::open_in_memory().unwrap();
    let id = ev(
        &s,
        "Daily",
        "2026-10-20T08:00:00Z",
        "2026-10-20T08:30:00Z",
        false,
        Some("FREQ=DAILY;INTERVAL=1"),
    );
    s.delete_event(id, Some(utc("2026-10-21T08:00:00Z")), Scope::This)
        .unwrap();
    assert_eq!(
        s.list_range(utc("2026-10-20T00:00:00Z"), utc("2026-10-23T00:00:00Z"))
            .unwrap()
            .len(),
        2
    );
    s.delete_event(id, None, Scope::All).unwrap();
    assert!(s
        .list_range(utc("2026-10-20T00:00:00Z"), utc("2026-10-23T00:00:00Z"))
        .unwrap()
        .is_empty());
    assert!(matches!(s.get_event(id), Err(StoreError::NotFound)));
}

fn task(s: &Store, title: &str, due: Option<&str>, has_time: bool) -> i64 {
    s.insert_task(&TaskInput {
        calendar_id: cal(s, "Personal"),
        title: title.into(),
        due: due.map(utc),
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
fn task_filters() {
    let s = Store::open_in_memory().unwrap();
    let now = utc("2026-10-20T08:00:00Z"); // 10:00 Rome
    let late = task(&s, "late", Some("2026-10-19T22:00:00Z"), false); // Oct 20? no: Oct 20 00:00 Rome → today
    let yesterday = task(&s, "yesterday", Some("2026-10-18T22:00:00Z"), false); // Oct 19 Rome
    let passed = task(&s, "passed", Some("2026-10-20T06:00:00Z"), true); // 08:00 Rome today, passed
    let later = task(&s, "later", Some("2026-10-20T16:00:00Z"), true); // 18:00 Rome today
    let future = task(&s, "future", Some("2026-10-23T22:00:00Z"), false);
    let nodue = task(&s, "nodue", None, false);
    let done = task(&s, "done", Some("2026-10-18T22:00:00Z"), false);
    s.set_task_completed(done, true, now).unwrap();

    let ids = |f| {
        s.list_tasks(f, now, Rome)
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(TaskFilter::Overdue), vec![yesterday, passed]);
    assert_eq!(ids(TaskFilter::Today), vec![late, later]);
    assert_eq!(ids(TaskFilter::Upcoming), vec![future, nodue]);
    assert_eq!(ids(TaskFilter::Completed), vec![done]);
    assert_eq!(ids(TaskFilter::All).len(), 7);
}

#[test]
fn search_matches_location() {
    let s = Store::open_in_memory().unwrap();
    let id = s
        .insert_event(&EventInput {
            calendar_id: cal(&s, "Personal"),
            title: "Coffee".into(),
            start: utc("2026-10-21T08:00:00Z"),
            end: utc("2026-10-21T09:00:00Z"),
            tz: "Europe/Rome".into(),
            all_day: false,
            location: Some("Caffè Sant'Eustachio".into()),
            attendees: vec![],
            notes: None,
            url: None,
            rrule: None,
            alerts: vec![],
        })
        .unwrap();
    let t = task(&s, "buy coffee beans", None, false);
    let hits = s.search("eustachio").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].kind, hits[0].id), (ItemKindDb::Event, id));
    let hits = s.search("COFFEE").unwrap();
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().any(|h| h.kind == ItemKindDb::Task && h.id == t));
}
