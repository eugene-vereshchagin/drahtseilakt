# Decisions

Why the fork is built the way it is. Each entry: the decision, then the reasons. Newest decisions go at the
end; a decision that is replaced is marked as such, not deleted.

## Fork OpenBallance, don't rewrite

*Replaced 2026-10-31 by "Everything from scratch, in Rust" below.*

The upstream code is clean C11 (~27k lines) with an honest split between a portable core and a host: the
core needs only `src/platform.h` and `src/render/gpu.h`, and the whole gasm binding is one file. It already
runs the original Virtools graphs, implements the ~180 Building Blocks the game uses and ports the Ipion
physics. The hard parts are done; what is missing (native host, resolutions, level 13, maps) sits on top.

## Keep C for the engine

*Replaced 2026-10-31 by "The language serves the code" below.*

Rewriting working code in another language buys nothing for the player and costs months. C is also the most
portable choice for the targets we have.

## SDL3 for the host, wgpu-native for graphics

*Replaced 2026-10-31 by "Everything from scratch, in Rust" below (graphics stay on wgpu, as the Rust library).*

SDL3 is the de-facto standard for windows, input, gamepads and audio on Linux, Windows and macOS, packaged by
the distributions. wgpu-native is WebGPU as a native library with a C API, running on Vulkan, Metal and
Direct3D 12; the renderer already speaks WebGPU and WGSL, so it moves over almost unchanged.

## Two level formats, side by side, permanently

The engine reads the original `.nmo` / `.cmo` and, later, an open format of our own. `.nmo` support is not
transitional: about 425 community maps exist only in it, many with their own scripts. The open format is
for editing and making levels. Original and converted copies of the same level double as a test of the
converter.

## No game data in the repository, upscaled or not

*Amended 2026-10-31: see "The private repository is a backup" below.*

Ballance's rights did not die with Cyparade: the game was re-released on Steam in January 2024 (Ziggurat
Interactive) and is sold on GOG. Original assets and anything derived from them (upscaled, retouched) stay
out of the repository. Besides the law, a DMCA notice on GitHub disables the whole repository, not a folder.
Higher-resolution textures come from a reproducible pipeline that runs on the player's own copy.

The repository is private for now (2026-10-30), which removes the takedown risk during development; the
rule stays because the code is meant to be published.

## A public release has its own content

*Replaced 2026-10-31 by "The game plays only the player's own data" below.*

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

Documentation is written in English, Russian, German and Simplified Chinese (added on 2026-11-02 for the large Chinese community). English is the primary version (the language of the
code, comments and commits); the others mirror its structure under `docs/<lang>/` and in `README.<lang>.md`.
All versions change in the same change. The site is built from `docs/` with one locale per language. German
because the game and the name are German. (Replaces the entry on two languages.)

## Open: OpenBallance or Ballanced as the base

*Closed 2026-10-31 by "Everything from scratch, in Rust" below: neither.*

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

## The game plays only the player's own data

Replaces "A public release has its own content". We make no substitute content: the game plays the data of
the player's own copy of Ballance, original levels and community maps alike. Improvements to that data —
textures upscaled up to 8K, for instance — come from scripts the player runs locally on their own copy; the
results stay on their machine and are never distributed by us.

## Native levels and original `.nmo`, both playable

The game is new and has its own level format; maps are converted into it. Players can still play any `.nmo`
as it is — the twelve levels, level 13 and every community map — so a conversion that fails or falls short
never takes a map away from anyone. This serves both those who want the original exactly and those who want
something new. It sharpens "Two level formats, side by side, permanently": both formats are played, not only
read.

## The private repository is a backup; the public one starts clean

While the repository is private it may hold whatever the work needs, original levels and other game data
included, as a backup. The public release is never this repository: it is a new repository, created from a
cleaned tree without this history, checked for game data before the first push. Until then the rule from
"No game data in the repository" still applies to everything meant to be published: code, tests and docs
do not depend on game data being in the repository.

## No browser build

The browser (gasm/WebAssembly) build came with upstream and is not a goal of this project. It is no longer a
target: no change has to keep it building, and it goes when the code it needs is replaced. The targets are
native Linux, Windows and macOS.

