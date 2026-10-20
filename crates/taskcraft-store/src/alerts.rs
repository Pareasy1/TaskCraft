//! Alert rows: storage, due-alert selection and re-arming for recurring items.
//!
//! Tasks with a due time get an implicit alert at the due time, stored as
//! `Before(0)` and hidden from `read_alerts`.

use crate::model::*;
use crate::{fmt_ts, parse_ts, recur, Store};
use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Alerts more than this late are dropped instead of shown.
const STALE_AFTER: Duration = Duration::hours(12);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueAlert {
    pub alert_id: i64,
    pub owner: ItemKindDb,
    pub owner_id: i64,
    pub title: String,
    pub when: DateTime<Utc>,
    pub location: Option<String>,
}

/// What an alert hangs off: start/due, recurrence and zone of its owner.
struct Owner {
    anchor: Option<DateTime<Utc>>,
    rrule: Option<String>,
    tz: Tz,
    due_has_time: bool,
}

fn owner(conn: &Connection, kind: ItemKindDb, id: i64) -> Result<Option<Owner>> {
    let sql = match kind {
        ItemKindDb::Event => "SELECT start_utc, rrule, tz, 0 FROM events WHERE id = ?1",
        ItemKindDb::Task => "SELECT due_utc, rrule, tz, due_has_time FROM tasks WHERE id = ?1",
    };
    Ok(conn
        .query_row(sql, [id], |r| {
            Ok(Owner {
                anchor: r.get::<_, Option<String>>(0)?.map(|s| parse_ts(&s)),
                rrule: r.get(1)?,
                tz: r.get::<_, String>(2)?.parse().unwrap_or(Tz::UTC),
                due_has_time: r.get(3)?,
            })
        })
        .optional()?)
}

