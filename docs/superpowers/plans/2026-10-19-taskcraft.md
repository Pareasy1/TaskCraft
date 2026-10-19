# TaskCraft Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and install `TaskCraft.app`, a personal macOS calendar/task app with a Fantastical-style menu-bar quick entry, natural-language parsing, local SQLite storage, Day/Week/Month views and notifications.

**Architecture:** Cargo workspace with two pure library crates (`taskcraft-parse`, `taskcraft-store`) and a Tauri 2 binary (`src-tauri`) that exposes them to a Svelte 5 + TypeScript UI through Tauri commands. One UI bundle serves three windows (`quick`, `main`, `settings`), selected by window label.

**Tech Stack:** Rust stable, chrono 0.4, chrono-tz 0.10, rusqlite 0.32 (`bundled`), rrule 0.13, serde, thiserror, tracing + tracing-appender, proptest; Tauri 2 with plugins global-shortcut, notification, autostart, positioner (`tray-icon`), dialog; Svelte 5, Vite, TypeScript, vitest, lucide-svelte.

**Spec:** `docs/superpowers/specs/2026-10-19-taskcraft-design.md`

## Global Constraints

- Clean-room: no Flexibits code, weights, data files, prompts, icons or assets.
- Storage: local SQLite only, at `~/Library/Application Support/TaskCraft/taskcraft.db`; WAL mode, `foreign_keys=ON`; schema version in `PRAGMA user_version`; copy to `taskcraft.db.bak-v<old>` before migrating.
- Timestamps stored as RFC 3339 UTC strings; `tz` column holds the IANA zone.
- Parser: English only; `parse` never panics, never fails, is deterministic; spans non-overlapping, sorted, on UTF-8 boundaries.
- Default hotkey ⌥⌘Space (`Alt+Super+Space`); default event length 60 min; seed calendars "Personal" (default) and "Work".
- Popover ~360×520, borderless, always-on-top, vibrancy, hides on blur.
- Scheduler tick 30 s and on wake; alerts more than 12 h late are marked fired without notifying.
- Logs: `~/Library/Logs/TaskCraft/taskcraft.log`, daily rotation, keep 7.
- Command errors returned as `{ code, message }`; no panics across the command boundary.
- Target macOS 27 Apple Silicon, minimum macOS 14; ad-hoc signature; bundle id `com.pareasy.taskcraft`.
- Parser test clock: `now = 2026-10-20T10:00 Europe/Rome` (Tuesday), `date_order = DayMonth`, calendars `["Personal","Work"]`.

## Review Focus

1. **DST transitions** — an event at 02:30 on 2026-10-25 (Europe/Rome, ambiguous) must resolve to the first 02:30 (00:30 UTC); 02:30 on 2027-03-28 (nonexistent) must become 03:30 local. Pinned in Task 6 (`to_utc_handles_dst`).
2. **Multi-day and all-day items crossing a view boundary** — an event from Oct 24 to Oct 26 must appear in a range query for Oct 25 only. Pinned in Task 7 (`range_includes_overlapping_multiday`).
3. **Monthly recurrence on the 31st** — "every month on the 31st" must skip months without a 31st, never land on the 30th or 1st. Pinned in Task 7 (`monthly_31st_skips_short_months`).
4. **Non-ASCII input** — accented names, emoji and Italian place names ("Caffè Sant'Eustachio") must keep valid spans and appear verbatim in title/location. Pinned in Task 5 (`unicode_location_and_spans`) and the proptest.
5. **Mac asleep for a long time** — on wake, alerts older than 12 h are silently marked fired; alerts within 12 h fire once, never twice. Pinned in Task 8 (`stale_alerts_skipped_recent_fire_once`).

---

## File Structure

```
Cargo.toml                         workspace (members: crates/*, src-tauri)
crates/taskcraft-parse/
  src/lib.rs                       pub API: parse, re-exports
  src/types.rs                     ParsedItem, When, Label, Span, Recurrence, AlertSpec, ParseOptions, DateOrder, ItemKind
  src/token.rs                     tokenizer
  src/rules/mod.rs                 tagging driver (longest match, ordered rules)
  src/rules/prefix.rs              task prefix
  src/rules/datetime.rs            dates, times, ranges, tonight
  src/rules/extras.rs              duration, recurrence, alerts, url, calendar
  src/rules/fields.rs              location, attendees, title assembly
  src/resolve.rs                   labels -> concrete When values
  tests/table.rs                   table-driven cases
  tests/props.rs                   proptest invariants
crates/taskcraft-store/
  src/lib.rs                       Store, pub types
  src/schema.rs                    migrations
  src/model.rs                     Calendar, EventInput, TaskInput, Occurrence, Task, Hit, Scope, TaskFilter, DueAlert
  src/time.rs                      to_utc, from_parsed
  src/events.rs  src/tasks.rs  src/calendars.rs
  src/recur.rs                     occurrence expansion
  src/alerts.rs                    materialization, due_alerts
  src/ics.rs                       export
  tests/*.rs
src-tauri/
  Cargo.toml  tauri.conf.json  capabilities/default.json  icons/
  src/main.rs                      builder, plugins, tray, windows
  src/state.rs                     AppState { store: Mutex<Store>, settings: RwLock<Settings> }
  src/settings.rs                  Settings load/save (settings.json)
  src/commands.rs                  all #[tauri::command]s
  src/scheduler.rs                 alert thread
  src/logging.rs
ui/
  package.json vite.config.ts index.html tsconfig.json
  src/main.ts                      mounts view by window label
  src/lib/api.ts                   typed wrappers over invoke
  src/lib/types.ts                 TS mirrors of Rust types
  src/lib/layout.ts                layoutDay, monthGrid, weekDays
  src/lib/layout.test.ts
  src/quick/Quick.svelte  HighlightInput.svelte  PreviewCard.svelte  MiniMonth.svelte  Upcoming.svelte
  src/main/Main.svelte  Sidebar.svelte  Toolbar.svelte  DayWeekGrid.svelte  MonthGrid.svelte  Inspector.svelte  TaskList.svelte
  src/settings/Settings.svelte
  src/styles.css                   tokens, light/dark, label colors
scripts/install.sh                 build, sign, copy, launch
scripts/smoke.sh                   post-install smoke test
```

