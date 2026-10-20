//! Duration, recurrence, alert, calendar and URL rules.

use super::datetime::{day_num, parse_time_token, small_number, weekday, weekday_plural};
use super::{Ctx, Match, Payload};
use crate::token::Token;
use crate::types::{Label, Recurrence};
use chrono::Weekday;

fn lower(toks: &[Token], i: usize) -> Option<&str> {
    toks.get(i).map(|t| t.lower.as_str())
}

fn labeled(len: usize, label: Label, payload: Payload) -> Option<Match> {
    Some(Match {
        len,
        labels: vec![label; len],
        payload,
    })
}

/// Minutes in one unit word ("min", "hours", "day"…).
fn unit_minutes(s: &str) -> Option<f64> {
    Some(match s {
        "m" | "min" | "mins" | "minute" | "minutes" => 1.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => 60.0,
        "d" | "day" | "days" => 1440.0,
        "w" | "wk" | "week" | "weeks" => 10080.0,
        _ => return None,
    })
}

/// "2 hours", "1.5 hours", "an hour", "half an hour", "30min".
/// Returns (minutes, tokens consumed).
fn amount_at(toks: &[Token], i: usize) -> Option<(u32, usize)> {
    let w0 = lower(toks, i)?;
    if w0 == "half"
        && matches!(lower(toks, i + 1), Some("an" | "a"))
        && lower(toks, i + 2).and_then(unit_minutes) == Some(60.0)
    {
        return Some((30, 3));
    }
    // Compact "30min" / "2h"
    let split = w0.find(|c: char| !c.is_ascii_digit() && c != '.');
    if let Some(at) = split.filter(|at| *at > 0) {
        if let (Ok(n), Some(u)) = (w0[..at].parse::<f64>(), unit_minutes(&w0[at..])) {
            return Some(((n * u).round() as u32, 1));
        }
    }
    let n: f64 = match w0.parse::<f64>() {
        Ok(n) if n > 0.0 => n,
        _ => f64::from(small_number(w0)?),
    };
    let u = unit_minutes(lower(toks, i + 1)?)?;
    Some(((n * u).round() as u32, 2))
}

pub(crate) fn duration_rule(toks: &[Token], i: usize, _ctx: &Ctx) -> Option<Match> {
    if lower(toks, i)? != "for" {
        return None;
    }
    let (mins, n) = amount_at(toks, i + 1)?;
    let mut labels = vec![Label::Duration; n + 1];
    labels[0] = Label::Separator;
    Some(Match {
        len: n + 1,
        labels,
        payload: Payload::Duration(mins),
    })
}

/// "alert 10 min before", "remind me 1 hour before", "alert at 2pm".
pub(crate) fn alert_rule(toks: &[Token], i: usize, _ctx: &Ctx) -> Option<Match> {
    let lead = match lower(toks, i)? {
        "alert" | "alarm" | "reminder" | "notify" => 1,
        "remind" if lower(toks, i + 1) == Some("me") => 2,
        _ => return None,
    };
    let j = i + lead;
    if matches!(lower(toks, j), Some("at" | "@")) {
        let te = parse_time_token(lower(toks, j + 1)?, true)?;
        return labeled(lead + 2, Label::Alert, Payload::AlertAt(te));
    }
    let (mins, n) = amount_at(toks, j)?;
    if lower(toks, j + n)? != "before" {
        return None;
    }
    labeled(lead + n + 1, Label::Alert, Payload::AlertBefore(mins))
}

/// Weekday list "monday and thursday", "mon, wed, fri". Returns days and tokens consumed.
fn weekday_list(toks: &[Token], i: usize, plural_ok: bool) -> Option<(Vec<Weekday>, usize)> {
    let one = |s: &str| weekday(s).or_else(|| if plural_ok { weekday_plural(s) } else { None });
    let mut days = vec![one(lower(toks, i)?)?];
    let mut n = 1;
    loop {
        let sep = lower(toks, i + n);
        if matches!(sep, Some("and" | "," | "&")) {
            if let Some(d) = lower(toks, i + n + 1).and_then(one) {
                days.push(d);
                n += 2;
                continue;
            }
        }
        break;
    }
    Some((days, n))
}

/// Optional "on <weekdays>" after a weekly rule.
fn on_weekdays(toks: &[Token], i: usize) -> (Vec<Weekday>, usize) {
    if lower(toks, i) == Some("on") {
        if let Some((d, n)) = weekday_list(toks, i + 1, true) {
            return (d, n + 1);
        }
    }
    (vec![], 0)
}

