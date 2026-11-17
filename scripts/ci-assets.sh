#!/usr/bin/env bash
# CI: makes sure assets/ is complete before a package is built. A fresh clone lacks the files git does not hold (level
# data, textures, our generated parts: .gitignore, "no binaries in git"). They come from an assets bundle, a .tar.gz
# with an assets/ folder in it, whose address and SHA-256 are the repository variables ASSETS_BUNDLE_URL and
# ASSETS_BUNDLE_SHA256 (a private release asset's API URL is fine; the secret ASSETS_TOKEN or TOOLBOX_TOKEN is
# sent as its bearer token). The manual is generated here (tools/make_manual.py). Then the checks of
# scripts/make-portable.sh run. Fails with a clear message when anything is missing; never builds an incomplete package.
set -euo pipefail
die() { echo "ci-assets: $*" >&2; echo "::error::ci-assets: $*"; exit 1; }
cd "$(dirname "$0")/.."

sha_of() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1
           elif command -v shasum >/dev/null; then shasum -a 256 "$1" | cut -d' ' -f1
           else sha256 -q "$1"; fi; }
complete() {
    [ -f "assets/own/3D Entities/MenuLevel/level.bin" ] && ls assets/levels/*/*/level.bin >/dev/null 2>&1
}

if ! complete; then
    url=${ASSETS_BUNDLE_URL:-} want=${ASSETS_BUNDLE_SHA256:-} token=${ASSETS_TOKEN:-}
    [ -n "$url" ] && [ -n "$want" ] || die "assets/ lacks the level data, textures and generated parts (not in git), and no bundle is set: add the repository variables ASSETS_BUNDLE_URL and ASSETS_BUNDLE_SHA256 (a .tar.gz with assets/ in it)"
    echo "ci-assets: downloading the assets bundle"
    auth=(); [ -z "$token" ] || auth=(-H "Authorization: Bearer $token")
    mkdir -p "${RUNNER_TEMP:-/tmp}"
    bundle="${RUNNER_TEMP:-/tmp}/assets-bundle.tar.gz"
    curl -fsSL "${auth[@]}" -H "Accept: application/octet-stream" -o "$bundle" "$url" || die "the assets bundle did not download from ASSETS_BUNDLE_URL"
    [ "$(sha_of "$bundle")" = "$want" ] || die "the assets bundle does not match ASSETS_BUNDLE_SHA256"
    tar -xzf "$bundle"
    rm -f "$bundle"
fi

if [ ! -f assets/manual/index.html ]; then
    py=python3; command -v python3 >/dev/null || py=python
    "$py" -I -c 'import markdown' 2>/dev/null || "$py" -m pip install --quiet --disable-pip-version-check "markdown==3.7"
    "$py" -I tools/make_manual.py
fi

[ -f assets/manual/index.html ] || die "no manual after tools/make_manual.py"
[ -f "assets/own/3D Entities/MenuLevel/level.bin" ] || die "assets/ lacks the generated parts (assets/own/make); the bundle is incomplete"
for sums in assets/levels/*/SHA256SUMS assets/own/SHA256SUMS assets/fonts/SHA256SUMS; do
    [ -f "$sums" ] || die "missing $sums"
    dir=$(dirname "$sums")
    while read -r h f; do
        [ -n "$h" ] || continue
        [ "$(sha_of "$dir/$f")" = "$h" ] || die "$dir/$f does not match $sums"
    done <"$sums"
done
echo "ci-assets: assets/ is complete"
