# Roadmap

What this fork is going to do, in order, and how we know each step is done. The reasons behind the choices
are in [decisions.md](decisions.md). Start here if you are picking the work up.

State on 2026-11-01: phase 0 is closed (decisions.md, "Everything from scratch, in Rust"). A first night of the
new engine got level 1 playable, ahead of the phase order:

- **Phase 1**: the reader reads every file of the game and all ~600 maps, every scene object decodes.
  Missing: fuzz targets, the inventory of Building Blocks, removing upstream's C code.
- **Phase 2**: levels on screen with their sky, any size and aspect ratio, gamma-space lighting like the
  original (its fixed-function formula: emissive + ambient + diffuse, times the texture; one light so far), 3D
  sprites (the extra points' silver balls), colour-keyed textures, reflections (TexGen) and material channels
  (the domes' blue). A main menu and a level menu
  in the prototype's own font; a free camera in debug mode. The in-game
  interface is the original's: its points box and life balls, the points in the game's own font. The menus
  look like the original's (its buttons, dark band and fonts). Missing: the menu's 3D scene behind them, the
  level end screen, the level's own lights, 8K check.
- **Phase 3**: the ball physics is IVP's, ported from the upstream's C (crates/ivp) and checked against the C
  game: a straight push of the wooden and of the stone ball and a 17-second route on level 1 agree to within 0.1
  and 0.8 units. Missing: floor materials per object.
- **Phase 4**: rules from the level's groups (start, reset points, checkpoints, extra points and lives, ball
  transformers, falling, finish); replays recorded, the best per level kept. Sectors, domes and the level start
  follow the C game, and so does the transformer's animation (ring, bars, the ball's change). The modules of levels 1 to 12 are made of
  their physical parts and joints as in the C game, and the finish's balloon carries the ball away as in the C game.
  The fans' updrafts lift the paper ball; the swings and sacks swing as in the C game; a stone ball breaks the plate
  bridge. Sound as the original's Sound_Manager and Music_Manager make it: event sounds, the ball's hits and rolling
  by floor material, the level's music. The fans hum near them, the plate bridge's rope tears. The menu sets the sound and music volumes.
  Particles as the original's blocks make them: the flames at the start and the checkpoints, the extra points'
  sparkles and trails, the fans' wind, the extra life's fizz. Missing: the ball's smoke, level 13's add-on.
- **Phase 6**: open format v1 with export and import; level 1 round-trips identically.


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

### Results so far (2026-10-30)

Owner's machine: Debian 13, GCC 14.2, CMake 3.31, Ninja, NVIDIA RTX 3060 (driver 550), KDE on X11. Ballanced
branch `sdl` at commit `ececf94`, preset `linux-x64-runtime`.

- **SDL3.** Debian 13 ships 3.2.10; Ballanced requires 3.4.8 or later. Built 3.4.8 from the release tarball,
  GPG signature (Sam Lantinga) verified, sha256 `e9fff7467fb60f037e6708da18b25560649e4c63edc2a69bb871b960d9cbfbba`.
- **Build.** Configure and build are clean; one GCC warning (`-Wfree-nonheap-object` in `XClassArray.h`,
  inlined into `CKSkin.cpp`), probably a false positive. The stage is 25 MB, 162 MB with the game data copied
  in through `BALLANCE_ASSETS_ROOT`.
- **Network.** The runtime has no network code. The build downloads the Noto fonts from the `main` branches
  of the notofonts repositories: checked by sha256, but not pinned to a commit, so an upstream font update
  breaks the build. `VXMATH_FONT_DIR` builds offline from local copies.
- **Running.** SDL_GPU on Vulkan. Main menu, level 1 finished; the ball feels like the original (owner's
  judgement). The window was resized live from 1280×720 to 3840×2069 without trouble; the picture stays 4:3
  with bars. Two sessions, 1.5 and 7 minutes, ended cleanly with nothing in the log.
- **Widescreen.** `--unlock-widescreen --unlock-high-resolution` only adds wide modes to the resolution list
  the game scripts offer; 1920×1080 is accepted, but the level cameras stay 4:3 and the renderer letterboxes
  them. The HUD, however, is laid out against the whole window, so on a wide window it sits beside the picture,
  partly on the black bars. Real widescreen is not implemented; in the original it took a BML mod.