/// Optional "on the 1st" after a monthly rule.
fn on_month_day(toks: &[Token], i: usize) -> (Option<u32>, usize) {
    let (skip_on, j) = if lower(toks, i) == Some("on") {
        (1, i + 1)
    } else {
        (0, i)
    };
    if lower(toks, j) == Some("the") {
        if let Some(d) = lower(toks, j + 1)
            .filter(|s| s.chars().any(|c| c.is_ascii_alphabetic()))
            .and_then(day_num)
        {
            return (Some(d), skip_on + 2);
        }
    }
    (None, 0)
}

const WEEKDAYS: [Weekday; 5] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
];

/// Builds a rule from a frequency word, then consumes "on …" qualifiers.
fn finish(toks: &[Token], i: usize, used: usize, freq: &str, interval: u32) -> Option<Match> {
    let (rule, extra) = match freq {
        "day" => (Recurrence::Daily { interval }, 0),
        "week" => {
            let (days, n) = on_weekdays(toks, i + used);
            (Recurrence::Weekly { interval, days }, n)
        }
        "month" => {
            let (md, n) = on_month_day(toks, i + used);
            (
                Recurrence::Monthly {
                    interval,
                    month_day: md,
                },
                n,
            )
        }
        "year" => (Recurrence::Yearly { interval }, 0),
        _ => return None,
    };
    labeled(used + extra, Label::Recurrence, Payload::Recurrence(rule))
}

pub(crate) fn recurrence_rule(toks: &[Token], i: usize, _ctx: &Ctx) -> Option<Match> {
    let w0 = lower(toks, i)?;
    match w0 {
        "daily" => return finish(toks, i, 1, "day", 1),
        "weekly" => return finish(toks, i, 1, "week", 1),
        "monthly" => return finish(toks, i, 1, "month", 1),
        "yearly" | "annually" => return finish(toks, i, 1, "year", 1),
        _ => {}
    }
    // "mondays", "on mondays"
    let on = usize::from(w0 == "on");
    if let Some((days, n)) = lower(toks, i + on)
        .and_then(weekday_plural)
        .and_then(|_| weekday_list(toks, i + on, true))
    {
        return labeled(
            on + n,
            Label::Recurrence,
            Payload::Recurrence(Recurrence::Weekly { interval: 1, days }),
        );
    }
    if w0 != "every" {
        return None;
    }
    let w1 = lower(toks, i + 1)?;
    match w1 {
        "day" => return finish(toks, i, 2, "day", 1),
        "week" => return finish(toks, i, 2, "week", 1),
        "month" => return finish(toks, i, 2, "month", 1),
        "year" => return finish(toks, i, 2, "year", 1),
        "weekday" => {
            return labeled(
                2,
                Label::Recurrence,
                Payload::Recurrence(Recurrence::Weekly {
                    interval: 1,
                    days: WEEKDAYS.to_vec(),
                }),
            )
        }
        "weekend" => {
            let days = vec![Weekday::Sat, Weekday::Sun];
            return labeled(
                2,
                Label::Recurrence,
                Payload::Recurrence(Recurrence::Weekly { interval: 1, days }),
            );
        }
        "other" => {
            let unit = lower(toks, i + 2)?;
            return finish(toks, i, 3, unit, 2);
        }
        _ => {}
    }
    if let Some((days, n)) = weekday_list(toks, i + 1, true) {
        return labeled(
            1 + n,
            Label::Recurrence,
            Payload::Recurrence(Recurrence::Weekly { interval: 1, days }),
        );
    }
    let n: u32 = small_number(w1)?;
    let unit = lower(toks, i + 2)?.trim_end_matches('s');
    finish(toks, i, 3, unit, n)
}

/// "until dec 31" — end of a recurrence.
pub(crate) fn until_rule(toks: &[Token], i: usize, ctx: &Ctx) -> Option<Match> {
    if lower(toks, i)? != "until" {
        return None;
    }
    let m = super::datetime::date_rule(toks, i + 1, ctx)?;
    let Payload::Date(d) = m.payload else {
        return None;
    };
    labeled(m.len + 1, Label::Recurrence, Payload::Until(d))
}

pub(crate) fn calendar_rule(toks: &[Token], i: usize, ctx: &Ctx) -> Option<Match> {
    let t = toks.get(i)?;
    let name = t.text.strip_prefix('/')?;
    let cal = ctx
        .opts
        .calendars
        .iter()
        .find(|c| c.eq_ignore_ascii_case(name))?;
    labeled(1, Label::Calendar, Payload::Calendar(cal.clone()))
}

pub(crate) fn url_rule(toks: &[Token], i: usize, _ctx: &Ctx) -> Option<Match> {
    let t = toks.get(i)?;
    let l = &t.lower;
    if l.starts_with("http://") || l.starts_with("https://") || l.starts_with("www.") {
        return labeled(1, Label::Url, Payload::Url(t.text.clone()));
    }
    None
}
