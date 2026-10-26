//! Task templates: a task's shape saved as JSON, with subtask due dates
//! stored as day offsets from the parent's due date.

use crate::labels::normalize_labels;
use crate::model::*;
use crate::{fmt_ts, name_key, Store};
use chrono::{DateTime, Days, NaiveTime, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const MAX_NAME_CHARS: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSubtask {
    pub title: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub checklist: Vec<String>,
    #[serde(default)]
    pub due_offset_days: Option<i64>,
    /// Local "HH:MM"; absent for a date-only due.
    #[serde(default)]
    pub due_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBody {
    pub title: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub checklist: Vec<String>,
    #[serde(default)]
    pub subtasks: Vec<TemplateSubtask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: i64,
    pub name: String,
    pub subtask_count: i64,
}

fn clean_name(name: &str) -> Result<&str> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > MAX_NAME_CHARS {
        return Err(StoreError::Rule(Rule::TemplateName));
    }
    Ok(n)
}

fn name_taken(conn: &Connection, name: &str, except: Option<i64>) -> Result<Option<i64>> {
    let key = name_key(name);
    let mut st = conn.prepare("SELECT id, name FROM templates")?;
    let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (id, n) = row?;
        if Some(id) != except && name_key(&n) == key {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

fn parse_tz(tz: &str) -> Tz {
    tz.parse().unwrap_or(chrono_tz::UTC)
}

fn local_date(t: DateTime<Utc>, tz: Tz) -> chrono::NaiveDate {
    t.with_timezone(&tz).date_naive()
}

fn label_names(t: &Task) -> Vec<String> {
    t.labels.iter().map(|l| l.name.clone()).collect()
}

impl Store {
    fn checklist_texts(&self, id: i64) -> Result<Vec<String>> {
        Ok(self
            .list_checklist(id)?
            .into_iter()
            .map(|c| c.text)
            .collect())
    }

    /// The parsed body of a template.
    pub fn template_body(&self, id: i64) -> Result<TemplateBody> {
        let json: String = self
            .conn
            .query_row("SELECT body FROM templates WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or(StoreError::NotFound)?;
        serde_json::from_str(&json).map_err(|e| StoreError::Invalid(e.to_string()))
    }

    /// Saves the task's shape as a template. Dates, ids and attachments are
    /// not kept; subtask dues become day offsets from the parent's due.
    pub fn save_template(&self, task_id: i64, name: &str, replace: bool) -> Result<i64> {
        let name = clean_name(name)?;
        self.in_tx(|tx| {
            let task = self.get_task(task_id)?;
            if task.parent_id.is_some() {
                return Err(StoreError::Invalid(
                    "a subtask cannot be saved as a template".into(),
                ));
            }
            let ptz = parse_tz(&task.tz);
            let pdate = task.due.map(|d| local_date(d, ptz));
            let mut subtasks = Vec::new();
            for s in self.list_subtasks(task_id)? {
                let stz = parse_tz(&s.tz);
                let (off, time) = match (pdate, s.due) {
                    (Some(p), Some(d)) => {
                        let local = d.with_timezone(&stz);
                        let off = (local.date_naive() - p).num_days();
                        let time = s.due_has_time.then(|| local.format("%H:%M").to_string());
                        (Some(off), time)
                    }
                    _ => (None, None),
                };
                subtasks.push(TemplateSubtask {
                    title: s.title.clone(),
                    notes: s.notes.clone(),
                    labels: label_names(&s),
                    priority: s.priority,
                    checklist: self.checklist_texts(s.id)?,
                    due_offset_days: off,
                    due_time: time,
                });
            }
            let body = TemplateBody {
                title: task.title.clone(),
                notes: task.notes.clone(),
                labels: label_names(&task),
                priority: task.priority,
                checklist: self.checklist_texts(task_id)?,
                subtasks,
            };
            let json =
                serde_json::to_string(&body).map_err(|e| StoreError::Invalid(e.to_string()))?;
            if let Some(existing) = name_taken(tx, name, None)? {
                if !replace {
                    return Err(StoreError::Duplicate);
                }
                tx.execute(
                    "UPDATE templates SET body = ?1 WHERE id = ?2",
                    params![json, existing],
                )?;
                return Ok(existing);
            }
            tx.execute(
                "INSERT INTO templates(name, body, created_at) VALUES (?1, ?2, ?3)",
                params![name, json, fmt_ts(Utc::now())],
            )?;
            Ok(tx.last_insert_rowid())
        })
    }

    fn template_infos(&self) -> Result<Vec<TemplateInfo>> {
        let mut st = self
            .conn
            .prepare("SELECT id, name, body FROM templates ORDER BY name COLLATE NOCASE, id")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, name, body) = row?;
            let subtask_count = serde_json::from_str::<TemplateBody>(&body)
                .map(|b| b.subtasks.len() as i64)
                .unwrap_or(0);
            out.push(TemplateInfo {
                id,
                name,
                subtask_count,
            });
        }
        Ok(out)
    }

    pub fn list_templates(&self) -> Result<Vec<TemplateInfo>> {
        self.template_infos()
    }

    /// Finds a template by name (Unicode case-insensitive, trimmed).
    pub fn template_by_name(&self, name: &str) -> Result<Option<TemplateInfo>> {
        let key = name_key(name.trim());
        Ok(self
            .template_infos()?
            .into_iter()
            .find(|t| name_key(&t.name) == key))
    }

    pub fn rename_template(&self, id: i64, name: &str) -> Result<()> {
        let name = clean_name(name)?;
        self.in_tx(|tx| {
            if name_taken(tx, name, Some(id))?.is_some() {
                return Err(StoreError::Duplicate);
            }
            let n = tx.execute(
                "UPDATE templates SET name = ?1 WHERE id = ?2",
                params![name, id],
            )?;
            if n == 0 {
                return Err(StoreError::NotFound);
            }
            Ok(())
        })
    }

    pub fn delete_template(&self, id: i64) -> Result<()> {
        let n = self
            .conn
            .execute("DELETE FROM templates WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// Creates a task from `input`, filling notes, labels and priority from
    /// the template where the input left them empty, then its checklist and
    /// subtasks (due dates relative to the new task's local due date).
    pub fn create_from_template(&self, template_id: i64, input: &TaskInput) -> Result<i64> {
        let body = self.template_body(template_id)?;
        self.in_tx(|_| {
            let mut t = input.clone();
            if t.notes.as_deref().is_none_or(|n| n.trim().is_empty()) {
                t.notes = body.notes.clone();
            }
            if t.priority == Priority::Medium {
                t.priority = body.priority;
            }
            t.labels = normalize_labels(
                &input
                    .labels
                    .iter()
                    .chain(body.labels.iter())
                    .cloned()
                    .collect::<Vec<_>>(),
            );
            t.status = TaskStatus::Todo;
            let id = self.insert_task(&t)?;
            for text in &body.checklist {
                self.add_checklist_item(id, text)?;
            }
            let tz = parse_tz(&t.tz);
            let base = t.due.map(|d| local_date(d, tz));
            for s in &body.subtasks {
                let (due, has_time) = match (base, s.due_offset_days) {
                    (Some(b), Some(off)) => {
                        let date = if off >= 0 {
                            b.checked_add_days(Days::new(off as u64))
                        } else {
                            b.checked_sub_days(Days::new(off.unsigned_abs()))
                        };
                        let time = s
                            .due_time
                            .as_deref()
                            .and_then(|h| NaiveTime::parse_from_str(h, "%H:%M").ok());
                        date.map_or((None, false), |d| {
                            let ndt = d.and_time(time.unwrap_or(NaiveTime::MIN));
                            (Some(crate::time::local_to_utc(ndt, tz)), time.is_some())
                        })
                    }
                    _ => (None, false),
                };
                let sid = self.insert_task(&TaskInput {
                    calendar_id: t.calendar_id,
                    title: s.title.clone(),
                    due,
                    due_has_time: has_time,
                    tz: t.tz.clone(),
                    notes: s.notes.clone(),
                    labels: s.labels.clone(),
                    parent_id: Some(id),
                    priority: s.priority,
                    ..Default::default()
                })?;
                for text in &s.checklist {
                    self.add_checklist_item(sid, text)?;
                }
            }
            Ok(id)
        })
    }
}
