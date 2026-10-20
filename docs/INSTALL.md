# Installing TaskCraft on another Mac

TaskCraft is a calendar and task app that lives in the Mac's menu bar. This guide is for installing it on someone else's Mac, for example a family member's.

**Before you start**

- **macOS only.** TaskCraft needs **macOS 14 Sonoma or newer**, on an Apple Silicon (M1 or later) or Intel Mac. It does not run on Windows or Linux.
- **Each Mac keeps its own data.** Nothing is shared or synced between Macs and there is no account. Everything stays on the Mac it was typed on.
- **Expect one extra "Open Anyway" step the first time.** TaskCraft is a personal app and isn't distributed through the App Store or checked (notarized) by Apple. macOS will block it once, and you allow it in System Settings. Step 3 shows how.

There are two ways to install it:

| | Who it's for | What you need |
|---|---|---|
| **A. Ready-made copy** (recommended) | Anyone | The file `TaskCraft-0.1.0-macos-universal.zip` |
| **B. Build it yourself** | Comfortable with Terminal | The source code and about 15 minutes |

---

## A. Install a ready-made copy

### 1. Get the file

Ask whoever runs TaskCraft for **`TaskCraft-0.1.0-macos-universal.zip`** (about 12 MB). AirDrop, a USB stick, or a shared folder all work. The same file works on both Apple Silicon and Intel Macs.

> **For the person sharing it:** create the file on your Mac with `./scripts/package.sh` from the project folder. It ends up in the `dist/` folder.

### 2. Put TaskCraft in Applications

1. Double-click the zip. **TaskCraft** appears next to it.
2. Drag **TaskCraft** into the **Applications** folder (Finder → Go → Applications).

### 3. Open it the first time

1. Open **Applications** and double-click **TaskCraft**.
2. macOS says it can't verify the app and offers **Done** (on macOS 14: **OK**) or **Move to Trash**. Click **Done** / **OK**. Do not move it to the Trash.
3. Open **System Settings** → **Privacy & Security**.
4. Scroll down to the **Security** section. Next to the line about TaskCraft being blocked, click **Open Anyway**.
5. Confirm with your password or Touch ID, then click **Open Anyway** again.

You only do this once. From then on TaskCraft opens normally.

<details>
<summary>Shortcut for Terminal users</summary>

Instead of steps 2–5, run this once and then open TaskCraft normally:

```bash
xattr -dr com.apple.quarantine /Applications/TaskCraft.app
```
</details>

### 4. Allow notifications

On first launch macOS asks whether TaskCraft may send notifications. Click **Allow**, or reminders for events and tasks won't appear. You can change this later in **System Settings** → **Notifications** → **TaskCraft**.

That's it. Continue with [Using TaskCraft](#using-taskcraft).

---

## B. Build it yourself

1. Install Apple's command-line tools. Run this in Terminal and accept the prompt:
   ```bash
   xcode-select --install
   ```
2. Install **Rust**, then add the TaskCraft build tool:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   cargo install tauri-cli --version "^2" --locked
   ```
3. Install **Node.js** (version 22 or newer) from <https://nodejs.org> or with Homebrew (`brew install node`).
4. Get the code and install:
   ```bash
   git clone https://github.com/Pareasy1/TaskCraft.git
   cd TaskCraft
   ./scripts/install.sh
   ```
   The script runs the tests, builds the app, signs it for this Mac, copies it to Applications and opens it. Because it was built on this Mac, there is no "Open Anyway" step.

To make a copy for other Macs instead, run `./scripts/package.sh` and follow section A with the file it creates.

---

## Using TaskCraft

- **Open quick entry** from anywhere with **⌥⌘Space** (Option-Command-Space), or click the calendar icon in the menu bar.
- **Type a sentence and press Return.** Quick entry understands **English**, even when the rest of the app is in Italian:
  - `Lunch with Sam tomorrow at 1` creates an event.
  - `todo pay rent friday 9am #home` creates a task with a label.
  - `Gym every monday at 7am alert 15 min before` creates a repeating event with a reminder.
- **Open the main window** from the menu-bar icon (or ⌘↩ after typing). It has Day, Week, Month and Tasks views.
- **Settings:** the gear in the main window's bottom-left corner, or ⌘,. There you can change the shortcut, turn on *Launch at login*, add calendars, and export everything as an `.ics` file.
- **Language:** TaskCraft uses English or Italian, following the Mac's language (System Settings → General → Language & Region).

Closing the main window keeps TaskCraft running in the menu bar so reminders still arrive. To quit it completely, use the power icon at the bottom of the quick-entry popover, or *Quit TaskCraft* in the menu-bar menu.

---

## Updating

Repeat section A with the newer zip, and choose **Replace** when copying into Applications. Your events and tasks are kept. If you built it yourself, run `git pull` and then `./scripts/install.sh` again.

## Moving or backing up your data

All data is in one folder:

```
~/Library/Application Support/TaskCraft/
```

- **Backup:** Settings → **Export .ics…** saves everything in a format Apple Calendar and Google Calendar can import. To keep TaskCraft's own data, quit TaskCraft and copy the folder above.
- **Moving to a new Mac:** quit TaskCraft on both Macs, then copy the folder above to the same place on the new Mac.

## Uninstalling

1. Quit TaskCraft (menu-bar icon → *Quit TaskCraft*).
2. Drag **TaskCraft** from Applications to the Trash.
3. To also delete your data, run this in Terminal (this cannot be undone):
   ```bash
   rm -rf ~/Library/Application\ Support/TaskCraft ~/Library/Logs/TaskCraft
   ```

## Troubleshooting

| Problem | Fix |
|---|---|
| **"TaskCraft is damaged and can't be opened"** or **"can't be verified"** | Follow step 3 above (*Open Anyway*), or run the Terminal shortcut. The app isn't damaged; macOS shows this for apps not notarized by Apple. |
| **⌥⌘Space opens a Finder search window instead** | macOS uses that shortcut for Finder search on some Macs. Pick another one in TaskCraft's Settings, or turn off *Show Finder search window* in System Settings → Keyboard → Keyboard Shortcuts → Spotlight. |
| **No reminders appear** | System Settings → Notifications → TaskCraft → allow notifications. Also check that TaskCraft is running (its icon is in the menu bar). |
| **Can't find the app after closing it** | It's still in the menu bar. Click the calendar icon there. |
| **Dates in quick entry aren't recognised** | Type the sentence in English (`tomorrow at 1`, `friday 9am`). |
| **It doesn't start on an older Mac** | TaskCraft needs macOS 14 Sonoma or newer. |
