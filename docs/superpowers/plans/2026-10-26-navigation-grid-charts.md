# Navigation, Grid view and charts — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Day/Week/Month/Tasks switcher with Calendar · Tasks · Grid areas, and add an editable Grid table with a charts panel.

**Architecture:** UI-only (no schema or command changes). Pure logic lives in three new modules — `lib/navigation.ts`, `lib/grid.ts`, `lib/gridStats.ts` — each with vitest; components `main/grid/GridView.svelte` (owner), `GridTable.svelte`, `GridCell*.svelte` editors and `ChartsPanel.svelte` render them. All edits go through the existing api/actions so every phase-2/3 rule (changeStatus prompts, auto-start, Undo, taskInput field preservation) applies.

**Tech Stack:** Svelte 5 runes, TypeScript, vitest; plain SVG for charts.

**Spec:** `docs/superpowers/specs/2026-10-26-navigation-grid-charts-design.md`

## Global Constraints

- Branch `feat/views` (from `feat/templates`). Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No new dependencies (no chart library). Implementers of chart code read the `dataviz` skill first.
- Strings in `ui/src/lib/i18n.ts` EN+IT; tokens only (style-rules/tokens tests green); drags allowing only `move` set `dropEffect`; every action reachable by keyboard.
- localStorage keys exactly: `taskcraft.calendarMode`, `taskcraft.grid.columns.<scope>`, `taskcraft.grid.charts`; every read/write in try/catch, UI works without storage.
- Top-level-only counts for stats (match open counts elsewhere); overdue = existing `isOverdue`; weeks use the user's weekStart; local dates.
- Mod+1/2/3 = Calendar/Tasks/Grid; Mod+4 reserved (do nothing yet); D/W/M only in Calendar and only when focus is not in a text field.
- Checks: `cd ui && npx vitest run && TZ=Europe/Rome npx vitest run && npx svelte-check --tsconfig ./tsconfig.app.json`.

## Review Focus

1. Inline edits from the grid must not reset other fields (priority, start, parentId, plan/bucket, attachments) or bypass completion prompts/auto-start. → Task 4 test `gridEditUsesTaskInput` (pure helper building the update) + visual.
2. Sorting with empty values and subtasks: subtasks never leave their parent; empty values last in both directions; third click restores default order. → Task 2 `gridRows` cases.
3. Weekly chart across DST and new year with Monday vs Sunday week start. → Task 3 `completedPerWeek` cases.
4. Pressing D/W/M while typing in any input (quick entry, grid title editor, search) must type the letter, not switch modes. → Task 1 `shortcutFor` cases.
5. A chart filter that leaves zero rows shows an empty table with the chip still visible to clear it; switching plan clears the chart filter. → Task 5 `chartFilter` + visual.

---

### Task 1: Navigation areas and calendar mode

**Files:** Create `ui/src/lib/navigation.ts` (+ `navigation.test.ts`). Modify `ui/src/main/Main.svelte` (replace `view` with `area` + `calendarMode`), `ui/src/main/Toolbar.svelte`, `ui/src/lib/i18n.ts`, `docs/INSTALL.md` (views sentence).

**Interfaces — Produces:** `type Area = 'calendar' | 'tasks' | 'grid'`; `type CalendarMode = 'day' | 'week' | 'month'`; `loadCalendarMode(): CalendarMode` / `saveCalendarMode(m)` (key `taskcraft.calendarMode`, default `'week'`, safe without storage); `shortcutFor(e: {key: string; mod: boolean; inTextField: boolean}, area: Area): { area?: Area; mode?: CalendarMode } | null` (Mod+1/2/3 → area; Mod+4 → null; D/W/M without mod, not in text field, only when area is calendar → mode; anything else null). Main exposes `area` to the sidebar so selecting a plan keeps `grid` when already in grid, else goes to `tasks`. i18n: `area.calendar` Calendar/Calendario, `area.tasks` Tasks/Attività, `area.grid` Grid/Griglia (keep `toolbar.day/week/month`).

