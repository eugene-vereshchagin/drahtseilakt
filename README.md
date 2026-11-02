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
turn the camera, Space lifts it, F11 is full screen, Esc pauses. Without a level named the game opens with its
main menu. The menus are the original's ([menus](docs/drahtseilakt/menu.md)), in the copy's own words: Start
(a level opens when the one before it is finished), Highscore (ten places a level, kept in our data folder),
Options, Credits, Exit; Esc in a level opens the pause menu, a lost run the dead menu, a finished one the points,
the name entry for a new highscore and the end menu.

Every finished run is saved as a replay (on Linux in `~/.local/share/drahtseilakt/replays/<level>/`, the
fastest as `best.replay`); `--replay FILE` plays one, `--headless` checks it without a window, `--screenshot
FILE.png` renders a frame. To test a place, `--sector N` begins at a checkpoint and `--at X,Y,Z` (or a level
object's name) anywhere, with `--ball wood|stone|paper`; such a run keeps its start in its replay and is never a
best run. Until the extended mode is turned on (Options, Extended, or `--debug` for one run) the game is the
original as closely as it can be. The *extended mode* opens every level, shows best times, the sound volume
beside the music's and a Custom levels list (every `.nmo` in `~/.local/share/drahtseilakt/maps/` on Linux), and
adds our tools: R restarts, F2 takes up a free camera to fly through the level
while the run waits, `[` and `]` slow the run down and speed it up again (the run itself stays the same tick by
tick), F5 saves the moment and F9 goes back to it (the run's record is cut back to it, so a long level can be
recorded piece by piece), and the sector and tick are shown. A run helped this way is kept but never the best. For testing, `--autopilot` lets a test player try the level without a window
(with `--record FILE` to keep its run); `--follow FILE` makes it follow the ball's way in a player's replay. `--locate DIR --record TRACK` finds the ball's way in the frames of a video of the original (DIR/s0001.jpg on, four a second, e.g. `ffmpeg -vf fps=4,scale=640:480`) by drawing the level from the places it may be and comparing; the test player follows such a track with `--follow TRACK`. `--frames DIR --replay FILE` draws a replay as such frames, with the true places, to check it. `drahtseilakt --help` lists everything.

## Legal

Files ported from OpenBallance (GPL-3.0, emdzej and contributors) say so in their headers; the `ivp` crate is the physics port.

Drahtseilakt is licensed under the GPL-3.0, like OpenBallance it began from. It needs your own copy of the
game and never ships Ballance's data. (While this repository is private, `levels/` keeps a backup of the
original levels for development; the public repository will not contain it.) Ballance is © 2004 Cyparade /
Atari. This project is not affiliated with them.
