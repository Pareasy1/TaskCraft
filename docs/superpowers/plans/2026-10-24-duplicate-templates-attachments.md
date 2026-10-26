# Duplicate, templates and attachments — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tasks can be duplicated with everything they contain, saved as reusable templates with relative subtask dates, and carry file/link attachments stored in TaskCraft's data folder.

**Architecture:** Three store modules (`attachments.rs`, `duplicate.rs`, `templates.rs`) on schema V5; file I/O stays in the store behind a `data_dir` parameter so it is testable with temp dirs; deletions queue files in `pending_file_deletes` and the scheduler purges them after 15 s so Undo can re-link them. The parser gains `+template`; Tauri adds commands, the open-file dialog permission and `tauri-plugin-opener`; the UI adds an Attachments section, Duplicate / Save as template / New from template, and a Templates list in Settings.

**Tech Stack:** Rust (rusqlite, chrono, serde_json), Tauri 2 (+ tauri-plugin-opener), Svelte 5 runes, vitest.

**Spec:** `docs/superpowers/specs/2026-10-24-duplicate-templates-attachments-design.md`

## Global Constraints

- Branch `feat/templates` (contains phases 1–2). Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Strings in `ui/src/lib/i18n.ts` EN+IT; tokens only; drags allowing only `move` set `dropEffect = 'move'`; every action reachable by menu/keyboard.
- Only new runtime dependency: `tauri-plugin-opener` (Rust crate `tauri-plugin-opener = "2"`, npm `@tauri-apps/plugin-opener` only if the UI calls it directly — prefer Rust commands). `serde_json` is already a workspace dependency.
- Attachment file limit 100 MiB (104_857_600 bytes); store path `<data_dir>/attachments/<task_id>/<n>-<sanitized>`; sanitize keeps letters, digits, `.`, `-`, `_`, space; others → `_`; name ≤ 120 chars; pending delete delay 15 s.
- Links: `http://` or `https://` only.
- Template names: trimmed, 1–60 chars, unique via the store's existing `name_key` (Unicode case-insensitive).
- Rule codes added: `attachment_too_large`, `attachment_url`, `template_name`.
- No SQL foreign keys on new task references; every task-delete path goes through `delete_task_row`.
- Verify the installed app (WebKit), not only Chrome, for UI tasks where possible.
- Env: `export PATH="$HOME/.cargo/bin:$PATH"`; UI checks `cd ui && npx vitest run && TZ=Europe/Rome npx vitest run && npx svelte-check --tsconfig ./tsconfig.app.json`.

## Review Focus

1. A file whose name repeats, has unicode, slashes, `..`, or is very long: stored safely inside `attachments/<task>/`, never overwriting another, never escaping the folder. → Task 1 `sanitize_and_unique_names`, `path_escape_refused`.
2. Deleting a task (or its plan/bucket with DeleteTasks) that has attachments on itself and its subtasks, then Undo within the toast: files come back; without Undo: files disappear after the delay. → Task 1 `cascade_queues_files`, `restore_relinks_queued_file`, `purge_after_delay`.
3. Creating from a template whose subtasks had offsets, for a due date on either side of a DST change, and for a task without a due. → Task 3 `offsets_across_dst`, `no_due_no_subtask_dues`.
4. Duplicating the last task in a bucket and a task between two others: order right after the original; subtasks keep their order. → Task 2 `duplicate_order`.
5. Opening an attachment whose file was deleted outside TaskCraft: a clear message with Remove, no crash. → Task 5 `open_missing_file_errors` + Task 7 UI handling.

---

### Task 1: Schema V5 and attachments in the store

**Files:** Modify `schema.rs` (V5, `CURRENT_VERSION = 5`), `model.rs`, `tasks.rs` (`delete_task_row` queues files, `attach_counts` adds `attachment_count`), `lib.rs`, `scripts/smoke.sh` (expects 5). Create `crates/taskcraft-store/src/attachments.rs`. Test: `tests/migrate_v5.rs`, `tests/attachments.rs`.

