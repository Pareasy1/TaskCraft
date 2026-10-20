#!/usr/bin/env bash
# Builds a shareable TaskCraft for other Macs: a universal app (Apple Silicon
# and Intel), signed ad hoc, zipped into dist/. See docs/INSTALL.md.
#
#   ./scripts/package.sh            # universal (needs the x86_64 Rust target; added if missing)
#   ./scripts/package.sh --native   # this Mac's architecture only (faster)
set -euo pipefail
cd "$(dirname "$0")/.."
source "$HOME/.cargo/env" 2>/dev/null || true

TARGET="universal-apple-darwin"
[ "${1:-}" = "--native" ] && TARGET=""

VERSION=$(python3 -c "import json; print(json.load(open('src-tauri/tauri.conf.json'))['version'])")

echo "==> Tests"
cargo test --workspace --quiet
npm --prefix ui ci --silent
npm --prefix ui run check
npm --prefix ui run test

echo "==> Build"
if [ -n "$TARGET" ]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  (cd src-tauri && cargo tauri build --bundles app --target "$TARGET")
  APP="target/$TARGET/release/bundle/macos/TaskCraft.app"
  ARCH="universal"
else
  (cd src-tauri && cargo tauri build --bundles app)
  APP="target/release/bundle/macos/TaskCraft.app"
  ARCH="$(uname -m)"
fi

echo "==> Sign (ad hoc)"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"

echo "==> Zip"
mkdir -p dist
ZIP="dist/TaskCraft-$VERSION-macos-$ARCH.zip"
rm -f "$ZIP"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
echo "Built $ZIP"
echo "Architectures: $(lipo -archs "$APP/Contents/MacOS/taskcraft")"
echo "SHA-256: $(shasum -a 256 "$ZIP" | cut -d' ' -f1)"
echo "Send this zip to the other Mac and follow docs/INSTALL.md."
