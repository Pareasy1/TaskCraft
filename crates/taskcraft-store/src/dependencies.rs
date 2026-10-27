use crate::model::{Dependency, Result, Rule, StoreError};
use crate::Store;
use chrono::{SecondsFormat, Utc};
use rusqlite::params;
use std::collections::HashSet;

impl Store {
    /// Makes `task_id` depend on `depends_on`. Refuses self-dependencies and
    /// cycles; an existing pair is a no-op.
    pub fn add_dependency(&self, task_id: i64, depends_on: i64) -> Result<()> {
        self.in_tx(|tx| {
            for id in [task_id, depends_on] {
                let n: i64 =
                    tx.query_row("SELECT COUNT(*) FROM tasks WHERE id = ?1", [id], |r| r.get(0))?;
                if n == 0 {
                    return Err(StoreError::NotFound);
                }
            }
            if task_id == depends_on {
                return Err(StoreError::Rule(Rule::DependencySelf));
            }
            // Cycle if `depends_on` already reaches `task_id`.
            let mut st = tx.prepare("SELECT depends_on FROM dependencies WHERE task_id = ?1")?;
            let mut seen: HashSet<i64> = HashSet::new();
            let mut stack = vec![depends_on];
            while let Some(n) = stack.pop() {
                if n == task_id {
                    return Err(StoreError::Rule(Rule::DependencyCycle));
                }
                if !seen.insert(n) {
                    continue;
                }
                let next: Vec<i64> = st
                    .query_map([n], |r| r.get(0))?
                    .collect::<std::result::Result<_, _>>()?;
                stack.extend(next);
            }
            drop(st);
            tx.execute(
                "INSERT OR IGNORE INTO dependencies(task_id, depends_on, created_at) VALUES (?1, ?2, ?3)",
                params![
                    task_id,
                    depends_on,
                    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
                ],
            )?;
            Ok(())
        })
    }

    pub fn remove_dependency(&self, task_id: i64, depends_on: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM dependencies WHERE task_id = ?1 AND depends_on = ?2",
            params![task_id, depends_on],
        )?;
        Ok(())
    }

    /// Rows where either side is in `task_ids` (all rows for `None`).
    pub fn list_dependencies(&self, task_ids: Option<&[i64]>) -> Result<Vec<Dependency>> {
        let mut st = self
            .conn
            .prepare("SELECT task_id, depends_on FROM dependencies ORDER BY task_id, depends_on")?;
        let all = st
            .query_map([], |r| {
                Ok(Dependency {
                    task_id: r.get(0)?,
                    depends_on: r.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(match task_ids {
            None => all,
            Some(ids) => all
                .into_iter()
                .filter(|d| ids.contains(&d.task_id) || ids.contains(&d.depends_on))
                .collect(),
        })
    }
}
