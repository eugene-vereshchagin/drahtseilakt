#!/usr/bin/env bash
# CI (macOS runner): Drahtseilakt.app and a .dmg with it. The assets sit in Contents/Resources/assets, where the game
# looks (../Resources/assets beside the program). UNSIGNED and not notarized (no Apple developer account): the arm64
# linker's ad-hoc signature is all there is, so the first start needs right-click > Open (or xattr -dr
# com.apple.quarantine Drahtseilakt.app); the README of the release says so. Player data goes to the user's profile
# (~/Library/Application Support), as for any installed program.
#
# Environment: ARCH (x86_64|aarch64), VERSION (tag X.Y.Z or git-<sha>), GAME_BIN. Needs assets/ in the checkout
# (scripts/ci-assets.sh), python3. Output: dist/drahtseilakt-<VERSION>-macos-<ARCH>.dmg. assets/ is COPIED.
set -euo pipefail
die() { echo "ci-macos-app: $*" >&2; echo "::error::ci-macos-app: $*"; exit 1; }
cd "$(dirname "$0")/.."
: "${ARCH:?}" "${VERSION:?}" "${GAME_BIN:?}"
[ -f "$GAME_BIN" ] && [ -d assets/own ] || die "needs the game binary and assets/"
plist_version=$VERSION; case $VERSION in git-*) plist_version="0.0.0" ;; esac
w=$(mktemp -d); trap 'rm -rf "$w"' EXIT
app="$w/dmg/Drahtseilakt.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$w/icon.iconset"
install -m 755 "$GAME_BIN" "$app/Contents/MacOS/drahtseilakt"
# the shipped assets, without development-only things
tar -C assets --exclude=./mods-dev --exclude=./own/make --exclude=__pycache__ --exclude='*.pyc' -cf - . \
    | (mkdir -p "$app/Contents/Resources/assets" && tar -C "$app/Contents/Resources/assets" -xf -)
# the icon: the game's icon (scripts/packaging/icon-512.png) scaled to the iconset sizes
for s in 16 32 64 128 256 512 1024; do python3 -I tools/icon.py "$s" "$w/s$s.png" scripts/packaging/icon-512.png; done
for pair in 16:icon_16x16 32:icon_16x16@2x 32:icon_32x32 64:icon_32x32@2x 128:icon_128x128 256:icon_128x128@2x \
            256:icon_256x256 512:icon_256x256@2x 512:icon_512x512 1024:icon_512x512@2x; do
    cp "$w/s${pair%%:*}.png" "$w/icon.iconset/${pair#*:}.png"
done
iconutil -c icns "$w/icon.iconset" -o "$app/Contents/Resources/drahtseilakt.icns"
cat >"$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Drahtseilakt</string>
  <key>CFBundleDisplayName</key><string>Drahtseilakt</string>
  <key>CFBundleIdentifier</key><string>io.github.eugene-vereshchagin.drahtseilakt</string>
  <key>CFBundleExecutable</key><string>drahtseilakt</string>
  <key>CFBundleIconFile</key><string>drahtseilakt</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$plist_version</string>
  <key>CFBundleVersion</key><string>$plist_version</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.puzzle-games</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
for f in LICENSE CREDITS.md THIRD-PARTY.md; do [ -f "$f" ] && install -m 644 "$f" "$w/dmg/$f"; done
cat >"$w/dmg/README.txt" <<README
Drahtseilakt $VERSION (macOS $ARCH). Drag Drahtseilakt.app to Applications.
The app is not signed: the first start needs right-click > Open, or in Terminal:
  xattr -dr com.apple.quarantine /Applications/Drahtseilakt.app
Your settings, replays and imported levels are kept in ~/Library/Application Support, not in the app.
The 13 picked maps are separate downloads (one .zip each): unpack each into
Drahtseilakt.app/Contents/Resources/assets/levels/drahtseilakt/ .
README
ln -s /Applications "$w/dmg/Applications"
mkdir -p dist
hdiutil create -quiet -volname "Drahtseilakt" -srcfolder "$w/dmg" -ov -format UDZO "dist/drahtseilakt-$VERSION-macos-$ARCH.dmg"
ls -l "dist/drahtseilakt-$VERSION-macos-$ARCH.dmg"
