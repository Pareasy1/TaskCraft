use chrono::{DateTime, TimeZone};
use chrono_tz::{Europe::Rome, Tz};
use taskcraft_parse::*;

fn now() -> DateTime<Tz> {
    Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap()
}

fn with_plans() -> ParseOptions {
    ParseOptions {
        plans: vec!["3D print".into(), "Casa".into()],
        ..Default::default()
    }
}

#[test]
fn slash_plan_matches_ignoring_spaces_and_case() {
    let s = "ship Rafael friday /3DPRINT #urgent";
    let r = parse_task(s, now(), &with_plans());
    assert_eq!(r.plan.as_deref(), Some("3D print"));
    assert_eq!(r.title, "ship Rafael");
    assert_eq!(r.tags, ["urgent"]);
    let plan_spans: Vec<&str> = r
        .spans
        .iter()
        .filter(|x| x.label == Label::Plan)
        .map(|x| &s[x.start..x.end])
        .collect();
    assert_eq!(plan_spans, ["/3DPRINT"]);
}

#[test]
fn unknown_slash_stays_in_title() {
    let r = parse_task("call /mom", now(), &with_plans());
    assert_eq!(r.plan, None);
    assert_eq!(r.title, "call /mom");
}

#[test]
fn slash_non_plans_untouched() {
    for s in ["pay 1/2 rent", "lunch w/ Sam", "read https://x.y/casa", "a / b"] {
        let a = parse_task(s, now(), &with_plans());
        let b = parse_task(s, now(), &ParseOptions::default());
        assert_eq!(a.plan, None, "{s}");
        assert_eq!(a.title, b.title, "{s}");
    }
}

#[test]
fn events_ignore_slash_plan() {
    let r = parse("dinner /casa tomorrow", now(), &with_plans());
    assert_eq!(r.plan, None);
}