**Interfaces — Produces:**
- `pub struct Attachment { id, task_id, kind: AttachmentKind, name: String, stored: Option<String>, url: Option<String>, size: Option<i64>, created_at: DateTime<Utc>, sort: f64 }` (serde camelCase; `AttachmentKind { File, Link }` → "file"/"link").
- `Rule::{AttachmentTooLarge, AttachmentUrl, TemplateName}` with codes `attachment_too_large`, `attachment_url`, `template_name`.
- `Task.attachment_count: i64`.
- On `Store`: `add_file_attachment(task_id, src: &Path, data_dir: &Path) -> Result<Attachment>`; `add_link_attachment(task_id, url: &str, name: Option<&str>) -> Result<Attachment>`; `list_attachments(task_id) -> Result<Vec<Attachment>>`; `rename_attachment(id, name: &str) -> Result<()>` (1–120 chars after trim else `Invalid`); `reorder_attachments(task_id, ids: &[i64]) -> Result<()>`; `remove_attachment(id, now) -> Result<Attachment>` (returns the removed row for Undo; queues its file); `restore_file_attachment(task_id, name: &str, stored: &str, size: i64) -> Result<Attachment>` (stored must be queued and exist on disk, else `NotFound`; un-queues); `purge_pending_files(now, data_dir: &Path) -> Result<usize>`; `attachment_path(id, data_dir: &Path) -> Result<PathBuf>` (canonicalized, must stay under `<data_dir>/attachments`, else `Invalid`; missing file → `NotFound`).
- Crate-private: `pub(crate) const MAX_ATTACHMENT_BYTES: u64 = 104_857_600;` and a test hook so tests can lower the limit (e.g. `add_file_attachment_with_limit(…, limit: u64)` crate-public under `#[doc(hidden)] pub` used by `add_file_attachment`); `sanitize_name(&str) -> String`; `queue_file(conn, stored, now)`.
- `delete_task_row` queues the stored files of the task and its subtasks (delete_after = now + 15 s, `now = Utc::now()`).

V5 SQL as in the spec (`templates`, `attachments`, `pending_file_deletes`).

