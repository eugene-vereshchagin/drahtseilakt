# Roadmap

What this fork is going to do, in order, and how we know each step is done. The reasons behind the choices
are in [decisions.md](decisions.md). Start here if you are picking the work up.

State on 2026-10-30: the fork is the upstream OpenBallance 0.1.3 plus project rules (`CLAUDE.md`) and these
documents. No engine code has been changed yet; the next step is phase 0.

## Phase 0 — evaluate Ballanced before writing a backend

[Ballanced](https://github.com/doyaGu/Ballanced) (by doyaGu, the author of BallanceModLoaderPlus) rebuilds the whole
Virtools runtime from source, module for module like the original: `CK2`, `VxMath`, a `RenderEngine` on SDL_GPU,
complete Building Block libraries including `physics_RT`, plugins, SDL3 input and sound managers, and the
`Player`. MIT (the repository) and Apache-2.0 (the Building Blocks), so its code may be used in this GPL project.
It has CMake presets for Linux, Windows and macOS (x64 and ARM64) and CI that runs its tests on Linux, but the
only release (0.1.0) is Windows-only. If its Linux build works, phase 1 may already be done there, and its full
Building Block libraries suit community maps better than our game-only set. Nobody has run it on Linux for us yet.

On the owner's machine, with his copy of the game:

1. Build the default `sdl` branch with `cmake --preset linux-x64-runtime` (submodules recursively; SDL3 from the
   distribution or built from a pinned release).
2. Run it and check: it starts; a level can be finished; the ball feels like the original; what it does with
   widescreen and high resolutions; whether a few community maps load.
3. Write down what was run and what was seen, in this file.

Then decide, and record the decision in decisions.md:

- **stay** on OpenBallance and continue with phase 1 as written;
- **move** the fork onto Ballanced — the plan stays, phase 1 shrinks to what is missing there;
- **borrow**: stay on OpenBallance and take parts of Ballanced (Building Blocks for maps, the SDL3 host).

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

## Phase 1b — recorded replays

Goal: nobody replays levels by hand to check that a change broke nothing.

- With the fixed 60 Hz step the game is deterministic: the same input from the same tick gives the same
  trajectory. A **replay** is a log of input per tick, recorded from live play, in a documented text format
  (versioned; it names the level by a hash of its file, so a replay never runs against the wrong data).
- Headless playback at full speed (upstream's `run_test` already drives the game headless with scripted
  input). A replay checks: checkpoints reached on the expected ticks, the finish, the score.
- Floating-point results can differ between CPU architectures and compilers, and small differences grow over
  hundreds of ticks. Replays are checked at key points with a tolerance; exact matching keeps one reference
  per platform.
- Where replays come from: the owner plays each level once; the original game under Wine (phase 5); speedrun
  recordings, if the community's formats can be converted.
- Replays contain only our input, no game data, so they live in the repository (`tests/replays/`) and run in CI.

Done when: a replay of each of the twelve levels is recorded, plays back headless, and fails when the physics
is deliberately changed.

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

## Phase 3 — round things look round

The original's curved geometry — the ball, rails, pipes, rounded platform edges, curved chutes — is coarse
polygons, and at modern resolutions every facet shows. This phase makes it look smooth. **Which techniques
are used, and how far each goes, is decided with the owner after trying them**; the candidates:

- **The ball, analytically.** A render primitive "analytic sphere": drawn by ray–sphere intersection per
  pixel, with per-pixel lighting and exact depth, so it is round at any resolution and distance. Which meshes
  are spheres comes from data (a table of the three ball meshes by name; in the open format a property of the
  object), never from a special case in the renderer. The texture keeps the original mapping.
- **Per-pixel lighting with smooth normals** for all geometry. Much of the faceted look is the original's
  per-vertex lighting smeared across faces; lighting per pixel makes curved surfaces read as curved without
  changing a vertex. Cheapest, largest win.
- **Phong tessellation** (or a similar normal-guided refinement): triangles are subdivided and bent along the
  vertex normals. Where neighbouring faces share normals (rails, pipes) the surface rounds; where an edge has
  split normals (box corners, platform edges) it stays sharp. One rule for all geometry, community maps
  included — the silhouette of a rail becomes a circle.
- **Rebuilding recognised primitives** (a rail as a circle swept along a path, regenerated at any detail):
  the cleanest result but harder and brittler; belongs in the converter (phase 6), not the runtime.

Fixed regardless of the choice: **physics stays on the original geometry.** Smoothing is visual only; the
ball rolls on the same polygons as in 2004, or its behaviour and every community map would change. Check that
the small gap between the visible and the physical surface (the ball looking a hair sunk or lifted) is not
noticeable.

Done when: the chosen techniques are in; the three balls are perfectly round at 8K and close up and otherwise
look like the originals; rails and other curved parts no longer show facets at the resolutions of phase 2;
the replays (phase 1b) give the same trajectories as before (nothing physical moved).

## Phase 3b — replays rendered to video

Goal: an 8K video of a perfect run of the new-looking Ballance, for a YouTube channel.

- Render a replay offline, frame by frame, at any resolution (up to 7680×4320) and a fixed frame rate,
  independent of real time — a slow machine still produces a perfectly smooth 8K video. Audio is rendered from
  the same deterministic run. Frames go to an encoder (e.g. ffmpeg) as a stream, not as files on disk.
- Cameras: the game camera first; a cinematic camera (smooth follow, wide shots of a level) later, as data
  in the replay or beside it, not as code per level.
- "Perfect" runs: the best human runs (owner, speedrunners) first. An automatic search for fast input — a
  TAS-style optimiser that tries input sequences until the ball reaches the next checkpoint sooner — is a
  later stretch goal; it needs no human, but it is a research project of its own.
- Uploading is publishing: nothing goes to YouTube without the owner's command. Footage made from the original
  assets is the rights holder's content; videos of a release with our own textures (see decisions.md) carry
  no such question.

Done when: one level's replay renders to an 8K video with sound that the owner is happy to publish.

## Phase 4 — level 13 and community maps

- **Level 13** ("Speed"): a free add-on released by Cyparade in 2004, not on the original disc. Loading is
  hard-wired to 12 levels (`src/app.c`, `Level_%02d.NMO`; the viewer accepts 1–12). Support 13 when the file
  is present, through the same path as the others.
- **Community maps**: ~425 maps (2009–today) are listed in
  [pseudoacetyl/BallanceCustomMaps](https://github.com/pseudoacetyl/BallanceCustomMaps) (`CustomMapList.md`:
  author, difficulty, type, videos); downloads are on the community's Mega drive, a curated pack on
  MediaFire, recent ones in the Ballance Discord `#new` channel
  ([FAQ](https://yyc12345.github.io/ballance-discord-rules/en/faq/)). A map is a `.NMO` in the same format as
  `Level_XX.NMO`. Sources, the catalogue's format and how to use it: [community-maps.md](community-maps.md).
  - A maps folder in the per-user data directory and an in-game menu to pick one. Original files are never
    touched (the community's other conventions are replacing `Level_XX.NMO` or BML's `ModLoader/Maps`).
  - A map that uses a Building Block we don't implement does not crash: the game reports which block is
    missing, by name and GUID. A headless run over the whole collection produces a frequency list of
    missing blocks; implement them in that order.
  - **Maps are untrusted input** (strangers' file shares). Before the maps menu exists, every parser that
    reads `.nmo` / `.cmo` has fuzz targets, runs under ASan + UBSan, and fails closed on malformed data.

- **Game profiles**: one setting with two values.
  - *Classic* — the original game and nothing else: the twelve levels and level 13, original rules, no
    third-party content. Replays, records and comparisons with the original are only meaningful here.
  - *Extended* — third-party content allowed, at the player's own risk, with a clear warning when switching:
    community maps, texture packs, and later mods. **Mods here means our own mod system** — data and scripts
    run in a sandbox (no file or network access), the same on every platform. The community's BML mods are
    Windows DLLs for the original runtime and cannot be loaded by this engine at all.
  - Graphics improvements (resolution, round geometry, filtering) are separate settings and available in both
    profiles: they change how the game looks, not what it is.

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
- Documentation in three languages (see `CLAUDE.md`). Upstream's documentation (user guide, internals)
  exists only in English for now; before the official site goes live it needs Russian and German
  translations, and the site has to be renamed from OpenBallance to Drahtseilakt.
