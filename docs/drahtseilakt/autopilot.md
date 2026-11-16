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
report (a stray transformer taken so: the ball goes into one making the way's kind, as a new ball does). Sooner at one spot: once a try from a place has gone back as far as the saves reach, standing
still there three times more (within 2 units), or losing the ball there six times more (within 12), is taken as it
comes (what fails there was decided earlier than any going back reaches); until then the tries keep going back
further, and a failure taken clears its spot (the next ball comes there with a history of its own). The hints may fire again after going back, as for a new ball. A hint whose ball ends a step 3 or more below
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
at X Y Z r R [sector N] [kind wood|stone|paper|sticky] [retry N] : STEP ; STEP ; ...
```

`retry N` (1 to 50) marks a hint whose outcome hangs on differences too small to write steps against (a crate
rammed onto another, a moving part met at its phase): a player tries such a place again. Before the hint fires the
run is saved; when it fails (its ball lost, standing still or off course while it runs, or a `check` not met), the
run goes back to that save and the hint runs again, varied and seeded (reproducible): a start delay of up to 3
ticks a try (15 at most) with no arrows, `pass` and `press` points moved up to 0.3 across, speeds up to 6% faster
or slower, pushes up to 3 ticks longer or shorter, a `trace`'s gain up to 15% off and its recorded way moved across as the points are (a closed-loop trace would pull every try back onto the same line) (the full width from the fourth
try). Each try counts against `--rewinds` (0: none), the run kept holds only the try that got through, and after
N tries again the failure is taken as it comes. The summary lists the tries each such hint took (`retries: line L:
N tries`). Written as a step, `retry N` does the same from that step on (the run saved as the next step begins;
the variation is of the steps after it, the delay before the first of them). Hints without it run as before.

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
| `shove X Y Z TX TY TZ T [R [V]]` | push the loose body (or a module's moving part) nearest X Y Z to TX TY TZ: round it to behind it, R from its middle (3.5, a ball's size; more for a crate, 10 at most), then on into it slowly (T ticks at most), and with V only while it is slower than V (a crate on a slope slides on by itself) — a board shoved into place, a ball rolled where it is wanted, a crate set under a rail |
| `ram X Y Z DX DZ B V T [S]` | ram the loose body (or a module's moving part) nearest X Y Z along the world direction (DX, DZ), through its middle (S units to its left looking along the ram, 3 at most): to the run-up point B units behind the body where it is now (2 to 30; round it when the ball is ahead or beside), or onto the line through it at least half that behind, then on through it no faster than V for T ticks of the charge (or until the ball stops against it), kept to the line through its middle as the body moves: a crate knocked into its pocket wherever the ball and the crate came to be, not along fixed coordinates |
| `press X Y Z SX SZ V T` | roll along the straight line from where the step begins to the point, kept to it sideways (SX SZ: an offset across it, in units, 3 at most: the side of a rail pair the ball keeps to), on while slower than V; done there, past it, or after T ticks — rails ridden leaning on their upper rail, a steady climb |
| `pass X Y Z V [T]` | roll through the point without stopping, no faster than V (a corner, a rail's end, a transformer on the way), kept to the line from the point the hint's last `pass` went through (a chain of them follows rails and tracks) |
| `cross DX DZ X Y Z R` | cross a moving part (a swing) toward the point: the moment and the length of the push along DX DZ are found on copies of the run from here (wait, push, let it carry, push off), the first that ends within R of the point on a floor, then played |
| `hold X Y Z T` | keep the ball on the point for T ticks: still where it must wait (on a slope, or come in at speed) |
| `along V T` | push along the way the ball is rolling, not faster than V, for T ticks: rails that lead it (a helix) taken at speed |
| `round X Z R V T [Y]` | go round the upright axis through (X, Z) at radius R and speed V (one way round, or the other when V is negative) for T ticks, or until the ball is above Y: up a helix of rails |
| `await X Y Z QX QY QZ D T` | hold the ball where it is until the loose body or module part nearest X Y Z is within D of QX QY QZ (T ticks at most): a swing or a lift met at the right moment |
| `swing X Y Z QX QY QZ D T` | hold the ball where it is until the loose body or module part nearest X Y Z has turned back at the end of its swing near QX QY QZ (within D, no longer moving toward it; T ticks at most): a pendulum's phase met however wide it swings |
| `ride X Y Z QX QY QZ D T [A]` | keep the ball over the loose body or module part nearest X Y Z until it is within D of QX QY QZ (T ticks at most); with A, A units ahead of its middle along the way it goes (behind, negative; 4 at most): on the front of a ball rolling down rails, not on its top |
| `replay FILE FROM TO` | a player's inputs of ticks FROM..TO of the replay FILE (beside the hint file), turned to the test player's view; begun with the ball held in a transformer, it starts as the ball is let go (FROM: the player's transform tick + 149), in step with the player |
| `sync X Y Z [T]` | keep the ball on the point until the sector's loose objects and modules have entered (a checkpoint's sector begins 48 ticks after it; T at most, 600): the state a `--sector N` run starts from |
| `until X Y Z N [T]` | keep the ball on the point until the sector's clock (the ticks since its modules entered the physics: at its checkpoint, at a `--sector N` start or again after a lost ball) reaches N (T ticks at most, 1800): a puzzle begun at the same moment of the sector, its moving things where they were, whether the run began at the checkpoint or came on from the sector before |
| `check X Y Z D [T]` | the hint's success condition: the ball within D of the point now or, giving no arrows, within T ticks; not met, the hint has failed (one marked `retry` is tried again, another just ends there) |
| `retry N [W]` | from here on the hint is tried again up to N times when it fails, as `retry N` before the `:` does from its start: the run is saved as the next step begins (a stretch before it that always works is not played again); with W, its `pass` and `press` points (and a `ram`'s line) are moved up to W across instead of 0.3 (2 at most): a ram whose outcome hangs on where it meets the body |
| `before N` | a success condition in time: the sector's clock (as `until` counts it) not past N as the hint comes here, else the hint has failed (a convoy of balls or a moving part met too late; the log prints the clock) |
| `placed X Y Z QX QY QZ D` | a success condition on a thing moved: the loose body or module part nearest X Y Z within D of QX QY QZ now, else the hint has failed (a crate rammed into its pocket, judged where it came to be, not by where the ball is; the log prints where it is) |
| `guard X Y Z DX DZ R N T` | keep the ball by the point as a deflector for the loose bodies that come to it: the nearest within R (1 to 50) that has not gone on is met from the side opposite (DX, DZ), the ball a little under a ball's size behind it and at most 4 units from the point, so it is turned that way; a body has gone on once it is 3 past the point along the direction or 2.5 below it; done when N of them have (64 at most) or after T ticks: balls rolling down a channel each turned into the queue at its end, whatever its exact path |
| `full STEP` | the step (one that pushes: not `turn`, `cross`, `trace` or `wait`) pushed as hard as the arrows of the view as it is allow, instead of the original's push on average: on a 45-degree map two arrows along an axis of the world (1.41 times), one along a diagonal (see "Modified originals"); on the original levels the same as without |
| `trace FILE FROM TO [GAIN]` | the same stretch of a recorded run followed closed-loop: the replay is run once as the hints load, for the ball's place and speed at each tick; then each tick the player's arrows are the feed-forward plus a pull (times GAIN, 1 by default, 0 for none) toward the recorded place and speed at the matching point (matched by progress along the run, kept near one tick a tick so the run's waits stay waits), turned into the arrow set nearest it in the view now (the part missed carried to the next tick). Over 4 units off the run, it steers to the run's way 30 ticks ahead until it is on it again, and after 10 s lost gives the ball back to the autopilot. Unlike `replay`, it holds when the ball arrives a little different (a chain of sectors: `sync` first, then `trace`) |

Directions are world directions (X east, Z north as the level is built), so a hint does not depend on where the
view is turned; the arrows are chosen for the view at each tick. A hint file is untrusted: its size (64 KiB),
the number of hints (256) and steps (64 a hint) and every number are capped, a replay is a plain file name beside
it, and a malformed line refuses the whole file with its line number. `HINTS_TRACE=1` prints the ball every ten
ticks while a hint drives. To compare two runs, `DRAHTSEILAKT_DUMP_STATE=S:T,...` (or `@N`, a game tick) with `--replay --headless` prints the whole simulation state at T on sector S's clock (the ticks since its modules entered the physics, the clock `until` waits for): the ball, every module's and trigger's state, every loose body and module part with its physics core, the physics' random generator.

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
plain sector 2 r 8 : X Y Z ; ...
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
- **plain** — no learned rule here (written by hand, `plain sector N r R : X Y Z ; ...`): the test player rides as
  it does by itself, where a rule learned on another level would lose the ball (on level 6, "bend, wood: don't
  dawdle" cuts the waits of a lift and of a paper ball in fans).

Each mark also teaches the test player the kind of place it was, for every level: a rule of the learned table
(`learned.txt` beside the hints; [autopilot-rules.md](autopilot-rules.md), "The learned table").

`AUTOPILOT_SITUATION=1` prints, besides the place every second, each change of what the zones and rules ask, with
the tick and the ball's place (`zone t14528 (137.9 -6.8 -554.8) [learned: ... → careful, at most 4.5]`, `none` when
nothing holds), and why a limit is let go.

A zones file is untrusted like a hint file: its size (64 KiB), the zones (256) and points (256 a zone) and every
number are capped; a malformed line is left out with a note naming it, the other zones still count.

## Modified originals

The community remakes of the original levels (levels.md, "Modified originals") hold the original's geometry: the
same objects at the same places. The **invisible maps** hide every object, the balloon's stand-in too (the game
makes the balloon there all the same, as the original's init does); their physics is the original's tick for tick,
so the original's runs play on them unchanged. The **45-degree maps** turn the reset points by an eighth of a turn:
the view looks along a diagonal of the world and the arrows push along the view's axes.

So the test player takes the original's hints and plans as they are (their places and directions are the world's),
and gives a world direction as the arrows of the view as it is, its offset from whole quarters too: on a 45-degree
map a push along an axis of the world is two arrows. Arrows held together add their pushes, so there an axis push
would be 1.41 times as hard as on the original and a diagonal one 0.71 times: a hint's steps (pushes, rail leans,
run-ups and rams, a `replay` stretch) were tuned in the original's view, so on such a map they give the original's
push on average instead: each tick the arrow set nearest it plus what the ticks before left over (a diagonal push of
the original, 1.41, is more than one arrow along that diagonal gives: one arrow there). A `cross` (tried on copies
of the run in the view as it is) and a `trace` (its own carry) are left as they are; on the original levels nothing
changes. Where the original's push is too little in such a view, a step is written `full` (the one rule for it, opt-in,
so the runs tuned without it stay as they were): it is given as the view's own arrows, as hard as they push, two
arrows along an axis of the world (1.41 times the original's axis push, as much as its diagonal one) and one along a
diagonal (no set gives more there). A ram, a kick or a climb the original made along a diagonal (1.41) is written
along an axis with `full` to have that push. A player's replay recorded on the original level (followed with
`--follow` and `--then`, or a hint's `replay` or `trace` stretch) is seen on the original level, the copy's own file,
when the map holds its geometry (the same objects at the same places, in any order); only the ball's way and its
pushes in the world are taken over, and the player's keys are turned into the arrows of the map's view. A remake
whose objects differ is no remake of it: such a replay is refused as before.

A replay of the original level also plays with `--replay` on a remake whose objects are turned the same way
too (an invisible map: the same physics and view, so the same run tick for tick; `--record` saves it as the map's).
On a 45-degree map the keys would push another way, so there it is refused.

Where it stands, 2026-11-15 (runs in `hints/runs/modified/`): the invisible maps, 12 of 13 finished from the start by
the test player (levels 1, 11, 12, 13 by the plan and hints alone, 4 and 9 following the owner's runs, the others following
the original's kept run; level 8 only as the original's kept run played on the map, `Level_08-original-run.replay`).
The 45-degree maps, levels 1-7, 9 and 10 finished following the original's kept run with no hints (the hints' open
steps were written for the original's view); level 8 the same way, 10:41.30. With the hints given as the original's
push (above), 2026-11-15: level 12 finished, 13:30.93 (`--follow` its kept run, its plan and `hints/Level_12-45.txt`:
two places taken more carefully, the turn back onto the upper row of sector 6 at z 694 and the corner onto the rail x 521.3 of sector 7
slower and wider; braking and steering there are diagonal pushes, weaker in this view). Level 11
finished, 14:44.20 with 3 lives (the same way, `hints/Level_11-45.txt`: two crate rams more in each of the first
two channels of sector 3; on the last rail pairs of sector 6 the first pair leans 1.2 instead of 0.8, and as the
ball comes to the end of the second slower than on the original, 7 to 8 instead of 9.7, a lean taking from the push
along the rail, and lands short on the crossing of the third, 8 ticks of push east set it onto the third's rails
before it rolls back between them). Level 13
(`hints/Level_13-45.txt`, 2026-11-16: `Level_13-full.txt` with sector 2 taken apart where the owner's run goes
another way in this view: the plank at z 686 that his ball knocks over in passing rammed down, the ball stopping 2
units short at the serpentine's crossing z 721 so the crate stays under it and stone 18 rolls over untouched, the
crate pressed under the crossings z 713 and 705 closed-loop with each crossing awaited, and on the beam a press
east after his launch keys so the paper ball is in the second fan's slot before the stone) was found to pass sectors 1 and 2
from the start when run with the level's plan: `--autopilot --plan hints/Level_13.plan --hints hints/Level_13-45.txt`.
Without `--plan` the way found is another (2255 points instead of 2530) and the ball stands at the end of sector 1;
everything the run reads is in `hints/` (no route, replay or learned table comes from the data folder), and it
plays tick for tick the same from an empty data folder as from one full of earlier runs. Checked again on
2026-11-16 from an empty data folder: the run is deterministic (the same recipe twice, and with the binary of
that merge, gives a byte-identical replay); it gets through sector 2 only by going back: the paper ball rolls behind
stone 18 down the rails instead of on it (the owner's ball is on its front) and falls into the gap at z 585 that the
owner jumps at 9 to 12, and the 13th try (gone back to 3:23, the seeded variation) clears it, CP2 at 3:44.88 with 3
lives. With `--rewinds 0` it is lost there; a faster `full` push off, before the stone or after it has fallen, did
not jump it. Sectors 3, 4 and 5 are through when begun at their checkpoints, with
`full` steps (above) where the owner's push was diagonal: in sector 3 the crates rammed `full` along the axes (crate
40 east straight from his run, no run-up), the paper transformer and the way round to the channel's mouth `full`, so
the ball is in the channel ahead of the lead stone (0:58.2, his 0:58.0); his run followed from z -753 to the stone
transformer (1:14.45, his 1:14.32), so the four swinging sacks are where his were and no stone falls there; at the
channel's end his run followed, then the ball held still while the last stone comes: all ten stones in the queue,
CP3 at 2:44.93 (his 2:44.92). In sector 4 his north-east kick off the raft into the first fan (x -134) is given as
its two parts, `full` north then `full` east (22 ticks each, then his run on): CP4 at 2:04.48 (his 2:04.18). In
sector 5 the push onto the helix and the `round` up it are `full`: the level finished from its checkpoint in 1:54.17
with no ball lost. From the start (2026-11-16) the run dies in
sector 3 at crate 40: it comes to the ram 0.2 to 1 unit off the checkpoint start's way (crate 38, rammed by his run
followed at gain 3, is knocked a little otherwise), the ball meets the crate south of its middle, the crate turns and
jams, and the ball stands at (-726.6, -219.4, -293.9); 30 tries back do not free it. Other press lines, a pass point
before the ram, standing still at the sector's entry before his run, and a `shove` after the ram each broke the start
at CP3 or did not help. Run on from sector 3, sector 4 is lost after the first fan: the raft (four loose balls) is in
the same place as with a start at CP4 (within 0.1), but the trace's place in his run lags 20 to 30 ticks behind, so the
ball comes to the second fan (x -76, z -1143) 3 units north of it, is lifted too little and falls to the lower floor,
lost at x 50 to 60. Riding the raft until it is where his was, then following on, gives both starts the same state but
misses the fan from it; rolled into the fan by `pass` steps, the ball drops into the gap under it. Level 13 is not
finished from the start yet (sector 3's crate 40 and sector 4's second fan in a run on).
Later on 2026-11-16 sector 4 was synced on the raft as the owner waits for it: after each fan's landing the ball stops and `await`s the raft's lead ball where it was as he left, then his run is traced on (his kick into the third fan split `full` north and east): from CP3, CP4 at 2:04.22 (his 2:04.18). Run on from sector 3 the await ends within 4 ticks of that state, but the ball still lands low after the second fan. For crate 40 the new `ram` step (a body rammed through its middle from a run-up, closed on where it is), a sync on the stone transformer's hold and a stop before the press all failed from CP3 already (the crate jams, or the paper ball is a second late for the stones); `ride` kept ahead of stone 18's middle passes sector 2 from CP1 4 s sooner but from the start never clears the gap. From an empty data folder the run is as before: dead in sector 3 at crate 40.

Compared state for state (2026-11-16, `DRAHTSEILAKT_DUMP_STATE`, see "Hint files"), the run on from the start and the start at CP3
differ in sector 3 in: the ball (it comes over the checkpoint rolling, a start puts it there still), the sector's loose
bodies and the modules' proximity tests by one tick (at a checkpoint the loose bodies enter a tick before the modules,
a `--sector` start puts both in at once), the four sacks' swing (its step count runs from the level's start: 38 against
139, the other parity), the physics' shared random generator and sleep test countdown (when resting bodies freeze, so
crate 42 settles 0.15 units and 1.7 degrees otherwise), and the ball's orientation. The stone transformer makes the
ball state equal (the same place, at rest), and a trace begun in it starts as the ball is let go: from there the two
runs agree to 0.0002 for 130 ticks, but ramming crate 38 (box on box) makes that 1 to 2 units within 50 ticks, so crate
40 jams in one run and not the other. Equal states cannot be made there (the orientation and the random generator are
not the hints'); every variant tried from the release (trace from it at ticks 2322 to 2328, a hold to the sector's
clock after it) jams crate 40 from CP3 as well. The crate puzzle needs steps whose outcome does not depend on such
small differences.

With the hints' tries again (2026-11-16, `retry`, `check`, `before` in "Hint files"): the gap z 585 hint is tried
again from its start (from the start: through on the 2nd try), sector 3 from before crate 38's ram (`retry 50 2`, a
check that crate 40 went and `before 3530` at the channel's mouth, where the CP3 start comes at 3468), sector 4 from
the kick off the raft. From CP3 and from CP4 each passes on its first try, as before. Run on from the start, crate 40
jams in nearly every try (the saves the old going back reached were after the ram, the jam showed only some 40 s
later, and the steps after a save replayed as written, the closed-loop trace pulling each try onto the same ram);
in five runs of 41 to 51 varied tries five got through the crates, all 3.5 to 6 s late at the channel's mouth, so
the stone convoy is ahead and the queue at CP3 stays short. Level 13 is still not finished from the start: the run on
meets the crates in another state, which the ram's line does not make up for. Later on 2026-11-16 the bay was compared at the moment its work begins (the run on handed over to the hints with `--replay --from`): crate 38, rammed by his trace, comes to lie 1.5 units further south and the press at crate 40, begun beside it, rubs it and meets crate 40 too slowly; crate 42 starts 3.3 degrees turned and turns on as it leaves its block. Rammed closed-loop and judged where they came to be (the `placed` step, a retry mark before each ram), both crates get into their pockets from either start (crate 42 in 26 of 31 tries from the start), but the rams take longer than the original's presses and the ball comes onto the stones' channel together with stone 21 (some 45 ticks late): the run behind the convoy is the one left open. Later the same day the bay was made faster: crate 40 rammed at 11 instead of 9, and crate 42 rammed straight from where crate 40 left the ball (no stop before it; `ram ... -1 -0.2 11 10 120 0`: from a moving ball the charge needs 120 ticks, at 110 the crate stops short), each judged `placed` and retried (crate 42 lands in 26 to 30 of 31 tries). The ball is at the hold (-731.5 -214.5 -260.3) at 3263 on the sector clock from CP3 and 3289 from the start (3321 to 3336 before; his 3278), waits there to 3278 (`until`) and rides over stone 21 into the channel ahead of the convoy; from CP3 the level passes CP3 again. A ram at crate 40 straight out of his run (no run-up), shorter charges and a combined push all left crate 40 jammed or short. What now decides sector 3 is the queue at CP3: whether stone 19 turns east at the channel's end or falls off west hangs on 0.1-unit differences it picks up between z -300 and -1100 (the sacks), and these change with the ball's departure from the hold by a few ticks (3274 loses it, 3278 and 3282 keep it from CP3); tries again from the channel's mouth with a check on the tenth stone found the queue shorter in every later try, so this is not yet written as a retry.

Finished from the start (2026-11-16, later; `hints/runs/modified/45deg/Level_13.replay`, from an empty data folder with the default going back): 10:25.58, no ball lost (4 lives), 83 rewinds: 1 in sector 2, 58 in sector 4 (the kick off the raft into the first fan: 2 times 21 tries failed, then 14 more) and 24 in sector 5 (the push up the helix). Two things made sector 3 pass from the start on its first try. Run on from the start the ball had come to the four swinging sacks 25 ticks later than from CP3 (the sacks swing the same on the sector's clock in both), so it did not still them and four stones fell there; now it runs down the channel `full` at 40 and waits at z -598 until the sector's clock is 3979, and both starts meet the sacks on the same tick (3977 to 3981 all keep the ten stones). At the channel's end the owner's trace is replaced by the new `guard` step: the ball as a deflector, each stone coming within 8 met from its west side and turned east into the queue, whatever its exact path; then each of the ten places is checked with `placed` before the ball rolls over the queue. From both starts the queue is 10 of 10 and CP3 passes with no rewind. Sectors 4 and 5 run on from sector 3 still pass only by going back: in sector 4 the run on matches the owner's run 23 ticks otherwise than a CP4 start (which waits for the raft at his tick 2457), and in sector 5 the ball is 6 units past the sync point, so the trace skips its first 405 ticks. The ride: standing (under 0.5 a second) 17.9% of the time (the waits for the stone convoy and the raft), within 1 of a floor's edge 8.7% of the ticks on floors, speed mean 5.3, median 3.6, 90% under 13.3, at most 24.4.

## The speedrun style

`--style speedrun` (with `--autopilot` or `--watch`) makes the test player ride as fast as it can, for speedrun
videos; `--style calm`, the default, is everything above, and its runs are unchanged. Where it finds the way by
itself, a speedrun keeps a far higher pace (11 on a rail, 9 and more on a floor, up to 22 on a wide one, 16 off the
floors), shuns an edge only within 0.8 of it and keeps to no middle, so it rides along edges and cuts corners; no
careful zone or learned rule holds it back. A flow zone's push for speed is not taken over: the steering's measure
stays how far along the way a choice gets (pushed for speed, it would rather roll back down a climb fast than up it
slowly). A hint's stops stay as written (a wait a puzzle needs: a copy of the run judges only the ball, not the
puzzle), but for the level's flow zones as in the calm style. Following a player's run, it does not stand where
the player stood: a stretch of 30 ticks or more with the player's ball slower than 0.8 and within 1.5 of where it
began is skipped, when the player's keys from its end, on a copy of the run, keep the ball on the player's line
for five seconds (a wait for a lift or a swing fails that and is kept). A failure is gone back from as in the calm
style, so the risk it takes costs tries, not the run kept.

A **calm zone** in a level's zones file (`calm sector N r R : X Y Z ; ...`, written by hand) asks a speedrun for the
calm style there, and nothing of the calm style: a puzzle the steady pace gets through and the speedrun's does not
(a paper ball to keep ahead of a stone one, a maze of pushers met at the player's phase). There the followed
player's still stretches are not skipped either.

For other tries at a level, `AUTOPILOT_SPEEDRUN_PACE=F` scales the pace (0.3 to 2), `AUTOPILOT_SPEEDRUN_NO_SKIP=1`
keeps the player's still stretches, and `AUTOPILOT_SPEEDRUN_SKIP_ONLY=1` rides the calm pace and only skips them.
The fastest runs are kept in `hints/runs/speedrun/`.

### The raw speedrun

`--style raw` is the speedrun as the first test player rode (2026-11-01/02, before the calm pacing, the edge margins
and the ride-quality rules), for the owner's remark that the speedrun went best then. It is a style of its own, so
`--style speedrun` and its recipes stay as they were. What the speedrun still holds back, and what the raw one does
instead:

| Rule (where) | `speedrun` | `raw` |
|---|---|---|
| Cruising speed by floor, cost above it (`Pace.fast`, `Route::cruise`) | 1 a unit over 11 (rail), 9-22 (floor), 16 | none: the physics is the limit |
| Cost below half the pace (`Pace.slow`, no idling) | 0.15 a unit | none |
| Edge margin (`Pace.safe`, `Pace.edge`, `rollouts`) | within 0.8 of an edge, 2 a unit | none |
| Off the middle (`Pace.middle`) | none | none |
| Arrow switches (`Pace.switch`, no zig-zag) | 0.05 an arrow | none |
| Following a player: up to 1.5 off a line that hugs an edge toward the middle (`track`, `TRACK_SLACK`) | yes | no: the edge as the player rode it |
| Flow zones' push for speed in the steering (`FLOW_SLOW`) | yes, in the level's flow zones | no |
| Careful zones and learned rules (caps, braking, `under_cap`, a hint's speeds capped) | none | none |
| Calm zones (the calm style there, the player's still stretches kept) | yes | no (`AUTOPILOT_RAW_CALM_ZONES=1`: yes) |
| Plain zones | (no learned rule anyway) | (no learned rule anyway) |
| A hint's stops (`goto`, `wait`, `hold`, a slow `pass` or `press`) | as written, but in flow zones | the same (`AUTOPILOT_RAW_FLOW_HINTS=1`: left out everywhere a copy of the run keeps the ball for the rest of the hint and a second) |
| A hint's speeds (`pass`, `press`, `along`: brake above it) | maxima | the same (`AUTOPILOT_RAW_FLOW_HINTS=1`: minima where the step is flowed, it pushes on above them) |
| The followed player's still stretches | skipped when the player's keys keep the line | skipped the same way |
| Going back (`Rewinds`): a lost ball, a stray transformer, a hint's failed check or off course, 20 s without getting on | yes | yes (these are failures, not ride quality) |

No rule of the calm style sends a run back for standing, an edge or a wobble: those only cost in the steering's
scores, and the raw style sets them to 0. A hint is a puzzle's solution written by hand, and a copy of the run judges
only the ball, not the puzzle (a pusher's plate pressed too hard, a paper ball to keep ahead of a stone one), so its
stops and speeds stay as written by default; on a level whose hints are mostly ways along rails and tracks, flowing
them all is the faster try. Where the raw risk fails at one place again and again (six tries), the tries after that
ride the calm style from the save to 20 way points past the place (`AUTOPILOT_RAW_FALLBACK=N`, 0:
never), so the raw ride costs a slow stretch there, not the run.

A level whose speedrun needs hints the calm run must not take has its own hint file, given with `--hints`:
`hints/Level_03-speedrun.txt` is `Level_03.txt` and one hint more (sector 4: the wood ball sent along the single rail
south of the checkpoint at once, where by itself it stood 8 s). The level's zones file stays the one beside it
(`hints/Level_03.zones`), and its calm zones count for the speedrun only. 2026-11-16, levels 3, 5, 9 and 13:

- Level 3: 5:26.92, 5 lives, no ball lost (calm 5:58.97), `--plan hints/Level_03.plan --hints
  hints/Level_03-speedrun.txt`. The sector 2 rails' waits (the ball settling in the dips at their feet) stay: shorter,
  it falls off. No way past the transformers of sector 1 was found (stone at the inlet of the forked platform cannot
  take the south arm to the paper transformer).
- Level 5: 11:29.95, 5 lives, no ball lost (calm 12:39.77), `--follow hints/Level_05-owner.replay --hints
  hints/Level_05.txt` with a calm zone over sector 5 (`hints/Level_05.zones`; at the speedrun's pace the ball fell
  off near (1113.7, -138, -47) there until game over). Sectors 1-4 at the speedrun's pace reach the last checkpoint
  at 8:25 (calm 9:35).
- Level 9: no faster run yet. Following the owner's run, the speedrun is 7 s ahead at the third checkpoint, then in
  sector 4 (the stone ball, the targets and the rails round the loop) it goes off near (423, -109, 568) again and
  again; a calm zone there, or over the whole sector, makes it slower than the calm run (16:55, two balls lost).
- Level 13: the speedrun equals the calm run (10:43.90): the level is driven by hints and the owner's traced runs,
  and its long stops are the puzzles' own (a crate awaited under the serpentine, 8 s; the stones that lift the paper
  ball at the end of sector 3, 20 s).

The runs of levels 3 and 5 play tick for tick the same on the invisible maps.

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
  as stone at speed, the boxes rammed off, the log after the drop in sector 5). That run no longer came again with
  the learned table (game over in sector 2) nor without it (stuck in sector 5); 2026-11-15, with plain zones where
  the rules learned on level 1 cut the waits of sectors 2 and 3 (`hints/Level_06.zones`) and the stone ball pushed
  off its transformer onto the hanging plates in sector 5 (left alone it rolled off the tower onto the rails above
  the log): 9:01.60, 4 lives, standing 17.2% of the ticks (16.5% before), within 1 of an edge 5.3% (2.5%), speed
  4.8 ± 4.5 (4.9 ± 4.4) (`hints/runs/Level_06.replay`).
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
- Level 12: finished from the start in 12:35.68 with no player's run at all, by itself through the walkthrough's plan
  and the hints written from its storyboard (`hints/Level_12.plan`, `.txt`, `.route`; `hints/runs/Level_12.replay`):
  the thin rail's serpentine, the target that drops the bridge of sector 3, the crease of tilted cubes and its gap, the
  four lifts with cages, the stairs of fans, the stone balls shoved into the helices, the boxes on the hump, the log
  half-pipes. Standing 5.7% of the ticks (1.4% outside transformers), within 1 of an edge 14.1%, speed 5.3 ± 3.8.
- Level 13: sector 1 by the hints; sector 2, from its start, by the hints made from the owner's run of it with his
  line followed (`hints/Level_13.txt`, `hints/Level_13-s2.replay`), with no going back: the planks, the crate set
  under each rail crossing as stone 18 comes, the paper ball's dive, the beam and the chute. Sector 3: past the stones
  along the side troughs as the owner went, the crates, the stone, paper and fan by hints, on by itself to the pit
  before CP3, where the paper ball waits for the stones to lift it (not yet done). From the level's start, sector 2
  is not yet reliable (the crate's pushes are sensitive to where the ball comes from); sectors 4 and 5 are open.
- Videos of the finished runs: `scripts/video.sh`, kept outside the repository.
- Our ten levels (levels.md, "Our levels and how to edit them"), 2026-11-03: each finished from the start in three
  styles, with the hint files their descriptions write (`assets/levels/<level>/pilot/`).