- [ ] **Step 1: Write failing tests** — `migrate_v5.rs`: `v4_keeps_everything` (V4 DB from `migrate_v4.rs` approach → version 5, tasks unchanged, `.bak-v4`). `attachments.rs` (tempdir data dir): `add_file_copies_into_data_dir` (content equal, size, stored under `attachments/<task>/`, source untouched); `too_large_rejected` (limit lowered to 10 bytes via the hook; 11-byte file → `Rule(AttachmentTooLarge)`, no file left, no row); `sanitize_and_unique_names` (`"a/b\\..:c?.stl"` → safe name without separators; adding the same file twice → two distinct stored paths; 300-char name capped at 120); `missing_source_not_found`; `link_validation` (`https://x.io/p` ok with default name `x.io`; `ftp://…`, `javascript:…`, `x.io` → `Rule(AttachmentUrl)`); `list_rename_reorder`; `remove_queues_then_purge_after_delay` (purge at now+10s keeps the file, at now+16s deletes it, count 1); `restore_relinks_queued_file`; `cascade_queues_files` (parent + subtask with files; delete parent → both files queued, rows gone; purge deletes both); `path_escape_refused` (a row whose stored path is `../../x` → `attachment_path` → `Invalid`); `attachment_count_on_task`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-store --test migrate_v5 --test attachments` → FAIL (compile).
- [ ] **Step 3: Implement.** Copy via a temp file in the same folder then `rename`; remove it on any later error.
- [ ] **Step 4: Run** `cargo test --workspace`, clippy → PASS.
- [ ] **Step 5: Commit** `feat(store): schema v5 and task attachments`

### Task 2: Duplicate a task

**Files:** Create `crates/taskcraft-store/src/duplicate.rs`. Test `tests/duplicate.rs`.

**Interfaces — Consumes:** Task 1 (attachments copy). **Produces:** `duplicate_task(id: i64, suffix: &str, data_dir: &Path) -> Result<i64>` per the spec (status todo; title + suffix; all fields copied; checklist unticked; subtasks copied with their checklists and attachments; attachments copied as new stored files / link rows; placement right after the original: top-level → midpoint to the next top-level task in the bucket or +1.0 (existing renumber rule when crowded), subtask → right after the original among siblings).

- [ ] **Step 1: Failing tests:** `duplicate_copies_everything` (labels, priority, start/due, alerts, notes, rrule; status todo even if source done; title "X (copy)"); `duplicate_order` (bucket [a,b,c]: duplicate a → [a,a',b,c]; duplicate c → [..,c,c']); `duplicate_with_subtasks_and_checklists` (children copied in order, all todo, checklist unticked, original untouched); `duplicate_copies_attachment_files` (new stored path, same content; removing the copy's attachment doesn't affect the original); `duplicate_subtask_stays_under_parent`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** in one transaction (file copies done first into temp names, renamed on commit; cleaned up on failure). **Step 4:** `cargo test --workspace`, clippy → PASS.
- [ ] **Step 5: Commit** `feat(store): duplicate tasks with their subtasks, checklists and attachments`

### Task 3: Templates in the store

**Files:** Create `crates/taskcraft-store/src/templates.rs`. Test `tests/templates.rs`.

**Interfaces — Produces:** `TemplateBody`/`TemplateSubtask` (serde, as in the spec); `pub struct TemplateInfo { id, name, subtask_count }`; `save_template(task_id, name: &str, replace: bool) -> Result<i64>`; `list_templates() -> Result<Vec<TemplateInfo>>` (by name, NOCASE); `rename_template(id, name)`; `delete_template(id)`; `create_from_template(template_id, input: &TaskInput) -> Result<i64>`; `template_by_name(name) -> Result<Option<TemplateInfo>>` (name_key match, for quick entry).

- [ ] **Step 1: Failing tests:** `save_computes_offsets` (parent due Fri 30 Oct; subtasks due Tue 27 10:00 and Fri 30 date-only and one undated → offsets -3 with "10:00", 0 with None, None); `save_drops_dates_and_attachments` (body has no dates/ids/attachments); `subtask_source_rejected`; `name_rules` (empty/61 chars → `Rule(TemplateName)`; duplicate (Unicode case) → `Duplicate`; `replace=true` overwrites, keeps id); `rename_and_delete`; `create_input_wins` (input title/due/plan/bucket used; template labels ∪ input labels; template priority used when input is Medium; template notes used when input notes empty); `create_children` (checklist + subtasks with checklists, all todo, sibling order); `offsets_across_dst` (Europe/Rome: due Mon 26 Oct 2026, offset -3 at 10:00 → Fri 23 Oct 10:00 local; due Mon 30 Mar 2026, offset -2 → Sat 28 Mar); `no_due_no_subtask_dues`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (offsets in the task's tz with local calendar-day arithmetic). **Step 4:** workspace tests, clippy → PASS.
- [ ] **Step 5: Commit** `feat(store): task templates with relative subtask dates`

### Task 4: `+template` in the parser

**Files:** `crates/taskcraft-parse/src/types.rs` (`Label::Template`, `ParseOptions.templates: Vec<String>` `#[serde(default)]`, `ParsedItem.template: Option<String>`), `lib.rs` (`mark_template` next to `mark_plan`, using `plan_key`). Test `tests/template_hint.rs`.

