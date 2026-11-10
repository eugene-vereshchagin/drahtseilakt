// Level statistics (`--level-stats DIR`): measures the original levels for designing our own — what the ball rolls
// over, sector by sector, along the way a finished run took (the runs in hints/), or, for a level without one, along
// the way the test player finds through its plan. Every number here is computed from the level's objects, its floors'
// geometry and the run; the judgements (hidden ways, tricks) are written by hand beside it
// (hints/level-stats-manual.csv) and joined in by scripts/level-stats.py, which also writes the report.
//
// The way is sampled every `STEP` units. At each sample the floor is read across the way, to each side: where it
// drops away, where something rises from it (a low rim or a wall), whether it curves up as a trough does. From the two
// sides and the width the sample gets one class of track, from easy to hard: a corridor (walls), a wide floor, a
// trough, posts, curbs, an open floor, a narrow open path, rails, a single rail, rails ridden on their side; or the
// ball is on a module's moving part, or in the air.

use crate::Options;
use crate::assets::{Data, Source};
use crate::sim::{BALL_RADIUS, BallKind, Event};
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// One tick of a replayed run.
#[derive(Clone, Copy, Debug)]
pub struct TickRec {
    pub pos: Vec3,
    pub ground: bool,
    /// As the game shows it (1 is the first).
    pub sector: usize,
    pub kind: BallKind,
    pub updraft: bool,
    pub held: bool,
    pub dying: bool,
}

/// A level as a run loads it, and the ball's way through it (app::stats_run).
pub struct Run {
    pub level: level::Level,
    pub modules: crate::scene::Modules,
    pub ticks: Vec<TickRec>,
    /// Each event with its tick and the sector it came in.
    pub events: Vec<(u64, Event, usize)>,
    /// The loose things (boxes, balls) and how far each got from where it began.
    pub moved: Vec<(String, f32)>,
    pub finished: bool,
    pub begun_elsewhere: bool,
    /// The test player's way through a plan's objects (floor points), when there is no run.
    pub route: Option<Vec<Vec3>>,
    /// The checkpoints, then the end.
    pub checkpoints: Vec<Vec3>,
}

/// The way is sampled this often (units).
const STEP: f32 = 0.5;
/// How far across the way the floor is read to each side.
const ACROSS: f32 = 12.0;
/// A rise from the floor at least this high, this near its foot, is a wall; a lower one a curb.
const WALL_HIGH: f32 = 1.6;
const RIM_LOW: f32 = 0.35;
/// A side rising at least this high (above the ball's middle) holds it as a wall does.
const HIGH: f32 = 2.5;
/// A gentle side at least this high makes a trough the ball sits in; a lower one is a low rim.
const TROUGH_HIGH: f32 = 1.25;
/// How far round the ball other rails are looked for (a pair's other rail).
const RAIL_REACH: f32 = 4.6;
/// Width of an open floor under which it is narrow (one and a half balls), and from which it is wide.
const NARROW: f32 = 6.0;
const WIDE: f32 = 12.0;
/// Turns: the way's heading compared over this length; over `SHARP` degrees a sharp turn, over `BEND` a bend.
const TURN_LEN: f32 = 8.0;
const SHARP: f32 = 60.0;
const BEND: f32 = 25.0;
/// A slope steeper than this (rise over run) is a ramp; a ramp at least `RAMP_LEN` long counts.
const RAMP: f32 = 0.2;
const RAMP_LEN: f32 = 2.0;
/// The ball in the air at least this long (ticks) is a flight.
const FLIGHT_TICKS: usize = 15;
/// A loose thing moved further than this was used (pushed, rammed).
const MOVED: f32 = 2.0;

