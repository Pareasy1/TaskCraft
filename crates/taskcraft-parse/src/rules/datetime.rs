//! Date, time, range and "tonight" rules.

use super::{Ctx, Match, Payload};
use crate::token::Token;
use crate::types::{DateOrder, Label};
use chrono::Weekday;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Meridiem {
    Am,
    Pm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimeExpr {
    pub hour: u32,
    pub minute: u32,
    pub meridiem: Option<Meridiem>,
}

impl TimeExpr {
    /// No am/pm and an hour that could be either (1–12).
    pub fn ambiguous(&self) -> bool {
        self.meridiem.is_none() && (1..=12).contains(&self.hour)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DateExpr {
    Today,
    Tomorrow,
    Yesterday,
    Weekday { day: Weekday, next: bool },
    NextWeek,
    NextMonth,
    InDays(u32),
    InWeeks(u32),
    MonthDay { month: u32, day: u32, year: Option<i32> },
    Numeric { a: u32, b: u32, year: Option<i32> },
    Ordinal(u32),
    Iso { year: i32, month: u32, day: u32 },
}

// ---------- small token helpers ----------

fn lower(toks: &[Token], i: usize) -> Option<&str> {
    toks.get(i).map(|t| t.lower.as_str())
}

pub(crate) fn weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" | "tues" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" | "thur" | "thurs" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    })
}

/// Plural weekday ("mondays") used by recurrence rules.
pub(crate) fn weekday_plural(s: &str) -> Option<Weekday> {
    s.strip_suffix('s').and_then(weekday)
}

fn month(s: &str) -> Option<u32> {
    let s = s.trim_end_matches('.');
    Some(match s {
        "january" | "jan" => 1,
        "february" | "feb" => 2,
        "march" | "mar" => 3,
        "april" | "apr" => 4,
        "may" => 5,
        "june" | "jun" => 6,
        "july" | "jul" => 7,
        "august" | "aug" => 8,
        "september" | "sep" | "sept" => 9,
        "october" | "oct" => 10,
        "november" | "nov" => 11,
        "december" | "dec" => 12,
        _ => return None,
    })
}

/// "25", "25th", "1st" → day number 1..=31.
pub(crate) fn day_num(s: &str) -> Option<u32> {
    let digits = s
        .strip_suffix("st")
        .or_else(|| s.strip_suffix("nd"))
        .or_else(|| s.strip_suffix("rd"))
        .or_else(|| s.strip_suffix("th"))
        .unwrap_or(s);
    let n: u32 = digits.parse().ok()?;
    (1..=31).contains(&n).then_some(n)
}

fn is_ordinal(s: &str) -> bool {
    ["st", "nd", "rd", "th"].iter().any(|suf| s.ends_with(suf)) && day_num(s).is_some()
}

fn year(s: &str) -> Option<i32> {
    if s.len() == 4 {
        let y: i32 = s.parse().ok()?;
        (1900..=2200).contains(&y).then_some(y)
    } else {
        None
    }
}

pub(crate) fn small_number(s: &str) -> Option<u32> {
    match s {
        "a" | "an" | "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        _ => s.parse().ok().filter(|n| *n > 0 && *n < 1000),
    }
}

// ---------- time parsing ----------

