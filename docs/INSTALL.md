# Installing TaskCraft

TaskCraft is a calendar and task app. It runs on **Mac, Windows and Linux**. Installing it takes a couple of minutes and **needs no Terminal or technical knowledge**.

**Download:** <https://pareasy1.github.io/TaskCraft/>

The page offers the right installer for your computer. All files are also on the [releases page](https://github.com/Pareasy1/TaskCraft/releases/latest).

> **Why the extra security step?** TaskCraft is a free, personal app. It isn't sold in an app store, and it isn't signed with a paid certificate. So the first time you open it, macOS or Windows asks you to confirm. You do this **once**, and the steps below show exactly where to click.

- [Mac](#mac)
- [Windows](#windows)
- [Linux](#linux)
- [Using TaskCraft](#using-taskcraft)
- [Updating, moving and removing](#updating-moving-and-removing)
- [Problems?](#problems)

---

## Mac

**You need:** macOS 14 Sonoma or newer. It works on both Apple Silicon (M1 and later) and Intel Macs.

1. On the download page, click **Download for Mac**. You get a file ending in **`.dmg`**.
2. Double-click the `.dmg` file. A window opens showing TaskCraft and an Applications folder.
3. **Drag TaskCraft onto the Applications folder.**
4. Open **Applications** (in Finder's sidebar) and double-click **TaskCraft**.
5. macOS says it can't verify the app. Click **Done** (on macOS 14: **OK**). Don't move it to the Trash.
6. Open **System Settings** → **Privacy & Security**. Scroll down to **Security**, and next to the message about TaskCraft click **Open Anyway**. Confirm with your password or Touch ID.
7. When macOS asks whether TaskCraft may send **notifications**, click **Allow**. This is what makes reminders work.

TaskCraft's icon now appears in the menu bar at the top of the screen. You can eject the TaskCraft disk on the desktop.

## Windows

**You need:** Windows 10 or 11 (64-bit).

1. On the download page, click **Download for Windows**. You get a file ending in **`setup.exe`**.
2. Open the file. You'll find it in your **Downloads** folder, or click it in the browser's download list.
3. If a blue box says **"Windows protected your PC"**, click **More info**, then **Run anyway**.
4. The installer opens. Click **Install** (choose a language first if it asks), then **Finish**. No administrator password is needed.
5. TaskCraft starts. Its icon sits in the **system tray**, at the bottom-right next to the clock.
   - If you don't see it, click the small **^** arrow by the clock.
   - To keep it always visible, drag it from there onto the taskbar.
6. If Windows asks whether TaskCraft may send **notifications**, click **Allow**.

You'll also find TaskCraft in the **Start** menu.

## Linux

**You need:** a 64-bit Ubuntu 22.04 or newer, or a similar system (Linux Mint, Pop!\_OS, Debian 12, Zorin OS). The `.AppImage` works on most other systems.

**Ubuntu, Mint, Debian and similar (recommended):**

1. On the download page, click **Download for Linux**. You get a file ending in **`.deb`**.
2. Double-click the file in your **Downloads** folder. The **App Center** (or **Software**) app opens. Click **Install** and enter your password.
   - If double-clicking opens something else, right-click the file and choose **Open With → App Center** (or **Software Install**).
3. Open **TaskCraft** from your applications menu.

**Any other system (AppImage):**

1. Download the **Linux AppImage** from the download page.
2. Right-click the file → **Properties** → **Permissions**, and tick **Allow executing file as program** (or *Executable as Program*).
3. Double-click it to start TaskCraft.

> **Tray icon on Linux.** Click TaskCraft's icon in the top bar to open its menu, then choose **Quick Entry**. On plain GNOME (for example Fedora), install the **AppIndicator and KStatusNotifierItem Support** extension so the icon appears. Ubuntu has it already.

---

## Using TaskCraft

- **Add something quickly from anywhere.** Press the quick-entry shortcut, type a sentence, and press **Return**:
  - **Mac:** **⌥⌘Space** (Option-Command-Space)
  - **Windows and Linux:** **Ctrl+Alt+Space**

  You can also click TaskCraft's icon in the menu bar or tray. Quick entry understands **English**, even when the rest of the app is in Italian:
  - `Lunch with Sam tomorrow at 1` adds an event.
  - `todo pay rent friday 9am #home` adds a task with a label.
  - `Gym every monday at 7am alert 15 min before` adds a repeating event with a reminder.
- **Plans keep different projects apart.** Your plans are listed in the sidebar. Each plan shows its tasks as columns (buckets) that you can drag cards between. In quick entry, add `/planname` to put a task in a plan, for example `todo ship Rafael friday /3dprint`.
- **Plan in the main window.** It has Day, Week, Month and Tasks views. **Double-click** anything to open its details.
- **Settings:** the gear at the bottom-left of the main window, or **⌘,** on Mac and **Ctrl+,** on Windows and Linux. There you can change the shortcut, choose **Launch at login**, add calendars, and export everything.
- **Language:** English or Italian, following your computer's language.

Closing the main window keeps TaskCraft running in the menu bar or tray, so reminders still arrive. To quit completely, choose **Quit TaskCraft** from its icon's menu.

**Your data stays on your computer.** There's no account and nothing is uploaded. Each computer keeps its own events and tasks; they don't sync with each other.

## Updating, moving and removing

**Update:** download the newer installer and install it the same way (on Mac, choose **Replace**). Your events and tasks are kept.

**Back up:** Settings → **Export .ics…** saves everything in a file that Apple Calendar, Google Calendar and Outlook can import.

**Move to another computer:** quit TaskCraft on both computers, then copy the TaskCraft data folder across to the same place on the new one:

| System | Data folder |
|---|---|
| Mac | `~/Library/Application Support/TaskCraft` (Finder → Go → Go to Folder…) |
| Windows | `%APPDATA%\TaskCraft` (paste into File Explorer's address bar) |
| Linux | `~/.local/share/TaskCraft` |

**Remove:**

- **Mac:** quit TaskCraft, then drag it from Applications to the Trash.
- **Windows:** Settings → Apps → Installed apps → TaskCraft → **Uninstall**.
- **Linux:** in App Center (or Software), go to **Installed**, find TaskCraft and click **Uninstall**. For the AppImage, delete the file.

Removing the app keeps your data folder (see the table above), in case you reinstall. Delete that folder too if you want everything gone.

## Problems?

| What you see | What to do |
|---|---|
| **Mac:** "TaskCraft can't be opened", "can't be verified" or "is damaged" | Follow Mac steps 5–6 (**Open Anyway**). The app isn't damaged; macOS says this about apps that aren't signed with a paid certificate. |
| **Windows:** "Windows protected your PC" | Click **More info**, then **Run anyway**. |
| **Mac:** ⌥⌘Space opens a Finder search window | Choose another shortcut in TaskCraft's Settings, or turn off *Show Finder search window* in System Settings → Keyboard → Keyboard Shortcuts → Spotlight. |
| **Linux:** the shortcut does nothing | Some Linux desktops (Wayland) don't allow global shortcuts. Use the tray icon → **Quick Entry**, or log in with an "on Xorg" session. |
| **No reminders appear** | Allow notifications for TaskCraft in your system's notification settings, and check TaskCraft is running (its icon is in the menu bar or tray). |
| **Can't find the app after closing it** | It's still running. Click its icon in the menu bar (Mac) or tray (Windows: the **^** by the clock). |
| **Dates aren't recognised** | Type the sentence in English: `tomorrow at 1`, `friday 9am`, `every monday`. |
| **Mac:** it won't start | TaskCraft needs macOS 14 Sonoma or newer. |
| **Linux:** the AppImage won't start | Use the `.deb` instead, or install the `libfuse2` package. |

---

## For developers

To build from source (Rust, Node.js 22+ and the Tauri CLI), see the project [README](../README.md).

- `./scripts/install.sh` builds and installs it on your own Mac.
- `./scripts/package.sh` makes a shareable Mac zip.
- Release installers for all three systems are built by GitHub Actions when a version tag such as `v0.2.0` is pushed (`.github/workflows/release.yml`).
