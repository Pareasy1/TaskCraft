use chrono::{DateTime, TimeZone};
use chrono_tz::{Europe::Rome, Tz};
use taskcraft_parse::*;

fn now() -> DateTime<Tz> {
    Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap()
}

fn t(s: &str) -> ParsedItem {
    parse_task(s, now(), &ParseOptions::default())
}

#[test]
fn priority_words_en_it() {
    use ParsedPriority::*;
    let cases = [
        ("!urgent", Urgent),
        ("!urgente", Urgent),
        ("!important", Important),
        ("!importante", Important),
        ("!medium", Medium),
        ("!media", Medium),
        ("!low", Low),
        ("!bassa", Low),
    ];
    for (word, want) in cases {
        for w in [
            word.to_string(),
            word.to_uppercase(),
            format!("!{}", &word[1..].to_uppercase()),
        ] {
            let s = format!("call mom {w} friday");
            let r = t(&s);
            assert_eq!(r.priority, Some(want), "{s}");
            assert_eq!(r.title, "call mom", "{s}");
            let spans: Vec<&str> = r
                .spans
                .iter()
                .filter(|x| x.label == Label::Priority)
                .map(|x| &s[x.start..x.end])
                .collect();
            assert_eq!(spans, [w.as_str()], "{s}");
        }
    }
    let r = t("call mom !URGENTE friday");
    assert_eq!(r.priority, Some(ParsedPriority::Urgent));
    assert_eq!(r.title, "call mom");
}

#[test]
fn first_priority_wins() {
    let r = t("x !low !urgent");
    assert_eq!(r.priority, Some(ParsedPriority::Low));
    assert_eq!(r.title, "x !urgent");
}

#[test]
fn trailing_punctuation_still_matches() {
    let r = t("call mom !urgent, friday");
    assert_eq!(r.priority, Some(ParsedPriority::Urgent));
}

#[test]
fn other_bang_words_untouched() {
    for s in ["wow!", "!yay", "hi !", "x !urgentish"] {
        let r = t(s);
        assert_eq!(r.priority, None, "{s}");
        assert!(!r.spans.iter().any(|x| x.label == Label::Priority), "{s}");
    }
    assert_eq!(t("wow!").title, "wow!");
    assert_eq!(t("!yay").title, "!yay");
    // A lone "!" is punctuation and attaches to the previous word, as before.
    assert_eq!(t("hi !").title, "hi!");
}

#[test]
fn events_ignore_priority() {
    let r = parse("dinner !urgent tomorrow", now(), &ParseOptions::default());
    assert_eq!(r.kind, ItemKind::Event);
    assert_eq!(r.priority, None);
}
