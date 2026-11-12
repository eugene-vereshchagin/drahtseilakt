# Virtools runtime semantics (CK2 2.1)

What OpenBallance's `src/ck/` reproduces, from `CK2.dll` (image base `0x24000000`). Addresses are
function entry points in the Ghidra project `ballance`, program `CK2.dll`.

## Files

`.cmo`/`.nmo` layout, state chunks and identifiers: see `src/ck/ck_file.h`. Object references inside
chunks saved with option 8 are indices into the file's object table.

### Base object (`CKObject::Load` 0x24021053)

| Identifier | Meaning |
|---|---|
| `0x4` present | hidden (neither `VISIBLE 0x40` nor `HIERARCHICALHIDE 0x200`) |
| `0x18` present | hierarchically hidden (`0x200`) |
| neither | visible (`0x40`) |

### Behavioral objects (`CKBeObject::Load` 0x2401bf56)

| Identifier | Fields |
|---|---|
| `0x800` | scripts: count, behavior references |
| `0x11` | attributes of a behavioural object (settled on real files, 2026-11-11): count *n*, *n* parameter references (the parameter objects holding the values), the count again, the attribute manager's GUID (two words), *n* file attribute indices. Example, an object with two attributes: `2, 0x385, 0x386, 2, 0x3d242466, 0, 0x29, 0x41`. The indices are the file's own and the same space as the *Attribute* pin of Has / Set / Remove Attribute and Objects With Attribute Iterator (the pins of a map hold the numbers that its objects carry), so no table is needed to match them; the attribute manager's chunk (identifier `0x52`: category count, attribute count; per category exists, name, flags; per attribute exists, name, parameter type GUID, category, compatible class, flags) only names them and types their values, and `Document::parse` skips it. `0x10` on behaviours is something else (not attributes: those objects are scripts), and no object of another class in the files examined (two community maps, the Gameplay file and a part module of the original) carries it; only objects of class 41 (3D objects) carried `0x11` there |
| `0x40` | priority word (data version >= 5: a flag word first, the priority follows only if it has bit `0x10000000`; in the community maps only the Level object has one: `0x10000000, 32000`) |
| `0x400` | the object's saved scene activity (CKObjectManager single-object activity): 1 activate, `0x10` deactivate, `0x20` leave, `0x40` reset, `0x80` the state as loaded is the initial value. A behaviour saves its own under `0x200000`. Most map files save none |

In `base.cmo` the `Level` object owns the nine top-level scripts: `Delete_All_Loaded_NMO`,
`Delete_Loaded_NMO`, `Loading_Manager`, `Default Level`, `Event_handler`, `Delete_LoadID`, `Level_Init`,
`Loading_Screen`, `Debug_Info`.

### Behavior (`CKBehavior::Save` 0x24004e1e; the load mirrors it)

Identifier `0x20`: `flags`; if `0x8000` prototype GUID + prototype version; if `0x4` priority; if `0x10`
compatible class id; if `0x40000` target object. Then a save mask and object arrays in this order:

| Mask bit | Array | Class |
|---|---|---|
| `0x100` | sub-behaviors | Behavior (8) |
| `0x80000` | sub-behavior links | BehaviorLink (6) |
| `0x4000` | parameter operations | ParameterOperation (4) |
| `0x200` | input parameters | ParameterIn (2) |
| `0x400` | output parameters | ParameterOut (3) |
| `0x20000` | local parameters | ParameterLocal (45) |
| `0x800` | inputs | BehaviorIO (9) |
| `0x1000` | outputs | BehaviorIO (9) |

- BehaviorIO: identifier `0x8`, flags (`1` input, `2` output).
- BehaviorLink: identifier `0x20`: delays (low word current, high word initial; both equal in the data
  seen so far), source IO, destination IO.
- ParameterIn: identifier `0x1000` (direct source) or `0x2000` (shared with another input): type GUID,
  source reference.
- ParameterOut / ParameterLocal: identifier `0x40`: type GUID, value mode, value (CKParameter load,
  FUN_2400867c): `0` type-specific sub-chunk, `1` buffer (u32 bytes + data; message and attribute types
  store names, parameter-type values store a GUID), `2` object reference, `3` none, other: skip one dword,
  value in the next. Four legacy GUIDs are remapped first. ParameterOut also has `0x20`: destinations.
- ParameterOperation: identifier `0x400`: operation GUID, argument count (3), input 1, input 2, output.

## Execution

### Frame (`CKBehaviorManager::Execute` 0x2400c40c)

1. For every object with scripts, in registration order: a script with a pending request
   (flags `0x10400000`) is `Activate(flags & 0x10000000, flags & 0x800000)` (activate, reset) and the
   request bits (`0x10c00000`) are cleared.
