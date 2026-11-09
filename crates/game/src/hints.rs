// Hints for the test player (autopilot.rs): where it cannot find the way itself (a puzzle, a fan to ride, things to
// push), a hint file tells it what a player does there. Claude writes them from the stuck reports' pictures, the
// owner's runs and the walkthrough videos (docs/drahtseilakt/autopilot.md). One file a level, a hint a line:
//
//   at X Y Z r R [sector N] [kind wood|stone|paper] : STEP ; STEP ; ...
//
// When the ball comes within R of the point (in that sector, as that ball), the steps run in order, once a life:
//
//   goto X Y Z [T]          roll to the point and stop there (T ticks at most, 600 if not given)
//   via X Y Z [T]           roll through the point without stopping (within 2 of it; T at most, 600); when it
//                           gets no nearer for a second and a half (a crest too high), it backs off and charges
//   push DX DZ T            push along the world direction (DX, DZ) for T ticks
//   air DX DZ T             push along (DX, DZ) until the ball has been in the air and landed again (T at most)
//   wait T                  no arrows for T ticks
//   turn N                  turn the view N quarters (negative: to the left)
//   become wood|stone|paper roll into the nearest transformer of that kind (1200 ticks at most)
//   sync X Y Z [T]         keep the ball on the point until the sector's loose objects have been put in their places
//                           (a checkpoint's sector begins 48 ticks after it): the state a run begun at that sector
//                           (`--sector N`) starts a recorded replay hint from, so those hints hold in a chain
//   replay FILE FROM TO     a player's inputs of ticks FROM..TO of the replay FILE (a plain name beside the hint file),
//                           turned to the test player's view
//   trace FILE FROM TO [GAIN] the same stretch closed-loop: the replay is run once as the hints load, for the ball's
//                           place and speed at each of its ticks; each tick the recorded arrows are the feed-forward,
//                           plus a correction (times GAIN, 1 if not given; 0 is none) toward the recorded place and
//                           speed at the matching point of the run (by progress along it, kept near one tick a tick),
//                           turned into the arrows nearest it in the view now. Lost (over 4 units off): it steers to
//                           the run's way ahead until it is on it again (or gives the ball back to the autopilot)
//
// The file is untrusted (hints may be shared): its size, the number of hints and steps and every number are capped,
// and a malformed line refuses the whole file with its line number.

use crate::sim::{BallKind, Sim, input};
use glam::Vec3;
use std::path::{Path, PathBuf};

pub const MAX_FILE: u64 = 64 * 1024;
const MAX_HINTS: usize = 256;
const MAX_STEPS: usize = 64;
/// The most replay steps a hint file may have (each plays its replay from the start to find the stretch).
const MAX_REPLAY_STEPS: usize = 16;
/// The largest replay a hint may name.
pub const MAX_REPLAY_FILE: u64 = 4 * 1024 * 1024;
const MAX_COORD: f32 = 100_000.0;
const MAX_TICKS: u64 = 60 * 60 * 10;
const MAX_RADIUS: f32 = 200.0;
const MAX_LEAD: f32 = 50.0;
/// The fastest a press step rolls, and how far off its line it keeps at most.
const MAX_PRESS_SPEED: f32 = 40.0;
const MAX_PRESS_OFFSET: f32 = 3.0;
/// A press is done once the ball, having rolled, has not moved faster than this for so many ticks (stopped at a
/// rail's end).
const PRESS_MOVING: f32 = 0.8;
const PRESS_SETTLE: u64 = 20;
/// The ticks a goto or become takes at most when the hint does not say.
const GOTO_TICKS: u64 = 600;
const BECOME_TICKS: u64 = 1200;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Goto(Vec3, u64),
    Via(Vec3, u64),
    Push(Vec3, u64),
    Air(Vec3, u64),
    Wait(u64),
    Turn(i32),
    Become(BallKind),
    /// As written: the replay's file (beside the hint file) and its ticks.
    Replay(PathBuf, u64, u64),
    /// A replay stretch read: the view's quarter at its first tick and the inputs of its ticks.
    Inputs(i32, Vec<u8>),
    /// As written: a replay stretch to follow closed-loop, its file, ticks and correction gain.
    Trace(PathBuf, u64, u64, f32),
    /// A trace read: the recorded run's ball and pushes, a tick each.
    Track(Box<Track>),
    /// Keep this far ahead of (behind, when negative) the loose body nearest the point as it rolls, for so many ticks.
    Lead(Vec3, f32, u64),
    /// Push the loose body (or module part) nearest the first point to the second, for so many ticks at most; going
    /// round it this far from its middle (a ball's size by default; a crate is bigger), and pushing only while it
    /// is slower than the speed, when one is given (a crate on a slope slides on by itself).
    Shove(Vec3, Vec3, u64, f32, Option<f32>),
    /// Hold the ball where it is until the loose body (or module part) nearest the first point is within the
    /// distance of the second, for so many ticks at most: a swing or a lift to get on or off at the right moment.
    Await(Vec3, Vec3, f32, u64),
    /// Keep the ball over the loose body (or module part) nearest the first point until it is within the distance
    /// of the second, for so many ticks at most: riding a swing across.
    Ride(Vec3, Vec3, f32, u64),
    /// Roll along the straight line from where the step begins to the point, kept to it sideways (with the offset
    /// from it, a vector across: rails ridden leaning on their upper rail), not faster than the speed, for so many
    /// ticks at most.
    Press(Vec3, Vec3, f32, u64),
    /// Push along the way the ball is rolling, not faster than the speed, for so many ticks: rails that lead it
    /// (a helix) taken at speed.
    Along(f32, u64),
    /// Keep the ball on the point for so many ticks: still where it must wait (a slope would roll it away).
    Hold(Vec3, u64),
    /// Keep the ball on the point until the current sector's objects have entered the physics (at most so many ticks).
    Sync(Vec3, u64),
    /// Roll through the point without stopping, no faster than the speed (a corner, a rail's end, a transformer
    /// rolled into on the way), for so many ticks at most.
    Pass(Vec3, f32, u64),
    /// Cross a moving part (a swing) toward the point: the moment and the length of the push along the direction
    /// are found on copies of the run from here (wait, push, let it carry, push off), the first that ends within the
    /// distance of the point, on a floor, the ball kept; then played. Closed on what the part is doing now.
    Cross(Vec3, Vec3, f32),
    /// Go round the upright axis through the point (x, z) at the radius, at the speed (one way round or, negative,
    /// the other), for so many ticks or until the ball is above the point's height: up a helix of rails.
    Round(Vec3, f32, f32, u64),
}

/// A recorded run as a `trace` step follows it: for each tick of the stretch, the ball's place and speed before it,
/// the world push the player's arrows gave in it, and whether the view was lifted.
#[derive(Clone, PartialEq, Default)]
pub struct Track {
    pub pos: Vec<Vec3>,
    pub vel: Vec<Vec3>,
    pub push: Vec<Vec3>,
    pub lift: Vec<bool>,
    /// The loose body the ball rode at each tick, if any (its object, and the ball's place and speed relative to it):
    /// on a raft the run is followed relative to the raft, wherever the raft is now.
    pub on: Vec<Option<(usize, Vec3, Vec3)>>,
    /// Before a loose body touches the ball (within `TRACE_MEET` ticks): that body's object and its place at the tick,
    /// so the trace waits where the run is when the body comes later than it did in the run.
    pub meet: Vec<Option<(usize, Vec3)>>,
    pub gain: f32,
}

/// A loose body carries the ball when its middle is below the ball's and this near across.
const CARRY_ACROSS: f32 = 3.5;
const CARRY_BELOW: f32 = 0.5;
/// A carrier this far from its place in the run is not followed.
const CARRY_TRUST: f32 = 8.0;

/// The loose body carrying the ball now, if any: its object and the ball's place and speed relative to it.
pub fn carrier(sim: &Sim) -> Option<(usize, Vec3, Vec3)> {
    let world = sim.world();
    let b = sim.ball.position;
    sim.loose()
        .iter()
        .filter_map(|&(object, body)| {
            let st = world.state(body)?;
            let d = b - st.pos;
            let across = Vec3::new(d.x, 0.0, d.z).length();
            (d.y > CARRY_BELOW && across < CARRY_ACROSS).then_some((object, d, sim.ball.velocity - st.vel, across))
        })
        .min_by(|a, c| a.3.total_cmp(&c.3))
        .map(|(o, d, v, _)| (o, d, v))
}

/// The loose body touching the ball at tick `i` of the run (its middle within `TOUCH` of the ball's), if any.
fn touch(track: &Track, bodies: &[Vec<(usize, Vec3)>], i: usize) -> Option<usize> {
    let ball = *track.pos.get(i)?;
    bodies
        .get(i)?
        .iter()
        .map(|&(o, p)| (o, p.distance(ball)))
        .filter(|b| b.1 < TOUCH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|b| b.0)
}