/// The classes of track, easiest first (the report's order), with their CSV names.
pub const CLASSES: [&str; 12] = [
    "corridor", "wide", "trough", "posts", "curbs", "open", "narrow", "rails", "monorail",
    "siderail", "module", "air",
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Floor,
    Rail,
    Stopper,
    Module,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Side {
    /// The floor drops away this far out.
    Open(f32),
    /// A steep rise this far out, this high (within 3 units of its foot).
    Wall(f32, f32),
    /// A gentle rise (a trough's or a sloped rim's side), this far out, this high.
    Slope(f32, f32),
    /// No edge within `ACROSS`.
    Far,
}

impl Side {
    fn dist(self) -> f32 {
        match self {
            Side::Open(d) | Side::Wall(d, _) | Side::Slope(d, _) => d,
            Side::Far => ACROSS,
        }
    }
    /// How high the side rises (0: it drops away, or no edge).
    fn height(self) -> f32 {
        match self {
            Side::Wall(_, h) | Side::Slope(_, h) => h,
            _ => 0.0,
        }
    }
}

/// The level's physical triangles by cell, for rays and nearness.
struct Geo {
    tris: Vec<([Vec3; 3], Kind)>,
    cells: HashMap<(i32, i32), Vec<u32>>,
}

const CELL: f32 = 4.0;

fn cell(v: f32) -> i32 {
    (v / CELL).floor() as i32
}

impl Geo {
    fn of(level: &level::Level, modules: &crate::scene::Modules) -> Geo {
        let mut tris = Vec::new();
        let add = |tris: &mut Vec<([Vec3; 3], Kind)>, mesh: &level::Mesh, m: Mat4, kind: Kind| {
            for part in &mesh.parts {
                for t in part.indices.as_chunks::<3>().0 {
                    let p = |i: u32| {
                        mesh.positions
                            .get(i as usize)
                            .map(|&v| m.transform_point3(Vec3::from(v)))
                    };
                    if let (Some(a), Some(b), Some(c)) = (p(t[0]), p(t[1]), p(t[2])) {
                        tris.push(([a, b, c], kind));
                    }
                }
            }
        };
        for g in &level.groups {
            let kind = match g.name.to_ascii_lowercase().as_str() {
                "phys_floors" => Kind::Floor,
                "phys_floorrails" => Kind::Rail,
                "phys_floorstopper" => Kind::Stopper,
                _ => continue,
            };
            for &i in &g.members {
                if let Some(o) = level.objects.get(i)
                    && let Some(mesh) = o.mesh.and_then(|k| level.meshes.get(k))
                {
                    add(&mut tris, mesh, Mat4::from_cols_array_2d(&o.matrix), kind);
                }
            }
        }
        // modules as they stand at the start: the moving ones (P_Modul_NN) are their own class; reset points,
        // the finish, transformers and domes are floor; loose things and pickups are not ground
        for i in 0..level.objects.len() {
            let Some(name) = crate::scene::module_of(level, i) else {
                continue;
            };
            if ["P_Ball", "P_Box", "P_Extra"]
                .iter()
                .any(|k| name.starts_with(k))
            {
                continue;
            }
            let kind = if name.starts_with("P_Modul") {
                Kind::Module
            } else {
                Kind::Floor
            };
            let (Some((_, module)), Some(o)) = (modules.get(name), level.objects.get(i)) else {
                continue;
            };
            let place = Mat4::from_cols_array_2d(&o.matrix);
            for mo in &module.objects {
                if let Some(mesh) = mo.mesh.and_then(|k| module.meshes.get(k)) {
                    add(
                        &mut tris,
                        mesh,
                        place * Mat4::from_cols_array_2d(&mo.matrix),
                        kind,
                    );
                }
            }
        }
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (k, (t, _)) in tris.iter().enumerate() {
            let (lo, hi) = (t[0].min(t[1]).min(t[2]), t[0].max(t[1]).max(t[2]));
            let (x0, x1, z0, z1) = (cell(lo.x), cell(hi.x), cell(lo.z), cell(hi.z));
            // (a triangle spanning more than a few hundred cells is a sky or a ground plane: no track)
            if (x1 - x0 + 1) * (z1 - z0 + 1) > 400 {
                continue;
            }
            for x in x0..=x1 {
                for z in z0..=z1 {
                    cells.entry((x, z)).or_default().push(k as u32);
                }
            }
        }
        Geo { tris, cells }
    }

    /// The triangles in the cells a box of `lo`..`hi` (in x and z) covers.
    fn within(&self, lo: Vec3, hi: Vec3) -> impl Iterator<Item = &([Vec3; 3], Kind)> {
        let (x0, x1, z0, z1) = (cell(lo.x), cell(hi.x), cell(lo.z), cell(hi.z));
        (x0..=x1)
            .flat_map(move |x| (z0..=z1).map(move |z| (x, z)))
            .filter_map(|c| self.cells.get(&c))
            .flatten()
            .filter_map(|&k| self.tris.get(k as usize))
    }

    /// The nearest hit of a ray from `o` along `d` (a unit vector) within `max`, among the triangles `take` keeps:
    /// how far, the hit triangle's normal (facing against the ray) and its kind.
    fn ray(
        &self,
        o: Vec3,
        d: Vec3,
        max: f32,
        take: impl Fn(Kind) -> bool,
    ) -> Option<(f32, Vec3, Kind)> {
        let e = o + d * max;
        let mut best: Option<(f32, Vec3, Kind)> = None;
        for (t, kind) in self.within(o.min(e), o.max(e)) {
            if !take(*kind) {
                continue;
            }
            if let Some(dist) = hit(t, o, d)
                && dist <= max
                && best.is_none_or(|b| dist < b.0)
            {
                let mut n = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
                if n.dot(d) > 0.0 {
                    n = -n;
                }
                best = Some((dist, n, *kind));
            }
        }
        best
    }

    /// The nearest points of the triangles within `r` of `c`, with their kinds.
    fn near(&self, c: Vec3, r: f32) -> Vec<(Vec3, Kind)> {
        let v = Vec3::splat(r);
        self.within(c - v, c + v)
            .map(|(t, k)| (crate::collide::closest_point(t, c), *k))
            .filter(|(p, _)| p.distance(c) <= r)
            .collect()
    }
}

/// Möller–Trumbore: how far along `d` from `o` the ray meets the triangle, either face.
fn hit(t: &[Vec3; 3], o: Vec3, d: Vec3) -> Option<f32> {
    let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-7 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - t[0];
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let dist = e2.dot(q) * inv;
    (dist >= 0.0).then_some(dist)
}

fn statics(k: Kind) -> bool {
    k != Kind::Module
}

/// One sample of the way.
#[derive(Clone, Debug)]
struct Sample {
    pos: Vec3,
    sector: usize,
    /// The class (an index into `CLASSES`).
    class: usize,
    /// The floor's width across the way (floors), or the rails' spread (rails).
    width: f32,
    sides: [Option<Side>; 2],
    ground: bool,
    updraft: bool,
    kind: BallKind,
}

/// What the floor is like across the way at the ball's centre `c`, heading `fwd`: the class, the width, the sides.
fn classify(geo: &Geo, c: Vec3, fwd: Vec3) -> (usize, f32, [Option<Side>; 2]) {
    let class = |n: &str| CLASSES.iter().position(|x| *x == n).unwrap_or(0);
    let contacts = geo.near(c, BALL_RADIUS + 0.35);
    if contacts.is_empty() {
        return (class("air"), 0.0, [None, None]);
    }
    let nearest = contacts
        .iter()
        .min_by(|a, b| a.0.distance(c).total_cmp(&b.0.distance(c)))
        .map(|x| x.1);
    if nearest == Some(Kind::Module) {
        return (class("module"), 0.0, [None, None]);
    }
    let right = Vec3::new(-fwd.z, 0.0, fwd.x);
    let rails: Vec<Vec3> = contacts
        .iter()
        .filter(|x| x.1 == Kind::Rail)
        .map(|x| x.0)
        .collect();
    if !rails.is_empty() {
        let up: Vec<f32> = rails
            .iter()
            .map(|p| (c - *p).normalize_or_zero().y)
            .collect();
        let vspread = rails.iter().map(|p| p.y).fold(f32::MIN, f32::max)
            - rails.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        // the rails beside the ball at its height or under it, further out: a pair is two of them a ball apart
        // (a way's floor point lies on one of a pair, the ball itself between them)
        let lat: Vec<f32> = geo
            .near(c, RAIL_REACH)
            .into_iter()
            .filter(|x| x.1 == Kind::Rail && x.0.y <= c.y + 0.5 && x.0.y >= c.y - BALL_RADIUS - 1.0)
            .map(|x| (x.0 - c).dot(right))
            .collect();
        let spread = lat.iter().copied().fold(f32::MIN, f32::max)
            - lat.iter().copied().fold(f32::MAX, f32::min);
        // no rail holds the ball from below: it rides them on their side (a pair one above the other, a steep
        // or vertical rail, kept on by its speed)
        let carried = up.iter().copied().fold(f32::MIN, f32::max);
        let name = if carried < 0.5 || vspread > 1.5 {
            "siderail"
        } else if spread > 2.0 {
            "rails"
        } else {
            "monorail"
        };
        return (class(name), spread, [None, None]);
    }
    // the floor under the ball
    let floor_y = geo
        .ray(c, -Vec3::Y, BALL_RADIUS + 1.0, statics)
        .map_or(c.y - BALL_RADIUS, |h| c.y - h.0);
    let base = Vec3::new(c.x, floor_y, c.z);
    // how high a rise beginning `from` out along `dir` gets within 3 units
    let rim = |dir: Vec3, from: f32| -> f32 {
        let mut top = 0.0f32;
        let mut d = from + 0.25;
        while d < from + 3.0 {
            let q = base + dir * d + Vec3::Y * 3.0;
            if let Some(h) = geo.ray(q, -Vec3::Y, 4.5, statics) {
                top = top.max(3.0 - h.0);
            }
            d += 0.25;
        }
        top
    };
    let side = |s: f32| -> Side {
        let dir = right * s;
        let low = geo.ray(base + Vec3::Y * RIM_LOW, dir, ACROSS, statics);
        let high = geo.ray(base + Vec3::Y * WALL_HIGH, dir, ACROSS, statics);
        let mut d = STEP / 2.0;
        while d < ACROSS {
            if low.is_some_and(|l| d >= l.0) {
                break;
            }
            let q = base + dir * d + Vec3::Y * RIM_LOW;
            if geo.ray(q, -Vec3::Y, RIM_LOW + 1.0, statics).is_none() {
                return Side::Open(d);
            }
            d += STEP / 2.0;
        }
        match low {
            Some(l) => {
                // (a rise the high ray meets just beyond its foot is at least that high)
                let h = rim(dir, l.0).max(if high.is_some_and(|h| h.0 <= l.0 + 1.5) {
                    WALL_HIGH
                } else {
                    0.0
                });
                if l.1.y.abs() > 0.45 {
                    Side::Slope(l.0, h)
                } else {
                    Side::Wall(l.0, h)
                }
            }
            None => Side::Far,
        }
    };
    let (l, r) = (side(-1.0), side(1.0));
    // a side's rise: a wall or railing (steep, at least `WALL_HIGH`; or any as high as the ball's middle), a
    // trough's side (gentle, from `TROUGH_HIGH`), or a low rim (a curb: the original's usual low border)
    let high = |x: Side| match x {
        Side::Wall(_, h) => h >= WALL_HIGH,
        Side::Slope(_, h) => h >= HIGH,
        _ => false,
    };
    let sloped = |x: Side| matches!(x, Side::Slope(_, h) if (TROUGH_HIGH..HIGH).contains(&h));
    let curb = |x: Side| x.height() > 0.0 && !high(x) && !sloped(x);
    let width = l.dist() + r.dist();
    let open = matches!(l, Side::Open(_)) || matches!(r, Side::Open(_));
    let name = if open {
        if width < NARROW {
            "narrow"
        } else if width < WIDE {
            "open"
        } else {
            "wide"
        }
    } else if high(l) && high(r) {
        "corridor"
    } else if (sloped(l) || sloped(r)) && (sloped(l) || high(l)) && (sloped(r) || high(r)) {
        "trough"
    } else if curb(l) || curb(r) {
        "curbs"
    } else if high(l) || high(r) {
        "corridor"
    } else {
        "wide"
    };
    (class(name), width, [Some(l), Some(r)])
}

/// The ticks of a run's tries that ended with the ball lost: from the try's beginning (its checkpoint or new ball) to
/// the loss.
fn wasted(run: &Run) -> Vec<bool> {
    let mut wasted = vec![false; run.ticks.len()];
    let mut began = 0usize;
    for &(t, e, _) in &run.events {
        let t = (t as usize).saturating_sub(1);
        match e {
            Event::Checkpoint(_) | Event::NewBall => began = t,
            Event::Died => {
                for w in wasted.iter_mut().take(t + 1).skip(began) {
                    *w = true;
                }
            }
            _ => {}
        }
    }
    wasted
}

/// The way's samples: the ball's centres every `STEP` along a run (without the tries that ended with the ball lost,
/// the time a transformer holds it and its dying), or along a plan's way of floor points.
fn samples(run: &Run, geo: &Geo) -> Vec<Sample> {
    let mut pts: Vec<(Vec3, usize, bool, bool, BallKind)> = Vec::new();
    if let Some(route) = &run.route {
        // a plan's way: floor points on a grid, a little off the middle; smoothed over 7 points, then sampled
        // every STEP along it with the ball set on the floor under each sample. The sector counts up as the way
        // reaches each checkpoint in order.
        let k = 3usize;
        let smooth: Vec<Vec3> = (0..route.len())
            .map(|i| {
                let w = &route[i.saturating_sub(k)..(i + k + 1).min(route.len())];
                w.iter().fold(Vec3::ZERO, |a, p| a + *p) / w.len().max(1) as f32
            })
            .collect();
        let mut sector = 1;
        let mut along = 0.0f32;
        for w in smooth.windows(2) {
            let (a, b) = (w[0], w[1]);
            let len = a.distance(b);
            // (a jump between floor points is a gap or a fan's lift: no ground under it to sample)
            if len > 6.0 {
                along = 0.0;
                continue;
            }
            while along < len {
                let p = a.lerp(b, along / len.max(1e-6));
                if let Some(cp) = run.checkpoints.get(sector - 1)
                    && sector < run.checkpoints.len()
                    && p.distance(*cp) < 8.0
                {
                    sector += 1;
                }
                let foot = geo
                    .ray(p + Vec3::Y * 3.0, -Vec3::Y, 6.0, statics)
                    .map_or(p, |h| p + Vec3::Y * (3.0 - h.0));
                pts.push((
                    foot + Vec3::Y * BALL_RADIUS,
                    sector,
                    true,
                    false,
                    BallKind::Wood,
                ));
                along += STEP;
            }
            along -= len;
        }
    } else {
        let wasted = wasted(run);
        let mut last: Option<Vec3> = None;
        for (k, r) in run.ticks.iter().enumerate() {
            if wasted.get(k).copied().unwrap_or(true) || r.held || r.dying {
                last = None;
                continue;
            }
            match last {
                Some(q) if q.distance(r.pos) < STEP => {}
                // (a jump of more than a few units in a tick is a ball put back: no way rolled)
                Some(q) if q.distance(r.pos) > 5.0 => last = Some(r.pos),
                _ => {
                    last = Some(r.pos);
                    pts.push((r.pos, r.sector, r.ground, r.updraft, r.kind));
                }
            }
        }
    }
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (p, sector, ground, updraft, kind) = pts[i];
            let a = pts[i.saturating_sub(6)].0;
            let b = pts[(i + 6).min(n - 1)].0;
            let fwd = Vec3::new(b.x - a.x, 0.0, b.z - a.z).normalize_or(Vec3::X);
            let (class, width, sides) = classify(geo, p, fwd);
            Sample {
                pos: p,
                sector,
                class,
                width,
                sides,
                ground,
                updraft,
                kind,
            }
        })
        .collect()
}

