# Mods

Mods add behaviour and features to the game — a sticky ball, a timer with splits, a turned view — and every player
switches each of them on or off. They are written in our own format: **data only**. A mod is a small text file of
rules built from a fixed library of the engine's building blocks; the game's runtime does what the rules say, as the
original's behaviour graphs are data driven by its runtime. Nothing in a mod is ever run as code (decisions.md, "Our
own data-only mod format").

The game ships its own mods, the *factory mods* (`assets/mods/`), written in this very format: they are the features
and the examples a modder starts from. A player's mods go into the data folder's `mods/`. Sound themes, texture and
model packs and levels are not mods: they have their own menus and folders ([sound-themes.md](sound-themes.md),
[packs.md](packs.md), [levels.md](levels.md)).

## Switching mods on and off

*Options → Extended → Mods* (the extended mode only; the classic mode runs no mod and plays as the original). The
screen lists every mod found: factory mods first, in a debug run the development mods (god-mode, below), then the
player's, each with *Yes* or *No*, or *refused* for a mod that could not be read (the help under the list says why).
A long list scrolls, with a bar beside it and "▲ / ▼ N more" (menu.md). Left and right switch the chosen mod; Enter
opens its page: *Switched on?*, its options (a key option shows its key), its description, author, version and
licence, and a warning when it changes the rules. The settings keep `mod.<key> on|off` and
`mod.<key>.<option> <value>`. A mod's view side takes effect at once; one that changes the run from the next start of
the level.

## The factory mods

| Mod | After the BML mod | What it does | Changes the run |
|---|---|---|---|
| `sticky-ball` | BallSticky, StickyAccess | what the ball touches holds it; pushing into a wall climbs it; options: stickiness, wooden ball only | yes (modded) |
| `physics-lab` | PhysicsEditor | gravity (0–60, original 20) and the physics' time factor (0.5–4, original 2) | yes (modded) |
| `dynamic-fov` | DynamicFov | the view widens with the ball's speed; option: strength | no |
| `free-view` | FreeViewRotation, AdvancedTravelCam | the mouse turns the view about the ball, a key turns it back; camera distance | no |
| `segment-timer` | Segment, RealTimeMeter, WatchYourTime | the run's time, the real time, each sector's split against the best run (green ahead, red behind) | no |
| `info-overlay` | PositionViewer, EnergyItemCounter, SysInfoDisplay | position, speed, extra points and lives left in the sector, frames a second, clock (UTC) | no |
| `quick-restart` | SpeedrunShortcuts | a key (Backspace, Delete, F4 or Q) starts the level again at once | no |
| `screenshot` | ScreenCapturer | a key (F12, F10, Print Screen or P) saves the frame as a PNG in the data folder's `screenshots/` | no |
| `spirit-trail` | SpiritTrail | the level's best run rolls beside this one as a translucent ball; option: opacity | no |
| `blackout` | Blackout | the level goes dark, a faint light over the ball; options: darkness, light | no |
| `smooth-camera` | AdvancedTravelCam (cinematic smoothing) | the camera eases into the view's turns, distance and field of view, and the free camera glides; option: smoothness | no |
| `interference` | Interference | translucent decoy balls wander around the ball; option: how many | no |
| `deformed-ball` | DeformedWB | the wooden ball's look is squeezed and stretched at random (the physics keeps its round ball); option: amount | no |
| `play-time` | WatchYourTime | the time played, per level and in all, on screen and kept in the data folder's `playtime.txt`; option: show it | no |
| `sector-jump` | SpeedrunShortcuts (set sector) | keys (F7 / PageUp, F6 / PageDown) start the level again at the next or the previous sector | yes (modded) |
| `free-steering` | FreeViewRotation (the steering too) | like `free-view`, but the arrows push along the turned view; the view's turn is recorded in the replay | yes (modded) |

## God mode