---

### Task 1: Toolchain and parser types

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `crates/taskcraft-parse/Cargo.toml`, `crates/taskcraft-parse/src/lib.rs`, `crates/taskcraft-parse/src/types.rs`
- Test: `crates/taskcraft-parse/tests/table.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn parse(input: &str, now: chrono::DateTime<chrono_tz::Tz>, opts: &ParseOptions) -> ParsedItem;
  pub struct ParseOptions { pub default_event_minutes: u32, pub calendars: Vec<String>, pub date_order: DateOrder }
  pub enum DateOrder { DayMonth, MonthDay }
  pub enum ItemKind { Event, Task }
  pub struct When { pub date: NaiveDate, pub time: Option<NaiveTime> }
  pub enum Label { Title, TaskPrefix, Date, Time, Duration, Recurrence, Alert, Location, Attendees, Calendar, Url, Separator }
  pub struct Span { pub start: usize, pub end: usize, pub label: Label }
  pub enum Recurrence { Daily{interval:u32}, Weekly{interval:u32, days:Vec<Weekday>}, Monthly{interval:u32, month_day:Option<u32>}, Yearly{interval:u32} } + until: carried as RecurrenceSpec { rule: Recurrence, until: Option<NaiveDate> }
  impl RecurrenceSpec { pub fn to_rrule(&self) -> String }   // e.g. "FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,TU,WE,TH,FR"
  pub enum AlertSpec { Before(u32 /*minutes*/), At(When) }
  pub struct ParsedItem { kind, title: String, start: Option<When>, end: Option<When>, all_day: bool, location: Option<String>, attendees: Vec<String>, calendar: Option<String>, recurrence: Option<RecurrenceSpec>, alerts: Vec<AlertSpec>, url: Option<String>, spans: Vec<Span> }
  ```
  All types derive `Debug, Clone, PartialEq, Serialize, Deserialize`; enums serialize as `camelCase` strings; `ParseOptions: Default` (60, ["Personal","Work"], DayMonth).