## The language serves the code

*Replaced 2026-10-31 by "Everything from scratch, in Rust" below.*

Replaces "Keep C for the engine". C and C++ are both fine — whatever the code we build on is written in, and
whatever keeps the sources clear. What matters is that everything is in source form and the project is in
order, not the language.

## Everything from scratch, in Rust

Closes phase 0 and replaces "Fork OpenBallance, don't rewrite", "SDL3 for the host, wgpu-native for
graphics" and "The language serves the code". The engine, the game, the converter and the tools are written
anew, and none of the code of other projects is taken.

The reason is provenance. Ballanced's physics is the leaked Ipion source from the Source engine ("All rights
reserved", no licence), and two of its modules have no licence at all; OpenBallance's physics was ported from
our own decompilation of the original DLL. Taking pieces means inheriting those questions, and translating
code into another language does not settle them: a translation is a derivative work. Written from scratch,
every line has a clear origin.

Why Rust: the engine's main risk is hostile input (community maps from strangers' file shares), and parsers in
safe Rust cannot read out of bounds or corrupt memory — a whole class of bugs the security rules otherwise
fight by hand. wgpu (Vulkan, Metal, Direct3D 12) is native to it, and cargo gives pinned, vendored, offline
builds and fuzzing on all three platforms.

## The physics is ported from OpenBallance (GPL-3.0)

Replaces "Rigid-body physics comes from a library: rapier" for the ball and the level's bodies. OpenBallance's
physics (`src/phys`, about 9,600 lines of C) is a port of the original game's `physics_RT.dll` written from our own
decompilation (`docs/physics.md`), and the project is GPL-3.0 as we are. Porting it to Rust is a derivative work
allowed by the GPL: we keep the copyright notice of the upstream author and the licence, mark each ported file with
its origin, and credit OpenBallance in the README. It is the closest thing to the original's feel that exists in
open code, and it spares us months of tuning by hand.

This is not an exception for Ballanced. Its physics is the leaked Ipion source ("All rights reserved") and it is
not used, in any language. rapier stays only as a comparison bench until the port matches the original.

## Rigid-body physics comes from a library: rapier

Replaces the rule that the physics is written by hand. The own solver for boxes and balls sank boxes, let them
pass one another and lost them; a ready, tested rigid-body engine does this properly. We depend on rapier3d 0.36
(Apache-2.0, compatible with GPL-3.0, maintained by dimforge), as we depend on wgpu and winit: a library, not
code taken from another game project. Our part is what the game is: the level as one static mesh, the original
game's material values with pair friction and elasticity as products (docs/physics.md, sections 3 and 4), the
player's control impulse, and one step per tick so that a replay reproduces.

## Other projects are references and test benches

OpenBallance, Ballanced and the original game under Wine are used to study behaviour and to compare against:
running them, reading their documentation, measuring them. No code is copied from them. The leaked Ipion
source is not opened at all: the physics is built from the original game's measured behaviour. The
reverse-engineering notes in `docs/` (file formats, Building Blocks, how the game behaves) stay as reference.

## Sound goes out through cpal

*2026-11-01.* The game plays the player's WAV files through cpal 0.18 (Apache-2.0, RustAudio): ALSA on Linux
(PipeWire and PulseAudio take it through their ALSA layer; the build needs the ALSA headers, `libasound2-dev` on
Debian), WASAPI on Windows, CoreAudio on macOS. It only opens an output stream; reading WAV, mixing, volume and
pitch are ours (crates/game/src/audio.rs), which keeps us close to what the original's sound scripts do. rodio
and kira bring decoders and mixers we do not need; miniaudio, SDL3 and OpenAL are C libraries. The crate is
pinned and its sources sit in the cargo cache, so the build stays offline.

## Levels live in one folder, four kinds

