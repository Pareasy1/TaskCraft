//! Natural-language parser for TaskCraft quick entry.

mod resolve;
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
    parse_kind(input, now, opts, false)
}

/// Like [`parse`], but the result is always a task (no prefix needed).
pub fn parse_task(input: &str, now: DateTime<Tz>, opts: &ParseOptions) -> ParsedItem {
    parse_kind(input, now, opts, true)
}

fn parse_kind(input: &str, now: DateTime<Tz>, opts: &ParseOptions, force_task: bool) -> ParsedItem {
    let lead_ws = input.len() - input.trim_start().len();
    let body = &input[lead_ws..];
    let mut spans = Vec::new();
    let mut kind = if force_task {
        ItemKind::Task
    } else {
        ItemKind::Event
    };
    let mut offset = lead_ws;
    if let Some(len) = rules::prefix::task_prefix_len(body) {
        kind = ItemKind::Task;
        spans.push(Span {
            start: lead_ws,
            end: lead_ws + len,
            label: Label::TaskPrefix,
        });
        offset += len;
    }

    let toks = token::tokenize(&input[offset..], offset);
    let mut tags: Vec<Tagged> = toks
        .iter()
        .map(|_| Tagged {
            label: None,
            payload: Payload::None,
        })
        .collect();
    let tags_found = if kind == ItemKind::Task {
        mark_tags(&toks, &mut tags)
    } else {
        Vec::new()
    };
    let plan = if kind == ItemKind::Task {
        mark_plan(&toks, &mut tags, opts)
    } else {
        None
    };
    let template = if kind == ItemKind::Task {
        mark_template(&toks, &mut tags, opts)
    } else {
        None
    };
    let priority = if kind == ItemKind::Task {
        mark_priority(&toks, &mut tags)
    } else {
        None
    };
    let ctx = Ctx { now, opts };
    rules::run_rules(&toks, rules::RULES, &ctx, &mut tags);
    let attendees = rules::fields::attendees(&toks, &mut tags);
    let location = rules::fields::location(&toks, &mut tags);

    let title = assemble_title(&toks, &mut tags);
    spans.extend(collect_spans(&toks, &tags));
    let r = resolve::resolve(&tags, &title, kind, &ctx);

    ParsedItem {
        kind,
        title,
        start: r.start,
        end: r.end,
        all_day: r.all_day,
        location,
        attendees,
        calendar: r.calendar,
        recurrence: r.recurrence,
        alerts: r.alerts,
        url: r.url,
        tags: tags_found,
        plan,
        template,
        priority,
        spans,
    }
}

/// `#` followed by 1–32 letters, digits, `-` or `_`, with at least one letter.
fn tag_name(t: &Token) -> Option<&str> {
    let name = t.text.strip_prefix('#')?;
    let ok = (1..=32).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        && name.chars().any(char::is_alphabetic);
    ok.then_some(name)
}

/// Labels `#tag` tokens and returns their names, deduplicated ignoring case.
fn mark_tags(toks: &[Token], tags: &mut [Tagged]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (t, tag) in toks.iter().zip(tags.iter_mut()) {
        if let Some(name) = tag_name(t) {
            tag.label = Some(Label::Tag);
            if !out.iter().any(|o| o.to_lowercase() == name.to_lowercase()) {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// Labels the first priority word (`!urgent`, `!bassa`, ...) and returns its
/// level. Later priority words and other `!words` stay in the title.
fn mark_priority(toks: &[Token], tags: &mut [Tagged]) -> Option<ParsedPriority> {
    for (t, tag) in toks.iter().zip(tags.iter_mut()) {
        if tag.label.is_some() {
            continue;
        }
        let level = match t.lower.as_str() {
            "!urgent" | "!urgente" => ParsedPriority::Urgent,
            "!important" | "!importante" => ParsedPriority::Important,
            "!medium" | "!media" => ParsedPriority::Medium,
            "!low" | "!bassa" => ParsedPriority::Low,
            _ => continue,
        };
        tag.label = Some(Label::Priority);
        return Some(level);
    }
    None
}

/// Lowercased alphanumeric characters only, so "3D print" and "/3dprint" match.
pub fn plan_key(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Labels the first `/plan` token matching a known plan and returns the
/// plan's exact name (a token that is also a calendar name is left to the
/// calendar rule). Other `/word` tokens are left for the title.
fn mark_plan(toks: &[Token], tags: &mut [Tagged], opts: &ParseOptions) -> Option<String> {
    for (t, tag) in toks.iter().zip(tags.iter_mut()) {
        if tag.label.is_some() {
            continue;
        }
        let Some(word) = t.text.strip_prefix('/') else {
            continue;
        };
        if !word.chars().any(char::is_alphabetic) {
            continue;
        }
        let key = plan_key(word);
        if let Some(name) = opts.plans.iter().find(|p| plan_key(p) == key) {
            // A name that is also a calendar stays unlabelled so the calendar
            // rule claims the token; the plan is still reported.
            if !opts.calendars.iter().any(|c| c.eq_ignore_ascii_case(word)) {
                tag.label = Some(Label::Plan);
            }
            return Some(name.clone());
        }
    }
    None
}

/// Labels the first `+template` token matching a known template and returns
/// the template's exact name. Other `+word` tokens stay in the title.
fn mark_template(toks: &[Token], tags: &mut [Tagged], opts: &ParseOptions) -> Option<String> {
    for (t, tag) in toks.iter().zip(tags.iter_mut()) {
        if tag.label.is_some() {
            continue;
        }
        let Some(word) = t.text.strip_prefix('+') else {
            continue;
        };
        if !word.chars().any(char::is_alphabetic) {
            continue;
        }
        let key = plan_key(word);
        if let Some(name) = opts.templates.iter().find(|n| plan_key(n) == key) {
            tag.label = Some(Label::Template);
            return Some(name.clone());
        }
    }
    None
}

/// Words dropped from the end of a title ("Lunch at", "Call with").
const DANGLING: &[&str] = &[
    "at", "@", "on", "with", "in", "from", "to", "for", "and", "&", "-",
];

fn trailing_junk(t: &Token) -> bool {
    matches!(t.text.as_str(), "," | "." | ";" | ":") || DANGLING.contains(&t.lower.as_str())
}

/// Untagged tokens become the title; they are labeled `Title` in place.
/// Trailing punctuation and dangling connector words are left unlabeled.
fn assemble_title(toks: &[Token], tags: &mut [Tagged]) -> String {
    let untagged: Vec<usize> = (0..toks.len())
        .filter(|&i| tags[i].label.is_none())
        .collect();
    let keep = untagged
        .iter()
        .rposition(|&i| !trailing_junk(&toks[i]))
        .map_or(0, |p| p + 1);
    let mut title = String::new();
    for &i in &untagged[..keep] {
        let (t, tag) = (&toks[i], &mut tags[i]);
        tag.label = Some(Label::Title);
        let attach = t.kind == TokKind::Punct
            && matches!(t.text.as_str(), "," | "." | "!" | "?" | ";" | ":" | ")");
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
            Some(last)
                if last.label == label && !matches!(label, Label::Separator | Label::Tag) =>
            {
                last.end = t.end
            }
            _ => spans.push(Span {
                start: t.start,
                end: t.end,
                label,
            }),
        }
    }
    spans
}