/// Open stretches beside which barriers come and go along the way are posts: a side open in some of the samples
/// within 4 units and walled in others.
fn mark_posts(s: &mut [Sample]) {
    let posts = CLASSES.iter().position(|c| *c == "posts").unwrap_or(0);
    let floorish = |c: usize| {
        matches!(
            CLASSES[c],
            "open" | "narrow" | "wide" | "curbs" | "corridor"
        )
    };
    let w = (4.0 / STEP) as usize;
    let mut out = Vec::with_capacity(s.len());
    for i in 0..s.len() {
        let lo = i.saturating_sub(w);
        let hi = (i + w + 1).min(s.len());
        let mut is_posts = false;
        if floorish(s[i].class) {
            for k in 0..2 {
                // the side alternates: open, a barrier, open again, a barrier again (at least three changes)
                let mut changes = 0;
                let mut prev: Option<bool> = None;
                for x in &s[lo..hi] {
                    if let Some(side) = x.sides[k] {
                        let b = side.height() > 0.0;
                        if prev.is_some_and(|p| p != b) {
                            changes += 1;
                        }
                        prev = Some(b);
                    }
                }
                if changes >= 3 {
                    is_posts = true;
                }
            }
        }
        out.push(is_posts);
    }
    for (x, p) in s.iter_mut().zip(out) {
        if p {
            x.class = posts;
        }
    }
}

