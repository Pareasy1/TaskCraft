//! Tagging driver: ordered rules, longest match wins at each position.

pub(crate) mod prefix;

use crate::token::Token;
use crate::types::*;
use chrono::DateTime;
use chrono_tz::Tz;

pub(crate) struct Ctx<'a> {
    pub now: DateTime<Tz>,
    pub opts: &'a ParseOptions,
}

/// Semantic payload attached to the first token of a match.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Payload {
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tagged {
    pub label: Option<Label>,
    pub payload: Payload,
}

pub(crate) struct Match {
    pub len: usize,
    /// Label for each token in the match (same length as `len`).
    pub labels: Vec<Label>,
    pub payload: Payload,
}

pub(crate) type Rule = fn(&[Token], usize, &Ctx) -> Option<Match>;

pub(crate) fn run_rules(toks: &[Token], rules: &[Rule], ctx: &Ctx, tags: &mut [Tagged]) {
    let mut i = 0;
    while i < toks.len() {
        if tags[i].label.is_some() {
            i += 1;
            continue;
        }
        let mut best: Option<Match> = None;
        for rule in rules {
            if let Some(m) = rule(toks, i, ctx) {
                let fits = m.len > 0 && (i..i + m.len).all(|j| j < toks.len() && tags[j].label.is_none());
                if fits && best.as_ref().map_or(true, |b| m.len > b.len) {
                    best = Some(m);
                }
            }
        }
        match best {
            Some(m) => {
                for (k, l) in m.labels.iter().enumerate() {
                    tags[i + k].label = Some(*l);
                }
                tags[i].payload = m.payload;
                i += m.len;
            }
            None => i += 1,
        }
    }
}
