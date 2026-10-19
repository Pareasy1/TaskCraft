use chrono::TimeZone;
use chrono_tz::Europe::Rome;
use proptest::prelude::*;
use taskcraft_parse::*;

fn check_spans(input: &str) {
    let now = Rome.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
    let r = parse(input, now, &ParseOptions::default());
    let mut prev_end = 0;
    for s in &r.spans {
        assert!(s.start < s.end, "empty span {s:?} in {input:?}");
        assert!(s.start >= prev_end, "overlap/unsorted {s:?} in {input:?}");
        assert!(s.end <= input.len());
        assert!(input.is_char_boundary(s.start) && input.is_char_boundary(s.end));
        prev_end = s.end;
    }
}

const WORDS: &[&str] = &[
    "at", "on", "with", "in", "from", "to", "until", "every", "next", "this", "the", "for", "alert",
    "remind", "me", "before", "todo", "3pm", "7", "noon", "tomorrow", "friday", "oct", "24", "25th",
    "2", "hours", "min", "day", "weeks", "monthly", "/work", "-", ",", "and", "Anna", "Caffè", "🍰",
    "6-7pm", "7/11", "2026-12-01", "tonight", "@", "1.5", "half", "an", "hour",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    #[test]
    fn parse_never_panics_and_spans_valid_random(s in "\\PC{0,80}") {
        check_spans(&s);
    }

    #[test]
    fn parse_never_panics_and_spans_valid_words(idx in proptest::collection::vec(0..WORDS.len(), 0..14)) {
        let s = idx.iter().map(|i| WORDS[*i]).collect::<Vec<_>>().join(" ");
        check_spans(&s);
    }
}
