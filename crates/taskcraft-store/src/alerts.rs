//! Alert rows: storage of alert specs and their computed fire times.

use crate::model::*;
use crate::{fmt_ts, parse_ts, Store};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};

impl Store {
    /// Replaces all alerts of an owner and computes their fire times from the
    /// owner's base start/due.
    pub(crate) fn write_alerts(
        &self,
        conn: &Connection,
        kind: ItemKindDb,
        id: i64,
        alerts: &[AlertSpecUtc],
    ) -> Result<()> {
        conn.execute(
            "DELETE FROM alerts WHERE owner_kind = ?1 AND owner_id = ?2",
            params![kind.as_str(), id],
        )?;
        let anchor = owner_anchor(conn, kind, id)?;
        for a in alerts {
            let (offset, absolute, fire_at) = match a {
                AlertSpecUtc::Before(m) => match anchor {
                    Some(t) => (
                        Some(i64::from(*m)),
                        None,
                        t - Duration::minutes(i64::from(*m)),
                    ),
                    None => continue,
                },
                AlertSpecUtc::At(t) => (None, Some(fmt_ts(*t)), *t),
            };
            conn.execute(
                "INSERT INTO alerts(owner_kind, owner_id, offset_min, absolute_utc, occurrence_utc, fire_at_utc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![kind.as_str(), id, offset, absolute, anchor.map(fmt_ts), fmt_ts(fire_at)],
            )?;
        }
        Ok(())
    }

    pub(crate) fn read_alerts(&self, kind: ItemKindDb, id: i64) -> Result<Vec<AlertSpecUtc>> {
        let mut st = self.conn.prepare(
            "SELECT offset_min, absolute_utc FROM alerts WHERE owner_kind = ?1 AND owner_id = ?2 ORDER BY id",
        )?;
        let rows = st.query_map(params![kind.as_str(), id], |r| {
            let off: Option<i64> = r.get(0)?;
            let abs: Option<String> = r.get(1)?;
            Ok(match (off, abs) {
                (Some(m), _) => AlertSpecUtc::Before(m as u32),
                (None, Some(a)) => AlertSpecUtc::At(parse_ts(&a)),
                (None, None) => AlertSpecUtc::Before(0),
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

/// Start of an event or due time of a task.
pub(crate) fn owner_anchor(
    conn: &Connection,
    kind: ItemKindDb,
    id: i64,
) -> Result<Option<DateTime<Utc>>> {
    let sql = match kind {
        ItemKindDb::Event => "SELECT start_utc FROM events WHERE id = ?1",
        ItemKindDb::Task => "SELECT due_utc FROM tasks WHERE id = ?1",
    };
    let v: Option<String> = conn.query_row(sql, [id], |r| r.get(0))?;
    Ok(v.map(|s| parse_ts(&s)))
}