2. For every object: `CKBeObject::ExecuteBehaviors` (0x2401af67): each script that is active is
   `Execute`d, in script order.

### `CKBehavior::Activate(active, reset)` (0x24003957)

If `reset` or the flags are all zero: reset (FUN_24005dc8): every link's current delay = initial delay,
the delayed-link list is emptied, every sub-behavior is deactivated and reset recursively, every IO is
deactivated, flags `&= 0xef3fffbf`, `|= 0x80000000`, and if the behavior is a script (`0x2`) its input 0 is
activated. Then the active bit (`0x1`) is set or cleared.

### `CKBehavior::Execute` (0x24003806)

Building blocks (`flags & 0x8`, FUN_240048c4) call the prototype's function with the behavior context.
If bit 0 of the return value is clear (`CKBR_OK`), the behavior is deactivated; `CKBR_ACTIVATENEXTFRAME`
(1) keeps it active. The function itself reads and clears its inputs and activates outputs.

Graphs:

1. **Prepare** (FUN_240044ec). For each sub-link, clear its "fired" bit (`2`). If the link's source IO is
   active: remember the source; mark the link fired; if its initial delay is 0 the destination IO is
   collected for activation, else (current delay := initial delay if 0) the link is queued once
   (queued bit `1`) on the graph's delayed list. Then all remembered sources are deactivated, all collected
   destinations activated (and an input IO activates its owner behavior). Finally the execution stack is
   rebuilt: sub-behaviors from last to first, every active one (other than the graph itself) is pushed and
   marked "stacked" (`0x100000`), so the first sub-behavior is on top.
2. **Loop**. While the stack is not empty: pop the top, clear its "stacked" bit, `Execute` it, then
   **propagate** its outputs (FUN_2400477f): each active output IO is deactivated; each of its links is
   marked fired; a zero-delay link activates the destination IO immediately and, if that is an input of a
   behavior other than this graph, activates that behavior and inserts it into the stack by priority
   (above every entry with a priority <= its own, below higher ones) unless already stacked; a delayed link
   is queued as in Prepare. The loop runs at most `BehaviorMaxIteration` times; beyond that the original
   logs "Infinite Loop Detected" (FUN_24005e7c) and flags the result.
3. **End** (FUN_2400465b). Every queued link: fired; current delay decremented; at `< 1` it is reset to 0,
   unqueued, its destination IO activated and the destination's owner behavior activated; otherwise it
   stays queued. The graph is then deactivated unless links remain queued or a sub-behavior has flags
   `0x41` (active, or `0x40`).

So a link with delay 0 fires within the same frame, a link with delay *n* fires *n* frames later, and a
behavior activated through a delayed link runs in the frame after the one in which the link fired.

### Parameters

Reads are pulled. Copying a parameter's value from a source (FUN_2400824d) first runs the source's
operation (FUN_2400a7b5 → `CKParameterOperation::DoOperation` 0x240098bb) if the source is an operation's
output. Operations call their function with (context, output, input 1, input 2) (inputs swapped by a
flag), then mark the output changed. An operation without a function copies input 1 to the output.

## Parameter types

The parameter manager keeps a table of types: GUID, name, how the value is held, the parent type, the size. Our
registry (`crates/script/src/types.rs`) has the types the community maps and the game use; the parents are the
C reference's (`src/ck/ck_types.c`): Boolean and Class ID are Integers; Time, Angle and Percentage are Floats;
Euler Angles a Vector; Behavioral Object and Script are Objects; 3D Entity, 2D Entity, Material, Texture, Mesh,
Group, Array, Sound and Level Behavioral Objects; 3D Object, Camera and Light 3D Entities; Wave Sound a Sound.
Values: Integer, Float, Boolean, String (64-byte buffer at creation), Vector (12 bytes), 2D Vector (8), Quaternion
(16, x y z w), Matrix (64, column-major), Color (16, RGBA), Rectangle (16), Box (24, min then max), Message and
Attribute (by name in the file), Object types (a reference), Object Array (references). A type not in the table
keeps its saved bytes. Defaults: zero, empty, no object; a Quaternion's is (0, 0, 0, 1), a Matrix's the identity.
Time is held as a Float (milliseconds) and Class ID as an Integer: the type, not the value, says which.

The object types filter by class: a String converted to an object type (Convert, Get Object By Name) finds the
first object of that name whose class is the type's class or derives from it (CKCID: 3D Object 41 → 3D Entity 33
→ Render Object 47 → Behavioral Object 19 → Scene Object 11 → Object 1; Camera 34, Light 38, Sprite 3D 37, Curve
43 → 3D Entity; Sprite 28 → 2D Entity 27 → Render Object; Material 30, Texture 31, Mesh 32, Group 23, Array 52,
Sound 24, Level 21, Scene 10 → Behavioral Object; Wave Sound 25 → Sound). The name matches in any case (as
`GetObjectByName`, docs/drahtseilakt/scripts.md); the C compares exactly. The game's pseudo-objects
(`CurrentLevel`, `Ball_Pos_Frame`...) have no class and match every type.

