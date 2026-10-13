#!/usr/bin/env bash
# Fast "does it actually work in the real app?" build.
#
# `tauri dev` covers most UI work, but some things only exist in a real bundle —
# the desktop pet (a second Tauri window), the dock icon, tray, and anything that
# depends on the app being installed. This builds a DEBUG bundle (much faster than
# release: no LTO, no dmg), installs it to /Applications, and relaunches it.
#
#   ./scripts/testbuild.sh          build + install + relaunch
#   ./scripts/testbuild.sh --no-run build + install only
#
set -euo pipefail
cd "$(dirname "$0")/.."

# pnpm requires a newer Node than the one first on PATH here (/usr/local/bin is
# v22.3, pnpm wants >=22.13). Prefer Homebrew's if it satisfies that, so the script
# doesn't die on an environment detail.
if command -v /opt/homebrew/bin/node >/dev/null 2>&1; then
  need=22.13
  have="$(/opt/homebrew/bin/node -v 2>/dev/null | tr -d v)"
  if [ "$(printf '%s\n%s\n' "$need" "$have" | sort -V | head -1)" = "$need" ]; then
    PATH="/opt/homebrew/bin:$PATH"
  fi
fi

APP="Superuser.app"
OUT="src-tauri/target/debug/bundle/macos/$APP"
LOG="/tmp/superuser-testbuild.log"

echo "▸ building debug bundle (this is the slow part)…"
# --bundles app skips the dmg, which is pure overhead for a local test.
if ! pnpm tauri build --debug --bundles app >"$LOG" 2>&1; then
  echo "✗ build failed — last 30 lines:"
  tail -30 "$LOG"
  exit 1
fi

[ -d "$OUT" ] || { echo "✗ expected bundle missing: $OUT"; tail -20 "$LOG"; exit 1; }

echo "▸ installing to /Applications…"
# Quit the running copy first, or the replace races a live process.
osascript -e "tell application \"Superuser\" to quit" >/dev/null 2>&1 || true
pkill -f "/Applications/$APP" >/dev/null 2>&1 || true
sleep 1
rm -rf "/Applications/$APP"
cp -R "$OUT" /Applications/
# Locally-built and unsigned: strip quarantine so it opens without the Gatekeeper prompt.
xattr -dr com.apple.quarantine "/Applications/$APP" 2>/dev/null || true

if [ "${1:-}" != "--no-run" ]; then
  echo "▸ launching…"
  open "/Applications/$APP"
fi
echo "✓ installed /Applications/$APP  (log: $LOG)"
