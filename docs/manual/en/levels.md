# Levels and folders

## What comes with the game

Two sets of maps are inside the game, so you can play at once without anything else:

- **Featured maps**: 13 maps picked by us, from community authors, with rounded shapes and textures we improved: they are not the ones the author first used.
- **Community maps, Included**: 13 more community maps in the files their authors made, with their credits.

All of these belong to their authors and are shown with thanks. If an author asks us to remove a map, we do.

## Your data folder

The game keeps your settings, scores, replays and your own levels in one data folder:

| System | Data folder |
|---|---|
| Linux | `~/.local/share/drahtseilakt/` |
| Windows | `%APPDATA%\drahtseilakt\` |
| macOS | `~/Library/Application Support/drahtseilakt/` |
| Portable copy | the folder `data/` beside the program (a file `portable.txt` lies there, and nothing outside the folder is used) |

In it, the folder `levels/` has these folders:

| Folder | What it holds |
|---|---|
| `ballance-original/` | your copy of the original game, put there by the toolbox |
| `ballance-remaster/` | the original levels remade in 4K, made by `drahtseilakt-toolbox` |
| `user-levels/` | your own and downloaded maps |
| `modified/` | community remakes of the original levels (rotated, reversed, invisible); they need your copy |

## The rule: a level is a folder

A level is a level **only** when it lies in its own named folder that holds **one `.nmo` file** (or a level in the open
format) **and a description file `level.txt`**. A loose `.nmo` thrown into the folder, or a folder without `level.txt`,
is not shown in the list.

```
user-levels/
  My Map/
    My Map.nmo
    level.txt
    preview.png        (optional picture)
  Season 1/            (a grouping folder: it holds no level itself)
    Another Map/
      Another Map.nmo
      level.txt
```

A folder with no level of its own is a grouping folder, and the game looks inside it. Do not put two levels into one
folder.

### level.txt

A plain text file of `key = value` lines. A line that begins with `#` is a comment.

```
# My first map
title = My Map
title.ru = Моя карта
author = Your Name
version = 1.0
difficulty = 4
```

| Key | Meaning |
|---|---|
| `title` | the name in the list (English; at most 40 characters are shown) |
| `title.ru`, `title.de`, `title.zh` ... | the name in another language, shown when the menus speak it |
| `author` | who made the map |
| `version` | the map's version |
| `difficulty` | how hard it is, a whole number from 1 to 10; the list is sorted by it |
| `order` | a number that puts the level earlier in the list |

Only `title` is really needed; the file may be up to 4 KB.

### "Some entries in the levels folder are wrong"

On the list of your levels the game may say that some entries are wrong. It means that it found a folder or a file
that does not follow the rule above: a `.nmo` lying loose, a folder without `level.txt`, or a folder with several
levels. To fix it, put each map into a folder of its own, add a `level.txt` with at least a `title` line, and open the
list again. The game's log names every entry it left out.

The easiest way to get this right is [the toolbox](toolbox.md): its *import maps* sorts a whole downloaded collection
into this shape.
