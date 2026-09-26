#!/usr/bin/env bash
# Proves the project does not care what its directory is called.
#   1. no absolute paths or folder name anywhere in the source or build output
#   2. the built app still runs from a directory with a different name
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
NAME=$(basename "$ROOT")
APP=$(sed -n 's/^name = "\(.*\)"/\1/p' "$ROOT/repo/Cargo.toml" | head -1)
export TMPDIR="$ROOT/tmp/portable"
export XDG_CACHE_HOME="$ROOT/tmp/cache"
mkdir -p "$TMPDIR" "$XDG_CACHE_HOME"
FAIL=0

say() { printf '\033[38;5;39m%-22s\033[0m %s\n' "$1" "$2"; }
ok()  { printf '\033[38;5;42m%-22s\033[0m %s\n' "$1" "$2"; }
bad() { printf '\033[38;5;203m%-22s\033[0m %s\n' "$1" "$2"; FAIL=1; }

say "project" "$ROOT"
say "folder name" "$NAME"

# `command grep` on purpose: the shell's grep skips ignored files and would
# hide exactly the build output this needs to see.
HITS=$(command grep -rn -e "$ROOT" -e '/home/' \
        "$ROOT/repo/src" "$ROOT/repo/Cargo.toml" "$ROOT/repo/scripts" 2>/dev/null | grep -v 'check-portable.sh' || true)
if [ -n "$HITS" ]; then bad "grep audit" "absolute path in source:"; echo "$HITS";
else ok "grep audit" "no absolute paths; app identity comes from manifest"; fi

BIN="$ROOT/tmp/target/release/$APP"
if [ ! -x "$BIN" ]; then bad "rename smoke test" "no release build at tmp/target/release"; else
  WORK="$ROOT/tmp/rename-check/$(date +%s)-wombat"
  mkdir -p "$WORK"
  cp "$BIN" "$WORK/"
  printf '%s\r\n' '# Text' '' '# Renamed' '' 'Running from a differently named directory.' '' '<span style="color:#22c55e">rendering-smoke</span>' '' '## **Formatted** heading' > "$WORK/doc.md"
  cp "$WORK/doc.md" "$WORK/doc.expected"
  # DISPLAY alone is not enough: with WAYLAND_DISPLAY set, GTK ignores the
  # virtual display and opens a real window on the desktop.
  OUT=$(cd "$WORK" && env -u WAYLAND_DISPLAY GDK_BACKEND=x11 XDG_SESSION_TYPE=x11 \
        EXIT_WHEN_READY=1 timeout 60 xvfb-run -a -s '-screen 0 1200x800x24' "./$APP" doc.md 2>&1 || true)
  if echo "$OUT" | grep -q READY_MS; then
    ok "rename smoke test" "ran from $(basename "$WORK") ($(echo "$OUT" | grep -o 'READY_MS=[0-9]*'))"
    cmp -s "$WORK/doc.md" "$WORK/doc.expected" && ok "save format" "unchanged document bytes preserved" || bad "save format" "document changed unexpectedly"
    [ -f "$WORK/settings.json" ] && ok "self-contained" "settings written beside the binary, not in \$HOME" \
      || bad "self-contained" "no settings.json beside the binary"
  else
    bad "rename smoke test" "did not start: $(echo "$OUT" | tail -1)"
  fi
  rm -rf "$WORK"
fi

[ $FAIL -eq 0 ] && ok "result" "portable" || bad "result" "not portable"
exit $FAIL
