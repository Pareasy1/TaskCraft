//! Range queries, task lists, scoped edits and search.

use crate::events::{event_from_row, EVENT_COLS};
use crate::model::*;
use crate::recur::{self, RuleText};
use crate::tasks::{task_from_row, TASK_COLS};
use crate::{fmt_ts, Store};
use chrono::{DateTime, Days, Utc};
use chrono_tz::Tz;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub event_id: i64,
    pub calendar_id: i64,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
    pub location: Option<String>,
    pub recurring: bool,
    pub color: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    This,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskFilter {
    Overdue,
    Today,
    Upcoming,
    Completed,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub kind: ItemKindDb,
    pub id: i64,
    pub title: String,
    pub when: Option<DateTime<Utc>>,
}

fn tz_of(s: &str) -> Tz {
    s.parse().unwrap_or(Tz::UTC)
}

impl Store {
    /// Occurrences overlapping `[start, end)` in visible calendars, sorted by start.
    pub fn list_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<Occurrence>> {
        let sql = format!(
            "SELECT {cols}, c.color FROM events e JOIN calendars c ON c.id = e.calendar_id
             WHERE c.visible = 1 AND e.start_utc < ?1 AND (e.rrule IS NOT NULL OR e.end_utc > ?2)",
            cols = EVENT_COLS
                .split(", ")
                .map(|c| format!("e.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(params![fmt_ts(end), fmt_ts(start)], |r| {
            let (id, e) = event_from_row(r)?;
            let color: String = r.get(12)?;
            Ok((id, e, color))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, e, color) = row?;
            let dur = e.end - e.start;
            let tz = tz_of(&e.tz);
            // All-day items span whole local days, whatever DST does to their length.
            let local_days = (e.end.with_timezone(&tz).date_naive()
                - e.start.with_timezone(&tz).date_naive())
            .num_days()
            .max(1);
            let end_of = |s: DateTime<Utc>| {
                if e.all_day {
                    let d = s.with_timezone(&tz).date_naive() + chrono::Duration::days(local_days);
                    crate::local_to_utc(d.and_hms_opt(0, 0, 0).unwrap_or_default(), tz)
                } else {
                    s + dur
                }
            };
            let make = |s: DateTime<Utc>, recurring: bool| Occurrence {
                event_id: id,
                calendar_id: e.calendar_id,
                title: e.title.clone(),
                start: s,
                end: end_of(s),
                all_day: e.all_day,
                location: e.location.clone(),
                recurring,
                color: color.clone(),
            };
            match &e.rrule {
                None => out.push(make(e.start, false)),
                Some(rule) => {
                    for s in recur::expand(rule, e.start, tz, dur, start, end)? {
                        out.push(make(s, true));
                    }
                }
            }
        }
        out.sort_by_key(|o| (o.start, o.event_id));
        Ok(out)
    }

    pub fn list_tasks(&self, f: TaskFilter, now: DateTime<Utc>, tz: Tz) -> Result<Vec<Task>> {
        let sql = format!("SELECT {TASK_COLS} FROM tasks t JOIN calendars c ON c.id = t.calendar_id WHERE c.visible = 1");
        let mut st = self.conn.prepare(&sql)?;
        let mut all: Vec<Task> = st
            .query_map([], task_from_row)?
            .collect::<std::result::Result<_, _>>()?;
        let mut labels = self.all_task_labels()?;
        for t in &mut all {
            t.labels = labels.remove(&t.id).unwrap_or_default();
        }

        let today = now.with_timezone(&tz).date_naive();
        let day_start = crate::local_to_utc(today.and_hms_opt(0, 0, 0).unwrap_or_default(), tz);
        let tomorrow = today.checked_add_days(Days::new(1)).unwrap_or(today);
        let next_start = crate::local_to_utc(tomorrow.and_hms_opt(0, 0, 0).unwrap_or_default(), tz);
        let overdue = |t: &Task| match t.due {
            Some(d) if t.due_has_time => d < now,
            Some(d) => d < day_start,
            None => false,
        };

        let mut v: Vec<Task> = all
            .into_iter()
            .filter(|t| match f {
                TaskFilter::All => true,
                TaskFilter::Completed => t.completed_at.is_some(),
                _ if t.completed_at.is_some() => false,
                TaskFilter::Overdue => overdue(t),
                TaskFilter::Today => {
                    !overdue(t) && t.due.is_some_and(|d| d >= day_start && d < next_start)
                }
                TaskFilter::Upcoming => t.due.is_none_or(|d| d >= next_start),
            })
            .collect();
        match f {
            TaskFilter::Completed => v.sort_by_key(|t| std::cmp::Reverse(t.completed_at)),
            // Tasks without a due date go last.
            _ => v.sort_by_key(|t| (t.due.is_none(), t.due, t.id)),
        }
        Ok(v)
    }

    /// Moves an event or one occurrence of a series. `This` on a recurring
    /// event excludes `occurrence` from the series and inserts a detached copy
    /// (returning its id). `All` on a series shifts it by the occurrence delta.
    pub fn move_event(
        &self,
        id: i64,
        occurrence: Option<DateTime<Utc>>,
        new_start: DateTime<Utc>,
        new_end: DateTime<Utc>,
        scope: Scope,
    ) -> Result<i64> {
        let mut e = self.get_event(id)?.input;
        if new_end < new_start {
            return Err(StoreError::Invalid("event ends before it starts".into()));
        }
        match (&e.rrule, scope) {
            (Some(rule), Scope::This) => {
                let occ =
                    occurrence.ok_or_else(|| StoreError::Invalid("occurrence required".into()))?;
                self.in_tx(|_| {
                    self.add_exdate(id, rule, occ)?;
                    let mut copy = e.clone();
                    copy.rrule = None;
                    copy.start = new_start;
                    copy.end = new_end;
                    self.insert_event(&copy)
                })
            }
            (Some(rule), Scope::All) => {
                // Shift the series (and its exceptions) by the occurrence's
                // wall-clock delta so exceptions keep matching.
                let tz = tz_of(&e.tz);
                let local = |t: DateTime<Utc>| t.with_timezone(&tz).naive_local();
                let delta = local(new_start) - local(occurrence.unwrap_or(e.start));
                let shift = |t: DateTime<Utc>| crate::local_to_utc(local(t) + delta, tz);
                let mut r = recur::split(rule);
                r.exdates = r.exdates.into_iter().map(shift).collect();
                e.start = shift(e.start);
                e.end = e.start + (new_end - new_start);
                e.rrule = Some(recur::join(&r));
                self.update_event(id, &e)?;
                Ok(id)
            }
            (None, _) => {
                e.start = new_start;
                e.end = new_end;
                self.update_event(id, &e)?;
                Ok(id)
            }
        }
    }

    pub fn delete_event(
        &self,
        id: i64,
        occurrence: Option<DateTime<Utc>>,
        scope: Scope,
    ) -> Result<()> {
        let e = self.get_event(id)?.input;
        if let (Some(rule), Scope::This) = (&e.rrule, scope) {
            let occ =
                occurrence.ok_or_else(|| StoreError::Invalid("occurrence required".into()))?;
            return self.add_exdate(id, rule, occ);
        }
        self.in_tx(|tx| {
            tx.execute("DELETE FROM events WHERE id = ?1", [id])?;
            tx.execute(
                "DELETE FROM alerts WHERE owner_kind = 'event' AND owner_id = ?1",
                [id],
            )?;
            Ok(())
        })
    }

    fn add_exdate(&self, id: i64, rule_col: &str, occ: DateTime<Utc>) -> Result<()> {
        let mut r: RuleText = recur::split(rule_col);
        r.exdates.push(occ);
        let alerts = self.read_alerts(ItemKindDb::Event, id)?;
        self.conn.execute(
            "UPDATE events SET rrule = ?1, updated_at = ?2 WHERE id = ?3",
            params![recur::join(&r), fmt_ts(Utc::now()), id],
        )?;
        // Re-arm alerts so none points at the excluded occurrence.
        self.in_tx(|tx| self.write_alerts(tx, ItemKindDb::Event, id, &alerts))?;
        Ok(())
    }

    /// Case-insensitive search over event title/location/notes and task title/notes.
    pub fn search(&self, q: &str) -> Result<Vec<Hit>> {
        let q = q.trim();
        if q.is_empty() {
            return Ok(vec![]);
        }
        let like = format!(
            "%{}%",
            q.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let mut st = self.conn.prepare(
            "SELECT 'event', id, title, start_utc FROM events
               WHERE title LIKE ?1 ESCAPE '\\' OR location LIKE ?1 ESCAPE '\\' OR notes LIKE ?1 ESCAPE '\\'
             UNION ALL
             SELECT 'task', id, title, due_utc FROM tasks
               WHERE title LIKE ?1 ESCAPE '\\' OR notes LIKE ?1 ESCAPE '\\'
                  OR id IN (SELECT tl.task_id FROM task_labels tl JOIN labels l ON l.id = tl.label_id
                            WHERE l.name LIKE ?1 ESCAPE '\\')
             ORDER BY 4 DESC LIMIT 50",
        )?;
        let rows = st.query_map([like], |r| {
            let kind: String = r.get(0)?;
            Ok(Hit {
                kind: if kind == "event" {
                    ItemKindDb::Event
                } else {
                    ItemKindDb::Task
                },
                id: r.get(1)?,
                title: r.get(2)?,
                when: r.get::<_, Option<String>>(3)?.map(|s| crate::parse_ts(&s)),
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}
