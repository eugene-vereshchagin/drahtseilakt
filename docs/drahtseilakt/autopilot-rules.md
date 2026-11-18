# How the test player drives: the rules

The test player's runs are the proof that a level can be played through, and their videos are watched. So it
drives as a confident, steady player does: on the middle of the track, at an even, moderate speed, without racing
lines and without hesitating at the obvious places. It slows down only where the level asks for it (a narrow
stretch, a timing). These rules say how; each level's route (`hints/Level_NN.route`) says where. The mechanics
they rest on are in [autopilot.md](autopilot.md). The `hints/` paths here are in the autopilot store (beside the
repository, `DRAHTSEILAKT_AUTOPILOT`), not in the repository.

## Pace

- **Cruising speed by floor.** On a rail or a narrow track a moderate speed (6 units a second), on a wide floor more
  (up to 10, with its width). Faster costs; slower than half of it costs too, where nothing calls for a wait.
- **No stop-and-go.** A hint's points are rolled through (`pass`), not stopped on (`goto`), unless the next step
  needs the ball still and in place: a run-up to a target, the start of a ram, the edge of a swing.
- **Stuck is short.** Two seconds without getting on, the test player tries its manoeuvres. Following a player's
  run the player's pace is the measure instead (four seconds, the plain steering), since the player may have
  waited there for a reason.

## Where the ball never stops

- **Rail ends and sills.** Where the way goes off a rail onto a floor, or up a small step, the ball keeps its
  momentum: held up there a moment, it backs off a little and takes it at speed.
- **Transformers.** Rolled into on the way, at speed; the game holds the ball while it changes, which is no stop
  of the player's. Held up by a transformer's rim, the ball goes in at once.
- **Gaps the way crosses.** Taken at once with a run-up, not after standing at the edge.
- **Straights, bends, sacks and swinging parts when clear.** No hesitation; a moving part in the way is waited for
  only as long as it is in the way.

## Rails and logs

- **A chain of points is a line.** Each `pass` keeps the ball on the line from the point passed before, so a chain
  written along rails or a narrow track follows them instead of cutting across.
- **Pairs of rails one above the other** (the ball rides the lower one leaning on the upper): `press` along them
  with an offset toward the upper rail, lined up on the pair's start first; fast enough at a pair's end to carry onto
  the next.
- **Wavy logs.** A ball stopped in a trough cannot climb out, a stone ball least of all: along a wavy log the ball
  keeps its speed (`press` at a high speed), between the transformers lying flat on it.

## Where the ball does stop or wait

- **Swings and lifts**: on the edge, still, until the moment found on copies of the run (`cross`).
- **Targets**: lined up exactly before the run-up (`hold`), then into the disc at speed and pushing on
  (`press` then `push`): a ball that only touches the disc leaves the bridge up.
- **Boxes to ram off**: a run-up from the same place for each ram, braking after it, so the ball does not follow
  the box over the edge.
- **Fans**: a paper ball waits over the grate only until it is lifted.

## Mechanisms

- **Planks standing on a hinge** (drawbridges, P_Modul_25): rammed down from a short run-up, braking at once after
  the hit so as not to follow the plank into the gap.
- **Pushers** (P_Modul_01): moved by hitting their disc, not by leaning on it; a few rams where one is not enough.
- **A loose ball as a key**: where a loose ball has to roll somewhere to trip a mechanism (a pusher's disc, a
  trapdoor), it is knocked there with a glancing hit along its row, the ball braking at once (level 11: two stone
  balls knocked off the grid onto the rails under it).
- **Seesaws** (P_Modul_30) are crossed from their low end with a run-up: from a standstill a stone ball does not get
  over the pivot.
- **A hint begins after a transformer has let the ball go**, not while it holds it: the way's place is found again
  after the hint, and a transformer passed under a hint is not known as passed.
- **A passed sector is gone**: its objects leave the physics and the picture, so the way never leads back into it.
- **Lifts with a cage** (P_Modul_03): a disc on a spring, held down by the walls of its cage standing loose on it.
  The walls are rammed off from the middle one by one (short rams, braking, so the ball stays on the disc); the disc
  rises to the next one's height, and the ball crosses onto it at a push (level 12).
- **A stair of fans**: each fan's column is narrow (2 wide) and holds a paper ball at its top (17 above the grate).
  From there the ball crosses at a push to the next grate below that height; slowly it falls between the columns.
  It must come over the next grate above it: where a column's top only reaches the next grate's height, the ball goes
  by a higher column first (level 12, sector 6).
- **Pins at a rail's end** on a platform stand closer than a ball is wide: the ball goes round them, not between.
- **A rail ending over another far below**: rolled off its end slowly, the ball drops straight onto the lower one.

## Balls