/// Fills `track.meet` from the loose bodies' places at each tick of the run (`bodies`, object and place).
pub fn fill_meet(track: &mut Track, bodies: &[Vec<(usize, Vec3)>]) {
    let n = track.on.len();
    track.meet = vec![None; n];
    for j in 0..n {
        if touch(track, bodies, j).is_some() {
            continue;
        }
        let next = (j + 1..(j + TRACE_MEET).min(n)).find_map(|i| touch(track, bodies, i));
        if let Some(object) = next
            && let Some(&(_, p)) = bodies.get(j).and_then(|b| b.iter().find(|b| b.0 == object))
        {
            track.meet[j] = Some((object, p));
        }
    }
}

/// How many ticks late the body about to touch the ball is, against the run (negative: early; 0 if none).
fn meet_late(track: &Track, k: usize, sim: &Sim) -> i64 {
    let Some(Some((object, _))) = track.meet.get(k) else {
        return 0;
    };
    let Some(body) = sim.loose().iter().find(|l| l.0 == *object).map(|l| l.1) else {
        return 0;
    };
    let Some(now) = sim.world().state(body).map(|s| s.pos) else {
        return 0;
    };
    // the run's tick whose place of the body is nearest where it is now (ties to the nearest tick)
    let mut best = (k, f32::INFINITY);
    for j in k.saturating_sub(TRACE_MEET)..(k + TRACE_MEET).min(track.meet.len()) {
        if let Some(Some((o, p))) = track.meet.get(j)
            && o == object
        {
            let cost = p.distance(now) + 0.002 * (j as f32 - k as f32).abs();
            if cost < best.1 {
                best = (j, cost);
            }
        }
    }
    k as i64 - best.0 as i64
}

/// Where the run had the ball at tick `k`, and its speed, as they are now: relative to the body it rode, when that
/// body is in the world, otherwise as recorded.
fn reference(track: &Track, k: usize, sim: &Sim) -> Option<(Vec3, Vec3)> {
    let (at, v) = (*track.pos.get(k)?, *track.vel.get(k)?);
    let Some(Some((object, d, dv))) = track.on.get(k) else {
        return Some((at, v));
    };
    let body = sim.loose().iter().find(|l| l.0 == *object).map(|l| l.1);
    // (a body far from where it was in the run, one put away or gone another way, is no guide)
    match body.and_then(|b| sim.world().state(b)) {
        Some(st) if (st.pos + *d).distance(at) < CARRY_TRUST => Some((st.pos + *d, st.vel + *dv)),
        _ => Some((at, v)),
    }
}

impl std::fmt::Debug for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Track({} ticks, gain {})", self.pos.len(), self.gain)
    }
}

/// How a trace keeps to its run: the correction's pull per unit off the recorded place and per unit/s off its speed.
const TRACE_KP: f32 = 0.35;
const TRACE_KD: f32 = 0.25;
/// The matching point is looked for this many ticks ahead of the last; skipping ahead costs this many units a
/// second skipped (so a wait in the run, the ball still, is kept as time, not jumped over).
const TRACE_WINDOW: usize = 90;
const TRACE_SKIP_COST: f32 = 30.0;
/// Further off the run than this the ball is lost: steered to the run's way this far ahead, looked for wider.
const TRACE_LOST: f32 = 4.0;
/// A lost ball is on the run again this near it.
const TRACE_FOUND: f32 = 2.5;
/// How far ahead of getting on a body the trace looks for it, and how late it may be (ticks) before the trace waits.
pub const TRACE_MEET: usize = 150;
/// A loose body this near the ball (middle to middle; balls touch at 4) touches it: a ball knocked on, a raft got on.
const TOUCH: f32 = 4.5;
const MEET_LATE: i64 = 3;
const TRACE_AHEAD: usize = 30;
const TRACE_SEARCH: usize = 600;
/// Lost this long, the trace ends (the autopilot takes over).
const TRACE_GIVE_UP: u64 = 600;

/// The nine arrow sets a trace chooses from (no Shift: the view is not turned).
const ARROW_SETS: [u8; 9] = [
    0,
    input::UP,
    input::DOWN,
    input::LEFT,
    input::RIGHT,
    input::UP | input::LEFT,
    input::UP | input::RIGHT,
    input::DOWN | input::LEFT,
    input::DOWN | input::RIGHT,
];

/// The world push of arrows with the view's axes (forward, right), as the sim gives it.
pub fn push_of(bits: u8, forward: Vec3, right: Vec3) -> Vec3 {
    let mut v = Vec3::ZERO;
    if bits & input::UP != 0 {
        v += forward;
    }
    if bits & input::DOWN != 0 {
        v -= forward;
    }
    if bits & input::SHIFT == 0 {
        if bits & input::RIGHT != 0 {
            v += right;
        }
        if bits & input::LEFT != 0 {
            v -= right;
        }
    }
    v
}

/// The arrow set whose push is nearest `want` (flat) with the view's axes, and what is left of `want`.
pub fn nearest_arrows(want: Vec3, forward: Vec3, right: Vec3) -> (u8, Vec3) {
    let want = Vec3::new(want.x, 0.0, want.z);
    let mut best = (0u8, want, f32::INFINITY);
    for bits in ARROW_SETS {
        let left = want - push_of(bits, forward, right);
        let d = left.length_squared();
        if d < best.2 - 1e-6 {
            best = (bits, left, d);
        }
    }
    (best.0, best.1)
}

