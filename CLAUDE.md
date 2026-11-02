# Drahtseilakt — working rules

Drahtseilakt is a new engine for Ballance (Cyparade / Atari, 2004), written from scratch in Rust. It plays the
data of the player's own copy — the original levels as they are, community maps natively or through their
Virtools scripts — and later its own open level format. A proper native desktop game: any resolution and aspect
ratio, level 13, a menu for community maps, a converter. The repository began as a fork of
[OpenBallance](https://github.com/emdzej/openballance); its C code is reference until phase 1 replaces it, and
no code of other projects is taken (decisions.md, "Everything from scratch, in Rust").

Owner: Veresk. Chat with him in Russian; code, comments, docs and commit messages are in English.

**Start here:** [docs/drahtseilakt/roadmap.md](docs/drahtseilakt/roadmap.md) — the agreed plan, in order, with
what "done" means for each phase — and [docs/drahtseilakt/decisions.md](docs/drahtseilakt/decisions.md) — why.
Work goes phase by phase; a change of plan is recorded there, not only in a chat.

## Security rules

Every change follows the secure-codegen rules from https://github.com/eugene-vereshchagin/secure-codegen-rules
(entry point `skills/secure-codegen/SKILL.md`; read it and the modules it routes to before writing code).
The active profile for this project:

- **Rust** (`lang/rust.md`): no `unsafe` in parsers or anything that touches file data; no `unwrap`/`expect`
  on input-derived values; checked arithmetic for sizes and offsets.
- **Untrusted input** (`modules/input.md`, `modules/inject.md`): `.nmo` / `.cmo` files are untrusted. Community
  maps come from strangers' file shares, so every parser treats its input as hostile: bounds-check every
  read, cap sizes, counts and recursion depth, fail closed on a malformed file — never crash, never
  execute data. The behaviour graphs in a map are data driven by our runtime, not code to trust.
- **Dependencies** (`modules/supply-chain.md`): exact, pinned versions; downloaded binaries checked against
  a checksum; no dependency added without checking it exists and is maintained.
- **CI** (`modules/infra.md`): actions pinned by full commit SHA, minimal `permissions:`.
- Changes to parsers, the file layer or CI are level C (`process.md`): independent review before done.

## Architecture in one screen

- A cargo workspace: `Cargo.toml` at the root, crates in `crates/` (the `.nmo` reader first; engine, game and
  tools follow the roadmap). Dependencies pinned in `Cargo.lock` and vendored; the build never needs the network.
- Rendering on wgpu (Vulkan, Metal, Direct3D 12); the window/input/audio host is chosen in phase 2.
- Levels in two forms, both playable permanently: the original `.nmo` and, later, our open format.
- `src/`, `tools/` (C, upstream OpenBallance) — reference only, leaving with phase 1. Never copy from them, from
  Ballanced, or from any leaked source; `docs/` holds the reverse-engineering notes (formats, behaviour).

## Conventions

- **One systemic approach.** No special cases for one level, one map or one platform. A new need is solved
  once, in the layer it belongs to, for every target.
- **Every target platform.** Linux, Windows and macOS, native. The browser (gasm) build is not a target.
  A change covers all of them; what could not be built or tested on some target is said explicitly.
- Style: `rustfmt` and `clippy` clean; a comment at the top of each file saying what it is for; comments that
  say *why*.
- Code, tests, fixtures and examples never depend on game data in the repository (see `.gitignore`). Tests
  that need the game read it from `OPENBALLANCE_DATA`; community maps come from `OPENBALLANCE_MAPS`, downloaded
  mods from `OPENBALLANCE_MODS` (see `docs/drahtseilakt/community-maps.md`). While the repository is private it
  may hold game data as a backup, only where the owner says; the public release is a new, clean repository
  (decisions.md, "The private repository is a backup").
- **Own content by default, the original's on the player's request** (owner's decision 2026-11-02, decisions.md
  "Own content by default"). Nothing of the original is ever shipped. The game comes with its own interface and
  parts, made as close to the original as we can ourselves (own drawings, sounds, freely licensed fonts); in the
  extended mode a switch loads the original's from the player's copy, or its remaster made by the toolbox. Levels
  are shared either with their own assets (free licence) or, for players who want it, so that the original's
  resources are attached from each player's own copy by the toolbox. The toolbox makes every resource the game can
  use out of the player's original files, in a couple of clicks. Both our native format and original `.nmo` play.
  What players copy into their folders is their business.
- **Mods are never executed.** BML mods are Windows DLLs: a source of feature requirements, read only, never
  loaded, never run — not even under Wine.
- **The game never goes online.** No network code in the game: fonts and every other resource ship with it or
  come from the player's copy. Only the build may download, pinned and checksummed, and it must also build offline.
- Docs are updated in the same change as the behaviour they describe.
- **Docs exist in English, Russian, German and Simplified Chinese** (`zh`, for the large Chinese community).
  English is primary; every other language mirrors the same path under `docs/<lang>/` (e.g.
  `docs/drahtseilakt/roadmap.md` ↔ `docs/ru/drahtseilakt/roadmap.md` ↔ `docs/de/drahtseilakt/roadmap.md` ↔
  `docs/zh/drahtseilakt/roadmap.md`) and has `README.<lang>.md`. A change to one updates all of them in the
  same commit. The site (VitePress, `docs/.vitepress/config.ts`) has an English root locale and one locale
  per other language; a new page gets its sidebar entry in every locale. Adding a language means: a
  `docs/<lang>/` mirror, a `README.<lang>.md`, a locale in the config, the switcher line in every README.

## Tests

- Data-free tests run everywhere: unit tests, synthetic minimal `.nmo` files built by the tests themselves,
  fuzz targets for every parser, debug builds with ASan + UBSan.
- Game-data tests (`ck_test`, `cab_test`, `run_test`) run locally against a real copy.
- Expected results come from the requirement or the original game, never from what the code happens to do.

## Process

- Development runs on the owner's machine, which has the game data. A change is committed after it has
  been built and tested there; say what was run and on which targets.
- **Never start a release, tag or publish without the owner's explicit command.**
- CI runs on demand and on pull requests, not on every push. Batch changes; don't trigger CI per small fix.
  Sometimes the owner wants to look before CI runs — ask when unsure.
- Commits are authored as `Veresk <eugene-vereshchagin@users.noreply.github.com>` (GitHub rejects the
  private address).
- Upstream attribution stays: the history, the GPL-3.0 license and the credit to emdzej in the README.
