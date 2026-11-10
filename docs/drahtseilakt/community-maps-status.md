# Community maps: where they stand

A survey of the 579 community maps in the owner's `levels/community/` (2026-11-02; the maps of
[CustomMapList](community-maps.md): Custom Maps 408, Pro Racing 97, Reverse Maps 35, 45-Degree Rotated Maps 13,
Invisible Maps 13, Original Maps 13). Each was loaded and played for 300 ticks without a window, exported to the
open format and loaded again, and its Virtools scripts listed (building block by name). It is the plan for the
script interpreter and for mods. **2026-11-09:** the interpreter's phase 1 is in ([scripts.md](scripts.md)): its core, the
template's blocks and the most used others; 173 of the 184 scripted maps (584 maps counted with `community-unsorted/`) have
every block known, 8 of the 36 with scripts of their own run with every block's full effect.

| | maps |
|---|---|
| load and play by the game's native rules | 579 |
| have the standard groups (start, finish, checkpoints, reset points) | 579 |
| export to the open format | 579 |
| load again from the open format | 578 |
| carry Virtools scripts of their own | 188 |

**The one that does not load again:** `Custom Maps/Beautiful.NMO` has an object whose matrix holds a number that is
not finite; the game plays the `.nmo`, but the open format refuses a matrix that is not finite. To do: the export
writes such an object as its game uses it (or leaves it out), with a warning.

**Scripts.** 188 maps carry their own behaviour graphs, all of them Custom Maps but 4 Reverse Maps. 142 of them carry
the same nine blocks (Activate Script, Binary Switch, Change Texture Video Format, Deactivate Script, Get Cell,
Object Delete, Op, Point Particle System, TT Scaleable Proximity), 7 more the same with Wait Message: the scripts
the map makers' template brings for the flames' particles, which the game draws natively. Whether these maps lose
anything without them is to be checked by playing them; the native rules already take every map through start,
checkpoints and finish. The other 39 maps use their own scripts: 107 blocks in all, most of them in one or two
maps.

**Blocks by how many maps use them** (the interpreter's order): Op 184, Get Cell 183, Object Delete 183, Activate Script 182, Binary Switch 182, Deactivate Script 182, Point Particle System 182, Change Texture Video Format 180, TT Scaleable Proximity 179, Wait Message 35, Group Iterator 22, Hide 20, Physicalize 19, Box Box Intersection 19, Set Position 18, Iterator 17, Show 16, Switch On Parameter 16, Add Row 15, Delayer 15, PlanarParticleSystem 14, Get Key Row 13, Object Copy 13, Send Message To Group 12, Test 11, Text Display 11, Set Cell 9, Wave Player 9, Nop 8, Restore IC 8; then 77 blocks in
fewer than 8 maps each (among them physics forces and impulses, text and font, keys and mouse, timers and counters,
lights, textures and sounds).

**What this means.** The native rules play every map; the interpreter for the template's nine blocks and the next
twenty would cover what the scripted maps add (moving and hidden objects, physics changes, copies, messages, text,
sounds). Maps are never run half-way: a map that needs a block not implemented is reported with its name.

## The audit: what a map asks for and what it gets (2026-11-10)

`drahtseilakt --audit FILE|DIR [--audit-csv FILE]` goes through a map or a folder of maps without a window, with
the player's copy and without it, and lists for each map: every body the physics makes of a level object that is
not drawn (an invisible blocker, with its name, place and the reason), the stand-ins no module is made for, the
modules and textures nobody has, the objects of classes the loader leaves out, the alpha tests the renderer does
not follow and the script blocks not known. The CSV has one row per map, copy and kind of issue with the names; the
summary ranks the kinds by the number of maps they hit. `--show-colliders` draws the floors and rails a map hides
(red) and its hidden stoppers (yellow) as translucent shapes in the game and in screenshots.

The first run over 605 maps (the 26 shipped and the 579 of the collection) found the causes of "invisible
objects", and three rules were changed for every map:

| cause | maps | now |
|---|---|---|
| a stand-in was known by its name (`P_Modul_26_01`) and only when it was a visible mesh without a material; maps made in Blender name it `P_Modul_26_01_.001`, give it materials or hide it, so no module was made and the stand-in stayed as an unseen or plain white solid | 71 + 29 + 22 | a member of a module's group is a stand-in, as the original's level init takes it |
| a floor, rail or stopper without a material collided but was not drawn | 42 | drawn plain |
| a group that is no module of the game (a mod's `P_Modul_Portal`) made its members solid | 13 | never a body; drawn as the object it is |

What is left, by the maps it hits (with the copy): floors and rails hidden in the file that collide, 204 maps, and
hidden stoppers, 160 (the maps' own invisible colliders, as in the original; `--show-colliders` shows them);
objects of classes not read, 58 (2D entities 34, curves 16, sounds 9, patch meshes 2); textures that name
no file and carry no pixels, 35; meshes with faces without a material, 30 (drawn grey); modules of mods, 17; script
blocks not known, 11; fully transparent floors, 6.

### Checked against the original: hidden colliders and object animations (2026-11-10)

**Hidden floors and rails stay solid.** The original's level init physicalizes every member of `Phys_Floors` and
`Phys_FloorRails`, and Physicalize does not look at whether the object is shown (the reference's port of the
block, `src/bb/bb_physics.c`). The original's own levels rely on it: 7 of its 12 level files keep 19 hidden
members there (`A02_inv_Coll_00`..`04` in level 6, `A07_Floor_Invisible_Col` in level 11, `A7_invisibleCollCube` in
level 12). No rule changed. In Go Ahead the hidden walls stand in pairs along the track on both sides of a
transformer (`Box04` / `Box05`): a corridor that keeps the ball from going round it, not a wall across the way; the
map has no scripts, so nothing takes them away.

**Object animations: nothing plays them.** 88 maps hold 9871 animation objects (classes 15 and 18). None of the 605
maps has a block that plays one (Play Animation 3D Entity, Play Global Animation) or a parameter that holds one,
and Virtools plays no animation by itself. 9287 of them are an exporter's bake: 35 keys each of position, rotation,
scale and scale axis, every key equal to the first. 356 carry no keys of their own, 8 are the arms of the game's
balloon. Nothing moves in the original, so the game moves nothing either. The audit counts them apart
(`animation-unplayed`), names one whose keys would move, and reports `animation-played` for a map that has
something to play one with. `nmo-dump --anims FILE...` prints each animation with its object, its controllers and
how far its keys go. What does move in community maps is moved by their scripts (Set Position, Rotate, Physicalize,
Set Physics Slider; about twenty maps), which is the script runtime's next phase ([scripts.md](scripts.md)).
