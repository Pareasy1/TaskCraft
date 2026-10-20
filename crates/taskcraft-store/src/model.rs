use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("not found")]
    NotFound,
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKindDb {
    Event,
    Task,
}

impl ItemKindDb {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ItemKindDb::Event => "event",
            ItemKindDb::Task => "task",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub visible: bool,
    pub sort: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarInput {
    pub id: Option<i64>,
    pub name: String,
    pub color: String,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum AlertSpecUtc {
    Before(u32),
    At(DateTime<Utc>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInput {
    pub calendar_id: i64,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub tz: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub notes: Option<String>,
    pub url: Option<String>,
    pub rrule: Option<String>,
    pub alerts: Vec<AlertSpecUtc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: i64,
    #[serde(flatten)]
    pub input: EventInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub calendar_id: i64,
    pub title: String,
    pub due: Option<DateTime<Utc>>,
    pub due_has_time: bool,
    pub tz: String,
    pub rrule: Option<String>,
    pub notes: Option<String>,
    pub alerts: Vec<AlertSpecUtc>,
    #[serde(default)]
    pub status: TaskStatus,
    /// Label names; missing labels are created on save.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Task workflow state. `Done` ⇔ `completed_at` is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn as_sql(self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
        }
    }

    pub fn from_sql(s: &str) -> TaskStatus {
        match s {
            "in_progress" => TaskStatus::InProgress,
            "done" => TaskStatus::Done,
            _ => TaskStatus::Todo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskLabel {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub calendar_id: i64,
    pub title: String,
    pub due: Option<DateTime<Utc>>,
    pub due_has_time: bool,
    pub tz: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub recurring: bool,
    pub rrule: Option<String>,
    pub notes: Option<String>,
    pub color: String,
    pub alerts: Vec<AlertSpecUtc>,
    pub status: TaskStatus,
    pub labels: Vec<TaskLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum NewItem {
    Event(EventInput),
    Task(TaskInput),
}
