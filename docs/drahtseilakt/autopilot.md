# The test player and its hints

`--autopilot` lets a test player play a level without a window: it finds a way through the level's floors and
transformers and steers the ball along it, trying each choice of arrows on a copy of the run. `--follow FILE`
makes it follow the ball's way in a player's replay instead. Some places it cannot pass alone: puzzles, levers
and pushers to hit first, updrafts to ride, rails to take at speed. For those it reads **hints**.

## Stuck reports

When the test player gets no further along its way for 20 seconds, it writes a report into `--report DIR`
(default: `drahtseilakt-stuck` in the system's temporary folder), one folder per place:

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
then the test player finds its place on the way again. Steps:

| Step | What it does |
|---|---|
| `goto X Y Z [T]` | roll to the point and stop there (T ticks at most, 600) |
| `via X Y Z [T]` | roll through the point without stopping; when it gets no nearer for 1.5 s it backs off and charges |
| `push DX DZ T` | push along the world direction (DX, DZ) for T ticks |
| `air DX DZ T` | push along (DX, DZ) until the ball has flown and landed again (T at most) |
| `wait T` | no arrows for T ticks |
| `turn N` | turn the view N quarters (negative: left) |
| `become KIND` | roll into the nearest transformer of that kind (1200 ticks at most) |
| `replay FILE FROM TO` | a player's inputs of ticks FROM..TO of the replay FILE (beside the hint file), turned to the test player's view |

Directions are world directions (X east, Z north as the level is built), so a hint does not depend on where the
view is turned; the arrows are chosen for the view at each tick. A hint file is untrusted: its size (64 KiB),
the number of hints (256) and steps (64 a hint) and every number are capped, a replay is a plain file name beside
it, and a malformed line refuses the whole file with its line number. `HINTS_TRACE=1` prints the ball every ten
ticks while a hint drives.

## Where it stands

- Level 1: the two pushers in sector 3 (hint: round by the lower lane, push them north); the level is finished.
- Level 2, sector 3: the drawbridge, the updrafts up to the stone transformer, the pusher, the rail down; sector
  4: the rail and the spiral ramp. The wavy half-pipe after it is not passed yet: the owner swings across it to
  keep the speed, which waypoints do not do.
- Level 5, sector 1: the lever P_Modul_37 is reached only over a narrow pair of rails taken at speed; with eight
  arrow directions the ball drifts off it. Not passed yet.
- Levels 3 (turning arms) and 4 (the stone-ball puzzle): no hints yet.
