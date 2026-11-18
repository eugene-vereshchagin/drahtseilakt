// A player for testing, not part of the game: it finds a way over the level's floors from the start through the
// checkpoints to the end, then steers the ball along it by trying each arrow combination on copies of the run and
// keeping the one that gets furthest without losing the ball. Its runs are ordinary replays, so they play back in the
// game like the owner's.
// Reason for the allow: a planner over the validated level model and sim state; reads only numbers it computed itself (env knobs are parsed with get/parse().ok()).
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use crate::sim::{Sim, input};
use glam::{Mat4, Vec3};
use level::Level;
use std::collections::{BinaryHeap, HashMap};

/// Floor points are sampled this far apart; neighbours are joined within `LINK` horizontally and `STEP` vertically.
const SPACING: f32 = 1.5;
const LINK: f32 = 3.2;
const STEP: f32 = 1.2;
/// Up a smooth slope (a wavy log's wave, a ramp) a link may climb this much of its length: a ball with way on rolls up it,
/// where a kerb as high, close by, stops it.
const SLOPE_CLIMB: f32 = 1.1;
/// A floor's triangle rises at most this steeply (the up part of its normal): a wave of a wavy log is rolled over.
const FLOOR_NORMAL: f32 = 0.68;
/// The same for the floors along a player's way (and the clearance and the field the steering reads there).
const FLOOR_NORMAL_FOLLOWED: f32 = 0.75;

/// How much a link `h` long may climb: a step, or a slope.
fn climb(h: f32) -> f32 {
    STEP.max(SLOPE_CLIMB * h)
}
/// A gap in a floor narrower than this (between points at one height) is no gap to the ball, which rolls over it.
const GAP: f32 = 5.5;
/// A way down may drop this far (one way only), at this extra cost per unit.
const DROP: f32 = 30.0;
const DROP_COST: f32 = 4.0;
/// Where no way over the floors reaches a goal, a leap over a gap this long at most, at this price.
const LEAP: f32 = 20.0;
const LEAP_COST: f32 = 200.0;
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

/// The direction a strip of rail points runs (offsets across the ground from one of them): the main axis of their
/// spread, if they spread at all.
fn rail_axis(offsets: &[Vec3]) -> Option<Vec3> {
    let n = offsets.len() as f32;
    if offsets.len() < 2 {
        return None;
    }
    let m = offsets.iter().fold(Vec3::ZERO, |a, r| a + *r) / n;
    let (mut xx, mut xz, mut zz) = (0.0, 0.0, 0.0);
    for r in offsets {
        let c = *r - m;
        xx += c.x * c.x;
        xz += c.x * c.z;
        zz += c.z * c.z;
    }
    if xx + zz < 1e-3 {
        return None;
    }
    // the angle of the covariance's main axis
    let a = 0.5 * (2.0 * xz).atan2(xx - zz);
    Some(Vec3::new(a.cos(), 0.0, a.sin()))
}

/// The way through the level: floor points, in order.
/// Way points between two passes of one place for the later pass to be another visit (a loop), not the same pass.
const LOOP_GAP: f32 = 6.0;

pub struct Route {
    pub points: Vec<Vec3>,
    /// Where each stage (checkpoint, end) is reached along the way.
    pub marks: Vec<usize>,
    /// Where the way rides a fan's updraft (the point it is lifted from), and which fan.
    pub lifts: Vec<(usize, usize)>,
    /// The floor points by cell (`CELL`), each with how far it is from an edge.
    floor: HashMap<(i32, i32), Vec<FloorPoint>>,
    /// The floor points the way was found over, and their indices by cell (`GAP`), to find ways over them anew.
    pts: Vec<Vec3>,
    grid: HashMap<(i32, i32), Vec<usize>>,
    /// The edges of the level's floors.
    pub edges: std::sync::Arc<crate::edges::Edges>,
}

/// A floor point as the steering reads it: how far it is from the nearest floor point without floor on every side
/// (0 for those: no footing), whether it is on a rail, and half its floor's width (`half` in `Route::search`).
struct FloorPoint {
    at: Vec3,
    coarse: f32,
    rail: bool,
    half: f32,
}

/// The middle of a floor is looked for this far round a point of it.
const MIDDLE_NEAR: f32 = 6.0;

/// How far ahead along the way the point is that the ball heads for (points), and how far round over the floors a
/// way to it is looked for.
const LOOK: usize = 20;
const FIELD: f32 = 150.0;
/// How many points ahead the ball is looked for along the way (more would find it on a later pass by the same
/// place, past a loop it should go round).
const AHEAD: usize = 40;
/// How near a goal of the way a floor point reaches it.
const GOAL: f32 = 3.5;

/// What a floor point on a transformer the way keeps off costs more (way points' worth of units).
const SHUN_COST: f32 = 60.0;

/// The cell of the floor lookup.
const CELL: f32 = 2.0;

/// The edges of the level's floors (edges.rs), built once a level.
pub fn level_edges(
    level: &Level,
    modules: &crate::scene::Modules,
) -> std::sync::Arc<crate::edges::Edges> {
    let (tris, own) = floor_triangles(level, modules);
    crate::edges::Edges::of(&tris, own)
}

/// The world triangles of the floors, rails (with the rail's object) and modules; and how many are the level's own
/// (they come first, the modules' after).
fn floor_triangles(
    level: &Level,
    modules: &crate::scene::Modules,
) -> (Vec<([Vec3; 3], Rail)>, usize) {
    let mut out = Vec::new();
    let add = |out: &mut Vec<([Vec3; 3], Rail)>, mesh: &level::Mesh, m: Mat4, rail: Rail| {
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
        let rails = n == "phys_floorrails";
        for &i in &g.members {
            let rail = rails.then_some(i);
            let Some(o) = level.objects.get(i) else {
                continue;
            };
            if let Some(mesh) = o.mesh.and_then(|k| level.meshes.get(k)) {
                add(&mut out, mesh, Mat4::from_cols_array_2d(&o.matrix), rail);
            }
        }
    }
    let own = out.len();
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
                add(
                    &mut out,
                    mesh,
                    place * Mat4::from_cols_array_2d(&mo.matrix),
                    None,
                );
            }
        }
    }
    (out, own)
}

/// A fan's updraft (Sim::updrafts): where it is measured to, the top of its volume, how far across it takes the ball.
pub type Fan = (Vec3, f32, f32);
/// A paper ball lifted by a fan is pushed off the top of its updraft onto a floor this far across from its column at
/// most, and the lift costs this much more than its length (a player rides one only where it leads somewhere).
const FAN_LAND: f32 = 14.0;
const FAN_COST: f32 = 20.0;

/// The rail object a floor point is on, if it is on a rail.
type Rail = Option<usize>;

