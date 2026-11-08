// For the test player (autopilot.rs), not part of the game: where the level's floors end. A floor's edge is where
// it drops away or meets a wall; the way's search and the steering keep the ball off them, as a player keeps to the
// middle of a track, and a run is summed up by how near them its ball rolled.

use glam::Vec3;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// A triangle of the level's floors with the rail object it belongs to, if any (autopilot::floor_triangles).
pub type Tri = ([Vec3; 3], Option<usize>);

/// Distances to an edge are looked for this far at most; further counts as this far.
pub const FAR: f32 = 8.0;
/// A triangle rising at most this steeply (the up part of its normal) is floor; one standing steeper is a wall.
const FLOOR_UP: f32 = 0.68;
const WALL_UP: f32 = 0.5;
/// A floor's border is cut into pieces this long, each an edge where no floor goes on just beyond it (this far out,
/// as high within `JOIN`): a seam between two floors' meshes is no edge, a drop or a wall's foot is.
const PIECE: f32 = 1.0;
const OUT: f32 = 0.35;
const JOIN: f32 = 0.6;
/// An edge counts for a place within this height of it.
const LEVEL: f32 = 1.0;
/// A wall's foot this near the nearest edge makes it a wall, not a drop.
const WALL_NEAR: f32 = 1.0;
/// The cell of the lookups.
const CELL: f32 = 4.0;

/// Things by the cell (`CELL`) they are in.
type Cells<T> = HashMap<(i32, i32), Vec<T>>;
/// The levels' edges built so far, by their floors' fingerprint.
type Cache = Mutex<Vec<(u64, Arc<Edges>)>>;

/// The edges of a level's floors, and its floor triangles to tell what is under a place.
pub struct Edges {
    /// Pieces of edges (each end, at the floor's height, and whether it is a wall's foot), by cell.
    edges: Cells<(Vec3, Vec3, bool)>,
    /// Up-facing triangles, and whether each is a rail's, by cell.
    floors: Cells<([Vec3; 3], bool)>,
}

fn cell(x: f32, z: f32) -> (i32, i32) {
    ((x / CELL).floor() as i32, (z / CELL).floor() as i32)
}

/// The cells a box across covers (at most 4096, for a huge triangle: its middle is what matters).
fn cells(lo: Vec3, hi: Vec3) -> impl Iterator<Item = (i32, i32)> {
    let (a, b) = (cell(lo.x, lo.z), cell(hi.x, hi.z));
    let (bx, bz) = (b.0.min(a.0 + 63), b.1.min(a.1 + 63));
    (a.0..=bx).flat_map(move |x| (a.1..=bz).map(move |z| (x, z)))
}

/// The height of triangle `t` over the place `(x, z)`, if the place is inside it seen from above.
fn height_in(t: &[Vec3; 3], x: f32, z: f32) -> Option<f32> {
    let [a, b, c] = *t;
    let d = (b.x - a.x) * (c.z - a.z) - (c.x - a.x) * (b.z - a.z);
    if d.abs() < 1e-6 {
        return None;
    }
    let u = ((x - a.x) * (c.z - a.z) - (c.x - a.x) * (z - a.z)) / d;
    let v = ((b.x - a.x) * (z - a.z) - (x - a.x) * (b.z - a.z)) / d;
    const E: f32 = -1e-4;
    (u >= E && v >= E && u + v <= 1.0 - E).then_some(a.y + (b.y - a.y) * u + (c.y - a.y) * v)
}