- [ ] **Step 1: Failing tests:** `plus_template_matches` (templates ["Ordine 3D"]: `"ship Rafael friday +ordine3d"` → template Some("Ordine 3D"), title "ship Rafael", one Template span); `unknown_plus_stays` (`"a +b"` → None, title "a +b"); `first_template_wins`; `plus_non_words_untouched` (`"1+1"`, `"c++ book"`, `"+"` unchanged); `events_ignore_template`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4:** `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** `feat(parse): +template in quick entry`

### Task 5: Tauri commands, opener, scheduler purge

**Files:** `src-tauri/Cargo.toml` (+ `tauri-plugin-opener = "2"`), `src-tauri/src/main.rs` (plugin init + handlers), `src-tauri/capabilities/default.json` (+ `dialog:allow-open`, opener permissions needed by the Rust-side calls if any), `src-tauri/src/state.rs` (`AppState.data_dir: PathBuf`; `in_memory()` uses a fresh temp dir), `src-tauri/src/commands.rs`, `src-tauri/src/scheduler.rs` (tick calls `purge_pending_files`), `crates/taskcraft-store/src/time.rs`/commands (`create_from_text` routes a matched template through `create_from_template`; `parse_options` fills `templates`).

**Interfaces — Produces commands:** `list_attachments{taskId}`, `pick_and_add_files{taskId}` (opens the multi-select open dialog via tauri-plugin-dialog, adds each file, returns the added `Attachment[]` and per-file errors as `{name, code}[]`), `add_link_attachment{taskId, url, name}`, `rename_attachment{id, name}`, `reorder_attachments{taskId, ids}`, `remove_attachment{id}` → Attachment, `restore_file_attachment{taskId, name, stored, size}`, `open_attachment{id}` (file → opener `open_path`; link → `open_url`), `reveal_attachment{id}` (opener `reveal_item_in_dir`), `duplicate_task{id}` (suffix from the UI language: EN " (copy)", IT " (copia)" — add to i18n.rs `Text`), `save_template{taskId, name, replace}`, `list_templates`, `rename_template{id, name}`, `delete_template{id}`, `create_from_template{templateId, input}`. Mutations call `changed(&app)`.

- [ ] **Step 1: Failing tests** in `commands::tests` (impl-level functions taking `&AppState`, like `create_from_text_impl`): `quick_entry_uses_template` (save a template from a task with 2 subtasks, then `create_from_text_impl("ship Rafael friday +name", …)` → new task titled "ship Rafael" with 2 subtasks); `duplicate_suffix_follows_language` (`Lang::It` → " (copia)"); `open_missing_file_errors` (attachment row whose file was deleted → `open_attachment_impl` returns code `not_found` without calling the opener — split resolution from opening so it is testable); `rule_codes_for_attachments`.
- [ ] **Step 2: Run** `cargo test -p taskcraft-app commands::tests` → FAIL. **Step 3: Implement** and register; scheduler purge with a unit test in scheduler.rs tests if the tick is testable there (`tick_purges_due_files`). **Step 4:** `cargo test --workspace`, clippy → PASS; `cargo build -p taskcraft-app` compiles with the plugin.
- [ ] **Step 5: Commit** `feat(app): commands for attachments, duplicates and templates`

### Task 6: UI data layer

**Files:** `ui/src/lib/types.ts`, `api.ts` (wrappers + `TRANSLATED_ERRORS` += 3 codes), `i18n.ts`, `dev/mock.ts` (templates/attachments in memory; `pick_and_add_files` returns a fake file), `quick/HighlightInput.svelte` (`template: '--label-calendar'`). Create `ui/src/lib/attachments.ts` (+ test).

**Interfaces — Produces:** types `Attachment`, `TemplateInfo`, `Task.attachmentCount`, `ParsedItem.template`, `Label` += `'template'`; helpers `formatSize(bytes: number, lang?)` ("812 B", "1.2 MB" / "1,2 MB" in IT), `urlHost(url: string): string`, `isWebUrl(s: string): boolean`; i18n keys (EN/IT): `attachments.title` Attachments/Allegati, `attachments.addFile` Add file…/Aggiungi file…, `attachments.addLink` Add link/Aggiungi link, `attachments.open` Open/Apri, `attachments.reveal` Show in folder/Mostra nella cartella (on Windows/Linux same text), `attachments.rename` Rename/Rinomina, `attachments.remove` Remove/Rimuovi, `attachments.missing` "The file is no longer available."/"Il file non è più disponibile.", `attachments.linkPlaceholder` "https://…", `task.duplicate` Duplicate/Duplica, `template.save` Save as template…/Salva come modello…, `template.name` Template name/Nome del modello, `template.replace` "Replace “{name}”?"/"Sostituire “{name}”?", `template.newFrom` New from template/Nuova da modello, `template.titleFor` "Title for {name}…"/"Titolo per {name}…", `template.none` "No templates yet. Save a task as a template from its menu."/"Nessun modello. Salva un'attività come modello dal suo menu.", `settings.templates` Templates/Modelli, `template.subtasks` "{n} subtasks"/"{n} sottotask", `error.attachment_too_large` "Files can be up to 100 MB."/"I file possono essere al massimo di 100 MB.", `error.attachment_url` "Links must start with http:// or https://."/"I link devono iniziare con http:// o https://.", `error.template_name` "Template names need 1 to 60 characters."/"Il nome del modello deve avere da 1 a 60 caratteri.".

- [ ] **Step 1: Failing tests** `attachments.test.ts`: formatSize cases (0, 999, 1024, 1_250_000 EN/IT, 104_857_600), urlHost (`https://www.x.io/a?b` → `x.io`; invalid → input), isWebUrl.
- [ ] **Step 2–4:** RED → implement → full UI checks GREEN.
- [ ] **Step 5: Commit** `feat(ui): attachment and template data layer`