- **Community maps.** A test harness (outside the repositories) puts a map in place of level 1 in a throwaway
  copy of the game root, runs Player on a private Xvfb display with software Vulkan (lavapipe), walks the menu
  with xdotool and captures the game window. Five sample maps, 74 KB to 4 MB, all loaded and ran with nothing
  in the log. The full set (579 `.nmo` and 19 archives) is still to be run.
- **Bug.** The bracket of the life counter in the HUD is drawn far to the left and drifts during play (seen
  in a maximised 3840×2069 window). At 640×480 the HUD matches the original exactly, small left bracket
  included, so the fault belongs to non-4:3 windows and the HUD layout above. Not reported upstream.
- **Text layout.** The tutorial text wraps differently from the original (a line the original prints whole is
  broken in two): glyph metrics or the text frame width differ.
- **Reference.** The owner's original game runs under Wine (Wine 10, its own prefix, a private copy of the
  game, the built-in `ddraw`/`d3dimm` instead of the bundled dgVoodoo) on a private Xvfb display, scripted the
  same way. Two traps: Xvfb has exactly one video mode, so the screen must be the game's mode, or the game
  asks for one in a dialog; and software OpenGL needs `WINE_LARGE_ADDRESS_AWARE=1`, otherwise the 32-bit
  process runs out of address space and `base.cmo` fails to load.
