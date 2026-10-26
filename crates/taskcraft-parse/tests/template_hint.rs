use chrono::{DateTime, TimeZone};
use chrono_tz::{Europe::Rome, Tz};
use taskcraft_parse::*;

fn now() -> DateTime<Tz> {
    Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap()
}

fn with_templates() -> ParseOptions {
    ParseOptions {
        templates: vec!["Ordine 3D".into(), "Casa".into()],
        ..Default::default()
    }
}

#[test]
fn plus_template_matches() {
    let s = "ship Rafael friday +ordine3d";
    let r = parse_task(s, now(), &with_templates());
    assert_eq!(r.template.as_deref(), Some("Ordine 3D"));
    assert_eq!(r.title, "ship Rafael");
    let spans: Vec<&str> = r
        .spans
        .iter()
        .filter(|x| x.label == Label::Template)
        .map(|x| &s[x.start..x.end])
        .collect();
    assert_eq!(spans, ["+ordine3d"]);
}

#[test]
fn unknown_plus_stays() {
    let r = parse_task("a +b", now(), &with_templates());
    assert_eq!(r.template, None);
    assert_eq!(r.title, "a +b");
}

#[test]
fn first_template_wins() {
    let s = "x +casa +ordine3d";
    let r = parse_task(s, now(), &with_templates());
    assert_eq!(r.template.as_deref(), Some("Casa"));
    let n = r.spans.iter().filter(|x| x.label == Label::Template).count();
    assert_eq!(n, 1);
    assert_eq!(r.title, "x +ordine3d");
}

#[test]
fn plus_non_words_untouched() {
    for s in ["1+1", "c++ book", "+"] {
        let a = parse_task(s, now(), &with_templates());
        let b = parse_task(s, now(), &ParseOptions::default());
        assert_eq!(a.template, None, "{s}");
        assert_eq!(a.title, b.title, "{s}");
    }
}

#[test]
fn events_ignore_template() {
    let r = parse("dinner +casa tomorrow", now(), &with_templates());
    assert_eq!(r.template, None);
}
