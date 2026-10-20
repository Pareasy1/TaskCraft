# TaskCraft on Mac, Windows and Linux — design

Date: 2026-10-20 · Approved in conversation (platforms, signing, hosting, design).

## Goal

Non-expert users on macOS, Windows and Linux install TaskCraft from one
download link with a familiar double-click installer and no Terminal.

## Decisions

| Topic | Decision |
|---|---|
| Platforms | macOS 14+, Windows 10/11, Linux (Ubuntu 22.04+ and derivatives) |
| "Single file" | Not possible across OSes; one download page picks the right installer |
| Installers | Mac: universal `.dmg` · Windows: NSIS `Setup.exe`, per-user (no admin) · Linux: `.deb` + `.AppImage` |
| Build | GitHub Actions on each OS: CI on PRs/pushes, release build on tags `v*` |
| Signing | Unsigned (free). One-time prompts documented: Mac "Open Anyway", Windows "More info → Run anyway" |
| Hosting | Public GitHub Releases + a GitHub Pages download page that detects the OS |

## App changes (one codebase)

- **Window chrome:** overlay title bar and hidden title only on macOS
  (`cfg(target_os = "macos")`); native decorations elsewhere. The sidebar's
  traffic-light spacer is shown only on macOS (`html[data-platform="mac"]`).
- **Keyboard:** a `platform` helper: the command modifier is ⌘ on macOS and
  Ctrl elsewhere; all shortcut checks and labels go through it. Default quick
  entry shortcut: `Alt+Super+Space` on macOS, `Control+Alt+Space` elsewhere.
- **Paths:** data in the OS data dir (already `dirs::data_dir()/TaskCraft`);
  logs in `~/Library/Logs/TaskCraft` on macOS, `<data dir>/TaskCraft/logs`
  elsewhere.
- **Tray:** the menu gains "Quick entry" (Linux tray icons cannot report
  clicks). The popover is placed near the tray where the OS reports a position
  and centred on screen otherwise.
- **Wake-from-sleep recheck:** macOS only (existing); other systems rely on the
  30 s tick.
- **Icons:** `scripts/make-icons.py` also writes `icon.ico` for Windows.
- Dev scripts `install.sh`, `smoke.sh`, `package.sh` stay macOS-only.

## Delivery

- `.github/workflows/ci.yml`: Rust tests, UI tests/check and a debug app build
  on macos-latest, windows-latest and ubuntu-22.04 for PRs and pushes to main.
- `.github/workflows/release.yml`: on tag `v*` (or manual run), `tauri-action`
  builds the installers on each OS and attaches them to a GitHub Release.
- `site/index.html` + `.github/workflows/pages.yml`: a one-page download site
  on GitHub Pages; it reads the latest release through the GitHub API and
  offers the installer for the visitor's OS, with the others listed.
- `docs/INSTALL.md` rewritten for non-experts: Mac, Windows, Linux sections,
  click by click, including the one-time security prompt, updating and
  uninstalling.

## Verification

- Unit tests for the platform helper and per-OS defaults; CI green on all three OSes.
- Local: macOS `.dmg` built and checked.
- Not possible here: running the app on Windows and Linux. A short manual
  checklist is handed to the user.

## Out of scope

Code signing/notarization, auto-update, mobile, app stores, Linux packages
other than `.deb`/AppImage.
