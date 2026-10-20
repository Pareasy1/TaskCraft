# Cross-platform installers — implementation plan

> Execute natively (user's choice). Spec: `docs/superpowers/specs/2026-10-20-cross-platform-design.md`.

Global constraints: no behaviour change on macOS; all text through i18n (EN/IT);
tests first for logic; no new runtime dependencies beyond Tauri plugins already used.

1. **Rust portability** — `src-tauri/src/{windows,state,logging,settings,tray}.rs`
   - Test first: `settings::default_hotkey()` is `Alt+Super+Space` on macOS and
     `Control+Alt+Space` elsewhere; `state::log_dir(home, data)` per OS.
   - Gate `title_bar_style`/`hidden_title` on macOS; tray menu "Quick entry"
     (i18n `quick_entry`); popover centred when no tray position is known.
2. **UI portability** — `ui/src/lib/platform.ts` (+ test)
   - `isMac(ua)`, `isMod(e)` (⌘ on Mac, Ctrl elsewhere), `modLabel(key)`
     ("⌘N" / "Ctrl+N"); replace every `metaKey` check and ⌘ label;
     `html[data-platform]`; hide the traffic-light spacer off macOS.
3. **Bundles and icons** — `scripts/make-icons.py` writes `icon.ico`;
   `tauri.conf.json` bundle targets per OS (dmg+app / nsis currentUser / deb+appimage).
4. **CI + release** — `.github/workflows/ci.yml`, `release.yml`.
5. **Download page** — `site/index.html`, `.github/workflows/pages.yml`.
6. **Guide** — rewrite `docs/INSTALL.md` (three OSes, no Terminal), README Download section.
7. **Verify and ship** — local suites + mac `.dmg`; push; watch CI on all three
   OSes and fix; PR; with the user's OK enable Pages and tag the first release.
