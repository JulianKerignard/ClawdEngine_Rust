#!/bin/bash
set -euo pipefail

APP_NAME="ClawdEngine"
BUNDLE_ID="com.clawdengine.editor"
VERSION="0.1.0"
BUILD_DIR="target/export"
APP_DIR="$BUILD_DIR/$APP_NAME.app"

echo "=== ClawdEngine macOS Export ==="
echo ""

# 1. Build release binary
echo "[1/6] Building release binary..."
$HOME/.cargo/bin/cargo build --release 2>&1 | tail -1

# 2. Create .app bundle structure
echo "[2/6] Creating .app bundle..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# 3. Copy binary
cp target/release/clawdengine "$APP_DIR/Contents/MacOS/"
echo "  Binary copied"

# 4. Copy assets
if [ -d "assets" ]; then
    cp -R assets "$APP_DIR/Contents/Resources/"
    echo "  Assets copied"
else
    echo "  [warn] No assets/ directory found"
fi

# 5. Generate Info.plist
echo "[3/6] Generating Info.plist..."
cat > "$APP_DIR/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>clawdengine</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>${APP_NAME}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
</dict>
</plist>
PLIST
echo "  Info.plist generated"

# 6. Generate .icns icon (if source icon exists)
echo "[4/6] Generating icon..."
if [ -f "assets/icon.png" ]; then
    ICONSET="$BUILD_DIR/${APP_NAME}.iconset"
    mkdir -p "$ICONSET"

    # Standard icon sizes for macOS
    for SIZE in 16 32 64 128 256 512; do
        sips -z $SIZE $SIZE assets/icon.png --out "$ICONSET/icon_${SIZE}x${SIZE}.png" >/dev/null 2>&1
    done
    # Retina (@2x) sizes
    for SIZE in 32 64 256 512 1024; do
        HALF=$((SIZE / 2))
        sips -z $SIZE $SIZE assets/icon.png --out "$ICONSET/icon_${HALF}x${HALF}@2x.png" >/dev/null 2>&1
    done

    iconutil -c icns "$ICONSET" -o "$APP_DIR/Contents/Resources/${APP_NAME}.icns"
    rm -rf "$ICONSET"
    echo "  Icon generated from assets/icon.png"
else
    echo "  [skip] No assets/icon.png — using default icon"
fi

# 7. Code signing (ad-hoc)
echo "[5/6] Code signing (ad-hoc)..."
codesign --force -s - "$APP_DIR/Contents/MacOS/clawdengine" 2>/dev/null
codesign --force -s - "$APP_DIR" 2>/dev/null
echo "  Signed (ad-hoc)"

# 8. Summary
echo "[6/6] Done!"
echo ""
echo "=== Export complete ==="
echo "  Output: $APP_DIR"
echo "  Size:   $(du -sh "$APP_DIR" | cut -f1)"
echo ""
echo "To run: open $APP_DIR"
echo "To verify: codesign --verify -vv $APP_DIR"
