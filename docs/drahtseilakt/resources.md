# Interface resources

The game asks for its files by the original's layout; each file plays a **role** (the menu's font, the fall's sound,
the wooden ball...). Following the decision "Own content by default" ([decisions.md](decisions.md)), three
providers can fill a role:

- **own**: our files shipped with the game in `assets/own/`, laid out as the original (the interface since stage 2:
  see "The own interface" below);
- **original**: the player's copy;
- **remaster**: the toolbox's remaster of the copy, in `levels/ballance-remaster/.interface/`.

The extended mode's **Interface** switch (own / original / remaster, setting `interface`) chooses which comes first;
it takes effect on the next start. The default, and what the classic mode plays, is the copy's own interface when the copy has its menu, else
**own** (decisions.md, "The copy's interface by default while ours is redone"). A role that no provider fills is
missing: the game says so once and goes on without it. A level's own folder (`<level>/game/`, see
[open-format.md](open-format.md)) still comes first for its parts.

Text the copy's font cannot draw (its font has Latin-1 only) is drawn with the shipped fonts (`assets/fonts`, SIL
OFL), as the own interface draws all its text: Russian and Chinese menus with the original's interface chosen, and the
Chinese and Cyrillic names of community maps.

| Role | The original's file | Own |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | Tektur (`assets/fonts`), drawn by the game |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | ours in the open format: `3D Entities/MenuLevel/` |
| texts in the copy's languages | `3D Entities/Language.nmo` | `Texts/menu.txt`, seven languages |
| menu texts, level table defaults | `base.cmo` | planned |
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | a level of ours carries its own settings (open-format.md) |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | ours in the open format: `3D Entities/Balls/` |
| transformer animation | `3D Entities/AnimTrafo.nmo` | ours: `3D Entities/AnimTrafo/` |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | ours: `3D Entities/PH/<module>/`, every module of the original |
| game font texture | `Textures/Font_1.tga` | Tektur, rasterised by the game |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | ours, same names |
| ball shadow | `Textures/HardShadow.bmp` | ours: `Textures/HardShadow.png` |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | ours for the letters A, B, C: `Textures/Sky/Sky_<letter>_<face>.png`; and the photographic skies by name (levels.md, "Skies"): `Mountains`, `Forest`, `Sea`, `Storm`, `Sunset`, `Night`, `Desert`, `Mist`, `Clouds` (`Sunset` and `Clouds` with only haze below the horizon: the game draws their sea of cloud when the level has one), `Textures/Sky/Sky_<name>_<face>.jpg` with a sixth face, `Top` (`make/make_skies.py`, CC0 photographs from Poly Haven, `CREDITS.md`) |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | ours, the same names as `.png` |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | sound themes (assets/sounds) |
| menu sounds | `Sounds/Menu_click, Menu_load` | sound themes (assets/sounds) |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | sound themes (assets/sounds) |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | ours: *system* eighteen tracks and eight atmospheres, Music/music.txt gives each part a pool (themes 1–3 the first nine tracks, themes 4 and 5 nine more); *neuro* fifteen synthesized phrases, one per part, its own atmospheres and end music |
| original levels | `3D Entities/Level/Level_NN.NMO` | the shipped sets, `assets/levels/` (levels.md) |

## The own interface

Stage 2 (2026-11-03): the menus, the in-game interface (points box, life balls, "ball lost"), the end screen and the
loading screen are our own, so a player with no copy of the original's interface files sees all of them, and a copy's
interface files are not even read while the own interface is in use.

- **Textures** (`assets/own/Textures/`): `Button01_select.tga`, `Button01_deselect.tga` and `Button01_special.tga`, each
  part where the game's code takes it from (the UV rectangles in `menu.rs`, `hud.rs`, `endscreen.rs`), in the
  original's kind of look made our own way (version 2, 2026-11-03): see-through glass, neutral grey or (chosen) dark
  red; steel tubes shaded as lit cylinders with a brushed grain; channels of tubes with a tinted fill (lilac grey,
  gold when chosen) curling round a chrome ball that reflects a sky; the highscore places with a pearl, the life ball
  and its frame, the points box with its rail, the end screen's glowing bar, the dark band.
- **The menu scene** (`assets/own/3D Entities/MenuLevel/`, the open level format): what the menus and the loading
  screen stand over, drawn by the game as it draws the original's MenuLevel.nmo, its camera going round the same
  circle with the same lens. An island of track pieces hanging in a dusk sky: a central block with a dark bowl,
  arms of track with raised borders, the start with four cups and a stone ball, three transformer boxes, two lantern
  towers on tall poles, steel rails and a spiral, a wooden ramp, columns reaching down into the clouds, and far
  floors. Its camera's curve and target are the nodes `I_MenuLevel_Curve_0..3` and `Cam_MenuLevel_Target`, the
  flames burn at `FourFlames_Flame_A..D`, and its own sky is the `Sky_*` objects, drawn round the camera.
