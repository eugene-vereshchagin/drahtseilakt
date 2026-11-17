#!/usr/bin/env bash
# Fetches the pinned nfpm (https://nfpm.goreleaser.com, MIT: makes .deb and .rpm without dpkg or rpmbuild) into
# third_party/nfpm/ (gitignored; never a system path) and prints the program's path. Version and SHA-256 of the
# archive are pinned below (the hashes are the release's own checksums.txt, v2.49.0, checked 2026-11-17).
#   scripts/fetch-nfpm.sh        Linux x86_64 and aarch64
set -euo pipefail
version=2.49.0
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) arch=x86_64 sha=2afaa5ffb5e3493402e31e7c67a5a1fb42133e3f66fc315ca0051934125561c7 ;;
    Linux-aarch64|Linux-arm64) arch=arm64 sha=d4eee71839e92c68cbd3b101640bad32d2856fc551acae0635be9118e48c9d9c ;;
    *) echo "fetch-nfpm: only Linux x86_64 and aarch64 are pinned" >&2; exit 1 ;;
esac
cd "$(dirname "$0")/.."
dir=third_party/nfpm-$version
if [ ! -x "$dir/nfpm" ]; then
    mkdir -p third_party/download "$dir"
    tgz=third_party/download/nfpm_${version}_Linux_$arch.tar.gz
    if [ ! -f "$tgz" ]; then
        curl -fsSL -o "$tgz" "https://github.com/goreleaser/nfpm/releases/download/v$version/nfpm_${version}_Linux_$arch.tar.gz"
    fi
    echo "$sha  $tgz" | sha256sum --check --quiet --strict || { echo "fetch-nfpm: $tgz does not match its pinned SHA-256" >&2; rm -f "$tgz"; exit 1; }
    tar -xzf "$tgz" -C "$dir" nfpm
fi
echo "$(pwd)/$dir/nfpm"
