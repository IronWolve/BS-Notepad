#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
export CARGO_TARGET_DIR="$ROOT/tmp/target" TMPDIR="$ROOT/tmp/build" XDG_CACHE_HOME="$ROOT/tmp/cache"
mkdir -p "$TMPDIR" "$XDG_CACHE_HOME"
command -v agent-browser >/dev/null || { echo 'Browser checks need agent-browser on PATH.'; exit 1; }
cargo test --release --offline --locked -j 2 --manifest-path "$ROOT/repo/Cargo.toml" export_browser_fixture
export AGENT_BROWSER_SESSION="notes-check-$$"
trap 'agent-browser close >/dev/null 2>&1 || true' EXIT
agent-browser open "file://$ROOT/tmp/browser-regressions/fixture.html"
agent-browser set viewport 1200 800
agent-browser eval --stdin < "$ROOT/repo/tests/ui-regressions.js"

agent-browser open "file://$ROOT/tmp/browser-regressions/fixture.html"
agent-browser eval --stdin < "$ROOT/repo/tests/image-regressions.js"
agent-browser open "file://$ROOT/tmp/browser-regressions/fixture.html"
agent-browser eval --stdin < "$ROOT/repo/tests/markdown-image-regressions.js"

agent-browser open "file://$ROOT/tmp/browser-regressions/fixture.html"
agent-browser eval --stdin < "$ROOT/repo/tests/sidebar-regressions.js"

agent-browser open "file://$ROOT/tmp/browser-regressions/fixture.html"
agent-browser eval --stdin < "$ROOT/repo/tests/appearance-regressions.js"
