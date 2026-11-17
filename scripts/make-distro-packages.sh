#!/usr/bin/env bash
# Makes the Linux distribution packages (.deb and .rpm, with the pinned nfpm: scripts/fetch-nfpm.sh) in three kinds,
# split by how often they change (docs/drahtseilakt/release.md):
#
#   data    drahtseilakt-data     (all / noarch) the base assets (ours, fonts, sounds, mods, manual, the showcase, the
#                                 community .nmo maps): from the checkout's assets/ (git holds them; CI), or from a base
#                                 data tarball with --tar (scripts/pack-assets.sh, the owner's machine)
#   levels  drahtseilakt-levels   (all / noarch) the 13 picked maps: from the .zip files pack-assets.sh made
#   game    drahtseilakt          (amd64+arm64 / x86_64+aarch64) the program, desktop file and icon; Depends on
#                                 drahtseilakt-data (>= DATA_VERSION). With --dev: drahtseilakt-dev (a build without
#                                 DRAHTSEILAKT_RELEASE, god mode; .deb only; conflicts with drahtseilakt)
#
#   scripts/make-distro-packages.sh data   [--tar FILE.tar.gz] --out DIR [--formats deb,rpm]
#   scripts/make-distro-packages.sh levels --zips DIR        --out DIR [--formats deb,rpm]
#   scripts/make-distro-packages.sh game   --bin FILE --arch x86_64|aarch64 --version V --out DIR [--dev] [--formats ..]
#
# Install paths: /usr/bin/drahtseilakt, /usr/share/drahtseilakt/assets (the game finds it beside the prefix:
# crates/gamedata resources.rs), /usr/share/applications/drahtseilakt.desktop, the icon in hicolor. The player's data
# stays in the user's profile. DATA_VERSION is scripts/DATA_VERSION. No original Ballance file is ever in a package.
# VERSION is a tag (1.2.3) or git-<sha> (becomes 0~git<sha>). An rpm's release is 1.
set -euo pipefail
die() { echo "make-distro-packages: $*" >&2; exit 1; }
say() { echo "make-distro-packages: $*"; }
here=$(cd "$(dirname "$0")/.." && pwd)
data_version=$(tr -d ' \n' <"$here/scripts/DATA_VERSION")
case $data_version in ''|*[!0-9]*) die "scripts/DATA_VERSION is not a number (e.g. 20261117)" ;; esac

kind=${1:-}; [ $# -gt 0 ] && shift || die "usage: $0 data|levels|game ... (see the header)"
tar_file="" zips="" bin="" arch="" version="" out="" dev=0 formats=deb,rpm
while [ $# -gt 0 ]; do
    case $1 in
        --tar) tar_file=${2:?}; shift 2 ;;
        --zips) zips=${2:?}; shift 2 ;;
        --bin) bin=${2:?}; shift 2 ;;
        --arch) arch=${2:?}; shift 2 ;;
        --version) version=${2:?}; shift 2 ;;
        --out) out=${2:?}; shift 2 ;;
        --formats) formats=${2:?}; shift 2 ;;
        --dev) dev=1; shift ;;
        *) die "unknown option $1" ;;
    esac
done
[ -n "$out" ] || die "--out DIR is needed"
mkdir -p "$out"; out=$(cd "$out" && pwd)
nfpm=${NFPM:-$("$here/scripts/fetch-nfpm.sh")}
stage=$(mktemp -d "$out/.stage.XXXXXX")
trap 'rm -rf "$stage"' EXIT
root="$stage/root"; share=usr/share/drahtseilakt
[ "$kind" = game ] || mkdir -p "$root/$share/assets"

maintainer="Veresk <eugene-vereshchagin@users.noreply.github.com>"
homepage="https://github.com/eugene-vereshchagin/drahtseilakt"
pkg_version=$data_version release=1 arch_deb=all arch_rpm=all conflicts="" provides="" replaces=""
deps_deb="" deps_rpm="" recs_deb="" recs_rpm=""
doc=""   # the package's doc folder name

