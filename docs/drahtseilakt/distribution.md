# How Drahtseilakt is distributed and where to find what

One page for two questions: *what do I download?* and *where in the repositories does a thing live?* The build
and upload steps are in [release.md](release.md); the reasons are in [decisions.md](decisions.md), linked below.

## What is in which download, and why

Drahtseilakt comes in three kinds of pieces that change at different speeds, so they are shipped apart:

- **The program** (the game, and in the portable archives and installers also the toolbox): every version.
- **The base data**: our own parts, textures, fonts, sounds, factory mods, the showcase level `00-Showcase`, the hidden
  `Parts_Test` and the 13 community `.nmo` maps. It is in git and changes now and then.
- **The 13 picked maps** (the Drahtseilakt set): rarely. One `.zip` each, and one Linux package, `drahtseilakt-levels`.

Why it is split this way:

- **GitHub's limits.** A file in git may not be over 100 MB, and a release asset not over 2 GiB. A picked map's
  `level.bin` is over 100 MB, so those maps are release assets only (about 1.1 GB together, each file under the limit);
  the base data (about 100 MB, no file over 3.6 MB) fits in git ([Binaries in git: the base data](decisions.md#binaries-in-git-the-base-data)).
- **Small program updates.** The levels do not change with the program, so a new program version rebuilds only the
  binary packages, and a player does not download 1 GB again. The program's version is the tag `X.Y.Z`; the data has
  its own, `scripts/DATA_VERSION` (a date), and the program packages require `drahtseilakt-data >= DATA_VERSION`.
- **Own content by default; nothing of the original is ever shipped.** The downloads hold only what we made or what
  its authors let us ship (credited in `CREDITS.md`, `THIRD-PARTY.md`). The original's resources are made on the
  player's machine from the player's own copy ([Own content by default](decisions.md#own-content-by-default-the-original-s-on-the-player-s-request)).
  That is legal caution and the project's content policy at once.
- **The toolbox prepares levels; the game only plays.** Importing your copy, remastering, converting downloaded maps
  and the Real-ESRGAN upscaler are the toolbox's job, a separate program in a repository of its own; the game plays
  what is in its folders ([The toolbox prepares levels](decisions.md#the-toolbox-prepares-levels-the-game-only-plays)).
  So the portable archives and installers carry both programs, and the Linux packages carry the game only.
- **The game never goes online.** It does not download levels, updates or fonts and does not check for a new version;
  you fetch files from GitHub yourself ([The game never goes online](decisions.md#the-game-never-goes-online)).
  Only the build may download, pinned and checksummed.
- **Portable or installed.** A portable copy (a `portable.txt` beside the program) keeps everything in its own folder and
  touches nothing else; an installed one keeps your data in your profile, so an uninstall never deletes it
  ([files.md](files.md)).
- **The dev build** is for development, not for players: a debug build with god mode on (`drahtseilakt-dev`, `-dev` in
  the name). It is made only on request (the workflow input `dev`) and is never part of a release.

The only level bound to the game in code is the showcase `00-Showcase`; all other levels are data in folders
([levels.md](levels.md)).

## I want X: download Y

Links give the newest release: `https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest/download/<file>`
(the files have no version in their names; it is in the tag and in the packages' metadata). Check a download against
`SHA256SUMS` from the same release.

| I want | Download |
|---|---|
| To play on Windows, installed (per user, no administrator, `%LOCALAPPDATA%\Programs\Drahtseilakt`) | `drahtseilakt-windows-x86_64-setup.exe` (ARM: `-aarch64-`) |
| To play on Windows, nothing installed | `drahtseilakt-windows-x86_64.zip` (ARM: `-aarch64`) |
| To play on Linux without installing | `drahtseilakt-linux-x86_64.tar.gz` (ARM: `-aarch64`) |
| Debian / Ubuntu packages | `drahtseilakt_amd64.deb` + `drahtseilakt-data_all.deb` (ARM: `drahtseilakt_arm64.deb`) |
| Fedora / openSUSE packages | `drahtseilakt.x86_64.rpm` + `drahtseilakt-data.noarch.rpm` (ARM: `drahtseilakt.aarch64.rpm`) |
| To play on macOS | `drahtseilakt-macos-aarch64.dmg` (Apple silicon) or `drahtseilakt-macos-x86_64.dmg` (Intel); unsigned: the first start needs right-click, Open. `.tar.gz` with the same names is the portable form |
| FreeBSD (experimental) | `drahtseilakt-freebsd-x86_64.tar.gz` |
| The toolbox | inside every portable archive and installer; or its own releases: [drahtseilakt-toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox/releases) |
| The 13 picked maps | the data release `data-<DATA_VERSION>`: one `.zip` per map (unpack into `assets/levels/drahtseilakt/`), or on Linux `drahtseilakt-levels` (`.deb`/`.rpm`) |
| The original Ballance levels | not downloadable from us: the toolbox imports them from your own copy |
| To try the newest unreleased code | build it ([README](https://github.com/eugene-vereshchagin/drahtseilakt#readme)) or take a workflow run's artifacts (`git-<sha>` builds, packages `0~git<sha>`) |

The portable archives and installers include the base data, so they play at once; the Linux packages need the data
package beside the program (`apt install ./drahtseilakt-data_all.deb ./drahtseilakt_amd64.deb`).
The exact file list of a release and the install lines are in [release.md](release.md).

## Where things live

| Where | What |
|---|---|
| `eugene-vereshchagin/drahtseilakt` (this repository) | the game, the engine crates (`crates/`: `nmo`, `level`, `gamedata`, `ivp` physics, `script`, `game`, ...), `assets/` (base data), `docs/`, `scripts/`, `tools/`, `hints/` (autopilot plans), `tests/` |
| `eugene-vereshchagin/drahtseilakt-toolbox` | the toolbox, with its own releases and CI; it reads levels through this repository's `crates/gamedata` and `crates/level` |
| `assets/` | what the game ships: our parts, fonts, sounds, factory mods, `levels/community/` and `levels/drahtseilakt/` (the picked maps' `level.txt` only; their files are release assets) |
| `docs/drahtseilakt/` | the project's docs (roadmap, decisions, formats, menus, mods); each also in `docs/ru/`, `docs/de/`, `docs/zh/` |
| `docs/manual/<lang>/` | the player's manual; `tools/make_manual.py` builds it into `assets/manual`, which is not in git: CI makes it (`scripts/ci-assets.sh`), and the game opens it with F1 |
| `scripts/` | packaging for the release (`ci-package.sh`, `make-distro-packages.sh`, `pack-assets.sh`, `DATA_VERSION`) |
| `.github/workflows/` | CI: tests (`drahtseilakt.yml`) and the release (`drahtseilakt-release.yml`); both run on demand |
| `docs/guide/`, `docs/howto/`, `docs/*.md` in the root | upstream OpenBallance's notes and reverse-engineering references, kept as reference |

## Rules for contributors

- **No original assets**, ever: no file, texture, sound or model of Ballance in git, in a package, a test or a
  fixture. Tests that need the game read it from `OPENBALLANCE_DATA`; nothing in a download comes from the original.
- **Pinned dependencies**: crates in `Cargo.lock` (vendored, the build works offline), actions by full commit SHA, tools
  and downloaded binaries by version and SHA-256. A dependency is added only after checking it exists, is maintained and
  has a GPL-3.0-compatible licence (`tools/third_party.py` and a test guard it).
- **Four languages**: a doc change goes into English, Russian, German and Simplified Chinese in the same commit; a new
  page gets a sidebar entry in every locale (`docs/.vitepress/config.ts`). Menu texts must fit in all of them.
- **CI runs on demand** and on pull requests, not on every push: batch your changes.
- **No network in the game**; only the build may download.
- **Releases only by the owner**: nobody tags, publishes or uploads a data release without his explicit command. A tag
  `X.Y.Z` makes a draft; he publishes it.

Details: `CLAUDE.md` in the repository root.
