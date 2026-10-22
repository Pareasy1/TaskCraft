use crate::model::*;
use crate::{fmt_ts, parse_ts, Store};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};

pub(crate) const TASK_COLS: &str =
    "t.id, t.calendar_id, t.title, t.due_utc, t.due_has_time, t.tz, t.completed_at, t.rrule, t.notes, c.color, t.status, t.plan_id, t.bucket_id, t.sort_order";

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
        plan_id: r.get(11)?,
        bucket_id: r.get(12)?,
        sort_order: r.get(13)?,
    })
}

impl Store {
    pub fn insert_task(&self, t: &TaskInput) -> Result<i64> {
        self.in_tx(|tx| {
        let now = fmt_ts(Utc::now());
        tx.execute(
            "INSERT INTO tasks(calendar_id, title, due_utc, due_has_time, tz, rrule, notes, status, completed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                t.calendar_id, t.title, t.due.map(fmt_ts), t.due_has_time, t.tz, t.rrule, t.notes,
                t.status.as_sql(), (t.status == TaskStatus::Done).then(|| now.clone()), now
            ],
        )?;
        let id = tx.last_insert_rowid();
        crate::labels::write_task_labels(tx, id, &t.labels)?;
        self.write_alerts(tx, ItemKindDb::Task, id, &t.alerts)?;
        Ok(id)
        })
    }

    /// Saving a task as Done when it was not done behaves like ticking it:
    /// a recurring task advances to its next occurrence instead.
    pub fn update_task(&self, id: i64, t: &TaskInput) -> Result<()> {
        self.in_tx(|tx| {
        let was: Option<String> = tx
            .query_row("SELECT status FROM tasks WHERE id = ?1", [id], |r| r.get(0))
            .optional()?;
        let newly_done = t.status == TaskStatus::Done
            && was.as_deref().map(TaskStatus::from_sql) != Some(TaskStatus::Done);
        let n = tx.execute(
            "UPDATE tasks SET calendar_id=?1, title=?2, due_utc=?3, due_has_time=?4, tz=?5, rrule=?6, notes=?7,
               status=?8, completed_at = CASE WHEN ?8 = 'done' THEN COALESCE(completed_at, ?9) END, updated_at=?9
             WHERE id=?10",
            params![
                t.calendar_id, t.title, t.due.map(fmt_ts), t.due_has_time, t.tz, t.rrule, t.notes,
                t.status.as_sql(), fmt_ts(Utc::now()), id
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        crate::labels::write_task_labels(tx, id, &t.labels)?;
        self.write_alerts(tx, ItemKindDb::Task, id, &t.alerts)?;
        if newly_done && t.rrule.is_some() && t.due.is_some() {
            self.set_task_completed(id, true, Utc::now())?;
        }
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
        t.labels = self.task_labels(id)?;
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
                            "UPDATE tasks SET due_utc = ?1, completed_at = NULL, status = 'todo', updated_at = ?2 WHERE id = ?3",
                            params![fmt_ts(next), fmt_ts(Utc::now()), id],
                        )?;
                        self.write_alerts(tx, ItemKindDb::Task, id, &t.alerts)
                    });
                }
            }
        }
        let n = self.conn.execute(
            "UPDATE tasks SET completed_at = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
            params![
                done.then(|| fmt_ts(now)),
                if done {
                    TaskStatus::Done
                } else {
                    TaskStatus::Todo
                }
                .as_sql(),
                fmt_ts(Utc::now()),
                id
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// `Done` behaves like ticking the checkbox (recurring tasks advance);
    /// other statuses reopen the task.
    pub fn set_task_status(&self, id: i64, status: TaskStatus, now: DateTime<Utc>) -> Result<()> {
        if status == TaskStatus::Done {
            return self.set_task_completed(id, true, now);
        }
        let n = self.conn.execute(
            "UPDATE tasks SET status = ?1, completed_at = NULL, updated_at = ?2 WHERE id = ?3",
            params![status.as_sql(), fmt_ts(Utc::now()), id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> Result<()> {
        self.in_tx(|tx| delete_task_row(tx, id))
    }
}

/// Deletes a task row and its alerts (task_labels cascade).
pub(crate) fn delete_task_row(conn: &rusqlite::Connection, id: i64) -> Result<()> {
    if conn.execute("DELETE FROM tasks WHERE id = ?1", [id])? == 0 {
        return Err(StoreError::NotFound);
    }
    conn.execute(
        "DELETE FROM alerts WHERE owner_kind = 'task' AND owner_id = ?1",
        [id],
    )?;
    Ok(())
}
