use crate::model::*;
use crate::plans::{first_bucket, next_sort};
use crate::subtasks::{reset_for_next_occurrence, start_parent_if_todo, sync_children_placement};
use crate::{fmt_ts, parse_ts, Store};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};

pub(crate) const TASK_COLS: &str =
    "t.id, t.calendar_id, t.title, t.due_utc, t.due_has_time, t.tz, t.completed_at, t.rrule, t.notes, c.color, t.status, t.plan_id, t.bucket_id, t.sort_order, t.parent_id, t.priority, t.start_utc, t.start_has_time";

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
        parent_id: r.get(14)?,
        priority: Priority::from_sql(&r.get::<_, String>(15)?),
        start: r.get::<_, Option<String>>(16)?.map(|s| parse_ts(&s)),
        start_has_time: r.get(17)?,
        subtask_open: 0,
        subtask_total: 0,
        subtask_in_progress: 0,
        checklist_done: 0,
        checklist_total: 0,
        attachment_count: 0,
    })
}

impl Store {
    pub fn insert_task(&self, t: &TaskInput) -> Result<i64> {
        self.in_tx(|tx| {
        let now = fmt_ts(Utc::now());
        check_start(t)?;
        let parent = check_parent(tx, None, t)?;
        let (plan_id, bucket_id, sort) = match parent {
            Some((_, p, b)) => (p, b, next_child_sort(tx, t.parent_id.unwrap_or_default())?),
            None => {
                let (p, b) = self.placement(tx, t.plan_id, t.bucket_id)?;
                (p, b, next_sort(tx, b)?)
            }
        };
        tx.execute(
            "INSERT INTO tasks(calendar_id, title, due_utc, due_has_time, tz, rrule, notes, status, completed_at, created_at, updated_at, plan_id, bucket_id, sort_order, parent_id, priority, start_utc, start_has_time)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                t.calendar_id, t.title, t.due.map(fmt_ts), t.due_has_time, t.tz, t.rrule, t.notes,
                t.status.as_sql(), (t.status == TaskStatus::Done).then(|| now.clone()), now,
                plan_id, bucket_id, sort, t.parent_id, t.priority.as_sql(),
                t.start.map(fmt_ts), t.start_has_time
            ],
        )?;
        let id = tx.last_insert_rowid();
        start_parent_if_todo(tx, id)?;
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
        let cur: Option<(i64, i64)> = tx
            .query_row("SELECT plan_id, bucket_id FROM tasks WHERE id = ?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        let cur = cur.ok_or(StoreError::NotFound)?;
        check_start(t)?;
        let old_parent: Option<i64> = tx.query_row("SELECT parent_id FROM tasks WHERE id = ?1", [id], |r| r.get(0))?;
        let parent = check_parent(tx, Some(id), t)?;
        let (plan_id, bucket_id, sort) = if let Some((pid, p, b)) = parent {
            // Subtasks live where their parent lives.
            let sort = (old_parent != Some(pid)).then(|| next_child_sort(tx, pid)).transpose()?;
            (p, b, sort)
        } else if t.bucket_id.is_none()
            && t.plan_id.is_none_or(|p| p == cur.0)
        {
            (cur.0, cur.1, None)
        } else {
            let (p, b) = self.placement(tx, t.plan_id, t.bucket_id)?;
            if b == cur.1 {
                (p, b, None)
            } else {
                (p, b, Some(next_sort(tx, b)?))
            }
        };
        // Clearing a parent promotes the task like `set_task_parent(id, None)`.
        let sort = if old_parent.is_some() && t.parent_id.is_none() {
            Some(next_sort(tx, bucket_id)?)
        } else {
            sort
        };
        let n = tx.execute(
            "UPDATE tasks SET calendar_id=?1, title=?2, due_utc=?3, due_has_time=?4, tz=?5, rrule=?6, notes=?7,
               status=?8, completed_at = CASE WHEN ?8 = 'done' THEN COALESCE(completed_at, ?9) END, updated_at=?9,
               plan_id=?11, bucket_id=?12, sort_order=COALESCE(?13, sort_order),
               parent_id=?14, priority=?15, start_utc=?16, start_has_time=?17
             WHERE id=?10",
            params![
                t.calendar_id, t.title, t.due.map(fmt_ts), t.due_has_time, t.tz, t.rrule, t.notes,
                t.status.as_sql(), fmt_ts(Utc::now()), id, plan_id, bucket_id, sort,
                t.parent_id, t.priority.as_sql(), t.start.map(fmt_ts), t.start_has_time
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        sync_children_placement(tx, id)?;
        // Only a start (or a move under a new parent) starts the parent; saving an
        // already in-progress subtask must not undo the user's reset of its parent.
        let started = t.status == TaskStatus::InProgress
            && was.as_deref().map(TaskStatus::from_sql) != Some(TaskStatus::InProgress);
        if started || old_parent != t.parent_id {
            start_parent_if_todo(tx, id)?;
        }
        crate::labels::write_task_labels(tx, id, &t.labels)?;
        self.write_alerts(tx, ItemKindDb::Task, id, &t.alerts)?;
        if newly_done && t.rrule.is_some() && t.due.is_some() {
            self.set_task_completed(id, true, Utc::now())?;
        }
        Ok(())
        })
    }

    /// Fills subtask, checklist and attachment counts with grouped queries.
    pub(crate) fn attach_counts(&self, tasks: &mut [Task]) -> Result<()> {
        use std::collections::HashMap;
        let mut subs: HashMap<i64, (i64, i64, i64)> = HashMap::new();
        let mut st = self.conn.prepare(
            "SELECT parent_id, sum(completed_at IS NULL), count(*), sum(status = 'in_progress')
             FROM tasks WHERE parent_id IS NOT NULL GROUP BY parent_id",
        )?;
        for r in st.query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?, r.get(3)?))))? {
            let (k, v) = r?;
            subs.insert(k, v);
        }
        let mut cl: HashMap<i64, (i64, i64)> = HashMap::new();
        let mut st = self
            .conn
            .prepare("SELECT task_id, sum(done), count(*) FROM checklist_items GROUP BY task_id")?;
        for r in st.query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))? {
            let (k, v) = r?;
            cl.insert(k, v);
        }
        let mut att: HashMap<i64, i64> = HashMap::new();
        let mut st = self
            .conn
            .prepare("SELECT task_id, count(*) FROM attachments GROUP BY task_id")?;
        for r in st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))? {
            let (k, v) = r?;
            att.insert(k, v);
        }
        for t in tasks {
            t.attachment_count = att.get(&t.id).copied().unwrap_or(0);
            (t.subtask_open, t.subtask_total, t.subtask_in_progress) =
                subs.get(&t.id).copied().unwrap_or((0, 0, 0));
            (t.checklist_done, t.checklist_total) = cl.get(&t.id).copied().unwrap_or((0, 0));
        }
        Ok(())
    }

    /// Resolves (plan, bucket): a bucket decides its plan, a plan alone its
    /// first bucket, neither the default plan's first bucket.
    fn placement(
        &self,
        conn: &rusqlite::Connection,
        plan_id: Option<i64>,
        bucket_id: Option<i64>,
    ) -> Result<(i64, i64)> {
        match (plan_id, bucket_id) {
            (p, Some(b)) => {
                let bp: i64 = conn
                    .query_row("SELECT plan_id FROM buckets WHERE id = ?1", [b], |r| {
                        r.get(0)
                    })
                    .optional()?
                    .ok_or_else(|| StoreError::Invalid("bucket does not exist".into()))?;
                if p.is_some_and(|p| p != bp) {
                    return Err(StoreError::Invalid("bucket is not in that plan".into()));
                }
                Ok((bp, b))
            }
            (Some(p), None) => {
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM plans WHERE id = ?1)",
                    [p],
                    |r| r.get(0),
                )?;
                if !exists {
                    return Err(StoreError::Invalid("plan does not exist".into()));
                }
                Ok((p, first_bucket(conn, p)?))
            }
            (None, None) => {
                let p = self.default_plan_id_in(conn)?;
                Ok((p, first_bucket(conn, p)?))
            }
        }
    }

    /// Drops a card into `bucket_id` between the neighbours `before` (above)
    /// and `after` (below), keeping a stable order via fractional sort keys.
    pub fn move_task(
        &self,
        task_id: i64,
        bucket_id: i64,
        before: Option<i64>,
        after: Option<i64>,
    ) -> Result<()> {
        self.in_tx(|tx| {
            let parent: Option<Option<i64>> = tx
                .query_row("SELECT parent_id FROM tasks WHERE id = ?1", [task_id], |r| r.get(0))
                .optional()?;
            match parent {
                None => return Err(StoreError::NotFound),
                Some(Some(_)) => {
                    return Err(StoreError::Invalid("a subtask cannot be moved on its own".into()))
                }
                Some(None) => {}
            }
            let plan_id: i64 = tx
                .query_row("SELECT plan_id FROM buckets WHERE id = ?1", [bucket_id], |r| r.get(0))
                .optional()?
                .ok_or(StoreError::NotFound)?;
            let neighbour = |n: Option<i64>| -> Result<Option<f64>> {
                let Some(n) = n else { return Ok(None) };
                if n == task_id {
                    return Err(StoreError::Invalid("neighbour is the moved task".into()));
                }
                tx.query_row(
                    "SELECT sort_order FROM tasks WHERE id = ?1 AND bucket_id = ?2 AND parent_id IS NULL",
                    params![n, bucket_id],
                    |r| r.get(0),
                )
                .optional()?
                .map(Some)
                .ok_or_else(|| StoreError::Invalid("neighbour is not in the target bucket".into()))
            };
            let (b_sort, a_sort) = (neighbour(before)?, neighbour(after)?);
            let (new, near): (f64, Vec<f64>) = match (b_sort, a_sort) {
                (Some(b), Some(a)) => ((a + b) / 2.0, vec![a, b]),
                (None, Some(a)) => (a - 1.0, vec![a]),
                (Some(b), None) => (b + 1.0, vec![b]),
                (None, None) => {
                    // The task may already be the last one; exclude it from the max.
                    let m: f64 = tx.query_row(
                        "SELECT coalesce(max(sort_order), 0.0) FROM tasks WHERE bucket_id = ?1 AND id <> ?2 AND parent_id IS NULL",
                        params![bucket_id, task_id],
                        |r| r.get(0),
                    )?;
                    (m + 1.0, vec![])
                }
            };
            let crowded = near.iter().any(|n| (new - n).abs() < 1e-6);
            let now = fmt_ts(Utc::now());
            if !crowded {
                tx.execute(
                    "UPDATE tasks SET plan_id = ?1, bucket_id = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
                    params![plan_id, bucket_id, new, now, task_id],
                )?;
                return sync_children_placement(tx, task_id);
            }
            let mut st = tx.prepare(
                "SELECT id FROM tasks WHERE bucket_id = ?1 AND id <> ?2 AND parent_id IS NULL ORDER BY sort_order, id",
            )?;
            let mut ids: Vec<i64> = st
                .query_map(params![bucket_id, task_id], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            drop(st);
            let at = match (before, after) {
                (Some(b), _) => ids.iter().position(|i| *i == b).map_or(ids.len(), |p| p + 1),
                (None, Some(a)) => ids.iter().position(|i| *i == a).unwrap_or(ids.len()),
                (None, None) => ids.len(),
            };
            ids.insert(at, task_id);
            for (i, id) in ids.iter().enumerate() {
                tx.execute(
                    "UPDATE tasks SET plan_id = ?1, bucket_id = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
                    params![plan_id, bucket_id, (i + 1) as f64, now, id],
                )?;
                sync_children_placement(tx, *id)?;
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
        self.attach_counts(std::slice::from_mut(&mut t))?;
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
                        reset_for_next_occurrence(tx, id, now)?;
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
        self.in_tx(|tx| {
            let was: String = tx
                .query_row("SELECT status FROM tasks WHERE id = ?1", [id], |r| r.get(0))
                .optional()?
                .ok_or(StoreError::NotFound)?;
            tx.execute(
                "UPDATE tasks SET status = ?1, completed_at = NULL, updated_at = ?2 WHERE id = ?3",
                params![status.as_sql(), fmt_ts(Utc::now()), id],
            )?;
            if TaskStatus::from_sql(&was) != status {
                start_parent_if_todo(tx, id)?;
            }
            Ok(())
        })
    }

    pub fn delete_task(&self, id: i64) -> Result<()> {
        self.in_tx(|tx| delete_task_row(tx, id))
    }
}

/// Deletes a task, its subtasks and their checklist items, alerts and
/// attachment rows (task_labels cascade); stored files are queued for a
/// delayed delete so Undo can still find them.
pub(crate) fn delete_task_row(conn: &rusqlite::Connection, id: i64) -> Result<()> {
    let mut st = conn.prepare("SELECT id FROM tasks WHERE parent_id = ?1")?;
    let mut all: Vec<i64> = st
        .query_map([id], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    drop(st);
    all.push(id);
    if conn.execute("DELETE FROM tasks WHERE id = ?1", [id])? == 0 {
        return Err(StoreError::NotFound);
    }
    let now = Utc::now();
    for t in all {
        crate::attachments::delete_rows_for_task(conn, t, now)?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", [t])?;
        conn.execute("DELETE FROM checklist_items WHERE task_id = ?1", [t])?;
        conn.execute(
            "DELETE FROM alerts WHERE owner_kind = 'task' AND owner_id = ?1",
            [t],
        )?;
    }
    Ok(())
}

fn check_start(t: &TaskInput) -> Result<()> {
    if let (Some(s), Some(d)) = (t.start, t.due) {
        if s > d {
            return Err(StoreError::Rule(Rule::StartAfterDue));
        }
    }
    Ok(())
}

/// Validates `parent_id`; returns (parent id, its plan, its bucket).
pub(crate) fn check_parent(
    conn: &rusqlite::Connection,
    me: Option<i64>,
    t: &TaskInput,
) -> Result<Option<(i64, i64, i64)>> {
    let Some(pid) = t.parent_id else {
        return Ok(None);
    };
    let row: Option<(Option<i64>, i64, i64)> = conn
        .query_row(
            "SELECT parent_id, plan_id, bucket_id FROM tasks WHERE id = ?1",
            [pid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (pp, plan, bucket) = row.ok_or(StoreError::NotFound)?;
    if pp.is_some() {
        return Err(StoreError::Rule(Rule::ParentIsSubtask));
    }
    if me == Some(pid) {
        return Err(StoreError::Invalid(
            "a task cannot be its own parent".into(),
        ));
    }
    if let Some(me) = me {
        let has: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE parent_id = ?1)",
            [me],
            |r| r.get(0),
        )?;
        if has {
            return Err(StoreError::Rule(Rule::HasSubtasks));
        }
    }
    if t.rrule.is_some() {
        return Err(StoreError::Rule(Rule::RepeatingSubtask));
    }
    Ok(Some((pid, plan, bucket)))
}

pub(crate) fn next_child_sort(conn: &rusqlite::Connection, parent: i64) -> Result<f64> {
    Ok(conn.query_row(
        "SELECT coalesce(max(sort_order), 0.0) + 1.0 FROM tasks WHERE parent_id = ?1",
        [parent],
        |r| r.get(0),
    )?)
}
