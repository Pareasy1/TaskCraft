use chrono::{DateTime, Utc};
use taskcraft_store::*;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

#[test]
fn ics_contains_expected_lines() {
    let s = Store::open_in_memory().unwrap();
    let cal = s.calendars().unwrap()[0].id;
    s.insert_event(&EventInput {
        calendar_id: cal,
        title: "Standup, daily; with notes".into(),
        start: utc("2026-10-19T07:00:00Z"),
        end: utc("2026-10-19T07:30:00Z"),
        tz: "Europe/Rome".into(),
        all_day: false,
        location: Some("Room 1".into()),
        attendees: vec![],
        notes: Some("A very long description that goes on and on so that the folded line exceeds seventy-five octets for sure".into()),
        url: None,
        rrule: Some("FREQ=WEEKLY;INTERVAL=1;BYDAY=MO".into()),
        alerts: vec![AlertSpecUtc::Before(10)],
    })
    .unwrap();
    s.insert_event(&EventInput {
        calendar_id: cal,
        title: "Trip".into(),
        start: utc("2026-10-23T22:00:00Z"),
        end: utc("2026-10-26T23:00:00Z"),
        tz: "Europe/Rome".into(),
        all_day: true,
        location: None,
        attendees: vec![],
        notes: None,
        url: None,
        rrule: None,
        alerts: vec![],
    })
    .unwrap();
    let t = s
        .insert_task(&TaskInput {
            calendar_id: cal,
            title: "buy milk".into(),
            due: Some(utc("2026-10-23T22:00:00Z")),
            due_has_time: false,
            tz: "Europe/Rome".into(),
            rrule: None,
            notes: None,
            alerts: vec![],
        })
        .unwrap();
    s.set_task_completed(t, true, utc("2026-10-20T10:00:00Z"))
        .unwrap();

    let ics = s.export_ics().unwrap();
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
    assert!(ics.trim_end().ends_with("END:VCALENDAR"));
    for needle in [
        "BEGIN:VEVENT",
        "RRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO",
        "SUMMARY:Standup\\, daily\\; with notes",
        "DTSTART:20261019T070000Z",
        "DTSTART;VALUE=DATE:20261024",
        "DTEND;VALUE=DATE:20261027",
        "BEGIN:VALARM",
        "TRIGGER:-PT10M",
        "BEGIN:VTODO",
        "DUE;VALUE=DATE:20261024",
        "STATUS:COMPLETED",
    ] {
        assert!(ics.contains(needle), "missing {needle}\n{ics}");
    }
    for line in ics.split("\r\n") {
        assert!(line.len() <= 75, "line too long: {line}");
    }
    assert!(ics.contains("\r\n "), "expected at least one folded line");
}
