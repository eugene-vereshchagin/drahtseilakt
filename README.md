# Drahtseilakt

**English** · [Русский](README.ru.md) · [Deutsch](README.de.md)

*Drahtseilakt* (German, "tightrope act") is what the Germans call any balancing act where one step aside
means a fall. Ballance — made in 2004 by Cyparade, a small German studio — is exactly that: a ball on a rail
over an abyss. The name is our nod to the game and to the people who made it.

Drahtseilakt is a new engine for **Ballance** (Cyparade / Atari, 2004), written from scratch in Rust. It plays
the data of your own copy of the game — the original levels exactly as they are, and community maps — on Linux,
Windows and macOS, at any resolution and aspect ratio up to 8K, with level 13 and a menu for community maps,
and later an open, documented level format and a converter to it.

The project started as a fork of [OpenBallance](https://github.com/emdzej/openballance) by emdzej, which runs the
original Virtools scripts on its own runtime; its history and reverse-engineering notes are kept, and its code
is being replaced (the [decisions](docs/drahtseilakt/decisions.md) say why).

Status: work in progress — phase 1 of the [roadmap](docs/drahtseilakt/roadmap.md), the reader for Virtools
files. Why the game needed this at all: [history](docs/drahtseilakt/history.md).

## Building and playing

Rust (stable, 1.87 or newer) and cargo; on Linux a Vulkan driver. You need your own copy of Ballance.

```sh
cargo build --release
target/release/drahtseilakt --data /path/to/Ballance --level 1
```

`--data` (or `OPENBALLANCE_DATA`) is the Ballance folder. `--level N` plays original level N, `--map FILE.nmo`
any level file or community map, `--open DIR` a level in the [open format](docs/drahtseilakt/open-format.md);
`--export DIR` converts the chosen level to it. Arrows roll the ball relative to the camera, Shift+Left/Right
turn the camera, Space lifts it, R restarts, F11 is full screen, Esc opens the menu. Without a level named the
game opens with its main menu (levels with their best times; custom levels, settings and debug are placeholders
for now); after a finished level Enter plays the next.

Every finished run is saved as a replay (on Linux in `~/.local/share/drahtseilakt/replays/<level>/`, the
fastest as `best.replay`); `--replay FILE` plays one, `--headless` checks it without a window, `--screenshot
FILE.png` renders a frame. To test a place, `--sector N` begins at a checkpoint and `--at X,Y,Z` (or a level
object's name) anywhere, with `--ball wood|stone|paper`; such a run keeps its start in its replay and is never a
best run. Sound and music volumes are on the menu's Settings page. In debug mode (`--debug`, or the switch on the menu's Debug page) F2 takes up a free camera to fly
through the level while the run waits. For testing, `--autopilot` lets a test player try the level without a window
(with `--record FILE` to keep its run). `drahtseilakt --help` lists everything.

## Legal

Files ported from OpenBallance (GPL-3.0, emdzej and contributors) say so in their headers; the `ivp` crate is the physics port.

Drahtseilakt is licensed under the GPL-3.0, like OpenBallance it began from. It needs your own copy of the
game and never ships Ballance's data. (While this repository is private, `levels/` keeps a backup of the
original levels for development; the public repository will not contain it.) Ballance is © 2004 Cyparade /
Atari. This project is not affiliated with them.
