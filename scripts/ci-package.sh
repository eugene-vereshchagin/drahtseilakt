#!/usr/bin/env bash
# CI: assembles the portable archive of one target, in the layout of scripts/make-portable.sh (program, toolbox,
# lib/ on Linux, realesrgan/, assets/, licences, VERSION, README.txt, portable.txt, an empty data/) and packs it:
# .zip on Windows, .tar.gz elsewhere, into dist/. Called from the workflows after the build and ci-assets.sh.
#
# Environment: OS_KIND (linux|windows|macos|freebsd), ARCH (x86_64|aarch64), DEV (1 = development flavour),
# TOOLBOX (1 = with the toolbox), VERSION (tag or commit), GAME_BIN, TOOL_BIN, TOOLBOX_DIR (its checkout).
# assets/ is MOVED into the package (it is large); run this once per job.
set -euo pipefail
die() { echo "ci-package: $*" >&2; echo "::error::ci-package: $*"; exit 1; }
cd "$(dirname "$0")/.."
here=$(pwd)
: "${OS_KIND:?}" "${ARCH:?}" "${VERSION:?}" "${GAME_BIN:?}"
DEV=${DEV:-0} TOOLBOX=${TOOLBOX:-1}
[ "$DEV" = 0 ] || [ "$OS_KIND" = linux ] || die "dev builds are Linux only"
sha_of() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1
           elif command -v shasum >/dev/null; then shasum -a 256 "$1" | cut -d' ' -f1
           else sha256 -q "$1"; fi; }

flavour=""; [ "$DEV" = 0 ] || flavour="-dev"
name="drahtseilakt-$OS_KIND-$ARCH$flavour"
out="dist/$name"
rm -rf "$out"; mkdir -p "$out"
exe=""; [ "$OS_KIND" != windows ] || exe=".exe"
[ -f "$GAME_BIN" ] || die "no game binary at $GAME_BIN"
[ "$TOOLBOX" = 0 ] || [ -f "${TOOL_BIN:?}" ] || die "no toolbox binary at $TOOL_BIN"

# --- programs and libraries ----------------------------------------------------------------------------------------
if [ "$OS_KIND" = linux ]; then
    install -m 755 "$GAME_BIN" "$out/drahtseilakt.bin"
    cat >"$out/drahtseilakt" <<'LAUNCHER'
#!/bin/sh
# Portable launcher: the game's own libraries first, the player's data in data/ beside it, nothing else touched.
here="$(cd "$(dirname "$0")" && pwd)"
LD_LIBRARY_PATH="$here/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" exec "$here/drahtseilakt.bin" "$@"
LAUNCHER
    chmod 755 "$out/drahtseilakt"
    mkdir -p "$out/lib/LICENSES"
    for lib in libasound.so.2 libudev.so.1 libcap.so.2; do
        path=$(ldd "$GAME_BIN" | awk -v l="$lib" '$1 == l { print $3 }')
        [ -n "$path" ] && [ -f "$path" ] || die "$lib: not found by ldd for the game"
        real=$(readlink -f "$path")
        install -m 644 "$real" "$out/lib/$lib"
        pkg=$(dpkg -S "$real" 2>/dev/null | head -n1 | cut -d: -f1) || pkg=""
        [ -n "$pkg" ] && [ -f "/usr/share/doc/$pkg/copyright" ] || die "$lib: no licence found (/usr/share/doc/<package>/copyright)"
        install -m 644 "/usr/share/doc/$pkg/copyright" "$out/lib/LICENSES/$lib.copyright"
        grep -o 'common-licenses/[A-Za-z0-9.-]*[A-Za-z0-9]' "/usr/share/doc/$pkg/copyright" | sort -u | while read -r ref; do
            if [ -f "/usr/share/$ref" ]; then install -m 644 "/usr/share/$ref" "$out/lib/LICENSES/${ref#common-licenses/}.txt"; fi
        done
    done
    [ "$TOOLBOX" = 0 ] || install -m 755 "$TOOL_BIN" "$out/drahtseilakt-toolbox"
else
    install -m 755 "$GAME_BIN" "$out/drahtseilakt$exe"
    [ "$TOOLBOX" = 0 ] || install -m 755 "$TOOL_BIN" "$out/drahtseilakt-toolbox$exe"
fi

# --- Real-ESRGAN for the toolbox (pinned download, checked), where the release has a build for the target -------------
esr_name="" esr_files=()
case "$OS_KIND-$ARCH" in
    linux-x86_64) esr_name=realesrgan-ncnn-vulkan; esr_files=(realesrgan-ncnn-vulkan 'models/*') ;;
    windows-x86_64) esr_name=realesrgan-ncnn-vulkan-windows; esr_files=(realesrgan-ncnn-vulkan.exe vcomp140.dll 'models/*') ;;
    macos-*) esr_name=realesrgan-ncnn-vulkan-macos; esr_files=(realesrgan-ncnn-vulkan 'models/*') ;;
esac
have_esr=0
if [ "$TOOLBOX" = 1 ] && [ -n "$esr_name" ]; then
    # the block of this name in the toolbox's third-party.txt or in ours
    field() { awk -v n="$esr_name" -v k="$1" '$1 == "name" { on = ($2 == n) } on && $1 == k { print $2; exit }' \
                  "${TOOLBOX_DIR:?}/third-party.txt" scripts/portable/realesrgan-extra.txt | head -n1; }
    url=$(field url) pinned=$(field sha256)
    [ -n "$url" ] && [ -n "$pinned" ] || die "no pin for $esr_name in third-party.txt or scripts/portable/realesrgan-extra.txt"
    zip="${RUNNER_TEMP:-/tmp}/realesrgan.zip"
    curl -fsSL -o "$zip" "$url" || die "Real-ESRGAN did not download"
    [ "$(sha_of "$zip")" = "$pinned" ] || die "the Real-ESRGAN download does not match its pinned SHA-256"
    mkdir -p "$out/realesrgan"
    unzip -q -o "$zip" "${esr_files[@]}" -d "$out/realesrgan"
    [ "$OS_KIND" = windows ] || chmod 755 "$out/realesrgan/realesrgan-ncnn-vulkan"
    install -m 644 scripts/portable/realesrgan-LICENSES.txt "$out/realesrgan/LICENSES.txt"
    rm -f "$zip"
    have_esr=1
