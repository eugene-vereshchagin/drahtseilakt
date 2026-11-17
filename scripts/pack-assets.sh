#!/bin/sh
# Packs the files git does not hold (levels, textures, generated parts: "no binaries in git") for hosting outside
# GitHub (a cloud folder), in two kinds:
#
#   OUT/drahtseilakt-data-<V>.tar.gz       (V = scripts/DATA_VERSION) assets/ without the picked maps' big folders (ours, fonts, sounds, the
#                                          community .nmo maps, the showcase): what every package needs. CI fetches it
#                                          (scripts/ci-assets.sh: repository variables ASSETS_BUNDLE_URL and
#                                          ASSETS_BUNDLE_SHA256, the latter printed here).
#   OUT/levels/<level>.zip                 one picked map each (assets/levels/drahtseilakt/NN-*), compressed, without
#                                          the PNG of a texture that has its .dtx beside it (the game draws the .dtx;
#                                          a level may carry only the compressed one, open-format.md): players
#                                          download the ones they want and unpack them into the game's
#                                          assets/levels/drahtseilakt/. GitHub takes no file over 2 GiB.
#   OUT/drahtseilakt-data_<V>-1_all.deb, drahtseilakt-data-<V>-1.noarch.rpm,
#   OUT/drahtseilakt-levels_<V>-1_all.deb, drahtseilakt-levels-<V>-1.noarch.rpm
#                                          the same data and levels as Linux packages (scripts/make-distro-packages.sh;
#                                          nfpm is fetched pinned into third_party/); --no-packages skips them
#   OUT/SHA256SUMS                         of everything above.
#
# The files are uploaded by hand to a GitHub release tagged data-<V> (docs/drahtseilakt/release.md); bump
# scripts/DATA_VERSION when the data changes.
#
#   scripts/pack-assets.sh [--no-packages] [OUT]      (default: ../drahtseilakt-portable/bundles)
set -eu
cd "$(dirname "$0")/.."
packages=1
if [ "${1:-}" = --no-packages ]; then packages=0; shift; fi
out=${1:-../drahtseilakt-portable/bundles}
dv=$(tr -d ' \n' <scripts/DATA_VERSION)
picks=assets/levels/drahtseilakt
mkdir -p "$out/levels"
out=$(cd "$out" && pwd)

# the picked maps: the numbered folders (NN-Name); the showcase and Parts_Test stay in the base bundle
big=$(cd "$picks" && ls -d [0-9][0-9]-*/ | tr -d / | grep -v "^00-")

# the base bundle: a copy of the picks' SHA256SUMS without the big folders, so the bundle checks against itself
stage=$(mktemp -d "$out/.stage.XXXXXX")
trap 'rm -rf "$stage"' EXIT
printf '%s\n' $big | sed 's/^/  /; s/$/\//' >"$stage/big"
grep -v -F -f "$stage/big" "$picks/SHA256SUMS" >"$stage/SHA256SUMS" || true
excl="--exclude=assets/mods-dev --exclude=assets/own/make --exclude=__pycache__ --exclude=*.pyc"
for d in $big; do excl="$excl --exclude=$picks/$d"; done
base="$out/drahtseilakt-data-$dv.tar.gz"
mkdir -p "$stage/a/$picks"
cp "$stage/SHA256SUMS" "$stage/a/$picks/SHA256SUMS"
# shellcheck disable=SC2086
tar -cf "${base%.gz}" $excl --exclude="$picks/SHA256SUMS" assets
tar -rf "${base%.gz}" -C "$stage/a" "$picks/SHA256SUMS"
rm -f "$base"
gzip -n "${base%.gz}"
echo "$base ($(du -h "$base" | cut -f1))"

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
    scripts/make-distro-packages.sh data --tar "$base" --out "$out"
    scripts/make-distro-packages.sh levels --zips "$out/levels" --out "$out"
fi
(cd "$out" && for f in "$(basename "$base")" levels/*.zip drahtseilakt-data[_-]*.deb drahtseilakt-data[_-]*.rpm \
    drahtseilakt-levels[_-]*.deb drahtseilakt-levels[_-]*.rpm; do [ -f "$f" ] && sha256sum "$f"; done >SHA256SUMS; true)
echo "ASSETS_BUNDLE_SHA256=$(sha256sum "$base" | cut -d' ' -f1)"
