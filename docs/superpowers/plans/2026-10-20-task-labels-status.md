# Task Labels, Status and Tasks Page Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional `#labels` and a To do / In progress / Done status to tasks, with a List/Board Tasks page redesigned via /frontend-design.

**Architecture:** Schema V2 adds `tasks.status`, `labels`, `task_labels`; the store keeps `status='done' ⇔ completed_at IS NOT NULL`. The parser tags `#word` tokens in task text. New Tauri commands expose status/labels; pure TS helpers do filtering and board grouping; Svelte components render List, Board, filters and a label manager.

**Tech Stack:** Rust (rusqlite, chrono, serde), Tauri 2, Svelte 5 runes, TypeScript, vitest.

**Spec:** `docs/superpowers/specs/2026-10-20-task-labels-status-design.md`

## Global Constraints

- Statuses fixed: SQL `todo` / `in_progress` / `done`; IPC (serde camelCase) `todo` / `inProgress` / `done`; display "To do", "In progress", "Done".
- Invariant after every store write: `status = 'done'` ⇔ `completed_at IS NOT NULL`.
- Labels on tasks only; name 1–32 chars, unique case-insensitively (`COLLATE NOCASE`), stored with original spelling of first use.
- Label palette (round-robin by existing label count): `#3b82f6 #f97316 #10b981 #a855f7 #ef4444 #eab308 #06b6d4 #ec4899`.
- Board: Done column shows 30 most recent unless "Show all"; within columns sort by due (none last) then id.
- Redesign limited to the Tasks page plus Inspector status/label fields; use existing CSS tokens; respect `prefers-reduced-motion`.
- Every mutating command emits `items-changed`.
- Clean-room, personal use; no new runtime dependencies (UI: none; Rust: none).
- Commit trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commands need `export PATH="$HOME/.cargo/bin:$PATH"`.

## Review Focus

1. Recurring task dragged to Done on the board → advances to next due, stays visible in To do (not lost, not marked complete forever). Test in Task 2.
2. Renaming a label to an existing name with different case (`Home` → `home` vs another `HOME`) → merge, no UNIQUE error, no duplicate links. Test in Task 3.
3. Saving a task in the Inspector without touching labels/status must keep them. `update_task` replaces labels with what it receives, so the UI must always send `status` and `labels`. Test in Task 6 (`formToTask(taskToForm(x))` round-trip).
4. Upgrading a real V1 database that is in WAL mode with uncommitted-to-main-file data → backup contains the rows. Test in Task 1.
5. `#` edge inputs: `C# meetup`, `#1 priority`, `##`, `#home,` (trailing comma), event text `Party #fun`. Test in Task 4.

---

### Task 1: Schema V2 and WAL-safe backup

**Files:**
- Modify: `crates/taskcraft-store/src/schema.rs`, `crates/taskcraft-store/src/lib.rs` (backup in `Store::open`)
- Test: `crates/taskcraft-store/tests/migrate_v2.rs`

**Interfaces:**
- Produces: `schema::CURRENT_VERSION == 2`; tables `labels(id, name UNIQUE COLLATE NOCASE, color)`, `task_labels(task_id, label_id)` with `ON DELETE CASCADE`; `tasks.status` with CHECK.