*2026-11-02.* All levels sit in `levels/` in the game's data folder, in four folders by origin and format:
`original/` (the player's copy of the original game, copied there by default), `community/` (community `.nmo`
maps), `remastered/` (the original levels converted to the open format with modern textures, made on the
player's machine) and `open/` (levels in the open format, own or converted). Any of them can be moved in the
settings; the default folder then keeps a README that says where they went. The classic mode sees only the
original. Some open-format levels come with the game when their authors want it, with a free licence and none of
the original's assets inside. The community is largest in China, so the docs are in Simplified Chinese too.
Details: [levels.md](levels.md).


## Shipped levels are new content

*2026-11-02.* "The game plays only the player's own data" forbids substitutes for the original's files. Levels
whose authors let them come with the game, under a free licence and with none of the original's assets inside,
are new content and may ship (levels.md). What players copy into their own folders is their business, not ours.


## The toolbox prepares levels; the game only plays

*2026-11-02.* Making remastered levels is a separate program, `drahtseilakt-toolbox`, in a repository of its own
(eugene-vereshchagin/drahtseilakt-toolbox, beside this one), shown and shipped as its own program; it shares the
level reading with the game through this repository's `crates/gamedata` and `crates/level`, taken by path.
The game plays the original `.nmo` files and whatever open-format levels are in its folders and knows nothing of
how they were made; it never runs another program. The toolbox converts the originals into `levels/remastered/`,
writes each curved mesh's rounder look into the level (the open format keeps it beside the mesh, which the physics
keeps using, so runs are the same) and makes the textures larger. Real-ESRGAN, which adds detail, is used when the
player has installed it; it is never downloaded and never a dependency of the game, so a plain player needs
nothing more than the game. Whether the toolbox's archive later carries Real-ESRGAN (its licence allows it,
pinned and checksummed) is decided at the release.


## Own content by default, the original's on the player's request

*2026-11-02.* This replaces "the game plays only the player's own data, no substitute content". The game ships with
its own interface and parts (menus, font, interface sounds, balls, modules where a level needs them), made by us as
close to the original as we can afford: own drawings and sounds, freely licensed fonts, nothing of the original. In
the extended mode a switch takes the original's instead, from the player's copy, or its remaster made by the
toolbox. Levels can be shared in two ways: with their own assets under a free licence, or, for players who want the
original's look, as levels that name the original's resources, which the toolbox attaches from each player's own
copy. The toolbox makes every resource the game can use out of the player's original files. So nobody's property is
shipped, and every player can put the original together on their own machine in a couple of clicks.

## The own interface is the default