/// The tick of `track` the ball matches, `next` being the tick due now (one on from the last matched): the nearest
/// recorded place from the last matched on, within the window, a tick off the due one costing `TRACE_SKIP_COST`
/// units a second (so a wait in the run stays a wait); `wide` looks further and without the cost (a lost ball found
/// again).
pub fn match_tick(track: &Track, next: usize, ball: Vec3, wide: bool) -> usize {
    let n = track.pos.len();
    if n == 0 {
        return 0;
    }
    let first = next.saturating_sub(1).min(n - 1);
    let end = (next + if wide { TRACE_SEARCH } else { TRACE_WINDOW }).min(n - 1);
    let mut best = (next.min(n - 1), f32::INFINITY);
    for j in first..=end {
        let Some(p) = track.pos.get(j) else { break };
        let skip = (j as f32 - next as f32).abs() / 60.0;
        let cost = p.distance(ball) + if wide { 0.0 } else { TRACE_SKIP_COST * skip };
        if cost < best.1 {
            best = (j, cost);
        }
    }
    best.0
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hint {
    pub at: Vec3,
    pub radius: f32,
    /// Sector numbers as the game shows them (1 is the first).
    pub sector: Option<usize>,
    pub kind: Option<BallKind>,
    pub steps: Vec<Step>,
    /// The line it was read from, for the reports.
    pub line: usize,
}

fn kind_of(s: &str) -> Option<BallKind> {
    match s.to_ascii_lowercase().as_str() {
        "wood" => Some(BallKind::Wood),
        "stone" => Some(BallKind::Stone),
        "paper" => Some(BallKind::Paper),
        _ => None,
    }
}

fn num(s: Option<&str>) -> Result<f32, String> {
    let v: f32 = s
        .ok_or("a number is missing")?
        .parse()
        .map_err(|_| "not a number")?;
    if v.is_finite() && v.abs() <= MAX_COORD {
        Ok(v)
    } else {
        Err("a number out of range".into())
    }
}

fn ticks(s: Option<&str>) -> Result<u64, String> {
    let v: u64 = s
        .ok_or("a tick count is missing")?
        .parse()
        .map_err(|_| "not a tick count")?;
    if v <= MAX_TICKS {
        Ok(v)
    } else {
        Err("a tick count out of range".into())
    }
}

fn direction(x: Option<&str>, z: Option<&str>) -> Result<Vec3, String> {
    let d = Vec3::new(num(x)?, 0.0, num(z)?).normalize_or_zero();
    if d == Vec3::ZERO {
        Err("a direction of no length".into())
    } else {
        Ok(d)
    }
}

/// A replay's file name: a plain name, beside the hint file (no folders, no way out of it).
/// A replay's file name beside the hint file: letters, digits, `_`, `-` and `.` only, ending in `.replay`, not
/// beginning with a dot, and no Windows device name (CON, NUL, COM1...).
fn plain_name(s: &str) -> Result<&str, String> {
    let stem = s
        .strip_suffix(".replay")
        .unwrap_or_default()
        .to_ascii_uppercase();
    let base = stem.split('.').next().unwrap_or_default();
    let device = matches!(base, "CON" | "PRN" | "AUX" | "NUL")
        || ((base.starts_with("COM") || base.starts_with("LPT")) && base.len() == 4);
    let plain = !stem.is_empty()
        && !s.starts_with('.')
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        && !device;
    if plain {
        Ok(s)
    } else {
        Err(format!("\"{s}\" is not a plain file name"))
    }
}

fn step(words: &[&str]) -> Result<Step, String> {
    let mut w = words.iter().copied();
    let name = w.next().ok_or("an empty step")?;
    let s = match name {
        "goto" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let t = match w.next() {
                Some(t) => ticks(Some(t))?,
                None => GOTO_TICKS,
            };
            Step::Goto(p, t)
        }
        "via" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let t = match w.next() {
                Some(t) => ticks(Some(t))?,
                None => GOTO_TICKS,
            };
            Step::Via(p, t)
        }
        "push" => Step::Push(direction(w.next(), w.next())?, ticks(w.next())?),
        "air" => Step::Air(direction(w.next(), w.next())?, ticks(w.next())?),
        "wait" => Step::Wait(ticks(w.next())?),
        "turn" => {
            let n: i32 = w
                .next()
                .ok_or("turn needs a count")?
                .parse()
                .map_err(|_| "not a turn count")?;
            if !(-3..=3).contains(&n) {
                return Err("turn is -3 to 3 quarters".into());
            }
            Step::Turn(n)
        }
        "lead" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let d = num(w.next())?;
            if !(-MAX_LEAD..=MAX_LEAD).contains(&d) {
                return Err(format!("lead is {MAX_LEAD} units at most"));
            }
            Step::Lead(p, d, ticks(w.next())?)
        }
        "shove" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let to = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let n = ticks(w.next())?;
            let round = match w.next() {
                Some(r) => num(Some(r))?,
                None => SHOVE_BEHIND,
            };
            if !(SHOVE_BEHIND..=MAX_SHOVE_ROUND).contains(&round) {
                return Err(format!("shove goes round at {SHOVE_BEHIND} to {MAX_SHOVE_ROUND}"));
            }
            let cap = match w.next() {
                Some(v) => {
                    let v = num(Some(v))?;
                    if !(0.5..=MAX_PRESS_SPEED).contains(&v) {
                        return Err(format!("shove is 0.5 to {MAX_PRESS_SPEED} fast"));
                    }
                    Some(v)
                }
                None => None,
            };
            Step::Shove(p, to, n, round, cap)
        }
        "await" | "ride" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let q = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let d = num(w.next())?;
            if !(0.0..=MAX_LEAD).contains(&d) {
                return Err(format!("{name} is within {MAX_LEAD} units at most"));
            }
            let t = ticks(w.next())?;
            if name == "await" {
                Step::Await(p, q, d, t)
            } else {
                Step::Ride(p, q, d, t)
            }
        }
        "round" => {
            let c = Vec3::new(num(w.next())?, 0.0, num(w.next())?);
            let r = num(w.next())?;
            if !(0.5..=MAX_LEAD).contains(&r) {
                return Err(format!("round is 0.5 to {MAX_LEAD} units across"));
            }
            let speed = num(w.next())?;
            if !(0.5..=MAX_PRESS_SPEED).contains(&speed.abs()) {
                return Err(format!("round is 0.5 to {MAX_PRESS_SPEED} fast"));
            }
            let t = ticks(w.next())?;
            // (done above this height, when given)
            let top = match w.next() {
                Some(y) => num(Some(y))?,
                None => f32::INFINITY,
            };
            Step::Round(Vec3::new(c.x, top, c.z), r, speed, t)
        }
        "cross" => {
            let d = direction(w.next(), w.next())?;
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let r = num(w.next())?;
            if !(0.5..=MAX_LEAD).contains(&r) {
                return Err(format!("cross is within 0.5 to {MAX_LEAD} units"));
            }
            Step::Cross(d, p, r)
        }
        "pass" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let speed = num(w.next())?;
            if !(0.5..=MAX_PRESS_SPEED).contains(&speed) {
                return Err(format!("pass is 0.5 to {MAX_PRESS_SPEED} fast"));
            }
            let t = match w.next() {
                Some(t) => ticks(Some(t))?,
                None => GOTO_TICKS,
            };
            Step::Pass(p, speed, t)
        }
        "hold" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            Step::Hold(p, ticks(w.next())?)
        }
        "sync" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            let n = match w.next() {
                Some(t) => ticks(Some(t))?,
                None => GOTO_TICKS,
            };
            Step::Sync(p, n)
        }
        "along" => {
            let speed = num(w.next())?;
            if !(0.5..=MAX_PRESS_SPEED).contains(&speed) {
                return Err(format!("along is 0.5 to {MAX_PRESS_SPEED} fast"));
            }
            Step::Along(speed, ticks(w.next())?)
        }
        "press" => {
            let p = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
            // (the offset from the straight line, in units: which side of the rails' middle the ball keeps to)
            let side = Vec3::new(num(w.next())?, 0.0, num(w.next())?);
            if side.length() > MAX_PRESS_OFFSET {
                return Err(format!(
                    "press keeps {MAX_PRESS_OFFSET} units off the line at most"
                ));
            }
            let speed = num(w.next())?;
            if !(0.5..=MAX_PRESS_SPEED).contains(&speed) {
                return Err(format!("press is 0.5 to {MAX_PRESS_SPEED} fast"));
            }
            Step::Press(p, side, speed, ticks(w.next())?)
        }
        "become" => {
            Step::Become(kind_of(w.next().ok_or("become needs a kind")?).ok_or("not a ball kind")?)
        }
        "replay" => {
            let file = plain_name(w.next().ok_or("replay needs a file")?)?;
            let (from, to) = (ticks(w.next())?, ticks(w.next())?);
            if to <= from {
                return Err("replay's ticks must grow".into());
            }
            Step::Replay(PathBuf::from(file), from, to)
        }
        "trace" => {
            let file = plain_name(w.next().ok_or("trace needs a file")?)?;
            let (from, to) = (ticks(w.next())?, ticks(w.next())?);
            if to <= from {
                return Err("trace's ticks must grow".into());
            }
            let gain = match w.next() {
                Some(g) => num(Some(g))?,
                None => 1.0,
            };
            if !(0.0..=4.0).contains(&gain) {
                return Err("trace's gain is 0 to 4".into());
            }
            Step::Trace(PathBuf::from(file), from, to, gain)
        }
        other => return Err(format!("unknown step \"{other}\"")),
    };
    if w.next().is_some() {
        return Err(format!("too many words for {name}"));
    }
    Ok(s)
}

fn hint(line: &str, number: usize) -> Result<Hint, String> {
    let (head, tail) = line
        .split_once(':')
        .ok_or("no ':' between the place and the steps")?;
    let words: Vec<&str> = head.split_whitespace().collect();
    let mut w = words.iter().copied();
    if w.next() != Some("at") {
        return Err("a hint begins with \"at\"".into());
    }
    let at = Vec3::new(num(w.next())?, num(w.next())?, num(w.next())?);
    if w.next() != Some("r") {
        return Err("the place needs its radius (r R)".into());
    }
    let radius = num(w.next())?;
    if !(radius > 0.0 && radius <= MAX_RADIUS) {
        return Err("the radius is out of range".into());
    }
    let (mut sector, mut kind) = (None, None);
    while let Some(k) = w.next() {
        match k {
            "sector" => {
                let n: usize = w
                    .next()
                    .ok_or("sector needs a number")?
                    .parse()
                    .map_err(|_| "not a sector")?;
                if !(1..=64).contains(&n) {
                    return Err("the sector is out of range".into());
                }
                sector = Some(n);
            }
            "kind" => {
                kind = Some(kind_of(w.next().ok_or("kind needs a ball")?).ok_or("not a ball kind")?)
            }
            other => return Err(format!("unknown condition \"{other}\"")),
        }
    }
    let steps: Vec<Step> = tail
        .split(';')
        .map(|s| s.split_whitespace().collect::<Vec<_>>())
        .filter(|s| !s.is_empty())
        .map(|s| step(&s))
        .collect::<Result<_, _>>()?;
    if steps.is_empty() || steps.len() > MAX_STEPS {
        return Err(format!("a hint has 1 to {MAX_STEPS} steps"));
    }
    Ok(Hint {
        at,
        radius,
        sector,
        kind,
        steps,
        line: number,
    })
}

/// The hints of a file's text; `#` begins a comment.
pub fn parse(text: &str) -> Result<Vec<Hint>, String> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let number = i.saturating_add(1);
        out.push(hint(line, number).map_err(|e| format!("line {number}: {e}"))?);
        if out.len() > MAX_HINTS {
            return Err(format!("more than {MAX_HINTS} hints"));
        }
    }
    let replays = out
        .iter()
        .flat_map(|h| &h.steps)
        .filter(|s| matches!(s, Step::Replay(..) | Step::Trace(..)))
        .count();
    if replays > MAX_REPLAY_STEPS {
        return Err(format!("more than {MAX_REPLAY_STEPS} replay steps"));
    }
    Ok(out)
}

/// The hint file at `path`: none there is no hints; one too big, a link or malformed is an error.
pub fn load(path: &Path) -> Result<Vec<Hint>, String> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    if !meta.is_file() {
        return Err(format!("{}: not a plain file", path.display()));
    }
    let bytes = read_plain(path, MAX_FILE)?;
    let text = String::from_utf8(bytes).map_err(|_| format!("{}: not UTF-8", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A file shared with the hints: a plain file itself (never a link, a device or a pipe), read up to `cap` bytes
/// whatever it says its length is.
pub fn read_plain(path: &Path, cap: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let plain = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file());
    if !plain {
        return Err(format!("{}: not a plain file", path.display()));
    }
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if u64::try_from(bytes.len()).map_or(true, |n| n > cap) {
        return Err(format!("{}: more than {cap} bytes", path.display()));
    }
    Ok(bytes)
}

/// Whether a folder on the way to a file the watcher's keys write is a link: the folder itself and those above it,
/// up to the current folder for a relative path, or within the home folder for an absolute one (above that the
/// system's own layout is no concern of ours).
fn linked_folder(dir: &Path) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    for d in dir.ancestors() {
        if d.as_os_str().is_empty() {
            break;
        }
        if dir.is_absolute()
            && !home
                .as_ref()
                .is_some_and(|h| d.starts_with(h) && d != h.as_path())
            && d != dir
        {
            break;
        }
        if std::fs::symlink_metadata(d).is_ok_and(|m| m.file_type().is_symlink()) {
            return Some(d.to_path_buf());
        }
    }
    None
}