/// Parses a single-token time: "3pm", "3:30pm", "15:30", "10a", "noon",
/// "midnight", or a bare hour "3" (only when `allow_bare`).
pub(crate) fn parse_time_token(s: &str, allow_bare: bool) -> Option<TimeExpr> {
    match s {
        "noon" | "midday" => return Some(TimeExpr { hour: 12, minute: 0, meridiem: Some(Meridiem::Pm) }),
        "midnight" => return Some(TimeExpr { hour: 0, minute: 0, meridiem: None }),
        _ => {}
    }
    let (body, mer) = if let Some(b) = s.strip_suffix("am").or_else(|| s.strip_suffix("a.m.")) {
        (b, Some(Meridiem::Am))
    } else if let Some(b) = s.strip_suffix("pm").or_else(|| s.strip_suffix("p.m.")) {
        (b, Some(Meridiem::Pm))
    } else if let Some(b) = s.strip_suffix('a') {
        (b, Some(Meridiem::Am))
    } else if let Some(b) = s.strip_suffix('p') {
        (b, Some(Meridiem::Pm))
    } else {
        (s, None)
    };
    if body.is_empty() || !body.chars().next()?.is_ascii_digit() {
        return None;
    }
    let (h, m, has_colon) = match body.split_once(|c| c == ':' || c == '.') {
        Some((h, m)) => {
            if m.len() != 2 {
                return None;
            }
            (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?, true)
        }
        None => {
            if body.len() > 2 {
                return None;
            }
            (body.parse::<u32>().ok()?, 0, false)
        }
    };
    if m > 59 {
        return None;
    }
    match mer {
        Some(_) if !(1..=12).contains(&h) => None,
        Some(_) => Some(TimeExpr { hour: h, minute: m, meridiem: mer }),
        None if h > 23 => None,
        None if !has_colon && !allow_bare => None,
        None => Some(TimeExpr { hour: h, minute: m, meridiem: None }),
    }
}

/// A time starting at token `i`, possibly two tokens ("3 pm"). Returns the
/// expression and the number of tokens consumed.
fn time_at(toks: &[Token], i: usize, allow_bare: bool) -> Option<(TimeExpr, usize)> {
    let t = toks.get(i)?;
    if let Some(next) = lower(toks, i + 1) {
        if matches!(next, "am" | "pm" | "a.m." | "p.m.") {
            let joined = format!("{}{}", t.lower, next);
            if let Some(te) = parse_time_token(&joined, false) {
                return Some((te, 2));
            }
        }
    }
    parse_time_token(&t.lower, allow_bare).map(|te| (te, 1))
}

/// "6-7pm", "8am-10:30am", "9-10:30" as one token.
fn dash_range_token(s: &str) -> Option<(TimeExpr, TimeExpr)> {
    let (a, b) = s.split_once('-')?;
    if b.contains('-') {
        return None;
    }
    Some((parse_time_token(a, true)?, parse_time_token(b, true)?))
}

fn is_range_connector(s: &str) -> bool {
    matches!(s, "to" | "until" | "till" | "til" | "-" | "–")
}

// ---------- date parsing ----------

/// A date expression starting at token `i`; returns (expr, tokens consumed,
/// labels). A leading "on" is consumed and labeled as a separator.
fn date_at(toks: &[Token], i: usize, ctx: &Ctx) -> Option<(DateExpr, Vec<Label>)> {
    if lower(toks, i) == Some("on") {
        let (e, mut labels) = date_core(toks, i + 1, ctx)?;
        labels.insert(0, Label::Separator);
        return Some((e, labels));
    }
    date_core(toks, i, ctx)
}

