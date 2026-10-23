use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Rome;
use taskcraft_parse::{parse, ParseOptions, When};
use taskcraft_store::*;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn personal(store: &Store) -> i64 {
    store
        .calendars()
        .unwrap()
        .iter()
        .find(|c| c.name == "Personal")
        .unwrap()
        .id
}

fn work(store: &Store) -> i64 {
    store
        .calendars()
        .unwrap()
        .iter()
        .find(|c| c.name == "Work")
        .unwrap()
        .id
}

fn sample_event(cal: i64) -> EventInput {
    EventInput {
        calendar_id: cal,
        title: "Lunch".into(),
        start: utc("2026-10-21T11:00:00Z"),
        end: utc("2026-10-21T12:00:00Z"),
        tz: "Europe/Rome".into(),
        all_day: false,
        location: Some("Trattoria".into()),
        attendees: vec!["Sam".into(), "Anna".into()],
        notes: Some("bring cake".into()),
        url: None,
        rrule: None,
        alerts: vec![AlertSpecUtc::Before(10)],
    }
}

#[test]
fn open_seeds_calendars() {
    let s = Store::open_in_memory().unwrap();
    let names: Vec<String> = s.calendars().unwrap().into_iter().map(|c| c.name).collect();
    assert_eq!(names, ["Personal", "Work"]);
    assert_eq!(s.schema_version().unwrap(), 4);
}

#[test]
fn migration_backs_up() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taskcraft.db");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE junk(x); INSERT INTO junk VALUES (1);")
        .unwrap();
    let s = Store::open(&path).unwrap();
    assert_eq!(s.schema_version().unwrap(), 4);
    assert!(dir.path().join("taskcraft.db.bak-v0").exists());
}

#[test]
fn reopen_does_not_reseed_or_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taskcraft.db");
    drop(Store::open(&path).unwrap());
    let s = Store::open(&path).unwrap();
    assert_eq!(s.calendars().unwrap().len(), 2);
    assert!(!dir.path().join("taskcraft.db.bak-v2").exists());
}

#[test]
fn event_roundtrip() {
    let s = Store::open_in_memory().unwrap();
    let input = sample_event(personal(&s));
    let id = s.insert_event(&input).unwrap();
    let e = s.get_event(id).unwrap();
    assert_eq!(e.id, id);
    assert_eq!(e.input, input);
}

#[test]
fn update_event_replaces_fields_and_alerts() {
    let s = Store::open_in_memory().unwrap();
    let id = s.insert_event(&sample_event(personal(&s))).unwrap();
    let mut changed = sample_event(work(&s));
    changed.title = "Late lunch".into();
    changed.alerts = vec![AlertSpecUtc::Before(30), AlertSpecUtc::Before(5)];
    s.update_event(id, &changed).unwrap();
    assert_eq!(s.get_event(id).unwrap().input, changed);
}

#[test]
fn get_missing_event_is_not_found() {
    let s = Store::open_in_memory().unwrap();
    assert!(matches!(s.get_event(999), Err(StoreError::NotFound)));
}

#[test]
fn task_complete_toggle() {
    let s = Store::open_in_memory().unwrap();
    let id = s
        .insert_task(&TaskInput {
            calendar_id: personal(&s),
            title: "buy milk".into(),
            due: None,
            due_has_time: false,
            tz: "Europe/Rome".into(),
            rrule: None,
            notes: None,
            alerts: vec![],
            ..Default::default()
        })
        .unwrap();
    let now = utc("2026-10-20T08:00:00Z");
    s.set_task_completed(id, true, now).unwrap();
    assert_eq!(s.get_task(id).unwrap().completed_at, Some(now));
    s.set_task_completed(id, false, now).unwrap();
    assert_eq!(s.get_task(id).unwrap().completed_at, None);
    s.delete_task(id).unwrap();
    assert!(matches!(s.get_task(id), Err(StoreError::NotFound)));
}

#[test]
fn delete_calendar_moves_items() {
    let s = Store::open_in_memory().unwrap();
    let (p, w) = (personal(&s), work(&s));
    let id = s.insert_event(&sample_event(w)).unwrap();
    s.delete_calendar(w, p).unwrap();
    assert_eq!(s.get_event(id).unwrap().input.calendar_id, p);
    assert_eq!(s.calendars().unwrap().len(), 1);
}

#[test]
fn upsert_calendar_creates_and_renames() {
    let s = Store::open_in_memory().unwrap();
    let id = s
        .upsert_calendar(&CalendarInput {
            id: None,
            name: "Gym".into(),
            color: "#22c55e".into(),
            visible: true,
        })
        .unwrap();
    s.upsert_calendar(&CalendarInput {
        id: Some(id),
        name: "Fitness".into(),
        color: "#22c55e".into(),
        visible: false,
    })
    .unwrap();
    let c = s
        .calendars()
        .unwrap()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap();
    assert_eq!((c.name.as_str(), c.visible), ("Fitness", false));
}

#[test]
fn to_utc_handles_dst() {
    let w = |y, m, d, h, mi| When {
        date: NaiveDate::from_ymd_opt(y, m, d).unwrap(),
        time: NaiveTime::from_hms_opt(h, mi, 0),
    };
    assert_eq!(
        to_utc(&w(2026, 10, 25, 2, 30), Rome),
        utc("2026-10-25T00:30:00Z")
    );
    assert_eq!(
        to_utc(&w(2027, 3, 28, 2, 30), Rome),
        utc("2027-03-28T01:30:00Z")
    );
}

#[test]
fn from_parsed_all_day_range() {
    let s = Store::open_in_memory().unwrap();
    let now = Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
    let p = parse("Trip Oct 24 to Oct 26", now, &ParseOptions::default());
    let cals = s.calendars().unwrap();
    let NewItem::Event(e) = from_parsed(&p, Rome, personal(&s), &cals, 60, &[]) else {
        panic!("expected event")
    };
    assert_eq!(e.start, utc("2026-10-23T22:00:00Z"));
    assert_eq!(e.end, utc("2026-10-26T23:00:00Z"));
    assert!(e.all_day);
}

#[test]
fn from_parsed_task_and_calendar() {
    let s = Store::open_in_memory().unwrap();
    let now = Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
    let p = parse("todo call mom at 6pm /work", now, &ParseOptions::default());
    let cals = s.calendars().unwrap();
    let NewItem::Task(t) = from_parsed(&p, Rome, personal(&s), &cals, 60, &[]) else {
        panic!("expected task")
    };
    assert_eq!(t.calendar_id, work(&s));
    assert_eq!(t.due, Some(utc("2026-10-20T16:00:00Z")));
    assert!(t.due_has_time);
    assert_eq!(t.title, "call mom");
}

#[test]
fn from_parsed_copies_tags() {
    let s = Store::open_in_memory().unwrap();
    let cals = s.calendars().unwrap();
    let now = Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
    let p = parse("todo: pay rent #bills", now, &ParseOptions::default());
    let NewItem::Task(t) = from_parsed(&p, Rome, personal(&s), &cals, 60, &[]) else {
        panic!("expected a task")
    };
    assert_eq!(t.labels, ["bills"]);
}
