# Planner timeline and dependencies — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add task dependencies (soft blocks, schema V6) and a fourth area, Planner: an editable timeline with draggable bars, a No dates panel and drawable dependency arrows.

**Architecture:** Store gains a `dependencies` table, an API with cycle refusal, and a `blocked_by` count on `Task`; Tauri exposes three commands. UI logic lives in pure modules `lib/timeline.ts` and `lib/dependencies.ts` (vitest); components in `ui/src/main/planner/` render HTML rows plus an SVG arrow overlay, with pointer-event drags (never HTML5 drag-and-drop). Every edit goes through existing api/actions (`taskInput`, `updateTask`, `changeStatus`, `notify` Undo).

**Tech Stack:** Rust (rusqlite), Tauri 2, Svelte 5 runes, TypeScript, vitest; plain SVG.

**Spec:** `docs/superpowers/specs/2026-10-27-planner-timeline-dependencies-design.md`

## Global Constraints

- Branch `feat/planner` (from `feat/views`). Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No new dependencies (Rust or npm). Env: `export PATH="$HOME/.cargo/bin:$PATH"`.
- Schema V6; migration inside the single-transaction `migrate` (`crates/taskcraft-store/src/schema.rs`), `CURRENT_VERSION = 6`; `scripts/smoke.sh` expects 6. No SQL foreign keys; all task deletes go through `delete_task_row`.
- Rule codes exactly `dependency_self`, `dependency_cycle`.
- Strings in `ui/src/lib/i18n.ts` EN+IT; tokens only (style-rules/tokens tests green); every action reachable by keyboard and menu.
- Timeline drags use pointer events (`pointerdown`/`pointermove`/`pointerup` with `setPointerCapture`), never `draggable`/HTML5 DnD — the macOS app runs in WebKit.
- localStorage keys exactly `taskcraft.planner.zoom` (default `'weeks'`) and `taskcraft.planner.undated` (default open); reads/writes in try/catch; UI works without storage.
- Zoom day widths exactly: days 32 px, weeks 14 px, months 4 px.
- Mod+1/2/3 stay Calendar/Tasks/Grid; Mod+4 = Planner. Switcher order Calendar · Tasks · Planner · Grid.
- Local dates everywhere (the app's local tz); bars are inclusive of start and due day; times of day are preserved by moves.
- Checks: `cd ui && npx vitest run && TZ=Europe/Rome npx vitest run && npx svelte-check --tsconfig ./tsconfig.app.json`; `cargo test --workspace`; `cargo clippy --workspace -- -D warnings`.
- Never touch the user's real DB; migration checks only on a snapshot copy.

## Review Focus

1. A drag across a DST change (Europe/Rome, last Sunday of March / October) must land on the intended local day and keep the time of day. → Task 3 `dragResult` DST cases under `TZ=Europe/Rome`.
2. Deleting a task that others depend on (or that depends on others) must leave no dangling rows, and Undo must bring the arrows back without failing when the other side is gone. → Task 1 `delete_removes_both_directions`; Task 2 `restore re-adds dependencies, skipping missing`.
3. Drawing an arrow onto the same bar, onto an existing dependant, or creating a cycle via a subtask/parent pair must refuse with the translated toast, never crash or duplicate. → Task 1 cycle/self/idempotent tests; Task 4 `wouldCycle` cases.
4. A blocked task completed from any surface (card glyph, Grid Space, details dialog, quick status menu) must ask first; the auto-start of a parent must not ask. → Task 4 `changeStatus` blocked-prompt tests.
5. A task with only a due date that has a time (e.g. 18:00): moving it keeps 18:00; adding a start via the left edge creates an all-day start ≤ due. → Task 3 `dragResult` cases.

---

### Task 1: Store — dependencies table, API, blocked count

**Files:** Create `crates/taskcraft-store/src/dependencies.rs`, `crates/taskcraft-store/tests/dependencies.rs`, `crates/taskcraft-store/tests/migrate_v6.rs`. Modify `schema.rs` (V6, `CURRENT_VERSION`), `model.rs` (`Rule::DependencySelf`, `Rule::DependencyCycle` + codes + Display "a task cannot depend on itself" / "this dependency would create a cycle"; `Dependency` struct; `Task.blocked_by: i64`), `tasks.rs` (`attach_counts` fills `blocked_by`; `delete_task_row` deletes rows on both sides for the task and its subtasks; Task constructors default 0), `lib.rs` (mod + re-export), `scripts/smoke.sh` (6).

**Interfaces — Produces:**
- SQL V6 exactly as in the spec (table `dependencies(task_id, depends_on, created_at)`, PK pair, CHECK, index `dependencies_depends_on`).
- `#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)] #[serde(rename_all = "camelCase")] pub struct Dependency { pub task_id: i64, pub depends_on: i64 }`.
- `Store::add_dependency(&self, task_id: i64, depends_on: i64) -> Result<()>` — NotFound if either task missing; `Rule(DependencySelf)` if equal; `Rule(DependencyCycle)` when following `depends_on` edges starting from `depends_on` reaches `task_id`; existing pair → Ok no-op (`INSERT OR IGNORE`).
- `Store::remove_dependency(&self, task_id: i64, depends_on: i64) -> Result<()>` (missing → Ok).
- `Store::list_dependencies(&self, task_ids: Option<&[i64]>) -> Result<Vec<Dependency>>` (rows where either side is in `task_ids`; `None` → all), ordered by `(task_id, depends_on)`.
- `Task.blocked_by` = count of `depends_on` rows whose task has `completed_at IS NULL`; serialized as `blockedBy`.

- [ ] **Step 1: Failing tests** `tests/dependencies.rs`: `add_and_list`; `add_is_idempotent`; `self_refused` (Rule code `dependency_self`); `direct_cycle_refused` (A→B then B→A); `three_long_cycle_refused` (A→B, B→C, C→A); `unknown_task_not_found`; `blocked_by_counts_open_only` (B depends on A and C; complete A → `blocked_by` 1); `delete_removes_both_directions` (delete middle of A→B→C → no rows mention it); `delete_parent_removes_subtask_rows`; `duplicate_does_not_copy` (`duplicate_task` result has `blocked_by` 0 and no rows); `list_filtered_by_ids`. `tests/migrate_v6.rs`: a v5 DB (built like `migrate_v5.rs` does for v4) migrates to 6 with tasks intact and an empty `dependencies` table; plus `#[ignore] real_db_copy_v6` driven by `TASKCRAFT_MIGRATE_CHECK` (same shape as `real_db_copy_v5`).
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test dependencies --test migrate_v6` → FAIL (compile).
- [ ] **Step 3: Implement.** Cycle walk: iterative DFS over `SELECT depends_on FROM dependencies WHERE task_id = ?` with a visited set. `blocked_by` via one grouped query `SELECT d.task_id, count(*) FROM dependencies d JOIN tasks t ON t.id = d.depends_on WHERE t.completed_at IS NULL GROUP BY d.task_id`.
- [ ] **Step 4: Run** `cargo test --workspace && cargo clippy --workspace -- -D warnings` → PASS.
- [ ] **Step 5: Commit** `feat(store): task dependencies (schema v6)`

### Task 2: Commands, api, types, Undo restore

**Files:** Modify `src-tauri/src/commands.rs` (+ `main.rs` handler list), `ui/src/lib/api.ts`, `ui/src/lib/types.ts`, `ui/src/lib/i18n.ts` (rule messages), `ui/src/lib/restore.ts` (+ `restore.test.ts`).

**Interfaces — Consumes:** Task 1. **Produces:**
- Commands `add_dependency(task_id, depends_on)`, `remove_dependency(task_id, depends_on)`, `list_dependencies(task_ids: Option<Vec<i64>>)`; add/remove emit the items-changed event like other task mutations.
- `api.addDependency(taskId: number, dependsOn: number): Promise<void>`, `api.removeDependency(taskId, dependsOn): Promise<void>`, `api.listDependencies(taskIds?: number[]): Promise<Dependency[]>`.
- `types.ts`: `interface Dependency { taskId: number; dependsOn: number }`; `Task.blockedBy: number`.
- i18n error codes: `dependency_self` EN "A task can't depend on itself." / IT "Un'attività non può dipendere da sé stessa."; `dependency_cycle` EN "That would make a loop of dependencies." / IT "Così si creerebbe un giro di dipendenze." (via `knownErrorMessage`).
- `TaskSnapshot` gains `dependsOn: number[]` and `blocks: number[]` (the other tasks' ids); `recreate` re-adds both directions after the task is created, each in try/catch, silently skipping a pair whose other task is gone (no toast for those).

- [ ] **Step 1: Failing tests** `restore.test.ts` (with the existing api mock pattern): `restore re-adds dependencies` (snapshot with dependsOn [5], blocks [7] → `addDependency(newId,5)` and `addDependency(7,newId)`); `restore re-adds dependencies, skipping missing` (addDependency rejects NotFound for one → others still added, no throw).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4:** cargo + UI checks → PASS.
- [ ] **Step 5: Commit** `feat: dependency commands and Undo restore`

### Task 3: Timeline geometry (pure)

**Files:** Create `ui/src/lib/timeline.ts` (+ `timeline.test.ts`).

**Interfaces — Produces:**
- `type Zoom = 'days' | 'weeks' | 'months'`; `DAY_WIDTH: Record<Zoom, number> = { days: 32, weeks: 14, months: 4 }`; `loadZoom(): Zoom` / `saveZoom(z)`; `loadUndatedOpen(): boolean` / `saveUndatedOpen(b)`.
- Day numbers: `dayNumber(d: Date): number` = local calendar days since 1970-01-01 (DST-safe, from local y/m/d); `dayDate(n: number): Date` = local midnight.
- `barSpan(t: {start: string|null; due: string|null}): {from: number; to: number} | null` (inclusive; only one date → 1-day span; none → null).
- `timelineRange(tasks, today: Date): {from: number; to: number}` = [min(earliest from, today) − 14, max(latest to, today) + 28], widened to ≥ 92 days by extending `to`.
- `dayToX(day, range, zoom) = (day − range.from) * DAY_WIDTH[zoom]`; `xToDay(x, range, zoom) = range.from + floor(x / DAY_WIDTH[zoom])`.
- `type DragMode = 'move' | 'start' | 'due'`; `dragResult(t: {start; startHasTime; due; dueHasTime}, mode: DragMode, deltaDays: number): {start: string|null; startHasTime: boolean; due: string|null; dueHasTime: boolean}` — shifts by local calendar days keeping time of day; `move` shifts both existing dates; `start` on a due-only task creates an all-day start on (due day + delta); `due` on a start-only task creates an all-day due; edges clamp so start day ≤ due day; returned ISO UTC strings.
- `placeUndated(day: number): {start: string; startHasTime: false; due: string; dueHasTime: false}` (local midnight of day for both).
- `headerTicks(range, zoom, weekStart: 0|1): {day: number; label: string; major: boolean}[]` (days: every day, major on month start; weeks: each week start; months: each month start).

- [ ] **Step 1: Failing tests** `timeline.test.ts`: barSpan cases (both, due-only, start-only, none); `dayNumber` round-trip through `dayDate` across 2026-03-29 and 2026-10-25; range minimums and today inclusion; `dayToX`/`xToDay` round trips at each zoom; dragResult: move +3 keeps 18:00 due time; move across 2026-10-25 under Europe/Rome keeps 09:00; `start` on due-only adds all-day start; start edge beyond due clamps to due day; `due` edge before start clamps; `placeUndated`; headerTicks week starts honour weekStart 0 vs 1; load/save with throwing storage → defaults.
- [ ] **Step 2: Run** `cd ui && npx vitest run src/lib/timeline.test.ts && TZ=Europe/Rome npx vitest run src/lib/timeline.test.ts` → FAIL. **Step 3: Implement.** **Step 4:** UI checks → PASS.
- [ ] **Step 5: Commit** `feat(ui): timeline geometry helpers`

### Task 4: Dependency helpers, blocked prompt, 🔒 badges

**Files:** Create `ui/src/lib/dependencies.ts` (+ test), `ui/src/main/tasks/BlockedBadge.svelte`. Modify `ui/src/lib/actions.ts` (+ `actions.test.ts`), `ui/src/lib/complete.svelte.ts` and its dialog host (`ui/src/main/CompleteDialog.svelte`) to support a yes/no "blocked" ask, `BoardCard.svelte`, `grid/GridTitleCell.svelte`, i18n.

**Interfaces — Consumes:** Task 2. **Produces:**
- `blockers(id: number, deps: Dependency[], tasks: Task[]): Task[]` (open tasks `id` depends on); `blocking(id, deps, tasks): Task[]` (tasks depending on `id`); `wouldCycle(taskId: number, dependsOn: number, deps: Dependency[]): boolean` (same rule as the store, plus self); `blockedPrompt(blockers: {title: string}[]): string` → one: `t('dep.blockedOne', {title})` EN "Depends on “{title}”, still open. Continue?" / IT "Dipende da “{title}”, ancora aperta. Continuare?"; several: `t('dep.blockedMany', {n})` EN "Depends on {n} open tasks. Continue?" / IT "Dipende da {n} attività ancora aperte. Continuare?".
- `requestBlocked(message: string): Promise<boolean>` in `complete.svelte.ts`, rendered by the existing centred dialog with Yes / No.
- `changeStatus`: when `s` is `inProgress` or `done`, the status actually changes, and `task.blockedBy > 0`, fetch `api.listDependencies([task.id])`, resolve blocker titles (`api.getTask`, open ones only), and `await requestBlocked(blockedPrompt(...))`; No → return with no change. Parents started by the store's auto-start never ask (only the task passed in is checked). The Grid's Status editor, card glyph, Inspector status control all go through `changeStatus` already — verify Inspector's save path: if it saves status via `updateTask`, add the same check before saving there (`confirmBlocked(task, newStatus): Promise<boolean>` exported from actions.ts and used by both).
- `BlockedBadge` props `{ count: number; titles?: string[] }`: "🔒 n" with `title`/aria-label `t('dep.waitingFor', {names})` EN "Waiting for: {names}" / IT "In attesa di: {names}", or `t('dep.openCount', {n})` EN "{n} open dependencies" / IT "{n} dipendenze aperte" when titles are unknown. Shown in BoardCard and GridTitleCell when `blockedBy > 0`.

- [ ] **Step 1: Failing tests** `dependencies.test.ts`: blockers excludes done; blocking; wouldCycle self / direct / 3-long / unrelated false; blockedPrompt one vs many (EN). `actions.test.ts`: `blocked task asks before starting` (No → `setTaskStatus` not called); `Yes proceeds`; `unblocked task does not ask`; `todo move does not ask`; `subtask start auto-starting a blocked parent does not ask for the parent`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4:** UI checks → PASS; visual: badge on card and grid, prompt from card glyph and Inspector.
- [ ] **Step 5: Commit** `feat(ui): blocked tasks ask before starting, 🔒 badges`

### Task 5: Dependencies section in the details dialog

**Files:** Create `ui/src/main/tasks/Dependencies.svelte`, `ui/src/main/tasks/TaskPicker.svelte` (searchable list, reused by Task 8). Modify `ui/src/main/Inspector.svelte` (section after Attachments, tasks and subtasks), i18n.

**Interfaces — Consumes:** Tasks 2, 4. **Produces:** `TaskPicker` props `{ tasks: Task[]; exclude: Set<number>; onpick: (t: Task) => void; oncancel: () => void }` — text filter (case/diacritic-insensitive on title), arrow keys + Enter, Esc cancels, shows plan name and status glyph per row. `Dependencies` props `{ task: Task; onopentask: (id: number) => void }`, loads `listDependencies([task.id])` and all tasks; "Depends on" rows (title, status glyph, remove ×) + "Add…" opening TaskPicker with `exclude` = self ∪ existing ∪ ids where `wouldCycle(task.id, x, deps)`; "Blocks" read-only rows (click opens). Add/remove call api, then `notify` with Undo (remove ↔ add). Strings: `dep.section` Dependencies/Dipendenze, `dep.dependsOn` Depends on/Dipende da, `dep.blocks` Blocks/Blocca, `dep.add` Add…/Aggiungi…, `dep.added` / `dep.removed` toasts, `dep.search` Search tasks/Cerca attività, `dep.none` None/Nessuna.
- [ ] **Step 1: Failing test** for any pure helper added (e.g. `pickerMatches(title, query)` in dependencies.ts: accents and case ignored). **Step 2–4:** RED → implement → UI checks PASS; visual EN/IT light/dark: add, cycle candidate hidden, remove + Undo, Blocks click.
- [ ] **Step 5: Commit** `feat(ui): dependencies in task details`

### Task 6: Planner area, rows and bars (read-only)

**Files:** Modify `ui/src/lib/navigation.ts` (+ test) (`Area` adds `'planner'`; `'4'` → planner), `Main.svelte` (render Planner; sidebar plan selection keeps `planner`/`grid`; "+" focuses new-task input in planner), `Toolbar.svelte` (switcher order, zoom segmented switch Days/Weeks/Months and "Today" only in Planner), i18n. Create `ui/src/main/planner/PlannerView.svelte` (owner: zoom, collapsed, filters shared like GridView, deps loading), `PlannerRows.svelte` (name column + bar rows), `PlannerHeader.svelte` (sticky ticks), `PlannerBar.svelte`; `ui/src/lib/timeline.ts` gains `timelineRows`.

**Interfaces — Consumes:** Tasks 3, 4; `gridRows`-style grouping (`groupWithParents`, `applyFilters`). **Produces:**
- `timelineRows<T>(tasks: T[], opts: {scope; buckets; plans; labels; status; collapsed: Set<string|number>}): TimelineRow<T>[]` where `TimelineRow<T> = { kind: 'group'; key: string; label: string; count: number; collapsed: boolean } | { kind: 'task'; task: T; depth: 0|1; dimmed: boolean; hasChildren: boolean; collapsed: boolean }` — groups by bucket (by plan in All) in sort order; group keys `b:<id>` / `p:<id>`; tasks in default order (bucket/plan sort, sortOrder, id); dated and undated tasks both get rows (an undated task's row simply has no bar) — undated ones are also listed in the panel (Task 7).
- Row height constant `ROW_H = 32` exported from timeline.ts (bars and arrows use it).
- Bars: inclusive span via `barSpan`; classes by status (`--progress-active` for in progress, dimmed done), danger outline when `isOverdue`, inner progress line from `progressSegments`, `BlockedBadge` when blocked, title inside if it fits else after. Weekend tint, today line, initial scroll today at ~⅓ width; "Today" re-scrolls. Name column 240 px, sticky; horizontal scroll shared with the header.
- Keyboard: rows roving tabindex, ↑/↓, ←/→ collapse/expand parents and groups, Enter/Mod+Enter and double-click open details.
- Strings: `area.planner` Planner/Planner, `planner.days` Days/Giorni, `planner.weeks` Weeks/Settimane, `planner.months` Months/Mesi, `planner.today` Today/Oggi, `planner.empty` "No tasks yet"/"Nessuna attività".

- [ ] **Step 1: Failing tests** `navigation.test.ts`: Mod+4 → planner; D/W/M do nothing in planner. `timeline.test.ts`: timelineRows grouping by bucket and by plan in All, collapsed group hides its tasks, dimmed parent, undated tasks present.
- [ ] **Step 2–4:** RED → implement → UI checks PASS; visual (dev server EN/IT light/dark): switcher, Mod+4, zoom persists, Today, bars for both/due-only/start-only, overdue outline, collapse, keyboard walk, 500-row mock (`?bulk=500` dev toggle from phase 4 if present) stays responsive.
- [ ] **Step 5: Commit** `feat(ui): Planner area with timeline rows`

### Task 7: Dragging bars, keyboard moves, No dates panel

**Files:** Modify `PlannerBar.svelte`, `PlannerView.svelte`, `PlannerRows.svelte`; create `ui/src/main/planner/UndatedPanel.svelte`; i18n; `ui/src/lib/style-rules.test.ts` (new rule: no `draggable` attribute or `ondragstart` under `ui/src/main/planner/`).

**Interfaces — Consumes:** Task 3 `dragResult`, `xToDay`, `placeUndated`, `loadUndatedOpen`/`saveUndatedOpen`; Task 6 rows. **Produces:** a shared `saveDates(task: Task, next: ReturnType<typeof dragResult>): Promise<void>` in `ui/src/lib/actions.ts` = `api.updateTask(task.id, {...taskInput(task), ...next})`, then `notify(t('planner.moved', {title}), undo → updateTask(before))`; on error the bar reverts and `onerror(errorMessage(e))`.
- Pointer drag on bar body = move, on 6 px edge handles = start/due; delta in whole days from pointer x; < 1 day movement = click; Esc cancels; floating label shows new dates (`fmtShortDate`); pointer capture so leaving the row keeps dragging.
- Keyboard on a focused task row: Alt+←/→ move ±1 day; Alt+Shift+←/→ change due ±1 day (uses `dragResult(...,'due',±1)`); each saves with Undo.
- UndatedPanel (right side, collapsible, state persisted): undated tasks of scope + filters; pointer-drag a row onto the grid → `placeUndated(xToDay(...))` → `saveDates`; keyboard alternative: Enter on a panel row puts it on today. Empty: `planner.allDated` "Every task has a date"/"Ogni attività ha una data". Strings: `planner.noDates` No dates/Senza date, `planner.moved` "Moved “{title}”"/"Spostata “{title}”", drag label.

- [ ] **Step 1: Failing tests:** style rule (planner has no HTML5 drag); `actions.test.ts` `saveDates keeps other fields and offers Undo`.
- [ ] **Step 2–4:** RED → implement → UI checks PASS; visual in dev server **and** a dev build of the app (WebKit) — `npm run tauri dev` or equivalent: move, both edges, clamp, Esc, Undo, keyboard moves, panel drag and Enter.
- [ ] **Step 5: Commit** `feat(ui): drag bars and place undated tasks on the timeline`

### Task 8: Dependency arrows: draw, select, remove, menus

**Files:** Create `ui/src/main/planner/ArrowLayer.svelte`; modify `PlannerBar.svelte` (connector dot), `PlannerView.svelte` (selection, Delete key, context menu via `lib/menu.svelte.ts`), `ui/src/lib/dependencies.ts` (+ test), i18n, `docs/INSTALL.md` + `README.md` (Planner bullet, Mod+4, dependencies).

**Interfaces — Consumes:** Tasks 2, 4, 5 (`TaskPicker`), 6, 7. **Produces:**
- `arrowPath(from: {x: number; y: number}, to: {x: number; y: number}): string` — SVG path from the end of A (right edge, row middle) to the start of B (left edge, row middle): horizontal out 8 px, vertical to target row, horizontal in; when `to.x < from.x + 16` route around (out 8, down/up half a row, back left past `to.x − 8`, then to target). `arrowConflict(aSpan, bSpan): boolean` = B's from < A's to.
- Arrows drawn for pairs where both rows are visible; warning colour when `arrowConflict`; click selects (thicker), Delete/Backspace removes with Undo; Esc clears selection.
- Connector dot at the bar's right end on hover/focus; pointer-drag to another bar → `api.addDependency(target.id, source.id)` with Undo; refusals show `errorMessage` toast; drop elsewhere nothing.
- Bar context menu (right-click, ContextMenu key, Shift+F10): Open details, Depends on… (TaskPicker with the same exclude rule), Remove dependency ▸ (submenu of current blockers), Remove dates (`saveDates` with all four date fields null/false).
- Strings: `dep.remove` Remove dependency/Rimuovi dipendenza, `planner.removeDates` Remove dates/Rimuovi date, `dep.linked` toast "“{b}” now depends on “{a}”"/"“{b}” ora dipende da “{a}”".

- [ ] **Step 1: Failing tests** `dependencies.test.ts`: `arrowPath` forward case exact string; backward case routes left of target; `arrowConflict` true/false/equal-day false.
- [ ] **Step 2–4:** RED → implement → UI checks PASS; visual (dev server + WebKit dev build): draw, self/cycle toast, conflict colour, select + Delete + Undo, menu actions, keyboard-only Depends on….
- [ ] **Step 5: Commit** `feat(ui): dependency arrows on the timeline`

### Task 9: Verify, install and PR (controller)

- [ ] **Step 1:** Full suites green: cargo test, clippy, UI checks (both TZ), svelte-check.
- [ ] **Step 2:** Read-only snapshot of the real DB → `TASKCRAFT_MIGRATE_CHECK=<copy> cargo test -p taskcraft-store --test migrate_v6 -- --ignored real_db_copy_v6 --nocapture` → PASS.
- [ ] **Step 3 (user-approved):** `./scripts/install.sh`, `./scripts/smoke.sh` (schema 6), check Planner in the installed app; push; PR against `feat/views` if #15 is unmerged (else `main`).
