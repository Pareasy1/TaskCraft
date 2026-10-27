# Planner timeline and dependencies — design (phase 5 of 5)

Status: approved in conversation, awaiting written-spec review.
Builds on phase 4 (`feat/views`, PR #15). Branch: `feat/planner`.

## Decisions (from the user)

- **Dependencies are "soft blocks":** an arrow A → B ("B depends on A"), a 🔒 on B while A is not done,
  and a confirmation (not a hard refusal) when B is started or completed while blocked. Dates never
  move automatically.
- **The timeline is editable:** drag a bar to move it, drag its edges to change start / due, and
  draw dependency arrows by dragging from one bar to another.
- **Tasks without dates** sit in a collapsible "No dates" panel; dragging one onto the timeline
  gives it dates.
- **Rendering is hand-made in Svelte** (HTML rows + SVG arrows), no new dependencies; drags use
  pointer events, not HTML5 drag-and-drop.

## Data model (schema V6)

```
dependencies(task_id INTEGER NOT NULL,      -- B, the blocked task
             depends_on INTEGER NOT NULL,   -- A, the blocker
             created_at TEXT NOT NULL,
             PRIMARY KEY (task_id, depends_on),
             CHECK (task_id <> depends_on))
CREATE INDEX dependencies_depends_on ON dependencies(depends_on);
```
- Migration V5→V6 in the single-transaction `migrate`, after the backup (`.bak-v5`).
  `scripts/smoke.sh` expects schema 6.
- No SQL foreign keys (as for the other later task columns); the store enforces integrity.
  `delete_task_row` deletes every dependency row where the task is either side.
- Any task or subtask may depend on any other, across plans.

## Store API

- `add_dependency(task_id, depends_on) -> Result<()>`.
  - Unknown task on either side: `NotFound`. Same id: `Rule(DependencySelf)`.
  - A cycle (`depends_on` already reaches `task_id` through existing rows, found by a walk over
    the table): `Rule(DependencyCycle)`.
  - An existing pair is a no-op (idempotent).
- `remove_dependency(task_id, depends_on) -> Result<()>` (missing pair: no-op).
- `list_dependencies(task_ids: Option<&[i64]>) -> Vec<Dependency { task_id, depends_on }>`;
  `None` returns all rows.
- `Task` gains `blocked_by: i64`, the number of tasks it depends on that are not done, filled by
  `attach_counts` with the same bulk-count pattern.
- New `Rule` codes: `dependency_self`, `dependency_cycle`.
- `duplicate_task` and templates do not copy dependencies.
- **Delete Undo:** the phase-2 restore helper also re-adds the deleted task's dependency rows (both
  directions) via `add_dependency`, skipping pairs whose other task no longer exists.

## Tauri

Commands `add_dependency`, `remove_dependency`, `list_dependencies`; they emit the usual
items-changed event.

## Blocked tasks in the existing UI

- Board card, Grid (Title cell) and the timeline show 🔒 n when `blockedBy > 0`, with a tooltip
  "Waiting for: A, B" (names resolved from the dependency list when available, otherwise "n open
  dependencies").
- `changeStatus` (and the Grid's Status editor, the details dialog's status control) asks
  "Depends on “A”, still open. Continue?" (Yes / No) when moving a blocked task to In progress or
  Done. Several blockers: "Depends on 3 open tasks. Continue?". No → nothing changes. The
  auto-start rule (a subtask starting starts its parent) does not ask for the parent.
- **Details dialog → Dependencies section** (tasks and subtasks):
  - "Depends on": rows with title, status glyph and a remove button; "Add…" opens a searchable list
    of other tasks (all plans, excluding itself and those that would form a cycle, done tasks
    included).
  - "Blocks": read-only rows of the tasks that depend on this one; clicking one opens its details.

## Planner area

- **Navigation:** the top switcher becomes Calendar · Tasks · Planner · Grid (IT: Calendario ·
  Attività · Planner · Griglia). **Mod+4** opens Planner; Mod+1/2/3 are unchanged in meaning
  (Calendar / Tasks / Grid). Planner shares the plan scope and the label / status filters with
  Tasks and Grid; selecting a plan in the sidebar stays in Planner when Planner is current. The
  toolbar shows the plan name or open-count title (as in Tasks / Grid), the zoom switch and a
  "Today" button. "+" creates a new task.
- **Layout:**
  - A left **name column** (resizable is out of scope; fixed width) and the **time grid** on the
    right, scrolling horizontally together with a sticky header of dates.
  - Rows: top-level tasks grouped by bucket (in All tasks: by plan), subtasks indented under their
    parent. Group headers and parents collapse; the collapsed set is session-only. A matching
    subtask under a non-matching parent shows a dimmed parent row (the `groupWithParents` rule).
  - Weekends are tinted; a vertical line marks today.
  - **Zoom:** Days / Weeks / Months (column = 1 day at three widths: 32 / 14 / 4 px), persisted in
    `taskcraft.planner.zoom` (try/catch). The header shows days, week starts (user's
    week-start setting) or months accordingly. The visible range spans from 2 weeks before the
    earliest dated task (or today) to 4 weeks after the latest, at least 3 months.
  - On open, the view scrolls so today is near the left third. "Today" scrolls there again.
- **Bars:**
  - From the local start date to the local due date, inclusive. Only due → a 1-day bar on the due
    date; only start → a 1-day bar on the start date.
  - Colour by status (todo neutral, in progress `--progress-active`, done dimmed); overdue (by
    `isOverdue`) gets a danger outline; a thin inner fill shows subtask progress when the task has
    subtasks; 🔒 when blocked. The title is drawn in the bar when it fits, otherwise after it.
- **Dragging (pointer events, snapping to whole local days):**
  - Body: moves start and due by the same number of days; times of day are kept.
  - Left edge: changes start (adds one to a due-only task). Right edge: changes due (adds one to a
    start-only task). The edge cannot cross the other end; the drag clamps.
  - A move shorter than one day is a click. Esc during a drag cancels it.
  - On release, one `updateTask` via `taskInput`, then an Undo toast.
  - The cursor and a floating label show the new dates while dragging.
- **No dates panel:** a collapsible panel on the right listing the scope's undated tasks (same
  filters). Dragging one onto the grid sets start = due = that local day (all-day). Its open state
  is persisted in `taskcraft.planner.undated` (try/catch). Empty: "Every task has a date".
