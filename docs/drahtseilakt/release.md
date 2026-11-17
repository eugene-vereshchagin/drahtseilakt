# Releases and packages

A release is three things that change at different speeds, so they are built and shipped apart: an update of the
program rebuilds only the binary packages.

| What | Changes | Built by | Released as |
|---|---|---|---|
| **Binary packages**: the program | every version | the workflow `Drahtseilakt release` (CI) | a release tagged `X.Y.Z` (a draft) |
| **Base data**: `assets/` without the 13 picked maps | rarely | `scripts/pack-assets.sh` (the owner's machine) | a release tagged `data-<DATA_VERSION>` |
| **Levels**: the 13 picked maps | rarely | `scripts/pack-assets.sh` (the owner's machine) | the same `data-<DATA_VERSION>` release |

The data and the levels are not in git ("no binaries in git"), so they cannot be built in CI; they are made on the
owner's machine from the local `assets/` and uploaded by hand.

## Base data

`assets/` without the picked maps (`assets/levels/drahtseilakt/NN-*`) and without development-only files (`mods-dev`,
`own/make`): our own parts, fonts, sounds, factory mods, the manual, the showcase level (`00-Showcase`, `Parts_Test`)
and the 13 community `.nmo` maps (`assets/levels/community`), which the game is bound to. About 93 MB. Never any
original Ballance file.

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

## Making the data and level packages

```sh
scripts/pack-assets.sh          # OUT defaults to ../drahtseilakt-portable/bundles
```

It makes, in OUT (`V` = `scripts/DATA_VERSION`):

| File | For |
|---|---|
| `drahtseilakt-data-V.tar.gz` | the other systems; CI downloads it for the portable archives, the installer, the .app |
| `drahtseilakt-data_V-1_all.deb`, `drahtseilakt-data-V-1.noarch.rpm` | Linux, base data |
| `levels/NN-Name.zip` (13) | every system: unpack into `assets/levels/drahtseilakt/` |
| `drahtseilakt-levels_V-1_all.deb`, `drahtseilakt-levels-V-1.noarch.rpm` | Linux, all 13 maps (depends on the data package) |
| `SHA256SUMS` | all of the above |

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
  --title "Drahtseilakt data $V" \
  --notes "Base data and the 13 picked maps for Drahtseilakt (data version $V). Programs: the releases X.Y.Z." \
  "drahtseilakt-data-$V.tar.gz" \
  "drahtseilakt-data_$V-1_all.deb" "drahtseilakt-data-$V-1.noarch.rpm" \
  "drahtseilakt-levels_$V-1_all.deb" "drahtseilakt-levels-$V-1.noarch.rpm" \
  levels/*.zip SHA256SUMS
```

Look at the draft, then publish it. Then point the CI at the new base data (repository variables, once per data
version). In a private repository a release asset is downloaded through its API URL (`ASSETS_TOKEN`, if set, else the
run's own token, is sent as the bearer):

```sh
V=$(cat scripts/DATA_VERSION)
URL=$(gh api "repos/eugene-vereshchagin/drahtseilakt/releases/tags/data-$V" \
  --jq ".assets[] | select(.name == \"drahtseilakt-data-$V.tar.gz\") | .url")
SUM=$(sha256sum "../drahtseilakt-portable/bundles/drahtseilakt-data-$V.tar.gz" | cut -d' ' -f1)
gh variable set ASSETS_BUNDLE_URL --repo eugene-vereshchagin/drahtseilakt --body "$URL"
gh variable set ASSETS_BUNDLE_SHA256 --repo eugene-vereshchagin/drahtseilakt --body "$SUM"
```

(For a public release the plain download URL
`https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/data-$V/drahtseilakt-data-$V.tar.gz` works too.)
`scripts/ci-assets.sh` warns when the URL does not carry the version of `scripts/DATA_VERSION`.

## The binary release (CI)

Actions -> "Drahtseilakt release" -> Run workflow (the files are run artifacts), or push a tag `X.Y.Z`: the run also
makes a **draft** release with the files below and `SHA256SUMS` (not the data or levels). The input `dev` adds the dev
flavour (never in a release). Nothing runs on a push to a branch.

With `<V>` the tag (or `git-<sha>` outside a tag), these files are made:

| Target | Files |
|---|---|
| Windows x86_64, arm64 | `drahtseilakt-<V>-windows-<arch>-setup.exe` (installer), `drahtseilakt-<V>-windows-<arch>.zip` (portable) |
| Linux x86_64, aarch64 | `drahtseilakt-<V>-linux-<arch>.tar.gz` (portable), `drahtseilakt_<V>-1_<amd64\|arm64>.deb`, `drahtseilakt-<V>-1.<x86_64\|aarch64>.rpm` |
| Linux x86_64 DEV (`dev` input) | `drahtseilakt-<V>-linux-x86_64-dev.tar.gz`, `drahtseilakt-dev_<V>-1_amd64.deb` |
| macOS arm64, x86_64 | `drahtseilakt-<V>-macos-<arch>.dmg` (unsigned `.app`), `drahtseilakt-<V>-macos-<arch>.tar.gz` (portable) |
| FreeBSD x86_64 (experimental) | `drahtseilakt-<V>-freebsd-x86_64.tar.gz` |

- **Portable archives** and the **Windows installer** include the base data, so they work standalone; the picked maps
  are the separate `.zip` files from the data release. The portable archive also has the toolbox.
- **Windows installer** (NSIS, built on Ubuntu from the portable zip): per user by default
  (`%LOCALAPPDATA%\Programs\Drahtseilakt`, no administrator), a Start menu folder, an uninstaller in "Apps". It installs no
  `portable.txt`, so the player's data goes to the profile and an uninstall never touches it. Unsigned: SmartScreen warns.
- **Linux packages**: the program in `/usr/bin/drahtseilakt` (system libraries: `libasound2`, `libudev1`), the data
  where the game looks for it, `/usr/share/drahtseilakt/assets` (the program finds it from `/usr/bin/../share`, then in
  `$XDG_DATA_DIRS`: `files.md`). A `.desktop` file and the icon (the site's favicon scaled by `tools/icon.py`,
  not a new file in git). `drahtseilakt` depends on `drahtseilakt-data (>= DATA_VERSION)`; `drahtseilakt-levels` is
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