- **How they are made**: by `assets/own/make/make_interface.py` (the textures) and `make_menu_scene.py` (the scene,
  its textures and sky), Python with numpy and Pillow, from nothing but the scripts and the shipped Tektur font; the
  outputs are committed, so the build needs no Python. As the generated sounds were: the original is measured (its
  palette, line widths, proportions and layout; its scene's floor heights, extents and sky colours by elevation),
  ours is drawn to those numbers, and `metric.py` scores how close each element and each screen comes (Lab colour
  histogram overlap, SSIM of the blurred lightness and of the edge maps); `scores.md` keeps the scores. Nothing of
  the original is copied, traced or resampled, and its pictures never enter the repository.
- **Photo textures** (version 3 of the menu scene, 2026-11-03): its concrete (floors, block, columns), brick borders,
  wooden ramp and crate and iron (collars, plinths, lantern posts) are made from freely licensed photo textures (CC0,
  ambientCG), graded by `make_menu_scene.py` to the measured palette and grain of the original's textures in those
  roles, with their normal maps. `assets/own/make/free_textures.py` pins each source archive by its SHA-256 (`fetch`
  downloads the missing ones, nothing is extracted or run); the sources are not shipped, the textures made from them
  are. Sources, authors and licences: `assets/own/CREDITS.md`; checksums of the shipped files: `assets/own/SHA256SUMS`.
- **Font**: Tektur (SIL OFL, `assets/fonts/Tektur-Variable.ttf`, pinned and checksummed in `assets/fonts/README.md`),
  a squared technical face close in feel to the original's wide menu font, drawn with small capitals for the lower
  case, widened and spaced to the original's measured proportions (its labels' widths within 2 %). What it lacks comes from Noto Sans, Chinese from Noto Sans SC.
  The glyphs are rasterised at run time into an atlas and laid out with the same rectangles, alignment, word wrap and
  shadow as the game's font, so the screens keep the original's layout (`crates/game/src/fallback.rs`).
- **Texts** (`assets/own/Texts/menu.txt`, built into the game): every menu, in-game and end screen text in English,
  Russian, German, Simplified Chinese, French, Spanish and Italian, under the original's `Language.nmo` keys where it
  has one. With the original's interface its copy's texts come first in the copy's five languages; for Russian and
  Chinese (which the copy lacks) ours, drawn in the shipped fonts.

