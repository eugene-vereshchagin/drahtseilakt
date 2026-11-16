# The open level format (version 1)

The engine's own level format: open, documented, editable with ordinary tools. Every level the game can load
— the original levels of the player's copy, community maps — converts into it on the player's machine
(`drahtseilakt --level N --export DIR`, `--map FILE --export DIR`), and the game plays it
(`drahtseilakt --open DIR`). Converted levels are made from the player's own data and are not distributed. With
`--textures own` an export carries our own free textures (resources.md, "Own textures") instead of the copy's
wherever we have one, the way to share a level's look with free assets only. Textures with the same picture are
written once and shared by the materials; textures not found are listed by name in the summary. `--textures own`
also writes no `game/` folder (nothing of the copy: the level uses the game's own parts and sounds);
`--export-parts keep|none` chooses it explicitly.

## Files

A folder holding:

- `level.gltf` — [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) (JSON);
- `level.bin` — its binary buffer (the only buffer, referenced by that name);
- `textures/NNNN_name.png` — one image per texture; an image's `uri` must name a plain file directly in `textures/`
  (no further folders), or be `game:<name>`, else the level is refused. `game:<name>` names **the game's texture of
  that name** instead of a file: no file is read for it; the game draws it with the chosen texture pack
  ([packs.md](packs.md)): the level's own `game/Textures/` picture of that name first, then the pack and its fallbacks,
  then ours, then the copy's (names compared as texture names are: without case, `_`, `-`, spaces or extension). The
  name is 1 to 128 characters, none of them `/`, `\`, `:` or a control character, not beginning with a dot; it is only
  compared with the textures' names, never made a path. Our levels name the textures they share this way (the track's
  sides, the borders' bricks, wood, rails, posts, checkpoint plates), so a texture pack changes them too, and keep their
  own special textures as files. `--export` keeps a `game:` image as it is. Levels made before keep working: their
  files are theirs;
- `level.txt` (optional) — its title, author and version as `key = value` lines; a level in `levels/` is a folder
  holding one level, named by this title ([levels.md](levels.md), "A level is a folder");
- `game/` — the game's files the level uses, by their paths in the player's copy, so that it plays without the
  copy's level files: its parts in this same open format, each a folder named by its file without `.nmo`
  (`game/3D Entities/Balls/`, `game/3D Entities/AnimTrafo/`, `game/3D Entities/PH/P_Box/`...: the balls, the
  transformer and the module of every object named after one), and as they are its sky's faces
  (`game/Textures/Sky/`), the particles' textures (`game/Textures/Particles/`), the ball's shadow and the sounds a run
  plays (`game/Sounds/`, not the menu's nor other levels' music). The game looks there first, then, for a level made for our
  own parts (its settings say `"parts": "own"`), in our own parts (`assets/own/`, [resources.md](resources.md)) and
  then the copy, so it plays the same with or without a copy; for any other level in the copy and then in ours. Any part of a path in `game/` that is a link finds nothing.
  `--export` and the toolbox write it.

## Compressed textures

Beside a large picture `textures/NNNN_name.png` a level may carry `textures/NNNN_name.dtx`: the same picture ready for
the graphics card — block-compressed, with its whole mip chain made beforehand. The game then neither decodes the PNG
nor filters mips when the level loads, and the texture takes an eighth (BC1) or a quarter (BC7) of the video memory.
The toolbox's `remaster` writes them (pictures of 512 x 512 and more whose sides are multiples of four); the level
file does not name them. A level's parts in `game/` carry theirs the same way.

The file (little-endian), `crates/gamedata/src/gputex.rs`:

| bytes | what |
|---|---|
| 0–3 | `DSTX` |
| 4–5 | version, 1 |
| 6 | format: 1 = BC1 (opaque colour, 4 bits a pixel), 2 = BC7 (with alpha, or a normal map with its height; 8 bits a pixel) |
| 7 | mip levels: always the whole chain down to 1 x 1 |
| 8–11, 12–15 | width, height: multiples of 4, at most 4096 |
| 16–23, 24–31 | the picture file it was made from: its length and its hash |
| 32–39 | the hash of the data that follows |
| 40– | the mip levels' blocks, the largest first, nothing after them |

The hash is FNV-1a over 8-byte little-endian words (the tail padded with zeros), then the length. BC1 blocks are in
the four-colour mode (the first colour the larger) or of one colour; BC7 blocks are of mode 6. The textures are not
sRGB (the game lights in gamma space, as the original).

The file is a stranger's like the rest of a level: the header is checked against the caps before anything is read, the
length must be exactly what the header implies, the data must match its hash and hold only the blocks named above, and
a link is never followed. A file that names another picture's length than the PNG beside it, or fails its header, is
passed over and the PNG is used; one whose data is damaged, or changed between being looked at and read, is said and its PNG drawn instead, smaller (remaster the level again). A file reached through a link that leads out of the level's folder is refused. A
level may carry the `.dtx` without the PNG.

- **Adapters.** BC is on every desktop card under Vulkan and Direct3D 12 and on every Mac under Metal. Where the
  adapter lacks it (some phones' chips, some software adapters) the game decodes the blocks itself, from the mip level
  of at most 1024 a side. Checked on Linux (Vulkan, NVIDIA) both ways; Windows and macOS not run.
- **Memory.** A level's own game files (`game/`) share a budget of 1 GiB of texture memory; one texture takes at most
  an eighth of what is left of it — a larger one is drawn without its largest mip levels (a PNG is halved), never plain.
  One picture carried by several parts (the same data) is uploaded and counted once.
- **Loading.** Textures are read eight at a time, each on its own thread, and uploaded as they come. The game prints
  how many it uploaded, their video memory and the time.

## Contents

- **Nodes**: one per object of the level, in order, with its name, its world `matrix` and, if it is drawn,
  its `mesh`. `extras.drahtseilakt.visible` tells whether the object is shown; `extras.drahtseilakt.parent` the
  index of the node it follows when a script moves that one (Set Parent in a map; the matrix stays the world's, so it
  is no glTF child; one that is no other node refuses the level, a loop is broken). A matrix with an entry that is not
  finite (a map may have one, and the game plays it) is kept whole in `extras.drahtseilakt.matrix`, those entries
  written `"nan"`, `"inf"` or `"-inf"`; glTF's `matrix` then has 0 for them.
- **Reset points** (group `PR_Resetpoints`, nodes `PR_Resetpoint_NN`, one per sector, the first at the start):
  where the ball comes back in sector NN. The node's rotation also says which way the track runs: the camera looks
  along the point's **-x axis** (as in the original levels), snapped to the nearest quarter turn on the ground, after
  a respawn, so the view hints at the way on. Our level builder writes each point with its -x axis along the track.
- **Meshes**: one per mesh with vertices; one primitive per material, all primitives of a mesh sharing the
  same `POSITION`, `NORMAL` and `TEXCOORD_0` accessors (floats), `indices` as 32-bit integers. A mesh of vertices
  without faces (the points a map's particles come from) is one primitive of `mode` 0 (points) without `indices` or
  `material`. A primitive's `mode` is 4 (triangles) or 0 (points) or absent (triangles); any other value refuses the level. A mesh's
  material channels (passes over the whole mesh with another material) are `extras.drahtseilakt.channels`:
  each its `material`, `blend` (source and destination factor names, as below) and `texcoord` k, the
  `TEXCOORD_k` set holding its coordinates; at most 16. A mesh may carry a finer look drawn in its place
  (`extras.drahtseilakt.look`: the index of another mesh; the toolbox's rounder shapes): the looks come after all
  the level's own meshes, each the look of one mesh, and nodes never place them; the physics and the rules use the
  mesh itself.
- **Materials**: `baseColorFactor` (diffuse), `baseColorTexture`, `normalTexture` (a normal map, tangent space with
  green up as glTF has it: the relief the toolbox gives remastered levels; it must name one of the level's textures; with
  `extras.drahtseilakt.parallax`, a depth in texture coordinates over 0 and at most 0.25, its alpha is a height,
  1 the top, and the renderer draws it with parallax so the relief keeps its depth at a glancing view),
  `emissiveFactor`, `doubleSided`; `alphaMode`
  `OPAQUE`, `MASK` (with `alphaCutoff`) or `BLEND`; unlit materials use `KHR_materials_unlit`.
  `extras.drahtseilakt` holds what glTF has no field for: `ambient`, `specular` and `power` (the highlight), `z_write`, `blend` (source and
  destination factor names: `zero`, `one`, `src_color`, `inv_src_color`, `src_alpha`, `inv_src_alpha`,
  `dest_alpha`, `inv_dest_alpha`, `dest_color`, `inv_dest_color`), `alpha_cutoff` when blending and alpha
  test are combined, and `texgen` (`reflect` or `chrome`) when the texture coordinates come from the view.
  **One picture, one look (2026-11-16):** `original` says what the material stands for in the original — `{"texture": "Floor_Side"}`, optionally with `"texgen"` (the rails' `chrome`): with a texture pack other than ours (the copy's, the remaster's, a player's pack keyed by the original's names) that has the texture, the game draws the material with it, as it draws the parts and balls, without the material's normal map and with the given texgen; with ours, or a pack that lacks it, the material keeps its own picture. The name follows the rules of a `game:` name, else the level is refused. `own_only: true` declares that a picture stands for nothing of the original (the signs, a community part's textures, the sticky ball) and stays ours in every look; the game ignores it, the checks require one of the two on every textured material of our levels (levelkit.py `ORIGINAL_OF`, `mark_originals.py` for a level made before). `models` never changes textures: the `textures` choice alone decides every picture.
- **Scene extras** `extras.drahtseilakt`: `format` (1), `groups` (name and member node indices — the game
  rules read these, see [nmo-format.md](nmo-format.md)), `sky` (sky letter), `start_ball` and `settings`: what the
  original takes from its level table and its rules by level number, so the level plays the same without the game's
  files — `music` (theme 1 to 1000), `light` (three numbers 0..1), `level_bonus` (points), `sector_balls` (a list, sector 1 first, of `wood`, `stone`, `paper`, `sticky` or null: the ball a run begun at that sector with `--sector` has; none takes the level's start ball), `tutorial` and
  `last_level` (true or false: the original's first and last level), and `source_hash` (16 hex digits: the FNV-1a of
  the `.nmo` it was converted from), and `parts` (`"own"`: the level is made for the game's own parts and takes them
  before the copy's; `"copy"` or none: the copy's first; our level kit writes `"own"`), and `fog` (only drawn: `color` and `color_down` three numbers 0..1 each, `density` 0..1 a unit of distance, `below` a height, `depth` a depth over 0: our levels have it, the original's none; levels.md). A replay recorded on that file plays on the converted level (the same level, so
  the same run); malformed settings refuse the level. Any level can name any hash, so a replay that plays on it is
  no proof of a run on the original. A level without settings (made before they were kept) plays with the defaults
  and says so; a remastered `Level_NN` takes them from the copy's table instead. The game files of `game/` stand in
  only for the level's own parts, sky, particles, shadow and a run's sounds: the interface's files (menus, font, the
  level table, menu sounds) never come from a level: they are the interface's (ours, or the copy's), and every file read is capped.
- **The fog** `settings.fog` (our levels), only drawn: what is far from the camera fades into the air's colour,
  `color` seen towards the horizon and `color_down` seen straight down (three numbers 0..1; without `color_down` one
  colour all round), `density` of it a unit of distance (1 - e^(-density × distance), 0..1), and what lies below
  `below` fades into it as well, wholly `depth` under it. Lit surfaces only (the sky, sprites and the sea of cloud
  keep their own colour); none in the original's levels. `color`, `density`, `below` and `depth` are needed; values
  out of range are held to it, a malformed one refuses the level.
- **The sea of cloud** `settings.clouds` (our levels; levels.md, "The sea of cloud"), only drawn: `top` (the height
  of its tops; needed), `thickness`, `cover` (0..1), `scale` (a cloud's size), `sun` (the direction towards the sun),
  `sun_color`, `sky` (the light from above), `shadow` (in the shade; colours in linear light, 1 white), `haze` (of the
  horizon's haze a unit of distance), `under` (0..1: how much of the world under it the haze there hides) and `wind`
  (x, z a second). Only `top` is needed; values out of range are held to it, a malformed one refuses the level.

- **Texture slots**: an image's `extras.drahtseilakt.slots` lists the images that are its further slots, in order (a
  texture of several pictures, an animation's frames a map's script shows with Set Current Slot). Each is another image
  of the level; at most 63 each and 4096 in all; anything else refuses the level.
- **Frames** `extras.drahtseilakt.frames` (a converted map's 2D frames: an author's signature in a corner, a logo;
  only drawn): each its `name`, `rect` (left, top, right, bottom in pixels of a 640 x 480 screen), `uv` (the part of
  the texture shown), `z_order` (lower drawn first), `material` (an index or null) and `visible`. At most 64; a `name` is at most 12288 bytes (3 x the 4096 a `.nmo` name holds, as lossy UTF-8 may triple it); a
  malformed one refuses the level.
- **Scripts** `extras.drahtseilakt.scripts` (a converted community map's own behaviour graphs; `script_json.rs`): `objects` (name, class and what each is to the engine: an entity, group, mesh, material, texture or array by index), `nodes` (the blocks, links, IOs, parameters and operations by object index), `arrays` (the maps' tables), `scripts` (the top-level graphs with the object each is attached to), and, when the map has them, `groups` (the members it gives each group), `attributes`, `priorities` (object and priority: a higher one runs its scripts first), `start_off` (the scripts and objects the scene starts deactivated) and `waves` (its wave sounds: object, file, looped, gain, pan, pitch). A level written before the last three were kept reads without them. Data only, driven by our runtime ([scripts.md](scripts.md)); the back references are derived on reading, every index is checked, and a malformed entry refuses the level. A level without it has no scripts (the template's flames are drawn from the emitters' frames either way). Our level kit writes them too (`assets/own/make/scriptkit.py`: graphs built block by block, the level's objects, materials and textures named by name), as the showcase level's own graphs are ([levels.md](levels.md)).

## Coordinates

glTF is right-handed, Virtools left-handed, both with Y up: Z is negated (positions, normals and matrices,
whose Z row and column change sign) and each triangle's winding is reversed. Negation is exact, so a level
converted and read back is the same level bit for bit: level 1 and its open copy give identical
trajectories for the same input and identical frames.

## Reading

Shared levels come from strangers, so the reader treats a folder as hostile: `level.gltf` up to 64 MiB,
`level.bin` up to 1 GiB, every accessor checked against its view and the buffer, indices against the vertex
count, image paths confined to the folder, and anything outside the subset above refused with an error.
