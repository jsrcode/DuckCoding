#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <path-to-app> <path-to-output-dmg> [volume-name]"
  exit 1
fi

APP_PATH="$1"
DMG_PATH="$2"
VOL_NAME="${3:-DuckCoding}"

if [[ ! -d "$APP_PATH" ]]; then
  echo "App bundle not found: $APP_PATH"
  exit 1
fi

DMG_DIR="$(dirname "$DMG_PATH")"
mkdir -p "$DMG_DIR"

echo "Creating DMG $DMG_PATH from $APP_PATH ..."
STAGING_DIR="$(mktemp -d)"
trap 'rm -rf "$STAGING_DIR"' EXIT

APP_NAME="$(basename "$APP_PATH")"
ditto "$APP_PATH" "$STAGING_DIR/$APP_NAME"

# Add Applications symlink for drag-and-drop installs
ln -s /Applications "$STAGING_DIR/Applications"

hdiutil create -volname "$VOL_NAME" -fs HFS+ -srcfolder "$STAGING_DIR" -ov -format UDZO "$DMG_PATH"
echo "DMG created."
