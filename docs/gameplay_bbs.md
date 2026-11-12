# Gameplay Building Blocks

This document gives the semantics of the Building Blocks a running level reaches that are not covered by
their own documents (see `docs/sky.md`, `docs/particles.md`, `docs/physics.md`). The GUIDs, DLLs and
function addresses come from `re/bb_map.txt`. The pins and defaults come from `tools/bbview.py`. The
behaviour comes from disassembly of the original DLLs (`data/BuildingBlocks/`, and `CK2_3D.dll` in
`data/RenderEngines/` for the engine helpers). Where the Ghidra decompile disagrees with the
disassembly, the disassembly wins.

The game values were probed with `ck_load` + `ck_param_resolve` over `base.cmo`, `3D Entities/*.nmo`,
`3D Entities/Level/*.NMO` and `3D Entities/PH/*.nmo`, matching behaviours by prototype GUID (76
instances). Wiring comes from `re/graphs/*.txt`.

Conventions:
- "Return 0" means `CKBR_OK`: the BB does not stay active.
- "Return `CKBR_ACTIVATENEXTFRAME`" (bit 0 set) means it runs again next frame.
- Vtable slots are derived as in `docs/sky.md` §9, unless a section says they were checked against the
  CK2_3D vtables.
- `rand()` is msvcrt `rand()` (`ck_rand` in our runtime).

