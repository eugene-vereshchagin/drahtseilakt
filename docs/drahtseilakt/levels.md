# Where levels live

How the game keeps its levels, agreed with the owner on 2026-11-02 (decisions.md, "Levels live in one folder,
four kinds"). The menus that show them are in [menu.md](menu.md).

## The folders

All levels sit in one folder, `levels/`, in the game's data folder (`~/.local/share/drahtseilakt/` on Linux,
`%APPDATA%\drahtseilakt\` on Windows, `~/Library/Application Support/drahtseilakt/` on macOS), divided by where
they come from and what format they are in:

| Folder | What | Where it comes from | Mode |
|---|---|---|---|
| `levels/original/` | the original game: `base.cmo`, `3D Entities/`, `Textures/`, `Sounds/` and the rest, level 13's add-on installed into it | copied by the game from the player's installed game or CD | classic and extended |
| `levels/community/` | community maps: a folder per level, a `.nmo` or an open-format level in it (below) | the player downloads them, `drahtseilakt-toolbox import-maps` sorts a download into folders; the folder's README says where | extended |
| `levels/remastered/` | the original levels converted to the open format, with textures upscaled to 4K–8K and what else a modern look needs | made on the player's machine by `drahtseilakt-toolbox remaster` from `original/`; never distributed | extended |
| `levels/open/` | levels in the open format: made in it, or converted community maps | made by players; some come with the game | extended |

The classic mode sees only `original/`; the extended mode sees all four (menu.md).

## A level is a folder

*Owner's decision of 2026-11-03* (decisions.md, "A level is a folder"). `community/` and `open/` (and `remastered/`) hold
levels, not whatever an archive unpacked: the custom levels menu shows a flat list of levels, sorted by title, and no
folders to open.

- **A level is a folder directly in the kind's folder** holding exactly one level: one `.nmo` file, or a level in the
  open format (`level.gltf`, `level.bin`, `textures/`; open-format.md). The open format is the clear one, easy to
  finish or fix up by hand: plain glTF and PNG files.
- Beside it, all optional: `level.txt`, simple `key = value` lines (`title`, `author`, `version`; `title.<code>`, its
  title in another language, e.g. `title.ru`, shown when the menus speak it; `order`, a number placing it before the
  levels without one, in its order; `hidden = yes`, kept out of the list of the levels shipped with the game, as the
  test level is; `#` starts a comment, other keys are ignored), a README (`README.txt` or `README.md`), a preview picture (`preview.png` or
  `preview.jpg`), and the folders of resources the level uses (`textures/`, `game/`).
- **The title** is level.txt's `title`, else the folder's name. Either is cleaned (control characters, direction
  overrides and other invisible format characters removed, spaces made one) and cut to **40 characters** (characters,
  not bytes) at a word, ending in "…", which the game's font draws as "...". Everywhere the game shows a level's name
  (the menu, the loading screen, the window's title) it shows this title. level.txt is read only when it is a plain
  file of at most 4 KiB; a larger one or a link is ignored.
- **A single `.nmo` put straight into the kind's folder is a level too**, named by its file: the simplest way to try
  a map.
- Ignored by the menu: any other file in a level's folder, every folder inside it (no nesting), hidden names (a dot
  first), links (never followed), and folders with no level or with several (several `.nmo`, or a `.nmo` beside
  `level.gltf`): those are not levels and are named on the console.
