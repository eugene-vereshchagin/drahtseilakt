# Drahtseilakt

**English** · [Русский](README.ru.md) · [Deutsch](README.de.md)

*Drahtseilakt* (German, "tightrope act") is what the Germans call any balancing act where one step aside
means a fall. Ballance — made in 2004 by Cyparade, a small German studio — is exactly that: a ball on a rail
over an abyss. The name is our nod to the game and to the people who made it.

Drahtseilakt is a fork of [OpenBallance](https://github.com/emdzej/openballance) by emdzej, a faithful
reimplementation of **Ballance** (Cyparade / Atari, 2004) that runs the original game's own Virtools scripts
on a new runtime, with a port of its Ipion physics engine. This fork aims at a proper native game: a real
window on Linux, Windows and macOS, any resolution and aspect ratio up to 8K, a really round ball, level 13
and a menu for community maps — and later an open, documented level format.

Status: work in progress, nothing of the fork's own goals is done yet. See the
[roadmap](docs/drahtseilakt/roadmap.md) and the [decisions](docs/drahtseilakt/decisions.md) behind it.

## How it works

Ballance keeps almost all of its game logic in Virtools behaviour graphs inside its `.cmo` / `.nmo` files.
The engine loads those files into a CK2-compatible object model and executes the graphs; the Building Blocks
they call are ported from the original DLLs, and the physics is a port of the Ipion (IVP) engine from
`physics_RT.dll`. The internals are documented in [`docs/`](docs/) (upstream's documentation site:
https://openballance.emdzej.pl).

## Building

Today the build is upstream's (the native backend is phase 1 of the roadmap):

```sh
tools/fetch-gasm-sdk.sh
cmake -S . -B build-gasm -DOPENBALLANCE_PLATFORM=gasm -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_TOOLCHAIN_FILE=.deps/gasm-c-sdk/cmake/gasm-toolchain.cmake -DWASI_SDK_PREFIX="$PWD/.deps/wasi-sdk"
cmake --build build-gasm -j        # build-gasm/openballance.wasm

cmake -S . -B build && cmake --build build -j   # engine library + tests (need your game data)
```

## Legal

Drahtseilakt is licensed under the GPL-3.0, like OpenBallance it is forked from. It contains no code or
assets from Ballance: it needs your own copy of the game. Ballance is © 2004 Cyparade / Atari. This project
is not affiliated with them.