/// Numbers of one sector (or of the whole level, sector 0).
#[derive(Default, Clone)]
struct Tally {
    values: Vec<(String, f64)>,
}

impl Tally {
    fn add(&mut self, key: &str, v: f64) {
        match self.values.iter_mut().find(|x| x.0 == key) {
            Some(x) => x.1 += v,
            None => self.values.push((key.to_owned(), v)),
        }
    }
    fn set(&mut self, key: &str, v: f64) {
        match self.values.iter_mut().find(|x| x.0 == key) {
            Some(x) => x.1 = v,
            None => self.values.push((key.to_owned(), v)),
        }
    }
}

/// The modules and things counted, with the CSV names.
const THINGS: [(&str, &str); 23] = [
    ("P_Extra_Point", "extra_points"),
    ("P_Extra_Life", "extra_lives"),
    ("P_Trafo_Wood", "trafo_wood"),
    ("P_Trafo_Stone", "trafo_stone"),
    ("P_Trafo_Paper", "trafo_paper"),
    ("P_Trafo_Sticky", "trafo_sticky"),
    ("P_Box", "boxes"),
    ("P_Ball_Wood", "loose_wood_balls"),
    ("P_Ball_Stone", "loose_stone_balls"),
    ("P_Ball_Paper", "loose_paper_balls"),
    ("P_Ball_Sticky", "loose_sticky_balls"),
    ("P_Dome", "domes"),
    ("P_Modul_01", "m01_pushers"),
    ("P_Modul_03", "m03_lifts"),
    ("P_Modul_08", "m08_swings"),
    ("P_Modul_17", "m17_poles"),
    ("P_Modul_18", "m18_fans"),
    ("P_Modul_19", "m19_flaps"),
    ("P_Modul_25", "m25_drawbridges"),
    ("P_Modul_26", "m26_sacks"),
    ("P_Modul_29", "m29_plates"),
    ("P_Modul_30", "m30_seesaws"),
    ("P_Modul_34", "m34_boxstone"),
];
const MORE_THINGS: [(&str, &str); 2] = [
    ("P_Modul_37", "m37_targets"),
    ("P_Modul_41", "m41_tipplates"),
];

