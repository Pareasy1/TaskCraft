# Planners, buckets and ordering — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tasks live in user-created plans with ordered buckets; the Board groups by bucket (or status/due/label) with drag ordering; quick entry routes with `/plan`.

**Architecture:** Schema V3 adds `plans`, `buckets`, `prefs` and three task columns; all invariants (no orphans, last plan/bucket, bucket∈plan, default plan) live in `taskcraft-store`. The parser recognises `/plan` against the known plan names (like it does for calendars). Tauri exposes thin commands; the UI adds a Planners sidebar section, a plan-scoped Tasks view with a bucket board, and pure helpers in `ui/src/lib/plans.ts`.

**Tech Stack:** Rust (rusqlite, chrono), Tauri 2, Svelte 5 runes, vitest.

**Spec:** `docs/superpowers/specs/2026-10-22-planners-buckets-design.md`

## Global Constraints

- Branch `feat/planners`. Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Every user-visible string goes through `ui/src/lib/i18n.ts` with EN and IT.
- Colours come from existing tokens; `tokens.test.ts` and `style-rules.test.ts` stay green.
- Drag-and-drop is never the only way: every drag has a menu/keyboard equivalent.
- No new runtime dependencies.
- Seed names: plan `My tasks` / `Le mie attività`, bucket `To do` / `Da fare`, plan colour `#3b82f6`.
- `sort_order` is REAL, spaced 1.0; renumber a bucket when a midpoint gap < 1e-6.
- Calendar behaviour (colours, visibility on Day/Week/Month) is unchanged: tasks keep `calendar_id`.
- Env for Rust: `export PATH="$HOME/.cargo/bin:$PATH"`. UI tests: `cd ui && npx vitest run`.

**Design detail beyond the spec (ruled while planning):**
- Plan-name matching lives in the parser (`ParseOptions.plans`), exactly like calendar names do,
  so the quick-entry preview can highlight the `/plan` token. Behaviour is what the spec asks for:
  unknown `/word` stays in the title.
- The default plan is stored in the DB (`prefs` table), because the store must move it when that plan is deleted.
- The store does not know the UI language when it migrates. It seeds English names and sets
  `prefs.seed_names_pending = 1`. The first `set_ui_language` call renames the seed plan/bucket
  (only if still named `My tasks` / `To do`) and clears the flag.

## Review Focus

1. A task whose `bucket_id` belongs to a different plan than its `plan_id` (e.g. update_task changing plan only) — the store must reject or re-home it, never save it inconsistent. → Task 3 test `update_plan_only_moves_to_first_bucket`.
2. Deleting the plan that is the default while quick entry is in use — the next quick entry must land in the new default, not fail. → Task 2 `delete_default_plan_reassigns_default` + Task 5 `quick_entry_after_default_deleted`.
3. Repeated drops in the same gap (dozens of moves between the same two cards) — order must stay correct after renumbering. → Task 3 `many_midpoint_moves_renumber`.
4. `/` tokens that aren't plans: `1/2`, `w/ Sam`, `https://x.y/a`, a lone `/` — must stay in the title / be parsed as before. → Task 4 `slash_non_plans_untouched`.
5. Renaming a plan to an existing name with different case (`casa` vs `Casa`) — rejected with a localized "already exists" toast, nothing changes. → Task 2 `duplicate_plan_name_case_insensitive` + Task 6 error-code test.

---

### Task 1: Schema V3, migration and model types

**Files:**
- Modify: `crates/taskcraft-store/src/schema.rs` (V3 const, `CURRENT_VERSION = 3`)
- Modify: `crates/taskcraft-store/src/model.rs` (types, errors, Task/TaskInput fields)
- Modify: `crates/taskcraft-store/src/tasks.rs` (`TASK_COLS`, `task_from_row`)
- Modify: `src-tauri/src/commands.rs` (`From<StoreError>` codes), `scripts/smoke.sh` (expects `3`)
- Test: `crates/taskcraft-store/tests/migrate_v3.rs`