fn date_core(toks: &[Token], i: usize, ctx: &Ctx) -> Option<(DateExpr, Vec<Label>)> {
    let w0 = lower(toks, i)?;
    let w1 = lower(toks, i + 1);
    let w2 = lower(toks, i + 2);
    let date = |e: DateExpr, n: usize| Some((e, vec![Label::Date; n]));

    match w0 {
        "today" => return date(DateExpr::Today, 1),
        "tomorrow" | "tmrw" | "tmr" => return date(DateExpr::Tomorrow, 1),
        "yesterday" => return date(DateExpr::Yesterday, 1),
        "next" => {
            if let Some(d) = w1.and_then(weekday) {
                return date(DateExpr::Weekday { day: d, next: true }, 2);
            }
            return match w1 {
                Some("week") => date(DateExpr::NextWeek, 2),
                Some("month") => date(DateExpr::NextMonth, 2),
                _ => None,
            };
        }
        "this" => {
            if let Some(d) = w1.and_then(weekday) {
                return date(DateExpr::Weekday { day: d, next: false }, 2);
            }
            return None;
        }
        "in" => {
            let n = w1.and_then(small_number)?;
            return match w2 {
                Some("day" | "days") => date(DateExpr::InDays(n), 3),
                Some("week" | "weeks") => date(DateExpr::InWeeks(n), 3),
                _ => None,
            };
        }
        "the" => {
            let w1 = w1?;
            if is_ordinal(w1) {
                return date(DateExpr::Ordinal(day_num(w1)?), 2);
            }
            return None;
        }
        _ => {}
    }
    if let Some(d) = weekday(w0) {
        return date(DateExpr::Weekday { day: d, next: false }, 1);
    }
    // "Oct 24", "Oct 24th", "Oct 24, 2026", "Oct 24 2026"
    if let Some(m) = month(w0) {
        let day = w1.and_then(day_num)?;
        let (yr, extra) = year_after(toks, i + 2);
        return date(DateExpr::MonthDay { month: m, day, year: yr }, 2 + extra);
    }
    // "24 Oct", "24th of October", "3 March 2027"
    if let Some(day) = day_num(w0) {
        let (m_idx, of) = if w1 == Some("of") { (i + 2, 1) } else { (i + 1, 0) };
        if let Some(m) = lower(toks, m_idx).and_then(month) {
            let (yr, extra) = year_after(toks, m_idx + 1);
            return date(DateExpr::MonthDay { month: m, day, year: yr }, 2 + of + extra);
        }
    }
    // ISO "2026-12-01"
    if let Some(e) = iso(w0) {
        return date(e, 1);
    }
    // Numeric "7/11", "7/11/26", "3.1.2027"
    if let Some(e) = numeric(w0) {
        return date(e, 1);
    }
    let _ = ctx;
    None
}

/// Optional year after a month/day: ", 2026" or "2026".
fn year_after(toks: &[Token], i: usize) -> (Option<i32>, usize) {
    match lower(toks, i) {
        Some(",") => match lower(toks, i + 1).and_then(year) {
            Some(y) => (Some(y), 2),
            None => (None, 0),
        },
        Some(s) => match year(s) {
            Some(y) => (Some(y), 1),
            None => (None, 0),
        },
        None => (None, 0),
    }
}

fn iso(s: &str) -> Option<DateExpr> {
    let mut parts = s.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 {
        return None;
    }
    Some(DateExpr::Iso { year: y.parse().ok()?, month: m.parse().ok()?, day: d.parse().ok()? })
}

fn numeric(s: &str) -> Option<DateExpr> {
    let sep = if s.contains('/') { '/' } else if s.matches('.').count() >= 1 && !s.contains(':') { '.' } else { return None };
    let parts: Vec<&str> = s.split(sep).collect();
    if !(2..=3).contains(&parts.len()) || parts.iter().any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    let a: u32 = parts[0].parse().ok()?;
    let b: u32 = parts[1].parse().ok()?;
    if parts[0].len() > 2 || parts[1].len() > 2 {
        return None;
    }
    let year = match parts.get(2) {
        Some(y) if y.len() == 2 => Some(2000 + y.parse::<i32>().ok()?),
        Some(y) if y.len() == 4 => Some(y.parse().ok()?),
        Some(_) => return None,
        None => None,
    };
    Some(DateExpr::Numeric { a, b, year })
}

/// Converts a numeric date to (month, day) using the configured order.
pub(crate) fn numeric_md(a: u32, b: u32, order: DateOrder) -> (u32, u32) {
    match order {
        DateOrder::DayMonth => (b, a),
        DateOrder::MonthDay => (a, b),
    }
}

// ---------- rules ----------

pub(crate) fn date_rule(toks: &[Token], i: usize, ctx: &Ctx) -> Option<Match> {
    let (e, labels) = date_at(toks, i, ctx)?;
    Some(Match { len: labels.len(), labels, payload: Payload::Date(e) })
}

