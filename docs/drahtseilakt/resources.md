# Interface resources

The game asks for its files by the original's layout; each file plays a **role** (the menu's font, the fall's sound,
the wooden ball...). Following the decision "Own content by default" ([decisions.md](decisions.md)), three
providers can fill a role:

- **own**: our files shipped with the game in `assets/own/`, laid out as the original (none yet: stage 2 makes them);
- **original**: the player's copy;
- **remaster**: the toolbox's remaster of the copy, in `levels/remastered/.interface/`.

The extended mode's **Interface** switch (own / original / remaster, setting `interface`) chooses which comes first;
it takes effect on the next start. The classic mode always plays the copy's. A role that no provider fills is
missing: the game says so once and goes on without it. A level's own folder (`<level>/game/`, see
[open-format.md](open-format.md)) still comes first for its parts.

Text the game's font cannot draw (its font has Latin-1 only) is drawn with the shipped Noto fonts
(`assets/fonts`, SIL OFL): the Chinese and Cyrillic names of community maps.

| Role | The original's file | Own |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | planned |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | planned |
| texts in the copy's languages | `3D Entities/Language.nmo` | planned |
| menu texts, level table defaults | `base.cmo` | planned |
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | planned |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | planned |
| transformer animation | `3D Entities/AnimTrafo.nmo` | planned |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | planned |
| game font texture | `Textures/Font_1.tga` | planned |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | planned |
| ball shadow | `Textures/HardShadow.bmp` | planned |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | planned |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | planned |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | ours (assets/own, CREDITS.md) |
| menu sounds | `Sounds/Menu_click, Menu_load` | ours (assets/own, CREDITS.md) |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | ours (assets/own, CREDITS.md) |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | ours: nine pieces, Music/music.txt maps them onto the five themes; no atmosphere part |
| original levels | `3D Entities/Level/Level_NN.NMO` | planned |

Own content comes in stages: interface (fonts, buttons, points box, life balls, texts, menu scene), then balls,
skies, particles and sounds (from open libraries), music in the spirit of a mahjong game with Chinese motifs,
then modules and levels of our own.

Sounds and music: the classic mode and the extended mode's *own* interface play ours (assets/own: Ogg Vorbis made by
`assets/own/prepare.py` from openly licensed recordings, CC0 or CC-BY, authors and changes in
`assets/own/CREDITS.md`, also named on the Credits screen); the *original* or *remaster* interface plays the copy's.
Our music never falls back to the copy's; a sound we lack does. The level's music is decoded ahead on a thread of its
own.
