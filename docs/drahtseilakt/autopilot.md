# The test player and its hints

`--autopilot` lets a test player play a level without a window: it finds a way through the level's floors and
transformers and steers the ball along it, trying each choice of arrows on a copy of the run. `--follow FILE`
makes it follow the ball's way in a player's replay instead. Some places it cannot pass alone: puzzles, levers
and pushers to hit first, updrafts to ride, rails to take at speed. For those it reads **hints**.

## Following a player's run

With `--follow REPLAY` the test player has the player's run tick by tick: where the ball was, how fast, which ball,
in which sector, and the keys pressed. Where its ball is where the player's was (within 2 units, as fast within 4,
the same ball, the same sector, not being lost), the player's own keys are one more choice at each decision (in the
player's view, the very keys, turns of the view too; in another view, the same pushes as its arrows), judged on a
copy of the run like every other choice and taken whenever they do not lose the ball: the player knew what the way's
search does not (a lever to hit first, a transformer to go back to, a place to wait on a lift). A short cut of its
own would lead off the player's line for good. Where the player's line is not at hand (the player fell there, or the
ball has strayed) the way's search steers, and stuck it also tries longer stretches of the player's keys from the
places near. The player's tick looked for first is the one the run has come to since the last match, so a place
where the player stood still is not played again and again. The player's last four seconds before losing a ball are no line to follow, and after a hint the player's line waits
ten seconds (the hint knows the place better). Stuck where no manoeuvre gets on, within 30 units of the player's
line, it rolls back to a place before where the player stood still and gives the player's keys from there (a
run-up, a gap crossed at speed), when on a copy of the run that keeps the ball and gets on. `AUTOPILOT_DEMO=1`
prints each match.