*2026-11-03, the owner's decision* (decisions.md, "God mode is a development mod"). What were the extended mode's
practice tools and endless lives are one mod, `god-mode`, written in this very format from the building blocks below.
It is a **development mod**: it lies in `assets/mods-dev/`, which the game reads only when it runs with `--debug`, and a
release's package leaves that folder out, so the game is played without it. Its page (Options → Extended → Mods →
God mode) shows its switches and keys; while it is on, its keys are shown in the bottom-left corner (option *Keys on
screen?*), and the sector and tick in the top-left.

| Option | Default | What |
|---|---|---|
| Endless lives? | yes | the last ball is never lost |
| Seconds back | 5 | how far *Back* goes (1–30) |
| Back | F3 (B, Z) | so many seconds back; pressed again, further (the game keeps the last half minute, a moment a second) |
| Save / Load | F5 (K) / F9 (L) | keep the moment, go back to it; a lost run waits for Load |
| Restart | R (T) | the level again |
| Slower / Faster | `[` (`-`) / `]` (`=`) | the game at half the speed, down to an eighth, and back |
| Free camera | F2 (C) | the camera flies (arrows fly and turn, A/D sideways, PageUp/PageDown, Home/End, Shift faster); the run waits |
| Keys on screen? | yes | the hint with the keys |

God mode changes the run: its runs are modded (below) — kept, but they open no level and are never a best run or a
highscore — and its replay names it, so the replay plays only in a debug run with the same god-mode. The old settings
`extended.tools` and `extended.endless-lives` are no longer read.

## Modded runs and replays

A mod that changes the run (any *run action*, below) makes the run **modded**: it is kept, but never the level's best
run, never a highscore, never compared by the ghost or the splits, and it does not open the next level
(`Level_NN-<ticks>-modded.replay`; a debug run's "All levels open" opens them all). Its replay names each such mod in its header —

    mod sticky-ball 9c41f0a2be1377d0 strength=60 wood_only=off

— its key, the hash of its mod.txt and its option values, so the replay plays back the same run, and only with that
very mod: a replay whose mod is missing or has changed, or whose options are missing, unknown or out of range, is
refused with the reason. A classic run's replay has no such
line and is exactly as before. A mod that only changes what is seen (the camera, the HUD, the light) leaves the run
classic.

## Write your own mod

Make a folder in the data folder's `mods/` (Linux `~/.local/share/drahtseilakt/mods/`, Windows
`%APPDATA%\drahtseilakt\mods\`, macOS `~/Library/Application Support/drahtseilakt/mods/`) with a plain name
(letters, digits, `-`, `_`), and put a `mod.txt` in it. Turn the extended mode on and find the mod on the Mods
screen. The factory mods in `assets/mods/` are the examples; copy one and change it.

The smallest mod:

    name.en Moon gravity
    on start
    do world.gravity 4

### The file

UTF-8 text, one statement a line, `#` begins a comment line. In this order:

- **Header** — `name.<lang>` (at least one; languages `en ru de zh fr es it`), `description.<lang>`, `author`,
  `version`, `licence`, `requires <game version>` (e.g. `0.1`; a newer one than the game refuses the mod).
- **Options** the player sets on the mod's page:
  - `option <name> switch on|off`
  - `option <name> number <min> <max> <step> <default>` (within ±1,000,000, a step of 0.001 or more, the default on
    the steps from `min`)
  - `option <name> key <Key1,Key2,...>` — keys by winit's names (`KeyQ`, `F12`, `Backspace`, `Home`...); the first
    is the default. The game's own keys are refused: Escape, Enter, the arrows, Shift, Space, Tab, Alt, F11 and
    Backquote; a key the player bound on the Controls page stays the game's. (R, F2, F3, F5, F9, `[` `]` were the
    practice tools'; they are god-mode's keys now and free for any mod.)
- **Labels** — `label <name> <lang> <text>`: an option's label is named like the option; HUD lines name theirs.
- **Variables** — `var <name> <number>`: numbers the rules keep, from their first value at each start.
- **Rules** — `on <event>`, then any `if <condition>` lines (all must hold), then `do <action>` lines, done in order.

Names (options, labels, variables) are lower case letters, digits and `_`, beginning with a letter.

### Events

