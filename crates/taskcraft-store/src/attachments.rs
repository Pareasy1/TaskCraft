//! Task attachments: files copied into the data folder, and web links.
//! Removed files are queued in `pending_file_deletes` and purged later so
//! Undo can still find them.

use crate::model::*;
use crate::{fmt_ts, parse_ts, Store};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// 100 MiB.
pub(crate) const MAX_ATTACHMENT_BYTES: u64 = 104_857_600;
const MAX_NAME: usize = 120;
/// How long a removed file stays on disk (longer than the Undo toast).
const PENDING_DELAY_SECS: i64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttachmentKind {
    File,
    Link,
}

impl AttachmentKind {
    fn as_str(self) -> &'static str {
        match self {
            AttachmentKind::File => "file",
            AttachmentKind::Link => "link",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: i64,
    pub task_id: i64,
    pub kind: AttachmentKind,
    pub name: String,
    pub stored: Option<String>,
    pub url: Option<String>,
    pub size: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub sort: f64,
}

const COLS: &str = "id, task_id, kind, name, stored, url, size, created_at, sort";

fn from_row(r: &Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: r.get(0)?,
        task_id: r.get(1)?,
        kind: if r.get::<_, String>(2)? == "link" {
            AttachmentKind::Link
        } else {
            AttachmentKind::File
        },
        name: r.get(3)?,
        stored: r.get(4)?,
        url: r.get(5)?,
        size: r.get(6)?,
        created_at: parse_ts(&r.get::<_, String>(7)?),
        sort: r.get(8)?,
    })
}

/// Keeps letters, digits, `.`, `-`, `_` and spaces; anything else becomes
/// `_`. Only the last path component is used; the result is 1–120 chars and
/// never just dots.
pub fn sanitize_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let mut s: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .take(MAX_NAME)
        .collect();
    if s.chars().all(|c| c == '.' || c == ' ') {
        s = "_".into();
    }
    s
}

pub(crate) fn queue_file(conn: &Connection, stored: &str, now: DateTime<Utc>) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO pending_file_deletes(stored, delete_after_utc) VALUES (?1, ?2)",
        params![stored, fmt_ts(now + Duration::seconds(PENDING_DELAY_SECS))],
    )?;
    Ok(())
}

/// Deletes a task's attachment rows and queues its stored files.
pub(crate) fn delete_rows_for_task(
    conn: &Connection,
    task_id: i64,
    now: DateTime<Utc>,
) -> Result<()> {
    let mut st = conn.prepare("SELECT stored FROM attachments WHERE task_id = ?1")?;
    let stored: Vec<Option<String>> = st
        .query_map([task_id], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    drop(st);
    for s in stored.into_iter().flatten() {
        queue_file(conn, &s, now)?;
    }
    conn.execute("DELETE FROM attachments WHERE task_id = ?1", [task_id])?;
    Ok(())
}

fn require_task(conn: &Connection, task_id: i64) -> Result<()> {
    conn.query_row("SELECT 1 FROM tasks WHERE id = ?1", [task_id], |_| Ok(()))
        .optional()?
        .ok_or(StoreError::NotFound)
}

fn next_sort(conn: &Connection, task_id: i64) -> Result<f64> {
    let max: Option<f64> = conn.query_row(
        "SELECT MAX(sort) FROM attachments WHERE task_id = ?1",
        [task_id],
        |r| r.get(0),
    )?;
    Ok(max.unwrap_or(0.0) + 1.0)
}

fn get(conn: &Connection, id: i64) -> Result<Attachment> {
    conn.query_row(
        &format!("SELECT {COLS} FROM attachments WHERE id = ?1"),
        [id],
        from_row,
    )
    .optional()?
    .ok_or(StoreError::NotFound)
}

fn insert(conn: &Connection, a: &Attachment) -> Result<Attachment> {
    conn.execute(
        "INSERT INTO attachments(task_id, kind, name, stored, url, size, created_at, sort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            a.task_id,
            a.kind.as_str(),
            a.name,
            a.stored,
            a.url,
            a.size,
            fmt_ts(a.created_at),
            a.sort
        ],
    )?;
    get(conn, conn.last_insert_rowid())
}

fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > MAX_NAME {
        return Err(StoreError::Invalid(
            "attachment name must be 1-120 characters".into(),
        ));
    }
    Ok(n.to_string())
}