/// Points over the up-facing triangles, `SPACING` apart, at most one per cell and height band: the lowest, so a
/// floor wins over the kerb along its edge.
fn floor_points(tris: &[([Vec3; 3], Rail)], normal_min: f32) -> Vec<(Vec3, Rail)> {
    let mut cells: HashMap<(i32, i32, i32), (Vec3, Rail)> = HashMap::new();
    for &([a, b, c], rail) in tris {
        let n = (b - a).cross(c - a);
        let len = n.length();
        if len < 1e-6 || (n.y / len).abs() < if rail.is_some() { 0.25 } else { normal_min } {
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
    let mut v: Vec<(Vec3, Rail)> = cells.into_values().collect();
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
        fans: &[Fan],
        start: Vec3,
        stages: &[Vec3],
    ) -> Route {
        Route::search(level, modules, fans, start, stages, false, &[], &[], false)
    }

    /// The way as `find`, kept off the transformers at `shun` (those a player does not mean to take: rolling over
    /// one makes the ball over), where a way round them is not much longer; the stretches to the goals at `grounded`
    /// (their indices in `stages`) without riding a fan.
    #[allow(clippy::too_many_arguments)]
    pub fn find_shunning(
        level: &Level,
        modules: &crate::scene::Modules,
        fans: &[Fan],
        start: Vec3,
        stages: &[Vec3],
        shun: &[Vec3],
        grounded: &[usize],
    ) -> Route {
        Route::search(
            level, modules, fans, start, stages, false, shun, grounded, true,
        )
    }

    /// The way through `places` close together (where a player's ball went): straight from one to the next where
    /// that line keeps to a floor, found over the floors where it does not (a wall, a step between them).
    pub fn through(
        level: &Level,
        modules: &crate::scene::Modules,
        fans: &[Fan],
        start: Vec3,
        places: &[Vec3],
    ) -> Route {
        Route::search(level, modules, fans, start, places, true, &[], &[], false)
    }

    #[allow(clippy::too_many_arguments)]
    fn search(
        level: &Level,
        modules: &crate::scene::Modules,
        fans: &[Fan],
        start: Vec3,
        stages: &[Vec3],
        straight_on: bool,
        shun: &[Vec3],
        grounded: &[usize],
        steep: bool,
    ) -> Route {
        let (tris, own) = floor_triangles(level, modules);
        let floor_edges = crate::edges::Edges::of(&tris, own);
        // (a way found for the test player alone, not along a player's, takes steeper floors: the waves of a wavy log)
        let found = floor_points(
            &tris,
            if steep {
                FLOOR_NORMAL
            } else {
                FLOOR_NORMAL_FOLLOWED
            },
        );
        let pts: Vec<Vec3> = found.iter().map(|f| f.0).collect();
        // the floor points on a transformer to keep off, each costing `SHUN_COST` more
        let shunned: Vec<bool> = pts
            .iter()
            .map(|p| {
                shun.iter().any(|t| {
                    Vec3::new(p.x - t.x, 0.0, p.z - t.z).length() < crate::sim::TRAFO_REACH
                        && (p.y - t.y).abs() < 3.0
                })
            })
            .collect();
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
        // the floor points within reach of a leap
        let near_wide = |p: Vec3| -> Vec<usize> {
            let (cx, cz) = ((p.x / GAP).floor() as i32, (p.z / GAP).floor() as i32);
            let r = (LEAP / GAP).ceil() as i32;
            let mut v = Vec::new();
            for dx in -r..=r {
                for dz in -r..=r {
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
                if rail.is_some() {
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
        // How far each point is from the nearest of the floor points without floor on every side (up to `CLEAR`):
        // 0 for those, so a ball coming down there is taken as coming down on no floor. Rails count as clear.
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
        let coarse: Vec<f32> = found
            .iter()
            .map(|&(p, rail)| {
                if rail.is_some() {
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
        // How far each point is from the edge of its floor, a drop or a wall's foot (edges.rs, up to `CLEAR`): the
        // way keeps to the middle, as a player who fears the edge does; on a narrow floor its middle is the way. A
        // rail is narrow by nature and holds the ball: its line is the way, counted as clear.
        let clear: Vec<f32> = found
            .iter()
            .map(|&(p, rail)| {
                if rail.is_some() {
                    CLEAR
                } else {
                    floor_edges.distance(p).min(CLEAR)
                }
            })
            .collect();
        // How far from its edges the middle of each point's floor is (the clearest point near, as high): half the
        // floor's width, up to `CLEAR`. The steering keeps the ball near it.
        let half: Vec<f32> = pts
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                near(p)
                    .into_iter()
                    .filter(|&w| {
                        pts.get(w).is_some_and(|q| {
                            (q.y - p.y).abs() < 1.0
                                && Vec3::new(q.x - p.x, 0.0, q.z - p.z).length() < MIDDLE_NEAR
                        })
                    })
                    .filter_map(|w| clear.get(w).copied())
                    .fold(clear.get(i).copied().unwrap_or(0.0), f32::max)
            })
            .collect();
        // Along which way each rail point's rail runs: the main axis of its own rail's points near.
        let axis: Vec<Option<Vec3>> = found
            .iter()
            .map(|&(p, rail)| {
                let r = rail?;
                let offsets: Vec<Vec3> = near(p)
                    .into_iter()
                    .filter_map(|w| found.get(w))
                    .filter(|f| f.1 == Some(r) && (f.0.y - p.y).abs() < 3.0)
                    .map(|f| Vec3::new(f.0.x - p.x, 0.0, f.0.z - p.z))
                    .filter(|o| o.length() < 2.0 * LINK)
                    .collect();
                rail_axis(&offsets)
            })
            .collect();
        if let Ok(v) = std::env::var("AUTOPILOT_NEAR") {
            let c: Vec<f32> = v.split(',').filter_map(|x| x.parse().ok()).collect();
            if let [x, y, z] = c[..] {
                for (i, f) in found.iter().enumerate() {
                    if f.0.distance(Vec3::new(x, y, z)) < 4.0 {
                        eprintln!(
                            "near {:?} rail {:?} axis {:?} edge {:?}",
                            f.0,
                            f.1,
                            axis.get(i),
                            edge.get(i)
                        );
                    }
                }
            }
        }
        // The fans: the floor points under each one's updraft (where a ball is lifted from) and those near its top
        // (where it can be pushed off onto).
        let mut lifts: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut lands: Vec<Vec<usize>> = Vec::new();
        for (f, &(at, top, across)) in fans.iter().enumerate() {
            let flat = |q: Vec3| Vec3::new(q.x - at.x, 0.0, q.z - at.z).length();
            let mut onto = Vec::new();
            for (i, &q) in pts.iter().enumerate() {
                let h = flat(q);
                if h < across && q.y > at.y - 4.0 && q.y < top - 4.0 {
                    lifts.entry(i).or_default().push(f);
                } else if h < across + FAN_LAND && q.y < top - 0.5 && q.y > at.y - 4.0 {
                    onto.push(i);
                }
            }
            lands.push(onto);
        }
        // A ball on rails stays on them (they hold it from the sides): it goes on or off a rail only along it (not
        // across a crossing rail, not sideways off it), and drops off only where it ends.
        let rail_ok = |u: usize, v: usize, dy: f32| -> bool {
            let (Some(&p), Some(&q)) = (pts.get(u), pts.get(v)) else {
                return false;
            };
            let dir = Vec3::new(q.x - p.x, 0.0, q.z - p.z).normalize_or_zero();
            let across = |w: usize| {
                axis.get(w)
                    .copied()
                    .flatten()
                    .is_some_and(|a| a.dot(dir).abs() < 0.7)
            };
            if across(u) || across(v) {
                return false;
            }
            let Some(r) = found.get(u).and_then(|f| f.1).filter(|_| dy < -STEP) else {
                return true;
            };
            !near(p).into_iter().filter_map(|w| found.get(w)).any(|f| {
                let o = Vec3::new(f.0.x - p.x, 0.0, f.0.z - p.z);
                f.1 == Some(r)
                    && (f.0.y - p.y).abs() < 1.0
                    && o.length() < LINK
                    && o.dot(dir) > 0.5 * o.length()
            })
        };
        let closest = |p: Vec3| -> usize {
            pts.iter()
                .enumerate()
                .min_by(|a, b| a.1.distance_squared(p).total_cmp(&b.1.distance_squared(p)))
                .map_or(0, |(i, _)| i)
        };
        let mut points = Vec::new();
        let mut marks = Vec::new();
        let mut rides = Vec::new();
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
        for (stage, &goal) in stages.iter().enumerate() {
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
            let mut dist: HashMap<usize, f32> = HashMap::new();
            let mut prev: HashMap<usize, usize> = HashMap::new();
            // the points reached by a fan's lift, and which fan
            let mut lifted: HashMap<usize, usize> = HashMap::new();
            // Dijkstra; where the goal is not reached over the floors, again with leaps over gaps (a drawbridge to
            // knock over, a jump, a moving part to wait for) at a high price, so the way crosses the narrowest one
            for leap in [false, true] {
                dist.clear();
                prev.clear();
                lifted.clear();
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
                        // (not onto or off a rail: a rail is got onto only where it begins)
                        let gap = h > LINK
                            && h <= GAP
                            && dy.abs() < 0.3
                            && found.get(u).is_some_and(|f| f.1.is_none())
                            && found.get(v).is_some_and(|f| f.1.is_none())
                            && open_gap(
                                p,
                                q,
                                near(p).into_iter().filter_map(|w| pts.get(w).copied()),
                            );
                        if v == u
                            || (h > LINK && !gap)
                            || dy > if steep { climb(h) } else { STEP }
                            || dy < -DROP
                        {
                            continue;
                        }
                        // up more than a step only on a slope: a floor point half way up between them (a kerb as high
                        // has none)
                        let mid = (p + q) * 0.5;
                        if dy > STEP
                            && !near(mid).into_iter().filter_map(|w| pts.get(w)).any(|m| {
                                Vec3::new(m.x - mid.x, 0.0, m.z - mid.z).length() < 1.2
                                    && m.y > p.y + 0.25 * dy
                                    && m.y < q.y - 0.25 * dy
                            })
                        {
                            continue;
                        }
                        if !rail_ok(u, v, dy) {
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
                            + if shunned.get(v).copied().unwrap_or(false) {
                                SHUN_COST
                            } else {
                                0.0
                            }
                            + if dy < -STEP { -dy * DROP_COST } else { 0.0 };
                        let nd = d + cost;
                        if dist.get(&v).is_none_or(|&x| nd < x) {
                            dist.insert(v, nd);
                            prev.insert(v, u);
                            lifted.remove(&v);
                            heap.push(Item(nd, v));
                        }
                    }
                    if leap {
                        for v in near_wide(p) {
                            let Some(&q) = pts.get(v) else { continue };
                            let h = Vec3::new(q.x - p.x, 0.0, q.z - p.z).length();
                            let dy = q.y - p.y;
                            if h <= GAP
                                || h > LEAP
                                || dy > STEP
                                || dy < -DROP
                                || edge.get(v).copied().unwrap_or(true)
                                || !rail_ok(u, v, dy)
                            {
                                continue;
                            }
                            let nd = d
                                + LEAP_COST
                                + 4.0 * h
                                + if dy < 0.0 { -dy * DROP_COST } else { 0.0 };
                            if dist.get(&v).is_none_or(|&x| nd < x) {
                                dist.insert(v, nd);
                                prev.insert(v, u);
                                lifted.remove(&v);
                                heap.push(Item(nd, v));
                            }
                        }
                    }
                    // over a fan: up its updraft and across onto a floor near its top
                    // (not on a stretch found over the floors only)
                    let lifts_here = if grounded.contains(&stage) {
                        None
                    } else {
                        lifts.get(&u)
                    };
                    for &f in lifts_here.into_iter().flatten() {
                        let Some(&(at, top, _)) = fans.get(f) else {
                            continue;
                        };
                        for &v in lands.get(f).into_iter().flatten() {
                            let Some(&q) = pts.get(v) else { continue };
                            let across = Vec3::new(q.x - at.x, 0.0, q.z - at.z).length();
                            let nd = d + (top - p.y).max(0.0) + across + FAN_COST;
                            if dist.get(&v).is_none_or(|&x| nd < x) {
                                dist.insert(v, nd);
                                prev.insert(v, u);
                                lifted.insert(v, f);
                                heap.push(Item(nd, v));
                            }
                        }
                    }
                }
                if dist.contains_key(&to) && at_goal(to) {
                    break;
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
            for (k, &i) in way.iter().enumerate() {
                let Some(&q) = pts.get(i) else { continue };
                // a fan's lift: straight up its column to the top, then across to where it lands
                if let (Some(&f), Some(&p)) =
                    (lifted.get(&i), k.checked_sub(1).and_then(|j| way.get(j)))
                {
                    rides.push((points.len().saturating_sub(1), f));
                    if let (Some(&(at, top, _)), Some(&a)) = (fans.get(f), pts.get(p)) {
                        let up = Vec3::new(at.x, top - 1.0, at.z);
                        for (s, e) in [(a, up), (up, q)] {
                            let n = ((s.distance(e) / SPACING).ceil() as usize).clamp(1, 200);
                            points.extend((1..n).map(|j| s + (e - s) * (j as f32 / n as f32)));
                        }
                    }
                }
                points.push(q);
            }
            // after a jump straight to the goal, on from the goal
            let jumped = !straight.is_empty();
            points.extend(straight);
            marks.push(points.len().saturating_sub(1));
            from = if jumped { goal_node } else { to };
        }
        let mut floor: HashMap<(i32, i32), Vec<FloorPoint>> = HashMap::new();
        for (i, p) in pts.iter().enumerate() {
            floor
                .entry(((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32))
                .or_default()
                .push(FloorPoint {
                    at: *p,
                    coarse: coarse.get(i).copied().unwrap_or(0.0),
                    rail: found.get(i).is_some_and(|f| f.1.is_some()),
                    half: half.get(i).copied().unwrap_or(0.0),
                });
        }
        Route {
            points,
            marks,
            lifts: rides,
            floor,
            pts,
            grid,
            edges: floor_edges,
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
        self.floor.values().flatten().map(|f| f.at)
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
    pub fn along(
        level: &Level,
        modules: &crate::scene::Modules,
        fans: &[Fan],
        path: Vec<Vec3>,
        steep: bool,
    ) -> Route {
        let start = path.first().copied().unwrap_or(Vec3::ZERO);
        let mut r = Route::search(level, modules, fans, start, &[], false, &[], &[], steep);
        r.marks = vec![path.len().saturating_sub(1)];
        r.points = path;
        r
    }

    /// The floor point under the ball at `p` (its nearest within 1.8 across, as high within 1.5), if any.
    fn floor_point(&self, p: Vec3) -> Option<&FloorPoint> {
        let f = p - Vec3::Y * crate::sim::BALL_RADIUS;
        let (cx, cz) = ((f.x / CELL).floor() as i32, (f.z / CELL).floor() as i32);
        let mut best: Option<(f32, &FloorPoint)> = None;
        for dx in -1..=1 {
            for dz in -1..=1 {
                for q in self.floor.get(&(cx + dx, cz + dz)).into_iter().flatten() {
                    let h = Vec3::new(q.at.x - f.x, 0.0, q.at.z - f.z).length();
                    if h < 1.8 && (q.at.y - f.y).abs() < 1.5 && best.is_none_or(|b| h < b.0) {
                        best = Some((h, q));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// How far the ball at `p` is from an edge, the coarse way: the clearance of the floor point under it to the
    /// floor points without floor on every side (0 with none under it, or on one of those: no footing).
    pub fn clearance(&self, p: Vec3) -> f32 {
        self.floor_point(p).map_or(0.0, |q| q.coarse)
    }

    /// The speed a steady player keeps at `p`: moderate on a rail or a narrow track, more on a wide floor (a
    /// speedrunner's pace in that style: `Pace`).
    pub fn cruise(&self, p: Vec3, style: Style) -> f32 {
        let pace = style.pace();
        match self.floor_point(p) {
            Some(q) if q.rail => pace.rail,
            Some(q) => (pace.base + pace.per_half * q.half).clamp(pace.rail, pace.wide),
            None => pace.calm,
        }
    }

    /// How far the ball at `p` is off the middle of its floor, and how much nearer than `SAFE` to its edge (edges.rs):
    /// none on a rail (its line is the way) or on no floor.
    pub fn off_middle(&self, p: Vec3) -> Option<(f32, f32)> {
        let q = self.floor_point(p).filter(|q| !q.rail)?;
        let d = self.edges.distance(p - Vec3::Y * crate::sim::BALL_RADIUS);
        Some(((q.half - d).max(0.0), (SAFE - d).max(0.0)))
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
        // the way may pass one place twice, out and back (the player went to push something there): the first
        // pass near the ball is where it is, so a loop is driven in its order, not cut short
        let mut best: Option<(f32, f32)> = None;
        let mut first: Option<(f32, f32)> = None;
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
            // a pass the ball is on, before the way leaves this place and comes back
            if first.is_none() && off < 1.5 {
                first = Some((i as f32 + t - 0.4 * off * off, off));
            }
            // (the nearest pass is kept while the segments go on from it; a later one only after the way left)
            if let (Some(f), Some(b)) = (first, best)
                && b.0 > f.0 + LOOP_GAP
            {
                best = Some(f);
                break;
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

/// How the test player rides where it finds the way by itself (`--style`): the calm style keeps a steady
/// player's pace on the middle of the tracks; the speedrun style goes as fast as the physics lets it, along edges
/// and round corners tight (no careful zone or learned rule holds it back), and does not stand where a player
/// stood still in the run it follows (`idle_skip`). A failure is gone back from as in the calm style, so the risk it takes costs time on the tries, not in the run kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    #[default]
    Calm,
    Speedrun,
    /// The raw speedrun: as the first test player rode, with nothing of the calm style left. No cost for speed (the
    /// physics is the only limit), for slowness, for an edge, off the middle or for switching arrows: a choice is
    /// worth how far along the way it gets. No careful, flow, calm or plain zone and no learned rule holds it back
    /// (AUTOPILOT_RAW_CALM_ZONES=1 keeps the calm zones), the followed player's still stretches are skipped and
    /// where the player hugged an edge, so may it. A hint is a puzzle's solution written by hand: its stops and
    /// speeds go as in the speedrun (as written, flowed in the level's flow zones); AUTOPILOT_RAW_FLOW_HINTS=1 flows
    /// every stop where a copy of the run keeps the ball and takes a flowed step's speed as a minimum (the copy sees
    /// the ball, not the puzzle: a pusher's plate pressed too hard, a ball to keep ahead). It goes back only from a lost ball, a stray transformer, a hint's
    /// failed check or a way that has not got on for 20 seconds (a dead end), as every style does.
    Raw,
}

/// A style's pace and line: the cruising speeds (`Route::cruise`), what speed above the pace costs, and what being
/// off the middle and near an edge costs (`rollouts`).
#[derive(Clone, Copy, Debug)]
pub struct Pace {
    calm: f32,
    rail: f32,
    wide: f32,
    base: f32,
    per_half: f32,
    fast: f32,
    slow: f32,
    safe: f32,
    edge: f32,
    middle: f32,
    switch: f32,
}

impl Style {
    pub fn parse(s: &str) -> Option<Style> {
        match s {
            "calm" => Some(Style::Calm),
            "speedrun" => Some(Style::Speedrun),
            "raw" => Some(Style::Raw),
            _ => None,
        }
    }

    pub fn pace(self) -> Pace {
        match self {
            Style::Calm => Pace {
                calm: CALM,
                rail: CRUISE_RAIL,
                wide: CRUISE_WIDE,
                base: CRUISE_BASE,
                per_half: CRUISE_PER_HALF,
                fast: FAST,
                slow: SLOW,
                safe: SAFE,
                edge: EDGE,
                middle: MIDDLE,
                switch: SWITCH,
            },
            // (a wide floor asks for about all the ball has; a rail for what it stays on through a bend; an edge
            // is only shunned within a ball's width of it, and the middle of a track is no line of its own)
            Style::Speedrun => Pace {
                calm: SPEEDRUN_CALM * speedrun_scale(),
                rail: SPEEDRUN_RAIL * speedrun_scale(),
                wide: SPEEDRUN_WIDE * speedrun_scale(),
                base: SPEEDRUN_BASE * speedrun_scale(),
                per_half: SPEEDRUN_PER_HALF * speedrun_scale(),
                fast: SPEEDRUN_FAST,
                slow: SPEEDRUN_SLOW,
                safe: SPEEDRUN_SAFE,
                edge: SPEEDRUN_EDGE,
                middle: 0.0,
                switch: SPEEDRUN_SWITCH,
            },
            // (the cruising speeds only weigh costs, and every cost is 0: progress alone decides)
            Style::Raw => Pace {
                calm: SPEEDRUN_CALM,
                rail: SPEEDRUN_RAIL,
                wide: SPEEDRUN_WIDE,
                base: SPEEDRUN_BASE,
                per_half: SPEEDRUN_PER_HALF,
                fast: 0.0,
                slow: 0.0,
                safe: 0.0,
                edge: 0.0,
                middle: 0.0,
                switch: 0.0,
            },
        }
    }
}

/// AUTOPILOT_SPEEDRUN_PACE scales the speedrun's pace (0.3 to 2; 1 when not set): another try at a level whose
/// run at the full pace fails or is slower (the risk differs, and so does what fails).
fn speedrun_scale() -> f32 {
    static SCALE: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *SCALE.get_or_init(|| {
        std::env::var("AUTOPILOT_SPEEDRUN_PACE")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .map_or(1.0, |v| v.clamp(0.3, 2.0))
    })
}

/// The raw speedrun rides the calm style past a place after this many tries there (`Pilot::fall_back`), to this
/// many way points past it.
const RAW_FALLBACK: u32 = 6;
const RAW_FALLBACK_PAST: usize = 20;

const SPEEDRUN_CALM: f32 = 16.0;
const SPEEDRUN_RAIL: f32 = 11.0;
const SPEEDRUN_WIDE: f32 = 22.0;
const SPEEDRUN_BASE: f32 = 9.0;
const SPEEDRUN_PER_HALF: f32 = 2.5;
const SPEEDRUN_FAST: f32 = 1.0;
const SPEEDRUN_SLOW: f32 = 0.15;
const SPEEDRUN_SAFE: f32 = 0.8;
const SPEEDRUN_EDGE: f32 = 2.0;
const SPEEDRUN_SWITCH: f32 = 0.05;

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

/// Not an arrow combination: the choice of the player's own pushes (Demo).
const DEMO_CHOICE: u8 = 0xff;
/// What the player's line is worth over a choice of ours, in way points: so much that it is always taken unless it
/// loses the ball. The player knew what the way's search does not (a lever to hit first, a transformer to go back
/// to, a place to wait); a short cut of ours that looks further leads off the player's line for good.
const DEMO_BONUS: f32 = 1000.0;

/// The steering: every `PERIOD` ticks each choice is tried for `HOLD` ticks and then `COAST` with no arrows on a copy of
/// the run; the one that ends furthest along the way without losing the ball is held.
const PERIOD: u64 = 6;
/// A player rolls calmly: above this speed every unit costs `FAST` way points (off the floors; on them the floor's
/// cruising speed, `Route::cruise`: moderate on a rail or a narrow track, more on a wide one).
const CALM: f32 = 7.0;
const FAST: f32 = 1.5;
const CRUISE_RAIL: f32 = 6.0;
const CRUISE_WIDE: f32 = 10.0;
const CRUISE_BASE: f32 = 4.5;
const CRUISE_PER_HALF: f32 = 1.2;
/// And a steady player does not stop where nothing calls for it: below half the cruising speed each unit costs
/// `SLOW` way points (less than a choice that keeps the ball is worth: a wait the level needs is still taken).
const SLOW: f32 = 0.4;
/// A player keeps to the middle of a track: over the samples (every sixth tick) of a tried choice on a floor, each
/// unit the ball is off the middle of its floor costs `MIDDLE` way points on average, and each unit nearer an edge
/// than `SAFE` `EDGE` more. Less than a choice that keeps the ball or gets on is worth, more than a little progress.
const SAFE: f32 = 3.0;
const EDGE: f32 = 2.7;
const MIDDLE: f32 = 0.6;
/// The watcher's zones (zones.rs). A flow zone: below `FLOW_PACE` of the floor's pace each unit costs `FLOW_SLOW`
/// way points (more than standing for a decision is worth, less than keeping the ball). A careful zone: above its
/// limit each unit costs `CAREFUL_FAST` more, off the way's line ahead (the middle the way's search found) each
/// unit `CAREFUL_LINE` on average, off the middle of the floor `CAREFUL_MIDDLE` times as much; the input is kept
/// under the limit (`under_cap`), braking above `CAREFUL_BRAKE` times it.
const FLOW_PACE: f32 = 0.8;
const FLOW_SLOW: f32 = 1.5;
const CAREFUL_FAST: f32 = 4.0;
const CAREFUL_LINE: f32 = 2.0;
const CAREFUL_MIDDLE: f32 = 3.0;
const CAREFUL_BRAKE: f32 = 1.25;
/// And a player holds a line: each arrow pressed or let go since the last decision costs this (no zig-zag).
const SWITCH: f32 = 0.15;
const ARROWS: u8 = input::UP | input::DOWN | input::LEFT | input::RIGHT;
const HOLD: u64 = 18;
const COAST: u64 = 36;

/// A transformer on the way: its place on the way, where it is, and the ball it makes.
pub type Gate = (usize, Vec3, crate::sim::BallKind);

/// One tick of a player's run the test player follows: the ball (where, how fast, which, in which sector, not being
/// lost) and what the player pushed then, in the world (so it reads the same in any view).
#[derive(Clone, Copy, Debug)]
pub struct DemoTick {
    pub pos: Vec3,
    pub vel: Vec3,
    pub kind: crate::sim::BallKind,
    pub sector: usize,
    pub alive: bool,
    pub push: Vec3,
    /// The keys as the player pressed them, and the view's quarter then: in the same view they are given as they
    /// were (turns of the view too, and pushes while the view turns, which no world direction gives exactly).
    pub bits: u8,
    pub quarter: crate::hints::Facing,
    /// In the player's last seconds before losing a ball (not `alive`): how many ticks before the loss. The line is
    /// kept to there but for the last second: what the player did then may be what the level needs (a bridge
    /// knocked down as the ball fell with it).
    pub doomed_in: Option<u32>,
}

/// A player's run, tick by tick. Where the test player's ball is where the player's was, as fast and the same
/// ball, it may do what the player did there: each decision tries the player's pushes as one more choice (and,
/// stuck, longer stretches of them), judged on a copy of the run like every other choice. One mechanism for every
/// place a person passed that the way's search does not: a narrow rail taken at speed, a swing across a trough, a
/// puzzle's pushes.
#[derive(Clone, Debug, Default)]
pub struct Demo {
    pub ticks: Vec<DemoTick>,
}

/// A speedrun does not stand where the player stood: a stretch of the followed run this long (ticks) or longer,
/// the player's ball slower than `IDLE_SPEED` and within `IDLE_NEAR` of where it began, is skipped when the
/// player's keys from its end, on a copy of the run, keep the ball on the player's line (within `TRACK_LOST`) for
/// `IDLE_CHECK` ticks: a wait a moving part needs (a lift, a swing, a pusher) fails that, and is kept.
const IDLE_MIN: usize = 30;
const IDLE_SPEED: f32 = 0.8;
const IDLE_NEAR: f32 = 1.5;
const IDLE_CHECK: u64 = 300;

/// The player's tick to go on from instead of `k`, past the still stretch beginning there (`IDLE_MIN`).
fn idle_skip(d: &Demo, k: usize, sim: &Sim) -> Option<usize> {
    let first = d.ticks.get(k)?;
    if sim.ball.position.distance(first.pos) > DEMO_NEAR
        || sim.ball.velocity.length() > 2.0 * IDLE_SPEED
    {
        return None;
    }
    let still = d
        .ticks
        .iter()
        .skip(k)
        .take_while(|t| {
            t.alive
                && t.sector == first.sector
                && t.kind == first.kind
                && t.vel.length() < IDLE_SPEED
                && t.pos.distance(first.pos) < IDLE_NEAR
        })
        .count();
    if still < IDLE_MIN {
        return None;
    }
    let end = k + still;
    let mut s = sim.clone();
    let lives = s.lives;
    for t in 0..IDLE_CHECK {
        let j = end + t as usize;
        let tick = d.ticks.get(j)?;
        s.step(d.input(j, crate::hints::Facing::of(&s.camera)));
        if s.lives < lives || s.is_dying() || s.game_over {
            return None;
        }
        if s.finished {
            return Some(end);
        }
        if !tick.alive || tick.sector != s.sector || tick.kind != s.ball.kind {
            // (the player's own ball was lost or changed further on: judged up to there)
            break;
        }
        if t % 10 == 9 && s.ball.position.distance(tick.pos) > TRACK_LOST {
            return None;
        }
    }
    Some(end)
}

/// A leap is tried with these run-ups (units back from the edge).
const LEAP_RUN_UPS: [f32; 4] = [6.0, 10.0, 15.0, 20.0];

/// Stuck this near a drawbridge the way goes over, the ball charges it, at most this often a bridge.
const BRIDGE_NEAR: f32 = 16.0;
const BRIDGE_CHARGES: usize = 3;

/// Stuck this near the next transformer, the ball is taken into it.
const GATE_STUCK: f32 = 10.0;
/// Held up this long (ticks) right by a gap the way crosses or the next transformer, the test player takes it at
/// once rather than after the two seconds it waits elsewhere, and this near the gap's edge counts as by it.
const HELD_UP: u64 = 45;
/// Stuck this long (ticks), rams, manoeuvres and the deeper search are tried, and again after as long (following a
/// player's run, longer: the player's line may wait there).
const STUCK_TRY: u64 = 120;
const STUCK_TRY_FOLLOWING: u64 = 240;
const GAP_CLOSE: f32 = 10.0;
/// A sill: the way off a rail onto a floor, or up a step of more than `SILL_STEP` (less than the ball's radius),
/// within `SILL_AHEAD` points; taken from a run-up of `SILL_BACK` ticks back, on to `SILL_PAST` points past it.
const SILL_STEP: f32 = 0.3;
const SILL_AHEAD: usize = 8;
const SILL_BACK: u64 = 25;
const SILL_PAST: usize = 4;
const SILL_NEAR: f32 = 3.0;

/// Keeping to the player's line (`Pilot::track`): how far ahead the courses are compared, how far our ball may be
/// from the player's before the line is lost, how far on it is looked for again, and what the player's own keys are
/// worth (in summed units) over an arrow choice.
const TRACK_AHEAD: u64 = 30;
const TRACK_LOST: f32 = 4.0;
const TRACK_RESYNC: usize = 3600;
const TRACK_OWN: f32 = 0.5;
/// Where the player's ball hugged an edge (nearer than `SAFE`), ours may keep up to `TRACK_SLACK` off the player's
/// line, each unit of it costing `TRACK_SLACK_COST` instead of a whole one, and each unit off the middle of the floor
/// `TRACK_MIDDLE`: so it keeps nearer the middle where that line is as fast, the player's line elsewhere.
const TRACK_SLACK: f32 = 1.5;
const TRACK_SLACK_COST: f32 = 0.25;
const TRACK_MIDDLE: f32 = 0.5;
/// The player's line is kept to until this long (a second) before a lost ball.
const TRACK_DOOMED: u32 = 60;
/// A ball lost this long (four seconds) after keeping to the player's last seconds before a loss was lost as the
/// player's was.
const AS_THE_PLAYER: u64 = 240;

/// A player's tick matches the ball within this distance and this difference of speed.
const DEMO_NEAR: f32 = 2.0;
const DEMO_SPEED: f32 = 4.0;
/// Stuck, the player's pushes are tried for these stretches from the places within `DEMO_STUCK_NEAR`.
const DEMO_STRETCHES: [u64; 3] = [240, 480, 900];
const DEMO_STUCK_NEAR: f32 = 4.0;
/// After a hint the player's line waits this long (ten seconds).
const DEMO_AFTER_HINT: u64 = 600;
/// Stuck within `RUN_UP_NEAR` of the player's line, it looks back up to `RUN_UP_BACK` of the player's ticks for a
/// place the player stood still within `RUN_UP_REACH`, and gives the player's keys from there for `RUN_UP_KEYS`.
const RUN_UP_NEAR: f32 = 30.0;
const RUN_UP_BACK: usize = 1200;
const RUN_UP_REACH: f32 = 30.0;
const RUN_UP_KEYS: usize = 1500;

impl Demo {
    /// The player's tick nearest the ball (same ball, same sector, not being lost), within `near` and, when `speed` is
    /// some, as fast within it; searched near `hint` first (the player's run goes on as ours does).
    fn find(&self, sim: &Sim, near: f32, speed: Option<f32>, hint: Option<usize>) -> Option<usize> {
        let b = &sim.ball;
        let fits = |t: &DemoTick| {
            t.alive
                && t.kind == b.kind
                && t.sector == sim.sector
                && t.pos.distance(b.position) < near
                && speed.is_none_or(|s| t.vel.distance(b.velocity) < s)
        };
        let best_in = |from: usize, to: usize| {
            self.ticks
                .get(from..to.min(self.ticks.len()))?
                .iter()
                .enumerate()
                .filter(|(_, t)| fits(t))
                .min_by(|x, y| {
                    x.1.pos
                        .distance(b.position)
                        .total_cmp(&y.1.pos.distance(b.position))
                })
                .map(|(i, _)| from + i)
        };
        // the player's run going on from where it was followed comes first (where the player stood still, every tick
        // of it is as near), then the nearest around it, then anywhere
        let near = hint
            .and_then(|h| best_in(h.saturating_sub(120), h + 600))
            .or_else(|| best_in(0, self.ticks.len()))?;
        let dist = |k: usize| {
            self.ticks
                .get(k)
                .map_or(f32::INFINITY, |t| t.pos.distance(b.position))
        };
        Some(
            match hint.filter(|&h| self.ticks.get(h).is_some_and(fits)) {
                Some(h) if dist(h) <= dist(near) + 0.5 => h,
                _ => near,
            },
        )
    }

    /// The player's input of tick `k` for a view at `quarter`: the very keys when the view is the player's, else the
    /// push as arrows of this view.
    fn input(&self, k: usize, quarter: crate::hints::Facing) -> u8 {
        self.ticks.get(k).map_or(0, |t| {
            if t.quarter == quarter {
                t.bits
            } else {
                crate::hints::arrows(t.push, quarter)
            }
        })
    }
}

/// A variation of the steering after a rewind (pilot_run goes back to a saved state when the ball is lost or the
/// way stands still): for a while each choice's score gets a seeded random addition and, on some tries, the player's
/// line is not taken, so the run from the saved state takes another course than the one that failed. The
/// simulation is deterministic, so a try is reproducible from its seed.
#[derive(Clone, Debug)]
struct Noise {
    /// The opening move: held until this tick, before anything else (a hint too: a hint's course is varied by the
    /// moment and the place it goes on from).
    opening: (u64, u8),
    seed: u32,
    until: u64,
    amp: f32,
    no_demo: bool,
}

#[derive(Clone)]
pub struct Pilot {
    pub route: std::sync::Arc<Route>,
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
    /// The hints for places it cannot pass alone (hints.rs): while one runs, it drives.
    pub hints: crate::hints::Runner,
    /// The player's run it follows, if any; the player's tick it matched last; and, when the player's pushes were
    /// the choice, from which of the player's ticks and from which tick of ours.
    pub demo: Option<std::sync::Arc<Demo>>,
    demo_at: Option<(usize, u64)>,
    demo_run: Option<(usize, u64)>,
    /// The player's still places already rolled back to.
    run_ups: Vec<usize>,
    /// The tick the player's line is followed again from, after a hint.
    demo_after: u64,
    /// The drawbridges (P_Modul_25: a plank standing up on a hinge, knocked over to lie across a gap), and how often
    /// each has been charged.
    pub bridges: Vec<Vec3>,
    charged: Vec<usize>,
    /// After a rewind: the variation of its choices (`perturb`).
    noise: Option<Noise>,
    /// Keeping to the followed player's ball tick by tick (`track`): the player's tick matched and our tick then, and
    /// the inputs chosen for the ticks up to the next decision.
    tracking: Option<(usize, u64)>,
    track_keys: Vec<(u64, u8)>,
    /// The last tick the player's line was kept to in the player's last seconds before a lost ball.
    doomed_at: Option<u64>,
    /// What steers now, for whoever watches (`status`).
    mode: &'static str,
    /// The watcher's zones (zones.rs): where not to dawdle, where to keep slow and on the middle.
    pub zones: std::sync::Arc<crate::zones::Zones>,
    /// What they ask where the ball is now.
    zone: crate::zones::Effect,
    /// What it learned from the watcher's marks on every level (learned.rs), and the rule holding now.
    pub learned: std::sync::Arc<crate::learned::Learned>,
    learned_now: Option<String>,
    /// What the zones and rules asked when AUTOPILOT_SITUATION last said it (it says each change).
    zone_said: String,
    /// Why a careful limit is let go here (the ball needs its momentum), if it is.
    momentum: Option<&'static str>,
    /// The level's modules (their stand-ins' places and module names), for the situation of a place.
    pub modules: std::sync::Arc<Vec<(Vec3, String)>>,
    /// How it rides where it finds the way by itself (`Style`), and how it rides here (a speedrun rides the calm
    /// style in a calm zone).
    pub style: Style,
    ride: Style,
    in_calm: bool,
    /// The raw speedrun after failing at one place again and again (`RAW_FALLBACK` tries): the calm style until the
    /// way is past it (this sector, up to this way point).
    calm_until: Option<(usize, usize)>,
}

/// The situation of a place (learned.rs): the way ahead read this far (points) for its bend, its slope and what
/// comes, a module this near counts, and the thresholds of the values.
const SITUATION_AHEAD: usize = 8;
const SITUATION_MODULE: f32 = 8.0;
const NARROW: f32 = 4.0;
const MEDIUM: f32 = 10.0;
const BEND: f32 = 20.0;
const SHARP: f32 = 60.0;
const SLOPE: f32 = 0.15;
const DROP_STEP: f32 = 2.0;
/// A sill rises this steeply at least (a ramp's start does not), a drop falls more than it goes across.
const SILL_STEEP: f32 = 0.6;

/// Where the physics needs momentum a careful zone keeps only its line (`Pilot::momentum`): the way is read this far
/// ahead (points, about 30 units: a run-up from the limit).
const MOMENTUM_AHEAD: usize = 20;
/// How each ball climbs by its own push (measured on the engine, `a_ball_climbs_by_its_push_as_the_table_says`):
/// the steepest slope it climbs at all (degrees), and how much speed it keeps per degree under that (its pace up a
/// slope, pushed on, is about `(steepest - slope) * per_degree`). Stone 17° (at 4.5: 12.6°), wood 36.7° (26°),
/// paper 45.9° (31°), our sticky ball 43° (22.5°; steeper it does not roll back: it clings, creeping up).
const CLIMB: [(crate::sim::BallKind, f32, f32); 4] = [
    (crate::sim::BallKind::Stone, 17.0, 1.05),
    (crate::sim::BallKind::Wood, 36.7, 0.42),
    (crate::sim::BallKind::Paper, 45.9, 0.30),
    (crate::sim::BallKind::Sticky, 43.0, 0.2195),
];

/// The steepest slope (degrees) ball `kind` climbs by its own push keeping `speed`: steeper needs a run-up.
fn steepest_climb(kind: crate::sim::BallKind, speed: f32) -> f32 {
    let (_, top, per) = CLIMB
        .iter()
        .find(|c| c.0 == kind)
        .copied()
        .unwrap_or(CLIMB[0]);
    (top - speed.max(0.0) / per).max(0.0)
}

/// How near the ball a point of the way must be to take the way up there (a run begun at a sector).
const TAKE_UP_NEAR: f32 = 8.0;

impl Pilot {
    pub fn new(route: Route, gates: Vec<Gate>) -> Pilot {
        Pilot {
            route: std::sync::Arc::new(route),
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
            hints: crate::hints::Runner::default(),
            demo: None,
            demo_at: None,
            demo_run: None,
            run_ups: Vec::new(),
            demo_after: 0,
            bridges: Vec::new(),
            charged: Vec::new(),
            noise: None,
            tracking: None,
            track_keys: Vec::new(),
            doomed_at: None,
            mode: "way search",
            zones: std::sync::Arc::default(),
            zone: crate::zones::Effect::default(),
            learned: std::sync::Arc::default(),
            learned_now: None,
            zone_said: String::new(),
            momentum: None,
            modules: std::sync::Arc::default(),
            style: Style::Calm,
            ride: Style::Calm,
            in_calm: false,
            calm_until: None,
        }
    }

    /// The ball is back at a saved state after a failure there (`attempt` from 1, the more the wider the variation):
    /// a short opening move of its own (a wait or a push of random length: moving parts are then met at another
    /// phase) and, for some seconds, random additions to its choices' scores; from the third try on every third
    /// one leaves the player's line, which may be what failed.
    /// A rewind of the raw speedrun after a failure at way point `at` of `sector` (the `attempt`th there): from
    /// `RAW_FALLBACK` tries on, the calm style rides from the save to `RAW_FALLBACK_PAST` points past it, as in a calm
    /// zone (the raw risk is taken everywhere but where it keeps failing). AUTOPILOT_RAW_FALLBACK=N changes the
    /// tries (0: never).
    pub fn fall_back(&mut self, attempt: u32, sector: usize, at: usize) {
        let after = std::env::var("AUTOPILOT_RAW_FALLBACK")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(RAW_FALLBACK);
        if self.style == Style::Raw && after > 0 && attempt >= after {
            self.calm_until = Some((sector, at + RAW_FALLBACK_PAST));
        }
    }

    pub fn perturb(&mut self, sim: &Sim, attempt: u32) {
        let mut seed = 0x9E37_79B9u32.wrapping_mul(attempt.wrapping_add(1))
            ^ (sim.tick as u32).wrapping_mul(0x85EB_CA6B);
        let mut rand = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let a = attempt.min(12);
        let opening = CHOICES[(rand() % 9) as usize];
        let len = 6 + u64::from(rand() % (12 + 6 * a));
        self.seed = rand() | 1;
        self.noise = Some(Noise {
            opening: (sim.tick + len, opening),
            seed: rand() | 1,
            until: sim.tick + 240 + 60 * u64::from(a),
            amp: 0.4 * a as f32,
            no_demo: attempt >= 3 && attempt.is_multiple_of(3),
        });
        self.hints.rearm();
        self.tracking = None;
        self.track_keys.clear();
        self.choice = opening;
        self.best = f32::NEG_INFINITY;
        self.moved = sim.tick;
        self.demo_run = None;
    }

    /// Keeping to the followed player's ball: once ours is where the player's was (as `Demo::find` matches), the
    /// player's run is a line in time, and at each decision the inputs for the next `PERIOD` ticks are chosen so that
    /// over the next `TRACK_AHEAD` ticks our ball stays nearest the player's (its place, and its speed a little): the
    /// player's own keys, or each arrow choice for `PERIOD` ticks and the player's keys after, tried on copies of the
    /// run. So a slip is corrected at once, and a puzzle's pushes (blocks, levers, a board) are made as the player
    /// made them. Where our ball is more than `TRACK_LOST` from the player's, or the player's ball is being lost, the
    /// line is taken up again further on where it passes near, or left to the way's search.
    fn track(&mut self, sim: &Sim) -> Option<u8> {
        let d = self
            .demo
            .clone()
            .filter(|_| sim.tick >= self.demo_after && !self.demo_off(sim.tick))?;
        let b = &sim.ball;
        let fits = |t: &DemoTick, near: f32| {
            (t.alive || t.doomed_in.is_some_and(|d| d > TRACK_DOOMED))
                && t.kind == b.kind
                && t.sector == sim.sector
                && t.pos.distance(b.position) < near
        };
        let k = match self.tracking {
            Some((k0, t0)) => {
                let k = k0 + usize::try_from(sim.tick.saturating_sub(t0)).unwrap_or(usize::MAX);
                if d.ticks.get(k).is_some_and(|t| fits(t, TRACK_LOST)) {
                    Some(k)
                } else {
                    // lost the line: taken up again where it passes near (after a player's lost ball, the new
                    // ball's way back past here), else left
                    let again = (k..(k + TRACK_RESYNC).min(d.ticks.len()))
                        .find(|&i| d.ticks.get(i).is_some_and(|t| fits(t, DEMO_NEAR)));
                    if again.is_none() && std::env::var_os("AUTOPILOT_DEMO").is_some() {
                        eprintln!("track lost at t{} (player's tick {k})", sim.tick);
                    }
                    again
                }
            }
            None => d.find(sim, DEMO_NEAR, Some(DEMO_SPEED), self.demo_expected(sim)),
        };
        let Some(mut k) = k else {
            self.tracking = None;
            self.track_keys.clear();
            return None;
        };
        // (AUTOPILOT_SPEEDRUN_NO_SKIP: the player's still stretches are kept, for a try without the skips)
        if self.style != Style::Calm
            && !self.in_calm
            && std::env::var_os("AUTOPILOT_SPEEDRUN_NO_SKIP").is_none()
            && let Some(skip) = idle_skip(&d, k, sim)
        {
            if std::env::var_os("AUTOPILOT_DEMO").is_some() {
                eprintln!(
                    "speedrun: the player's still ticks {k}..{skip} skipped at t{}",
                    sim.tick
                );
            }
            k = skip;
        }
        if self.tracking.is_none() && std::env::var_os("AUTOPILOT_DEMO").is_some() {
            eprintln!("track from t{} (player's tick {k})", sim.tick);
        }
        self.tracking = Some((k, sim.tick));
        self.demo_at = Some((k, sim.tick));
        if d.ticks.get(k).is_some_and(|t| t.doomed_in.is_some()) {
            self.doomed_at = Some(sim.tick);
        }
        let q = crate::hints::Facing::of(&sim.camera);
        let lives = sim.lives;
        // the player's keys from tick `k + t`, in our view (as a copy's view turns with them, each tick's own)
        let theirs = |s: &Sim, t: u64| d.input(k + t as usize, crate::hints::Facing::of(&s.camera));
        let route = self.route.clone();
        let raw = self.ride == Style::Raw;
        let cost_of = |first: Option<u8>| -> (f32, Vec<u8>) {
            let mut s = sim.clone();
            let mut keys = Vec::new();
            let mut cost = 0.0;
            for t in 0..TRACK_AHEAD {
                let i = match first {
                    Some(c) if t < PERIOD => c,
                    _ => theirs(&s, t),
                };
                if t < PERIOD {
                    keys.push(i);
                }
                s.step(i);
                if s.lives < lives || s.is_dying() || s.game_over {
                    return (f32::INFINITY, keys);
                }
                if s.finished {
                    return (f32::NEG_INFINITY, keys);
                }
                if let Some(p) = d.ticks.get(k + t as usize + 1)
                    && t % 3 == 2
                {
                    let off = s.ball.position.distance(p.pos);
                    // where the player's ball hugged an edge, ours may keep up to `TRACK_SLACK` nearer the middle
                    // (as fast, the speed still counts): each unit off the player's line within it costs less than
                    // a unit off the middle
                    // (the raw speedrun hugs it as the player did)
                    let hugged = !raw && route.off_middle(p.pos).is_some_and(|m| m.1 > 0.0);
                    let middle = route.off_middle(s.ball.position).map(|m| m.0);
                    cost += match middle.filter(|_| hugged) {
                        Some(m) => {
                            (off - TRACK_SLACK).max(0.0)
                                + TRACK_SLACK_COST * off.min(TRACK_SLACK)
                                + TRACK_MIDDLE * m
                        }
                        None => off,
                    } + 0.1 * s.ball.velocity.distance(p.vel);
                }
            }
            (cost, keys)
        };
        let mut best = cost_of(None);
        // the player's own keys are kept unless an arrow choice is clearly nearer
        best.0 -= TRACK_OWN;
        for &c in CHOICES.iter() {
            let tried = cost_of(Some(c));
            if tried.0 < best.0 {
                best = tried;
            }
        }
        if best.0 == f32::INFINITY {
            // every course loses the ball here: left to the way's search
            self.tracking = None;
            self.track_keys.clear();
            return None;
        }
        let _ = q;
        self.track_keys = best
            .1
            .iter()
            .enumerate()
            .map(|(t, &i)| (sim.tick + 1 + t as u64, i))
            .collect();
        self.choice = 0;
        self.track_keys.first().map(|x| x.1)
    }

    /// The random addition to a choice's score now (none when not varied).
    fn jitter(&mut self, tick: u64) -> f32 {
        let Some(n) = self.noise.as_mut().filter(|n| tick < n.until) else {
            return 0.0;
        };
        n.seed ^= n.seed << 13;
        n.seed ^= n.seed >> 17;
        n.seed ^= n.seed << 5;
        n.amp * (n.seed % 1000) as f32 / 1000.0
    }

    /// Whether the player's line is left this time (a varied try).
    fn demo_off(&self, tick: u64) -> bool {
        self.noise
            .as_ref()
            .is_some_and(|n| n.no_demo && tick < n.until)
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

    /// A run begun elsewhere than the level's start (a sector): the way is taken up at its point nearest the ball,
    /// past the transformers before it (the way may pass there again later, so the first point near enough wins).
    pub fn take_up(&mut self, sim: &Sim) {
        let f = sim.ball.position - Vec3::Y * crate::sim::BALL_RADIUS;
        let near = self
            .route
            .points
            .iter()
            .position(|p| p.distance(f) < TAKE_UP_NEAR);
        let Some(i) = near.or_else(|| self.route.nearest(sim.ball.position, usize::MAX)) else {
            return;
        };
        self.at = i;
        self.gate = self
            .gates
            .iter()
            .position(|g| g.0 > i)
            .unwrap_or(self.gates.len());
    }

    /// A new ball after a lost one: the way is taken up again where it stands (the reset point is behind the place
    /// the ball was lost), and whatever was planned there is dropped.
    pub fn new_ball(&mut self, sim: &Sim) {
        self.take_up(sim);
        // The new ball is the one lost (the original's New Ball script), not always the kind the way was taken in
        // here: then it first becomes that kind at the transformer nearest (the levels keep one by the reset points
        // for this); a transformer ahead that makes the kind it already is has nothing to do for it. Where the way's
        // next transformer is nearer than one of that kind, the ball is made over there anyway and goes on as it is.
        let wanted = self
            .gates
            .iter()
            .take_while(|g| g.0 <= self.at)
            .last()
            .map(|g| g.2);
        let pos = sim.ball.position;
        let next = self
            .gates
            .get(self.gate)
            .map_or(f32::INFINITY, |g| g.1.distance(pos));
        // there by a via (it backs off for a run-up where the ball cannot climb), then into it,
        let fetch = wanted.filter(|&k| k != sim.ball.kind).and_then(|k| {
            let near = sim
                .trafos()
                .filter(|(_, kind)| *kind == k)
                .min_by(|a, b| a.0.distance(pos).total_cmp(&b.0.distance(pos)));
            near.filter(|(at, _)| at.distance(pos) <= next)
                .map(|(at, _)| (at, k))
        });
        if let Some((at, k)) = fetch {
            self.hints.add(crate::hints::Hint {
                at: sim.ball.position,
                radius: 6.0,
                sector: Some(sim.sector + 1),
                kind: Some(sim.ball.kind),
                // and back to the reset point, where the way (and a followed run) goes on from
                steps: vec![
                    crate::hints::Step::Via(at, 900),
                    crate::hints::Step::Become(k),
                    crate::hints::Step::Goto(sim.ball.position, 900),
                ],
                full: Vec::new(),
                line: 0,
                retry: 0,
            });
        } else {
            while self
                .gates
                .get(self.gate)
                .is_some_and(|g| g.2 == sim.ball.kind)
            {
                self.gate += 1;
            }
        }
        self.kind = Some(sim.ball.kind);
        self.best = f32::NEG_INFINITY;
        self.moved = sim.tick;
        self.plan.clear();
        self.last = Some(sim.ball.position);
        self.choice = 0;
    }

    /// Whether the ball becoming `kind` now at `at` is a mistake: no hint made it, and it is not the way's next
    /// transformer (or the one just gone through) making that kind there. A flat transformer rolled over by a slip:
    /// a failure to go back from.
    pub fn stray_transform(&self, kind: crate::sim::BallKind, at: Vec3) -> bool {
        // (the way's next transformer, or the one just gone through, making this kind here: no stray)
        let wanted = |i: usize| {
            self.gates
                .get(i)
                .is_some_and(|g| g.2 == kind && g.1.distance(at) < 2.0 * crate::sim::TRAFO_REACH)
        };
        // (not on the player's line, which knows the way's kinds better than the gates found from it; not before the
        // way's first transformer, where the ball may first have to become the way's kind)
        self.hints.at().is_none()
            && self.tracking.is_none()
            && self.gate > 0
            && !wanted(self.gate)
            && !self.gate.checked_sub(1).is_some_and(wanted)
    }

    /// Whether a ball lost now was lost as the player lost one here: the player's line was kept to into the
    /// player's last seconds before a lost ball (within `AS_THE_PLAYER` ticks). Such a loss is not gone back from:
    /// what the player's ball did as it went may be what the level needs (a drawbridge knocked down with it).
    pub fn lost_as_the_player(&self, sim: &Sim) -> bool {
        self.doomed_at
            .is_some_and(|t| sim.tick <= t + AS_THE_PLAYER)
    }

    /// How far along the way the ball has come (route points).
    pub fn reached(&self) -> usize {
        self.at
    }

    /// A hint of its own from the followed run, when stuck near the player's line: roll to a place before this one
    /// where the player stood still (in this sector, as this ball, within reach), then give the player's keys from
    /// there on, past this place. Each place the player stood is tried twice at most.
    fn run_up(&mut self, sim: &Sim, here: f32) -> Option<crate::hints::Hint> {
        let d = self.demo.as_ref()?;
        let near = d.find(sim, RUN_UP_NEAR, None, self.demo_expected(sim))?;
        let b = &sim.ball;
        let rest = (near.saturating_sub(RUN_UP_BACK)..near).rev().find(|&k| {
            d.ticks.get(k).is_some_and(|t| {
                t.alive
                    && t.kind == b.kind
                    && t.sector == sim.sector
                    && t.vel.length() < 1.0
                    && t.pos.distance(b.position) < RUN_UP_REACH
                    && (t.pos.y - b.position.y).abs() < 2.0
                    && self.run_ups.iter().filter(|&&r| r == k).count() < 2
            })
        })?;
        self.run_ups.push(rest);
        let t = d.ticks.get(rest)?;
        let keys: Vec<u8> = (rest..rest + RUN_UP_KEYS)
            .map_while(|k| d.ticks.get(k).filter(|t| t.alive).map(|t| t.bits))
            .collect();
        let hint = crate::hints::Hint {
            at: b.position,
            radius: 3.0,
            sector: Some(sim.sector + 1),
            kind: Some(b.kind),
            steps: vec![
                crate::hints::Step::Goto(t.pos, 600),
                crate::hints::Step::Inputs(t.quarter, keys),
            ],
            full: Vec::new(),
            line: 0,
            retry: 0,
        };
        // tried on a copy: it must keep the ball and end further along the way
        let mut trial = crate::hints::Runner::new(vec![hint.clone()]);
        let mut s = sim.clone();
        let lives = s.lives;
        for _ in 0..(600 + RUN_UP_KEYS) {
            let Some(i) = trial.input(&s) else { break };
            s.step(i);
            if s.lives < lives || s.is_dying() || s.game_over {
                return None;
            }
            if s.finished {
                break;
            }
        }
        (s.finished || self.progress(s.ball.position) > here + 2.0).then_some(hint)
    }

    /// A charge at a drawbridge near (within `BRIDGE_NEAR`) that the way ahead passes over: through it to the first
    /// point of the way well past it. Each bridge is charged `BRIDGE_CHARGES` times at most.
    fn charge(&mut self, sim: &Sim) -> Option<crate::hints::Hint> {
        let b = sim.ball.position;
        let flat = |p: Vec3, q: Vec3| Vec3::new(p.x - q.x, 0.0, p.z - q.z).length();
        let (k, &bridge) = self.bridges.iter().enumerate().find(|(k, g)| {
            flat(**g, b) < BRIDGE_NEAR
                && self.charged.iter().filter(|&&c| c == *k).count() < BRIDGE_CHARGES
        })?;
        let ahead = self
            .route
            .points
            .get(self.at..(self.at + 80).min(self.route.points.len()))?;
        // the way passes over it: a stretch of it within reach of the bridge's foot
        let over = ahead.windows(2).position(|w| {
            let ab = w[1] - w[0];
            let t = ((bridge - w[0]).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            flat(w[0] + ab * t, bridge) < 5.0
        })?;
        let past = ahead
            .iter()
            .skip(over)
            .find(|p| flat(**p, b) > flat(bridge, b) + 8.0)?;
        self.charged.push(k);
        let lift = Vec3::Y * crate::sim::BALL_RADIUS;
        let via = crate::hints::Step::Via(*past + lift, 600);
        // the charges: straight through; or a ram from a run-up along the way, braking at once after it so as not to
        // follow a plank that falls into the gap, a wait while it settles, then across
        let foot = ahead.get(over).copied().unwrap_or(bridge);
        let dir = Vec3::new(bridge.x - foot.x, 0.0, bridge.z - foot.z);
        let way_dir = ahead
            .get(over + 1)
            .map_or(Vec3::ZERO, |n| Vec3::new(n.x - foot.x, 0.0, n.z - foot.z))
            .normalize_or_zero();
        let dir = if way_dir != Vec3::ZERO {
            way_dir
        } else {
            dir.normalize_or_zero()
        };
        let mut charges = vec![vec![via.clone()]];
        // (along the way first, then from each side: a plank standing across the way falls when hit from where it
        // faces)
        let sides = [dir, Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z];
        for (dir, back) in sides.iter().flat_map(|&d| [8.0f32, 14.0].map(|b| (d, b))) {
            let from = bridge - dir * back;
            if dir == Vec3::ZERO || self.route.clearance(from + lift) <= 0.0 {
                continue;
            }
            for run in [30u64, 50, 80] {
                charges.push(vec![
                    crate::hints::Step::Goto(from + lift, 400),
                    crate::hints::Step::Push(dir, run),
                    crate::hints::Step::Push(-dir, 45),
                    crate::hints::Step::Wait(120),
                    via.clone(),
                ]);
            }
        }
        let here = self.progress(sim.ball.position);
        for steps in charges {
            let hint = crate::hints::Hint {
                at: b,
                radius: 6.0,
                sector: Some(sim.sector + 1),
                kind: Some(sim.ball.kind),
                steps,
                full: Vec::new(),
                line: 0,
                retry: 0,
            };
            // tried on a copy first: some bridges are lowered by a lever, not knocked over, and a charge at them
            // throws the ball off; it must keep the ball and get on
            let mut trial = crate::hints::Runner::new(vec![hint.clone()]);
            let mut s = sim.clone();
            let lives = s.lives;
            let mut lost = false;
            for t in 0..1500 {
                let i = trial.input(&s).unwrap_or(0);
                s.step(i);
                if s.lives < lives || s.is_dying() || s.game_over {
                    lost = true;
                    break;
                }
                if s.finished || (t > 60 && trial.at().is_none()) {
                    break;
                }
            }
            let ok = !lost && (s.finished || self.progress(s.ball.position) > here + 2.0);
            eprintln!(
                "charge the drawbridge at {bridge:?} ({} steps): {}",
                hint.steps.len(),
                if ok { "taken" } else { "no use" }
            );
            if ok {
                return Some(hint);
            }
        }
        None
    }

    /// A leap over a gap the way crosses just ahead (two of its points further apart than a gap the ball rolls over):
    /// back from its edge for a run-up, then pushing across until the ball has flown and landed. The run-ups are
    /// tried on a copy of the run, the shortest that keeps the ball and gets on is taken.
    fn leap(&mut self, sim: &Sim) -> Option<crate::hints::Hint> {
        // (not on a followed player's line: a gap in it is where the player flew, and the player's keys take it)
        if self.demo.as_ref().is_some_and(|d| {
            d.find(sim, RUN_UP_NEAR, None, self.demo_expected(sim))
                .is_some()
        }) {
            return None;
        }
        let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
        let ahead = self
            .route
            .points
            .get(self.at..(self.at + LOOK).min(self.route.points.len()))?;
        let k = ahead
            .windows(2)
            .position(|w| flat(w[1] - w[0]).length() > GAP)?;
        let (edge, over) = (*ahead.get(k)?, *ahead.get(k + 1)?);
        let dir = flat(over - edge).normalize_or_zero();
        let here = self.progress(sim.ball.position);
        let lift = Vec3::Y * crate::sim::BALL_RADIUS;
        for back in LEAP_RUN_UPS {
            let from = edge - dir * back;
            if self.route.clearance(from + lift) <= 0.0 {
                continue;
            }
            let hint = crate::hints::Hint {
                at: sim.ball.position,
                radius: 6.0,
                sector: Some(sim.sector + 1),
                kind: Some(sim.ball.kind),
                steps: vec![
                    crate::hints::Step::Goto(from + lift, 400),
                    crate::hints::Step::Air(dir, 300),
                ],
                full: Vec::new(),
                line: 0,
                retry: 0,
            };
            let mut trial = crate::hints::Runner::new(vec![hint.clone()]);
            let mut s = sim.clone();
            let lives = s.lives;
            let mut lost = false;
            for t in 0..900 {
                let i = trial.input(&s).unwrap_or(0);
                s.step(i);
                if s.lives < lives || s.is_dying() || s.game_over {
                    lost = true;
                    break;
                }
                if s.finished || (t > 30 && trial.at().is_none() && s.ball.on_ground) {
                    break;
                }
            }
            if !lost && (s.finished || self.progress(s.ball.position) > here + 2.0) {
                eprintln!("leap from {edge:?} to {over:?} with a run-up of {back}");
                return Some(hint);
            }
        }
        None
    }

    /// Into the next transformer, near it (its rim holds the ball off): through it, backing off for a run-up.
    fn gate_hint(&self, sim: &Sim) -> Option<crate::hints::Hint> {
        let &(_, g, k) = self.gates.get(self.gate)?;
        if sim.ball.kind == k || g.distance(sim.ball.position) >= GATE_STUCK {
            return None;
        }
        let (t, _) = sim
            .trafos()
            .filter(|(_, kind)| *kind == k)
            .min_by(|a, b| a.0.distance(g).total_cmp(&b.0.distance(g)))?;
        Some(crate::hints::Hint {
            at: sim.ball.position,
            radius: 6.0,
            sector: Some(sim.sector + 1),
            kind: Some(sim.ball.kind),
            steps: vec![
                crate::hints::Step::Via(t, 900),
                crate::hints::Step::Become(k),
            ],
            full: Vec::new(),
            line: 0,
            retry: 0,
        })
    }

    /// Held up where the way goes off a rail onto a floor, or up a little step (a sill): a short run-up back along the
    /// way and on over it at speed, as a player keeps the momentum there.
    fn over_sill(&self, sim: &Sim) -> Option<crate::hints::Hint> {
        let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
        let pts = &self.route.points;
        let end = (self.at + SILL_AHEAD).min(pts.len().saturating_sub(1));
        let rail = |p: Vec3| self.route.floor_point(p).is_some_and(|q| q.rail);
        let k = (self.at..end).find(|&k| {
            let (a, b) = (pts[k], pts[k + 1]);
            (rail(a) && !rail(b)) || (b.y > a.y + SILL_STEP && b.y < a.y + crate::sim::BALL_RADIUS)
        })?;
        // (right at it, and with floor behind for the run-up)
        if flat(pts[k + 1] - sim.ball.position).length() > SILL_NEAR {
            return None;
        }
        let past = *pts.get((k + SILL_PAST).min(pts.len() - 1))?;
        let dir = flat(past - sim.ball.position).normalize_or_zero();
        if dir == Vec3::ZERO || self.route.clearance(sim.ball.position - dir * SILL_NEAR) <= 0.0 {
            return None;
        }
        Some(crate::hints::Hint {
            at: sim.ball.position,
            radius: 6.0,
            sector: Some(sim.sector + 1),
            kind: Some(sim.ball.kind),
            steps: vec![
                crate::hints::Step::Push(-dir, SILL_BACK),
                crate::hints::Step::Via(past + Vec3::Y * crate::sim::BALL_RADIUS, 240),
            ],
            full: Vec::new(),
            line: 0,
            retry: 0,
        })
    }

    /// The ball is by a gap the way crosses just ahead (within `GAP_CLOSE` of its near edge).
    fn gap_close(&self, sim: &Sim) -> bool {
        let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
        let end = (self.at + LOOK).min(self.route.points.len());
        self.route.points.get(self.at..end).is_some_and(|ahead| {
            ahead.windows(2).any(|w| {
                flat(w[1] - w[0]).length() > GAP
                    && flat(w[0] - sim.ball.position).length() < GAP_CLOSE
            })
        })
    }

    /// Where the followed player's run is now: the tick last matched, gone on by the ticks since.
    fn demo_expected(&self, sim: &Sim) -> Option<usize> {
        self.demo_at
            .map(|(k, at)| k + usize::try_from(sim.tick.saturating_sub(at)).unwrap_or(0))
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
        // pumping: to and fro along a choice, swinging higher each time (out of a trough between waves too steep to
        // climb from a standstill, a heavy ball above all)
        for &c in CHOICES.iter().skip(1) {
            for swing in [40u64, 70] {
                for times in [2u64, 4] {
                    let mut plan = Vec::new();
                    let mut t = sim.tick;
                    for _ in 0..times {
                        t += swing;
                        plan.push((t, opposite(c)));
                        t += swing;
                        plan.push((t, c));
                    }
                    plan.push((t + 2 * swing, c));
                    plans.push(plan);
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
        // the player's pushes from the places near where the player's ball was, for longer stretches
        if let Some(d) = &self.demo {
            let mut starts: Vec<usize> = Vec::new();
            for speed in [Some(DEMO_SPEED), None] {
                if let Some(k) = d.find(sim, DEMO_STUCK_NEAR, speed, self.demo_expected(sim))
                    && !starts.contains(&k)
                {
                    starts.push(k);
                }
            }
            for k in starts {
                for len in DEMO_STRETCHES {
                    // as held choices, a tick each (the copy's view does not turn: the arrows are for this one)
                    let q = crate::hints::Facing::of(&sim.camera);
                    let mut plan: Vec<(u64, u8)> = Vec::new();
                    for i in 0..len {
                        let c = d.input(k + i as usize, q);
                        match plan.last_mut() {
                            Some(last) if last.1 == c => last.0 = sim.tick + i + 1,
                            _ => plan.push((sim.tick + i + 1, c)),
                        }
                    }
                    plans.push(plan);
                }
            }
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

    /// A beam search over sequences of held choices (`SEARCH_HOLDS` long, or a wait), `SEARCH_DEPTH` deep, keeping
    /// the `SEARCH_BEAM` best states at each depth (one a place: the beam stays spread). A state is judged after a
    /// coast with no arrows, by how far along the way it is and how little stands in the way; a lost ball ends a
    /// sequence. The best sequence that gets on from `here` is the plan. Deterministic: the order of the tries is
    /// fixed and ties keep the first.
    fn search(&self, sim: &Sim, here: f32) -> Option<Vec<(u64, u8)>> {
        let mut actions: Vec<(u8, u64)> = Vec::new();
        for &c in CHOICES.iter().skip(1) {
            for &d in &SEARCH_HOLDS {
                actions.push((c, d));
            }
        }
        actions.push((0, 30));
        let base = here - self.blocking(sim) - self.parts_in_way(sim);
        // (state, plan, score)
        let mut beam: Vec<Node> = vec![(sim.clone(), Vec::new(), base)];
        let mut best: Option<(f32, Vec<(u64, u8)>)> = None;
        let lives = sim.lives;
        for _ in 0..SEARCH_DEPTH {
            let jobs: Vec<(usize, (u8, u64))> = (0..beam.len())
                .flat_map(|i| actions.iter().map(move |&a| (i, a)))
                .collect();
            let run = |&(i, (c, d)): &(usize, (u8, u64))| -> Option<Node> {
                let (from, plan, _) = beam.get(i)?;
                let mut s = from.clone();
                for _ in 0..d {
                    s.step(c);
                    if s.lives < lives || s.is_dying() || s.game_over {
                        return None;
                    }
                }
                let mut plan = plan.clone();
                plan.push((s.tick, c));
                if s.finished {
                    return Some((s, plan, f32::INFINITY));
                }
                // judged where it comes to after a coast
                let mut judge = s.clone();
                for _ in 0..SEARCH_COAST {
                    judge.step(0);
                    if judge.lives < lives || judge.is_dying() || judge.game_over {
                        return None;
                    }
                }
                if !judge.ball.on_ground && !judge.in_updraft() {
                    return None;
                }
                let v = self.progress(judge.ball.position)
                    - self.blocking(&judge)
                    - self.parts_in_way(&judge);
                Some((s, plan, v))
            };
            let tried: Vec<Option<Node>> = if std::env::var_os("AUTOPILOT_PARALLEL").is_some() {
                std::thread::scope(|scope| {
                    let run = &run;
                    let chunks: Vec<_> = jobs
                        .chunks(jobs.len().div_ceil(SEARCH_THREADS).max(1))
                        .map(|ch| scope.spawn(move || ch.iter().map(run).collect::<Vec<_>>()))
                        .collect();
                    chunks
                        .into_iter()
                        .flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
                        .collect()
                })
            } else {
                jobs.iter().map(run).collect()
            };
            let mut next: Vec<Node> = tried.into_iter().flatten().collect();
            next.sort_by(|a, b| b.2.total_cmp(&a.2));
            for n in &next {
                if n.2 > base + 0.5 && best.as_ref().is_none_or(|b| n.2 > b.0) {
                    best = Some((n.2, n.1.clone()));
                }
            }
            if best.as_ref().is_some_and(|b| b.0 == f32::INFINITY) {
                break;
            }
            // one a place (2 units), the best first
            let mut kept: Vec<Node> = Vec::new();
            for n in next {
                if kept.len() >= SEARCH_BEAM {
                    break;
                }
                if kept
                    .iter()
                    .all(|k| k.0.ball.position.distance(n.0.ball.position) > 2.0)
                {
                    kept.push(n);
                }
            }
            if kept.is_empty() {
                break;
            }
            beam = kept;
        }
        if let Some((v, plan)) = &best {
            eprintln!(
                "search at tick {}: {} steps, {:.1} -> {v:.1}",
                sim.tick,
                plan.len(),
                base
            );
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

    /// How much the modules' moving parts stand on the way ahead (a pusher's block in the lane, a plank standing up):
    /// each part whose middle is over the next stretch of the way, above its floor and not under the ball, counts as
    /// ten points of it. Only the search when stuck weighs this: a part moved out of the lane is worth a sequence.
    fn parts_in_way(&self, sim: &Sim) -> f32 {
        let ahead = self
            .route
            .points
            .get(self.at..(self.at + 40).min(self.route.points.len()))
            .unwrap_or_default();
        let world = sim.world();
        let ball = sim.ball.position;
        let mut n = 0.0;
        for m in sim.module_bodies() {
            let Some(st) = world.state(m.body) else {
                continue;
            };
            let p = st.pos;
            if p.distance(ball) < 4.0 {
                continue;
            }
            let over = ahead.iter().any(|q| {
                let up = p.y - q.y;
                Vec3::new(p.x - q.x, 0.0, p.z - q.z).length() < 2.5 && up > 0.5 && up < 4.0
            });
            if over {
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

    /// The place on the way found anew after something else drove since tick `began` (a hint, the watcher's hands):
    /// the way's point nearest the ball before the next transformer; but what drove may have gone past
    /// transformers, making the ball over: where that point is far from the ball, the way is taken up at its first
    /// point on from here near the ball where the way's ball is of this kind.
    fn find_place(&mut self, sim: &Sim, began: u64) {
        // a hint knows the place better than the followed run: the player's line waits a while after one
        self.demo_after = sim.tick + DEMO_AFTER_HINT;
        // the transformers of the way it made the ball over in, in their order, are gone through: the way is taken
        // up after the last of them (a hint begun as the ball rolls into one has the pilot miss it, and a way that
        // crosses itself would put the ball back before it)
        let mut passed = None;
        for (_, e) in sim.events.iter().filter(|(t, _)| *t >= began) {
            if let crate::sim::Event::Transform(k) = e
                && let Some(&(i, _, gk)) = self.gates.get(self.gate)
                && gk == *k
            {
                passed = Some(i);
                self.gate += 1;
                self.kind = Some(*k);
            }
        }
        if let Some(i) = passed {
            // the place on from it nearest the ball, before the transformer next
            let f = sim.ball.position - Vec3::Y * crate::sim::BALL_RADIUS;
            let after = (i..self.until().min(self.route.points.len())).min_by(|&a, &b| {
                let d = |k: usize| {
                    self.route
                        .points
                        .get(k)
                        .map_or(f32::INFINITY, |p| p.distance_squared(f))
                };
                d(a).total_cmp(&d(b))
            });
            self.at = self.at.max(after.unwrap_or(i));
        }
        // the way's point nearest the ball before the next transformer; but a hint may have gone past
        // transformers, making the ball over itself: where that point is far from the ball, the way is taken up
        // at its first point on from here near the ball where the way's ball is of this kind
        let f = sim.ball.position - Vec3::Y * crate::sim::BALL_RADIUS;
        let near = self.route.nearest(sim.ball.position, self.until());
        let off = near
            .and_then(|i| self.route.points.get(i))
            .is_none_or(|p| p.distance(f) >= TAKE_UP_NEAR);
        let made_over = sim
            .events
            .iter()
            .rev()
            .take_while(|(t, _)| *t >= began)
            .any(|(_, e)| matches!(e, crate::sim::Event::Transform(_)));
        let kind = |i: usize| {
            self.gates
                .iter()
                .take_while(|g| g.0 <= i)
                .last()
                .map(|g| g.2)
        };
        let on = (off && made_over)
            .then(|| {
                (self.until()..self.route.points.len()).find(|&i| {
                    self.route
                        .points
                        .get(i)
                        .is_some_and(|p| p.distance(f) < TAKE_UP_NEAR)
                        && kind(i).is_none_or(|k| k == sim.ball.kind)
                })
            })
            .flatten();
        if let Some(i) = on {
            self.at = i;
            self.gate = self
                .gates
                .iter()
                .position(|g| g.0 > i)
                .unwrap_or(self.gates.len());
        } else if let Some(i) = near {
            self.at = self.at.max(i);
        }
        self.best = f32::NEG_INFINITY;
        self.moved = sim.tick;
        self.plan.clear();
        self.last = Some(sim.ball.position);
        self.choice = 0;
    }

    /// The watcher hands the ball back after driving it since tick `began`: the place on the way is found anew,
    /// and getting on is counted from here.
    pub fn resume(&mut self, sim: &Sim, began: u64) {
        self.find_place(sim, began);
        self.kind = Some(sim.ball.kind);
        self.gate = self
            .gate
            .max(self.gates.iter().take_while(|g| g.0 <= self.at).count());
        self.tracking = None;
        self.track_keys.clear();
        self.demo_run = None;
        self.demo_after = 0;
    }

    /// What steers now, in a word (for the evidence of a learned rule).
    pub fn mode(&self) -> &'static str {
        if self.tracking.is_some() {
            "keeping to the player's line"
        } else {
            self.mode
        }
    }

    /// The situation of the ball at `p` (as `kind`), the way read from its point `at` (learned.rs): what it is on,
    /// the floor's width, what borders it, the way's bend and slope ahead, what comes just ahead, the module near.
    pub fn situation(
        &self,
        p: Vec3,
        at: usize,
        kind: crate::sim::BallKind,
    ) -> crate::learned::Situation {
        let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
        let foot = p - Vec3::Y * crate::sim::BALL_RADIUS;
        let mut v: [Option<String>; 8] = Default::default();
        let set = |v: &mut [Option<String>; 8], f: &str, val: &str| {
            if let Some(k) = crate::learned::FEATURES.iter().position(|x| *x == f)
                && let Some(slot) = v.get_mut(k)
            {
                *slot = Some(val.to_owned());
            }
        };
        let under = self.route.floor_point(p);
        set(
            &mut v,
            "surface",
            match under {
                Some(q) if q.rail => "rail",
                Some(_) => "floor",
                None => "off",
            },
        );
        if let Some(q) = under.filter(|q| !q.rail) {
            let w = 2.0 * q.half;
            set(
                &mut v,
                "width",
                if w < NARROW {
                    "narrow"
                } else if w < MEDIUM {
                    "medium"
                } else {
                    "wide"
                },
            );
            set(
                &mut v,
                "border",
                match self.route.edges.border(foot) {
                    Some(true) => "walls",
                    Some(false) => "open",
                    None => "far",
                },
            );
        }
        let pts = &self.route.points;
        // (the way read from its point nearest the ball a little on from `at`: the place on the way lags a hint)
        let at = (at..(at + LOOK).min(pts.len()))
            .min_by(|&a, &b| {
                pts[a]
                    .distance_squared(foot)
                    .total_cmp(&pts[b].distance_squared(foot))
            })
            .unwrap_or(at);
        let pt = |k: usize| pts.get(k.min(pts.len().saturating_sub(1))).copied();
        if let (Some(a), Some(b), Some(c)) = (
            pt(at),
            pt(at + SITUATION_AHEAD / 2),
            pt(at + SITUATION_AHEAD),
        ) {
            let (d1, d2) = (
                flat(b - a).normalize_or_zero(),
                flat(c - b).normalize_or_zero(),
            );
            if d1 != Vec3::ZERO && d2 != Vec3::ZERO {
                let angle = d1.dot(d2).clamp(-1.0, 1.0).acos().to_degrees();
                set(
                    &mut v,
                    "curve",
                    if angle < BEND {
                        "straight"
                    } else if angle < SHARP {
                        "bend"
                    } else {
                        "sharp"
                    },
                );
            }
            let run = flat(c - a).length();
            if run > 1.0 {
                let s = (c.y - a.y) / run;
                set(
                    &mut v,
                    "slope",
                    if s > SLOPE {
                        "up"
                    } else if s < -SLOPE {
                        "down"
                    } else {
                        "flat"
                    },
                );
            }
        }
        // what comes just ahead along the way, the first of: a gap, a drop, a rail's end or start, a sill, a
        // transformer
        let end = (at + SITUATION_AHEAD).min(pts.len().saturating_sub(1));
        let mut ahead = (at..end).find_map(|k| self.link(k)).unwrap_or("none");
        if ahead == "none"
            && self
                .gates
                .iter()
                .any(|g| g.0 >= at && g.0 <= end + SITUATION_AHEAD && g.1.distance(p) < 12.0)
        {
            ahead = "transformer";
        }
        set(&mut v, "ahead", ahead);
        let module = self
            .modules
            .iter()
            .filter(|m| m.0.distance(p) < SITUATION_MODULE)
            .min_by(|a, b| a.0.distance(p).total_cmp(&b.0.distance(p)))
            .map_or("none", |m| m.1.as_str());
        set(&mut v, "module", module);
        set(&mut v, "kind", &format!("{kind:?}").to_ascii_lowercase());
        crate::learned::Situation { values: v }
    }

    /// What the way's link from point `k` to the next is, if anything special: a gap, a drop, a rail's end or start,
    /// a sill (for the situation of a place, and where a careful zone must let the ball keep its momentum).
    fn link(&self, k: usize) -> Option<&'static str> {
        let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
        let pts = &self.route.points;
        let rail = |q: Vec3| {
            self.route
                .floor_point(q + Vec3::Y * crate::sim::BALL_RADIUS)
                .is_some_and(|f| f.rail)
        };
        let (Some(&a), Some(&b)) = (pts.get(k), pts.get(k + 1)) else {
            return None;
        };
        let h = flat(b - a).length();
        // (a step up out of the slope the way had before it, not a ramp's rise)
        let before = k
            .checked_sub(1)
            .and_then(|j| pts.get(j))
            .map_or(0.0, |&z| a.y - z.y);
        let rise = b.y - a.y;
        if h > GAP {
            Some("gap")
        } else if a.y - b.y > DROP_STEP && a.y - b.y > h {
            Some("drop")
        } else if rail(a) && !rail(b) {
            Some("rail-end")
        } else if !rail(a) && rail(b) {
            Some("rail-start")
        } else if rise > SILL_STEP
            && rise < crate::sim::BALL_RADIUS
            && rise > SILL_STEEP * h
            && before < rise / 3.0
        {
            Some("sill")
        } else {
            None
        }
    }

    /// Where a careful zone's (or careful rule's) speed limit `cap` must not hold, because the physics needs the
    /// ball's momentum there (autopilot-rules.md, "Zones the watcher marks"): why, in words, or None. The same places
    /// let a flow zone's pace go (no `cap`): backing off for a run-up there is no dawdling. A hint running
    /// says speed (a push up a climb, along rails, a ram, or a pass that is the run-up to one), or the way ahead of the
    /// ball (`MOMENTUM_AHEAD`) has a gap to cross or a climb steeper than the ball takes by its own push at the
    /// limit (`steepest_climb`), or just ahead a sill. There the zone keeps only its line (no corner cut), not its limit.
    fn momentum(&self, sim: &Sim, cap: Option<f32>) -> Option<&'static str> {
        let p = sim.ball.position;
        if let Some(why) = cap.and_then(|cap| self.hints.says_speed(p, cap)) {
            return Some(why);
        }
        let pts = &self.route.points;
        if pts.len() < 2 {
            return None;
        }
        let foot = p - Vec3::Y * crate::sim::BALL_RADIUS;
        // (the way's point nearest the ball a little on from `at`: the place on the way lags a hint)
        let at = (self.at..(self.at + LOOK).min(pts.len()))
            .min_by(|&a, &b| {
                pts[a]
                    .distance_squared(foot)
                    .total_cmp(&pts[b].distance_squared(foot))
            })
            .unwrap_or(self.at);
        let end = (at + MOMENTUM_AHEAD).min(pts.len() - 1);
        // (a flow zone: the climb the ball takes at a careful zone's pace)
        let steep = steepest_climb(sim.ball.kind, cap.unwrap_or(crate::zones::CAREFUL_SPEED))
            .to_radians()
            .tan();
        for k in at..end {
            match self.link(k) {
                Some("gap") => return Some("a gap to cross ahead"),
                // (a sill is taken at the pace when the ball is at it; a rail is got on slowly, or by a climb)
                Some("sill") if k < at + SITUATION_AHEAD => return Some("a sill ahead"),
                _ => {}
            }
            // a climb over two links (about three units), not one point's noise
            if let (Some(&a), Some(&c)) = (pts.get(k), pts.get(k + 2)) {
                let run = Vec3::new(c.x - a.x, 0.0, c.z - a.z).length();
                if run > 1.0 && (c.y - a.y) / run > steep {
                    return Some("a climb that needs a run-up ahead");
                }
            }
        }
        None
    }

    /// The way's point nearest the ball at `p` (for the situation of a place it passed).
    pub fn nearest_point(&self, p: Vec3) -> usize {
        self.route.nearest(p, usize::MAX).unwrap_or(self.at)
    }

    /// What steers now and where to, for whoever watches: the mode, the way's point reached of all, the next
    /// transformer, the hint running.
    pub fn status(&self) -> String {
        let gate = self.gates.get(self.gate).map_or_else(String::new, |g| {
            format!(
                "   next: {:?} transformer at ({:.0} {:.0} {:.0})",
                g.2, g.1.x, g.1.y, g.1.z
            )
        });
        let mode = if self.tracking.is_some() {
            "keeping to the player's line"
        } else {
            self.mode
        };
        let hint = self
            .hints
            .describe()
            .map_or_else(String::new, |h| format!("\n{h}"));
        let zone = self.zone_words();
        format!(
            "{mode}{zone}   way {} / {}{gate}{hint}",
            self.at,
            self.route.points.len()
        )
    }

    /// What the zones or the learned rules ask now, in words for the status line (empty: nothing).
    fn zone_words(&self) -> String {
        let mut zone = String::new();
        if self.zone.flow {
            zone += "   [flow zone]";
        }
        if let Some(l) = &self.learned_now {
            zone = format!("   [learned: {l}]");
        } else if let Some(c) = self.zone.cap {
            zone += &format!("   [careful zone, at most {c}]");
        }
        if self.zone.plain {
            zone += "   [plain zone: no learned rule]";
        }
        if let Some(why) = self.momentum {
            let what = if self.zone.cap.is_some() {
                "limit"
            } else {
                "pace"
            };
            zone += &format!("   [{what} let go: {why}]");
        }
        zone
    }

    /// The input for the next tick of `sim`: as the steering, a hint or the player's line has it, kept under a
    /// careful zone's speed limit.
    pub fn next(&mut self, sim: &Sim) -> u8 {
        self.zone = self.zones.at(sim.ball.position, sim.sector);
        // the level's own zones first (a plain zone too: there no learned rule holds); elsewhere what was learned on
        // any level, where the place is of its kind
        self.learned_now = None;
        // a speedrun holds back nowhere: no careful zone or rule. Its pace is the progress along the way itself (a
        // flow zone's push for speed would rather roll back down a climb fast than up it slowly); a hint's stops go
        // as the level's flow zones have them (a wait a puzzle needs stays: a copy of the run judges only the ball)
        // (a calm zone asks the calm style of a speedrun, and nothing of the calm style)
        // (AUTOPILOT_SPEEDRUN_SKIP_ONLY: a speedrun at the calm pace that still skips the followed player's still
        // stretches, for a try at a level where the speedrun's pace fails)
        // (the raw speedrun: no zone at all, but a calm one with AUTOPILOT_RAW_CALM_ZONES)
        let calm = std::mem::take(&mut self.zone.calm);
        // (the raw speedrun's fallback, past the place it kept failing at: over)
        if self
            .calm_until
            .is_some_and(|(s, u)| sim.sector != s || self.at > u)
        {
            self.calm_until = None;
        }
        self.in_calm = (self.style == Style::Raw && self.calm_until.is_some())
            || calm
                && (self.style == Style::Speedrun
                    || (self.style == Style::Raw
                        && std::env::var_os("AUTOPILOT_RAW_CALM_ZONES").is_some()));
        self.ride = self.style;
        if self.in_calm
            || (self.style == Style::Speedrun
                && std::env::var_os("AUTOPILOT_SPEEDRUN_SKIP_ONLY").is_some())
        {
            self.ride = Style::Calm;
        }
        // (the raw speedrun: no zone in the steering; a flow zone still flows a hint's stops, as in the speedrun)
        let mut hint_flow = false;
        if self.ride == Style::Raw {
            hint_flow = self.zone.flow;
            self.zone = crate::zones::Effect::default();
        } else if self.ride == Style::Speedrun {
            self.zone.cap = None;
        } else if self.zone == crate::zones::Effect::default() && !self.learned.is_empty() {
            let here = self.situation(sim.ball.position, self.at, sim.ball.kind);
            if let Some((e, label)) = self.learned.at(&here) {
                self.zone = e;
                self.learned_now = Some(label);
            }
        }
        // where the physics needs the ball's momentum, a careful limit holds back nothing (only its line is kept), and
        // a flow zone's pace does not forbid backing off for a run-up
        self.momentum = if self.zone.cap.is_some() || self.zone.flow {
            self.momentum(sim, self.zone.cap)
        } else {
            None
        };
        self.hints.zone = self.limit();
        self.hints.zone.flow |= hint_flow;
        // (AUTOPILOT_RAW_FLOW_HINTS: every hint's stop flowed where a copy keeps the ball, its speeds as minimums)
        self.hints.raw =
            self.ride == Style::Raw && std::env::var_os("AUTOPILOT_RAW_FLOW_HINTS").is_some();
        if self.hints.raw {
            self.hints.zone.flow = true;
        }
        // AUTOPILOT_SITUATION: the place as the learned rules see it, every second, and each change of what the
        // zones and rules ask (which zone or rule, from which tick)
        if std::env::var_os("AUTOPILOT_SITUATION").is_some() {
            let p = sim.ball.position;
            if sim.tick.is_multiple_of(60) {
                eprintln!(
                    "situation t{} ({:.1} {:.1} {:.1}) {}",
                    sim.tick,
                    p.x,
                    p.y,
                    p.z,
                    self.situation(p, self.at, sim.ball.kind).describe()
                );
            }
            let said = self.zone_words();
            if said != self.zone_said {
                eprintln!(
                    "zone t{} ({:.1} {:.1} {:.1}) {}",
                    sim.tick,
                    p.x,
                    p.y,
                    p.z,
                    if said.is_empty() { "none" } else { said.trim() }
                );
                self.zone_said = said;
            }
        }
        let i = self.decide(sim);
        match self.limit().cap {
            Some(cap) => under_cap(i, sim, cap),
            None => i,
        }
    }

    /// What the zones ask of the ball's speed and of the hints now: their effect, without the careful limit and the
    /// flow where the ball needs its momentum (`momentum`): there a run-up's back-off and stop are no dawdling.
    fn limit(&self) -> crate::zones::Effect {
        let mut e = self.zone;
        if self.momentum.is_some() {
            e.cap = None;
            e.flow = false;
        }
        e
    }

    /// The input for the next tick of `sim`, before a careful zone's limit.
    fn decide(&mut self, sim: &Sim) -> u8 {
        if let Some((until, c)) = self.noise.as_ref().map(|n| n.opening)
            && sim.tick < until
        {
            self.mode = "a varied opening";
            return c;
        }
        // a hint for this place drives while it runs; after it, the place on the way is found anew
        if let Some(i) = self.hints.input(sim) {
            self.mode = "hint";
            return i;
        }
        if std::mem::take(&mut self.hints.ended) {
            let began = self.hints.began;
            self.find_place(sim, began);
        }
        // a new ball (the old one lost) starts again at a reset point: find it on the way anew (seen every tick, so
        // that a plan or the player's keys carrying the ball far is no new ball)
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
        if let Some(&(_, i)) = self.track_keys.iter().find(|k| k.0 == sim.tick + 1) {
            return i;
        }
        if !sim.tick.is_multiple_of(PERIOD) {
            if let (Some((k, from)), Some(d)) = (self.demo_run, &self.demo) {
                let t = usize::try_from(sim.tick.saturating_sub(from)).unwrap_or(usize::MAX);
                return d.input(k.saturating_add(t), crate::hints::Facing::of(&sim.camera));
            }
            return self.choice;
        }
        self.demo_run = None;
        // A player turns the view to face the way ahead (Shift with Left or Right, a quarter a press): when the way
        // ahead points well away from where "up" pushes, one press this tick, released the next. Not where the
        // followed player's line is at hand: it turns the view as the player did.
        let on_demo = self
            .demo
            .as_ref()
            .filter(|_| sim.tick >= self.demo_after)
            .and_then(|d| d.find(sim, DEMO_NEAR, Some(DEMO_SPEED), self.demo_expected(sim)))
            .is_some()
            || self.tracking.is_some();
        if !on_demo && let Some(turn) = self.turn(sim) {
            return turn;
        }
        // the next transformer is gone through: the ball has become its kind, or was its kind already and reached it
        // (or the way's ball was of its kind before it too: then it changes nothing, and its rim, which a ball of its
        // kind does not sink into, is no place to roll onto)
        let had = *self.kind.get_or_insert(sim.ball.kind);
        let before = self
            .gate
            .checked_sub(1)
            .and_then(|i| self.gates.get(i))
            .map(|g| g.2);
        if let Some(&(_, g, k)) = self.gates.get(self.gate) {
            let reached = g.distance(sim.ball.position) < crate::sim::TRAFO_REACH;
            let idle = before.map_or(had == k, |b| b == k);
            if sim.ball.kind == k && (had != k || reached || idle) {
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
        // on the followed player's line: kept to it tick by tick
        if let Some(i) = self.track(sim) {
            return i;
        }
        // held up a moment right at a gap the way crosses, or at the next transformer's rim: no need to stand there
        // for the full four seconds (a steady player goes on at once): the leap, or into the transformer, now
        let held = sim.tick.saturating_sub(self.moved);
        // (following a player's run, the player's pace is the measure: the steady style is for a way of its own)
        let steady = self.demo.is_none();
        let stuck_try = if steady {
            STUCK_TRY
        } else {
            STUCK_TRY_FOLLOWING
        };
        if steady && held > HELD_UP && held <= stuck_try {
            if self.gap_close(sim)
                && let Some(hint) = self.leap(sim)
            {
                self.moved = sim.tick;
                self.hints.add(hint);
                return 0;
            }
            if let Some(hint) = self.gate_hint(sim).or_else(|| self.over_sill(sim)) {
                self.moved = sim.tick;
                self.hints.add(hint);
                return 0;
            }
        }
        // stuck for two seconds: try rams and random manoeuvres and keep the best that gets on (a steady player does
        // not stand about long)
        if sim.tick.saturating_sub(self.moved) > stuck_try {
            // found or not, the next try comes after four seconds more
            self.moved = sim.tick;
            // stuck at a drawbridge the way goes over: charge it along the way (backing off for a run-up while it
            // stands), to knock it over and roll across (first: no manoeuvre knows to)
            if let Some(hint) = self.charge(sim).or_else(|| self.leap(sim)) {
                self.hints.add(hint);
                return 0;
            }
            if let Some(plan) = self.manoeuvre(sim, here) {
                self.plan = plan;
                self.mode = "a manoeuvre (stuck)";
                if let Some(&(_, c)) = self.plan.first() {
                    self.choice = c;
                    return c;
                }
            }
            // stuck by the next transformer (its rim holds the ball off): into it, backing off for a run-up
            if let Some(hint) = self.gate_hint(sim) {
                self.hints.add(hint);
                return 0;
            }
            // no manoeuvre gets on, near the followed player's line: back to where the player stood before this
            // place, and from there the player's keys (a run-up the player took, a gap crossed at speed), when on a
            // copy of the run that keeps the ball and gets on
            if let Some(hint) = self.run_up(sim, here) {
                self.hints.add(hint);
                return 0;
            }
            // nothing short gets on: a search over sequences of held choices (beam search on copies of the run)
            if let Some(plan) = self.search(sim, here) {
                self.plan = plan;
                self.mode = "beam search (stuck)";
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
        // the player's pushes from where the player's ball was where ours is, as fast: one more choice
        let matched = self
            .demo
            .as_ref()
            .filter(|_| sim.tick >= self.demo_after && !self.demo_off(sim.tick))
            .and_then(|d| d.find(sim, DEMO_NEAR, Some(DEMO_SPEED), self.demo_expected(sim)));
        if let Some(k) = matched {
            self.demo_at = Some((k, sim.tick));
        }
        let mut demo_won = false;
        // stuck for two seconds: hold each choice for up to two seconds
        let stuck = sim.tick.saturating_sub(self.moved) / 120;
        let hold = HOLD * (1 + (stuck % 4) * 2);
        let choices: Vec<u8> = CHOICES
            .iter()
            .copied()
            .chain(matched.map(|_| DEMO_CHOICE))
            .collect();
        let demo_k = matched.zip(self.demo.as_deref());
        // in a careful zone the way's line ahead is the line kept to (no corner cut), also where its limit is let go
        // for the ball's momentum
        let zone = self.limit();
        let line_end = (self.at + LOOK).min(self.route.points.len());
        let line = self
            .zone
            .cap
            .and_then(|_| self.route.points.get(self.at.saturating_sub(2)..line_end))
            .filter(|l| l.len() >= 2);
        let pace = self.ride.pace();
        let tried = rollouts(sim, &self.route, demo_k, &choices, hold, low, line, pace);
        for (&c, (s, lost, edge)) in choices.iter().zip(tried) {
            let from_demo = c == DEMO_CHOICE;
            let lives = sim.lives;
            let mut lost = lost;
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
            // the player's line is judged by what happens, not by the way's floors: a ball waiting on a lift or riding
            // a module stands on no floor of the way, and that is no loss; only a lost ball, or one falling fast, is
            if from_demo {
                let gone = s.lives < lives || s.is_dying() || s.game_over;
                let falling = !s.ball.on_ground && s.ball.velocity.y < -12.0 && !way_down;
                lost = gone || falling;
            }
            // made over into a kind neither the way's next transformer nor the last one makes (a flat transformer
            // rolled over on the way): as bad as a loss
            let kinds = |i: usize| self.gates.get(i).map(|g| g.2);
            if s.ball.kind != sim.ball.kind
                && self.gate > 0
                && kinds(self.gate) != Some(s.ball.kind)
                && kinds(self.gate - 1) != Some(s.ball.kind)
            {
                lost = true;
            }
            let score = if lost {
                f32::NEG_INFINITY
            } else if s.finished {
                f32::INFINITY
            } else {
                let v = s.ball.velocity.length();
                // (the pace for the floor, not above a careful zone's limit)
                let cruise = |p: Vec3| {
                    let c = self.route.cruise(p, self.ride);
                    zone.cap.map_or(c, |cap| c.min(cap))
                };
                self.progress(s.ball.position)
                    - self.blocking(&s)
                    - if uphill && zone.cap.is_none() {
                        0.0
                    } else {
                        pace.fast
                            * (v - if steady || zone.cap.is_some() { cruise(s.ball.position) } else { pace.calm })
                                .max(0.0)
                    }
                    // a careful zone: over its limit costs a lot more
                    - zone.cap.map_or(0.0, |cap| CAREFUL_FAST * (v - cap).max(0.0))
                    - if steady {
                        pace.slow * (0.5 * cruise(s.ball.position) - v).max(0.0)
                    } else {
                        0.0
                    }
                    // a flow zone: slower than most of the pace costs more than a wait is worth (a choice that loses
                    // the ball is still never taken)
                    - if zone.flow {
                        FLOW_SLOW * (FLOW_PACE * cruise(s.ball.position) - v).max(0.0)
                    } else {
                        0.0
                    }
                    - edge
                    - if from_demo { 0.0 } else { pace.switch * ((c ^ self.choice) & ARROWS).count_ones() as f32 }
                    // the player's own line, when as good, is taken: it knows what the search does not
                    + if from_demo { DEMO_BONUS } else { 0.0 }
                    + self.jitter(sim.tick)
            };
            if std::env::var_os("AUTOPILOT_SCORES").is_some() {
                eprintln!(
                    "t{} at {} choice {c:02x}: {score:.2} lost {lost} -> {:?}",
                    sim.tick, self.at, s.ball.position
                );
            }
            if score > best.0 {
                best = (score, c);
                demo_won = from_demo;
            }
        }
        // AUTOPILOT_DEMO: at each decision, which of the followed player's ticks matched and whether it was taken
        if std::env::var_os("AUTOPILOT_DEMO").is_some() {
            eprintln!(
                "demo t{} ball {:?} v {:.1} matched {matched:?} won {demo_won}",
                sim.tick,
                sim.ball.position,
                sim.ball.velocity.length()
            );
        }
        self.mode = if demo_won {
            "the player's keys"
        } else {
            "way search"
        };
        if demo_won && let Some(k) = matched {
            self.demo_run = Some((k, sim.tick));
            self.choice = 0;
            return self
                .demo
                .as_ref()
                .map_or(0, |d| d.input(k, crate::hints::Facing::of(&sim.camera)));
        }
        self.choice = best.1;
        self.choice
    }
}

/// Each choice tried on a copy of the run: held for `hold` ticks (the player's keys from tick `k` of the followed run
/// for `DEMO_CHOICE`), then `COAST` with no arrows, and on while in the air (up to two seconds): the copy, whether
/// the ball was lost (fell below `low`, or came down on no floor), and what its line off the middle costs. The
/// copies are run in threads when AUTOPILOT_PARALLEL is set; the results are in the order of the choices, so the run
/// is the same.
// the rollout takes the whole situation by value; a struct for one call site would only rename the arguments
#[allow(clippy::too_many_arguments)]
fn rollouts(
    sim: &Sim,
    route: &Route,
    demo: Option<(usize, &Demo)>,
    choices: &[u8],
    hold: u64,
    low: f32,
    line: Option<&[Vec3]>,
    pace: Pace,
) -> Vec<(Sim, bool, f32)> {
    let one = |c: u8| {
        let mut s = sim.clone();
        let lives = s.lives;
        let mut lost = false;
        // (off the middle, nearer an edge than `SAFE`: summed over the samples on a floor; off the way's line in a
        // careful zone, over all samples)
        let (mut off, mut near, mut samples) = (0.0, 0.0, 0u32);
        let (mut astray, mut line_samples) = (0.0, 0u32);
        for t in 0..hold + COAST {
            let input = match demo {
                Some((k, d)) if c == DEMO_CHOICE => {
                    d.input(k + t as usize, crate::hints::Facing::of(&s.camera))
                }
                _ if t < hold => c,
                _ => 0,
            };
            s.step(input);
            if t % 6 == 5
                && let Some(l) = line
            {
                astray += off_line(s.ball.position - Vec3::Y * crate::sim::BALL_RADIUS, l);
                line_samples += 1;
            }
            if t % 6 == 5
                && s.ball.on_ground
                && let Some((o, n)) = route.off_middle(s.ball.position)
            {
                off += o;
                // (`n` is how much nearer than `SAFE`; a style may shun the edge only nearer)
                near += (n - (SAFE - pace.safe)).max(0.0);
                samples += 1;
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
        // (a ball an updraft holds up is judged where it floats)
        while !lost && !s.finished && !s.ball.on_ground && flight < 120 && !s.in_updraft() {
            s.step(0);
            flight += 1;
            if s.lives < lives || s.is_dying() || s.game_over || s.ball.position.y < low {
                lost = true;
            }
        }
        // still falling after two seconds, or down on no floor of the level: lost
        if !lost
            && !s.finished
            && !s.in_updraft()
            && (!s.ball.on_ground || route.clearance(s.ball.position) <= 0.0)
            && flight > 0
        {
            lost = true;
        }
        let middle = if line.is_some() {
            pace.middle * CAREFUL_MIDDLE
        } else {
            pace.middle
        };
        let edge = (middle * off + pace.edge * near) / samples.max(1) as f32
            + CAREFUL_LINE * astray / line_samples.max(1) as f32;
        (s, lost, edge)
    };
    if std::env::var_os("AUTOPILOT_PARALLEL").is_none() {
        return choices.iter().map(|&c| one(c)).collect();
    }
    std::thread::scope(|scope| {
        let one = &one;
        let handles: Vec<_> = choices
            .iter()
            .map(|&c| scope.spawn(move || one(c)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    })
}

/// How far the floor point `f` is from the way's line `l` (its segments).
fn off_line(f: Vec3, l: &[Vec3]) -> f32 {
    l.windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let ab = b - a;
            let len2 = ab.length_squared();
            let t = if len2 > 1e-9 {
                ((f - a).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            f.distance(a + ab * t)
        })
        .fold(f32::INFINITY, f32::min)
        .min(10.0)
}

/// Input `i` kept under the speed limit `cap` (a careful zone): over it, the arrows that push along the way the ball
/// rolls are let go (those across it stay, to keep the line); well over it, the ball is braked. A turn of the view is
/// left as it is.
fn under_cap(i: u8, sim: &Sim, cap: f32) -> u8 {
    let v = Vec3::new(sim.ball.velocity.x, 0.0, sim.ball.velocity.z);
    let speed = v.length();
    if i & input::SHIFT != 0 || speed <= cap {
        return i;
    }
    let q = crate::hints::Facing::of(&sim.camera);
    if speed > cap * CAREFUL_BRAKE {
        return (i & !ARROWS) | crate::hints::arrows(-v, q);
    }
    let dir = v / speed;
    let mut out = i & !ARROWS;
    for bit in [input::UP, input::DOWN, input::LEFT, input::RIGHT] {
        if i & bit != 0 && crate::hints::world_push(bit, q).dot(dir) <= 0.2 {
            out |= bit;
        }
    }
    out
}

/// A state of the search: the copy of the run, the held choices that led to it (until which tick each), its score.
type Node = (Sim, Vec<(u64, u8)>, f32);

/// The beam search when stuck: how long each choice is held, how many are chained, how many states are kept at each
/// depth, the coast before a state is judged, and how many threads share the tries (with AUTOPILOT_PARALLEL).
const SEARCH_HOLDS: [u64; 3] = [20, 45, 90];
const SEARCH_DEPTH: usize = 4;
const SEARCH_BEAM: usize = 6;
const SEARCH_COAST: u64 = 40;
const SEARCH_THREADS: usize = 10;

/// How often a state is saved to go back to (two seconds), and how many are kept (thinned out as they age).
const SAVE_EVERY: u64 = 120;
const SAVES_KEPT: usize = 24;
/// Standing still this long (20 s) is a failure to go back from, like a lost ball.
pub const REWIND_STILL: u64 = 1200;
/// A state saved less than this before a failure (a second) is not gone back to: the ball may be doomed there.
const SAVE_MARGIN: u64 = 60;
/// A failure is not gone back from further than this (a minute): a cause that early is a way to find anew, not a try.
const BACK_MOST: u64 = 3600;
/// No state is saved in the first seconds of a run (the ball is still being brought in).
const SAVE_FROM: u64 = 300;
/// Tries at one place before a failure there is taken as it comes (the ball lost, the run given up), and how much
/// further along the way (points) a failure must come to count as a new place.
const TRIES_AT_PLACE: u32 = 30;
const NEW_PLACE: usize = 6;
/// Standing still at the same spot (within this many units) this many times more, once a try from there has gone back
/// as far as the saves reach (the oldest within `BACK_MOST`, or the first since the sector began), is not tried again:
/// the way is blocked there, and going back once more only shows the same wall.
const STUCK_SPOT: f32 = 2.0;
const STUCKS_AT_SPOT: u32 = 3;
/// Nor is losing the ball at the same spot this many times more, on the same condition: what fails there was decided
/// earlier than any going back reaches, and the ball is taken as lost (the sector again, from its start). Before that
/// the tries go back twice as far every third time, as at any place: a run-up two or four saves back is often what
/// gets through (counting the tries alone gave up a level's hard stretch just before the try that passed it).
const LOST_SPOT: f32 = 12.0;
const LOSTS_AT_SPOT: u32 = 6;

/// Going back in the run: the simulation is deterministic and can be copied, so instead of losing a ball, or standing
/// still where its way does not get on, the test player goes back to a state saved a little before and tries again,
/// varied (Pilot::perturb). Tries at one place go back further the more there have been: the cause may lie earlier
/// (a run-up too slow, a bad line into a bend). The run kept is the one that got through; its replay plays as any.
pub struct Rewinds<T> {
    saves: Vec<(u64, T)>,
    /// The furthest place (sector, way point) a failure came at, and how many tries failed there.
    front: (usize, usize),
    tries: u32,
    total: u32,
    max_total: u32,
    /// Where the ball stood still or was lost before (and which), how many times, and whether a try from there has
    /// already gone back as far as the saves reach.
    spots: Vec<(Vec3, bool, u32, bool)>,
}

impl<T: Clone> Rewinds<T> {
    pub fn new(max_total: u32) -> Rewinds<T> {
        Rewinds {
            saves: Vec::new(),
            front: (0, 0),
            tries: 0,
            total: 0,
            max_total,
            spots: Vec::new(),
        }
    }

    /// Whether a state is to be saved now: every `SAVE_EVERY` ticks, the ball not being lost (on the ground or not: on
    /// rails it seldom is).
    pub fn due(&self, sim: &Sim) -> bool {
        self.max_total > 0
            && sim.tick >= SAVE_FROM
            && !sim.is_dying()
            && !sim.finished
            && self
                .saves
                .last()
                .is_none_or(|s| sim.tick >= s.0 + SAVE_EVERY)
    }

    pub fn save(&mut self, tick: u64, state: T) {
        // full: every second one of the older half goes, so the saves reach back further the older they are
        if self.saves.len() >= SAVES_KEPT {
            let half = self.saves.len() / 2;
            let mut i = 0;
            self.saves.retain(|_| {
                i += 1;
                i > half || i % 2 == 1
            });
        }
        self.saves.push((tick, state));
    }

    pub fn total(&self) -> u32 {
        self.total
    }

    /// Whether the run goes back at all (`--rewinds` is not 0).
    pub fn enabled(&self) -> bool {
        self.max_total > 0
    }

    /// One going back of a hint's own (`retry`), counted against the cap: whether it may be taken (`--rewinds 0`
    /// never goes back).
    pub fn take_one(&mut self) -> bool {
        if self.total >= self.max_total {
            return false;
        }
        self.total += 1;
        true
    }

    /// The run went back to `tick` on its own (a hint tried again): the states saved after it were on the failed
    /// course and are not gone back to.
    pub fn drop_after(&mut self, tick: u64) {
        self.saves.retain(|s| s.0 <= tick);
    }

    /// The states saved so far are not gone back to (a stretch driven by hand since: it is kept).
    pub fn forget(&mut self) {
        self.saves.clear();
    }

    /// A failure at tick `at` (the ball lost then, or the way standing still since then) at `place` (sector, way
    /// point): the state to go back to and which try at this place it is, or None when the failure is to be taken.
    /// The saves after the one gone back to are dropped (they were on the failed course).
    /// `spot`: where the ball stood still, or was lost (`lost`); once a try from one spot has gone back as far as the
    /// saves reach, the third standing still there after it, or the sixth loss, is taken as it comes. A failure taken
    /// clears its spot: the next ball comes there from the sector's start, with a history of its own.
    pub fn back(
        &mut self,
        at: u64,
        place: (usize, usize),
        spot: Vec3,
        lost: bool,
    ) -> Option<(T, u32)> {
        let (near, most) = if lost {
            (LOST_SPOT, LOSTS_AT_SPOT)
        } else {
            (STUCK_SPOT, STUCKS_AT_SPOT)
        };
        let i = match self
            .spots
            .iter()
            .position(|s| s.1 == lost && s.0.distance(spot) <= near)
        {
            Some(i) => i,
            None => {
                self.spots.push((spot, lost, 0, false));
                self.spots.len() - 1
            }
        };
        // (counted only once a try from there has gone back as far as the saves reach: before that, going back
        // further is still to be tried)
        if self.spots[i].3 {
            self.spots[i].2 += 1;
        }
        let (n, deepest) = (self.spots[i].2, self.spots[i].3);
        if n >= most && deepest {
            self.spots.swap_remove(i);
            return None;
        }
        if place.0 > self.front.0 || (place.0 == self.front.0 && place.1 > self.front.1 + NEW_PLACE)
        {
            self.front = place;
            self.tries = 0;
        }
        self.tries += 1;
        if self.tries > TRIES_AT_PLACE || self.total >= self.max_total {
            return None;
        }
        let before = self
            .saves
            .iter()
            .rposition(|s| s.0 + SAVE_MARGIN <= at && s.0 + BACK_MOST >= at)?;
        // three tries from the last save before the failure, then three from each one going back twice as far
        let back = match self.tries {
            0..=3 => 0,
            t => 1usize << ((t - 4) / 3).min(10),
        };
        // (not further back than `BACK_MOST` before the failure)
        let oldest = self
            .saves
            .iter()
            .position(|s| s.0 + BACK_MOST >= at)
            .unwrap_or(before)
            .min(before);
        let k = before.saturating_sub(back).max(oldest);
        if k == oldest {
            self.spots[i].3 = true;
        }
        self.saves.truncate(k + 1);
        self.total += 1;
        self.saves.last().map(|s| (s.1.clone(), self.tries))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn raw_style_has_no_costs_but_progress() {
        assert_eq!(Style::parse("raw"), Some(Style::Raw));
        let p = Style::Raw.pace();
        for c in [p.fast, p.slow, p.safe, p.edge, p.middle, p.switch] {
            assert_eq!(c, 0.0);
        }
        // the calm style keeps its costs
        let c = Style::Calm.pace();
        assert!(c.fast > 0.0 && c.slow > 0.0 && c.edge > 0.0 && c.middle > 0.0);
    }

    #[test]
    fn rewinds_give_up_at_the_same_spot() {
        let mut r = Rewinds::<u32>::new(600);
        for t in 0..10 {
            r.save(300 + t * 120, t as u32);
        }
        let spot = Vec3::new(1.0, 2.0, 3.0);
        // (standing still there: three tries from the last save, then from further back)
        for _ in 0..3 {
            assert_eq!(r.back(1500, (1, 10), spot, false).map(|s| s.0), Some(9));
        }
        assert_eq!(
            r.back(1500, (1, 10), spot + Vec3::X, false).map(|s| s.0),
            Some(8)
        );
        // (with one save left, gone back to: three more standing still there, the third taken as it comes)
        let mut one = Rewinds::<u32>::new(600);
        one.save(300, 0);
        assert!(one.back(1500, (1, 10), spot, false).is_some());
        assert!(one.back(1500, (1, 10), spot + Vec3::X, false).is_some());
        assert!(one.back(1500, (1, 10), spot, false).is_some());
        assert!(one.back(1500, (1, 10), spot, false).is_none());
        // (elsewhere, and a lost ball, still go back)
        assert!(
            one.back(1500, (1, 10), spot + Vec3::Z * 10.0, false)
                .is_some()
        );
        assert!(one.back(1500, (1, 10), spot, true).is_some());
    }

    #[test]
    fn losses_at_one_spot_go_back_as_far_as_the_saves_reach_before_one_is_taken() {
        let mut r = Rewinds::<u32>::new(600);
        for t in 0..10 {
            r.save(300 + t * 120, t as u32);
        }
        let spot = Vec3::new(1.0, 2.0, 3.0);
        let mut gone_to = Vec::new();
        while let Some((s, _)) = r.back(
            1500,
            (1, 10),
            spot + Vec3::X * (gone_to.len() % 3) as f32,
            true,
        ) {
            gone_to.push(s);
            assert!(gone_to.len() < 30, "never taken");
        }
        // (more than six tries, going back further every third, the last ones from the oldest save)
        assert!(gone_to.len() > 6, "{gone_to:?}");
        assert_eq!(gone_to.last(), Some(&0), "{gone_to:?}");
        assert!(gone_to.windows(2).all(|w| w[1] <= w[0]), "{gone_to:?}");
        // (the loss taken clears the spot: the next ball lost there is gone back from again)
        r.save(1600, 11);
        assert!(r.back(1800, (2, 0), spot, true).is_some());
    }

    #[test]
    fn a_strip_of_rail_points_runs_along_its_length() {
        // a pair of rails along x, 1.5 apart
        let pts: Vec<Vec3> = (-4..=4)
            .flat_map(|i| {
                [
                    Vec3::new(i as f32 * 1.5, 0.0, -0.75),
                    Vec3::new(i as f32 * 1.5, 0.0, 0.75),
                ]
            })
            .collect();
        let a = rail_axis(&pts).expect("an axis");
        assert!(a.x.abs() > 0.99, "along x: {a:?}");
        assert!(rail_axis(&[Vec3::ZERO]).is_none(), "one point runs nowhere");
    }

    #[test]
    fn progress_grows_along_the_way_and_ignores_what_is_off_it() {
        let points: Vec<Vec3> = (0..10)
            .map(|i| Vec3::new(i as f32 * 2.0, 0.0, 0.0))
            .collect();
        let r = Route {
            points,
            marks: vec![9],
            lifts: Vec::new(),
            floor: HashMap::new(),
            pts: Vec::new(),
            grid: HashMap::new(),
            edges: crate::edges::Edges::of(&[], 0),
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

    /// A pilot on the flat test floor with a straight way east and these hints and zones.
    #[allow(clippy::expect_used)]
    fn flat_pilot(hints: &str, zones: &str) -> (Pilot, Sim) {
        let points: Vec<Vec3> = (0..40)
            .map(|i| Vec3::new(i as f32 * 1.5, 0.0, 0.0))
            .collect();
        pilot_on(points, hints, zones)
    }

    /// The same with the way through `points`.
    #[allow(clippy::expect_used)]
    fn pilot_on(points: Vec<Vec3>, hints: &str, zones: &str) -> (Pilot, Sim) {
        let sim = Sim::new(&crate::sim::tests::flat_level(), crate::sim::BallKind::Wood);
        let r = Route {
            points,
            marks: vec![39],
            lifts: Vec::new(),
            floor: HashMap::new(),
            pts: Vec::new(),
            grid: HashMap::new(),
            edges: crate::edges::Edges::of(&[], 0),
        };
        let mut p = Pilot::new(r, Vec::new());
        p.hints = crate::hints::Runner::new(crate::hints::parse(hints).expect("hints"));
        let (z, notes) = crate::zones::parse(zones);
        assert!(notes.is_empty(), "{notes:?}");
        p.zones = std::sync::Arc::new(z);
        (p, sim)
    }

    /// The inputs of the first `n` ticks the pilot gives, and the fastest the ball rolled over the floor after `from`.
    fn drive(p: &mut Pilot, sim: &mut Sim, n: u64, from: u64) -> (Vec<u8>, f32) {
        let mut inputs = Vec::new();
        let mut fastest = 0.0f32;
        for t in 0..n {
            let i = p.next(sim);
            inputs.push(i);
            sim.step(i);
            if t >= from {
                let v = sim.ball.velocity;
                fastest = fastest.max(Vec3::new(v.x, 0.0, v.z).length());
            }
        }
        (inputs, fastest)
    }

    #[test]
    fn a_flow_zone_leaves_out_a_hints_stop_that_is_not_needed() {
        let hint = "at 0 2 0 r 10 : wait 120 ; push 1 0 60";
        // as written: two seconds standing first
        let (mut p, mut sim) = flat_pilot(hint, "");
        let (i, _) = drive(&mut p, &mut sim, 60, 0);
        assert!(i.iter().all(|&i| i == 0), "the wait is kept outside a zone");
        // in a flow zone the wait goes: pushing at once
        let (mut p, mut sim) = flat_pilot(hint, "flow sector 1 r 10 : 0 2 0");
        let (i, _) = drive(&mut p, &mut sim, 10, 0);
        assert!(
            i.iter().take(3).any(|&i| i != 0),
            "the wait is left out in a flow zone: {i:?}"
        );
        // but not where leaving it out would lose the ball: a push that runs off the floor's edge
        let off = "at 0 2 0 r 10 : wait 120 ; push 1 0 3000";
        let (mut p, mut sim) = flat_pilot(off, "flow sector 1 r 10 : 0 2 0");
        let (i, _) = drive(&mut p, &mut sim, 60, 0);
        assert!(i.iter().all(|&i| i == 0), "a wait the ball needs is kept");
    }

    #[test]
    fn a_careful_zone_keeps_the_ball_under_its_speed() {
        let hint = "at 0 2 0 r 10 : push 1 0 400";
        let (mut p, mut sim) = flat_pilot(hint, "");
        let (_, free) = drive(&mut p, &mut sim, 400, 0);
        assert!(free > 8.0, "pushed freely it runs fast: {free}");
        let (mut p, mut sim) =
            flat_pilot(hint, "careful sector 1 r 20 speed 3 : -50 2 0 ; 150 2 0");
        let (_, kept) = drive(&mut p, &mut sim, 400, 0);
        assert!(kept < 3.0 * 1.3, "kept under the limit: {kept}");
        // and the pilot's own steering too, with no hint
        let (mut p, mut sim) = flat_pilot("", "careful sector 1 r 20 speed 3 : -50 2 0 ; 150 2 0");
        let (_, own) = drive(&mut p, &mut sim, 400, 0);
        assert!(own < 3.0 * 1.3, "its own steering under the limit: {own}");
    }

    #[test]
    fn a_careful_zone_lets_its_limit_go_where_the_ball_needs_momentum() {
        // a hint's press at speed (a climb, rails, a ram) is not held back, though the line is still kept
        let zone = "careful sector 1 r 20 speed 3 : -50 2 0 ; 150 2 0";
        let (mut p, mut sim) = flat_pilot("at 0 2 0 r 10 : press 40 2 0 0 0 8 300", zone);
        let _ = drive(&mut p, &mut sim, 2, 0);
        assert!(
            p.status()
                .contains("limit let go: the hint pushes on at speed"),
            "{}",
            p.status()
        );
        let (_, fast) = drive(&mut p, &mut sim, 200, 0);
        assert!(fast > 3.0 * 1.5, "the press keeps its speed: {fast}");
        // a pass faster than the limit just before such a press is its run-up; a slow press is no reason
        let (mut p, mut sim) = flat_pilot(
            "at 0 2 0 r 10 : pass 10 2 0 8 ; press 40 2 0 0 0 8 300",
            zone,
        );
        let _ = drive(&mut p, &mut sim, 2, 0);
        assert!(
            p.status().contains("limit let go: the hint's run-up"),
            "{}",
            p.status()
        );
        let (mut p, mut sim) = flat_pilot("at 0 2 0 r 10 : press 40 2 0 0 0 2 300", zone);
        let _ = drive(&mut p, &mut sim, 2, 0);
        assert!(!p.status().contains("let go"), "{}", p.status());
        // the way ahead: a gap to cross, a climb steeper than the ball takes by its push
        let way = |rise: f32, gap_at: Option<usize>| {
            let mut x = 0.0;
            (0..40)
                .map(|i| {
                    x += if Some(i) == gap_at { 8.0 } else { 1.5 };
                    Vec3::new(
                        x - 1.5,
                        if i > 5 { (i - 5) as f32 * rise } else { 0.0 },
                        0.0,
                    )
                })
                .collect::<Vec<_>>()
        };
        let bare = |points: Vec<Vec3>| pilot_on(points, "", "");
        let (p, sim) = bare(way(0.0, None));
        assert_eq!(p.momentum(&sim, Some(3.0)), None, "a flat way");
        let (p, sim) = bare(way(0.0, Some(12)));
        assert_eq!(p.momentum(&sim, Some(3.0)), Some("a gap to cross ahead"));
        // 0.45 in 1.5 is about 17 degrees: too steep for a stone ball at the limit, not for a wooden one
        let (p, mut sim) = bare(way(0.45, None));
        assert_eq!(p.momentum(&sim, Some(3.0)), None, "wood climbs it");
        sim.ball.kind = crate::sim::BallKind::Stone;
        assert_eq!(
            p.momentum(&sim, Some(3.0)),
            Some("a climb that needs a run-up ahead")
        );
        // there a flow zone's pace is let go too: backing off for the run-up is no dawdling
        let (mut p, mut sim) = pilot_on(way(0.45, None), "", "flow sector 1 r 50 : 0 2 0");
        sim.ball.kind = crate::sim::BallKind::Stone;
        let _ = p.next(&sim);
        assert!(
            p.status()
                .contains("[pace let go: a climb that needs a run-up ahead]"),
            "{}",
            p.status()
        );
    }

    #[test]
    fn a_hint_whose_ball_ends_a_step_well_below_its_point_is_off_course() {
        for (y, off) in [(2.0, false), (8.0, true)] {
            let (mut p, mut sim) =
                flat_pilot(&format!("at 0 2 0 r 10 : pass 6 {y} 0 5 40 ; wait 200"), "");
            let _ = drive(&mut p, &mut sim, 120, 0);
            assert_eq!(p.hints.at().map(|a| a.1), Some(1), "on the wait");
            assert_eq!(p.hints.off_course(), off, "a pass to height {y}");
        }
    }

    #[test]
    fn a_hints_check_fails_it_when_not_met_and_a_retry_hint_is_seen_before_it_fires() {
        for (check, failed) in [("check 6 2 0 3 60", false), ("check 30 2 0 1 30", true)] {
            let (mut p, mut sim) = flat_pilot(
                &format!("at 0 2 0 r 10 retry 3 : pass 6 2 0 5 40 ; {check} ; wait 200"),
                "",
            );
            // before its first tick the hint is about to fire, and is marked to be tried again
            assert_eq!(p.hints.would_fire(&sim), Some(0));
            assert_eq!(p.hints.retry_of(0), 3);
            let _ = drive(&mut p, &mut sim, 150, 0);
            assert_eq!(p.hints.would_fire(&sim), None, "fired once a life");
            assert_eq!(p.hints.take_mark(), None, "no mark in it");
            if failed {
                assert_eq!(p.hints.take_failed(), Some(0));
                assert_eq!(p.hints.running_hint(), None, "a failed hint ends");
            } else {
                assert_eq!(p.hints.take_failed(), None);
                assert_eq!(p.hints.at(), Some((0, 2)), "on the wait after the check");
            }
            assert_eq!(p.hints.take_failed(), None, "taken once");
        }
    }

    #[test]
    fn a_retry_step_marks_where_the_hint_is_tried_again_from() {
        let (mut p, mut sim) = flat_pilot("at 0 2 0 r 10 : wait 5 ; retry 4 ; wait 50", "");
        let _ = drive(&mut p, &mut sim, 3, 0);
        assert_eq!(p.hints.take_mark(), None, "not there yet");
        let _ = drive(&mut p, &mut sim, 5, 0);
        assert_eq!(p.hints.take_mark(), Some((0, 4, 0.3)));
        // on at once: the step after it runs
        assert_eq!(p.hints.at(), Some((0, 2)));
        assert_eq!(p.hints.take_mark(), None, "taken once");
    }

    #[test]
    fn a_hints_own_tries_count_against_the_cap_and_drop_the_saves_after() {
        let mut r: Rewinds<u32> = Rewinds::new(2);
        assert!(r.enabled());
        for t in [400, 520, 640] {
            r.save(t, t as u32);
        }
        r.drop_after(520);
        assert_eq!(
            r.saves.iter().map(|s| s.0).collect::<Vec<_>>(),
            vec![400, 520]
        );
        assert!(r.take_one() && r.take_one());
        assert!(!r.take_one(), "capped");
        assert_eq!(r.total(), 2);
        let mut never: Rewinds<u32> = Rewinds::new(0);
        assert!(!never.enabled() && !never.take_one());
    }

    #[test]
    fn a_ball_climbs_by_its_push_as_the_table_says() {
        use crate::sim::BallKind;
        // pushed uphill from a standstill for ten seconds on a plane tilted so: its pace up it then
        let pace = |kind: BallKind, deg: f32| {
            let mut level = crate::sim::tests::flat_level();
            let t = deg.to_radians().tan();
            for p in &mut level.meshes[0].positions {
                p[1] = p[0] * t;
            }
            let mut sim = Sim::new(&level, kind);
            for _ in 0..600 {
                sim.step(crate::hints::arrows(
                    Vec3::X,
                    crate::hints::Facing::of(&sim.camera),
                ));
            }
            sim.ball.velocity.x
        };
        for kind in [
            BallKind::Stone,
            BallKind::Wood,
            BallKind::Paper,
            BallKind::Sticky,
        ] {
            let at = steepest_climb(kind, 4.5);
            let (gentler, steeper) = (pace(kind, at - 1.5), pace(kind, at + 1.5));
            assert!(
                gentler > 4.5 && steeper < 4.5,
                "{kind:?} at {at}: {gentler} {steeper}"
            );
            let back = pace(kind, steepest_climb(kind, 0.0) + 1.5);
            if kind == BallKind::Sticky {
                assert!(back > -0.1 && back < 0.5, "the sticky ball clings: {back}");
            } else {
                assert!(back < 0.0, "{kind:?} rolls back");
            }
        }
    }

    /// A way through `points` whose floor points are rails where `rail` says (else a floor 10 wide).
    fn railed_route(points: Vec<Vec3>, rail: impl Fn(usize) -> bool) -> Route {
        let mut floor: HashMap<(i32, i32), Vec<FloorPoint>> = HashMap::new();
        for (k, &at) in points.iter().enumerate() {
            let cell = ((at.x / CELL).floor() as i32, (at.z / CELL).floor() as i32);
            let rail = rail(k);
            floor.entry(cell).or_default().push(FloorPoint {
                at,
                coarse: 3.0,
                rail,
                half: if rail { 0.5 } else { 5.0 },
            });
        }
        let marks = vec![points.len() - 1];
        Route {
            points,
            marks,
            lifts: Vec::new(),
            floor,
            pts: Vec::new(),
            grid: HashMap::new(),
            edges: crate::edges::Edges::of(&[], 0),
        }
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn a_mark_on_one_level_teaches_the_same_kind_of_place_on_another() {
        use crate::sim::BallKind;
        // level A: rails east from x 50, ending at their tenth point; the watcher marked the stretch before the end
        let a = railed_route(
            (0..30)
                .map(|i| Vec3::new(50.0 + 1.5 * i as f32, 0.0, 40.0))
                .collect(),
            |k| k < 10,
        );
        let pa = Pilot::new(a, Vec::new());
        let samples: Vec<_> = (3..9)
            .map(|k| {
                let p = pa.route.points[k] + Vec3::Y * crate::sim::BALL_RADIUS;
                pa.situation(p, pa.nearest_point(p), BallKind::Wood)
            })
            .collect();
        let rule =
            crate::learned::learn(&samples, crate::zones::Kind::Flow, None, "level A".into())
                .expect("a rule from the rail's end");
        assert_eq!(
            rule.line(),
            "flow when surface=rail ahead=rail-end kind=wood   # level A"
        );
        let table = std::sync::Arc::new(crate::learned::Learned { rules: vec![rule] });
        // level B: other rails, north through the start, ending a few points on; a hint waits there
        let b = || {
            railed_route(
                (0..30)
                    .map(|i| Vec3::new(0.0, 0.0, -9.0 + 1.5 * i as f32))
                    .collect(),
                |k| k < 10,
            )
        };
        let hint = crate::hints::parse("at 0 2 0 r 10 : wait 120 ; push 0 1 60").expect("hint");
        let run = |learned: Option<std::sync::Arc<crate::learned::Learned>>| {
            let mut sim = Sim::new(&crate::sim::tests::flat_level(), BallKind::Wood);
            let mut p = Pilot::new(b(), Vec::new());
            p.hints = crate::hints::Runner::new(hint.clone());
            p.take_up(&sim);
            if let Some(l) = learned {
                p.learned = l;
            }
            let (i, _) = drive(&mut p, &mut sim, 30, 0);
            (i, p.status())
        };
        let (plain, _) = run(None);
        assert!(
            plain.iter().all(|&i| i == 0),
            "not taught: the wait is kept"
        );
        let (taught, status) = run(Some(table.clone()));
        assert!(
            taught.iter().take(3).any(|&i| i != 0),
            "taught on level A: no dawdling on level B: {taught:?}"
        );
        assert!(
            status.contains("learned: rail, rail-end ahead, wood → don't dawdle"),
            "{status}"
        );
        // a level's own zone comes first: a careful zone there and no flow from the table
        let mut sim = Sim::new(&crate::sim::tests::flat_level(), BallKind::Wood);
        let mut p = Pilot::new(b(), Vec::new());
        p.hints = crate::hints::Runner::new(hint.clone());
        p.take_up(&sim);
        p.learned = table;
        p.zones = std::sync::Arc::new(crate::zones::parse("careful sector 1 r 10 : 0 2 0").0);
        let (own, _) = drive(&mut p, &mut sim, 30, 0);
        assert!(
            own.iter().all(|&i| i == 0),
            "the level's zone first: {own:?}"
        );
    }
}
