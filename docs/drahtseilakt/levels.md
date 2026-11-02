# Where levels live

How the game keeps its levels, agreed with the owner on 2026-11-02 (decisions.md, "Levels live in one folder,
four kinds"). The menus that show them are in [menu.md](menu.md).

## The folders

All levels sit in one folder, `levels/`, in the game's data folder (`~/.local/share/drahtseilakt/` on Linux,
`%APPDATA%\drahtseilakt\` on Windows, `~/Library/Application Support/drahtseilakt/` on macOS), divided by where
they come from and what format they are in:

| Folder | What | Where it comes from | Mode |
|---|---|---|---|
| `levels/original/` | the original game: `base.cmo`, `3D Entities/`, `Textures/`, `Sounds/` and the rest, level 13's add-on installed into it | copied by the game from the player's installed game or CD | classic and extended |
| `levels/community/` | community maps: a folder per level, a `.nmo` or an open-format level in it (below) | the player downloads them, `drahtseilakt-toolbox import-maps` sorts a download into folders; the folder's README says where | extended |
| `levels/remastered/` | the original levels converted to the open format, with textures upscaled to 4K–8K and what else a modern look needs | made on the player's machine by `drahtseilakt-toolbox remaster` from `original/`; never distributed | extended |
| `levels/open/` | levels in the open format: made in it, or converted community maps | made by players; some come with the game | extended |

The classic mode sees only `original/`; the extended mode sees all four (menu.md).

## A level is a folder

*Owner's decision of 2026-11-03* (decisions.md, "A level is a folder"). `community/` and `open/` (and `remastered/`) hold
levels, not whatever an archive unpacked: the custom levels menu shows a flat list of levels, sorted by title, and no
folders to open.

- **A level is a folder directly in the kind's folder** holding exactly one level: one `.nmo` file, or a level in the
  open format (`level.gltf`, `level.bin`, `textures/`; open-format.md). The open format is the clear one, easy to
  finish or fix up by hand: plain glTF and PNG files.
- Beside it, all optional: `level.txt`, simple `key = value` lines (`title`, `author`, `version`; `#` starts a comment,
  other keys are ignored), a README (`README.txt` or `README.md`), a preview picture (`preview.png` or
  `preview.jpg`), and the folders of resources the level uses (`textures/`, `game/`).
- **The title** is level.txt's `title`, else the folder's name. Either is cleaned (control characters, direction
  overrides and other invisible format characters removed, spaces made one) and cut to **40 characters** (characters,
  not bytes) at a word, ending in "…", which the game's font draws as "...". Everywhere the game shows a level's name
  (the menu, the loading screen, the window's title) it shows this title. level.txt is read only when it is a plain
  file of at most 4 KiB; a larger one or a link is ignored.
- **A single `.nmo` put straight into the kind's folder is a level too**, named by its file: the simplest way to try
  a map.
- Ignored by the menu: any other file in a level's folder, every folder inside it (no nesting), hidden names (a dot
  first), links (never followed), and folders with no level or with several (several `.nmo`, or a `.nmo` beside
  `level.gltf`): those are not levels and are named on the console.
- Collections are not a concept: the 13 rotated originals are 13 level folders ("45-Degree Rotated - Level 07").
- `drahtseilakt-toolbox import-maps <folder>` sorts a downloaded collection into this shape: a folder per map with
  its `.nmo`, its picture as the preview, and a level.txt that says where it came from (and its whole title when the
  folder's name had to lose characters). A generic file name (`Level_07.NMO`) is named after its collection; the same
  file found twice becomes one level; packed archives are named, not opened; names are made safe for Linux, Windows
  and macOS (no reserved names, no trailing dots or spaces, at most 40 characters, a second "Hello Mars" numbered
  "(2)"); the source is only read. A second run adds only what is new.

## Levels shipped with the game

Our own levels come with the game in its `assets/levels/`, a folder per level in the open format, made of our own
parts only (assets/own, [resources.md](resources.md)), so they play with no copy of the original at all. The menu
shows them with the open-format levels of `levels/open/`: under Start when there is no copy (the list "Our levels"),
under the extended mode's "Open format" always. They are made by scripts in `assets/own/make/` (`levelkit.py` builds
a level from track pieces and the stand-ins of the parts; `make_test_level.py` the first one, `Parts_Test`, a few
hundred units of track with every part on it or beside it), whose outputs are committed. Their licence is the
game's (GPL-3.0-only).

