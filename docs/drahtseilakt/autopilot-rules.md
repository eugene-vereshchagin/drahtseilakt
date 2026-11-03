# How the test player drives: the rules

The test player's runs are the proof that a level can be played through, and their videos are watched. So it
drives as a confident, steady player does: on the middle of the track, at an even, moderate speed, without racing
lines and without hesitating at the obvious places. It slows down only where the level asks for it (a narrow
stretch, a timing). These rules say how; each level's route (`hints/Level_NN.route`) says where. The mechanics
they rest on are in [autopilot.md](autopilot.md).

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

## Balls

- **Paper** over fans and along walls of air; light: it needs a run-up for slopes and sagging plates.
- **Wood** for rails and most targets; heavy enough to tip a target hit fast and pushed.
- **Stone** to ram boxes and push balls; it breaks hanging plates (P_Modul_29): it does not cross them, unless the
  level means it to drop through them with the plates (level 6's towers).

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
