//! Turns tagged payloads into concrete dates and times.

use crate::rules::datetime::{numeric_md, DateExpr, Meridiem, TimeExpr};
use crate::rules::{Ctx, Payload, Tagged};
use crate::types::*;
use chrono::{Datelike, Days, Months, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

const EVENING_WORDS: &[&str] = &["dinner", "drinks", "party", "concert", "movie", "show"];

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Resolved {
    pub start: Option<When>,
    pub end: Option<When>,
    pub all_day: bool,
}

/// Facts collected from tagged tokens; only the first of each kind is used.
#[derive(Default)]
struct Facts {
    date: Option<DateExpr>,
    date_range: Option<(DateExpr, DateExpr)>,
    time: Option<TimeExpr>,
    time_range: Option<(TimeExpr, TimeExpr)>,
    tonight: bool,
}

pub(crate) fn resolve(tags: &[Tagged], title: &str, kind: ItemKind, ctx: &Ctx) -> Resolved {
    let mut f = Facts::default();
    for t in tags {
        match &t.payload {
            Payload::Date(d) if f.date.is_none() => f.date = Some(*d),
            Payload::DateRange(a, b) if f.date_range.is_none() => f.date_range = Some((*a, *b)),
            Payload::Time(te) if f.time.is_none() => f.time = Some(*te),
            Payload::TimeRange(a, b) if f.time_range.is_none() => f.time_range = Some((*a, *b)),
            Payload::Tonight => f.tonight = true,
            _ => {}
        }
    }
    let lower_title = title.to_lowercase();
    let evening = f.tonight || EVENING_WORDS.iter().any(|w| lower_title.split_whitespace().any(|t| t == *w));
    let today = ctx.now.date_naive();
    let now_time = ctx.now.time();

    // Times (start, optional end) as NaiveTime.
    let (mut start_t, end_t) = match (f.time_range, f.time) {
        (Some((a, b)), _) => {
            let (s, e) = resolve_range(a, b, evening);
            (Some(s), Some(e))
        }
        (None, Some(t)) => (Some(resolve_time(t, evening)), None),
        (None, None) => (None, None),
    };
    if f.tonight && start_t.is_none() {
        start_t = NaiveTime::from_hms_opt(20, 0, 0);
    }

    // Dates.
    let date_expr = f.date_range.map(|r| r.0).or(f.date).or(f.tonight.then_some(DateExpr::Today));
    let mut start_d = match date_expr {
        Some(e) => resolve_date(e, today, ctx),
        None => None,
    };
    let range_end_d = f.date_range.and_then(|(_, b)| resolve_date(b, start_d.unwrap_or(today), ctx));

    if start_d.is_none() {
        if let Some(t) = start_t {
            start_d = Some(if t < now_time { today + Days::new(1) } else { today });
        } else if kind == ItemKind::Event {
            start_d = Some(today);
        }
    } else if let (Some(DateExpr::Weekday { next: false, .. }), Some(t), Some(d)) = (date_expr, start_t, start_d) {
        if d == today && t < now_time {
            start_d = Some(d + Days::new(7));
        }
    }

    let Some(sd) = start_d else {
        return Resolved::default();
    };
    let start = When { date: sd, time: start_t };

    if kind == ItemKind::Task {
        return Resolved { start: Some(start), end: None, all_day: false };
    }

    match start_t {
        None => {
            let end = range_end_d.filter(|e| *e >= sd).map(|e| When { date: e, time: None });
            Resolved { start: Some(start), end, all_day: true }
        }
        Some(st) => {
            let start_dt = NaiveDateTime::new(sd, st);
            let end_dt = match end_t {
                Some(et) => {
                    let ed = range_end_d.unwrap_or(sd);
                    let mut e = NaiveDateTime::new(ed, et);
                    if e <= start_dt {
                        e += chrono::Duration::days(1);
                    }
                    e
                }
                None => start_dt + chrono::Duration::minutes(i64::from(ctx.opts.default_event_minutes)),
            };
            Resolved {
                start: Some(start),
                end: Some(When { date: end_dt.date(), time: Some(end_dt.time()) }),
                all_day: false,
            }
        }
    }
}

fn hm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h % 24, m, 0).unwrap_or(NaiveTime::MIN)
}

fn with_meridiem(t: TimeExpr, mer: Meridiem) -> NaiveTime {
    let h = t.hour % 12;
    match mer {
        Meridiem::Am => hm(h, t.minute),
        Meridiem::Pm => hm(h + 12, t.minute),
    }
}