- **Dependency arrows (SVG overlay):**
  - From the end of A's bar to the start of B's bar, with an elbow path and an arrowhead. Arrows
    involving a collapsed or filtered-out task are not drawn; the 🔒 still shows.
  - When B's start is before A's due, the arrow uses the warning colour. Visual only.
  - **Drawing:** hovering a bar shows a connector dot at its right end; dragging the dot onto
    another bar calls `add_dependency(target, source)`. Self / cycle refusals show the translated
    toast. A drop elsewhere does nothing.
  - **Removing:** clicking an arrow selects it (thicker stroke); Delete / Backspace removes it,
    with Undo. The bar menu also lists "Remove dependency ▸".
- **Keyboard and menus:**
  - Rows are focusable (roving tabindex); ↑ / ↓ move between rows; Enter or Mod+Enter opens the
    details; ← / → on a parent collapse / expand.
  - Alt+← / Alt+→ move the focused task's bar by one day; Alt+Shift+← / → change its due by one
    day (a 1-day bar gains a due if it had only a start). Each step offers Undo.
  - The bar's context menu (right-click, or the menu key / Shift+F10): Open details, "Depends on…"
    (opens the same searchable list as the dialog), "Remove dependency ▸", "Remove dates".
  - Double-click a bar or a name opens the details.
- **Size:** responsive with 500 rows; plain rows unless a 500-row render exceeds 16 ms per frame
  in the dev build (same rule as phase 4).

## Pure helpers (`ui/src/lib/`)

- `timeline.ts`: `barSpan(task, tz) -> {startDay, endDay} | null` (local day numbers),
  `timelineRange(tasks, today)`, `dayToX(day, range, zoom)`, `xToDay(...)`,
  `dragResult(task, mode: 'move'|'start'|'due', deltaDays) -> {start, due}` (keeps times, clamps).
- `timelineRows(tasks, scope, filters, collapsed)` reusing the grid's grouping helpers.
- `dependencies.ts`: `blockers(id, deps, tasks)`, `wouldCycle(from, to, deps)`,
  `arrowPath(fromBox, toBox)`, `blockedPrompt(blockers, lang)`.

## Strings

All in EN/IT: Planner, Days/Weeks/Months, Today, No dates, the empty state, Depends on…, Blocks,
Remove dependency, Remove dates, the blocked prompt, the 🔒 tooltip, the rule toasts
(self / cycle), the drag date label.

## Error handling

- Rule codes map to localized toasts; a failed drag save reverts the bar and shows the error.
- `start_after_due` cannot come from a drag (clamped) but is still mapped.

## Testing

- **Store:** V5→V6 migration (and the real-DB copy check driven by `TASKCRAFT_MIGRATE_CHECK`);
  add / remove / idempotent add; self and cycle refusal (direct and 3-long); `blocked_by` counts
  (done blockers don't count); delete removes both directions; duplicate does not copy.
- **Restore:** Undo of a deleted task re-adds its dependency rows.
- **UI (vitest):** `barSpan` (due-only, start-only, both, across DST, TZ=Europe/Rome),
  `dragResult` (move keeps times, edge clamps, adding a start to a due-only task), `xToDay` /
  `dayToX` round trips at each zoom, `timelineRange`, `wouldCycle`, `arrowPath`, `blockedPrompt`,
  `timelineRows`, the Mod+4 mapping, persistence with storage unavailable, the blocked prompt in
  `changeStatus`.
- **Style rules:** the existing `dropEffect` rule still passes; the timeline uses no HTML5 drag.
- **Visual:** dev server (EN/IT, light/dark), then the installed app (WebKit): drag move / resize,
  arrow drawing and removal, the No dates panel, zoom, Today, keyboard moves, the blocked prompt.

## Out of scope

Automatic rescheduling of dependants; dependency types other than finish-to-start; lag days;
critical path; baselines; resizable name column; printing / exporting the timeline; recurring
tasks drawn as repeated bars (a recurring task shows its current occurrence only).
