use chrono::{DateTime, NaiveDate, TimeZone};
use chrono_tz::{Europe::Rome, Tz};
use taskcraft_parse::*;

fn now() -> DateTime<Tz> {
    Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap()
}

fn t(s: &str) -> ParsedItem {
    parse_task(s, now(), &ParseOptions::default())
}

fn tag_text<'a>(s: &'a str, p: &ParsedItem) -> Vec<&'a str> {
    p.spans
        .iter()
        .filter(|x| x.label == Label::Tag)
        .map(|x| &s[x.start..x.end])
        .collect()
}

#[test]
fn tags_removed_from_title() {
    let s = "buy milk #home #Errands friday";
    let r = t(s);
    assert_eq!(r.title, "buy milk");
    assert_eq!(r.tags, ["home", "Errands"]);
    assert_eq!(
        r.start.unwrap().date,
        NaiveDate::from_ymd_opt(2026, 10, 23).unwrap()
    );
    assert_eq!(tag_text(s, &r), ["#home", "#Errands"]);
}

#[test]
fn trailing_punct_and_dupes() {
    let r = t("call #home, #HOME");
    assert_eq!(r.title, "call");
    assert_eq!(r.tags, ["home"]);
}

#[test]
fn not_tags() {
    let r = t("C# meetup");
    assert!(r.tags.is_empty());
    assert_eq!(r.title, "C# meetup");
    assert!(t("#1 priority").tags.is_empty());
    assert!(t("## notes").tags.is_empty());
    assert!(t(&format!("#{}", "a".repeat(33))).tags.is_empty());
}

#[test]
fn events_keep_hash() {
    let r = parse("Party #fun saturday", now(), &ParseOptions::default());
    assert_eq!(r.kind, ItemKind::Event);
    assert!(r.tags.is_empty());
    assert!(r.title.contains("#fun"));
}

#[test]
fn prefix_task_tags() {
    let r = parse("todo: pay rent #bills", now(), &ParseOptions::default());
    assert_eq!(r.tags, ["bills"]);
    assert_eq!(r.title, "pay rent");
}
