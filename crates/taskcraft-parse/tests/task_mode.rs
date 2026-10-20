use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone};
use chrono_tz::{Europe::Rome, Tz};
use taskcraft_parse::*;

fn now() -> DateTime<Tz> {
    Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap()
}

fn t(s: &str) -> ParsedItem {
    parse_task(s, now(), &ParseOptions::default())
}

#[test]
fn dated_text_becomes_task_with_due() {
    let r = t("pay rent friday 9am");
    assert_eq!(r.kind, ItemKind::Task);
    assert_eq!(r.title, "pay rent");
    assert_eq!(
        r.start,
        Some(When {
            date: NaiveDate::from_ymd_opt(2026, 10, 23).unwrap(),
            time: NaiveTime::from_hms_opt(9, 0, 0),
        })
    );
    assert_eq!(r.end, None);
}

#[test]
fn undated_text_becomes_task_without_due() {
    let r = t("buy milk");
    assert_eq!(r.kind, ItemKind::Task);
    assert_eq!(r.title, "buy milk");
    assert_eq!(r.start, None);
}

#[test]
fn explicit_prefix_still_recognised() {
    let r = t("todo: call mom tomorrow");
    assert_eq!(r.kind, ItemKind::Task);
    assert_eq!(r.title, "call mom");
    assert_eq!(r.spans[0].label, Label::TaskPrefix);
}
