# Subtasks, checklist, priority and start date — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tasks get one level of full-detail subtasks with parent progress, a checklist, a priority and an optional start date, across store, quick entry, details dialog, board, list and calendar.

**Architecture:** Subtasks are ordinary `tasks` rows with `parent_id`; the store enforces the one-level, same-placement, no-repeat and cascade rules. Checklist items live in a new `checklist_items` table. `Task` carries subtask/checklist counts so the UI never counts by itself for cards. The UI keeps a single `allTasks` array, and pure helpers in `ui/src/lib/subtasks.ts` split it into top-level tasks and children.

**Tech Stack:** Rust (rusqlite, chrono), Tauri 2, Svelte 5 runes, vitest.

**Spec:** `docs/superpowers/specs/2026-10-23-subtasks-checklist-priority-design.md`

## Global Constraints

- Branch `feat/subtasks` (contains phase 1 + drag fix). Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Every user-visible string goes through `ui/src/lib/i18n.ts` with EN and IT; `i18n.test.ts` key parity stays green.
- Colours come from tokens in `ui/src/styles.css`; `tokens.test.ts` (contrast) and `style-rules.test.ts` stay green. Drags allowing only `move` must set `dropEffect = 'move'` (style-rules test).
- Drag-and-drop is never the only way: every drag has a menu/keyboard equivalent.
- No new runtime dependencies.
- No SQL foreign keys on new `tasks` columns; the store enforces integrity. Every task-delete path goes through the one shared cleanup (`delete_task_row`), which removes subtasks, checklist items, alerts and label links.
- Priority values (serde/SQL): `urgent`, `important`, `medium` (default), `low`.
- Quick-entry priority words (case-insensitive, first wins): `!urgent`/`!urgente`, `!important`/`!importante`, `!medium`/`!media`, `!low`/`!bassa`.
- Checklist text: trimmed, 1–200 chars.
- Env: `export PATH="$HOME/.cargo/bin:$PATH"`; UI checks: `cd ui && npx vitest run && npx svelte-check --tsconfig ./tsconfig.app.json`.

## Review Focus

1. Moving a parent (drag to another bucket, Move to bucket, plan change in details, plan/bucket delete with MoveTo): its subtasks must follow; a subtask must never be left in a different plan/bucket. → Task 2 `subtasks_follow_parent_everywhere`.
2. Saving a parent from the details dialog (status Done via the dialog, or editing title) must not lose or reorder its subtasks/checklist, and Undo of a completion must restore subtasks too. → Task 2 `update_parent_keeps_children` + Task 8 `completeUndoRestoresChildren` (UI helper).
3. A filter or date tab that hides a parent but shows its subtask (e.g. subtask due today, parent undated): the subtask appears under a dimmed parent header, never alone and never twice. → Task 6 `groupWithParents` cases.
4. Timed start/due spanning midnight or several weeks, and spans whose start is before the visible range: every day in range shows the task once, with the right start/middle/end marker. → Task 6 `spanRole` cases.
5. Repeating parent completed: advances, its subtasks reopen and its checklist unticks; a non-repeating parent completed with "Only this task" leaves subtasks open. → Task 2 `repeat_reset` + `complete_only_parent`.

---

### Task 1: Schema V4, model and parent/priority/start rules

**Files:**
- Modify: `crates/taskcraft-store/src/schema.rs` (V4, `CURRENT_VERSION = 4`), `model.rs`, `tasks.rs`, `src-tauri/src/commands.rs` (error codes), `scripts/smoke.sh` (expects 4)
- Test: `crates/taskcraft-store/tests/migrate_v4.rs`, `crates/taskcraft-store/tests/subtasks.rs`

