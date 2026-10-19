use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone};
use chrono_tz::{Europe::Rome, Tz};
use taskcraft_parse::*;

pub fn now() -> DateTime<Tz> {
    Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap()
}

pub fn p(s: &str) -> ParsedItem {
    parse(s, now(), &ParseOptions::default())
}

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

pub fn w(y: i32, m: u32, day: u32, hm: Option<(u32, u32)>) -> When {
    When { date: d(y, m, day), time: hm.map(|(h, mi)| NaiveTime::from_hms_opt(h, mi, 0).unwrap()) }
}

#[test]
fn plain_title_is_all_day_today() {
    let r = p("Brunch");
    assert_eq!(r.kind, ItemKind::Event);
    assert_eq!(r.title, "Brunch");
    assert_eq!(r.start, Some(w(2026, 10, 20, None)));
    assert!(r.all_day);
    assert_eq!(r.spans, vec![Span { start: 0, end: 6, label: Label::Title }]);
}

#[test]
fn task_prefixes() {
    for (input, prefix_end) in [
        ("todo buy milk", 4),
        ("To-do: buy milk", 6),
        ("remind me to buy milk", 12),
        ("✓ buy milk", "✓".len()),
    ] {
        let r = p(input);
        assert_eq!(r.kind, ItemKind::Task, "{input}");
        assert_eq!(r.title, "buy milk", "{input}");
        assert_eq!(r.spans[0], Span { start: 0, end: prefix_end, label: Label::TaskPrefix }, "{input}");
    }
}

#[test]
fn prefix_only_at_start() {
    let r = p("Buy milk todo");
    assert_eq!(r.kind, ItemKind::Event);
    assert_eq!(r.title, "Buy milk todo");
}

#[test]
fn todoist_is_not_a_prefix() {
    let r = p("todoist sync");
    assert_eq!(r.kind, ItemKind::Event);
    assert_eq!(r.title, "todoist sync");
}
