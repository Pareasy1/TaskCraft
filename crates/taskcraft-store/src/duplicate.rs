//! Duplicating a task with its subtasks, checklists and attachments.

use crate::model::*;
use crate::{fmt_ts, Store};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

const COPY_COLS: &str = "calendar_id, due_utc, due_has_time, tz, rrule, notes, plan_id, bucket_id, priority, start_utc, start_has_time";

/// Places `new` right after `orig` among its siblings: the midpoint to the
/// next sibling, `+1.0` when it is last, and a full renumber when the gap
/// is too small to split.
fn place_after(conn: &Connection, orig: i64, new: i64) -> Result<()> {
    let (parent, bucket, sort): (Option<i64>, i64, f64) = conn.query_row(
        "SELECT parent_id, bucket_id, sort_order FROM tasks WHERE id = ?1",
        [orig],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    // Siblings: the children of one parent, or the top-level tasks of a bucket.
    let (scope, key) = match parent {
        Some(p) => ("parent_id = ?1", p),
        None => ("parent_id IS NULL AND bucket_id = ?1", bucket),
    };
    let next: Option<f64> = conn.query_row(
        &format!(
            "SELECT min(sort_order) FROM tasks WHERE {scope} AND id <> ?2 AND sort_order > ?3"
        ),
        params![key, new, sort],
        |r| r.get(0),
    )?;
    let at = next.map_or(sort + 1.0, |n| (sort + n) / 2.0);
    let crowded = next.is_some_and(|n| (at - sort).abs() < 1e-6 || (n - at).abs() < 1e-6);
    let now = fmt_ts(Utc::now());
    if !crowded {
        conn.execute(
            "UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![at, now, new],
        )?;
        return Ok(());
    }
    let mut st = conn.prepare(&format!(
        "SELECT id FROM tasks WHERE {scope} AND id <> ?2 ORDER BY sort_order, id"
    ))?;
    let mut ids: Vec<i64> = st
        .query_map(params![key, new], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    drop(st);
    let pos = ids
        .iter()
        .position(|i| *i == orig)
        .map_or(ids.len(), |p| p + 1);
    ids.insert(pos, new);
    for (i, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![(i + 1) as f64, now, id],
        )?;
    }
    Ok(())
}

/// Copies one task row (todo, not completed) plus labels, alerts and
/// unticked checklist items; returns the new id. `parent` overrides the
/// parent (subtasks of a copy); sort order is set by the caller.
fn copy_row(
    store: &Store,
    conn: &Connection,
    id: i64,
    title_suffix: &str,
    parent: Option<i64>,
) -> Result<i64> {
    let now = fmt_ts(Utc::now());
    let n = conn.execute(
        &format!(
            "INSERT INTO tasks(title, status, completed_at, created_at, updated_at, parent_id, sort_order, {COPY_COLS})
             SELECT title || ?1, 'todo', NULL, ?2, ?2, ?3, sort_order, {COPY_COLS} FROM tasks WHERE id = ?4"
        ),
        params![title_suffix, now, parent, id],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound);
    }
    let new = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO task_labels(task_id, label_id) SELECT ?1, label_id FROM task_labels WHERE task_id = ?2",
        params![new, id],
    )?;
    store.write_alerts(
        conn,
        ItemKindDb::Task,
        new,
        &store.read_alerts(ItemKindDb::Task, id)?,
    )?;
    conn.execute(
        "INSERT INTO checklist_items(task_id, text, done, sort) SELECT ?1, text, 0, sort FROM checklist_items WHERE task_id = ?2",
        params![new, id],
    )?;
    Ok(new)
}

impl Store {
    /// Copies a task as a new `todo` task titled `<title><suffix>`, with its
    /// labels, alerts, checklist (unticked), subtasks and attachments (files
    /// become new stored files under `data_dir`). It lands right after the
    /// original among its siblings. All-or-nothing: copied files are removed
    /// if anything fails.
    pub fn duplicate_task(&self, id: i64, suffix: &str, data_dir: &Path) -> Result<i64> {
        let made: RefCell<Vec<PathBuf>> = RefCell::new(Vec::new());
        let res = self.in_tx(|tx| {
            let new = copy_row(self, tx, id, suffix, None)?;
            // The copy keeps the original's parent (a duplicated subtask).
            tx.execute(
                "UPDATE tasks SET parent_id = (SELECT parent_id FROM tasks WHERE id = ?1) WHERE id = ?2",
                params![id, new],
            )?;
            place_after(tx, id, new)?;
            crate::attachments::copy_for_duplicate(tx, id, new, data_dir, &made)?;
            let mut st = tx.prepare("SELECT id FROM tasks WHERE parent_id = ?1 ORDER BY sort_order, id")?;
            let kids: Vec<i64> = st
                .query_map([id], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            drop(st);
            for (i, k) in kids.into_iter().enumerate() {
                let nk = copy_row(self, tx, k, "", Some(new))?;
                tx.execute(
                    "UPDATE tasks SET sort_order = ?1 WHERE id = ?2",
                    params![(i + 1) as f64, nk],
                )?;
                crate::attachments::copy_for_duplicate(tx, k, nk, data_dir, &made)?;
            }
            Ok(new)
        });
        if res.is_err() {
            for p in made.borrow().iter() {
                let _ = std::fs::remove_file(p);
            }
            // Folders are listed before their files; remove_dir only
            // succeeds on the now-empty ones.
            for p in made.borrow().iter().rev() {
                let _ = std::fs::remove_dir(p);
            }
        }
        res
    }
}
