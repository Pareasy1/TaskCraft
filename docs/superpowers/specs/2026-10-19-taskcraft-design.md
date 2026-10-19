# TaskCraft — Design Spec

Date: 2026-10-19
Status: Approved in conversation, pending written review

## 1. Purpose

A personal-use macOS calendar and task app written in Rust, with a
Fantastical-style interface: a menu-bar quick-entry popover that turns
natural-language sentences into events and tasks, plus a main window
with Day/Week/Month views, a task list, and notifications.

It is a clean-room implementation. Behaviour and layout are inspired by
Fantastical; no Flexibits code, model weights, data files, prompts,
icons, or other assets are used.

### Success criteria

1. `TaskCraft.app` installs into `/Applications`, launches, and shows a
   menu-bar icon.
2. A global hotkey (default ⌥⌘Space) or the menu-bar icon opens the
   quick-entry popover.
3. Typed English sentences produce correct events and tasks (verified by
   the parser test table), with live highlighted preview while typing.
4. Saved items appear in the popover's upcoming list and in the main
   window's Day, Week and Month views and task list.
5. Event alerts and task due times trigger macOS notifications,
   including alerts missed while the Mac was asleep.
6. All data lives in one local SQLite file and can be exported as `.ics`.

### Constraints and decisions

- Storage: local SQLite only (no EventKit, no sync).
- UI framework: Tauri 2 (Rust core, web-view UI in TypeScript + Svelte).
- Parser: rule-based, English only for v1.
- Distribution: personal; ad-hoc code signature, no Apple Developer
  account, no notarization.
- Target: macOS 27 (Apple Silicon), minimum macOS 14.

### Out of scope for v1

Calendar sync (iCloud/Google/Exchange), EventKit, languages other than
English, LLM-based parsing, named holidays ("Christmas"), complex
recurrence exceptions, invitations/attendee email, iOS, weather, travel
time.

## 2. Architecture

Cargo workspace at `/Users/pareasy/Documents/VS_Projects/TaskCraft`:

| Unit | Responsibility | Depends on |
|---|---|---|
| `crates/taskcraft-parse` (lib) | `parse(input, now, opts) -> ParsedItem`. Pure, no I/O. | `chrono`, `chrono-tz` |
| `crates/taskcraft-store` (lib) | SQLite schema, migrations, CRUD, range queries with recurrence expansion, `.ics` export. | `rusqlite` (bundled), `rrule`, `chrono` |
| `src-tauri` (`taskcraft-app` bin) | Tray icon, global shortcut, windows, Tauri commands, alert scheduler, logging. | the two crates, Tauri 2 + plugins (global-shortcut, notification, autostart, positioner) |
| `ui/` | Svelte + TypeScript front end: popover, main window, inspector, settings. | Tauri commands only |

The UI never touches the database directly; every read and write goes
through a Tauri command implemented in Rust.

### Data flow

1. User types in the popover → UI calls `parse_preview(text)` (debounced
   ~50 ms) → Rust returns `ParsedItem` with label spans → UI renders the
   highlighted input and preview card.
2. Enter → `create_from_text(text)` → parse + insert in a transaction →
   Rust emits `items-changed` → all open views re-query their range.
3. Views call `list_range(start, end)` and `list_tasks(filter)`;
   recurring items are expanded into occurrences for the range.
4. Scheduler thread wakes every 30 s, selects unfired alerts with
   `fire_at <= now`, posts notifications, sets `fired_at`.

## 3. Data model

Database: `~/Library/Application Support/TaskCraft/taskcraft.db`.
Schema version tracked with `PRAGMA user_version`.

```sql
calendars(id INTEGER PK, name TEXT UNIQUE NOT NULL, color TEXT NOT NULL,
          visible INTEGER NOT NULL DEFAULT 1, sort INTEGER NOT NULL DEFAULT 0)

events(id INTEGER PK, calendar_id INTEGER NOT NULL REFERENCES calendars,
       title TEXT NOT NULL, start_utc TEXT NOT NULL, end_utc TEXT NOT NULL,
       tz TEXT NOT NULL, all_day INTEGER NOT NULL DEFAULT 0,
       location TEXT, attendees TEXT, notes TEXT, url TEXT, rrule TEXT,
       created_at TEXT NOT NULL, updated_at TEXT NOT NULL)

tasks(id INTEGER PK, calendar_id INTEGER NOT NULL REFERENCES calendars,
      title TEXT NOT NULL, due_utc TEXT, due_has_time INTEGER NOT NULL DEFAULT 0,
      tz TEXT NOT NULL, completed_at TEXT, rrule TEXT, notes TEXT,
      created_at TEXT NOT NULL, updated_at TEXT NOT NULL)

alerts(id INTEGER PK, owner_kind TEXT NOT NULL CHECK(owner_kind IN ('event','task')),
       owner_id INTEGER NOT NULL, offset_min INTEGER, absolute_utc TEXT,
       occurrence_utc TEXT, fire_at_utc TEXT NOT NULL, fired_at TEXT)
```

