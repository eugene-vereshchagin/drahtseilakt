#!/usr/bin/env bash
# CI: makes sure assets/ is ready before a package is built. The base data is in git (owner's decision 2026-11-17, docs/drahtseilakt/
# release.md), so a checkout has it; nothing is downloaded. Not in git, and so made here: the player's manual
# (tools/make_manual.py, cheap). The 13 picked maps (assets/levels/drahtseilakt/NN-*) are not in a checkout either (release
# assets): the entries of the levels' SHA256SUMS for a folder that is absent are dropped, so the packages check against
# themselves. Then the checks of scripts/make-portable.sh run. Fails with a clear message when anything is missing; never
# builds an incomplete package.
set -euo pipefail
die() { echo "ci-assets: $*" >&2; echo "::error::ci-assets: $*"; exit 1; }
cd "$(dirname "$0")/.."

# from stdin: given a Windows path (backslashes), sha256sum escapes its line and the hash gains a leading "\"
sha_of() { if command -v sha256sum >/dev/null; then sha256sum <"$1" | cut -d' ' -f1
           elif command -v shasum >/dev/null; then shasum -a 256 <"$1" | cut -d' ' -f1
           else sha256 -q <"$1"; fi; }

[ -f "assets/own/3D Entities/MenuLevel/level.bin" ] && ls assets/levels/*/*/level.bin >/dev/null 2>&1 \
    || die "assets/ lacks the base data: is this a complete checkout of the repository?"

if [ ! -f assets/manual/index.html ]; then
    py=python3; command -v python3 >/dev/null || py=python
    if ! "$py" -I -c 'import markdown' 2>/dev/null; then
        # python-markdown into a throw-away venv: a user-site install is invisible to -I, and Homebrew's Python refuses a
        # system one (PEP 668)
        venv="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/manual-venv"
        "$py" -m venv "$venv"
        py="$venv/bin/python"; [ -x "$py" ] || py="$venv/Scripts/python.exe"
        "$py" -m pip install --quiet --disable-pip-version-check "markdown==3.7"
    fi
    "$py" -I tools/make_manual.py
fi
[ -f assets/manual/index.html ] || die "no manual after tools/make_manual.py"

# the checkout holds only the text source (level.txt) of a picked map, which would show as a broken level in the game:
# remove such a folder (the maps come with their own release assets); then the levels' SHA256SUMS drops their entries
for d in assets/levels/drahtseilakt/[0-9][1-9]-* assets/levels/drahtseilakt/[1-9][0-9]-*; do
    [ -d "$d" ] && [ ! -f "$d/level.bin" ] && rm -rf "$d"
done
sums=assets/levels/drahtseilakt/SHA256SUMS
if [ -f "$sums" ]; then
    : >"$sums.tmp"
    while IFS= read -r line; do
        f=${line#*  }
        [ -d "assets/levels/drahtseilakt/${f%%/*}" ] && printf '%s\n' "$line" >>"$sums.tmp"
    done <"$sums" || true
    mv "$sums.tmp" "$sums"
fi
for sums in assets/levels/*/SHA256SUMS assets/own/SHA256SUMS assets/fonts/SHA256SUMS; do
    [ -f "$sums" ] || die "missing $sums"
    dir=$(dirname "$sums")
    while read -r h f; do
        [ -n "$h" ] || continue
        [ "$(sha_of "$dir/$f")" = "$h" ] || die "$dir/$f does not match $sums"
    done <"$sums"
done
echo "ci-assets: assets/ is complete"
