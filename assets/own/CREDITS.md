# Credits: the photo textures in the game's own files

Some of our own textures are made from freely licensed photo textures: graded to the measured palette and grain of
the original's texture in the same role, tiled, resized, with their normal maps, by the scripts in `make/`
(`free_textures.py` pins each source by its SHA-256 and reads it; `make_menu_scene.py` makes the menu scene's
textures from them). The sources themselves are not shipped. CC0 asks for no credit; we thank the authors all the
same. Everything else in `assets/own/` is drawn by our scripts from nothing.

Licence: CC0 1.0 Universal, https://creativecommons.org/publicdomain/zero/1.0/ (ambientCG's licence page:
https://docs.ambientcg.com/license/).

| Source | Author | Licence | Released | Archive (taken 2026-11-03) | SHA-256 of the archive | Made into |
|---|---|---|---|---|---|---|
| [Concrete047A](https://ambientcg.com/a/Concrete047A) | Lennart Demes, ambientCG | CC0 1.0 | 2025-10-09 | `Concrete047A_1K-JPG.zip` | `94e04bdf80c4fa377f8c736183963cdd911131a26673e2462495e45bf679102f` | menu scene: `floor_top`, `pad_top` |
| [Concrete012](https://ambientcg.com/a/Concrete012) | Lennart Demes, ambientCG | CC0 1.0 | 2018-10-28 | `Concrete012_1K-JPG.zip` | `3424d5cbbab7191d5180f362de0792c99e6a1abd81b6eb3a431c4dfc52fefe7e` | menu scene: `floor_side`, `column` |
| [Bricks005](https://ambientcg.com/a/Bricks005) | Lennart Demes, ambientCG | CC0 1.0 | 2018-03-06 | `Bricks005_1K-JPG.zip` | `5d7b5584245ffa94eb74b9173c412e33a7cb012b75632a5125b426f9ff9fa0f6` | menu scene: `floor_border` |
| [WoodFloor040](https://ambientcg.com/a/WoodFloor040) | Lennart Demes, ambientCG | CC0 1.0 | 2020-06-14 | `WoodFloor040_1K-JPG.zip` | `c83a01f1733f2a1f6e95ad6703689eb12fe259e77245d4c0a5652728d07c87cb` | menu scene: `wood`, `oak` |
| [Metal063](https://ambientcg.com/a/Metal063) | Lennart Demes, ambientCG | CC0 1.0 | 2026-03-30 | `Metal063_1K-JPG.zip` | `cc0a7f83d4bfd6c5d38638d17a0c214a004ccd295b8c554c959537bc278167ed` | menu scene: `iron` |
| [Metal009](https://ambientcg.com/a/Metal009) | Lennart Demes, ambientCG | CC0 1.0 | 2018-08-16 | `Metal009_1K-JPG.zip` | `1b1970219b40fe707e7194b28702f1a65b0d227bbea56da69dfbc8d8b5cfcb5e` | menu scene: `grate` |

Each archive is `https://ambientcg.com/get?file=<archive>`; `python3 -I make/free_textures.py fetch` downloads the
missing ones and refuses any whose checksum differs. `SHA256SUMS` lists the files made from them, as committed.
