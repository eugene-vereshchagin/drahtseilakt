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
/// A gap in a floor narrower than this (between points at one height) is no gap to the ball, which rolls over it.
const GAP: f32 = 5.5;
/// A way down may drop this far (one way only), at this extra cost per unit.
const DROP: f32 = 15.0;
const DROP_COST: f32 = 4.0;
/// Within this of an edge the way costs more, the nearer the more.
const CLEAR: f32 = 6.0;

/// Whether a gap from `p` to `q` (floor points at one height, farther apart than `LINK`) is open: no floor between
/// them stands higher (a thin wall's top), going by the floor points `between` near them.
fn open_gap(p: Vec3, q: Vec3, mut between: impl Iterator<Item = Vec3>) -> bool {
    let d = q - p;
    let len2 = d.x * d.x + d.z * d.z;
    !between.any(|w| {
        let t = (((w.x - p.x) * d.x + (w.z - p.z) * d.z) / len2.max(1e-6)).clamp(0.0, 1.0);
        let c = p + d * t;
        t > 0.0
            && t < 1.0
            && Vec3::new(w.x - c.x, 0.0, w.z - c.z).length() < 1.0
            && w.y > p.y.max(q.y) + STEP
    })
}

/// The way through the level: floor points, in order.
pub struct Route {
    pub points: Vec<Vec3>,
    /// Where each stage (checkpoint, end) is reached along the way.
    pub marks: Vec<usize>,
    /// The floor points by cell (`CELL`), each with how far it is from an edge.
    floor: HashMap<(i32, i32), Vec<(Vec3, f32)>>,
    /// The floor points the way was found over, and their indices by cell (`GAP`), to find ways over them anew.
    pts: Vec<Vec3>,
    grid: HashMap<(i32, i32), Vec<usize>>,
}

/// How far ahead along the way the point is that the ball heads for (points), and how far round over the floors a
/// way to it is looked for.
const LOOK: usize = 20;
const FIELD: f32 = 150.0;
/// How many points ahead the ball is looked for along the way (more would find it on a later pass by the same
/// place, past a loop it should go round).
const AHEAD: usize = 40;
/// How near a goal of the way a floor point reaches it.
const GOAL: f32 = 3.5;

