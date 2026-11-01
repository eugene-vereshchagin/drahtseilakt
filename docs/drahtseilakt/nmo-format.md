# The Virtools file format (.nmo / .cmo)

How `crates/nmo` reads Ballance's levels and the community maps. Worked out from the files themselves — the
game's own files and about 600 community maps, all of which read completely — with the identifier constants
of the Virtools SDK headers. All numbers are little-endian; "word" means 32 bits.

## Container

| Offset | Size | Field |
|---|---|---|
| 0 | 8 | signature `Nemo Fi\0` |
| 8 | 4 | checksum |
| 12 | 4 | Virtools version (`0x13022002` for Ballance's Virtools 2.1, `0x26052005` for 3.5) |
| 16 | 4 | file version (8 in every file seen; header 1 exists from 5) |
| 20 | 4 | zero |
| 24 | 4 | write mode (bit `8`: whole file compressed) |
| 28 | 4 | header 1, stored size |
| 32 | 4 | data, stored size |
| 36 | 4 | data, unpacked size |
| 40 | 4 | manager count |
| 44 | 4 | object count |
| 48 | 4 | highest object id |
| 52 | 4 | product version |
| 56 | 4 | product build |
| 60 | 4 | header 1, unpacked size |

Then header 1, then the data. A part is zlib-compressed exactly when its stored and unpacked sizes differ.
After the data, files written by Virtools 3.5 append **included files** until the end: a name (word length
+ bytes) and contents (word size + bytes); usually only `vtthumbnail.jpg`, some maps also carry sounds and
pictures.

**Checksum**: Adler-32 started from 0 instead of 1, over the 64-byte header with the checksum field zeroed,
then over the stored header 1 and data (not the included files). It matches in every file seen; the reader
reports a mismatch instead of refusing, since whether the original game checks it is unknown.

## Header 1: the object table

Per object: id, class id, index into the data, name length, name bytes (in the writer's code page — often
GBK in Chinese maps). Plugin dependencies follow and are not needed.

## Data: managers and objects

First one record per manager (GUID: 2 words, size word, chunk), then one per object in table order (size
word, chunk; size 0 means no chunk).

**State chunk**: a version word (data version, class id, chunk version and option bits, one byte each), a word
count, the words, then optional lists selected by option bits (`1` object ids, `4` sub-chunks, `2`
managers; each a word count and the words). Option `8` means object references are indices into the
object table; `0xFFFFFFFF` is a null reference.

Inside the words, fields are grouped under **identifiers** forming a linked list: `[identifier, index of
the next identifier (0 at the end), fields...]`. A string is a byte length and the bytes padded to whole
words; writers may leave garbage after the terminating NUL.

## Classes a level is made of

| Class | Id | Identifier | Fields |
|---|---|---|---|
| any object | | `0x4` / `0x18` | present: hidden / hierarchically hidden |
| Group | 23 | `0xFFFFF` | count, member references |
| Material | 30 | `0x1000` | diffuse, ambient, specular, emissive (ARGB words), specular power (float), texture reference, border colour, modes word, flags word |
| | | `0x10000` | effect: parameter reference, effect (1 TexGen, 2 TexGen with a referential); the parameter's value is the TexGen type (CK2_3D.dll: None 0, Transform 1, Reflect 2, Chrome 3, Planar 4, cube maps 31..34) |
| Parameter | 46 | `0x40` | type GUID (2 words), a word, size in bytes, the value |
| Texture | 31 | `0x10000` | slot count, file name of each slot |
| | | `0x20000` | embedded pixels: slot count, then bits per pixel, width, height, masks A R G B, a word, pixel count, pixels; `[1, 0]` when the pixels live in a file |
| | | `0x2FF000` | load options: a word (`0x100` a transparent colour, `0x200` a video format follows), then by the section's size the transparent colour (0xRRGGBB), the current slot, the video format; texels of that colour are dropped by an alpha test |
| Mesh | 32 | `0x100000` | material slots: count, then (reference, word) per slot |
| | | `0x20000` | vertex count, save flags, size (data words + 1), then positions, colours, speculars, normals, UVs as the flags say |
| | | `0x10000` | face count, then two words per face: indices 0 and 1 (16 bits each), index 2 and the material slot |
| | | `0x4000` | material channels (the mesh drawn again with another material, blended over itself): count, then per channel material reference, flags (`0x1` active), source and destination blend, UV count (0 or the vertex count) and the UV pairs |
| 3D entity / object, camera, light | 33, 41, 34, 35, 38, 39 | `0x100000` | a word, flags, 3×3 axes (rows), position — the world matrix |
| 3D object | 41 | `0x4000` | current mesh reference, count, mesh references |
| Sprite 3D | 37 | `0x400000` | mode (0 faces the view, 1 turns about x, 2 about y, 3 as placed), half width, half height, offset x and y (-1..1 of the half size), texture rectangle u0 v0 u1 v1, material reference; drawn unlit in the material's diffuse colour |
| Camera, target camera | 34, 35 | `0xfc00000` | (data version 5 and later) projection, field of view (horizontal, radians), orthographic zoom, aspect (width, height as 16-bit halves), near and far planes |
| Target camera | 35 | `0x10000000` | its target object |
| Curve | 43 | `0xffc00000` | (data version 5 and later) control points: count and references in curve order, fitting coefficient, step count, open flag |
| Curve point | 36 | `0x10000000` | (data version 5 and later) the curve, user tangents (instead of TCB), linear, tension, continuity, bias, in and out tangents (curve-local) |
| Data array | 52 | `0x1000` | columns: count, then name (string) and type (1 int, 2 float, 3 string, 4 object, 5 parameter + type GUID) |
| | | `0x2000` | rows: count, then one value per column |

**Vertex save flags**: `0x1` one colour for all, `0x2` one specular for all, `0x4` no normals (the reader
computes smooth ones), `0x8` one UV for all, `0x10` no positions.

**Material modes word**, nibbles from the lowest: texture blend mode, min filter, mag filter, source blend,
destination blend (Virtools blend factors: 1 zero, 2 one, 3 src colour, 4 inv src colour, 5 src alpha, 6 inv
src alpha, …), shade mode, fill mode. **Flags word**: bit `0x1` two-sided, `0x2` z-write, `0x8` alpha
blending, `0x10` alpha test; byte 1 the depth comparison, byte 2 the alpha comparison, byte 3 the alpha
reference.

## What the game reads from levels

A level holds geometry and **groups**; the game's logic lives in `base.cmo` and works on group names:
`PS_Levelstart` (start; its Z axis points back at the camera), `PR_Resetpoints` (ball centres per sector),
`PC_Checkpoints`, `PE_Levelende`, `Sector_NN`, `Phys_Floors` / `Phys_FloorRails` / `Phys_FloorStopper`
(collision), `P_*` (modules and items, whose models come from `3D Entities/PH/*.nmo`), sound groups.
Objects in those groups whose mesh has no material are editor placeholders and are never drawn. The sky and
starting ball of each level are in the data array `AllLevel` of `3D Entities/Levelinit.nmo`.
