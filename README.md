# Drahtseilakt

**English** · [Русский](README.ru.md) · [Deutsch](README.de.md) · [简体中文](README.zh.md)

*Drahtseilakt* (German, "tightrope act") is what the Germans call any balancing act where one step aside
means a fall. Ballance — made in 2004 by Cyparade, a small German studio — is exactly that: a ball on a rail
over an abyss. The name is our nod to the game and to the people who made it.

Drahtseilakt is a new engine for **Ballance** (Cyparade / Atari, 2004), written from scratch in Rust. It runs
natively on Linux, Windows and macOS, at any resolution and aspect ratio up to 4K, with its own interface, parts and
sounds, so it plays out of the box. It plays the original levels from your own copy of the game (including level 13),
community maps (as `.nmo` files, with their Virtools scripts), and levels in its own open, documented
[level format](docs/drahtseilakt/open-format.md). The game ships with 13 community maps as their authors made them and has 13 more picked ones (the Drahtseilakt set, a
separate download), all credited to their authors ([CREDITS.md](CREDITS.md)).
Nothing of the original game is ever shipped; the toolbox makes whatever you want from it out of your own copy.

The project started as a fork of [OpenBallance](https://github.com/emdzej/openballance) by emdzej. Its history and
reverse-engineering notes (`docs/`) are kept and its physics is ported to Rust; the rest is new code
([decisions](docs/drahtseilakt/decisions.md) say why). Why the game needed a second life at all:
[history](docs/drahtseilakt/history.md).

**Status:** work in progress, version 0.1.0, not yet released publicly. The engine, the menus, the sound, the
physics, the replays, the community-map scripts, the open format and the packaging are in place; what is left before a
release is listed under "Release criteria" in the [roadmap](docs/drahtseilakt/roadmap.md). A release is made by the
owner only; until then the downloads below are drafts or CI builds.

## Download

Every release has installers and portable archives for Windows, Linux and macOS (FreeBSD experimental), Linux
`.deb`/`.rpm` packages, and the [toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox) inside the
portable archives and installers. The 13 picked maps are a separate download that rarely changes. File names keep the version. The newest release is `https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest`; a script takes it from the API (`GET https://api.github.com/repos/eugene-vereshchagin/drahtseilakt/releases/latest`, then the `assets[]` file whose `name` matches e.g. `drahtseilakt-*-windows-x86_64-setup.exe`, and its `browser_download_url`).
Which file is for whom, and why the downloads are split like this: [distribution.md](docs/drahtseilakt/distribution.md);
the exact list and install lines: [release.md](docs/drahtseilakt/release.md).

## Building and playing

Rust (stable, 1.89 or newer) and cargo; on Linux a Vulkan driver, and udev's headers for the gamepads (`libudev-dev`
on Debian/Ubuntu, `systemd-devel` on Fedora). The build works offline: dependencies are pinned and vendored.

```sh
cargo build --release
target/release/drahtseilakt
```

From a checkout the game finds the base data in `assets/`. It opens with its main menu; `--level N` plays original
level N, `--map FILE.nmo` any level file or community map, `--open DIR` a level in the open format, `--replay FILE`
plays a recorded run; `drahtseilakt --help` lists everything. Arrows roll the ball relative to the camera, Shift+Left/Right
turn the camera, Space lifts it, F11 is full screen, Esc pauses, F1 opens the manual; gamepads work too, and
Options → Controls binds other keys. Game menus, modes and options: [menus](docs/drahtseilakt/menu.md); where the game
keeps its files, portable or installed: [files](docs/drahtseilakt/files.md); where levels live, how to make and share
one: [levels](docs/drahtseilakt/levels.md), [making levels](docs/drahtseilakt/making-levels.md).
On Windows the release build opens no console window; messages go to `drahtseilakt.log` in the data folder.

The game needs no copy of Ballance to start. To play the original levels, point the toolbox at your installed
game or CD image (or run `drahtseilakt --import DIR`); the original's levels, sounds and look then appear in the game.

## The toolbox

`drahtseilakt-toolbox` is a separate program in its own repository,
[eugene-vereshchagin/drahtseilakt-toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox), with its own
releases. It prepares levels on your machine: it imports your copy of Ballance, remasters the original levels (rounder
shapes, textures several times larger; with Real-ESRGAN when you have it installed, else a plain resize), converts
downloaded community maps and checks that they play. The game only plays; it never runs another program. To build both
from source, clone the toolbox beside this repository (it reads levels through this repository's crates). The
[manual](docs/manual/en/toolbox.md) describes it for players.

## Documentation

The docs are in English, Russian, German and Simplified Chinese (`docs/`, `docs/ru/`, `docs/de/`, `docs/zh/`), also
as a site built with VitePress (`docs/.vitepress`). Start with the [roadmap](docs/drahtseilakt/roadmap.md) (the plan and
what "done" means) and the [decisions](docs/drahtseilakt/decisions.md) (why). Players' manual: `docs/manual/<lang>/`,
shipped with the game. For testing there are a headless replay check, a test player (`--autopilot`,
[the test player](docs/drahtseilakt/autopilot.md)) and fuzzing of every parser ([fuzzing](docs/drahtseilakt/fuzzing.md)).
Contributors: the rules are in [distribution.md](docs/drahtseilakt/distribution.md) and `CLAUDE.md`.

## Who made it

The code, the documentation and the generated sounds and music are written by **Claude** (Anthropic's AI model,
working in Claude Code). **Veresk** owns the project: the idea, the decisions, the plan, the playing and the ear
that says what is right. The commits carry both names.

## Legal and credits

Files ported from OpenBallance (GPL-3.0, emdzej and contributors) say so in their headers; the `ivp` crate is the physics port.
The credit to OpenBallance and emdzej stays with the history of this repository.

Everything in a download that was not written for Drahtseilakt, with its licence (the upstream, fonts, texture
sources, sounds and music, the shipped community levels and their authors, every crate compiled into the program),
is listed in [THIRD-PARTY.md](THIRD-PARTY.md), which ships beside `LICENSE`; the maps and their authors are in
[CREDITS.md](CREDITS.md). `tools/third_party.py` regenerates the crate list from `Cargo.lock` offline, and a test fails
on a dependency without a GPL-3.0-compatible licence.

Drahtseilakt is licensed under the GPL-3.0, like OpenBallance it began from. It never ships Ballance's data and needs
no copy of the game to start. (While this repository is private, `levels/` keeps a backup of the original levels for
development; the public repository will not contain it.) The community maps belong to their authors; if an author
objects, we remove the map. Ballance is © 2004 Cyparade / Atari. This project is not affiliated with them.
