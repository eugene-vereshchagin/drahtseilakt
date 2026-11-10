# Credits: the photo textures in the game's own files

Some of our own textures are made from freely licensed photo textures: graded to the measured palette and grain of
the original's texture in the same role, tiled, resized, with their normal maps, by the scripts in `make/`
(`free_textures.py` pins each source by its SHA-256 and reads it; `make_menu_scene.py` makes the menu scene's
textures from them, `make_textures.py` the levels', their parts' and the balls' in `Textures/`, listed in
`TEXTURES.md`). The sources themselves are not shipped. CC0 asks for no credit; we thank the authors all the
same. Everything else in `assets/own/` is drawn by our scripts from nothing. The levels shipped with the game (`assets/levels/`, made by `make/levelkit.py`) carry copies of
some of these textures (`Textures/Floor_*`, `Wood_Plain`, `Column_beige`, `Rail_Environment`) in their `textures/`
folders: the same CC0 pictures, credited here.

Licence: CC0 1.0 Universal, https://creativecommons.org/publicdomain/zero/1.0/ (ambientCG's licence page:
https://docs.ambientcg.com/license/).

| Source | Author | Licence | Released | Archive (taken 2026-11-03) | SHA-256 of the archive | Made into |
|---|---|---|---|---|---|---|
| [Concrete047A](https://ambientcg.com/a/Concrete047A) | Lennart Demes, ambientCG | CC0 1.0 | 2025-10-09 | `Concrete047A_1K-JPG.zip` | `94e04bdf80c4fa377f8c736183963cdd911131a26673e2462495e45bf679102f` | menu scene: `floor_top`, `pad_top`; levels: every floor top, `Modul15`, `Modul16` |
| [Concrete012](https://ambientcg.com/a/Concrete012) | Lennart Demes, ambientCG | CC0 1.0 | 2018-10-28 | `Concrete012_1K-JPG.zip` | `3424d5cbbab7191d5180f362de0792c99e6a1abd81b6eb3a431c4dfc52fefe7e` | menu scene: `floor_side`, `column`; levels: `Floor_Side`, `Column_beige`, `Column_beige_fade` |
| [Bricks005](https://ambientcg.com/a/Bricks005) | Lennart Demes, ambientCG | CC0 1.0 | 2018-03-06 | `Bricks005_1K-JPG.zip` | `5d7b5584245ffa94eb74b9173c412e33a7cb012b75632a5125b426f9ff9fa0f6` | menu scene: `floor_border`; levels: the floors' brick strips and the checkpoint's rings |
| [WoodFloor040](https://ambientcg.com/a/WoodFloor040) | Lennart Demes, ambientCG | CC0 1.0 | 2020-06-14 | `WoodFloor040_1K-JPG.zip` | `c83a01f1733f2a1f6e95ad6703689eb12fe259e77245d4c0a5652728d07c87cb` | menu scene: `wood`, `oak`; levels: the boards of every wood texture |
| [Metal063](https://ambientcg.com/a/Metal063) | Lennart Demes, ambientCG | CC0 1.0 | 2026-03-30 | `Metal063_1K-JPG.zip` | `cc0a7f83d4bfd6c5d38638d17a0c214a004ccd295b8c554c959537bc278167ed` | menu scene: `iron`; levels: `Metal_stained`, every iron band, plate and rivet |
| [Metal009](https://ambientcg.com/a/Metal009) | Lennart Demes, ambientCG | CC0 1.0 | 2018-08-16 | `Metal009_1K-JPG.zip` | `1b1970219b40fe707e7194b28702f1a65b0d227bbea56da69dfbc8d8b5cfcb5e` | menu scene: `grate`; levels: the checkpoint's grate, `Modul18`'s plate |
| [Marble012](https://ambientcg.com/a/Marble012) | Lennart Demes, ambientCG | CC0 1.0 | 2020-01-26 | `Marble012_1K-JPG.zip` | `b04384c6d49ce73baeb70db7ea4abfe9948fc9b74b9fd92c3766970662bf2c43` | levels: `Column_blue` |
| [RoofingTiles015A](https://ambientcg.com/a/RoofingTiles015A) | Lennart Demes, ambientCG | CC0 1.0 | 2023-12-15 | `RoofingTiles015A_1K-JPG.zip` | `5bb040c4c08592b607bba47cbf7384c2769bea1b49d088c463fa296708c0f743` | levels: `Tower_Roof` |
| [WoodFloor041](https://ambientcg.com/a/WoodFloor041) | Lennart Demes, ambientCG | CC0 1.0 | 2020-06-14 | `WoodFloor041_1K-JPG.zip` | `e7364ef32a8db269b2475e76a7e1832eca82bdf6574191cd96b1cdb2b362cdc6` | levels: `Wood_Old`, `Wood_Raft` |
| [Bricks053](https://ambientcg.com/a/Bricks053) | Lennart Demes, ambientCG | CC0 1.0 | 2021-02-14 | `Bricks053_1K-JPG.zip` | `8fdff3ad68df3c9aadc47e118c72af639357076af5d835f7986f2999740d5c0f` | levels: `Brick`, `Modul16`'s stones |
| [Fabric066](https://ambientcg.com/a/Fabric066) | Lennart Demes, ambientCG | CC0 1.0 | 2021-12-12 | `Fabric066_1K-JPG.zip` | `bcd274190979010b2056cab1b10d0420da6e2f58a8e1cf37bc527d8a67772684` | levels: `sandsack` |

Each archive is `https://ambientcg.com/get?file=<archive>`; `python3 -I make/free_textures.py fetch` downloads the
missing ones and refuses any whose checksum differs. `SHA256SUMS` lists the files made from them, as committed.

## The photographic skies

The skies a level names `Mountains`, `Forest`, `Sea`, `Storm`, `Sunset`, `Night`, `Desert`, `Mist` and `Clouds`
(`Textures/Sky/Sky_<name>_<face>.jpg`) are made by `make/make_skies.py` from HDR panoramas of
[Poly Haven](https://polyhaven.com/) (licence: CC0 1.0 Universal, https://polyhaven.com/license; checked on each
asset's page 2026-11-03): each panorama's sky, tone-mapped with a fixed exposure, over a world below the horizon that
the script draws itself (sea, forest, mountains, canyons, a misty valley; under `Sunset` and `Clouds` only the haze, the
game drawing their sea of cloud), lit by the same panorama. `Sunset` is graded towards a golden hour (warmer
towards the sun, its dusty horizon band cleared towards the sky above it).
The panoramas are not shipped; `python3 -I make/make_skies.py fetch` downloads them once into
`texture-sources/skies/` and refuses any whose checksum differs.

| Sky | Source | Authors | Licence | Published | File (taken 2026-11-03) | SHA-256 of the file |
|---|---|---|---|---|---|---|
| `Sky_Mountains` | [table_mountain_1_puresky](https://polyhaven.com/a/table_mountain_1_puresky) | Greg Zaal, Jarod Guest, Poly Haven | CC0 1.0 | 2022-12-03 | `table_mountain_1_puresky_4k.hdr` | `1b1c70d89fdd257b242c6790d1a3e92bcaa4e72ea5f6644641df29e9efbb8617` |
| `Sky_Forest` | [kloofendal_48d_partly_cloudy_puresky](https://polyhaven.com/a/kloofendal_48d_partly_cloudy_puresky) | Greg Zaal, Jarod Guest, Poly Haven | CC0 1.0 | 2022-11-19 | `kloofendal_48d_partly_cloudy_puresky_4k.hdr` | `3061c00a16ecae748e84fd6c44c04804f539c118bb66325db858bad87b11bf88` |
| `Sky_Sea` | [kloppenheim_06_puresky](https://polyhaven.com/a/kloppenheim_06_puresky) | Greg Zaal, Jarod Guest, Poly Haven | CC0 1.0 | 2022-11-26 | `kloppenheim_06_puresky_4k.hdr` | `1bebd7cbbfb5c48a642f03d93bffbabb2c42d0b109061f37a72259d1cbdb3053` |
| `Sky_Storm` | [kloppenheim_07_puresky](https://polyhaven.com/a/kloppenheim_07_puresky) | Greg Zaal, Jarod Guest, Poly Haven | CC0 1.0 | 2022-12-18 | `kloppenheim_07_puresky_4k.hdr` | `98a94dc8c290a89de0147642750beb9dae4c791ee33c1689bebbf58126e97f22` |
| `Sky_Sunset` | [rocky_ridge_puresky](https://polyhaven.com/a/rocky_ridge_puresky) | Greg Zaal, Jarod Guest, Poly Haven | CC0 1.0 | 2022-12-04 | `rocky_ridge_puresky_4k.hdr` (taken 2026-11-03) | `82efe60eea01151530c16f841270bfbdb5dd5eafdc3f285fb3cc368d8c0a7501` |
| `Sky_Night` | [rogland_clear_night](https://polyhaven.com/a/rogland_clear_night) | Greg Zaal, Poly Haven | CC0 1.0 | 2024-10-08 | `rogland_clear_night_4k.hdr` | `133d77ddf6a69926a138574169e5cdf4c04b785e8d22545274aa285f1ec3d879` |
| `Sky_Desert` | [qwantani_late_afternoon_puresky](https://polyhaven.com/a/qwantani_late_afternoon_puresky) | Greg Zaal, Jarod Guest, Poly Haven | CC0 1.0 | 2025-07-18 | `qwantani_late_afternoon_puresky_4k.hdr` | `d820fa7a84732f69641655dcbea33b01ff78e6a4833ee3ca6f3bc938a0c09bb7` |
| `Sky_Mist` | [kloppenheim_01_puresky](https://polyhaven.com/a/kloppenheim_01_puresky) | Greg Zaal, Jarod Guest, Poly Haven | CC0 1.0 | 2022-11-20 | `kloppenheim_01_puresky_4k.hdr` | `e667700214b1141dd0890c6e45decf9c84964b3c6596b717a48c56e3d893bde3` |
| `Sky_Clouds` | [kloofendal_48d_partly_cloudy_puresky](https://polyhaven.com/a/kloofendal_48d_partly_cloudy_puresky) | Greg Zaal, Jarod Guest, Poly Haven | CC0 1.0 | 2022-11-19 | `kloofendal_48d_partly_cloudy_puresky_4k.hdr` | `3061c00a16ecae748e84fd6c44c04804f539c118bb66325db858bad87b11bf88` |

Each file is `https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/4k/<file>`. `SHA256SUMS` lists the faces made from them.
