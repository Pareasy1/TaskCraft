//! Natural-language parser for TaskCraft quick entry.

mod types;

pub use types::*;

use chrono::DateTime;
use chrono_tz::Tz;

/// Parse a quick-entry sentence. Never panics, never fails: in the worst case
/// the whole input becomes the title.
pub fn parse(input: &str, now: DateTime<Tz>, _opts: &ParseOptions) -> ParsedItem {
    let trimmed = input.trim();
    let start = input.len() - input.trim_start().len();
    let spans = if trimmed.is_empty() {
        vec![]
    } else {
        vec![Span { start, end: start + trimmed.len(), label: Label::Title }]
    };
    ParsedItem {
        kind: ItemKind::Event,
        title: trimmed.to_string(),
        start: Some(When { date: now.date_naive(), time: None }),
        end: None,
        all_day: true,
        location: None,
        attendees: vec![],
        calendar: None,
        recurrence: None,
        alerts: vec![],
        url: None,
        spans,
    }
}
