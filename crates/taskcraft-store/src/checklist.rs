//! Lightweight per-task checklists (not tasks; no dates, labels or alerts).

use crate::model::*;
use crate::Store;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const MAX_TEXT: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub id: i64,
    pub task_id: i64,
    pub text: String,
    pub done: bool,
    pub sort: f64,
}

fn clean(text: &str) -> Result<String> {
    let t = text.trim();
    if t.is_empty() || t.chars().count() > MAX_TEXT {
        return Err(StoreError::Rule(Rule::ChecklistText));
    }
    Ok(t.to_string())
}

fn require_task(conn: &Connection, task_id: i64) -> Result<()> {
    conn.query_row("SELECT 1 FROM tasks WHERE id = ?1", [task_id], |_| Ok(()))
        .optional()?
        .ok_or(StoreError::NotFound)
}

fn list(conn: &Connection, task_id: i64) -> Result<Vec<ChecklistItem>> {
    let mut st = conn.prepare(
        "SELECT id, task_id, text, done, sort FROM checklist_items
         WHERE task_id = ?1 ORDER BY sort, id",
    )?;
    let rows = st
        .query_map([task_id], |r| {
            Ok(ChecklistItem {
                id: r.get(0)?,
                task_id: r.get(1)?,
                text: r.get(2)?,
                done: r.get::<_, i64>(3)? != 0,
                sort: r.get(4)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;
    Ok(rows)
}

impl Store {
    pub fn list_checklist(&self, task_id: i64) -> Result<Vec<ChecklistItem>> {
        require_task(&self.conn, task_id)?;
        list(&self.conn, task_id)
    }

    /// Appends an item after the existing ones.
    pub fn add_checklist_item(&self, task_id: i64, text: &str) -> Result<i64> {
        let text = clean(text)?;
        self.in_tx(|tx| {
            require_task(tx, task_id)?;
            let max: Option<f64> = tx.query_row(
                "SELECT MAX(sort) FROM checklist_items WHERE task_id = ?1",
                [task_id],
                |r| r.get(0),
            )?;
            tx.execute(
                "INSERT INTO checklist_items (task_id, text, done, sort) VALUES (?1, ?2, 0, ?3)",
                params![task_id, text, max.unwrap_or(0.0) + 1.0],
            )?;
            Ok(tx.last_insert_rowid())
        })
    }

    pub fn update_checklist_item(
        &self,
        id: i64,
        text: Option<&str>,
        done: Option<bool>,
    ) -> Result<()> {
        let text = text.map(clean).transpose()?;
        self.in_tx(|tx| {
            let mut n = 0;
            if let Some(t) = &text {
                n = tx.execute(
                    "UPDATE checklist_items SET text = ?1 WHERE id = ?2",
                    params![t, id],
                )?;
            }
            if let Some(d) = done {
                n = tx.execute(
                    "UPDATE checklist_items SET done = ?1 WHERE id = ?2",
                    params![d, id],
                )?;
            }
            if text.is_none() && done.is_none() {
                tx.query_row("SELECT 1 FROM checklist_items WHERE id = ?1", [id], |_| {
                    Ok(())
                })
                .optional()?
                .ok_or(StoreError::NotFound)?;
            } else if n == 0 {
                return Err(StoreError::NotFound);
            }
            Ok(())
        })
    }

    /// `ids` must be exactly the task's items; renumbers 1.0, 2.0, ...
    pub fn reorder_checklist(&self, task_id: i64, ids: &[i64]) -> Result<()> {
        self.in_tx(|tx| {
            require_task(tx, task_id)?;
            let mut have: Vec<i64> = list(tx, task_id)?.iter().map(|i| i.id).collect();
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
                    "UPDATE checklist_items SET sort = ?1 WHERE id = ?2",
                    params![(i + 1) as f64, id],
                )?;
            }
            Ok(())
        })
    }

    pub fn delete_checklist_item(&self, id: i64) -> Result<()> {
        let n = self
            .conn
            .execute("DELETE FROM checklist_items WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// Turns an item into a subtask of its task (appended last, inheriting
    /// the parent's calendar and time zone) and removes the item.
    pub fn checklist_item_to_subtask(&self, id: i64) -> Result<i64> {
        self.in_tx(|tx| {
            let (task_id, text): (i64, String) = tx
                .query_row(
                    "SELECT task_id, text FROM checklist_items WHERE id = ?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .ok_or(StoreError::NotFound)?;
            let (calendar_id, tz): (i64, String) = tx.query_row(
                "SELECT calendar_id, tz FROM tasks WHERE id = ?1",
                [task_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let new = self.insert_task(&TaskInput {
                parent_id: Some(task_id),
                title: text,
                calendar_id,
                tz,
                ..Default::default()
            })?;
            tx.execute("DELETE FROM checklist_items WHERE id = ?1", [id])?;
            Ok(new)
        })
    }
}