fn kind_of(s: &str) -> ItemKindDb {
    if s == "task" {
        ItemKindDb::Task
    } else {
        ItemKindDb::Event
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_alert(
    conn: &Connection,
    kind: ItemKindDb,
    id: i64,
    offset: Option<i64>,
    absolute: Option<DateTime<Utc>>,
    occurrence: Option<DateTime<Utc>>,
    fire_at: DateTime<Utc>,
    fired_at: Option<DateTime<Utc>>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO alerts(owner_kind, owner_id, offset_min, absolute_utc, occurrence_utc, fire_at_utc, fired_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            kind.as_str(),
            id,
            offset,
            absolute.map(fmt_ts),
            occurrence.map(fmt_ts),
            fmt_ts(fire_at),
            fired_at.map(fmt_ts)
        ],
    )?;
    Ok(())
}

struct PrevRow {
    offset: Option<i64>,
    absolute: Option<DateTime<Utc>>,
    occurrence: Option<DateTime<Utc>>,
    fire_at: DateTime<Utc>,
    fired_at: Option<DateTime<Utc>>,
}

fn previous_rows(conn: &Connection, kind: ItemKindDb, id: i64) -> Result<Vec<PrevRow>> {
    let mut st = conn.prepare(
        "SELECT offset_min, absolute_utc, occurrence_utc, fire_at_utc, fired_at FROM alerts
         WHERE owner_kind = ?1 AND owner_id = ?2",
    )?;
    let ts = |v: Option<String>| v.map(|s| parse_ts(&s));
    let rows = st.query_map(params![kind.as_str(), id], |r| {
        Ok(PrevRow {
            offset: r.get(0)?,
            absolute: ts(r.get(1)?),
            occurrence: ts(r.get(2)?),
            fire_at: parse_ts(&r.get::<_, String>(3)?),
            fired_at: ts(r.get(4)?),
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Fired state of an identical previous alert (same spec and fire time).
fn carried(
    prev: &[PrevRow],
    offset: Option<i64>,
    absolute: Option<DateTime<Utc>>,
    fire_at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    prev.iter()
        .find(|p| p.offset == offset && p.absolute == absolute && p.fire_at == fire_at)
        .and_then(|p| p.fired_at)
}

impl Store {
    /// Replaces all alerts of an owner, computing fire times from its start/due.
    /// Edits must not re-fire alerts: an unchanged one-time alert keeps its
    /// fired state, and a recurring alert resumes after the last occurrence it
    /// already fired for.
    pub(crate) fn write_alerts(
        &self,
        conn: &Connection,
        kind: ItemKindDb,
        id: i64,
        alerts: &[AlertSpecUtc],
    ) -> Result<()> {
        let previous = previous_rows(conn, kind, id)?;
        conn.execute(
            "DELETE FROM alerts WHERE owner_kind = ?1 AND owner_id = ?2",
            params![kind.as_str(), id],
        )?;
        let Some(o) = owner(conn, kind, id)? else {
            return Ok(());
        };
        let mut specs = alerts.to_vec();
        if kind == ItemKindDb::Task && o.due_has_time && !specs.contains(&AlertSpecUtc::Before(0)) {
            specs.push(AlertSpecUtc::Before(0));
        }
        for a in &specs {
            match (a, o.rrule.as_deref(), o.anchor) {
                (AlertSpecUtc::Before(m), Some(rule), Some(start)) => {
                    let m = i64::from(*m);
                    let progress = previous
                        .iter()
                        .filter(|p| p.offset == Some(m) && p.fired_at.is_some())
                        .filter_map(|p| p.occurrence)
                        .max();
                    let after = progress.unwrap_or(start - Duration::seconds(1));
                    let occ = recur::next_after(rule, start, o.tz, after)?;
                    if let Some(occ) = occ {
                        insert_alert(
                            conn,
                            kind,
                            id,
                            Some(m),
                            None,
                            Some(occ),
                            occ - Duration::minutes(m),
                            None,
                        )?;
                    }
                }
                (AlertSpecUtc::Before(m), _, Some(t)) => {
                    let m = i64::from(*m);
                    let fire_at = t - Duration::minutes(m);
                    let fired = carried(&previous, Some(m), None, fire_at);
                    insert_alert(conn, kind, id, Some(m), None, Some(t), fire_at, fired)?;
                }
                (AlertSpecUtc::Before(_), _, None) => {}
                (AlertSpecUtc::At(t), _, anchor) => {
                    let fired = carried(&previous, None, Some(*t), *t);
                    insert_alert(conn, kind, id, None, Some(*t), anchor, *t, fired)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn read_alerts(&self, kind: ItemKindDb, id: i64) -> Result<Vec<AlertSpecUtc>> {
        let hide_implicit = kind == ItemKindDb::Task
            && owner(&self.conn, kind, id)?.is_some_and(|o| o.due_has_time);
        let mut st = self.conn.prepare(
            "SELECT offset_min, absolute_utc FROM alerts
             WHERE owner_kind = ?1 AND owner_id = ?2
             GROUP BY offset_min, absolute_utc ORDER BY MIN(id)",
        )?;
        let rows = st.query_map(params![kind.as_str(), id], |r| {
            let off: Option<i64> = r.get(0)?;
            let abs: Option<String> = r.get(1)?;
            Ok(match (off, abs) {
                (Some(m), _) => AlertSpecUtc::Before(u32::try_from(m).unwrap_or(0)),
                (None, Some(a)) => AlertSpecUtc::At(parse_ts(&a)),
                (None, None) => AlertSpecUtc::Before(0),
            })
        })?;
        let v: Vec<AlertSpecUtc> = rows.collect::<std::result::Result<_, _>>()?;
        Ok(v.into_iter()
            .filter(|a| !(hide_implicit && *a == AlertSpecUtc::Before(0)))
            .collect())
    }

    /// Unfired alerts with `fire_at <= now`. Alerts more than 12 h late, and
    /// alerts of completed tasks, are marked fired without being returned.
    pub fn due_alerts(&self, now: DateTime<Utc>) -> Result<Vec<DueAlert>> {
        let stale: Vec<i64> = {
            let mut st = self.conn.prepare(
                "SELECT a.id FROM alerts a LEFT JOIN tasks t ON a.owner_kind = 'task' AND t.id = a.owner_id
                 WHERE a.fired_at IS NULL AND (a.fire_at_utc < ?1 OR t.completed_at IS NOT NULL)",
            )?;
            let ids = st.query_map([fmt_ts(now - STALE_AFTER)], |r| r.get(0))?;
            ids.collect::<std::result::Result<_, _>>()?
        };
        for id in stale {
            self.mark_fired(id, now)?;
        }

        let mut st = self.conn.prepare(
            "SELECT a.id, a.owner_kind, a.owner_id, a.occurrence_utc, a.fire_at_utc,
                    COALESCE(e.title, t.title), e.location
             FROM alerts a
             LEFT JOIN events e ON a.owner_kind = 'event' AND e.id = a.owner_id
             LEFT JOIN tasks t ON a.owner_kind = 'task' AND t.id = a.owner_id
             WHERE a.fired_at IS NULL AND a.fire_at_utc <= ?1
               AND (e.id IS NOT NULL OR t.id IS NOT NULL)
             ORDER BY a.fire_at_utc, a.id",
        )?;
        let rows = st.query_map([fmt_ts(now)], |r| {
            let fire_at = parse_ts(&r.get::<_, String>(4)?);
            Ok(DueAlert {
                alert_id: r.get(0)?,
                owner: kind_of(&r.get::<_, String>(1)?),
                owner_id: r.get(2)?,
                when: r
                    .get::<_, Option<String>>(3)?
                    .map_or(fire_at, |s| parse_ts(&s)),
                title: r.get(5)?,
                location: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Marks an alert fired. For a relative alert on a recurring item, arms
    /// the same alert for the next occurrence after `max(occurrence, now)`.
    pub fn mark_fired(&self, alert_id: i64, now: DateTime<Utc>) -> Result<()> {
        self.in_tx(|tx| {
            let row = tx
                .query_row(
                    "SELECT owner_kind, owner_id, offset_min, occurrence_utc FROM alerts WHERE id = ?1",
                    [alert_id],
                    |r| {
                        Ok((
                            kind_of(&r.get::<_, String>(0)?),
                            r.get::<_, i64>(1)?,
                            r.get::<_, Option<i64>>(2)?,
                            r.get::<_, Option<String>>(3)?.map(|s| parse_ts(&s)),
                        ))
                    },
                )
                .optional()?;
            let Some((kind, owner_id, offset, occurrence)) = row else {
                return Err(StoreError::NotFound);
            };
            tx.execute("UPDATE alerts SET fired_at = ?1 WHERE id = ?2", params![fmt_ts(now), alert_id])?;

            let (Some(offset), Some(o)) = (offset, owner(tx, kind, owner_id)?) else { return Ok(()) };
            let (Some(rule), Some(start)) = (o.rrule.as_deref(), o.anchor) else { return Ok(()) };
            let after = occurrence.map_or(now, |occ| occ.max(now));
            if let Some(next) = recur::next_after(rule, start, o.tz, after)? {
                insert_alert(tx, kind, owner_id, Some(offset), None, Some(next), next - Duration::minutes(offset), None)?;
            }
            Ok(())
        })
    }
}
