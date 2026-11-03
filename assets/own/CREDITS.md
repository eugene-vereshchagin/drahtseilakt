# Credits: the photo textures in the game's own files

Some of our own textures are made from freely licensed photo textures: graded to the measured palette and grain of
the original's texture in the same role, tiled, resized, with their normal maps, by the scripts in `make/`
(`free_textures.py` pins each source by its SHA-256 and reads it; `make_menu_scene.py` makes the menu scene's
textures from them, `make_textures.py` the levels', their parts' and the balls' in `Textures/`, listed in
`TEXTURES.md`). The sources themselves are not shipped. CC0 asks for no credit; we thank the authors all the
same. Everything else in `assets/own/` is drawn by our scripts from nothing.

Licence: CC0 1.0 Universal, https://creativecommons.org/publicdomain/zero/1.0/ (ambientCG's licence page:
https://docs.ambientcg.com/license/).

| Source | Author | Licence | Released | Archive (taken 2026-11-03) | SHA-256 of the archive | Made into |
|---|---|---|---|---|---|---|
| [Concrete047A](https://ambientcg.com/a/Concrete047A) | Lennart Demes, ambientCG | CC0 1.0 | 2025-10-09 | `Concrete047A_1K-JPG.zip` | `94e04bdf80c4fa377f8c736183963cdd911131a26673e2462495e45bf679102f` | menu scene: `floor_top`, `pad_top`; levels: every floor top, `Modul15`, `Modul16` |
| [Concrete012](https://ambientcg.com/a/Concrete012) | Lennart Demes, ambientCG | CC0 1.0 | 2018-10-28 | `Concrete012_1K-JPG.zip` | `3424d5cbbab7191d5180f362de0792c99e6a1abd81b6eb3a431c4dfc52fefe7e` | menu scene: `floor_side`, `column`; levels: `Floor_Side`, `Column_beige`, `Column_beige_fade` |
| [Bricks005](https://ambientcg.com/a/Bricks005) | Lennart Demes, ambientCG | CC0 1.0 | 2018-03-06 | `Bricks005_1K-JPG.zip` | `5d7b5584245ffa94eb74b9173c412e33a7cb012b75632a5125b426f9ff9fa0f6` | menu scene: `floor_border`; levels: the floors' brick strips and the checkpoint's rings |
| [WoodFloor040](https://ambientcg.com/a/WoodFloor040) | Lennart Demes, ambientCG | CC0 1.0 | 2020-06-14 | `WoodFloor040_1K-JPG.zip` | `c83a01f1733f2a1f6e95ad6703689eb12fe259e77245d4c0a5652728d07c87cb` | menu scene: `wood`, `oak`; levels: the boards of every wood texture, the wooden ball |
| [Metal063](https://ambientcg.com/a/Metal063) | Lennart Demes, ambientCG | CC0 1.0 | 2026-03-30 | `Metal063_1K-JPG.zip` | `cc0a7f83d4bfd6c5d38638d17a0c214a004ccd295b8c554c959537bc278167ed` | menu scene: `iron`; levels: `Metal_stained`, every iron band, plate and rivet |
| [Metal009](https://ambientcg.com/a/Metal009) | Lennart Demes, ambientCG | CC0 1.0 | 2018-08-16 | `Metal009_1K-JPG.zip` | `1b1970219b40fe707e7194b28702f1a65b0d227bbea56da69dfbc8d8b5cfcb5e` | menu scene: `grate`; levels: the checkpoint's grate, `Modul18`'s plate |
| [Rock030](https://ambientcg.com/a/Rock030) | Lennart Demes, ambientCG | CC0 1.0 | 2020-03-29 | `Rock030_1K-JPG.zip` | `2fd0bc2d214e2487c19062ed0798a85dd135b41cfc12bbaed42d5acae238f989` | levels: the stone ball (`Ball_Stone`) |
| [Paper003](https://ambientcg.com/a/Paper003) | Lennart Demes, ambientCG | CC0 1.0 | 2018-08-30 | `Paper003_1K-JPG.zip` | `90b44da4c960ba044b37c26713c50ef1986687a522f1bbe847aeb9b471409040` | levels: the paper ball's paper (`Ball_Paper`; its print is drawn) |
| [Marble012](https://ambientcg.com/a/Marble012) | Lennart Demes, ambientCG | CC0 1.0 | 2020-01-26 | `Marble012_1K-JPG.zip` | `b04384c6d49ce73baeb70db7ea4abfe9948fc9b74b9fd92c3766970662bf2c43` | levels: `Column_blue` |
| [RoofingTiles015A](https://ambientcg.com/a/RoofingTiles015A) | Lennart Demes, ambientCG | CC0 1.0 | 2023-12-15 | `RoofingTiles015A_1K-JPG.zip` | `5bb040c4c08592b607bba47cbf7384c2769bea1b49d088c463fa296708c0f743` | levels: `Tower_Roof` |
| [WoodFloor041](https://ambientcg.com/a/WoodFloor041) | Lennart Demes, ambientCG | CC0 1.0 | 2020-06-14 | `WoodFloor041_1K-JPG.zip` | `e7364ef32a8db269b2475e76a7e1832eca82bdf6574191cd96b1cdb2b362cdc6` | levels: `Wood_Old`, `Wood_Raft` |
| [Bricks053](https://ambientcg.com/a/Bricks053) | Lennart Demes, ambientCG | CC0 1.0 | 2021-02-14 | `Bricks053_1K-JPG.zip` | `8fdff3ad68df3c9aadc47e118c72af639357076af5d835f7986f2999740d5c0f` | levels: `Brick`, `Modul16`'s stones |
| [Fabric066](https://ambientcg.com/a/Fabric066) | Lennart Demes, ambientCG | CC0 1.0 | 2021-12-12 | `Fabric066_1K-JPG.zip` | `bcd274190979010b2056cab1b10d0420da6e2f58a8e1cf37bc527d8a67772684` | levels: `sandsack` |

Each archive is `https://ambientcg.com/get?file=<archive>`; `python3 -I make/free_textures.py fetch` downloads the
missing ones and refuses any whose checksum differs. `SHA256SUMS` lists the files made from them, as committed.
