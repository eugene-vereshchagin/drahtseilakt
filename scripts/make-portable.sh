#!/usr/bin/env bash
# Builds the game and the toolbox in release and assembles a portable copy of both: the folder a player unpacks and
# plays from, with nothing outside it touched (docs/drahtseilakt/files.md, "Portable"). The build is offline.
#
#   scripts/make-portable.sh [options] [OUT]
#
#   --target T            linux (x86_64-unknown-linux-gnu, the default) or windows (x86_64-pc-windows-gnu, cross-built
#                         with MinGW); macOS and others are to be added here as further cases of `target_case`
#   --toolbox DIR         the toolbox's checkout (default: drahtseilakt-toolbox beside the game's checkout)
#   --assets DIR          the shipped assets to copy (default: assets/ of the game's checkout; the generated files in
#                         it, levels, textures and the manual, are local only, so a fresh clone lacks them)
#   --dev                 a development build (Linux only): with the development mods (assets/mods-dev: god-mode), marked
#                         DEV in its folder name, VERSION and README.txt; the default is a release build, without them
#   --no-toolbox          leave the toolbox out (e.g. when its crates for the target are not in the cargo cache:
#                         a toolbox for Windows needs `cargo fetch` in the toolbox's checkout once, with the network)
#   --no-build            assemble from the binaries built before (target/<triple>/release of both repositories)
#   --with-originals DIR  import the original levels from a copy of Ballance in DIR into the portable data/, as the
#                         game's own import does (`--import`); only for the owner's private test builds: the result
#                         carries the original game and must never be published. Never the default.
#
# OUT defaults to drahtseilakt-portable/<os>-<arch>/ beside the game's checkout (linux-x86_64, windows-x86_64), and
# drahtseilakt-portable/<os>-<arch>-dev/ for --dev.
# An OUT made before is brought up to date: the program, its libraries, assets and texts are replaced, the player's
# data/ (and anything else the script did not put there) is kept. Any other non-empty folder is refused.
set -euo pipefail

die() { echo "make-portable: $*" >&2; exit 1; }
say() { echo "make-portable: $*"; }

here=$(cd "$(dirname "$0")/.." && pwd)
# the main checkout (a worktree's common git folder lies in it): where the toolbox and the portable folders sit beside
main=$(cd "$(git -C "$here" rev-parse --path-format=absolute --git-common-dir)/.." && pwd)
beside=$(dirname "$main")

target=linux toolbox="$beside/drahtseilakt-toolbox" with_toolbox=1 dev=0 assets="" build=1 originals="" out=""
while [ $# -gt 0 ]; do
    case $1 in
        --target) target=${2:?--target needs a value}; shift 2 ;;
        --toolbox) toolbox=${2:?--toolbox needs a folder}; shift 2 ;;
        --assets) assets=${2:?--assets needs a folder}; shift 2 ;;
        --no-build) build=0; shift ;;
        --no-toolbox) with_toolbox=0; shift ;;
        --dev) dev=1; shift ;;
        --with-originals) originals=${2:?--with-originals needs a folder}; shift 2 ;;
        -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d; s/^# \{0,1\}//'; exit 0 ;;
        -*) die "unknown option $1" ;;
        *) [ -z "$out" ] || die "one output folder only"; out=$1; shift ;;
    esac
done

# One case per target: the Rust triple, the folder name, the program's file suffix.
target_case() {
    case $1 in
        linux|x86_64-unknown-linux-gnu) triple=x86_64-unknown-linux-gnu os=linux exe="" ;;
        windows|x86_64-pc-windows-gnu) triple=x86_64-pc-windows-gnu os=windows exe=.exe ;;
        # TODO macOS (aarch64/x86_64-apple-darwin, an .app bundle with assets in Resources/) and the BSDs
        *) die "unknown target $1 (linux or windows)" ;;
    esac
    arch=${triple%%-*}
}
target_case "$target"
[ -n "$assets" ] || assets="$here/assets"
[ "$dev" = 0 ] || [ "$os" = linux ] || die "--dev builds are for Linux only; $os gets release builds"
kind=release; [ "$dev" = 0 ] || kind=dev
[ -n "$out" ] || out="$beside/drahtseilakt-portable/$os-$arch$([ "$dev" = 0 ] || echo -dev)"
[ "$with_toolbox" = 0 ] || [ -d "$toolbox" ] || die "no toolbox checkout at $toolbox (--toolbox DIR)"