The run's events, in the run's ticks (deterministic): `start` (before the first tick), `tick` (every tick, before the
physics), `sector` (a checkpoint reached), `lost` (the ball lost), `newball`, `transform`, `extra` (an extra point or
life taken), `finish`.

The view's events: `frame` (every frame drawn), `key <option>` (the key of one of the mod's key options pressed).

### Values

| Value | | Value | |
|---|---|---|---|
| `ball.x` `ball.y` `ball.z` | where the ball is | `real.time` | real seconds since the run began * |
| `ball.speed` `ball.vy` | its speed, its vertical speed | `frame.dt` `frame.fps` | the frame's length, frames a second * |
| `ball.kind` | 0 wood, 1 stone, 2 paper | `clock.hour` `clock.minute` `clock.second` | the clock, UTC * |
| `ball.touching` `ball.wall` | it touches something; something steep | `mouse.dx` `mouse.dy` | the mouse's motion this frame * |
| `run.tick` `run.time` | the run's ticks, seconds | `extras.points` `extras.points_all` | extra points left in the sector, and all * |
| `practice.speed` | the game's speed (1, or down to 1/8) * | `practice.saved` | a moment is saved (1 or 0) * |
| `run.sector` `run.lives` `run.points` | sector (1 first), lives, points | `extras.lives` `extras.lives_all` | extra lives left, and all * |
| `input.turn` | the view's turn handed on by `view.turn`, degrees | `playtime.level` `playtime.all` | seconds played of this level, of all levels * |
| `input.forward` `input.backward` `input.left` `input.right` `input.rotation` `input.overview` | the game's keys held (1 or 0) | `best.time` | the level's best time, seconds (-1: none) * |

`opt.<name>` is an option's value; a variable is read by its name. Values marked * are the view's: a rule on a run's
event cannot read them (the run must not depend on what is seen), nor a variable a view rule sets.

### Expressions

Numbers, values, `+ - * /`, comparisons `< <= > >= == !=`, `and or not`, `true false`, parentheses, and the functions
`min(a, b)`, `max(a, b)`, `abs(x)`, `clamp(x, lo, hi)`, `if(c, a, b)`, `floor(x)`, `sqrt(x)`. A condition holds when
it is not 0. Every expression is total: a division by zero is 0, so is anything not a finite number; there are no
loops; nesting and size are capped.

### Actions

Run actions (only on a run's event; they make the mod *modded*):

- `do ball.stick <0..1>` — the ball holds to what it touches with this strength, against gravity, and a push into a
  wall turns up along it.
- `do world.gravity <0..60>` — gravity's pull (the original's 20).
- `do world.time <0.5..4>` — the physics' time factor (the original's 2).
- `do lives.endless <0|1>` — the last ball is never lost (god-mode).
- `do steer.yaw <degrees>` — the arrows' push turned about the vertical by so many degrees (the camera turns by the same
  rotation, so the arrows keep pushing along a turned view). With `input.turn` it follows the view (below).

