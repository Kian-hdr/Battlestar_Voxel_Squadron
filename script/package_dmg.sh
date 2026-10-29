#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${BSVX_SIGN_IDENTITY:?Set your Developer ID Application identity}"
APP="$PWD/dist/Battlestar_Voxel_Squadron.app"
DMG="$PWD/dist/Battlestar_Voxel_Squadron_v0.4.0_macOS_Apple_Silicon.dmg"
[[ -d "$APP" ]] || { echo 'Package and notarize the app first.' >&2; exit 1; }
codesign --verify --strict "$APP"
xcrun stapler validate "$APP"
STAGE=$(mktemp -d /private/tmp/bsvx-dmg.XXXXXX)
trap 'rm -rf "$STAGE"' EXIT
ditto "$APP" "$STAGE/Battlestar_Voxel_Squadron.app"
ln -s /Applications "$STAGE/Applications"
cp LICENSE THIRD_PARTY_NOTICES.md "$STAGE/"
cat > "$STAGE/Read_Me.txt" <<'EOF'
Battlestar: Voxel Squadron v0.4.0
Created by Kian Konrad Tajbakhsh

Drag Battlestar_Voxel_Squadron.app to Applications and launch it.
Apple Silicon Mac, macOS12 or newer; Metal-capable graphics.
Enter starts, A/D or arrows move, hold Space to fire, Escape pauses.
Effects volume and mute are available in the menu. F11 toggles fullscreen.

Unofficial noncommercial fan prototype. Third-party franchise rights remain
with their owners. Original source is MIT; third-party assets are separate.
https://github.com/Kian-hdr/Battlestar_Voxel_Squadron
EOF
hdiutil create -volname 'Voxel Squadron' -srcfolder "$STAGE" -format UDZO -ov "$DMG"
codesign --force --timestamp --sign "$BSVX_SIGN_IDENTITY" "$DMG"
echo "$DMG"
