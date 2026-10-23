use chrono::{NaiveDate, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DateOrder {
    DayMonth,
    MonthDay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseOptions {
    pub default_event_minutes: u32,
    pub calendars: Vec<String>,
    pub date_order: DateOrder,
    /// Plan names that `/plan` quick-entry hints can route a task to.
    #[serde(default)]
    pub plans: Vec<String>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            default_event_minutes: 60,
            calendars: vec!["Personal".into(), "Work".into()],
            date_order: DateOrder::DayMonth,
            plans: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Event,
    Task,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct When {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Label {
    Title,
    TaskPrefix,
    Date,
    Time,
    Duration,
    Recurrence,
    Alert,
    Location,
    Attendees,
    Calendar,
    Url,
    Separator,
    /// `#word` on a task.
    Tag,
    Plan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub label: Label,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "freq")]
pub enum Recurrence {
    Daily {
        interval: u32,
    },
    Weekly {
        interval: u32,
        days: Vec<Weekday>,
    },
    Monthly {
        interval: u32,
        month_day: Option<u32>,
    },
    Yearly {
        interval: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceSpec {
    pub rule: Recurrence,
    pub until: Option<NaiveDate>,
}

impl RecurrenceSpec {
    /// RRULE body, e.g. `FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,TU`.
    pub fn to_rrule(&self) -> String {
        let mut s = match &self.rule {
            Recurrence::Daily { interval } => format!("FREQ=DAILY;INTERVAL={interval}"),
            Recurrence::Weekly { interval, days } => {
                let mut s = format!("FREQ=WEEKLY;INTERVAL={interval}");
                if !days.is_empty() {
                    let d: Vec<&str> = days.iter().map(|d| weekday_code(*d)).collect();
                    s.push_str(&format!(";BYDAY={}", d.join(",")));
                }
                s
            }
            Recurrence::Monthly {
                interval,
                month_day,
            } => {
                let mut s = format!("FREQ=MONTHLY;INTERVAL={interval}");
                if let Some(md) = month_day {
                    s.push_str(&format!(";BYMONTHDAY={md}"));
                }
                s
            }
            Recurrence::Yearly { interval } => format!("FREQ=YEARLY;INTERVAL={interval}"),
        };
        if let Some(u) = self.until {
            s.push_str(&format!(";UNTIL={}", u.format("%Y%m%d")));
        }
        s
    }
}

fn weekday_code(d: Weekday) -> &'static str {
    match d {
        Weekday::Mon => "MO",
        Weekday::Tue => "TU",
        Weekday::Wed => "WE",
        Weekday::Thu => "TH",
        Weekday::Fri => "FR",
        Weekday::Sat => "SA",
        Weekday::Sun => "SU",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum AlertSpec {
    Before(u32),
    At(When),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedItem {
    pub kind: ItemKind,
    pub title: String,
    pub start: Option<When>,
    pub end: Option<When>,
    pub all_day: bool,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub calendar: Option<String>,
    pub recurrence: Option<RecurrenceSpec>,
    pub alerts: Vec<AlertSpec>,
    pub url: Option<String>,
    /// Label names from `#word` tokens (tasks only), without the `#`.
    pub tags: Vec<String>,
    pub plan: Option<String>,
    pub spans: Vec<Span>,
}