/// The cell of the floor lookup.
const CELL: f32 = 2.0;

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
        // loose balls and boxes are things in the way, a dome is a hump to go round, module 34 a box with a stone
        // on it to push away: not floors
        let Some(name) = crate::scene::module_of(level, i).filter(|n| {
            !["P_Ball", "P_Box", "P_Dome", "P_Modul_34"]
                .iter()
                .any(|k| n.starts_with(k))
        }) else {
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
        Route::search(level, modules, start, stages, false)
    }

    /// The way through `places` close together (where a player's ball went): straight from one to the next where
    /// that line keeps to a floor, found over the floors where it does not (a wall, a step between them).
    pub fn through(
        level: &Level,
        modules: &crate::scene::Modules,
        start: Vec3,
        places: &[Vec3],
    ) -> Route {
        Route::search(level, modules, start, places, true)
    }

    fn search(
        level: &Level,
        modules: &crate::scene::Modules,
        start: Vec3,
        stages: &[Vec3],
        straight_on: bool,
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
            grid.entry(((p.x / GAP).floor() as i32, (p.z / GAP).floor() as i32))
                .or_default()
                .push(i);
        }
        let near = |p: Vec3| -> Vec<usize> {
            let (cx, cz) = ((p.x / GAP).floor() as i32, (p.z / GAP).floor() as i32);
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
        // the line from `a` to `b`, if every step of it is just over a floor point
        let on_floor = |a: Vec3, b: Vec3| -> Option<Vec<Vec3>> {
            let n = (a.distance(b) / SPACING).ceil() as usize;
            (1..=n.max(1))
                .map(|k| a + (b - a) * (k as f32 / n.max(1) as f32))
                .map(|p| {
                    near(p)
                        .into_iter()
                        .filter_map(|v| pts.get(v))
                        .any(|q| {
                            Vec3::new(q.x - p.x, 0.0, q.z - p.z).length() < 1.6
                                && (q.y - p.y).abs() < 1.0
                        })
                        .then_some(p)
                })
                .collect()
        };
        for &goal in stages {
            let goal_node = closest(goal);
            if straight_on
                && let Some(a) = pts.get(from).copied()
                && a.distance(goal) < 12.0
                && let Some(line) = on_floor(a, goal)
            {
                points.extend(line);
                marks.push(points.len().saturating_sub(1));
                from = goal_node;
                continue;
            }
            // any floor point near the goal will do (the first reached): of two rails side by side, the one the way
            // is on
            let at_goal =
                |i: usize| i == goal_node || pts.get(i).is_some_and(|p| p.distance(goal) < GOAL);
            let mut to = goal_node;
            // Dijkstra
            let mut dist: HashMap<usize, f32> = HashMap::new();
            let mut prev: HashMap<usize, usize> = HashMap::new();
            let mut heap = BinaryHeap::new();
            dist.insert(from, 0.0);
            heap.push(Item(0.0, from));
            while let Some(Item(d, u)) = heap.pop() {
                if at_goal(u) {
                    to = u;
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
                    // (a narrow gap at one height is rolled over)
                    let gap = h > LINK
                        && h <= GAP
                        && dy.abs() < 0.3
                        && open_gap(
                            p,
                            q,
                            near(p).into_iter().filter_map(|w| pts.get(w).copied()),
                        );
                    if v == u || (h > LINK && !gap) || dy > STEP || dy < -DROP {
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
            // after a jump straight to the goal, on from the goal
            let jumped = !straight.is_empty();
            points.extend(straight);
            marks.push(points.len().saturating_sub(1));
            from = if jumped { goal_node } else { to };
        }
        let mut floor: HashMap<(i32, i32), Vec<(Vec3, f32)>> = HashMap::new();
        for (p, c) in pts.iter().zip(&clear) {
            floor
                .entry(((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32))
                .or_default()
                .push((*p, *c));
        }
        Route {
            points,
            marks,
            floor,
            pts,
            grid,
        }
    }

    /// The floor points within `GAP` across of `p` (a floor point or a ball's floor).
    fn around(&self, p: Vec3) -> impl Iterator<Item = usize> + '_ {
        let (cx, cz) = ((p.x / GAP).floor() as i32, (p.z / GAP).floor() as i32);
        (-1..=1)
            .flat_map(move |dx| (-1..=1).map(move |dz| (cx + dx, cz + dz)))
            .flat_map(|c| self.grid.get(&c).into_iter().flatten().copied())
    }

    /// The floor point the ball at `p` is on (its nearest at its height, within `GAP` across: the floor points have
    /// holes, under a box's first place, say), if any, and how far across it is.
    fn node(&self, p: Vec3) -> Option<(usize, f32)> {
        let f = p - Vec3::Y * crate::sim::BALL_RADIUS;
        self.around(f)
            .filter_map(|i| self.pts.get(i).map(|q| (i, *q)))
            .filter(|(_, q)| (q.y - f.y).abs() < 1.5)
            .map(|(i, q)| (i, Vec3::new(q.x - f.x, 0.0, q.z - f.z).length()))
            .filter(|&(_, h)| h < GAP)
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// How far a ball has to roll over the floors from each floor point to way point `target` (as far as `limit`):
    /// up a step no higher than `STEP`, down any drop up to `DROP`, over gaps narrower than `GAP`.
    fn cost_to(&self, target: usize, limit: f32) -> HashMap<usize, f32> {
        let mut dist: HashMap<usize, f32> = HashMap::new();
        let Some((goal, _)) = self
            .points
            .get(target)
            .and_then(|&t| self.node(t + Vec3::Y * crate::sim::BALL_RADIUS))
        else {
            return dist;
        };
        let mut heap = BinaryHeap::new();
        dist.insert(goal, 0.0);
        heap.push(Item(0.0, goal));
        while let Some(Item(d, v)) = heap.pop() {
            if dist.get(&v).is_some_and(|&x| d > x) || d > limit {
                continue;
            }
            let Some(&q) = self.pts.get(v) else { continue };
            // the ways into v: from a point u whose way to v the ball can roll
            for u in self.around(q) {
                let Some(&p) = self.pts.get(u) else { continue };
                let h = Vec3::new(q.x - p.x, 0.0, q.z - p.z).length();
                let dy = q.y - p.y;
                let gap = h > LINK
                    && h <= GAP
                    && dy.abs() < 0.3
                    && open_gap(
                        p,
                        q,
                        self.around(q).filter_map(|w| self.pts.get(w).copied()),
                    );
                if u == v || (h > LINK && !gap) || dy > STEP || dy < -DROP {
                    continue;
                }
                let nd = d + h.max(0.1) + if dy < -STEP { -dy } else { 0.0 };
                if dist.get(&u).is_none_or(|&x| nd < x) {
                    dist.insert(u, nd);
                    heap.push(Item(nd, u));
                }
            }
        }
        dist
    }

    /// Every floor point of the level (as the way was found over them).
    pub fn floor_points(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.floor.values().flatten().map(|f| f.0)
    }

    /// The point of the way nearest the ball at `p`, before point `until` (the way may pass there again later).
    pub fn nearest(&self, p: Vec3, until: usize) -> Option<usize> {
        let f = p - Vec3::Y * crate::sim::BALL_RADIUS;
        self.points
            .iter()
            .take(until)
            .enumerate()
            .min_by(|a, b| a.1.distance_squared(f).total_cmp(&b.1.distance_squared(f)))
            .map(|(i, _)| i)
    }

    /// A way given as floor points in order (a player's run): the level's floors are read for the edges.
    pub fn along(level: &Level, modules: &crate::scene::Modules, path: Vec<Vec3>) -> Route {
        let start = path.first().copied().unwrap_or(Vec3::ZERO);
        let mut r = Route::find(level, modules, start, &[]);
        r.marks = vec![path.len().saturating_sub(1)];
        r.points = path;
        r
    }

    /// How far the ball at `p` is from an edge: the clearance of the floor point under it (0 with none under it).
    pub fn clearance(&self, p: Vec3) -> f32 {
        let f = p - Vec3::Y * crate::sim::BALL_RADIUS;
        let (cx, cz) = ((f.x / CELL).floor() as i32, (f.z / CELL).floor() as i32);
        let mut best: Option<(f32, f32)> = None;
        for dx in -1..=1 {
            for dz in -1..=1 {
                for &(q, c) in self.floor.get(&(cx + dx, cz + dz)).into_iter().flatten() {
                    let h = Vec3::new(q.x - f.x, 0.0, q.z - f.z).length();
                    if h < 1.8 && (q.y - f.y).abs() < 1.5 && best.is_none_or(|b| h < b.0) {
                        best = Some((h, c));
                    }
                }
            }
        }
        best.map_or(0.0, |b| b.1)
    }

    /// How far along the way `p` is, searching from point `from` up to `until`: on the segment it is nearest to
    /// (within reach), its index plus the part of it passed, less the square of the distance off it (keep to the way,
    /// do not cut corners along the edge).
    #[cfg(test)]
    fn progress(&self, p: Vec3, from: usize, until: usize) -> f32 {
        self.progress_on(p, from, until).0
    }

    /// `progress`, and whether `p` is on the way (within reach of it) rather than beside it.
    fn progress_on(&self, p: Vec3, from: usize, until: usize) -> (f32, bool) {
        let floor = p - Vec3::Y * crate::sim::BALL_RADIUS;
        let end = (from + AHEAD)
            .min(self.points.len().saturating_sub(1))
            .min(until.max(from + 1));
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
                best = Some((i as f32 + t - 0.4 * off * off, off));
            }
        }
        // off the way (landed beside it after a drop): the point ahead that is best for its nearness
        best.map_or_else(
            || {
                // only a little way ahead: further on the way may pass close by on another track
                let v = (from..(from + 15).min(end).max(from + 1))
                    .filter_map(|i| {
                        self.points
                            .get(i)
                            .map(|q| i as f32 - 0.5 * reach(floor, *q))
                    })
                    .fold(f32::NEG_INFINITY, f32::max)
                    .max(from as f32 - 100.0);
                (v, false)
            },
            |b| (b.0, true),
        )
    }
}

/// How far a floor point `q` is from `floor` for the ball there: a point above it (a ledge, a ramp's top) is
/// further than it looks, as the ball must go round to climb to it.
fn reach(floor: Vec3, q: Vec3) -> f32 {
    let h = Vec3::new(q.x - floor.x, 0.0, q.z - floor.z).length();
    let dy = q.y - floor.y;
    let up = if dy > STEP { 4.0 * dy } else { dy };
    h.hypot(up)
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
/// A player rolls calmly: above this speed every unit costs `FAST` way points.
const CALM: f32 = 7.0;
const FAST: f32 = 1.5;
/// Nearer an edge than `SAFE`, every unit nearer costs `EDGE` way points at each sample (every sixth tick) of a
/// tried choice: a player keeps to the middle.
const SAFE: f32 = 3.0;
const EDGE: f32 = 0.3;
const HOLD: u64 = 18;
const COAST: u64 = 36;

/// A transformer on the way: its place on the way, where it is, and the ball it makes.
pub type Gate = (usize, Vec3, crate::sim::BallKind);

pub struct Pilot {
    pub route: Route,
    at: usize,
    choice: u8,
    /// The tick the way last went on: when it stands still, the choices are held longer (to push a box away).
    moved: u64,
    /// The furthest along the way it has been.
    best: f32,
    /// Where the ball was at the last decision.
    last: Option<Vec3>,
    /// The tick of the last turn of the view.
    turned: u64,
    /// The transformers to roll into, in order: their place on the way, where they are and the ball they make; and
    /// the next one.
    gates: Vec<Gate>,
    gate: usize,
    /// The ball's kind when the last gate opened: a gate opens when the ball has become its kind since (it went
    /// through), or, already of its kind then, when the ball reaches it (rolling through without a stop).
    kind: Option<crate::sim::BallKind>,
    /// A manoeuvre found while stuck: choices held until their ticks.
    plan: Vec<(u64, u8)>,
    seed: u32,
    /// The way point a little ahead that the ball heads for, and how far each floor point near is from it over the
    /// floors.
    field: Option<(usize, HashMap<usize, f32>)>,
}

impl Pilot {
    pub fn new(route: Route, gates: Vec<Gate>) -> Pilot {
        Pilot {
            route,
            gates,
            gate: 0,
            kind: None,
            at: 0,
            choice: 0,
            moved: 0,
            best: f32::NEG_INFINITY,
            last: None,
            turned: 0,
            plan: Vec::new(),
            seed: 0x2545_F491,
            field: None,
        }
    }

    /// How far along the way the ball at `p` is, held before the next transformer until the ball has been in it:
    /// near it, the nearer the better.
    fn progress(&self, p: Vec3) -> f32 {
        // off the way, on a floor near the point it heads for: how far it is from that point over the floors (a
        // ledge above the ball is not near, the way round to the ramp up to it is the way)
        let (v, on_way) = self.route.progress_on(p, self.at, self.until());
        if !on_way
            && let Some((target, field)) = &self.field
            && let Some((node, across)) = self.route.node(p)
        {
            return *target as f32
                - (field.get(&node).copied().unwrap_or(FIELD) + across) / SPACING;
        }
        match self.gates.get(self.gate) {
            Some(&(i, g, _)) if v > i as f32 - 1.0 => {
                (i as f32 - 1.0)
                    - 0.3 * Vec3::new(g.x - p.x, 0.0, g.z - p.z).length()
                    - 0.3 * (g.y - p.y).abs()
            }
            _ => v,
        }
    }

    /// The point the way is searched up to: the next transformer's (the way may pass by later on, after it, in
    /// another kind of ball).
    fn until(&self) -> usize {
        self.gates.get(self.gate).map_or(usize::MAX, |g| g.0)
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
        // First the rams: roll back one way, then hard the other (a wooden ball moves a box only with a run-up) and
        // brake, so as not to follow the box off the edge; then random manoeuvres of three held choices.
        let opposite = |c: u8| {
            let mut o = 0;
            for (a, b) in [(input::UP, input::DOWN), (input::LEFT, input::RIGHT)] {
                if c & a != 0 {
                    o |= b;
                }
                if c & b != 0 {
                    o |= a;
                }
            }
            o
        };
        let mut plans: Vec<Vec<(u64, u8)>> = Vec::new();
        for &c in CHOICES.iter().skip(1) {
            for back in [30u64, 60, 100] {
                for run in [30u64, 60, 120] {
                    for brake in [0u64, 20, 40] {
                        let t = sim.tick + back + run;
                        plans.push(vec![
                            (sim.tick + back, opposite(c)),
                            (t, c),
                            (t + brake, opposite(c)),
                        ]);
                    }
                }
            }
        }
        for _ in 0..40 {
            let mut plan = Vec::new();
            let mut t = sim.tick;
            for _ in 0..3 {
                let c = CHOICES[(self.rand() % 8 + 1) as usize];
                t += 30 + u64::from(self.rand() % 90);
                plan.push((t, c));
            }
            plans.push(plan);
        }
        for plan in plans {
            let t = plan.last().map_or(sim.tick, |p| p.0);
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
            let v = self.progress(s.ball.position) - self.blocking(&s);
            if v > here - self.blocking(sim) + 0.5 && best.as_ref().is_none_or(|b| v > b.0) {
                best = Some((v, plan));
            }
        }
        best.map(|b| b.1)
    }

    /// How much the loose things (boxes, balls) stand in the way ahead: each one on the next stretch of the way
    /// counts as ten points of it, so pushing one off the way is worth a manoeuvre.
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
                n += 10.0;
            }
        }
        n
    }

    /// A quarter turn of the view towards the way ahead, if it is needed now.
    fn turn(&mut self, sim: &Sim) -> Option<u8> {
        // not more often than every second and a half: a player does not swing the view at every bend
        if sim.tick < self.turned + 90 {
            return None;
        }
        let (Some(&a), Some(&b)) = (
            self.route.points.get(self.at),
            self.route.points.get(self.at + 20),
        ) else {
            return None;
        };
        let d = Vec3::new(b.x - a.x, 0.0, b.z - a.z).normalize_or_zero();
        if d == Vec3::ZERO || sim.camera.push_forward().dot(d) > 0.3 {
            return None;
        }
        let right = sim.camera.push_right();
        self.turned = sim.tick;
        Some(
            input::SHIFT
                | if right.dot(d) > 0.0 {
                    input::RIGHT
                } else {
                    input::LEFT
                },
        )
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
        // A player turns the view to face the way ahead (Shift with Left or Right, a quarter a press): when the way
        // ahead points well away from where "up" pushes, one press this tick, released the next.
        if let Some(turn) = self.turn(sim) {
            return turn;
        }
        // a new ball (the old one lost) starts again at a reset point: find it on the way anew
        if self
            .last
            .is_some_and(|l| l.distance(sim.ball.position) > 20.0)
        {
            if let Some(i) = self.route.nearest(sim.ball.position, self.until()) {
                self.at = i;
                self.gate = self.gates.iter().take_while(|g| g.0 <= i).count();
                self.kind = Some(sim.ball.kind);
            }
            self.best = f32::NEG_INFINITY;
            self.plan.clear();
        }
        self.last = Some(sim.ball.position);
        // the next transformer is gone through: the ball has become its kind, or was its kind already and reached it
        let had = *self.kind.get_or_insert(sim.ball.kind);
        if let Some(&(_, g, k)) = self.gates.get(self.gate) {
            let reached = g.distance(sim.ball.position) < crate::sim::TRAFO_REACH;
            if sim.ball.kind == k && (had != k || reached) {
                self.gate += 1;
                self.kind = Some(sim.ball.kind);
            }
        }
        // only on the way does the ball get on along it
        let (on, on_way) = self
            .route
            .progress_on(sim.ball.position, self.at, self.until());
        // (not past a transformer not yet gone through)
        let on = on.min(self.until().saturating_sub(1) as f32);
        if on_way && on > self.at as f32 {
            self.at = on.floor().max(0.0) as usize;
        }
        // the point to head for, a little ahead (not past the next transformer)
        let target = (self.at + LOOK)
            .min(self.until())
            .min(self.route.points.len().saturating_sub(1));
        if self.field.as_ref().is_none_or(|f| f.0 != target) {
            self.field = Some((target, self.route.cost_to(target, FIELD)));
        }
        let here = self.progress(sim.ball.position);
        if std::env::var_os("AUTOPILOT_TRACE").is_some() {
            let node = self.route.node(sim.ball.position);
            eprintln!(
                "t {} at {} gate {} kind {:?} target {} {:?} here {here:.2} node {:?} field {}",
                sim.tick,
                self.at,
                self.gate,
                sim.ball.kind,
                target,
                self.route.points.get(target),
                node.map(|n| (self.route.pts.get(n.0), n.1)),
                self.field.as_ref().map_or(0, |f| f.1.len())
            );
        }
        // getting on: a quarter of a point further than ever
        if here > self.best + 0.25 {
            self.best = here;
            self.moved = sim.tick;
        }
        // stuck for four seconds: try rams and random manoeuvres and keep the best that gets on
        if sim.tick.saturating_sub(self.moved) > 240 {
            // found or not, the next try comes after four seconds more
            self.moved = sim.tick;
            if let Some(plan) = self.manoeuvre(sim, here) {
                self.plan = plan;
                if let Some(&(_, c)) = self.plan.first() {
                    self.choice = c;
                    return c;
                }
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
        // the way ahead climbs (a rail uphill): a run-up is needed, so speed is no fault there
        let uphill = {
            let here_y = self.route.points.get(self.at).map_or(0.0, |p| p.y);
            self.route
                .points
                .iter()
                .skip(self.at)
                .take(25)
                .any(|p| p.y > here_y + 2.0)
        };
        let mut best = (f32::NEG_INFINITY, self.choice);
        for &c in &CHOICES {
            let mut s = sim.clone();
            let lives = s.lives;
            let mut lost = false;
            // stuck for two seconds: hold each choice for up to two seconds
            let stuck = sim.tick.saturating_sub(self.moved) / 120;
            let hold = HOLD * (1 + (stuck % 4) * 2);
            let mut edge = 0.0;
            for t in 0..hold + COAST {
                s.step(if t < hold { c } else { 0 });
                if t % 6 == 5 && s.ball.on_ground {
                    edge += (SAFE - self.route.clearance(s.ball.position)).max(0.0);
                }
                if s.lives < lives || s.is_dying() || s.game_over || s.ball.position.y < low {
                    lost = true;
                    break;
                }
                if s.finished {
                    break;
                }
            }
            // still in the air (a drop off a rail, a jump): it is judged where it lands, up to two seconds on
            let mut flight = 0;
            while !lost && !s.finished && !s.ball.on_ground && flight < 120 {
                s.step(0);
                flight += 1;
                if s.lives < lives || s.is_dying() || s.game_over || s.ball.position.y < low {
                    lost = true;
                }
            }
            // still falling after two seconds, or down on no floor of the level: lost
            if !lost
                && !s.finished
                && (!s.ball.on_ground || self.route.clearance(s.ball.position) <= 0.0)
                && flight > 0
            {
                lost = true;
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
                self.progress(s.ball.position)
                    - self.blocking(&s)
                    - if uphill {
                        0.0
                    } else {
                        FAST * (s.ball.velocity.length() - CALM).max(0.0)
                    }
                    - EDGE * edge
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
            floor: HashMap::new(),
            pts: Vec::new(),
            grid: HashMap::new(),
        };
        let up = Vec3::Y * crate::sim::BALL_RADIUS;
        let a = r.progress(Vec3::new(3.0, 0.0, 0.0) + up, 0, usize::MAX);
        let b = r.progress(Vec3::new(3.5, 0.0, 0.0) + up, 0, usize::MAX);
        let c = r.progress(Vec3::new(9.0, 0.0, 1.0) + up, 0, usize::MAX);
        assert!(a < b && b < c, "{a} {b} {c}");
        assert!((a - 1.5).abs() < 1e-4);
        // far off the way: no point within reach, so behind where the search began
        assert!(r.progress(Vec3::new(9.0, 0.0, 20.0) + up, 2, usize::MAX) < 2.0);
    }
}