- [ ] **Step 1: Write failing tests** in `tests/migrate_v2.rs` (build the V1 file by hand with `rusqlite` executing the V1 DDL copied from `schema.rs` and `PRAGMA user_version=1`, `journal_mode=WAL`, keeping that connection open while calling `Store::open`):
  - `v1_completed_task_becomes_done`: V1 row with `completed_at` set → after `Store::open`, `get_task(id).status == TaskStatus::Done`; open row → `Todo`; `labels` empty.
  - `backup_includes_wal_rows`: rows inserted through a WAL connection that is still open (no checkpoint) → `taskcraft.db.bak-v1` opened with rusqlite has the same `SELECT count(*) FROM tasks`.
  - `fresh_db_is_v2`: `Store::open_in_memory()` → `schema_version() == 2`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test migrate_v2` — Expected: FAIL (compile error: no `status`/`TaskStatus`, or version 1).
- [ ] **Step 3: Implement.** Add `const V2` with the spec's SQL; `migrate` runs `if v < 2 { V2 }`. In `Store::open` replace `std::fs::copy` with `conn.execute("VACUUM INTO ?1", [bak_path_str])` (delete a stale backup file first; VACUUM INTO refuses to overwrite). Add `TaskStatus` (see Task 2 Interfaces) and `status` in `Task`/`task_from_row` (column added to `TASK_COLS`) now so the test compiles.
- [ ] **Step 4: Run** `cargo test -p taskcraft-store` — Expected: all pass (existing `crud.rs` backup tests still green).
- [ ] **Step 5: Commit** `feat(store): schema v2 (status, labels) and WAL-safe migration backup`.

### Task 2: Task status in the store

**Files:**
- Modify: `crates/taskcraft-store/src/model.rs`, `crates/taskcraft-store/src/tasks.rs`; existing `TaskInput { .. }` literals in `crates/taskcraft-store/tests/*.rs` and `src/time.rs` (add `..Default::default()`)
- Test: `crates/taskcraft-store/tests/status.rs`

**Interfaces:**
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)] #[serde(rename_all = "camelCase")] pub enum TaskStatus { #[default] Todo, InProgress, Done }` with `fn as_sql(self) -> &'static str` and `fn from_sql(&str) -> TaskStatus` (unknown → `Todo`).
  - `TaskInput` derives `Default`; gains `#[serde(default)] pub status: TaskStatus` and `#[serde(default)] pub labels: Vec<String>`.
  - `Task` gains `pub status: TaskStatus`, `pub labels: Vec<TaskLabel>` (labels filled in Task 3; empty here).
  - `Store::set_task_status(&self, id: i64, status: TaskStatus, now: DateTime<Utc>) -> Result<()>`.

- [ ] **Step 1: Write failing tests** in `tests/status.rs`:
  - `in_progress_round_trip`: insert, `set_task_status(id, InProgress)` → `status == InProgress`, `completed_at == None`.
  - `done_sets_completed_at_and_back`: `Done` → `completed_at.is_some()`; then `Todo` → `None`, `status == Todo`.
  - `checkbox_keeps_invariant`: `set_task_completed(id, true)` → `status == Done`; `false` → `Todo`.
  - `recurring_done_advances_and_resets` (Review Focus 1): weekly task due 2026-10-20 09:00 Europe/Rome, status `InProgress`; `set_task_status(id, Done)` → due 2026-10-27 09:00 local, `completed_at None`, `status == Todo`.
  - `insert_with_done_status_sets_completed_at`: `TaskInput { status: Done, .. }` → `completed_at.is_some()`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test status` — Expected: FAIL (no `set_task_status`).
- [ ] **Step 3: Implement.** `insert_task`/`update_task` write `status` and set `completed_at` = existing value or `now` when Done, NULL otherwise. `set_task_status(Done)` delegates to `set_task_completed(id, true, now)`; other statuses clear `completed_at`. `set_task_completed` writes `status` in both branches (recurring advance → `todo`).
- [ ] **Step 4: Run** `cargo test -p taskcraft-store` — Expected: all pass.
- [ ] **Step 5: Commit** `feat(store): task status with done/completed_at invariant`.

### Task 3: Labels in the store

**Files:**
- Create: `crates/taskcraft-store/src/labels.rs` (register in `lib.rs`)
- Modify: `model.rs` (`TaskLabel`), `tasks.rs` (write/read links), `query.rs` (`list_tasks` fills labels; `search` matches label names)
- Test: `crates/taskcraft-store/tests/labels.rs`

**Interfaces:**
- Produces:
  - `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct TaskLabel { pub id: i64, pub name: String, pub color: String }`
  - `Store::list_labels(&self) -> Result<Vec<TaskLabel>>` (by name, case-insensitive).
  - `Store::update_label(&self, id: i64, name: &str, color: &str) -> Result<()>` — empty/over-32-char name → `StoreError::Invalid`; existing other label with same name (NOCASE) → merge into it.
  - `Store::delete_label(&self, id: i64) -> Result<()>` (`NotFound` if absent).
  - `pub(crate) fn normalize_labels(names: &[String]) -> Vec<String>` — trim, strip one leading `#`, drop empty/over-32, dedupe case-insensitively keeping first spelling.

- [ ] **Step 1: Write failing tests** in `tests/labels.rs`:
  - `labels_created_and_reused_case_insensitively`: task A with `["home", "#Urgent"]`, task B with `["HOME"]` → `list_labels()` names `["home", "Urgent"]`; both tasks' labels contain the same `home` id; colors are the first two palette entries.
  - `update_replaces_links`: update A with `["work"]` → A labels `["work"]`; `home` still exists (used by B).
  - `rename_and_merge` (Review Focus 2): labels `home`, `HOUSE`; A has both; `update_label(house_id, "Home", c)` → one label `home`, A has it once, no error.
  - `rename_rejects_empty`: `update_label(id, "  ", c)` → `Err(Invalid)`.
  - `delete_label_unlinks`: delete `home` → B labels empty, task still exists.
  - `list_and_get_return_labels`: `list_tasks(All)` and `get_task` include labels sorted by name.
  - `search_matches_label`: `search("urgent")` returns task A.
  - `deleting_task_removes_links`: `delete_task(A)` → `SELECT count(*) FROM task_labels WHERE task_id=A` is 0 (via `foreign_keys=ON`).
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test labels` — Expected: FAIL (missing functions).
- [ ] **Step 3: Implement.** In the insert/update transaction: `normalize_labels`, `INSERT OR IGNORE INTO labels(name,color)` with palette color by `SELECT count(*) FROM labels`, `DELETE FROM task_labels WHERE task_id=?`, re-insert links. Read labels with one query per task (`get_task`) and one grouped query for `list_tasks` (`SELECT tl.task_id, l.id, l.name, l.color ... ORDER BY l.name COLLATE NOCASE`). Merge: `INSERT OR IGNORE INTO task_labels SELECT task_id, :keep FROM task_labels WHERE label_id=:old`, then delete `:old`.
- [ ] **Step 4: Run** `cargo test -p taskcraft-store && cargo clippy -p taskcraft-store --all-targets` — Expected: pass, 0 warnings.
- [ ] **Step 5: Commit** `feat(store): task labels with create, rename/merge, delete, search`.

### Task 4: `#tag` parsing

**Files:**
- Modify: `crates/taskcraft-parse/src/types.rs` (`Label::Tag`, `ParsedItem.tags`), `crates/taskcraft-parse/src/lib.rs` (tag pass), `crates/taskcraft-store/src/time.rs` (`from_parsed` copies tags into `TaskInput.labels`)
- Test: `crates/taskcraft-parse/tests/tags.rs`, add one case to `crates/taskcraft-store/tests/crud.rs`

**Interfaces:**
- Produces: `Label::Tag` (serde `tag`), `ParsedItem { .., pub tags: Vec<String> }`.

- [ ] **Step 1: Write failing tests** in `tests/tags.rs` using `parse_task` (and `parse` for events), `now()` = 2026-10-20 10:00 Rome:
  - `tags_removed_from_title`: `"buy milk #home #Errands friday"` → title `"buy milk"`, tags `["home","Errands"]`, due Friday, spans contain two `Label::Tag` spans covering `#home` and `#Errands` byte ranges.
  - `trailing_punct_and_dupes`: `"call #home, #HOME"` → title `"call"`, tags `["home"]`.
  - `not_tags` (Review Focus 5): `"C# meetup"` → tags `[]`, title `"C# meetup"`; `"#1 priority"` → tags `[]`; `"## notes"` → tags `[]`.
  - `events_keep_hash`: `parse("Party #fun saturday")` → kind Event, tags `[]`, title contains `"#fun"`.
  - `prefix_task_tags`: `parse("todo: pay rent #bills")` → tags `["bills"]`.
  - In `crud.rs`: `from_parsed_copies_tags` → `NewItem::Task(t)` with `t.labels == ["bills"]`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-parse --test tags` — Expected: FAIL (no `tags` field).
- [ ] **Step 3: Implement.** In `parse_kind`, when `kind == Task`, before `run_rules`, mark tokens whose text matches `^#[\p{L}\p{N}_-]{1,32}$` with at least one letter as `Label::Tag` (Payload::None) and collect tag names (without `#`, case-insensitive dedupe). `assemble_title` already skips labeled tokens. Add `--label-tag` CSS var mapping later (Task 7).
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: all pass (table tests unchanged).
- [ ] **Step 5: Commit** `feat(parse): #tags on tasks become labels`.

### Task 5: Commands and TS bindings

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/main.rs` (register), `ui/src/lib/types.ts`, `ui/src/lib/api.ts`, `ui/src/dev/mock.ts`

**Interfaces:**
- Consumes: Tasks 2–3 store API.
- Produces (Rust, each emits `items-changed`):
  - `pub fn update_label_impl(st: &AppState, id: i64, name: &str, color: &str) -> CmdResult<()>` — rejects a color that is not `#rrggbb` with `CmdError` code `invalid`; otherwise delegates to the store.
  - `set_task_status(id: i64, status: TaskStatus) -> CmdResult<()>`
  - `list_labels() -> CmdResult<Vec<TaskLabel>>`
  - `update_label(id: i64, name: String, color: String) -> CmdResult<()>`
  - `delete_label(id: i64) -> CmdResult<()>`
- Produces (TS):
  - `type TaskStatus = 'todo' | 'inProgress' | 'done'`; `interface TaskLabel { id: number; name: string; color: string }`
  - `TaskInput` gains `status: TaskStatus; labels: string[]`; `Task` gains `status: TaskStatus; labels: TaskLabel[]`; `ParsedItem` gains `tags: string[]`; `Label` union gains `'tag'`.
  - `api.setTaskStatus(id, status)`, `api.listLabels()`, `api.updateLabel(id, name, color)`, `api.deleteLabel(id)`.
  - Mock: tasks carry statuses (one `inProgress`) and labels (`home`, `work`, `errands`); handlers for the four commands mutate mock state.

- [ ] **Step 1: Write failing tests** in `commands.rs` tests: `update_label_validates` — `update_label_impl(&st, id, "home", "red")` → `Err` with code `"invalid"`; `update_label_impl(&st, id, "", "#123456")` → `Err` code `"invalid"`; valid input → `Ok` and `list_labels` shows it. Also add the integration test `as_task_tags_create_labels` (`create_from_text_impl(&st, "pay rent #bills friday", now(), true)` → task labels `["bills"]`); it is expected to pass already after Task 4 and is not the RED test.
- [ ] **Step 2: Run** `cargo test -p taskcraft-app update_label_validates` — Expected: FAIL (no `update_label_impl`).
- [ ] **Step 3: Implement** the four commands and TS changes.
- [ ] **Step 4: Run** `cargo test --workspace && cargo clippy --workspace --all-targets && npm --prefix ui run check && npm --prefix ui run test` — Expected: all green, 0 warnings, 0 errors.
- [ ] **Step 5: Commit** `feat(app): status and label commands, TS types and mock`.

### Task 6: Pure UI logic

**Files:**
- Modify: `ui/src/lib/tasks.ts`, `ui/src/lib/tasks.test.ts`, `ui/src/lib/edit.ts` (+ test in `ui/src/lib/edit.test.ts`, create if absent)

**Interfaces:**
- Produces:
  - `type StatusFilter = 'any' | 'todo' | 'inProgress'`
  - `applyFilters<T extends TaskLike>(tasks: T[], f: { status: StatusFilter; labels: number[] }): T[]` — `labels` empty = no label filter; otherwise any-of by label id.
  - `boardColumns<T extends TaskLike>(tasks: T[], opts: { showAllDone: boolean }): { status: TaskStatus; tasks: T[]; hidden: number }[]` — always 3 columns in order todo, inProgress, done; open columns sorted by due (none last) then id; done sorted by `completedAt` desc, first 30 unless `showAllDone`, `hidden` = count cut.
  - `nextStatus(s: TaskStatus): TaskStatus` — todo→inProgress→done→todo.
  - `TaskLike` gains `status` and `labels: { id: number }[]`.
  - `TaskForm` gains `status: TaskStatus; labels: string[]`; `taskToForm`/`formToTask` round-trip them.

- [ ] **Step 1: Write failing tests**: `applyFilters` (status inProgress only; labels any-of [home, work]; both combined); `boardColumns` (ordering; 35 done → 30 shown, hidden 5; showAllDone → 35, hidden 0); `nextStatus` cycle; `formToTask(taskToForm(input))` preserves `status` and `labels` (Review Focus 3).
- [ ] **Step 2: Run** `npm --prefix ui run test` — Expected: FAIL (missing exports).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `npm --prefix ui run test && npm --prefix ui run check` — Expected: all pass, 0 errors.
- [ ] **Step 5: Commit** `feat(ui): status/label filters, board columns, form round-trip`.

### Task 7: Tasks page redesign — List, Board, filters, label manager

Load `/frontend-design` (Skill `frontend-design`) before writing markup/CSS.

**Files:**
- Modify: `ui/src/main/TasksView.svelte`, `ui/src/main/Main.svelte` (pass labels), `ui/src/main/Inspector.svelte` (status segmented control + label chips with autocomplete), `ui/src/quick/HighlightInput.svelte` (`tag: '--label-tag'`), `ui/src/styles.css` (`--label-tag` light/dark)
- Create: `ui/src/main/tasks/TaskRow.svelte`, `ui/src/main/tasks/TaskBoard.svelte`, `ui/src/main/tasks/StatusPill.svelte`, `ui/src/main/tasks/LabelChip.svelte`, `ui/src/main/tasks/LabelManager.svelte`

**Interfaces:**
- Consumes: Task 5 API, Task 6 helpers.
- Produces: List/Board switch persisted in `localStorage['taskcraft.tasks.layout']` (wrapped in try/catch); clicking a chip toggles it in the label filter; board drag uses HTML5 DnD (`dataTransfer` task id) and keyboard `←`/`→` on a focused card calls `api.setTaskStatus`.

- [ ] **Step 1:** Build components per spec §UI and the visual direction (including: preview line under the add field lists parsed `#labels`; the Completed date filter is hidden in Board mode; label names over 32 chars or empty show the error banner); status colors: To do `var(--muted)`, In progress amber `#f59e0b` dot with pulse, Done green `#10b981`; motion wrapped in `@media (prefers-reduced-motion: no-preference)`.
- [ ] **Step 2: Run** `npm --prefix ui run check && npm --prefix ui run test` — Expected: 0 errors, all pass.
- [ ] **Step 3: Visual check** with Vite on `:5199` and the dev mock in Chrome: List and Board, light and dark (`document.documentElement.dataset.theme`), drag a card To do → In progress, open Manage labels, rename a label, filter by two labels, no console errors. Stop Vite, close the tab.
- [ ] **Step 4: Commit** `feat(ui): redesigned Tasks page with board, status and labels`.

### Task 8: Docs, install, PR

**Files:**
- Modify: `README.md` (Tasks page: `#labels`, statuses, Board)

- [ ] **Step 1:** Update README.
- [ ] **Step 2: Run** `cargo test --workspace && npm --prefix ui run test && ./scripts/install.sh && ./scripts/smoke.sh` — Expected: green; "All smoke checks passed."
- [ ] **Step 3: Commit** `docs: labels and status`; push `feat/task-labels-status`; open PR against `feat/tasks-page`.