# --- what ships is complete: the generated assets exist and match their checksums -------------------------------
[ -f "$assets/manual/index.html" ] || die "$assets has no manual (tools/make_manual.py); is it a fresh clone? (--assets DIR)"
[ -f "$assets/own/3D Entities/MenuLevel/level.bin" ] || die "$assets lacks the generated parts (assets/own/make)"
for sums in "$assets"/levels/*/SHA256SUMS "$assets/own/SHA256SUMS" "$assets/fonts/SHA256SUMS"; do
    [ -f "$sums" ] || die "missing $sums"
    (cd "$(dirname "$sums")" && sha256sum --check --quiet --strict SHA256SUMS) || die "$sums does not match its files"
done
for lvl in "$assets"/levels/*/*/; do
    [ -f "$lvl/level.txt" ] || continue
    if [ -f "$lvl/level.gltf" ] && [ ! -f "$lvl/level.bin" ]; then die "$lvl has level.gltf without level.bin"; fi
done

# --- build ------------------------------------------------------------------------------------------------------
if [ "$build" = 1 ]; then
    if [ "$os" = windows ]; then
        command -v x86_64-w64-mingw32-gcc >/dev/null || die "the windows target needs x86_64-w64-mingw32-gcc (MinGW)"
    fi
    say "building the game ($triple)"
    (cd "$here" && VXMATH_FONT_DIR="$here/assets/fonts" cargo build --release --offline --target "$triple" -p drahtseilakt)
    if [ "$with_toolbox" = 1 ]; then
        say "building the toolbox ($triple)"
        (cd "$toolbox" && cargo build --release --offline --target "$triple") \
            || die "the toolbox did not build for $triple (crates missing offline? run 'cargo fetch' in $toolbox once, or --no-toolbox)"
    fi
fi
game_bin="${CARGO_TARGET_DIR:-$here/target}/$triple/release/drahtseilakt$exe"
tool_bin="$toolbox/target/$triple/release/drahtseilakt-toolbox$exe"
[ -f "$game_bin" ] || die "no game binary at $game_bin"
[ "$with_toolbox" = 0 ] || [ -f "$tool_bin" ] || die "no toolbox binary at $tool_bin"

# --- the output folder: new, empty, or a portable made before (its data/ is kept) --------------------------------
if [ -e "$out" ]; then
    [ -d "$out" ] || die "$out is not a folder"
    if [ -n "$(ls -A "$out")" ] && [ ! -f "$out/portable.txt" ]; then
        die "$out is not empty and not a portable copy; refusing to write into it"
    fi
