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

## Building

Rust (stable) and cargo. Dependencies are vendored; the build needs no network. Build instructions follow
with the first code of phase 1.

## Legal

Drahtseilakt is licensed under the GPL-3.0, like OpenBallance it began from. It needs your own copy of the
game and never ships Ballance's data. (While this repository is private, `levels/` keeps a backup of the
original levels for development; the public repository will not contain it.) Ballance is © 2004 Cyparade /
Atari. This project is not affiliated with them.
