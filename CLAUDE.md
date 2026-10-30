# Drahtseilakt — working rules

Drahtseilakt is a fork of [OpenBallance](https://github.com/emdzej/openballance): a reimplementation of Ballance
(Cyparade / Atari, 2004) that runs the original Virtools scripts on its own runtime, with a port of the Ipion
physics engine. The fork's goal is a proper native desktop game: a real window, any resolution and aspect ratio,
level 13, a menu for community maps, and later a converter from the original data to open formats.

Owner: Veresk. Chat with him in Russian; code, comments, docs and commit messages are in English.

**Start here:** [docs/drahtseilakt/roadmap.md](docs/drahtseilakt/roadmap.md) — the agreed plan, in order, with
what "done" means for each phase — and [docs/drahtseilakt/decisions.md](docs/drahtseilakt/decisions.md) — why.
Work goes phase by phase; a change of plan is recorded there, not only in a chat.

## Security rules

Every change follows the secure-codegen rules from https://github.com/eugene-vereshchagin/secure-codegen-rules
(entry point `skills/secure-codegen/SKILL.md`; read it and the modules it routes to before writing code).
The active profile for this project:

- **C** (`lang/c-cpp.md`): no `strcpy`/`sprintf`/`strcat`; overflow-checked sizes; every allocation checked.
- **Untrusted input** (`modules/input.md`, `modules/inject.md`): `.nmo` / `.cmo` files are untrusted. Community
  maps come from strangers' file shares, so every parser treats its input as hostile: bounds-check every
  read, cap sizes, counts and recursion depth, fail closed on a malformed file — never crash, never
  execute data. The behaviour graphs in a map are data driven by our runtime, not code to trust.
- **Dependencies** (`modules/supply-chain.md`): exact, pinned versions; downloaded binaries checked against
  a checksum; no dependency added without checking it exists and is maintained.
- **CI** (`modules/infra.md`): actions pinned by full commit SHA, minimal `permissions:`.
- Changes to parsers, the file layer or CI are level C (`process.md`): independent review before done.

## Architecture in one screen

- `src/` — the portable core in C11. It talks to the host only through `src/platform.h` (input, audio,
  storage, parameters) and `src/render/gpu.h` (a WebGPU-style graphics API with WGSL shaders).
- Backends implement those two headers: `platform_gasm.c` (upstream, runs in gasm and the browser) and the
  native backend (SDL3 + wgpu-native). The core never includes a backend or OS header.
- `src/ck` the Virtools runtime, `src/bb` the Building Blocks (one file per original DLL), `src/phys` the Ipion
  port, `src/render` the renderer, `src/vfs*` + `src/iscab.c` reading the player's own copy of the game.
- Levels exist in two forms the engine will understand side by side: the original `.nmo` (permanent — the
  ~425 community maps exist only in it) and, later, the open format produced by the converter.
- Details: `docs/internals/`.

## Conventions

- **One systemic approach.** No special cases for one level, one map or one platform. A new need is solved
  once, in the layer it belongs to, for every target.
- **Every target platform.** Linux, Windows and macOS for the native build, plus the gasm/browser build.
  A change covers all of them; what could not be built or tested on some target is said explicitly.
- Style follows the existing code: 4 spaces, opening brace of a function on its own line, short
  `snake_case` names, a comment at the top of each file saying what it is for, comments that say *why*.
- No game data in the repository, ever — not in tests, fixtures or examples (see `.gitignore`). Tests that need
  the game read it from `OPENBALLANCE_DATA`.
- Docs are updated in the same change as the behaviour they describe.
- **Docs exist in English and Russian.** English is primary; the Russian version mirrors the same path under
  `docs/ru/` (e.g. `docs/drahtseilakt/roadmap.md` ↔ `docs/ru/drahtseilakt/roadmap.md`) and `README.ru.md`.
  A change to one updates the other in the same commit. The site (VitePress, `docs/.vitepress/config.ts`)
  has an `en` root locale and a `ru` locale; a new page gets its sidebar entry in both.

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
