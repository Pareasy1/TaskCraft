#!/usr/bin/env bash
# Builds TaskCraft, signs it ad hoc and installs it into /Applications.
# Re-run to update. Your data in ~/Library/Application Support/TaskCraft is kept.
set -euo pipefail
cd "$(dirname "$0")/.."
source "$HOME/.cargo/env" 2>/dev/null || true

echo "==> Tests"
cargo test --workspace --quiet
npm --prefix ui ci --silent
npm --prefix ui run check
npm --prefix ui run test

echo "==> Build"
(cd src-tauri && cargo tauri build --bundles app)
APP="target/release/bundle/macos/TaskCraft.app"

echo "==> Sign (ad hoc)"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"

echo "==> Install"
osascript -e 'quit app "TaskCraft"' >/dev/null 2>&1 || true
pkill -f "/Applications/TaskCraft.app/Contents/MacOS/taskcraft" 2>/dev/null || true
sleep 1
rm -rf /Applications/TaskCraft.app
ditto "$APP" /Applications/TaskCraft.app
xattr -dr com.apple.quarantine /Applications/TaskCraft.app 2>/dev/null || true

echo "==> Launch"
open -a /Applications/TaskCraft.app
echo "TaskCraft installed. Press ⌥⌘Space or click the menu-bar icon."
