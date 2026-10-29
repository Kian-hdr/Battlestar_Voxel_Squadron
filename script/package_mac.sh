#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export MACOSX_DEPLOYMENT_TARGET=12.0
if [[ "${1:-}" != "--no-build" ]]; then cargo build --release --locked; fi
APP="dist/Battlestar_Voxel_Squadron.app"
# Preserve the existing bundle identity/bookmark when applying the game rename.
if [[ -d "dist/Battlestar_Voxel.app" && ! -e "$APP" ]]; then
  mv "dist/Battlestar_Voxel.app" "$APP"
fi
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/battlestar_voxel "$APP/Contents/MacOS/Battlestar_Voxel"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>Battlestar_Voxel</string>
<key>CFBundleIdentifier</key><string>dev.kiankonrad.battlestar-voxel</string>
<key>CFBundleName</key><string>Battlestar: Voxel Squadron</string>
<key>CFBundleDisplayName</key><string>Battlestar: Voxel Squadron</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.4.0</string>
<key>CFBundleVersion</key><string>13</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSHumanReadableCopyright</key><string>Copyright 2026 Kian Konrad Tajbakhsh. Unofficial non-commercial fan project.</string>
<key>CFBundleIconName</key><string>Battlestar_Voxel</string>
<key>CFBundleIconFile</key><string>Battlestar_Voxel</string>
<key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>
PLIST
cp assets/App_Icon/Compiled/Assets.car "$APP/Contents/Resources/Assets.car"
cp assets/App_Icon/Compiled/Battlestar_Voxel.icns "$APP/Contents/Resources/Battlestar_Voxel.icns"
cp LICENSE THIRD_PARTY_NOTICES.md "$APP/Contents/Resources/"
if [[ -d docs/Dependency_Licenses ]]; then
  ditto docs/Dependency_Licenses "$APP/Contents/Resources/Dependency_Licenses"
fi
cp assets/Audio/Public/NOTICE.txt "$APP/Contents/Resources/Audio_Public_Notice.txt"
cp assets/Fonts/PressStart2P_OFL.txt "$APP/Contents/Resources/PressStart2P_OFL.txt"
cp assets/Fonts/Silkscreen_OFL.txt "$APP/Contents/Resources/Silkscreen_OFL.txt"
cp assets/Audio/Licenses/Attribution.txt "$APP/Contents/Resources/Audio_Attribution.txt"
cp assets/Audio/Licenses/Weapon_Source_Notice.txt "$APP/Contents/Resources/Weapon_Source_Notice.txt"
cp assets/Audio/Licenses/Explosion_Source_Notice.txt "$APP/Contents/Resources/Explosion_Source_Notice.txt"
# Local builds are ad-hoc; public packaging explicitly selects Developer ID.
if [[ -n "${BSVX_SIGN_IDENTITY:-}" ]]; then
  codesign --force --options runtime --timestamp --sign "$BSVX_SIGN_IDENTITY" "$APP"
else
  codesign --force --sign - "$APP"
fi
plutil -lint "$APP/Contents/Info.plist"
codesign --verify --strict "$APP"
echo "$APP"
