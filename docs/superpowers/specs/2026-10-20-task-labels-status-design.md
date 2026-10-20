# Task labels, status and Tasks page redesign — design

Date: 2026-10-20 · Builds on: `2026-10-19-taskcraft-design.md` and the Tasks page (PR #2)

## Goal

Let the user tag tasks with optional labels, track work with a status
(To do · In progress · Done), view tasks as a date-grouped list or a board,
and give the Tasks page a polished look. Personal use, local data only.

## Decisions (agreed with the user)

| Topic | Decision |
|---|---|
| Statuses | Fixed: `todo`, `in_progress`, `done`. Displayed "To do", "In progress", "Done". |
| Done vs checkbox | Same thing. Ticking sets `done`; unticking sets `todo`. |
| Layouts | List (existing date grouping) and Board (status columns), toggled; choice remembered per viewer (localStorage). |
| Labels | Typed as `#word`, auto-created, several per task, optional. Tasks only. |
| Redesign scope | Tasks page only, plus the minimum side-panel additions to edit status and labels. |
| Board order | Within a column, by due date (no due date last), then id. No manual ordering. |

## Data (schema V2)

Migration from V1, in one transaction:

```sql
ALTER TABLE tasks ADD COLUMN status TEXT NOT NULL DEFAULT 'todo'
  CHECK(status IN ('todo','in_progress','done'));
UPDATE tasks SET status = 'done' WHERE completed_at IS NOT NULL;
CREATE TABLE labels(
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  color TEXT NOT NULL
);
CREATE TABLE task_labels(
  task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  label_id INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
  PRIMARY KEY(task_id, label_id)
);
```

- `CURRENT_VERSION` becomes 2. A fresh database runs V1 then V2.
- **Backup fix (deferred minor #17):** the pre-migration backup uses
  `VACUUM INTO '<db>.bak-v<old>'` instead of a file copy, so committed data
  still in the `-wal` file is included.
- Invariant: `status = 'done'` ⇔ `completed_at IS NOT NULL`.
- Label colors are assigned round-robin from a fixed 8-color palette
  (by count of existing labels) when a label is created.

## Store API (`taskcraft-store`)

- `TaskStatus` enum (`Todo`, `InProgress`, `Done`; serde `todo` / `inProgress` / `done` in camelCase over IPC, `todo` / `in_progress` / `done` in SQL).
- `TaskInput` gains `status: TaskStatus` and `labels: Vec<String>` (both
  `#[serde(default)]`). `Task` gains `status` and `labels: Vec<TaskLabel>` where
  `TaskLabel { id, name, color }` (named to avoid the parser's span `Label`).
- `insert_task` / `update_task`: label names are trimmed, a leading `#`
  stripped, empty names dropped, duplicates (case-insensitive) collapsed;
  missing labels are created; links are replaced. Status `done` on insert or
  update sets `completed_at` (if not already set); any other status clears it.
- `set_task_status(id, status, now)`:
  - `done` behaves exactly like `set_task_completed(id, true)` today; for a
    recurring task that means advancing `due` to the next occurrence and the
    task stays open, so its status becomes `todo`.
  - `todo` / `in_progress` clear `completed_at` and set the status.
- `set_task_completed(id, done)` keeps working and maintains the invariant
  (`true` → `done`, `false` → `todo`).
- `list_labels() -> Vec<TaskLabel>` ordered by name.
- `update_label(id, name, color)`: empty name → `Invalid`. If another label
  already has that name (case-insensitive), the two are merged: links move to
  the existing label and `id` is deleted.
- `delete_label(id)`: removes the label and its links.
- `list_tasks` and `get_task` return labels; `search` also matches label names.

## Parser (`taskcraft-parse`)

- New label `Label::Tag` for spans and `ParsedItem.tags: Vec<String>`.
- A token is a tag when it starts with `#`, is at a word start, and the rest
  is 1–32 chars of letters, digits, `-` or `_` containing at least one letter
  (`#home`, `#q4-plan`; not `#1`, not the `#` in `C#`).
- Only when the item is a task (prefix or `parse_task`). In event text the
  token stays in the title and no tag is produced.
- Tags are removed from the title; original spelling kept; duplicates
  (case-insensitive) collapsed in first-seen order.
- `from_parsed` copies `tags` into `TaskInput.labels`.

## Commands (`src-tauri`)

`set_task_status(id, status)`, `list_labels()`, `update_label(id, name, color)`,
`delete_label(id)`. All mutating commands emit `items-changed`. Existing
commands unchanged apart from the new fields.

## UI

### Tasks page (redesigned)

1. **Header row:** add field (live highlighting incl. `#tags`, preview line
   shows labels) and a List / Board segmented switch.
2. **Filter bar:** date filters (All · Today · Upcoming · No date ·
   Completed, with counts), a Status filter (Any · To do · In progress), and
   label chips; selecting several chips shows tasks having *any* of them.
   A "Manage labels" popover lists labels with rename, color swatch picker and
   delete.
3. **List:** existing date grouping. Each row: checkbox, title, label chips,
   status pill (click cycles To do → In progress → Done; a small menu picks
   directly), repeat icon, due, calendar dot.
4. **Board:** columns To do · In progress · Done with counts. Cards: title,
   due (red when overdue), labels, calendar dot, repeat icon. Drag a card to
   another column to change status (HTML5 drag and drop); keyboard: focus a
   card, `←`/`→` moves it one column. Done shows the 30 most recent with a
   "Show all" toggle. The Completed date filter is hidden in Board mode
   (Done column covers it).
5. Date, status and label filters apply to both layouts.

Clicking a label chip anywhere on the page toggles it in the filter.

### Side panel (task)

Adds a Status segmented control and a labels field (chips with ✕, typing
offers matching existing labels, Enter adds).

### Visual direction ("quiet desk planner", via /frontend-design)

Warm neutral surfaces in light and dark; a clearer type scale for the page;
status colors used sparingly (grey To do, amber dot with a soft pulse for In
progress, green Done); soft-tinted label chips; board cards with gentle depth
and a lift while dragging; small purposeful motion (check-off, row exit,
column reflow), disabled under `prefers-reduced-motion`. Built on the app's
existing CSS tokens so it sits next to the calendar views.

## Error handling

Command failures show the existing error banner. A failed drag leaves the
card in its original column (state is re-read on `items-changed`). Empty or
invalid label names are rejected with a message.

## Testing

- **Store:** V1→V2 migration with existing rows (completed → `done`; backup
  file exists and contains rows written while in WAL mode); status ⇔
  `completed_at` sync for normal and recurring tasks; label create,
  case-insensitive reuse, rename, merge, delete; labels returned by
  `list_tasks`/`get_task`; search by label.
- **Parser:** tags extracted and removed from the title; `C#`, `#1`, event
  text, duplicates, highlighting spans.
- **Commands:** `create_from_text(.., as_task=true)` with tags creates labels;
  `set_task_status` round trip.
- **UI (vitest):** status and label filtering; board column grouping and
  ordering; Done column limit.
- **Visual:** dev mock in Chrome, light and dark, list and board.
- **Install:** `scripts/install.sh` and `scripts/smoke.sh` pass.

## Out of scope

Custom statuses, manual card ordering, labels on events, label hierarchy,
restyling the rest of the app.