- Collections are not a concept: the 13 rotated originals are 13 level folders ("45-Degree Rotated - Level 07").
- `drahtseilakt-toolbox import-maps <folder>` sorts a downloaded collection into this shape: a folder per map with
  its `.nmo`, its picture as the preview, and a level.txt that says where it came from (and its whole title when the
  folder's name had to lose characters). A generic file name (`Level_07.NMO`) is named after its collection; the same
  file found twice becomes one level; packed archives are named, not opened; names are made safe for Linux, Windows
  and macOS (no reserved names, no trailing dots or spaces, at most 40 characters, a second "Hello Mars" numbered
  "(2)"); the source is only read. A second run adds only what is new.

## Levels shipped with the game

Our own levels come with the game in its `assets/levels/`, a folder per level in the open format, made of our own
parts only (assets/own, [resources.md](resources.md)), so they play with no copy of the original at all. The menu
shows them with the open-format levels of `levels/open/`: under Start when there is no copy (the list "Our levels"),
under the extended mode's "Open format" always. They are made by scripts in `assets/own/make/` (`levelkit.py` builds
a level from track pieces and the stand-ins of the parts; `make_test_level.py` the first one, `Parts_Test`, a few
hundred units of track with every part on it or beside it and a stretch of track of each safety, for the tests; it is
hidden from the players' list by `hidden = yes` in its level.txt and still opens with `--open`), whose outputs are
committed. Their licence is the
game's (GPL-3.0-only).

## Our levels and how to edit them

*2026-11-03.* Ten levels of our own, from a first roll to the gauntlet, ship with the game (the release criterion of
roadmap.md). Each is new: its layout is ours, only the mechanics are the original's. The menu lists them in this
order (`order` in level.txt), by their names in the menus' language:

| # | Level | Sectors | Idea | Sky, music, bonus |
|---|---|---|---|---|
| 1 | First Roll | 4 | the original's first level in spirit: a wooden bridge, the first rails and a loop of them, boxes, the stone transformer and back, loose balls, a dome, the stone on the crate, two pushers, wooden ramps up between railed landings; gutters everywhere, railings where a fall is possible | A, 1, 100 |
| 2 | Wood and Stone | 4 | the wooden ball climbs the ramps the stone one cannot; the stone ball shoves crates and loose balls off ledges without borders | B, 2, 200 |
| 3 | Paper Winds | 4 | the paper ball rides fans up to the floors above; plate bridges hold it | C, 3, 300 |
| 4 | Swing Time | 4 | things that move: swinging sacks, turning arms, a flap wheel, swings that bridge a gap while they swing by | A, 4, 400 |
| 5 | Bridges and Boards | 5 | seesaws under every ball; plate bridges that break under the stone one | B, 5, 500 |
| 6 | Stone Work | 5 | crates, loose balls and domes in the way: shoved off by the stone ball, gone round by the wooden one | C, 1, 600 |
| 7 | Rail Runner | 5 | long rails through curves, up and down; narrow floors without borders | A, 2, 700 |
| 8 | Shifting Shapes | 5 | every sector wants another ball: paper for fans and plate bridges, stone for crates, wood for ramps and seesaws | B, 3, 800 |
| 9 | Leap of Faith | 5 | speed and jumps: long runs down, ramps that throw the ball over gaps, rails taken fast | C, 4, 900 |
| 10 | The Gauntlet | 6 | narrow ways, every ball and every part one after another | A, 5, 1000 |

Every level starts with the wooden ball and has checkpoints with their flames, extra points and an extra life.

**A level is written as text.** Each folder holds its description, `level.lvl`, a small language read by
`assets/own/make/leveldsl.py`, which writes the level beside it (`level.gltf`, `level.bin`, `textures/`, `level.txt`)
with `levelkit.py`. The description lays the track like a turtle: a cursor (a point on the floor's top and a heading,
along +X at the start, left is +Z) moves on with each piece, and parts are put where it is. A line is a command; `#`
begins a comment:

```
title Paper Winds            # the level's name; title.ru, title.de, title.zh: in other languages
order 3                      # its place in the list
sky C                        # A, B, C or a sky by name: Mountains, Sea, ... (below)
music 3                      # the theme, 1 to 5
bonus 300                    # the level bonus
ball wood                    # the start ball
columns 45                   # columns under the floors every 45 units of way, for the look

start                        # the start's platform; the way goes on from its front edge
safety railings              # how the track keeps the ball on: railings, gutter, border or none (below)
width 12                     # the floor's width (borders off|on, surface stone|wood)
floor 30                     # a straight floor 30 long (floor 30 up 4, floor 30 down 6: a ramp)
right 90 26                  # a floor turning 90 degrees to the right on a radius of 26 (left A R)
point                        # an extra point here (life: an extra life; side S, ahead A: beside, further on)
trafo paper                  # a transformer
fan ahead -5                 # a fan 5 back from here: a paper ball rises on it to about 17 over its floor
gap 4 up 12                  # 4 ahead and 12 up: nothing built (a jump, or the floor a fan carries the ball to)
checkpoint                   # a checkpoint (the heading a multiple of 90): the next sector begins
finish                       # the balloon, 22 ahead
```

Besides: `rails L`, `rails-left A R`, `rails-right A R` (rails, also up or down); `box`, `dome`, `loose wood|stone|paper`;
`seesaw` (over a gap of its board's length), `platebridge` (a gap of 14.8), `swingbridge` (a swing in a gap of 5),
`sack`, `arms`, `flaps`; `module P_Modul_NN` with `ahead`, `side`, `up`, `turn`, `tilt` for any other part; `move`,
`turn`, `mark NAME` and `back NAME` to move the cursor without building. The whole list is at the top of
leveldsl.py.

`railing left|right|both|off` puts steel railings along the floors' edges from there on, on any profile (level 1:
gutters, and railings where a fall is possible). Two of the original's puzzles are commands of their own: `crateblock`
(the stone on the crate, P_Modul_34: a slot of 6 across the way, the stone standing in it on its crate, which stands
on a floor 5 lower that the description lays under the slot; a ball on that floor pushes the crate out from under the
stone, and the stone slides down level with the way) and `pusher left|right` (P_Modul_01: a slot of 1.6 across a way
at most 6 wide, the pusher's block filling the way, its target plate standing 5 out past the edge on that side over a
floor 2.5 lower, from where a ball pushes the block out of the way). The floors under them are laid with `mark`,
`move`, `turn` and `back`.

**Skies.** `sky` names the sky box round the level: `A`, `B`, `C` (ours, drawn above a sea of cloud, as the
original's letters) or one of the photographic skies by name: `Mountains` (a sunset sky over hazy mountains with
snow on the peaks), `Forest` (summer clouds over a forest canopy with glades and lakes), `Sea` (a calm sea at
sunset, mirroring the clouds), `Storm` (a heavy overcast over a dark sea with whitecaps), `Sunset` (a golden sunset over
a sea of cloud), `Clouds` (a summer day over a sea of cloud), `Night` (the Milky Way over a dark lake ringed by hills), `Desert` (a late afternoon over red
canyons), `Mist` (an evening glow over hills with mist in the valleys). Their skies are CC0 photographs from Poly
Haven; what lies below the horizon is drawn by `assets/own/make/make_skies.py`, seen as from high above, so the
track hangs far over it (`assets/own/CREDITS.md`). Any other name is a set of files `Textures/Sky/Sky_<name>_<face>`
(Front, Right, Back, Left, Down, and Top if it has one) a level folder or the textures bring; a name is letters,
digits, `_` and `-`. `--sky NAME` on the command line plays any level under another sky.

**The sea of cloud.** Under `Sunset` and `Clouds` our levels hang over a sea of cumulus that the game draws as a
volume (`crates/game/src/clouds.wgsl`): billows of tiling noise, lit by the sky's sun (bright where it meets them,
blue-violet in their shade) and by the sky above, fading far off into the sky's own haze at the horizon; the columns
and supports sink into it. Under `Mountains`, `Forest`, `Mist` and `Desert` it is broken banks with gaps through
which the land shows far below. The level kit writes it into the level's settings (`clouds`,
[open-format.md](open-format.md)) from the sky's light (`assets/own/make/sky_light.json`, written by
`make_skies.py`) and its table (`levelkit.py`, `CLOUD_SEA`); a level changes it with
`clouds cover C below B thickness T size S` (any of them: how much of it is cloud, 0..1; its tops B under the lowest
floor, 35 if not given; T deep, 110; a cloud S across, 85) or has none with `clouds off`. The original's levels have
none. The settings file's `cloud-quality` (`low`, `medium` (the default), `high`; `off` draws none) sets how finely
it is drawn, and *Clouds?* off hides it as well ([menu.md](menu.md)). On an RTX 3060 at 2560×1440 it costs about
3.5, 4.5 and 6 ms a frame (low, medium, high).

