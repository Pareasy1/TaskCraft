# TaskCraft

A personal calendar and task app (Mac, Windows, Linux) with Fantastical-style natural-language
quick entry. Rust core (parser, SQLite store, alerts) with a Tauri 2 + Svelte UI.
All data stays on your computer.

## Download

**Mac, Windows and Linux:** <https://pareasy1.github.io/TaskCraft/> (picks the right installer for your computer).
Step-by-step help, no Terminal needed: [docs/INSTALL.md](docs/INSTALL.md).

## Using it

- **Quick entry:** press **⌥⌘Space** (changeable in Settings) or click the
  menu-bar icon, then type a sentence:
  - `Lunch with Sam tomorrow at 1 at Trattoria da Enzo`
  - `Standup every weekday at 9:30 /work`
  - `Dentist friday 3pm alert 30 min before`
  - `todo buy milk friday` · `remind me to call mom at 6pm`
  - `Trip Oct 24 to Oct 26` · `Yoga 6-7pm` · `Gym in 3 days for 2 hours at 7am`

  **↩** adds, **⌘↩** adds and opens it, **esc** clears or closes.
- **Grid:** a table of the selected plan (sort by column, edit title/status/priority/bucket/dates in place with Undo, choose columns, fold subtasks), with a charts panel on top whose bars filter the table. Top menu: Calendar / Tasks / Grid (⌘1/⌘2/⌘3, Ctrl on Windows/Linux).
- **Main window:** ⌘1 Calendar · ⌘2 Tasks · ⌘3 Grid (Ctrl on Windows/Linux); inside Calendar, D / W / M switch Day, Week and Month · ⌘N new event · ⌘T today.
  Drag events to move them, drag the bottom edge to resize, double-click empty
  space to create. Closing the window keeps TaskCraft in the menu bar.
- **Tasks page:** ⌘2 (or *Tasks* in the top menu). Everything typed there becomes a
  task — dates are optional ("buy milk", "pay rent friday 9am"). Filter by
  All / Today / Upcoming / No date / Completed; tasks are grouped by due day,
  overdue first. Click a task or event to select it; double-click it (or press Return) to open its details in a dialog.
  - **Status:** To do → In progress → Done. Click the round status mark to move a
    task to the next status, or right-click it to pick one. Done is the same as ticking it off.
  - **Labels:** type `#home`, `#work`… in a task to label it (created
    automatically). Click a label to filter by it; the tag icon opens the label
    manager to rename, recolor, merge or delete labels.
  - **Board:** switch *List / Board* to see To do · In progress · Done columns.
    Drag cards between columns, or focus a card and press ← / →.
  - **Plans:** group tasks into plans (listed in the sidebar), each with buckets shown as
    draggable columns. Type `/planname` in quick entry to file a task in a plan, e.g.
    `todo ship Rafael friday /3dprint`.
  - **Subtasks, checklists and priority:** open a task's details (double-click) to add subtasks, a checklist and a start date. Set priority in quick entry with `!urgent` / `!urgente`, e.g. `todo ship Rafael friday /3dprint !urgente`.
  - **Duplicate, templates and attachments:** duplicate a task from its card menu, save it as a template and reuse it with `+name` in quick entry (e.g. `todo ship Rafael friday /3dprint +ordine3d`), and attach files (copied into the data folder). The subtask progress bar shows in-progress steps in yellow; starting a subtask starts the task, and finishing all of them asks to finish the task.

## Install / update

```bash
scripts/install.sh   # tests, builds, signs ad hoc, installs to /Applications, launches
scripts/smoke.sh     # post-install checks
```

Requires Rust (rustup), Node, and the Tauri CLI (`cargo install tauri-cli --version "^2"`).

## Where things live

- Data: `~/Library/Application Support/TaskCraft/taskcraft.db` (+ `settings.json`)
- Logs: `~/Library/Logs/TaskCraft/`
- **Settings:** the gear in the main window's bottom-left corner, ⌘, or *Settings…* in the menu-bar menu. It opens as a dialog over the main window.
- Backup / move to another calendar app: Settings → Export .ics…

## Uninstall

```bash
rm -rf /Applications/TaskCraft.app ~/Library/Application\ Support/TaskCraft ~/Library/Logs/TaskCraft
```

## Development

```bash
cargo test --workspace          # parser, store, app
npm --prefix ui run test        # UI layout helpers
cd src-tauri && cargo tauri dev # run with hot reload
npm --prefix ui run dev         # browser preview with a mock backend: http://localhost:5173/?window=main|quick (add &lang=it for Italian)
```
