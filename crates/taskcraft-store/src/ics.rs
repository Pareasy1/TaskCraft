//! iCalendar (RFC 5545) export of all events and tasks.

use crate::events::{event_from_row, EVENT_COLS};
use crate::model::*;
use crate::tasks::{task_from_row, TASK_COLS};
use crate::{recur, Store};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn utc_stamp(t: DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

fn local_stamp(t: DateTime<Utc>, tz: Tz) -> String {
    t.with_timezone(&tz).format("%Y%m%dT%H%M%S").to_string()
}

fn local_date(t: DateTime<Utc>, tz: Tz) -> String {
    t.with_timezone(&tz).format("%Y%m%d").to_string()
}

/// RFC 5545 PRIORITY: 1 is the highest, 9 the lowest (0 would mean undefined).
fn ical_priority(p: Priority) -> u8 {
    match p {
        Priority::Urgent => 1,
        Priority::Important => 3,
        Priority::Medium => 5,
        Priority::Low => 9,
    }
}

/// Folds a content line at 75 octets without splitting UTF-8 characters.
fn fold(line: &str, out: &mut String) {
    let mut width = 0;
    for c in line.chars() {
        let n = c.len_utf8();
        if width + n > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += n;
    }
    out.push_str("\r\n");
}

impl Store {
    pub fn export_ics(&self) -> Result<String> {
        let mut lines: Vec<String> = vec![
            "BEGIN:VCALENDAR".into(),
            "VERSION:2.0".into(),
            "PRODID:-//TaskCraft//EN".into(),
            "CALSCALE:GREGORIAN".into(),
        ];
        let stamp = utc_stamp(Utc::now());

        let mut st = self.conn.prepare(&format!(
            "SELECT {EVENT_COLS} FROM events ORDER BY start_utc"
        ))?;
        let events: Vec<(i64, EventInput)> = st
            .query_map([], event_from_row)?
            .collect::<std::result::Result<_, _>>()?;
        for (id, mut e) in events {
            e.alerts = self.read_alerts(ItemKindDb::Event, id)?;
            let tz: Tz = e.tz.parse().unwrap_or(Tz::UTC);
            lines.push("BEGIN:VEVENT".into());
            lines.push(format!("UID:event-{id}@taskcraft"));
            lines.push(format!("DTSTAMP:{stamp}"));
            // Local wall-clock times with TZID, so recurrences keep their
            // time of day across DST in the importing app.
            if e.all_day {
                lines.push(format!("DTSTART;VALUE=DATE:{}", local_date(e.start, tz)));
                lines.push(format!("DTEND;VALUE=DATE:{}", local_date(e.end, tz)));
            } else {
                lines.push(format!(
                    "DTSTART;TZID={}:{}",
                    tz.name(),
                    local_stamp(e.start, tz)
                ));
                lines.push(format!(
                    "DTEND;TZID={}:{}",
                    tz.name(),
                    local_stamp(e.end, tz)
                ));
            }
            lines.push(format!("SUMMARY:{}", esc(&e.title)));
            if let Some(l) = &e.location {
                lines.push(format!("LOCATION:{}", esc(l)));
            }
            if let Some(n) = &e.notes {
                lines.push(format!("DESCRIPTION:{}", esc(n)));
            }
            if let Some(u) = &e.url {
                lines.push(format!("URL:{u}"));
            }
            if let Some(r) = &e.rrule {
                let r = recur::split(r);
                lines.push(format!("RRULE:{}", recur::normalize_until(&r.rule, tz)));
                if !r.exdates.is_empty() {
                    if e.all_day {
                        let ex: Vec<String> =
                            r.exdates.iter().map(|d| local_date(*d, tz)).collect();
                        lines.push(format!("EXDATE;VALUE=DATE:{}", ex.join(",")));
                    } else {
                        let ex: Vec<String> =
                            r.exdates.iter().map(|d| local_stamp(*d, tz)).collect();
                        lines.push(format!("EXDATE;TZID={}:{}", tz.name(), ex.join(",")));
                    }
                }
            }
            for a in &e.alerts {
                lines.push("BEGIN:VALARM".into());
                lines.push("ACTION:DISPLAY".into());
                lines.push(format!("DESCRIPTION:{}", esc(&e.title)));
                match a {
                    AlertSpecUtc::Before(m) => lines.push(format!("TRIGGER:-PT{m}M")),
                    AlertSpecUtc::At(t) => {
                        lines.push(format!("TRIGGER;VALUE=DATE-TIME:{}", utc_stamp(*t)))
                    }
                }
                lines.push("END:VALARM".into());
            }
            lines.push("END:VEVENT".into());
        }

        let mut st = self.conn.prepare(&format!(
            "SELECT {TASK_COLS} FROM tasks t JOIN calendars c ON c.id = t.calendar_id ORDER BY t.id"
        ))?;
        let tasks: Vec<Task> = st
            .query_map([], task_from_row)?
            .collect::<std::result::Result<_, _>>()?;
        for t in tasks {
            let tz: Tz = t.tz.parse().unwrap_or(Tz::UTC);
            lines.push("BEGIN:VTODO".into());
            lines.push(format!("UID:task-{}@taskcraft", t.id));
            lines.push(format!("DTSTAMP:{stamp}"));
            lines.push(format!("SUMMARY:{}", esc(&t.title)));
            if let Some(d) = t.start {
                if t.start_has_time {
                    lines.push(format!("DTSTART:{}", utc_stamp(d)));
                } else {
                    lines.push(format!("DTSTART;VALUE=DATE:{}", local_date(d, tz)));
                }
            }
            if let Some(d) = t.due {
                if t.due_has_time {
                    lines.push(format!("DUE:{}", utc_stamp(d)));
                } else {
                    lines.push(format!("DUE;VALUE=DATE:{}", local_date(d, tz)));
                }
            }
            if let Some(n) = &t.notes {
                lines.push(format!("DESCRIPTION:{}", esc(n)));
            }
            if let Some(r) = &t.rrule {
                let tz: Tz = t.tz.parse().unwrap_or(Tz::UTC);
                lines.push(format!(
                    "RRULE:{}",
                    recur::normalize_until(&recur::split(r).rule, tz)
                ));
            }
            lines.push(format!("PRIORITY:{}", ical_priority(t.priority)));
            if let Some(p) = t.parent_id {
                lines.push(format!("RELATED-TO;RELTYPE=PARENT:task-{p}@taskcraft"));
            }
            match t.completed_at {
                Some(c) => {
                    lines.push("STATUS:COMPLETED".into());
                    lines.push(format!("COMPLETED:{}", utc_stamp(c)));
                }
                None => lines.push("STATUS:NEEDS-ACTION".into()),
            }
            lines.push("END:VTODO".into());
        }
        lines.push("END:VCALENDAR".into());

        let mut out = String::new();
        for l in &lines {
            fold(l, &mut out);
        }

        Ok(out)
    }
}