fi
mkdir -p "$out"
out=$(cd "$out" && pwd)
case $out/ in "$here"/*|"$assets"/*|"$toolbox"/*) die "$out lies inside a source folder" ;; esac
# everything the script puts there, removed first so that an update leaves nothing stale; data/ is never in this list
shipped=(drahtseilakt drahtseilakt.bin drahtseilakt.exe drahtseilakt-toolbox drahtseilakt-toolbox.exe lib assets
         realesrgan README.txt LICENSE CREDITS.md THIRD-PARTY.md VERSION portable.txt)
for f in "${shipped[@]}"; do rm -rf "${out:?}/$f"; done
find "$out" -maxdepth 1 -name '*.dll' -type f -delete

# --- programs and the libraries they need -------------------------------------------------------------------------
case $os in
    linux)
        install -m 755 "$game_bin" "$out/drahtseilakt.bin"
        cat >"$out/drahtseilakt" <<'EOF'
#!/bin/sh
# Portable launcher: the game's own libraries first, the player's data in data/ beside it, nothing else touched.
here="$(cd "$(dirname "$0")" && pwd)"
LD_LIBRARY_PATH="$here/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" exec "$here/drahtseilakt.bin" "$@"
EOF
        chmod 755 "$out/drahtseilakt"
        # sound, input and udev: not on every desktop (glibc and the GPU/audio drivers are the system's). Taken from
        # where this machine resolves them, links followed.
        mkdir -p "$out/lib"
        for lib in libasound.so.2 libudev.so.1 libcap.so.2; do
            path=$(ldd "$game_bin" | awk -v l="$lib" '$1 == l { print $3 }')
            [ -n "$path" ] && [ -f "$path" ] || die "$lib: not found by ldd for the game"
            real=$(readlink -f "$path")
            install -m 644 "$real" "$out/lib/$lib"
            # its licence: the distribution's copyright file of the package that holds it, and the licence texts that
            # file refers to (LGPL-2.1 for libasound and libudev, BSD-3 / GPL-2 for libcap)
            pkg=$(dpkg -S "$real" 2>/dev/null | head -n1 | cut -d: -f1) || pkg=""
            [ -n "$pkg" ] && [ -f "/usr/share/doc/$pkg/copyright" ] || die "$lib: no licence found (/usr/share/doc/<package>/copyright)"
            mkdir -p "$out/lib/LICENSES"
            install -m 644 "/usr/share/doc/$pkg/copyright" "$out/lib/LICENSES/$lib.copyright"
            grep -o 'common-licenses/[A-Za-z0-9.-]*[A-Za-z0-9]' "/usr/share/doc/$pkg/copyright" | sort -u | while read -r ref; do
                if [ -f "/usr/share/$ref" ]; then install -m 644 "/usr/share/$ref" "$out/lib/LICENSES/${ref#common-licenses/}.txt"; fi
            done
        done
        # the toolbox links glibc only (its window libraries are loaded at run time, as the game's): no launcher
        if [ "$with_toolbox" = 1 ]; then install -m 755 "$tool_bin" "$out/drahtseilakt-toolbox"; fi
        ;;
    windows)
        install -m 755 "$game_bin" "$out/drahtseilakt.exe"
        bins=("$game_bin")
        if [ "$with_toolbox" = 1 ]; then install -m 755 "$tool_bin" "$out/drahtseilakt-toolbox.exe"; bins+=("$tool_bin"); fi
        # the MinGW runtime DLLs a program imports (system DLLs come with Windows): copied beside it
        for bin in "${bins[@]}"; do
            x86_64-w64-mingw32-objdump -p "$bin" | awk '/DLL Name:/ { print $3 }'
        done | sort -u | while read -r dll; do
            for dir in /usr/x86_64-w64-mingw32/lib /usr/lib/gcc/x86_64-w64-mingw32/*-posix; do
                if [ -f "$dir/$dll" ]; then install -m 644 "$dir/$dll" "$out/$dll"; say "with $dll"; break; fi
            done
        done
        ;;
esac

# --- Real-ESRGAN for the toolbox, from the pinned download, with its licences -------------------------------------
zip="$toolbox/third_party/download/realesrgan.zip"
pinned=$(awk '$1 == "sha256" { print $2; exit }' "$toolbox/third-party.txt")
if [ "$with_toolbox" = 0 ]; then
    :
elif [ "$os" = linux ] && [ -f "$zip" ] && [ -n "$pinned" ]; then
    echo "$pinned  $zip" | sha256sum --check --quiet --strict || die "$zip does not match third-party.txt"
    mkdir -p "$out/realesrgan"
    # the program and the models that come with it; the demo pictures and video stay out
    unzip -q -o "$zip" realesrgan-ncnn-vulkan 'models/*' -d "$out/realesrgan"
    chmod 755 "$out/realesrgan/realesrgan-ncnn-vulkan"
    install -m 644 "$here/scripts/portable/realesrgan-LICENSES.txt" "$out/realesrgan/LICENSES.txt"
else
    # TODO windows: pin realesrgan-ncnn-vulkan-20220424-windows.zip in the toolbox's third-party.txt (URL, SHA-256)
    # and unpack it here; until then the toolbox there enlarges textures with Lanczos only
    say "no Real-ESRGAN for $os (not pinned or not downloaded): the toolbox enlarges textures with Lanczos"
fi

# --- assets: everything that ships; development-only things stay out -----------------------------------------------
# mods-dev (god-mode, --debug only; only in --dev builds), the scripts that make our parts (own/make), Python caches
say "copying assets from $assets"
mkdir -p "$out/assets"
devmods=(--exclude=./mods-dev); [ "$dev" = 0 ] || devmods=()
tar -C "$assets" "${devmods[@]}" --exclude=./own/make --exclude=__pycache__ --exclude='*.pyc' -cf - . \
    | tar -C "$out/assets" -xf -

# --- texts ----------------------------------------------------------------------------------------------------------
for f in LICENSE CREDITS.md THIRD-PARTY.md; do
    if [ -f "$here/$f" ]; then install -m 644 "$here/$f" "$out/$f"; fi
done
describe() { git -C "$1" describe --always --dirty --tags 2>/dev/null || echo unknown; }
printf 'drahtseilakt %s\ndrahtseilakt-toolbox %s\ntarget %s\nbuild %s\n' "$(describe "$here")" "$(describe "$toolbox" | sed "s/^/$([ "$with_toolbox" = 1 ] || echo 'not included, ')/")" \
    "$triple" "$([ "$dev" = 1 ] && echo 'DEV (debug on, development mods)' || echo release)" >"$out/VERSION"
if [ "$os" = windows ]; then
    run_game="drahtseilakt.exe" run_tool="drahtseilakt-toolbox.exe" sysreq="Windows 10 or later (64-bit) with a \
Direct3D 12 or Vulkan driver"
else
    run_game="./drahtseilakt" run_tool="./drahtseilakt-toolbox" sysreq="Linux x86-64: glibc and the GPU/audio \
drivers (Vulkan or OpenGL, ALSA/PipeWire); lib/ carries the sound, input and udev libraries (their licences in lib/LICENSES/)"
fi
devnote=""
if [ "$dev" = 1 ]; then
    devnote="A development build: a played game is a debug run (extended mode, all levels open), and the development
mod god-mode (assets/mods-dev; Options > Extended > Mods) is there. Not for players.
"
fi
cat >"$out/README.txt" <<EOF
Drahtseilakt — portable copy ($os $arch)$([ "$dev" = 1 ] && printf '%s' ", DEV BUILD")
${devnote}
Play: run $run_game.$( [ "$with_toolbox" = 1 ] && printf '%s' " The toolbox: run $run_tool (a window; '$run_tool --help' lists its commands).")

Everything stays in this folder: the game$( [ "$with_toolbox" = 1 ] && printf ', the toolbox'), the assets (assets/) and the player's data (data/:
settings, replays, highscores, imported and converted levels, sound themes). portable.txt switches portable mode
on: nothing outside this folder is read or written. Copy the whole folder to move it; data/ is yours.

$( [ "$with_toolbox" = 1 ] && printf '%s\n%s\n%s' "The toolbox converts and remasters levels: community maps (.nmo) into the open format, the original Ballance" \
"levels into remastered ones (rounder shapes, larger textures), the original's sound theme. It writes into data/" \
"here, where the game finds the results." )$( [ -d "$out/realesrgan" ] && printf '%s' " Real-ESRGAN (realesrgan/, its licences in
realesrgan/LICENSES.txt) makes the larger textures.")

Ships with 13 community maps and our own levels, parts, textures, sounds and music. The original Ballance levels
need a copy of the game: import it from the menu (it is copied into data/).

Needs: $sysreq.
Licence: GPL-3.0 (LICENSE); everything else in it and its licences: THIRD-PARTY.md; credits: CREDITS.md and
assets/own/CREDITS.md. Versions: VERSION.
EOF
: >"$out/portable.txt"
mkdir -p "$out/data"

# --- the originals, for the owner's private test builds only -------------------------------------------------------
if [ -n "$originals" ]; then
    say "importing the original levels from $originals into $out/data (PRIVATE BUILD: never publish it)"
    case $os in
        linux) "$out/drahtseilakt" --import "$originals" ;;
        # the Windows program does the same under Wine; its paths are this folder's, so data/ is the same place
        windows) command -v wine >/dev/null || die "--with-originals for windows needs wine"
                 WINEDEBUG=-all wine "$out/drahtseilakt.exe" --import "$originals" ;;
    esac
    echo "private build: the original levels were imported from a copy of Ballance; never publish this folder" \
        >"$out/PRIVATE-BUILD.txt"
fi

say "done: $out ($(du -sh "$out" | cut -f1))"