**Interfaces:**
- Produces:
  - `pub enum Priority { Urgent, Important, #[default] Medium, Low }` (serde camelCase; `as_sql`/`from_sql`).
  - `pub enum Rule { ParentIsSubtask, HasSubtasks, RepeatingSubtask, StartAfterDue, ChecklistText }` and `StoreError::Rule(Rule)`; command codes `parent_is_subtask`, `has_subtasks`, `repeating_subtask`, `start_after_due`, `checklist_text`.
  - `TaskInput` += `#[serde(default)] parent_id: Option<i64>, priority: Priority, start: Option<DateTime<Utc>>, start_has_time: bool`.
  - `Task` += `parent_id: Option<i64>, priority: Priority, start: Option<DateTime<Utc>>, start_has_time: bool, subtask_open: i64, subtask_total: i64, checklist_done: i64, checklist_total: i64` (counts filled by `get_task` and `list_tasks`; 0 for subtasks' subtask counts).

V4 SQL (`if v < 4`):
```sql
ALTER TABLE tasks ADD COLUMN parent_id INTEGER;
ALTER TABLE tasks ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('urgent','important','medium','low'));
ALTER TABLE tasks ADD COLUMN start_utc TEXT;
ALTER TABLE tasks ADD COLUMN start_has_time INTEGER NOT NULL DEFAULT 0;
CREATE TABLE checklist_items(id INTEGER PRIMARY KEY, task_id INTEGER NOT NULL, text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0, sort REAL NOT NULL);
CREATE INDEX tasks_parent ON tasks(parent_id, sort_order);
CREATE INDEX checklist_task ON checklist_items(task_id, sort);
```

Rules in `insert_task`/`update_task` (in this order): start > due (both set) → `Rule(StartAfterDue)`; `parent_id` given: parent missing → `NotFound`; parent has a parent → `Rule(ParentIsSubtask)`; parent == self → `Invalid`; the task itself has subtasks → `Rule(HasSubtasks)`; the task being made a subtask has an `rrule` → `Rule(RepeatingSubtask)` (a repeating parent is fine). A subtask's plan/bucket are copied from the parent (input plan/bucket ignored); new subtask `sort_order` = max among siblings + 1.0.

- [ ] **Step 1: Write failing tests.** `migrate_v4.rs`: `v3_tasks_keep_everything` (V3 DB built like `migrate_v3.rs` does with 2 tasks in a plan → after open: version 4, `priority == Medium`, `start == None`, `parent_id == None`, plan/bucket/sort unchanged, `.bak-v3` exists). `subtasks.rs`: `insert_subtask_copies_parent_placement`, `subtask_of_subtask_rejected` (→ `Rule(ParentIsSubtask)`), `parent_with_children_cannot_become_subtask` (→ `Rule(HasSubtasks)`), `repeating_subtask_rejected` (→ `Rule(RepeatingSubtask)`), `self_parent_rejected` (→ `Invalid`), `start_after_due_rejected` (→ `Rule(StartAfterDue)`; start == due OK), `priority_and_start_round_trip`, `counts_on_task` (parent with 3 subtasks, 1 done, and 2 checklist rows inserted via plain SQL, 1 done → `subtask_open 2, subtask_total 3, checklist_done 1, checklist_total 2`).
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test migrate_v4 --test subtasks` → FAIL (compile).
- [ ] **Step 3: Implement** V4, model, TASK_COLS/task_from_row, rules, counts (one grouped query per list, not per task), error mapping, smoke.sh `4`. Fix other tests' literals with `..Default::default()`.
- [ ] **Step 4: Run** `cargo test --workspace` and clippy → PASS.
- [ ] **Step 5: Commit** `feat(store): schema v4 with subtasks, priority and start date`

### Task 2: Subtask behaviour in the store

**Files:**
- Create: `crates/taskcraft-store/src/subtasks.rs` (`mod subtasks` in lib.rs)
- Modify: `tasks.rs`, `plans.rs`, `query.rs`
- Test: `crates/taskcraft-store/tests/subtasks.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces (on `Store`): `list_subtasks(parent_id: i64) -> Result<Vec<Task>>` (sibling order); `reorder_subtasks(parent_id: i64, ids: &[i64]) -> Result<()>` (exact set else `Invalid`); `set_task_parent(task_id: i64, parent_id: Option<i64>) -> Result<()>` (same rules as Task 1; `None` promotes: keeps plan/bucket, appended at the end of the bucket via `next_sort`); `complete_with_subtasks(task_id: i64, now: DateTime<Utc>) -> Result<()>` (open subtasks → done, then the task through `set_task_completed`).
- Behaviour changes: `move_task` on a subtask → `Invalid`; moving/re-placing a parent (move_task, update_task placement change, delete_plan/delete_bucket MoveTo) updates its subtasks' plan_id/bucket_id in the same transaction; `delete_task_row` deletes subtasks and checklist items of the task and of its subtasks (plus their alerts/labels); `set_task_completed` on a repeating parent that advances reopens its subtasks (`status 'todo'`, `completed_at NULL`) and unticks its checklist (and its subtasks' checklists); `list_tasks(TaskFilter::Today)` excludes tasks whose `start` is after the end of today (tz-local); `list_plans().open_count` counts top-level open tasks only.

