#!/usr/bin/env bash
# Build portable packages. Installation is explicit and never starts the app.
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
REPO="$ROOT/repo"
BIN=$(sed -n 's/^name = "\(.*\)"/\1/p' "$REPO/Cargo.toml" | head -1)
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO/Cargo.toml" | head -1)
WIN_DEST=${WINDOWS_DEST:-/mnt/c/work/$BIN}
TARGET=${1:-all}
INSTALL=${2:-}
case "$TARGET" in all|linux|windows) ;; *) echo "Usage: $0 [all|linux|windows] [--install]"; exit 2;; esac
[ -z "$INSTALL" ] || [ "$INSTALL" = --install ] || { echo "Unknown option: $INSTALL"; exit 2; }
export CARGO_TARGET_DIR="$ROOT/tmp/target"
export TMPDIR="$ROOT/tmp/build"
export XDG_CACHE_HOME="$ROOT/tmp/cache"
mkdir -p "$TMPDIR" "$XDG_CACHE_HOME"
A=$'\033[38;5;39m'; OK=$'\033[38;5;42m'; Z=$'\033[0m'
if [ ! -t 1 ] || [ -n "${NO_COLOR:-}" ]; then A=""; OK=""; Z=""; fi
row() { printf "${A}%-18s${Z} %s\n" "$1" "$2"; }
row project "$BIN $VER"
row targets "$TARGET"
row jobs "${BUILD_JOBS:-2}"
row output "$ROOT/deploy"
build() {
  local target=$1 label=$2 suffix=$3 started=$SECONDS
  local args=(--offline --locked --release -j "${BUILD_JOBS:-2}" --manifest-path "$REPO/Cargo.toml")
  [ -z "$target" ] || args+=(--target "$target")
  nice -n 15 cargo build "${args[@]}"
  local built="$CARGO_TARGET_DIR/${target:+$target/}release/$BIN$suffix"
  test -s "$built"
  mkdir -p "$ROOT/deploy/$label"
  cp "$built" "$ROOT/deploy/$label/$BIN$suffix"
  row "$label" "$(stat -c%s "$built") bytes; $((SECONDS-started))s"
}
if [ "$TARGET" != windows ]; then build "" linux ""; fi
if [ "$TARGET" != linux ]; then
  build x86_64-pc-windows-gnu windows .exe
  W2VER=$(grep -A1 'name = "webview2-com-sys"' "$REPO/Cargo.lock" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)
  W2DLL=$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -path "*/webview2-com-sys-$W2VER/x64/WebView2Loader.dll" -print -quit)
  test -n "$W2DLL" || { echo "Missing cached WebView2Loader.dll $W2VER"; exit 1; }
  cp "$W2DLL" "$ROOT/deploy/windows/WebView2Loader.dll"
  cp "$REPO/scripts/register-file-types.ps1" "$ROOT/deploy/windows/register-file-types.ps1"
  printf '{"name":"%s","version":"%s"}\n' "$BIN" "$VER" > "$ROOT/deploy/windows/installed.json"
fi
if [ "$INSTALL" = --install ]; then
  [ "$TARGET" != linux ] || { echo 'Installation requires a Windows build.'; exit 2; }
  mkdir -p "$WIN_DEST"
  ARCHIVE="$WIN_DEST/previous-$(date +%Y%m%d-%H%M%S)-$$"
  for file in "$BIN.exe" notepad.exe WebView2Loader.dll installed.json register-file-types.ps1; do
    if [ -f "$WIN_DEST/$file" ]; then
      mkdir -p "$ARCHIVE"
      cp -p "$WIN_DEST/$file" "$ARCHIVE/$file"
    fi
  done
  for file in "$BIN.exe" WebView2Loader.dll installed.json register-file-types.ps1; do
    cp "$ROOT/deploy/windows/$file" "$WIN_DEST/$file"
  done
  row installed "$WIN_DEST/$BIN.exe"
  row preserved "Settings and previous binaries retained."
fi
printf "${OK}Build complete.${Z} No application was started.\n"
row windows "$ROOT/deploy/windows/$BIN.exe"
row linux "$ROOT/deploy/linux/$BIN"
