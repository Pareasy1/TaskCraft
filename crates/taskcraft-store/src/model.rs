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
    #[error("already exists")]
    Duplicate,
    #[error("cannot delete the last plan")]
    LastPlan,
    #[error("cannot delete the last bucket")]
    LastBucket,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Rule(Rule),
}

/// Business rules the store enforces on subtasks, dates and checklists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    ParentIsSubtask,
    HasSubtasks,
    RepeatingSubtask,
    StartAfterDue,
    ChecklistText,
    AttachmentTooLarge,
    AttachmentUrl,
    TemplateName,
}

impl Rule {
    pub fn code(self) -> &'static str {
        match self {
            Rule::ParentIsSubtask => "parent_is_subtask",
            Rule::HasSubtasks => "has_subtasks",
            Rule::RepeatingSubtask => "repeating_subtask",
            Rule::StartAfterDue => "start_after_due",
            Rule::ChecklistText => "checklist_text",
            Rule::AttachmentTooLarge => "attachment_too_large",
            Rule::AttachmentUrl => "attachment_url",
            Rule::TemplateName => "template_name",
        }
    }
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Rule::ParentIsSubtask => "a subtask cannot have subtasks",
            Rule::HasSubtasks => "a task with subtasks cannot become a subtask",
            Rule::RepeatingSubtask => "a subtask cannot repeat",
            Rule::StartAfterDue => "start date is after the due date",
            Rule::ChecklistText => "checklist item text is empty or too long",
            Rule::AttachmentTooLarge => "the file is larger than 100 MiB",
            Rule::AttachmentUrl => "only http and https links can be attached",
            Rule::TemplateName => "template name is empty or too long",
        })
    }
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
    #[serde(default)]
    pub plan_id: Option<i64>,
    #[serde(default)]
    pub bucket_id: Option<i64>,
    #[serde(default)]
    pub parent_id: Option<i64>,
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub start: Option<DateTime<Utc>>,
    #[serde(default)]
    pub start_has_time: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Priority {
    Urgent,
    Important,
    #[default]
    Medium,
    Low,
}

impl Priority {
    pub fn as_sql(self) -> &'static str {
        match self {
            Priority::Urgent => "urgent",
            Priority::Important => "important",
            Priority::Medium => "medium",
            Priority::Low => "low",
        }
    }

    pub fn from_sql(s: &str) -> Priority {
        match s {
            "urgent" => Priority::Urgent,
            "important" => Priority::Important,
            "low" => Priority::Low,
            _ => Priority::Medium,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub plan_id: i64,
    pub bucket_id: i64,
    pub sort_order: f64,
    pub parent_id: Option<i64>,
    pub priority: Priority,
    pub start: Option<DateTime<Utc>>,
    pub start_has_time: bool,
    pub subtask_open: i64,
    pub subtask_total: i64,
    pub checklist_done: i64,
    pub checklist_total: i64,
    pub attachment_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub sort: i64,
    pub open_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub id: i64,
    pub plan_id: i64,
    pub name: String,
    pub sort: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeleteMode {
    MoveTo(i64),
    DeleteTasks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum NewItem {
    Event(EventInput),
    Task(TaskInput),
}