The own interface's default highscore lists are ours too (ten names, 3000 points down to 750). Still from the copy when
there is one: the default keys (base.cmo's DB_Options; the arrows, Shift and Space without it).

## Our own parts

Stage 3 (2026-11-03, decisions.md "The game plays out of the box"): every part a level can name has ours, in the open
format under `assets/own/3D Entities/` (the modules of `PH/`, `Balls/`, `AnimTrafo/`) and `assets/own/Textures/`
(skies, particles, the ball's shadow; a picture asked for as `.bmp` or `.tga` may be ours as `.png`), made by
`assets/own/make/make_parts.py` from nothing but the script. They are made to the original's measured sizes: every
object the game's code names (parts, joint frames, flame frames, the balls' meshes, whose boxes the physics uses) has
the measured pose, every collision mesh the measured dimensions as a shape of our own (a box, a frustum, a prism,
an ellipsoid), and the physics values (masses, frictions, joints, forces) are the game's own, unchanged
(crates/game/src/modules.rs); only the looks are new. `crates/game/src/partcheck.rs` runs every module of levels 1
to 12 with the copy's part and with ours from the same start and input and compares the ways.

Where a part's drawn shape is richer than its plain one, the plain shape stays the object's mesh and the drawn one is
its look (open-format.md): the physics takes its hull from the mesh, and the test player reads the modules' meshes as
floors and walls, so both see the same thing whatever is drawn. The lift (`P_Modul_03`, 2026-11-12) is drawn this
way: a turned metal disc with a bolted rim and an oak deck, its loose panels red-painted frames with a boarded foot
and bars above (the ball is seen through them); its slab, panels and gate are unchanged as meshes.
Five more parts are drawn this way since (second round, 2026-11-12), every object, pivot and plain mesh as before: the
sack (`P_Modul_26`: a sewn canvas sack tied at the neck, a twisted rope with an iron eye hanging in the ring of a
bolted plate), the flap wheel (`P_Modul_19`: framed panels of oak boards, a spoked wheel, bearing collars and
flanges), the turning arms (`P_Modul_17`: a painted, banded arm on a turned drum, framed paddles with a whole target on
each face), the fan (`P_Modul_18`: hub, four twisted blades, a shroud ring and a wire guard, all of which turn with
the rotor, so all but the blades are round about the axle) and the pusher (`P_Modul_01`: a target on a framed head
plate, a painted girder, the upright a framed panel). Their new materials (`..._Green`, `..._Bullseye`, `..._Cloth`
and the like) are not among the texture pack's roles, so they are always drawn with the part's own pictures
(`paint_green`, `bullseye`, `sack_cloth` and the lift's).

A level takes our parts first when it says it is made for them (its settings' `"parts": "own"`, which our level kit
`assets/own/make/levelkit.py` writes), so it plays the same with or without a copy; any other level (the copy's, a
community map, a converted, remastered or older open-format level) takes the copy's first, as it was made, and ours
for what the copy lacks (with no copy, ours; with a copy, our part standing in is said once). A level's own `game/` folder comes before both.

## Own textures

Stage 3 (2026-11-03): the levels, their parts and the balls are drawn with our own textures by default (decisions.md,
"Own textures by default"): `assets/own/Textures/<name>.png` stands in for the original's texture of that name,
whichever level asks for it — an original level, a community map, a module, the balls — compared by name without case,
`_`, `-` or spaces (`BallWood.bmp` is `Ball_Wood`). 45 of the 63 textures the thirteen levels draw with are ours (the
floors and their brick strips, the checkpoint, the columns, bricks, every wood, the metal, the roofs, the markings, the
reflections, the three balls); the skies, particles, glows, shadows and a few pictures still come from the copy.
`assets/own/TEXTURES.md` lists every texture with what it is, its size, how often it is used and whether ours stands in.
They are made by `assets/own/make/make_textures.py` from CC0 photo textures (`free_textures.py`, pinned and
checksummed; `assets/own/CREDITS.md`) and shapes drawn by the script, to the palette, grain and layout measured on the
original's texture of the same name (its brick strips, bands, rings and shading as fractions of its side), at twice its
resolution, tiling where it tiles and wrapping the balls as it does. Our textures are the built-in texture pack `own`:
the extended page's *Textures* line chooses among the texture packs ([packs.md](packs.md): `own`, `original` the
copy's, `remastered` a remastered level's own upscaled pictures, and the player's packs in the data folder's
`textures/`), its *Models* line among the model packs (the parts' and balls' looks only, never their physics); the
classic mode offers the same lines on its Graphics page, with ours and the parts each level takes as the defaults. An open-format level's own textures (files in its folder)
stay its maker's; the textures it names as the game's (`game:` images, open-format.md) follow the pack. `--textures
PACK` chooses for one run; `--export` keeps the copy's unless given `--textures own`, which writes ours into the
exported level. Our files ship with the game and are
read through the same size caps as any texture; a level's name only picks among them, never makes a path.

The balls and the dome are the exception (2026-11-11): they are our own designs, not matched to the original's
textures. `assets/own/make/ballskins.py` draws the wooden ball as a blank glued up of blocks, the stone ball as
grey-white veined marble and the paper ball as a crumpled newspaper, each with a normal map, and the dome is riveted
brass ribs over blue enamel panels; the bodies the physics uses are unchanged (`assets/own/TEXTURES.md`).

Which textures ours (or any texture pack's) replace, when a level is loaded (so drawing and export agree): in the copy's levels and parts, and
in their remasters (`levels/ballance-remaster/`), every texture we have one of, by name; in a stranger's map (a community
`.nmo`) a texture it names by file when that file is the copy's (a file its own folder brings stays its own), and
pixels it carries only when they are the copy's texture of that name, pixel for pixel, so a map's own pictures are
never covered; an open-format level's own files never. A texture's file name from a `.nmo` is only ever its last part,
looked for in the game's texture folders; only an open-format level's own folder is read by path. The menu scene
follows the interface: ours with the own interface, the copy's MenuLevel with its own textures with the original's.

Own content comes in stages: interface (fonts, buttons, points box, life balls, texts, menu scene), then balls,
skies, particles and sounds (from open libraries), music in the spirit of a mahjong game with Chinese motifs,
then modules and levels of our own.

Sounds and music come in **sound themes**, each a folder (`Sounds/` and `Music/` by the names the game asks for,
Ogg Vorbis or WAV, `Music/music.txt` and `Sounds/sounds.txt` tables giving a role several files, a pool the game picks from at random, a `theme.txt` with its name in four languages, author,
licence and `fallback`, `none` for no other theme). The game ships two in `assets/sounds/`: *system* (openly licensed recordings and the owner's
music, made by `assets/sounds/system/prepare.py`, CC0 or CC-BY, authors and changes in its `CREDITS.md`, also on the
Credits screen) and *neuro* (effects and level music synthesized for the game, GPL-3.0-only; the level music is
short guzheng-like plucked phrases, one for each of the five themes' three parts; it holds every role the game asks for and takes nothing from another
theme, `fallback none`).
Players add their own themes in the data folder's `sounds/<name>/`; *original* among them is made by the toolbox from
the player's copy and never shipped (until it exists, choosing it plays the copy's sounds). The extended page's
*Sounds* line chooses the theme (`sound-theme NAME`); *neuro* plays until the player chooses another, and always in
the classic mode (decisions.md, "The generated sound theme is the default"); a choice once made is kept. A sound the theme and its
fallback lack comes from the copy (a level's own sounds); music never does. A player's theme folder is untrusted:
plain names only, no links, a capped number of themes, sounds decoded through the audio caps. The music that may play
next is decoded ahead on a thread of its own. How to make a theme, every role and the tables' format:
[Sound themes](sound-themes.md).