- [ ] **Step 1: Failing tests** `navigation.test.ts`: Mod+1/2/3 mapping; Mod+4 → null; `d` in calendar not in text field → `{mode:'day'}`; `d` with `inTextField: true` → null; `w` in tasks area → null; `M` uppercase also works; load/save round-trip; load with throwing localStorage → `'week'`; invalid stored value → `'week'`.
- [ ] **Step 2: Run** `cd ui && npx vitest run src/lib/navigation.test.ts` → FAIL.
- [ ] **Step 3: Implement** helpers, then Main/Toolbar: area buttons (Calendar/Tasks/Grid, with Mod+N hints), Day|Week|Month segmented control + title + prev/next/today only in Calendar; Grid area renders a placeholder `<GridView>` stub file (`main/grid/GridView.svelte` with an empty-state message) that Task 4 fills; "+" = new event in Calendar, new task in Tasks/Grid. Update INSTALL.md sentence to "Calendar (day, week, month), Tasks and Grid".
- [ ] **Step 4: Run** full UI checks → PASS; visual: switching areas/modes, shortcuts, mode remembered after reload.
- [ ] **Step 5: Commit** `feat(ui): Calendar, Tasks and Grid areas`

### Task 2: Grid rows, sorting, columns and keyboard model

**Files:** Create `ui/src/lib/grid.ts` (+ `grid.test.ts`).

**Interfaces — Produces:**
- `type GridColumn = 'title' | 'status' | 'priority' | 'bucket' | 'plan' | 'labels' | 'start' | 'due' | 'progress' | 'attachments'`; `ALL_COLUMNS` in that order; `defaultColumns(scope: number | 'all'): GridColumn[]` (all; `plan` only when scope is `'all'`).
- `type GridSort = { column: GridColumn; dir: 'asc' | 'desc' } | null`; `nextSort(current: GridSort, column: GridColumn): GridSort` (asc → desc → null).
- `interface GridRow<T> { task: T; depth: 0 | 1; dimmed: boolean; hasChildren: boolean; collapsed: boolean }`; `gridRows<T extends Task-like>(tasks: T[], opts: { scope; buckets: Bucket[]; plans: Plan[]; labels: number[]; status: StatusFilter; chart: ChartFilter | null; sort: GridSort; collapsed: Set<number> }): GridRow<T>[]` — top-level rows in default order (bucket sort then sortOrder; in All: plan sort, bucket sort, sortOrder) or by `sort` (empty values last in both directions; ties by default order); subtasks under parents, sorted among siblings by the same key, hidden when collapsed; filtered subtask with hidden parent → dimmed parent row (reuse `groupWithParents` from lib/subtasks.ts).
- `loadColumns(scope)` / `saveColumns(scope, cols)` (key `taskcraft.grid.columns.<scope>`, title always included, unknown names dropped, safe without storage).
- `nextCell(pos: {row: number; col: number}, key: string, shape: {rows: number; cols: number}): {row: number; col: number}` (arrows clamp at edges, Home/End within row).
- `type ChartFilter = { kind: 'status'; value: TaskStatus } | { kind: 'priority'; value: Priority } | { kind: 'group'; value: number }` is defined here (grid.ts) and used by `gridRows` to filter; Task 3 imports it.

- [ ] **Step 1: Failing tests** `grid.test.ts`: default order by bucket then sortOrder; All scope by plan then bucket; sort by due asc/desc with nulls last both ways; third click restores default; subtasks stay under parent and sort among themselves; chart filter by status/priority/group (group = bucketId in a plan, planId in All); collapsed parent hides children; dimmed parent for filtered subtask; columns load/save incl. storage throwing and unknown names; nextCell edges and Home/End.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4:** UI checks → PASS.
- [ ] **Step 5: Commit** `feat(ui): grid rows, sorting and keyboard model`

### Task 3: Grid statistics and chart filter

**Files:** Create `ui/src/lib/gridStats.ts` (+ `gridStats.test.ts`).

**Interfaces — Produces:**
- `interface GridStats { open: number; inProgress: number; overdue: number; completed7d: number; byStatus: {key: TaskStatus; n: number}[]; byPriority: {key: Priority; n: number}[]; byGroup: {key: number; label: string; color?: string; n: number}[]; perWeek: {weekStart: Date; n: number}[] }`.
- `gridStats(tasks, opts: { scope; buckets; plans; now: Date; weekStart: 0 | 1 }): GridStats` — top-level tasks only; byGroup = buckets in order (plan scope) or plans in order (All); perWeek = 8 weeks ending with the current week, local week starts.
- Imports `ChartFilter` from `lib/grid.ts` (Task 2); `toggleChartFilter(current, next): ChartFilter | null` (same → null). Filtering itself happens in Task 2's `gridRows`.

- [ ] **Step 1: Failing tests:** tiles counts (subtasks excluded; overdue via isOverdue incl. date-only today not overdue; completed within 7×24h boundary); byStatus/byPriority order and zeros; byGroup in bucket order and plan order for All; `completedPerWeek` across Europe/Rome DST (29 Mar, 25 Oct 2026) and the 2026→2027 year boundary for weekStart 0 and 1; toggle same filter clears.
- [ ] **Step 2–4:** RED → implement → `TZ=Europe/Rome npx vitest run` and default → PASS.
- [ ] **Step 5: Commit** `feat(ui): grid statistics and chart filter`