case $kind in
    data)
        name=drahtseilakt-data doc=$name
        if [ -n "$tar_file" ]; then
            [ -f "$tar_file" ] || die "--tar FILE: no such base data tarball (scripts/pack-assets.sh)"
            tar -xzf "$tar_file" -C "$root/$share" assets
        else
            # the checkout (git holds the base data); the manual is generated first (scripts/ci-assets.sh)
            [ -f "$here/assets/manual/index.html" ] || die "no assets/manual: run scripts/ci-assets.sh first (or pass --tar)"
            tar -C "$here" --exclude=assets/mods-dev --exclude=assets/own/make --exclude=__pycache__ --exclude='*.pyc' \
                --exclude='assets/levels/drahtseilakt/[0-9][1-9]-*' --exclude='assets/levels/drahtseilakt/[1-9][0-9]-*' \
                -cf - assets | tar -C "$root/$share" -xf -
            # the checks of a package are against what it holds: the picked maps' entries go from the levels' sums
            lsums="$root/$share/assets/levels/drahtseilakt/SHA256SUMS"
            if [ -f "$lsums" ]; then
                while IFS= read -r line; do
                    f=${line#*  }
                    [ -d "$root/$share/assets/levels/drahtseilakt/${f%%/*}" ] && printf '%s\n' "$line"
                done <"$lsums" >"$lsums.tmp" || true
                mv "$lsums.tmp" "$lsums"
            fi
        fi
        # development-only things never ship here
        rm -rf "$root/$share/assets/mods-dev" "$root/$share/assets/own/make"
        find "$root/$share/assets" -name __pycache__ -type d -prune -exec rm -rf {} + 2>/dev/null || true
        find "$root/$share/assets" -name '*.pyc' -type f -delete
        # the picked maps belong to the levels package, never here
        for d in "$root/$share/assets/levels/drahtseilakt"/[1-9][0-9]-* "$root/$share/assets/levels/drahtseilakt"/0[1-9]-*; do [ -e "$d" ] && die "the tarball holds a picked map: $d"; done
        desc="Drahtseilakt base data (assets)\n Our own parts, fonts, sounds, mods, the manual, the showcase level and the\n community maps. Shared by every Drahtseilakt package; the picked maps are in\n drahtseilakt-levels."
        ;;
    levels)
        name=drahtseilakt-levels doc=$name
        [ -d "$zips" ] || die "--zips DIR: the folder with the NN-*.zip maps (pack-assets.sh: OUT/levels)"
        dest="$root/$share/assets/levels/drahtseilakt"; mkdir -p "$dest"
        n=0
        for z in "$zips"/[0-9][0-9]-*.zip; do case $z in */00-*) continue ;; esac
            [ -f "$z" ] || continue
            # a zip carries its folder at the top and its own SHA256SUMS: check the names, then unpack
            unzip -Z1 "$z" | grep -q -E '^(/|.*\.\./)' && die "$z has an unsafe path"
            unzip -q -o "$z" -d "$dest"; n=$((n + 1))
        done
        [ "$n" -gt 0 ] || die "no NN-*.zip in $zips"
        say "$n maps unpacked"
        deps_deb="drahtseilakt-data (>= $data_version)" deps_rpm="drahtseilakt-data >= $data_version"
        desc="Drahtseilakt, the 13 picked maps\n Community maps remastered for Drahtseilakt (rounded shapes, 4K textures);\n about 2.8 GB installed."
        ;;
    game)
        [ -f "$bin" ] || die "--bin FILE: the game program"
        [ -n "$version" ] || die "--version is needed"
        case $arch in
            x86_64) arch_deb=amd64 arch_rpm=x86_64 ;;
            aarch64) arch_deb=arm64 arch_rpm=aarch64 ;;
            *) die "--arch x86_64 or aarch64" ;;
        esac
        case $version in
            git-*) pkg_version="0~git${version#git-}" ;;
            [0-9]*.[0-9]*.[0-9]*) pkg_version=$version ;;
            *) die "--version is a tag X.Y.Z or git-<sha>" ;;
        esac
        name=drahtseilakt; [ "$dev" = 0 ] || { name=drahtseilakt-dev; conflicts=drahtseilakt provides=drahtseilakt replaces=drahtseilakt; }
        doc=$name
        [ "$dev" = 0 ] || [ "$formats" = deb ] || { say "the dev flavour is a .deb only"; formats=deb; }
        install -D -m 755 "$bin" "$root/usr/bin/drahtseilakt"
        install -D -m 644 "$here/scripts/packaging/drahtseilakt.desktop" "$root/usr/share/applications/drahtseilakt.desktop"
        # the icon: the game's icon (scripts/packaging/icon-512.png, the site's wooden ball), scaled by tools/icon.py (standard library only)
        mkdir -p "$root/usr/share/icons/hicolor/256x256/apps"
        python3 -I "$here/tools/icon.py" 256 "$root/usr/share/icons/hicolor/256x256/apps/drahtseilakt.png" "$here/scripts/packaging/icon-512.png"
        install -D -m 644 "$here/scripts/packaging/icon-512.png" "$root/usr/share/icons/hicolor/512x512/apps/drahtseilakt.png"
        chmod 644 "$root/usr/share/icons/hicolor/256x256/apps/drahtseilakt.png"
        if [ "$dev" = 1 ]; then
            [ -d "$here/assets/mods-dev" ] || die "no assets/mods-dev for the dev flavour"
            mkdir -p "$root/$share/assets"; cp -r "$here/assets/mods-dev" "$root/$share/assets/mods-dev"
        fi
        pkg_version_data=$data_version
        deps_deb="drahtseilakt-data (>= $pkg_version_data), libc6 (>= 2.35), libasound2 | libasound2t64, libudev1"
        # rpm: package names differ between Fedora and openSUSE (systemd-libs / libudev1, alsa-lib / libasound2), so the
        # hard dependencies are sonames, which both distributions provide; the 64-bit marker is right for x86_64 and aarch64.
        # The window-system and graphics libraries the game loads at run time (winit, wgpu) are recommends only: a
        # Wayland-only or X11-only system lacks some of them.
        deps_rpm="drahtseilakt-data >= $pkg_version_data, libc.so.6(GLIBC_2.35)(64bit), libasound.so.2()(64bit), libudev.so.1()(64bit)"
        recs_deb="libvulkan1, libxkbcommon-x11-0, libx11-6, libxcursor1, libxi6, libwayland-client0"
        recs_rpm="libvulkan.so.1()(64bit), libxkbcommon-x11.so.0()(64bit), libX11.so.6()(64bit), libXcursor.so.1()(64bit), libXi.so.6()(64bit), libwayland-client.so.0()(64bit)"
        desc="Drahtseilakt, a ball-rolling puzzle game\n A new engine for Ballance (2004). The program only: its data is in\n drahtseilakt-data, the picked maps in drahtseilakt-levels."
        [ "$dev" = 0 ] || desc="Drahtseilakt DEVELOPMENT build\n Debug on, extended mode, the god-mode mod. Not for players; conflicts with\n drahtseilakt."
        ;;
    *) die "kind: data, levels or game" ;;
