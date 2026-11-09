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
