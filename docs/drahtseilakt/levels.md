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
| `levels/community/` | community maps, `.nmo`, in folders as the player likes (the menu opens them, four deep) | the player downloads them; the folder's README says where | extended |
| `levels/remastered/` | the original levels converted to the open format, with textures upscaled to 4K–8K and what else a modern look needs | made on the player's machine by our scripts from `original/`; never distributed | extended |
| `levels/open/` | levels in the open format: made in it, or converted community maps | made by players; some come with the game | extended |

The classic mode sees only `original/`; the extended mode sees all four (menu.md).

## The original game: copied by default

The game copies what it needs from the player's own copy into `levels/original/` once: from an installed game
(`drahtseilakt --import <Ballance folder>`) or straight from the game's CD or its image
(`drahtseilakt --import Ballance.cue`, or `.bin`, `.iso`: the InstallShield 6 archive in `SETUP/`, read by
`crates/cdimage`). A start without the game opens a window that does the same: it lists what it finds on the
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
where to get it, and for `remastered/` which script makes it. The texts ship with the game (docs in every
language); the README's links are text, and the game never opens them.

## Runs and highscores by kind

Replays, best times and highscores are kept per kind, so a community map named like an original level never
counts as one: `replays/<level>/` for the original levels, `replays/maps/<folder>/<map>/` for the others (`community`, `open`, `remastered`).

## Languages

The community is largest in China (Baidu Tieba, the Chinese Ballance Forum, Bilibili; community-maps.md), so the
docs and the folders' READMEs are also in Simplified Chinese, next to English, Russian and German.

## Order of work

1. Done (2026-11-02): this page and the decision; Chinese docs.
2. Done: the `levels/` folders with their READMEs; the settings that move them, with the README left behind.
3. Done: `--import` from an installed game or from the CD image (the game also finds the imported copy
   without `--data`). The European CD's 318 game files come out identical to an installed copy.
4. The custom levels menu reading `community/` and `open/`; `remastered/` is made by `drahtseilakt-toolbox remaster`:
   the converter (phase 6), rounder shapes and the textures N times larger; the parts all levels share (the balls,
   the modules, the transformer) are remade the same way into the hidden `remastered/.shared/`, and a remastered
   level draws them so (Real-ESRGAN when installed, else Lanczos; phase 7). An original level
   the copy lacks (a copy without level 13) is played in the levels menu from its export in `remastered/`.
5. Levels that come with the game, as authors offer them.
