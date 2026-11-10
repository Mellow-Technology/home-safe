#!/usr/bin/env bash
# Builds "Home Safe.app" (a universal binary for Apple silicon and Intel) and
# packs it into Home-Safe-macos.dmg and Home-Safe-macos.zip, with the
# extension in Home-Safe-extension.zip, in target/release-macos/. The names
# carry no version, so releases/latest/download/<name> links stay stable.
#
#   scripts/bundle-macos.sh             # both architectures
#   ARCHS=aarch64 scripts/bundle-macos.sh   # just this Mac's, for a quick try
#
# Signing: with SIGN_IDENTITY set (a "Developer ID Application: …" identity in
# the keychain) the app is signed with the hardened runtime; otherwise it gets
# an ad-hoc signature, which runs locally but which Gatekeeper warns about
# when the app is downloaded. Notarizing is left to the caller (see
# .github/workflows/release.yml).
set -euo pipefail

cd "$(dirname "$0")/.."

ARCHS=${ARCHS:-"aarch64 x86_64"}
SIGN_IDENTITY=${SIGN_IDENTITY:-}
export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-12.0}

NAME="Home Safe"
EXE=home-safe
VERSION=$(cargo metadata --no-deps --format-version 1 |
  python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="home-safe"))')

OUT=target/release-macos
APP="$OUT/$NAME.app"
rm -rf "$OUT"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

bins=()
for arch in $ARCHS; do
  target="$arch-apple-darwin"
  cargo build --release --locked -p home-safe --target "$target"
  bins+=("target/$target/release/$EXE")
done
lipo -create -output "$APP/Contents/MacOS/$EXE" "${bins[@]}"
lipo -info "$APP/Contents/MacOS/$EXE"

cp crates/app/assets/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

cat >"$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>$NAME</string>
    <key>CFBundleExecutable</key>
    <string>$EXE</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>com.mellow.HomeSafe</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>$NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.lifestyle</string>
    <key>LSMinimumSystemVersion</key>
    <string>$MACOSX_DEPLOYMENT_TARGET</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>© Mellow Technology. Licensed under the LGPL-3.0.</string>
</dict>
</plist>
EOF
plutil -lint "$APP/Contents/Info.plist"

if [[ -n "$SIGN_IDENTITY" ]]; then
  codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$APP"
else
  codesign --force --sign - "$APP"
fi
codesign --verify --strict --verbose=2 "$APP"

# A disk image with a shortcut to Applications to drag the app onto.
stage="$OUT/dmg"
mkdir -p "$stage"
cp -R "$APP" "$stage/"
ln -s /Applications "$stage/Applications"
DMG="$OUT/Home-Safe-macos.dmg"
hdiutil create -volname "$NAME" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$DMG"
rm -rf "$stage"
if [[ -n "$SIGN_IDENTITY" ]]; then
  codesign --force --timestamp --sign "$SIGN_IDENTITY" "$DMG"
fi

ditto -c -k --keepParent "$APP" "$OUT/Home-Safe-macos.zip"

# The extension, to load unpacked (it has no build step).
(cd extension && zip -qr "../$OUT/Home-Safe-extension.zip" manifest.json src icons)

echo "Built $VERSION:"
ls -lh "$OUT"