esac
[ "$kind" = game ] || { arch_deb=all arch_rpm=all; }

# licence and credits in the package's doc folder
docdir="$root/usr/share/doc/$doc"; mkdir -p "$docdir"
install -m 644 "$here/LICENSE" "$docdir/copyright"
for f in CREDITS.md THIRD-PARTY.md; do [ -f "$here/$f" ] && install -m 644 "$here/$f" "$docdir/$f"; done
# no original Ballance file in a package, checked by name
if find "$root" \( -iname '*.nmo' -path '*/Ballance/*' -o -iname 'Ballance.exe' -o -iname 'Base.cmo' \) | grep -q .; then
    die "an original Ballance file is in the package tree"
fi
# plain modes: directories 755, files 644 (the program 755)
find "$root" -type d -exec chmod 755 {} +
find "$root" -type f ! -path "$root/usr/bin/*" -exec chmod 644 {} +

# the nfpm config, one per format (dependencies are written differently in each)
for fmt in ${formats//,/ }; do
    case $fmt in deb) arch=$arch_deb deps=$deps_deb recs=$recs_deb ;; rpm) arch=$arch_rpm deps=$deps_rpm recs=$recs_rpm ;; *) die "format deb or rpm" ;; esac
    cfg="$stage/nfpm-$fmt.yaml"
    {
        echo "name: $name"
        echo "arch: $arch"
        echo "platform: linux"
        echo "version_schema: none"; echo "version: \"$pkg_version\""
        echo "release: \"$release\""
        echo "section: games"
        echo "priority: optional"
        echo "maintainer: \"$maintainer\""
        echo "description: |"; printf '%b\n' "$desc" | sed 's/^/  /' 
        echo "vendor: Veresk"
        echo "homepage: \"$homepage\""
        echo "license: GPL-3.0"
        if [ -n "$deps" ]; then echo "depends:"; IFS=, read -ra ds <<<"$deps"; for d in "${ds[@]}"; do echo "  - \"${d# }\""; done; fi
        if [ -n "$recs" ]; then echo "recommends:"; IFS=, read -ra rs <<<"$recs"; for r in "${rs[@]}"; do echo "  - \"${r# }\""; done; fi
        if [ -n "$conflicts" ]; then echo "conflicts: [\"$conflicts\"]"; echo "provides: [\"$provides\"]"; echo "replaces: [\"$replaces\"]"; fi
        echo "contents:"
        echo "  - src: $root/"
        echo "    dst: /"
        echo "    type: tree"
        if [ "$fmt" = deb ]; then echo "deb:"; echo "  compression: gzip"; fi
    } >"$cfg"
    "$nfpm" package --config "$cfg" --packager "$fmt" --target "$out/" >/dev/null
done
for f in "$out"/${name}[_-]*.deb "$out"/${name}[_-]*.rpm; do
    [ -f "$f" ] || continue
    [ -n "$(find "$f" -newer "$stage/root" -print 2>/dev/null)" ] || continue
    size=$(stat -c %s "$f")
    [ "$size" -lt 2147483648 ] || die "$f is $size bytes: over GitHub's 2 GiB limit"
    say "$f ($(du -h "$f" | cut -f1))"
done
