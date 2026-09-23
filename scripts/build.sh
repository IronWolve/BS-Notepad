#!/usr/bin/env bash
# Builds both targets on this host and installs the Windows binary to
# C:\work\<project>\. Everything derives from the project folder name, so a
# rename moves the install with it.
set -u
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
NAME=$(basename "$ROOT")
REPO="$ROOT/repo"
BIN=$(sed -n 's/^name = "\(.*\)"/\1/p' "$REPO/Cargo.toml" | head -1)
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO/Cargo.toml" | head -1)
WIN_DEST="/mnt/c/work/$NAME"

A=$'\033[38;5;39m'; OK=$'\033[38;5;42m'; WARN=$'\033[38;5;214m'; ERR=$'\033[38;5;203m'
L=$'\033[38;5;245m'; V=$'\033[38;5;255m'; D=$'\033[2m'; Z=$'\033[0m'
[ -t 1 ] || { A=""; OK=""; WARN=""; ERR=""; L=""; V=""; D=""; Z=""; }
row() { printf "${L}%-18s${Z} ${V}%s${Z}\n" "$1" "$2"; }
mb()  { awk -v b="$(stat -c%s "$1" 2>/dev/null || echo 0)" 'BEGIN{printf "%.1f MB", b/1048576}'; }

printf "${A}%s${Z} ${D}%s${Z}\n" "build" "$NAME"
row "project" "$ROOT"
row "binary" "$BIN $VER"
row "windows dest" "$WIN_DEST"
echo

fail=0
build() { # label target outpath
  local label=$1 target=$2 out=$3 t0 dt
  t0=$SECONDS
  if [ -n "$target" ]; then (cd "$REPO" && nice -n 15 cargo build --release -j 6 --target "$target" -q)
  else (cd "$REPO" && nice -n 15 cargo build --release -j 6 -q); fi
  dt=$((SECONDS - t0))
  if [ -f "$out" ]; then printf "${OK}%-18s${Z} ${V}%s${Z} ${D}in %ss${Z}\n" "$label" "$(mb "$out")" "$dt"
  else printf "${ERR}%-18s${Z} %s\n" "$label" "build produced nothing"; fail=1; fi
}

build "linux" "" "$ROOT/tmp/target/release/$BIN"
build "windows" "x86_64-pc-windows-gnu" "$ROOT/tmp/target/x86_64-pc-windows-gnu/release/$BIN.exe"
[ $fail -eq 0 ] || exit 1
echo

# Copy named files only. Never a mirror: a delete-based sync would wipe
# whatever the user keeps in these folders.
mkdir -p "$ROOT/deploy/linux" "$ROOT/deploy/windows"
cp "$ROOT/tmp/target/release/$BIN" "$ROOT/deploy/linux/$BIN"
cp "$ROOT/tmp/target/x86_64-pc-windows-gnu/release/$BIN.exe" "$ROOT/deploy/windows/$BIN.exe"
row "deploy/linux" "$BIN"
row "deploy/windows" "$BIN.exe"

# The mingw build links the WebView2 loader dynamically, so the DLL has to
# travel with the exe. Version comes from the lock file, never hardcoded.
W2VER=$(grep -A1 'name = "webview2-com-sys"' "$REPO/Cargo.lock" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)
W2DLL=$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" \
          -path "*webview2-com-sys-$W2VER/x64/WebView2Loader.dll" 2>/dev/null | head -1)
if [ -n "$W2DLL" ]; then
  cp "$W2DLL" "$ROOT/deploy/windows/WebView2Loader.dll"
  row "deploy/windows" "WebView2Loader.dll ($W2VER)"
else
  printf "${ERR}%-18s${Z} %s\n" "missing dll" "WebView2Loader.dll for $W2VER not found"; exit 1
fi

if [ -d /mnt/c/work ]; then
  mkdir -p "$WIN_DEST"
  # An existing build is moved into a version folder INSIDE the install dir.
  # Never a sibling directory, and never deleted.
  if [ -f "$WIN_DEST/$BIN.exe" ]; then
    OLD=$(sed -n 's/.*"version"[: ]*"\([^"]*\)".*/\1/p' "$WIN_DEST/installed.json" 2>/dev/null | head -1)
    [ -n "$OLD" ] || OLD="unknown-$(date +%Y%m%d-%H%M%S)"
    ARCHIVE="$WIN_DEST/$BIN-$OLD"
    mkdir -p "$ARCHIVE"
    mv -f "$WIN_DEST/$BIN.exe" "$ARCHIVE/$BIN.exe"
    [ -f "$WIN_DEST/WebView2Loader.dll" ] && mv -f "$WIN_DEST/WebView2Loader.dll" "$ARCHIVE/WebView2Loader.dll"
    [ -f "$WIN_DEST/installed.json" ] && mv -f "$WIN_DEST/installed.json" "$ARCHIVE/installed.json"
    if [ -f "$ARCHIVE/$BIN.exe" ]; then
      printf "${D}%-18s %s${Z}\n" "kept previous" "$BIN-$OLD\\ (inside the install dir)"
    else
      printf "${ERR}%-18s${Z} %s\n" "archive failed" "$ARCHIVE"; exit 1
    fi
  fi
  cp "$ROOT/deploy/windows/$BIN.exe" "$WIN_DEST/$BIN.exe"
  cp "$ROOT/deploy/windows/WebView2Loader.dll" "$WIN_DEST/WebView2Loader.dll"
  cp "$REPO/scripts/register-file-types.ps1" "$WIN_DEST/register-file-types.ps1"
  printf '{"name":"%s","version":"%s","installed":"%s"}\n' \
    "$BIN" "$VER" "$(date -Is)" > "$WIN_DEST/installed.json"
  # Say it only if it is true.
  if [ -f "$WIN_DEST/$BIN.exe" ] && [ -f "$WIN_DEST/WebView2Loader.dll" ]; then
    printf "${OK}%-18s${Z} ${V}%s${Z}\n" "installed" "C:\\work\\$NAME\\$BIN.exe ($VER)"
  else
    printf "${ERR}%-18s${Z} %s\n" "install failed" "files missing at $WIN_DEST"; exit 1
  fi
else
  printf "${WARN}%-18s${Z} %s\n" "skipped install" "/mnt/c/work not present"
fi
echo
printf "${OK}%-18s${Z} ${V}%s${Z}\n" "done" "run it: see below"
printf "${D}  linux   %s/deploy/linux/%s <file.md>${Z}\n" "$ROOT" "$BIN"
printf "${D}  windows C:\\\\work\\\\%s\\\\%s.exe <file.md>${Z}\n" "$NAME" "$BIN"
