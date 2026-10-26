# Duplicate, templates and attachments — design (phase 3 of 5)

Status: approved in conversation, awaiting written-spec review.
Builds on phase 2 (`2026-10-23-subtasks-checklist-priority-design.md`, branch `feat/subtasks`, PR #13).
Branch: `feat/templates` (from `feat/subtasks`).

## Decisions (from the user)

- **Templates are made from an existing task** ("Save as template…"), with **relative dates** for
  subtasks.
- **Attachments are copied into TaskCraft's data folder** (plus web links), and open with the
  system's default app.
- The user approved adding the official `tauri-plugin-opener` (open files/URLs, reveal in folder).

## Data model (schema V5)

```
templates(id, name TEXT NOT NULL UNIQUE COLLATE NOCASE, body TEXT NOT NULL, created_at TEXT NOT NULL)
attachments(id, task_id INTEGER NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('file','link')),
            name TEXT NOT NULL, stored TEXT,      -- file: relative path under <data>/attachments/
            url TEXT,                             -- link: http(s) URL
            size INTEGER,                         -- file bytes
            created_at TEXT NOT NULL, sort REAL NOT NULL)
pending_file_deletes(stored TEXT PRIMARY KEY, delete_after_utc TEXT NOT NULL)
```
Migration V4→V5 in the single-transaction `migrate`, after the backup (`.bak-v4`).
No SQL foreign keys on `attachments.task_id`. Every task-delete path deletes the task's
attachment rows inside `delete_task_row`, which keeps the cleanup in one place. Each stored file
is queued in `pending_file_deletes` with `delete_after_utc = now + 15 s` instead of being removed
at once.

`templates.body` is JSON (serde) of:
```
TemplateBody { title, notes, labels: [String], priority, checklist: [String],
               subtasks: [{ title, notes, labels, priority, checklist: [String],
                            due_offset_days: Option<i64>, due_time: Option<"HH:MM"> }] }
```
`due_offset_days` is the subtask's local due date minus the parent's local due date, in calendar
days. It is set only when both have a due. `due_time` is set when the subtask's due has a time.
Template names: trimmed, 1–60 chars, duplicates compared Unicode case-insensitively with the
store's existing `name_key` (`Duplicate`).

## Store API

- `duplicate_task(id, now) -> i64` duplicates one task.
  - **Copied fields:** title + suffix, notes, labels, priority, start, due, alerts, rrule, plan,
    bucket and calendar.
  - **Status:** always `todo`.
  - **Placement:**
    - A top-level copy goes right after the original in its bucket. Its `sort_order` is the
      midpoint to the next top-level task, or +1.0 if the original is last. The existing
      renumbering applies.
    - A subtask copy stays under the same parent, right after the original.
  - **Children:** checklist items are copied unticked; subtasks are copied (status `todo`,
    unticked checklists); attachments are copied (see below).
  - **Title suffix:** passed in by the caller (the localized " (copy)" / " (copia)"); the API is
    `duplicate_task(id, suffix: &str)`.
- `save_template(task_id, name, replace: bool) -> i64` builds the body from the task.
  - A task that is itself a subtask gives `Invalid`.
  - An existing name with `replace=false` gives `Duplicate`; with `replace=true` it overwrites
    the body.
- `list_templates() -> Vec<TemplateInfo{id,name,subtask_count}>`, `rename_template(id, name)`,
  `delete_template(id)`.
- `create_from_template(template_id, input: TaskInput) -> i64` creates the task from `input`.
  - **Input wins over the template:** the input's title, due, plan, bucket, calendar and alerts
    are used. Template notes, labels and priority fill in only where the input left them empty or
    at default (labels: union of both).
  - **Created children:** the template's checklist; its subtasks with their checklists; subtask due
    dates computed from the new task's local due date + `due_offset_days`, at `due_time` when given
    (in the task's tz). No parent due means subtasks get no due.
  - Runs in one transaction.
- Attachments:
  - `add_file_attachment(task_id, src_path, data_dir) -> Attachment` validates and stores the file.
    - A missing or unreadable source gives `NotFound`; a file larger than 100 MiB gives
      `Rule(AttachmentTooLarge)`.
    - The file is copied to `<data_dir>/attachments/<task_id>/<n>-<sanitized name>`. Sanitizing
      keeps letters, digits, `.`, `-`, `_` and spaces, replaces anything else with `_`, and caps the
      name at 120 chars. `n` makes the name unique.
    - The copy is written to a temp name and then renamed; if the DB insert fails, the file is
      removed.
  - `add_link_attachment(task_id, url, name: Option<&str>) -> Attachment` accepts only http/https,
    otherwise `Rule(AttachmentUrl)`. The name defaults to the URL host.
  - `list_attachments(task_id)`, `rename_attachment(id, name)`, `reorder_attachments(task_id, ids)`.
  - `remove_attachment(id, now)` deletes the row and queues the file.
  - `restore_file_attachment(task_id, name, stored, size) -> Attachment` is used by Undo (of a
    removed attachment or a deleted task). It re-links a queued stored file and removes it from
    `pending_file_deletes`; if the file is gone, `NotFound`. The 15 s delay is longer than the
    Undo toast, so Undo always finds the file.
  - `purge_pending_files(now, data_dir) -> usize` deletes the files that are due, and is called by
    the scheduler tick. A file missing on disk counts as already done.
  - Duplicating copies each file to a new stored path; links are copied as rows.
- `Task` gains `attachment_count`, filled by the same bulk counts as the subtask and checklist
  counts.
- New `Rule` codes: `attachment_too_large`, `attachment_url`, `template_name`.

## Parser

`+word` on a task, like `/plan`:
- It matches a template name via `ParseOptions.templates`, compared with `plan_key`.
- It sets `ParsedItem.template: Option<String>` to the exact template name and labels the token
  `Label::Template`.
- An unknown `+word` stays in the title. The first match wins. Events ignore it.

## Tauri

- Commands for all of the above. The data dir comes from `AppState`.
- `pick_files` uses the existing dialog plugin's open dialog with multiple selection (add the
  `dialog:allow-open` permission).
- `open_attachment(id)` and `reveal_attachment(id)` resolve the stored path inside the data dir.
  They refuse any path that escapes `<data>/attachments`, then call `tauri-plugin-opener`
  `open_path` / `reveal_item_in_dir`.
- `open_url` (links) is only called for http(s).
- Quick entry: `create_from_text` with a matched template calls `create_from_template`.
- The scheduler tick calls `purge_pending_files`.

## UI

- **Card menu and details dialog menu:**
  - "Duplicate" (also on subtask rows in the dialog).
  - "Save as template…": a small dialog for the name; a duplicate name asks "Replace “X”?".
- **"New from template ▸":**
  - next to the plan's "New task" input, and in each bucket column footer;
  - opens a menu listing templates;
  - picking one turns the input into "Title for X…" (Enter creates through
    `create_from_template` with the typed title, the plan/bucket and no due) and then opens the
    new task's details.
- **Settings → Templates:** list with subtask count, Rename (inline), Delete (confirm).
- **Details dialog → Attachments section** (tasks and subtasks):
  - rows with an icon (file/link), name, size or host, and Open, Show in folder (files only),
    and a "…" menu with Rename and Remove;
  - "Add file…" (multi-select) and "Add link" (inline URL input; Enter adds);
  - dropping files from Finder/Explorer onto the section is not supported (Tauri's file-drop
    handler is disabled for HTML5 drag-and-drop); the buttons are the way.
- **Board card:** 📎 n when it has attachments.
- **Quick entry highlight:** `template` uses the calendar label colour.
- **Delete Undo** (phase 2 restore helper) also restores attachments via
  `restore_file_attachment` and links via `add_link_attachment`.

All strings EN/IT; tokens only; drags with only `move` set `dropEffect`; menu equivalents for
all actions.

## Error handling

- Rule codes map to localized toasts: too large, bad URL, template name, duplicate name.
- A missing attachment file on Open shows "The file is no longer available" and offers Remove.
- A copy failing midway leaves no partial file and no row.

## Testing

- **Store:**
  - V4→V5 migration;
  - `duplicate_task`:
    - order right after the original;
    - subtasks, unticked checklists and attachments copied;
    - status todo;
    - subtask duplicate;
  - templates:
    - save from a task: offsets computed, dates dropped, attachments excluded;
    - subtask source rejected;
    - name rules and replace;
    - rename, delete;
  - `create_from_template`:
    - input wins;
    - due + offsets across DST, at `due_time`;
    - no due gives no subtask dues;
  - attachments:
    - copy into a temp data dir;
    - 100 MiB limit (use a sparse/large temp file or a smaller limit injected for tests);
    - sanitizing and unique names;
    - link validation;
    - remove queues the file, purge deletes it after the delay, restore re-links it;
    - cascade delete queues all files of the task and its subtasks;
    - path-escape refusal.
- **Parser:** `+template` matching, unknown kept, first wins, events ignore.
- **Commands:** quick entry with `+template`.
- **UI (vitest):** pure helpers (size formatting, URL host, menu wiring).
- **Visual:** dev server and the installed app (EN/IT, light/dark).

## Out of scope

Attachment previews/thumbnails; drag-and-drop of files from the OS; editing a template's content
other than by re-saving from a task; templates for events; attachments on events; sync.
