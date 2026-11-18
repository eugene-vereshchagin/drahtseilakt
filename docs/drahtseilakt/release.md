# Releases and packages

A release is three things that change at different speeds, so they are built and shipped apart: an update of the
program rebuilds only the binary packages.

| What | Changes | Built by | Released as |
|---|---|---|---|
| **Binary packages**: the program, and the base data as `.deb`/`.rpm` | every version | the workflow `Drahtseilakt release` (CI) | a release tagged `X.Y.Z` (a draft) |
| **Levels**: the 13 picked maps | rarely | `scripts/pack-assets.sh` (the owner's machine) | a release tagged `data-<DATA_VERSION>` |

**The base data is in git** (owner's decision 2026-11-17, decisions.md "Binaries in git: the base data"), so CI takes it
from the checkout and downloads nothing; the data `.deb`/`.rpm` are built in the release workflow from it. Only the 13
picked maps are not in git (a `level.bin` there is over GitHub's 100 MB file limit); they are made on the owner's machine
from the local `assets/` and uploaded by hand.

## Base data

`assets/` without the picked maps (`assets/levels/drahtseilakt/NN-*`, whose `level.txt` stays in git) and without
development-only files (`mods-dev`, `own/make`): our own parts and textures, fonts, sounds, factory mods, the showcase
level (`00-Showcase`, `Parts_Test`) and the 13 community `.nmo` maps (`assets/levels/community`), which the game is
bound to. About 100 MB in git (355 files, the largest 3.6 MB), 93 MB as a package. Never any original Ballance file
(checked by hash against a copy of the original when it went in). Not in git, because CI makes it cheaply: the player's
manual (`assets/manual`, `tools/make_manual.py`, run by `scripts/ci-assets.sh`). `.gitignore` ignores the picked maps'
files except their `level.txt`, and the derived `.dtx`, `level.source`, `*.remaster/`, `textures.original/`.

## Levels

The 13 picked maps, one `.zip` each (29-204 MB, 1.1 GB together), and the same maps in one package
`drahtseilakt-levels` (2.8 GB installed, 1.1 GB packed). Every file is under GitHub's 2 GiB limit; the scripts refuse a
package that is over it.

## Versions

- `scripts/DATA_VERSION` is a date, `YYYYMMDD` (e.g. `20261117`). It is the version of the data **and** the version the
  program packages require: they depend on `drahtseilakt-data (>= DATA_VERSION)`.
- Bump it when the data or the levels change (and commit), then pack, then upload. A program release that needs
  newer data than the latest data release needs a data release first.
- The program's version is the tag `X.Y.Z`. A run of the workflow without a tag is `0~git<sha>` in the packages.
- A data release is tagged `data-<DATA_VERSION>`; the workflow's tag filter (`[0-9]+.[0-9]+.[0-9]+`) does not match it,
  so it starts nothing.

## Making the level packages

```sh
scripts/pack-assets.sh          # OUT defaults to ../drahtseilakt-portable/bundles
```

It makes, in OUT (`V` = `scripts/DATA_VERSION`):

| File | For |
|---|---|
| `levels/NN-Name.zip` (13) | every system: unpack into `assets/levels/drahtseilakt/` |
| `drahtseilakt-levels_V-1_all.deb`, `drahtseilakt-levels-V-1.noarch.rpm` | Linux, all 13 maps (depends on the data package) |
| `SHA256SUMS` | all of the above |

`--with-data` also makes the old-style `drahtseilakt-data-V.tar.gz` and data packages from the local `assets/`, for checking
packages locally; they are not uploaded: CI builds `drahtseilakt-data` from git
(`scripts/make-distro-packages.sh data --out DIR`, from the checkout; `--tar FILE` takes a tarball instead).

`.deb` and `.rpm` are made by [nfpm](https://nfpm.goreleaser.com) 2.49.0, one tool for both, so the same script runs
on the owner's machine (no `rpmbuild` needed) and in CI. `scripts/fetch-nfpm.sh` downloads it into
`third_party/` (gitignored; never into a system path) and checks its SHA-256, pinned in the script (the release's own
`checksums.txt`). `scripts/pack-assets.sh --no-packages` skips the packages.

## Uploading the data release

Run by the owner, from the checkout; nothing here runs by itself:

```sh
V=$(cat scripts/DATA_VERSION)
cd ../drahtseilakt-portable/bundles && sha256sum --check --quiet SHA256SUMS
gh release create "data-$V" --repo eugene-vereshchagin/drahtseilakt --draft --latest=false \
  --title "Drahtseilakt levels $V" \
  --notes "The 13 picked maps for Drahtseilakt (data version $V). Programs and base data: the releases X.Y.Z." \
  "drahtseilakt-levels_$V-1_all.deb" "drahtseilakt-levels-$V-1.noarch.rpm" \
  levels/*.zip SHA256SUMS
```

Look at the draft, then publish it. Nothing else has to be set: CI needs no variable, secret or download for the data.

## The binary release (CI)

Actions -> "Drahtseilakt release" -> Run workflow (the files are run artifacts), or push a tag `X.Y.Z`: the run also
makes a **draft** release with the files below and `SHA256SUMS` (not the levels). The input `dev` adds the dev
flavour (never in a release). Nothing runs on a push to a branch.

**Release file names keep the version** (people read them): a tag release publishes the files below as they are, e.g. `drahtseilakt-0.1.0-windows-x86_64-setup.exe`, `drahtseilakt_0.1.0-1_amd64.deb`. A script or a site takes the newest release from the API (`GET https://api.github.com/repos/eugene-vereshchagin/drahtseilakt/releases/latest`), picks from `assets[]` the file whose `name` matches a pattern such as `drahtseilakt-*-windows-x86_64-setup.exe` and uses its `browser_download_url`; <https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest> shows the newest in a browser. The picked maps are in the release tagged `data-<DATA_VERSION>`, not in "latest". Which file for whom: [distribution.md](distribution.md).

With `<V>` the tag (or `git-<sha>` outside a tag), these files are made:

| Target | Files |
|---|---|
| Windows x86_64, aarch64 | `drahtseilakt-<V>-windows-<arch>-setup.exe` (installer), `drahtseilakt-<V>-windows-<arch>.zip` (portable) |
| Linux x86_64, aarch64 | `drahtseilakt-<V>-linux-<arch>.tar.gz` (portable), `drahtseilakt_<V>-1_<amd64\|arm64>.deb`, `drahtseilakt-<V>-1.<x86_64\|aarch64>.rpm` |
| Linux (x86_64 job, once) | `drahtseilakt-data_<DATA_VERSION>-1_all.deb`, `drahtseilakt-data-<DATA_VERSION>-1.noarch.rpm` |
| Linux x86_64 DEV (`dev` input) | `drahtseilakt-<V>-linux-x86_64-dev.tar.gz`, `drahtseilakt-dev_<V>-1_amd64.deb` |
| macOS arm64, x86_64 | `drahtseilakt-<V>-macos-<arch>.dmg` (unsigned `.app`), `drahtseilakt-<V>-macos-<arch>.tar.gz` (portable) |
| FreeBSD x86_64 (experimental) | `drahtseilakt-<V>-freebsd-x86_64.tar.gz` |

- **Portable archives** and the **Windows installer** include the base data, so they work standalone; the picked maps
  are the separate `.zip` files from the data release (`data-<DATA_VERSION>`). The portable archive also has the toolbox.
- **Windows installer** (NSIS, built on Ubuntu from the portable zip): per user by default
  (`%LOCALAPPDATA%\Programs\Drahtseilakt`, no administrator), a Start menu folder, an uninstaller in "Apps". It installs no
  `portable.txt`, so the player's data goes to the profile and an uninstall never touches it. The installer, the uninstaller and the shortcuts carry our icon (`scripts/packaging/icon.ico`, installed beside the
  program). Unsigned: SmartScreen warns. **Not yet:** the icon inside `drahtseilakt.exe` itself (Explorer, the taskbar of a
  portable copy): that needs a resource-embedding build dependency that is not in the offline crate cache; the window
  itself has the icon (set at run time).
- **A portable copy** (`portable.txt` beside the program) finds its assets only in its own `assets/`, never in a source
  tree, `/usr/share` or `XDG_DATA_DIRS`.
- **Linux packages**: the program in `/usr/bin/drahtseilakt` (depends on `libasound2`, `libudev1`; the .rpm on the sonames `libasound.so.2`, `libudev.so.1`, `libc.so.6(GLIBC_2.35)`, which Fedora and openSUSE both provide, unlike package names such as `systemd-libs`; recommends the window-system and graphics libraries the game loads at run time: `libvulkan1`, `libxkbcommon-x11-0`, `libx11-6`, `libxcursor1`, `libxi6`, `libwayland-client0`), the data
  where the game looks for it, `/usr/share/drahtseilakt/assets` (the program finds it from `/usr/bin/../share`, then in
  `$XDG_DATA_DIRS`: `files.md`). A `.desktop` file and the icon (`scripts/packaging/icon-512.png`, our wooden ball, the same picture as the site's
  and the window's; 512 px and a 256 px copy scaled by `tools/icon.py`). `drahtseilakt` depends on `drahtseilakt-data (>= DATA_VERSION)`; `drahtseilakt-levels` is
  optional. The toolbox is not in them (portable only).
  `drahtseilakt-dev` provides and conflicts with `drahtseilakt` (a `.deb` only, from the `dev` input).
- **macOS**: the `.app` holds the data in `Contents/Resources/assets`; it is not signed or notarized, the first start
  needs right-click -> Open.

Install on Debian/Ubuntu: `sudo apt install ./drahtseilakt-data_V-1_all.deb ./drahtseilakt_X.Y.Z-1_amd64.deb`
(and `./drahtseilakt-levels_V-1_all.deb`); Fedora: `sudo dnf install ./drahtseilakt-data-V-1.noarch.rpm ./drahtseilakt-X.Y.Z-1.x86_64.rpm`.

## Pinning and permissions

Actions are pinned by full commit SHA, the Rust toolchain by its exact version, crates by `Cargo.lock`, nfpm and
Real-ESRGAN by SHA-256; the workflow has `contents: read`, and only the release job `contents: write`. NSIS comes from
the Ubuntu 24.04 package archive (signed by the distribution, not hash-pinned; the version is printed in the log).
