# Navigation, Grid view and charts — design (phase 4 of 5)

Status: approved in conversation, awaiting written-spec review.
Builds on phase 3 (`feat/templates`, PR #14). Branch: `feat/views`.

## Decisions (from the user)

- The top switcher becomes **Calendario · Attività · Griglia**. **Planner** (timeline) is
  added with phase 5. Calendario contains a small **Day | Week | Month** switch that remembers the
  last choice.
- **Grid:** an editable table for the plan selected in the sidebar (or All tasks), with subtasks
  indented.
- **Charts:** a collapsible panel above the table.

## Navigation

- `Main.svelte` view state becomes `area: 'calendar' | 'tasks' | 'grid'` plus
  `calendarMode: 'day' | 'week' | 'month'`. `calendarMode` is persisted in localStorage
  (`taskcraft.calendarMode`, read and written in try/catch); the default is `week`.
- **Toolbar:** the area buttons are Calendar / Tasks / Grid (EN), Calendario / Attività / Griglia
  (IT). The title and the prev / next / today controls show only in Calendar, together with the
  Day | Week | Month segmented switch. The "+" button keeps its behaviour: new event in Calendar,
  new task in Tasks and in Grid.
- **Shortcuts:**
  - **Mod+1 / Mod+2 / Mod+3:** Calendar / Tasks / Grid. Mod+4 is reserved for Planner (phase 5).
  - **D / W / M:** without a modifier and only when focus is not in a text field, they switch the
    calendar mode while in Calendar.
  - **Existing shortcuts:** Mod+N, Mod+T and Mod+, keep their meaning.
- **Sidebar plan selection:** it opens Tasks, or stays in Grid when Grid is the current area.
  The plan scope is shared by Tasks and Grid.
- **Migration of saved state:** none needed. The old `view` value was not persisted.
- **Docs:** the INSTALL.md mention of "Day, Week, Month and Tasks views" is updated.

## Grid (table)

- **Rows:** top-level tasks of the scope, with subtasks indented under them. Each parent is
  collapsible; the collapsed set is session-only. Filters are those of the Tasks page: the label
  filter from the sidebar and the status filter. A subtask matching the filters whose parent does
  not appears under a dimmed parent row (the phase-2 `groupWithParents` rule).
- **Columns:**
  - Title, Status, Priority, Bucket, Plan (only in All tasks), Labels, Start, Due, Progress
    (subtask bar + "x of y"), Attachments (📎 count).
  - A "Columns" menu toggles visibility, persisted per scope in localStorage
    (`taskcraft.grid.columns.<scope>`). Title is always visible.
- **Sorting:**
  - Clicking a header sorts ascending, then descending, then back to the default order.
  - The default order is bucket order, then `sortOrder`; in All tasks it is plan order, then that.
  - Subtasks stay under their parent and are sorted among themselves by the same key.
  - Empty values sort last.
  - The sort state is shown with `aria-sort`.
- **Inline editing:**
  - Title: text input. Enter saves, Esc cancels. An empty title is refused inline.
  - Status, Priority, Bucket: popup menu (the existing `lib/menu.svelte.ts`). Bucket is not
    editable for subtasks, because they follow the parent. Status changes go through
    `changeStatus`, so the phase-2/3 prompts and the auto-start rule apply.
  - Start / Due: date input (time is kept if present). The store's start ≤ due rule shows the
    translated error.
  - Labels, notes, subtasks and attachments: double-click the row (or press Mod+Enter on it) to
    open the details dialog.
  - Every edit goes through the existing api (`updateTask`, `moveTask` for a bucket change at the
    end of the target bucket, `changeStatus`), preserving all other fields via `taskInput`, and
    offers Undo like the other views.
- **Keyboard:**
  - The table is an ARIA grid: arrow keys move the active cell, Home/End move to the row's first or
    last cell, Enter starts editing and F2 does the same, Esc cancels, Space on the Status cell
    cycles the status like the glyph does.
  - Mod+Enter opens the row's details.
  - Left/Right on the Title cell of a parent collapses or expands its subtasks, or moves to the
    neighbouring cell when that cell is already in that state.
- **Size:** the table must stay responsive with 500 rows. Windowing is used only if a 500-row
  render exceeds 16 ms per frame in the dev build; otherwise plain rows.

## Charts panel

- **Placement:** above the table and collapsible, with its state persisted in localStorage
  (`taskcraft.grid.charts`). It covers the scope's tasks after the label and status filters, but
  before the chart filter itself.
- **4 stat tiles:**
  - **Open:** top-level tasks not done.
  - **In progress:** top-level.
  - **Overdue:** open top-level tasks past due, by the same rule as `isOverdue`.
  - **Completed in the last 7 days:** top-level tasks with `completedAt` within the last 7 × 24 h.
- **Bar charts:**
  - By **Status**: To do / In progress / Done.
  - By **Priority**: Urgent / Important / Medium / Low.
  - By **Bucket**, in bucket order. In All tasks it is by **Plan** instead.
  - **Completed per week**: the last 8 weeks ending with the current week. Weeks start on the
    user's week-start setting; local dates.
- **What is counted:** top-level tasks only, so the numbers match the open counts used elsewhere.
- **Click a bar → filter the table:**
  - Clicking a Status / Priority / Bucket bar filters the table to that value. A chip "Priority:
    Urgent ×" above the table clears it, and clicking the same bar again also clears it. One chart
    filter at a time.
  - Weekly bars are not clickable.
- **Rendering:**
  - Plain SVG in Svelte, no chart library (no new dependencies).
  - Colours come from tokens. Status bars use the status colours, priority bars the
    danger/warning/neutral tokens, and buckets/plans one neutral series colour with the plan colour
    for plans.
  - Light and dark mode.
  - Each chart has a text alternative (a visually hidden table or an aria-label listing the
    values). Bars show their value as text.
  - Implementers read the `dataviz` skill before writing chart code.
- **Empty state:** "No tasks yet" in place of the charts when the scope has no tasks.

## Strings

All in EN/IT:
- Area names, Day/Week/Month, the Columns menu, column headers, the sort hint, the chart titles,
  the stat tiles, the chart filter chip, empty states.

## Testing

- **Pure helpers (vitest):**
  - `gridRows(tasks, scope, filters, sort, collapsed)`: ordering, subtasks under parents, dimmed
    parents, empty values last, the three sort states.
  - `gridStats(tasks, now, weekStart)`: tiles, the overdue rule, the 7-day window, 8 week buckets
    across a DST change and a year boundary.
  - `chartFilter` application.
  - Grid keyboard navigation (`nextCell(pos, key, shape)`).
  - Area/shortcut mapping.
  - localStorage persistence for calendarMode and columns (with storage unavailable).
- **Visual:** dev server (EN/IT, light/dark), then the installed app: navigation and shortcuts,
  sort, every inline editor, Undo, subtask collapse, the columns menu, charts, click-to-filter.

## Out of scope

The Planner timeline and dependencies (phase 5); exporting the table; custom chart builders;
editing labels inline in the grid.