- [ ] **Step 1: Install toolchain** — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable` then `source ~/.cargo/env && cargo --version`. Expected: cargo 1.9x.
- [ ] **Step 2: Create workspace** — root `Cargo.toml` with `[workspace] resolver = "2"`, `members = ["crates/*", "src-tauri"]` (add `src-tauri` in Task 9; start with `crates/*`), shared `[workspace.dependencies]` for chrono (`serde`), chrono-tz (`serde`), serde (`derive`). `.gitignore`: `target/`, `ui/node_modules/`, `ui/dist/`.
- [ ] **Step 3: Write failing test** in `tests/table.rs`: helper `fn now() -> DateTime<Tz>` returning the Global Constraints clock; `fn p(s:&str)->ParsedItem` using default options. Test `plain_title_is_all_day_today`: `p("Brunch")` → kind Event, title "Brunch", start `When{2026-10-20, None}`, all_day true, spans `[Span{0,6,Title}]`.
- [ ] **Step 4: Run** `cargo test -p taskcraft-parse` — Expected: compile error (`parse` missing).
- [ ] **Step 5: Implement types in `types.rs`** and a `parse` that returns the whole trimmed input as title with that default. **Run** — Expected: PASS.
- [ ] **Step 6: Commit** `feat(parse): workspace and parser types`.

### Task 2: Tokenizer and task prefix

**Files:**
- Create: `src/token.rs`, `src/rules/mod.rs`, `src/rules/prefix.rs`
- Modify: `src/lib.rs`
- Test: `tests/table.rs`

**Interfaces:**
- Produces:
  ```rust
  pub(crate) struct Token { pub text: String, pub lower: String, pub start: usize, pub end: usize, pub kind: TokKind } // TokKind { Word, Number, Punct }
  pub(crate) fn tokenize(s: &str) -> Vec<Token>;     // words keep internal ' and - ("Sant'Eustachio", "to-do"); "3pm", "10:30", "7/11", "2026-12-01" stay single tokens
  pub(crate) struct Tagged { pub label: Option<Label>, pub payload: Payload }  // one per token
  pub(crate) trait Rule { fn try_match(&self, toks:&[Token], i:usize, ctx:&Ctx) -> Option<Match> } // Match { len: usize, labels: Vec<(usize /*token offset*/, Label)>, payload: Payload }
  pub(crate) fn run_rules(toks:&[Token], rules:&[&dyn Rule], ctx:&Ctx) -> Vec<Tagged>; // at each untagged position try every rule, keep the longest match; ties -> earlier rule in list
  ```
  `Payload` is an enum filled in by later tasks (`None` for now).

- [ ] **Step 1: Failing tests**: `tokenize_keeps_compound_tokens` (`"Call mom at 3pm, 7/11!"` → texts `["Call","mom","at","3pm",",","7/11","!"]` with correct byte offsets); `task_prefixes` table: `"todo buy milk"`, `"To-do: buy milk"`, `"remind me to buy milk"`, `"✓ buy milk"` → kind Task, title "buy milk", first span label TaskPrefix covering the prefix (and colon).
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** tokenizer, driver, and prefix detection (prefix list from spec §4.1, longest first, only at input start).
- [ ] **Step 4: Run** — Expected: PASS (Task 1 test still passes).
- [ ] **Step 5: Commit** `feat(parse): tokenizer and task prefixes`.

### Task 3: Dates, times, ranges and resolution

**Files:**
- Create: `src/rules/datetime.rs`, `src/resolve.rs`
- Test: `tests/table.rs`

**Interfaces:**
- Consumes: Task 2 driver.
- Produces: rules `DateRule`, `TimeRule`, `TimeRangeRule`, `DateRangeRule`, `TonightRule`; `Payload::{Date(DateExpr), Time(TimeExpr), TimeRange(TimeExpr,TimeExpr), DateRange(DateExpr,DateExpr)}`; `pub(crate) fn resolve(tagged:&[Tagged], toks:&[Token], title_words:&str, now:DateTime<Tz>, opts:&ParseOptions) -> Resolved { start, end, all_day }`.
  `DateExpr` variants: `Today, Tomorrow, Yesterday, Weekday{day, next:bool}, NextWeek, NextMonth, InDays(u32), InWeeks(u32), MonthDay{month, day, year:Option<i32>}, Numeric{a,b,year:Option<i32>}, Ordinal(u32), Iso(NaiveDate)`. `TimeExpr { hour:u32, minute:u32, meridiem: Option<Am|Pm>, bare: bool }`.

- [ ] **Step 1: Failing table tests** (each asserts kind, title, start, end, all_day):

  | input | title | start | end | all_day |
  |---|---|---|---|---|
  | `Meeting tomorrow at 3pm` | Meeting | 10-21 15:00 | 10-21 16:00 | false |
  | `Dinner at 7` | Dinner | 10-20 19:00 | 10-20 20:00 | false |
  | `Meeting at 9` | Meeting | 10-21 09:00 | 10-21 10:00 | false |
  | `Call at 5` | Call | 10-20 17:00 | 10-20 18:00 | false |
  | `Lunch at noon` | Lunch | 10-20 12:00 | 10-20 13:00 | false |
  | `Review friday 14:30` | Review | 10-23 14:30 | 10-23 15:30 | false |
  | `Party next friday 9pm` | Party | 10-30 21:00 | 10-30 22:00 | false |
  | `Coffee tuesday 11am` | Coffee | 10-20 11:00 | 10-20 12:00 | false |
  | `Coffee tuesday 9am` | Coffee | 10-27 09:00 | 10-27 10:00 | false |
  | `Yoga 6-7pm` | Yoga | 10-20 18:00 | 10-20 19:00 | false |
  | `Workshop from 2 to 4:30pm thursday` | Workshop | 10-22 14:00 | 10-22 16:30 | false |
  | `Night shift 10pm to 6am` | Night shift | 10-20 22:00 | 10-21 06:00 | false |
  | `Movie tonight` | Movie | 10-20 20:00 | 10-20 21:00 | false |
  | `Flight 7/11 8am-10:30am` | Flight | 11-07 08:00 | 11-07 10:30 | false |
  | `Trip Oct 24 to Oct 26` | Trip | 10-24 | 10-26 | true |
  | `Exam 3 March` | Exam | 2027-03-03 | — | true |
  | `Recap Oct 1` | Recap | 2027-10-01 | — | true |
  | `Review 2026-12-01 at 10:00` | Review | 12-01 10:00 | 12-01 11:00 | false |
  | `Call in 2 weeks` | Call | 11-03 | — | true |
  | `Pay bills the 25th` | Pay bills | 10-25 | — | true |
  | `Sync next week` | Sync | 10-26 (Monday) | — | true |

  Plus `month_day_order`: with `DateOrder::MonthDay`, `Flight 7/11` → 2027-07-11 all-day. Plus `time_spans`: `p("Meeting tomorrow at 3pm").spans` == `[Title 0..7, Date 8..16, Separator 17..19, Time 20..23]`.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** rules and `resolve` per spec §4 "Resolve" (bare 1–6 → pm, 7–11 → am unless title contains dinner/drinks/party/concert/movie/show → pm; `at` before a time is a Separator span; weekday without `next` includes today only when the resolved time has not passed, otherwise +7; date-only → all_day; time without date → today or tomorrow if passed; a range's explicit meridiem propagates to a bare start when that keeps start ≤ end (`6-7pm` → 18:00); end < start → end +1 day; missing end → start + `default_event_minutes`; all-day ranges keep the end date inclusive).
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** `feat(parse): dates, times and ranges`.

### Task 4: Duration, recurrence, alerts, calendar, URL

**Files:**
- Create: `src/rules/extras.rs`
- Modify: `src/resolve.rs`
- Test: `tests/table.rs`

**Interfaces:**
- Produces: `Payload::{Duration(u32 min), Recurrence(RecurrenceSpec), Alert(AlertSpec), Calendar(String), Url(String)}`; duration overrides default end; recurrence with no explicit date sets start to the first matching occurrence ≥ now.

- [ ] **Step 1: Failing tests:**
  - `Gym in 3 days for 2 hours at 7am` → start 10-23 07:00, end 10-23 09:00.
  - `Standup every weekday at 9:30 /work` → title "Standup", calendar Some("Work"), rrule `FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,TU,WE,TH,FR`, start 10-21 09:30, end 10:30.
  - `Pay rent monthly on the 1st` → rrule `FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=1`, start 2026-11-01 all-day.
  - `Team sync every 2 weeks on monday at 10 until dec 31` → `FREQ=WEEKLY;INTERVAL=2;BYDAY=MO;UNTIL=20261231`, start 10-26 10:00.
  - `Water plants every day` → `FREQ=DAILY;INTERVAL=1`, start 10-20 all-day.
  - `Party friday 9pm alert 1 hour before` → alerts `[Before(60)]`; `Dentist tomorrow 3pm remind me 30 min before` → `[Before(30)]` and kind stays Event (prefix only at start).
  - `Standup tomorrow 9am https://meet.example.com/abc` → url Some, title "Standup".
  - `/nonexistent lunch` → calendar None, "/nonexistent" stays in title.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** rules (recurrence also accepts `every N weeks on <weekday>[ and <weekday>]`) in the order listed in spec §4.3 (URL, Calendar, Recurrence, Alert, Duration before time/date rules in the driver list).
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** `feat(parse): duration, recurrence, alerts, calendar, url`.