*2026-11-03.* Following "Own content by default": the game ships its own interface (menus, in-game interface, end and
loading screens: our textures drawn by a script in the repository, the freely licensed Tektur font, our texts), and it
is what every player sees by default, in the classic mode always. The extended mode's Interface switch chooses the
original's from the player's copy, or its remaster. The own menus speak seven languages (English, Russian, German,
Simplified Chinese, French, Spanish, Italian): a Language page in Options, in the classic mode too, since a player who
cannot read the menus cannot play; the default is the system's language, else English. With the original's interface
and a language its copy lacks (Russian, Chinese), our texts are drawn in the shipped fonts, as its Latin-1 font cannot.
Tektur was chosen over Michroma (closer to the original's wide letters, but Latin only) so that every Latin and
Cyrillic text has one design.

## The generated sound theme is the default

*2026-11-02.* The owner's decision: the generated sound theme (*neuro*: effects synthesized by code, tuned against
measurements of the original's sounds, nothing of the original in them) ships as the game's default sound theme. It
plays until the player chooses another theme on the extended page, and always in the classic mode. *system* stays
the theme every theme falls back to and stays selectable, as do *original* and the player's own themes. A choice the
player made is kept; the settings write `sound-theme` only once the player has chosen, so a player who never chose
follows the default.

## A level is a folder

*2026-11-03, the owner's decision.* The custom levels menu showed whatever lay in `levels/community/`: archive dumps
with folders of folders, pictures, documents, packed archives. Now a level is a folder directly in the kind's folder
holding exactly one level, a `.nmo` or a level in the open format (the clear format, easy to finish or fix up by
hand), with an optional `level.txt` (title, author, version), README, preview picture and the resource folders it
uses; anything else and any deeper nesting is ignored, and a folder with no level or several is not one. A single
loose `.nmo` in the kind's folder is a level too, the simplest drop-in. Players may name a level in level.txt, but
a title is cleaned and capped at 40 characters (cut at a word, "…"), in the menu and wherever the game shows the
name, so a title can never break the interface. The menu is a flat list by title. Collections are not a concept: a
series becomes one folder per level. `drahtseilakt-toolbox import-maps` sorts a download into this shape. Details:
[levels.md](levels.md), "A level is a folder".

## The copy's interface by default while ours is redone

2026-11-03, owner: the first own interface (purple pill buttons, a painted background) had no likeness to the
original. Until the redone one convinces, the default — and the classic mode — plays the copy's own interface when
the player's copy has its menu, else ours. Nothing of the original is shipped by this: it comes from the player's own
copy, as the levels do. The interface no longer decides the sound theme.

## The game plays out of the box

*2026-11-03, the owner's release condition.* An original level's layout and its parts' models (`3D Entities/PH/*.nmo`,
`Balls.nmo` and the rest) are Cyparade's and Atari's work; its mechanics are free. So before the release the game
needs its own parts and its own levels, and it must ship in working order *before* anyone imports anything: it
starts and plays with no copy of the original at all. Without a copy the menu offers our own levels (those shipped
with the game and those in `levels/open/`) and the import of a copy; everything a run needs (interface, sounds,
fonts, menu scene, the balls and their pieces, the modules, skies, particles, the ball's shadow, default keys, a
level's settings) comes from what the game ships. Our parts are made by scripts in `assets/own/make` to the
original's measured sizes, with its physics values unchanged, so they behave as the original's do; nothing of its
geometry is copied. Release criteria: [roadmap.md](roadmap.md), "Release criteria".

## Own textures by default

2026-11-03, the owner's decision: our own textures for the levels, their parts and the balls (made by a script in the
repository from CC0 photo textures, graded to the original's measured palette, grain and layout; nothing of the
original) are what every level is drawn with by default, in the classic mode always; the extended page's Textures
switch (`textures original`) draws the copy's. The owner chose to have them applied without reviewing each first:
one that does not please is replaced later. Ours stand in by the original's texture name, whichever level asks (an
original level, a community map, a part); what we have no texture for (skies, particles, glows, shadows, a few
pictures) still comes from the copy. An open-format level's own textures stay its maker's. Exporting a level with
`--textures own` writes ours into it, so it can be shared with free assets only. (Refined after review: in a stranger's map only the copy's own textures are replaced, never its own pictures; the
remasters of the copy's levels are replaced like the copy's; the menu scene follows the interface.)

## Texture and model packs

2026-11-03, the owner's request: what the levels look like comes in packs the player chooses, as the sounds come in
themes ([packs.md](packs.md)): texture packs (built in: `own`, `original`, `remastered`) and model packs (built in:
`level`, `own`, `original`), each its own line on the extended page, and any folder a player drops into the data
folder's `textures/` or `models/` is a new pack. A pack only replaces what it has; what it lacks comes from its fallback,
then the default. A model pack changes only the meshes drawn: the physics keeps the measured shapes (the original's for
the copy's levels, ours for our levels), so play and replays never change. Our own levels name the textures they share
as the game's (`game:` images, open-format.md) instead of carrying copies, so packs change them too and each level is
about half the size. The classic mode stays as it was: our textures, the parts each level takes. `original` now means
the copy's pictures in a remastered level too; the remaster's own upscaled pictures are `remastered`.

## Our own data-only mod format

2026-11-03, the owner's decision: the game has mods — additions of behaviour and features each player switches on
or off — in a format of our own, and the features themselves are mods: the game ships *factory mods*
(`assets/mods/`) written in the very format a player uses, which serve as the examples ([mods.md](mods.md)). A mod is
data: a mod.txt of rules (`on <event>`, `if <condition>`, `do <action>`) over a fixed library of the engine's
building blocks, with options, labels, variables and small total expressions, interpreted by our runtime as the
original's behaviour graphs are by its runtime. Nothing in a mod is run as code: no scripts, no paths, no links, no
assets; a mod folder holds a mod.txt and at most a readme and a licence, and anything else refuses it. Strangers'
mods are hostile input: strict parsing, caps on every count, an error naming the line. Rules on the run's events read
only the run and are the only ones that may change it, so runs stay deterministic; a mod that changes the run marks
it modded, and its replay names the mod with its hash and options (never a best run, never a highscore; classic runs
and their replays are unchanged). Sound themes, texture and model packs and levels are not mods: they keep their own
menus. BML mods are Windows DLLs: they were surveyed statically as requirements and are never loaded, run or
imported — not even under Wine.

## God mode is a development mod

*2026-11-03, the owner's decision* ("our debug tools should become a special mod, god mode, with free saves, endless
lives and going back; we may not ship it with the main game at all, so playing is more interesting"). The extended
mode's practice tools (save and load the moment, seconds back, slower and faster, the free camera, restart, the sector
and tick on screen) and endless lives are no longer additions of the game: they are one mod, `god-mode`, written in our
data-only mod format from building blocks the format gained for it (`lives.endless`, `rewind`, `save`, `load`,
`slower`, `faster`, `freecam`, `keys`), so its keys are options on its page and on screen while it is on. It is a
*development mod*: it lies in `assets/mods-dev/`, read only in a run started with `--debug`, and a release's package
leaves that folder out. "All levels open" is likewise a debug run's only; a player's game unlocks the levels by playing
them. A god-mode run changes the run, so it is modded: kept, but it opens no level and is never a best run or a
highscore, and its replay names the mod. Details: [mods.md](mods.md), "God mode".

