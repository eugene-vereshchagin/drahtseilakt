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
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | sound themes (assets/sounds) |
| menu sounds | `Sounds/Menu_click, Menu_load` | sound themes (assets/sounds) |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | sound themes (assets/sounds) |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | ours: *system* nine pieces, Music/music.txt maps them onto the five themes; *neuro* fifteen synthesized phrases, one per part; no atmosphere part |
| original levels | `3D Entities/Level/Level_NN.NMO` | planned |

Own content comes in stages: interface (fonts, buttons, points box, life balls, texts, menu scene), then balls,
skies, particles and sounds (from open libraries), music in the spirit of a mahjong game with Chinese motifs,
then modules and levels of our own.

Sounds and music come in **sound themes**, each a folder (`Sounds/` and `Music/` by the names the game asks for,
Ogg Vorbis or WAV, a `music.txt` table for the level themes, a `theme.txt` with its name in four languages, author,
licence and `fallback`). The game ships two in `assets/sounds/`: *system* (openly licensed recordings and the owner's
music, made by `assets/sounds/system/prepare.py`, CC0 or CC-BY, authors and changes in its `CREDITS.md`, also on the
Credits screen) and *neuro* (effects and level music synthesized for the game, GPL-3.0-only; the level music is
short guzheng-like plucked phrases, one for each of the five themes' three parts; missing roles from *system*).
Players add their own themes in the data folder's `sounds/<name>/`; *original* among them is made by the toolbox from
the player's copy and never shipped (until it exists, choosing it plays the copy's sounds). The extended page's
*Sounds* line chooses the theme (`sound-theme NAME`); *neuro* plays until the player chooses another, and always in
the classic mode (decisions.md, "The generated sound theme is the default"); a choice once made is kept. A sound the theme and its
fallback lack comes from the copy (a level's own sounds); music never does. A player's theme folder is untrusted:
plain names only, no links, a capped number of themes, sounds decoded through the audio caps. The level's music is
decoded ahead on a thread of its own.
