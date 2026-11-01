// A player for testing, not part of the game: it finds a way over the level's floors from the start through the
// checkpoints to the end, then steers the ball along it by trying each arrow combination on copies of the run and
// keeping the one that gets furthest without losing the ball. Its runs are ordinary replays, so they play back in the
// game like the owner's.

use crate::sim::{Sim, input};
use glam::{Mat4, Vec3};
use level::Level;
use std::collections::{BinaryHeap, HashMap};

/// Floor points are sampled this far apart; neighbours are joined within `LINK` horizontally and `STEP` vertically.
const SPACING: f32 = 1.5;
const LINK: f32 = 3.2;
const STEP: f32 = 1.2;
/// A way down may drop this far (one way only), at this extra cost per unit.
const DROP: f32 = 15.0;
const DROP_COST: f32 = 4.0;
/// Within this of an edge the way costs more, the nearer the more.
const CLEAR: f32 = 6.0;

/// The way through the level: floor points, in order.
pub struct Route {
    pub points: Vec<Vec3>,
    /// Where each stage (checkpoint, end) is reached along the way.
    pub marks: Vec<usize>,
}

/// The world triangles of the level's floors and rails and of the modules at their stand-ins.
/// The world triangles of the floors, rails (flagged) and modules.
fn floor_triangles(level: &Level, modules: &crate::scene::Modules) -> Vec<([Vec3; 3], bool)> {
    let mut out = Vec::new();
    let mut add = |mesh: &level::Mesh, m: Mat4, rail: bool| {
        for part in &mesh.parts {
            for t in part.indices.as_chunks::<3>().0 {
                let p = |i: u32| {
                    mesh.positions
                        .get(i as usize)
                        .map(|&v| m.transform_point3(Vec3::from(v)))
                };
                if let (Some(a), Some(b), Some(c)) = (p(t[0]), p(t[1]), p(t[2])) {
                    out.push(([a, b, c], rail));
                }
            }
        }
    };
    for g in &level.groups {
        let n = g.name.to_ascii_lowercase();
        if n != "phys_floors" && n != "phys_floorrails" {
            continue;
        }
        let rail = n == "phys_floorrails";
        for &i in &g.members {
            let Some(o) = level.objects.get(i) else {
                continue;
            };
            if let Some(mesh) = o.mesh.and_then(|k| level.meshes.get(k)) {
                add(mesh, Mat4::from_cols_array_2d(&o.matrix), rail);
            }
        }
    }
    for i in 0..level.objects.len() {
        // loose balls and boxes are things in the way, not floors
        let Some(name) = crate::scene::module_of(level, i)
            .filter(|n| !n.starts_with("P_Ball") && !n.starts_with("P_Box"))
        else {
            continue;
        };
        let Some((_, module)) = modules.get(name) else {
            continue;
        };
        let Some(o) = level.objects.get(i) else {
            continue;
        };
        let place = Mat4::from_cols_array_2d(&o.matrix);
        for mo in &module.objects {
            if let Some(mesh) = mo.mesh.and_then(|k| module.meshes.get(k)) {
                add(mesh, place * Mat4::from_cols_array_2d(&mo.matrix), false);
            }
        }
    }
    out
}

/// Points over the up-facing triangles, `SPACING` apart, at most one per cell and height band: the lowest, so a
/// floor wins over the kerb along its edge.
fn floor_points(tris: &[([Vec3; 3], bool)]) -> Vec<(Vec3, bool)> {
    let mut cells: HashMap<(i32, i32, i32), (Vec3, bool)> = HashMap::new();
    for &([a, b, c], rail) in tris {
        let n = (b - a).cross(c - a);
        let len = n.length();
        if len < 1e-6 || (n.y / len).abs() < if rail { 0.25 } else { 0.75 } {
            continue;
        }
        let edge = (b - a).length().max((c - a).length()).max((c - b).length());
        let k = ((edge / SPACING).ceil() as usize).clamp(1, 200);
        for i in 0..=k {
            for j in 0..=(k - i) {
                let (u, v) = (i as f32 / k as f32, j as f32 / k as f32);
                let p = a + (b - a) * u + (c - a) * v;
                let key = (
                    (p.x / SPACING).floor() as i32,
                    (p.z / SPACING).floor() as i32,
                    (p.y / 3.0).floor() as i32,
                );
                let e = cells.entry(key).or_insert((p, rail));
                if p.y < e.0.y {
                    *e = (p, rail);
                }
            }
        }
    }
    let mut v: Vec<(Vec3, bool)> = cells.into_values().collect();
    v.sort_by(|(a, _), (b, _)| {
        a.x.total_cmp(&b.x)
            .then(a.z.total_cmp(&b.z))
            .then(a.y.total_cmp(&b.y))
    });
    v
}