## Levels are listed by difficulty, with English titles

*2026-11-03, the owner's decision* ("tidy up the custom levels into a format, so it isn't a dump"). A level's level.txt
gives its `difficulty`, a whole number 1 to 10 (anything else is unknown), and its `title` in English, with its names in
other languages as `title.<code>`. The menu shows each level's difficulty and title and sorts by difficulty, then title;
levels of unknown difficulty after, levels with no title (named by their folder) last. Our ten levels are rated 1 to 10.
The toolbox fills level.txt from the community's catalogue (its English name, the original name, the author, its 1–5
difficulty doubled), and what is neither catalogued nor comes with a readme, archives and folders that are not one level
are set aside in `levels/community-unsorted/` for the player to look at by hand, not listed. The rules for authors and
players are on one page: [making-levels.md](making-levels.md).

## Classic mode offers the look choices

*2026-11-09, the owner's decision.* The classic mode also offers the look choices that were the extended mode's alone:
*Textures* and *Models* on its Graphics page and *Sound theme* on its Sound page (the original's lines first, in
their order; ours below, like *Fullscreen?*), with the same settings keys (`textures`, `models`, `sound-theme`). Every
pack found appears: the built-in `own`, `original` (only when the player's copy is present), `remastered`, and any
pack in the data folder, so resources a player converts himself just show up. The defaults are unchanged (own
textures, the level's parts, the default theme). Gameplay and rules stay the original's, and replays do not depend on
the look. Rendering fixes (any resolution, widescreen, filtering, smoothing) are always on and not switchable. This
supersedes the earlier notes that the classic mode always draws our textures and the parts each level takes and always
plays the default theme.

## Start by meaning

*Owner's decision, 2026-11-09.* Start no longer shows levels by folder (Original, Remastered, Open format, Modified
originals, Community picks, My maps) but by meaning, the same in the classic and the extended mode, with an item left
out when it is empty or unavailable: *Classic Ballance* (the 13 originals of the player's copy), *Drahtseilakt* (our own
13 levels), *Ballance Challenges* (the community's remakes of the originals, formerly "Modified originals"; needs the
copy) and *Community maps* (a submenu: *Best*, the 13 shipped picks with their authors, and *Downloaded*, the player's
folder). The player's folder is `levels/downloaded/` and takes both community `.nmo` maps and open-format levels;
`levels/community/` and `levels/open/` are still read next to it and never moved, and the keys under which runs are
kept follow the folder a level is in, so existing runs stay. *Remastered* leaves Start: it becomes the Options line
*Look: original / remastered* (shown only when an export exists), a setting rather than a kind of level; phase 2 turns
the remaster into texture and model packs. Every item says on the notes panel what it is and where it comes from.