**How the track keeps the ball on** (`safety`): the original's help shrinks from level to level, and ours follows
it. Measured on the original's thirteen levels (its floors' edges, numbers only): in levels 1, 3 and 5 about two
thirds of the floors' edges are profiled (the floor rising 0.7 over 2.5 to its edge, a gutter that rolls a drifting
ball back to the middle; a third bare), from level 6 on only a quarter to a third (the rest bare); raised walls
are rare (only the wooden half pipes of level 4 and a few pieces of 12 and 13); steel railings (a pipe about 2.6 over
the floor, on posts) run along about 490 and 420 units of the floors' edges in levels 1 and 2, 130 to 540 in levels 3
to 8, under 90 in 9 to 11 and none in 12. Our levels go the same way, a step clearer:
level 1 has gutters with railings where a fall is possible (round the start, along the outer side of turns, on the
landings, beside floors with a drop), as the original's first level, level 2 railings, 3 and 4 brick borders (3 a
sector of railings), 5 to 8 gutters (5 a sector of
borders, 8 its last bare), 9 and 10 bare floors (9 its first sector a gutter). `safety KIND` sets it from there on
(the first one in a file is the level's); `borders off` makes the floors bare from there on, `borders on` gives them
back the level's safety. What each is:

- `railings`: steel railings along both edges, rails at 1.3 and 2.6 over the floor on posts every 6 units, in the
  rails' physics group as the original's;
- `gutter`: the original's profiled floor, its edges 0.7 over a flat middle, rising over 2.5 in two facets;
- `border`: brick borders 1 wide and 2.2 high along both edges, the menu scene's bricks. Higher than the ball's
  middle (its radius is 2), so a ball meets the face square and turns back: a lower border (1.4) was ridden over by
  a ball thrown at it at 25 units a second;
- `none`: the bare floor, its edges marked by the brick strip.

Every border and railing is part of the floors' physics: the test `borders_and_railings_keep_the_ball_on_the_track`
(ownlevels.rs) throws each ball sideways at 12, 25 and 40 units a second into the railings and the borders of
Parts_Test and pushes it on into them, and it stays on; a ball drifting into a gutter rolls back; thrown off the bare
floor it falls.

**The track's look** is made, like the parts, to what makes the original's floors read (measured on its floors and
textures, numbers only): a texture spans 5 units of floor; slabs of 5 by 5 with joints between them, each slab its
own tone, dirt and wear, four along a texture (20 units before it repeats), one of three sets of slabs by the level;
a kerb along each edge with the original's strip of small bricks, its groove and its shade; the gutter's slopes darker
towards their crease; the shade a border casts beside it; the sides darker concrete with their own brick strip. The
textures are `assets/own/Track/` (make_textures.py, step `track`); the borders take the menu scene's brick. They are
mapped by the distance along the whole track, and pieces laid one after another (straight, curved, a width changing
over a piece) are one mesh, so the textures run on from piece to piece without a seam.

**No seams** (2026-11-03): a stretch of floor is one mesh from the start or a checkpoint to the next, its
cross-section carried upright across the cursor's own heading (exact on a turn), so a straight running into a turn or
a ramp into a flat has the very same cross-section where they meet: no wedge of nothing between them; each ring of
points is shared by the quads before and after it (no crack, no T-junction), and the textures are mapped by the
distance along the way. The start's and the checkpoints' pieces are joined the same way: their edges where a floor
runs on have their corners at the floor's own cross-section points, and their tops are mapped as the track's. The test
`our_floors_have_no_seams` (ownlevels.rs) checks the floors of every level the kit made (version 2 on, in level.gltf's
`asset.generator`): wherever a floor's top ends, there is no other top beside it at its height
(`the_seam_finder_sees_a_crack` checks the finder itself). A checkpoint on a narrow way has a smaller bowl, and its
depth with it, so its slopes stay the measured ones (a stone ball resting in a steeper bowl did not roll out of it).

**Room, rails and clouds** (2026-11-03, after the owner played level 1): a way must leave the ball (radius 2) at least
5.5 across at the height of its middle, between its borders or its railings' rails (levelkit.MIN_FREE); the kit
refuses to write a level that has a narrower place and lists every one (a way 6 wide with brick borders left 4, and
the ball wedged). A box or a loose ball must leave that much room beside it, with what lies across the way near it,
unless it is marked `shove` and the ball the way has there (the start's, then each transformer's) is no lighter (wood
1.9, stone 10, paper 0.2, a box 1); no loose ball lies within 14 of the end of rails (it rolls into their mouth).
Rails are the original's width, measured: tubes 0.6 thick, their middles 3.9 apart, so the ball sits deep between
them; ours lie on the floor they leave (the ball drops 0.48 onto them), and where a floor runs on from an end each
tube bends out flat over the floor's flat, a mouth seen from above (none on a gutter, which leads the ball to the
middle itself); posts under the tubes hold them, and ties across between them only with `rails ... ties`, for later
levels. A transformer stands just before what its ball is for. Every profile's flat top is cut at the same places
across (1, 1.25 and 2.5 in from the edges, the last at least 1.5 out from the middle, where a narrow gutter's foot
is), so stretches of another profile meeting end to end share their vertices; every face turns the way its normal says
(the tracks' faces were wound and their normals turned inwards before). The track's concrete is darker and greyer than
the original's sandy tone and its slabs' joints fainter (the owner's wish). Under every level lies a cloud layer as
the original's (`SkyLayer`, hidden in the file, shown by the game when the options have clouds, drawn added over the
sky): a plane 165 under the lowest floor, 500 past the level on every side, our own grey clouds on black repeating
every 675.

**Supports, turns and looks** (2026-11-03, the owner's third look): rails are held by a few saddles under both tubes
(below the ball riding between them) on columns down into the haze, about one every 40 along them, none near their
ends (the floors hold them there) nor over another floor; `railing turns` puts a railing on the outer side of every
turn from there on (level 1 has it), and a railing on a turn's inner side alone is refused. The cloud layer is laid
only under the skies that are a sea of cloud (A, B, C, Sunset); level 1 is under `Mountains`. The track's sides and
the columns are streaked by rain from their top edge, the columns cast in rings (`Track_Column`). A part's mesh can
carry a finer look (ownkit.py, `Mesh.look`: what the game draws, the mesh itself staying the physics'): the
transformers are drawn as the original's kind of pad (a chrome frame round a disc of the ball's material, coloured
studs on a riveted plate), the paper ball as a crumpled sheet, the balloons' ropes tied to the basket.

**Gates for the balls** (2026-11-03, measured in the sim on ramps 20 long): the wooden ball and the paper one climb a
ramp of 33 degrees from rest; the stone ball stops on 19 degrees from rest and on 24 with a run-up of 30, so a ramp
rising 10 over 20 (27 degrees) is a gate only wood (or paper) passes. Level 1 puts one right after each wood
transformer, and a stone transformer right before what needs the stone ball (a loose stone ball, the crate, a crate on
a bare stretch, the pushers); its long empty straights are cut and turned into bends. A dome must leave the room of a
way on both sides of it (the DSL refuses a level where it does not, at the width the floor has there), and a box or a
loose ball is checked at that width too. Columns under the floors and under rails keep clear of every floor below them
by its own width.

**Air and gates to find** (2026-11-03): a level in the open format may carry a fog in its settings (`fog`: `color`
towards the horizon, `color_down` straight down, `density` a unit of distance, and `below`/`depth`: under that height
things fade wholly into it within that depth). The renderer draws it on everything lit (the sky, sprites and the cloud
layer keep their colour); the physics never sees it, so replays are unchanged, and the original's levels have none.
Our kit sets it from the sky's own colours (the haze just under its horizon, the world below) and lets the columns and
the rails' supports reach 240 down into it, so they fade out instead of ending; every sky but Night gets the cloud
layer, as two decks of scattered banks over a world of its own. Not every switch of ball stands on the line: in level
1 the stone ball comes to a climb it cannot make, with the wood transformer to the side of the square before it, and
the wooden ball comes to a crate it cannot shift, with the stone transformer on the side of the floor below (a fast
wooden ball can shift it now and then); each sector that needs a ball still gives one.

**After the owner's concept art** (2026-11-03, his pictures as a reference only): rails of worn, brushed galvanised
steel with a light rust tint and its relief (`Track_Rail`, a normal map), a clamp round each tube every 9 along them
with its strap bent down under the ball's path to the other tube, and at the supports a thin steel rod from the strap
down into the haze (the fittings only drawn); the balls drawn smooth (their coarse meshes stay the physics'); the
start on a stone tower down into the haze; lanterns on wooden posts at the start and beside every checkpoint.

**Parts stand on flat floor** (2026-11-03): the kit refuses a level where a transformer's pad, a box or a dome does
not stand on a flat, level patch at its own foot (its footprint sampled: every floor top under it within 0.05 of its
foot, none missing) and names each; a gutter is widened, or the part moved, where one stands. A ball that changes
sheds its skin: its pieces are curved fragments of its own shell, its texture outside, a thin inside.

**The start and the checkpoints** are bowls, as the original's (measured, numbers only): the start a round platform
of radius 10 with a bowl of radius 7.5 and 1.25 deep in its middle (the ball comes to rest in it), four flame cups on
its rim; a checkpoint a piece of floor 10 long with a bowl of radius 5 sunk 1.26 into it (to a lip) and two flame
cups across the way, at ±7 to the sides, where the part's flames burn. The floors before and after a checkpoint end at
its piece. Their frames and reset points stand in the bowls (the reset points 3.65 and 3.33 over the bottom, as the
original's).

**The test player's knowledge of a level** is in the same file. From the way laid, leveldsl.py writes the test
player's hint files for three styles into the level's `pilot/` folder (autopilot.md, "Hint files"): `safe` (rolls
from point to point and stops, takes every checkpoint, no risks), `speed` (rolls through the points without stopping)
and `collector` (as safe, and fetches every extra point and life off the way). Gaps, seesaws and turning arms are
taken at speed in every style (the careful ones with a run-up); a place that needs more says so with `pilot STYLES :
STEPS` (the hint steps, their points as `ahead side up` about the cursor; e.g. a wait timed for a swing) and `nopilot
STYLES` / `autopilot STYLES` around a stretch the styles take on their own.

**To change a level**: edit its `level.lvl`, then

```
scripts/own-levels.sh assets/levels/03_Paper_Winds
```

builds it and lets the test player play it in each style; each run that finishes is kept as
`pilot/<style>.replay` (the runs are played in a data folder of their own under `target/`). The test
`crates/game/src/ownlevels.rs` plays every level's three runs with no copy of the original and fails when one does
not finish, so a level is never shipped without a run that proves it can be finished. `scripts/video.sh
assets/levels/03_Paper_Winds assets/levels/03_Paper_Winds/pilot/speed.replay out.mp4` renders a run as a video.
A level weighs 3.3 to 3.8 MB, 2.6 MB of it the textures it carries (the format keeps a level's
textures in its own folder).

## The original game: copied by default

The game copies what it needs from the player's own copy into `levels/original/` once: from an installed game
(`drahtseilakt --import <Ballance folder>`) or straight from the game's CD or its image
(`drahtseilakt --import Ballance.cue`, or `.bin`, `.iso`: the InstallShield 6 archive in `SETUP/`, read by
`crates/cdimage`). The copy is optional (decisions.md, "The game plays out of the box"): without one the game starts
with its own levels, and the main menu's **Import** opens a window that does the same: it lists what it finds on the
computer (installed games in the usual folders of Windows, Wine, GOG, Heroic and Lutris; discs in their drives; CD
images in the home, Desktop and Downloads folders), takes a folder or image dropped on it or a path typed, and starts
the game with the copy. The
copy is the player's own data on the player's machine; nothing is downloaded and nothing goes into the
repository. `--data DIR` still plays from any folder without copying.

The folder's README lists what to copy by hand: `base.cmo` and the folders `3D Entities`, `Textures` and `Sounds` of
an installed Ballance (the game reads nothing else), and, for level 13, the SuDu add-on installed into that copy as its readme says.

## Any folder can be moved

Each of the four can be pointed elsewhere in the settings (`levels.original`, `levels.community`,
`levels.remastered`, `levels.open`), for a big disk or a shared folder. The default folder then keeps only a
README saying where the levels are, so a player who looks there finds them.

## What comes with the game

The game is installed with some levels in `levels/open/` (our own) and converted community maps whose authors
want them shipped with the game. Each shipped level needs:

- its author's permission and a free licence, recorded next to it (`LICENSE` in its folder);
- nothing of the original game inside: no textures, sounds or meshes of Ballance. A converted map that uses the
  original's textures names them, and the game takes them from `levels/original/` (the player's copy), as the
  map did in the original;
- a replay that finishes it, checked like the original levels' replays.

The repository holds such levels only once their licence allows it; the original game's files are never in it.

## Every folder explains itself

On its first start the game writes a README into each folder, in the player's language: what belongs there,
where to get it and how levels are laid out, and for `remastered/` which program makes it (the toolbox). A README
an earlier version wrote, untouched, is brought up to date; one the player changed stays. The texts ship with the game (docs in every
language); the README's links are text, and the game never opens them.

## Runs and highscores by kind

Replays, best times and highscores are kept per kind, so a community map named like an original level never
counts as one: `replays/<level>/` for the original levels, `replays/maps/<kind>/<map>/` for the others (`community`, `open`, `remastered`), a level folder's
under `replays/levels/<kind>/<level folder>/<map>/`, so two folders' `Level_07.NMO` keep their runs apart and a
loose `Foo.nmo` and a folder `Foo/` never share. Runs an earlier version kept in `replays/maps/<level folder>/` are
moved there the first time the level is played (only through real folders, never over runs already there).

## Languages

The community is largest in China (Baidu Tieba, the Chinese Ballance Forum, Bilibili; community-maps.md), so the
docs and the folders' READMEs are also in Simplified Chinese, next to English, Russian and German.

## Order of work

1. Done (2026-11-02): this page and the decision; Chinese docs.
2. Done: the `levels/` folders with their READMEs; the settings that move them, with the README left behind.
3. Done: `--import` from an installed game or from the CD image (the game also finds the imported copy
   without `--data`). The European CD's 318 game files come out identical to an installed copy.
4. The custom levels menu reading `community/` and `open/`; `remastered/` is made by `drahtseilakt-toolbox remaster`:
   the converter (phase 6), rounder shapes and the textures N times larger; each level's parts (the balls,
   the modules, the transformer) are remade the same way into its own `game/` folder with its sky and sounds, so it
   plays without the copy's level files (open-format.md) (Real-ESRGAN when installed, else Lanczos; phase 7). An original level
   the copy lacks (a copy without level 13) is played in the levels menu from its export in `remastered/`.
5. Levels that come with the game, as authors offer them.
