# Roadmap

What this fork is going to do, in order, and how we know each step is done. The reasons behind the choices
are in [decisions.md](decisions.md). Start here if you are picking the work up.

State on 2026-10-30: the fork is the upstream OpenBallance 0.1.3 plus project rules (`CLAUDE.md`) and these
documents. No engine code has been changed yet.

## Phase 1 — native backend

Goal: a normal native executable (Linux, Windows, macOS) that opens a window and plays the game, with no gasm.

- New backend implementing `src/platform.h` with **SDL3**: window, keyboard (the core wants DirectInput key
  codes, `plat_dik_keys`), mouse, gamepads, audio output, user files (saves, settings) in the platform's
  standard per-user data directory, launch parameters from the command line.
- New implementation of `src/render/gpu.h` on **wgpu-native**. `gpu.h` is already WebGPU-shaped with WGSL
  shaders and JSON descriptors, so this is a translation layer, not a new renderer. `gpu.h` and
  `platform_gasm.c` (`gpu_*` forwarding) show the exact contract.
- **Fixed 60 Hz simulation.** `src/app.c` calls `ck_process(&ctx, 1000.0f / 60)` once per rendered frame,
  which is only correct because gasm calls the game exactly 60 times a second. Natively the simulation must
  tick at a fixed 60 Hz from an accumulator, whatever the display does; the renderer draws at the display's
  rate and interpolates entity transforms between the last two ticks. Logic and physics never see a
  variable step (the physics and every community map are tuned to 60 Hz).
- CMake: a `native` platform target producing the game executable, next to the existing `gasm` target and
  the data-free `ob_core` library. Dependencies at exact, pinned versions; anything downloaded is checked
  against a checksum. Hardening flags from the secure-codegen C rules in release builds; ASan + UBSan in
  debug builds.
- The gasm backend stays and keeps building: the browser build is a target too.
- Packaging basics: `.desktop` file and icon on Linux.

Done when: the native build plays all twelve levels on Linux, Windows and macOS at the original speed on a
60 Hz and on a 144 Hz display, and the gasm build still builds and runs.

## Phase 2 — any resolution and aspect ratio, up to 8K

- 3D: the projection is hard-wired to 4:3 in `src/render/render.c` (the three `4.0f / 3.0f` call sites:
  game camera, default camera, sky) and the picture is letterboxed. Use the real drawable aspect ratio.
- 2D: menus and the HUD live in a 640×480 coordinate space (`src/app.c` maps the mouse into it,
  `render.c` draws the foreground in it). Scale it to the drawable with one rule: the 640×480 layout is kept
  at 4:3 in the middle, and elements anchored to an edge stay on that edge. Mouse mapping uses the same rule.
- HiDPI: render at the real pixel size of the window (SDL3 pixel density), not logical points.
- 8K (7680×4320) fits WebGPU's default 8192 texture/framebuffer limit; verify, don't assume.
- Image quality: anisotropic filtering on world textures (most of a Ballance level is seen at a grazing
  angle), MSAA. Both configurable.

Done when: 16:9, 21:9, 16:10 and 4:3 windows show an undistorted picture and a usable HUD and menu, from
1280×720 to 7680×4320, and resizing the window works.

## Phase 3 — the ball is really round

- A render primitive "analytic sphere": drawn by ray–sphere intersection per pixel, with per-pixel lighting
  and exact depth, so it is round at any resolution and distance.
- Which meshes are spheres comes from data (a table of the three ball meshes by their names in the game
  files; in the open format it is a property of the object), never from a special case in the renderer.
- The texture keeps the original mapping, so each ball looks like itself, without facets.
- Physics is untouched: the ball is already a sphere there.

Done when: the three balls are perfectly round at 8K and close up, and look like the originals otherwise.

## Phase 4 — level 13 and community maps

- **Level 13** ("Speed"): a free add-on released by Cyparade in 2004, not on the original disc. Loading is
  hard-wired to 12 levels (`src/app.c`, `Level_%02d.NMO`; the viewer accepts 1–12). Support 13 when the file
  is present, through the same path as the others.
- **Community maps**: ~425 maps (2009–today) are listed in
  [pseudoacetyl/BallanceCustomMaps](https://github.com/pseudoacetyl/BallanceCustomMaps) (`CustomMapList.md`:
  author, difficulty, type, videos); downloads are on the community's Mega drive, a curated pack on
  MediaFire, recent ones in the Ballance Discord `#new` channel
  ([FAQ](https://yyc12345.github.io/ballance-discord-rules/en/faq/)). A map is a `.NMO` in the same format as
  `Level_XX.NMO`.
  - A maps folder in the per-user data directory and an in-game menu to pick one. Original files are never
    touched (the community's other conventions are replacing `Level_XX.NMO` or BML's `ModLoader/Maps`).
  - A map that uses a Building Block we don't implement does not crash: the game reports which block is
    missing, by name and GUID. A headless run over the whole collection produces a frequency list of
    missing blocks; implement them in that order.
  - **Maps are untrusted input** (strangers' file shares). Before the maps menu exists, every parser that
    reads `.nmo` / `.cmo` has fuzz targets, runs under ASan + UBSan, and fails closed on malformed data.

Done when: level 13 plays; a folder of community maps is listed and playable; the missing-block report
exists; the parsers have been fuzzed without findings for a meaningful run.

## Phase 5 — physics checked against the original

The upstream port has not been compared numerically with the original game (`docs/guide/status.md`).

- Record reference trajectories from the original game (run once under Wine purely as a measuring
  instrument): ball type, start, input sequence → position and velocity per tick.
- Replay the same scenarios headless in our engine and compare automatically. A mismatch points at a tick
  and a contact, not at a feeling.
- These become regression tests.

## Phase 6 — open level format and converter

- An open, documented level format: geometry glTF 2.0, textures PNG, sounds Opus, object placement and
  properties in a text file. The spec lives in `docs/`.
- A converter from `.nmo` to it, as a separate tool.
- The engine understands both formats side by side, permanently: `.nmo` stays because the community maps
  exist only in it.
- Making converted levels play without the Virtools graphs means moving game logic from graphs to typed
  entities in the engine — an architecture change, done after phases 1–5.
- Test: each original level and its converted copy must produce the same trajectories for the same input.
- Texture upscaling is a step of this pipeline: the repository holds the pipeline and its parameters (model
  with an open licence, per-texture settings and masks), never upscaled original textures. See decisions.md.

## Throughout

- Fix the findings from applying the secure-codegen rules to upstream code as the touched code comes up:
  CI actions are pinned by tag (`actions/checkout@v7`) instead of by commit SHA and have no `permissions:`;
  no hardening flags in CMake; two `strcpy` of constants in `src/app.c`.
- Data-free tests run everywhere (unit, synthetic `.nmo`, fuzzing, sanitizers); game-data tests run locally
  against `OPENBALLANCE_DATA`.
- Documentation in two languages (see `CLAUDE.md`). Upstream's documentation (user guide, internals) exists
  only in English for now; before the official site goes live it needs a Russian translation, and the site
  has to be renamed from OpenBallance to Drahtseilakt.