/// How far `p` is across from the piece `a`-`b`, and the piece's height at its point nearest `p`.
fn across(p: Vec3, a: Vec3, b: Vec3) -> (f32, f32) {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let l2 = dx * dx + dz * dz;
    let t = if l2 > 1e-8 {
        (((p.x - a.x) * dx + (p.z - a.z) * dz) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let c = a + (b - a) * t;
    (Vec3::new(p.x - c.x, 0.0, p.z - c.z).length(), c.y)
}

impl Edges {
    /// The edges of the floors `tris`, of which the first `own` are the level's own (the rest the modules', whose
    /// steep sides, moving parts, are no walls). Built once a level: kept for the level's triangles seen again.
    pub fn of(tris: &[Tri], own: usize) -> Arc<Edges> {
        static CACHE: OnceLock<Cache> = OnceLock::new();
        let key = fingerprint(tris);
        let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
        if let Ok(c) = cache.lock()
            && let Some((_, e)) = c.iter().find(|(k, _)| *k == key)
        {
            return e.clone();
        }
        let e = Arc::new(Edges::build(tris, own));
        if let Ok(mut c) = cache.lock() {
            if c.len() >= 4 {
                c.remove(0);
            }
            c.push((key, e.clone()));
        }
        e
    }

    fn build(tris: &[Tri], own: usize) -> Edges {
        let mut floors: Cells<([Vec3; 3], bool)> = HashMap::new();
        let up = |t: &[Vec3; 3]| {
            let n = (t[1] - t[0]).cross(t[2] - t[0]);
            let l = n.length();
            if l < 1e-6 {
                None
            } else {
                Some((n.y / l).abs())
            }
        };
        for (t, rail) in tris {
            if up(t).is_some_and(|u| u >= if rail.is_some() { 0.25 } else { FLOOR_UP }) {
                let lo = t[0].min(t[1]).min(t[2]);
                let hi = t[0].max(t[1]).max(t[2]);
                for c in cells(lo, hi) {
                    floors.entry(c).or_default().push((*t, rail.is_some()));
                }
            }
        }
        let mut e = Edges {
            edges: HashMap::new(),
            floors,
        };
        let mut pieces = Vec::new();
        for (k, (t, rail)) in tris.iter().enumerate() {
            let Some(u) = up(t) else { continue };
            if rail.is_some() {
                continue;
            }
            if u >= FLOOR_UP {
                // each side of a floor triangle: its pieces with no floor just beyond them
                for i in 0..3 {
                    let (a, b, c) = (t[i], t[(i + 1) % 3], t[(i + 2) % 3]);
                    let side = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
                    let len = side.length();
                    if len < 1e-3 {
                        continue;
                    }
                    let mut out = Vec3::new(-side.z, 0.0, side.x) / len;
                    if out.dot(Vec3::new(c.x - a.x, 0.0, c.z - a.z)) > 0.0 {
                        out = -out;
                    }
                    let n = ((len / PIECE).ceil() as usize).clamp(1, 400);
                    for j in 0..n {
                        let (p, q) = (
                            a + (b - a) * (j as f32 / n as f32),
                            a + (b - a) * ((j + 1) as f32 / n as f32),
                        );
                        let m = (p + q) * 0.5 + out * OUT;
                        if !e.floor_at(m, JOIN, false) {
                            pieces.push((p, q, false));
                        }
                    }
                }
            } else if u < WALL_UP && k < own {
                // a wall's foot: where it stands, just over its lowest point
                let y0 = t[0].y.min(t[1].y).min(t[2].y);
                let h = y0 + 0.25;
                let mut cut = Vec::new();
                for i in 0..3 {
                    let (a, b) = (t[i], t[(i + 1) % 3]);
                    if (a.y - h) * (b.y - h) < 0.0 {
                        let s = (h - a.y) / (b.y - a.y);
                        cut.push(a + (b - a) * s);
                    }
                }
                if let [a, b] = cut[..] {
                    pieces.push((Vec3::new(a.x, y0, a.z), Vec3::new(b.x, y0, b.z), true));
                }
            }
        }
        for (a, b, wall) in pieces {
            for c in cells(a.min(b), a.max(b)) {
                e.edges.entry(c).or_default().push((a, b, wall));
            }
        }
        e
    }

    /// Whether a floor (a rail too, with `rails`) is at `p`, within `tol` of its height.
    fn floor_at(&self, p: Vec3, tol: f32, rails: bool) -> bool {
        self.floors
            .get(&cell(p.x, p.z))
            .into_iter()
            .flatten()
            .filter(|(_, r)| rails || !r)
            .any(|(t, _)| height_in(t, p.x, p.z).is_some_and(|y| (y - p.y).abs() < tol))
    }

    /// What is under the place `p` on a floor (the ball's lowest point): `Some(true)` a rail, `Some(false)` a floor,
    /// `None` nothing within `tol` of its height.
    pub fn under(&self, p: Vec3, tol: f32) -> Option<bool> {
        let mut best: Option<(f32, bool)> = None;
        for (t, r) in self.floors.get(&cell(p.x, p.z)).into_iter().flatten() {
            if let Some(y) = height_in(t, p.x, p.z) {
                let d = (y - p.y).abs();
                if d < tol && best.is_none_or(|b| d < b.0) {
                    best = Some((d, *r));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// Whether a rail's triangle is within `r` across of `p` (and as high within `tol`).
    pub fn rail_near(&self, p: Vec3, r: f32, tol: f32) -> bool {
        let lo = p - Vec3::new(r, 0.0, r);
        let hi = p + Vec3::new(r, 0.0, r);
        cells(lo, hi).any(|c| {
            self.floors.get(&c).into_iter().flatten().any(|(t, rail)| {
                *rail
                    && t.iter().any(|v| {
                        Vec3::new(v.x - p.x, 0.0, v.z - p.z).length() < r + 2.0
                            && (v.y - p.y).abs() < tol
                    })
            })
        })
    }

    /// How far across the place `p` on a floor is from the nearest edge as high (within `LEVEL`), up to `FAR`.
    pub fn distance(&self, p: Vec3) -> f32 {
        let lo = p - Vec3::new(FAR, 0.0, FAR);
        let hi = p + Vec3::new(FAR, 0.0, FAR);
        let mut d = FAR;
        for c in cells(lo, hi) {
            for &(a, b, _) in self.edges.get(&c).into_iter().flatten() {
                let (h, y) = across(p, a, b);
                if h < d && (y - p.y).abs() < LEVEL {
                    d = h;
                }
            }
        }
        d
    }

    /// What borders the place `p` on a floor within `FAR` across (the test player's situations, learned.rs): its
    /// nearest edge as high, and whether a wall's foot is within `WALL_NEAR` of that (a wall stands there, the ball
    /// is held) or not (the floor drops away). None: no edge that near.
    pub fn border(&self, p: Vec3) -> Option<bool> {
        let lo = p - Vec3::new(FAR, 0.0, FAR);
        let hi = p + Vec3::new(FAR, 0.0, FAR);
        let mut near: Option<f32> = None;
        let mut walls: Vec<f32> = Vec::new();
        for c in cells(lo, hi) {
            for &(a, b, wall) in self.edges.get(&c).into_iter().flatten() {
                let (h, y) = across(p, a, b);
                if h < FAR && (y - p.y).abs() < LEVEL {
                    near = Some(near.map_or(h, |n| n.min(h)));
                    if wall {
                        walls.push(h);
                    }
                }
            }
        }
        let d = near?;
        Some(walls.iter().any(|&w| w < d + WALL_NEAR))
    }

    /// How far the ball at `ball` (its centre) on the ground is from an edge, for a run's summary: `None` on a rail
    /// (it keeps to the rail's line) or on no floor of the level (a module's part, a box); 0 with its centre past an
    /// edge it still rolls on.
    pub fn ball_clearance(&self, ball: Vec3) -> Option<f32> {
        let f = ball - Vec3::Y * crate::sim::BALL_RADIUS;
        if self.rail_near(f, 1.5, 2.5) {
            return None;
        }
        match self.under(f, 0.8) {
            Some(false) => Some(self.distance(f)),
            Some(true) => None,
            None => (self.distance(f) < crate::sim::BALL_RADIUS).then_some(0.0),
        }
    }
}

/// A key for a level's floors: their number and a hash of their corners.
fn fingerprint(tris: &[Tri]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ tris.len() as u64;
    for (t, r) in tris {
        for v in t {
            for x in [v.x, v.y, v.z] {
                h = (h ^ u64::from(x.to_bits())).wrapping_mul(0x0100_0000_01b3);
            }
        }
        h = (h ^ r.map_or(0, |r| r as u64 + 1)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// How near the edges a run's ball rolled: summed over its ticks on a floor (not on rails).
#[derive(Clone, Copy, Debug, Default)]
pub struct EdgeStats {
    pub sum: f64,
    pub n: u64,
    pub min: Option<f32>,
    /// Ticks within one unit of an edge.
    pub close: u64,
}

impl EdgeStats {
    pub fn add(&mut self, d: f32) {
        self.sum += f64::from(d);
        self.n += 1;
        self.min = Some(self.min.map_or(d, |m| m.min(d)));
        if d < 1.0 {
            self.close += 1;
        }
    }

    /// One tick of a run: the ball, if on the ground.
    pub fn tick(&mut self, edges: &Edges, ball: &crate::sim::Ball) {
        if ball.on_ground
            && let Some(d) = edges.ball_clearance(ball.position)
        {
            self.add(d);
        }
    }

    /// The line of a run's summary.
    pub fn line(&self) -> String {
        if self.n == 0 {
            return "edges: no ticks on a floor".into();
        }
        format!(
            "edges: mean {:.2}, min {:.2}, within 1 for {:.1}% of {} ticks on floors",
            self.sum / self.n as f64,
            self.min.unwrap_or(0.0),
            100.0 * self.close as f64 / self.n as f64,
            self.n
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two triangles: a square floor from (x0, 0) to (x1, w) at height y.
    fn square(x0: f32, x1: f32, w: f32, y: f32) -> Vec<Tri> {
        let (a, b, c, d) = (
            Vec3::new(x0, y, 0.0),
            Vec3::new(x1, y, 0.0),
            Vec3::new(x1, y, w),
            Vec3::new(x0, y, w),
        );
        vec![([a, b, c], None), ([a, c, d], None)]
    }

    #[test]
    fn a_lane_is_clearest_in_its_middle_and_a_seam_is_no_edge() {
        // a lane 8 wide in two meshes meeting at x = 10
        let mut tris = square(0.0, 10.0, 8.0, 0.0);
        tris.extend(square(10.0, 20.0, 8.0, 0.0));
        let e = Edges::build(&tris, tris.len());
        assert!((e.distance(Vec3::new(10.0, 0.0, 4.0)) - 4.0).abs() < 0.05);
        assert!((e.distance(Vec3::new(10.0, 0.0, 1.0)) - 1.0).abs() < 0.05);
        assert!((e.distance(Vec3::new(5.0, 0.0, 4.0)) - 4.0).abs() < 0.05);
        assert_eq!(e.under(Vec3::new(5.0, 0.2, 4.0), 0.8), Some(false));
        assert_eq!(e.under(Vec3::new(5.0, 3.0, 4.0), 0.8), None);
    }

    #[test]
    fn a_wall_standing_on_a_floor_is_an_edge() {
        let mut tris = square(0.0, 20.0, 20.0, 0.0);
        // a wall along z = 6, from y = 0 up to 3
        let (a, b, c, d) = (
            Vec3::new(0.0, 0.0, 6.0),
            Vec3::new(20.0, 0.0, 6.0),
            Vec3::new(20.0, 3.0, 6.0),
            Vec3::new(0.0, 3.0, 6.0),
        );
        tris.push(([a, b, c], None));
        tris.push(([a, c, d], None));
        let e = Edges::build(&tris, tris.len());
        assert!((e.distance(Vec3::new(10.0, 0.0, 8.0)) - 2.0).abs() < 0.05);
        // a floor high above is not this one: its edges do not count here
        let mut high = square(0.0, 20.0, 20.0, 0.0);
        high.extend(square(0.0, 20.0, 3.0, 10.0));
        let e = Edges::build(&high, high.len());
        assert!(e.distance(Vec3::new(10.0, 0.0, 4.0)) > 3.9);
    }
}