- Timestamps are RFC 3339 UTC strings; `tz` is the IANA zone used when
  the item was created and for all-day/recurrence expansion.
- Seed calendars: "Personal" (default) and "Work".
- `attendees` is a newline-separated list of names (display only).
- Recurring items are stored once; alerts for recurring items are
  materialized for the next occurrence only and re-materialized after
  firing.
- Before running a migration, the db file is copied to
  `taskcraft.db.bak-v<old>`.

## 4. Parser (`taskcraft-parse`)

### Interface

```rust
pub fn parse(input: &str, now: DateTime<Tz>, opts: &ParseOptions) -> ParsedItem;

pub struct ParseOptions {
    pub default_event_minutes: u32,      // 60
    pub calendars: Vec<String>,          // known names for "/work"
    pub date_order: DateOrder,           // DayMonth | MonthDay
}

pub struct ParsedItem {
    pub kind: ItemKind,                  // Event | Task
    pub title: String,
    pub start: Option<When>,             // When { date: NaiveDate, time: Option<NaiveTime> }
    pub end: Option<When>,
    pub all_day: bool,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub calendar: Option<String>,
    pub recurrence: Option<Recurrence>,  // serializes to RRULE
    pub alerts: Vec<AlertSpec>,          // Before(minutes) | At(When)
    pub url: Option<String>,
    pub spans: Vec<Span>,                // { start: usize, end: usize, label: Label } (byte offsets)
}

pub enum Label { Title, TaskPrefix, Date, Time, Duration, Recurrence,
                 Alert, Location, Attendees, Calendar, Url, Separator }
```

### Pipeline

1. **Task prefix.** Case-insensitive leading `todo`, `to do`, `to-do`,
   `task`, `reminder`, `remind me to`, `remind me`, `remember to`, `✓`,
   optionally followed by `:`. Sets `kind = Task`.
2. **Tokenize** into word / number / punctuation tokens with byte ranges.
3. **Tag** with ordered rule matchers (longest match wins at each
   position; tagged tokens are not re-tagged):
   - URL (`https?://…`, `www.…`)
   - Calendar (`/name` matching a known calendar, case-insensitive)
   - Recurrence: `every day|weekday|weekend|week|month|year`,
     `every <weekday>[ and <weekday>…]`, `every N days|weeks|months`,
     `daily|weekly|monthly|yearly`, `monthly on the <ordinal>`,
     optional `until <date>`
   - Alert: `alert|remind me <N> <unit> before`, `alert at <time>`
   - Duration: `for <N> <min|minutes|h|hour(s)>`, `for an hour`,
     `for half an hour`
   - Time range: `<time>-<time>`, `from <time> to|until|- <time>`,
     `<time> to <time>`
   - Time: `noon`, `midnight`, `<h>[:mm][am|pm|a|p]`, `<HH:MM>`,
     `at <h>` (bare hour), `morning|afternoon|evening`
     (09:00, 14:00, 18:00)
   - Date+time: `tonight` (today 20:00 unless a time is given)
   - Date: `today`, `tomorrow`, `yesterday`,
     `[this|next] <weekday>`, `<weekday>`, `next week|month`,
     `in N days|weeks`, `<month> <day>[,] [year]`, `<day> <month> [year]`,
     `<a>/<b>[/<y>]` interpreted using `ParseOptions.date_order`
     (D/M or M/D; the app's Settings value, defaulting from the macOS
     region), `the <ordinal>`, ISO `YYYY-MM-DD`;
     date ranges `<date> to|until|- <date>`
   - Attendees: separator `with` followed by name words up to the next
     tagged token; split on `,`, `and`, `&`
   - Location: separator `at|in|@` followed by untagged words up to the
     next tagged token or separator. `at` followed by a time is a time,
     never a location. `in` followed by a duration/number is not a
     location.
   - Remaining untagged tokens (with separators that did not start a
     field) form the **title**, whitespace-normalized; trailing
     separators are trimmed.
4. **Resolve**:
   - Weekday without qualifier → next occurrence including today if the
     time has not passed; `next <weekday>` → occurrence in the following
     week.
   - Date without year → this year if not yet past, else next year.
   - Bare hour 1–6 → pm, 7–11 → am unless title context implies evening
     (`dinner`, `drinks`, `party`, `concert`, `movie`, `show` → pm);
     12 → noon.
   - End before start on the same day → end moves to the next day.
   - Event with start time and no end/duration → end = start +
     `default_event_minutes`. Start with date only → all-day event.
   - No date but a time → today, or tomorrow if that time already passed.
   - No date and no time → event: all-day today; task: no due date.
   - Tasks use `start` as the due date/time; `end` is ignored.

