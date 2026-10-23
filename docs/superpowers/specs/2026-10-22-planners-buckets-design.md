# Planners, buckets and ordering — design (phase 1 of 5)

Status: approved in conversation 2026-10-22, awaiting written-spec review.

## Context

The user asked for Microsoft Planner–style features. The agreed roadmap:

1. **Planners + buckets + ordering** (this spec)
2. Subtasks (full details, parent progress) + checklist + priority + start date
3. Duplicate + templates + attachments
4. Grid view + charts
5. Timeline + dependencies

Phase 1 comes first because buckets, templates and the timeline all hang off plans.
TaskCraft stays local, private and accountless; Mac/Windows/Linux; English/Italian.

## Decisions (from the user)

- **Plans are separate, labels are shared.** Every task belongs to exactly one plan.
  Labels (`#tags`) are global and work across plans.
- **Buckets are independent of status**, like Planner. Status stays a property of the
  task (the circle on the card). The Board has a *Group by* menu.
- **Quick entry** goes to a default plan (Settings), first bucket; `/planname` overrides.
- **Sidebar**: a *Planners* section listing plans, plus *All tasks*.
- **Deleting** a non-empty plan or bucket asks: move the tasks elsewhere, or delete them too.

## Data model (schema V3)

```
plans(id, name UNIQUE NOCASE NOT NULL, color NOT NULL, sort INTEGER NOT NULL)
buckets(id, plan_id → plans ON DELETE CASCADE, name NOT NULL, sort INTEGER NOT NULL)
tasks + plan_id INTEGER NOT NULL → plans
      + bucket_id INTEGER NOT NULL → buckets
      + sort_order REAL NOT NULL      -- position inside its bucket
settings: default_plan_id
```

Migration V2→V3 (inside the existing single-transaction `migrate`, after the existing
pre-migration backup):
- create plan "My tasks" (stored name follows the UI language at migration time:
  "My tasks" / "Le mie attività"), colour `#3b82f6`, sort 0;
- create bucket "To do" / "Da fare" in it;
- assign every existing task to that plan/bucket; `sort_order` = current display order
  (open tasks by due date then id, matching today's list order), spaced by 1.0.
- A fresh database gets the same default plan and bucket.

`sort_order` is a REAL so a move between two neighbours takes their midpoint; when the
gap falls below 1e-6 the bucket is renumbered 1.0, 2.0, … in one transaction.

Invariants enforced in the store (not only the UI):
- a task's bucket belongs to the task's plan;
- the last plan cannot be deleted (`Error::LastPlan`);
- a plan always has ≥1 bucket (`Error::LastBucket`);
- deleting a plan/bucket requires an explicit choice for its tasks:
  `DeleteMode::MoveTo(target)` or `DeleteMode::DeleteTasks`; there is no orphaning;
- deleting the default plan moves the default to the first remaining plan.

Tasks keep `calendar_id`: calendar colour and visibility on Day/Week/Month are unchanged.
The plan colour is used in task views (sidebar dot, card stripe in *All tasks*).

## Store API (taskcraft-store)

- `list_plans() -> Vec<Plan>` (with `open_count`), `create_plan(name, color)`,
  `update_plan(id, PlanPatch{name?, color?})`, `reorder_plans(ids)`,
  `delete_plan(id, DeleteMode)`.
- `list_buckets(plan_id)`, `create_bucket(plan_id, name)`, `rename_bucket`,
  `reorder_buckets(plan_id, ids)`, `delete_bucket(id, DeleteMode)`.
- `move_task(task_id, bucket_id, before: Option<i64>, after: Option<i64>)` —
  sets plan from the bucket and computes `sort_order`.
- `TaskInput` gains optional `plan_id`, `bucket_id`; omitted → default plan, first bucket,
  appended at the end.
- `Task` gains `plan_id`, `bucket_id`, `sort_order`.
- Duplicate plan names are rejected (`Error::Duplicate`) case-insensitively.

## Parser (taskcraft-parse)

- `/word` token in a task: `parse_task` returns `plan_hint: Option<String>` and removes the
  token from the title. Matching to a plan happens in the app layer: compare after
  lower-casing and removing spaces and punctuation (`/3dprint` ↔ "3D print").
- Unknown hint → token is put back into the title, task goes to the default plan.
- Only a token starting with `/` after whitespace or at the start counts; URLs and dates
  like `1/2` are not hints.

## UI

**Sidebar** (below the calendar views):
```
PLANNER            +
 ● Le mie attività   4
 ● 3D print          7
 ☰ Tutte le attività 13
```
- Each plan: colour dot, name, open-task count; context menu and a "…" button with
  Rename, Colour, Delete. *+* creates "New plan" in inline-rename mode.
- Plans can be reordered by drag (and Move up/down in the menu).

**Plan view** (Tasks view scoped to a plan): header with plan name, List | Board toggle,
*Group by* menu (Board only): **Bucket** (default), Status, Due date, Label.
(Priority is added in phase 2.)

**Board grouped by bucket:**
- Columns are buckets in order; "+ Add bucket" column at the end.
- Drag cards between and within columns; drag column headers to reorder buckets.
- Double-click a bucket title to rename; bucket header menu: Rename, Move left/right, Delete.
- "+ New task" at the bottom of each column creates the task in that bucket.
- Keyboard/a11y equivalents: card menu "Move to bucket ▸" and "Move up / Move down";
  drag-and-drop is never the only way.
- Other groupings (Status/Due/Label) behave like today's board; dragging between status
  columns changes status, as now; within-column order follows `sort_order`.

**List** inside a plan: grouped as today (`taskPageGroups`), ordered by `sort_order` within
groups where no date ordering applies.

**All tasks**: today's Tasks page across all plans; cards/rows show a small plan dot.
No bucket grouping here.

**Details dialog**: adds *Plan* and *Bucket* pickers (bucket list follows the chosen plan).

**Settings**: *Default plan for new tasks* picker.

**Delete dialog** (plan or bucket with tasks): "This plan has 7 tasks." —
radio *Move them to [plan/bucket ▾]* (default) / *Delete them too*; buttons Cancel / Delete.
Empty plan/bucket: simple confirm. Last plan / last bucket: Delete is disabled with a
tooltip explaining why.

All strings in `lib/i18n.ts` (EN/IT); colours from existing tokens; contrast tests stay green.

## Error handling

- Store errors (`Duplicate`, `LastPlan`, `LastBucket`, not found) map to localized toasts.
- A failed move rolls back and the board re-renders from the store's state.

## Testing

- **Store (Rust):** V2→V3 migration keeps every task and order; fresh DB has default
  plan/bucket; move between/within buckets; midpoint renumbering; delete plan/bucket with
  MoveTo and DeleteTasks; LastPlan/LastBucket errors; bucket-plan consistency; default plan
  reassignment; duplicate names.
- **Parser:** `/3dprint` extracted; `1/2`, URLs not extracted; unknown hint kept in title.
- **UI logic (vitest):** plan-name matching; board grouping by bucket; computing
  before/after neighbours for a drop; open counts.
- **Visual:** dev server with mock data via Claude in Chrome, EN and IT, light/dark.

## Out of scope (later phases)

Priority, subtasks, checklist, start date (phase 2); duplicate/templates/attachments
(phase 3); grid/charts (phase 4); timeline/dependencies (phase 5); sync/sharing.
