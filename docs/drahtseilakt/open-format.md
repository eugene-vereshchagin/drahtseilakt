# The open level format (version 1)

The engine's own level format: open, documented, editable with ordinary tools. Every level the game can load
— the original levels of the player's copy, community maps — converts into it on the player's machine
(`drahtseilakt --level N --export DIR`, `--map FILE --export DIR`), and the game plays it
(`drahtseilakt --open DIR`). Converted levels are made from the player's own data and are not distributed.

## Files

A folder holding:

- `level.gltf` — [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) (JSON);
- `level.bin` — its binary buffer (the only buffer, referenced by that name);
- `textures/NNNN_name.png` — one image per texture.

## Contents

- **Nodes**: one per object of the level, in order, with its name, its world `matrix` and, if it is drawn,
  its `mesh`. `extras.drahtseilakt.visible` tells whether the object is shown.
- **Meshes**: one per mesh with geometry; one primitive per material, all primitives of a mesh sharing the
  same `POSITION`, `NORMAL` and `TEXCOORD_0` accessors (floats), `indices` as 32-bit integers.
- **Materials**: `baseColorFactor` (diffuse), `baseColorTexture`, `emissiveFactor`, `doubleSided`; `alphaMode`
  `OPAQUE`, `MASK` (with `alphaCutoff`) or `BLEND`; unlit materials use `KHR_materials_unlit`.
  `extras.drahtseilakt` holds what glTF has no field for: `ambient`, `z_write`, `blend` (source and
  destination factor names: `zero`, `one`, `src_color`, `inv_src_color`, `src_alpha`, `inv_src_alpha`,
  `dest_alpha`, `inv_dest_alpha`, `dest_color`, `inv_dest_color`) and `alpha_cutoff` when blending and alpha
  test are combined.
- **Scene extras** `extras.drahtseilakt`: `format` (1), `groups` (name and member node indices — the game
  rules read these, see [nmo-format.md](nmo-format.md)), `sky` (sky letter) and `start_ball`.

## Coordinates

glTF is right-handed, Virtools left-handed, both with Y up: Z is negated (positions, normals and matrices,
whose Z row and column change sign) and each triangle's winding is reversed. Negation is exact, so a level
converted and read back is the same level bit for bit: level 1 and its open copy give identical
trajectories for the same input and identical frames.

## Reading

Shared levels come from strangers, so the reader treats a folder as hostile: `level.gltf` up to 64 MiB,
`level.bin` up to 1 GiB, every accessor checked against its view and the buffer, indices against the vertex
count, image paths confined to the folder, and anything outside the subset above refused with an error.
