//! Natural-language parser for TaskCraft quick entry.

mod rules;
mod token;
mod types;

pub use types::*;

use chrono::DateTime;
use chrono_tz::Tz;
use rules::{Ctx, Payload, Tagged};
use token::{TokKind, Token};

/// Parse a quick-entry sentence. Never panics, never fails: in the worst case
/// the whole input becomes the title.
pub fn parse(input: &str, now: DateTime<Tz>, opts: &ParseOptions) -> ParsedItem {
    let lead_ws = input.len() - input.trim_start().len();
    let body = &input[lead_ws..];
    let mut spans = Vec::new();
    let mut kind = ItemKind::Event;
    let mut offset = lead_ws;
    if let Some(len) = rules::prefix::task_prefix_len(body) {
        kind = ItemKind::Task;
        spans.push(Span { start: lead_ws, end: lead_ws + len, label: Label::TaskPrefix });
        offset += len;
    }

    let toks = token::tokenize(&input[offset..], offset);
    let mut tags: Vec<Tagged> = toks.iter().map(|_| Tagged { label: None, payload: Payload::None }).collect();
    let ctx = Ctx { now, opts };
    rules::run_rules(&toks, &[], &ctx, &mut tags);

    let title = assemble_title(&toks, &mut tags);
    spans.extend(collect_spans(&toks, &tags));

    ParsedItem {
        kind,
        title,
        start: match kind {
            ItemKind::Event => Some(When { date: now.date_naive(), time: None }),
            ItemKind::Task => None,
        },
        end: None,
        all_day: kind == ItemKind::Event,
        location: None,
        attendees: vec![],
        calendar: None,
        recurrence: None,
        alerts: vec![],
        url: None,
        spans,
    }
}

/// Untagged tokens become the title; they are labeled `Title` in place.
fn assemble_title(toks: &[Token], tags: &mut [Tagged]) -> String {
    let mut title = String::new();
    for (t, tag) in toks.iter().zip(tags.iter_mut()) {
        if tag.label.is_some() {
            continue;
        }
        tag.label = Some(Label::Title);
        let attach = t.kind == TokKind::Punct && matches!(t.text.as_str(), "," | "." | "!" | "?" | ";" | ":" | ")");
        if !title.is_empty() && !attach {
            title.push(' ');
        }
        title.push_str(&t.text);
    }
    title
}

/// Merges consecutive tokens with the same label into spans.
fn collect_spans(toks: &[Token], tags: &[Tagged]) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    for (t, tag) in toks.iter().zip(tags) {
        let Some(label) = tag.label else { continue };
        match spans.last_mut() {
            Some(last) if last.label == label && label != Label::Separator => last.end = t.end,
            _ => spans.push(Span { start: t.start, end: t.end, label }),
        }
    }
    spans
}
