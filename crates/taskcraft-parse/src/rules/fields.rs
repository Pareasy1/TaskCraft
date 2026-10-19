//! Attendee and location fields, matched over tokens left untagged by the
//! date/time/extra rules.

use super::Tagged;
use crate::token::Token;
use crate::types::Label;

/// Words that end a free-text field (and may start the next one).
const STOP: &[&str] = &["with", "at", "@", "in", "on", "for", "from"];

fn free_run(toks: &[Token], tags: &[Tagged], from: usize) -> usize {
    let mut n = 0;
    while let (Some(t), Some(tag)) = (toks.get(from + n), tags.get(from + n)) {
        if tag.label.is_some() || STOP.contains(&t.lower.as_str()) {
            break;
        }
        n += 1;
    }
    n
}

/// Tags "with A, B and C"; returns the attendee names.
pub(crate) fn attendees(toks: &[Token], tags: &mut [Tagged]) -> Vec<String> {
    let mut names = Vec::new();
    for i in 0..toks.len() {
        if tags[i].label.is_some() || toks[i].lower != "with" {
            continue;
        }
        let n = free_run(toks, tags, i + 1);
        if n == 0 {
            continue;
        }
        tags[i].label = Some(Label::Separator);
        let mut current: Vec<&str> = Vec::new();
        for k in i + 1..i + 1 + n {
            tags[k].label = Some(Label::Attendees);
            match toks[k].lower.as_str() {
                "," | "and" | "&" => {
                    if !current.is_empty() {
                        names.push(current.join(" "));
                        current.clear();
                    }
                }
                _ => current.push(&toks[k].text),
            }
        }
        if !current.is_empty() {
            names.push(current.join(" "));
        }
        break;
    }
    names
}

/// Tags "at|@|in <place>"; returns the location text.
pub(crate) fn location(toks: &[Token], tags: &mut [Tagged]) -> Option<String> {
    for i in 0..toks.len() {
        if tags[i].label.is_some() || !matches!(toks[i].lower.as_str(), "at" | "@" | "in") {
            continue;
        }
        let n = free_run(toks, tags, i + 1);
        // Trailing punctuation is not part of the place.
        let n = (0..=n)
            .rev()
            .find(|&k| {
                k == 0
                    || !matches!(
                        toks[i + k].lower.as_str(),
                        "," | "." | "!" | "?" | ";" | ":"
                    )
            })
            .unwrap_or(0);
        if n == 0 {
            continue;
        }
        if toks[i].lower == "in"
            && toks[i + 1]
                .text
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        {
            continue;
        }
        tags[i].label = Some(Label::Separator);
        let mut place = String::new();
        for k in i + 1..i + 1 + n {
            tags[k].label = Some(Label::Location);
            if !place.is_empty() && !matches!(toks[k].text.as_str(), "," | ".") {
                place.push(' ');
            }
            place.push_str(&toks[k].text);
        }
        return Some(place);
    }
    None
}
