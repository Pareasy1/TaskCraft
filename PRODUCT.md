# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

TaskCraft is a macOS desktop app: a Tauri 2 shell around a Svelte 5 web UI, installed to /Applications. "web" describes the UI technology; the experience is a Mac app.

## Users

The owner and a few people they know (family or a small team), each using TaskCraft on their own Mac, every day. They plan their days, capture events and tasks quickly while doing something else, and carry tasks through to done.

## Product Purpose

A personal calendar and task manager. One typed sentence becomes an event or a task ("Lunch with Sam tomorrow at 1", "todo pay rent friday #home"). Events live in Day / Week / Month views; tasks live in a list or a board with statuses and labels; alerts arrive as macOS notifications. Success means capturing takes seconds and nothing planned is forgotten.

## Positioning

- **Private and local.** No account, no cloud, no subscription. All data is a SQLite file on that Mac (`~/Library/Application Support/TaskCraft`).
- **Tasks with a real workflow next to the calendar.** Tasks have statuses (To do, In progress, Done), `#labels`, and a board, not just a checklist beside a calendar.

## Operating Context

- Quick entry from anywhere: a menu-bar popover opened by a global shortcut (default ⌥⌘Space) or the menu-bar icon.
- A main window for planning (Day, Week, Month, Tasks) and a Settings window.
- Notifications for event and task alerts; the app keeps running in the menu bar when its window is closed, and can launch at login.
- Each person's Mac holds its own data; nothing is shared between Macs.

## Capabilities and Constraints

- Natural-language parsing is rule-based and understands **English only**; the interface follows the Mac's language (English or Italian), dates follow the matching locale.
- Events: all-day, timed, repeating (with "this / all" edits), alerts, location, people, URL; drag and keyboard editing; `.ics` export.
- Tasks: optional due date and time, repeating, statuses, labels (typed as `#name`), list and board views, undo after changes.
- Calendars are local and user-named (seeded "Personal" and "Work").
- Built and installed locally with an ad-hoc signature (`scripts/install.sh`); not distributed through the App Store.
- **Open decision:** sharing or syncing data between people or Macs is not planned; do not design for it unless decided.

## Brand Commitments

- Name: **TaskCraft**.
- App icon: the blue rounded square with a hand-drawn calendar, checklist and pencil (`Icon/TaskCraft.png`). It is the identity and stays.
- The menu-bar icon is a monochrome template image (`src-tauri/icons/tray@2x.png`), as macOS expects.

## Evidence on Hand

- Real product: source in this repository, installed build in /Applications.
- No testimonials, users' quotes, metrics, pricing or press exist; never invent them.

## Product Principles

1. **Capture first.** Anything that slows down typing a sentence and pressing Return is a regression.
2. **Local and private by default.** Never add a feature that needs an account or sends data off the Mac without an explicit decision.
3. **A Mac app, not a web page.** System font, light and dark mode, keyboard shortcuts, menu-bar behaviour and native window conventions come first.
4. **Tasks are work, not a list.** Status and labels are first-class wherever tasks appear.
5. **Readable for everyone who uses it.** Accessibility is a floor, not a feature.

## Accessibility & Inclusion

WCAG 2.1 AA is the baseline: text contrast ≥ 4.5:1 (status marks and controls ≥ 3:1) in light and dark mode, full keyboard operation including alternatives to dragging, visible focus, labelled controls, screen-reader announcements for confirmations and errors, and `prefers-reduced-motion` respected. `ui/src/lib/tokens.test.ts` and `ui/src/lib/style-rules.test.ts` guard part of this.
