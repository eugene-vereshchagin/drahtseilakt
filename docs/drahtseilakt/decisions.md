# Decisions

Why the fork is built the way it is. Each entry: the decision, then the reasons. Newest decisions go at the
end; a decision that is replaced is marked as such, not deleted.

## Fork OpenBallance, don't rewrite

The upstream code is clean C11 (~27k lines) with an honest split between a portable core and a host: the
core needs only `src/platform.h` and `src/render/gpu.h`, and the whole gasm binding is one file. It already
runs the original Virtools graphs, implements the ~180 Building Blocks the game uses and ports the Ipion
physics. The hard parts are done; what is missing (native host, resolutions, level 13, maps) sits on top.

## Keep C for the engine

Rewriting working code in another language buys nothing for the player and costs months. C is also the most
portable choice for the targets we have.

## SDL3 for the host, wgpu-native for graphics

SDL3 is the de-facto standard for windows, input, gamepads and audio on Linux, Windows and macOS, packaged by
the distributions. wgpu-native is WebGPU as a native library with a C API, running on Vulkan, Metal and
Direct3D 12; the renderer already speaks WebGPU and WGSL, so it moves over almost unchanged.

## Two level formats, side by side, permanently

The engine reads the original `.nmo` / `.cmo` and, later, an open format of our own. `.nmo` support is not
transitional: about 425 community maps exist only in it, many with their own scripts. The open format is
for editing and making levels. Original and converted copies of the same level double as a test of the
converter.

## No game data in the repository, upscaled or not

Ballance's rights did not die with Cyparade: the game was re-released on Steam in January 2024 (Ziggurat
Interactive) and is sold on GOG. Original assets and anything derived from them (upscaled, retouched) stay
out of the repository. Besides the law, a DMCA notice on GitHub disables the whole repository, not a folder.
Higher-resolution textures come from a reproducible pipeline that runs on the player's own copy.

The repository is private for now (2026-10-30), which removes the takedown risk during development; the
rule stays because the code is meant to be published.

## A public release has its own content

A release for everyone ships the engine with newly made textures and levels — drawn anew in the spirit of
the original, not derived from its pixels — and uses the original game's data when the player has a copy
(the Freedoom model). Retouching original textures does not make them ours.

## Fixed 60 Hz simulation, interpolated rendering

The physics and every community map are tuned to the original 60 Hz step. The simulation always ticks at
60 Hz; only drawing follows the display, interpolating between ticks. A variable step would change how the
ball behaves and break maps.

## The ball is drawn analytically

A sphere drawn per pixel by ray intersection is round at any resolution, which a mesh never is. The set of
sphere-drawn objects comes from data, so this is a renderer feature, not a special case.

## Security rules apply to everything

The owner's secure-codegen rules (https://github.com/eugene-vereshchagin/secure-codegen-rules) govern all
code. The weight falls on the parsers: community maps are untrusted files from strangers, so the `.nmo`
reader is fuzzed and fails closed.

## Workflow

Development runs with a local agent on the owner's machine (which has the game data); changes go to git after
they have been tested there. CI runs on demand and on pull requests. Nothing is released, tagged or published
without the owner's explicit command.

## The name

*Drahtseilakt* is German for a tightrope act, and figuratively for any balancing act where one step aside
means a fall — which is what rolling a ball along a rail over an abyss is. German as a nod to Cyparade, the
German studio that made Ballance.

## Documentation in two languages

*Replaced by "Documentation in several languages" below.*

Documentation is written in English and Russian. English is the primary version (it is the language of the
code, comments and commits); the Russian one mirrors its structure under `docs/ru/` and in `README.ru.md`.
Both change in the same change. The site is built from `docs/` with two locales.

## Documentation in several languages

Documentation is written in English, Russian and German. English is the primary version (the language of the
code, comments and commits); the others mirror its structure under `docs/<lang>/` and in `README.<lang>.md`.
All versions change in the same change. The site is built from `docs/` with one locale per language. German
because the game and the name are German. (Replaces the entry on two languages.)

## Open: OpenBallance or Ballanced as the base

Found after the fork was made: [Ballanced](https://github.com/doyaGu/Ballanced) rebuilds the complete Virtools
runtime natively on SDL3 and may already do much of phase 1. The choice between staying, moving and borrowing is
made after phase 0 has actually run it on Linux, not from its README. Until then this entry stays open.

## Visual smoothing never touches physics

Making curved geometry look round (phase 3) is a rendering matter only. Collision geometry stays exactly the
original's, because the ball's behaviour and every community map are tuned to it. Which smoothing techniques
are used is open until they have been tried; this constraint is not.

## The repository stays private until the game is playable

Nothing is shown publicly — repository, site, videos — until all original levels (the twelve and level 13)
are playable natively. Raw code is not shown. Going public is the owner's command, as with any release.

## The game never goes online

The finished game makes no network connections: nothing is downloaded at start or while playing — no fonts,
shaders or other resources on demand, no update checks, no telemetry. Everything it needs ships with it (fonts
included, with their licences) or comes from the player's own copy of Ballance. Community maps and mods are
files the player puts in place; the game does not fetch them. Build-time dependencies are a separate matter:
the build may download them, pinned and checked against a checksum, and must also work offline from local copies.
