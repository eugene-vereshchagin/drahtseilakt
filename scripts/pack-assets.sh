#!/bin/sh
# Packs the files git does not hold (levels, textures, generated parts: "no binaries in git") for hosting outside
# GitHub (a cloud folder), in two kinds:
#
#   OUT/drahtseilakt-assets-<rev>.tar.gz   assets/ without the picked maps' big folders (ours, fonts, sounds, the
#                                          community .nmo maps, the showcase): what every package needs. CI fetches it
#                                          (scripts/ci-assets.sh: repository variables ASSETS_BUNDLE_URL and
#                                          ASSETS_BUNDLE_SHA256, the latter printed here).
#   OUT/levels/<level>.zip                 one picked map each (assets/levels/drahtseilakt/NN-*, 100-550 MB, mostly
#                                          4K textures): players download the ones they want and unpack them into
#                                          the game's assets/levels/drahtseilakt/. GitHub takes no file over 2 GiB,
#                                          so these never go into a release archive.
#   OUT/SHA256SUMS                         of everything above.
#
#   scripts/pack-assets.sh [OUT]      (default: ../drahtseilakt-portable/bundles)
set -eu
cd "$(dirname "$0")/.."
out=${1:-../drahtseilakt-portable/bundles}
picks=assets/levels/drahtseilakt
rev=$(git rev-parse --short HEAD)
mkdir -p "$out/levels"
out=$(cd "$out" && pwd)

# the picked maps: the numbered folders (NN-Name); the showcase and Parts_Test stay in the base bundle
big=$(cd "$picks" && ls -d [0-9][0-9]-*/ | tr -d /)

# the base bundle: a copy of the picks' SHA256SUMS without the big folders, so the bundle checks against itself
stage=$(mktemp -d "$out/.stage.XXXXXX")
trap 'rm -rf "$stage"' EXIT
printf '%s\n' $big | sed 's/^/  /; s/$/\//' >"$stage/big"
grep -v -F -f "$stage/big" "$picks/SHA256SUMS" >"$stage/SHA256SUMS" || true
excl=""
for d in $big; do excl="$excl --exclude=$picks/$d"; done
base="$out/drahtseilakt-assets-$rev.tar.gz"
mkdir -p "$stage/a/$picks"
cp "$stage/SHA256SUMS" "$stage/a/$picks/SHA256SUMS"
# shellcheck disable=SC2086
tar -cf "${base%.gz}" $excl --exclude="$picks/SHA256SUMS" assets
tar -rf "${base%.gz}" -C "$stage/a" "$picks/SHA256SUMS"
rm -f "$base"
gzip -n "${base%.gz}"
echo "$base ($(du -h "$base" | cut -f1))"

# one zip per picked map, with the folder at its top, and its own checksums inside
for d in $big; do
    z="$out/levels/$d.zip"
    rm -f "$z"
    (cd "$picks" && grep -F "  $d/" SHA256SUMS | sed "s,  $d/,  ," >"$stage/sums" &&
        cp "$stage/sums" "$d/SHA256SUMS" && zip -q -r -0 "$z" "$d" && rm "$d/SHA256SUMS")
    echo "$z ($(du -h "$z" | cut -f1))"
done

(cd "$out" && sha256sum "$(basename "$base")" levels/*.zip >SHA256SUMS)
echo "ASSETS_BUNDLE_SHA256=$(sha256sum "$base" | cut -d' ' -f1)"