### Task 5: Location, attendees, title assembly, invariants

**Files:**
- Create: `src/rules/fields.rs`, `tests/props.rs`
- Modify: `src/lib.rs`
- Test: `tests/table.rs`, `tests/props.rs`

**Interfaces:**
- Produces: final `parse` pipeline: prefix → tokenize → tag (extras, datetime) → fields (attendees, location over remaining untagged tokens) → title from untagged tokens (separators that didn't start a field fold into the title; trailing separators trimmed) → resolve → spans (merge adjacent same-label tokens, separators get their own `Separator` span).

- [ ] **Step 1: Failing tests:**
  - `Lunch with Sam tomorrow at 1` → title "Lunch", attendees ["Sam"], start 10-21 13:00.
  - `Dinner with Anna, Marco and Luca friday 8pm at Trattoria da Enzo` → attendees ["Anna","Marco","Luca"], location "Trattoria da Enzo", start 10-23 20:00.
  - `Meet at the station` → title "Meet", location "the station", all-day today.
  - `Dentist Friday 3pm at Via Roma 12` → location "Via Roma 12".
  - `Call in 2 days` → no location; title "Call"; start 10-22 all-day.
  - `todo buy milk friday` → Task, title "buy milk", start 10-23 no time.
  - `remind me to call mom at 6pm` → Task, title "call mom", start 10-20 18:00, end None.
  - `unicode_location_and_spans`: `Caffè con Zoë 🍰 domani at Caffè Sant'Eustachio` → location "Caffè Sant'Eustachio"; every span boundary satisfies `input.is_char_boundary`.
  - `empty_and_whitespace`: `""` and `"   "` → title "", spans empty, no panic.
- [ ] **Step 2: proptest** `parse_never_panics_and_spans_valid`: arbitrary `String` (including `\\PC{0,80}`) → parse returns; spans sorted, non-overlapping, on char boundaries, within input length.
- [ ] **Step 3: Run** — Expected: FAIL.
- [ ] **Step 4: Implement**. **Run** `cargo test -p taskcraft-parse` — Expected: all PASS; table has ≥150 cases total (add variants of the above rows for casing, punctuation and word order until it does).
- [ ] **Step 5: Commit** `feat(parse): location, attendees, title; invariants`.

### Task 6: Store schema, time conversion, CRUD

**Files:**
- Create: `crates/taskcraft-store/{Cargo.toml, src/lib.rs, src/schema.rs, src/model.rs, src/time.rs, src/calendars.rs, src/events.rs, src/tasks.rs}`
- Test: `crates/taskcraft-store/tests/crud.rs`

**Interfaces:**
- Consumes: `taskcraft_parse::{ParsedItem, When, AlertSpec}`.
- Produces:
  ```rust
  pub struct Store { /* rusqlite::Connection */ }
  impl Store {
    pub fn open(path:&Path) -> Result<Store>;          // creates dirs, backup-then-migrate, WAL, FKs, seeds calendars
    pub fn open_in_memory() -> Result<Store>;
    pub fn calendars(&self) -> Result<Vec<Calendar>>;   // Calendar { id, name, color, visible, sort }
    pub fn upsert_calendar(&self, c:&CalendarInput) -> Result<i64>;  // CalendarInput { id:Option<i64>, name, color, visible }
    pub fn delete_calendar(&self, id:i64, move_to:i64) -> Result<()>;
    pub fn insert_event(&self, e:&EventInput) -> Result<i64>;
    pub fn update_event(&self, id:i64, e:&EventInput) -> Result<()>;
    pub fn get_event(&self, id:i64) -> Result<Event>;
    pub fn insert_task(&self, t:&TaskInput) -> Result<i64>;
    pub fn update_task(&self, id:i64, t:&TaskInput) -> Result<()>;
    pub fn set_task_completed(&self, id:i64, done:bool, now:DateTime<Utc>) -> Result<()>;
    pub fn delete_task(&self, id:i64) -> Result<()>;
  }
  pub struct EventInput { calendar_id:i64, title, start:DateTime<Utc>, end:DateTime<Utc>, tz:String, all_day:bool, location:Option<String>, attendees:Vec<String>, notes:Option<String>, url:Option<String>, rrule:Option<String>, alerts:Vec<AlertSpecUtc> }
  pub struct TaskInput  { calendar_id:i64, title, due:Option<DateTime<Utc>>, due_has_time:bool, tz:String, rrule:Option<String>, notes:Option<String>, alerts:Vec<AlertSpecUtc> }
  pub enum AlertSpecUtc { Before(u32), At(DateTime<Utc>) }
  pub enum NewItem { Event(EventInput), Task(TaskInput) }
  pub fn to_utc(w:&When, tz:Tz) -> DateTime<Utc>;      // date-only -> local midnight; ambiguous -> earliest; nonexistent -> +1h
  pub fn from_parsed(p:&ParsedItem, tz:Tz, default_calendar_id:i64, calendars:&[Calendar], default_event_minutes:u32) -> NewItem;
  pub type Result<T> = std::result::Result<T, StoreError>; // thiserror: Db, NotFound, Invalid(String)
  ```
  All-day events store `start` = local midnight of first day and `end` = local midnight after the last day, both in UTC.

- [ ] **Step 1: Failing tests:** `open_seeds_calendars` (names ["Personal","Work"], `user_version` = 1); `migration_backs_up` (open file db at version 0 created by hand → file `taskcraft.db.bak-v0` exists after `open`); `event_roundtrip` (insert then `get_event` equal, attendees preserved); `task_complete_toggle`; `delete_calendar_moves_items` (event in Work, delete Work moving to Personal → event's calendar_id is Personal); `to_utc_handles_dst` (2026-10-25 02:30 Rome → `2026-10-25T00:30:00Z`; 2027-03-28 02:30 Rome → `2027-03-28T01:30:00Z`); `from_parsed_all_day_range` (parse "Trip Oct 24 to Oct 26" → start `2026-10-23T22:00:00Z`, end `2026-10-26T23:00:00Z`).
- [ ] **Step 2: Run** `cargo test -p taskcraft-store` — Expected: FAIL.
- [ ] **Step 3: Implement** schema exactly as spec §3; alerts table written by `insert_*`/`update_*` (replace all alerts for the owner).
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** `feat(store): schema, conversion and CRUD`.

### Task 7: Recurrence expansion, range queries, edits by scope, search

**Files:**
- Create: `src/recur.rs`
- Modify: `src/events.rs`, `src/tasks.rs`, `src/lib.rs`
- Test: `tests/range.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Occurrence { event_id:i64, calendar_id:i64, title, start:DateTime<Utc>, end:DateTime<Utc>, all_day:bool, location:Option<String>, recurring:bool, color:String }
  pub enum Scope { This, All }
  pub enum TaskFilter { Overdue, Today, Upcoming, Completed, All }
  pub struct Task { id, calendar_id, title, due:Option<DateTime<Utc>>, due_has_time:bool, completed_at:Option<DateTime<Utc>>, recurring:bool, notes:Option<String>, color:String }
  pub struct Hit { kind: ItemKindDb /*Event|Task*/, id:i64, title, when:Option<DateTime<Utc>> }
  impl Store {
    pub fn list_range(&self, start:DateTime<Utc>, end:DateTime<Utc>) -> Result<Vec<Occurrence>>; // overlap test: occ.start < end && occ.end > start; only visible calendars; sorted by start
    pub fn list_tasks(&self, f:TaskFilter, now:DateTime<Utc>, tz:Tz) -> Result<Vec<Task>>;
    pub fn move_event(&self, id:i64, occurrence:Option<DateTime<Utc>>, new_start:DateTime<Utc>, new_end:DateTime<Utc>, scope:Scope) -> Result<i64>; // This on a recurring event: add EXDATE to parent, insert detached copy, return its id
    pub fn delete_event(&self, id:i64, occurrence:Option<DateTime<Utc>>, scope:Scope) -> Result<()>;
    pub fn search(&self, q:&str) -> Result<Vec<Hit>>;   // case-insensitive LIKE on title, location, notes; limit 50
  }
  ```
  `rrule` column stores the RRULE body; EXDATEs are appended as `\nEXDATE:<UTC list>`; expansion builds an `rrule::RRuleSet` with `DTSTART;TZID=<tz>` so wall-clock times survive DST.

- [ ] **Step 1: Failing tests:** `range_includes_overlapping_multiday` (Oct 24–26 all-day event returned for range Oct 25 00:00–24:00 local); `weekly_expansion_keeps_wall_clock_across_dst` (every Monday 09:00 Rome from Oct 19: occurrences Oct 19 07:00Z, Oct 26 08:00Z); `monthly_31st_skips_short_months` (from 2026-10-31, next 3 → Oct 31, Dec 31, Jan 31); `until_is_respected`; `move_this_occurrence_detaches` (range afterwards shows moved copy and no original at old time); `delete_all_removes_series`; `task_filters` (overdue/today/upcoming/completed with fixed now); `search_matches_location`.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** `feat(store): recurrence, ranges, scoped edits, search`.

### Task 8: Alerts and ICS export

**Files:**
- Create: `src/alerts.rs`, `src/ics.rs`
- Test: `tests/alerts.rs`, `tests/ics.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct DueAlert { alert_id:i64, owner: ItemKindDb, owner_id:i64, title:String, when:DateTime<Utc>, location:Option<String> }
  impl Store {
    pub fn due_alerts(&self, now:DateTime<Utc>) -> Result<Vec<DueAlert>>; // marks fire_at < now-12h as fired and excludes them
    pub fn mark_fired(&self, alert_id:i64, now:DateTime<Utc>) -> Result<()>; // for recurring owners, materializes the alert for the next occurrence
    pub fn export_ics(&self) -> Result<String>;   // VCALENDAR with VEVENT (DTSTART/DTEND/RRULE/EXDATE/LOCATION/DESCRIPTION/URL/VALARM) and VTODO (DUE/STATUS/COMPLETED)
  }
  ```
  Insert/update of a task with `due_has_time` adds an implicit `At(due)` alert.

- [ ] **Step 1: Failing tests:** `stale_alerts_skipped_recent_fire_once` (alerts at now-13h and now-1h → `due_alerts` returns only the second; after `mark_fired` a second call returns nothing; the first has `fired_at` set); `before_offset_computes_fire_at` (event 15:00Z with Before(10) → fire_at 14:50Z); `recurring_alert_rematerializes` (daily 09:00 with Before(5): after firing day 1, next due alert is day 2 08:55 local); `task_due_time_implicit_alert`; `ics_contains_expected_lines` (export contains `BEGIN:VEVENT`, `RRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO`, `BEGIN:VTODO`, `TRIGGER:-PT10M`, and lines are folded at 75 octets).
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(store): alerts and ics export`.

### Task 9: Tauri app shell, settings, logging, commands

**Files:**
- Create: `src-tauri/{Cargo.toml, build.rs, tauri.conf.json, capabilities/default.json, src/main.rs, src/state.rs, src/settings.rs, src/commands.rs, src/logging.rs}`, `src-tauri/icons/*` (generated with `cargo tauri icon` from an original 1024px SVG `assets/taskcraft.svg` drawn in this task: rounded square, gradient, white calendar glyph), `ui/` scaffold (`npm create vite@latest ui -- --template svelte-ts`), `ui/src/lib/{api.ts,types.ts}`
- Modify: root `Cargo.toml` members.
- Test: `src-tauri/src/commands.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: Tasks 1–8.
- Produces:
  ```rust
  pub struct Settings { hotkey:String /*"Alt+Super+Space"*/, launch_at_login:bool, default_calendar_id:i64, default_event_minutes:u32 /*60*/, week_start: chrono::Weekday /*Mon*/, date_order: DateOrder /*from macOS region: DayMonth unless AppleICUDateFormatStrings / locale is en_US*/ }
  // persisted as JSON at ~/Library/Application Support/TaskCraft/settings.json
  pub struct CmdError { code:String, message:String }  // Serialize; From<StoreError>
  ```
  Commands (all `Result<_, CmdError>`, names as in spec §6): `parse_preview(text) -> ParsedItem`, `create_from_text(text) -> ItemRef{kind,id}`, `create_event`, `update_event`, `move_event`, `delete_event`, `create_task`, `update_task`, `set_task_completed`, `delete_task`, `list_range`, `list_tasks`, `search`, `list_calendars`, `upsert_calendar`, `delete_calendar`, `get_settings`, `set_settings`, `export_ics(path)`. Every mutating command emits `items-changed` to all windows.
  TS: `api.ts` exports one typed function per command; `types.ts` mirrors the Rust structs.

- [ ] **Step 1: Install Tauri CLI** `cargo install tauri-cli --version "^2" --locked`; Expected: `cargo tauri --version` prints 2.x.
- [ ] **Step 2: Failing tests** in `commands.rs` against an in-memory store via a plain function layer (`fn create_from_text_impl(state:&AppState, text:&str, now:DateTime<Tz>) -> Result<ItemRef, CmdError>`): `creates_event_in_default_calendar`, `creates_task_from_prefix`, `slash_calendar_overrides_default`.
- [ ] **Step 3: Run** `cargo test -p taskcraft-app` — Expected: FAIL.
- [ ] **Step 4: Implement** shell: `tauri.conf.json` with `identifier: com.pareasy.taskcraft`, `macOSPrivateApi: true`, `bundle.macOS.minimumSystemVersion: "14.0"`, windows declared in Rust (not config); dev URL `http://localhost:5173`, frontendDist `../ui/dist`; tauri features `tray-icon`, `macos-private-api`; logging per Global Constraints.
- [ ] **Step 5: Run** tests — Expected: PASS; `cargo tauri build --debug --bundles app` succeeds.
- [ ] **Step 6: Commit** `feat(app): tauri shell, settings, commands`.

### Task 10: Menu-bar quick entry

**Files:**
- Create: `ui/src/quick/{Quick,HighlightInput,PreviewCard,MiniMonth,Upcoming}.svelte`, `ui/src/styles.css`, `ui/src/lib/layout.ts` (`monthGrid`), `ui/src/lib/layout.test.ts`
- Modify: `src-tauri/src/main.rs`, `ui/src/main.ts`

**Interfaces:**
- Consumes: Task 9 commands.
- Produces: `monthGrid(year:number, month:number /*0-11*/, weekStart:0|1) -> Date[]` (always 42 days); label colors as CSS vars `--label-date, --label-time, --label-location, --label-attendees, --label-recurrence, --label-alert, --label-calendar, --label-url, --label-duration, --label-prefix`.

- [ ] **Step 1: Failing vitest** `monthGrid`: October 2026 with weekStart 1 → first cell Mon 2026-09-28, last cell Sun 2026-11-08, length 42.
- [ ] **Step 2: Run** `npm --prefix ui run test` — Expected: FAIL.
- [ ] **Step 3: Implement** UI per spec §5 "Menu-bar popover": `HighlightInput` = transparent `<input>` over a mirrored `<div>` coloring spans from `parse_preview` (debounce 50 ms); Enter → `create_from_text`, clear input, keep open; Esc → hide; ⌘Enter → create then show `main` focused on the item. Rust: tray icon (template image), left click toggles `quick` positioned with `Position::TrayBottomCenter`; window 360×520, `decorations:false`, `transparent:true`, `always_on_top:true`, `skip_taskbar:true`, `effects: [Popover]`; hide on `Focused(false)`; global shortcut from settings toggles it.
- [ ] **Step 4: Verify** vitest PASS; `cargo tauri dev`, press ⌥⌘Space, type `Lunch with Sam tomorrow at 1`, Enter → item appears in Upcoming under Wednesday 21; Esc hides.
- [ ] **Step 5: Commit** `feat(ui): menu-bar quick entry`.

### Task 11: Main window — Day/Week/Month, sidebar, inspector, search, tasks

**Files:**
- Create: `ui/src/main/{Main,Sidebar,Toolbar,DayWeekGrid,MonthGrid,Inspector,TaskList}.svelte`
- Modify: `ui/src/lib/layout.ts`, `ui/src/lib/layout.test.ts`, `src-tauri/src/main.rs`

**Interfaces:**
- Produces: `layoutDay(items:{id:string,start:Date,end:Date}[], dayStart:Date) -> {id, top:number /*minutes from 00:00*/, height:number, col:number, cols:number}[]` (overlapping items share width; cluster = transitive overlap); `weekDays(anchor:Date, weekStart:0|1) -> Date[7]`; `snap15(minutes:number) -> number`.

- [ ] **Step 1: Failing vitest:** `layoutDay` three items 9–10, 9:30–11, 10–10:30 → cols 2 for the 9–11 cluster: item1 col0, item2 col1, item3 col0; `weekDays` for Wed 2026-10-21 weekStart 1 → Mon 19 … Sun 25; `snap15(67)` = 60, `snap15(68)` = 75.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** views per spec §5 "Main window": Day/Week hour grid 48 px/hour, all-day strip, now-line, drag move/resize with `snap15` calling `move_event` (recurring → in-app "This event / All events" choice), double-click creates a 1-hour event and opens the inspector; Month 6×7 with 3 items + "+N more"; sidebar calendars (visibility toggles call `upsert_calendar`), mini month, task sections with checkboxes; toolbar Today/◀/▶/segmented control with ⌘1/⌘2/⌘3; search field calls `search` and lists hits; inspector edits all fields, title field re-parses on blur via `parse_preview`, delete asks inside the popover. Listen to `items-changed` to re-query. Window 1100×720, min 800×560, created on demand.
- [ ] **Step 4: Verify** vitest PASS; in `cargo tauri dev`: create `Standup every weekday at 9:30`, see it Mon–Fri in Week view; drag Thursday's to 10:00 choosing "This event" → only Thursday moves; switch ⌘3 to Month and back.
- [ ] **Step 5: Commit** `feat(ui): main window views`.

### Task 12: Settings window, autostart, export

**Files:**
- Create: `ui/src/settings/Settings.svelte`
- Modify: `src-tauri/src/main.rs`, `src-tauri/src/commands.rs`

**Interfaces:**
- Consumes: `get_settings`, `set_settings`, calendar commands, `export_ics`.
- Produces: `set_settings` re-registers the global shortcut (unregister old, register new; on failure keep old and return `CmdError{code:"hotkey_unavailable"}`) and toggles autostart via `tauri-plugin-autostart` (`MacosLauncher::LaunchAgent`).

- [ ] **Step 1: Failing test** `set_settings_rejects_bad_hotkey` (hotkey `"Nope+X"` → `code == "hotkey_invalid"`, settings unchanged).
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** settings UI per spec §5 "Settings" (hotkey recorder field, launch at login, default calendar, default event length, week start, date order, calendar add/rename/color/delete with move-to choice, "Export .ics…" using the dialog plugin save panel).
- [ ] **Step 4: Run** — Expected: PASS; manual: change hotkey to ⌃⌥K, it works immediately; export writes a file that `head -1` shows `BEGIN:VCALENDAR`.
- [ ] **Step 5: Commit** `feat(app): settings, autostart, export`.

### Task 13: Alert scheduler and notifications

**Files:**
- Create: `src-tauri/src/scheduler.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `Store::due_alerts`, `Store::mark_fired`.
- Produces: `pub fn spawn(app: AppHandle)`: loop `due_alerts(now)` → for each, notification (title = item title; body = local time range and location) via `tauri-plugin-notification`, then `mark_fired`; sleeps 30 s or until `Notify` is triggered by `items-changed` or by `RunEvent::Resumed`/wake (`NSWorkspaceDidWakeNotification` observed via `objc2-app-kit`). Keeps `last_fired: Mutex<Option<ItemRef>>`; on `RunEvent::Reopen` (what a notification click delivers) it shows `main` and emits `focus-item` with that ref.

- [ ] **Step 1: Failing test** `tick_fires_and_marks` using a `trait Notifier { fn notify(&self, title:&str, body:&str) }` fake: two due alerts → fake records 2 calls, store has none due afterwards.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** `fn tick(store:&Store, notifier:&dyn Notifier, now) -> Result<usize>` and the thread around it; request notification permission on first launch.
- [ ] **Step 4: Run** — Expected: PASS; manual in dev build: `Test alert in 2 minutes alert 1 min before` → notification within ~1 min 30 s.
- [ ] **Step 5: Commit** `feat(app): alert scheduler and notifications`.

### Task 14: Build, sign, install, smoke test

**Files:**
- Create: `scripts/install.sh`, `scripts/smoke.sh`, `README.md`

**Interfaces:**
- Consumes: everything.
- Produces: `/Applications/TaskCraft.app`.

- [ ] **Step 1: `scripts/install.sh`**: `cargo test --workspace && npm --prefix ui ci && npm --prefix ui run check && npm --prefix ui run test && cargo tauri build --bundles app`; `codesign --force --deep --sign - target/release/bundle/macos/TaskCraft.app`; quit a running TaskCraft (`osascript -e 'quit app "TaskCraft"'`); replace `/Applications/TaskCraft.app` (`ditto`); `xattr -dr com.apple.quarantine /Applications/TaskCraft.app`; `open -a TaskCraft`.
- [ ] **Step 2: `scripts/smoke.sh`**: wait for the process; check the log file exists and the db has `user_version = 1` (`sqlite3`); run the release binary's `--smoke "Lunch with Sam tomorrow at 1"` flag (implemented in `main.rs` behind a CLI arg, release-safe: parses, inserts into a temp db copy, prints the created row as JSON, exits 0) and assert the JSON title is "Lunch".
- [ ] **Step 3: Run** `scripts/install.sh && scripts/smoke.sh` — Expected: both exit 0; menu-bar icon visible; ⌥⌘Space opens the popover.
- [ ] **Step 4: README** with install, update (re-run script), data/log locations, uninstall (`rm -rf /Applications/TaskCraft.app ~/Library/Application\ Support/TaskCraft ~/Library/Logs/TaskCraft`).
- [ ] **Step 5: Commit** `chore: install and smoke scripts`.