Practice deeds (only on `key`; they reach into the run from outside it, so a mod with one is modded too, and while it
is on the game keeps the run's moments, one a second for the last half minute):

- `do rewind <1..30>` — so many seconds back; `do save`, `do load` — keep the moment, back to it;
- `do slower`, `do faster` — the game's speed halved (down to 1/8) or doubled (up to as it is);
- `do freecam` — the free camera taken up or put down.

A jump (only on `key`; it makes the mod modded, without the practice's moments):

- `do sector.set <n>` — the run begins again at sector n (1 is the first), with that sector's ball, as `--sector` does;
  the replay names the start. The next ordinary restart is the level's start again.

View actions (only on `frame` or `key`):

- `do camera.fov <degrees>` — the view widened by so many degrees (-30 to 60).
- `do camera.distance <0.5..2>` — the camera nearer or farther.
- `do camera.yaw <degrees>` — the view turned about the ball (the arrows keep pushing along the camera's own
  heading, unless the mod also hands the turn to the run: `view.turn` and `steer.yaw`).
- `do camera.smooth <0..1>` — the camera eased: its turn, distance and field of view, and the free camera, follow what
  is asked with a time constant of 0.05 s (0) to 0.65 s (1) instead of at once.
- `do decoys <0..8>` — so many translucent copies of the ball wander around it. `do ball.deform <0..1>` — the wooden
  ball's drawn shape squeezed and stretched at random (0 round, 1 by a quarter); the physics' ball stays round.
- `do playtime` — this frame counts as time played (on `frame`): the level's and the total play time, kept in the data
  folder's `playtime.txt` (one line a level, written every ten seconds and when a menu opens). Read as
  `playtime.level` and `playtime.all`.
- `do view.turn <degrees>` — the view's turn handed to the run as whole degrees (0 to 359): the run reads it as
  `input.turn`. It is a second input of the run, recorded in the replay as `turn <tick> <degrees>` lines where it
  changes (none in a run that does not use it), so a replay plays the same steering without a mouse.
- `do light <0..1>` — the level's light; `do lantern <0..1>` — a light over the ball.
- `do show <corner> <label|-> <format> <value>[, <value>[, <value>]]` — a HUD line this frame; corners `top-left
  top-right bottom-left bottom-right`; formats `time` (mm:ss.hh from seconds), `int`, `dec1`, `dec2`, `clock` (hh:mm),
  `ratio` (a/b).
- `do splits <corner>` — the sectors' splits against the best run.
- `do keys <corner>` — the mod's own key options as HUD lines, "label: key" (god-mode's hint).
- `do ghost <0..1>` — the best run as a translucent ball, so opaque.
- `do screenshot`, `do restart` — only on `key` (never every frame); one screenshot a second at most.

Both: `do set <variable> = <expression>`. A variable a view rule sets belongs to the view; the run's rules may not set
or read it.

### Examples

The sticky ball (`assets/mods/sticky-ball/mod.txt`, its rule):

    option strength number 10 100 10 60
    option wood_only switch off
    on tick
    do ball.stick if(opt.wood_only and ball.kind != 0, 0, opt.strength / 100)

The free view: a view variable turned by the mouse, a key that turns it back:

    option sensitivity number 1 20 1 4
    option reset key KeyV,KeyB,KeyN,Home
    var yaw 0
    on frame
    do set yaw = yaw + mouse.dx * opt.sensitivity / 20
    do camera.yaw yaw
    on key reset
    do set yaw = 0

A speed warning, shown only above 30:

    name.en Speed warning
    label fast en Fast!
    on frame
    if ball.speed > 30
    do show bottom-right fast dec1 ball.speed

## What a mod may hold, and the caps

A mod comes from a stranger, so the game reads it as hostile and fails closed:

- the mod's folder holds only `mod.txt` and, at most, a `readme.txt`/`README.md` and a `licence.txt`/`license.txt`
  (64 KiB each, never read by the game). A link, a sub-folder or any other file — a DLL, a script, a picture —
  refuses the whole mod;
- mod.txt: UTF-8, at most 32 KiB and 512 lines; every line must be one the format knows (an unknown key, event, value,
  action, option, corner or format refuses the mod, the error naming the line); texts at most 160 characters, no
  control characters or bidirectional overrides;
- at most 16 options, 32 variables, 128 labels, 64 rules, 8 conditions and 16 actions a rule, 256 actions; an
  expression at most 64 parts and 16 deep; a key option at most 8 keys; at most 32 mods a folder;
- a work budget: the expression nodes of all the mods switched on together at most 20,000 (each mod's are counted
  when it is read); a mod that would pass it is not switched on, and the Mods screen says why;
- no paths, no links, no file names, no network: a mod can only name the building blocks above.

## Not yet (queued)

- **Companion / two balls** (DualBallControl): a second ball the keys switch to needs a second player body in the
  run (the physics, the steering, the camera, the checkpoints and the death all know one ball), a building block of its
  own, with the switch recorded in the replay. The original's mod finds a dual map by a second start object
  (`PS_FourFlames_01_Dual`); none of the community maps at hand is known to have one (their files are compressed, so
  a plain search proves nothing). Left for its own change.
- **A sticky ball kind** of its own with its transformer (BallSticky's `P_Trafo_Sticky`): today any ball can be made
  sticky.
- **Teleport to a spawn point** (NewSpawn): god-mode's save and load and `--sector` cover the practice.
- A fuzz target for mod.txt (the parser is strict, capped and unit-tested; fuzzing comes with the release work).

## The BML mods surveyed

Thirty BML/BML+ mods (`ref/Mods`, Windows DLLs) were read statically — their strings and readmes only; none was run.
What each does, whether we have it, and how it maps to our engine:

| BML mod | What it does | Ours |
|---|---|---|
| AdvancedTravelCam | a travel camera to fly anywhere, pick a point with the mouse, cinematic smoothing | god-mode's free camera (F2); `free-view` (distance, turn); factory `smooth-camera` (cinematic smoothing) |
| BallanceOptiFine | forces shadows and a transitional column onto old custom maps; texture format | the renderer draws the ball's shadow on every floor; not a mod |
| BaseCmoCfg | edits base.cmo's GameSettings (God mode, Debug) | `--debug` and its god-mode (endless lives and the rest) |
| Blackout | darkens everything, a faint light above the ball | factory `blackout` |
| Coredump | dumps the running Virtools context into an .nmo | not needed (our runtime is ours; `--export` writes levels) |
| CursorClipper | keeps the cursor inside the window | not needed: a run takes nothing from the cursor, `mouse.dx` reads raw motion |
| DeformedWB | deforms the wooden ball's look at random | factory `deformed-ball` |
| DualBallControl | dual-ball maps, a key switches the ball | queued (needs a second player body) |
| DynamicFov | the field of view follows the ball's speed | factory `dynamic-fov`; the fixed field of view is in Graphics |
| EnergyItemCounter | extra points and lives left in the sector | factory `info-overlay` |
| ExtraSector | loads maps with more than eight sectors | the engine: up to 64 sectors, no loader needed |
| FontCraft | another font for the interface | our own interface and freely licensed fonts, any script; not a mod |
| FreeViewRotation | the mouse turns the view | factory `free-view` (the view only), `free-steering` (the steering follows; modded) |
| Interference | translucent decoy balls around the ball | factory `interference` |
| LanternFader | alpha test on the lanterns' material against frame drops | not needed (our renderer) |
| MapScripts | runs maps' callback scripts | planned: the map-script interpreter (scripts.md); not a mod |
| NewSpawn | set a spawn point, teleport to it, record a spirit ball | god-mode (F5/F9), `--sector`, `--at`; `spirit-trail` |
| PhysicsEditor | gravity, physics time factor, game speed | factory `physics-lab` (modded); the game speed is god-mode's `[` `]` |
| PositionViewer | the ball's coordinates | factory `info-overlay` |
| RealTimeMeter | real time of play, across levels | factory `segment-timer` (real time) |
| ScreenCapturer | a key saves a screenshot (PNG/JPEG, clipboard) | factory `screenshot` (PNG; no clipboard) |
| Segment | each sector's time against the record | factory `segment-timer` |
| SpeedrunShortcuts | cheat mode, set spawn, restart, set sector | factory `quick-restart`; god-mode; `sector-jump` (set sector; modded) |
| SpiritTrail | the record run as a translucent ball | factory `spirit-trail` |
| StickyAccess | switch to the sticky ball under cheat mode | factory `sticky-ball` |
| SysInfoDisplay | system time and battery on screen | factory `info-overlay` (clock, UTC); no battery |
| TASSupport | record and play input, skip rendering, exit at the end | the engine: replays are deterministic input recordings (`--record`, `--replay`, `--headless`), god-mode's slow motion |
| WatchYourTime | time played per level | factory `play-time` (`playtime.txt`; also `segment-timer`) |
| BallSticky | a sticky ball type that climbs walls | factory `sticky-ball` (modded) |
| BMLModuls | new modules (wooden drop and rotating boards) | level parts, not mods: the open format's parts ([open-format.md](open-format.md)) |

The original's limits these mods work around (frame rate cap, 4:3, low resolutions, eight sectors, a bitmap font,
fragile ball-change timing) are not ours to copy.