## The original game: copied by default

The game copies what it needs from the player's own copy into `levels/original/` once: from an installed game
(`drahtseilakt --import <Ballance folder>`) or straight from the game's CD or its image
(`drahtseilakt --import Ballance.cue`, or `.bin`, `.iso`: the InstallShield 6 archive in `SETUP/`, read by
`crates/cdimage`). The copy is optional (decisions.md, "The game plays out of the box"): without one the game starts
with its own levels, and the main menu's **Import** opens a window that does the same: it lists what it finds on the
computer (installed games in the usual folders of Windows, Wine, GOG, Heroic and Lutris; discs in their drives; CD
images in the home, Desktop and Downloads folders), takes a folder or image dropped on it or a path typed, and starts
the game with the copy. The
copy is the player's own data on the player's machine; nothing is downloaded and nothing goes into the
repository. `--data DIR` still plays from any folder without copying.

The folder's README lists what to copy by hand: `base.cmo` and the folders `3D Entities`, `Textures` and `Sounds` of
an installed Ballance (the game reads nothing else), and, for level 13, the SuDu add-on installed into that copy as its readme says.

## Any folder can be moved

Each of the four can be pointed elsewhere in the settings (`levels.original`, `levels.community`,
`levels.remastered`, `levels.open`), for a big disk or a shared folder. The default folder then keeps only a
README saying where the levels are, so a player who looks there finds them.

## What comes with the game

The game is installed with some levels in `levels/open/` (our own) and converted community maps whose authors
want them shipped with the game. Each shipped level needs:

- its author's permission and a free licence, recorded next to it (`LICENSE` in its folder);
- nothing of the original game inside: no textures, sounds or meshes of Ballance. A converted map that uses the
  original's textures names them, and the game takes them from `levels/original/` (the player's copy), as the
  map did in the original;
- a replay that finishes it, checked like the original levels' replays.

The repository holds such levels only once their licence allows it; the original game's files are never in it.

## Every folder explains itself

On its first start the game writes a README into each folder, in the player's language: what belongs there,
where to get it and how levels are laid out, and for `remastered/` which program makes it (the toolbox). A README
an earlier version wrote, untouched, is brought up to date; one the player changed stays. The texts ship with the game (docs in every
language); the README's links are text, and the game never opens them.

## Runs and highscores by kind

Replays, best times and highscores are kept per kind, so a community map named like an original level never
counts as one: `replays/<level>/` for the original levels, `replays/maps/<kind>/<map>/` for the others (`community`, `open`, `remastered`), a level folder's
under `replays/levels/<kind>/<level folder>/<map>/`, so two folders' `Level_07.NMO` keep their runs apart and a
loose `Foo.nmo` and a folder `Foo/` never share. Runs an earlier version kept in `replays/maps/<level folder>/` are
moved there the first time the level is played (only through real folders, never over runs already there).

## Languages

The community is largest in China (Baidu Tieba, the Chinese Ballance Forum, Bilibili; community-maps.md), so the
docs and the folders' READMEs are also in Simplified Chinese, next to English, Russian and German.

## Order of work

1. Done (2026-11-02): this page and the decision; Chinese docs.
2. Done: the `levels/` folders with their READMEs; the settings that move them, with the README left behind.
3. Done: `--import` from an installed game or from the CD image (the game also finds the imported copy
   without `--data`). The European CD's 318 game files come out identical to an installed copy.
4. The custom levels menu reading `community/` and `open/`; `remastered/` is made by `drahtseilakt-toolbox remaster`:
   the converter (phase 6), rounder shapes and the textures N times larger; each level's parts (the balls,
   the modules, the transformer) are remade the same way into its own `game/` folder with its sky and sounds, so it
   plays without the copy's level files (open-format.md) (Real-ESRGAN when installed, else Lanczos; phase 7). An original level
   the copy lacks (a copy without level 13) is played in the levels menu from its export in `remastered/`.
5. Levels that come with the game, as authors offer them.