- **Paper** over fans and along walls of air; light: it needs a run-up for slopes and sagging plates.
- **Wood** for rails and most targets; heavy enough to tip a target hit fast and pushed.
- **Stone** to ram boxes and push balls; it breaks hanging plates (P_Modul_29): it does not cross them, unless the
  level means it to drop through them with the plates (level 6's towers).

## Zones the watcher marks

Where the owner sees the test player drive wrong, he marks the stretch while watching (`--watch`, F8 and F4;
[autopilot.md](autopilot.md), "Zones"), and the mark is a rule for that stretch from then on, in `<level>.zones`
beside the hints:

- **flow** ("don't dawdle here"): no stop, wait or slowing down there that the ball does not need. A hint's `goto`
  is rolled through, its `wait` and `hold` left out, a slow `pass` or `press` taken at 5, each only when a copy of
  the run shows the ball is kept so; the steering keeps at least 0.8 of the floor's pace, except before a gap or a
  climb that needs a run-up: backing off and stopping for it is no dawdling, so there a flow zone asks nothing.
- **careful** ("keep the speed limit here and don't cut corners"): not faster than the zone's speed (4.5 by
  default), on the way's middle line, no racing line; whatever drives is held under the limit. **Never where the
  physics needs momentum**: before a gap to cross or a climb steeper than the ball takes by its own push (stone
  12.6°, wood 26°, paper 31° at 4.5; by its push alone stone climbs nothing steeper than 17°, wood 37°, paper
  46°; the way read about 30 units ahead), at a sill, and where the running hint says speed (a
  `press`, `along` or `round` faster than the limit, or a `pass` faster than it that is the run-up to one), the
  limit is let go and only the line is kept. A mark meant to slow the ball down there could only lose it.

A zone of the watcher's comes before the pace rules above: it is what the owner saw.

## The learned table: `hints/learned.txt`

A zone also teaches the test player what kind of place it was, for every level, those not yet passed too (12, 13,
our own). At each mark it reads the place along the marked stretch, every tenth of a second, by eight features:

| Feature | Values |
|---|---|
| surface | `floor`, `rail`, `off` (on no floor of the way: a module's part, in the air) |
| width | the floor's: `narrow` (under 4), `medium` (under 10), `wide` |
| border | the floor's nearest edge within 8: `walls` (a wall stands there), `open` (it drops away), `far` |
| curve | the way's turn over the next 8 points: `straight` (under 20°), `bend` (under 60°), `sharp` |
| slope | the way's rise over them: `flat`, `up`, `down` (more than 0.15) |
| ahead | the first within them: `gap`, `drop`, `rail-end`, `rail-start`, `sill`, `transformer`, or `none` |
| module | the nearest module within 8 (`P_Modul_25`, `P_Box`, ...), or `none` |
| kind | `wood`, `stone`, `paper`, `sticky` |

What was the same over most (60%) of the stretch becomes a rule: the surface, the ball, and the values special to
the place (not a plain medium or wide floor, straight, flat, nothing ahead, no module); the plain ones are left out,
they would only make the rule hold in fewer places. A stretch with nothing special teaches nothing (its zone stays
the level's own). The rule is added to `learned.txt` beside the hints, one file for all levels, with where it was
learned and what the test player did there:

```
flow when surface=rail ahead=rail-end kind=wood   # Level_01 sector 2 (147.2 -2.1 -552.0) 2026-11-03: 3.0 s, stopping 40%, mostly hint, off the middle 0.0
careful speed 4.5 when surface=floor width=narrow curve=sharp kind=stone
```

A rule holds where every feature it names has that value; it asks what a zone of its kind asks (flow, careful). Every
run reads the table and looks at the place each tick: where the ball is in none of the level's own zones (they come
first) and rules hold, flow if any of them is flow, the lowest limit of the careful ones; the status line names the
most particular rule holding (`learned: rail, rail-end ahead, wood → don't dawdle`), and with F12 also the place as
the rules see it (`here: surface=rail ...`). A rule names at least two features, one of them special to the place,
so that none holds almost everywhere; a careful rule names two special to the place, as it holds the ball back
wherever it holds (a sharp bend alone is every level's every sharp corner; a bend near a reset point is not every
bend). A careful mark along a stretch with only one special feature teaches no rule (its zone stays the level's
own), and such a line in the table is left out with a note. Undoing a mark (Backspace) also takes out the rule it taught (by the mark's id in its evidence). A rule the table
has already is not added again; a table full (512 rules) takes no more, and the screen says so. The table is
plain text: a rule that turns out too wide is narrowed by adding a feature, or deleted. Read like a hint file, it is
untrusted: size (64 KiB), rules (512) and numbers capped, values from the lists above, a malformed line left out with
a note. `AUTOPILOT_SITUATION=1` prints the place as the rules see it every second of a run.

The features tell 2079 kinds of place apart without the module (1701 on floors, 189 on rails, 189 off them), each of
them again with each module near.

## A level's route: `hints/Level_NN.route`

One file a level, read by people and agents (the test player reads the plan and the hints made from it): for each
sector, its stretches in order, each with the ball it is driven as, the objects passed in order, what is done at
each and the pace.

```
sector 1 wood                         # the sector and the ball it begins with
  start → end platform x 166          pass, cruise
  rails x 122 south → Paper_01        pass into it → paper
  Modul_18_01 (fan)                   wait over the grate → lifted onto the rails x 80
  rails x 80 south → Wood_01          press at a push (a slope) → wood
  rails x 80 north, U over the fan    pass, cruise (a wood ball is not lifted)
  rails x 96 south → CP1              press, cruise; on over the sill onto the platform
```

Actions: **pass** (through, at the pace), **press** (along rails or a straight line, kept to it), **round** (a
helix), **hold** (still, lined up), **cross** (a swing, at the found moment), **ram** (boxes, from a run-up, braking),
**target** (hold, then press and push into the disc), **wait** (only over a fan's grate or for a moving part).
