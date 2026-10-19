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
    When {
        date: d(y, m, day),
        time: hm.map(|(h, mi)| NaiveTime::from_hms_opt(h, mi, 0).unwrap()),
    }
}

#[test]
fn plain_title_is_all_day_today() {
    let r = p("Brunch");
    assert_eq!(r.kind, ItemKind::Event);
    assert_eq!(r.title, "Brunch");
    assert_eq!(r.start, Some(w(2026, 10, 20, None)));
    assert!(r.all_day);
    assert_eq!(
        r.spans,
        vec![Span {
            start: 0,
            end: 6,
            label: Label::Title
        }]
    );
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
        assert_eq!(
            r.spans[0],
            Span {
                start: 0,
                end: prefix_end,
                label: Label::TaskPrefix
            },
            "{input}"
        );
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

type Hm = Option<(u32, u32)>;

/// (input, title, start (y,m,d,hm), end (y,m,d,hm) or None, all_day)
fn check(
    input: &str,
    title: &str,
    start: (i32, u32, u32, Hm),
    end: Option<(i32, u32, u32, Hm)>,
    all_day: bool,
) {
    check_with(input, title, start, end, all_day, &ParseOptions::default());
}

fn check_with(
    input: &str,
    title: &str,
    start: (i32, u32, u32, Hm),
    end: Option<(i32, u32, u32, Hm)>,
    all_day: bool,
    opts: &ParseOptions,
) {
    let r = parse(input, now(), opts);
    assert_eq!(r.title, title, "title of {input:?}");
    assert_eq!(
        r.start,
        Some(w(start.0, start.1, start.2, start.3)),
        "start of {input:?}"
    );
    assert_eq!(
        r.end,
        end.map(|e| w(e.0, e.1, e.2, e.3)),
        "end of {input:?}"
    );
    assert_eq!(r.all_day, all_day, "all_day of {input:?}");
}

#[test]
fn dates_and_times() {
    check(
        "Meeting tomorrow at 3pm",
        "Meeting",
        (2026, 10, 21, Some((15, 0))),
        Some((2026, 10, 21, Some((16, 0)))),
        false,
    );
    check(
        "Dinner at 7",
        "Dinner",
        (2026, 10, 20, Some((19, 0))),
        Some((2026, 10, 20, Some((20, 0)))),
        false,
    );
    check(
        "Meeting at 9",
        "Meeting",
        (2026, 10, 21, Some((9, 0))),
        Some((2026, 10, 21, Some((10, 0)))),
        false,
    );
    check(
        "Call at 5",
        "Call",
        (2026, 10, 20, Some((17, 0))),
        Some((2026, 10, 20, Some((18, 0)))),
        false,
    );
    check(
        "Lunch at noon",
        "Lunch",
        (2026, 10, 20, Some((12, 0))),
        Some((2026, 10, 20, Some((13, 0)))),
        false,
    );
    check(
        "Review friday 14:30",
        "Review",
        (2026, 10, 23, Some((14, 30))),
        Some((2026, 10, 23, Some((15, 30)))),
        false,
    );
    check(
        "Party next friday 9pm",
        "Party",
        (2026, 10, 30, Some((21, 0))),
        Some((2026, 10, 30, Some((22, 0)))),
        false,
    );
    check(
        "Coffee tuesday 11am",
        "Coffee",
        (2026, 10, 20, Some((11, 0))),
        Some((2026, 10, 20, Some((12, 0)))),
        false,
    );
    check(
        "Coffee tuesday 9am",
        "Coffee",
        (2026, 10, 27, Some((9, 0))),
        Some((2026, 10, 27, Some((10, 0)))),
        false,
    );
    check(
        "Yoga 6-7pm",
        "Yoga",
        (2026, 10, 20, Some((18, 0))),
        Some((2026, 10, 20, Some((19, 0)))),
        false,
    );
    check(
        "Workshop from 2 to 4:30pm thursday",
        "Workshop",
        (2026, 10, 22, Some((14, 0))),
        Some((2026, 10, 22, Some((16, 30)))),
        false,
    );
    check(
        "Night shift 10pm to 6am",
        "Night shift",
        (2026, 10, 20, Some((22, 0))),
        Some((2026, 10, 21, Some((6, 0)))),
        false,
    );
    check(
        "Movie tonight",
        "Movie",
        (2026, 10, 20, Some((20, 0))),
        Some((2026, 10, 20, Some((21, 0)))),
        false,
    );
    check(
        "Flight 7/11 8am-10:30am",
        "Flight",
        (2026, 11, 7, Some((8, 0))),
        Some((2026, 11, 7, Some((10, 30)))),
        false,
    );
    check(
        "Trip Oct 24 to Oct 26",
        "Trip",
        (2026, 10, 24, None),
        Some((2026, 10, 26, None)),
        true,
    );
    check("Exam 3 March", "Exam", (2027, 3, 3, None), None, true);
    check("Recap Oct 1", "Recap", (2027, 10, 1, None), None, true);
    check(
        "Review 2026-12-01 at 10:00",
        "Review",
        (2026, 12, 1, Some((10, 0))),
        Some((2026, 12, 1, Some((11, 0)))),
        false,
    );
    check("Call in 2 weeks", "Call", (2026, 11, 3, None), None, true);
    check(
        "Pay bills the 25th",
        "Pay bills",
        (2026, 10, 25, None),
        None,
        true,
    );
    check("Sync next week", "Sync", (2026, 10, 26, None), None, true);
}

#[test]
fn more_date_time_variants() {
    check(
        "meeting TOMORROW at 3PM",
        "meeting",
        (2026, 10, 21, Some((15, 0))),
        Some((2026, 10, 21, Some((16, 0)))),
        false,
    );
    check(
        "tomorrow 3pm meeting",
        "meeting",
        (2026, 10, 21, Some((15, 0))),
        Some((2026, 10, 21, Some((16, 0)))),
        false,
    );
    check(
        "Standup today at 11:15",
        "Standup",
        (2026, 10, 20, Some((11, 15))),
        Some((2026, 10, 20, Some((12, 15)))),
        false,
    );
    check(
        "Brunch sunday at 11am",
        "Brunch",
        (2026, 10, 25, Some((11, 0))),
        Some((2026, 10, 25, Some((12, 0)))),
        false,
    );
    check(
        "Call mom this saturday",
        "Call mom",
        (2026, 10, 24, None),
        None,
        true,
    );
    check(
        "Gym monday 7am",
        "Gym",
        (2026, 10, 26, Some((7, 0))),
        Some((2026, 10, 26, Some((8, 0)))),
        false,
    );
    check(
        "Drinks at 6",
        "Drinks",
        (2026, 10, 20, Some((18, 0))),
        Some((2026, 10, 20, Some((19, 0)))),
        false,
    );
    check(
        "Breakfast at 8",
        "Breakfast",
        (2026, 10, 21, Some((8, 0))),
        Some((2026, 10, 21, Some((9, 0)))),
        false,
    );
    check(
        "Concert at 9",
        "Concert",
        (2026, 10, 20, Some((21, 0))),
        Some((2026, 10, 20, Some((22, 0)))),
        false,
    );
    check(
        "Late call at midnight",
        "Late call",
        (2026, 10, 21, Some((0, 0))),
        Some((2026, 10, 21, Some((1, 0)))),
        false,
    );
    check(
        "Report due december 3",
        "Report due",
        (2026, 12, 3, None),
        None,
        true,
    );
    check(
        "Holiday Dec 24 - Dec 26",
        "Holiday",
        (2026, 12, 24, None),
        Some((2026, 12, 26, None)),
        true,
    );
    check("Exam 15 jan 2027", "Exam", (2027, 1, 15, None), None, true);
    check(
        "Party nov 7, 2026 at 8pm",
        "Party",
        (2026, 11, 7, Some((20, 0))),
        Some((2026, 11, 7, Some((21, 0)))),
        false,
    );
    check("Call 25/12", "Call", (2026, 12, 25, None), None, true);
    check("Call 3/1/2027", "Call", (2027, 1, 3, None), None, true);
    check(
        "Meeting 9-10:30",
        "Meeting",
        (2026, 10, 21, Some((9, 0))),
        Some((2026, 10, 21, Some((10, 30)))),
        false,
    );
    check(
        "Session from 3pm until 5pm",
        "Session",
        (2026, 10, 20, Some((15, 0))),
        Some((2026, 10, 20, Some((17, 0)))),
        false,
    );
    check(
        "Session 3pm to 5pm",
        "Session",
        (2026, 10, 20, Some((15, 0))),
        Some((2026, 10, 20, Some((17, 0)))),
        false,
    );
    check("Walk in 3 days", "Walk", (2026, 10, 23, None), None, true);
    check("Plan next month", "Plan", (2026, 11, 1, None), None, true);
    check(
        "Lunch tomorrow at 12:30pm",
        "Lunch",
        (2026, 10, 21, Some((12, 30))),
        Some((2026, 10, 21, Some((13, 30)))),
        false,
    );
    check(
        "Dinner tonight at 9",
        "Dinner",
        (2026, 10, 20, Some((21, 0))),
        Some((2026, 10, 20, Some((22, 0)))),
        false,
    );
    check(
        "Something yesterday",
        "Something",
        (2026, 10, 19, None),
        None,
        true,
    );
    check(
        "Call friday 10a",
        "Call",
        (2026, 10, 23, Some((10, 0))),
        Some((2026, 10, 23, Some((11, 0)))),
        false,
    );
}

#[test]
fn month_day_order() {
    let opts = ParseOptions {
        date_order: DateOrder::MonthDay,
        ..Default::default()
    };
    check_with(
        "Flight 7/11",
        "Flight",
        (2027, 7, 11, None),
        None,
        true,
        &opts,
    );
}

#[test]
fn time_spans() {
    let r = p("Meeting tomorrow at 3pm");
    assert_eq!(
        r.spans,
        vec![
            Span {
                start: 0,
                end: 7,
                label: Label::Title
            },
            Span {
                start: 8,
                end: 16,
                label: Label::Date
            },
            Span {
                start: 17,
                end: 19,
                label: Label::Separator
            },
            Span {
                start: 20,
                end: 23,
                label: Label::Time
            },
        ]
    );
}

fn rrule(input: &str) -> Option<String> {
    p(input).recurrence.map(|r| r.to_rrule())
}

#[test]
fn durations() {
    check(
        "Gym in 3 days for 2 hours at 7am",
        "Gym",
        (2026, 10, 23, Some((7, 0))),
        Some((2026, 10, 23, Some((9, 0)))),
        false,
    );
    check(
        "Call tomorrow 4pm for 30 min",
        "Call",
        (2026, 10, 21, Some((16, 0))),
        Some((2026, 10, 21, Some((16, 30)))),
        false,
    );
    check(
        "Nap at 2 for an hour",
        "Nap",
        (2026, 10, 20, Some((14, 0))),
        Some((2026, 10, 20, Some((15, 0)))),
        false,
    );
    check(
        "Break at 3pm for half an hour",
        "Break",
        (2026, 10, 20, Some((15, 0))),
        Some((2026, 10, 20, Some((15, 30)))),
        false,
    );
    check(
        "Hike saturday 8am for 1.5 hours",
        "Hike",
        (2026, 10, 24, Some((8, 0))),
        Some((2026, 10, 24, Some((9, 30)))),
        false,
    );
}

#[test]
fn recurrence_rules() {
    let r = p("Standup every weekday at 9:30 /work");
    assert_eq!(r.title, "Standup");
    assert_eq!(r.calendar.as_deref(), Some("Work"));
    assert_eq!(
        r.recurrence.as_ref().map(|x| x.to_rrule()).as_deref(),
        Some("FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,TU,WE,TH,FR")
    );
    assert_eq!(r.start, Some(w(2026, 10, 21, Some((9, 30)))));
    assert_eq!(r.end, Some(w(2026, 10, 21, Some((10, 30)))));

    let r = p("Pay rent monthly on the 1st");
    assert_eq!(r.title, "Pay rent");
    assert_eq!(
        rrule("Pay rent monthly on the 1st").as_deref(),
        Some("FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=1")
    );
    assert_eq!(r.start, Some(w(2026, 11, 1, None)));
    assert!(r.all_day);

    let r = p("Team sync every 2 weeks on monday at 10 until dec 31");
    assert_eq!(r.title, "Team sync");
    assert_eq!(
        r.recurrence.as_ref().map(|x| x.to_rrule()).as_deref(),
        Some("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO;UNTIL=20261231")
    );
    assert_eq!(r.start, Some(w(2026, 10, 26, Some((10, 0)))));

    let r = p("Water plants every day");
    assert_eq!(r.title, "Water plants");
    assert_eq!(
        rrule("Water plants every day").as_deref(),
        Some("FREQ=DAILY;INTERVAL=1")
    );
    assert_eq!(r.start, Some(w(2026, 10, 20, None)));

    assert_eq!(
        rrule("Piano every tuesday and thursday 5pm").as_deref(),
        Some("FREQ=WEEKLY;INTERVAL=1;BYDAY=TU,TH")
    );
    assert_eq!(
        p("Piano every tuesday and thursday 5pm").start,
        Some(w(2026, 10, 20, Some((17, 0))))
    );
    assert_eq!(
        rrule("Report weekly").as_deref(),
        Some("FREQ=WEEKLY;INTERVAL=1")
    );
    assert_eq!(
        rrule("Backup every 3 days").as_deref(),
        Some("FREQ=DAILY;INTERVAL=3")
    );
    assert_eq!(
        rrule("Birthday yearly on oct 30").as_deref(),
        Some("FREQ=YEARLY;INTERVAL=1")
    );
    assert_eq!(
        p("Birthday yearly on oct 30").start,
        Some(w(2026, 10, 30, None))
    );
    assert_eq!(
        rrule("Brunch every weekend").as_deref(),
        Some("FREQ=WEEKLY;INTERVAL=1;BYDAY=SA,SU")
    );
    assert_eq!(
        rrule("Swim mondays at 7am").as_deref(),
        Some("FREQ=WEEKLY;INTERVAL=1;BYDAY=MO")
    );
    assert_eq!(
        rrule("Bills every month on the 31st").as_deref(),
        Some("FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31")
    );
    assert_eq!(
        p("Bills every month on the 31st").start,
        Some(w(2026, 10, 31, None))
    );
}

#[test]
fn alerts() {
    let r = p("Party friday 9pm alert 1 hour before");
    assert_eq!(r.title, "Party");
    assert_eq!(r.alerts, vec![AlertSpec::Before(60)]);
    let r = p("Dentist tomorrow 3pm remind me 30 min before");
    assert_eq!(r.kind, ItemKind::Event);
    assert_eq!(r.title, "Dentist");
    assert_eq!(r.alerts, vec![AlertSpec::Before(30)]);
    let r = p("Flight friday 6am alert 2 hours before alert 1 day before");
    assert_eq!(
        r.alerts,
        vec![AlertSpec::Before(120), AlertSpec::Before(1440)]
    );
    let r = p("Meeting friday 3pm alert at 2pm");
    assert_eq!(
        r.alerts,
        vec![AlertSpec::At(w(2026, 10, 23, Some((14, 0))))]
    );
}

#[test]
fn url_and_calendar() {
    let r = p("Standup tomorrow 9am https://meet.example.com/abc");
    assert_eq!(r.url.as_deref(), Some("https://meet.example.com/abc"));
    assert_eq!(r.title, "Standup");
    let r = p("/nonexistent lunch");
    assert_eq!(r.calendar, None);
    assert_eq!(r.title, "/nonexistent lunch");
    let r = p("lunch /PERSONAL");
    assert_eq!(r.calendar.as_deref(), Some("Personal"));
    assert_eq!(r.title, "lunch");
}

#[test]
fn people_and_places() {
    let r = p("Lunch with Sam tomorrow at 1");
    assert_eq!(r.title, "Lunch");
    assert_eq!(r.attendees, vec!["Sam"]);
    assert_eq!(r.start, Some(w(2026, 10, 21, Some((13, 0)))));

    let r = p("Dinner with Anna, Marco and Luca friday 8pm at Trattoria da Enzo");
    assert_eq!(r.title, "Dinner");
    assert_eq!(r.attendees, vec!["Anna", "Marco", "Luca"]);
    assert_eq!(r.location.as_deref(), Some("Trattoria da Enzo"));
    assert_eq!(r.start, Some(w(2026, 10, 23, Some((20, 0)))));

    let r = p("Meet at the station");
    assert_eq!(r.title, "Meet");
    assert_eq!(r.location.as_deref(), Some("the station"));
    assert_eq!(r.start, Some(w(2026, 10, 20, None)));
    assert!(r.all_day);

    let r = p("Dentist Friday 3pm at Via Roma 12");
    assert_eq!(r.title, "Dentist");
    assert_eq!(r.location.as_deref(), Some("Via Roma 12"));

    let r = p("Call in 2 days");
    assert_eq!(r.location, None);
    assert_eq!(r.title, "Call");
    assert_eq!(r.start, Some(w(2026, 10, 22, None)));

    let r = p("Coffee @ Blue Bottle 9am tomorrow");
    assert_eq!(r.title, "Coffee");
    assert_eq!(r.location.as_deref(), Some("Blue Bottle"));
    assert_eq!(r.start, Some(w(2026, 10, 21, Some((9, 0)))));

    let r = p("Workshop in Milan next friday");
    assert_eq!(r.location.as_deref(), Some("Milan"));
    assert_eq!(r.title, "Workshop");

    let r = p("Call with Bob & Alice");
    assert_eq!(r.attendees, vec!["Bob", "Alice"]);
    assert_eq!(r.title, "Call");
}

#[test]
fn tasks_end_to_end() {
    let r = p("todo buy milk friday");
    assert_eq!(r.kind, ItemKind::Task);
    assert_eq!(r.title, "buy milk");
    assert_eq!(r.start, Some(w(2026, 10, 23, None)));
    assert_eq!(r.end, None);

    let r = p("remind me to call mom at 6pm");
    assert_eq!(r.kind, ItemKind::Task);
    assert_eq!(r.title, "call mom");
    assert_eq!(r.start, Some(w(2026, 10, 20, Some((18, 0)))));
    assert_eq!(r.end, None);

    let r = p("task: file taxes");
    assert_eq!(r.kind, ItemKind::Task);
    assert_eq!(r.title, "file taxes");
    assert_eq!(r.start, None);
}

#[test]
fn unicode_location_and_spans() {
    let input = "Caffè con Zoë 🍰 domani at Caffè Sant'Eustachio";
    let r = p(input);
    assert_eq!(r.location.as_deref(), Some("Caffè Sant'Eustachio"));
    assert_eq!(r.title, "Caffè con Zoë 🍰 domani");
    for s in &r.spans {
        assert!(input.is_char_boundary(s.start) && input.is_char_boundary(s.end));
    }
}

#[test]
fn empty_and_whitespace() {
    for input in ["", "   "] {
        let r = p(input);
        assert_eq!(r.title, "");
        assert!(r.spans.is_empty());
    }
}

/// Every base case must parse the same when lowercased, uppercased or with
/// trailing punctuation.
#[test]
fn casing_and_punctuation_variants() {
    let base = [
        "Meeting tomorrow at 3pm",
        "Dinner at 7",
        "Lunch at noon",
        "Review friday 14:30",
        "Party next friday 9pm",
        "Yoga 6-7pm",
        "Night shift 10pm to 6am",
        "Movie tonight",
        "Trip Oct 24 to Oct 26",
        "Call in 2 weeks",
        "Pay bills the 25th",
        "Sync next week",
        "Gym in 3 days for 2 hours at 7am",
        "Standup every weekday at 9:30",
        "Lunch with Sam tomorrow at 1",
        "Meet at the station",
        "Dentist Friday 3pm at Via Roma 12",
        "Exam 3 March",
        "Walk in 3 days",
        "Session 3pm to 5pm",
        "Brunch sunday at 11am",
        "Concert at 9",
        "Report due december 3",
        "Holiday Dec 24 - Dec 26",
        "Plan next month",
        "Water plants every day",
        "Swim mondays at 7am",
        "Party friday 9pm alert 1 hour before",
        "Call friday 10a",
        "Lunch tomorrow at 12:30pm",
    ];
    for b in base {
        let want = p(b);
        for v in [b.to_lowercase(), format!("{b}."), format!("  {b}  ")] {
            let got = p(&v);
            assert_eq!(
                (got.start, got.end, got.all_day),
                (want.start, want.end, want.all_day),
                "variant {v:?}"
            );
            assert_eq!(
                got.title.to_lowercase(),
                want.title.to_lowercase(),
                "variant {v:?}"
            );
            assert_eq!(got.recurrence, want.recurrence, "variant {v:?}");
        }
    }
}
