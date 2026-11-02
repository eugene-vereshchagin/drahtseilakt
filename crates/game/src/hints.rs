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
//   replay FILE FROM TO     a player's inputs of ticks FROM..TO of the replay FILE (a plain name beside the hint file),
//                           turned to the test player's view
//
// The file is untrusted (hints may be shared): its size, the number of hints and steps and every number are capped,
// and a malformed line refuses the whole file with its line number.

use crate::sim::{BallKind, Sim, input};
use glam::Vec3;
use std::path::{Path, PathBuf};

const MAX_FILE: u64 = 64 * 1024;
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
    /// Keep this far ahead of (behind, when negative) the loose body nearest the point as it rolls, for so many ticks.
    Lead(Vec3, f32, u64),
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
    let stem = s.strip_suffix(".replay").unwrap_or_default().to_ascii_uppercase();
    let base = stem.split('.').next().unwrap_or_default();
    let device = matches!(base, "CON" | "PRN" | "AUX" | "NUL")
        || ((base.starts_with("COM") || base.starts_with("LPT")) && base.len() == 4);
    let plain = !stem.is_empty()
        && !s.starts_with('.')
        && s.len() <= 128
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
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
        .filter(|s| matches!(s, Step::Replay(..)))
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

/// A view at `quarter`, for the axes its arrows push along.
fn view(quarter: i32) -> crate::sim::Camera {
    crate::sim::Camera {
        quarter,
        yaw: 0.0,
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
}

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
}

impl Runner {
    /// Where a running hint is (its index and step), so that a hint going through its steps counts as getting on.
    pub fn at(&self) -> Option<(usize, usize)> {
        self.running.as_ref().map(|r| (r.hint, r.step))
    }

    /// Whether a running hint is playing a recorded run: a replay step gets on by itself however long it is.
    pub fn replaying(&self) -> bool {
        self.running.as_ref().is_some_and(|r| {
            let step = self.hints.get(r.hint).and_then(|h| h.steps.get(r.step));
            matches!(step, Some(Step::Replay(..) | Step::Inputs(..)))
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
            eprintln!(
                "hint (line {}) at tick {}",
                self.hints.get(i).map_or(0, |h| h.line),
                sim.tick
            );
            self.began = sim.tick;
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
            if let Some(i) = drive(step, run, sim) {
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
            eprintln!(
                "  step {} done at tick {} ball ({:.1} {:.1} {:.1})",
                run.step, sim.tick, p.x, p.y, p.z
            );
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
        if let Some(Step::Inputs(q, _)) =
            self.hints.get(run.hint).and_then(|h| h.steps.get(run.step))
        {
            run.quarter = *q;
        }
    }
}

/// The input of `step` for this tick, or None when it is done.
fn drive(step: &Step, run: &mut Running, sim: &Sim) -> Option<u8> {
    let t = sim.tick.saturating_sub(run.start);
    let quarter = sim.camera.quarter;
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
            Some(if t.is_multiple_of(4) { input::SHIFT | key } else { 0 })
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
        Step::Lead(at, d, n) => {
            if t >= *n {
                return None;
            }
            let world = sim.world();
            let body = *run.body.get_or_insert_with(|| {
                sim.loose()
                    .iter()
                    .filter_map(|&(_, b)| world.state(b).map(|st| (b, st.pos.distance(*at))))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map_or(usize::MAX, |n| n.0)
            });
            let st = world.state(body)?;
            let v = Vec3::new(st.vel.x, 0.0, st.vel.z);
            // ahead along where it rolls (or, still, on the side the ball is), a little ahead of where it will be
            let along = if v.length() > 1.0 {
                v.normalize()
            } else {
                Vec3::new(ball.position.x - st.pos.x, 0.0, ball.position.z - st.pos.z).normalize_or_zero()
                    * d.signum()
            };
            let p = st.pos + along * *d + v * 0.3;
            let off = Vec3::new(p.x - ball.position.x, 0.0, p.z - ball.position.z);
            let rel = Vec3::new(ball.velocity.x, 0.0, ball.velocity.z) - v;
            Some(arrows(off.clamp_length_max(6.0) - 0.8 * rel, quarter))
        }
        Step::Replay(..) => None,
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    #[test]
    fn replay_names_are_plain_and_their_steps_few() {
        for good in ["run.replay", "Level_02-owner.replay", "a.b.replay"] {
            assert!(plain_name(good).is_ok(), "{good}");
        }
        for bad in ["../x.replay", "a/b.replay", "x.txt", ".replay", ".hidden.replay", "CON.replay", "nul.replay",
            "COM1.replay", "aux.x.replay", "a b.replay", "a:b.replay", "a\\b.replay"]
        {
            assert!(plain_name(bad).is_err(), "{bad}");
        }
        let line = "at 0 0 0 r 5 : replay run.replay 1 2";
        assert!(parse(&vec![line; 16].join("\n")).is_ok());
        assert!(parse(&vec![line; 17].join("\n")).is_err(), "too many replay steps");
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