**Interfaces:**
- Produces:
  - `pub struct Plan { id: i64, name: String, color: String, sort: i64, open_count: i64 }` (serde camelCase)
  - `pub struct Bucket { id: i64, plan_id: i64, name: String, sort: i64 }`
  - `pub enum DeleteMode { MoveTo(i64), DeleteTasks }` (serde: `{"moveTo": id}` / `"deleteTasks"`, `rename_all = "camelCase"`)
  - `StoreError::{Duplicate, LastPlan, LastBucket}` (messages "already exists", "cannot delete the last plan", "cannot delete the last bucket")
  - `Task` gains `plan_id: i64, bucket_id: i64, sort_order: f64` (drop `Eq` derive on `Task` if needed because of f64; keep `PartialEq`)
  - `TaskInput` gains `#[serde(default)] plan_id: Option<i64>, #[serde(default)] bucket_id: Option<i64>`

V3 SQL (append to `migrate` as `if v < 3`):
```sql
CREATE TABLE plans(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, color TEXT NOT NULL, sort INTEGER NOT NULL);
CREATE TABLE buckets(id INTEGER PRIMARY KEY, plan_id INTEGER NOT NULL REFERENCES plans(id) ON DELETE CASCADE, name TEXT NOT NULL, sort INTEGER NOT NULL);
CREATE TABLE prefs(key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT INTO plans(id, name, color, sort) VALUES (1, 'My tasks', '#3b82f6', 0);
INSERT INTO buckets(id, plan_id, name, sort) VALUES (1, 1, 'To do', 0);
INSERT INTO prefs VALUES ('default_plan_id', '1'), ('seed_names_pending', '1');
ALTER TABLE tasks ADD COLUMN plan_id INTEGER NOT NULL DEFAULT 1 REFERENCES plans(id);
ALTER TABLE tasks ADD COLUMN bucket_id INTEGER NOT NULL DEFAULT 1 REFERENCES buckets(id);
ALTER TABLE tasks ADD COLUMN sort_order REAL NOT NULL DEFAULT 0;
CREATE INDEX tasks_bucket ON tasks(bucket_id, sort_order);
```
Then, in Rust inside the same transaction: number existing tasks 1.0, 2.0, … ordered by
`(completed_at IS NOT NULL, due_utc IS NULL, due_utc, id)`.