### Guarantees

- `parse` never panics and never fails; worst case the whole input is the
  title.
- Deterministic given `(input, now, opts)`.
- Spans are non-overlapping, sorted, and lie on UTF-8 boundaries.

## 5. Interface (Tauri + Svelte)

### Menu-bar popover (window `quick`)

- Tray icon (custom monochrome template icon). Click or ⌥⌘Space toggles
  a borderless, always-on-top, vibrancy (`NSVisualEffectView` via Tauri
  `windowEffects`) window ~360×520 positioned under the tray icon
  (positioner plugin); hides on blur.
- Input with live highlighted overlay (colors per Label) and a preview
  card. Enter = save, Esc = hide, ⌘Enter = save and open in main window.
- Mini month with dots on days with items; selecting a day filters the
  list.
- Upcoming list grouped by day (next 14 days), overdue + today tasks
  pinned on top with checkboxes.
- Footer: open main window, settings, quit.

### Main window (window `main`)

- Sidebar: calendar list with color and visibility toggles, mini month,
  task sections (Overdue, Today, Upcoming, Completed).
- Toolbar: Today, ◀ ▶, Day/Week/Month segmented control (⌘1/⌘2/⌘3),
  search (title/location/notes).
- Day and Week: hourly grid, all-day strip, red now-line, drag to move,
  drag bottom edge to resize (15-min snap), double-click to create.
- Month: 6-row grid; up to 3 items per cell plus "+N more".
- Inspector popover on click: all fields editable; title field accepts
  natural language re-parse; delete with confirmation inside the app UI.
- Light/dark follows the system. Icons: Lucide (ISC license) and custom.

### Settings (window `settings`)

Hotkey, launch at login, default calendar, default event length, week
start day, date order (M/D vs D/M), calendars (add/rename/color/delete),
export `.ics`.

## 6. Tauri commands

`parse_preview(text) -> ParsedItem`,
`create_from_text(text) -> ItemRef`,
`create_event(EventInput)`, `update_event(id, EventInput)`,
`move_event(id, new_start, new_end, scope)`, `delete_event(id, scope)`,
`create_task(TaskInput)`, `update_task(id, TaskInput)`,
`set_task_completed(id, bool)`, `delete_task(id)`,
`list_range(start, end) -> Vec<Occurrence>`,
`list_tasks(filter) -> Vec<Task>`, `search(q) -> Vec<Hit>`,
`list_calendars`, `upsert_calendar`, `delete_calendar(id, move_to)`,
`get_settings`, `set_settings`, `export_ics(path)`.

`scope` for recurring items: `this` | `all` (v1 supports `all` and
`this`, where `this` adds an EXDATE and creates a detached event).

Errors are returned as `{ code, message }`; the UI shows a dismissible
banner. All errors are also logged.

## 7. Notifications and scheduler

- Thread started at launch; ticks every 30 s and on wake from sleep.
- Selects `alerts` with `fired_at IS NULL AND fire_at_utc <= now`;
  alerts older than 12 h are marked fired without notifying; others post
  a notification (title, time, location) via the notification plugin and
  set `fired_at`.
- Tasks with a due time get an implicit alert at the due time.
- Clicking a notification focuses the main window on the item.
- Recurring items: after firing, the alert for the next occurrence is
  materialized.

## 8. Logging and errors

- `tracing` to `~/Library/Logs/TaskCraft/taskcraft.log` (daily rotation, keep 7).
- No panics across the command boundary; all commands return `Result`.
- DB opened with WAL mode and `foreign_keys=ON`.

## 9. Testing

- `taskcraft-parse`: table-driven tests (~150 cases) with fixed `now`
  (2026-10-20 10:00 Europe/Rome, a Tuesday) covering every rule above,
  plus property tests that `parse` never panics and spans are valid.
- `taskcraft-store`: tests on in-memory SQLite for CRUD, migrations,
  recurrence expansion (daily, weekly on days, monthly, until, EXDATE),
  alert materialization, `.ics` export round-trip check with a parser.
- App: `cargo tauri build` succeeds; smoke script launches the built app,
  invokes `create_from_text` through a debug-only IPC test hook, and
  checks the row in the db.
- UI: `svelte-check` and `vitest` for date-grid layout helpers (overlap
  columns, month grid generation).

## 10. Build and install

1. Install toolchain: `rustup` (stable), Tauri CLI (`cargo install
   tauri-cli --version ^2`), Node (present, v26).
2. `cargo test --workspace`, `npm --prefix ui run check`.
3. `cargo tauri build --bundles app`.
4. Ad-hoc sign: `codesign --force --deep --sign - TaskCraft.app`.
5. Copy to `/Applications/TaskCraft.app`, remove quarantine attribute,
   launch.
6. Optional: enable "Launch at login" in Settings.
