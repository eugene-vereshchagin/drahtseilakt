#!/usr/bin/env bash
# CI: makes the Windows installer (NSIS) from a Windows portable archive, on an Ubuntu runner (makensis packs files for
# any Windows architecture, so one job serves x86_64 and arm64; the installer itself is a 32-bit program that both
# run). Per-user install by default (no administrator: %LOCALAPPDATA%\Programs\Drahtseilakt), a Start menu folder,
# an uninstaller listed under "Apps" (HKCU). The portable files are installed WITHOUT portable.txt and data/, so an
# installed game keeps the player's data in the user's profile (%APPDATA%) and an uninstall never touches it. The
# uninstaller deletes exactly the files the installer wrote (a generated list), never a whole folder it did not make.
#
#   scripts/ci-windows-installer.sh ARCHIVE.zip VERSION ARCH OUTDIR      (ARCH: x86_64 | aarch64)
# Needs: makensis (apt: nsis), unzip.
set -euo pipefail
die() { echo "ci-windows-installer: $*" >&2; echo "::error::ci-windows-installer: $*"; exit 1; }
zip=${1:?archive.zip} version=${2:?version} arch=${3:?arch} outdir=${4:?outdir}
[ -f "$zip" ] || die "no $zip"
command -v makensis >/dev/null || die "makensis is missing (apt-get install nsis)"
here=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
unzip -q "$zip" -d "$work/unz"
src=$(find "$work/unz" -mindepth 1 -maxdepth 1 -type d | head -n1)
[ -f "$src/drahtseilakt.exe" ] || die "the archive has no drahtseilakt.exe"
rm -rf "$src/data" "$src/portable.txt"
mkdir -p "$outdir"; outdir=$(cd "$outdir" && pwd)

# the install and uninstall lists, from the tree (names with a quote or dollar are refused: NSIS would read them)
find "$src" -name '*[$"`]*' | grep -q . && die "a file name has a quote or dollar sign"
inst="$work/files.nsh" uninst="$work/unfiles.nsh"
: >"$inst"; : >"$uninst"
(cd "$src" && find . -type d | sort) | while IFS= read -r d; do
    rel=${d#./}; w=${rel//\//\\}
    if [ "$d" = . ]; then echo 'SetOutPath "$INSTDIR"' >>"$inst"; else echo "SetOutPath \"\$INSTDIR\\$w\"" >>"$inst"; fi
    (cd "$src/$d" && find . -maxdepth 1 -type f | sort) | while IFS= read -r f; do
        echo "File \"$src/$rel/${f#./}\"" | sed 's,/\./,/,' >>"$inst"
    done
done
(cd "$src" && find . -type f | sort) | sed 's,^\./,,; s,/,\\,g; s,^,Delete "$INSTDIR\\,; s,$,",' >"$uninst"
(cd "$src" && find . -mindepth 1 -type d | sort -r) | sed 's,^\./,,; s,/,\\,g; s,^,RMDir "$INSTDIR\\,; s,$,",' >>"$uninst"

haveTool=0; [ -f "$src/drahtseilakt-toolbox.exe" ] && haveTool=1
exe="drahtseilakt-$version-windows-$arch-setup.exe"
makensis -V2 -DVERSION="$version" -DARCH="$arch" -DOUTFILE="$outdir/$exe" -DHAVE_TOOLBOX="$haveTool" \
    -DFILES="$inst" -DUNFILES="$uninst" "$here/scripts/packaging/installer.nsi"
ls -l "$outdir/$exe"
