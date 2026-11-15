# Making and sharing levels

What a level's author and a player need to know to put a level where the game finds it, name it so the menu shows
it well, and share it. The details behind each rule are in [levels.md](levels.md) (the folders), the
[open format](open-format.md) and [menu.md](menu.md) (the list); the owner's decisions behind them in
[decisions.md](decisions.md) ("A level is a folder", "Own content by default").

## Where a level goes

All levels live in `levels/` in the game's data folder (`~/.local/share/drahtseilakt/` on Linux,
`%APPDATA%\drahtseilakt\` on Windows, `~/Library/Application Support/drahtseilakt/` on macOS). A level of your own or
someone else's goes into `levels/user-levels/`, either of two kinds:

- community maps (Virtools `.nmo` files, the original's format);
- levels in the [open format](open-format.md) (plain glTF and PNG files, easy to finish or fix up by
  hand).

**One level, one folder, directly in that folder:**

*Since 2026-11-12 `levels/user-levels/` is the one recommended folder (the earlier `downloaded/`, `community/` and `open/` were moved into it at the first start, nothing deleted).* In the menu: *Start, Community maps, User levels*.

```
levels/user-levels/
  Green Dam/                 the level's folder: a plain name, at most 40 characters
    Green Dam.NMO            the level: exactly one .nmo ...
    level.txt                its name, author, difficulty (below; optional, but without a title it is listed last)
    README.txt               optional: what it is, how to play it, its licence
    preview.png              optional: a picture of it (preview.jpg also)
levels/user-levels/
  Sky Road/
    level.gltf level.bin     ... or one level in the open format
    textures/                its own textures (PNG), directly in textures/
    game/                    the game's files it carries (its parts, sky, sounds: open-format.md)
    level.txt  README.md  preview.jpg
```

**The rule:** a level counts as a level only when it lies in its own named folder that holds either one `.nmo` file or an open-format level (`level.gltf` ...), plus the description file `level.txt`. Subfolders for grouping are fine (`levels/user-levels/Pro Racing/Season 1/Green Dam/`). A level folder without `level.txt`, and a loose `.nmo` straight in `user-levels/`, are not listed; the game's log names each one and says what to do. (Earlier a loose `.nmo` was a level of its own and level.txt was optional.)

## level.txt

Plain UTF-8 text, one `key = value` a line, `#` begins a comment; unknown keys are ignored. At most 4 KiB and 64 lines
are read; a larger file or a link is ignored as if there were none.

| Key | What | Rules |
|---|---|---|
| `title` | the level's name, **in English** | needed to be listed among the named levels; at most 40 characters (cut at a word, "…") |
| `title.<code>` | its name in another language: `title.ru`, `title.de`, `title.zh`, `title.fr`, `title.es`, `title.it` | shown when the menus speak that language, else the English title; a code is 1–8 lower-case letters; at most 16 of them |
| `author` | who made it | at most 40 characters |
| `version` | its version | at most 40 characters |
| `difficulty` | how hard it is | a whole number, **1** (easy) to **10** (hardest); anything else (0, 11, 3.5, "hard") is read as unknown |
| `order` | its place in a list of one author's levels | a whole number; levels of the same difficulty with one come first, in its order |
| `sky` | the sky the map plays under | a name of the game's sky boxes: `A`, `B`, `C` or one of ours (`Mountains`, `Forest`, `Sea`, `Storm`, `Sunset`, `Night`, `Desert`, `Mist`, `Clouds`); it wins over the level file's own and the original's table, which is only read when a copy is there and the map says nothing |
| `music` | the music theme | a whole number 1 to 5 |
| `light` | the colour of the light | three numbers 0 to 1, `1 0.9 0.8` |
| `hidden` | `yes`: kept out of the list (a level for tests) | still opens with `--open` |
| `source` | where the toolbox found it | written by `import-maps`; not shown |

Every value is cleaned: control characters, direction overrides and other invisible format characters are removed,
runs of spaces become one. Our own levels' level.txt, for example:

```
title = Paper Winds
title.ru = Бумажные ветра
title.de = Papierwinde
title.zh = 纸之风
author = Drahtseilakt
version = 1
licence = GPL-3.0-only
order = 3
difficulty = 3
```

## Names

- **The folder's name** is plain: letters, digits, spaces and `-_.()` work everywhere; it must not be one of the
  names Windows reserves (`CON`, `NUL`, `COM1`...), end in a dot or a space, or begin with a dot (hidden). At most 40
  characters. The toolbox makes every name it writes safe for Linux, Windows and macOS.
- **The title is English.** A level from another community keeps its own name as `title.<code>` (a Chinese map's as
  `title.zh`), so every player can read the list and the original name is not lost.
- Keep the version in `version`, not in the title, where you can ("Lost City", `version = 1.1`).

## What the menu shows, in which order