pub(crate) fn date_range_rule(toks: &[Token], i: usize, ctx: &Ctx) -> Option<Match> {
    let (a, mut labels) = date_at(toks, i, ctx)?;
    let j = i + labels.len();
    if !is_range_connector(lower(toks, j)?) && lower(toks, j) != Some("through") {
        return None;
    }
    let (b, lb) = date_core(toks, j + 1, ctx)?;
    labels.push(Label::Separator);
    labels.extend(lb);
    Some(Match { len: labels.len(), labels, payload: Payload::DateRange(a, b) })
}

pub(crate) fn time_rule(toks: &[Token], i: usize, _ctx: &Ctx) -> Option<Match> {
    let lead = matches!(lower(toks, i)?, "at" | "@");
    let j = i + usize::from(lead);
    let (te, n) = time_at(toks, j, lead)?;
    let mut labels = Vec::new();
    if lead {
        labels.push(Label::Separator);
    }
    labels.extend(std::iter::repeat(Label::Time).take(n));
    Some(Match { len: labels.len(), labels, payload: Payload::Time(te) })
}

pub(crate) fn time_range_rule(toks: &[Token], i: usize, _ctx: &Ctx) -> Option<Match> {
    let w0 = lower(toks, i)?;
    // "[at|from] 6-7pm"
    let lead = matches!(w0, "at" | "@" | "from");
    let j = i + usize::from(lead);
    if let Some((a, b)) = toks.get(j).and_then(|t| dash_range_token(&t.lower)) {
        let mut labels = Vec::new();
        if lead {
            labels.push(Label::Separator);
        }
        labels.push(Label::Time);
        return Some(Match { len: labels.len(), labels, payload: Payload::TimeRange(a, b) });
    }
    // "[from|at] X to|until|- Y"; without "from", X or Y must be explicit
    let (a, na) = time_at(toks, j, lead)?;
    let k = j + na;
    if !is_range_connector(lower(toks, k)?) {
        return None;
    }
    let (b, nb) = time_at(toks, k + 1, true)?;
    if !lead && a.ambiguous() && b.ambiguous() && a.meridiem.is_none() && b.meridiem.is_none() {
        let a_explicit = toks[j].lower.contains(':');
        if !a_explicit {
            return None;
        }
    }
    let mut labels = Vec::new();
    if lead {
        labels.push(Label::Separator);
    }
    labels.extend(std::iter::repeat(Label::Time).take(na));
    labels.push(Label::Separator);
    labels.extend(std::iter::repeat(Label::Time).take(nb));
    Some(Match { len: labels.len(), labels, payload: Payload::TimeRange(a, b) })
}

/// "tonight" → today, evening.
pub(crate) fn tonight_rule(toks: &[Token], i: usize, _ctx: &Ctx) -> Option<Match> {
    (lower(toks, i)? == "tonight").then(|| Match { len: 1, labels: vec![Label::Date], payload: Payload::Tonight })
}

/// "morning", "afternoon", "evening" (optionally "in the …" / "this …").
pub(crate) fn part_of_day_rule(toks: &[Token], i: usize, _ctx: &Ctx) -> Option<Match> {
    let (skip, word) = match lower(toks, i)? {
        "in" if lower(toks, i + 1) == Some("the") => (2, lower(toks, i + 2)?),
        "this" => (1, lower(toks, i + 1)?),
        w => (0, w),
    };
    let (hour, meridiem) = match word {
        "morning" => (9, Some(Meridiem::Am)),
        "afternoon" => (14, None),
        "evening" => (18, None),
        _ => return None,
    };
    let mut labels = vec![Label::Time; skip + 1];
    if skip > 0 {
        labels[0] = Label::Separator;
    }
    Some(Match {
        len: skip + 1,
        labels,
        payload: Payload::Time(TimeExpr { hour, minute: 0, meridiem }),
    })
}

