//! Local wall-clock ↔ UTC conversion and ParsedItem → store input mapping.

use crate::model::*;
use chrono::{DateTime, Days, Duration, LocalResult, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use taskcraft_parse::{AlertSpec, ItemKind, ParsedItem, When};

/// Date-only values mean local midnight. Ambiguous local times (DST end)
/// take the earliest instant; nonexistent ones (DST start) move forward 1h.
pub fn to_utc(w: &When, tz: Tz) -> DateTime<Utc> {
    local_to_utc(
        NaiveDateTime::new(w.date, w.time.unwrap_or(NaiveTime::MIN)),
        tz,
    )
}

pub fn local_to_utc(ndt: NaiveDateTime, tz: Tz) -> DateTime<Utc> {
    match tz.from_local_datetime(&ndt) {
        LocalResult::Single(t) => t.with_timezone(&Utc),
        LocalResult::Ambiguous(a, _) => a.with_timezone(&Utc),
        LocalResult::None => match tz.from_local_datetime(&(ndt + Duration::hours(1))) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => t.with_timezone(&Utc),
            LocalResult::None => Utc.from_utc_datetime(&ndt),
        },
    }
}

fn alerts_utc(alerts: &[AlertSpec], tz: Tz) -> Vec<AlertSpecUtc> {
    alerts
        .iter()
        .map(|a| match a {
            AlertSpec::Before(m) => AlertSpecUtc::Before(*m),
            AlertSpec::At(w) => AlertSpecUtc::At(to_utc(w, tz)),
        })
        .collect()
}

/// Maps a parsed sentence to a store input. `/name` picks a calendar by
/// name; otherwise `default_calendar_id` is used.
pub fn from_parsed(
    p: &ParsedItem,
    tz: Tz,
    default_calendar_id: i64,
    calendars: &[Calendar],
    default_event_minutes: u32,
) -> NewItem {
    let calendar_id = p
        .calendar
        .as_ref()
        .and_then(|name| calendars.iter().find(|c| c.name.eq_ignore_ascii_case(name)))
        .map_or(default_calendar_id, |c| c.id);
    let rrule = p.recurrence.as_ref().map(|r| r.to_rrule());
    let alerts = alerts_utc(&p.alerts, tz);

    match p.kind {
        ItemKind::Task => NewItem::Task(TaskInput {
            calendar_id,
            title: p.title.clone(),
            due: p.start.as_ref().map(|w| to_utc(w, tz)),
            due_has_time: p.start.is_some_and(|w| w.time.is_some()),
            tz: tz.name().to_string(),
            rrule,
            notes: None,
            alerts,
        }),
        ItemKind::Event => {
            let start_w = p.start.unwrap_or(When {
                date: Utc::now().with_timezone(&tz).date_naive(),
                time: None,
            });
            let (start, end) = if p.all_day || start_w.time.is_none() {
                let last = p.end.map_or(start_w.date, |e| e.date);
                let after = last.checked_add_days(Days::new(1)).unwrap_or(last);
                (
                    to_utc(
                        &When {
                            date: start_w.date,
                            time: None,
                        },
                        tz,
                    ),
                    to_utc(
                        &When {
                            date: after,
                            time: None,
                        },
                        tz,
                    ),
                )
            } else {
                let s = to_utc(&start_w, tz);
                let e = p.end.map_or(
                    s + Duration::minutes(i64::from(default_event_minutes)),
                    |e| to_utc(&e, tz),
                );
                (s, e)
            };
            NewItem::Event(EventInput {
                calendar_id,
                title: p.title.clone(),
                start,
                end,
                tz: tz.name().to_string(),
                all_day: p.all_day || start_w.time.is_none(),
                location: p.location.clone(),
                attendees: p.attendees.clone(),
                notes: None,
                url: p.url.clone(),
                rrule,
                alerts,
            })
        }
    }
}