- [ ] **Step 1: Write failing tests** in `tests/migrate_v3.rs` (build a V2 DB like `migrate_v2.rs` does, with 3 tasks: due 2026-10-25, due 2026-10-21, no due):
  - `v2_tasks_land_in_seed_plan_in_list_order`: after `Store::open`, `schema_version() == 3`; every task has `plan_id == 1, bucket_id == 1`; sorting by `sort_order` gives titles `["due21", "due25", "nodue"]`; sort orders are `[1.0, 2.0, 3.0]`.
  - `fresh_db_has_seed_plan_and_bucket`: `open_in_memory()` → `list_plans()` is one plan `My tasks`, `list_buckets(1)` is one bucket `To do`, `default_plan_id() == 1`.
  - `migration_writes_backup`: `<db>.bak-v2` exists after opening.
  - `real_db_copy` (`#[ignore]`): reads the path from env `TASKCRAFT_MIGRATE_CHECK`, counts `tasks` with plain rusqlite, opens it with `Store::open`, asserts the same count, `schema_version() == 3`, and every task has a bucket of its own plan. Used in Task 9 on a copy of the user's DB.
  (`list_plans`, `list_buckets`, `default_plan_id` are stubs added in Task 2; for this task add them as minimal reads in `crates/taskcraft-store/src/plans.rs`.)
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test migrate_v3` → FAIL (no plans table / methods).
- [ ] **Step 3: Implement** V3, model types, `TASK_COLS` += `t.plan_id, t.bucket_id, t.sort_order`, minimal `list_plans() -> Result<Vec<Plan>>` (ordered by `sort`, `open_count` = tasks with `completed_at IS NULL`), `list_buckets(plan_id) -> Result<Vec<Bucket>>`, `default_plan_id() -> Result<i64>`. Map new errors in `commands.rs`: `Duplicate→"duplicate"`, `LastPlan→"last_plan"`, `LastBucket→"last_bucket"`. Update `scripts/smoke.sh` schema check to `3`.
- [ ] **Step 4: Run** `cargo test --workspace` → all PASS (existing tests still pass; fix struct literals of `TaskInput` in tests with `..Default::default()` where they break).
- [ ] **Step 5: Commit** `feat(store): schema v3 with plans, buckets and task ordering`

### Task 2: Plan and bucket management in the store

**Files:**
- Modify: `crates/taskcraft-store/src/plans.rs`, `lib.rs` (`mod plans`)
- Test: `crates/taskcraft-store/tests/plans.rs`

**Interfaces:**
- Consumes: Task 1 types.
- Produces (all on `Store`):
  - `create_plan(&self, name: &str, color: &str) -> Result<i64>` — trims; empty → `Invalid`; duplicate (NOCASE) → `Duplicate`; appended last; creates one bucket named `bucket_name` = the current seed bucket name stored in `prefs.new_bucket_name` (defaults `To do`; set by `localize_seed`).
  - `update_plan(&self, id: i64, name: Option<&str>, color: Option<&str>) -> Result<()>`
  - `reorder_plans(&self, ids: &[i64]) -> Result<()>` — ids must be exactly the set of plans, else `Invalid`.
  - `delete_plan(&self, id: i64, mode: DeleteMode) -> Result<()>` — last plan → `LastPlan`; `MoveTo(p)` moves tasks to plan `p`'s first bucket appended at end (p == id → `Invalid`); `DeleteTasks` deletes tasks via the same path as `delete_task` (alerts and labels cleaned). If it was the default, default becomes the first remaining plan by `sort`.
  - `create_bucket(&self, plan_id: i64, name: &str) -> Result<i64>` (appended last), `rename_bucket(&self, id: i64, name: &str) -> Result<()>`, `reorder_buckets(&self, plan_id: i64, ids: &[i64]) -> Result<()>`
  - `delete_bucket(&self, id: i64, mode: DeleteMode) -> Result<()>` — last bucket of its plan → `LastBucket`; `MoveTo(b)` requires `b` in the same plan (else `Invalid`), tasks appended at end of `b`.
  - `set_default_plan(&self, id: i64) -> Result<()>` (NotFound if missing)
  - `localize_seed(&self, plan_name: &str, bucket_name: &str) -> Result<()>` — if `seed_names_pending`: rename plan 1 if still `My tasks`, bucket 1 if still `To do`; store `new_bucket_name = bucket_name`; clear the flag. Idempotent.

- [ ] **Step 1: Write failing tests** in `tests/plans.rs`:
  - `create_plan_has_one_bucket_and_goes_last`
  - `duplicate_plan_name_case_insensitive`: create `Casa`; `create_plan("casa", …)` → `Err(Duplicate)`; `update_plan(other, Some("CASA"), None)` → `Err(Duplicate)`.
  - `empty_plan_name_rejected`: `"  "` → `Err(Invalid(_))`.
  - `cannot_delete_last_plan` → `Err(LastPlan)`; `cannot_delete_last_bucket` → `Err(LastBucket)`.
  - `delete_plan_move_to_keeps_tasks`: plan B with 2 tasks, target A already has 1 → after `MoveTo(A)` A's first bucket has 3 tasks, the moved ones after the existing one, in their previous relative order.
  - `delete_plan_delete_tasks_removes_tasks_and_alerts`: task with alert and label → task gone, `alerts` rows for it gone, label itself remains.
  - `delete_default_plan_reassigns_default`: default = B; delete B → `default_plan_id()` is the first remaining plan by sort.
  - `delete_bucket_move_to_other_plan_rejected` → `Err(Invalid(_))`.
  - `reorder_plans_and_buckets`: reorder and read back; wrong id set → `Err(Invalid(_))`.
  - `localize_seed_renames_once`: `localize_seed("Le mie attività","Da fare")` renames; second call with other names does nothing; a user-renamed seed plan is not renamed (set name `Lavoro` before calling → still `Lavoro`); new plans get bucket `Da fare`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test plans` → FAIL.
