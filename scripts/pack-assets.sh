#!/bin/sh
# Packs the picked maps (the one thing git does not hold: "no binaries in git" is lifted for the base data, 2026-11-17,
# but a picked map's level.bin is over GitHub's 100 MB file limit) for the GitHub release data-<V>:
#
#   OUT/levels/<level>.zip                 one picked map each (assets/levels/drahtseilakt/NN-*), compressed, without
#                                          the PNG of a texture that has its .dtx beside it (the game draws the .dtx;
#                                          a level may carry only the compressed one, open-format.md): players
#                                          download the ones they want and unpack them into the game's
#                                          assets/levels/drahtseilakt/. GitHub takes no file over 2 GiB.
#   OUT/drahtseilakt-levels_<V>-1_all.deb, drahtseilakt-levels-<V>-1.noarch.rpm
#                                          the same maps as Linux packages (scripts/make-distro-packages.sh; nfpm is
#                                          fetched pinned into third_party/); --no-packages skips them
#   OUT/SHA256SUMS                         of everything above.
#
# With --with-data it also makes the base data as it was shipped before it went into git, for a local check of the
# packages: OUT/drahtseilakt-data-<V>.tar.gz (assets/ without the picked maps, development-only files and the generated
# manual aside) and the drahtseilakt-data .deb/.rpm from it. CI makes the data .deb/.rpm from the checkout itself
# (scripts/make-distro-packages.sh data), and the data is not uploaded to the data release.
#
# The files are uploaded by hand to a GitHub release tagged data-<V> (docs/drahtseilakt/release.md); bump
# scripts/DATA_VERSION when the data or the levels change (V = scripts/DATA_VERSION).
#
#   scripts/pack-assets.sh [--no-packages] [--with-data] [OUT]      (default OUT: ../drahtseilakt-portable/bundles)
set -eu
cd "$(dirname "$0")/.."
packages=1 with_data=0
while [ $# -gt 0 ]; do
    case $1 in
        --no-packages) packages=0; shift ;;
        --with-data) with_data=1; shift ;;
        --*) echo "pack-assets: unknown option $1" >&2; exit 2 ;;
        *) break ;;
    esac
done
out=${1:-../drahtseilakt-portable/bundles}
dv=$(tr -d ' \n' <scripts/DATA_VERSION)
picks=assets/levels/drahtseilakt
mkdir -p "$out/levels"
out=$(cd "$out" && pwd)

# the picked maps: the numbered folders (NN-Name); the showcase and Parts_Test stay in the base bundle
big=$(cd "$picks" && ls -d [0-9][0-9]-*/ | tr -d / | grep -v "^00-")

stage=$(mktemp -d "$out/.stage.XXXXXX")
trap 'rm -rf "$stage"' EXIT
base="$out/drahtseilakt-data-$dv.tar.gz"
if [ "$with_data" = 1 ]; then
# the base bundle: a copy of the picks' SHA256SUMS without the big folders, so the bundle checks against itself
printf '%s\n' $big | sed 's/^/  /; s/$/\//' >"$stage/big"
grep -v -F -f "$stage/big" "$picks/SHA256SUMS" >"$stage/SHA256SUMS" || true
excl="--exclude=assets/mods-dev --exclude=assets/own/make --exclude=__pycache__ --exclude=*.pyc"
for d in $big; do excl="$excl --exclude=$picks/$d"; done
mkdir -p "$stage/a/$picks"
cp "$stage/SHA256SUMS" "$stage/a/$picks/SHA256SUMS"
# shellcheck disable=SC2086
tar -cf "${base%.gz}" $excl --exclude="$picks/SHA256SUMS" assets
tar -rf "${base%.gz}" -C "$stage/a" "$picks/SHA256SUMS"
rm -f "$base"
gzip -n "${base%.gz}"
echo "$base ($(du -h "$base" | cut -f1))"
fi

# one zip per picked map, with the folder at its top and its own checksums inside, without the PNGs a .dtx stands for
for d in $big; do
    z="$out/levels/$d.zip"
    rm -f "$z"
    (cd "$picks" && find "$d" -type f ! -name SHA256SUMS | sort | while IFS= read -r f; do
        case "$f" in */textures/*.png) [ -f "${f%.png}.dtx" ] && continue ;; esac
        printf '%s\n' "$f"
    done) >"$stage/files"
    mkdir -p "$stage/z/$d"
    (cd "$picks" && while IFS= read -r f; do sha256sum "$f"; done <"$stage/files") |
        sed "s,  $d/,  ," >"$stage/z/$d/SHA256SUMS"
    (cd "$picks" && zip -q -9 "$z" -@ <"$stage/files")
    (cd "$stage/z" && zip -q -9 "$z" "$d/SHA256SUMS")
    echo "$z ($(du -h "$z" | cut -f1))"
done

if [ "$packages" = 1 ]; then
    if [ "$with_data" = 1 ]; then scripts/make-distro-packages.sh data --tar "$base" --out "$out"; fi
    scripts/make-distro-packages.sh levels --zips "$out/levels" --out "$out"
fi
files="levels/*.zip drahtseilakt-levels[_-]*.deb drahtseilakt-levels[_-]*.rpm"
if [ "$with_data" = 1 ]; then files="$(basename "$base") drahtseilakt-data[_-]*.deb drahtseilakt-data[_-]*.rpm $files"; fi
# shellcheck disable=SC2086
(cd "$out" && for f in $files; do [ -f "$f" ] && sha256sum "$f"; done >SHA256SUMS; true)