/// The sector (from 1) of a level object: its `Sector_NN` group, else the sector of the nearest object in one.
pub(crate) fn sector_of(level: &level::Level, i: usize) -> usize {
    for g in level.groups_of(i) {
        if let Some(n) = g
            .strip_prefix("Sector_")
            .and_then(|n| n.parse::<usize>().ok())
        {
            return n;
        }
    }
    level
        .objects
        .get(i)
        .and_then(|o| crate::sim::sector_near(level, Vec3::from(o.position())))
        .map_or(1, |s| s + 1)
}

/// Measures one level's run (or plan's way) into tallies by sector (index 0: the whole level).
fn measure(run: &Run, sectors: usize) -> (Vec<Tally>, Vec<Sample>) {
    let geo = Geo::of(&run.level, &run.modules);
    let mut s = samples(run, &geo);
    mark_posts(&mut s);
    let mut t = vec![Tally::default(); sectors + 1];
    let both = |t: &mut Vec<Tally>, sector: usize, key: &str, v: f64| {
        if let Some(x) = t.get_mut(0) {
            x.add(key, v);
        }
        if let Some(x) = t.get_mut(sector) {
            x.add(key, v);
        }
    };
    for x in t.iter_mut() {
        x.set("time_s", 0.0);
        x.set("clean_time_s", 0.0);
        x.set("path_len", 0.0);
        for c in CLASSES {
            x.set(&format!("len_{c}"), 0.0);
        }
    }
    // lengths by class, widths
    let mut widths: Vec<Vec<f32>> = vec![Vec::new(); sectors + 1];
    let mut gaps: Vec<Vec<f32>> = vec![Vec::new(); sectors + 1];
    for (i, x) in s.iter().enumerate() {
        let step = s
            .get(i + 1)
            .filter(|n| n.sector == x.sector)
            .map_or(STEP, |n| n.pos.distance(x.pos).min(5.0));
        both(&mut t, x.sector, "path_len", step as f64);
        both(
            &mut t,
            x.sector,
            &format!("len_{}", CLASSES[x.class]),
            step as f64,
        );
        if x.updraft {
            both(&mut t, x.sector, "len_fan_flight", step as f64);
        }
        both(
            &mut t,
            x.sector,
            &format!("len_ball_{:?}", x.kind).to_ascii_lowercase(),
            step as f64,
        );
        let c = CLASSES[x.class];
        let list = if matches!(c, "rails" | "monorail" | "siderail") {
            &mut gaps
        } else {
            &mut widths
        };
        if !matches!(c, "air" | "module") {
            for k in [0, x.sector] {
                if let Some(l) = list.get_mut(k) {
                    l.push(x.width);
                }
            }
        }
    }
    for k in 0..=sectors {
        let med = |v: &mut Vec<f32>| {
            v.sort_by(f32::total_cmp);
            v.get(v.len() / 2).copied().unwrap_or(0.0) as f64
        };
        if let Some(x) = t.get_mut(k) {
            x.set("floor_width_median", med(&mut widths[k]));
            x.set("rail_spread_median", med(&mut gaps[k]));
        }
    }
    // turns and slopes, on the ground
    let ground: Vec<&Sample> = s.iter().filter(|x| x.ground).collect();
    let n = (TURN_LEN / STEP) as usize;
    // the heading of the chord from sample `i` half a turn's length on
    let heading = |i: usize| -> Option<Vec3> {
        let a = ground.get(i)?.pos;
        let b = ground.get(i + n / 2)?.pos;
        let d = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
        (d.length() > TURN_LEN / 4.0).then(|| d.normalize())
    };
    let mut i = 0;
    while i + n < ground.len() {
        let (a, b) = (heading(i), heading(i + n / 2));
        let near = ground[i].pos.distance(ground[i + n].pos) < TURN_LEN * 2.0;
        if let (Some(a), Some(b), true) = (a, b, near) {
            let angle = a.dot(b).clamp(-1.0, 1.0).acos().to_degrees();
            if angle > SHARP {
                let sector = ground[i].sector;
                both(&mut t, sector, "sharp_turns", 1.0);
                // at an edge: a turn where the track drops away or is rails
                let edgy = ground[i..=i + n].iter().any(|x| {
                    matches!(
                        CLASSES[x.class],
                        "open" | "narrow" | "rails" | "monorail" | "siderail"
                    )
                });
                if edgy {
                    both(&mut t, sector, "sharp_turns_at_edge", 1.0);
                }
                i += n;
                continue;
            } else if angle > BEND {
                both(&mut t, ground[i].sector, "bends", 1.0);
                i += n;
                continue;
            }
        }
        i += 1;
    }
    // ramps: stretches steeper than RAMP up or down, at least RAMP_LEN long; a hump is a rise followed by a fall
    // within 8 units
    let grade = |i: usize| -> Option<f32> {
        let a = ground.get(i.saturating_sub(4))?.pos;
        let b = ground.get((i + 4).min(ground.len().saturating_sub(1)))?.pos;
        let run = Vec3::new(b.x - a.x, 0.0, b.z - a.z).length();
        (run > 1.5 && a.distance(b) < 8.0).then(|| (b.y - a.y) / run)
    };
    let mut cur = 0i32;
    let mut len = 0.0;
    let mut last_up_end: Option<usize> = None;
    for i in 0..ground.len() {
        let g = grade(i).unwrap_or(0.0);
        let dir = if g > RAMP {
            1
        } else if g < -RAMP {
            -1
        } else {
            0
        };
        if dir == cur {
            len += STEP;
            continue;
        }
        if cur != 0 && len >= RAMP_LEN {
            let sector = ground[i.saturating_sub(1)].sector;
            if cur > 0 {
                both(&mut t, sector, "ramps_up", 1.0);
                last_up_end = Some(i);
            } else {
                both(&mut t, sector, "ramps_down", 1.0);
            }
        }
        if dir < 0 && last_up_end.is_some_and(|e| i - e < (8.0 / STEP) as usize) {
            both(&mut t, ground[i].sector, "humps", 1.0);
            last_up_end = None;
        }
        cur = dir;
        len = STEP;
    }
    // times, flights, events (runs only)
    if run.route.is_none() {
        let mut air = 0usize;
        let mut lift = false;
        let lost = wasted(run);
        for (k, r) in run.ticks.iter().enumerate() {
            both(&mut t, r.sector, "time_s", 1.0 / 60.0);
            if !lost.get(k).copied().unwrap_or(true) {
                both(&mut t, r.sector, "clean_time_s", 1.0 / 60.0);
            }
            let flying = !r.ground && !r.dying && !r.held;
            if flying {
                air += 1;
                lift |= r.updraft;
            } else {
                if air >= FLIGHT_TICKS {
                    both(
                        &mut t,
                        r.sector,
                        if lift { "fan_flights" } else { "jumps" },
                        1.0,
                    );
                    both(&mut t, r.sector, "air_time_s", air as f64 / 60.0);
                }
                air = 0;
                lift = false;
            }
        }
        for &(_, e, sector) in &run.events {
            match e {
                Event::ExtraPoint => both(&mut t, sector, "taken_extra_points", 1.0),
                Event::ExtraLife => both(&mut t, sector, "taken_extra_lives", 1.0),
                Event::Transform(_) => both(&mut t, sector, "transforms", 1.0),
                Event::Died => both(&mut t, sector, "balls_lost", 1.0),
                _ => {}
            }
        }
    }
    // the level's things by sector
    let level = &run.level;
    for (i, o) in level.objects.iter().enumerate() {
        let groups: Vec<&str> = level.groups_of(i).collect();
        for (prefix, key) in THINGS.iter().chain(MORE_THINGS.iter()) {
            // an object counts by its own group (P_Box), not by sector or sound groups
            if groups.iter().any(|g| g.eq_ignore_ascii_case(prefix)) {
                let _ = o;
                both(&mut t, sector_of(level, i).min(sectors), key, 1.0);
            }
        }
    }
    for (name, d) in &run.moved {
        if *d > MOVED
            && let Some(i) = level.objects.iter().position(|o| &o.name == name)
        {
            both(&mut t, sector_of(level, i).min(sectors), "loose_moved", 1.0);
        }
    }
    (t, s)
}