### Task 7: Attachments in the details dialog, card count, Undo

**Files:** Create `ui/src/main/tasks/Attachments.svelte`; modify `Inspector.svelte` (section for tasks and subtasks), `BoardCard.svelte` (📎 n), `ui/src/lib/restore.ts` (+ tests in `restore.test.ts`: snapshots include attachments; restore re-adds files via `restoreFileAttachment` and links via `addLinkAttachment`, in original order).

- Rows: icon (file/link), name, size or host, Open, Show in folder (files), "…" menu (Rename inline, Remove with Undo toast → `restoreFileAttachment` / `addLinkAttachment`). "Add file…" → `pickAndAddFiles` (per-file errors shown as one toast listing names); "Add link" → inline URL input (Enter adds; Esc with text clears; invalid → inline error). Open failure with `not_found` → toast `attachments.missing` with action Remove. Reorder: drag (dropEffect move) + Move up/down in the menu.
- [ ] **Step 1: Failing tests** in `restore.test.ts` as above. **Step 2–4:** RED → implement → UI checks GREEN; visual in dev server AND in the installed app (`./scripts/install.sh` is the controller's job — instead run `cargo tauri dev` only if quick; otherwise note it) — EN/IT, light/dark.
- [ ] **Step 5: Commit** `feat(ui): attachments in task details`

### Task 8: Duplicate, Save as template, New from template, Settings → Templates

**Files:** `BucketBoard.svelte`/`TaskBoard.svelte` card menus and Inspector menu (Duplicate, Save as template…), `SubtaskList.svelte` row menu (Duplicate), create `ui/src/main/tasks/TemplateNameDialog.svelte`, `TasksView.svelte` + `BucketBoard.svelte` ("New from template ▸" menu next to New task inputs → title prompt → `createFromTemplate` → open details), `SettingsDialog.svelte` (Templates list: name, subtask count, inline Rename, Delete with confirm), helper `ui/src/lib/templates.ts` (`templateMenuItems(templates)`, `nameConflict(name, templates)`) + test.

- [ ] **Step 1: Failing tests** `templates.test.ts`: `nameConflict` uses Unicode case-insensitive comparison (`"Città"` vs `"CITTÀ"`); menu items sorted by name and empty-state item when none.
- [ ] **Step 2–4:** RED → implement → UI checks GREEN; visual EN/IT, light/dark: duplicate a task with subtasks, save as template (incl. replace prompt), new from template in a bucket, quick entry `+name`, rename/delete in Settings.
- [ ] **Step 5: Commit** `feat(ui): duplicate tasks and use templates`

### Task 9: Verify, docs, install and PR

**Files:** `docs/INSTALL.md` (bullet: duplicate, templates with `+name`, attachments stored in the data folder — moving the data folder moves them too), `README.md`.

- [ ] **Step 1:** Full suites green.
- [ ] **Step 2:** Read-only snapshot of the real DB → ignored test `real_db_copy_v5` in `migrate_v5.rs` on a copy → same task count, version 5.
- [ ] **Step 3 (controller, user-approved):** install, smoke (schema 5), test opening an attachment in the installed app, push, PR against `feat/subtasks` if #13 is unmerged (else `main`).
- [ ] **Step 4: Commit** `docs: duplicate, templates and attachments in the guide`