- [ ] **Step 3: Implement** in `plans.rs`; every mutating method uses `self.in_tx`. Map SQLite UNIQUE violation on `plans.name` to `Duplicate` (check before insert/update with a NOCASE query; don't rely on parsing error text).
- [ ] **Step 4: Run** `cargo test -p taskcraft-store` → PASS.
- [ ] **Step 5: Commit** `feat(store): create, rename, reorder and delete plans and buckets`

### Task 3: Task placement and ordering

**Files:**
- Modify: `crates/taskcraft-store/src/tasks.rs`, `plans.rs`
- Test: `crates/taskcraft-store/tests/ordering.rs`

**Interfaces:**
- Consumes: Task 1–2.
- Produces:
  - `insert_task`: resolves placement with `fn placement(tx, plan_id: Option<i64>, bucket_id: Option<i64>) -> Result<(i64, i64)>`:
    bucket given → its plan (if `plan_id` also given and differs → `Invalid`); only plan → its first bucket by sort; neither → default plan's first bucket. New task `sort_order` = max in bucket + 1.0.
  - `update_task`: `None`/`None` keeps current placement; a change of plan without bucket → first bucket of the new plan, appended; same rules as insert otherwise.
  - `move_task(&self, task_id: i64, bucket_id: i64, before: Option<i64>, after: Option<i64>) -> Result<()>` — `before`/`after` are the ids of the neighbour tasks the card is dropped between (above / below), both in `bucket_id` (else `Invalid`). Sets `plan_id` from the bucket. Order: both → midpoint; only `after` → `after.sort - 1.0`; only `before` → `before.sort + 1.0`; neither → max + 1.0. If the resulting gap to a neighbour < 1e-6, renumber the bucket 1.0… (keeping the intended order) and recompute.
  - `list_tasks` unchanged in filtering; still returns `plan_id/bucket_id/sort_order`.

- [ ] **Step 1: Write failing tests** in `tests/ordering.rs`:
  - `insert_defaults_to_default_plan_end`: two inserts with no placement → both in default plan's first bucket, sort 1.0 and 2.0.
  - `insert_with_bucket_sets_plan`: bucket of plan B → `plan_id == B`.
  - `insert_mismatched_plan_bucket_rejected` → `Err(Invalid(_))`.
  - `update_plan_only_moves_to_first_bucket`: task in A/bucket2; update with `plan_id = Some(B), bucket_id = None` → in B's first bucket, last.
  - `update_without_placement_keeps_it`.
  - `move_between_two_cards`: bucket [a,b,c]; move c between a and b → order [a,c,b].
  - `move_to_top_and_bottom`: before=None after=Some(first) → top; neither → bottom of target bucket.
  - `move_across_plans_updates_plan`.
  - `move_neighbour_in_other_bucket_rejected` → `Err(Invalid(_))`.
  - `many_midpoint_moves_renumber`: bucket [a,b,x1..x60]; 60 times move the last card between a and the card right after a → final order is a, x60, x59, …, x1, b (each new card directly after a), and all sort_orders are distinct.
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test ordering` → FAIL.
- [ ] **Step 3: Implement.** Renumbering: read the bucket ordered by `(sort_order, id)`, place the moved task at its target index, write 1.0, 2.0, … in one transaction.
- [ ] **Step 4: Run** `cargo test -p taskcraft-store` → PASS.
- [ ] **Step 5: Commit** `feat(store): place tasks in plans and buckets, move with stable ordering`

### Task 4: `/plan` in the parser

**Files:**
- Modify: `crates/taskcraft-parse/src/types.rs` (`Label::Plan`, `ParseOptions.plans: Vec<String>` default `vec![]`, `ParsedItem.plan: Option<String>`), `lib.rs`
- Test: `crates/taskcraft-parse/tests/plan_hint.rs`

**Interfaces:**
- Produces:
  - `pub fn plan_key(name: &str) -> String` — lowercase, keep only alphanumeric chars (`"3D print"` → `"3dprint"`, `"Le mie attività"` → `"lemieattività"`).
  - On tasks only (like `#tags`): a token whose text is `/` + word with ≥1 letter, whose `plan_key` equals `plan_key` of one entry in `opts.plans`, gets `Label::Plan` and `ParsedItem.plan = Some(<that plan's exact name>)`. First match wins; later `/plan` tokens stay in the title. Non-matching tokens are untouched.

- [ ] **Step 1: Write failing tests** (opts with `plans: vec!["3D print".into(), "Casa".into()]`):
  - `slash_plan_matches_ignoring_spaces_and_case`: `parse_task("ship Rafael friday /3DPRINT #urgent")` → `plan == Some("3D print")`, title `"ship Rafael"`, tags `["urgent"]`, one span with `Label::Plan` covering `/3DPRINT`.
  - `unknown_slash_stays_in_title`: `"call /mom"` → `plan == None`, title `"call /mom"`.
  - `slash_non_plans_untouched`: `"pay 1/2 rent"`, `"lunch w/ Sam"`, `"read https://x.y/casa"`, `"a / b"` → `plan == None` and titles equal to today's parse with `plans: vec![]`.
  - `events_ignore_slash_plan`: `parse("dinner /casa tomorrow")` (event) → `plan == None`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-parse --test plan_hint` → FAIL.
- [ ] **Step 3: Implement** `mark_plan(toks, tags, opts) -> Option<String>` next to `mark_tags`, called in the same `if kind == ItemKind::Task` branch. Add `plan: Label` mapping to the UI later (Task 6).
- [ ] **Step 4: Run** `cargo test -p taskcraft-parse` → PASS.
- [ ] **Step 5: Commit** `feat(parse): route quick-entry tasks with /plan`

### Task 5: Tauri commands

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/main.rs` (register), `crates/taskcraft-store/src/time.rs` (`from_parsed` carries plan), `src-tauri/src/i18n.rs` (seed names)
- Test: `src-tauri/src/commands.rs` `mod tests`

**Interfaces:**
- Consumes: Tasks 2–4.
- Produces commands (args camelCase from JS):
  `list_plans() -> Vec<Plan>`, `create_plan(name, color) -> i64`, `update_plan(id, name: Option<String>, color: Option<String>)`,
  `reorder_plans(ids)`, `delete_plan(id, mode: DeleteMode)`, `list_buckets(planId) -> Vec<Bucket>`,
  `create_bucket(planId, name) -> i64`, `rename_bucket(id, name)`, `reorder_buckets(planId, ids)`,
  `delete_bucket(id, mode)`, `move_task(taskId, bucketId, before, after)`,
  `get_default_plan() -> i64`, `set_default_plan(id)`. Every mutating command calls `changed(&app)`. Colours validated with existing `is_hex_color` → `invalid`.
- `parse_options` fills `plans` from `list_plans()` names.
- `from_parsed(…, plans: &[Plan])`: `TaskInput.plan_id` = id of the plan named `p.plan` (exact), else `None` (store falls back to default).
- `set_ui_language` also calls `store.localize_seed(text.seed_plan, text.seed_bucket)`; add to `Text`: EN `seed_plan: "My tasks", seed_bucket: "To do"`, IT `"Le mie attività"`, `"Da fare"`.

- [ ] **Step 1: Write failing tests** in `commands.rs` tests (use `AppState::in_memory()` and `create_from_text_impl`):
  - `quick_entry_routes_slash_plan`: create plan `3D print`; `create_from_text_impl(&st, "ship Rafael /3dprint", now, true)` → task's `plan_id` is that plan, title `ship Rafael`.
  - `quick_entry_without_plan_uses_default`: set default to plan B → new task in B.
  - `quick_entry_after_default_deleted`: default B, delete B with `MoveTo(A)` → quick entry lands in A.
  - `store_errors_have_codes`: `CmdError::from(StoreError::Duplicate).code == "duplicate"`, same for `last_plan`, `last_bucket`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-app commands::tests` → FAIL.
- [ ] **Step 3: Implement** commands and registration.
- [ ] **Step 4: Run** `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** `feat(app): commands for plans, buckets and moving tasks`

### Task 6: UI data layer: types, API, i18n, pure helpers, mock

**Files:**
- Modify: `ui/src/lib/types.ts`, `ui/src/lib/api.ts`, `ui/src/lib/i18n.ts`, `ui/src/dev/mock.ts`, `ui/src/quick/HighlightInput.svelte` (`plan: '--label-calendar'` colour), `ui/src/styles.css` only if a token is needed
- Create: `ui/src/lib/plans.ts`
- Test: `ui/src/lib/plans.test.ts`, extend `ui/src/lib/api` error test if one exists (else in `plans.test.ts`)

**Interfaces:**
- Produces:
  - types: `Plan { id; name; color; sort; openCount }`, `Bucket { id; planId; name; sort }`, `DeleteMode = { moveTo: number } | 'deleteTasks'`; `Task` += `planId, bucketId, sortOrder`; `TaskInput` += `planId?: number | null, bucketId?: number | null`; `Label` += `'plan'`; `ParsedItem` += `plan: string | null`.
  - `api.listPlans, createPlan(name, color), updatePlan(id, {name?, color?}), reorderPlans(ids), deletePlan(id, mode), listBuckets(planId), createBucket(planId, name), renameBucket(id, name), reorderBuckets(planId, ids), deleteBucket(id, mode), moveTask(taskId, bucketId, before, after), getDefaultPlan(), setDefaultPlan(id)`.
  - `TRANSLATED_ERRORS` += `'duplicate', 'last_plan', 'last_bucket'`.
  - `plans.ts`:
    - `export type GroupBy = 'bucket' | 'status' | 'due' | 'label'`
    - `bucketColumns<T extends {bucketId:number; sortOrder:number; id:number}>(buckets: Bucket[], tasks: T[]): { bucket: Bucket; tasks: T[] }[]` — buckets in `sort` order, tasks by `(sortOrder, id)`, empty buckets kept.
    - `dropNeighbours(ids: number[], movingId: number, index: number): { before: number | null; after: number | null }` — `ids` is the target column's current order (may contain `movingId`); `index` is the insertion index in that list *without* the moving card. `before` = card above, `after` = card below.
    - `dueColumns<T>(tasks: T[], now: Date)` → keys `overdue | today | thisWeek | later | nodate` (open tasks only) and `labelColumns<T>(tasks: T[], labels: TaskLabel[])` → one column per label (task appears under each of its labels) plus `nolabel`.
    - `nextPlanColor(plans: Plan[]): string` — first of the label palette (`#3b82f6, #f97316, #10b981, #a855f7, #ef4444, #eab308, #06b6d4, #ec4899`) not in use, else cycles by count.
  - i18n keys (EN / IT): `plans.title` Planners/Planner · `plans.all` All tasks/Tutte le attività · `plans.new` New plan/Nuovo piano · `plans.rename` Rename/Rinomina · `plans.color` Colour/Colore · `plans.delete` Delete plan…/Elimina piano… · `plans.moveUp`/`plans.moveDown` Move up/Sposta su, Move down/Sposta giù · `buckets.add` Add bucket/Aggiungi bucket · `buckets.new` New bucket/Nuovo bucket · `buckets.delete` Delete bucket…/Elimina bucket… · `buckets.moveLeft`/`buckets.moveRight` Move left/Sposta a sinistra, Move right/Sposta a destra · `task.moveTo` Move to bucket/Sposta nel bucket · `task.moveUp`/`task.moveDown` · `groupBy.label` Group by/Raggruppa per · `groupBy.bucket` Bucket · `groupBy.status` Status/Stato · `groupBy.due` Due date/Scadenza · `groupBy.label_` Label/Etichetta · `due.thisWeek` This week/Questa settimana · `due.later` Later/Più avanti · `label.none` No label/Senza etichetta · `del.planHasTasks` "This plan has {n} tasks."/"Questo piano ha {n} attività." (+ `One` variants) · `del.bucketHasTasks` (same for bucket) · `del.moveTo` Move them to/Spostale in · `del.deleteToo` Delete them too/Eliminale · `del.confirmEmpty` "Delete “{name}”?"/"Eliminare “{name}”?" · `del.lastPlan` "You need at least one plan."/"Serve almeno un piano." · `del.lastBucket` "A plan needs at least one bucket."/"Un piano ha bisogno di almeno un bucket." · `details.plan` Plan/Piano · `details.bucket` Bucket · `settings.defaultPlan` Default plan for new tasks/Piano predefinito per le nuove attività · `error.duplicate` "That name is already used."/"Questo nome è già in uso." · `error.last_plan`, `error.last_bucket` (= del.lastPlan / del.lastBucket).
  - mock.ts: two plans (`Le mie attività` 3 buckets, `3D print` with buckets `Da stampare`, `Post-produzione`, `Da spedire`), tasks distributed, all new commands implemented in memory (move with simple index splice).

- [ ] **Step 1: Write failing tests** in `plans.test.ts`:
  - `bucketColumns orders buckets and cards, keeps empty buckets`.
  - `dropNeighbours`: ids `[1,2,3]`, moving 3 to index 0 → `{before:null, after:1}`; moving 1 to index 1 (list without 1 is [2,3]) → `{before:2, after:3}`; moving 9 (from another column) to index 3 → `{before:3, after:null}`; empty column → both null.
  - `dueColumns` with `now = 2026-10-22T10:00` (Thursday): overdue / today / thisWeek (≤ Sunday 25th) / later / nodate assignment; completed tasks excluded.
  - `labelColumns`: task with two labels appears in both; unlabeled → `nolabel`.
  - `nextPlanColor` skips used colours.
  - `errorMessage({code:'duplicate'})` returns the translated string.
- [ ] **Step 2: Run** `cd ui && npx vitest run src/lib/plans.test.ts` → FAIL.
- [ ] **Step 3: Implement** types, api, i18n, `plans.ts`, mock, highlight colour.
- [ ] **Step 4: Run** `cd ui && npx vitest run && npx svelte-check` → PASS, 0 errors.
- [ ] **Step 5: Commit** `feat(ui): plan and bucket data layer, helpers and strings`

### Task 7: Sidebar Planners section, plan scope, delete dialog

**Files:**
- Modify: `ui/src/main/Main.svelte` (load plans; `scope = $state<number | 'all'>`; clicking a plan sets `view='tasks'` + scope; Toolbar title shows plan name in plan scope), `ui/src/main/Sidebar.svelte`
- Create: `ui/src/main/plans/PlanList.svelte`, `ui/src/main/plans/DeleteDialog.svelte`
- Test: `ui/src/lib/plans.test.ts` (logic only) + visual check

**Interfaces:**
- Consumes: Task 6 api/helpers.
- Produces:
  - `PlanList` props: `{ plans: Plan[]; scope: number | 'all'; allCount: number; onselect: (s: number | 'all') => void; onerror: (m: string) => void }`. Renders the section shown in the spec; `+` creates `t('plans.new')` (suffix ` 2`, ` 3`… when taken) with `nextPlanColor` and enters inline rename (Enter saves, Esc cancels, empty → revert). Row "…" button and `contextmenu` open a menu: Rename, Colour (the 8 palette swatches), Move up, Move down, Delete…. Rows are draggable to reorder (`reorderPlans`). Rows are buttons with `aria-current` on the active scope.
  - `DeleteDialog` props: `{ kind: 'plan' | 'bucket'; name: string; taskCount: number; targets: {id:number; name:string}[]; isLast: boolean; onconfirm: (mode: DeleteMode) => void; oncancel: () => void }` — uses the same `<dialog>` pattern as the Inspector; radio Move (default, select of `targets`) / Delete them too; empty → only the confirm sentence; `isLast` → Delete disabled with the `del.lastPlan`/`del.lastBucket` text shown.
  - Pure helper (add to `plans.ts` with a test in this task): `uniqueName(base: string, taken: string[]): string` — case-insensitive; `"New plan"` → `"New plan 2"`, `"New plan 3"`.
  - The sidebar's existing task sections (when not in tasks mode) stay unchanged; the Planners section sits below the calendars list; the labels section stays in tasks mode.
  - Also fix the clipped LabelManager popover seen in the user's screenshot (Sidebar `.mgr :global(.pop)` positioned inside `overflow:hidden`): open it as a fixed-position popover anchored to the button so it is not cut off. Verify visually.

- [ ] **Step 1: Write failing test** `uniqueName` cases above + `"new plan"` taken → `"New plan 2"`.
- [ ] **Step 2: Run** `npx vitest run src/lib/plans.test.ts` → FAIL.
- [ ] **Step 3: Implement** helper, components, Main/Sidebar wiring.
- [ ] **Step 4: Run** `npx vitest run && npx svelte-check` → PASS; then visual: `npx vite --port 5199`, Claude in Chrome: create/rename/recolour/reorder/delete (empty, with tasks → move, with tasks → delete, last plan disabled); `?lang=it`; dark mode; label popover not clipped.
- [ ] **Step 5: Commit** `feat(ui): planners in the sidebar with rename, colour, reorder and delete`

### Task 8: Plan view: Group by, bucket board, details and settings

**Files:**
- Modify: `ui/src/main/TasksView.svelte` (accept `scope`, `plans`, `buckets`; filter tasks by plan; Group-by menu on Board in plan scope, persisted per plan in localStorage key `taskcraft.groupBy.<planId>` with try/catch; New-task input passes `planId`), `ui/src/main/tasks/TaskBoard.svelte` (generic columns: accept `columns: {key, title, color?, tasks}[]` and callbacks), `ui/src/main/tasks/TaskRow.svelte` (plan dot in All tasks), `ui/src/main/Inspector.svelte` (Plan and Bucket selects), `ui/src/main/SettingsDialog.svelte` (Default plan select via `getDefaultPlan/setDefaultPlan`)
- Create: `ui/src/main/tasks/BucketBoard.svelte`
- Test: `ui/src/lib/plans.test.ts` + visual

**Interfaces:**
- Consumes: Task 6 helpers (`bucketColumns`, `dropNeighbours`, `dueColumns`, `labelColumns`), Task 7 `DeleteDialog`, `scope`.
- Produces:
  - `BucketBoard` props: `{ plan: Plan; buckets: Bucket[]; tasks: Task[]; onopen; onerror }`. Cards draggable within/between columns → `api.moveTask(id, bucketId, before, after)` using `dropNeighbours` with the drop index computed from the pointer's position over card midpoints. Column headers draggable → `reorderBuckets`. Header: double-click to rename inline; "…" menu Rename / Move left / Move right / Delete… (DeleteDialog with targets = other buckets). Last column "+ Add bucket" → `createBucket` with `uniqueName(t('buckets.new'))` in rename mode. Each column footer "+ New task" → inline title input → `createTask({ …, planId, bucketId })`, the same defaults the Tasks page uses today. Card menu ("…" button, also context menu): Move to bucket ▸ (list), Move up, Move down — all via `moveTask`. Status circle on each card works as today (`changeStatus`). Done cards stay in their bucket, drawn with the existing done style.
  - Group by Status: today's `TaskBoard` (drag changes status). Due / Label: `TaskBoard` with `dueColumns` / `labelColumns`, no drop target (dragging disabled); ordering inside columns by `sortOrder`.
  - All tasks scope: today's page unchanged plus the plan dot; no Group-by menu.
  - List inside a plan: today's grouped list filtered to the plan; the "No date" group is ordered by `sortOrder` instead of id.
  - Any failed `moveTask`/`reorderBuckets` shows the error toast and reloads tasks and buckets from the store, so the board snaps back.
  - Inspector: Plan select (all plans) and Bucket select (buckets of the chosen plan; changing plan resets bucket to that plan's first). Saving passes `planId/bucketId`.

- [ ] **Step 1: Write failing test** for the drop-index helper: `dropIndex(midpoints: number[], y: number): number` in `plans.ts` — midpoints `[10, 30, 50]`: y 5 → 0, y 20 → 1, y 60 → 3.
- [ ] **Step 2: Run** `npx vitest run src/lib/plans.test.ts` → FAIL.
- [ ] **Step 3: Implement** helper and components.
- [ ] **Step 4: Run** `npx vitest run && npx svelte-check` → PASS; visual (mock data, EN+IT, light+dark): drag a card between buckets and within a bucket; reorder buckets; add/rename/delete bucket; card menu moves; Group by switch persists per plan; New task in a column lands there; Inspector plan/bucket change; Settings default plan; keyboard-only path through the card menu.
- [ ] **Step 5: Commit** `feat(ui): bucket board with drag ordering, group-by and plan pickers`

### Task 9: Verify, docs and install

**Files:**
- Modify: `docs/INSTALL.md` (Using TaskCraft: one bullet on plans and `/plan`), `README.md` features list if it lists features

- [ ] **Step 1: Run** the full suites: `cargo test --workspace`, `cd ui && npx vitest run && npx svelte-check`, `cargo clippy --workspace -- -D warnings`. Expected: all green.
- [ ] **Step 2:** Copy the user's real DB (`~/Library/Application Support/TaskCraft/taskcraft.db`, plus `-wal`/`-shm` if present) into the scratchpad, then run `TASKCRAFT_MIGRATE_CHECK=<copy path> cargo test -p taskcraft-store --test migrate_v3 real_db_copy -- --ignored`. Expected: PASS. Never open the original.
- [ ] **Step 3:** With the user's OK, `./scripts/install.sh` then `./scripts/smoke.sh` (expects schema 3).
- [ ] **Step 4: Commit** `docs: planners in the install guide`