/// The replays of a level found in `hints`: the clean runs first (hints/runs), then the owner's.
pub(crate) fn replays(hints: &Path, n: u32) -> Vec<PathBuf> {
    let stem = format!("Level_{n:02}");
    let mut out: Vec<PathBuf> = std::fs::read_dir(hints.join("runs"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|f| f.starts_with(&stem) && f.ends_with(".replay"))
        })
        .collect();
    out.sort();
    let owner = hints.join(format!("{stem}-owner.replay"));
    if owner.is_file() {
        out.push(owner);
    }
    out
}

/// `--level-stats DIR`: measures the original levels 1 to 13 (or `--level N` alone) and writes
/// `DIR/level-stats-auto.csv` (one row a sector and one for the whole level, for each source: a finished run, a
/// plan's way) and the samples of each way (`DIR/samples/`) to check the classes against. With `--open LEVEL` and
/// `--replay RUN` it measures that level (one of ours) along that run instead, into `DIR/level-stats-own.csv`.
pub fn run(o: &Options, data: &Data, out: &Path, hints: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out.join("samples")).map_err(|e| format!("{}: {e}", out.display()))?;
    if let Source::Open(dir) = &o.source {
        return run_open(o, data, dir, out);
    }
    let levels: Vec<u32> = match o.source {
        Source::Original(n) if o.chosen => vec![n],
        _ => (1..=13).collect(),
    };
    let mut rows: Vec<(String, String, String, usize, Tally)> = Vec::new();
    for n in levels {
        let mut lo = o.clone();
        lo.source = Source::Original(n);
        lo.replay = None;
        let data = data.for_source(&lo.source);
        let mut sources: Vec<(String, Run)> = Vec::new();
        // the first finished run from the level start, and the time of every finished one
        let mut times: Vec<String> = Vec::new();
        for r in replays(hints, n) {
            lo.replay = Some(r.clone());
            match crate::app::stats_run(&lo, &data, None) {
                Ok(run) => {
                    let name = r
                        .file_name()
                        .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
                    let ticks = run.ticks.len();
                    println!(
                        "level {n}: {name}: {} after {:.1} s{}",
                        if run.finished {
                            "finished"
                        } else {
                            "not finished"
                        },
                        ticks as f64 / 60.0,
                        if run.begun_elsewhere {
                            " (begun at a sector)"
                        } else {
                            ""
                        }
                    );
                    if run.finished && !run.begun_elsewhere {
                        times.push(format!("{name}={:.1}", ticks as f64 / 60.0));
                        if !sources.iter().any(|s| s.0.starts_with("run")) {
                            sources.push((format!("run:{name}"), run));
                        }
                    }
                }
                Err(e) => println!("level {n}: {}: {e}", r.display()),
            }
        }
        let plan = hints.join(format!("Level_{n:02}.plan"));
        if plan.is_file() {
            lo.replay = None;
            match crate::app::stats_run(&lo, &data, Some(&plan)) {
                Ok(run) => sources.push(("plan".to_owned(), run)),
                Err(e) => println!("level {n}: {}: {e}", plan.display()),
            }
        }
        for (source, run) in &sources {
            let kind = if source.starts_with("run") { "run" } else { "plan" };
            let file = out.join("samples").join(format!("Level_{n:02}-{kind}.csv"));
            for (k, t) in measure_into(run, &file)?.into_iter().enumerate() {
                rows.push((n.to_string(), source.clone(), times.join(" "), k, t));
            }
        }
    }
    write_rows(&rows, &out.join("level-stats-auto.csv"))
}

