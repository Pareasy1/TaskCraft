#!/usr/bin/env bash
# Post-install checks for /Applications/TaskCraft.app.
set -euo pipefail
BIN=/Applications/TaskCraft.app/Contents/MacOS/taskcraft
DB="$HOME/Library/Application Support/TaskCraft/taskcraft.db"
LOGS="$HOME/Library/Logs/TaskCraft"
fail() { echo "FAIL: $*" >&2; exit 1; }

for _ in $(seq 1 20); do pgrep -f "$BIN" >/dev/null && break; sleep 0.5; done
pgrep -f "$BIN" >/dev/null || fail "TaskCraft is not running"
echo "ok   app running"

ls "$LOGS"/taskcraft*.log >/dev/null 2>&1 || fail "no log file in $LOGS"
echo "ok   log file present"

[ "$(sqlite3 "$DB" 'PRAGMA user_version')" = "5" ] || fail "database schema version is not 5"
echo "ok   database schema v5"

OUT=$("$BIN" --smoke "Lunch with Sam tomorrow at 1") || fail "smoke run failed"
TITLE=$(printf '%s' "$OUT" | /usr/bin/python3 -I -c 'import json,sys; print(json.load(sys.stdin)["title"])')
[ "$TITLE" = "Lunch" ] || fail "smoke title was '$TITLE'"
echo "ok   parser + store smoke: $OUT"

codesign --verify --deep --strict /Applications/TaskCraft.app || fail "signature invalid"
echo "ok   signature valid"
echo "All smoke checks passed."
