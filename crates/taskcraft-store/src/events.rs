use crate::model::*;
use crate::{fmt_ts, parse_ts, Store};
use chrono::Utc;
use rusqlite::{params, OptionalExtension, Row};

pub(crate) const EVENT_COLS: &str =
    "id, calendar_id, title, start_utc, end_utc, tz, all_day, location, attendees, notes, url, rrule";

pub(crate) fn event_from_row(r: &Row) -> rusqlite::Result<(i64, EventInput)> {
    let attendees: Option<String> = r.get(8)?;
    Ok((
        r.get(0)?,
        EventInput {
            calendar_id: r.get(1)?,
            title: r.get(2)?,
            start: parse_ts(&r.get::<_, String>(3)?),
            end: parse_ts(&r.get::<_, String>(4)?),
            tz: r.get(5)?,
            all_day: r.get(6)?,
            location: r.get(7)?,
            attendees: attendees
                .map(|a| {
                    a.lines()
                        .map(str::to_string)
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            notes: r.get(9)?,
            url: r.get(10)?,
            rrule: r.get(11)?,
            alerts: vec![],
        },
    ))
}

impl Store {
    pub fn insert_event(&self, e: &EventInput) -> Result<i64> {
        validate_event(e)?;
        self.in_tx(|tx| {
        let now = fmt_ts(Utc::now());
        tx.execute(
            "INSERT INTO events(calendar_id, title, start_utc, end_utc, tz, all_day, location, attendees, notes, url, rrule, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                e.calendar_id, e.title, fmt_ts(e.start), fmt_ts(e.end), e.tz, e.all_day, e.location,
                join_attendees(&e.attendees), e.notes, e.url, e.rrule, now
            ],
        )?;
        let id = tx.last_insert_rowid();
        self.write_alerts(tx, ItemKindDb::Event, id, &e.alerts)?;
        Ok(id)
        })
    }

    pub fn update_event(&self, id: i64, e: &EventInput) -> Result<()> {
        validate_event(e)?;
        self.in_tx(|tx| {
        let n = tx.execute(
            "UPDATE events SET calendar_id=?1, title=?2, start_utc=?3, end_utc=?4, tz=?5, all_day=?6, location=?7,
             attendees=?8, notes=?9, url=?10, rrule=?11, updated_at=?12 WHERE id=?13",
            params![
                e.calendar_id, e.title, fmt_ts(e.start), fmt_ts(e.end), e.tz, e.all_day, e.location,
                join_attendees(&e.attendees), e.notes, e.url, e.rrule, fmt_ts(Utc::now()), id
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        self.write_alerts(tx, ItemKindDb::Event, id, &e.alerts)?;
        Ok(())
        })
    }

    pub fn get_event(&self, id: i64) -> Result<Event> {
        let sql = format!("SELECT {EVENT_COLS} FROM events WHERE id = ?1");
        let (id, mut input) = self
            .conn
            .query_row(&sql, [id], event_from_row)
            .optional()?
            .ok_or(StoreError::NotFound)?;
        input.alerts = self.read_alerts(ItemKindDb::Event, id)?;
        Ok(Event { id, input })
    }
}

fn join_attendees(a: &[String]) -> Option<String> {
    (!a.is_empty()).then(|| a.join("\n"))
}

fn validate_event(e: &EventInput) -> Result<()> {
    if e.end < e.start {
        return Err(StoreError::Invalid("event ends before it starts".into()));
    }
    e.tz.parse::<chrono_tz::Tz>()
        .map_err(|_| StoreError::Invalid(format!("unknown time zone {}", e.tz)))?;
    Ok(())
}