## Operations

`CKParameterManager::GetOperationFunction` (0x24010f6d, as the C reference `ck_op_function` has it) looks a
function up by (operation, output type, input 1 type, input 2 type): the exact signature; else, recursively, with
the parents: if input 1 has a parent — (both inputs' parents) with the output's parent, then with the output, if
input 2 has one; then (input 1's parent, input 2) with the output's parent, then with the output — then, if input
2 has a parent, (input 1, input 2's parent) with the output's parent, then with the output; last the output's
parent alone. A missing input is the None type, which has no parent. When that finds nothing, the same lookup
with the inputs swapped; a function found so is called with the inputs swapped. Without any function a graph's
own operation copies input 1 to its output (DoOperation); an Op block writes nothing and fires Out (the C
reference's Op; the notes are silent on it). The original logs the missing signature; we log it once per run.

The functions (`crates/script/src/ops.rs`, `TABLE`), from the C reference unless marked *name* (the C has no
function: the meaning is the operation's registered name, not yet checked against the original):

| Operation | Signatures (output ← inputs) | Function |
|---|---|---|
| Addition, Subtraction, Multiplication | Integer ← Integer, Integer; Float ← Float, Float | wrapping integer arithmetic; single-precision float |
| Multiplication | Float ← Integer, Float; Integer ← Float, Integer | `(float)i * f`; `(int)(f * (float)i)` (converted as Convert Float → Integer) |
| Division | Integer, Float | a divisor of 0 becomes 1 (and is logged in the original); `i32::MIN / -1` faults in the original's x86 code (the game would stop); ours gives `i32::MIN` (and 0 for the remainder) |
| Modulo `2a3f4ab6` | Integer ← Integer, Integer | `r = a % d`, `r + d` when `r < 0`; 0 for a divisor of 0 |
| Addition | String ← String, String | concatenation (ours stops growing at 64 KiB) |
| Addition, Subtraction *name* | Vector ← Vector, Vector | per component |
| Subtraction, Division *name* | Float ← Integer, Float | as floats (a divisor of 0 becomes 1) |
| Multiplication, Division *name* | Vector ← Vector, Float | each component (a divisor of 0 becomes 1) |
| Multiplication *name* | Vector ← Matrix, Vector | the point transformed, translation included |
| Opposite *name* | Integer, Float, Vector | negation |
| Normalize, Cross Product, Get Magnitude, Arc Cosine *name* | Vector; Vector ← Vector, Vector; Float ← Vector; Float ← Float | unit vector (zero stays zero); cross product; length; `acos` (NaN outside [-1, 1]) |
| Set X / Y / Z *name* | Vector ← Vector, Float | the vector with that component replaced |
| Get X / Y / Z / W *name* | Float ← Quaternion | the component |
| Get Y | Float ← Vector; Float ← 3D Entity | the component; the entity's world position's y (nothing written without an entity) |
| Get Right / Up / Dir *name* | Vector ← 3D Entity | the world matrix's column 0 / 1 / 2, normalized |
| Convert | Float ← Integer; Integer ← Float; 2D Vector ← Float, Float | `(float)`; truncation toward 0, out of range or NaN `i32::MIN` (x86's integer indefinite); (x, y) |
| Convert | String ← Integer, Float, Boolean, Time, Percentage | the input type's string function: `%d`; `%g`; `TRUE` / `FALSE` |
| Convert | Integer, Object ← String | the output type's string function: the leading integer (`%d`, else 0); the object by name and class (above) |
| Convert *name* | Integer ← Boolean; Quaternion ← Matrix | 0 / 1; the rotation's quaternion |
| Get Object By Name | Object ← String | as Convert |
| Get Length, Get Name, Class ID `35aa0ddb` | Integer ← String; String ← Object; Class ID ← Behavioral Object | bytes; the name ("" without one); the class id (0 without one) |
| Row count `fb556b23`, Get Count | Integer ← Array; Integer ← Group | rows; members |
| Script `250020d4`, script count `551c20ba` | Script ← Behavioral Object, Integer; Integer ← Behavioral Object | the object's script by index (none outside); how many |
| Get Current | Mesh ← 3D Entity | the current mesh |
| Get Mesh *name* | Mesh ← 3D Entity, Integer | index 0: the current mesh; else none (the entity's other meshes are not kept) |
| Material `6b8c78bd` | Material ← Mesh, Integer; Material ← 2D Entity | the mesh's material by index (none outside); a 2D entity's (none: 2D entities are not modelled yet) |
| Get Material List, Get Element | Object Array ← Mesh; Object ← Object Array, Integer | the mesh's materials; the element (none outside) |
| Get World Matrix, Get Position | Matrix ← 3D Entity; Vector ← 3D Entity [, 3D Entity] | nothing written without an entity; the position in the referential's frame when input 2 is one |
| Get Position | 2D Vector ← 2D Entity | nothing written (2D entities later) |
| bounding box `31a7649f`, `6ea214cb` | Box ← 3D Entity; Vector ← Box | the world box; its max corner |
| Get Diffuse Color, Get Emissive Color | Color ← Material | the material's colour (emissive alpha 1: the level keeps RGB) |

`%g` is C's: six significant digits, trailing zeros dropped, an exponent (`1e+06`, `1e-05`) below 1e-4 and from
1e6. Through the parents these cover more signatures: Time + Time is Float addition, Angle × Float Float
multiplication, String → Array / Texture / Group / 3D Entity... the object string function, Boolean → Float the
Integer conversion, Get Name of a Mesh or an Array Object → String. The census (`script-dump --census`) lists every
signature met with its standing (the row that answers it and where the row comes from): of the 142 in the
community maps and the game, 114 have a function, 87 of them through C rows. The level-2 oracle
(`crates/script/tests/oracle_blocks.rs`, `op_table`) runs the C rows over values it can build (Integer, Float, Time,
Percentage, Boolean, String, Vector, 2D Vector, Box, objects by name) against the C: 66 of the 142 signatures are answered by rows
checked so. The entity, mesh and material rows are checked by unit tests only; `ops.rs` has a table of hand-computed values.

Read from the original's own code (rows with source `dll`). `ParameterOperations.dll` registers every operation with
`RegisterOperationType` (GUID + name) and `RegisterOperationFunction` (operation, output type, input types, function);
`ballance/re/ops_extract.py` and `ops_table.py` (outside the repository) decode the registrations out of the disassembly into `ballance/re/ops.txt` (423 rows:
GUID, name, output, input 1, input 2, function address), and each function below was read in the disassembly. The
readings that came from the maps first were checked against it; two were wrong (`5e866012` is Contain String, not
Equal; `6ab94f5a` is Is Derived From, not Equal).

| GUID | Name | Signature and reading |
|---|---|---|
| `1ada7470` | Absolute Value | Integer (wrapping: the minimum stays), Float |
| `6ea214cb` / `7c2e01a8` | Min / Max | `b < a ? b : a` / `b > a ? b : a` (a NaN keeps input 1); the Box row of `6ea214cb` is the C's |
| `265c7d00` | Get Distance | Float ← Vector, Vector / 3D Entity, 3D Entity (world positions); Vector ← 3D Entity, 3D Entity is input 2's position minus input 1's |
| `5e866012` | Contain String | Boolean: input 1 contains input 2 (strstr; the DLL stores the pointer, so any hit is true; a missing string is false) |
| `6ab94f5a` | Is Derived From | Boolean: the class of object 1 is, or derives from, the class of object 2 (`CKIsChildClassOf`) |
| `6c62476b` | Equal | strings (strcmp, case matters), Integer, Boolean, Object (the ids); Float and Vector, Euler, 2D Vector, Quaternion, Rectangle, Color, Matrix bit for bit (`repz cmps`: 0.0 and -0.0 differ); Integer with Float numerically |
| `7c27550b` | Inverse | Matrix (and Vector, 2D Vector, Float, Color) |
| `014405eb` | Xor | Integer `a ^ b`, Boolean on the truth values |
| `0a547387` | Is Visible | Boolean ← 3D / 2D Entity (the object's own visible flag) |
| `0dd635d5` | Get Children | Object Array ← 3D Entity (at most 4096, paid from the tick's work) |
| `5bf9718f` | Get Parent | 3D / 2D Entity ← itself; nothing written for an object without a parent |
| `5a4e13fe` / `37f9040e` | Get Width / Get Height | Integer ← nothing (the render area; the game says 640 x 480) or ← Texture |
| `48c10000` | Set Alpha | Color ← Color, Float: alpha = the number, at most 1 (no lower bound) |
| `49c100e0` | Degre To Radian | Float ← Float: `x * 0.017453292` |
| `748c3502` | Division | Quaternion: VxMath's `Vx3DQuaternionDivide(P, Q)` = conj(Q) * P |
| `250020d4` | Get Script | Script ← Behavioral Object, String: the first script whose name contains the string (strstr) |
| `6b8c78bd` | Get Material | also of `23c8061b` (read as the 3D Sprite): the first material of its mesh |

Not done: Get Width / Get Height of a 2D Entity (the host has no 2D entity size), the object-array Equal, and the mixed
Integer / Float forms of Min, Max and Division. The census now lists no unknown operation in the maps.