#[derive(PartialEq)]
struct Item(f32, usize);
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        o.0.total_cmp(&self.0)
    }
}

impl Route {
    /// The way from `start` through `stages` (checkpoints, then the end) over the level's floors.
    pub fn find(
        level: &Level,
        modules: &crate::scene::Modules,
        start: Vec3,
        stages: &[Vec3],
    ) -> Route {
        let tris = floor_triangles(level, modules);
        let found = floor_points(&tris);
        let pts: Vec<Vec3> = found.iter().map(|f| f.0).collect();
        if std::env::var_os("AUTOPILOT_DEBUG").is_some() {
            eprintln!(
                "autopilot: {} triangles, {} floor points, start {start:?}",
                tris.len(),
                pts.len()
            );
        }
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, p) in pts.iter().enumerate() {
            grid.entry(((p.x / LINK).floor() as i32, (p.z / LINK).floor() as i32))
                .or_default()
                .push(i);
        }
        let near = |p: Vec3| -> Vec<usize> {
            let (cx, cz) = ((p.x / LINK).floor() as i32, (p.z / LINK).floor() as i32);
            let mut v = Vec::new();
            for dx in -1..=1 {
                for dz in -1..=1 {
                    if let Some(c) = grid.get(&(cx + dx, cz + dz)) {
                        v.extend(c);
                    }
                }
            }
            v
        };
        // A floor point without floor on every side (a kerb's top, the edge) is no way to go, unless on a rail.
        let edge: Vec<bool> = found
            .iter()
            .map(|&(p, rail)| {
                if rail {
                    return false;
                }
                let mut sides = [false; 4];
                for v in near(p) {
                    let Some(&q) = pts.get(v) else { continue };
                    let d = q - p;
                    if d.y.abs() > 0.3 || Vec3::new(d.x, 0.0, d.z).length() > 2.6 {
                        continue;
                    }
                    if d.x > 0.7 {
                        sides[0] = true;
                    }
                    if d.x < -0.7 {
                        sides[1] = true;
                    }
                    if d.z > 0.7 {
                        sides[2] = true;
                    }
                    if d.z < -0.7 {
                        sides[3] = true;
                    }
                }
                !sides.iter().all(|&x| x)
            })
            .collect();
        // How far each point is from the nearest edge (up to `CLEAR`): the way keeps to the middle, as a player who
        // fears the edge does. Rails are narrow by nature and count as clear.
        let mut edges: HashMap<(i32, i32), Vec<Vec3>> = HashMap::new();
        for (p, _) in found.iter().zip(&edge).filter(|(_, e)| **e) {
            edges
                .entry((
                    (p.0.x / CLEAR).floor() as i32,
                    (p.0.z / CLEAR).floor() as i32,
                ))
                .or_default()
                .push(p.0);
        }
        let clear: Vec<f32> = found
            .iter()
            .map(|&(p, rail)| {
                if rail {
                    return CLEAR;
                }
                let (cx, cz) = ((p.x / CLEAR).floor() as i32, (p.z / CLEAR).floor() as i32);
                let mut c = CLEAR;
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        for q in edges.get(&(cx + dx, cz + dz)).into_iter().flatten() {
                            if (q.y - p.y).abs() < 1.5 {
                                c = c.min(Vec3::new(q.x - p.x, 0.0, q.z - p.z).length());
                            }
                        }
                    }
                }
                c
            })
            .collect();
        let closest = |p: Vec3| -> usize {
            pts.iter()
                .enumerate()
                .min_by(|a, b| a.1.distance_squared(p).total_cmp(&b.1.distance_squared(p)))
                .map_or(0, |(i, _)| i)
        };
        let mut points = Vec::new();
        let mut marks = Vec::new();
        let mut from = closest(start - Vec3::Y * crate::sim::BALL_RADIUS);
        for &goal in stages {
            let goal_node = closest(goal);
            let to = goal_node;
            // Dijkstra
            let mut dist: HashMap<usize, f32> = HashMap::new();
            let mut prev: HashMap<usize, usize> = HashMap::new();
            let mut heap = BinaryHeap::new();
            dist.insert(from, 0.0);
            heap.push(Item(0.0, from));
            while let Some(Item(d, u)) = heap.pop() {
                if u == to {
                    break;
                }
                if dist.get(&u).is_some_and(|&x| d > x) {
                    continue;
                }
                let Some(&p) = pts.get(u) else { continue };
                for v in near(p) {
                    let Some(&q) = pts.get(v) else { continue };
                    let h = Vec3::new(q.x - p.x, 0.0, q.z - p.z).length();
                    let dy = q.y - p.y;
                    if v == u || h > LINK || dy > STEP || dy < -DROP {
                        continue;
                    }
                    let edgy = if edge.get(v).copied().unwrap_or(true) {
                        40.0
                    } else {
                        0.0
                    };
                    let fear = (CLEAR - clear.get(v).copied().unwrap_or(0.0)) / CLEAR;
                    let cost = h.max(0.1) * (1.0 + 6.0 * fear * fear)
                        + edgy
                        + if dy < -STEP { -dy * DROP_COST } else { 0.0 };
                    let nd = d + cost;
                    if dist.get(&v).is_none_or(|&x| nd < x) {
                        dist.insert(v, nd);
                        prev.insert(v, u);
                        heap.push(Item(nd, v));
                    }
                }
            }
            if std::env::var_os("AUTOPILOT_DEBUG").is_some() {
                eprintln!(
                    "autopilot: stage {goal:?}: from {:?} to {:?}, reached {} ({} points; nearest {:?})",
                    pts.get(from),
                    pts.get(to),
                    dist.contains_key(&to),
                    dist.len(),
                    dist.keys()
                        .filter_map(|&k| pts.get(k))
                        .min_by(|a, b| a.distance(goal).total_cmp(&b.distance(goal)))
                );
            }
            if std::env::var_os("AUTOPILOT_DEBUG").is_some() && !dist.contains_key(&to) {
                let mut far: Vec<(f32, Vec3)> = dist
                    .iter()
                    .filter_map(|(&k, &d)| pts.get(k).map(|p| (d, *p)))
                    .collect();
                far.sort_by(|a, b| b.0.total_cmp(&a.0));
                for (d, p) in far.iter().take(12) {
                    eprintln!("  far {d:7.1}: {:7.2} {:7.2} {:7.2}", p.x, p.y, p.z);
                }
                if let Some(n) = dist
                    .keys()
                    .filter_map(|&k| pts.get(k))
                    .min_by(|a, b| a.distance(goal).total_cmp(&b.distance(goal)))
                {
                    for (i, p) in pts.iter().enumerate() {
                        if p.distance(*n) < 8.0 {
                            eprintln!(
                                "  {:7.2} {:7.2} {:7.2} {}",
                                p.x,
                                p.y,
                                p.z,
                                dist.contains_key(&i)
                            );
                        }
                    }
                }
            }
            // Not reachable over the floors (a jump, a moving part): to the reached point nearest the goal, then
            // straight on to it.
            let (to, straight) = if dist.contains_key(&to) {
                (to, Vec::new())
            } else {
                let near_goal = dist
                    .keys()
                    .copied()
                    .min_by(|&a, &b| {
                        let d = |k: usize| pts.get(k).map_or(f32::INFINITY, |p| p.distance(goal));
                        d(a).total_cmp(&d(b))
                    })
                    .unwrap_or(from);
                let a = pts.get(near_goal).copied().unwrap_or(goal);
                let n = ((goal.distance(a) / SPACING).ceil() as usize).min(2000);
                let line: Vec<Vec3> = (1..=n)
                    .map(|k| a + (goal - a) * (k as f32 / n as f32))
                    .collect();
                (near_goal, line)
            };
            let mut way = vec![to];
            let mut at = to;
            while let Some(&p) = prev.get(&at) {
                way.push(p);
                at = p;
                if p == from {
                    break;
                }
            }
            way.reverse();
            points.extend(way.iter().filter_map(|&i| pts.get(i).copied()));
            points.extend(straight);
            marks.push(points.len().saturating_sub(1));
            from = goal_node;
        }
        Route { points, marks }
    }

    /// How far along the way `p` is, searching from point `from` on: on the segment it is nearest to (within reach),
    /// its index plus the part of it passed, less a little for the distance off it.
    fn progress(&self, p: Vec3, from: usize) -> f32 {
        let floor = p - Vec3::Y * crate::sim::BALL_RADIUS;
        let end = (from + 120).min(self.points.len().saturating_sub(1));
        let mut best: Option<(f32, f32)> = None;
        for i in from..end {
            let (Some(&a), Some(&b)) = (self.points.get(i), self.points.get(i + 1)) else {
                break;
            };
            let ab = b - a;
            let t = if ab.length_squared() > 1e-6 {
                ((floor - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let off = floor.distance(a + ab * t);
            // the nearest segment (the later one of equals) says where along the way it is
            if off < 3.5 && best.is_none_or(|b| off <= b.1 + 1e-4) {
                best = Some((i as f32 + t - 0.5 * off, off));
            }
        }
        best.map_or_else(
            || from as f32 - floor.distance(self.points.get(from).copied().unwrap_or(floor)),
            |b| b.0,
        )
    }
}

/// The arrow combinations the player may hold (the camera is not turned, so the pushes stay on the world axes).
const CHOICES: [u8; 9] = [
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

/// The steering: every `PERIOD` ticks each choice is tried for `HOLD` ticks and then `COAST` with no arrows on a copy of
/// the run; the one that ends furthest along the way without losing the ball is held.
const PERIOD: u64 = 6;
const HOLD: u64 = 18;
const COAST: u64 = 36;

pub struct Pilot {
    pub route: Route,
    at: usize,
    choice: u8,
    /// The tick the way last went on: when it stands still, the choices are held longer (to push a box away).
    moved: u64,
    /// A manoeuvre found while stuck: choices held until their ticks.
    plan: Vec<(u64, u8)>,
    seed: u32,
}

impl Pilot {
    pub fn new(route: Route) -> Pilot {
        Pilot {
            route,
            at: 0,
            choice: 0,
            moved: 0,
            plan: Vec::new(),
            seed: 0x2545_F491,
        }
    }

    /// How far along the way the ball has come (route points).
    pub fn reached(&self) -> usize {
        self.at
    }

    fn rand(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed
    }

    /// Random manoeuvres of two held choices (half a second to two seconds each, then a second with no arrows):
    /// the one ending furthest along the way without losing the ball, if it gets on from `here`.
    fn manoeuvre(&mut self, sim: &Sim, here: f32) -> Option<Vec<(u64, u8)>> {
        let mut best: Option<(f32, Vec<(u64, u8)>)> = None;
        for _ in 0..60 {
            let mut plan = Vec::new();
            let mut t = sim.tick;
            for _ in 0..3 {
                let c = CHOICES[(self.rand() % 8 + 1) as usize];
                t += 30 + u64::from(self.rand() % 90);
                plan.push((t, c));
            }
            let mut s = sim.clone();
            let lives = s.lives;
            let mut lost = false;
            while s.tick < t + 60 {
                let c = plan.iter().find(|p| s.tick < p.0).map_or(0, |p| p.1);
                s.step(c);
                if s.lives < lives || s.is_dying() || s.game_over {
                    lost = true;
                    break;
                }
            }
            if lost {
                continue;
            }
            let v = self.route.progress(s.ball.position, self.at) - self.blocking(&s);
            if v > here - self.blocking(sim) + 0.5 && best.as_ref().is_none_or(|b| v > b.0) {
                best = Some((v, plan));
            }
        }
        best.map(|b| b.1)
    }

    /// How much the loose things (boxes, balls) stand in the way ahead: each one on the next stretch of the way
    /// counts as five points of it, so pushing one off the way is worth a manoeuvre.
    fn blocking(&self, sim: &Sim) -> f32 {
        let ahead: Vec<Vec3> = self
            .route
            .points
            .iter()
            .skip(self.at)
            .take(40)
            .copied()
            .collect();
        let world = sim.world();
        let mut n = 0.0;
        for &(_, body) in sim.loose() {
            let Some(st) = world.state(body) else {
                continue;
            };
            let p = st.pos;
            if ahead.iter().any(|q| {
                Vec3::new(p.x - q.x, 0.0, p.z - q.z).length() < 2.5 && (p.y - q.y).abs() < 4.0
            }) {
                n += 5.0;
            }
        }
        n
    }

    /// The input for the next tick of `sim`.
    pub fn next(&mut self, sim: &Sim) -> u8 {
        if let Some(&(until, c)) = self.plan.first() {
            if sim.tick < until {
                return c;
            }
            self.plan.remove(0);
            if let Some(&(_, c)) = self.plan.first() {
                self.choice = c;
                return c;
            }
        }
        if !sim.tick.is_multiple_of(PERIOD) {
            return self.choice;
        }
        let here = self.route.progress(sim.ball.position, self.at);
        if here > self.at as f32 {
            self.at = here.floor().max(0.0) as usize;
            self.moved = sim.tick;
        }
        // stuck for four seconds: try random two-part manoeuvres and keep the best that gets on
        if sim.tick.saturating_sub(self.moved) > 240
            && let Some(plan) = self.manoeuvre(sim, here)
        {
            self.plan = plan;
            self.moved = sim.tick;
            if let Some(&(_, c)) = self.plan.first() {
                self.choice = c;
                return c;
            }
        }
        // below every floor of the way ahead: falling
        let low = self
            .route
            .points
            .iter()
            .skip(self.at.saturating_sub(10))
            .take(140)
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min)
            - 6.0;
        let mut best = (f32::NEG_INFINITY, self.choice);
        for &c in &CHOICES {
            let mut s = sim.clone();
            let lives = s.lives;
            let mut lost = false;
            // stuck for two seconds: hold each choice for up to two seconds
            let stuck = sim.tick.saturating_sub(self.moved) / 120;
            let hold = HOLD * (1 + (stuck % 4) * 2);
            for t in 0..hold + COAST {
                s.step(if t < hold { c } else { 0 });
                if s.lives < lives || s.is_dying() || s.game_over || s.ball.position.y < low {
                    lost = true;
                    break;
                }
                if s.finished {
                    break;
                }
            }
            // in the air and well below where it was: falling off, unless the way itself goes down there
            let way_down = self
                .route
                .points
                .get(self.at + 1..self.at + 30)
                .is_some_and(|w| {
                    w.iter()
                        .any(|p| p.y < sim.ball.position.y - crate::sim::BALL_RADIUS - 3.0)
                });
            if !s.ball.on_ground && s.ball.position.y < sim.ball.position.y - 6.0 && !way_down {
                lost = true;
            }
            let score = if lost {
                f32::NEG_INFINITY
            } else if s.finished {
                f32::INFINITY
            } else {
                self.route.progress(s.ball.position, self.at)
                    + if c == self.choice { 0.05 } else { 0.0 }
            };
            if std::env::var_os("AUTOPILOT_SCORES").is_some() {
                eprintln!(
                    "t{} at {} choice {c:02x}: {score:.2} lost {lost} -> {:?}",
                    sim.tick, self.at, s.ball.position
                );
            }
            if score > best.0 {
                best = (score, c);
            }
        }
        self.choice = best.1;
        self.choice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_grows_along_the_way_and_ignores_what_is_off_it() {
        let points: Vec<Vec3> = (0..10)
            .map(|i| Vec3::new(i as f32 * 2.0, 0.0, 0.0))
            .collect();
        let r = Route {
            points,
            marks: vec![9],
        };
        let up = Vec3::Y * crate::sim::BALL_RADIUS;
        let a = r.progress(Vec3::new(3.0, 0.0, 0.0) + up, 0);
        let b = r.progress(Vec3::new(3.5, 0.0, 0.0) + up, 0);
        let c = r.progress(Vec3::new(9.0, 0.0, 1.0) + up, 0);
        assert!(a < b && b < c, "{a} {b} {c}");
        assert!((a - 1.5).abs() < 1e-4);
        // far off the way: no point within reach, so behind where the search began
        assert!(r.progress(Vec3::new(9.0, 0.0, 20.0) + up, 2) < 2.0);
    }
}