/// Writes `text` as the file at `path` for the watcher's keys (zones, the learned table, a hand-driven stretch and
/// its hint): never through a link (the file, or a folder on the way: `linked_folder`), and whole or not at all:
/// written to a new file beside it, synced, and renamed into place.
pub fn write_plain(path: &Path, text: &str) -> Result<(), String> {
    use std::io::Write as _;
    let shown = |p: &Path| p.display().to_string();
    if std::fs::symlink_metadata(path).is_ok_and(|m| !m.is_file()) {
        return Err(format!("{}: not a plain file, not written", shown(path)));
    }
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    if let Some(l) = linked_folder(&dir) {
        return Err(format!("{}: a link on the way, not written", shown(&l)));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", shown(&dir)))?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        // a leftover of an earlier run (a file or a link) is removed itself, never followed
        if std::fs::symlink_metadata(&tmp).is_ok() {
            std::fs::remove_file(&tmp)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", shown(path))
    })
}

/// The text of a plain file the watcher's keys add to (empty when there is none yet), read up to `cap` bytes.
pub fn read_text(path: &Path, cap: u64) -> Result<String, String> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
        Ok(_) => {}
    }
    String::from_utf8(read_plain(path, cap)?).map_err(|_| format!("{}: not UTF-8", path.display()))
}

/// A view at `quarter`, for the axes its arrows push along.
fn view(quarter: i32) -> crate::sim::Camera {
    crate::sim::Camera {
        quarter,
        yaw: 0.0,
        base: 0.0,
        lift: 0.0,
        position: Vec3::ZERO,
        step: Vec3::ZERO,
        target: Vec3::ZERO,
    }
}

/// The world push of arrows `bits` with the view at `quarter` (the sim's own rule: Left / Right only without Shift).
pub fn world_push(bits: u8, quarter: i32) -> Vec3 {
    let cam = view(quarter);
    let (f, r) = (cam.push_forward(), cam.push_right());
    let mut v = Vec3::ZERO;
    if bits & input::UP != 0 {
        v += f;
    }
    if bits & input::DOWN != 0 {
        v -= f;
    }
    if bits & input::SHIFT == 0 {
        if bits & input::RIGHT != 0 {
            v += r;
        }
        if bits & input::LEFT != 0 {
            v -= r;
        }
    }
    v
}

/// The arrows that push along `v` (its larger parts) with the view at `quarter`.
pub fn arrows(v: Vec3, quarter: i32) -> u8 {
    let cam = view(quarter);
    let (f, r) = (v.dot(cam.push_forward()), v.dot(cam.push_right()));
    let big = f.abs().max(r.abs());
    if big < 1e-3 {
        return 0;
    }
    let mut bits = 0;
    if f > 0.35 * big {
        bits |= input::UP;
    } else if f < -0.35 * big {
        bits |= input::DOWN;
    }
    if r > 0.35 * big {
        bits |= input::RIGHT;
    } else if r < -0.35 * big {
        bits |= input::LEFT;
    }
    bits
}

/// A step running: its tick of start, and what it keeps (the replay's view quarter, whether the ball has flown).
#[derive(Debug, Clone)]
struct Running {
    hint: usize,
    step: usize,
    start: u64,
    quarter: i32,
    flown: bool,
    prev: u8,
    /// A via's nearest yet and when, and the tick its backing off for a run-up ends.
    nearest: f32,
    nearest_at: u64,
    back_until: u64,
    /// The loose body a lead keeps to.
    body: Option<usize>,
    /// Where a press step began.
    from: Vec3,
    /// A cross step's pushes found, one a tick (world directions; empty: none found).
    pushes: Vec<Vec3>,
    /// The point the last pass step went through (kept from step to step: the next pass keeps to the line from it).
    passed: Option<Vec3>,
    /// In a flow zone (zones.rs) the step is run without its stop: a goto rolled through, a wait or hold left out,
    /// a slow pass or press taken at `FLOW_SPEED`, when a copy of the run shows the ball is kept so.
    flowed: bool,
    /// A trace's tick of its run due next, what its arrows left of the push wanted, and since when it is lost.
    k: usize,
    residual: Vec3,
    lost_since: Option<u64>,
}

/// A step ending with the ball this far below its point has lost it to a floor beneath (`Runner::off_course`).
const OFF_COURSE_DROP: f32 = 3.0;

/// The point a step rolls the ball to, if it has one.
fn step_point(step: &Step) -> Option<Vec3> {
    match *step {
        Step::Goto(p, _)
        | Step::Via(p, _)
        | Step::Pass(p, _, _)
        | Step::Press(p, _, _, _)
        | Step::Hold(p, _)
        | Step::Sync(p, _) => Some(p),
        _ => None,
    }
}

/// A pass faster than a careful zone's limit this near before a push at speed (along the hint's points) is its
/// run-up, which the zone does not hold back.
const RUN_UP: f32 = 25.0;

/// In a flow zone a pass or press slower than this is taken at it.
pub const FLOW_SPEED: f32 = 5.0;
/// A flowed step is tried on a copy of the run this long (the rest of the hint) and a second after.
const FLOW_CHECK: u64 = 600;
const FLOW_COAST: u64 = 60;

/// Runs the hints for the test player.
#[derive(Debug, Clone, Default)]
pub struct Runner {
    pub hints: Vec<Hint>,
    fired: Vec<bool>,
    running: Option<Running>,
    lives: Option<u32>,
    /// A hint has just ended: the pilot finds its place on the way anew.
    pub ended: bool,
    /// The tick the last hint began.
    pub began: u64,
    /// What the watcher's zones ask where the ball is now (set by the test player each tick).
    pub zone: crate::zones::Effect,
    /// A copy of the runner trying a step on a copy of the run: says nothing.
    quiet: bool,
    /// A step of the running hint ended well below its point (the ball fell to a floor beneath): the hint is off
    /// course, and its further steps are no getting on (the test player goes back from there as from a way that
    /// stands still).
    off_course: bool,
}

impl Runner {
    /// Where a running hint is (its index and step), so that a hint going through its steps counts as getting on.
    pub fn at(&self) -> Option<(usize, usize)> {
        self.running.as_ref().map(|r| (r.hint, r.step))
    }