| BB | DLL | GUID | Uses | Section |
|---|---|---|---|---|
| TT Extra | TT_Gravity_RT | `36106bd9:51813906` | 1 | [link](#tt-extra) |
| TT Simple Shadow | TT_Gravity_RT | `7f0517c2:52cc76bb` | 1 | [link](#tt-simple-shadow) |
| TT_TextureSine | TT_Gravity_RT | `009c1208:3a8d779e` | 1 | [link](#tt_texturesine) |
| TT Set Dynamic Position | TT_Toolbox_RT | `0fd4755f:7de22dc8` | 4 | [link](#tt-set-dynamic-position) |
| TT_LinearVolume | TT_Toolbox_RT | `09b335b3:12d17cdc` | 3 | [link](#tt_linearvolume) |
| TT_Timer | TT_Toolbox_RT | `6ac67901:7d2a6059` | 1 | [link](#tt_timer) |
| TT LookAt | TT_Toolbox_RT | `3d4861f8:2861703d` | 6 | [link](#tt-lookat) |
| TT ConvertPixel-Homogen | TT_Toolbox_RT | `18f96977:18e20f83` | 8 | [link](#tt-convertpixel-homogen) |
| TT_SplitString | TT_Toolbox_RT | `5ae74e2f:799c49b5` | 1 | [link](#tt_splitstring) |
| TT_PreloadTextures | TT_Toolbox_RT | `416b4f0e:192d339a` | 1 | [link](#tt_preloadtextures) |
| TT_FlushTextures | TT_Toolbox_RT | `734f73b4:01fc79ac` | 1 | [link](#tt_flushtextures) |
| Ray Intersection | Logics | `671e4a87:383b2912` | 2 | [link](#ray-intersection) |
| Box Box Intersection | Collisions | `64154401:76cf37af` | 2 | [link](#box-box-intersection) |
| Planar Filter | Visuals | `00cd320b:32ed010b` | 1 | [link](#planar-filter) |
| Get Nearest In Group | Logics | `085207eb:584950d8` | 1 | [link](#get-nearest-in-group) |
| Set Parent | 3DTransfo | `9d9d9d98:7e7a7f75` | 7 | [link](#set-parent) |
| Streaming Event | Logics | `1f0b52bf:4c3342dd` | 8 | [link](#streaming-event) |
| Per Second | Logics | `448e54ce:75a655c5` | 4 | [link](#per-second) |
| Timer | Logics | `a2a5a63a:e4e7e8e5` | 8 | [link](#timer) |
| Random Switch | Logics | `79d72fde:2e9d0912` | 2 | [link](#random-switch) |
| Remove Row | Logics | `1fa57136:14310857` | 4 | [link](#remove-row) |
| Objects With Attribute Iterator | Logics | `6bc1494c:0c816ad3` | 1 | [link](#objects-with-attribute-iterator) |
| Fill Group By Class | Logics | `4445257b:70016c57` | 4 | [link](#fill-group-by-class) |
| Load String | Logics | `391555d6:42f2500e` | 1 | [link](#load-string) |
| Texture Scroller | Materials | `f11d010a:fb1d010a` | 2 | [link](#texture-scroller) |
| Set Current Slot | Materials | `aaaa213a:eaa8d52a` | 1 | [link](#set-current-slot) |

# Part 1. TT_Gravity_RT.dll

## TT Extra

`TT Extra`, GUID `36106bd9:51813906`, `TT_Gravity_RT.dll` (base `0x10000000`), category `TT_Gravity`.
It drives the "extra points" pickup (`P_Extra_Point.nmo`): six small glowing balls orbit a centre
billboard; when the player ball comes close they burst outward, then home in on the ball one by one,
each touch counting as a hit.

| Function | Address |
|---|---|
| prototype creation | `0x100012a0` |
| execute | `0x10001640` |
| behavior callback | `0x10001890` (all messages) |
| find child by name suffix | `0x100019d0` |
| initialize | `0x10001ac0` |
| release (list + flags) | `0x10001ec0` -> `0x10001f10` |
| show / activate ("On") | `0x10002030` |
| hide ("Off") | `0x10002160` |
| state dispatcher | `0x10002310` |
| squared distance of two `VxVector` passed by value | `0x10002360` |
| squared distance entity-position to vector (`-1` for NULL entity) | `0x10002390` |
| state 0: orbit / wait for ball | `0x10002400` |
| state 1: fly away | `0x10002870` |
| state 2: home in / hit | `0x10002a50` |

The Ghidra decompile of every helper is garbled (stack slots merged, wrong registers); everything below
is from the disassembly.

### Prototype

Inputs `0 On`, `1 Off`, `2 Initialize`. Outputs `0 Activated`, `1 Hit`, `2 Ready`.
Behavior flags `0x40000` (targetable), `SetFlags(1)`.

| # | Input parameter | Type | Default | Game (`P_Extra_Point`) | Meaning |
|---|---|---|---|---|---|
| 0 | `Ball` | 3D Entity | - | runtime (`Ball_Pos_Frame` from the CurrentLevel array) | the player ball |
| 1 | `Number of Smallballs` | Integer | 6 | 6 | N, clamped to >= 1. At most 8 (name table size). |
| 2 | `Activationdistance` | Float | 2 | 3 | burst radius (compared squared) |
| 3 | `Extra_Points CollDistance` | Float | 2 | 4 | hit threshold, compared against the **squared** distance (so 4 means 2 units) |
| 4 | `Rotationspeed` | Float | 5 | 5 | orbit speed, rad/s |
| 5 | `Awayforce` | Float | 50 | 1 | fly-away push |
| 6 | `Awaydamping` | Float | 0.5 | 0.3 | fly-away velocity retention |
| 7 | `Force` | Float | 0.06 | 0.12 | homing gain of ball 0 |
| 8 | `Damping` | Float | 0.9 | 0.95 | homing velocity retention |
| 9 | `Forcewidth` | Float | 0.06 | 0.08 | spread of the homing gain over the N balls |
| 10 | `Flyawaytime` | Float | `1000.0` | 1000 | fly-away duration, ms |
| 11 | `Hitframegroup` | Group | - | `P_Extra_Point_Hitframes` | frames placed at each hit |
| 12 | `Flying Extra?` | Boolean | FALSE | from a `Parameter Selector` | TRUE: no floor glow |
| 13 | `Exactness Framedelay` | Integer | 2 | 2 | only exists while setting `Exactness?` is TRUE |

Output parameter `0 Current Hits` (Integer).

Locals: `0 Activationstatus` (bool), `1 Smallballlist` (raw pointer, 4 bytes; the list below),
`2 Shadowobject` (3D entity: the floor glow), `3 Centerobject` (3D entity: the centre billboard),
`4 Status` (int, default -1), `5 Initialized` (bool), `6 Timecounter` (float ms), `7 Ballcounter` (int),
`8 Timevalue` (float s), `9 Next Check` (int), `10 Away Positionsave` (vector).
Setting (local 11): `Exactness?` (bool, default TRUE; game TRUE).

List node (`new(0x1c)`, singly linked, in child order):

| Offset | Field |
|---|---|
| +0x00 | next |
| +0x04 | the small ball (a `CKSprite3D`, class 37) |
| +0x08 | saved position (world), 12 bytes: the previous-frame position used as velocity memory |
| +0x14 | homing gain `g_i` |
| +0x18 | byte alive (1 until hit) |

### Hierarchy lookup (`0x100019d0`)

`find(offset, key, entity)`: for each child `c` of `entity` (`GetChildrenCount` +0x88, `GetChild` +0x8c)
copy `strlen(key)` characters of `c`'s name starting at `offset` and `strcmp` them with `key`; the
first match wins. The copy is `new`-ed and never freed (a leak per call; harmless).

With `P_Extra_Point_MF` as target (the owner; `P_Extra_Point_` is 14 characters, `P_Extra_Point_Ball`
18):
- offset 14 `"Floor"` -> `P_Extra_Point_Floor` (a `CK3dObject` with the floor-glow mesh),
- offset 14 `"Ball0"` -> `P_Extra_Point_Ball0` (the centre sprite),
- offset 18 `"1"`..`"8"` (table `0x10012064`, index i gives `"i+1"`) -> `P_Extra_Point_Ball1..6`.

Each sprite has one child frame `P_Extra_Point_FrameK` carrying a script: frames 1..6 run a
`TT_TimedependentPointParticlesystem` (the trails), frame 0 a `Point Particle System` (the burst). The
hit frames `P_Extra_Point_Ball_HitFrame0x` (group order 01, 02, 04, 03, 05, 06) run a `Point Particle
System` each.

### Execute (`0x10001640`)

`dt` below is `behcontext.DeltaTime` (ms, offset +4).

1. **Off** active: clear it, `Activationstatus = FALSE`, run *hide* (`0x10002160`), **return 0**
   (the BB deactivates). Nothing else that frame.
2. Read `Initialized`.
3. **On** active: clear it.
   - If not initialized: console `"Extra is'nt intialized yet, this could slow all down."`, run
     *initialize*; on failure console `"Could'nt intialize the Extra."` and return `0xa008`.
   - `Activationstatus = TRUE`; `Timevalue = dt * 0.001`.
   - If `Exactness?`: `Next Check = pin 13`.
   - Run *show* (`0x10002030`); failure -> console `"Could'nt activate the Extra."`, return `0xa008`.
   - Fall through to step 5 with active = TRUE.
4. Else **Initialize** active: clear it. If already initialized go to step 5 with active = FALSE
   (so it just returns 1). Otherwise run *initialize*: success -> **return 0**, failure -> console
   message, return `0xa008`.
   Else (no input): active = `Activationstatus`.
5. If active and initialized:
   - If `Exactness?`: `Next Check -= 1`; if it reached 0: `Timevalue = dt * 0.001`,
     `Next Check = pin 13`. Store `Next Check`. So `Timevalue` is the frame time sampled once every
     `Framedelay` frames (game: every 2nd frame) and held in between. With `Exactness?` FALSE it keeps
     the value of the On frame forever.
   - Run the state for `Status` (0, 1, 2; any other value does nothing). A non-zero result: console
     `"Could'nt execute the Extra."`, return `0xa008`.
6. **Return 1** (stay active), also while inactive.

`0xa008` / `0xa004` have bit 0 clear, so the BB is not re-activated by the return value.

### Initialize (`0x10001ac0`)

```
scene = ctx.GetCurrentScene()
if Status == -1:  list = NULL                    (a stale pointer is simply dropped)
elif Status == 1: release(); on error return 0xa008
elif list != NULL: return 0                      (already built)
N = max(1, pin1); Ballcounter = N
t = GetTarget(); if !t return 0xa004
flying = pin12
floor  = find(14,"Floor",t); center = find(14,"Ball0",t)
errors: (!flying && !floor) -> "There is no Floorglowobject in hirachy!";
        !center -> "There is no Centerbillboard in hirachy!"; both return 0xa008
Shadowobject = floor; Centerobject = center; scene NULL -> 0xa008
if floor: scene.DeActivate(floor); floor.Show(HIDE)
s = center.GetChild(0).GetScript(0); if s: scene.DeActivate(s) else console "1Script not found %s"
center.Show(HIDE)
step = pin9 / N;  acc = pin7 - step
for i in 0..N-1:
    b = find(18, str(i+1), t)       missing -> "Missing one or more Billboards." return 0xa008
    scene.DeActivate(b.GetChild(0).GetScript(0)); b.Show(HIDE)
    node = {next NULL, b, saved = b.GetPosition(world), alive 1, g = (acc += step)}
    b.GetChild(0) missing -> "There is no Frame at one of the Billboards." 0xa008
    its script missing -> "There is no Script at one of the Billboardframes." 0xa008
    script.Activate(FALSE, FALSE)   (CKBehavior::Activate: marks it inactive)
    append node
Smallballlist = head; Timecounter = pin10; Initialized = TRUE; Status = 0
return 0
```

So `g_i = Force + i * Forcewidth / N`. Game: 0.12, 0.1333, 0.1467, 0.16, 0.1733, 0.1867. On an error
part way through, the nodes built so far are leaked and the list is not stored.

### Show (`0x10002030`) and hide (`0x10002160`)

Both need a list and a current scene (else `0xa008`, ignored by execute).

- Show: `center.Show(SHOW)`; if floor and not flying `floor.Show(SHOW)`. For each node:
  `scene.Activate(frame.GetScript(0), reset=TRUE)` (the "8Script not found" message if absent),
  `sprite.Show(SHOW)`, then the same `scene.Activate` again (duplicate, harmless). The centre frame's
  script is not touched.
- Hide: `scene.DeActivate(center.GetChild(0).GetScript(0))`, `center.Show(HIDE)`; floor hidden unless
  flying; for each node: deactivate the frame script (three times), `sprite.Show(HIDE)`.
  `Status` is left as it is.

### State 0: orbit and wait (`0x10002400`)

Target `t` and ball `B` (pin 0) required (`0xa004` / `0xa008`). `Bp = B.GetPosition(world)`.

If `|t.pos - Bp|^2 <= Activationdistance^2` (world position of the target, x87 compare, NaN counts as
inside), the extra fires:
1. `Status = 1`.
2. If not flying and floor: `floor.Show(HIDE)`.
3. Centre: deactivate its frame script, `center.Show(HIDE)`; then `f = center.GetChild(0)`:
   `f.Show(SHOW)` and `scene.Activate(f.GetScript(0), reset=TRUE)`: the burst particle system starts
   at the centre.
4. For every node: `saved = sprite.GetPosition(world)`.
5. `Current Hits = 1`; **activate output 0 `Activated`**.
6. `Away Positionsave = (Bp.x, Bp.y - 2.1, Bp.z)`.
7. Return 0 (no orbit step this frame).

Otherwise every node orbits: `angle = Rotationspeed * Timevalue` (radians). The axis depends on the
node index i (jump table `0x10002854`):

| i | axis |
|---|---|
| 0 | (0, 1, 0) |
| 1 | (0, 0, 1) |
| 2 | (1, 0, 0) |
| 3 | (0.5, 0.5, 0.707) |
| 4 | (0.707, -0.707, 0) |
| 5 | (-0.5, -0.5, 0.707) |
| >5 | keeps axis 5 |

(0.707 is `0x3f34fdf4` = 0.70700002; the axes are not normalized exactly, `Vx3DMatrixFromRotation`
builds the matrix from the vector as given.)

```
p  = sprite.GetPosition(ref = t)                 (MF-local)
M  = Vx3DMatrixFromRotation(axis_i, angle)
p' = Vx3DRotateVector(M, p)
L  = |p'|; if L != 2.0: p' = p' * (2 / L)        (radius forced to 2 in MF-local units)
sprite.SetPosition(p', ref = t, keepChildren = FALSE)
```

Return 0.

### State 1: fly away (`0x10002870`)

```
A = Away Positionsave;  A.y -= 3.0                (so A = ball position at firing - (0, 5.1, 0))
Timecounter -= dt
if Timecounter <= 0: Status = 2; return 0          (Timecounter is not stored in this case)
store Timecounter
k = Awayforce * Timevalue;  d = Awaydamping
for each node (alive or not):
    O = node.saved;  P = sprite.GetPosition(world);  node.saved = P
    N = P + d * (P - O) + k * (P - A)
    sprite.SetPosition(N, world, keepChildren = FALSE)
```

`Timecounter` is only reset by *initialize*, so the fly-away phase happens once per Initialize.
Game: 1000 ms, `k = 1 * dt_s`, `d = 0.3`.

### State 2: home in and hit (`0x10002a50`)

```
if Ballcounter <= 0: Status = 3; activate output 2 "Ready"; return 0
Bp = Ball.GetPosition(world);  damp = pin8;  hits = 0
for each node, index i counting all nodes:
    if !alive: continue
    P = sprite.GetPosition(world)
    if |Bp - P|^2 <= CollDistance:                  (pin 3, default 2.0 if unreadable; NOT squared)
        hits++; alive = 0; Ballcounter -= 1 (stored)
        deactivate the frame script (twice); sprite.Show(HIDE)
        hf = Hitframegroup.GetObject(i)
        if hf: hf.SetPosition(sprite.GetPosition(world), world, FALSE)
               scene.Activate(hf.GetScript(0), reset=TRUE)
    else:
        O = node.saved; node.saved = P
        N = P + (Bp - P) * g_i * Timevalue + (P - O) * damp
        sprite.SetPosition(N, world, FALSE)
if hits > 0: Current Hits = hits (this frame's count); activate output 1 "Hit"
return 0
```

State 3 does nothing (execute keeps returning 1). The game wires `Ready -> TT Extra.Off`, and
`Activated -> TT Scaleable Proximity.Off`, so On never fires again for the same extra.

### Callback (`0x10001890`)

| Message | Action |
|---|---|
| 2 DELETE, 4 DETACH, 9 RESET, 17 DEACTIVATESCRIPT | if `Initialized`: *release* (deactivate all frame scripts, hide every sprite, free the nodes, list = NULL), `Initialized = FALSE`, `Status = -1`. Return 0. |
| 3 ATTACH, 11 LOAD | `Smallballlist = NULL` (4 bytes), `Initialized = FALSE`, `Activationstatus = FALSE`. `Status` is only read, not reset (the file stores -1). |
| 13 SETTINGSEDITED | `Exactness?` TRUE and fewer than 14 inputs: create `Exactness Framedelay` (Integer). FALSE and 14 inputs: remove and destroy input 13. |

### Game wiring (`P_Extra_Point_MF Script`)

A `Ray Intersection` from `(0,-2,0)` along `(0,-1,0)` in the MF frame, depth 1.5, feeds a
`Parameter Selector` (True -> 0, False -> 1) whose value is `Flying Extra?`; its Out fires
`Initialize` and, one frame later, a `TT Scaleable Proximity` (distance 80) whose EnterRange/ExitRange
drive On/Off. `Hit` and `Activated` send messages (score), `Binary Switch.False` (extra already taken,
from an array cell) drives Off.

Points, measured in the C build with an extra point moved onto the ball on level 1: 100 when it fires and 20 for
each small ball that reaches the ball (220 in all); no hit in the first 60 frames (the fly-away), five by 120, all
six by 200. The port runs this block in `crates/game/src/extra.rs`, the small balls starting at the module's
`P_Extra_Point_Frame1..6`, and gives the same.

### Engine calls

| Class | Offset | Method |
|---|---|---|
| CKObject | +0x00 | Show(option) (0 hide, 1 show) |
| CK3dEntity | +0x88 / +0x8c | GetChildrenCount / GetChild(i) |
| CK3dEntity | +0x124 | SetPosition(const VxVector*, ref, keepChildren) |
| CK3dEntity | +0x128 | GetPosition(VxVector*, ref) |

Imports: `CKScene::Activate(obj, reset)` / `DeActivate`, `CKBeObject::GetScript(0)`,
`CKBehavior::Activate`, `CKGroup::GetObject`, `Vx3DMatrixFromRotation` (`0x100101f4`),
`Vx3DRotateVector` (`0x100101f8`). Constants: `0x10010210` 0.001, `0x10010214` -1.0, `0x10010218` 1.0,
`0x1001021c` 2.0, `0x10010220` 2.1, `0x10010224` 0.0, `0x10010228` 3.0.

## TT Simple Shadow

`TT Simple Shadow`, GUID `7f0517c2:52cc76bb`, `TT_Gravity_RT.dll`. The blob shadow under the player
ball. It is **not** a mesh or a projected quad: it adds an extra **material channel** to the meshes of
the "floor" objects below the target, writes planar-projected UVs for that channel every frame, and
lets the renderer multiply the frame buffer by the shadow texture.

| Function | Address |
|---|---|
| prototype creation | `0x10004350` |
| execute | `0x10004450` |
| pre-render callback (render context) | `0x10004780` |
| behavior callback | `0x10004bd0` |
| receiver search | `0x10004c10` |
| remove channels | `0x10004eb0` |

### Prototype

Inputs `0 On`, `1 Off`; outputs `0 Exit On`, `1 Exit Off`. Flags `0x40000` (targetable), `SetFlags(1)`.

| # | Input parameter | Type | Default | Game (`Balls.nmo`, script `Ball_Shadow`) |
|---|---|---|---|---|
| 0 | `Texture` | Texture | NULL | `HardShadow` |
| 1 | `Size Scale` | Float | `2.0` | 1.3 |
| 2 | `Maximum Height` | Float | `100.0` | 20 |

Local 0 `Data`, raw pointer (4 bytes) to a `new(0xd8)` block:

| Offset | Field |
|---|---|
| +0x00 | CK_ID of the shadow material |
| +0x04 | CK_ID of the texture currently set (init -1) |
| +0x08 | size scale (init 2.0; rewritten from pin 1 every frame) |
| +0x0c | CK_ID receivers[50] |
| +0xd4 | receiver count |

The target is the ball (set at runtime through the target parameter; statically unresolved in the file).
Wiring: `Ball_Shadow.Start -> On` (delay 1); a `Get Cell`/`Test` loop and a `Switch On Message`
(two messages) fire `Off`.

### Execute (`0x10004450`)

`rc = behcontext.CurrentRenderContext` (+0x18). No target -> `0xa004`.

- **Off**: clear, activate output 1 `Exit Off`. If `Data` exists: find the material by name
  `"TT_SimpleShadow Material"` (class 30); `GetObject(Data.mat)` NULL -> return `0xa008`. If they are
  the same object: remove its channel from every receiver (below), `mat.SetTexture0(NULL)`,
  `DestroyObject(mat)`. `Data = NULL` (the block is leaked). **Return 0.**
- **On**: clear, activate output 0 `Exit On`.
  1. If a material named `"TT_SimpleShadow Material"` exists: `SetTexture0(NULL)` and destroy it.
  2. `mat = CreateObject(CKCID_MATERIAL, "TT_SimpleShadow Material", options)`; options is
     `CK_OBJECTCREATION_DYNAMIC` (0x10) when the behavior's object flags contain both bits of `0x108`,
     else 0.
  3. `SetEmissive(1,1,1,1)` (+0x78), `SetDiffuse(1,1,1,1)` (+0x68), `SetSpecular(0,0,0,1)` (+0x70),
     `SetAmbient(0,0,0,1)` (+0x60), `SetTextureAddressMode(3 = CLAMP)` (+0xa0),
     `SetTextureBlendMode(7 = COPY)` (+0x88). The texture is not set here.
  4. If an old `Data` exists: "remove channels" for its receivers, but using the **new** material, so
     nothing is found (a bug; the old channels point at the destroyed material). The old block leaks.
  5. New `Data = {mat id, -1, 2.0, zeros, count 0}`.
- Always (also with no input): `rc.AddPreRenderCallBack(0x10004780, behavior, temporary=TRUE)`
  (+0x80), **return 1**. A temporary callback runs once and is dropped, so it is re-armed every frame
  while the BB is active.

### Pre-render callback (`0x10004780`)

Called as `(CKRenderContext*, CKBehavior*)`, before the scene is drawn.

```
t = beh.GetTarget(); d = Data; mat = GetObject(d.mat)
old = copy of d.receivers[0..d.count)                 (CKMemoryPool)
tex = pin0; if tex != d.tex: d.tex = tex; mat.SetTexture0(GetObject(tex))
d.scale = pin1 (default 2.0);  H = pin2 (default 100.0)
Lt = t.GetBoundingBox(local = TRUE)                    (VxBbox is {Max, Min})
k  = 1 / ((Lt.Max.x - Lt.Min.x) * |row0 of t.GetWorldMatrix()| * d.scale)
search receivers -> d.receivers, d.count, P[i], S[i]   (0x10004c10, below)
for id in old not in d.receivers:                      (left the shadow)
    e = GetObject(id); if 3D entity and mesh = e.GetCurrentMesh():
        c = mesh.GetChannelByMaterial(mat); if c >= 0: mesh.RemoveChannel(c)
for i in 0..d.count-1:
    e = GetObject(d.receivers[i]); must be a 3D entity with a current mesh
    c = mesh.GetChannelByMaterial(mat)
    if c < 0: c = mesh.AddChannel(mat, CopySrcUv = FALSE)
              mesh.SetChannelSourceBlend(c, VXBLEND_ZERO = 1)
              mesh.SetChannelDestBlend(c, VXBLEND_SRCCOLOR = 3)
    uv  = mesh.GetTextureCoordinatesPtr(&uvStride, c)
    pos = mesh.GetPositionsPtr(&posStride)             (mesh-local vertex positions)
    a = k * S[i].x;  b = k * S[i].z
    for every vertex v:  uv.u = 0.5 + a * (P[i].x - v.x)
                         uv.v = 0.5 + b * (P[i].z - v.z)
    mesh.UVChanged()
Data = d  (rewritten)
```

So the shadow is a square of world side `ballWidth * SizeScale` (ball width = local bbox X extent times
the world X scale; about 2 * 1.3 for the game), centred on the ball's XZ position, projected straight
down (along the receiver's local Y) onto every vertex of the receiver mesh. U grows toward the
receiver's local -X, V toward local -Z (mirrored; irrelevant for the round `HardShadow`). Outside
[0,1] the clamp address mode repeats the border texel, so the texture must have a white border.
`S[i]` makes the mapping come out in world-sized units for scaled receivers; a rotated receiver (about
Y) rotates the square with it.

Rendering the channel: the receiver is drawn normally, then the channel pass draws the same faces with
the channel UVs, the shadow material (texture COPY, i.e. the texel colour) and blending
`src * 0 + dst * srcColor`: the floor is multiplied by the texture (white = no change). The channel is
lit or unlit per the `AddChannel` default (not changed here).

### Receiver search (`0x10004c10`)

Arguments `(Data, target, behavior, poolP, poolS, H)`.

```
T = t.GetBoundingBox(local = FALSE)                    (world AABB of the ball)
m = 1 / ((T.Max.x - T.Min.x) * d.scale)
list = AttributeManager.GetGlobalAttributeListPtr(FloorManager.GetFloorAttribute())
count = 0
for o in list:
    o.IsVisible() (+0x0c) and not o.IsAllOutsideFrustrum() (+0xd8)   (last frame's culling result)
    F = o.GetBoundingBox(FALSE)
    F.Min.x <= T.Max.x and F.Min.z <= T.Max.z and F.Max.x >= T.Min.x and F.Max.z >= T.Min.z
    F.Min.y <= T.Max.y and T.Min.y - F.Max.y <= H     (floor top at most H below the ball bottom)
    P = t.GetPosition(ref = o)                        (ball position in the floor's frame)
    L = o.GetBoundingBox(TRUE);  S = o.GetScale(local = TRUE)
    sx = S.x * m;  sz = S.z * m
    (L.Max.x - P.x) * sx >= -0.5 and (L.Min.x - P.x) * sx <= 0.5
    (L.Max.z - P.z) * sz >= -0.5 and (L.Min.z - P.z) * sz <= 0.5
    count < 50
    -> poolP[count] = P; poolS[count] = S; receivers[count] = o.id; count++
d.count = count
```

The floor manager is `Collisions.dll`'s (GUID `0x420936f9:0`); its vtable `0x2531a498` slot +0xac
(`0x25316d60`) returns the `"Floor"` attribute type it registered. The level files carry no `Floor`
attributes; `Levelinit.nmo`'s `set Floor` graph puts an attribute (file index 29, most likely `Floor`)
on every member of the level's `Shadow` group. So the shadow receivers are the `Shadow` group (18
floor objects in Level_01).

### Callback (`0x10004bd0`)

5 PAUSE: `rc.RemovePreRenderCallBack(0x10004780, beh)` (+0x84). 6 RESUME:
`rc.AddPreRenderCallBack(0x10004780, beh, TRUE)`. Everything else: nothing. RESET/DELETE do not
remove the channels; only Off does.

### Engine calls

| Class | Offset | Method | Confidence |
|---|---|---|---|
| CKObject | +0x0c | IsVisible | high |
| CKRenderContext | +0x80 / +0x84 | AddPreRenderCallBack(fn, arg, temporary) / RemovePreRenderCallBack | high (SDK order anchored on sky.md's +0xe0 GetViewRect) |
| CKMaterial | +0x60 / +0x68 / +0x70 / +0x78 | SetAmbient / SetDiffuse / SetSpecular / SetEmissive | medium-high (SDK order anchored on sky.md's +0x64 GetDiffuse) |
| CKMaterial | +0x7c / +0x84 | GetTexture(0) / SetTexture0 | medium-high |
| CKMaterial | +0x88 / +0xa0 | SetTextureBlendMode / SetTextureAddressMode | medium-high |
| CK3dEntity | +0xd8 | IsAllOutsideFrustrum (CK2_3D `0x10005d22`: flag bit 2 set by IsInViewFrustrum +0xc8) | high |
| CK3dEntity | +0xec | GetCurrentMesh | high |
| CK3dEntity | +0x128 | GetPosition(VxVector*, ref) | high |
| CK3dEntity | +0x144 | GetScale(VxVector*, local) (CK2_3D `0x10007b06`) | high |
| CK3dEntity | +0x174 | GetWorldMatrix | high |
| CK3dEntity | +0x1c0 | GetBoundingBox(local) (CK2_3D `0x10009144`) | high |
| CKMesh | +0xb4 | GetPositionsPtr(DWORD* stride) (`0x1001c236`) | high |
| CKMesh | +0xb8 | GetTextureCoordinatesPtr(DWORD* stride, channel) (`0x1001c597`; -1 = base UVs) | high |
| CKMesh | +0xe4 | UVChanged (`0x1001e14e`) | high |
| CKMesh | +0x16c / +0x170 / +0x178 | AddChannel(mat, copyUv) / RemoveChannel(index) / GetChannelByMaterial | high |
| CKMesh | +0x1a8 / +0x1ac | SetChannelSourceBlend / SetChannelDestBlend (write channel +8 / +0xc) | high |

The CKMesh slots were checked in CK2_3D's mesh vtable `0x10084c28` (+0x168 GetChannelCount,
+0x174 RemoveChannelByMaterial, +0x17c..+0x1a4 the other channel accessors, +0x1b0 BuildNormals).

## TT_TextureSine

`TT_TextureSine`, GUID `009c1208:3a8d779e`, `TT_Gravity_RT.dll`, compatible class 0x20 (mesh), author
"Virtools". Execute `0x10006910`, callback `0x10006af0`. One input `In`, one output `Out`. It wobbles a
mesh's UVs around (0.5, 0.5).

Pins: `0 X Amplitude` (Float, 0.1), `1 Y Amplitude` (Float, 0.1), `2 Velocity` (Float, 1),
`3 Channel` (Integer, -1). Locals: 0 = reference UVs (raw buffer, 8 bytes per vertex), 1 = phase
(float seconds). Game: one use, `P_Extra_Life.nmo` (`P_Extra_Life Anim Script`, target the extra-life
mesh): 0.2, 0.2, 1, -1; the saved reference buffer has 536 bytes = 67 UVs. It is fired by
`Scale.Out` inside a per-frame `Bezier Progression` loop, so it runs once per frame while that loop
runs.

Execute (runs once per activation):
```
mesh = target;  n_ch = mesh.GetChannelCount()          (+0x168)
if ch < -1 or ch >= n_ch: clear In, activate Out, return 0xa008
uv = mesh.GetModifierUVs(&stride, ch)                  (+0x80 -> GetTextureCoordinatesPtr)
n  = mesh.GetModifierUVCount(ch)                       (+0x84 -> vertex count)
for i in 0..n-1:
    ang = 4 * (i / n - 0.5) + phase * Velocity
    uv[i].u = ref[i].u + (0.5 - ref[i].u) * cos(ang) * XAmp
    uv[i].v = ref[i].v + (0.5 - ref[i].v) * sin(ang) * YAmp
mesh.ModifierUVMove()                                  (+0x88 -> UVChanged)
phase += dt_ms * 0.001
if phase * Velocity > 2*pi: phase -= 2*pi / Velocity
clear In; activate Out; return 0
```
The reference is always the base UV set (channel -1), even when `Channel` names another channel.

Callback: 3 ATTACH copies the base UVs (channel -1) into local 0; 4 DETACH writes them back and calls
`ModifierUVMove`. A port with file-loaded locals just uses the saved buffer. Constants: `0x10010250`
0.5, `0x10010274` 4.0, `0x10010260` 2*pi, `0x10010210` 0.001.

# Part 2. TT_Toolbox_RT.dll

All from `TT_Toolbox_RT.dll`, image base `0x10000000`, author Terratools, version `0x10000`. The
`fn=` addresses in `re/bb_map.txt` (`0x100015eb`, `0x100016bd`, ...) are 5-byte `jmp` thunks. The real
bodies are listed below. `BehaviorContext` offsets used: `+0x04` DeltaTime (ms, float), `+0x08`
CKContext, `+0x18` current render context.

Common CK3dEntity vtable slots (same derivation as docs/sky.md §9; they match the argument pushes):

| Offset | Method |
|---|---|
| +0x108 | Rotate3f(x, y, z, angle, ref, keepChildren) |
| +0x124 | SetPosition(const VxVector*, ref, keepChildren) |
| +0x128 | GetPosition(VxVector*, ref) |
| +0x12c | SetOrientation(dir, up, right, ref, keepChildren) |
| +0x130 | GetOrientation(dir, up, right, ref) (NULL outputs are skipped) |
| +0x140 / +0x144 | SetScale(const VxVector*, keepChildren, local) / GetScale(VxVector*, local) |
| +0x17c | Transform(VxVector *dst, const VxVector *src, ref) (from ref's local frame to world) |

## TT Set Dynamic Position

GUID `0fd4755f:7de22dc8`. Declaration `0x100046d0`, prototype `0x10004770`, execute `0x10004a80`
(thunk `0x100015eb`). No callback. Compatible class `0x21` (CK3dEntity). Behavior flags `0x40000`
(targetable). The target is a 3D entity. Description: "Follows an 3D Object smoothly".

This is a damped spring. Each frame it moves the target a fraction of the way toward another
object (plus an offset). Both positions are measured in an optional reference frame.

Inputs: `0 On`, `1 Off`. Outputs: `0 On`, `1 Off`.

| # | Input param | Type | Default | Meaning |
|---|---|---|---|---|
| 0 | `Object` | 3D Entity | NULL | object to follow (T = its position) |
| 1-3 | `Force X/Y/Z` | Float | 1.0 | spring gain per axis, per second |
| 4-6 | `Damping X/Y/Z` | Float | 0.0 | fraction of last frame's displacement kept (per frame, not scaled by dt) |
| 7-9 | `Offset X/Y/Z` | Float | 0.0 | **subtracted** from the target point |
| 10 | `Coordinate System` (file name `Coordinate-System`) | 3D Entity | NULL | reference frame for every Get/SetPosition |
| 11 | `MaxDistance2Target` | Float | 0.0 | clamp (buggy, see below). Only used if > 0 |

Outputs: `0 OldPosition`, `1 DeltaTarget`, `2 NewPosition` (Vector), `3 fDist` (Float). Only output 0
is ever written, and it receives the **new** position. Outputs 1-3 are never written.

Locals: `0 Status` (Bool, default FALSE; only its low byte is written and tested; the upper three bytes
are stack garbage, which is why saved files show values like -5.15e-14). `1 Old Position` (Vector).

Execute, in order (R = pin 10 entity or NULL):
1. If `Off` (in 1) is active: clear it, write Status = 0, then fall through to the update (step 3).
2. Else if `On` (in 0) is active: clear it, **activate output 0 now**, and write Status = 1.
   Then, if the target is NULL, return `0xa004`. Otherwise `OldPos = target.GetPosition(R)` (+0x128),
   store it in local 1, and **return 1 without moving**.
   Else (neither input): read Status from local 0.
3. Update: target NULL → return `0xa004`. `Object` NULL → return `0xa004`. Neither activates an
   output. `0xa004` has bit 0 clear, so the BB deactivates.
   - `T = Object.GetPosition(R)`. `P0 = local 1`. `X = target.GetPosition(R)`. Then **local 1 = X**
     (stored before the move).
   - `V = X - P0` (last frame's displacement). `Dl = T - X`.
   - `k = DeltaTime * 0.001` (seconds; constant `0x10040024`).
   - Per axis `a`: `N.a = X.a + V.a*Damping.a + (Dl.a - Offset.a)*Force.a*k`.
   - `Rv = T - N`, `dist = |Rv|` (0 if Rv is exactly zero).
   - Clamp only if `MaxDist > 0.0` (double compare against `0x10040028`) and `dist > MaxDist`. The
     code then copies Rv and calls `VxVector::Normalize` **only when it equals zero** (inverted test).
     So normally it is not normalized, and `N = T - Rv*MaxDist`. That is the right answer only if
     MaxDist is 1. The game never enables the clamp (all instances have 0).
   - `target.SetPosition(&N, R, keepChildren=FALSE)` (+0x124).
   - `SetOutputParameterValue(0, &N)`.
   - If Status: return 1 (stay active). Otherwise activate output 1 (`Off`) and return 0.

Consequences: the first On frame only records the position. Off does one last spring step and then
fires `Off`. Offset is subtracted, so the aim point is `T - Offset`. Damping depends on frame rate;
force is dt-scaled.

Game usage (all have MaxDist 0, Coordinate-System NULL unless noted):

| File / parent graph | Object | Force | Damping | Offset | Notes |
|---|---|---|---|---|---|
| Gameplay `dephysic Ball` (BB #528) | `Trafo` (local) | 2,2,2 | 0.7 each | 0,-3,0 | Coordinate-System = the same `Trafo`, so T = 0 and the ball is pulled to Trafo-local (0,+3,0). Physicalize.Out2 → On; On → Delayer(1350 ms) → Off |
| Gameplay root (BB #2989) | `Cam_Pos Frame` | 5, 0.8, 5 | 0.5, 0.3, 0.5 | 0 | camera-position follower |
| Gameplay root (BB #3025) | `BallPos_Frame` | 10,10,10 | 0 | 0 | camera-target follower (pure lag, no inertia) |
| PH/PE_Balloon `UFO` | `PE_UFO_TargetPosRef` | 1,1,1 | 0.7 each | 0 | Show.Out → On (delay 1) |

In Gameplay, `Init Ingame.Out 0 → On` (delay 1). `BallManager.Ball Off → Off`, and also → `On` with
delay 1. The two root instances are chained: Off → Off and On → On.

## TT_LinearVolume

GUID `09b335b3:12d17cdc`. Prototype `0x100214c0`, execute `0x10021600` (thunk `0x100016bd`). The
curve helper is `0x10021590` (thunk `0x10001339`). Compatible class `0x13`. The description string was
copy-pasted ("plays different samples according...").

Input `In`, output `Out`. Pin `0 Vol. in Decibel` (Float, default `0`). Output `0 Linear Volume` (Float).
No locals.

Execute: clear In. Read v (initialised to 0 before GetInputParameterValue). Compute:

    f(v) = 1.0                  if v > 1.0
         = 0.0                  if v <= 0.01     (double 0.01 at 0x100400d0)
         = pow(50.0, v) * 0.02  otherwise        (= 50^(v-1); _CIpow at 0x10028a40)

Write the result to output 0, activate Out, and return 0. Despite the pin name, the input is a
normalised 0..1 level. The output is a perceptual curve: 1 → 1.0, 0.5 → 0.141, and just above 0.01
→ 0.0208 (there is a jump down to 0 at 0.01).

Game: `Sound.nmo` `HitSound Woodenflaps` (input = `PhysicsCollDetection` `Speed (0-1)`, output →
Play Sound Instance), `Sound.nmo` `Fade In Music` (input = a `MusicVolume` array cell), and
`Menu.nmo` `Volume`.

## Sounds (Sounds.dll and TT ProximityVolumeControl)

Read from the old C reference (`src/bb/bb_sounds.c`, `bb_tt_misc.c`; it follows the DLL functions listed there).
These blocks act on a *wave sound* object (class 25): the target, or in the old layout a first pin of type Wave Sound
(then every pin index shifts by one). Anything else as target: the block does nothing.

- **Wave Player** `5bde0e45:2e2107d5`. Inputs Play, Stop, Pause/Resume; outputs Start Playing, End Playing,
  Paused/Resumed; pins Fade In, Fade Out, Loop. Play: set the loop flag from the pin, play with the fade-in, fire
  Start Playing; a non-looping Play keeps the block active (next frame), a looping one returns at once. Pause/Resume:
  fires Paused/Resumed; a playing sound is paused and the block stays active, a paused one resumes. Stop: stop with
  the fade-out; with a fade of 0 fire End Playing and return. With no input active: End Playing once the sound is no
  longer playing, else stay active. (Our fade is not modelled: Stop ends the sound at once.)
- **Volume Control** `33613f21:776b185b` (pin Gain, default 1), **Pitch Control** `7b7c731e:175b27a3` (pin Pitch,
  default 1), **Panning Control** `6a5f17af:63480864` (pin Pan, default 0): clear In, fire Out, set the one value on
  the sound. Not limited to 1 (volume above 1 is allowed).
- **Play Sound Instance** `00283a35:38ef6b48`. Pins 2D, Object, Position, Direction, Min Delay, Volume (index 5, default
  1, also in the old layout). Clears In, fires Out, plays a one-shot copy of the sound at Volume. The C reference
  plays it 2D always; ours places it when 2D is off (object's position plus Position in its frame, else the world
  Position): our choice, the audio attenuates it linearly to nothing at 100 units from the ball.
- **TT ProximityVolumeControl** `38571b61:64cd2174`. Inputs On, Off; outputs On, Off; pins Object 1, Object 2, Near,
  Far, Sound; local Status. On: Status on, fire On. Off: Status off. Every execution: d2 = squared distance of object 2
  in object 1's frame, near2/far2 the squared pins; g = 0 if d2 is not below far2, 1 if d2 <= near2, else
  1 - (d2 - near2) / (far2 - near2). The sound's gain is 1 for g > 1, 0 for g < 0, else 0.02 * 50^g (so out of range
  it is 0.02, not silence). Then, with Status off: fire Off and return; else stay active. A missing object or sound:
  nothing happens (the original returns an error code).
- **TT_LinearVolume** is described below (same curve, but cut to 0 at or below 0.01).
- **Midi Player** is in none of the player's libraries: a named no-op.

Determinism: what a script can ask of a sound (is it playing) is answered from the run's tick clock and the sound's
length in ticks, taken from the game's sound set without an audio device; a file the set does not have is silent
and ends at once. The audio only receives commands.

## TT_Timer

GUID `6ac67901:7d2a6059`. Prototype `0x10022710`, execute `0x10022800` (thunk `0x10001302`).
Compatible class `0x13`. Description "Chronometer (ON Start, OFF Stop...)".

Inputs: `0 On/Reset`, `1 Pause`, `2 Play`, `3 Off`. Outputs: `Exit On`, `Exit Pause`, `Exit Play`,
`Exit Off`. Output param `0 Elapsed Time` (Time `54b4422b:730f0f4f`, ms float, default "0m 0s 0ms").

The state is **not** in the BB. It lives in a global manager `TT_Sceneanager` (sic): GUID
`60632e28:7d3b3c7d`, constructor `0x10027130`. It has a running byte at `+0x28` (init 1) and elapsed ms
at `+0x2c` (init 0). All of the manager's own callbacks return 0, so it never ticks by itself.
Helpers: `0x10027320` reset (running = 1, elapsed = 0), `0x10027340` pause (running = 0), `0x10027360`
play (running = 1), `0x10027380` get, `0x100273a0` add(dt) (only if running).

Execute:
1. On/Reset active: reset, clear input, activate Exit On. **Fall through** to the next checks.
2. Pause active: pause, clear, activate Exit Pause, return 1.
3. Play active: play, clear, activate Exit Play, return 1.
4. Off active: **reset** (running = 1, elapsed = 0), clear, activate Exit Off, return 0.
5. Otherwise: `out = elapsed + dt`, then `add(dt)`, write output 0, return 1.

Quirk: while paused, each update frame still outputs `elapsed + dt`, a value that jitters by the frame
time. Game: one instance in `Gameplay.nmo` `Gameplay_Energy`. Init → On/Reset. Exit On → Pause (same
frame). Switch On Message outputs → Pause/Play. Exit On → Timer.In 0.

## TT LookAt

GUID `3d4861f8:2861703d`. Prototype `0x10010c20`, execute `0x10010e00` (thunk `0x100012e9`). The
orientation helper is `0x10011190` (thunk `0x100015dc`). Compatible class `0x21`, behavior flags
`0x40000` (targetable).

In `In`, out `Out`. Pins: `0 Position` (Vector), `1 Referential` (3D Entity), `2 Following Speed`
(Percentage, default 100 % = 1.0), `3 Hierarchy` (Bool TRUE), `4/5/6 look X/Y/Z-Axis` (Bool, default
"false"; the code pre-sets 1 before reading). Settings: `0 Time Based` (Bool TRUE), `1 Direction`
(Direction enum: X=1, -X=2, Y=3, -Y=4, Z=5, -Z=6; default Z; the code default is 5), `2 Roll` (Angle, 0).

Execute (runs once, returns 0):
1. Target NULL → return `0xa004`.
2. `E = target.GetPosition(NULL)` (world). `W = Referential ? Ref.Transform(Position) : Position`
   (+0x17c).
3. speed = pin 2. If Time Based: `speed *= DeltaTime * 0.07`. Clamp `speed <= 1`. At 60 fps and
   100 % this is 1.17 → 1, so it snaps.
4. `D = W - E`. If D == 0: skip to 7. Else zero the components whose `look ?-Axis` flag is TRUE (TRUE
   means *ignore* that axis), then normalise if the length is non-zero.
5. `Hierarchy` is read and inverted, then **never used**. The helper always gets keepChildren = 0.
6. Helper(ent, D, dir, roll, up = VxVector::axisY(), speed, keepChildren = 0, keepScale = 0):
   - keepScale == 0 → `S = GetScale(local)` (+0x144).
   - If dir is even (-X/-Y/-Z), `D = -D`.
   - It halves `up.x` and `up.z`, and has a dead "if up.x/2 > 1.5 → up = current up" test. With
     up = (0,1,0) both have no effect.
   - Z/-Z case (5, 6): `C = GetOrientation` dir (current world Z). `N = C + (D - C)*speed`,
     `R = up × N`, `U = N × R`, normalise each non-zero vector,
     `SetOrientation(N, U, R, NULL, keepChildren)`.
   - Y/-Y case (3, 4): `C` = current up axis. `N = C + (D - C)*speed`, `A = N × up`, `B = A × N`,
     normalise, `SetOrientation(dir=A, up=N, right=B)`.
   - X/-X case (1, 2): same pattern, starting from the current right axis. The game does not use it.
   - If roll != 0: `Rotate3f` about local Z (for Z) or local Y (for Y) by roll, with ref = self.
   - keepScale == 0 → `SetScale(S, 0, local=TRUE)` (+0x140).
7. Clear In, activate Out, return 0.

Game (Gameplay.nmo): `ExtraLife`, `HolzTrafo`, `SteinTrafo`, `Extrapoint`, `Checkpoint` use
Direction -Z, look Y-Axis TRUE (yaw only), speed 1, Position = an Op result, Referential NULL. They are
retriggered through Binary Switch loops with delay 1. `set Ballarrows` uses Direction Y, Referential =
a Camera parameter (`3cf24d6f:216204f9`), Position (0,0,0) (the camera origin), Hierarchy TRUE.

## TT ConvertPixel-Homogen

GUID `18f96977:18e20f83`. Prototype `0x1000e8c0`, execute `0x1000e9e0` (thunk `0x10001230`).
Compatible class `0x13`. In `On`, out `Exit On`. Pin `0 Position` (2D Vector, "0,0"). Setting `0
Pixel to homogen?:` (Bool). Outputs `0 X`, `1 Y` (Float), `2 2D Vector`.

Execute: clear In, **activate Exit On first**. Read pos and the setting.
`W = rc->vt+0xd4()`, `H = rc->vt+0xd0()` (CKRenderContext GetWidth / GetHeight, on the behavior
context's render context).
- TRUE: `x = pos.x / W`, `y = pos.y / H`.
- FALSE: `x = (float)(int)(pos.x * W)`, `y = (float)(int)(pos.y * H)`, truncating with `__ftol`.

Write X, Y, and (x, y). Return 0.

Game: Menu.nmo `Init` has 6 instances with FALSE (for example (0.035, 0), (0, 0.017), (0.03, 0),
(0.013, 0), (0, 0.015)). These turn fractions into pixel offsets. Gameplay `Move LifeEnd` has 2
instances with TRUE and a source-fed position.

## TT_SplitString

GUID `5ae74e2f:799c49b5`. Prototype `0x10012140`, execute `0x10012220` (thunk `0x10001082`).
Behavior flags `0x400` (variable output params). In `In`, out `Out`. Pins `0 Text`, `1 Delimiter`
(String). Output `0 Elements Found` (Int), then N string outputs (the game has `Pout 1`..`Pout 10`).

Execute: clear In, **activate Out first**. If the Text pointer is NULL, return 0. Otherwise:
- s = CKStrdup(text) (never freed). The delimiter defaults to " " if NULL or empty. It is a whole
  string, matched with `strstr` (`0x100289c0`), so it can be several characters.
- Loop: find the next delimiter. If it is at the current position (empty token), skip it without
  counting. Otherwise `count++`, copy the token into a 512-byte stack buffer, and
  `SetStringValue` (+0x5c) output `count`. If that output does not exist, **break**. Advance past the
  delimiter.
- The non-empty tail after the last delimiter is also a token. If its output does not exist, **return
  without writing the count**.
- Write `Elements Found = count` and return 0.

Game: Gameplay `load Tutorialtext`, delimiter `*`, fed by `Load String.Out`. Out → Set Row.

## TT_PreloadTextures

GUID `416b4f0e:192d339a`. Prototype `0x100162f0`, execute `0x10016390` (thunk `0x100012df`).
Description "Copies all textures of the current ... to video memory". In `In`, out `Out`. Output
`0 NumberOfTextures` (Int).

Execute: clear In. For every object of class `0x1f` (CKTexture) in the context: if
`!tex->IsInVideoMemory()` (vt +0x70) and `tex->SystemToVideoMemory(behctx.render_context, FALSE)`
(vt +0x68) succeeds, then count++. Write the count, activate Out, return 0.

## TT_FlushTextures

GUID `734f73b4:01fc79ac`. Prototype `0x10015810`, execute `0x10015890` (thunk `0x100014ba`).
Description "Flushes all Textures from VideoMemory". Execute: clear In, call
`CKRenderManager vt+0x98()` (FlushTextures), activate Out, return 0.

Game (both): `Levelinit.nmo` graph `Preload Textures`: In → Flush → Preload → Out. For a GL port
these are "upload every texture now". The count only feeds a local.

# Part 3. Intersections, filters and hierarchy

## Ray Intersection

`Ray Intersection`, GUID `671e4a87:383b2912`, `Logics.dll` (image base `0x25480000`). Category `Logics/Test`,
author Virtools, version `0x10000`, compatible class `0x13` (BeObject). Behavior flags `0x40000`
(targetable), `SetFlags(1)`.

| Function | Address |
|---|---|
| declaration | `0x25482a70` |
| prototype creation | `0x25482b00` |
| execute | `0x25483090` |
| per-candidate test (cdecl helper) | `0x25482cf0` |
| callback | none |

The Ghidra decompile of both `0x25483090` and `0x25482cf0` is garbled (stack slots merged, arguments of
virtual calls lost). What follows is from the disassembly, with stack slots tracked by hand.

In short: cast one ray (a half-line, not a segment) in world space, test it against the **mesh triangles**
of every candidate 3D entity (a whole group, or every 3D object / character / sprite3D in the level), keep
the hit nearest to the ray origin, and succeed if that hit is closer than `Depth`.

### Pins

Inputs `0 In`. Outputs `0 True`, `1 False`.

| # | Input parameter | Type | Default | Meaning |
|---|---|---|---|---|
| 0 | `Ray Origin` | Vector | `0,0,0` | in the `Referential` frame (world if NULL) |
| 1 | `Ray Direction` | Vector | `0,0,1` | in the `Referential` frame; normalised after transforming |
| 2 | `Referential` | 3D Entity | NULL | frame of pins 0/1 |
| 3 | `Depth` | Float | `100` | maximum world distance from origin to hit |
| 4 | `Filter` | Group | NULL | candidates; NULL = all 3D objects, characters, sprite3Ds |

| # | Output parameter | Type | Written when |
|---|---|---|---|
| 0 | `Object Intersected` | 3D Entity | always on a hit |
| 1 | `Face Index` | Integer | hit and setting 0 set |
| 2 | `Nearest Vertex Index` | Integer | hit, setting 0, entity has a current mesh |
| 3 | `Intersection Point` | Vector (world) | same |
| 4 | `Intersection Normal` | Vector (world, unit) | same |
| 5 | `Distance` | Float (world) | same |

Settings (local parameters):

| # | Name | Type | Default | Meaning |
|---|---|---|---|---|
| 0 | `Output Param Update` | Boolean | TRUE | write outputs 1..5 on a hit |
| 1 | `Skip Owner` | Boolean | TRUE | exclude `GetTarget()` (the owner when there is no target pin) from the candidates |
| 2 | `Skip Transparent` | Boolean | FALSE | despite the name: skip candidates whose `IsVisible()` (CKObject +0xc) is 0 |

On a miss nothing is written; the outputs keep their previous values.

### Execute (`0x25483090`)

```
tgt = GetTarget(); if !tgt: return CKBR_OWNERERROR (0xa004)        // input stays active
ActivateInput(0, FALSE)
skipOwner = 1; GetLocal(1, &skipOwner); if !skipOwner: tgt = NULL   // tgt = object to exclude
skipInvis = 0; GetLocal(2, &skipInvis)     // missing local -> stays 0
O = pin0, D = pin1
if ref = pin2: O = ref->Transform(O)            (+0x17c, local point -> world)
               D = ref->TransformVector(D)      (+0x184, local vector -> world, scale included)
D.Normalize()
depth = 100; read pin3
S = { O, D, E = O + D, depth, desc (44 bytes), best = NULL, best_d2 = depth*depth }
scene = GetCurrentScene()
if group = pin4: for i in 0..count-1: o = group[i]; if o != tgt: test(o, S, scene, skipInvis)
else: for each class 0x29 (CK3dObject), then 0x28 (CKCharacter), then 0x25 (CKSprite3D), in
      GetObjectsListByClassID order: o != tgt -> test(o, ...)
if best && best_d2 < depth*depth:
    SetOutputParameterObject(0, best)
    upd = 1; GetLocal(0, &upd)
    if upd && (mesh = best->GetCurrentMesh() (+0xec)):
        mesh->GetFaceVertexIndex(desc.FaceIndex, &a, &b, &c)   (CKMesh +0xfc)
        pa, pb, pc = mesh->GetVertexPosition(a/b/c)            (CKMesh +0xc8, local)
        da, db, dc = |p - desc.Point| (local-space distances)
        nearest = da<db ? (da<dc ? a : c) : (db<dc ? b : c)    // ties go to the later vertex
        P = best->Transform(desc.Point); N = best->TransformVector(desc.Normal); N.Normalize()
        out1 = desc.FaceIndex, out2 = nearest, out3 = P, out4 = N, out5 = sqrt(best_d2)
    ActivateOutput(0)            // True
    return CKBR_OK
ActivateOutput(1)                // False
return CKBR_OK
```

The BB never stays active. `best_d2` starts at `depth^2` and only strictly smaller distances replace it,
so the final `< depth^2` check is the same as "a hit was recorded". The squared distance is measured in
world space, between the world ray origin and the world hit point.

### Candidate test (`0x25482cf0`, cdecl `(obj, S*, scene, skipInvis)`)

```
if !CKIsChildClassOf(obj, 0x21 CK3dEntity): return
isChar = CKIsChildClassOf(obj, 0x28)
if !obj->IsInScene(scene): return
if skipInvis && !obj->IsVisible(): return
if CKIsChildClassOf(obj, 0x25 Sprite3D):
    mesh_test(obj)                                   // no broad phase
    return
r = obj->GetRadius() (+0x1d0); C = obj->GetBaryCenter() (+0x1cc, world)
V = C - O; v2 = V.V; t = V.D
if !(v2 < (r + depth)^2): return                     // sphere out of reach
if !(r + t > 0): return                              // sphere entirely behind the origin
if !(v2 - t*t < r*r): return                         // ray misses the sphere
if !VxIntersect::RayBox(ray{O,D}, obj->GetBoundingBox(FALSE) (+0x1c0, world AABB)): return
if !isChar: mesh_test(obj)
else for i = GetBodyPartCount()-1 downto 0 (+0x1e8 / +0x1e4 GetBodyPart(i)):
    if part && RayBox(ray, part->GetBoundingBox(FALSE)): mesh_test(part), but record obj
mesh_test(e):
    desc.u/v/distance cleared
    if e->RayIntersection(&S.O, &S.E, &desc, NULL, 0) (+0x15c) == 0: return
    W = e->Transform(desc.Point); d2 = |W - O|^2
    if d2 < S.best_d2: S.best_d2 = d2; S.best = obj (the character, not the body part); S.desc = desc
```

Character quirk: the recorded object is the character, but the face index and point come from the body
part. The execute then asks the character for its current mesh, which is normally NULL, so outputs 1..5
are not written (True still fires). Ballance has no characters, so this does not matter.

`VxIntersectionDesc` (44 bytes): `+0 Object`, `+4 IntersectionPoint` (entity-local), `+0x10
IntersectionNormal` (local, interpolated, not unit), `+0x1c TexU`, `+0x20 TexV`, `+0x24 Distance`,
`+0x28 FaceIndex`.

### Engine helpers the test relies on

**CK3dEntity::GetRadius** (CK2_3D `0x10008ee2`): `mesh->GetRadius()` times the largest row length of the
world matrix (rows 0..2). Without a mesh, half the largest world-box extent (or a constant if the box is
invalid). **CKMesh radius / barycenter** (`0x1001f13d`, `0x1001f189`, cached by `0x1001f1c4`):
barycenter = the mean of all vertex positions; radius = the largest distance from that mean to a vertex.
**GetBaryCenter** (`0x100090c1`) = `Transform(mesh barycenter)`, or the world position if there is no mesh.

**VxIntersect::RayBox(ray, box)** (VxMath `0x2428c650`): an infinite half-line against an AABB. `VxBbox` is
`{Max, Min}` (Max first). `h = (Max-Min)/2`, `c = (Max+Min)/2`, `Dl = O - c`.
- For each axis i: reject if `|Dl_i| > h_i` and `Dl_i * dir_i >= 0` (the origin is outside that slab and
  moving away).
- `W = dir x Dl`. Reject if `|W_x| > |d_y| h_z + |d_z| h_y`, or `|W_y| > |d_x| h_z + |d_z| h_x`, or
  `|W_z| > |d_x| h_y + |d_y| h_x`.
- Otherwise 1. There is no length limit.

**CK3dEntity::RayIntersection(Pos1, Pos2, desc, Ref, options)** (CK2_3D `0x10008dc5`):
- Returns 0 if there is no current mesh (entity `+0x68`).
- If `Ref != this`, it maps Pos1 and Pos2 into the entity frame with `InverseTransform` (+0x180). `Ref`
  NULL means world.
- `dir = Pos2 - Pos1` (local, not renormalised).
- `desc->Object = Ref`. It calls the mesh ray routine through the function pointer `0x1008f28c`. That
  pointer is `0x1002ea85`, or the SSE twin `0x1002f52e`: same logic.
- The call is `(mesh, Pos1, dir, desc, segment=options, entity world matrix)`.
- On a hit: `desc->Object = this` and `desc->Distance *= |Pos2 - Pos1|` (the original Ref-frame vector).
- Returns the routine's result.

**Mesh ray routine** (`0x1002ea85`):
1. There must be at least one vertex and one face. Vertices have a 32-byte stride
   (`pos, normal, u, v`); faces are ushort triples, plus a 16-byte per-face record whose first 12 bytes are
   the face normal.
2. Broad phase, only if faces > 15:
   - `a = dir x Y`; if `|a|^2 < eps` (constant `0x10083828`), use `a = dir x X`.
   - `b = dir x a`, then `a = dir x b`.
   - Each vertex gets 2 bits per axis from `v.a < O.a` vs `>=`, and the same for b.
   - A face is skipped when its three vertices share a bit, so the whole triangle is on one side of the
     ray line. If all vertices share a bit, nothing is tested.

   This is a pure speed-up and gives exactly the same result as testing every face.
3. For each face, `mat = mesh->GetFaceMaterial(f)` (+0x100):
   - If `mat && mat->IsTwoSided()` (CKMaterial +0xc0), use `VxIntersect::RayFace`, or `SegmentFace` when
     `segment`.
   - Otherwise use `RayFaceCulled` / `SegmentFaceCulled`.

   The BB passes options 0, so it uses the ray variants. Faces with no material are culled.
4. RayFace (VxMath `0x2428d260`):
   - Plane `(n, d = -n.p0)` from the stored face normal.
   - RayPlane (`0x2428bba0`):
     - `den = n.dir`; reject if `|den| < 1.1920929e-7`.
     - `t = -(n.O + d) / den`; reject if `t < -1.1920929e-7`.
     - `P = O + t*dir`.
   - RayPlaneCulled (`0x2428bc70`) instead rejects unless `den <= -1.1920929e-7`: the ray must hit the
     front side, against the normal.
   - Then PointInFace (`0x2428d0b0`):
     - Drop the axis with the largest `|n|` component. Ties keep the earlier axis: X, then Y, then Z.
     - Take three 2D edge cross products `s0 = (P-p0) x (p1-p0)`, `s1 = (P-p1) x (p2-p1)`,
       `s2 = (P-p2) x (p0-p2)`, with the sign convention `e.u*f.v - e.v*f.u` as in the code.
     - Inside iff all three are `< 0`, or all are `>= 0`. Zero counts as positive.
5. Keep the face with the smallest `t` (start `1e35`, strict `<`). Count every intersected face.
6. For the nearest face:
   - `desc.Point = O + t*dir` (local).
   - Barycentric weights come from `VxIntersect::GetPointCoefficients`.
   - `desc.Normal` = weighted sum of the three **vertex normals**. It is not the face normal and is not
     normalised.
   - `desc.u/v` = the weighted vertex UVs.
   - A screen-space UV re-computation happens when material `+0xe0` returns 0 and `desc->Object` is the
     render context's camera. It is irrelevant here.
   - `desc.Distance = t`, `desc.FaceIndex = face`.
7. Alpha pick test `0x1002e82f`, on the nearest face's material:
   - Take its texture (`GetTexture` +0x7c) and the bitmap's `PickThreshold` (`CKBitmapData +0x1c`).
   - If the threshold is nonzero, wrap or clamp the UV by the texture address mode (+0xa4) and read the
     texel.
   - If `alpha < threshold`, the **whole entity reports no hit**. The next-nearest face is not tried.
   - A threshold of 0 (the default) always passes.
8. Return the number of faces hit (0 if none).

**Port recipe**:
- Build the world ray `O`, unit `D`.
- For each candidate (in the order above, excluding the owner, not in the scene => skip):
  1. Do the sphere and AABB reject (optional, result-neutral except that the sphere test uses `depth`).
  2. Transform `O` and `O+D` into the entity's local frame.
  3. Ray-cast the half-line against the local mesh triangles, with backface culling unless the material
     is two-sided and the epsilons above.
  4. Take the nearest hit, transform the point to world, and compare squared world distances (strict `<`).

### Game usage

Two instances. Both are in the init scripts of `3D Entities/PH/P_Extra_Life.nmo` (BB #186,
id 6298) and `P_Extra_Point.nmo` (id 6272).

| Pin | Value |
|---|---|
| Ray Origin | (0, -2, 0) |
| Ray Direction | (0, -1, 0) |
| Referential | the extra itself (`P_Extra_Life_MF` / `P_Extra_Point_MF`) |
| Depth | 1.5 |
| Filter | `Op` result = group `Phys_Floors` (fetched by name) |

Settings: `Output Param Update` = FALSE, `Skip Owner` = TRUE. These files were saved with only 2 locals,
so `Skip Transparent` is absent and reads as 0.

Meaning: "is there a floor within 1.5 units, starting 2 units below the extra, straight down in its frame?"
- Life: `True -> Binary Memory.In 0`, `False -> In 1`.
- Point: `True/False -> Parameter Selector.In 0/1`, which feeds `TT Extra.Initialize` (the `Flying Extra?`
  choice).

Only `Object Intersected` is written.

## Box Box Intersection

`Box Box Intersection`, GUID `64154401:76cf37af`, `Collisions.dll` (base `0x25300000`). Category
`Collisions/Intersection`. It needs the Collision Manager `38244712:00000000`. `SetFlags(1)`, no callback.

| Function | Address |
|---|---|
| declaration / prototype | `0x253056e0` / `0x25305780` |
| execute | `0x25305850` |
| `CKCollisionManager::BoxBoxIntersection` (manager vtable `0x2531a300` slot +0xac) | `0x25312430` |
| `VxOBB(const VxBbox&, const VxMatrix&)` (inlined ctor in Collisions) | `0x253141e0` |
| `VxIntersect::OBBOBB` (VxMath) | `0x24287830` |
| `VxIntersect::AABBAABB` / `AABBOBB` (VxMath) | `0x242877b0` / `0x24287dd0` |

Pins: input `0 In`; outputs `0 True`, `1 False`. Parameters:
- `Entity 0`, `Entity 1` (3D Entity).
- `Hierarchy 0`, `Hierarchy 1` (Boolean, default FALSE).

The PH file version names them `Entity 1/2`, `Hierarchy 1/2`; the indices are the same. There are no
settings and no outputs parameters.

### Execute (`0x25305850`)

```
cm = GetManagerByGuid(COLLISION_MANAGER_GUID)
ActivateInput(0, FALSE)
e0 = pin0 object, e1 = pin1 object
if e0 && e1:
    h0 = 0; read pin2; h1 = 0; read pin3
    if cm->BoxBoxIntersection(e0, h0, TRUE, e1, h1, TRUE): ActivateOutput(0); return CKBR_OK
ActivateOutput(1); return CKBR_OK
```

So the BB always asks for **local boxes**, which makes it an oriented-box (OBB vs OBB) test. It is a pure
box test with no mesh triangles. It fires one output per activation and never stays active.

### `BoxBoxIntersection(ent1, hiera1, local1, ent2, hiera2, local2)` (`0x25312430`, thiscall, `ret 0x18`)

For each entity:
1. Start the box as `Max = (-1e6)x3`, `Min = (+1e6)x3` (inverted, then overwritten).
2. If `GetClassID() == 0x28` (character):
   - `ent = GetRootBodyPart()` (+0x1dc). If NULL, return 0.
   - `M = ent->GetWorldMatrix()` (+0x174), only if local.
   - `box = ent->GetHierarchicalBox(local)` (+0x1c8). Characters always use the hierarchical box.
3. Otherwise:
   - `M = GetWorldMatrix()`, only if local.
   - `box = hiera ? GetHierarchicalBox(local) (+0x1c8) : GetBoundingBox(local) (+0x1c0)`.

Then:
- **neither local**: `AABBAABB(box1, box2)` on the world boxes.
- **both local** (the BB): `OBBOBB(VxOBB(box1, M1), VxOBB(box2, M2))`.
- **only local1**: `AABBOBB(box2, VxOBB(box1, M1))`; **only local2**: `AABBOBB(box1, VxOBB(box2, M2))`.

Box sources (CK2_3D):
- `GetBoundingBox(TRUE)` = the entity's local box `+0x14c`, refreshed by `UpdateBox` (`0x10006113`).
  - With a mesh, it is the mesh's local vertex AABB, merged with the skin box if there is a skin.
  - Without a mesh, it is all zeros.
  - A user-set box (entity flag `0x10`) wins.
- `GetBoundingBox(FALSE)` = `+0x164`, the local box transformed by `VxBbox::TransformFrom`: the AABB of
  the 8 transformed corners.
- `GetHierarchicalBox(TRUE)` (`0x1000926b`) takes the scene-graph node's world box (the union over the
  entity and its descendants). It maps that box into the entity frame by the inverse world matrix, again
  as the AABB of the corners, so it is loose.

`VxOBB(box, M)` (`0x253141e0`):
- `center = M * ((Max+Min)/2)` (Vx3DMultiplyMatrixVector, translation included).
- `axis_i = row_i(M) / |row_i(M)|` for i = 0..2.
- `extent_i = (Max_i - Min_i) * 0.5 * |row_i(M)|`, so scale is folded into the extents.
- Layout: `+0 center`, `+0xc/+0x18/+0x24 axes`, `+0x30 extents`.

`OBBOBB(A, B)` (`0x24287830`) is the standard 15-axis separating-axis test (Gottschalk), with no epsilon:
- Setup: `T = B.c - A.c` (world), `R_ij = A.axis_i . B.axis_j`, `a = A.ext`, `b = B.ext`.
- A's axes, for each i: separated if `|T.A_i| > a_i + sum_j b_j |R_ij|`.
- B's axes, for each j: separated if `|T.B_j| > b_j + sum_i a_i |R_ij|`.
- The 9 cross axes `A_i x B_j`: separated if
  `|(T.A_{i+2}) R_{i+1,j} - (T.A_{i+1}) R_{i+2,j}| > a_{i+1}|R_{i+2,j}| + a_{i+2}|R_{i+1,j}| + b_{j+1}|R_{i,j+2}| + b_{j+2}|R_{i,j+1}|`,
  indices mod 3.
- The order is: A axes, B axes, then the cross axes with i outer and j inner.
- It returns 0 at the first separating axis, otherwise 1. Every comparison is `radius < |projection|`
  → separated, so touching boxes count as intersecting.

`AABBAABB(A, B)` (`0x242877b0`): 1 iff `A.Min <= B.Max` and `A.Max >= B.Min` on all three axes (inclusive).

### Game usage

- **`3D Entities/Gameplay.nmo`** (id 9051, `BallManager`):
  - `Entity 0` = the `Group Iterator.Element` over the `DepthTestCubes Group`; `Entity 1` = `ActiveBall`;
    both hierarchies FALSE.
  - `Group Iterator.Loop Out -> In`; `True -> Deactivate Ball.In 0` (the ball has left the level volume);
    `False -> Group Iterator.Loop In` (delay 1).
  - This is the OBB of each depth-test cube against the OBB of the ball's mesh box.
  - `New Ball.output0 -> Group Iterator.In`; `Group Iterator.Out -> In` (delay 1). So one cube is tested a frame,
    in the group's order, and after the last one a frame passes without a test: a round is N + 1 frames, started
    by each new ball. The test runs before the frame's physics, so it sees the ball where the last frame left it.
    The port does the same (`Sim::fell_off`), its phase fitted to the C game on level 2 (a ball lost at the same
    tick, 331, on the same input).
- **`3D Entities/PH/P_Modul_18.nmo`** (id 6510, `Physics Force`):
  - `Entity 1` = `Get Cell.Cell Value` (the ball); `Entity 2` = `P_Modul_18_Kollisionsquader` (a 3D
    object); hierarchies FALSE.
  - It runs on `TT Scaleable Proximity.InRange`. Both outputs go to `Binary Switch.In`, which turns the
    physics force on or off.

## Planar Filter

`Planar Filter`, GUID `00cd320b:32ed010b`, `Visuals.dll`. Category `Visuals/FX`, compatible class `0x13`.
Declaration `0x25787980`, prototype `0x25787a10`, execute `0x25787b50`, callback `0x25787be0`
(all messages), render callback `0x25787cd0` (a label).

Inputs `0 On`, `1 Off`; outputs `0 Exit On`, `1 Exit Off`. Parameters:

| # | Name | Type | Default |
|---|---|---|---|
| 0 | `Filtering Color` | Color | 255,255,255,128 |
| 1 | `Additional Color` | Color | 0,0,0,0 |
| 2 | `Texture` | Texture | NULL |
| 3 | `Source Blend` | VXBLEND_MODE | Source Alpha (5) |
| 4 | `Dest Blend` | VXBLEND_MODE | Inverse Source Alpha (6) |
| 5 | `Texture Mag Mode` | filter mode | Linear (2) |
| 6 | `Texture Min Mode` | filter mode | Linear (2) |

Execute:
- `Off` is checked first. If active: clear it, `ActivateOutput(1)`,
  `rc->RemovePostSpriteRenderCallBack(0x25787cd0, beh)` (CKRenderContext +0x94), return 0.
- Else if `On`: clear it, `ActivateOutput(0)`, `rc->AddPostSpriteRenderCallBack(0x25787cd0, beh, FALSE)`
  (+0x90).
- In every case other than Off, return 1 (stay active). `rc` is `behcontext.CurrentRenderContext`.

The callback is registered on the render context, not on an entity, and runs after the 2D sprites. So the
quad covers everything, including 2D.

Render callback `(rc, beh)` reads all 7 pins every frame, then:
1. `SetTexture(tex, 0, 0)` (+0xf0).
2. Render states (+0xe8): FOGENABLE(28)=0, WRAP0(128)=0, CULLMODE(22)=1 (none), SRCBLEND(19)=pin3,
   DESTBLEND(20)=pin4, ALPHABLENDENABLE(27)=1, ZENABLE(7)=0, ZWRITEENABLE(14)=0.
3. Texture stage states, stage 0 (+0xf4): TEXTUREMAPBLEND(39)=4 (modulate-alpha), MAGFILTER(17)=pin5,
   MINFILTER(16)=pin6.
4. SPECULARENABLE(29)=1.
5. `GetDrawPrimitiveStructure(0x230, 4)` (+0x98). With `GetViewRect` (+0xe0) giving `(l,t,r,b)`, the
   pre-transformed vertices are:
   - positions `(l,t,0,1) (r,t,0,1) (r,b,0,1) (l,b,0,1)`;
   - diffuse = `RGBAFTOCOLOR(Filtering Color)` and specular = `RGBAFTOCOLOR(Additional Color)`, on all 4;
   - UV `(0,0) (1,0) (1,1) (0,1)`.
6. Indices `0,1,2,3` (+0x9c); `DrawPrimitive(VX_TRIANGLEFAN=6, idx, 4, data)` (+0x15c).
7. Restore ZWRITEENABLE=1, ZENABLE=1, and FOGENABLE=1 if the context reports fog (+0x148).

Net effect: a full-viewport quad, `texture*diffuse` (plus the specular colour), blended `src*SB + dst*DB`.
With the defaults: `lerp(dst, filter.rgb, filter.a) + additional`.

Callback messages:
- 4 DETACH: remove (if there is an owner and a render context).
- 5 PAUSE, 9 RESET, 17 DEACTIVATESCRIPT: remove if active.
- 6 RESUME, 16 ACTIVATESCRIPT: add if active and the parent script is active in the scene.
- 15 NEWSCENE: add if playing, active and the script is active in the scene; otherwise remove.

Game usage: one instance, `Gameplay.nmo` id 8352, script `Überblendung` (cross-fade).
- `Filtering Color` comes from a parameter source (an animated fade colour; the file value is 0,0,0,0).
- `Additional Color` = 0; Texture NULL (so an untextured flat colour); blend 5/6; filters 2/2.

## Get Nearest In Group

`Get Nearest In Group`, GUID `085207eb:584950d8`, `Logics.dll`, execute `0x25488f40`, no callback.

- Input `In`; output `Out`.
- Parameters: `Group` (Group), `Position` (Vector, `0,0,0`), `Referential` (3D Entity).
- Output parameters: `Nearest Object` (3D Entity), `Distance` (Float).

```
ActivateInput(0, FALSE); ActivateOutput(0)          // Out fires even on error
g = pin0; if !g: return CKBR_PARAMETERERROR (0xa008)
P = pin1; if ref = pin2: P = ref->Transform(P) (+0x17c)
SetOutputParameterObject(0, NULL)
best = FLT_MAX (0x7f7fffff); bo = NULL
for i in 0..count-1: o = g[i]
    if o != ref && CKIsChildClassOf(o, 0x21):
        W = o->GetPosition(NULL) (+0x128, world); d2 = |W - P|^2
        if d2 < best: best = d2; bo = o                 // strict: first of equals wins
out0 = bo; out1 = sqrt(best)                           // empty group -> NULL, 1.8446743e19
return CKBR_OK
```

The referential itself is excluded, but the owner is not.

Game usage: `Gameplay.nmo` id 6129, `Trafo Manager`.
- Group = `Trafo Group`, Position (0,0,0), Referential = `ActiveBall`: the nearest transformer to the
  ball's origin.
- `Out -> Test.In`; `Test.False -> In` (delay 1).

## Set Parent

`Set Parent`, GUID `9d9d9d98:7e7a7f75`, `3DTransfo.dll`, execute `0x25002a70`. Behavior flags `0x40000`
(targetable), no callback.

- Input `In`, output `Out`, one parameter `Parent` (3D Entity).

```
t = GetTarget(); if !t: return CKBR_OWNERERROR (0xa004)
ActivateInput(0, FALSE); ActivateOutput(0)
t->SetParent(pin0 object, KeepWorldPos=TRUE)       // CK3dEntity +0x90
return CKBR_OK
```

The pin is not NULL-checked; NULL unparents to the root.

CK2_3D `SetParent` (`0x1000932b`):
- Returns 0 if parent == self, and 0 if the new parent is a descendant of self (cycle).
- Returns 1 immediately if it is already the parent.
- Otherwise it removes self from the old parent's child list, relinks the scene-graph node, and sets the
  new parent.
- With KeepWorldPos the world matrix is kept and the local matrix is recomputed.

Game usage: 7 instances.
- `Gameplay.nmo`:
  - `set new Ball` and `Set Init-Positions`: Parent = `ActiveBall`, each followed by `Set World Matrix`.
  - `Gameplay_Events` twice: Parent local NULL, i.e. unparent. One is followed by `Set Clipping Planes`.
  - `Set Pfeilrunter`: Parent = local `Parent` / `Tut Frame`.
- `PE_Balloon.nmo`, `Greif Anim`: Parent = `PE_UFO_Body`.

# Part 4. Logics and Materials

## Streaming Event

`Streaming Event`, GUID `1f0b52bf:4c3342dd`, `Logics.dll` (category `Logics/Streaming`, version `0x20000`,
compatible class `0x13`). Description string: "Activates outputs on the input activation change."

| Function | Address |
|---|---|
| declaration | `0x254868d0` |
| prototype creation | `0x25486960` |
| execute (v2, current) | `0x25486aa0` |
| execute (v1, legacy) | `0x25486a20` |
| behavior callback | `0x25486b70` (mask `0xffffffff`) |

Prototype: inputs `In 0`, `In 1`; outputs `Out 0`, `Out 1` (variable inputs, flags `0x2000080`; the
callback keeps output count = input count). No input parameters.

| Kind | # | Name | Type | Default | Meaning |
|---|---|---|---|---|---|
| pout | 0 | `Last Activated Output` | Integer | `-1` | index of the last output fired |
| local | 0 | (unnamed) | Integer | `0` | bit mask: bit i = "input i was active in the previous execution" |

### Execute (`0x25486aa0`, used when version >= 0x20000; every game instance is 0x20000)

```
mask = local0
for i in 0 .. GetInputCount()-1:            # index order
    if IsInputActive(i):
        ActivateInput(i, FALSE)
        if !(mask & (1 << (i & 31))):
            mask |= 1 << i
            pout0 = i                        # SetOutputParameterValue BEFORE ActivateOutput
            ActivateOutput(i, TRUE)
    else:
        mask &= ~(1 << i)
local0 = mask
return CKBR_OK (0)                           # never stays active
```

So an output fires on the *rising edge* of its input, where "edge" is measured between consecutive
**executions** of the BB, not between frames. The BB does not stay active, so it only runs when some input is
activated. If input 0 fires on frame N and nothing fires on N+1..N+9, bit 0 is still set on N+10 and a new
`In 0` produces nothing. The bit is cleared only when the BB executes (any other input active) without
input i.

When several inputs are active in the same execution, every one whose bit was clear fires, in ascending
index order. `Last Activated Output` ends up as the highest index that fired. If none fired, it keeps
its old value.

### Legacy execute (`0x25486a20`, version < 0x20000; not used by the game)

It reads `last = pout0` and scans the inputs in order. For each active input it deactivates it; the first
active input with `i != last` sets `pout0 = i`, fires `Out i` and returns. Inputs after that one stay
active (not cleared). It has no mask. Returns 0.

### Callback (`0x25486b70`)

- 3 ATTACH, 9 RESET: `pout0 = -1`, `local0 = 0`.
- 11 LOAD: if `GetVersion() < 0x20000`, switch to the legacy function. Then, like RESET: `pout0 = -1`,
  `local0 = 0`.
- 12 EDITED: add `"Out %d"` outputs until output count = input count, or delete trailing outputs.
- Everything else: return 0.

A port should reset on load/reset/attach. The saved local is 0 in every file anyway.

### Game usage (8 instances, all version 0x20000, local 0 = 0)

- `Gameplay.nmo`, `Gameplay_Events` (id 11618, 3 in / 3 out): `Binary Switch.False -> In 0`,
  `Binary Switch.True -> In 1`, `set first Checkpoint.Out 0 -> In 2`. `Out 0 -> Key Event.On`,
  `Out 1 -> Key Event.Off`, `Out 2` is unconnected. The Binary Switch loops on itself with `delay=1`, so it
  fires every frame. Streaming Event turns that per-frame level into On/Off edges for the Key Event. `In 2`
  only clears the other two bits, so after a checkpoint the next True/False fires again.
- `Gameplay.nmo`, `MoveKeys` (id 14906, 2/2): `TT Scaleable Proximity.EnterRange` and `Wait For All.Out`
  go to `In 0`. `Out 0 -> Nop.In 0` and `Out 0 -> In 1 (delay=1)`. `Out 1` is unconnected. It is a
  debouncer: while `In 0` stays active every frame, `Out 0` fires only once (in the frame after that, In 0 and
  In 1 are active together, so bit 0 stays set). `Out 0` can fire again only after one execution without
  `In 0`.
- `Menu.nmo`: 4 instances in `Menu_Opt_Graphics` and 2 in `Menu_Opt_Sound` (2 or 3 inputs). They are fed
  by `Up/Down Switch` / `Text` outputs and drive the `On`/`Off`/`In 2` pins of `Synch`, `Clouds`, `Back`,
  `Resolution` and `Volume`.

## Per Second

GUID `448e54ce:75a655c5`, `Logics.dll`. Execute `0x25490b50`, callback `0x25490c20`. Flags `0xc000400`
(variable parameter inputs/outputs). In `In`, out `Out`. Default pins: `pIn 0` / `pOut 0`, Float. The game
renames and retypes them.

Execute:
1. `ActivateInput(0, FALSE)`, then `ActivateOutput(0, TRUE)`. This happens first, before the values are
   computed (it makes no difference inside the call).
2. `s = (float)(ctx.DeltaTime * 0.001f)` (constant `0x254a13f4` = 0.001). DeltaTime is in ms, so s is in
   seconds, rounded to float.
3. For i in 0..GetOutputParameterCount()-1: take input pin i. Its source goes through the shared-input
   chain (`0x25483d70`: follows `+0x18` while flag `0x4000000` is set, same as `ck_param_resolve`). If
   there is no source, skip it (the output is unchanged and not marked changed). Otherwise, for every
   float k of the output (`GetDataSize(out)/4` floats), set `out[k] = s * in[k]`. The input read pointer is
   `GetReadDataPtr(1)`, vtable `+0x54`. The output write pointer is `GetWriteDataPtr`, `+0x58`. Then
   `DataChanged()`.
4. Return 0.

Callback, 12 EDITED only (editor): retypes an Integer pout to Float, creates or retypes a matching
`pIn %d`, and removes extra inputs.

Game values (4): `P_Modul_18.nmo`, pin `X` = -15.0 (Float), pout `Y`, looped with `Rotate` (delay 1).
`PE_Balloon.nmo` `UFO`: Angle 2.61799 (150 deg/s). `Balls.nmo` `Rotate Lighting Sphere`: Angle 6.28319
(2 pi/s). `Gameplay.nmo` `animate SkyLayer`: Vector2D local `Skytranslation` (saved as (0,0)). Its
`pOut 0` feeds `Texture Scroller.Scroll Vector`.

## Timer

GUID `a2a5a63a:e4e7e8e5`, `Logics.dll`. Declaration and prototype near `0x2549e390`. Execute
`0x2549e390`, callback `0x2549e540`.

- Ins: `In`, `Loop In`. Outs: `Out`, `Loop Out`.
- Pin 0: `Duration`, Time (`54b4422b:730f0f4f`, float **milliseconds**). Default `0m 3s 0ms`.
- Pouts: 0 `Elapsed Time`, 1 `Delta Time` (both Time).

Execute:
```
if IsInputActive(0): ActivateInput(0,FALSE); elapsed = 0; pout0 = 0
else:                ActivateInput(1,FALSE); elapsed = pout0
elapsed = (float)(elapsed + ctx.DeltaTime)          # the first frame already adds dt
pout0 = elapsed; pout1 = ctx.DeltaTime
if !(elapsed < Duration): ActivateOutput(0)  (Out)  # fcomp C0 test at 0x2549e426; NaN -> Loop Out
else:                     ActivateOutput(1)  (Loop Out)
return 0
```
If `In` and `Loop In` are both active, `In` wins and `Loop In` stays active (it is not cleared).
The loop is made in the graph (`Loop Out -> Loop In`, normally with delay 1).

Callback, 11 LOAD: if `version < 0x10005` and the Duration source is an Integer, use the legacy function
`0x2549e460` (integer ms). Otherwise use `0x2549e390` and bump the version to `0x10005`. Every game instance
is 0x10005 with a Time duration.

Game durations (8): Balls `Rotate Lighting Sphere` 3000. Gameplay `Fadeout Manager` 20000 (x3),
`wait for continue` 25000, `Wait` and `Gameplay_Blitz` from sources (for example `Random.Rand`). Menu
`Wait` 4000.

## Random Switch

GUID `79d72fde:2e9d0912`, `Logics.dll`. Execute `0x254861b0`, callback `0x25486300` (12 EDITED: keeps
`Coef %d` pins = output count).

- In: `In`. Outs: `Out 1..Out N`. Pins: `Coef i`, Float, default 1.
- Setting (local 0): `Forbid twice the same`, Boolean. Local 1: Integer, the last output index.

Execute:
```
ActivateInput(0,FALSE)
excl = forbid ? local1 : -1
sum = 0; w[i] = 1.0 then GetInputParameterValue(i) for every i != excl; sum += w[i]
r = (float)(rand() * sum * (1/32767))             # exactly one msvcrt rand() per execution; const 0x254a13ec
acc = 0
for i in 0..N-1, i != excl:
    acc += w[i]
    if r <= acc: ActivateOutput(i); if forbid: local1 = i;  break    # test ah,0x41 / jnp at 0x2548629e
return 0
```
`r <= acc` means a zero sum picks the first allowed output, and `rand() = 32767` picks the last one with
nonzero weight. If nothing matches (for example negative weights), no output fires. With forbid set, the
saved local1 is 0, so output 0 cannot be picked on the first activation.

Game: `Sound.nmo` `Music_Atmo` and `Music_Theme`: 3 outputs, coefs 1/1/1, forbid = FALSE, so each output
has probability 1/3.

## Remove Row

GUID `1fa57136:14310857`, `Logics.dll`. Execute `0x25496680`, no callback. Target is a CKDataArray.

- In: `In`. Outs: `Removed`, `Not Present`. Pin: `Row Index`, Integer.

If there is no target, return `0xa004` (error; the input stays active and no output fires). If
`0 <= row < GetRowCount()`, call `RemoveRow(row)`, then `ActivateInput(0,FALSE)` and `Removed`. Otherwise
`ActivateInput(0,FALSE)` and `Not Present`. Returns 0.

Game: base.cmo `Check Highscore` row 10. Gameplay `sub Life` row from a source. Menu `Init` and
`Fullversion?` row 4.

## Objects With Attribute Iterator

GUID `6bc1494c:0c816ad3`, `Logics.dll`. Execute `0x25494f10`, callback `0x25495020`.

- Ins: `In`, `Loop In`. Outs: `Out`, `Loop Out`.
- Pin 0: `Attribute` (`3ea34ee9:09fa5366`).
- Pouts: `Object` (BeObject), `Attribute Value`.
- Local 0: index, Integer.

Execute:
1. `list = AttributeManager->GetAttributeListPtr(attr)`. This is the manager's global list of every
   object that has this attribute, in the order the attributes were added. It is not filtered by scene.
2. If `In` is active: clear it and set `idx = 0`. Otherwise clear `Loop In` and set `idx = local0 + 1`.
3. If `idx >= list.size`: fire `Out` and return 0. local0 is not written.
4. Otherwise:
   - `pout Object = list[idx]`;
   - `local0 = idx`;
   - if the object has an attribute parameter and pout 1 exists, `pout1.CopyValue(attrParam)`
     (vtable `+0x50`);
   - fire `Loop Out`;
   - return 0.

Callback, 12 EDITED: sets pout 1 to the attribute's parameter type, or disables pout 1 if the attribute
has no parameter.

Game: `Gameplay.nmo` `create TrafoGroup` (id 7039). Attribute `TrafoType` (runtime index 22).
`Object Create.Out -> In`, `Loop Out -> Add To Group.In -> Loop In` (no delay). The whole walk happens in
one frame, and `Out` returns to the parent.

## Fill Group By Class

GUID `4445257b:70016c57`, `Logics.dll`. Execute `0x25488c60`, no callback. Target: a CKGroup.

- In: `In`. Out: `Out`.
- Pins: 0 `Class` (class id), 1 `Derived Classes` (Bool), 2 `Only In Current Scene` (Bool, default 1).

Execute:
1. `ActivateInput(0,FALSE)` and `ActivateOutput(0)` come **first**.
2. If there is no target, return `0xa008`.
3. Class range: with Derived, cid in [0, 0x37]. Without Derived, cid in [c, c+1] **inclusive**
   (`0x25488cf7`: `hi = c+1`).
4. For each cid in the range with `CKIsChildClassOf(cid, c)`, walk `GetObjectsListByClassID(cid)` in
   context order. Skip objects with flag byte `+0xc & 2` (`CK_OBJECT_PRIVATE`). If "current scene" is set,
   also require `ctx.CurrentScene->IsObjectHere(obj)`. Then `group->AddObject(obj)` (duplicates are
   rejected by CKGroup).
5. Return 0.

The off-by-one matters: for class 24 (CKSound) without Derived, it also collects 25 (CKWaveSound). For 30
(Material) and 31 (Texture), c+1 is not a child class, so nothing extra is added.

Port: a group's members are graph object ids of any kind (entities, meshes, materials, textures, behaviours), seeded
from the file's group chunk (else from the level's entity members), kept in the run's state and changed by Add To
Group (a behavioural object, not the group itself, not twice), Remove From Group, Group Clear and this block. Objects
with no class (the engine's pseudo-objects) are not collected; the order is by class, then creation. "Only In
Current Scene" is taken as always true (a map's objects all are in its scene). Group Iterator, Is In Group, Get
Nearest In Group (entities only), Send Message To Group and the group count operation read the same state.

Game (`Levelinit.nmo`): `set MipMap` with classes 30 and 31, `Preload Sound` with class 24. All have
Derived = 0 and current scene = 1.

## Load String

GUID `391555d6:42f2500e`, `Logics.dll`. Execute `0x2549f0c0`, no callback.

- In: `In`. Outs: `Out`, `File Error`. Pin: `File` (String). Pout: `String`.

Execute:
1. `ActivateInput(0,FALSE)`.
2. Resolve the file name with `PathManager->ResolveFileName(name, category 1 = data paths, -1)`.
3. `fopen(name, "r")` in **text mode**.
   - If that fails, fire `File Error`.
   - Otherwise fire `Out` (before reading). The size comes from `fseek/ftell`. It reads with
     `fread(buf, 1, size)`; CRLF becomes LF, so the byte count can be smaller than size. It sets
     `buf[n] = 0`, then `pout String = buf` (n+1 bytes).
4. Return 0.

Game: `Gameplay.nmo` `load Tutorialtext`. `Create String` builds `Text\Tutorial<n>.txt` (data/Text holds
Tutorial1..). Then `Load String.Out -> TT_SplitString` (delimiter `*`), and `File Error -> Op -> TT_Debug`.

## Texture Scroller

GUID `f11d010a:fb1d010a`, `Materials.dll`. Execute `0x255049f0`, no callback. Target: a CKMesh.

- In: `In`. Out: `Out`.
- Pins: 0 `Scroll Vector` (Vector2D), 1 `Channel` (Integer, default -1 = base UVs).

Execute (from the disassembly):
1. `ActivateInput(0,FALSE)` and `ActivateOutput(0)` come first.
2. If the target is not a child of CKCID_MESH (0x20), return `0xa004`.
3. `(su, sv) = Scroll Vector` (default 0,0). `ch = Channel` (default -1).
4. `uv = mesh->GetModifierUVs(&stride, ch)` (vtable `+0x80`). If NULL, return 0.
5. `n = GetVertexCount()` (`+0x8c`). If the mesh is a patch mesh (CKCID 0x35),
   `n = GetModifierUVCount(ch)` (`+0x84`).
6. **Wrap step:** `su -= (float)ftol(uv[0].u)` and `sv -= (float)ftol(uv[0].v)`. ftol truncates toward
   zero. The integer part of vertex 0's current coordinate is removed from the offset for all vertices, so
   the UVs stay near 0.
7. For every vertex: `u += su`, `v += sv`. The step is `stride` bytes.
8. `ModifierUVMove()` (`+0x88`, no arguments pushed). Return 0.

The offset is per execution, not per second. The game scales it with `Per Second` before this BB.

Game:
- `AnimTrafo.nmo` `FlashAnim`: vector from a `Parameter Selector` (resolved value (0.5, 0)), channel -1.
- `Gameplay.nmo` `animate SkyLayer`: vector = `Per Second(Skytranslation)`, channel -1. It is looped
  through `Binary Switch` with delay 1, so it runs every frame while `Skylayer ein?` is true.

## Set Current Slot

GUID `aaaa213a:eaa8d52a`, `Materials.dll`. Execute `0x25501110`, no callback. Target: a CKTexture.

- Pin: `Slot Index`, Integer.

`ActivateInput(0,FALSE)` and `ActivateOutput(0)` come first. If there is no target, return `0xa004`.
Otherwise call `CKBitmapData::SetCurrentSlot(slot)` on the texture's bitmap part (`this+0x50`), which
selects which image slot is shown. Return 0.

Game: `Gameplay.nmo` `Init` (id 12996). Slot 0, fired after a `Set Cell`.

## The transformer animation (AnimTrafo.nmo), measured

`AnimTrafo_MF Script` runs Ring_Open, Up'n Down (with FlashAnim) and Ring_Close; all its objects sit at the
file's origin and are placed at the transformer's world matrix. Measured in the C game (60 frames a second),
counting from the frame the animation's objects are placed (one frame after the ball is taken):

| Frames | What |
|---|---|
| 0–9 | The four `AnimTrafo_Ringpart` move by (-0.5, 0, -0.5) in their `AnimTrafo_Frame0N`, speeding up (≈ t²). |
| 22–142 | `AnimTrafo_Bars` rise to (0, 5.2, 0) by 69, stay to 93 and come back down; the ring parts and `AnimTrafo_Flashfield` move with them. The flash field is shown from 22, hidden at 142. |
| 142–153 | The ring parts move back, speeding up (≈ t²). |
| 147 | The old ball is hidden; the new one is shown at 148 (149 frames after the ball was taken). |
| 153 | All of the animation's objects are hidden. |

While the bars move, `Set Diffuse` sets the material of the new kind's transformer shadow
(`P_Trafo_<kind>_Shadow`, shared by every transformer of that kind) to (1,1,1,1) → (1,1,1,0.196) by the same
progression, so those shadows fade; and from the ring's opening on, `FlashAnim` scrolls the flash field's texture
by (0.5, 0) and (-0.5, 0) in turn, a step every 5 frames (a Delayer of 50 ms, which takes 4 frames, and a link's
delay). The flash field is a horizontal disc of radius 2.46 that rises with the bars.

The old ball is pulled to the transformer's (0, 3, 0) (Set Physics Spring above): half way in 7 frames, 90% in 17,
which a critically damped spring of 0.23 per frame matches. The new ball keeps the old one's orientation. The
transformer's own `P_Trafo_*_MF` and `_Shadow`, hidden in their module file, are shown from the level start and
stay shown throughout.

## TT Scaleable Proximity (as the port runs it)

`TT_Toolbox_RT`, GUID `2a2a63ca:00826175`; the C reference's `bb_tt_scaleable_proximity` (src/bb/bb_interface.c) is
the description, `crates/game/src/proximity.rs` the port. The distance between ObjectA and ObjectB is compared
squared with the Distance pin. The test runs every N frames, N going linearly in the **squared** distance from the
minimum frame delay at the near exactness to the maximum at the far one (truncated, at least 1); the first test
after On happens at once. A test gives Enter (within, and it was not or the block was just switched on), In, Exit
or Out. The module scripts' values:

| Script | Distance | Exactness | Frames |
|---|---|---|---|
| P_Modul_01, 19, 25, 30, 34, 37 wake-up | 50 | 55 / 100 | 10 / 60 |
| P_Modul_03 wake-up | 35 | 35 / 50 | 10 / 60 |
| P_Modul_29 wake-up; its stone-ball test | 80; 4 | 85 / 100; 8 / 20 | 10 / 60; 2 / 30 |
| P_Modul_18 activation; its updraft test | 80; 7 | 85 / 100; 12 / 20 | 10 / 60; 1 / 10 |
| PE_Balloon wake-up; finish | 70; 1 | 75 / 100; 5 / 10 | 10 / 60; 1 / 10 |

# Part 5. Flow blocks, settled by the C oracle

The blocks below had no notes of their own. Their behaviour is taken from the C reference (`src/bb`, itself written
from the disassembly of `Logics.dll`) by the level-2 differential oracle (`crates/script/tests/oracle_blocks.rs`,
docs/drahtseilakt/scripts.md): one synthetic graph per kind, pins swept over edge values, traced frame by frame
against the C. Where a note above already says something (Timer, Per Second), the note is the rule and the C was
found to agree with it. Common points:

- A block reads and clears only the inputs it handles. An input that was active and is not handled **stays
  active**: the next run of the block still sees it (Binary Memory with both inputs on, Parameter Selector with
  several, Timer with In and Loop In, Group Iterator without a group).
- The state that counts frames or steps lives in the block's **output parameter**, as in the original: it survives
  a reset of the script (a reset does not touch parameters) and other blocks read it. A block without that output
  starts from 0 every time.
- "Done" means the block returns `CKBR_OK` (inactive after the run), "stays" means `CKBR_ACTIVATENEXTFRAME`.

## Delayer

`15d472a5:3bea409f`. In `In`; out `Out`; pIn 0 `Time to Wait` (Time, ms); pOut 0 `Elapsed Time` (Time). Each run:
if In is active it is cleared and Elapsed Time set to 0; then `Elapsed Time += delta time` (single precision) is
written; if `Time to Wait <= Elapsed Time` Out fires and the block is done, else it stays. So the frame In fires
already counts: at 1000/60 ms a frame, 50 ms fires in the 3rd run, 500 ms in the 31st; a wait of 0 or less fires in
the first run. Deactivating the script mid-wait and activating it again without a reset resumes the count; a reset
stops the block (it is inactive and its In cleared) but leaves Elapsed Time as it was.

## Counter

`998f000f:f000f899`. Ins `In`, `Loop In`; outs `Out`, `Loop Out`; pIns `Count`, `Start Index`, `Step` (Integers;
Step 0 counts as 1); pOut 0 `Value` (Integer; optional). Each run: if In is active it is cleared and the value
becomes `Start Index`; otherwise Loop In is cleared and the value becomes pOut 0 + Step (0 + Step without the
output). The value is written to pOut 0; then Out fires if `Count <= (value - Start) / Step` (integer division
truncating to zero), else Loop Out. Done. So Count *n* > 0 gives *n* Loop Outs (values Start, Start+Step, ...) and
then Out with pOut 0 = Start + n·Step; Count 0 or less gives Out at once (pOut 0 = Start). A negative Step counts
down the same way. The arithmetic wraps on overflow.

## Sequencer

`42530844:257b6053`, version 2. Ins `Reset`, `In`; outs `Exit Reset`, `Out 1` .. `Out n`; pOut 0 (the current
index, Integer). Each run, In first: if In is active it is cleared, the index = pOut 0 + 1, set to 0 if
`n <= index` (that is, past the last Out), written, and `Out (index + 1)` fires. Then Reset: if active it is
cleared, the index set to -1 and Exit Reset fires. Done. Both in one run: the Out fires and the index ends at -1.
With no Out at all nothing but Exit Reset ever fires.

## Wait For All

`c044a999:fdfefaf7`. Ins `In 0` .. `In n`; out `Out`. Each run: if every input is active they are all cleared and
Out fires (done); otherwise nothing is cleared and the block stays, so the inputs that came are kept until the
others do.

## Identity

`15151652:aeefffd5`. Each pIn *i* is copied to pOut *i*, In cleared, Out fired — only when the block has as many
pIns as pOuts. Otherwise the original throws "Wrong number of Input/Output parameters": nothing happens (In stays
active, no Out).

## Parameter Selector

`63eb2b0c:27e2767e`. Ins `In 0` .. `In n`; pIns 0 .. n; pOut 0. The first active input (lowest index) is cleared
and its pIn copied to pOut 0; the other active inputs stay active. Out fires on every run.

## Binary Memory

`d02d67dd:10211fdd`. Ins `True`, `False`; outs the same. If True is active it is cleared, out `True` fires and
pOut 0 = TRUE; otherwise False is cleared, out `False` fires and pOut 0 = FALSE. A False that came with a True stays
active.

## Test

`17d66d26:726b7dec`. pIn 0 the operator (1 ==, 2 !=, 3 <, 4 <=, 5 >, 6 >=), pIns 1 and 2 the operands. In is
cleared. The operands compare as floats (Integers and Booleans converted), or as strings when either pin is a
String. An operator of 0 or less fires nothing; one above 6 compares false (out `False`).

## Group Iterator

`6050252f:3aa82d40`. Ins `In`, `Loop In`; outs `Out`, `Loop Out`; pIn 0 `Group`; pOut 0 `Element`, pOut 1 `Index`
(else local 0). Without a group (or not a group) Out fires and **no input is cleared**. Otherwise the index is
pOut 1 (local 0) + 1 on Loop In, 0 on In (the input cleared); past the last member Out fires, else Element and Index
are written and Loop Out fires. Done. A block with neither pOut 1 nor local 0 (only seen in made-up graphs) walks
the members by a private index in our runtime (stage 0); the C reads 0 every time and so repeats member 1.

## Wait Message

`4587ffee:4587ffdd`. Ins `In`, `Stop`; out `Out`; pIn 0 `Message`. A run registers on In (cleared) a wait for that
message on the block's target (its owner), and on Stop (cleared) removes the block's waits for it; without a message
type nothing happens at all. The block is done after the run, but a block that waits keeps its graph active (the
`0x40` behaviour flag, docs/ck-runtime.md). The message manager delivers the frame's messages at its end: a waiting
block whose message reached its object (sent to it, to a group it is in, or broadcast) has its Out activated there,
stops waiting and loses the wait; the graph then fires Out's links at the start of its next run (frame N+1 for a
message sent in frame N), without running the block. A reset of the script ends the waiting (the flag is cleared);
a wait that remains then catches its message without effect.

## Iterator

`198f0af9:0268249f`. Target: an array. Ins `In`, `Loop In`; outs `Out`, `Loop Out`; pIns `Starting Row`, `Ending Row`
(-1 = the last), `Backward`; pOut 0 `Row Index`, then one pOut per column from pOut 1. Without an array nothing
happens. With *n* rows and `Starting Row >= n` (so always for an empty array) Out fires at once and no input is
cleared. Otherwise: on In (cleared) the row is Ending Row when backward, else Starting Row, and this is the first
step; on Loop In (cleared) it is Row Index ∓ 1, wrapping round the array (below 0 to *n*-1, past *n*-1 to 0). The
row is written to Row Index. The stop row is one past the end (Ending Row + 1, or 0 past the last row) forward,
Starting Row - 1 (or *n*-1 below 0) backward; reaching it, except on the first step, fires Out (done). Else the row's
cells go to pOut 1.. (a cell only into an output of its type; a row outside the array writes nothing) and Loop Out
fires. So Starting Row above Ending Row walks round the end of the array, and a range covering the whole array
starts again at its first row only by a new In.

## Get Key Row

`49064205:10e72f7a`. Target: an array; pIn 0 the key; outs `Found`, `Not Found`; one pOut per column. The key column
is the **array's own** (saved under identifier `0x4000` of the array: sort order, sort column, key column; -1 or
absent: none), not the first column. An array without a key column logs "No column criterion defined" and nothing
happens (In stays active). Otherwise In is cleared, the first row whose key cell equals the key is copied to the
pOuts and Found fires, else Not Found.

## Arrays: typed cells and the row and column blocks

Settled by the C oracle (`oracle_blocks.rs`: every block below against the upstream runtime on random arrays of all
column kinds, thousands of cases each). A column has a kind: 1 Integer, 2 Float, 3 String, 4 Object, 5 Parameter (the
last holds a value of any type: it goes out as it is and Set Cell stores the parameter; tests and sorting on it find
nothing to compare). Rows and columns are
changed in the run's copy of the array (rewound with the run); the key column belongs to the array and moves with its
columns.

- **Typed key.** A block that searches or tests builds its key from a pin of the column's own type (Integer pin for an
  Integer column, Float for Float, String for String, the Object type itself for Object). A pin of another type gives
  a key of 0 (for strings "", for objects none): the search still runs. Comparisons: 1 equal, 2 not equal, 3 less, 4
  less or equal, 5 greater, 6 greater or equal; any other operation is false. Strings compare byte by byte, objects by
  their ids as 32-bit numbers (none is 0). A start row outside the array, or a column outside it, finds nothing.
- **Typed output.** A cell goes into an output parameter only if the types agree (Integer cell into Integer pout,
  Float into Float, String into String; an Object cell into a pout of an object type, and then only when the object's
  class derives from it, else "none"); otherwise the pout is left as it is. This holds for Get Cell, Get Row, Get Key
  Row, Iterator and Get Highest. Without a target array none of these blocks does anything and In stays active.
- **Set Cell, Change Value If**: a String column takes the parameter's text (a Boolean as TRUE/FALSE, an object by
  name, none as NULL); the other kinds take the parameter's four bytes as they are, whatever its type (an Integer into
  a Float column is the float with those bits; another type's bytes into an Object column are an object id).
- **Set Row**: pin 0 the row, pins 1.. the cells by column, each taken only if its pin is of the column's type. Setting
  `Unique Key Column` (on when absent): if the array has a key column and another row than the one being set already has
  the key (taken from the pin of that column), nothing is written, In is cleared and `Already Present` (output 1)
  fires; otherwise the row (if it exists) is written and Out fires.
- **Insert Column** (Index -1 or past the end: append; Name; Column Type 1..5): In is cleared and Out fires first,
  then, with an array, the column is inserted with default cells (0, 0.0, "", none); a key column at or after the new
  place moves up by one. A column type outside 1..5, or an array of 256 columns, refuses the insert (ours; the C takes
  any number). **Remove Column** (Index): the same order; a key column removed means no key column, a later one moves
  down by one.
- **Sort Rows** (Sorting Column, Ascending): sorts by the column with the quicksort of the C library the original
  was built with (the middle row swapped to the front, partitioned by it, runs of up to eight rows by selection of
  the maximum), so rows with equal keys end in the order the original gives them. The column's order: integers and
  objects by a wrapped 32-bit difference, floats by the sign of the difference, strings byte by byte. In is cleared,
  Out fires.
- **Test Cell** (Row, Column, Operation, reference): True or False; the input is *not* cleared. **Get Highest**
  (Column): the row with the highest cell (the first of equal ones), `Row Number` -1 for none, the row's cells to the
  outputs 1.., `Found` or `Not Found`. **Value Count** (Column default 1, Operator, reference): the number of rows
  that satisfy it. **Remove Key Row** (reference): removes the first row whose key cell equals it; `Removed` or `Not
  Found`; without a key column nothing happens. **Remove Row If** (Column, Operation, reference): removes every row
  that satisfies it, `Number Removed`; `Found` if any. **Change Value If** (Column, Operation, reference, new value):
  sets the cell of every row that satisfies it, `Number Changed`; `Found` if any. **Row Search** is typed like the
  others and a negative first row finds nothing.
- **Move Row** (Src Row, Dest Row): *no C reference exists* (the upstream runtime lacks the block); from its pins and
  the array operation of that name: the row at Src is taken out and put at Dest (both inside the array, else nothing
  changes); Out fires. Not checked against the original.

## Attributes

Settled on real files and by the C oracle's notes (the oracle's own graphs cannot name an attribute: its attribute
types come from a file's manager chunk). An object carries attributes: a number (the file's attribute index, see
[ck-runtime.md](ck-runtime.md), identifier `0x11`) and a value. The runtime lists them in the order they were given
(the file's first, then those blocks set), and deletes an object's with it; every run has its own lists (rewound
with it). Caps: 64 attributes per object, indices 0 to 65535, 65536 in all.

- **Has Attribute** (pin Attribute, default -1): In is cleared; if the target (a behavioural object) has the attribute
  `True` fires and pout 0 gets its value, else `False`. A pin that is not a number (a name) names no attribute.
- **Set Attribute** (Attribute, Attribute Value): In is cleared and Out fires first; with a target the attribute is
  added if the object lacks it, and takes the value of pin 1. (The C refuses an index the attribute manager does not
  know; without the manager's chunk any index in range is accepted here.)
- **Remove Attribute**: In cleared, Out fires; the attribute is dropped if the target has it.
- **Objects With Attribute Iterator** (Attribute): over every object that has it, in the order they got it, not
  filtered by scene; pout 0 the object, pout 1 the value; the index is kept in local 0. In starts at 0, Loop In goes
  on by one; past the last, `Out` fires and the index is left.

## Messages

A message is a record: its type (by name), the sender (the sending block's target, its script's object unless the
block has a target of its own), the receiver and the parameters attached (pins of the sending block, referenced:
whoever reads them later reads their value then). The message manager delivers a frame's messages at the start of
the next (docs/ck-runtime.md); an object keeps the messages that reached it for that frame only. In the C reference
an object keeps them only when it waits: a matching Wait Message, or a Switch On Message on it (its callback marks
the object on load and attach). Ours keeps them for every object: blocks that read them are on waiting objects.

- **Send Message** `a20e8d5b:df002150`: pIns Message, Dest, then the data. In cleared, Out fired; to the object, or
  to each member of a group.
- **Send Message To Group** `5f906952:6df11649`: pIns Message, Group, then the data (from pIn 1 before version 2).
- **Broadcast Message** `3d6c4ae1:72ae2cd6` (C, version 2): pIn Message, pIn Class (a Class ID; default Behavioral
  Object 19), then the data; to every object whose class is that class or derives from it. Without a sender (no
  target and no owner) nothing happens and In stays active. In cleared, Out fired.
- **Switch On Message** `1bb23f1d:17ff14b9` (C): ins On, Off; pIns Message 0..n; outs one per pIn. Off active:
  cleared, done, nothing fired. Otherwise On cleared if active, and out *i* fires for every message of pIn *i*'s
  type the owner received this frame; the block stays active (it polls every frame).
- **Get Message Data** `45875fee:45875fdd` (no notes and no C: from its saved interface, pIn Message, pOut 0
  Sender, pOuts 1.. the data, local "Can get several Message per Frame"): In cleared; the first message of that type
  that reached the target this frame writes its sender to pOut 0 and its parameters, in order, to pOuts 1..; Out
  fires whether one came or not. The local's second message per frame is not modelled.

The oracle (`switch_on_message`, Send Message and Broadcast Message as drivers, classes 19 and 33) agrees with the C.

## Execute Script: ours differs from the C on purpose

The C reference files an activation request (the script starts in the next frame, like Activate Script). Our notes
(docs/drahtseilakt/scripts.md, stage 0; scripts-port-plan.md item 18) have it run the script at once inside the
caller's frame, as `CKBehavior::Execute` does; the notes win, so the oracle does not compare this block.

## Input blocks (Controllers.dll and friends): keys, mouse, text

Source: the C reference `src/bb/bb_controllers.c`, `bb_tt_misc.c` (Input String) and `bb_environment.c` (Show Mouse
Cursor), read, not copied. All of them read the keyboard and the mouse of **the current tick from the run's input
stream** (replay format 11, `docs/drahtseilakt/scripts.md` "Input"), never from the live machine. Keys are
DirectInput scan codes (0 to 255). "Just pressed" means down in this tick and up in the tick before. The mouse is in
the original's 640x480 frame; buttons are left, right, middle.

### Key Event `1af2274b:6b8c1524`

Inputs On, Off; outputs Pressed, Released; input parameters: one per key waited (any number, a key code each);
output parameter Key State. Off (checked first) clears itself and ends the block. On clears itself and sets Key State
to FALSE. Then, each tick while active: if all the keys of the input parameters are down and the state is FALSE, the
state becomes TRUE and Pressed fires; if one is up and the state is TRUE, it becomes FALSE and Released fires. Key
State is written each tick. The block stays active. With no key pins the "all keys down" test holds from the start
(Pressed fires once after On).

### Key Waiter `016d010b:017d010b`

Input In; output Out; input parameter Key; output parameter Key; local 0 "Wait For Any Key". While the input is
active: with Wait For Any Key, the first key (lowest scan code, 1 to 255) just pressed clears the input, fires Out and
gives its code to the output parameter Key; otherwise, if the key of the input parameter is down, the input clears
and Out fires. If nothing matches the block stays active and looks again next tick.

### Switch On Key `1f49612e:6cd63de0`

Not in the C reference. Modelled on Switch On Message: inputs On, Off; one input parameter per output, each a key
code; output i fires in the tick its key is just pressed. Off ends the block, On is cleared; the block stays active.
(No instance of it was checked against a real file's pin layout; any extra pins are keys by position.)

### Mouse Waiter `014d010b:015d010b`

Inputs On, Off; locals 0 "Off input exists" (default 1), 1 the mask of outputs that exist (default 0xffff; bit 0 Move,
then pairs of bits Pressed/Released for left (bits 1, 2), right (3, 4), middle (5, 6); bit 7 wheel, not fired here);
outputs exist in that order, only those whose bit is set. Output parameters: Mouse Position (Vector 2D), Wheel
Direction (0 here: the stream has no wheel), Left Button, Middle Button, Right Button. On clears itself; Off (when it
exists) clears itself and ends. Each tick while active: the previous position and button values are those the output
parameters held; the position and the three buttons are written; Move fires when the position changed, a Pressed
output when the button went from up to down, Released when from down to up. The block stays active.

### Input String `693e7e4b:100104c4`

Inputs On, Off; outputs On, Off, Updated String (called the 3 outputs 0, 1, 2); input parameters Reset String and End
Key (used when Multiline); output parameters String, Key. Locals: 0 Max Size (default 32; this port clamps it to
1..256, a bounded string), 1 Keyboard Repetition (no source: ignored), 2 Multiline, 3 Disable Keyboard Section (bit 0
the numeric keypad, bit 1 the cursor block), 4 Use Caret, 5 the caret, 6 the buffer, 7 "on". Behaviour:

- On: "on" is set; the text becomes Reset String (cut to Max Size), the caret goes to its end; String is the text
  (with a backspace character `\b` at the caret when Use Caret); output On fires. Off: when "on", String is the text
  without the caret mark, "on" clears; output Off fires; the block ends.
- Each tick otherwise, the keys just pressed are walked in scan code order; each is given to the output parameter
  Key. A single-line block ends on Return or keypad Enter (String = the text, output Off fires, the block ends); a
  multiline one ends on the End Key pin's key. Else: Backspace deletes before the caret, Delete after it; Left and
  Right move the caret (unless section bit 1); the keypad keys insert their characters (unless section bit 0); Return
  in a multiline block inserts a newline; other keys insert the character of a US layout (Shift or right Shift held:
  the shifted one), nothing for keys without a character and for Escape. Home, End, Page Up and Page Down do nothing.
- A character inserted at the caret moves the tail right; the text is cut at Max Size. A full text takes no character
  at its end. After any edit String is updated and Updated String fires.

### Show Mouse Cursor `16f6368f:506b60fc`

Input In; output Out; input parameter Show (Boolean). In clears itself, fires Out, and shows or hides the system's
cursor by Show. The cursor is the window's only: no gameplay effect.

### Mouse Cursor `140c3eac:6e502a81`, Mouse Cursor System `2483576d:57c66324`

Not in the C reference. By policy the engine draws its own interface: these only fire Out on In, and when their
first input parameter is a Boolean it shows or hides the system cursor as Show Mouse Cursor does. Nothing else.

## Cameras

Cameras.dll blocks (`crates/script/src/blocks/cameras.rs`). They change the view only, never the simulation.

- **Get Current Camera**: on its input, writes the camera object (none: the game's own camera) and its position.
- **Set As Active Camera**: attaches the view to the target object; the view sits at its place and looks along its z axis.
- **Set Clipping Planes**: sets near and far (defaults 0.1 and 200); applied only when near > 0 and far > near.
- **Set FOV** (`aaaacccc:aaaacccc`): pin Angle in radians across the 4:3 view (InGameCam's is 1.0123); the host clamps it to 1..179 degrees. Applied to the active view whatever the target.
- A run starts with the original camera; a new run (restart, or a rewind that rebuilds the run) starts without these changes.

## Text (script text boxes: Text Display, 2D Text, font blocks)

Source: `docs/fonts.md` sections 6.1-6.5, and the old C `src/bb/bb_interface.c` where the notes are silent. The
original draws a map's text with its texture font (2D Text) or a GDI sprite (Text Display); we draw every box in
the engine's own pixel font, so only what decides *where, how big, what colour* is kept.

- **Box.** A block that shows text has a box: text, rectangle in the 640x480 frame, wanted line height, colour,
  weight, alignment. It exists while the block is on. The block re-sends it every tick (a Text Display and a 2D Text
  both stay active until Off); the engine replaces the box of that block, so a changed text shows at once.
- **Gone when:** Off input (Exit Off fires, the box goes: fonts.md 6.4 "no callback is registered, so the text is
  gone from the next frame"); the block's script is deactivated, reset or deleted; the run restarts or rewinds (the
  boxes live in the run's state, not the renderer's).
- **Text Display** (fonts.md 6.5): pins Offset (the sprite's top left, frame pixels), Color, Alignment (CK sprite
  text enum Center 1, Left 2, Right 4, Top 8, Bottom 16, VCenter 32, HCenter 64), Size, Text and extra pins. Text =
  each text pin's string followed by a space. The sprite is the local Sprite Size (default 320x32). The C
  (`bb_text_display`) only handles On/Off and draws nothing; the notes' semantics are followed. *Not in the notes:*
  the unit of Size: taken as points at 96 dpi (px = size * 4/3); an unset Size (0) is 13 px. A colour pin that
  holds no colour is white.
- **2D Text** (fonts.md 6.4): needs a 2D entity (class 27-29) as target, else nothing is drawn (the C pushes the
  draw only `if (ck_2dentity(...))`). On -> Exit On, Off -> Exit Off. Pins: Font, Text, Alignment (Center 0, Left
  1, Right 2, Top 4, Bottom 8), Margins (left, top, right, bottom; default 2), Offset. The 2D entity's own
  position and size are not known to the engine yet (3.5, second part): the box is the whole frame inside the
  margins plus the offset. Output Line Count = lines split at '\n'; Text Extents is not written. Text Properties
  flags are not read: text always wraps at the box's width.
- **Fonts.** TT CreateFontEx (name, texture): no name -> nothing; no texture -> no font (fonts.md 6.1 step 1); else
  a font of that name, output = its 1-based index. Create System Font: a font by name, bold when Font Weight >= 600,
  output Font Created and Success; with no room (64 fonts) nothing, and no Error output either (fonts.md 6.3).
  Set Font Properties: Out first; a Font index that does not exist is left alone; Scale and Color are kept.
  A second font of an existing name replaces it (a restarted level makes its fonts again). A font stands for a line
  height of 32 px * scale.y (the glyph cell of the original's font texture); no font = scale 0.5, white. Names
  map to one face of ours: the name only identifies the font.
- **Fit.** Wrapped at spaces and '\n', then shrunk (to 0.6 frame pixels per font pixel), then cut at the box's
  edge. Caps: 64 boxes, 1024 characters per box, 2048 glyphs drawn per frame.
- **3D Text, Movie Player:** known, do nothing (inputs taken, one log note). 3D Text would need the camera's
  projection of the object; left for later.
- Text never affects the simulation: the boxes are only read by the renderer.

## The game's facade

What a map's scripts read or wait for that the map does not hold: the game's own objects, arrays and messages
(plan items 1.4 and 3.9). Layouts are those saved in `Gameplay.nmo` and `Balls.nmo`; the engine fills the cells
from the run (`crates/game/src/facade.rs`). Everything is read-only for the rules: a script's Set Cell on a game
array is kept in the run's scripts only (at most 256 cells) and never reaches the simulation, so replays and the
original levels (which carry no scripts) stay exact. A name the map holds itself is never replaced.

Objects the graph gets when the map lacks them: `Ball`, `Ball_Stone`, `Ball_Paper`, `Ball_Wood` (the live ball),
`Ball_Pos_Frame`, and as plain named objects `Particle_Flames`, `All_Gameplay`, `All_Sound`, `All_Balls`, `PH`,
`Floor`, `Light_Ingame`, `InGameCam`; tables `CurrentLevel`, `IngameParameter`, `Energy`, `Physicalize_GameBall`.

| Array | Columns (kind) | Cells |
|---|---|---|
| `CurrentLevel` (1 row) | Level ID (int), ActiveBall (object), Ball_Pos_Frame (object), CurrentResetpoint, Activation Phase? (bool), Points (int) | 0 (a map has no table number); the object of the ball's kind; the ball's frame; none; the ball is in play; the run's points |
| `IngameParameter` (1 row) | Activetrafo (object), Activate Sector, Deactivate Sector (int), Tutorial activ?, RollSound activate? (bool) | none; the sector now; the sector left last (0 before a change); false; true |
| `Energy` (1 row) | Points, Lifes, StartPoints, StartLifes (int), Timefactor (float), LifeBonus (int) | points; lives; 1000; 3; 500 (ms between two points counted off); 200 |
| `Physicalize_GameBall` (3 rows) | Ballname, Friction, Elasticity, Mass, CollGroup, Linear Damp, Rot Damp, Force | Balls.nmo's rows: Ball_Paper, Ball_Stone, Ball_Wood |

Messages are read from the file's message manager (a Message parameter saved as an index is resolved to its name).
The engine broadcasts them (to every object, no sender) before the tick's scripts run, from what changed in the run
since the tick before, so a Wait Message hears them in that tick.

| Game event | Message name(s), in order | Original sender (Gameplay.nmo) |
|---|---|---|
| level start (first tick) | `Start Music` | Init Ingame |
| checkpoint reached (sector rises) | `Checkpoint reached` | the sector scripts |
| ball lost (lives drop) | `Ball Off`, `Sub Life`; with the last life also `Game Over` | BallManager; `Sub Life` is waited by Energy |
| extra life (lives rise) | `Life_Up` | the points/lives script |
| extra point (points rise, no life lost) | `Extrapoint` | waited by the points display |
| level finished | `Level_Finish`, `End Level` | the finish script |
| ball changed (transformer) | `Trafo` | Trafo Manager |

Also emitted (2026-11-12; names read from Gameplay.nmo and Levelinit.nmo): `last Checkpoint reached` after `Checkpoint reached`
when the last checkpoint of a level that has checkpoints is passed; `Dead` when the ball starts to die (before `Ball Off`);
`Part_Loaded` once, before `Start Music` in the first tick (the original sends it for each part while loading);
`Pause Level` when the pause menu opens over the run and `Unpause Level` then `Refresh` when it closes with Continue
(sent by the app through `Sim::notify_pause`, heard by the scripts at their next tick; a pause is not in a replay's
input, so a script that waits for these hears them only in a live run).
Not emitted: the `BallNav` / `CamNav` activate messages (they belong to the game's own scripts, not run here).
Open points: `CurrentLevel`'s Level ID is 0 because a map carries no level number; `Timefactor` is the counter's
period in milliseconds (the original's parameter was not read).
