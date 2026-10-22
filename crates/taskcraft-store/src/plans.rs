use crate::model::*;
use crate::tasks::delete_task_row;
use crate::Store;
use rusqlite::{params, Connection, OptionalExtension};

impl Store {
    pub fn list_plans(&self) -> Result<Vec<Plan>> {
        let mut st = self.conn.prepare(
            "SELECT p.id, p.name, p.color, p.sort,
               (SELECT count(*) FROM tasks t WHERE t.plan_id = p.id AND t.completed_at IS NULL)
             FROM plans p ORDER BY p.sort, p.id",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Plan {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                sort: r.get(3)?,
                open_count: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn list_buckets(&self, plan_id: i64) -> Result<Vec<Bucket>> {
        let mut st = self.conn.prepare(
            "SELECT id, plan_id, name, sort FROM buckets WHERE plan_id = ?1 ORDER BY sort, id",
        )?;
        let rows = st.query_map([plan_id], |r| {
            Ok(Bucket {
                id: r.get(0)?,
                plan_id: r.get(1)?,
                name: r.get(2)?,
                sort: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn default_plan_id(&self) -> Result<i64> {
        let v: String = self.conn.query_row(
            "SELECT value FROM prefs WHERE key = 'default_plan_id'",
            [],
            |r| r.get(0),
        )?;
        v.parse()
            .map_err(|_| StoreError::Invalid("default_plan_id".into()))
    }

    pub fn create_plan(&self, name: &str, color: &str) -> Result<i64> {
        let name = clean_name(name)?;
        self.in_tx(|tx| {
            if plan_name_taken(tx, name, None)? {
                return Err(StoreError::Duplicate);
            }
            let sort: i64 =
                tx.query_row("SELECT coalesce(max(sort) + 1, 0) FROM plans", [], |r| {
                    r.get(0)
                })?;
            tx.execute(
                "INSERT INTO plans(name, color, sort) VALUES (?1, ?2, ?3)",
                params![name, color, sort],
            )?;
            let id = tx.last_insert_rowid();
            let bucket = pref(tx, "new_bucket_name")?.unwrap_or_else(|| "To do".into());
            tx.execute(
                "INSERT INTO buckets(plan_id, name, sort) VALUES (?1, ?2, 0)",
                params![id, bucket],
            )?;
            Ok(id)
        })
    }

    pub fn update_plan(&self, id: i64, name: Option<&str>, color: Option<&str>) -> Result<()> {
        self.in_tx(|tx| {
            if !exists(tx, "plans", id)? {
                return Err(StoreError::NotFound);
            }
            if let Some(n) = name {
                let n = clean_name(n)?;
                if plan_name_taken(tx, n, Some(id))? {
                    return Err(StoreError::Duplicate);
                }
                tx.execute("UPDATE plans SET name = ?1 WHERE id = ?2", params![n, id])?;
            }
            if let Some(c) = color {
                tx.execute("UPDATE plans SET color = ?1 WHERE id = ?2", params![c, id])?;
            }
            Ok(())
        })
    }

    pub fn reorder_plans(&self, ids: &[i64]) -> Result<()> {
        self.in_tx(|tx| reorder(tx, "plans", "1 = 1", [], ids))
    }

    pub fn delete_plan(&self, id: i64, mode: DeleteMode) -> Result<()> {
        self.in_tx(|tx| {
            if !exists(tx, "plans", id)? {
                return Err(StoreError::NotFound);
            }
            let count: i64 = tx.query_row("SELECT count(*) FROM plans", [], |r| r.get(0))?;
            if count <= 1 {
                return Err(StoreError::LastPlan);
            }
            let mut st = tx.prepare(
                "SELECT t.id FROM tasks t LEFT JOIN buckets b ON b.id = t.bucket_id
                 WHERE t.plan_id = ?1 ORDER BY b.sort, t.bucket_id, t.sort_order, t.id",
            )?;
            let task_ids: Vec<i64> = st
                .query_map([id], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            drop(st);
            match mode {
                DeleteMode::MoveTo(p) => {
                    if p == id {
                        return Err(StoreError::Invalid(
                            "target plan is the deleted plan".into(),
                        ));
                    }
                    if !exists(tx, "plans", p)? {
                        return Err(StoreError::Invalid("target plan not found".into()));
                    }
                    let bucket: i64 = tx.query_row(
                        "SELECT id FROM buckets WHERE plan_id = ?1 ORDER BY sort, id LIMIT 1",
                        [p],
                        |r| r.get(0),
                    )?;
                    move_tasks(tx, &task_ids, p, bucket)?;
                }
                DeleteMode::DeleteTasks => {
                    for t in task_ids {
                        delete_task_row(tx, t)?;
                    }
                }
            }
            tx.execute("DELETE FROM buckets WHERE plan_id = ?1", [id])?;
            tx.execute("DELETE FROM plans WHERE id = ?1", [id])?;
            if self.default_plan_id_in(tx)? == id {
                let first: i64 =
                    tx.query_row("SELECT id FROM plans ORDER BY sort, id LIMIT 1", [], |r| {
                        r.get(0)
                    })?;
                set_pref(tx, "default_plan_id", &first.to_string())?;
            }
            Ok(())
        })
    }

    pub fn create_bucket(&self, plan_id: i64, name: &str) -> Result<i64> {
        let name = clean_name(name)?;
        self.in_tx(|tx| {
            if !exists(tx, "plans", plan_id)? {
                return Err(StoreError::NotFound);
            }
            let sort: i64 = tx.query_row(
                "SELECT coalesce(max(sort) + 1, 0) FROM buckets WHERE plan_id = ?1",
                [plan_id],
                |r| r.get(0),
            )?;
            tx.execute(
                "INSERT INTO buckets(plan_id, name, sort) VALUES (?1, ?2, ?3)",
                params![plan_id, name, sort],
            )?;
            Ok(tx.last_insert_rowid())
        })
    }

    pub fn rename_bucket(&self, id: i64, name: &str) -> Result<()> {
        let name = clean_name(name)?;
        self.in_tx(|tx| {
            if tx.execute(
                "UPDATE buckets SET name = ?1 WHERE id = ?2",
                params![name, id],
            )? == 0
            {
                return Err(StoreError::NotFound);
            }
            Ok(())
        })
    }

    pub fn reorder_buckets(&self, plan_id: i64, ids: &[i64]) -> Result<()> {
        self.in_tx(|tx| reorder(tx, "buckets", "plan_id = ?1", [plan_id], ids))
    }

    pub fn delete_bucket(&self, id: i64, mode: DeleteMode) -> Result<()> {
        self.in_tx(|tx| {
            let plan_id: i64 = tx
                .query_row("SELECT plan_id FROM buckets WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .optional()?
                .ok_or(StoreError::NotFound)?;
            let count: i64 = tx.query_row(
                "SELECT count(*) FROM buckets WHERE plan_id = ?1",
                [plan_id],
                |r| r.get(0),
            )?;
            if count <= 1 {
                return Err(StoreError::LastBucket);
            }
            let mut st =
                tx.prepare("SELECT id FROM tasks WHERE bucket_id = ?1 ORDER BY sort_order, id")?;
            let task_ids: Vec<i64> = st
                .query_map([id], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            drop(st);
            match mode {
                DeleteMode::MoveTo(b) => {
                    let target_plan: Option<i64> = tx
                        .query_row("SELECT plan_id FROM buckets WHERE id = ?1", [b], |r| {
                            r.get(0)
                        })
                        .optional()?;
                    if b == id || target_plan != Some(plan_id) {
                        return Err(StoreError::Invalid(
                            "target bucket must be another bucket of the same plan".into(),
                        ));
                    }
                    move_tasks(tx, &task_ids, plan_id, b)?;
                }
                DeleteMode::DeleteTasks => {
                    for t in task_ids {
                        delete_task_row(tx, t)?;
                    }
                }
            }
            tx.execute("DELETE FROM buckets WHERE id = ?1", [id])?;
            Ok(())
        })
    }

    pub fn set_default_plan(&self, id: i64) -> Result<()> {
        self.in_tx(|tx| {
            if !exists(tx, "plans", id)? {
                return Err(StoreError::NotFound);
            }
            set_pref(tx, "default_plan_id", &id.to_string())
        })
    }

    /// Renames the untouched seed plan/bucket to localized names, once.
    /// Returns whether anything was renamed.
    pub fn localize_seed(&self, plan_name: &str, bucket_name: &str) -> Result<bool> {
        self.in_tx(|tx| {
            if pref(tx, "seed_names_pending")?.as_deref() != Some("1") {
                return Ok(false);
            }
            let plan_name = clean_name(plan_name)?;
            let bucket_name = clean_name(bucket_name)?;
            let mut renamed = 0;
            // Skip the plan rename if it would collide with another plan.
            if !plan_name_taken(tx, plan_name, Some(1))? {
                renamed += tx.execute(
                    "UPDATE plans SET name = ?1 WHERE id = 1 AND name = 'My tasks' AND name <> ?1",
                    [plan_name],
                )?;
            }
            renamed += tx.execute(
                "UPDATE buckets SET name = ?1 WHERE id = 1 AND name = 'To do' AND name <> ?1",
                [bucket_name],
            )?;
            set_pref(tx, "new_bucket_name", bucket_name)?;
            set_pref(tx, "seed_names_pending", "0")?;
            Ok(renamed > 0)
        })
    }

    pub(crate) fn default_plan_id_in(&self, conn: &Connection) -> Result<i64> {
        Ok(pref(conn, "default_plan_id")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(-1))
    }
}

/// Next `sort_order` that places a task after all others in the bucket.
pub(crate) fn next_sort(conn: &Connection, bucket_id: i64) -> Result<f64> {
    Ok(conn.query_row(
        "SELECT coalesce(max(sort_order), 0.0) + 1.0 FROM tasks WHERE bucket_id = ?1",
        [bucket_id],
        |r| r.get(0),
    )?)
}

fn move_tasks(conn: &Connection, task_ids: &[i64], plan_id: i64, bucket_id: i64) -> Result<()> {
    let base = next_sort(conn, bucket_id)?;
    for (i, t) in task_ids.iter().enumerate() {
        conn.execute(
            "UPDATE tasks SET plan_id = ?1, bucket_id = ?2, sort_order = ?3 WHERE id = ?4",
            params![plan_id, bucket_id, base + i as f64, t],
        )?;
    }
    Ok(())
}

fn clean_name(name: &str) -> Result<&str> {
    let n = name.trim();
    if n.is_empty() {
        return Err(StoreError::Invalid("name is empty".into()));
    }
    Ok(n)
}

fn plan_name_taken(conn: &Connection, name: &str, except: Option<i64>) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM plans WHERE name = ?1 COLLATE NOCASE AND id != ?2)",
        params![name, except.unwrap_or(-1)],
        |r| r.get(0),
    )?)
}

fn exists(conn: &Connection, table: &str, id: i64) -> Result<bool> {
    Ok(conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)"),
        [id],
        |r| r.get(0),
    )?)
}

fn pref(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM prefs WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

fn set_pref(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO prefs(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Rewrites `sort` of the rows matching `scope` so they follow `ids`; `ids`
/// must be exactly the matching set. `scope` has at most one bound param.
fn reorder<P: rusqlite::Params>(
    conn: &Connection,
    table: &str,
    scope: &str,
    scope_params: P,
    ids: &[i64],
) -> Result<()> {
    let mut st = conn.prepare(&format!("SELECT id FROM {table} WHERE {scope}"))?;
    let mut have: Vec<i64> = st
        .query_map(scope_params, |r| r.get(0))?
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
        conn.execute(
            &format!("UPDATE {table} SET sort = ?1 WHERE id = ?2"),
            params![i as i64, id],
        )?;
    }
    Ok(())
}

/// First bucket of a plan by sort order.
pub(crate) fn first_bucket(conn: &Connection, plan_id: i64) -> Result<i64> {
    conn.query_row(
        "SELECT id FROM buckets WHERE plan_id = ?1 ORDER BY sort, id LIMIT 1",
        [plan_id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or(StoreError::NotFound)
}