Under *Start → Community maps → Downloaded*, a flat list, one row a level: a star and its
difficulty (★7; ★? when unknown) and its title (in the menus' language when level.txt has it). The list is sorted:

1. by difficulty, easiest first; levels of unknown difficulty after all those with one;
2. among the same difficulty, by `order` where given, then by title;
3. levels without a `title` in level.txt (named only by their folder) last.

A long list scrolls, with a bar and "▼ N more". A title too long for its row is drawn smaller or loses its middle
("..."); it never leaves its row.

## What is ignored, what is set aside

The menu looks only at the folders directly in `community/` and `open/` and only at the level in each. It ignores,
without harm: any other file in a level's folder, every folder inside it (no nesting: a collection is one folder per
level, "45-Degree Rotated - Level 07"), hidden names, links (never followed), and a folder with no level or with
several (several `.nmo`, or a `.nmo` beside `level.gltf`): such a folder is not a level and is named on the console.

`levels/community-unsorted/` holds what the toolbox set aside for you to look at by hand: packed archives
(`.zip`, `.rar`, `.7z`, `.tar.gz`...), folders that are not one level, and maps that are neither in the community's
catalogue nor come with a readme. Its `README.txt` lists each and why. The game never lists it; unpack, sort and name
what you want to keep, give it a level.txt, and move it into `community/`.

## Importing a collection with the toolbox

The toolbox (`drahtseilakt-toolbox`, a separate program; the game itself never runs it) sorts a downloaded collection
into this shape:

```
drahtseilakt-toolbox import-maps ~/Downloads/Maps --catalogue CustomMapList.md
```

- every `.nmo` under the folder (folders of any depth) becomes a level folder in `levels/user-levels/` (`--to DIR` for
  another), with the map, its picture as the preview and a level.txt saying where it came from;
- with `--catalogue` (the community's map list, `CustomMapList.md`) a map found in it gets its English title, its
  original name (`title.zh` for a Chinese one), its author and its difficulty (the catalogue rates 1–5 in halves; it
  is doubled to our 1–10);
- a map neither in the catalogue nor with a readme beside it, packed archives and folders that are not one level are
  copied into `levels/community-unsorted/` (`--unsorted DIR` for another) with a README.txt saying why; the source is
  only ever read;
- `--refresh levels/community --catalogue CustomMapList.md [--source DIR]` brings a folder imported before up to
  date: it rewrites only the level.txt files (keeping your own lines where the catalogue has nothing) and moves the
  folders that do not qualify into the unsorted folder. Back up your level.txt files first if you edited them.

## Sharing a level

- **Your own assets, a free licence.** A level shared with its files carries only what you made or what is under a
  free licence (CC0, CC-BY, GPL...), with the licence in its folder (`LICENSE` or a line in level.txt and README) and
  the credits for others' work.
- **Never the original's files.** No textures, sounds, meshes or `.nmo` parts of Ballance go into a shared level. A
  level that uses the original's parts or textures names them instead: an open-format level with `"parts": "copy"`
  (the default) takes its parts from each player's own copy, one made with `"parts": "own"` from the game's own parts
  (open-format.md, resources.md); a `game:` texture is drawn from the player's texture pack or copy. The toolbox makes
  what a player's copy provides; what players copy into their own folders is their business.
- **Levels shipped with the game** additionally need their author's permission, a replay that finishes them, and
  nothing of the original inside ([levels.md](levels.md), "What comes with the game").

## Limits

A level comes from a stranger, so the game reads it as hostile and refuses what passes a limit instead of crashing:

| What | Limit |
|---|---|
| level.txt | 4 KiB, 64 lines, a value 40 characters, 16 other-language titles |
| a folder | 10,000 entries looked at in `community/` or `open/`, 4,096 in a level's folder |
| a `.nmo` | 256 MiB; every structure in it bounds-checked and capped (nmo-format.md) |
| an open-format level | `level.gltf` 64 MiB, `level.bin` 1 GiB, images only directly in `textures/`, paths confined to the folder (open-format.md, "Reading") |

A level the game cannot load says why in the menu and is not started.

## Our own level kit: what it checks

Our levels are written as text (`level.lvl`, [levels.md](levels.md), "Our levels and how to edit them") and built by
`assets/own/make/leveldsl.py` with `levelkit.py`. A level made with the kit is refused, with the places named, when it
breaks what makes a level playable (the room and flat-floor checks and the seam test arrive with the level 1 work,
branch `own-level-1`, where levels.md describes each):

- **no seams**: a stretch of floor is one mesh from the start or a checkpoint to the next, so the ball never meets an
  edge between two pieces;
- **room for the ball**: nothing of the floors or rails within 12 over a transformer, a fan or the balloon; a way under
  another floor or rails has its underside at most 3.6 over it (a wall the ball meets) or 6 and more (room to pass),
  never between, where a ball would wedge;
- **flat floor under parts**: a transformer's pad, a box or a dome stands on a flat, level patch of floor at its foot;
- **borders that hold**: railings, brick borders and gutters keep a ball thrown at them on the track (a test throws
  each ball at 12, 25 and 40 units a second);
- **a start and a finish, a title**, checkpoints on a heading the camera can take;
- **it can be finished**: the test player plays every level in three styles, and a level without a finishing run is
  not shipped (`scripts/own-levels.sh`).

A level may also be made by a script of its own on top of the kit: `assets/own/make/make_synthetic_level.py` builds
the shipped showcase level ([levels.md](levels.md), "The showcase level") from a description held in the script, with
commands of its own (signs, a gate, exhibits), materials the kit's floors do not use and behaviour graphs written by
`scriptkit.py` into the level's `scripts`; the same checks apply.