/// One of our levels (an open-format folder) measured along the run `o.replay`.
fn run_open(o: &Options, data: &Data, dir: &Path, out: &Path) -> Result<(), String> {
    let replay = o.replay.clone().ok_or("--level-stats with --open needs --replay RUN")?;
    let data = data.for_source(&o.source);
    let run = crate::app::stats_run(o, &data, None)?;
    let name = dir.file_name().map_or_else(String::new, |f| f.to_string_lossy().into_owned());
    let run_name = replay.file_name().map_or_else(String::new, |f| f.to_string_lossy().into_owned());
    println!(
        "{name}: {run_name}: {} after {:.1} s",
        if run.finished { "finished" } else { "not finished" },
        run.ticks.len() as f64 / 60.0
    );
    let file = out.join("samples").join(format!("{name}-run.csv"));
    let mut tallies = measure_into(&run, &file)?;
    // how the ride went (the whole level's row): the share of the time the ball stood (not counting a transformer's
    // hold), and its speed while it rolled
    let ride = ride(&run.ticks);
    println!(
        "{name}: standing {:.1}% of the time, speed p10 {:.1} p50 {:.1} p90 {:.1} units/s",
        ride[0] * 100.0,
        ride[1],
        ride[2],
        ride[3]
    );
    if let Some(t) = tallies.first_mut() {
        for (k, v) in ["standing_share", "speed_p10", "speed_p50", "speed_p90"].into_iter().zip(ride) {
            t.set(k, v);
        }
    }
    let rows: Vec<_> = tallies
        .into_iter()
        .enumerate()
        .map(|(k, t)| (name.clone(), format!("run:{run_name}"), format!("{:.1}", run.ticks.len() as f64 / 60.0), k, t))
        .collect();
    write_rows(&rows, &out.join("level-stats-own.csv"))
}

