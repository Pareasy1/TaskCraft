# Subtasks, checklist, priority and start date — design (phase 2 of 5)

Status: approved in conversation 2026-10-23, awaiting written-spec review.
Builds on phase 1 (`2026-10-22-planners-buckets-design.md`, branch `feat/planners`, PR #12).
Branch: `feat/subtasks` (from `feat/planners`).

## Decisions (from the user)

- **One level**: a task can have subtasks; a subtask cannot have subtasks.
- **Subtasks live inside their parent**: same plan and bucket as the parent; the board card
  shows progress and expands; the list indents them; the calendar shows each dated subtask
  on its own day.
- **Completion asks and suggests**, never automatic: completing a parent with open subtasks
  asks; completing the last open subtask offers to complete the parent.
- **Checklist** on both tasks and subtasks, counted separately from subtasks.
- **Priority and start date like Planner**: priority Urgent/Important/Medium/Low with
  quick-entry words; start date shown as a span on the calendar and hiding the task from
  "Today" before it starts.

## Data model (schema V4)

```
tasks + parent_id INTEGER NULL        -- the parent task; NULL for top-level tasks
      + priority TEXT NOT NULL DEFAULT 'medium'
          CHECK(priority IN ('urgent','important','medium','low'))
      + start_utc TEXT NULL           -- same encoding as due_utc
      + start_has_time INTEGER NOT NULL DEFAULT 0
checklist_items(id, task_id INTEGER NOT NULL, text TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0, sort REAL NOT NULL)
CREATE INDEX tasks_parent ON tasks(parent_id, sort_order);
CREATE INDEX checklist_task ON checklist_items(task_id, sort);
```

Migration V3→V4 inside the single-transaction `migrate`, after the existing backup
(`.bak-v3`). Existing tasks: priority `medium`, no start, no parent, no checklist.
As in V3, no SQL foreign keys are added to existing `tasks` columns (SQLite limitation
with defaults); the store enforces integrity. `checklist_items` is a new table, but deletion
is still done explicitly by the store, alongside alerts and labels, so that every task-delete
path has one shared cleanup.

Subtasks are ordinary rows in `tasks`. They reuse status, due, labels, notes, alerts,
priority, start date and calendar.

### Store invariants

- `parent_id` must point at an existing **top-level** task. Setting a parent on a task that has
  subtasks, or pointing at a subtask, is `Invalid`. A task cannot be its own parent.
- A subtask's `plan_id` and `bucket_id` always equal its parent's. Setting a parent copies them.
  Moving the parent (`move_task`, `update_task` placement change, plan/bucket delete with
  MoveTo) moves its subtasks too. `move_task` on a subtask is `Invalid` (subtasks are
  reordered with `reorder_subtasks`).
- Subtask order: `sort_order` among siblings, spaced 1.0, appended last when added.
- Subtasks cannot repeat: a subtask with `rrule` is `Invalid`, and setting a parent on a
  repeating task is `Invalid`.
- When a repeating parent advances to its next occurrence (`set_task_completed` path), its
  subtasks are reopened (status `todo`, `completed_at` NULL) and its checklist items unticked.
  Their due dates are left unchanged.
- Deleting a task deletes its subtasks and every checklist item of both, plus their alerts
  and label links, in one transaction. The same applies to plan/bucket DeleteTasks.
- Promoting a subtask (`parent_id` → NULL) keeps its plan and bucket and appends it at the
  end of that bucket.
- `start` must not be after `due` when both are set (`Invalid`).

### Store API

- `TaskInput` gains `parent_id: Option<i64>` (`#[serde(default)]`), `priority: Priority`
  (default Medium), `start: Option<DateTime<Utc>>`, `start_has_time: bool`.
- `Task` gains `parent_id`, `priority`, `start`, `start_has_time`, `subtask_open: i64`,
  `subtask_total: i64`, `checklist_done: i64`, `checklist_total: i64`.
- `list_subtasks(parent_id) -> Vec<Task>` (sibling order); `reorder_subtasks(parent_id, ids)`.
- `set_task_parent(task_id, parent_id: Option<i64>)` — promote/demote with the rules above.
- `complete_with_subtasks(task_id, now)` — marks the parent and its open subtasks done.
- Checklist: `list_checklist(task_id)`, `add_checklist_item(task_id, text) -> i64`,
  `update_checklist_item(id, text: Option<&str>, done: Option<bool>)`,
  `reorder_checklist(task_id, ids)`, `delete_checklist_item(id)`,
  `checklist_item_to_subtask(id) -> i64`. This creates a subtask titled with the item's text
  and deletes the item. On a subtask it is `Invalid`.
- Checklist text: trimmed, 1–200 chars, otherwise `Invalid`.
- `list_tasks` keeps returning every task, subtasks included. The UI groups them.
  "Today/Upcoming" filtering in the store treats a task whose start is after today as not
  in Today.

## Parser (quick entry)

Priority words on tasks, anywhere in the sentence, removed from the title and highlighted:
`!urgent`/`!urgente`, `!important`/`!importante`, `!low`/`!bassa`, plus `!medium`/`!media`.
They match case-insensitively, and the first one wins. `ParsedItem.priority: Option<Priority>`.
`Label::Priority`. Other `!words` stay in the title. Events ignore them.
Quick entry has no syntax for start dates or subtasks in this phase.

## UI

**Details dialog (Inspector), task:**
- Priority select, Start date (+ optional time) next to Due.
- **Subtasks** section on a top-level task:
  - a list of rows, each with a status circle, title, due and labels; click opens that
    subtask's details;
  - an "Add subtask" input (Enter adds and keeps focus);
  - reorder by drag or Move up/down in the row menu;
  - row menu: Open, Move up, Move down, Make normal task, Delete.
- **Checklist** section:
  - rows with a checkbox and inline-editable text;
  - an "Add item" input (Enter adds);
  - reorder by drag or Alt+↑/↓;
  - row menu: Move up, Move down, Make subtask (top-level only), Delete.
- On a subtask: a "Part of: Spedire Rafael" link to the parent, a Parent picker
  ("None" = make normal task), and no Repeat field.
- On a top-level task without subtasks and without repeat: a Parent picker listing top-level
  tasks of the same plan (excluding itself).

**Board card:**
- Priority mark before the title.
- "▸ 2 of 4" with a thin progress bar when it has subtasks; "☑ 3/5" when it has a checklist.
- Clicking ▸ expands its subtasks under the card, each with a status circle (togglable)
  and title. Expanded state is remembered per task for the session.
- Start date shown as "24 → 28 ott" when both are set.
- Subtasks never appear as their own cards.
- Card menu gains Priority ▸ (four items).

**Group by Priority** added to the plan board: columns Urgent, Important, Medium, Low.
Dragging between them changes priority. Order inside a column follows `sortOrder`.

**List (Tasks page):**
- Top-level tasks are grouped as today.
- Subtasks are indented under their parent, collapsible, and in sibling order.
- A subtask whose parent is filtered out by a tab/filter appears under a dimmed parent
  header, so it never floats alone.

**Calendar (Day/Week/Month):**
- Dated subtasks appear on their day with the title "Subtask · Parent".
- A task with start and due appears in the all-day strip of every day from start to due
  (timed or not).
- First and last days are marked as such, and the days in between are marked as
  continuing.
- Moving it by keyboard or drag shifts start and due together.

**Completion prompts:**
- Ticking a parent with open subtasks opens a small dialog: "Complete the 2 open subtasks
  too?" with "Complete all" (default), "Only this task" and Cancel.
- Ticking the last open subtask shows a toast "All subtasks of Spedire Rafael are done" with
  a "Complete it" button.

**Deleting a parent:** the confirmation names the subtask count.

**Sidebar task sections and quick-entry popup's upcoming list:** list top-level tasks only,
with progress.

All strings in EN and IT; existing tokens for colours (priority red/orange from existing
danger/warning tokens; contrast tests stay green).

## Error handling

Store `Invalid` reasons map to localized toasts:
- parent is a subtask;
- task has subtasks;
- repeating;
- start after due;
- checklist text length.

A failed reorder snaps back by reloading, as in phase 1.

## Testing

- **Store:**
  - V3→V4 migration keeps everything;
  - parent rules (one level, self, repeating);
  - subtasks follow parent moves and plan/bucket deletes;
  - cascade delete removes subtasks, checklist, alerts;
  - repeat reset;
  - `complete_with_subtasks`;
  - promote/demote placement;
  - checklist CRUD, order, convert-to-subtask;
  - start ≤ due;
  - counts on `Task`;
  - Today/Upcoming with start dates.
- **Parser:**
  - each priority word in EN and IT;
  - other `!words` untouched;
  - events ignore them;
  - first wins.
- **UI (vitest):**
  - subtask grouping and indentation;
  - orphan-parent header;
  - progress text;
  - span days for start→due, including timed and multi-week;
  - group by priority;
  - the "last subtask done" decision.
- **Visual:** dev server with mock data, EN/IT, light/dark.

## Out of scope

Subtasks of subtasks, repeating subtasks, quick-entry syntax for subtasks or start dates,
showing the checklist on the card (only its count), templates/duplicate (phase 3).