fi

# --- assets: moved in; development-only things stay out of a release ---------------------------------------------------
[ -d assets ] || die "no assets/ (run scripts/ci-assets.sh first)"
rm -f assets/sounds/system/prepare.py
rm -rf assets/own/make
find assets -name __pycache__ -type d -prune -exec rm -rf {} + 2>/dev/null || true
find assets -name '*.pyc' -type f -delete
[ "$DEV" = 1 ] || rm -rf assets/mods-dev
mv assets "$out/assets"

# --- texts ---------------------------------------------------------------------------------------------------------------
for f in LICENSE CREDITS.md THIRD-PARTY.md; do
    [ -f "$f" ] && install -m 644 "$f" "$out/$f"
done
tool_ver="not included"
if [ "$TOOLBOX" = 1 ]; then tool_ver=$(git -C "$TOOLBOX_DIR" describe --always --dirty --tags 2>/dev/null || echo unknown); fi
printf 'drahtseilakt %s\ndrahtseilakt-toolbox %s\ntarget %s-%s\nbuild %s\n' "$VERSION" "$tool_ver" "$OS_KIND" "$ARCH" \
    "$([ "$DEV" = 1 ] && echo 'DEV (debug on, development mods)' || echo release)" >"$out/VERSION"
case $OS_KIND in
    windows) run_game="drahtseilakt.exe" run_tool="drahtseilakt-toolbox.exe"
             sysreq="Windows 10 or later with a Direct3D 12 or Vulkan driver" ;;
    macos) run_game="./drahtseilakt" run_tool="./drahtseilakt-toolbox"
           sysreq="macOS 12 or later (Metal). The program is not signed: the first start needs right-click > Open, or: xattr -dr com.apple.quarantine <this folder>" ;;
    freebsd) run_game="./drahtseilakt" run_tool="./drahtseilakt-toolbox"
             sysreq="FreeBSD with a Vulkan driver (EXPERIMENTAL build)" ;;
    *) run_game="./drahtseilakt" run_tool="./drahtseilakt-toolbox"
       sysreq="Linux: glibc and the GPU/audio drivers (Vulkan or OpenGL, ALSA/PipeWire); lib/ carries the sound, input and udev libraries (their licences in lib/LICENSES/)" ;;
esac
{
    echo "Drahtseilakt - portable copy ($OS_KIND $ARCH, $VERSION)$([ "$DEV" = 1 ] && echo ', DEBUG DEV BUILD')"
    if [ "$DEV" = 1 ]; then
        echo
        echo "A development build, not for players: a played game is a debug run (extended mode, all levels open) and the"
        echo "development mod god-mode (assets/mods-dev; Options > Extended > Mods) is there."
    fi
    echo
    echo "Play: run $run_game."
    if [ "$TOOLBOX" = 1 ]; then
        echo "The toolbox: run $run_tool (a window; '$run_tool --help' lists its commands). It converts and remasters levels:"
        echo "community maps (.nmo) into the open format, the original Ballance levels into remastered ones, the original's"
        echo "sound theme. It writes into data/ here, where the game finds the results."
        if [ "$have_esr" = 1 ]; then
            echo "Real-ESRGAN (realesrgan/, licences in realesrgan/LICENSES.txt) makes the larger textures."
        else
            echo "This package has no Real-ESRGAN build (none exists for $OS_KIND $ARCH): the toolbox cannot make larger textures here."
        fi
    fi
    echo
    echo "Everything stays in this folder: the game, the assets (assets/) and the player's data (data/: settings, replays,"
    echo "highscores, imported and converted levels, sound themes). portable.txt switches portable mode on: nothing outside"
    echo "this folder is read or written. Copy the whole folder to move it; data/ is yours."
    echo
    echo "The original Ballance levels need a copy of the game: import it from the menu (it is copied into data/)."
    echo "The 13 picked community maps in the open format (100-550 MB each, mostly 4K textures) are separate downloads,"
    echo "one .zip a map, in the release tagged data-<number> (docs/drahtseilakt/release.md): unpack each into"
    echo "assets/levels/drahtseilakt/ in this folder."
    echo
    echo "Needs: $sysreq."
    echo "Licence: GPL-3.0 (LICENSE); everything else in it and its licences: THIRD-PARTY.md; credits: CREDITS.md and"
    echo "assets/own/CREDITS.md. Versions: VERSION."
} >"$out/README.txt"
: >"$out/portable.txt"
mkdir -p "$out/data"

# --- pack ------------------------------------------------------------------------------------------------------------------
archive="drahtseilakt-$VERSION-$OS_KIND-$ARCH$flavour"
if [ "$OS_KIND" = windows ]; then
    (cd dist && 7z a -tzip -mx=5 -bso0 -bsp0 "$archive.zip" "$name")
    ls -l "dist/$archive.zip"
else
    # a tarball keeps the program executable
    tar -czf "dist/$archive.tar.gz" -C dist "$name"
    ls -l "dist/$archive.tar.gz"
fi
rm -rf "$out"