/// Standing below this speed (units a second).
const STANDING: f32 = 0.5;

/// The share of the ticks the ball stood (held by a transformer or dying not counted), and the 10th, 50th and 90th
/// percentiles of its speed while it rolled.
fn ride(ticks: &[TickRec]) -> [f64; 4] {
    let mut speeds: Vec<f32> = Vec::new();
    let (mut standing, mut counted) = (0usize, 0usize);
    for w in ticks.windows(2) {
        if w[1].held || w[1].dying || w[0].dying {
            continue;
        }
        let v = w[1].pos.distance(w[0].pos) * 60.0;
        counted += 1;
        if v < STANDING {
            standing += 1;
        } else {
            speeds.push(v);
        }
    }
    speeds.sort_by(f32::total_cmp);
    let pct = |q: f64| speeds.get(((speeds.len() as f64 - 1.0) * q).round().max(0.0) as usize).copied().unwrap_or(0.0) as f64;
    [standing as f64 / counted.max(1) as f64, pct(0.1), pct(0.5), pct(0.9)]
}

/// Measures a way and writes its samples to `file`: the tallies, the whole level's first, then each sector's.
fn measure_into(run: &Run, file: &Path) -> Result<Vec<Tally>, String> {
    let sectors = run.checkpoints.len().max(1);
    let (mut tallies, samples) = measure(run, sectors);
    for t in tallies.iter_mut() {
        t.set("finished", run.finished as u8 as f64);
        t.set("level_bonus", run.level.settings.level_bonus.unwrap_or(0) as f64);
    }
    let mut text = String::from("x,y,z,sector,class,width,left,right,ball\n");
    for x in &samples {
        let side = |s: Option<Side>| s.map_or_else(String::new, |s| format!("{s:?}").replace(',', ";"));
        let _ = writeln!(
            text,
            "{:.2},{:.2},{:.2},{},{},{:.2},{},{},{:?}",
            x.pos.x,
            x.pos.y,
            x.pos.z,
            x.sector,
            CLASSES[x.class],
            x.width,
            side(x.sides[0]),
            side(x.sides[1]),
            x.kind
        );
    }
    std::fs::write(file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(tallies)
}

/// The rows (level, source, finished runs, sector, tally) as CSV, under one header over every key any row has.
fn write_rows(rows: &[(String, String, String, usize, Tally)], file: &Path) -> Result<(), String> {
    // one header over every key any row has, in the order they first came
    let mut keys: Vec<String> = Vec::new();
    for r in rows {
        for (k, _) in &r.4.values {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
        }
    }
    let mut text = format!("level,source,finished_runs,sector,{}\n", keys.join(","));
    for (n, source, times, sector, t) in rows {
        let _ = write!(text, "{n},{source},{times},{sector}");
        for k in &keys {
            let v = t.values.iter().find(|x| &x.0 == k).map_or(0.0, |x| x.1);
            let _ = write!(text, ",{}", (v * 100.0).round() / 100.0);
        }
        text.push('\n');
    }
    std::fs::write(file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    println!("written {}", file.display());
    Ok(())
}
