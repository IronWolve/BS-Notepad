#!/usr/bin/env bash
# Native Mac packaging. --setup permits downloading the already-declared crates.
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
REPO="$ROOT/repo"
[ "$(uname -s)" = Darwin ] || { echo 'Run this script on the Mac.'; exit 1; }
[ "${1:-}" = '' ] || [ "$1" = --setup ] || { echo 'Usage: build-macos.sh [--setup]'; exit 2; }
BIN=$(sed -n 's/^name = "\([^"]*\)"/\1/p' "$REPO/Cargo.toml" | head -1)
VER=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$REPO/Cargo.toml" | head -1)
NAME=$(sed -n 's/^display-name = "\([^"]*\)"/\1/p' "$REPO/Cargo.toml")
export CARGO_HOME="$ROOT/tmp/cargo" RUSTUP_HOME="$ROOT/tmp/rustup" CARGO_TARGET_DIR="$ROOT/tmp/target" TMPDIR="$ROOT/tmp/build"
export PATH="$CARGO_HOME/bin:$PATH" MACOSX_DEPLOYMENT_TARGET=14.0
mkdir -p "$TMPDIR" "$ROOT/deploy/macos"
command -v cargo >/dev/null || { echo 'A project-local Rust 1.95 toolchain is required.'; exit 1; }
ARGS=(--release --locked -j "${BUILD_JOBS:-2}" --manifest-path "$REPO/Cargo.toml")
[ "${1:-}" = --setup ] || ARGS+=(--offline)
printf 'Building %s %s for %s\n' "$NAME" "$VER" "$(uname -m)"
nice -n 15 cargo build "${ARGS[@]}"
STAGE="$ROOT/tmp/macos-package-$$"
mkdir -p "$STAGE/$NAME.app/Contents/MacOS" "$STAGE/$NAME.app/Contents/Resources" "$STAGE/icon.iconset"
APP="$STAGE/$NAME.app"
cp "$CARGO_TARGET_DIR/release/$BIN" "$APP/Contents/MacOS/$BIN"
rustc --edition=2021 -O "$REPO/tools/export-icon.rs" -o "$STAGE/export-icon"
"$STAGE/export-icon" "$STAGE/icon.png"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$STAGE/icon.png" --out "$STAGE/icon.iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" "$STAGE/icon.png" --out "$STAGE/icon.iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$STAGE/icon.iconset" -o "$APP/Contents/Resources/AppIcon.icns"
cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>$NAME</string>
<key>CFBundleDisplayName</key><string>$NAME</string>
<key>CFBundleIdentifier</key><string>com.ironwolve.$BIN</string>
<key>CFBundleExecutable</key><string>$BIN</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$VER</string>
<key>CFBundleVersion</key><string>$VER</string>
<key>CFBundleIconFile</key><string>AppIcon</string>
<key>LSMinimumSystemVersion</key><string>14.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSPrincipalClass</key><string>NSApplication</string>
<key>CFBundleDocumentTypes</key><array><dict>
<key>CFBundleTypeName</key><string>Text document</string>
<key>CFBundleTypeRole</key><string>Editor</string>
<key>LSHandlerRank</key><string>Alternate</string>
<key>LSItemContentTypes</key><array><string>public.text</string><string>net.daringfireball.markdown</string><string>public.json</string></array>
<key>CFBundleTypeExtensions</key><array><string>md</string><string>markdown</string><string>txt</string><string>json</string><string>yaml</string><string>toml</string><string>rs</string><string>py</string><string>js</string></array>
</dict></array>
</dict></plist>
EOF
plutil -lint "$APP/Contents/Info.plist"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
file "$APP/Contents/MacOS/$BIN"
otool -L "$APP/Contents/MacOS/$BIN"
DEST="$ROOT/deploy/macos/$NAME.app"
if [ -d "$DEST" ]; then mv "$DEST" "$ROOT/deploy/macos/$NAME-$VER-previous-$(date +%Y%m%d-%H%M%S).app"; fi
mv "$APP" "$DEST"
ARCHIVE="$ROOT/deploy/macos/$BIN-$VER-$(uname -m).zip"
ditto -c -k --sequesterRsrc --keepParent "$DEST" "$ARCHIVE"
printf '%s\n' "$NAME $VER" "Keep the app in a writable folder. Its private data is stored in $BIN-data beside the app, outside the signed bundle." "This build is ad-hoc signed for local use; it is not notarized for public distribution." > "$ROOT/deploy/macos/Read Me.txt"
shasum -a 256 "$ARCHIVE"
printf 'App: %s\nArchive: %s\nNo application was started.\n' "$DEST" "$ARCHIVE"
