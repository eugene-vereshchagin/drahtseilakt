# Community maps: where they are and how to use the catalogue

Reference for phase 4 of the [roadmap](roadmap.md) and for anyone (human or agent) who needs community maps
or data about them. Snapshot of the catalogue: 2026-09-25.

## Sources

| What | Where | Notes |
|---|---|---|
| Catalogue (the list) | [pseudoacetyl/BallanceCustomMaps — `CustomMapList.md`](https://github.com/pseudoacetyl/BallanceCustomMaps/blob/main/CustomMapList.md) | MIT-licensed repository; machine access below |
| All maps (files) | [Mega: Custom Maps](https://mega.nz/folder/ARR3RRBL#kvoVNZVOxbUmpDBUqLUccg) | from the catalogue's author |
| Curated pack | [MediaFire: Ballance Map Packs](http://www.mediafire.com/folder/vyzo7893i0oln/Ballance_Map_Packs) | "high-quality" maps, version 7.6; updated only every few years, so recent maps are missing |
| Players, mods | [Mega: Players](https://mega.nz/folder/EIYiyASK#GGanOfhFVcSWjnjxOzJjIQ), [Mega: Mods](https://mega.nz/folder/wEB11B5D#FhU0JpzqomuFCAkMuBePWA) | |
| New maps | Ballance International Discord, channel `#new` | how-to: [Discord FAQ](https://yyc12345.github.io/ballance-discord-rules/en/faq/) |
| Chinese community | Baidu Tieba (Ballance bar), Chinese Ballance Forum; videos on Bilibili | the biggest audience; the catalogue's IDs point there |

**Access from an agent.** The catalogue is plain Markdown on GitHub:
`git clone --depth 1 https://github.com/pseudoacetyl/BallanceCustomMaps` or
`https://raw.githubusercontent.com/pseudoacetyl/BallanceCustomMaps/main/CustomMapList.md`.
Mega and MediaFire are usually not reachable from cloud sandboxes; map files are downloaded by the owner on his
machine.

## The catalogue's format

One Markdown table, one row per map, oldest first:

| Column | Content |
|---|---|
| Posted | release date `YYYY-MM-DD`; in parentheses when approximate, `(~…)` when estimated |
| Baidu Tieba ID & Chinese Ballance Forum ID | post IDs; Forum IDs look like `C126`; may be empty or `N/A` |
| Original Name | usually Chinese |
| English Name | translated |
| Author | |
| Difficulty | 1–5, sometimes halves (`2.5`) or ranges (`2.5–3`) |
| Type | Comprehensive, Technical, Logical, Casual, Skillogic, Racing, Puzzle, Landscape |
| Videos | YouTube links, several separated by `<br />`; may be empty |

Parse it defensively: a few rows contain a `|` inside a name, which shifts the columns (in the 2026-09-25 snapshot
a handful of rows have an author or name where the difficulty should be). Validate each row and report the
malformed ones instead of guessing.

## The collection in numbers (2026-09-25)

- About 427 maps, posted from 2009 to 2026; 20–60 new maps a year.
- Difficulty: roughly 75 at 1, 75 at 2, 100 at 3, 100 at 4, 65 at 5.
- Types: about half Comprehensive; then Technical (~85), Logical (~55), Casual (~30), Skillogic, Racing, Puzzle,
  Landscape.
- Videos: 366 maps have at least one, **99 have two or more**. Maps that several people played and published
  are a natural "top 100" — the first selection to test with.

## Community conventions for installing maps

- **Replacing a level**: copy the map over one of `3D Entities/Level/Level_01.NMO` … `Level_13.NMO`. No mods needed.
- **BML**: put maps into `ModLoader/Maps`; BallanceModLoader(Plus) adds a map menu.
- Our engine uses neither: maps go into its own maps folder and are never written into the game's files
  (phase 4).

How maps are made: Blender 3.6 with [BallanceBlenderHelper](https://github.com/yyc12345/BallanceBlenderHelper),
exported through Virtools 3.5; older maps through 3ds Max plugins.

## Local copies on the owner's machine

The owner has downloaded the maps and the mods. They live **outside the repository**, like the game data, and
are found through environment variables:

| Variable | Points at | Used by |
|---|---|---|
| `OPENBALLANCE_DATA` | the game: installed folder, CD folder or disc image | game-data tests, the game |
| `OPENBALLANCE_MAPS` | a folder of community maps (`.nmo`, as downloaded, archives unpacked) | map tests, the missing-Building-Block report |
| `OPENBALLANCE_MODS` | a folder of downloaded mods | reading only, see below |

Unpacking the downloads is part of the work and follows the archive rules (`modules/input.md`): every path
inside an archive is checked (no `..`, no absolute paths, no links), with limits on file count and unpacked
size. The engine's own maps folder for players (phase 4) is separate from this development corpus.

**Maps** are data. They are what phase 4 loads and what the tests run over: a headless pass over the whole
folder that loads each map, plays it for a while and reports crashes and missing Building Blocks.

**Mods are code.** BML mods are Windows DLLs that the original player loads into its own process. They are
never executed — not by the engine, not by a test, not "just to see" under Wine on the owner's machine. They
are useful only as a source of requirements: what the community added to the game (speedometers, map menus,
camera options, fixes) tells us which features players want. Such a feature is reimplemented in our engine from
its observable behaviour, the same way as any other; nothing is loaded from a mod.

## What a map carries beyond its scene

Found by the audit (`drahtseilakt --audit DIR`) over 606 maps; `nmo-dump --extras FILE` lists them for one map.

- **2D frames** (34 maps): an author's signature or logo in a corner of the screen. A frame the file shows, whose
  material has a texture, is drawn over the picture under the game's interface, where the file puts it in the
  original's 640 x 480 context, by Z order (at most 16 a map). A hidden frame or one without a texture is what a
  script of the map shows or fills: not drawn. **Not drawn yet either:** a frame whose picture is saved inside the
  map (nearly all of them): the reader takes that bitmap for packed pixels where the file has it in planes, so it
  would come out tiled and upside down. An open-format level does not carry frames yet.
- **Textures without a picture** (31 maps): a texture with no pixels and no file name, named after the game's
  texture, often cut to a DOS name by an exporter (`METAL_ST`, `RAIL_ENV`, `WOOD_MET.Copy`), or with no name under
  a material named after one (`_aaaSkyLayer`). It is taken for that game's texture (ours, a pack's or the copy's).
  Where a cut name fits several, the shortest is taken (`FLOOR_TO` is `Floor_Top_Flat`): a guess, listed by the
  audit as `texture-by-name`. The original draws these without a texture.
- **Textures nobody has** (9 maps): a file the author did not put into the map. Drawn in the material's colour,
  toned down, never blank white.
- **Sounds** (8 maps): read (file, loop, gain), but the files are not in the maps and only the maps' scripts play
  them: not played.
- **Curves** (16 maps): shapes an exporter left; no block and no parameter of any map reads one. Not drawn.
- Not read: patch meshes (2 maps), MIDI sounds (3 maps).

## Rules for this project

- **Maps are untrusted input**: files from strangers' drives. They go through the hardened, fuzzed parsers only
  (see `CLAUDE.md`).
- **Map files never go into the repository** — they are their authors' work, not ours to redistribute under
  the GPL. A selection is kept as data only: name, author, date, catalogue row, link.
- Expected looks and routes come from the catalogue's videos, not from what our engine happens to show.