/// Bare hours: 1–6 → pm, 7–11 → am, 12 → noon; evening context → pm.
fn resolve_time(t: TimeExpr, evening: bool) -> NaiveTime {
    if let Some(m) = t.meridiem {
        return with_meridiem(t, m);
    }
    if !t.ambiguous() {
        return hm(t.hour, t.minute);
    }
    if t.hour == 12 {
        return hm(12, t.minute);
    }
    if evening || t.hour <= 6 {
        with_meridiem(t, Meridiem::Pm)
    } else {
        with_meridiem(t, Meridiem::Am)
    }
}

fn resolve_range(a: TimeExpr, b: TimeExpr, evening: bool) -> (NaiveTime, NaiveTime) {
    // End's meridiem propagates to an ambiguous start when it keeps start <= end.
    if a.ambiguous() && b.meridiem.is_some() {
        let e = resolve_time(b, evening);
        let same = with_meridiem(a, b.meridiem.unwrap_or(Meridiem::Am));
        let s = if same <= e {
            same
        } else {
            let other = if b.meridiem == Some(Meridiem::Pm) { Meridiem::Am } else { Meridiem::Pm };
            with_meridiem(a, other)
        };
        return (s, e);
    }
    let s = resolve_time(a, evening);
    if b.ambiguous() {
        // Pick the reading of the end that comes soonest after the start.
        let am = with_meridiem(b, Meridiem::Am);
        let pm = with_meridiem(b, Meridiem::Pm);
        let e = if am > s { am } else if pm > s { pm } else { am };
        return (s, e);
    }
    (s, resolve_time(b, evening))
}

fn ymd(y: i32, m: u32, d: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(y, m, d)
}

/// Month/day without a year: this year if not past, else next year.
fn upcoming_md(month: u32, day: u32, today: NaiveDate) -> Option<NaiveDate> {
    match ymd(today.year(), month, day) {
        Some(d) if d >= today => Some(d),
        _ => ymd(today.year() + 1, month, day),
    }
}

fn monday_of(d: NaiveDate) -> NaiveDate {
    d - Days::new(u64::from(d.weekday().num_days_from_monday()))
}

fn weekday_on_or_after(d: NaiveDate, w: Weekday) -> NaiveDate {
    let diff = (7 + w.num_days_from_monday() - d.weekday().num_days_from_monday()) % 7;
    d + Days::new(u64::from(diff))
}

pub(crate) fn resolve_date(e: DateExpr, today: NaiveDate, ctx: &Ctx) -> Option<NaiveDate> {
    match e {
        DateExpr::Today => Some(today),
        DateExpr::Tomorrow => today.checked_add_days(Days::new(1)),
        DateExpr::Yesterday => today.checked_sub_days(Days::new(1)),
        DateExpr::Weekday { day, next: false } => Some(weekday_on_or_after(today, day)),
        DateExpr::Weekday { day, next: true } => {
            Some(monday_of(today) + Days::new(7 + u64::from(day.num_days_from_monday())))
        }
        DateExpr::NextWeek => Some(monday_of(today) + Days::new(7)),
        DateExpr::NextMonth => {
            let first = ymd(today.year(), today.month(), 1)?;
            first.checked_add_months(Months::new(1))
        }
        DateExpr::InDays(n) => today.checked_add_days(Days::new(u64::from(n))),
        DateExpr::InWeeks(n) => today.checked_add_days(Days::new(7 * u64::from(n))),
        DateExpr::MonthDay { month, day, year: Some(y) } => ymd(y, month, day),
        DateExpr::MonthDay { month, day, year: None } => upcoming_md(month, day, today),
        DateExpr::Numeric { a, b, year } => {
            let (m, d) = numeric_md(a, b, ctx.opts.date_order);
            match year {
                Some(y) => ymd(y, m, d),
                None => upcoming_md(m, d, today),
            }
        }
        DateExpr::Ordinal(day) => match ymd(today.year(), today.month(), day) {
            Some(d) if d >= today => Some(d),
            _ => {
                let next = ymd(today.year(), today.month(), 1)?.checked_add_months(Months::new(1))?;
                ymd(next.year(), next.month(), day)
            }
        },
        DateExpr::Iso { year, month, day } => ymd(year, month, day),
    }
}
