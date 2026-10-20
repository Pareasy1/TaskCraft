//! Task labels: created on demand from names, linked through `task_labels`.

use crate::model::*;
use crate::Store;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

const PALETTE: [&str; 8] = [
    "#3b82f6", "#f97316", "#10b981", "#a855f7", "#ef4444", "#eab308", "#06b6d4", "#ec4899",
];
const MAX_LEN: usize = 32;

/// Trims, strips one leading `#`, drops empty or over-long names and
/// case-insensitive duplicates (first spelling wins).
pub(crate) fn normalize_labels(names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for n in names {
        let n = n.trim();
        let n = n.strip_prefix('#').unwrap_or(n).trim();
        if n.is_empty() || n.chars().count() > MAX_LEN {
            continue;
        }
        if !out.iter().any(|o| o.to_lowercase() == n.to_lowercase()) {
            out.push(n.to_string());
        }
    }
    out
}

/// Replaces the task's labels with `names`, creating missing labels.
pub(crate) fn write_task_labels(conn: &Connection, task_id: i64, names: &[String]) -> Result<()> {
    conn.execute("DELETE FROM task_labels WHERE task_id = ?1", [task_id])?;
    for name in normalize_labels(names) {
        let existing: Option<i64> = conn
            .query_row("SELECT id FROM labels WHERE name = ?1", [&name], |r| {
                r.get(0)
            })
            .optional()?;
        let id = match existing {
            Some(id) => id,
            None => {
                let count: i64 = conn.query_row("SELECT count(*) FROM labels", [], |r| r.get(0))?;
                let color = PALETTE[count as usize % PALETTE.len()];
                conn.execute(
                    "INSERT INTO labels(name, color) VALUES (?1, ?2)",
                    params![name, color],
                )?;
                conn.last_insert_rowid()
            }
        };
        conn.execute(
            "INSERT OR IGNORE INTO task_labels(task_id, label_id) VALUES (?1, ?2)",
            params![task_id, id],
        )?;
    }
    Ok(())
}

const LABEL_ORDER: &str = "ORDER BY l.name COLLATE NOCASE";

impl Store {
    /// Labels of one task, by name.
    pub(crate) fn task_labels(&self, task_id: i64) -> Result<Vec<TaskLabel>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT l.id, l.name, l.color FROM task_labels tl JOIN labels l ON l.id = tl.label_id
             WHERE tl.task_id = ?1 {LABEL_ORDER}"
        ))?;
        let v = st
            .query_map([task_id], |r| {
                Ok(TaskLabel {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        Ok(v)
    }

    /// Labels of every task, keyed by task id.
    pub(crate) fn all_task_labels(&self) -> Result<HashMap<i64, Vec<TaskLabel>>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT tl.task_id, l.id, l.name, l.color FROM task_labels tl JOIN labels l ON l.id = tl.label_id {LABEL_ORDER}"
        ))?;
        let mut map: HashMap<i64, Vec<TaskLabel>> = HashMap::new();
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                TaskLabel {
                    id: r.get(1)?,
                    name: r.get(2)?,
                    color: r.get(3)?,
                },
            ))
        })?;
        for row in rows {
            let (task, label) = row?;
            map.entry(task).or_default().push(label);
        }
        Ok(map)
    }

    pub fn list_labels(&self) -> Result<Vec<TaskLabel>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT l.id, l.name, l.color FROM labels l {LABEL_ORDER}"
        ))?;
        let v = st
            .query_map([], |r| {
                Ok(TaskLabel {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        Ok(v)
    }

    /// Renames and recolors a label. Renaming to another label's name
    /// (ignoring case) merges this label into that one.
    pub fn update_label(&self, id: i64, name: &str, color: &str) -> Result<()> {
        let Some(name) = normalize_labels(&[name.to_string()]).pop() else {
            return Err(StoreError::Invalid(format!(
                "A label name needs 1–{MAX_LEN} characters"
            )));
        };
        self.in_tx(|tx| {
            let other: Option<i64> = tx
                .query_row(
                    "SELECT id FROM labels WHERE name = ?1 AND id <> ?2",
                    params![name, id],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(keep) = other {
                tx.execute(
                    "INSERT OR IGNORE INTO task_labels(task_id, label_id)
                     SELECT task_id, ?1 FROM task_labels WHERE label_id = ?2",
                    params![keep, id],
                )?;
                if tx.execute("DELETE FROM labels WHERE id = ?1", [id])? == 0 {
                    return Err(StoreError::NotFound);
                }
                return Ok(());
            }
            let n = tx.execute(
                "UPDATE labels SET name = ?1, color = ?2 WHERE id = ?3",
                params![name, color, id],
            )?;
            if n == 0 {
                return Err(StoreError::NotFound);
            }
            Ok(())
        })
    }

    pub fn delete_label(&self, id: i64) -> Result<()> {
        if self
            .conn
            .execute("DELETE FROM labels WHERE id = ?1", [id])?
            == 0
        {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
}
