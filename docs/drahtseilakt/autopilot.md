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

`--then REPLAY` (up to 8 times) joins a run begun at a sector (a replay that starts with `start N`): from that sector
on, the way, its transformers and the player's ticks are that run's. So a level is followed from its start with the
owner's whole run and a later one that got through a sector the whole run did not.

A player's tries that lost the ball are left out of the way followed: each one's stretch from where it came into the
sector it was lost in (the new ball starts that sector again). A run that did not finish is followed to the end of
its try in its last sector that got furthest, and from there on the test player finds the way by itself (with
`--plan`, through the plan's objects from the next checkpoint on).

## The way's search

`--plan FILE` names the level's objects to pass through, in order, one a line (transformers, fans, checkpoints,
`#` comments), as a player who knows the level does; without one the transformers within 30 of the way found are
taken in its order. The plans written from a walkthrough video are kept as `hints/Level_NN.plan`.

The way is searched over the floor points of the level's floors, rails and modules. Besides rolling from point to
point, climbing a little and dropping down (30 units at most):

- **Rails** hold the ball from the sides: a way goes onto or off a rail only along it (not across a rail crossing
  under or over, not sideways off it), drops off one only where it ends, and no gap is crossed onto a rail.
- **Updrafts** (P_Modul_18): from the floor under a fan, up its column to the top of its volume and across onto a
  floor near its top (14 units round it at most). Only a paper ball rises; the transformers on the way see to that.
  A ball an updraft holds up is judged where it floats, not as falling.
- **Leaps**: where no way over the floors reaches the next goal, the search is made again allowing gaps of up to 20
  units at a high price, so the way crosses the narrowest one (a drawbridge to knock over, a part to wait for)
  instead of heading through the air.
- A transformer of the kind the ball already is (that the way came in with) changes nothing and is gone through
  at once.

Stuck within 16 of a **drawbridge** (P_Modul_25, a plank standing up on a hinge) that the way passes over, the
test player first tries a charge at it along the way (backing off for a run-up while it stands), when on a copy of
the run the charge keeps the ball and gets on: some bridges fall when hit, others only by a lever.

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
then the test player finds its place on the way again (past the transformers, when the hint made the ball over). Steps:

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
| `replay FILE FROM TO` | a player's inputs of ticks FROM..TO of the replay FILE (beside the hint file), turned to the test player's view; begun with the ball held in a transformer, it starts as the ball is let go (FROM: the player's transform tick + 149), in step with the player |

Directions are world directions (X east, Z north as the level is built), so a hint does not depend on where the
view is turned; the arrows are chosen for the view at each tick. A hint file is untrusted: its size (64 KiB),
the number of hints (256) and steps (64 a hint) and every number are capped, a replay is a plain file name beside
it, and a malformed line refuses the whole file with its line number. `HINTS_TRACE=1` prints the ball every ten
ticks while a hint drives.

## Videos of its runs

`--video FILE` with `--replay` draws the run without a window, one frame a tick (60 a second) at `--size`, as raw
RGB into FILE, and mixes its sound as the window plays it (into `DRAHTSEILAKT_SOUND_FILE`); it goes on six seconds
past the finish for the end screen. `scripts/video.sh LEVEL REPLAY OUT.mp4 [WxH]` joins the two into an H.264 video
with ffmpeg (the frames go through a pipe, nothing large is kept; the game itself never runs another program).

## Where it stands

Following the owner's runs (`--autopilot --follow`), 2026-11-02 (the finished runs are kept in `hints/runs/`):

- Level 1: finished from the start in 8:00.70 (hints for the ramp, the crate, the pushers and the rails).
- Level 2: finished from the start in 5:47.25 (with `--then Level_02-s5.replay`, the owner's run of sector 5).
- Level 3: finished from the start in 8:34.07.
- Level 4: finished from sector 2 in 7:55.15 (`--follow` the owner's run, `--then` the one from sector 3; a hint into the paper transformer on the slope); from the start to sector 5 (route 2347 of 2953).
- Level 5: finished from the start in 12:03.58 (hints for both crates and the pusher).
- Level 6: with the owner's run (it ends at the convex log in sector 2) and the plan `hints/Level_06.plan`, the test
  player goes on by itself, knocks the drawbridge over and has reached sector 3 once; usually it rolls past the way
  down off the log (2026-11-03).
- Level 7: to sector 5 (route 2635 of 3261): the plank the paper ball lands on must first be shoved into place as stone.
- Levels 8–13 (2026-11-03): 8 by itself with the walkthrough's plan to the first wood transformer (route 149 of
  1118); 9 following the owner's run to sector 3; 10 following the owner's finished run with `hints/Level_10.txt` to
  sector 2; 11 in sector 1 (the bridges and pushers before the first checkpoint); 12 and 13 to where the owner's runs
  end (sector 1; sector 2, where the stone ball stands at a fan it must ride as paper).
- Videos of the finished runs: `scripts/video.sh`, kept outside the repository.
