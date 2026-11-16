# Credits: the game's own sounds and music

Every file in `Sounds/` and `Music/` is made from an openly licensed recording (CC0 1.0 or CC-BY 3.0/4.0) by
`prepare.py`: cut, made mono or stereo, matched in loudness to the original's sound of the same role, looped
without a seam where the game loops it (crossfaded), the stone ball's rolling and dome hit pitched down, the
level music cut to at most 100 s and faded in (1 s) and out (5 s), all encoded as Ogg Vorbis. The third round's
level tracks (2026-11-03) are matched in loudness to the first nine (their mean, about −17 LUFS). The atmospheres
(`atmo_*`) are cut to about 27 s (shorter sources kept whole), shaped into one slow swell and fade like the
original's, matched to the original's atmospheres (about −32 LUFS); the shakuhachi's cicadas are filtered out (low
pass at 2.6 kHz) and the tonal wind's rumble (high pass at 90 Hz). `Music/music.txt` says which track plays in which
role; several in one role form a pool the game picks from. Freesound sources are the public 128 kbps HQ previews
of the sounds named. The CC-BY works are used under their licences with the attribution below; the changes are
those just listed; the fixes of 2026-11-12 are listed in their rows (`prepare.py --fixes`: rail loops, iron-ring rail hits, synthesized fall); `Misc_Trafo` is slowed to 2.5 s (time-stretch, pitch lowered), raised 6 dB and faded out. Licences:
CC0 1.0 https://creativecommons.org/publicdomain/zero/1.0/, CC-BY 3.0 https://creativecommons.org/licenses/by/3.0/,
CC-BY 4.0 https://creativecommons.org/licenses/by/4.0/.

