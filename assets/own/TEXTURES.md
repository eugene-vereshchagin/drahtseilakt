# The own textures for the levels, their parts and the balls

`Textures/<name>.png` stands in for the original's texture `<name>` wherever a level, a part or the ball module asks
for it (by its name or its file's, compared without case, `_`, `-` or spaces: `BallWood.bmp` is `Ball_Wood`), when
the textures setting is `own` (the default; resources.md, "Own textures"). Each is made by `make/make_textures.py`
from the CC0 photo textures in `CREDITS.md` and shapes drawn there, to the palette, grain and layout measured on the
original's texture of the same name, at twice its resolution; nothing of the original is in them.

Every texture the original levels (1 to 13) and the parts they use draw with, as the game exports them: what it is,
the original's size, in how many levels it appears, how many triangles carry it over all thirteen, and whether ours
stands in for it (*ours*) or it still comes from the player's copy (*copy*: skies, particles, glows, shadows and a
few pictures and transparent parts, a later stage). The sky boxes (`Textures/Sky/Sky_A..L_*`) and the particles
(`Textures/Particles/`) come from the copy too. *Score* is how close ours comes to the original's texture
(`make/metric.py`'s score, 0 to 100, ours scaled to the original's size; mean 65.4 over the 42 distinct textures
scored): high where the layout is measured exactly (floors, stripes, target), lower where only the kind of thing is
matched (boards and grain fall elsewhere than the original's).

| Texture | What it is | Size | Levels | Triangles | Ours | Score |
|---|---|---|---:|---:|---|---:|
| `Rail_Environment` | the rails' reflection (chrome mapping) | 32x32 | 13 | 222963 | ours | 65.2 |
| `Floor_Side` | the floors' sides | 128x128 | 13 | 36621 | ours | 86.8 |
| `Misc_Ufo` | the balloon's envelope (reflection) | 128x128 | 13 | 20384 | ours | 60.4 |
| `BallStone.bmp` | the stone ball (Balls) | 128x128 | 13 | 20280 | ours | 57.4 |
| `Column_blue` | the blue columns | 128x128 | 13 | 16978 | ours | 71.5 |
| `BallWood.bmp` | the wooden ball (Balls) | 128x128 | 13 | 14950 | ours | 51.7 |
| `Floor_Top_Flat` | the floors' tops, brick strips both sides | 128x128 | 13 | 14904 | ours | 89.5 |
| `Trafo_Environment` | the transformers' reflection | 64x64 | 13 | 12768 | ours | 43.3 |
| `BallPaper.bmp` | the paper ball (Balls) | 128x128 | 13 | 12480 | ours | 43.1 |
| `Wood_Metal` | boards with riveted iron bands (boxes, modules) | 128x128 | 13 | 12136 | ours | 51.3 |
| `PE_Bal_balloons` | the balloon's ropes and cloth | 128x128 | 13 | 10816 | copy | |
| `floor_top_Checkpoint` | the checkpoint's floor | 128x128 | 13 | 10190 | ours | 79.3 |
| `Floor_Top_Profil` | the profiled (hollow) floors' tops | 128x128 | 13 | 10085 | ours | 89.7 |
| `Metal_stained` | stained metal (modules, balloon) | 64x64 | 13 | 9860 | ours | 80.8 |
| `Column_beige` | the beige columns | 128x128 | 13 | 9296 | ours | 73.5 |
| `Wood_Plain2` | light boards | 128x128 | 7 | 8968 | ours | 52.9 |
| `Modul18` | a module's steel plate and warning stripes | 64x64 | 12 | 8754 | ours | 55.7 |
| `Trafo_Anim_Environment` | the transformer animation's reflection | 64x64 (in AnimTrafo.nmo) | 13 | 6032 | ours |  |
| `Laterne_Verlauf` | the lanterns' light cone | 16x16 | 13 | 5920 | copy | |
| `Floor_Top_Borderless` | the floors' tops without strips | 128x128 | 13 | 4650 | ours | 91.0 |
| `Column_beige_fade` | the columns fading downwards | 128x128 | 13 | 4576 | ours | 87.5 |
| `Wood_Panel` | dark boards in a frame | 128x128 | 10 | 4322 | ours | 49.5 |
| `Dome` | the dome's reflection | 128x128 | 7 | 3360 | ours | 47.2 |
| `PE_Bal_platform` | the balloon's platform | 256x256 | 13 | 3042 | copy | |
| `E_Holzbeschlag` | a crate's side in a riveted iron frame | 128x128 | 13 | 2699 | ours | 54.4 |
| `Modul18_Gitter` | a module's grate (transparent) | 64x64 | 12 | 2542 | ours | 54.8 |
| `Ball_Stone` | stone pieces (stone ball pickup, modules) | 128x128 | 13 | 2510 | ours | 57.4 |
| `Ball_LightningSphere3` | the new ball's lightning | 64x64 | 13 | 2288 | copy | |
| `Wood_Old` | old weathered boards | 128x128 | 13 | 2142 | ours | 51.9 |
| `Ball_Paper` | paper pieces (paper ball pickup) | 128x128 | 13 | 2082 | ours | 43.1 |
| `P_Ball_Wood` | the wooden ball pickup | 128x128 (in P_Ball_Wood.nmo) | 7 | 1960 | ours | 51.7 |
| `Ball_Wood` | wooden pieces (modules) | 128x128 | 13 | 1764 | ours | 51.7 |
| `P_Extra_Life_Oil` | the extra life's oil drop | 128x128 | 13 | 1664 | copy | |
| `Modul11_13_Wood` | light boards with a strap | 128x128 | 8 | 1596 | ours | 45.8 |
| `Tower_Roof` | the towers' roofs (shingles) | 64x64 | 13 | 1550 | ours | 57.5 |
| `Modul03_Wall` | a module's wall with a sign | 256x256 | 4 | 1536 | copy | |
| `Floor_Top_Border` | the floors' tops, a brick strip one side | 128x128 | 13 | 1490 | ours | 90.3 |
| `Laterne_Glas` | the lanterns' glass | 32x32 | 13 | 1480 | ours | 90.5 |
| `Floor_Top_ProfilFlat` | the profiled floors' flat ends | 128x128 | 13 | 1272 | ours | 83.6 |
| `Wood_Misc` | a crate's parts (plank, hinges, log end) | 128x128 | 12 | 1256 | ours | 58.9 |
| `Stick_Stripes` | the sticks' red and white stripes | 32x32 | 13 | 1136 | ours | 98.4 |
| `Modul11_Wood` | light boards with straps | 128x128 | 6 | 1036 | ours | 57.7 |
| `Brick` | light bricks | 128x128 | 8 | 988 | ours | 64.7 |
| `sandsack` | the sandbags | 128x128 | 6 | 930 | ours | 49.0 |
| `Modul16` | concrete with light stones | 128x128 | 3 | 810 | ours | 77.9 |
| `Sky_Vortex` | the last level's vortex | 128x128 | 1 | 720 | copy | |
| `Target` | the target | 64x64 | 11 | 576 | ours | 82.8 |
| `Stick_Bottom` | the sticks' foot (reflection) | 64x64 | 12 | 560 | copy | |
| `Wood_Nailed` | boards with nailed planks | 128x128 | 4 | 492 | ours | 42.5 |
| `Wood_Raft` | the raft's boards | 128x128 | 12 | 360 | ours | 63.2 |
| `Laterne_Schatten` | the lanterns' shadow | 16x16 | 13 | 296 | copy | |
| `Modul03_Floor` | a module's round wooden floor | 128x128 | 4 | 240 | copy | |
| `ExtraBall` | the extra points' balls | 64x64 | 13 | 208 | copy | |
| `Trafo_FlashField` | the transformer's flash | 64x64 | 13 | 182 | copy | |
| `Wood_MetalStripes` | boards with iron straps | 128x128 | 3 | 171 | ours | 56.3 |
| `Modul30_d_Seiten.bmp` |  | 128x128 | 8 | 128 | ours | 58.2 |
| `Modul15` | concrete with thin brick strips | 128x128 | 2 | 120 | ours | 85.5 |
| `Trafo_Shadow_Big` | the transformers' shadow | 64x64 | 13 | 76 | copy | |
| `P_Extra_Life_Shadow` | the extra life's shadow | 64x64 | 13 | 26 | copy | |
| `Misc_UFO_Flash` | the balloon's flash | 128x128 | 13 | 26 | copy | |
| `FloorGlow` | the extra point's glow | 64x64 | 13 | 26 | copy | |
| `SkyLayer` | the cloud layer | 256x256 | 12 | 24 | copy | |
| `DomeShadow` | the dome's shadow | 64x64 | 7 | 14 | copy | |