- `Player --help` is not an option: it is ignored and the game starts.
- **Tests** (`linux-x64-tests`, 967 tests, run on Xvfb with lavapipe). GCC: all pass, but 36 CK2 tests fail
  when run in parallel and pass serially — they share some resource; a test-isolation issue, not an engine
  bug. Clang 19: builds only with `lld` (RenderEngine uses LTO, and GNU ld cannot read Clang's bitcode); two
  `RectTransformReferenceTest` cases fail on floating-point exception flags (FE_INVALID raised in unused SIMD
  lanes), which pass under GCC. The opt-in GPU tests pass except one exact-depth pixel test on lavapipe, still
  to be run on real hardware. GoogleTest is fetched by tag, not by commit.
- **Offline build.** With `VXMATH_FONT_DIR` pointing at our bundled fonts (`assets/fonts`) and
  `FETCHCONTENT_FULLY_DISCONNECTED`, Ballanced configures and builds without network access.
- **Original levels.** Each of the twelve, put in place of level 1, loads and runs with nothing in the log
  (load only: nobody played them through).
- **Not yet checked:** `--unlock-framerate`, the full map set.

## Phase 1 — foundation and the Virtools file reader

Goal: a clean Rust codebase that reads every `.nmo` / `.cmo` the game and the community have produced, safely.

- A cargo workspace in this repository (`Cargo.toml` at the root, crates in `crates/`). Dependencies are
  pinned in `Cargo.lock` and vendored, so the build works offline; each crate is checked before it is added
  (exists, maintained, licence compatible with GPL-3.0).
- The reader: file header, compressed parts, object table, state chunks, manager data, embedded files. The
  input is hostile: every read is bounds-checked, sizes, counts and nesting are capped, and a malformed file
  fails with an error that names the offset — never a panic. No `unsafe` in the reader.
- `nmo-dump`, a command-line tool that prints what a file holds: objects, groups, meshes, materials, textures,
  behaviour graphs.
- Tests: synthetic files built by the tests themselves; fuzz targets (cargo-fuzz); local tests over the
  player's copy (`OPENBALLANCE_DATA`), the thirteen levels in `levels/` and the map collection
  (`OPENBALLANCE_MAPS`).
- **Inventory**: across all maps, which object classes, group names, Building Blocks and parameter types occur
  and how often. It sets the order of phases 4 and 5.
- The upstream C code leaves the build (history keeps it); `docs/` stays as reference.

Done when: every file of the game and all ~600 maps either parse completely or fail with a clean error;
fuzzing has run for a meaningful time without findings; the inventory exists.

## Phase 2 — the first level on screen

- Window, input and audio host; renderer on wgpu (Vulkan, Metal, Direct3D 12). Whether the window comes from
  an SDL3 binding or from pure-Rust crates (winit, gilrs, cpal) is decided at the start and recorded.
- Level 1 drawn from its `.nmo` with the player's textures: meshes, materials, transparency, sky, lights.
- Any resolution and aspect ratio from the first frame, no 4:3: the 3D view uses the real aspect ratio; the
  2D layout keeps 640×480 at 4:3 in the middle and anchors edge elements to the edges; HiDPI; 8K checked.
- The fixed 60 Hz simulation tick with interpolated rendering from the start.
- A free camera to fly through any level.

Done when: all thirteen levels can be flown through and look like the original (owner's judgement, frames
compared with the original under Wine).

## Phase 3 — the ball

- **Reference**: the original game under Wine as a measuring instrument — scripted input, the ball's position
  and velocity per tick read from the running game without changing its files (the method is worked out in
  this phase). Recorded scenarios become reference trajectories.
- **Own physics**, from scratch: the three balls on static geometry with friction, damping and gravity as
  measured; boxes and moving objects follow in phase 4. The collision geometry is the original's, polygon for
  polygon.
- Ball control and the game camera, as in the original.

Done when: the reference scenarios stay within tolerance, and the owner rolls through level 1 and says the
ball feels right.

## Phase 4 — the game

- The rules natively, driven by the level's data (its group conventions): start, sectors, checkpoints and
  resets, lives, score, extra points and lives, ball transformers, the physical and moving objects, the
  finish. Menu, HUD (laid out for any aspect ratio), sound, music.
- **Replays**: a per-tick input log in a documented, versioned text format, the level identified by its hash.
  Headless playback checks checkpoints, finish and score. Replays hold only our input, so they live in the
  repository (`tests/replays/`) and run in CI.
- Level 13.

Done when: all thirteen levels can be finished, and each has a replay that passes.

## Phase 5 — community maps

- A maps folder in the per-user data directory and an in-game list (title, author, difficulty from the
  catalogue).
- Maps that follow the standard conventions play through phase 4's native rules.
- Maps with their own Virtools scripts: an interpreter for behaviour graphs, with Building Blocks implemented
  in the order the inventory gives. A map that needs a block we lack is not run half-way: it is reported by
  block name and GUID and marked unsupported.
- **Game profiles**: *Classic* — the original game only (twelve levels and level 13, original rules; replays
  and records count only here); *Extended* — third-party content at the player's own risk, with a clear
  warning: community maps, texture packs, later mods. Mods means our own system — data and scripts in a
  sandbox without file or network access; BML mods are Windows DLLs and are never loaded.

Done when: most maps play; the report lists the rest, each with its reason.

## Phase 6 — open level format and converter

- An open, documented level format: geometry glTF 2.0, textures PNG, sounds Opus, placement and properties in
  a text file. The spec lives in `docs/`.
- A converter from `.nmo`, run by the player on their own files. The engine plays both formats permanently
  (decisions.md).
- Test: each original level and its converted copy give the same trajectories for the same replay.

## Phase 7 — looks and extras

- **Round things look round**: the ball drawn analytically; per-pixel lighting with smooth normals;
  normal-guided tessellation for rails and pipes. Visual only: physics stays on the original polygons. Which
  techniques, and how far, is decided with the owner after trying them.
- **Local texture enhancement**: scripts the player runs on their own copy to upscale textures (up to 8K);
  the repository holds the pipeline and its parameters, never the results.
- **Replays rendered to video**: offline, frame by frame, at any resolution up to 8K with sound. Nothing is
  uploaded without the owner's command.

## Throughout

- The security rules (`CLAUDE.md`): hostile input, pinned and vendored dependencies, CI actions pinned by
  commit SHA with minimal `permissions:` (upstream's workflows pin by tag and must be fixed or replaced).
- Data-free tests run everywhere (unit, synthetic files, fuzzing); game-data tests run locally.
- Documentation in three languages. Upstream's English-only documentation is reference material and is not
  translated unless it becomes ours.