| Our file | Plays as | Work | Author | Licence | Source |
|---|---|---|---|---|---|
| `Sounds/Misc_StartLevel.ogg` | Misc_StartLevel | 121800_1525789-hq.mp3 | BOSS MUSIC | CC0 1.0 | https://freesound.org/people/BOSS%20MUSIC/sounds/121800/ |
| `Sounds/Misc_Checkpoint.ogg` | Misc_Checkpoint | 169855_3126007-hq.mp3 | GnoteSoundz | CC0 1.0 | https://freesound.org/people/GnoteSoundz/sounds/169855/ |
| `Sounds/Misc_Fall.ogg` | Misc_Fall | synthesized (prepare.py `fall_sound`: falling glide 490 to 90 Hz and a soft thud, 1.9 s, -22 LUFS) | Drahtseilakt project | GPL-3.0-only | no source (replaces freesound 159408 by noirenex, CC0, whose steady 700 Hz tone was harsh) |
| `Sounds/Misc_Lightning.ogg` | Misc_Lightning | spark.wav | BMacZero | CC0 | https://opengameart.org/content/electricity-sound-effects-0 |
| `Sounds/Misc_Trafo.ogg` | Misc_Trafo | qubodup-megaswosh1.wav | qubodup | CC0 | https://opengameart.org/content/wind-hit-time-morph |
| `Sounds/Extra_Start.ogg` | Extra_Start | glass_004.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/interface-sounds |
| `Sounds/Extra_Hit.ogg` | Extra_Hit | glass_002.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/interface-sounds |
| `Sounds/Extra_Life_Blob.ogg` | Extra_Life_Blob | life_pickup.flac | Lamoot | CC-BY 3.0 | https://opengameart.org/content/life-pickup-yo-frankie |
| `Sounds/Misc_extraball.ogg` | Misc_extraball | powerUp2.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/digital-audio |
| `Sounds/Misc_RopeTears.ogg` | Misc_RopeTears | 443833_9184212-hq.mp3 | checholio | CC0 1.0 | https://freesound.org/people/checholio/sounds/443833/ |
| `Sounds/Music_thunder.ogg` | Music_thunder | 446753_1790434-hq.mp3 | BlueDelta | CC0 1.0 | https://freesound.org/people/BlueDelta/sounds/446753/ |
| `Sounds/Misc_Ventilator.ogg` | Misc_Ventilator | 242042_1900515-hq.mp3 | Ezcah | CC0 1.0 | https://freesound.org/people/Ezcah/sounds/242042/ |
| `Sounds/Menu_click.ogg` | Menu_click | click1.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/ui-audio |
| `Sounds/Menu_load.ogg` | Menu_load | select_001.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/interface-sounds |
| `Sounds/Roll_Paper.ogg` | Roll_Paper | 545532_6233742-hq.mp3 | rsellick | CC0 1.0 | https://freesound.org/people/rsellick/sounds/545532/ |
| `Sounds/Roll_Wood_Wood.ogg` | Roll_Wood_Wood | 629001_13875907-hq.mp3 | KaleidacousticsAudio | CC0 1.0 | https://freesound.org/people/KaleidacousticsAudio/sounds/629001/ |
| `Sounds/Roll_Wood_Stone.ogg` | Roll_Wood_Stone | 182345_1864955-hq.mp3 | Claire.H | CC-BY 3.0 | https://freesound.org/people/Claire.H/sounds/182345/ |
| `Sounds/Roll_Wood_Metal.ogg` | Roll_Wood_Metal | !Roll_Wood_Metal_C.ogg ('hollow tube', modal synthesis) | Drahtseilakt project | GPL-3.0-only | tools/audio-synth/rails.py (replaces freesound 537939 by belanhud, CC0, a clatter like maracas) |
| `Sounds/Roll_Stone_Wood.ogg` | Roll_Stone_Wood | 629001_13875907-hq.mp3 | KaleidacousticsAudio | CC0 1.0 | https://freesound.org/people/KaleidacousticsAudio/sounds/629001/ |
| `Sounds/Roll_Stone_Stone.ogg` | Roll_Stone_Stone | 182345_1864955-hq.mp3 | Claire.H | CC-BY 3.0 | https://freesound.org/people/Claire.H/sounds/182345/ |
| `Sounds/Roll_Stone_Metal.ogg` | Roll_Stone_Metal | !Roll_Stone_Metal_A.ogg ('growl + ring', modal synthesis) | Drahtseilakt project | GPL-3.0-only | tools/audio-synth/rails.py (replaces freesound 537939 by belanhud, CC0) |
| `Sounds/Hit_Wood_Wood.ogg` | Hit_Wood_Wood | impactWood_medium_000.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/impact-sounds |
| `Sounds/Hit_Wood_Stone.ogg` | Hit_Wood_Stone | impactGeneric_light_000.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/impact-sounds |
| `Sounds/Hit_Wood_Metal.ogg` | Hit_Wood_Metal | impactMetal_light_000.ogg + iron ring | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/impact-sounds ; changed: iron ring of five partials added, 0.9 s, low-passed at 5 kHz |
| `Sounds/Hit_Wood_Dome.ogg` | Hit_Wood_Dome | impactPlate_light_000.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/impact-sounds |
| `Sounds/Hit_Stone_Stone.ogg` | Hit_Stone_Stone | impactMining_000.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/impact-sounds |
| `Sounds/Hit_Stone_Wood.ogg` | Hit_Stone_Wood | impactWood_heavy_000.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/impact-sounds |
| `Sounds/Hit_Stone_Metal.ogg` | Hit_Stone_Metal | impactMetal_medium_000.ogg + iron ring | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/impact-sounds ; changed: iron ring (3 semitones lower) added, 0.9 s |
| `Sounds/Hit_Stone_Kuppel.ogg` | Hit_Stone_Kuppel | impactPlate_light_002.ogg | Kenney (www.kenney.nl) | CC0 1.0 | https://kenney.nl/assets/impact-sounds |
| `Sounds/Hit_Paper.ogg` | Hit_Paper | 334200_4290188-hq.mp3 | WasabiWielder | CC0 1.0 | https://freesound.org/people/WasabiWielder/sounds/334200/ |
| `Music/Music_Final.ogg` | Music_Final | 108248_327532-hq.mp3 | Nick-Nac | CC0 1.0 | https://freesound.org/people/Nick-Nac/sounds/108248/ |
| `Music/Music_LastFinal.ogg` | Music_LastFinal | Orient Theme.ogg | WakianTech (K-ManZ) | CC-BY 3.0 | https://opengameart.org/content/orient-theme-jingle |
| `Music/Music_EndCheckpoint.ogg` | Music_EndCheckpoint | moonlit_mirage_loop.flac | Tsorthan Grove | CC-BY 4.0 | https://opengameart.org/content/moonlit-mirage |
| `Music/ripples.ogg` | Music_Theme_1_1 | Ripples.mp3 | Kevin MacLeod | CC-BY 4.0 | https://incompetech.com/music/royalty-free/music.html (search 'Ripples') |
| `Music/guzheng_city.ogg` | Music_Theme_1_2 | Guzheng City.mp3 | Kevin MacLeod | CC-BY 4.0 | https://incompetech.com/music/royalty-free/music.html (search 'Guzheng City') |
| `Music/shenyang.ogg` | Music_Theme_1_3 | Shenyang.mp3 | Kevin MacLeod | CC-BY 4.0 | https://incompetech.com/music/royalty-free/music.html (search 'Shenyang') |
| `Music/liyan.ogg` | Music_Theme_2_1 | liyan.mp3 | elerya | CC-BY 3.0 | https://opengameart.org/content/liyan |
| `Music/neolein_472707.ogg` | Music_Theme_2_2 | 472707_4596312-hq.mp3 | neolein | CC0 1.0 | https://freesound.org/people/neolein/sounds/472707/ |
| `Music/neolein_573972.ogg` | Music_Theme_2_3 | 573972_4596312-hq.mp3 | neolein | CC0 1.0 | https://freesound.org/people/neolein/sounds/573972/ |
| `Music/cold_to_lukewarm.ogg` | Music_Theme_3_1 | cold to lukewarm mixia_0.ogg | Spring Spring | CC-BY 3.0 | https://opengameart.org/content/cold-to-lukewarm-mixia |
| `Music/misty_mountains.ogg` | Music_Theme_3_2 | RPG_-_Misty_Mountains.ogg | Hitctrl | CC-BY 3.0 | https://opengameart.org/content/rpg-misty-mountains |
| `Music/ishikari_lore.ogg` | Music_Theme_3_3 | Ishikari Lore.mp3 | Kevin MacLeod | CC-BY 4.0 | https://incompetech.com/music/royalty-free/music.html (search 'Ishikari Lore') |
| `Music/gentle_guzheng.ogg` | Music_Theme_4_1 | Gentle intro guzheng (512943_6125364-hq.mp3) | robcro6010 | CC-BY 4.0 | https://freesound.org/people/robcro6010/sounds/512943/ |
| `Music/guzheng_loop.ogg` | Music_Theme_4_1 | GUZHENG CUSTOMIZE YOUR LOOP 01 G 112 BPM (520729_197130-hq.mp3) | CarlosCarty | CC-BY 4.0 | https://freesound.org/people/CarlosCarty/sounds/520729/ |
| `Music/senbazuru.ogg` | Music_Theme_4_2 | Senbazuru (Senbazuru.mp3) | Kevin MacLeod | CC-BY 4.0 | https://incompetech.com/music/royalty-free/music.html (search 'Senbazuru') |
| `Music/finding_movement.ogg` | Music_Theme_4_3 | Finding Movement (Finding Movement.mp3) | Kevin MacLeod | CC-BY 4.0 | https://incompetech.com/music/royalty-free/music.html (search 'Finding Movement') |
| `Music/orient_peace_valley.ogg` | Music_Theme_4_3 | Orient Peace Valley (orien1_0.ogg) | Tozan | CC0 | https://opengameart.org/content/orient-peace-valley |
| `Music/blossom_valley.ogg` | Music_Theme_5_1 | blossom valley in paradise (521367_4596312-hq.mp3) | neolein | CC0 1.0 | https://freesound.org/people/neolein/sounds/521367/ |
| `Music/guzheng_fantasie.ogg` | Music_Theme_5_2 | Ghuzheng Fantasie (541306_4596312-hq.mp3) | neolein | CC0 1.0 | https://freesound.org/people/neolein/sounds/541306/ |
| `Music/asian_duet.ogg` | Music_Theme_5_2 | Asian Duet (asian_duet.mp3) | elerya | CC-BY 3.0 | https://opengameart.org/content/asian-duet |
| `Music/crane_dance.ogg` | Music_Theme_5_3 | 鹤舞白沙 (582209_5462375-hq.mp3) | Amingliang | CC-BY 4.0 | https://freesound.org/people/Amingliang/sounds/582209/ |
| `Music/atmo_quartz_bowl.ogg` | Music_Atmo_1 | Quartz crystal singing bowl (129219_649468-hq.mp3) | juskiddink | CC-BY 4.0 | https://freesound.org/people/juskiddink/sounds/129219/ |
| `Music/atmo_tibetan_bowl.ogg` | Music_Atmo_1 | Tibetan_singing_bowl (196365_1847371-hq.mp3) | anunquietmind | CC0 1.0 | https://freesound.org/people/anunquietmind/sounds/196365/ |
| `Music/atmo_mystic_forest.ogg` | Music_Atmo_1 | Mystic Forest Ambient (559389_2282212-hq.mp3) | szegvari | CC0 1.0 | https://freesound.org/people/szegvari/sounds/559389/ |
| `Music/atmo_reverb_flute.ogg` | Music_Atmo_2 | Pristine Flute w Deep Reverb (815787_2520418-hq.mp3) | CVLTIV8R | CC0 1.0 | https://freesound.org/people/CVLTIV8R/sounds/815787/ |
| `Music/atmo_shakuhachi.ogg` | Music_Atmo_2 | shakuhachi flute and Cicada (361508_313780-hq.mp3) | macdaddyno1 | CC0 1.0 | https://freesound.org/people/macdaddyno1/sounds/361508/ |
| `Music/atmo_tonal_wind.ogg` | Music_Atmo_2 | Soft Tonal Wind_02 (397092_3287894-hq.mp3) | janbezouska | CC0 1.0 | https://freesound.org/people/janbezouska/sounds/397092/ |
| `Music/atmo_winter_temple.ogg` | Music_Atmo_3 | Slow Time Winter Temple Atmo Asia Ambience Fantasy Dark Drone Cinematic Surround (612645_2282212-hq.mp3) | szegvari | CC0 1.0 | https://freesound.org/people/szegvari/sounds/612645/ |
| `Music/atmo_wind_synth.ogg` | Music_Atmo_3 | wind ambient synth (151783_1457270-hq.mp3) | botpl | CC-BY 3.0 | https://freesound.org/people/botpl/sounds/151783/ |

Attribution of the third round's CC-BY works, as their licences ask: "Gentle intro guzheng" by robcro6010, "GUZHENG
CUSTOMIZE YOUR LOOP 01 G 112 BPM" by CarlosCarty, "鹤舞白沙" by Amingliang, "Quartz crystal singing bowl" by
juskiddink (all CC-BY 4.0, freesound.org), "Senbazuru" and "Finding Movement" by Kevin MacLeod (incompetech.com,
CC-BY 4.0), "Asian Duet" by elerya (CC-BY 3.0, opengameart.org), "wind ambient synth" by botpl (CC-BY 3.0,
freesound.org); each cut, faded and levelled as described above, the quartz bowl also shaped into a swell.
| `Sounds/Hit_Sack.ogg` | Hit_Sack (the swinging sack hits the ball) | synthesized (prepare.py `sack_sound`: 85 Hz knock, low rustle, 0.4 s) | Drahtseilakt project | GPL-3.0-only | no source |
