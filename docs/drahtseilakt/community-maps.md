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

## Rules for this project

- **Maps are untrusted input**: files from strangers' drives. They go through the hardened, fuzzed parsers only
  (see `CLAUDE.md`).
- **Map files never go into the repository** — they are their authors' work, not ours to redistribute under
  the GPL. A selection is kept as data only: name, author, date, catalogue row, link.
- Expected looks and routes come from the catalogue's videos, not from what our engine happens to show.