    /// Whether the step running says the ball needs speed above `cap` (a careful zone's limit), and why: a push up a
    /// climb, along rails or into a ram (`press`, `along`, `round` faster than it), or a `pass` faster than it that
    /// is the run-up to one of those within `RUN_UP` of the ball, along the hint's points. There the zone lets the
    /// limit go (autopilot.rs, `Pilot::momentum`).
    pub fn says_speed(&self, ball: Vec3, cap: f32) -> Option<&'static str> {
        let run = self.running.as_ref()?;
        let steps = self.hints.get(run.hint)?.steps.get(run.step..)?;
        let mut from = ball;
        let mut gone = 0.0;
        for (i, step) in steps.iter().enumerate() {
            let pushes = match *step {
                Step::Press(_, _, v, _) | Step::Along(v, _) => v > cap,
                Step::Round(_, _, v, _) => v.abs() > cap,
                _ => false,
            };
            if pushes {
                return Some(if i == 0 {
                    "the hint pushes on at speed"
                } else {
                    "the hint's run-up"
                });
            }
            match *step {
                Step::Pass(p, v, _) if v > cap => {
                    gone += from.distance(p);
                    from = p;
                    if gone > RUN_UP {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        None
    }

    /// Whether the running hint went off course: a step ended with the ball well below its point.
    pub fn off_course(&self) -> bool {
        self.running.is_some() && self.off_course
    }

    /// What runs now, for whoever watches: the hint's line, its step and the step, if a hint runs.
    pub fn describe(&self) -> Option<String> {
        let r = self.running.as_ref()?;
        let h = self.hints.get(r.hint)?;
        let step = h.steps.get(r.step).map_or_else(String::new, |s| {
            let t = format!("{s:?}");
            t.chars().take(48).collect()
        });
        Some(format!(
            "hint line {} step {}/{}: {step}",
            h.line,
            r.step + 1,
            h.steps.len()
        ))
    }

    /// The running hint's step is skipped (the watcher sees it will not do): on with the next one.
    pub fn skip_step(&mut self, sim: &Sim) {
        if let Some(r) = self.running.as_mut() {
            r.step += 1;
            self.begin(sim);
        }
    }

    /// Whether a running hint is playing a recorded run: a replay step gets on by itself however long it is.
    pub fn replaying(&self) -> bool {
        self.running.as_ref().is_some_and(|r| {
            let step = self.hints.get(r.hint).and_then(|h| h.steps.get(r.step));
            matches!(
                step,
                Some(Step::Replay(..) | Step::Inputs(..) | Step::Trace(..) | Step::Track(..))
            )
        })
    }
}

impl Runner {
    pub fn new(hints: Vec<Hint>) -> Runner {
        let fired = vec![false; hints.len()];
        Runner {
            hints,
            fired,
            ..Runner::default()
        }
    }

    /// After the test player went back in its run (instead of losing a ball): the hints may fire again, as for a new
    /// ball (the one running goes on).
    pub fn rearm(&mut self) {
        let running = self.running.as_ref().map(|r| r.hint);
        for (i, f) in self.fired.iter_mut().enumerate() {
            *f = Some(i) == running;
        }
    }

    /// One more hint, made while playing (the test player's own, from the run it follows).
    pub fn add(&mut self, hint: Hint) {
        if self.hints.len() < MAX_HINTS * 4 {
            self.hints.push(hint);
            self.fired.push(false);
        }
    }

    /// The input of a hint for this tick, or None when no hint drives.
    pub fn input(&mut self, sim: &Sim) -> Option<u8> {
        // a lost ball: the hints may fire again for the new one
        let lives = sim.lives;
        if self.lives.is_some_and(|l| lives < l) {
            self.fired.iter_mut().for_each(|f| *f = false);
            self.running = None;
        }
        self.lives = Some(lives);
        if sim.is_dying() {
            self.running = None;
            return None;
        }
        if self.running.is_none() {
            let pos = sim.ball.position;
            let i = self.hints.iter().enumerate().position(|(i, h)| {
                !self.fired.get(i).copied().unwrap_or(true)
                    && h.at.distance(pos) <= h.radius
                    && h.sector.is_none_or(|s| s == sim.sector + 1)
                    && h.kind.is_none_or(|k| k == sim.ball.kind)
            })?;
            if let Some(f) = self.fired.get_mut(i) {
                *f = true;
            }
            if !self.quiet {
                eprintln!(
                    "hint (line {}) at tick {}",
                    self.hints.get(i).map_or(0, |h| h.line),
                    sim.tick
                );
            }
            self.began = sim.tick;
            self.off_course = false;
            self.running = Some(Running {
                hint: i,
                step: 0,
                start: sim.tick,
                quarter: 0,
                flown: false,
                prev: 0,
                nearest: f32::INFINITY,
                nearest_at: sim.tick,
                back_until: 0,
                body: None,
                from: Vec3::ZERO,
                pushes: Vec::new(),
                passed: None,
                flowed: false,
                k: 0,
                residual: Vec3::ZERO,
                lost_since: None,
            });
            self.begin(sim);
        }
        loop {
            let run = self.running.as_mut()?;
            let Some(step) = self.hints.get(run.hint).and_then(|h| h.steps.get(run.step)) else {
                self.running = None;
                self.ended = true;
                return None;
            };
            let step = adjust(step, run.flowed, self.zone.cap);
            if let Some(i) = drive(&step, run, sim) {
                // HINTS_TRACE: the ball every ten ticks while a hint drives, to write hints by
                if std::env::var_os("HINTS_TRACE").is_some() && sim.tick.is_multiple_of(10) {
                    let (p, v) = (sim.ball.position, sim.ball.velocity);
                    eprintln!(
                        "    t {} step {} input {i:02x} ({:.1} {:.1} {:.1}) v ({:.1} {:.1} {:.1})",
                        sim.tick,
                        run.step + 1,
                        p.x,
                        p.y,
                        p.z,
                        v.x,
                        v.y,
                        v.z
                    );
                }
                return Some(i);
            }
            run.step += 1;
            let p = sim.ball.position;
            let below = step_point(&step).is_some_and(|q| q.y - p.y > OFF_COURSE_DROP);
            if !self.quiet {
                eprintln!(
                    "  step {} done at tick {} ball ({:.1} {:.1} {:.1}){}",
                    run.step,
                    sim.tick,
                    p.x,
                    p.y,
                    p.z,
                    if below && !self.off_course {
                        ", well below its point: off course"
                    } else {
                        ""
                    }
                );
            }
            self.off_course |= below;
            self.begin(sim);
        }
    }

    /// A step begins now.
    fn begin(&mut self, sim: &Sim) {
        let Some(run) = self.running.as_mut() else {
            return;
        };
        run.start = sim.tick;
        run.flown = false;
        run.prev = 0;
        run.nearest = f32::INFINITY;
        run.nearest_at = sim.tick;
        run.back_until = 0;
        run.body = None;
        run.from = Vec3::ZERO;
        run.pushes.clear();
        run.flowed = false;
        run.k = 0;
        run.residual = Vec3::ZERO;
        run.lost_since = None;
        let step = self.hints.get(run.hint).and_then(|h| h.steps.get(run.step));
        if let Some(Step::Inputs(q, _)) = step {
            run.quarter = *q;
        }
        // in a flow zone a step that stops the ball goes without its stop, when a copy of the run keeps the ball so
        if self.zone.flow && step.is_some_and(|s| flowing(s).is_some()) && self.flow_keeps_ball(sim)
        {
            if let Some(run) = self.running.as_mut() {
                run.flowed = true;
            }
            if !self.quiet {
                eprintln!("  step {} without its stop (a flow zone)", run_step(self));
            }
        }
    }

    /// Whether the running step, flowed (`adjust`), keeps the ball: the rest of the hint and a second after, on a
    /// copy of the run (as written from there on: one step at a time is flowed).
    fn flow_keeps_ball(&self, sim: &Sim) -> bool {
        let mut r = self.clone();
        r.zone = crate::zones::Effect::default();
        r.quiet = true;
        let Some(run) = r.running.as_mut() else {
            return false;
        };
        run.flowed = true;
        let mut s = sim.clone();
        let lives = s.lives;
        let gone = |s: &Sim| s.lives < lives || s.is_dying() || s.game_over;
        let mut coast = 0;
        for _ in 0..FLOW_CHECK + FLOW_COAST {
            let i = if r.running.is_some() {
                r.input(&s).unwrap_or(0)
            } else {
                0
            };
            if r.running.is_none() {
                coast += 1;
                if coast > FLOW_COAST {
                    break;
                }
            }
            s.step(i);
            if gone(&s) {
                return false;
            }
            if s.finished {
                break;
            }
        }
        true
    }
}

/// The running step's number (from 1), for the log.
fn run_step(r: &Runner) -> usize {
    r.running.as_ref().map_or(0, |run| run.step + 1)
}

/// What a step does without its stop in a flow zone: a goto rolled through, a wait or hold left out, a slow pass or
/// press at `FLOW_SPEED`; None for a step that has no stop to leave out.
fn flowing(step: &Step) -> Option<Step> {
    match *step {
        Step::Goto(p, n) => Some(Step::Via(p, n)),
        Step::Wait(_) => Some(Step::Wait(0)),
        Step::Hold(p, _) => Some(Step::Hold(p, 0)),
        Step::Pass(p, v, n) if v < FLOW_SPEED => Some(Step::Pass(p, FLOW_SPEED, n)),
        Step::Press(p, side, v, n) if v < FLOW_SPEED => Some(Step::Press(p, side, FLOW_SPEED, n)),
        _ => None,
    }
}

/// The step as the zones have it run: without its stop (`flowed`), and with its speed not above a careful zone's
/// limit (`cap`).
fn adjust(step: &Step, flowed: bool, cap: Option<f32>) -> std::borrow::Cow<'_, Step> {
    use std::borrow::Cow;
    let mut out = Cow::Borrowed(step);
    if flowed && let Some(f) = flowing(step) {
        out = Cow::Owned(f);
    }
    if let Some(c) = cap {
        let capped = match *out {
            Step::Pass(p, v, n) if v > c => Some(Step::Pass(p, c, n)),
            Step::Press(p, side, v, n) if v > c => Some(Step::Press(p, side, c, n)),
            Step::Along(v, n) if v > c => Some(Step::Along(c, n)),
            Step::Round(p, r, v, n) if v.abs() > c => Some(Step::Round(p, r, c * v.signum(), n)),
            _ => None,
        };
        if let Some(s) = capped {
            out = Cow::Owned(s);
        }
    }
    out
}

/// The loose body (and, with `parts`, the module part) nearest `at`; usize::MAX for none.
fn nearest_body(sim: &Sim, at: Vec3, parts: bool) -> usize {
    let world = sim.world();
    let parts = sim.module_bodies().iter().filter(|_| parts).map(|m| m.body);
    sim.loose()
        .iter()
        .map(|&(_, b)| b)
        .chain(parts)
        .filter_map(|b| world.state(b).map(|st| (b, st.pos.distance(at))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(usize::MAX, |n| n.0)
}

/// The waits before a cross step's push, its lengths and the coasts after it that are tried (ticks), how long the
/// push off goes on, and how long the ball is let settle before it is judged.
const CROSS_WAITS: std::ops::RangeInclusive<u64> = 0..=150;
const CROSS_WAIT_STEP: usize = 5;
const CROSS_PUSHES: [u64; 9] = [40, 55, 70, 85, 100, 115, 130, 145, 160];
const CROSS_COASTS: [u64; 4] = [0, 20, 40, 60];
const CROSS_OFF: u64 = 60;
const CROSS_SETTLE: u64 = 60;

/// A cross step's pushes, one a tick: the first wait, push, coast found on copies of the run that brings the ball
/// within `r` of `to`, on a floor and not lost; none found, none.
fn cross_plan(sim: &Sim, dir: Vec3, to: Vec3, r: f32) -> Vec<Vec3> {
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
    let quarter = sim.camera.quarter;
    for wait in CROSS_WAITS.step_by(CROSS_WAIT_STEP) {
        // (the wait played once, shared by every push after it)
        let mut waited = sim.clone();
        for _ in 0..wait {
            waited.step(0);
        }
        for push in CROSS_PUSHES {
            for coast in CROSS_COASTS {
                let mut s = waited.clone();
                let lives = s.lives;
                let mut plan: Vec<Vec3> = vec![Vec3::ZERO; usize::try_from(wait).unwrap_or(0)];
                let mut lost = false;
                for k in 0..push + coast + CROSS_OFF + CROSS_SETTLE {
                    let p = if k < push || (k >= push + coast && k < push + coast + CROSS_OFF) {
                        dir
                    } else {
                        Vec3::ZERO
                    };
                    if k < push + coast + CROSS_OFF {
                        plan.push(p);
                    }
                    s.step(arrows(p, quarter));
                    if s.lives < lives || s.is_dying() || s.game_over {
                        lost = true;
                        break;
                    }
                }
                if !lost
                    && s.ball.on_ground
                    && flat(s.ball.position - to).length() < r
                    && s.ball.position.y > to.y - 2.0
                {
                    return plan;
                }
            }
        }
    }
    Vec::new()
}

/// A shove is done with the body this near its place; the ball pushes it from this far behind its middle.
const SHOVE_NEAR: f32 = 1.0;
const SHOVE_BEHIND: f32 = 3.5;
/// The farthest a shove goes round its body (a large crate).
const MAX_SHOVE_ROUND: f32 = 10.0;

/// The input of `step` for this tick, or None when it is done.
fn drive(step: &Step, run: &mut Running, sim: &Sim) -> Option<u8> {
    let t = sim.tick.saturating_sub(run.start);
    let quarter = sim.camera.quarter;
    // (a chain of pass steps is a line; any other step between them breaks it)
    if !matches!(step, Step::Pass(..)) {
        run.passed = None;
    }
    let ball = &sim.ball;
    // rolling to a point: push towards it, less the speed already going there, so the ball stops on it
    let towards = |p: Vec3| {
        let d = Vec3::new(p.x - ball.position.x, 0.0, p.z - ball.position.z);
        let v = Vec3::new(ball.velocity.x, 0.0, ball.velocity.z);
        arrows(d.clamp_length_max(6.0) - 0.8 * v, quarter)
    };
    match step {
        Step::Goto(p, limit) => {
            let d = Vec3::new(p.x - ball.position.x, 0.0, p.z - ball.position.z).length();
            let still = Vec3::new(ball.velocity.x, 0.0, ball.velocity.z).length() < 2.0;
            (t < *limit && !(d < 1.0 && still)).then(|| towards(*p))
        }
        Step::Via(p, limit) => {
            let d = Vec3::new(p.x - ball.position.x, 0.0, p.z - ball.position.z);
            // across the way's sideways speed only, so the ball keeps its run along it
            let v = Vec3::new(ball.velocity.x, 0.0, ball.velocity.z);
            let side = v - d.normalize_or_zero() * v.dot(d.normalize_or_zero());
            if t >= *limit || d.length() < 2.0 {
                return None;
            }
            if d.length() < run.nearest - 0.3 {
                run.nearest = d.length();
                run.nearest_at = sim.tick;
            }
            // no nearer for a second and a half: back off for a run-up, then charge
            if sim.tick > run.nearest_at + 90 {
                run.back_until = sim.tick + 40;
                run.nearest_at = sim.tick + 40;
                run.nearest = f32::INFINITY;
            }
            if sim.tick < run.back_until {
                return Some(arrows(-d, quarter));
            }
            Some(arrows(d.normalize_or_zero() * 6.0 - 0.8 * side, quarter))
        }
        Step::Push(d, n) => (t < *n).then(|| arrows(*d, quarter)),
        Step::Air(d, n) => {
            if !ball.on_ground {
                run.flown = true;
            }
            (t < *n && !(run.flown && ball.on_ground)).then(|| arrows(*d, quarter))
        }
        Step::Wait(n) => (t < *n).then_some(0),
        Step::Turn(n) => {
            // a press every fourth tick, released between
            let presses = u64::from(n.unsigned_abs());
            if t >= presses * 4 {
                return None;
            }
            let key = if *n > 0 { input::RIGHT } else { input::LEFT };
            Some(if t.is_multiple_of(4) {
                input::SHIFT | key
            } else {
                0
            })
        }
        Step::Become(k) => {
            if ball.kind == *k || t >= BECOME_TICKS {
                return None;
            }
            let near = sim.trafos().filter(|(_, kind)| kind == k).min_by(|a, b| {
                a.0.distance(ball.position)
                    .total_cmp(&b.0.distance(ball.position))
            })?;
            Some(towards(near.0))
        }
        Step::Shove(at, to, n, round, cap) => {
            if t >= *n {
                return None;
            }
            let body = *run.body.get_or_insert_with(|| nearest_body(sim, *at, true));
            let st = sim.world().state(body)?;
            let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
            let left = flat(*to - st.pos);
            if left.length() < SHOVE_NEAR {
                return None;
            }
            let dir = left.normalize();
            // behind the body as seen from where it must go, then on into it
            let rel = flat(ball.position - st.pos);
            let behind = st.pos - dir * *round;
            let off = flat(behind - ball.position);
            let v = flat(ball.velocity);
            let ahead = rel.dot(dir) > -0.5 * *round;
            if ahead || (off.length() > 2.0 && rel.normalize_or_zero().dot(-dir) < 0.9) {
                // round it on the side the ball is: to beside it, then behind
                let perp = Vec3::new(-dir.z, 0.0, dir.x);
                let side = perp * rel.dot(perp).signum();
                // (beside it, then on round by its corner: going straight from beside to behind would cut across a
                // large body's corner)
                let beside = st.pos + side * *round;
                let to = if !ahead {
                    behind
                } else if flat(beside - ball.position).length() > 1.5 {
                    beside
                } else {
                    st.pos + (side - dir).normalize_or_zero() * *round
                };
                let d = flat(to - ball.position);
                return Some(arrows(d.clamp_length_max(6.0) - 0.8 * v, quarter));
            }
            // pushing: slowly, so the body is not knocked past the place or off the floor; with a speed, not while the
            // body goes faster (the ball holds back)
            if cap.is_some_and(|c| flat(st.vel).length() > c) {
                return Some(arrows(-0.8 * v, quarter));
            }
            let along = dir * 3.0 + (off - dir * off.dot(dir));
            Some(arrows(along - 0.8 * (v - flat(st.vel)), quarter))
        }
        Step::Lead(at, d, n) => {
            if t >= *n {
                return None;
            }
            let world = sim.world();
            let body = *run
                .body
                .get_or_insert_with(|| nearest_body(sim, *at, false));
            let st = world.state(body)?;
            let v = Vec3::new(st.vel.x, 0.0, st.vel.z);
            // ahead along where it rolls (or, still, on the side the ball is), a little ahead of where it will be
            let along = if v.length() > 1.0 {
                v.normalize()
            } else {
                Vec3::new(ball.position.x - st.pos.x, 0.0, ball.position.z - st.pos.z)
                    .normalize_or_zero()
                    * d.signum()
            };
            let p = st.pos + along * *d + v * 0.3;
            let off = Vec3::new(p.x - ball.position.x, 0.0, p.z - ball.position.z);
            let rel = Vec3::new(ball.velocity.x, 0.0, ball.velocity.z) - v;
            Some(arrows(off.clamp_length_max(6.0) - 0.8 * rel, quarter))
        }
        Step::Await(at, q, d, n) => {
            if t >= *n {
                return None;
            }
            if t == 0 {
                run.from = ball.position;
            }
            let body = *run.body.get_or_insert_with(|| nearest_body(sim, *at, true));
            let st = sim.world().state(body)?;
            // the ball held where it was as the step began (an edge may slope)
            (st.pos.distance(*q) > *d).then(|| towards(run.from))
        }
        Step::Ride(at, q, d, n) => {
            if t >= *n {
                return None;
            }
            let body = *run.body.get_or_insert_with(|| nearest_body(sim, *at, true));
            let st = sim.world().state(body)?;
            if st.pos.distance(*q) <= *d {
                return None;
            }
            let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
            let off = flat(st.pos - ball.position);
            let rel = flat(ball.velocity - st.vel);
            Some(arrows(off.clamp_length_max(6.0) - 0.8 * rel, quarter))
        }
        Step::Press(p, side, speed, n) => {
            let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
            if t == 0 {
                run.from = ball.position;
            }
            let line = flat(*p - run.from);
            let along = line.normalize_or_zero();
            let across = Vec3::new(-along.z, 0.0, along.x);
            let rel = flat(ball.position - run.from);
            let v = flat(ball.velocity);
            // (still moving: the end of a pair of rails stops the ball short of the point, and then it is done)
            if v.length() > PRESS_MOVING {
                run.nearest_at = sim.tick;
                run.flown = true;
            }
            // done there, or past it (the point is a rail's end: what comes after is another step's), or stopped
            if t >= *n
                || flat(*p - ball.position).length() < 1.0
                || rel.dot(along) >= line.length()
                || (run.flown && sim.tick > run.nearest_at + PRESS_SETTLE)
            {
                return None;
            }
            // across: to the line (and its offset), damped; along: on while slower than the speed
            let off = rel.dot(across) - side.dot(across);
            let a = -(1.5 * off + 0.8 * v.dot(across));
            let f = if v.dot(along) < *speed { 1.0 } else { -0.3 };
            Some(arrows(along * f + across * a.clamp(-1.0, 1.0), quarter))
        }
        Step::Round(c, r, speed, n) => {
            if t >= *n || ball.position.y > c.y {
                return None;
            }
            let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
            let out = flat(ball.position - *c);
            let u = out.normalize_or_zero();
            let around = Vec3::new(-u.z, 0.0, u.x) * speed.signum();
            let v = flat(ball.velocity);
            // across: to the radius, damped; around: on while slower than the speed
            let a = -(1.5 * (out.length() - *r) + 0.8 * v.dot(u));
            let f = if v.dot(around) < speed.abs() {
                1.0
            } else {
                -0.3
            };
            Some(arrows(around * f + u * a.clamp(-1.0, 1.0), quarter))
        }
        Step::Hold(p, n) => (t < *n).then(|| towards(*p)),
        Step::Sync(p, n) => (t < *n && sim.sector_entering()).then(|| towards(*p)),
        Step::Cross(d, p, r) => {
            if t == 0 {
                run.pushes = cross_plan(sim, *d, *p, *r);
            }
            let push = *run.pushes.get(usize::try_from(t).ok()?)?;
            Some(arrows(push, quarter))
        }
        Step::Pass(p, speed, n) => {
            let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
            let d = flat(*p - ball.position);
            if t >= *n || d.length() < 1.5 {
                run.passed = Some(*p);
                return None;
            }
            // kept to the line from the point passed before (a chain of points is a way along rails and tracks:
            // the ball on it, not cutting across), or straight to the point for the first
            let from = run
                .passed
                .filter(|f| flat(*p - *f).length() > 1.0)
                .unwrap_or(ball.position);
            let line = flat(*p - from);
            let along = line.normalize_or_zero();
            let across = Vec3::new(-along.z, 0.0, along.x);
            let v = flat(ball.velocity);
            let off = flat(ball.position - from).dot(across);
            let a = -(1.5 * off + 0.8 * v.dot(across));
            let f = if v.dot(along) < *speed { 1.0 } else { -0.5 };
            Some(arrows(along * f + across * a.clamp(-1.0, 1.0), quarter))
        }
        Step::Along(speed, n) => {
            if t >= *n {
                return None;
            }
            let v = Vec3::new(ball.velocity.x, 0.0, ball.velocity.z);
            Some(if v.length() < *speed {
                arrows(v, quarter)
            } else {
                0
            })
        }
        Step::Replay(..) | Step::Trace(..) => None,
        Step::Track(track) => {
            if t == 0 && sim.is_held() {
                run.start = run.start.saturating_add(1);
                return Some(0);
            }
            trace(track, run, sim, t)
        }
        Step::Inputs(_, inputs) => {
            // a transformer holding the ball as the step begins holds its recorded run too: it starts as the ball
            // is let go, in step with the player's whatever way the ball came in (later holds are the player's own)
            if t == 0 && sim.is_held() {
                run.start = run.start.saturating_add(1);
                return Some(0);
            }
            let bits = *inputs.get(usize::try_from(t).ok()?)?;
            // the player's view turns are followed in the quarter kept, not made: the arrows are turned to ours
            let pressed = bits & !run.prev;
            run.prev = bits;
            if bits & input::SHIFT != 0 {
                if pressed & input::LEFT != 0 {
                    run.quarter -= 1;
                }
                if pressed & input::RIGHT != 0 {
                    run.quarter += 1;
                }
            }
            Some(arrows(world_push(bits, run.quarter), quarter) | (bits & input::LIFT))
        }
    }
}

/// A trace's input this tick: the recorded push where the ball is along the run, plus the pull back onto it, as the
/// arrows nearest (the part they miss carried to the next tick); lost, toward the run's way ahead.
fn trace(track: &Track, run: &mut Running, sim: &Sim, t: u64) -> Option<u8> {
    let n = track.pos.len();
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
    let ball = &sim.ball;
    if n == 0 || run.k >= n || t > n as u64 + TRACE_GIVE_UP * 2 {
        return None;
    }
    let lost = run.lost_since.is_some();
    let k = match_tick(track, run.k, ball.position, lost);
    // the body that touches the ball next (knocks it on, carries it) comes later than in the run: wait at this tick
    let lag = if lost { 0 } else { meet_late(track, k, sim) };
    let late = lag > MEET_LATE;
    run.k = if late { k } else { k + 1 };
    let (at, v_at) = reference(track, k, sim)?;
    let off = at.distance(ball.position);
    if std::env::var_os("TRACE_LOG").is_some() && t.is_multiple_of(30) {
        let (p, v) = (ball.position, ball.velocity);
        let w = v_at;
        eprintln!(
            "    trace t {t} k {k}{}{} off {off:.2} ball ({:.1} {:.1} {:.1}) run ({:.1} {:.1} {:.1}) v ({:.1} {:.1}) run v ({:.1} {:.1})",
            if track.on.get(k).is_some_and(Option::is_some) { " on" } else { "" },
            if late { " waits".to_string() } else if lag != 0 { format!(" lag {lag}") } else { String::new() },
            p.x, p.y, p.z, at.x, at.y, at.z, v.x, v.z, w.x, w.z
        );
    }
    let (fwd, right) = (sim.camera.push_forward(), sim.camera.push_right());
    let lift = if track.lift.get(k).copied().unwrap_or(false) {
        input::LIFT
    } else {
        0
    };
    if off > TRACE_LOST || (lost && off > TRACE_FOUND) {
        if run.lost_since.is_none() {
            eprintln!("  trace lost at tick {} (run tick {k}, {off:.1} off)", sim.tick);
        }
        let since = *run.lost_since.get_or_insert(sim.tick);
        if sim.tick > since + TRACE_GIVE_UP {
            return None;
        }
        run.residual = Vec3::ZERO;
        let q = *track.pos.get((k + TRACE_AHEAD).min(n - 1))?;
        let want = flat(q - ball.position).clamp_length_max(6.0) / 6.0 - 0.15 * flat(ball.velocity);
        return Some(nearest_arrows(want, fwd, right).0 | lift);
    }
    if run.lost_since.take().is_some() {
        eprintln!("  trace on its run again at tick {} (run tick {k})", sim.tick);
    }
    let ff = if late {
        Vec3::ZERO
    } else {
        track.push.get(k).copied().unwrap_or(Vec3::ZERO)
    };
    let pull = (TRACE_KP * flat(at - ball.position) + TRACE_KD * flat(v_at - ball.velocity))
        .clamp_length_max(1.5)
        * track.gain;
    let (bits, left) = nearest_arrows(ff + pull + run.residual, fwd, right);
    run.residual = left.clamp_length_max(1.0);
    Some(bits | lift)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    #[test]
    fn a_trace_step_parses_with_its_gain() {
        let h = parse("at 0 0 0 r 5 : trace run.replay 10 90 ; trace run.replay 0 5 0.5").unwrap();
        assert_eq!(h[0].steps[0], Step::Trace(PathBuf::from("run.replay"), 10, 90, 1.0));
        assert_eq!(h[0].steps[1], Step::Trace(PathBuf::from("run.replay"), 0, 5, 0.5));
        assert!(parse("at 0 0 0 r 5 : trace run.replay 9 9").is_err());
        assert!(parse("at 0 0 0 r 5 : trace run.replay 0 9 5").is_err());
        assert!(parse("at 0 0 0 r 5 : trace ../run.replay 0 9").is_err());
        let line = "at 0 0 0 r 5 : trace run.replay 1 2";
        assert!(parse(&vec![line; 17].join("\n")).is_err(), "traces count as replays");
    }

    #[test]
    fn a_trace_picks_the_arrows_nearest_its_push() {
        let (f, r) = (Vec3::Z, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(nearest_arrows(Vec3::Z, f, r).0, input::UP);
        assert_eq!(nearest_arrows(Vec3::new(0.9, 0.0, 0.9), f, r).0, input::UP | input::RIGHT);
        assert_eq!(nearest_arrows(Vec3::new(-1.0, 0.0, -0.1), f, r).0, input::LEFT);
        assert_eq!(nearest_arrows(Vec3::new(0.1, 0.0, 0.0), f, r).0, 0);
        // a view turned: forward is world -X
        let (f, r) = (Vec3::NEG_X, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(nearest_arrows(Vec3::NEG_X, f, r).0, input::UP);
        assert_eq!(nearest_arrows(Vec3::Z, f, r).0, input::RIGHT);
        // a half push left over: carried, so two ticks give one arrow press
        let (bits, left) = nearest_arrows(Vec3::new(0.0, 0.0, 0.4), Vec3::Z, Vec3::X);
        assert_eq!(bits, 0);
        assert_eq!(nearest_arrows(Vec3::new(0.0, 0.0, 0.4) + left, Vec3::Z, Vec3::X).0, input::UP);
    }

    #[test]
    fn a_trace_matches_by_progress_but_keeps_a_wait() {
        let mut track = Track::default();
        // ten ticks still at the origin, then one unit a tick along x
        for i in 0..40 {
            track.pos.push(Vec3::new((i.max(10) - 10) as f32, 0.0, 0.0));
        }
        // still in the wait: the tick due, not the wait's end
        assert_eq!(match_tick(&track, 0, Vec3::ZERO, false), 0);
        assert_eq!(match_tick(&track, 5, Vec3::ZERO, false), 5);
        // the ball ahead of the run: matched ahead
        assert_eq!(match_tick(&track, 13, Vec3::new(8.0, 0.0, 0.0), false), 18);
        // never further back than the last matched
        assert_eq!(match_tick(&track, 31, Vec3::ZERO, false), 30);
        // lost: found again further
        assert_eq!(match_tick(&track, 0, Vec3::new(25.0, 0.0, 0.0), true), 35);
    }

    #[test]
    fn a_sync_step_parses_with_its_default_ticks() {
        let h = parse("at 0 0 0 r 5 : sync 1 2 3 ; sync 1 2 3 40").unwrap();
        assert_eq!(h[0].steps[0], Step::Sync(Vec3::new(1.0, 2.0, 3.0), GOTO_TICKS));
        assert_eq!(h[0].steps[1], Step::Sync(Vec3::new(1.0, 2.0, 3.0), 40));
    }

    #[test]
    fn replay_names_are_plain_and_their_steps_few() {
        for good in ["run.replay", "Level_02-owner.replay", "a.b.replay"] {
            assert!(plain_name(good).is_ok(), "{good}");
        }
        for bad in [
            "../x.replay",
            "a/b.replay",
            "x.txt",
            ".replay",
            ".hidden.replay",
            "CON.replay",
            "nul.replay",
            "COM1.replay",
            "aux.x.replay",
            "a b.replay",
            "a:b.replay",
            "a\\b.replay",
        ] {
            assert!(plain_name(bad).is_err(), "{bad}");
        }
        let line = "at 0 0 0 r 5 : replay run.replay 1 2";
        assert!(parse(&vec![line; 16].join("\n")).is_ok());
        assert!(
            parse(&vec![line; 17].join("\n")).is_err(),
            "too many replay steps"
        );
    }

    use super::*;

    #[test]
    fn a_hint_line_reads_with_its_steps() {
        let h = parse(
            "# the pushers\nat 260 -2.7 -790 r 6 sector 3 kind wood : goto 262 -2.7 -815 ; push 0 1 60 ; wait 30\n\
             at 1 2 3 r 1 : replay run.replay 100 160 ; turn -1 ; become paper ; air 1 0 200 # a fan\n",
        )
        .unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].sector, Some(3));
        assert_eq!(h[0].kind, Some(BallKind::Wood));
        assert_eq!(
            h[0].steps[0],
            Step::Goto(Vec3::new(262.0, -2.7, -815.0), GOTO_TICKS)
        );
        assert_eq!(h[0].steps[1], Step::Push(Vec3::Z, 60));
        assert_eq!(
            parse("at 0 0 0 r 1 : via 1 2 3 40").unwrap()[0].steps[0],
            Step::Via(Vec3::new(1.0, 2.0, 3.0), 40)
        );
        assert_eq!(
            h[1].steps[0],
            Step::Replay(PathBuf::from("run.replay"), 100, 160)
        );
        assert_eq!(h[1].steps[2], Step::Become(BallKind::Paper));
        assert_eq!(h[1].line, 3);
        let moving = parse("at 0 0 0 r 1 : lead 1 2 3 -4 300 ; shove 1 2 3 7 8 9 500").unwrap();
        assert_eq!(moving[0].steps[0], Step::Lead(Vec3::new(1.0, 2.0, 3.0), -4.0, 300));
        assert_eq!(moving[0].steps[1], Step::Shove(Vec3::new(1.0, 2.0, 3.0), Vec3::new(7.0, 8.0, 9.0), 500, SHOVE_BEHIND, None));
        let crate_ = parse("at 0 0 0 r 1 : shove 1 2 3 7 8 9 500 5.5 2").unwrap();
        assert_eq!(crate_[0].steps[0], Step::Shove(Vec3::new(1.0, 2.0, 3.0), Vec3::new(7.0, 8.0, 9.0), 500, 5.5, Some(2.0)));
        assert!(parse("at 0 0 0 r 1 : shove 1 2 3 7 8 9 500 99").is_err());
        let swing =
            parse("at 0 0 0 r 1 : await 1 2 3 4 5 6 0.5 200 ; ride 1 2 3 4 5 6 0.5 90 ; press 1 2 3 0 1 6 200").unwrap();
        assert_eq!(
            swing[0].steps[0],
            Step::Await(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0), 0.5, 200)
        );
        assert_eq!(
            swing[0].steps[1],
            Step::Ride(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0), 0.5, 90)
        );
        assert_eq!(
            swing[0].steps[2],
            Step::Press(Vec3::new(1.0, 2.0, 3.0), Vec3::Z, 6.0, 200)
        );
        assert!(
            parse("at 0 0 0 r 1 : press 1 2 3 0 4 6 200").is_err(),
            "too far off the line"
        );
        assert_eq!(
            parse("at 0 0 0 r 1 : along 12 240").unwrap()[0].steps[0],
            Step::Along(12.0, 240)
        );
        assert_eq!(
            parse("at 0 0 0 r 1 : hold 1 2 3 60").unwrap()[0].steps[0],
            Step::Hold(Vec3::new(1.0, 2.0, 3.0), 60)
        );
        assert_eq!(
            parse("at 0 0 0 r 1 : pass 1 2 3 5").unwrap()[0].steps[0],
            Step::Pass(Vec3::new(1.0, 2.0, 3.0), 5.0, GOTO_TICKS)
        );
        assert_eq!(
            parse("at 0 0 0 r 1 : cross 0 -1 1 2 3 4").unwrap()[0].steps[0],
            Step::Cross(-Vec3::Z, Vec3::new(1.0, 2.0, 3.0), 4.0)
        );
        assert_eq!(
            parse("at 0 0 0 r 1 : round 1 2 6 -9 300").unwrap()[0].steps[0],
            Step::Round(Vec3::new(1.0, f32::INFINITY, 2.0), 6.0, -9.0, 300)
        );
        assert_eq!(
            parse("at 0 0 0 r 1 : round 1 2 6 9 300 45").unwrap()[0].steps[0],
            Step::Round(Vec3::new(1.0, 45.0, 2.0), 6.0, 9.0, 300)
        );
        assert!(
            parse("at 0 0 0 r 1 : lead 1 2 3 99 300").is_err(),
            "a lead is 50 units at most"
        );
    }

    #[test]
    fn malformed_hints_are_refused_with_their_line() {
        for bad in [
            "at 1 2 3 : wait 5",
            "at 1 2 r 3 : wait 5",
            "at 1 2 3 r 0 : wait 5",
            "at 1 2 3 r 300 : wait 5",
            "at 1 2 3 r 1 :",
            "at 1 2 3 r 1 : fly 3",
            "at 1 2 3 r 1 : wait 9999999",
            "at 1 2 3 r 1 : wait -1",
            "at nan 2 3 r 1 : wait 1",
            "at 1e9 2 3 r 1 : wait 1",
            "at 1 2 3 r 1 : push 0 0 10",
            "at 1 2 3 r 1 : replay ../x.replay 1 2",
            "at 1 2 3 r 1 : replay a/b 1 2",
            "at 1 2 3 r 1 : replay x 5 5",
            "at 1 2 3 r 1 : turn 9",
            "at 1 2 3 r 1 sector 0 : wait 1",
            "at 1 2 3 r 1 kind iron : wait 1",
            "at 1 2 3 r 1 : wait 1 2",
            "near 1 2 3 r 1 : wait 1",
        ] {
            let e = parse(&format!("\n{bad}")).unwrap_err();
            assert!(e.starts_with("line 2:"), "{bad}: {e}");
        }
        let many = "at 0 0 0 r 1 : wait 1\n".repeat(MAX_HINTS + 1);
        assert!(parse(&many).is_err());
        let long = format!("at 0 0 0 r 1 : {}", "wait 1 ; ".repeat(MAX_STEPS + 1));
        assert!(parse(&long).is_err());
    }

    #[test]
    fn arrows_turn_with_the_view() {
        // a world push read in one view and written in another pushes the same way
        for q in -4..4 {
            for bits in [
                input::UP,
                input::DOWN | input::LEFT,
                input::RIGHT,
                input::UP | input::RIGHT,
            ] {
                let w = world_push(bits, q);
                for mine in -4..4 {
                    assert_eq!(world_push(arrows(w, mine), mine), w, "{bits} {q} {mine}");
                }
            }
        }
    }
}
