use crate::model::*;
use crate::{fmt_ts, parse_ts, Store};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};

pub(crate) const TASK_COLS: &str =
    "t.id, t.calendar_id, t.title, t.due_utc, t.due_has_time, t.tz, t.completed_at, t.rrule, t.notes, c.color, t.status";

pub(crate) fn task_from_row(r: &Row) -> rusqlite::Result<Task> {
    let rrule: Option<String> = r.get(7)?;
    Ok(Task {
        id: r.get(0)?,
        calendar_id: r.get(1)?,
        title: r.get(2)?,
        due: r.get::<_, Option<String>>(3)?.map(|s| parse_ts(&s)),
        due_has_time: r.get(4)?,
        tz: r.get(5)?,
        completed_at: r.get::<_, Option<String>>(6)?.map(|s| parse_ts(&s)),
        recurring: rrule.is_some(),
        rrule,
        notes: r.get(8)?,
        color: r.get(9)?,
        alerts: vec![],
        status: TaskStatus::from_sql(&r.get::<_, String>(10)?),
        labels: vec![],
    })
}

impl Store {
    pub fn insert_task(&self, t: &TaskInput) -> Result<i64> {
        self.in_tx(|tx| {
        let now = fmt_ts(Utc::now());
        tx.execute(
            "INSERT INTO tasks(calendar_id, title, due_utc, due_has_time, tz, rrule, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![t.calendar_id, t.title, t.due.map(fmt_ts), t.due_has_time, t.tz, t.rrule, t.notes, now],
        )?;
        let id = tx.last_insert_rowid();
        self.write_alerts(tx, ItemKindDb::Task, id, &t.alerts)?;
        Ok(id)
        })
    }

    pub fn update_task(&self, id: i64, t: &TaskInput) -> Result<()> {
        self.in_tx(|tx| {
        let n = tx.execute(
            "UPDATE tasks SET calendar_id=?1, title=?2, due_utc=?3, due_has_time=?4, tz=?5, rrule=?6, notes=?7, updated_at=?8
             WHERE id=?9",
            params![t.calendar_id, t.title, t.due.map(fmt_ts), t.due_has_time, t.tz, t.rrule, t.notes, fmt_ts(Utc::now()), id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        self.write_alerts(tx, ItemKindDb::Task, id, &t.alerts)?;
        Ok(())
        })
    }

    pub fn get_task(&self, id: i64) -> Result<Task> {
        let sql = format!("SELECT {TASK_COLS} FROM tasks t JOIN calendars c ON c.id = t.calendar_id WHERE t.id = ?1");
        let mut t = self
            .conn
            .query_row(&sql, [id], task_from_row)
            .optional()?
            .ok_or(StoreError::NotFound)?;
        t.alerts = self.read_alerts(ItemKindDb::Task, id)?;
        Ok(t)
    }

    /// Completing a recurring task moves it to its next occurrence instead;
    /// it is only marked done once the series has ended.
    pub fn set_task_completed(&self, id: i64, done: bool, now: DateTime<Utc>) -> Result<()> {
        let t = self.get_task(id)?;
        if done {
            if let (Some(rule), Some(due)) = (&t.rrule, t.due) {
                let tz: chrono_tz::Tz = t.tz.parse().unwrap_or(chrono_tz::Tz::UTC);
                if let Some(next) = crate::recur::next_after(rule, due, tz, due)? {
                    return self.in_tx(|tx| {
                        tx.execute(
                            "UPDATE tasks SET due_utc = ?1, completed_at = NULL, updated_at = ?2 WHERE id = ?3",
                            params![fmt_ts(next), fmt_ts(Utc::now()), id],
                        )?;
                        self.write_alerts(tx, ItemKindDb::Task, id, &t.alerts)
                    });
                }
            }
        }
        let n = self.conn.execute(
            "UPDATE tasks SET completed_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![done.then(|| fmt_ts(now)), fmt_ts(Utc::now()), id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> Result<()> {
        self.in_tx(|tx| {
            if tx.execute("DELETE FROM tasks WHERE id = ?1", [id])? == 0 {
                return Err(StoreError::NotFound);
            }
            tx.execute(
                "DELETE FROM alerts WHERE owner_kind = 'task' AND owner_id = ?1",
                [id],
            )?;
            Ok(())
        })
    }
}
