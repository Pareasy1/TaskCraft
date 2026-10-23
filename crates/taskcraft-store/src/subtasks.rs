//! Subtask behaviour: ordering, re-parenting, completion and placement sync.

use crate::model::*;
use crate::plans::next_sort;
use crate::tasks::{check_parent, next_child_sort};
use crate::{fmt_ts, Store};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

/// Subtasks live where their parent lives: copies the parent's plan and
/// bucket onto every child. Called from every path that re-places a parent.
pub(crate) fn sync_children_placement(conn: &Connection, parent_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET
           plan_id = (SELECT plan_id FROM tasks WHERE id = ?1),
           bucket_id = (SELECT bucket_id FROM tasks WHERE id = ?1)
         WHERE parent_id = ?1",
        [parent_id],
    )?;
    Ok(())
}

impl Store {
    /// Subtasks of `parent_id` in sibling order.
    pub fn list_subtasks(&self, parent_id: i64) -> Result<Vec<Task>> {
        let mut st = self
            .conn
            .prepare("SELECT id FROM tasks WHERE parent_id = ?1 ORDER BY sort_order, id")?;
        let ids: Vec<i64> = st
            .query_map([parent_id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        ids.into_iter().map(|id| self.get_task(id)).collect()
    }

    /// `ids` must be exactly the subtasks of `parent_id`.
    pub fn reorder_subtasks(&self, parent_id: i64, ids: &[i64]) -> Result<()> {
        self.in_tx(|tx| {
            let mut st = tx.prepare("SELECT id FROM tasks WHERE parent_id = ?1")?;
            let mut have: Vec<i64> = st
                .query_map([parent_id], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            drop(st);
            let mut want = ids.to_vec();
            have.sort_unstable();
            want.sort_unstable();
            if have != want {
                return Err(StoreError::Invalid(
                    "ids do not match the existing set".into(),
                ));
            }
            for (i, id) in ids.iter().enumerate() {
                tx.execute(
                    "UPDATE tasks SET sort_order = ?1 WHERE id = ?2",
                    params![(i + 1) as f64, id],
                )?;
            }
            Ok(())
        })
    }

    /// `Some(parent)` makes the task a subtask (last sibling, parent's
    /// placement); `None` promotes it, keeping its plan and bucket and
    /// appending it to the bucket.
    pub fn set_task_parent(&self, task_id: i64, parent_id: Option<i64>) -> Result<()> {
        self.in_tx(|tx| {
            let (old, rrule, bucket): (Option<i64>, Option<String>, i64) = tx
                .query_row(
                    "SELECT parent_id, rrule, bucket_id FROM tasks WHERE id = ?1",
                    [task_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?
                .ok_or(StoreError::NotFound)?;
            if old == parent_id {
                return Ok(());
            }
            let now = fmt_ts(Utc::now());
            match parent_id {
                None => {
                    tx.execute(
                        "UPDATE tasks SET parent_id = NULL, sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                        params![next_sort(tx, bucket)?, now, task_id],
                    )?;
                }
                Some(pid) => {
                    let probe = TaskInput {
                        parent_id: Some(pid),
                        rrule,
                        ..Default::default()
                    };
                    let (_, plan, bucket) =
                        check_parent(tx, Some(task_id), &probe)?.ok_or(StoreError::NotFound)?;
                    tx.execute(
                        "UPDATE tasks SET parent_id = ?1, plan_id = ?2, bucket_id = ?3, sort_order = ?4, updated_at = ?5 WHERE id = ?6",
                        params![pid, plan, bucket, next_child_sort(tx, pid)?, now, task_id],
                    )?;
                }
            }
            Ok(())
        })
    }

    /// Marks every open subtask done, then the task itself (through
    /// `set_task_completed`, so a repeating task advances).
    pub fn complete_with_subtasks(&self, task_id: i64, now: DateTime<Utc>) -> Result<()> {
        self.in_tx(|tx| {
            self.get_task(task_id)?;
            tx.execute(
                "UPDATE tasks SET completed_at = ?1, status = 'done', updated_at = ?2
                 WHERE parent_id = ?3 AND completed_at IS NULL",
                params![fmt_ts(now), fmt_ts(Utc::now()), task_id],
            )?;
            self.set_task_completed(task_id, true, now)
        })
    }
}

/// A repeating task came back for its next occurrence: reopen its subtasks,
/// re-arm their alerts still ahead of `now` (ticking a subtask marks its
/// alerts fired), and untick its checklist and theirs.
pub(crate) fn reset_for_next_occurrence(
    conn: &Connection,
    task_id: i64,
    now: DateTime<Utc>,
) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET status = 'todo', completed_at = NULL, updated_at = ?2 WHERE parent_id = ?1",
        params![task_id, fmt_ts(Utc::now())],
    )?;
    conn.execute(
        "UPDATE alerts SET fired_at = NULL
         WHERE owner_kind = 'task' AND fire_at_utc > ?2
           AND owner_id IN (SELECT id FROM tasks WHERE parent_id = ?1)",
        params![task_id, fmt_ts(now)],
    )?;
    conn.execute(
        "UPDATE checklist_items SET done = 0
         WHERE task_id = ?1 OR task_id IN (SELECT id FROM tasks WHERE parent_id = ?1)",
        [task_id],
    )?;
    Ok(())
}
