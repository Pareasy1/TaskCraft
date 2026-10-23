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
            ..Default::default()
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
        "DTSTART;TZID=Europe/Rome:20261019T090000",
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

/// The VTODO block for the task with this id.
fn vtodo(ics: &str, id: i64) -> Vec<String> {
    let uid = format!("UID:task-{id}@taskcraft");
    let mut out = vec![];
    let mut cur: Option<Vec<String>> = None;
    for l in ics.replace("\r\n ", "").split("\r\n") {
        match l {
            "BEGIN:VTODO" => cur = Some(vec![]),
            "END:VTODO" => {
                let block = cur.take().unwrap();
                if block.contains(&uid) {
                    out = block;
                }
            }
            _ => {
                if let Some(b) = cur.as_mut() {
                    b.push(l.to_string());
                }
            }
        }
    }
    out
}

#[test]
fn vtodo_exports_priority_start_and_parent() {
    let s = Store::open_in_memory().unwrap();
    let cal = s.calendars().unwrap()[0].id;
    let base = |title: &str| TaskInput {
        calendar_id: cal,
        title: title.into(),
        tz: "Europe/Rome".into(),
        ..Default::default()
    };
    let urgent = s
        .insert_task(&TaskInput {
            priority: Priority::Urgent,
            due: Some(utc("2026-10-25T16:00:00Z")),
            due_has_time: true,
            start: Some(utc("2026-10-23T07:30:00Z")),
            start_has_time: true,
            ..base("Ship")
        })
        .unwrap();
    let dated = s
        .insert_task(&TaskInput {
            priority: Priority::Low,
            due: Some(utc("2026-10-25T23:00:00Z")),
            start: Some(utc("2026-10-22T22:00:00Z")),
            ..base("Trip prep")
        })
        .unwrap();
    let sub = s
        .insert_task(&TaskInput {
            parent_id: Some(urgent),
            priority: Priority::Important,
            ..base("Test")
        })
        .unwrap();
    let plain = s.insert_task(&base("plain")).unwrap();

    let ics = s.export_ics().unwrap();
    let a = vtodo(&ics, urgent);
    assert!(a.contains(&"PRIORITY:1".to_string()), "{a:?}");
    assert!(a.contains(&"DTSTART:20261023T073000Z".to_string()), "{a:?}");
    assert!(!a.iter().any(|l| l.starts_with("RELATED-TO")), "{a:?}");

    let b = vtodo(&ics, dated);
    assert!(b.contains(&"PRIORITY:9".to_string()), "{b:?}");
    assert!(
        b.contains(&"DTSTART;VALUE=DATE:20261023".to_string()),
        "{b:?}"
    );

    let c = vtodo(&ics, sub);
    assert!(c.contains(&"PRIORITY:3".to_string()), "{c:?}");
    assert!(
        c.contains(&format!(
            "RELATED-TO;RELTYPE=PARENT:task-{urgent}@taskcraft"
        )),
        "{c:?}"
    );

    // A medium task without start or parent: as before, plus PRIORITY:5.
    let d = vtodo(&ics, plain);
    let d: Vec<&str> = d
        .iter()
        .map(String::as_str)
        .filter(|l| !l.starts_with("DTSTAMP"))
        .collect();
    assert_eq!(
        d,
        [
            &format!("UID:task-{plain}@taskcraft")[..],
            "SUMMARY:plain",
            "PRIORITY:5",
            "STATUS:NEEDS-ACTION",
        ]
    );
}
