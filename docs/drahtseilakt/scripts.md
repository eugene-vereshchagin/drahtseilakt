# Scripts: the maps' own behaviour graphs

Community maps may carry Virtools behaviour graphs of their own ("scripts"): the flames' particles of the
map makers' template, portals that teleport the ball, objects that hide, move or turn, texts and sounds. The
game plays every map by its native rules (start, checkpoints, finish, modules); the scripts add to that. This
page says how a graph lies in the `.nmo`, how our runtime drives it as **data** (nothing in a map runs as code:
[decisions.md](decisions.md), "Our own data-only mod format"), which building blocks it knows, and how far
that takes the maps. The per-map table is [scripts-maps.csv](scripts-maps.csv).

Phase 1 (2026-11-09): the interpreter core, the template's blocks and the most used of the others; proven on
the maps below, headless. Code: `crates/nmo/src/behavior.rs` (the file side), `crates/script` (graph model, runtime,
blocks), `crates/game/src/scripting.rs` (the engine's side). Format reference: [ck-runtime.md](../ck-runtime.md).

## What the file holds

A graph is a set of objects of a few classes, each with its state chunk (identifier → fields, as every object in
the file; references are indices into the object table):

| Class | Object | Fields read |
|---|---|---|
| 8 Behavior | a script (flag `0x2`), a graph (a box of blocks) or a building block (flag `0x8`, with its prototype GUID) | flags, prototype GUID and version, priority, the parameter holding the target (if targetable and set), then the lists: sub-behaviours, links, parameter operations, input / output / local parameters, inputs, outputs |
| 6 BehaviorLink | an arrow between two IOs | the delay in frames (saved twice: current and initial), source IO, destination IO |
| 9 BehaviorIO | an input or output of a behaviour | flags (1 input, 2 output); the owner is the behaviour that lists it |
| 2 ParameterIn | a block's input parameter (pin) | its type GUID and its source: a parameter it reads (direct) or another input it shares |
| 3 ParameterOut, 45 ParameterLocal | an output parameter (pout) or a local | type GUID, the saved value (a plain buffer: int, float, bool, string, vector, matrix, colour; an object reference; a message or attribute by name; a parameter type), and for outputs the destinations the value is copied to when it changes |
| 4 ParameterOperation | an operation inside a graph (`+`, `Get Position`...) | operation GUID, its two inputs and its output |
| 52 DataArray | a table the scripts read and fill (`PortalOutList`...) | column names and kinds, rows |
| any CKBeObject | the object a script is attached to | identifier `0x800`: its scripts |

Pin, output and local names are the parameter objects' names, so blocks are read by pin *name* with the index as
a fallback. Messages and attributes are stored by name. The `Op` block keeps its operation's GUID in its second
and third locals (the first is a function pointer of the writer's machine, meaningless here).

What a map cannot tell us: which scripts the scene starts active (that lives in the composition's scene, not in
a level file) — every script starts active and reset, as the game's loader activates a loaded level's scripts;
and the game's own objects a script names (`Ball_Pos_Frame`, the `CurrentLevel` table, the `All_Gameplay`
group), which the engine answers (below).

## The interpreter's model

`script::Graph` is the file's graphs as data: one node per file object (block, link, IO, parameter, operation),
the arrays, the scripts (behaviours no graph lists as a sub-behaviour) with the object each is attached to, and
for each IO the links that leave it. Each file object also gets its meaning to the engine (`ObjKind`: the level
entity, group, mesh, material or texture by index, an array, a behaviour, or nothing). The runtime adds the
game's pseudo-objects: `Ball_Pos_Frame` (the ball) and the tables `CurrentLevel`, `IngameParameter`, `Energy`.

`script::Runtime` holds the state — which behaviours and IOs are active, each link's countdown, each
parameter's value, the arrays' rows, a little scratch per block (an iterator's index, a delayer's time, a
proximity test's last answer), the messages in flight — and runs one tick as the original's behaviour manager
runs a frame ([ck-runtime.md](../ck-runtime.md), "Execution"):

1. Every pending activation request (Activate Script, Deactivate Script, Activate Object) of a script whose object
   is active in the scene is applied; then every active script of such an object is executed, in the manager's
   order: objects by priority (higher first, file order among equals), each object's scripts in its list's order.
2. A **building block** runs its prototype's code (`blocks/`, found in the registry): it reads and clears its active inputs, reads its
   pins (pulled: an input follows its source, and a value computed by an operation is computed first), acts
   through the `Host`, writes its outputs (copied on to their destinations) and activates its output IOs. It
   returns whether it stays active for the next tick.
3. A **graph** fires the links from its active IOs (a zero-delay link activates the destination now; a delayed
   link is queued), stacks its active sub-behaviours (the first on top), then pops and executes until the stack is
   empty, propagating each one's outputs: a zero-delay link activates the destination and puts its behaviour on
   the stack by priority; a delayed link is queued. At the end every queued link counts down one frame and fires
   at zero. The graph stays active while links are queued or sub-behaviours active. A loop is cut at 8000 pops.
4. Messages sent in a tick (Send Message To Group, Send Message) reach their waiters (Wait Message, matched by
   name, to the owner or to all) in the next tick; the engine is told too.

Scripts run once a tick before the physics (the original's order in a frame), in run ticks, from the same input
as everything else, so a run stays deterministic and a replay plays back the same. A replay recorded before
**format 8** runs without scripts, as it was recorded ([replay.rs](../../crates/game/src/replay.rs)). A replay of
a map with scripts in **format 9** is refused as stale: its scripts activated at once, so it would play otherwise
(format 10, stage 0). A format-9 run of a level without scripts plays as before. Levels
without scripts are untouched: there is no runtime at all.

**The engine's side** (`Host`, implemented in `scripting.rs`): matrices and visibility of level objects
(kept as overlays the renderer reads: `script_hidden`, `script_moved`), the ball's position (Set Position on
the ball puts a new ball there at rest, as the original re-physicalizes it), bounding boxes (the mesh's box
in the object's frame, transformed), groups' members, the game's tables (`CurrentLevel`: Level ID, ActiveBall,
Ball_Pos_Frame, CurrentResetpoint, `Activation Phase?` = the ball is in play, Points — the layout of
Gameplay.nmo), and a log of what the scripts did (sounds, texts, messages, copies, physics asked for),
printed by `--headless` as `script:` lines. Particles: the template's flame emitters are already drawn
natively from their frames ([modules](modules/)), so Point Particle System only reports its switch.

What phase 1 does **not** do: physics for scripted objects other than the ball (Physicalize, SetPhysicsForce,
Physics Impulse are accepted and logged), copies of objects (Object Copy returns nothing), texts and sounds
(logged; the HUD and the sound engine come next), keys other than the game's (Key Event never fires: a run's
input is the game's keys only), the original's 2D interface blocks (never: the game has its own).

## Phase 2 (2026-11-11): copies, "This", groups, progressions

- **"This"**: a script's own object is a local parameter saved without a value (class 45 with the empty section
  `0x200` alone). It is read as `Owner` (the open format writes `["this", 0]`) and bound to the script's object
  when the runtime starts; before, such a parameter was lost and every block reading it got nothing.
- **Object Copy** makes a copy: an object of the run with the original's place, mesh and box, a name of its own
  (the original's and a three-digit number, `Base_Portal_In002`: templates tell their copies apart by name —
  the name's exact form in the original is not known), and a copy of every script attached to the original, bound
  to the copy ("This" is the copy) and in the state the original's is in. The run's graph is its own from the first
  copy on (the level's is not touched; a rewound run copies again and gets the same). At most 256 copies a run (tries
  count, made or refused), 16384 nodes a copied script; the nodes a copy makes are paid from the tick's work
  budget, and a copy that does not fit in it, or in the graph's cap, is refused whole. The copies are **not drawn and have no body** yet.
- **Box Box Intersection** with an object without a mesh tests its place as an empty box (the original's box of
  such an object). With these three the **Portal** maps' teleports work: the template copies a mesh-less frame
  with the script that tests the ball against "This" to each portal.
- **Groups** change in the run: Add To Group, Remove From Group, Group Clear (level objects only).
- **Progressions**: Linear Progression, Bezier Progression (its 2D curve is not decoded: a straight line),
  Interpolator, Get / Set Component, TT_Timer.
- **Random**, **Random Switch**: one generator with a fixed seed in the run's state, stepped by each draw, so
  a run and its replay draw the same numbers. (The original's are not reproducible at all.)
- Set Euler Orientation, Scale, Get Nearest In Group, Row Search, Remove Row, Object Rename.

Still not done, most wanted first: **physics by script** for level objects (Physicalize of a floor or a box,
Physics Impulse, SetPhysicsForce, Set Physics Slider): an object a script moves, shows or hides keeps the body it
was loaded with, so a platform moved by a script is drawn at its new place and collides at its old one; drawing
the copies (the portals' particle frames); Attach Script, Object Create; sounds through the sound engine. The test
player (`--autopilot`) does not know the portals: its way stops at one. A replay of a scripted map recorded
before this change may play differently (the scripts now do more).

## Stage 0 (2026-11-11): core conformance

The first stage of porting the script system as a whole (not map by map): the core, so that every later block stands on the original's execution model.

**The registry of prototypes** (`crates/script/src/blocks/`). One declaration per prototype: GUID, name, the
player's library it lives in, the pin schema (input and output parameters by index, as the prototype names
them), flags (targetable, variable inputs / pins / outputs), standing (full, partial, no-op), older pin layouts
as data, the run function and an optional event function. The modules mirror the player's libraries
(`logics.rs` = Logics.dll, `narratives.rs`, `transfo3d.rs`, `tt_toolbox.rs`, ...); a prototype not declared is
reported once as before.

**The executor** (`crates/script/src/exec.rs`), as the original's behaviour manager
([ck-runtime.md](../ck-runtime.md), "Execution"):

- *Activation requests.* Activate Script and Deactivate Script file a request on the script; the next tick applies
  every request before any script runs. A script activated by another one starts in the tick after, never in the
  same one; one deactivated still finishes the tick it is in. Activation and deactivation asked in one tick: the
  later wins. Activate Script: pin 0 Reset?, pins 1.. the scripts; an older layout (pin 0 the script, pin 1 Reset?,
  default true) is data of its declaration, chosen when pin 0 is not a boolean; local 0 Awake Object (default
  true) wakes a sleeping owner.
- *Reset is separate from active.* A behaviour is reset when asked, or when activated for the first time; a script
  activated again without Reset? resumes where it was (its delayed links go on counting). Before stage 0 every
  activation of an inactive script reset it.
- *Order.* Objects by their priority (identifier `0x40`; in the maps only the Level object has one, 32000),
  stable; each object's scripts in its order.
- *Start state.* Each script starts reset and active, unless its saved activity (identifier `0x200000` of a
  behaviour, `0x400` of its object: bit 1 activate, `0x10` deactivate) starts it deactivated: 24 scripts of the
  community maps do. The saved behaviour flags say nothing of it (1357 of the 1539 scripts that run have no active
  bit there).
- *Objects in the scene.* Deactivate Object puts an object to sleep: its scripts are not run (their state is kept)
  until Activate Object wakes it, which also asks for its scripts' activation (reset if asked). Object Delete of an
  object deletes its scripts with it (the manager never runs them again); of a behaviour, that behaviour.
- *Block callbacks.* A declaration may carry an event function for Reset (at the play's start), Activate Script /
  Deactivate Script (its script, or its script's object, changes state), Delete and Copy. The particle emitters use
  it (docs/particles.md, section 8): deactivating the owning script takes the particles off at once, activating it
  again brings back an emitter that was on.
- *Block state where the original keeps it.* Group Iterator and Collection Iterator keep their index in output
  parameter 1 (Index), else in local 0, as the original's (read by other blocks, and kept as theirs is); the rest
  keep a scratch state, cleared when the script is reset (whether the original's blocks clear theirs then is not
  established: it sends no reset message on a script's reset).
- *Execute Script* activates the script (reset if pin 0 says so) and runs it at once inside the caller's frame;
  the manager does not run it again that tick. The block waits (stays active) and fires Out in the first later tick
  in which the script is no longer active. A script cannot run inside itself; nesting is capped at 8.
- *Trace.* `Runtime::set_trace(true)` keeps a line per frame (`frame N`), behaviour run (`run <id> <name>` for a
  block, `graph <id> <name>`), IO fired (`io <owner> <name> in<k>|out<k>`), script activation applied and
  parameter written (`param <id> = <value>`), capped at 2^20 lines; `script-dump --run N FILE` prints it.

**Not in the player.** Eight kinds found in maps have a GUID in none of the player's libraries: Explode, Midi
Player, Run VSL, Add Mesh, Remove Mesh, Select Mesh, Get Nearest Object and Object Rename `6f074ca7:45427431`
(Narratives.dll has a block of that name with another GUID). The original has no function for them, so they do
nothing there. They are declared as **named no-ops** (owner's decision 2026-11-11): known, inputs cleared, no
output, one log line. The Object Rename written in phase 2 stays in the code behind the cargo feature
`absent-as-authored` (off), for a later optional mod that plays such maps as their authors meant.

## Hostile input

A map comes from a stranger. Every list is capped (objects 2^20, array rows 2^16, 4096 messages a tick, 2000
log lines), every index is checked, recursion is bounded (48 levels, 8000 stack pops a tick), a malformed
object is skipped and the rest of the graph runs; a block the runtime does not know is reported once
(`script block not known: <name> (<GUID>)`) and its inputs cleared, so a map never spins on it. The reader never
panics on file data (`crates/nmo/src/behavior.rs` has its own tests on synthetic chunks; the fuzz target of the
file reader covers these classes too).

## Building blocks: the list

`nmo-dump --scripts FILE` prints a map's blocks by prototype with counts. The table is over the owner's 584
maps (`levels/community/` and `community-unsorted/`, 2026-11-09): 184 maps carry scripts, 148 of them only the
template (the nine blocks plus Wait Message), 36 scripts of their own. *maps* = maps using the block, *blocks* =
blocks in all maps. Status: **no-op** — in none of the player's libraries, so the original does nothing with it; known and does nothing (section "Stage 0"); **done** — runs with its effect; **partial** — runs, effect limited as said;
**planned** — not yet; **never** — the original's own interface (mouse, fonts, 2D text, camera, fog), replaced
by the engine's.

| Block | GUID | maps | blocks | status |
|---|---|---|---|---|
| Op | `2d5d6d01:6a353eb0` | 184 | 896 | done |
| Object Delete | `74120ded:76524673` | 183 | 231 | done |
| Get Cell | `33b99f51:07d95c45` | 183 | 486 | done |
| Activate Script | `4c7e7bc3:0b693155` | 182 | 1140 | done |
| Point Particle System | `506b40f7:30852e46` | 182 | 799 | done |
| Deactivate Script | `14367c05:635b24f9` | 182 | 738 | done |
| Binary Switch | `eb506901:984afccc` | 182 | 214 | done |
| Change Texture Video Format | `f56d92ef:41eb20c1` | 180 | 4125 | done |
| TT Scaleable Proximity | `2a2a63ca:00826175` | 179 | 183 | done |
| Wait Message | `4587ffee:4587ffdd` | 35 | 76 | done |
| Group Iterator | `6050252f:3aa82d40` | 22 | 64 | done |
| Hide | `31d97d82:78d54d98` | 20 | 76 | done |
| Physicalize | `7522370e:37ec15ec` | 19 | 98 | partial: runs; the ball only (other objects: phase 2) |
| Box Box Intersection | `64154401:76cf37af` | 19 | 44 | done |
| Set Position | `e456e78a:456789aa` | 18 | 84 | done |
| Iterator | `198f0af9:0268249f` | 17 | 41 | done |
| Keep Active | `7160133a:1f2532fe` | 17 | 35 | done |
| Show | `a85a213a:ef78d52a` | 16 | 66 | done |
| Switch On Parameter | `4c42aace:1da45635` | 16 | 28 | done |
| Add Row | `1c7e5dc6:3f6423c2` | 15 | 36 | done |
| Delayer | `15d472a5:3bea409f` | 15 | 49 | done |
| PlanarParticleSystem | `49957bfe:0ff27ffc` | 14 | 37 | done |
| Activate Object | `5f003fba:5e574f60` | 14 | 49 | done |
| Get Key Row | `49064205:10e72f7a` | 13 | 38 | done |
| Object Copy | `3f6b0ac7:47d20f78` | 13 | 42 | partial: the copy and its scripts are made and run (own name, `This` bound to it); not drawn, no body |
| Send Message To Group | `5f906952:6df11649` | 12 | 31 | done |
| Text Display | `f22d010a:f2cd010a` | 11 | 32 | done |
| Test | `17d66d26:726b7dec` | 11 | 96 | done |
| Get Current Camera | `368f22b1:222957e4` | 11 | 12 | partial: none |
| Set Orientation | `625874aa:aa694132` | 10 | 10 | done |
| Set Cell | `30ed1c6d:4a3b7067` | 9 | 17 | done |
| Wave Player | `5bde0e45:2e2107d5` | 9 | 18 | done |
| Nop | `302561c4:0d282980` | 8 | 50 | done |
| Restore IC | `766e4e44:4fac6d52` | 8 | 23 | done |
| Identity | `15151652:aeefffd5` | 6 | 38 | done |
| Key Event | `1af2274b:6b8c1524` | 6 | 22 | partial: never fires (keys are not in a run) |
| Parameter Selector | `63eb2b0c:27e2767e` | 6 | 20 | done |
| Add To Group | `00024125:785420ab` | 5 | 11 | done |
| Rotate | `ffffffee:eeffffff` | 5 | 14 | done |
| Object Create | `271538e6:2fae49ac` | 4 | 9 | planned |
| Bezier Progression | `6bb8699a:29fb6a4b` | 4 | 22 | partial: runs as a straight line (the block's 2D curve is not decoded) |
| Interpolator | `35503950:2dde7a65` | 4 | 11 | done (colours mixed in RGB) |
| SetPhysicsForce | `56e20c57:0b926068` | 4 | 5 | partial: accepted, no force yet |
| Get Component | `2689057b:1a1a446a` | 4 | 8 | done |
| Set World Matrix | `aa4aa6f0:ddefdef4` | 4 | 9 | done |
| Set Diffuse | `e1e1e1e1:1e1e1e1e` | 4 | 11 | partial: accepted, no effect |
| Volume Control | `33613f21:776b185b` | 4 | 8 | planned |
| Collection Iterator | `419602ec:7822779e` | 4 | 20 | done |
| Set Texture | `eb123eb5:5be321be` | 3 | 3 | partial: accepted, no effect |
| Set Component | `6e800755:57b64acb` | 3 | 12 | done |
| Set Physics Slider | `2973360e:23d31aa7` | 3 | 3 | planned |
| Per Second | `448e54ce:75a655c5` | 3 | 7 | done |
| Counter | `998f000f:f000f899` | 3 | 9 | done |
| Binary Memory | `d02d67dd:10211fdd` | 3 | 7 | done |
| Get Nearest In Group | `085207eb:584950d8` | 3 | 4 | done |
| Is In Group | `58a1210b:14e5715c` | 3 | 18 | done |
| Mouse Waiter | `014d010b:015d010b` | 3 | 3 | never (the engine's own interface) |
| Set Light Color | `32115590:78951230` | 3 | 3 | planned |
| Timer | `a2a5a63a:e4e7e8e5` | 3 | 4 | done |
| Physics Impulse | `0c7e39bb:16db20d5` | 3 | 6 | planned |
| Midi Player | `843cb43a:a12dac48` | 2 | 2 | no-op |
| Object Rename | `6f074ca7:45427431` | 2 | 4 | no-op |
| Set Font Properties | `dacfbd61:7a6e65e7` | 2 | 6 | never (the engine's own interface) |
| 2D Text | `055b29fe:662d5ca0` | 2 | 2 | never (the engine's own interface) |
| Fill Group By Class | `4445257b:70016c57` | 2 | 8 | planned |
| 3D Text | `1f5b2fe7:6d355175` | 2 | 3 | never (the engine's own interface) |
| Row Search | `78863443:45af59b6` | 2 | 2 | done |
| Create String | `4bcd2f9d:382652e2` | 2 | 2 | planned |
| Proximity | `5321cacb:dcdc5213` | 2 | 2 | done |
| Bezier Transform | `6d433e1b:191954b4` | 2 | 3 | planned |
| Set Attribute | `373040f2:05e01b34` | 2 | 13 | planned |
| Set Emissive | `d1d1d1d1:30303030` | 2 | 2 | partial: accepted, no effect |
| Chrono | `5e2f1788:4f965a44` | 2 | 3 | done |
| Execute Script | `706c5a40:5bb31a0b` | 2 | 9 | done |
| Send Message | `a20e8d5b:df002150` | 2 | 6 | done |
| Attach Script | `1dba5f4a:14481212` | 2 | 3 | planned |
| Clear Array | `35c9352f:7b1a193b` | 2 | 7 | done |
| Get Delta Time | `47dc3232:64bf203a` | 2 | 3 | done |
| Play Sound Instance | `00283a35:38ef6b48` | 2 | 7 | planned |
| Group Clear | `1d12662e:42687a3c` | 2 | 9 | done |
| Scale | `41236987:a54a87a6` | 2 | 7 | done |
| Remove From Group | `d0147412:0258520d` | 2 | 2 | done |
| Linear Progression | `fff45680:aa512a39` | 2 | 3 | done |
| Set Blend Modes | `2f572a66:6a9a0088` | 2 | 5 | planned |
| Set Material Z Buffer | `1144022f:68fd055c` | 2 | 5 | planned |
| Create System Font | `936334fc:f243684f` | 1 | 1 | never (the engine's own interface) |
| Movie Player | `778d16d4:1dd60060` | 1 | 1 | never (the engine's own interface) |
| TT ProximityVolumeControl | `38571b61:64cd2174` | 1 | 1 | planned |
| Blend Textures | `53ee4292:7d932bb1` | 1 | 7 | planned |
| Get SubString | `10b16051:26173b39` | 1 | 1 | planned |
| Iterator If | `6bec4be6:12d64c7c` | 1 | 2 | planned |
| TT_LinearVolume | `09b335b3:12d17cdc` | 1 | 1 | planned |
| Set Ambient | `aaaaaaaa:aeae1287` | 1 | 1 | never (the engine's own interface) |
| Set Specular | `20202020:30303030` | 1 | 1 | planned |
| Write In Texture | `1d57024d:7c064238` | 1 | 1 | never (the engine's own interface) |
| Show Mouse Cursor | `16f6368f:506b60fc` | 1 | 1 | never (the engine's own interface) |
| Get Nearest Object | `4321abcd:0eeaabdd` | 1 | 1 | no-op |
| Input String | `693e7e4b:100104c4` | 1 | 1 | never (the engine's own interface) |
| Mouse Cursor System | `2483576d:57c66324` | 1 | 1 | never (the engine's own interface) |
| Mouse Cursor | `140c3eac:6e502a81` | 1 | 1 | never (the engine's own interface) |
| Set Pickable | `f1499052:ebe9bbf1` | 1 | 1 | never (the engine's own interface) |
| TT_Timer | `6ac67901:7d2a6059` | 1 | 1 | done |
| Set IC | `72671a24:161a2347` | 1 | 1 | partial: accepted (initial conditions stay as loaded) |
| Remove Row | `1fa57136:14310857` | 1 | 1 | done |
| Shuffle Order | `270f177a:4ca20023` | 1 | 2 | planned |
| Move Row | `101a62f4:6e2d25f6` | 1 | 2 | planned |
| Set FOV | `aaaacccc:aaaacccc` | 1 | 3 | never (the engine's own interface) |
| TT Restore IC | `7e0f2f58:232d3109` | 1 | 4 | done |
| Streaming Event | `1f0b52bf:4c3342dd` | 1 | 1 | done |
| Pitch Control | `7b7c731e:175b27a3` | 1 | 3 | planned |
| Deactivate Object | `160f4b7d:67de224e` | 1 | 1 | done |
| Random | `0c622386:1c3054f7` | 1 | 35 | done (a fixed seed: every run draws the same numbers) |
| Set Euler Orientation | `0c4966d8:6c0c6d14` | 1 | 7 | done (X, then Y, then Z) |
| Has Attribute | `25b54079:6ff90545` | 1 | 4 | planned |
| Set Material | `37865d50:2e285fab` | 1 | 3 | planned |
| Objects With Attribute Iterator | `6bc1494c:0c816ad3` | 1 | 7 | planned |
| SphericalParticleSystem | `67c88f47:8880721e` | 1 | 1 | planned |
| Set 3DSprite Mode | `0c5d0457:c64c7000` | 1 | 1 | planned |
| Set Current Slot | `aaaa213a:eaa8d52a` | 1 | 3 | planned |
| Sequencer | `42530844:257b6053` | 1 | 1 | done |
| Calculator | `4bc209b8:5b3679b0` | 1 | 8 | planned |
| TT CreateFontEx | `260e4eb0:0e256b90` | 1 | 1 | never (the engine's own interface) |
| Get Message Data | `45875fee:45875fdd` | 1 | 1 | planned |
| Edit 2D Entity | `7fc54c91:3f486670` | 1 | 6 | never (the engine's own interface) |
| Set Clipping Planes | `652316a2:01aa09a9` | 1 | 2 | never (the engine's own interface) |
| Set Fog | `151aaaaa:ffd5bdbc` | 1 | 3 | never (the engine's own interface) |
| TT Sky | `36691920:3b261630` | 1 | 1 | never (the engine's own interface) |
| Get Row | `33b77f41:07b95c45` | 1 | 1 | done |
| Set As Active Camera | `368f0ab1:2d8957e4` | 1 | 2 | never (the engine's own interface) |
| Physics WakeUp | `38b851b5:72ca74ac` | 1 | 1 | planned |
| Set Parent | `9d9d9d98:7e7a7f75` | 1 | 1 | planned |
| Scan String | `4afa2f11:034872e2` | 1 | 1 | planned |
| Set Alpha Test | `1cb5661e:5b5d3fda` | 1 | 1 | planned |
| Change Texture Size | `7a7c4d7b:468d0f33` | 1 | 1 | partial: accepted, no effect |
| Is Same Texture | `021e6fa7:6fe74610` | 1 | 1 | no-op |
| Sort Rows | `6f623e68:62bb5a98` | 1 | 5 | planned |
| Get Texture Info | `250c4d8d:288960d9` | 1 | 2 | planned |
| Replace Texture | `?` | 1 | 1 | planned |
| Renamer | `?` | 1 | 4 | planned |
| To Lower Keys | `?` | 1 | 1 | planned |
| Insert Column | `05cb7802:04a64a58` | 1 | 3 | planned |
| Replace Material | `?` | 1 | 1 | planned |
| Is Same Material | `?` | 1 | 1 | planned |
| Add Mesh | `7a1d593e:12c5648d` | 1 | 1 | no-op |
| Remove Mesh | `0002145e:6e1b2592` | 1 | 4 | no-op |
| Select Mesh | `43890176:58b2244e` | 1 | 1 | no-op |
| Is Same Mesh | `?` | 1 | 1 | planned |
| Switch On Key | `1f49612e:6cd63de0` | 1 | 2 | never (the engine's own interface) |
| Wait For All | `c044a999:fdfefaf7` | 1 | 1 | done |
| Key Waiter | `016d010b:017d010b` | 1 | 1 | never (the engine's own interface) |
| Delete Dynamic Objects | `0cba3195:53440e4c` | 1 | 1 | planned |
| Random Switch | `79d72fde:2e9d0912` | 1 | 1 | done (the same generator) |
| Explode | `002d010a:003d010a` | 1 | 1 | no-op |

## Where the maps stand

With the blocks above, **173 of the 184 scripted maps** have every block known to the runtime: the 148
template maps and 25 of the 36 with scripts of their own. Of those 25, eight use only blocks with their full
effect (AAAAAA Class Islands Scenery, Be Besieged on All Sides 2.0, Painstaking Journey, Queen Ball (Easy
Ver.), Take the leap, Traveling on Rubber Walls, Trial of the Ballance King, 反向Level_11); the other 17 run
with a limited block: the nine **Portal** maps (Dueling With the Ice Cube ×3, Find Your Place, In the Field 2.0,
Periodic Travel, Rectangular Elements, See You Tomorrow, Traveling Through the Void 2.0) share one portal script
whose Object Copy (the portals' particle frames) was not made in phase 1, so the portals did nothing (phase 2 makes them work, see above); 2010s, A Little Journey, All-We-Know physicalize objects by script; Board-Na
Leona sets a texture; Family Portrait 2 and 反向Level_08 wait for keys; Omit Superfluous Things resizes a
texture; Queen Ball (Hard Ver.) sets a physics force.

The 11 maps with blocks not yet known, and what they need, most blocking first: Add To Group (5 maps), Bezier
Progression, Get Component, Interpolator, Object Create, Volume Control (4 each), Set Component, Set Physics
Slider, Get Nearest In Group, Set Light Color, Physics Impulse (3), then blocks of one or two maps. The big ones
are **Landfill in the Clouds** (87 kinds of blocks, 991 links), **Shuffle Between Earth and Moon** (a texture
editor of its own), **Morning Mist**, **Back and Forth**, **Misty**, **A Peaceful Night at the Seashore**,
**Fierce Battles** (a mouse-driven menu), **Stairway to the Clouds**, **Weird Balance** (Explode).

Each map is listed in [scripts-maps.csv](scripts-maps.csv): its kind (template or own), top-level graphs,
blocks, the blocks beyond the template, the blocks not covered, and whether phase 1 covers it.

## The order of work to reach all maps

1. **Groups and progressions** (5–6 maps each): Add To Group, Remove From Group, Group Clear, Fill Group By
   Class; Bezier / Linear Progression, Interpolator, Bezier Transform (curves need the level's curves: class 43 is
   read for the open format already); Get / Set Component; Get Nearest In Group, Get Nearest Object.
2. **Physics by script** (the Portal family's full effect, 2010s, A Little Journey, All-We-Know, Misty, Morning
   Mist, Landfill): Physicalize of level objects (a fixed concave body like the floors', or a convex body with the
   `Physicalize_*` tables' material), Unphysicalize, Set Physics Slider, SetPhysicsForce, Physics Impulse, Physics
   WakeUp; objects moved by script move their body.
3. **Copies and created objects**: Object Copy (an entity with the original's mesh, drawn and physicalizable),
   Object Create, Object Rename, Attach Script (a copy of a script bound to a new owner).
4. **Sound and text through the engine**: Wave Player on the map's own sounds (class 24/25 are not read yet:
   the sound data lives in the map), Volume Control, Pitch Control, Play Sound Instance, Midi Player (never:
   no MIDI; a note), Text Display / 2D Text in the HUD's own font.
5. **Looks**: Set Texture, Set Diffuse / Emissive / Ambient / Specular, Blend Textures, Set Blend Modes, Set
   Material Z Buffer, Set Alpha Test, Set Current Slot, Change Texture Size, Replace Texture / Material, Add /
   Remove / Select Mesh, Scale, Set Euler Orientation, Set 3DSprite Mode, Set Light Color, Set Fog, Set FOV
   (the engine's camera is its own: a note).
6. **The rest of logic**: Random (seeded from the run, deterministic), Random Switch, Shuffle Order, Calculator
   (expressions), Row Search, Sort Rows, Move / Remove Row, Insert Column, Has / Set Attribute, Objects With
   Attribute Iterator, Iterator If, Get Message Data, Execute / Attach Script, Scan String, Create String, Get
   SubString, To Lower Keys, Renamer.
7. **Never**: the mouse (Mouse Cursor, Mouse Waiter, Show Mouse Cursor, Input String, Key Waiter, Switch On
   Key), fonts (Create System Font, TT CreateFontEx, Set Font Properties), Movie Player, Write In Texture,
   Edit 2D Entity, Set As Active Camera, Set Clipping Planes, TT Sky: the game's own interface and camera take
   their place; a map that needs them for play is reported by name.

Semantics not knowable from the files, to be checked against the original by playing: the exact frame a
delayed link fires relative to the physics step; Keep Active's "one at a time" local; the key column of Get
Key Row (taken as the first column); TT Scaleable Proximity's first test after On (taken as at once).
