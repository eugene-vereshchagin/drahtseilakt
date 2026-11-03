# The open level format (version 1)

The engine's own level format: open, documented, editable with ordinary tools. Every level the game can load
— the original levels of the player's copy, community maps — converts into it on the player's machine
(`drahtseilakt --level N --export DIR`, `--map FILE --export DIR`), and the game plays it
(`drahtseilakt --open DIR`). Converted levels are made from the player's own data and are not distributed. With
`--textures own` an export carries our own free textures (resources.md, "Own textures") instead of the copy's
wherever we have one, the way to share a level's look with free assets only.

## Files

A folder holding:

- `level.gltf` — [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) (JSON);
- `level.bin` — its binary buffer (the only buffer, referenced by that name);
- `textures/NNNN_name.png` — one image per texture; an image's `uri` must name a plain file directly in `textures/`
  (no further folders), else the level is refused;
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

## Contents

- **Nodes**: one per object of the level, in order, with its name, its world `matrix` and, if it is drawn,
  its `mesh`. `extras.drahtseilakt.visible` tells whether the object is shown.
- **Meshes**: one per mesh with geometry; one primitive per material, all primitives of a mesh sharing the
  same `POSITION`, `NORMAL` and `TEXCOORD_0` accessors (floats), `indices` as 32-bit integers. A mesh's
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
- **Scene extras** `extras.drahtseilakt`: `format` (1), `groups` (name and member node indices — the game
  rules read these, see [nmo-format.md](nmo-format.md)), `sky` (sky letter), `start_ball` and `settings`: what the
  original takes from its level table and its rules by level number, so the level plays the same without the game's
  files — `music` (theme 1 to 1000), `light` (three numbers 0..1), `level_bonus` (points), `tutorial` and
  `last_level` (true or false: the original's first and last level), and `source_hash` (16 hex digits: the FNV-1a of
  the `.nmo` it was converted from), and `parts` (`"own"`: the level is made for the game's own parts and takes them
  before the copy's; `"copy"` or none: the copy's first; our level kit writes `"own"`), and `fog` (only drawn: `color` and `color_down` three numbers 0..1 each, `density` 0..1 a unit of distance, `below` a height, `depth` a depth over 0: our levels have it, the original's none; levels.md). A replay recorded on that file plays on the converted level (the same level, so
  the same run); malformed settings refuse the level. Any level can name any hash, so a replay that plays on it is
  no proof of a run on the original. A level without settings (made before they were kept) plays with the defaults
  and says so; a remastered `Level_NN` takes them from the copy's table instead. The game files of `game/` stand in
  only for the level's own parts, sky, particles, shadow and a run's sounds: the interface's files (menus, font, the
  level table, menu sounds) never come from a level: they are the interface's (ours, or the copy's), and every file read is capped.

## Coordinates

glTF is right-handed, Virtools left-handed, both with Y up: Z is negated (positions, normals and matrices,
whose Z row and column change sign) and each triangle's winding is reversed. Negation is exact, so a level
converted and read back is the same level bit for bit: level 1 and its open copy give identical
trajectories for the same input and identical frames.

## Reading

Shared levels come from strangers, so the reader treats a folder as hostile: `level.gltf` up to 64 MiB,
`level.bin` up to 1 GiB, every accessor checked against its view and the buffer, indices against the vertex
count, image paths confined to the folder, and anything outside the subset above refused with an error.
