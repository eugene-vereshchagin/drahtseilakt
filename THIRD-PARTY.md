# Third-party components

Drahtseilakt is free software under the GNU General Public License, version 3 only (`LICENSE`). This file lists
everything in a Drahtseilakt download that was not written for it, or that keeps its own licence: the upstream
project it began from, the fonts, the sources of its textures and skies, its sounds and music, the levels it ships
and the crates compiled into the program. It ships with every download, beside `LICENSE` and `CREDITS.md`.

Nothing of the original Ballance (© 2004 Cyparade / Atari) is part of a download: its levels, textures, sounds and
interface come only from the player's own copy, at the player's request. This project is not affiliated with
Cyparade, Atari or Ziggurat Interactive.

The detailed records (checksums of every source, what was done to each file) stay in the files named in each
section; this is the summary. The code-dependency part at the end is generated from `Cargo.lock` by
`tools/third_party.py` (offline, from the local crate cache) and checked by `crates/game/tests/third_party.rs`: a
dependency without a licence, or with one that is not compatible with GPL-3.0, fails the tests.

## Upstream: OpenBallance

| Component | Author | Licence | Where |
|---|---|---|---|
| [OpenBallance](https://github.com/emdzej/openballance), the project this repository was forked from | emdzej and contributors | GPL-3.0 | the repository's history; `src/` and `tools/` (C, reference only, not in the program) |
| The physics, ported from OpenBallance's `src/phys` to Rust | emdzej and contributors (port: Drahtseilakt) | GPL-3.0 | `crates/ivp/` (each ported file says so in its header) |

Everything else in the program, the tools and the documentation was written for Drahtseilakt and is under
GPL-3.0-only, as are the game's own drawings, models, interface texts, mods, manual and the generated sound theme
(listed below as "this project").

## Fonts

All under the SIL Open Font License 1.1, shipped unmodified with their licence texts in `assets/fonts/` (and a copy
in `assets/manual/fonts/` for the manual). Details and sources: `assets/fonts/README.md`.

| File | Font | Version | Author | Licence | Licence text |
|---|---|---|---|---|---|
| `NotoSans-Regular.ttf`, `NotoSans-Bold.ttf` | Noto Sans | 2.008 | Google, the Noto project authors | OFL-1.1 | `OFL-NotoSans.txt` |
| `NotoSansSC-Regular.otf` | Noto Sans SC (subset) | 2.004 | Google, Adobe, the Noto project authors | OFL-1.1 | `OFL.txt` |
| `Tektur-Variable.ttf` | Tektur | 1.005 | Adam Jagosz | OFL-1.1 | `OFL-Tektur.txt` |

The fonts are separate files the program reads at run time, not compiled into it (an aggregate, as the OFL and the
GPL both allow).

## Textures and their sources

| What | Made from | Author | Licence | Record |
|---|---|---|---|---|
| The interface (`assets/own/Textures/Button01_*`), particles, shadows, skies A–C, the track's textures, the splash picture, the parts and balls (`assets/own/3D Entities/`), the menu scene | drawn and modelled by scripts in `assets/own/make/` | this project | GPL-3.0-only | `assets/own/README.md` |
| The levels', parts' and balls' textures (`assets/own/Textures/*.png`) and the menu scene's surfaces | 11 photo textures of [ambientCG](https://ambientcg.com/) (Concrete047A, Concrete012, Bricks005, Bricks053, WoodFloor040, WoodFloor041, Metal063, Metal009, Marble012, RoofingTiles015A, Fabric066), graded, tiled and resized by `make/make_textures.py` | Lennart Demes, ambientCG | CC0-1.0 | `assets/own/CREDITS.md`, `assets/own/TEXTURES.md` |
| The photographic skies `Sky_Mountains`, `Forest`, `Sea`, `Storm`, `Sunset`, `Night`, `Desert`, `Mist`, `Clouds` | 8 HDR panoramas of [Poly Haven](https://polyhaven.com/) (table_mountain_1, kloofendal_48d_partly_cloudy, kloppenheim_01/06/07, rocky_ridge, rogland_clear_night, qwantani_late_afternoon), the land below drawn by `make/make_skies.py` and `make/skyland.py` | Greg Zaal, Jarod Guest, Poly Haven | CC0-1.0 | `assets/own/CREDITS.md` |

The source photographs and panoramas are not shipped; the scripts download them pinned by SHA-256.

## Sounds and music

Two sound themes, `assets/sounds/<theme>/`. The licence texts: CC0 1.0
<https://creativecommons.org/publicdomain/zero/1.0/>, CC BY 3.0 <https://creativecommons.org/licenses/by/3.0/>,
CC BY 4.0 <https://creativecommons.org/licenses/by/4.0/>.

**Generated** (`neuro`, the default): every effect and piece synthesized for Drahtseilakt by code; this project,
GPL-3.0-only (`assets/sounds/neuro/CREDITS.md`). The synthesizer that made it, and the system theme's rail loops, is this
project's own code in `tools/audio-synth/` (GPL-3.0-only; its README says how the sounds are rebuilt).

**System** (`system`): 60 files made by `prepare.py` from openly licensed recordings (cut, levelled, looped,
encoded; each file's changes in `assets/sounds/system/CREDITS.md`): 36 from CC0 works, 20 from CC BY works, 4
synthesized by this project (GPL-3.0-only).

CC BY works, used under their licences with this attribution (what was changed: `assets/sounds/system/CREDITS.md`):

| Work | Author | Licence | Source |
|---|---|---|---|
| Ripples; Guzheng City; Shenyang; Ishikari Lore; Senbazuru; Finding Movement | Kevin MacLeod (incompetech.com) | CC BY 4.0 | <https://incompetech.com/music/royalty-free/music.html> |
| moonlit_mirage_loop | Tsorthan Grove | CC BY 4.0 | <https://opengameart.org/content/moonlit-mirage> |
| Gentle intro guzheng | robcro6010 | CC BY 4.0 | <https://freesound.org/people/robcro6010/sounds/512943/> |
| GUZHENG CUSTOMIZE YOUR LOOP 01 G 112 BPM | CarlosCarty | CC BY 4.0 | <https://freesound.org/people/CarlosCarty/sounds/520729/> |
| 鹤舞白沙 | Amingliang | CC BY 4.0 | <https://freesound.org/people/Amingliang/sounds/582209/> |
| Quartz crystal singing bowl | juskiddink | CC BY 4.0 | <https://freesound.org/people/juskiddink/sounds/129219/> |
| Orient Theme (jingle) | WakianTech (K-ManZ) | CC BY 3.0 | <https://opengameart.org/content/orient-theme-jingle> |
| liyan; Asian Duet | elerya | CC BY 3.0 | <https://opengameart.org/content/liyan>, <https://opengameart.org/content/asian-duet> |
| cold to lukewarm (mixia) | Spring Spring | CC BY 3.0 | <https://opengameart.org/content/cold-to-lukewarm-mixia> |
| RPG - Misty Mountains | Hitctrl | CC BY 3.0 | <https://opengameart.org/content/rpg-misty-mountains> |
| life_pickup (Yo Frankie!) | Lamoot | CC BY 3.0 | <https://opengameart.org/content/life-pickup-yo-frankie> |
| freesound 182345 | Claire.H | CC BY 3.0 | <https://freesound.org/people/Claire.H/sounds/182345/> |
| wind ambient synth | botpl | CC BY 3.0 | <https://freesound.org/people/botpl/sounds/151783/> |

CC0 works (no attribution required; credited with thanks): Kenney (www.kenney.nl: Interface Sounds, UI Audio,
Digital Audio, Impact Sounds), and on freesound.org and OpenGameArt.org BOSS MUSIC, GnoteSoundz, BMacZero, qubodup,
checholio, BlueDelta, Ezcah, rsellick, KaleidacousticsAudio, WasabiWielder, Nick-Nac, neolein, Tozan and others,
each named per file in `assets/sounds/system/CREDITS.md`.

## Levels

The levels shipped with the game are community maps by players of Ballance. **They belong to their authors and are
not covered by the game's licence.** They are shipped with credit and thanks; no written permission is recorded for
any of them, and a map is removed if its author asks (the owner's policy for community maps and remasters). Each
folder's `level.txt` names the author and where the map was posted; `CREDITS.md` has the full tables and
`SHA256SUMS` in each set's folder the checksums.

**Drahtseilakt set** (`assets/levels/drahtseilakt/`): converted from each author's `.nmo` to the open format,
rounded and with textures upscaled; where a map did not embed a texture, ours (above) stands in.

**No picture of the original game ships in a level** (owner's decision, 2026-11-16). Some maps carried copies of the
original Ballance's textures inside them; every shipped picture (the open-format levels' files, the pictures saved in
the community maps' `.nmo` files) is compared with every picture of the original game, pixel for pixel and by a
perceptual hash that sees through rescaling, turning and small colour changes, in the colours and in the alpha
(`gamedata::provenance`; the test `crates/gamedata/tests/shipped_originals.rs` fails on a match). Each match was
replaced by our own picture of the same name (above), turned and sized as the map's (`own-pictures`; a map's saved
picture is rewritten in place by `nmo::patch`, nothing else in the file changes). Those maps are marked
"proprietary textures removed" below and in their `level.txt` (`note`, in every language of the interface).

| # | Map | Author | Source | Permission |
|---|---|---|---|---|
| 1 | Map 260801 (260801图) | 小锂电池 | <https://forum.ballance.top/d/117> | none recorded; credited, removed on request |
| 2 | Bright Galaxy v1.2 (璀璨星河) | 其乐融融 | <https://tieba.baidu.com/p/8159356350> | none recorded; credited, removed on request |
| 3 | Beginners Happiness 5 (萌新快乐图5) | dahkjdas | video only: <https://www.youtube.com/watch?v=jQi5n__Yg9M> | none recorded; credited, removed on request |
| 4 | Edge of the Shade | Kry | video only: <https://www.youtube.com/watch?v=WJPVbtzQbEg> | none recorded; credited, removed on request |
| 5 | See You Tomorrow (明天见) | 不药而愈的匿名安慰 | <https://forum.ballance.top/d/119> | none recorded; credited, removed on request |
| 6 | Push the Envelope 3.0 (挑战极限3.0) | lydjjlyd | video only: <https://www.youtube.com/watch?v=MwOy6QMdy8E> | none recorded; credited, removed on request |
| 7 | The Long Night of Ages v1.1 (岁月长夜V1.1) | 四叶草 (dyethx) | <https://forum.ballance.top/d/113> | none recorded; credited, removed on request |
| 8 | Dreaming Back to 2025 (梦回2025) | 其乐融融 | video only: <https://www.youtube.com/watch?v=MBJzOvgkLVk> | none recorded; credited, removed on request |
| 9 | Evolution of Logic (逻辑时代) | dahkjdas | <https://tieba.baidu.com/p/6700441841> | none recorded; credited, removed on request |
| 10 | Illusion and Reality (幻实) (proprietary textures removed) | Entity_303 | <https://tieba.baidu.com/p/9145563541> | none recorded; credited, removed on request |
| 11 | Singular Forest (单轨森林) | ZYL | <https://tieba.baidu.com/p/8267347663> | none recorded; credited, removed on request |
| 12 | Eastern Nights (东方夜谭) (proprietary textures removed) | perfectthought | video only: <https://www.youtube.com/watch?v=nM1oyg-Rhxg> | none recorded; credited, removed on request |
| 13 | Queen Ball (Hard Ver.) (proprietary textures removed) | pseudoacetyl | video only: <https://www.youtube.com/watch?v=Sq10sBrBQ8g> | none recorded; credited, removed on request |

**Community set** (`assets/levels/community/`): the authors' `.nmo` files, unmodified except for the pictures of
the original game five of them carried (above), which are ours now. The list and difficulty come
from the catalogue [BallanceCustomMaps](https://github.com/pseudoacetyl/BallanceCustomMaps) by pseudoacetyl (MIT;
the licence covers the list, not the maps).

| # | Map | Author | Posted | Permission |
|---|---|---|---|---|
| 1 | Scarborough Fair (斯卡布罗集市) | yyc12345 | 2020-07-16 | none recorded; credited, removed on request |
| 2 | Ballance Windows v1.1 (平衡视窗V1.1) | hhmmdd123 | 2014-08-27 | none recorded; credited, removed on request |
| 3 | Ballance Bewildering City v1.3 (平衡迷城V1.3) | 平衡总统 | 2015-02-23 | none recorded; credited, removed on request |
| 4 | Moli Thinking (茉莉思考) (proprietary textures removed) | ZYL | 2013-02-23 | none recorded; credited, removed on request |
| 5 | Totole Chicken (Bouillon) (太太乐鸡(精)) (proprietary textures removed) | 傲骨银狼ヶ璇 | 2010-08-27 | none recorded; credited, removed on request |
| 6 | Family Portrait (全家福) | vipeazone | 2012-03-01 | none recorded; credited, removed on request |
| 7 | The SBlender | speedystoneball | 2023-05-28 | none recorded; credited, removed on request |
| 8 | Cross Street v1.9 (十字街道1.9) (proprietary textures removed) | 平衡四边球 | 2016-08-07 | none recorded; credited, removed on request |
| 9 | Scary Maze v1.2 (恐怖迷宫V1.2) (proprietary textures removed) | Sterd | 2012-08-05 | none recorded; credited, removed on request |
| 10 | Temporarily Nameless (暂无名图) | lydjjlyd | 2018-12-28 | none recorded; credited, removed on request |
| 11 | Dangerous Sections of Roads (危险路段) | 平衡总统 | 2015-06-29 | none recorded; credited, removed on request |
| 12 | Twilight City (暮光之城) (proprietary textures removed) | Nice_melodies | 2016-08-07 | none recorded; credited, removed on request |
| 13 | Metropolis Cross (都会十字) | dahkjdas | 2020-12-31 | none recorded; credited, removed on request |

The hidden test levels `Parts_Test` and `00_Синтетический_уровень` are this project's (GPL-3.0-only).

## System libraries in the portable Linux build

The portable Linux folder carries three shared libraries in `lib/`, unmodified binaries from Debian 13 (their
sources: <https://sources.debian.org/>, package versions below). They are separate files the program links at run
time.

| File | Project | Debian package | Licence |
|---|---|---|---|
| `libasound.so.2` | ALSA library (alsa-lib) | libasound2t64 1.2.14-1+deb13u1 | LGPL-2.1-or-later |
| `libudev.so.1` | systemd's libudev | libudev1 257.13-1~deb13u1 | LGPL-2.1-or-later |
| `libcap.so.2` | libcap | libcap2 1:2.75-10+deb13u1 | BSD-3-Clause OR GPL-2.0-only |

## Code dependencies

<!-- third-party:crates:begin (generated by tools/third_party.py; do not edit by hand) -->

Crates compiled into `drahtseilakt` on any of its platforms (Linux, Windows, macOS; an upper bound, since a crate is counted when any build in Cargo.lock enables it): 286, each with the licence its manifest declares (an `OR` means the crate may be used under either; we take the one compatible with GPL-3.0). Their licence texts are in each crate's source at its version on crates.io.

Licences, by count: 130× MIT OR Apache-2.0; 62× MIT; 15× Zlib OR Apache-2.0 OR MIT; 14× Apache-2.0 OR MIT; 10× MIT/Apache-2.0; 9× Apache-2.0; 6× Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT; 6× MIT OR Apache-2.0 OR Zlib; 5× Apache-2.0/MIT; 5× Unlicense OR MIT; 3× BSD-3-Clause; 3× ISC; 3× Zlib; 2× BSD-2-Clause OR Apache-2.0 OR MIT; 2× BSD-3-Clause OR Apache-2.0; 2× BSD-3-Clause OR MIT OR Apache-2.0; 2× MIT OR Zlib OR Apache-2.0; 1× (MIT OR Apache-2.0) AND Unicode-3.0; 1× 0BSD OR MIT OR Apache-2.0; 1× Apache-2.0 / MIT; 1× Apache-2.0 AND MIT; 1× BSD-2-Clause; 1× BSD-2-Clause OR MIT OR Apache-2.0; 1× MIT OR Apache-2.0 OR LGPL-2.1-or-later.

| Crate | Version | Licence |
|---|---|---|
| ab_glyph | 0.2.32 | Apache-2.0 |
| ab_glyph_rasterizer | 0.1.10 | Apache-2.0 |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| ahash | 0.8.12 | MIT OR Apache-2.0 |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 |
| alsa | 0.11.0 | Apache-2.0/MIT |
| alsa-sys | 0.4.0 | MIT |
| android-activity | 0.6.1 | MIT OR Apache-2.0 |
| android-properties | 0.2.2 | MIT |
| android_system_properties | 0.1.6 | MIT OR Apache-2.0 |
| arrayref | 0.3.9 | BSD-2-Clause |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| as-raw-xcb-connection | 1.0.1 | MIT OR Apache-2.0 |
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT |
| bit-set | 0.10.0 | Apache-2.0 OR MIT |
| bit-vec | 0.9.1 | Apache-2.0 OR MIT |
| bitflags | 1.3.2 | MIT/Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| block2 | 0.5.1 | MIT |
| block2 | 0.6.2 | MIT |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT |
| byteorder | 1.5.0 | Unlicense OR MIT |
| byteorder-lite | 0.1.0 | Unlicense OR MIT |
| bytes | 1.12.1 | MIT |
| calloop | 0.13.0 | MIT |
| calloop-wayland-source | 0.3.0 | MIT |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 |
| chrono | 0.4.45 | MIT OR Apache-2.0 |
| codespan-reporting | 0.13.1 | Apache-2.0 |
| combine | 4.6.9 | MIT |
| concurrent-queue | 2.5.0 | Apache-2.0 OR MIT |
| core-foundation | 0.9.4 | MIT OR Apache-2.0 |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 |
| core-graphics | 0.23.2 | MIT OR Apache-2.0 |
| core-graphics-types | 0.1.3 | MIT OR Apache-2.0 |
| coreaudio-rs | 0.14.2 | MIT/Apache-2.0 |
| cpal | 0.18.2 | Apache-2.0 |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 |
| crunchy | 0.2.4 | MIT |
| cursor-icon | 1.2.0 | MIT OR Apache-2.0 OR Zlib |
| dasp_sample | 0.11.0 | MIT OR Apache-2.0 |
| dispatch | 0.2.0 | MIT |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT |
| dlib | 0.5.3 | MIT |
| document-features | 0.2.12 | MIT OR Apache-2.0 |
| downcast-rs | 1.2.1 | MIT/Apache-2.0 |
| dpi | 0.1.2 | Apache-2.0 AND MIT |
| equivalent | 1.0.2 | Apache-2.0 OR MIT |
| errno | 0.3.14 | MIT OR Apache-2.0 |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| fnv | 1.0.7 | Apache-2.0 / MIT |
| foldhash | 0.2.0 | Zlib |
| foreign-types | 0.5.0 | MIT/Apache-2.0 |
| foreign-types-macros | 0.2.4 | MIT/Apache-2.0 |
| foreign-types-shared | 0.3.1 | MIT/Apache-2.0 |
| futures-core | 0.3.34 | MIT OR Apache-2.0 |
| futures-task | 0.3.34 | MIT OR Apache-2.0 |
| futures-util | 0.3.34 | MIT OR Apache-2.0 |
| gethostname | 1.1.0 | Apache-2.0 |
| getrandom | 0.3.4 | MIT OR Apache-2.0 |
| gilrs | 0.11.2 | Apache-2.0/MIT |
| gilrs-core | 0.6.8 | Apache-2.0/MIT |
| glam | 0.34.1 | MIT OR Apache-2.0 |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib |
| glutin_wgl_sys | 0.6.1 | Apache-2.0 |
| gpu-allocator | 0.28.0 | MIT OR Apache-2.0 |
| half | 2.7.1 | MIT OR Apache-2.0 |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| hermit-abi | 0.5.3 | MIT OR Apache-2.0 |
| iana-time-zone | 0.1.65 | MIT OR Apache-2.0 |
| iana-time-zone-haiku | 0.1.2 | MIT OR Apache-2.0 |
| image | 0.25.10 | MIT OR Apache-2.0 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| inotify | 0.11.5 | ISC |
| inotify-sys | 0.1.8 | ISC |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| jni | 0.22.4 | MIT OR Apache-2.0 |
| jni-macros | 0.22.4 | MIT OR Apache-2.0 |
| jni-sys | 0.3.1 | MIT OR Apache-2.0 |
| jni-sys | 0.4.1 | MIT OR Apache-2.0 |
| jni-sys-macros | 0.4.1 | MIT OR Apache-2.0 |
| js-sys | 0.3.105 | MIT OR Apache-2.0 |
| khronos-egl | 6.0.0 | MIT/Apache-2.0 |
| lewton | 0.10.2 | MIT OR Apache-2.0 |
| libc | 0.2.189 | MIT OR Apache-2.0 |
| libloading | 0.8.9 | ISC |
| libm | 0.2.16 | MIT |
| libredox | 0.1.25 | MIT |
| libudev-sys | 0.1.4 | MIT |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| linux-raw-sys | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| litrs | 1.0.0 | MIT OR Apache-2.0 |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |
| log | 0.4.34 | MIT OR Apache-2.0 |
| mach2 | 0.6.0 | BSD-2-Clause OR MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| memmap2 | 0.9.11 | MIT OR Apache-2.0 |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 |
| naga | 30.0.1 | MIT OR Apache-2.0 |
| naga-types | 30.0.1 | MIT OR Apache-2.0 |
| ndk | 0.9.0 | MIT OR Apache-2.0 |
| ndk-context | 0.1.1 | MIT OR Apache-2.0 |
| ndk-sys | 0.6.0+11769913 | MIT OR Apache-2.0 |
| nix | 0.31.3 | MIT |
| num-derive | 0.4.2 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| num_enum | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 |
| num_enum_derive | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 |
| objc-sys | 0.3.5 | MIT |
| objc2 | 0.5.3 | MIT |
| objc2 | 0.6.5 | MIT |
| objc2-app-kit | 0.2.2 | MIT |
| objc2-app-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-audio-toolbox | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-avf-audio | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-cloud-kit | 0.2.2 | MIT |
| objc2-contacts | 0.2.2 | MIT |
| objc2-core-audio | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-core-audio-types | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-core-data | 0.2.2 | MIT |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-core-graphics | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-core-image | 0.2.2 | MIT |
| objc2-core-location | 0.2.2 | MIT |
| objc2-encode | 4.1.0 | MIT |
| objc2-foundation | 0.2.2 | MIT |
| objc2-foundation | 0.3.2 | MIT |
| objc2-io-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-io-surface | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-link-presentation | 0.2.2 | MIT |
| objc2-metal | 0.2.2 | MIT |
| objc2-metal | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-quartz-core | 0.2.2 | MIT |
| objc2-quartz-core | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-symbols | 0.2.2 | MIT |
| objc2-ui-kit | 0.2.2 | MIT |
| objc2-uniform-type-identifiers | 0.2.2 | MIT |
| objc2-user-notifications | 0.2.2 | MIT |
| ogg | 0.8.0 | BSD-3-Clause |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| orbclient | 0.3.55 | MIT |
| ordered-float | 5.4.0 | MIT |
| owned_ttf_parser | 0.25.1 | Apache-2.0 |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 |
| pin-project | 1.1.13 | Apache-2.0 OR MIT |
| pin-project-internal | 1.1.13 | Apache-2.0 OR MIT |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| plain | 0.2.3 | MIT/Apache-2.0 |
| png | 0.18.1 | MIT OR Apache-2.0 |
| polling | 3.11.0 | Apache-2.0 OR MIT |
| pollster | 0.4.0 | Apache-2.0/MIT |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT |
| portable-atomic-util | 0.2.8 | Apache-2.0 OR MIT |
| presser | 0.3.1 | MIT OR Apache-2.0 |
| proc-macro-crate | 3.5.0 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| profiling | 1.0.18 | MIT OR Apache-2.0 |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 |
| quick-xml | 0.41.0 | MIT |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |
| range-alloc | 0.1.5 | MIT OR Apache-2.0 |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib |
| raw-window-metal | 1.1.0 | MIT OR Apache-2.0 |
| redox_syscall | 0.4.1 | MIT |
| redox_syscall | 0.5.18 | MIT |
| redox_syscall | 0.9.5 | MIT |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 |
| rfd | 0.17.2 | MIT |
| rustc-hash | 1.1.0 | Apache-2.0/MIT |
| rustix | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| rustix | 1.1.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| sctk-adwaita | 0.10.1 | MIT |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| simd_cesu8 | 1.2.0 | Apache-2.0 OR MIT |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 |
| slab | 0.4.12 | MIT |
| slotmap | 1.1.1 | Zlib |
| smallvec | 1.16.1 | MIT OR Apache-2.0 |
| smithay-client-toolkit | 0.19.2 | MIT |
| smol_str | 0.2.2 | MIT OR Apache-2.0 |
| spirv | 0.4.0+sdk-1.4.341.0 | Apache-2.0 |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |
| strict-num | 0.1.1 | MIT |
| syn | 2.0.119 | MIT OR Apache-2.0 |
| syn | 3.0.5 | MIT OR Apache-2.0 |
| termcolor | 1.4.1 | Unlicense OR MIT |
| thiserror | 1.0.69 | MIT OR Apache-2.0 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 |
| tiny-skia | 0.11.4 | BSD-3-Clause |
| tiny-skia-path | 0.11.4 | BSD-3-Clause |
| tinyvec | 1.13.3 | Zlib OR Apache-2.0 OR MIT |
| toml_datetime | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_edit | 0.25.15+spec-1.1.0 | MIT OR Apache-2.0 |
| toml_parser | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 |
| tracing | 0.1.44 | MIT |
| tracing-core | 0.1.36 | MIT |
| ttf-parser | 0.25.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |
| uuid | 1.29.0 | Apache-2.0 OR MIT |
| vec_map | 0.8.2 | MIT/Apache-2.0 |
| wasip2 | 1.0.1+wasi-0.2.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasm-bindgen | 0.2.128 | MIT OR Apache-2.0 |
| wasm-bindgen-futures | 0.4.78 | MIT OR Apache-2.0 |
| wasm-bindgen-macro | 0.2.128 | MIT OR Apache-2.0 |
| wasm-bindgen-macro-support | 0.2.128 | MIT OR Apache-2.0 |
| wasm-bindgen-shared | 0.2.128 | MIT OR Apache-2.0 |
| wayland-backend | 0.3.17 | MIT |
| wayland-client | 0.31.15 | MIT |
| wayland-csd-frame | 0.3.0 | MIT |
| wayland-cursor | 0.31.14 | MIT |
| wayland-protocols | 0.32.13 | MIT |
| wayland-protocols-plasma | 0.3.12 | MIT |
| wayland-protocols-wlr | 0.3.12 | MIT |
| wayland-scanner | 0.31.11 | MIT |
| wayland-sys | 0.31.11 | MIT |
| web-sys | 0.3.105 | MIT OR Apache-2.0 |
| web-time | 1.1.0 | MIT OR Apache-2.0 |
| wgpu | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core-deps-apple | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core-deps-emscripten | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core-deps-windows-linux-android | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-hal | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-naga-bridge | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-types | 30.0.1 | MIT OR Apache-2.0 |
| winapi-util | 0.1.11 | Unlicense OR MIT |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 |
| winit | 0.30.13 | Apache-2.0 |
| winnow | 1.0.4 | MIT |
| wit-bindgen | 0.46.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| x11-dl | 2.21.0 | MIT |
| x11rb | 0.13.2 | MIT OR Apache-2.0 |
| x11rb-protocol | 0.13.2 | MIT OR Apache-2.0 |
| xcursor | 0.3.11 | MIT |
| xkbcommon-dl | 0.4.2 | MIT |
| xkeysym | 0.2.1 | MIT OR Apache-2.0 OR Zlib |
| zerocopy | 0.8.57 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerocopy-derive | 0.8.57 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zlib-rs | 0.6.7 | Zlib |
| zmij | 1.0.23 | MIT |
| zune-core | 0.5.3 | MIT OR Apache-2.0 OR Zlib |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib |

Build-time only (build scripts and their helpers; not in the program, listed so the check sees every crate in Cargo.lock): 17.

| Crate | Version | Licence |
|---|---|---|
| autocfg | 1.5.1 | Apache-2.0 OR MIT |
| cc | 1.4.6 | MIT OR Apache-2.0 |
| cfg_aliases | 0.2.2 | MIT |
| find-msvc-tools | 0.1.12 | MIT OR Apache-2.0 |
| getrandom | 0.4.3 | MIT OR Apache-2.0 |
| gl_generator | 0.14.0 | Apache-2.0 |
| jobserver | 0.1.35 | MIT OR Apache-2.0 |
| khronos_api | 3.1.0 | Apache-2.0 |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 |
| same-file | 1.0.6 | Unlicense/MIT |
| semver | 1.0.28 | MIT OR Apache-2.0 |
| shlex | 2.0.1 | MIT OR Apache-2.0 |
| version_check | 0.9.5 | MIT/Apache-2.0 |
| walkdir | 2.5.0 | Unlicense/MIT |
| xml-rs | 0.8.29 | MIT |

<!-- third-party:crates:end -->
