# Texture packs and model packs

What the levels look like comes from two kinds of **packs**, chosen separately, as sounds come from
[sound themes](sound-themes.md):

- a **texture pack** gives the pictures the levels, their parts and the balls are drawn with;
- a **model pack** gives the look of the parts and the balls: the meshes drawn for a box, a fan, the wooden ball.

A model pack changes only what is **drawn**. The physics always uses the shapes the game measured (the original's for
the copy's levels, ours for our own levels: [resources.md](resources.md), "Our own parts"), so a pack never changes how
a level plays, and every replay plays the same with any pack. This page is for pack authors and players.

## Choosing a pack

In the extended mode, Options → Extended has the lines **Textures** and **Models**; the arrows go round every pack
found, each shown by its name in the menu's language. The choice applies from the next level loaded and is kept in the
settings (`textures KEY`, `models KEY` in `settings.txt`). The classic mode always draws with our textures and the
parts each level takes. For one run: `--textures KEY` and `--models KEY`.

| Kind | Built-in packs | The default |
|---|---|---|
| Textures | `own` (ours, free), `original` (the player's copy), `remastered` (a remastered level's own upscaled pictures, else the copy's) | `own` |
| Models | `level` (each level with the parts it takes), `own` (our parts' looks), `original` (the copy's parts' looks) | `level` |

## Where packs live

In the data folder, beside `sounds/`: `textures/<pack>/` and `models/<pack>/` (`~/.local/share/drahtseilakt/` on Linux,
`%APPDATA%\drahtseilakt\` on Windows, `~/Library/Application Support/drahtseilakt/` on macOS). To install a pack, copy
its folder there; the game finds the packs when it starts, so restart it after adding one.

The folder's name is the pack's key (what the settings save): letters, digits, `_`, `-` and `.`, not beginning with a
dot, at most 64 characters. A folder named like a built-in pack (`own`, `original`, `remastered`, `level`) or `none` is
ignored. At most 32 packs of each kind are taken (the built-in ones count).

## pack.txt

The same shape as a sound theme's `theme.txt`: plain text, UTF-8, at most 4 KiB, one `key value` per line, `#` begins a
comment.

| Key | Meaning |
|---|---|
| `name.en`, `name.ru`, `name.de`, `name.zh` | The pack's name in the menu, per language (at most 64 characters); without one the English name, else the folder's name. |
| `author`, `licence` | Who made it and under which licence (for people). |
| `fallback` | The pack what this one lacks comes from: a pack's key, or `none`. Without it: textures `own`, models `level`. |

The order a texture or a part is looked for in: the chosen pack, its fallback, that pack's fallback (at most eight, each
once), then the default (textures: ours, then the level's own picture; models: the level's own part). So a pack only
replaces what it has.

## Texture packs

```
textures/
  stone/
    pack.txt
    CREDITS.md             who made each picture, under which licence (recommended)
    Floor_Top_Border.png   a picture for the original's texture Floor_Top_Border
    Wood_Plain.jpg
    Sky/Sky_A_Front.png    the sky's faces (also accepted at the top)
    Particles/Particle_Flames.png
```

- A picture is named by the **texture's name**: the original's texture names (`assets/own/TEXTURES.md` lists every one
  the levels draw with, what it is and its size), the track's (`Track_Side`, `Track_floor_border`, ...) and any other
  texture a level names. Names are compared without case, `_`, `-` and spaces, and without the extension:
  `ball_wood.png` is `Ball_Wood.bmp`.
- PNG, JPEG, BMP or TGA; at most 32 MB a file and 8192 pixels a side; at most 4096 pictures a pack. Alpha counts
  (the original's "transparent colour" is not applied to a pack's picture).
- Only plain files directly in the pack's folder, `Sky/` and `Particles/` are read; links are never followed. A picture
  that cannot be read or decoded is passed over for the next pack. A level's pictures from a level folder and from
  packs share one budget of 256 million pixels.

Which textures a pack replaces (the same rule as our own textures, [resources.md](resources.md), "Own textures"):
every texture of the copy's levels, parts and balls and of their remasters; in a stranger's map only the copy's own
textures it uses (never a picture the map brings); in an open-format level only the textures it names as the game's
(`game:` images, [open-format.md](open-format.md)), never the files of its own folder. A level's own `game/Textures/`
picture of a name comes before any pack. The sky, particles and the ball's shadow are taken from a player's pack when it
has them, before the built-in packs.

### An example: a texture pack

1. Make the folder `textures/myfloors/` in the data folder.
2. Write `pack.txt`:
   ```
   name.en My floors
   name.ru Мои полы
   author Jane Doe
   licence CC0 1.0
   fallback own
   ```
3. Draw `Floor_Top_Border.png` and `Floor_Side.png` (the original's are 256 × 256 and tile; ours, in
   `assets/own/Textures/`, are a good start: copy one and paint over it).
4. Start the game, Options → Extended → Textures: *My floors*, and load a level. Every floor with a border is now yours;
   everything else is ours (the fallback).

## Model packs

```
models/
  round/
    pack.txt
    PH/P_Box/level.gltf     the box's look, a part in the open format
    PH/P_Box/level.bin
    PH/P_Box/textures/...   its own pictures (kept as they are by any texture pack)
    Balls/level.gltf        the balls, their pieces and shadows
    AnimTrafo/level.gltf    the transformer
```

- Laid out as the game's own parts (`assets/own/3D Entities/`): a part is a folder in the
  [open level format](open-format.md), named as the original's file without `.nmo` (`PH/P_Modul_18/`, `Balls/`).
  The easiest start is to copy the part from `assets/own/3D Entities/` and change its meshes.
- A part's mesh is drawn instead of the mesh of the **object of the same name** in the part the game plays with (the
  names the game's code uses: `P_Box_MF`, `Ball_Wood`, the pieces `Ball_Wood_Piece_01`...), moved into that object's
  place, else instead of the mesh of the same name. A mesh of the game's part that the pack does not match is drawn as
  it is. A mesh several objects share takes a look only when all of them match the same mesh in the same place.
- The pack's materials and textures are its own: its `textures/` files are never replaced by a texture pack; its
  `game:` images follow the texture pack.
- Checked as untrusted (packs are shared): only real folders and files, `level.gltf` at most 8 MB and `level.bin` at most
  64 MB, read by the open format's reader (every accessor checked), at most 1,048,576 vertices and 2,097,152 triangles a
  part, every number finite. A part that fails any check is left out (said once on the terminal) and the next pack in
  the chain gives it; the game never fails for a pack.
- Only the look: collision shapes, masses, joints and every other value stay the game's.

### An example: a model pack

1. Copy `assets/own/3D Entities/PH/P_Box/` (from the game's folder) to `models/bigbox/PH/P_Box/` in the data folder.
2. Write `models/bigbox/pack.txt` with `name.en Big box`.
3. Edit the box's mesh (Blender imports and exports glTF; keep the object names, export as glTF Separate with the
   textures in `textures/`, then put back the scene's `extras.drahtseilakt` as the original file has it).
4. Options → Extended → Models: *Big box*. The box is drawn as yours; it still rolls, falls and collides as the game's.

## Licensing and sharing

- Use your own pictures and meshes, or works under CC0 or CC-BY; list them in `CREDITS.md`.
- **Never include the original game's textures or models** (or any other game's), not even edited. The built-in
  `original` packs take them from each player's own copy, on their machine.

## Reference

- Code: `crates/gamedata/src/packs.rs` (finding and choosing packs, pack.txt), `crates/gamedata/src/assets.rs`
  (`put_pack_textures`, `game_texture`, `put_pack_looks`, `load_pack_part`).
- Fuzzed: `pack_txt` ([fuzzing.md](fuzzing.md)); a model pack's part goes through the open format's reader
  (`open_level`).