- [ ] **Step 1: Write failing tests** in `tests/subtasks.rs`: `list_and_reorder_subtasks`; `subtasks_follow_parent_everywhere` (parent with 2 subtasks: move_task to another bucket → subtasks' bucket follows; update_task plan change → follow; delete_bucket MoveTo → follow; delete_plan MoveTo → follow); `move_subtask_rejected`; `promote_subtask_appends_to_bucket`; `demote_task_under_parent` (sort after existing siblings); `delete_parent_cascades` (subtask, checklist rows of both, alerts of the subtask → all gone; labels remain); `delete_plan_delete_tasks_cascades`; `repeat_reset` (daily repeating parent, one subtask done, one checklist row done → set_task_completed advances due, subtask status todo, checklist undone); `complete_with_subtasks_marks_all`; `complete_only_parent` (set_task_completed on non-repeating parent leaves subtasks open); `update_parent_keeps_children` (update_task title/status of parent → subtasks and checklist unchanged); `today_excludes_not_started` (due today, start tomorrow → not in Today; start today → in Today); `plan_open_count_top_level_only`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test subtasks` → FAIL.
- [ ] **Step 3: Implement.** Keep the subtask-following update in one crate-private helper `sync_children_placement(conn, parent_id)` called from every placement path.
- [ ] **Step 4: Run** `cargo test --workspace`, clippy → PASS.
- [ ] **Step 5: Commit** `feat(store): subtasks follow, cascade and reset with their parent`

### Task 3: Checklist in the store

**Files:**
- Create: `crates/taskcraft-store/src/checklist.rs`
- Test: `crates/taskcraft-store/tests/checklist.rs`

**Interfaces:**
- Produces: `pub struct ChecklistItem { id: i64, task_id: i64, text: String, done: bool, sort: f64 }` (serde camelCase); `list_checklist(task_id) -> Result<Vec<ChecklistItem>>`; `add_checklist_item(task_id, text: &str) -> Result<i64>` (appended); `update_checklist_item(id, text: Option<&str>, done: Option<bool>) -> Result<()>`; `reorder_checklist(task_id, ids: &[i64]) -> Result<()>`; `delete_checklist_item(id) -> Result<()>`; `checklist_item_to_subtask(id) -> Result<i64>` (new subtask of the item's task, title = text, inherits the parent's calendar and tz, appended last among siblings; item deleted; item on a subtask → `Rule(ParentIsSubtask)`). Text rule → `Rule(ChecklistText)`. Missing task/item → `NotFound`.

- [ ] **Step 1: Write failing tests**: `add_list_order`, `text_trimmed_and_limited` (`"  a  "` → `"a"`; `""`/201 chars → `Rule(ChecklistText)`; 200 OK), `tick_and_rename`, `reorder_exact_set`, `convert_to_subtask` (title, parent, item gone), `convert_on_subtask_rejected`, `missing_task_not_found`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test checklist` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace`, clippy → PASS.
- [ ] **Step 5: Commit** `feat(store): task checklists`

### Task 4: Priority words in the parser

**Files:**
- Modify: `crates/taskcraft-parse/src/types.rs` (`Label::Priority`, `ParsedItem.priority: Option<ParsedPriority>` with `ParsedPriority { Urgent, Important, Medium, Low }` serde camelCase), `lib.rs` (`mark_priority` next to `mark_tags`, tasks only)
- Test: `crates/taskcraft-parse/tests/priority.rs`

- [ ] **Step 1: Write failing tests**: `priority_words_en_it` (each of the 8 words, any case, e.g. `"call mom !URGENTE friday"` → `Urgent`, title `"call mom"`, one `Label::Priority` span); `first_priority_wins` (`"x !low !urgent"` → `Low`, `"!urgent"` stays in title); `other_bang_words_untouched` (`"wow!"`, `"!yay"`, `"hi !"` → priority None, title unchanged vs. a parse with no priority logic); `events_ignore_priority` (`parse("dinner !urgent tomorrow")` → None).
- [ ] **Step 2: Run** `cargo test -p taskcraft-parse --test priority` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** `feat(parse): priority words in quick entry`

### Task 5: Tauri commands

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`, `crates/taskcraft-store/src/time.rs` (`from_parsed` maps `ParsedPriority` → `Priority`)

**Interfaces:**
- Produces commands (camelCase args): `list_subtasks(parentId)`, `reorder_subtasks(parentId, ids)`, `set_task_parent(taskId, parentId|null)`, `complete_with_subtasks(id)`, `list_checklist(taskId)`, `add_checklist_item(taskId, text)`, `update_checklist_item(id, text|null, done|null)`, `reorder_checklist(taskId, ids)`, `delete_checklist_item(id)`, `checklist_item_to_subtask(id) -> i64`. Mutations call `changed(&app)`.

- [ ] **Step 1: Write failing tests** in `commands::tests`: `quick_entry_sets_priority` (`create_from_text_impl("pay rent !urgente", …, true)` → task priority Urgent, title "pay rent"); `rule_errors_have_codes` (each `Rule` → its code).
- [ ] **Step 2: Run** `cargo test -p taskcraft-app commands::tests` → FAIL.
- [ ] **Step 3: Implement** and register.
- [ ] **Step 4: Run** `cargo test --workspace`, clippy → PASS.
- [ ] **Step 5: Commit** `feat(app): commands for subtasks and checklists`

### Task 6: UI data layer and pure helpers

**Files:**
- Modify: `ui/src/lib/types.ts`, `api.ts` (wrappers + `TRANSLATED_ERRORS` += the 5 rule codes), `i18n.ts`, `styles.css` (+ `--warning` light `#b45309` / dark `#fbbf24`; add `'warning'` to the TEXT list in `tokens.test.ts`), `dev/mock.ts` (subtasks/checklist/priority/start in memory, all new commands), `quick/HighlightInput.svelte` (`priority: '--label-alert'`), `lib/tasks.ts` (today view excludes not-started tasks)
- Create: `ui/src/lib/subtasks.ts`
- Test: `ui/src/lib/subtasks.test.ts`, `ui/src/lib/tasks.test.ts`, `ui/src/lib/calendarTasks.test.ts`

**Interfaces:**
- Produces in `subtasks.ts`:
  - `childrenByParent<T extends {id:number; parentId:number|null; sortOrder:number}>(tasks: T[]): Map<number, T[]>` (sibling order) and `topLevel<T>(tasks: T[]): T[]`.
  - `groupWithParents<T>(shown: T[], all: T[]): { parent: T; dimmed: boolean; children: T[] }[]` — every shown top-level task with its shown children; a shown subtask whose parent is not shown appears under its parent with `dimmed: true`; a parent appears once.
  - `progressLabel(t: {subtaskOpen:number; subtaskTotal:number}): string | null` — `t('subtasks.progress', {done, total})` ("2 of 4"/"2 di 4"), null when total 0; `checklistLabel` likewise ("3/5").
  - `completionAsk(task, children): 'askChildren' | 'none'` (done requested on a task with open children → ask) and `offerParentAfter(task, siblings, parent): boolean` (the just-completed subtask was the last open sibling and the parent is open).
  - `priorityColumns<T>(tasks: T[]): {key: Priority; tasks: T[]}[]` (Urgent, Important, Medium, Low; order by sortOrder).
- Produces in `calendarTasks.ts`: `spanRole(task, day: Date): 'start' | 'middle' | 'end' | null` (null when no start or start and due on the same day) and `tasksForDay` puts spanning tasks in `allDay` on every covered day with a `span` field on `DayTask`; `shiftStart(task, newDue: string): string | null` (start moved by the same delta as due).
- Types: `Priority`, `ChecklistItem`, Task/TaskInput fields from Task 1, `ParsedItem.priority`, `Label` += `'priority'`.
- i18n keys (EN/IT): `priority.urgent` Urgent/Urgente, `priority.important` Important/Importante, `priority.medium` Medium/Media, `priority.low` Low/Bassa, `priority.title` Priority/Priorità, `groupBy.priority` Priority/Priorità, `start.title` Start/Inizio, `subtasks.title` Subtasks/Sottotask, `subtasks.add` Add subtask/Aggiungi sottotask, `subtasks.progress` "{done} of {total}"/"{done} di {total}", `subtasks.partOf` "Part of: {title}"/"Parte di: {title}", `subtasks.parent` Parent task/Attività principale, `subtasks.none` None/Nessuna, `subtasks.makeTask` Make normal task/Rendi attività normale, `subtasks.expand`/`subtasks.collapse` Show subtasks/Mostra sottotask, Hide subtasks/Nascondi sottotask, `checklist.title` Checklist, `checklist.add` Add item/Aggiungi voce, `checklist.toSubtask` Make subtask/Trasforma in sottotask, `complete.askChildren` "Complete the {n} open subtasks too?"/"Completare anche i {n} sottotask aperti?" (+ One variant), `complete.all` Complete all/Completa tutto, `complete.onlyThis` Only this task/Solo questa attività, `complete.allDone` "All subtasks of {title} are done"/"Tutti i sottotask di {title} sono completati", `complete.parent` Complete it/Completala, `delete.withSubtasks` "…and its {n} subtasks"/"…e i suoi {n} sottotask", `error.parent_is_subtask` "A subtask can't have subtasks."/"Un sottotask non può avere sottotask.", `error.has_subtasks` "This task has subtasks, so it can't become a subtask."/"Questa attività ha sottotask, quindi non può diventare un sottotask.", `error.repeating_subtask` "Subtasks can't repeat."/"I sottotask non possono ripetersi.", `error.start_after_due` "The start can't be after the due date."/"L'inizio non può essere dopo la scadenza.", `error.checklist_text` "Checklist items need 1 to 200 characters."/"Le voci della checklist devono avere da 1 a 200 caratteri.".

- [ ] **Step 1: Write failing tests**: `subtasks.test.ts` — `childrenByParent orders siblings`; `groupWithParents` (parent+child shown; child shown without parent → dimmed parent; child and parent both shown → parent once); `progressLabel` (0 → null; 2/4 → "2 of 4"); `completionAsk`; `offerParentAfter` (last open sibling → true; other sibling still open → false; parent already done → false); `priorityColumns`. `calendarTasks.test.ts` — `spanRole` (start Mon due Wed: Mon start, Tue middle, Wed end, Thu null; same-day → null; timed span across midnight → start/end) and `tasksForDay` shows a spanning task once per covered day in allDay with its role; `shiftStart` keeps the delta. `tasks.test.ts` — today view excludes a task whose start is tomorrow.
- [ ] **Step 2: Run** `cd ui && npx vitest run` → new tests FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `npx vitest run && npx svelte-check --tsconfig ./tsconfig.app.json` → PASS.
- [ ] **Step 5: Commit** `feat(ui): subtask, checklist and priority data layer`

### Task 7: Details dialog: priority, start, subtasks, checklist, parent

**Files:**
- Modify: `ui/src/main/Inspector.svelte`, `ui/src/lib/edit.ts` (+ tests in `edit.test.ts` for the new fields round-tripping through `taskInput`/`mergeTaskSave`)
- Create: `ui/src/main/tasks/SubtaskList.svelte`, `ui/src/main/tasks/Checklist.svelte`

**Interfaces:**
- Consumes: Task 5 commands via `api`, Task 6 helpers/i18n, `lib/menu.svelte.ts` popup menu pattern.
- Produces: `SubtaskList` props `{ parent: Task; subtasks: Task[]; onopen: (t: Task) => void; onerror: (m: string) => void }`; `Checklist` props `{ task: Task; canConvert: boolean; onerror }` (loads its items with `api.listChecklist`, reloads on items-changed).
- Inspector: Priority select; Start date (+ time when Due has a time) next to Due with inline `error.start_after_due` when invalid (Save disabled); on a top-level task: SubtaskList and Checklist sections; on a subtask: "Part of" link (opens parent), no Repeat field; Parent picker (top-level tasks of the same plan without a parent, excluding itself; "None"; hidden for tasks that have subtasks or repeat) calling `api.setTaskParent`. Delete confirmation includes `delete.withSubtasks` when it has subtasks.
- SubtaskList rows: StatusGlyph (cycles status via `changeStatus`), title button (opens), due, labels; "Add subtask" input creates via `api.createTask({ …defaults, parentId })` and keeps focus; reorder by drag (dropEffect move) and row menu Move up/Move down/Open/Make normal task/Delete.
- Checklist rows: checkbox, inline-editable text (Enter/blur saves, Esc reverts), Alt+↑/↓ and drag reorder, row menu Move up/Move down/Make subtask (only if `canConvert`)/Delete; "Add item" input (Enter adds, keeps focus).

- [ ] **Step 1: Write failing tests** in `edit.test.ts`: `taskInput carries priority, start and parent`; `mergeTaskSave keeps the latest parent when the dialog didn't change it`.
- [ ] **Step 2: Run** `npx vitest run src/lib/edit.test.ts` → FAIL.
- [ ] **Step 3: Implement** helpers then components.
- [ ] **Step 4: Run** full UI checks → PASS; visual (dev server + Claude in Chrome if available, EN/IT, light/dark): add/reorder/open/promote subtasks, checklist add/tick/edit/reorder/convert, priority, start validation, parent picker.
- [ ] **Step 5: Commit** `feat(ui): subtasks and checklist in the details dialog`

### Task 8: Board, list, completion prompts, sidebar and quick window

**Files:**
- Modify: `ui/src/main/Main.svelte` (pass `topLevel(allTasks)` to boards/lists/sidebar, `childrenByParent` map down), `ui/src/main/tasks/BoardCard.svelte` (priority mark, "▸ 2 of 4" + bar, "☑ 3/5", start→due "24 → 28 ott", expandable children with status circles; expanded set per session in a module-level `Set`), `BucketBoard.svelte` / `TaskBoard.svelte` / `TasksView.svelte` (Group by Priority: drag between columns sets priority; card menu Priority ▸), `TaskRow.svelte` + `TasksView.svelte` list (indented, collapsible children via `groupWithParents`), `ui/src/lib/actions.ts` (`changeStatus` consults `completionAsk`/`offerParentAfter`), `ui/src/main/TaskList.svelte` (sidebar: top-level only with progress), `ui/src/quick/Quick.svelte` + `Upcoming.svelte` (top-level only with progress)
- Create: `ui/src/lib/complete.svelte.ts` (pending completion request store), `ui/src/main/CompleteDialog.svelte`
- Test: `ui/src/lib/subtasks.test.ts` (+ cases below)

**Interfaces:**
- Consumes: Tasks 5–7.
- Produces: `complete.svelte.ts` exports `requestComplete(task: Task, openChildren: number): Promise<'all' | 'only' | 'cancel'>` and reactive `pending` for `CompleteDialog` (rendered once in Main; modal; default button "Complete all"; Esc = cancel). `changeStatus(task, 'done', ctx)` where `ctx = { children: Task[]; siblings: Task[]; parent: Task | null }`: asks when `completionAsk` says so (`all` → `api.completeWithSubtasks`, `only` → `api.setTaskStatus`), else completes; then if `offerParentAfter` → `notify(t('complete.allDone',…), action t('complete.parent') → changeStatus(parent,'done', …))`. Undo after "Complete all" restores the parent and the subtasks it closed (`completeUndo(before: TaskInput, closed: {id, input}[])` pure helper building the undo calls).
- Pure helpers + tests: `completeUndo` (`completeUndoRestoresChildren`: returns one update per closed child plus the parent).

- [ ] **Step 1: Write failing tests**: `completeUndoRestoresChildren`; `groupWithParents` with collapsed parents keeps headers (if a collapse helper is added).
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** full UI checks → PASS; visual: card progress/expand/tick, list indent/collapse, dimmed parent header, Group by Priority drag + card menu, completion dialog (all/only/cancel, Undo), last-subtask toast, sidebar and quick window show top-level with progress.
- [ ] **Step 5: Commit** `feat(ui): subtask progress on cards and lists, completion prompts, priority grouping`

### Task 9: Calendar: subtask titles and start→due spans

**Files:**
- Modify: `ui/src/main/DayWeekGrid.svelte`, `ui/src/main/MonthGrid.svelte`, `ui/src/main/Main.svelte` (`moveTask` shifts start with `shiftStart`; Undo restores both)

- Subtask entries render "Subtask · Parent" (parent title from the map); spanning tasks render in the all-day strip with start/middle/end styling (rounded/flat ends, "↦"/"→" markers in an accessible label: `t('span.start'|'span.middle'|'span.end')` — add EN "starts"/"continues"/"ends", IT "inizia"/"continua"/"finisce"); keyboard ←/→ and drag on any span day shift start and due together.
- [ ] **Step 1: Write failing test** in `calendarTasks.test.ts`: `movedDue on a span also shifts start` via `shiftStart` used by a new `movedSpan(task, key): { due: string; start: string | null } | null`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** full UI checks → PASS; visual in Day, Week, Month (EN/IT, light/dark).
- [ ] **Step 5: Commit** `feat(ui): subtasks and multi-day tasks on the calendar`

### Task 10: Verify, docs, install and PR

**Files:** `docs/INSTALL.md` (one bullet: subtasks/checklist in the details dialog, priority words `!urgent`/`!urgente`), `README.md` features list.

- [ ] **Step 1:** Full suites: `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, UI checks → green.
- [ ] **Step 2:** Snapshot the real DB read-only (`sqlite3 -readonly "file:…?mode=ro" ".backup <scratch>"`), migrate a copy with an ignored `real_db_copy_v4` test in `migrate_v4.rs` (env `TASKCRAFT_MIGRATE_CHECK`) → same task count, version 4.
- [ ] **Step 3:** With the user's OK: `./scripts/install.sh`, `./scripts/smoke.sh` (schema 4), push `feat/subtasks`, open a PR against `feat/planners` if PR #12 is not merged yet, else against `main`.
- [ ] **Step 4: Commit** `docs: subtasks, checklist and priority in the guide`