/// A stored path must be relative, under `attachments/`, with plain components.
fn check_stored_shape(stored: &str) -> Result<()> {
    let p = Path::new(stored);
    let plain = p.components().all(|c| matches!(c, Component::Normal(_)));
    if !plain || !p.starts_with("attachments") || p.components().count() < 2 {
        return Err(StoreError::Invalid(
            "attachment path is outside the data folder".into(),
        ));
    }
    Ok(())
}

/// Returns the host of an http(s) URL, or None if it is not one.
fn http_host(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let auth = rest.split(['/', '?', '#']).next().unwrap_or("");
    let auth = auth.rsplit('@').next().unwrap_or("");
    let host = if let Some(v6) = auth.strip_prefix('[') {
        v6.split(']').next().unwrap_or("")
    } else {
        auth.split(':').next().unwrap_or("")
    };
    (!host.is_empty()).then(|| host.to_string())
}

impl Store {
    pub fn add_file_attachment(
        &self,
        task_id: i64,
        src: &Path,
        data_dir: &Path,
    ) -> Result<Attachment> {
        self.add_file_attachment_with_limit(task_id, src, data_dir, MAX_ATTACHMENT_BYTES)
    }

    /// Test hook: same as `add_file_attachment` with a custom size limit.
    #[doc(hidden)]
    pub fn add_file_attachment_with_limit(
        &self,
        task_id: i64,
        src: &Path,
        data_dir: &Path,
        limit: u64,
    ) -> Result<Attachment> {
        require_task(&self.conn, task_id)?;
        let meta = std::fs::metadata(src).map_err(|_| StoreError::NotFound)?;
        if !meta.is_file() {
            return Err(StoreError::NotFound);
        }
        if meta.len() > limit {
            return Err(StoreError::Rule(Rule::AttachmentTooLarge));
        }
        let raw = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let safe = sanitize_name(&raw);
        let display: String = raw.trim().chars().take(MAX_NAME).collect();
        let display = if display.is_empty() {
            safe.clone()
        } else {
            display
        };

        let rel_dir = format!("attachments/{task_id}");
        let dir = data_dir.join(&rel_dir);
        std::fs::create_dir_all(&dir)?;
        let mut n = 1;
        let (rel, full) = loop {
            let file = format!("{n}-{safe}");
            let full = dir.join(&file);
            if !full.exists() {
                break (format!("{rel_dir}/{file}"), full);
            }
            n += 1;
        };
        let tmp = dir.join(format!(".{n}-incoming.tmp"));
        let copied = std::fs::copy(src, &tmp).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            StoreError::from(e)
        });
        let size = match copied {
            // The file may have grown between stat and copy.
            Ok(sz) if sz > limit => {
                let _ = std::fs::remove_file(&tmp);
                return Err(StoreError::Rule(Rule::AttachmentTooLarge));
            }
            Ok(sz) => sz,
            Err(e) => return Err(e),
        };
        if let Err(e) = std::fs::rename(&tmp, &full) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
        let res = self.in_tx(|tx| {
            // A queued file at the same path (never reused: names are unique
            // on disk) would be a stale entry; drop it defensively.
            tx.execute("DELETE FROM pending_file_deletes WHERE stored = ?1", [&rel])?;
            insert(
                tx,
                &Attachment {
                    id: 0,
                    task_id,
                    kind: AttachmentKind::File,
                    name: display,
                    stored: Some(rel.clone()),
                    url: None,
                    size: Some(size as i64),
                    created_at: Utc::now(),
                    sort: next_sort(tx, task_id)?,
                },
            )
        });
        if res.is_err() {
            let _ = std::fs::remove_file(&full);
        }
        res
    }

    pub fn add_link_attachment(
        &self,
        task_id: i64,
        url: &str,
        name: Option<&str>,
    ) -> Result<Attachment> {
        let url = url.trim();
        let host = http_host(url).ok_or(StoreError::Rule(Rule::AttachmentUrl))?;
        let name = match name.map(str::trim).filter(|n| !n.is_empty()) {
            Some(n) => clean_name(n)?,
            None => host.chars().take(MAX_NAME).collect(),
        };
        self.in_tx(|tx| {
            require_task(tx, task_id)?;
            insert(
                tx,
                &Attachment {
                    id: 0,
                    task_id,
                    kind: AttachmentKind::Link,
                    name,
                    stored: None,
                    url: Some(url.to_string()),
                    size: None,
                    created_at: Utc::now(),
                    sort: next_sort(tx, task_id)?,
                },
            )
        })
    }

    pub fn list_attachments(&self, task_id: i64) -> Result<Vec<Attachment>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {COLS} FROM attachments WHERE task_id = ?1 ORDER BY sort, id"
        ))?;
        let rows = st
            .query_map([task_id], from_row)?
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows)
    }

    pub fn rename_attachment(&self, id: i64, name: &str) -> Result<()> {
        let name = clean_name(name)?;
        let n = self.conn.execute(
            "UPDATE attachments SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// `ids` must be exactly the task's attachments; renumbers 1.0, 2.0, ...
    pub fn reorder_attachments(&self, task_id: i64, ids: &[i64]) -> Result<()> {
        self.in_tx(|tx| {
            require_task(tx, task_id)?;
            let mut have: Vec<i64> = {
                let mut st = tx.prepare("SELECT id FROM attachments WHERE task_id = ?1")?;
                let rows = st.query_map([task_id], |r| r.get(0))?;
                rows.collect::<std::result::Result<_, _>>()?
            };
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
                    "UPDATE attachments SET sort = ?1 WHERE id = ?2",
                    params![(i + 1) as f64, id],
                )?;
            }
            Ok(())
        })
    }

    /// Deletes the row and queues its file; returns the removed row (for Undo).
    pub fn remove_attachment(&self, id: i64, now: DateTime<Utc>) -> Result<Attachment> {
        self.in_tx(|tx| {
            let a = get(tx, id)?;
            tx.execute("DELETE FROM attachments WHERE id = ?1", [id])?;
            if let Some(s) = &a.stored {
                queue_file(tx, s, now)?;
            }
            Ok(a)
        })
    }

    /// Re-links a file that is still queued for deletion (Undo). A file that
    /// is not queued has already been purged (or never existed): `NotFound`.
    /// The file may live under another task's folder (a deleted task that is
    /// restored gets a new id).
    pub fn restore_file_attachment(
        &self,
        task_id: i64,
        name: &str,
        stored: &str,
        size: i64,
    ) -> Result<Attachment> {
        check_stored_shape(stored)?;
        let name = clean_name(name)?;
        self.in_tx(|tx| {
            require_task(tx, task_id)?;
            let n = tx.execute(
                "DELETE FROM pending_file_deletes WHERE stored = ?1",
                [stored],
            )?;
            if n == 0 {
                return Err(StoreError::NotFound);
            }
            insert(
                tx,
                &Attachment {
                    id: 0,
                    task_id,
                    kind: AttachmentKind::File,
                    name,
                    stored: Some(stored.to_string()),
                    url: None,
                    size: Some(size),
                    created_at: Utc::now(),
                    sort: next_sort(tx, task_id)?,
                },
            )
        })
    }

    /// Deletes queued files whose delay has passed; returns how many.
    pub fn purge_pending_files(&self, now: DateTime<Utc>, data_dir: &Path) -> Result<usize> {
        let due: Vec<String> = {
            let mut st = self
                .conn
                .prepare("SELECT stored FROM pending_file_deletes WHERE delete_after_utc <= ?1")?;
            let rows = st.query_map([fmt_ts(now)], |r| r.get(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let mut n = 0;
        for stored in due {
            if check_stored_shape(&stored).is_ok() {
                let full = data_dir.join(&stored);
                match std::fs::remove_file(&full) {
                    Ok(()) => n += 1,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => continue, // retry on the next tick
                }
                if let Some(parent) = full.parent() {
                    let _ = std::fs::remove_dir(parent); // only if empty
                }
            }
            self.conn.execute(
                "DELETE FROM pending_file_deletes WHERE stored = ?1",
                [&stored],
            )?;
        }
        Ok(n)
    }

    /// Canonical path of a stored file; refuses anything outside
    /// `<data_dir>/attachments`.
    pub fn attachment_path(&self, id: i64, data_dir: &Path) -> Result<PathBuf> {
        let a = get(&self.conn, id)?;
        let stored = a
            .stored
            .ok_or_else(|| StoreError::Invalid("attachment is not a file".into()))?;
        check_stored_shape(&stored)?;
        let root = data_dir
            .join("attachments")
            .canonicalize()
            .map_err(|_| StoreError::NotFound)?;
        let full = data_dir
            .join(&stored)
            .canonicalize()
            .map_err(|_| StoreError::NotFound)?;
        if !full.starts_with(&root) {
            return Err(StoreError::Invalid(
                "attachment path is outside the data folder".into(),
            ));
        }
        Ok(full)
    }
}
