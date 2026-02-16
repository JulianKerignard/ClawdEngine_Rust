#!/bin/bash
set -euo pipefail

# Export a ClawdEngine game project as a standalone macOS .app
# Usage: export-game.sh <project_path> <game_name> [scene_name]
#
# Example:
#   bash scripts/export-game.sh ~/ClawdEngine\ Projects/MyGame "My Game" main

PROJECT_PATH="${1:?Usage: export-game.sh <project_path> <game_name> [scene_name]}"
GAME_NAME="${2:?Usage: export-game.sh <project_path> <game_name> [scene_name]}"
SCENE_NAME="${3:-scene}"

# Resolve absolute path
PROJECT_PATH="$(cd "$PROJECT_PATH" && pwd)"

SAFE_NAME=$(echo "$GAME_NAME" | tr -d ' ')
BUILD_DIR="$PROJECT_PATH/export"
APP_DIR="$BUILD_DIR/$SAFE_NAME.app"
BUNDLE_ID="com.clawdengine.$(echo "$SAFE_NAME" | tr '[:upper:]' '[:lower:]')"

echo "=== ClawdEngine Game Export ==="
echo "  Project: $PROJECT_PATH"
echo "  Game:    $GAME_NAME"
echo "  Scene:   $SCENE_NAME"
echo ""

# 1. Build release binary
echo "[1/5] Building release binary..."
$HOME/.cargo/bin/cargo build --release 2>&1 | tail -3

# 2. Create .app bundle
echo "[2/5] Creating .app bundle..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
cp target/release/clawdengine "$APP_DIR/Contents/MacOS/"
echo "  Binary copied"

# 3. Copy project assets
echo "[3/5] Copying project assets..."
for DIR in scenes meshes textures audio; do
    if [ -d "$PROJECT_PATH/$DIR" ]; then
        cp -R "$PROJECT_PATH/$DIR" "$APP_DIR/Contents/Resources/"
        echo "  $DIR/ copied"
    fi
done

# Generate game.ron manifest
cat > "$APP_DIR/Contents/Resources/game.ron" << EOF
(
    name: "$GAME_NAME",
    startup_scene: "scenes/$SCENE_NAME.ron",
)
EOF
echo "  game.ron generated"

# 4. Generate Info.plist
echo "[4/5] Generating Info.plist..."
cat > "$APP_DIR/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${GAME_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${GAME_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleExecutable</key>
    <string>clawdengine</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
</dict>
</plist>
PLIST

# Generate icon if available
if [ -f "$PROJECT_PATH/icon.png" ]; then
    ICONSET="$BUILD_DIR/${SAFE_NAME}.iconset"
    mkdir -p "$ICONSET"
    for SIZE in 16 32 64 128 256 512; do
        sips -z $SIZE $SIZE "$PROJECT_PATH/icon.png" --out "$ICONSET/icon_${SIZE}x${SIZE}.png" >/dev/null 2>&1
    done
    for SIZE in 32 64 256 512 1024; do
        HALF=$((SIZE / 2))
        sips -z $SIZE $SIZE "$PROJECT_PATH/icon.png" --out "$ICONSET/icon_${HALF}x${HALF}@2x.png" >/dev/null 2>&1
    done
    iconutil -c icns "$ICONSET" -o "$APP_DIR/Contents/Resources/${SAFE_NAME}.icns"
    rm -rf "$ICONSET"
    echo "  Icon generated"
else
    echo "  [skip] No icon.png in project"
fi

# 5. Code signing
echo "[5/5] Code signing (ad-hoc)..."
codesign --force -s - "$APP_DIR/Contents/MacOS/clawdengine" 2>/dev/null
codesign --force -s - "$APP_DIR" 2>/dev/null
echo "  Signed"

# Summary
echo ""
echo "=== Export complete ==="
echo "  Output: $APP_DIR"
echo "  Size:   $(du -sh "$APP_DIR" | cut -f1)"
echo ""
echo "To run:    open $APP_DIR"
echo "To verify: codesign --verify -vv $APP_DIR"