**Keeping to the player's line.** Once its ball is where the player's was, the player's run is a line in time: at
each decision (every tenth of a second) the inputs for the next tenth are chosen so that over the next half second
its ball stays nearest the player's (the place, and a little the speed): the player's own keys, or each choice of
arrows for a tenth and the player's keys after it, all tried on copies of the run. A slip is corrected at once, and
a puzzle's pushes (blocks in a maze of lanes, a lever, a board) are made as the player made them. Where its ball is
more than 4 units from the player's, or the player's ball is being lost there, the line is taken up again further on
where the player's ball passes near (after a lost ball, the new one's way back past here), or left to the way's
search. A hint for the place still comes first (it was written where the player's line was not enough). The line
is kept to into the player's last seconds before a lost ball, but for the last one: what the player did then may be
what the level needs (a bridge knocked down as the ball fell with it). The way followed leaves out the player's last
four seconds before a lost ball (the ball was rolling off), so a run that did not finish is followed to where the
player still had the ball.

`--then REPLAY` (up to 8 times) joins a run begun at a sector (a replay that starts with `start N`): from that sector
on, the way, its transformers and the player's ticks are that run's. So a level is followed from its start with the
owner's whole run and a later one that got through a sector the whole run did not.

A player's tries that lost the ball are left out of the way followed: each one's stretch from where it came into the
sector it was lost in (the new ball starts that sector again). A run that did not finish is followed to the end of
its try in its last sector that got furthest, but no further than just past its last transformer there (after it
the player was looking for the way, and may have rolled where a ball is stuck: a stone ball into a fan's pit; the
player's line ends there too), and from there on the test player finds the way by itself (with
`--plan`, through the plan's objects from the next checkpoint on).

## The way's search

`--plan FILE` names the level's objects to pass through, in order, one a line (transformers, fans, checkpoints,
`#` comments), or places on the way as `X Y Z` (a rail to take rather than another, a point the way must come by),
as a player who knows the level does; without one the transformers within 30 of the way found are
taken in its order. The plans written from a walkthrough video are kept as `hints/Level_NN.plan`.

The way is searched over the floor points of the level's floors, rails and modules. Besides rolling from point to
point, climbing a little (a step of 1.2; where the test player finds the way by itself, also a smooth slope up to 47
degrees, a wavy log: a link climbing more than a step needs a floor point half way up, which a kerb has not) and dropping down (30
units at most):

- **Rails** hold the ball from the sides: a way goes onto or off a rail only along it (not across a rail crossing
  under or over, not sideways off it), drops off one only where it ends, and no gap is crossed onto a rail.
- **Updrafts** (P_Modul_18): from the floor under a fan, up its column to the top of its volume and across onto a
  floor near its top (14 units round it at most). Only a paper ball rises; the transformers on the way see to that.
  A ball an updraft holds up is judged where it floats, not as falling.
- **Leaps**: where no way over the floors reaches the next goal, the search is made again allowing gaps of up to 20
  units at a high price, so the way crosses the narrowest one (a drawbridge to knock over, a part to wait for)
  instead of heading through the air.
- The transformers not on the way (not in the plan, or not near the way found) are kept off where a way round them
  is not much longer: rolling over one makes the ball over.
- A transformer of the kind the ball already is (that the way came in with) changes nothing and is gone through
  at once.

Stuck within 16 of a **drawbridge** (P_Modul_25, a plank standing up on a hinge) that the way passes over, the
test player first tries a charge at it along the way (backing off for a run-up while it stands), when on a copy of
the run the charge keeps the ball and gets on: some bridges fall when hit, others only by a lever. Straight through
first; then rams from a run-up of 8 or 14 along the way, braking at once after the hit (so as not to follow a plank
that falls into the gap), a wait while it settles, and across.

## Keeping to the middle

A player keeps to the middle of a track, not along its edge; so does the test player. The level's floors are read
once a level for their **edges**: where a floor's border has no floor going on just beyond it (a drop), or a wall
stands on it (its foot); a seam between two floors' meshes is no edge. Each floor point knows how far it is from
the nearest edge as high, and how far the middle of its floor is (the clearest point near: half the floor's width).

- **The way's search** pays more for a point the nearer it is to an edge, so the way follows the middle of a track;
  on a narrow floor its middle is the only way. A rail is no floor with edges: its line is the way.
- **The steering** judges each choice tried on a copy of the run also by how far off the middle of its floor the
  ball rolls (and more where it comes nearer than 3 to an edge): less than a choice that keeps the ball or gets on
  is worth, more than a little progress. Each arrow pressed or let go since the last decision costs a little too, so
  the line is smooth, not a zig-zag. And it rolls at a steady player's speed for its floor: on a rail or a narrow
  track a moderate one (6), on a wide floor more (up to 10, with the floor's width); faster costs, and so does
  rolling slower than half of it where nothing calls for a wait (a wait the level needs is still taken: a choice
  that loses the ball is never worth it). Held up a moment where the way goes off a rail onto a floor or up a
  small step (a sill), it backs off a little and takes it at speed. This steady style is for a way of its own;
  following a player's run, the player's pace is the measure.
- **Keeping to a player's line**, where the player's ball came nearer than 3 to an edge, its ball may keep up to 1.5
  off that line toward the middle, when as fast; elsewhere the line is the player's.

Each run's summary ends with how near the edges its ball rolled, over its ticks on a floor (not on rails): `edges:
mean M, min N, within 1 for P% of T ticks on floors`. The same line ends `--replay FILE --headless`, so any run,
a player's too, can be compared.

## Going back and trying again

The simulation is deterministic and can be copied, so the test player saves its run every two seconds (the saves
thin out as they age, reaching back several minutes) and, when its ball is lost, a transformer other than the way's next one, rolled over by a slip, makes
it over (off the player's line; a choice tried on a copy that makes the ball over so counts as losing
it), or its way
has not got on for 20 seconds, goes back to a save and tries again, varied: a short opening move of its own (a wait
or a push of random length, before anything else, a running hint too, so moving parts are met at another phase) and,
for some seconds, random additions to its choices' scores; from the third try on every third one leaves the
player's line. Three tries go from the last save before the failure, then three from each save twice as far back
(the cause may lie earlier: a run-up too slow, a bad line into a bend). After 30 tries at one place (a place: 6 way
points further than the last), the failure is taken as it comes: the ball is lost, or the run is given up with its
report (a stray transformer taken so: the ball goes into one making the way's kind, as a new ball does). The hints may fire again after going back, as for a new ball. A hint whose ball ends a step 3 or more below
the step's point (fallen to a floor beneath it) is off course: its further steps are no getting on, so it goes back
from there as from a way that stands still, instead of playing the rest of the hint from the wrong floor. A ball lost as the player lost one there (within
four seconds of keeping to the player's last seconds before a loss) is not gone back from, where the player's run got past that sector afterwards: what the
player's ball did as it went may be what the level needs (a drawbridge knocked down as the ball fell with it). `--rewinds N` caps how often it goes back in
a run (600; 0: never). The variation is seeded, so a run is reproducible; the run kept, and its replay, is the one
that got through, without the tries that failed.

## The search when stuck

Stuck for two seconds (by a gap the way crosses or the next transformer's rim: three quarters of a second, and it takes the leap or rolls into the transformer at once), the test player tries rams, pumping (to and fro along a choice, two or four swings: out of a
trough between waves too steep to climb from a standstill) and random manoeuvres on copies of the run. Stuck where
no ram, manoeuvre, charge or leap gets on, the test player searches deeper: a beam search over
sequences of held arrows (each held a third, three quarters or a second and a half, or no arrows for half a second),
four deep, keeping the six best states at each depth, one a place. Each is judged after a coast of two thirds of a
second by how far along the way the ball is, less how much stands in the way ahead: loose boxes and balls, and the
modules' moving parts over the way (a pusher's block in the lane, a plank standing up). A lost ball ends a sequence.
The best that gets on is held. With `AUTOPILOT_PARALLEL=1` the copies of the run of each decision and of the search
are run in threads, side by side; the order of the results is fixed, so the run is the same.

## Stuck reports

When the test player gets no further along its way for 20 seconds, it writes a report into `--report DIR`
(default: `stuck/` in the game's data folder), a new folder per place:

- `report.txt`: the level, tick, sector, lives, the ball's kind, place, speed and the view's quarter, the way
  ahead, the transformers, loose bodies and level objects near (with their groups);
- `view.png`: what the player sees; `top.png` and `wide.png`: the place from above, near and wide.

From a report, the owner's replays (followed with `AUTOPILOT_TRACE_FOLLOW=1`, which prints the followed ball's
place every half second) and walkthrough videos, a hint is written.

## Hint files

One file a level: `hints/<level>.txt` in the game's data folder (`hints/Level_01.txt` for level 1), or the file
`--hints FILE` names. The hints written so far are kept in the repository under `hints/`, with the replays they
use; they hold no game data. A hint a line, `#` begins a comment:

```
at X Y Z r R [sector N] [kind wood|stone|paper] : STEP ; STEP ; ...
```

When the ball comes within R of the point (in that sector, as that ball), the steps run in order, once a life;
then the test player finds its place on the way again (past the transformers, when the hint made the ball over: the plan's
transformers the hint rolled the ball into, in their order, count as gone through, even one the hint began in). Steps:

| Step | What it does |
|---|---|
| `goto X Y Z [T]` | roll to the point and stop there (T ticks at most, 600) |
| `via X Y Z [T]` | roll through the point without stopping; when it gets no nearer for 1.5 s it backs off and charges |
| `push DX DZ T` | push along the world direction (DX, DZ) for T ticks |
| `air DX DZ T` | push along (DX, DZ) until the ball has flown and landed again (T at most) |
| `wait T` | no arrows for T ticks |
| `turn N` | turn the view N quarters (negative: left) |
| `become KIND` | roll into the nearest transformer of that kind (1200 ticks at most) |
| `lead X Y Z D T` | keep D units ahead of the loose body nearest the point as it rolls (behind it, when D is negative), for T ticks: a stone ball pushing the paper one through fans, a ball to wait for |
| `shove X Y Z TX TY TZ T` | push the loose body (or a module's moving part) nearest X Y Z to TX TY TZ: round it to behind it, then on into it slowly (T ticks at most) — a board shoved into place, a ball rolled where it is wanted |
| `press X Y Z SX SZ V T` | roll along the straight line from where the step begins to the point, kept to it sideways (SX SZ: an offset across it, in units, 3 at most: the side of a rail pair the ball keeps to), on while slower than V; done there, past it, or after T ticks — rails ridden leaning on their upper rail, a steady climb |
| `pass X Y Z V [T]` | roll through the point without stopping, no faster than V (a corner, a rail's end, a transformer on the way), kept to the line from the point the hint's last `pass` went through (a chain of them follows rails and tracks) |
| `cross DX DZ X Y Z R` | cross a moving part (a swing) toward the point: the moment and the length of the push along DX DZ are found on copies of the run from here (wait, push, let it carry, push off), the first that ends within R of the point on a floor, then played |
| `hold X Y Z T` | keep the ball on the point for T ticks: still where it must wait (on a slope, or come in at speed) |
| `along V T` | push along the way the ball is rolling, not faster than V, for T ticks: rails that lead it (a helix) taken at speed |
| `round X Z R V T [Y]` | go round the upright axis through (X, Z) at radius R and speed V (one way round, or the other when V is negative) for T ticks, or until the ball is above Y: up a helix of rails |
| `await X Y Z QX QY QZ D T` | hold the ball where it is until the loose body or module part nearest X Y Z is within D of QX QY QZ (T ticks at most): a swing or a lift met at the right moment |
| `ride X Y Z QX QY QZ D T` | keep the ball over the loose body or module part nearest X Y Z until it is within D of QX QY QZ (T ticks at most) |
| `replay FILE FROM TO` | a player's inputs of ticks FROM..TO of the replay FILE (beside the hint file), turned to the test player's view; begun with the ball held in a transformer, it starts as the ball is let go (FROM: the player's transform tick + 149), in step with the player |

Directions are world directions (X east, Z north as the level is built), so a hint does not depend on where the
view is turned; the arrows are chosen for the view at each tick. A hint file is untrusted: its size (64 KiB),
the number of hints (256) and steps (64 a hint) and every number are capped, a replay is a plain file name beside
it, and a malformed line refuses the whole file with its line number. `HINTS_TRACE=1` prints the ball every ten
ticks while a hint drives.

## Watching it play

`--watch` (with `--level N`, `--open DIR` or `--sector N`, and `--follow`, `--then`, `--plan`, `--hints` as for
`--autopilot`) runs the test player in the game's window: its inputs drive the game each tick instead of the
keyboard. Its decisions (tries on copies of the run, searches) are made on a thread of their own between frames; a
decision that takes longer holds the view and shows "thinking". Going back to a saved state jumps there visibly, and
the line says what failed and back to when. The status in the top left corner: the speed, the time, sector, lives,
how far the ball is from the edge of its floor, how often it went back, what steers (the player's line, the way's
search, a hint and its step, a manoeuvre or the beam search when stuck), the way's point reached and the next
transformer. The watcher's keys, in a box bottom left in the menus' language (the state that is on stands out):

| Key | What it does |
|---|---|
| Space | pause, go on |
| + / − | faster, slower (¼× to 8×; only how many ticks a frame: the run stays the same) |
| F5 | take over: the arrows drive, the test player watches; F5 again hands the ball back from where it is now. The stretch is saved beside the hints as `<level>-hand-<tick>.replay` (the run up to then, so it replays exactly) and a hint line playing it (`at … : replay <file> FROM TO`) is added to `<level>-hand.txt`, both named on the screen; copied into the level's hint file, the test player plays it there next time. The saved states before it are not gone back to |
| F6 | skip the running hint's step |
| F7 | a stuck report of this place (as for a dead run, into `--report`) |
| F8 | mark a zone "don't dawdle here" (flow; Zones below): a tap marks the ball's path over the last 3 seconds up to the tap; held, the zone goes on while the key is down (from 3 seconds before it went down) and ends when it is let go |
| F4 | mark a zone "keep the speed limit here and don't cut corners" (careful), the same way |
| [ / ] | how far back a mark reaches: 1 to 10 seconds (3) |
| Backspace | undo the last zone marked in this window, and the rule it taught (each mark's lines carry its id, `# mark=<id>`, so another session's lines are never taken out); while a key is held, that mark is cancelled |
| F12 | show or hide the marked zones in the world, as translucent beads along their paths (flow green, careful orange) |
| F1 | hide or show the box of keys |
| Esc | the pause menu (leave the level, quit) |

At the end, or when the window is closed, the run's summary is printed and the whole run, the hand-driven stretches
too, is saved at `--record`.

## Zones

The watcher teaches the test player while watching (`--watch`): a key marks the stretch the ball just went as a
zone, and the test player obeys it from then on, in this run and every later one, headless too. A mark reaches back
(3 seconds by default, `[` and `]` change it) because one first has to notice that the ball rolls wrong: a tap of
F8 or F4 marks the ball's path over the last seconds up to the tap; held, the zone goes on while the key is down and
ends when it is let go, so no zone is ever left open. Paused (Space), a tap marks the last seconds before the pause.
The stretch just marked lights up for a few seconds, a zone being marked shows as it grows, and the status line
names the zone the ball is in.

The zones are written at once, a line each, to `<level>.zones` beside the level's hint file (with `--hints
hints/Level_01.txt`: `hints/Level_01.zones`), which every run of the level reads. With `--hints FILE` the marks are written in that file's folder (the learned
table too); without it, in `hints/` of the game's data folder. A file is written whole or not at all (a new file
beside it renamed into place), never through a link; a zones file full (256 zones) takes no more, and the screen
says so:

```
flow sector 1 r 4 : X Y Z ; X Y Z ; ...
careful sector 2 r 4 [speed V] : X Y Z ; ...
```

The points are the ball's path through the stretch (one point: a place); the zone is everything within `r` of that
path in that sector.

- **flow** — don't dawdle here. A hint's step that stops the ball goes without its stop: a `goto` is rolled through,
  a `wait` or `hold` left out, a `pass` or `press` slower than 5 taken at 5; but only when, on a copy of the run, the
  rest of the hint and a second after keep the ball (a stop the ball needs stays). The steering pays for rolling
  slower than 0.8 of the floor's pace more than a wait is worth (a choice that loses the ball is still never taken),
  except where the ball needs a run-up (a gap or a too steep climb ahead, as for a careful zone below): backing off
  and stopping for it is no dawdling, so there the zone asks nothing (a hint's stop stays too).
- **careful** — keep the speed limit here and don't cut corners. Not faster than `speed` (4.5 when not given; edit
  the line to change it): the steering pays four times as much for every unit over it, a hint's `pass`, `press`,
  `along` and `round` are capped to it, and whatever drives (the steering, a hint, the player's line), the arrows
  pushing along the ball's run are let go above the limit and the ball is braked above 1.25 times it. The way's
  line ahead (the middle the way's search found) is kept to: each unit off it costs 2, and off the middle of the
  floor three times as much as elsewhere. Where the physics needs the ball's momentum the limit is let go and only
  the line is kept: the way ahead (about 30 units) has a gap to cross or a climb steeper than the ball takes by its
  own push at the limit (at 4.5: stone 12.6°, wood 26°, paper 31°; measured on the engine, a stone ball climbs
  nothing steeper than 17° by its push, wood 37°, paper 46°), a sill comes just ahead, or the running hint says
  speed (a `press`, `along` or `round` faster than the limit: a climb, rails, a ram; or a `pass` faster than it
  within 25 units before one: its run-up). The status line says so (`[limit let go: a gap to cross ahead]`). A careful learned rule holds
  back the same way, and is let go in the same places.

Each mark also teaches the test player the kind of place it was, for every level: a rule of the learned table
(`learned.txt` beside the hints; [autopilot-rules.md](autopilot-rules.md), "The learned table").

`AUTOPILOT_SITUATION=1` prints, besides the place every second, each change of what the zones and rules ask, with
the tick and the ball's place (`zone t14528 (137.9 -6.8 -554.8) [learned: ... → careful, at most 4.5]`, `none` when
nothing holds), and why a limit is let go.

A zones file is untrusted like a hint file: its size (64 KiB), the zones (256) and points (256 a zone) and every
number are capped; a malformed line is left out with a note naming it, the other zones still count.

## Videos of its runs

`--video FILE` with `--replay` draws the run without a window, one frame a tick (60 a second) at `--size`, as raw
RGB into FILE, and mixes its sound as the window plays it (into `DRAHTSEILAKT_SOUND_FILE`); it goes on six seconds
past the finish for the end screen. `scripts/video.sh LEVEL REPLAY OUT.mp4 [WxH]` joins the two into an H.264 video
with ffmpeg (the frames go through a pipe, nothing large is kept; the game itself never runs another program).

## Where it stands

Following the owner's runs (`--autopilot --follow`), with going back and keeping to the player's line, 2026-11-03
(the finished runs are kept in `hints/runs/`, their videos outside the repository, made with `scripts/video.sh`):

- Level 1: finished from the start in 6:12.27 (hints for the ramp, the crate, the pushers and the rails).
  By itself with no player's run, from the walkthrough's storyboard (`hints/Level_01.plan`, `.route`, `.txt`;
  the hints that follow the owner's run are `Level_01-follow.txt`): 5:17.12, standing 9.1% of the time.
  With the owner's marks (`hints/Level_01.zones`, `hints/learned.txt`), 2026-11-03: 6:23.63, standing 9.7% of the
  ticks (under 0.3 a second; the run before 9.7% by the same count), within 1 of an edge 4.3% (4.5%), speed 4.8 ± 3.6
  (5.5 ± 4.0) (`hints/runs/Level_01-clean.replay`).
- Level 2: finished from the start in 6:13.27 (with `--then Level_02-s5.replay`, the owner's run of sector 5).
  By itself with no player's run, from the walkthrough's storyboard (`hints/Level_02.plan`, `.route`, `.txt`;
  the hints that follow the owner's run are `Level_02-follow.txt`): 5:23.90, standing 5.9% of the time
  (`hints/runs/Level_02-clean.replay`).
- Level 3: finished from the start in 7:45.48 (the lever and the bridge in sector 4 as the owner did them).
  By itself with no player's run, from the walkthrough's storyboard (`hints/Level_03.plan`, `.route`, `.txt`;
  the hints that follow the owner's run are `Level_03-follow.txt`): 5:58.97, standing 5.5% of the time
  (`hints/runs/Level_03-clean.replay`).
- Level 4: finished from the start in 9:19.47 (`--follow` the owner's run, `--then` the one from sector 3).
- Level 5: finished from the start in 12:39.77 (hints for both crates and the pusher).
- Level 6: finished from the start in 8:01.22: sector 1 along the owner's line, sectors 2–5 by itself with the
  plan and the hints from the walkthrough's storyboard (`hints/Level_06.txt`: the wavy log there as wood and back
  as stone at speed, the boxes rammed off, the log after the drop in sector 5).
- Level 7: finished from the start in 12:04.28, following the owner's finished run.
- Level 8: finished from the start in 9:59.40 with no player's run at all: by itself through the walkthrough's
  plan (`hints/Level_08.plan`, with places on the way) and the hints written from its storyboard
  (`hints/Level_08.txt`: rail pairs ridden leaning on their upper rail, a helix, a swing, the targets that drop
  the maze's bridges, boxes rammed off).
- Level 9: finished from the start in 12:35.27, following the owner's finished run (the maze of pushers in sector 3
  is solved as the owner solved it).
- Level 10: finished from the start in 15:31.00, following the owner's finished run with `hints/Level_10.txt`.
- Level 11: finished from the start in 13:43.73 with no player's run at all, by itself through the walkthrough's
  plan and the hints written from its storyboard (`hints/Level_11.plan`, `.txt`, `.route`): the planks of the castle
  maze rammed down, a loose stone ball rammed into the rails that drive a pusher, the targets, the crates rammed out
  of the channels, the maze of sector 4, the fans and swinging crates of sector 5, and in sector 6 two loose stone balls
  knocked off the grid (one shoves the pusher between the seesaws, one opens a trapdoor) and the last rail pairs.
- Levels 12–13: 12 in sector 2 (its thin rails); 13 in sector 2, past the paper transformer and up the fan, at the
  beam the fans 18_10–12 blow along.
- Videos of the finished runs: `scripts/video.sh`, kept outside the repository.
- Our ten levels (levels.md, "Our levels and how to edit them"), 2026-11-03: each finished from the start in three
  styles, with the hint files their descriptions write (`assets/levels/<level>/pilot/`).