### Task 4: Grid table view with inline editing

**Files:** Fill `ui/src/main/grid/GridView.svelte` (owner: scope, filters, sort, collapsed, columns, chart filter state; header with Columns menu and filter chip), create `ui/src/main/grid/GridTable.svelte` and cell editors (`GridTitleCell.svelte`, menu-based cells reuse `lib/menu.svelte.ts`), modify `Main.svelte` (pass tasks/plans/buckets/labels/scope/weekStart; open details), `ui/src/lib/i18n.ts` (column headers, Columns menu, sort hints, chip text, empty states). Create `ui/src/lib/gridEdit.ts` (+ test).

**Interfaces — Consumes:** Tasks 1–3; `api.updateTask`, `api.moveTask`, `changeStatus` (lib/actions.ts, with ctx like other views — reuse Main's `statusCtx`), `taskInput` (lib/edit.ts), `notify` for Undo, `progressSegments`/`barTrack` for the progress cell.
**Produces:** `gridEdit.ts`: `editInput(task: Task, field: 'title' | 'priority' | 'start' | 'due', value): TaskInput` built on `taskInput(task)` so all other fields are preserved; `bucketMove(task, bucketId, columnTasks): { bucketId; before: null; after: number | null }` appending at the end (top-level only).

- ARIA grid (`role="grid"`, rows/gridcells, `aria-sort` on headers, `aria-rowindex`), roving tabindex via `nextCell`; Enter/F2 edit, Esc cancel, Space cycles status, Mod+Enter opens details, Left/Right on a parent's title collapse/expand; double-click row opens details. Subtask rows: Bucket cell read-only. Edits: title (empty refused inline), status (changeStatus), priority (updateTask via editInput), bucket (moveTask), start/due (date input keeping time; start>due shows translated error). Each edit offers Undo like other views. Plan column only in All.
- [ ] **Step 1: Failing tests** `gridEdit.test.ts`: `gridEditUsesTaskInput` (priority edit keeps start, parentId, labels, alerts, plan/bucket); due edit keeps time-of-day when the task had one; `bucketMove` appends after the last top-level task and refuses subtasks.
- [ ] **Step 2–4:** RED → implement → UI checks PASS; visual (dev server, EN/IT, light/dark): sort, every editor, Undo, collapse, keyboard-only walk, columns menu persistence, 500-row mock responsiveness (add a dev-only mock toggle `?bulk=500` if needed).
- [ ] **Step 5: Commit** `feat(ui): editable grid view`

### Task 5: Charts panel

**Files:** Create `ui/src/main/grid/ChartsPanel.svelte` (+ small `BarChart.svelte`, `StatTile.svelte`); modify `GridView.svelte` (panel above the table, collapse persisted in `taskcraft.grid.charts`, chart filter chip), i18n (tile and chart titles, "No tasks yet", chip "{chart}: {value} ×", text alternatives).

**Interfaces — Consumes:** Task 3 `gridStats`, `ChartFilter`, `toggleChartFilter`.
- Read the `dataviz` skill before writing chart code. Horizontal bar charts in SVG; values as text on bars; status colours from status tokens, priority from danger/warning/neutral tokens, groups one neutral series (plan colour for plans); light/dark; each chart has a visually hidden table of its values; bars of Status/Priority/Group are buttons (keyboard + click) that toggle the chart filter with `aria-pressed`; weekly chart not interactive. Switching scope clears the chart filter.
- [ ] **Step 1: Failing test** (if any new pure helper, e.g. `barGeometry(values, width)` for scaling with a zero max) → RED. **Step 2–4:** implement → UI checks PASS; visual: tiles, all charts EN/IT light/dark, click-to-filter and chip clear, empty scope.
- [ ] **Step 5: Commit** `feat(ui): charts panel in the grid`

### Task 6: Verify, docs, install and PR

- [ ] **Step 1:** Full suites: `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, UI checks → green.
- [ ] **Step 2:** `docs/INSTALL.md` "Using TaskCraft": one bullet on the Grid (table you can sort and edit, charts you can click to filter) and the new menu/shortcuts; README features line.
- [ ] **Step 3 (controller, user-approved):** install, smoke, check navigation + grid + charts in the installed app, push, PR against `feat/templates` if #14 is unmerged (else `main`).
- [ ] **Step 4: Commit** `docs: grid and charts in the guide`
