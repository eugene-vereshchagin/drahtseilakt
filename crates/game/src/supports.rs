// What a swinging sack hangs on. The original's sack (P_Modul_26) hangs from a crossbar that floats in the air; our
// own part brings pieces of a support (beams and mounts of timber, iron pipe and rail tube, a stone column, a striped
// mast, a timber post, a post of rail tube, a wall plate: the part's `Supports_<module>` group,
// assets/own/make/make_parts.py) and this file decides, for every sack of a level as it is loaded, what is put
// together of them and where: a free post beside the track, a gate of two posts, a beam to a wall or a pillar of the
// level that stands near, or, for a sack with no room of its own, a cross-beam to a neighbour that has a support. A
// sack still bare after that is looked at once more with longer beams, and beams along its swing. The
// holder's mount, the beam and the post are of one material, and every member is level or upright. It is drawn only:
// the physics never hears of it, so a run and its replay are the same with or without it (the `sack-supports` mod
// switches it, docs/drahtseilakt/mods.md).
//
// A support may stand only where nothing of it comes near what the game needs: the space the ball rolls, hops and
// flies in over the floors and rails, every other module's reach (a sack's own swing, a fan's draught, a pusher, a
// box that is pushed about), the level's other geometry, the usual camera's sight of the ball, and the supports
// already placed. Nothing of that is known for one level or one map: it is measured on the level's own geometry,
// with the pieces' own meshes. Where no candidate is clear the sack hangs as the original's does: when in doubt,
// no support.
// Reason for the allow: geometry derived from the validated level model.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use crate::collide::closest_point;
use crate::scene::{FIRST_MODULE_MODEL, Modules, Placement};
use glam::{Mat4, Vec3};
use level::Level;
use std::collections::HashMap;

/// The module that has supports, the frame its sack hangs from and the names of its pieces.
const MODULE: &str = "P_Modul_26";
const JOINT: &str = "P_Modul_26_Balljoint_oben";
const PIECES: [&str; N] = [
    "Beam_Wood",
    "Beam_Iron",
    "Beam_Rail",
    "Mount_Wood",
    "Mount_Iron",
    "Mount_Rail",
    "Column",
    "Mast",
    "Post",
    "Rail",
    "Wall",
    "Column_Shaft",
    "Mast_Shaft",
    "Post_Shaft",
    "Rail_Shaft",
];
const N: usize = 15;
/// The materials (timber, iron pipe, rail tube): a beam and a mount of each, in this order.
const WOOD: usize = 0;
const IRON: usize = 1;
const TUBE: usize = 2;
const MOUNT: usize = 3;
const WALL_PIECE: usize = 10;
/// A post's plain shaft, put under it as often as the level is deep (the last pieces, in the posts' order).
const SHAFT: usize = 5;
const FIRST_SHAFT: usize = 11;
const MAX_SHAFTS: usize = 6;

/// The designs: a free stone column, a striped mast, a timber post, a gate of two timber posts, a post of rail tube
/// bent over into its beam, a beam to a wall or a pillar of the level, a cross-beam to a neighbour's holder.
pub const DESIGNS: [&str; ND] = ["column", "mast", "post", "gate", "rail", "wall", "link"];
const ND: usize = 7;
const GATE: usize = 3;
const TO_WALL: usize = 5;
const LINK: usize = 6;
/// The post piece of each free design, and each design's material (a link takes its neighbour's).
const POST_PIECE: [usize; 5] = [6, 7, 8, 8, 9];
const MATERIAL: [usize; 6] = [IRON, IRON, WOOD, WOOD, TUBE, IRON];
/// The bare beam, tested before the designs.
const BARE: usize = usize::MAX;

/// The ball's radius and the air kept between its space and a support.
const BALL_RADIUS: f32 = 2.0;
const AIR: f32 = 2.5;
/// How high over a floor or rail the ball's space reaches (its centre in a hop), as lifts of the surface.
const LIFTS: [f32; 4] = [0.0, 3.0, 6.0, 9.0];
/// The air kept to the level's other geometry (a support never cuts through scenery).
const SOLID_AIR: f32 = 0.6;
/// The pieces are tested at points this far apart at most; the tests add the half of it to their distances.
const SPACING: f32 = 1.2;
const SLACK: f32 = 0.9;
/// The grid the triangles are looked up by (x, z), and how far a test reaches.
const CELL: f32 = 8.0;
/// (with the half width of the widest plain shaft, which is tested along its axis only)
const REACH: f32 = BALL_RADIUS + AIR + SLACK + 1.7;
/// A triangle over more cells than this is tested for every point.
const MAX_CELLS: i64 = 1024;

/// A sack's reach from its joint (the rope, the sack, some air), and how far it swings out: wide along its push
/// (the holder's z), less across, where only the ball knocks it (the tangents of 65 and 28 degrees from straight down).
const SWING_REACH: f32 = 19.5;
const SWING_ALONG: f32 = 2.14;
const SWING_ACROSS: f32 = 0.53;
/// The air between another sack's swing and a support; the sack's own support starts at its joint.
const SWING_AIR: f32 = 2.0;
const OWN_SWING_AIR: f32 = 0.3;
/// What is kept free about another module's stand-in beyond its box (its parts move), about a marker's, and the
/// least of it.
const MODULE_AIR: f32 = 8.0;
const MARKER_AIR: f32 = 6.0;
const MODULE_LEAST: f32 = 12.0;
/// A fan's draught carries the ball up: a column of this radius, this far up.
const FAN: &str = "P_Modul_18";
const FAN_RADIUS: f32 = 12.0;
const FAN_UP: f32 = 90.0;

/// The usual camera (sim.rs: 22 behind the ball, 35 over it) in its four headings, and how near its sight of the
/// ball a wide post may come beside its own half width. A shaft narrower than `WIDE` never hides the ball.
const CAM_BACK: f32 = 22.0;
const CAM_UP: f32 = 35.0;
const SIGHT_AIR: f32 = 1.0;
const WIDE: f32 = 0.8;
/// Floors this near a sack (horizontally) are where the ball is looked at from.
const SIGHT_NEAR: f32 = 40.0;

/// The beam's lengths tried, the shortest first, and its directions about the holder's vertical, in degrees from
/// the holder's x (across the sack's swing), the nearest to it first.
const LENGTHS: [f32; 8] = [5.0, 6.5, 8.0, 9.5, 11.0, 13.0, 15.0, 17.0];
const YAWS: [f32; 10] = [
    0.0, 180.0, 25.0, -25.0, 155.0, -155.0, 50.0, -50.0, 130.0, -130.0,
];
/// A sack that found nothing, and no neighbour to hang on, is looked at once more with longer beams and with beams
/// along its swing as well (the nearest to across first): the post then stands further along the track's side.
const MORE_LENGTHS: [f32; 4] = [20.0, 23.0, 26.0, 30.0];
const MORE_YAWS: [f32; 6] = [75.0, -75.0, 105.0, -105.0, 90.0, -90.0];
/// A wall is looked for every so many degrees, this near and this far, and must face the beam this squarely.
const WALL_STEP: usize = 5;
/// (at the height of the beams' middles over the joint)
const WALL_AT: f32 = 0.5;
const WALL_NEAR: f32 = 3.5;
const WALL_FAR: f32 = 18.0;
const WALL_FACING: f32 = 0.8;
/// The scenery this near a wall plate's place is the wall it is on.
const WALL_OPEN: f32 = 2.0;
/// A floor's face this steep (its normal's y; not a rail's) no wider than `PILLAR` is a pillar's side: no ball rolls
/// on it, and a wall plate may sit on it. The beam comes to it through what would be the ball's space over a floor:
/// the pillar's sides are exempt from that test `PATCH` to either side of the plate, from the face inwards.
const STEEP: f32 = 0.3;
const PILLAR: f32 = 7.0;
const PATCH: f32 = 2.5;
/// A sack with no room of its own hangs on a cross-beam to a neighbour's holder this near, about as high as its own
/// (both holders upright).
const LINK_FAR: f32 = 34.0;
const LINK_LEVEL: f32 = 0.25;
const LINK_UPRIGHT: f32 = 0.995;
/// A holder with the level's own geometry this near its joint hangs on that already (a beam or an arch of the map).
const HELD: f32 = 1.0;
/// A piece's triangles with no side this long are its small parts (bolt heads), which lie on its larger ones: the
/// search does not test them, the check does.
const SMALL: f32 = 0.2;
/// A post's foot must be this far under the level's lowest floor (it goes down into the clouds), and nothing is
/// tested deeper than this under it (what is there is not seen from the track).
const FOOT_UNDER: f32 = 40.0;
const DEEP: f32 = 30.0;
/// Supports keep this far from each other.
const BETWEEN: f32 = 1.5;
/// No more sacks than this get a support looked for (a map of a thousand sacks loads as fast as any).
const MAX_SACKS: usize = 64;

/// A triangle of the level a support must keep away from.
struct Tri {
    p: [Vec3; 3],
    /// A floor or rail: the ball's space lies over it. Otherwise scenery.
    ball: bool,
    /// A floor's or rail's face too steep to roll on (a pillar's side).
    steep: bool,
    /// The box no point outside of can be too near it (the triangle's, with the ball's space over it and the reach).
    lo: Vec3,
    hi: Vec3,
}

/// A place another module needs.
enum Keep {
    Sphere(Vec3, f32),
    /// A vertical column: centre of its foot, radius, height.
    Column(Vec3, f32, f32),
    /// A sack's swing: its joint, its axes (across, up, along) and the level object.
    Swing(Vec3, [Vec3; 3], usize),
}

/// The level as the supports see it.
pub struct Space {
    tris: Vec<Tri>,
    grid: HashMap<(i32, i32), Vec<u32>>,
    wide: Vec<u32>,
    keeps: Vec<Keep>,
    /// Where the ball's centre is when it rolls: points over the floors' tops.
    rolls: Vec<Vec3>,
    /// The lowest floor or rail: under it the ball is lost, and a post's foot is out of sight.
    lowest: f32,
}

fn cell(v: f32) -> i32 {
    (v / CELL).floor().clamp(-1.0e6, 1.0e6) as i32
}

fn finite(m: &Mat4) -> bool {
    m.to_cols_array()
        .iter()
        .all(|v| v.is_finite() && v.abs() < 1.0e6)
}

fn in_group(level: &Level, i: usize, names: &[&str]) -> bool {
    level
        .groups_of(i)
        .any(|g| names.iter().any(|n| n.eq_ignore_ascii_case(g)))
}

/// The world triangles of a mesh under `matrix`.
fn triangles(mesh: &level::Mesh, matrix: &Mat4, out: &mut Vec<[Vec3; 3]>) {
    for part in &mesh.parts {
        for t in part.indices.as_chunks::<3>().0 {
            let v = |k: usize| {
                t.get(k)
                    .and_then(|&i| mesh.positions.get(i as usize))
                    .map(|p| matrix.transform_point3(Vec3::from(*p)))
            };
            if let (Some(a), Some(b), Some(c)) = (v(0), v(1), v(2))
                && (b - a).cross(c - a).length_squared() > 1e-10
                && a.is_finite()
                && b.is_finite()
                && c.is_finite()
            {
                out.push([a, b, c]);
            }
        }
    }
}

/// The points a triangle is tested at: it is halved along its longest side until no side is longer than `spacing`,
/// and the pieces' middles are taken (no point of the triangle is then farther than 0.6 `spacing` from one of them).
/// Pieces wholly under `floor` are left out.
fn sample(t: [Vec3; 3], spacing: f32, floor: f32, out: &mut Vec<Vec3>) {
    let mut stack = vec![t];
    while let Some([a, b, c]) = stack.pop() {
        if a.y.max(b.y).max(c.y) < floor || out.len() > 400_000 {
            continue;
        }
        let (ab, bc, ca) = ((b - a).length(), (c - b).length(), (a - c).length());
        let longest = ab.max(bc).max(ca);
        if longest <= spacing {
            out.push((a + b + c) / 3.0);
        } else if longest == ab {
            let m = (a + b) * 0.5;
            stack.extend([[a, m, c], [m, b, c]]);
        } else if longest == bc {
            let m = (b + c) * 0.5;
            stack.extend([[a, b, m], [a, m, c]]);
        } else {
            let m = (c + a) * 0.5;
            stack.extend([[a, b, m], [m, b, c]]);
        }
    }
}

/// Where the ray from `o` along `d` meets the triangle (Möller-Trumbore), as its distance.
fn ray_hit(t: &[Vec3; 3], o: Vec3, d: Vec3) -> Option<f32> {
    let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let s = o - t[0];
    let u = s.dot(p) / det;
    let q = s.cross(e1);
    let v = d.dot(q) / det;
    let at = e2.dot(q) / det;
    (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && at > 0.0).then_some(at)
}

/// Why a candidate is not clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    Ball,
    Scenery,
    Module,
    Sight,
    Foot,
    Support,
}

impl Space {
    pub fn new(level: &Level) -> Space {
        let mut s = Space {
            tris: Vec::new(),
            grid: HashMap::new(),
            wide: Vec::new(),
            keeps: Vec::new(),
            rolls: Vec::new(),
            lowest: f32::MAX,
        };
        let mut world = Vec::new();
        for (i, o) in level.objects.iter().enumerate() {
            let Some(mesh) = o.mesh.and_then(|m| level.meshes.get(m)) else {
                continue;
            };
            let matrix = Mat4::from_cols_array_2d(&o.matrix);
            if !finite(&matrix) {
                continue;
            }
            let rail = in_group(level, i, &["Phys_FloorRails"]);
            let ball = rail || in_group(level, i, &["Phys_Floors"]);
            let stand_in = crate::scene::module_of(level, i);
            let marker = level.groups_of(i).any(crate::scene::is_marker_group);
            world.clear();
            triangles(mesh, &matrix, &mut world);
            if !ball && (stand_in.is_some() || marker) {
                s.keep(i, stand_in, &matrix, &world);
                continue;
            }
            // scenery: what is seen (and the stoppers, seen or not); the cloud layer is no thing to run into
            let scenery = (o.visible
                && mesh.parts.iter().any(|p| p.material.is_some())
                && o.name != "SkyLayer")
                || in_group(level, i, &["Phys_FloorStopper"]);
            if !ball && !scenery {
                continue;
            }
            for &p in &world {
                s.add(p, ball, rail);
            }
        }
        s
    }

    fn add(&mut self, p: [Vec3; 3], ball: bool, rail: bool) {
        let Ok(id) = u32::try_from(self.tris.len()) else {
            return;
        };
        let lo = p[0].min(p[1]).min(p[2]);
        let hi = p[0].max(p[1]).max(p[2]);
        if ball {
            self.lowest = self.lowest.min(lo.y);
            // the ball rolls over what faces up
            let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            if n.y.abs() > 0.7 && self.rolls.len() < 200_000 {
                let mut pts = Vec::new();
                sample(p, 4.0, f32::MIN, &mut pts);
                self.rolls
                    .extend(pts.iter().map(|q| *q + Vec3::Y * BALL_RADIUS));
            }
        }
        let (x0, x1) = (cell(lo.x - REACH), cell(hi.x + REACH));
        let (z0, z1) = (cell(lo.z - REACH), cell(hi.z + REACH));
        let cells = (i64::from(x1) - i64::from(x0) + 1) * (i64::from(z1) - i64::from(z0) + 1);
        if cells > MAX_CELLS {
            self.wide.push(id);
        } else {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    self.grid.entry((x, z)).or_default().push(id);
                }
            }
        }
        let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
        let up = if ball {
            LIFTS.iter().copied().fold(0.0, f32::max)
        } else {
            0.0
        };
        self.tris.push(Tri {
            p,
            ball,
            steep: ball && !rail && n.y.abs() < STEEP,
            lo: lo - Vec3::splat(REACH),
            hi: hi + Vec3::new(REACH, REACH + up, REACH),
        });
    }

    /// What a stand-in's module needs free about it.
    fn keep(&mut self, i: usize, module: Option<&str>, matrix: &Mat4, world: &[[Vec3; 3]]) {
        let (lo, hi) = world.iter().flatten().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(lo, hi), p| (lo.min(*p), hi.max(*p)),
        );
        let (centre, half) = if lo.x <= hi.x {
            ((lo + hi) * 0.5, (hi - lo).length() * 0.5)
        } else {
            (matrix.w_axis.truncate(), 0.0)
        };
        match module {
            Some(m) if m.eq_ignore_ascii_case(MODULE) => {
                let axes = [
                    matrix.x_axis.truncate().normalize_or_zero(),
                    matrix.y_axis.truncate().normalize_or_zero(),
                    matrix.z_axis.truncate().normalize_or_zero(),
                ];
                // (the joint's place comes with the module: `plan` sets it; until then the stand-in's top)
                self.keeps
                    .push(Keep::Swing(Vec3::new(centre.x, hi.y, centre.z), axes, i));
            }
            Some(m) if m.eq_ignore_ascii_case(FAN) => {
                self.keeps.push(Keep::Column(
                    Vec3::new(centre.x, lo.y - MODULE_AIR, centre.z),
                    FAN_RADIUS,
                    FAN_UP,
                ));
            }
            Some(_) => self
                .keeps
                .push(Keep::Sphere(centre, (half + MODULE_AIR).max(MODULE_LEAST))),
            None => self.keeps.push(Keep::Sphere(centre, half + MARKER_AIR)),
        }
    }

    /// What matters about the sack hanging at `joint`: the modules' places a support of it could reach, and where
    /// the ball rolls near it.
    fn about(&self, joint: Vec3, reach: f32) -> About {
        let flat = |v: Vec3| Vec3::new(v.x - joint.x, 0.0, v.z - joint.z).length();
        About {
            keeps: (0..self.keeps.len())
                .filter(|&i| match self.keeps.get(i) {
                    Some(Keep::Sphere(c, r)) | Some(Keep::Column(c, r, _)) => flat(*c) < reach + r,
                    Some(Keep::Swing(c, ..)) => flat(*c) < reach + SWING_REACH,
                    None => false,
                })
                .collect(),
            wide: self
                .wide
                .iter()
                .copied()
                .filter(|&i| {
                    self.tris.get(i as usize).is_some_and(|t| {
                        let (lo, hi) = (
                            t.p[0].min(t.p[1]).min(t.p[2]),
                            t.p[0].max(t.p[1]).max(t.p[2]),
                        );
                        let r = reach + REACH;
                        lo.x - r < joint.x
                            && joint.x < hi.x + r
                            && lo.z - r < joint.z
                            && joint.z < hi.z + r
                    })
                })
                .collect(),
            rolls: self
                .rolls
                .iter()
                .copied()
                .filter(|b| flat(*b) < SIGHT_NEAR)
                .collect(),
        }
    }

    /// The triangles that may be within reach of `p`.
    fn near<'a>(&'a self, p: Vec3, about: &'a About) -> impl Iterator<Item = &'a Tri> + 'a {
        self.grid
            .get(&(cell(p.x), cell(p.z)))
            .into_iter()
            .flatten()
            .chain(about.wide.iter())
            .filter_map(|&i| self.tris.get(i as usize))
    }

    /// The distance from `p` to the ball's space (over the floors and rails near it; `REACH` and more when none is
    /// near), or what `p` is too near to. `wide`: `p` stands for everything this near it (a shaft's axis for its
    /// skin). `own`: the sacks whose holders the support begins at. `wall`: where a wall plate sits and the beam's
    /// direction to it (`on_plate`).
    fn point(
        &self,
        p: Vec3,
        own: [usize; 2],
        wall: Wall,
        about: &About,
        wide: f32,
    ) -> Result<f32, Fault> {
        let mut least = f32::MAX;
        for t in self.near(p, about) {
            if p.cmplt(t.lo).any() || p.cmpgt(t.hi).any() {
                continue;
            }
            if on_plate(t, p, wall, 0.0) {
                continue;
            }
            if t.ball {
                for lift in LIFTS {
                    let q = p - Vec3::Y * lift;
                    let d = (closest_point(&t.p, q) - q).length() - BALL_RADIUS;
                    least = least.min(d);
                    if d < AIR + SLACK + wide {
                        return Err(Fault::Ball);
                    }
                }
            } else if (closest_point(&t.p, p) - p).length() < SOLID_AIR + SLACK * 0.5 + wide {
                return Err(Fault::Scenery);
            }
        }
        for k in about.keeps.iter().filter_map(|&i| self.keeps.get(i)) {
            let inside = match *k {
                Keep::Sphere(c, r) => (p - c).length() < r + wide,
                Keep::Column(foot, r, up) => {
                    let v = p - foot;
                    v.y > 0.0 && v.y < up && Vec3::new(v.x, 0.0, v.z).length() < r + wide
                }
                Keep::Swing(joint, [across, up, along], object) => {
                    let v = p - joint;
                    let down = -v.dot(up);
                    let mine = own.contains(&object);
                    let air = wide + if mine { OWN_SWING_AIR } else { SWING_AIR };
                    let out = (v.dot(across) / SWING_ACROSS).hypot(v.dot(along) / SWING_ALONG);
                    // (its own support begins at the joint)
                    let begins = mine && v.length() < 1.5;
                    !begins && v.length() < SWING_REACH && down > -air && out < down + air
                }
            };
            if inside {
                return Err(Fault::Module);
            }
        }
        Ok(least)
    }

    /// Whether the level's own geometry is at a holder's joint: the map hangs that sack on something itself.
    fn holds(&self, joint: Vec3) -> bool {
        let at = joint + Vec3::Y * 0.13;
        let near = self
            .grid
            .get(&(cell(at.x), cell(at.z)))
            .into_iter()
            .flatten()
            .chain(&self.wide);
        near.filter_map(|&i| self.tris.get(i as usize)).any(|t| {
            !at.cmplt(t.lo).any()
                && !at.cmpgt(t.hi).any()
                && (closest_point(&t.p, at) - at).length() < HELD
        })
    }

    /// Whether a shaft (its axis from `top` to `foot`, `half` wide) stands in the usual camera's sight of the ball
    /// rolling near the sack.
    fn hides(top: Vec3, foot: Vec3, half: f32, about: &About) -> bool {
        let reach = CAM_BACK + half + SIGHT_AIR;
        about
            .rolls
            .iter()
            .filter(|b| Vec3::new(b.x - top.x, 0.0, b.z - top.z).length() <= reach && b.y < top.y)
            .any(|b| {
                [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                    .iter()
                    .any(|back| {
                        // (the sight from just over the ball: a post that stands under the ball's height hides nothing)
                        let from = *b + (*back * CAM_BACK + Vec3::Y * CAM_UP) * 0.03;
                        let eye = *b + *back * CAM_BACK + Vec3::Y * CAM_UP;
                        segments_distance(top, foot, from, eye) < half + SIGHT_AIR
                    })
            })
    }
}

/// Where a wall plate sits, and the beam's direction to it.
type Wall = Option<(Vec3, Vec3)>;

/// Whether `t` is, where it is nearest `p`, what a wall plate sits on: the scenery within `WALL_OPEN` (and `more`) of
/// its place, or a pillar's side there (`PATCH`).
fn on_plate(t: &Tri, p: Vec3, wall: Wall, more: f32) -> bool {
    wall.is_some_and(|(w, dir)| {
        if !t.ball {
            return (w - p).length() <= WALL_OPEN + more;
        }
        let c = closest_point(&t.p, p) - w;
        let along = c.dot(dir);
        let aside = c - dir * along;
        // (the face itself and the pillar's other sides behind it)
        t.steep && along > -0.5 && along < PILLAR + 0.5 && aside.x.hypot(aside.z) < PATCH + more
    })
}

/// What matters about one sack (`Space::about`).
struct About {
    keeps: Vec<usize>,
    /// The wide triangles (those the grid does not hold) that reach near the sack.
    wide: Vec<u32>,
    rolls: Vec<Vec3>,
}

/// The least distance between the segments a-b and c-d (Ericson, Real-Time Collision Detection, 5.1.9).
fn segments_distance(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> f32 {
    let (d1, d2, r) = (b - a, d - c, a - c);
    let (aa, ee, f) = (d1.length_squared(), d2.length_squared(), d2.dot(r));
    let (s, t);
    if aa <= 1e-9 && ee <= 1e-9 {
        return r.length();
    }
    if aa <= 1e-9 {
        s = 0.0;
        t = (f / ee).clamp(0.0, 1.0);
    } else {
        let c1 = d1.dot(r);
        if ee <= 1e-9 {
            t = 0.0;
            s = (-c1 / aa).clamp(0.0, 1.0);
        } else {
            let bb = d1.dot(d2);
            let denom = aa * ee - bb * bb;
            let s0 = if denom > 1e-9 {
                ((bb * f - c1 * ee) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t0 = (bb * s0 + f) / ee;
            if t0 < 0.0 {
                t = 0.0;
                s = (-c1 / aa).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t = 1.0;
                s = ((bb - c1) / aa).clamp(0.0, 1.0);
            } else {
                t = t0;
                s = s0;
            }
        }
    }
    ((a + d1 * s) - (c + d2 * t)).length()
}

/// One piece of a support as it is drawn: the piece (an index into `PIECES`) and its place in the world.
#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub piece: usize,
    pub matrix: Mat4,
}

/// The support chosen for a sack.
#[derive(Debug, Clone)]
pub struct Support {
    /// An index into `DESIGNS`.
    pub design: usize,
    pub length: f32,
    /// The beam's direction about the holder's vertical, degrees from its x.
    pub yaw: f32,
    /// The least distance from the support to the ball's space over the floors and rails (capped at the tests' reach).
    pub clearance: f32,
    /// Its material (`WOOD`, `IRON`, `TUBE`), where a wall plate sits, and the sack a link's cross-beam goes to.
    material: usize,
    wall: Wall,
    to: Option<usize>,
    pub pieces: Vec<Piece>,
    /// The points it was tested at.
    points: Vec<Vec3>,
}

/// A sack and what was found for it.
pub struct Sack {
    /// The level object (the stand-in).
    pub object: usize,
    /// The holder is at the level's own geometry: it hangs on that, and gets no support.
    pub held: bool,
    /// The support with one design for all (the first clear of `DESIGNS`), and with the designs varied.
    pub plain: Option<Support>,
    pub varied: Option<Support>,
    /// Why each design has no clear candidate (the last fault seen), for the report.
    pub faults: [Option<Fault>; ND],
}

/// The supports of a level's sacks.
#[derive(Default)]
pub struct Plan {
    pub sacks: Vec<Sack>,
    /// How long the search took, in seconds.
    pub took: f32,
    /// The module's model (an index into the modules) and each piece's mesh in it.
    model: usize,
    meshes: [Option<usize>; N],
    /// Each piece's own place in the module (its object's matrix).
    nodes: [Mat4; N],
}

/// The pieces of the module's file: each one's triangles in the module's frame, and its mesh.
struct Kit {
    tris: [Vec<[Vec3; 3]>; N],
    /// The triangles the search tests (`SMALL`).
    test: [Vec<[Vec3; 3]>; N],
    meshes: [Option<usize>; N],
    nodes: [Mat4; N],
    joint: Vec3,
    /// Each beam's middle (y, z) and its half thickness about it.
    beams: [(f32, f32, f32); 3],
    /// Each post piece's shaft (what of it is well under the beam): its axis (x, z), its top and its half width.
    shafts: [Option<(Vec3, f32)>; N],
}

impl Kit {
    fn new(module: &Level) -> Option<Kit> {
        let group = module.group(&format!("Supports_{MODULE}"))?;
        let joint = module
            .object(JOINT)
            .map(|(_, o)| Vec3::from(o.position()))?;
        let mut kit = Kit {
            tris: Default::default(),
            test: Default::default(),
            meshes: [None; N],
            nodes: [Mat4::IDENTITY; N],
            joint,
            beams: [(0.0, 0.0, 0.0); 3],
            shafts: [None; N],
        };
        for (k, name) in PIECES.iter().enumerate() {
            let found = module
                .object(&format!("{MODULE}_Support_{name}"))
                .filter(|(i, _)| group.members.contains(i));
            // (a part without the plain shafts has posts of one length)
            let Some((_, o)) = found else {
                if k >= FIRST_SHAFT {
                    continue;
                }
                return None;
            };
            let mesh = o.mesh?;
            let matrix = Mat4::from_cols_array_2d(&o.matrix);
            if !finite(&matrix) {
                return None;
            }
            let (tris, meshes) = (kit.tris.get_mut(k)?, kit.meshes.get_mut(k)?);
            triangles(module.meshes.get(mesh)?, &matrix, tris);
            let large = |t: &&[Vec3; 3]| (0..3).any(|k| (t[k] - t[(k + 1) % 3]).length() >= SMALL);
            *kit.test.get_mut(k)? = tris.iter().filter(large).copied().collect();
            *meshes = Some(mesh);
            *kit.nodes.get_mut(k)? = matrix;
            if let Some(beam) = kit.beams.get_mut(k) {
                let (lo, hi) = tris.iter().flatten().fold(
                    (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                    |(lo, hi), p| (lo.min(*p), hi.max(*p)),
                );
                let (mid, half) = ((lo + hi) * 0.5, (hi - lo) * 0.5);
                *beam = (mid.y, mid.z, half.y.hypot(half.z));
            }
            // the shaft: what lies more than 5 under the joint
            let (lo, hi) = tris.iter().flatten().filter(|p| p.y < joint.y - 5.0).fold(
                (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                |(lo, hi), p| (lo.min(*p), hi.max(*p)),
            );
            if lo.x <= hi.x {
                let top = tris.iter().flatten().map(|p| p.y).fold(f32::MIN, f32::max);
                let half = (hi.x - lo.x).max(hi.z - lo.z) * 0.5;
                *kit.shafts.get_mut(k)? = Some((
                    Vec3::new((lo.x + hi.x) * 0.5, top, (lo.z + hi.z) * 0.5),
                    half,
                ));
            }
        }
        Some(kit)
    }

    /// The lowest point of a piece (its foot).
    fn foot(&self, piece: usize) -> f32 {
        self.tris
            .get(piece)
            .into_iter()
            .flatten()
            .flatten()
            .map(|p| p.y)
            .fold(f32::MAX, f32::min)
    }
}

/// A candidate's pieces: the holder's mount and the beam `length` long towards `yaw`, of the `material`, then the
/// design's post at the beam's far end (a gate's second post at the other side, turned back).
fn pieces(
    place: &Mat4,
    yaw: f32,
    length: f32,
    material: usize,
    design: usize,
    shafts: usize,
    shaft: f32,
) -> Vec<Piece> {
    let turn = *place * Mat4::from_rotation_y(yaw.to_radians());
    let far = turn * Mat4::from_translation(Vec3::X * length);
    let beam = |m: Mat4| Piece {
        piece: material,
        matrix: m * Mat4::from_scale(Vec3::new(length, 1.0, 1.0)),
    };
    // (the timber head lies along the holder's plate; the others turn with their beam)
    let mut out = vec![beam(turn)];
    if design != BARE {
        out.push(Piece {
            piece: MOUNT + material,
            matrix: if material == WOOD { *place } else { turn },
        });
    }
    // a post, and under it as many plain shafts as the level is deep
    let stand = |out: &mut Vec<Piece>, post: usize, at: Mat4| {
        out.push(Piece {
            piece: post,
            matrix: at,
        });
        for k in 1..=shafts {
            out.push(Piece {
                piece: post + SHAFT,
                matrix: at * Mat4::from_translation(Vec3::NEG_Y * shaft * k as f32),
            });
        }
    };
    if design == TO_WALL {
        out.push(Piece {
            piece: WALL_PIECE,
            matrix: far,
        });
    } else if let Some(&post) = POST_PIECE.get(design) {
        stand(&mut out, post, far);
        if design == GATE {
            let back = turn * Mat4::from_rotation_y(std::f32::consts::PI);
            out.push(beam(back));
            stand(
                &mut out,
                post,
                back * Mat4::from_translation(Vec3::X * length),
            );
        }
    }
    out
}

/// Tests placed pieces at the points of their meshes: the least distance to the ball's space.
fn clear(
    space: &Space,
    kit: &Kit,
    about: &About,
    parts: &[Piece],
    own: [usize; 2],
    wall: Wall,
    points: &mut Vec<Vec3>,
) -> Result<f32, Fault> {
    let mut clearance = REACH;
    // the beams first, along their middles, as thick as they are; then the posts, the mounts last (the same for
    // many candidates, and seldom what is in the way)
    for p in parts.iter().filter(|p| p.piece < MOUNT) {
        let Some((y, z, half)) = kit.beams.get(p.piece).copied() else {
            continue;
        };
        let (a, b) = (
            p.matrix.transform_point3(Vec3::new(0.0, y, z)),
            p.matrix.transform_point3(Vec3::new(1.0, y, z)),
        );
        let steps = ((b - a).length() / SPACING).ceil().clamp(1.0, 400.0);
        for k in 0..=steps as usize {
            let q = a.lerp(b, k as f32 / steps);
            clearance = clearance.min(space.point(q, own, wall, about, half)? - half);
            points.push(q);
        }
    }
    let posts = parts
        .iter()
        .filter(|p| (WALL_PIECE - 4..FIRST_SHAFT).contains(&p.piece));
    let mounts = parts
        .iter()
        .filter(|p| (MOUNT..MOUNT + 3).contains(&p.piece));
    for p in posts.chain(mounts) {
        let from = points.len();
        for t in kit.test.get(p.piece).into_iter().flatten() {
            let w = [
                p.matrix.transform_point3(t[0]),
                p.matrix.transform_point3(t[1]),
                p.matrix.transform_point3(t[2]),
            ];
            sample(w, SPACING, space.lowest - DEEP, points);
        }
        for &q in points.get(from..).into_iter().flatten() {
            clearance = clearance.min(space.point(q, own, wall, about, 0.0)?);
        }
    }
    Ok(clearance)
}

/// A cross-beam from the holder of the sack `object` (at `place`) to the neighbour `to`'s (its joint at `end`), of
/// the neighbour's material: level, straight from joint to joint.
fn link(
    space: &Space,
    kit: &Kit,
    place: &Mat4,
    object: usize,
    to: usize,
    end: Vec3,
    material: usize,
) -> Result<Support, Fault> {
    let joint = place.transform_point3(kit.joint);
    let (along, up) = (end - joint, place.y_axis.truncate().normalize_or_zero());
    let length = along.length();
    let x = along.normalize_or_zero();
    let z = x.cross(up).normalize_or_zero();
    if !(length > 3.0 && length <= LINK_FAR + 1.0) || z == Vec3::ZERO {
        return Err(Fault::Foot);
    }
    let frame = Mat4::from_cols(
        x.extend(0.0),
        z.cross(x).extend(0.0),
        z.extend(0.0),
        joint.extend(1.0),
    ) * Mat4::from_translation(-kit.joint);
    let mut parts = pieces(&frame, 0.0, length, material, LINK, 0, 0.0);
    if let Some(mount) = parts.get_mut(1).filter(|_| material == WOOD) {
        mount.matrix = *place;
    }
    let about = space.about((joint + end) * 0.5, length * 0.5 + 4.0);
    let mut points = Vec::new();
    let clearance = clear(space, kit, &about, &parts, [object, to], None, &mut points)?;
    let yaw = place.inverse().transform_vector3(x);
    Ok(Support {
        design: LINK,
        length,
        yaw: (-yaw.z).atan2(yaw.x).to_degrees(),
        clearance,
        material,
        wall: None,
        to: Some(to),
        pieces: parts,
        points,
    })
}

/// Tests a candidate; clear, it is the support.
#[allow(clippy::too_many_arguments)]
fn candidate(
    space: &Space,
    kit: &Kit,
    about: &About,
    place: &Mat4,
    object: usize,
    material: usize,
    design: usize,
    yaw: f32,
    length: f32,
    wall: Wall,
) -> Result<Support, Fault> {
    let own = [object, usize::MAX];
    let joint = place.transform_point3(kit.joint);
    // a post goes down into the clouds: its foot well under the lowest floor, on plain shafts where the level is deep
    let (mut shafts, mut shaft) = (0usize, 0.0f32);
    if let Some(&post) = POST_PIECE.get(design) {
        let depth = kit.joint.y - kit.foot(post);
        let need = joint.y - (space.lowest - FOOT_UNDER);
        if need > depth {
            shaft = kit.joint.y - kit.foot(post + SHAFT);
            if !(shaft > 1.0 && shaft.is_finite()) {
                return Err(Fault::Foot);
            }
            shafts = ((need - depth) / shaft).ceil() as usize;
            if shafts > MAX_SHAFTS {
                return Err(Fault::Foot);
            }
        }
    }
    let parts = pieces(place, yaw, length, material, design, shafts, shaft);
    // the cheap test first: a wide shaft in the camera's sight of the ball
    for p in &parts {
        if let Some((axis, half)) = kit.shafts.get(p.piece).copied().flatten()
            && half >= WIDE
            && p.piece < FIRST_SHAFT
        {
            let top = p.matrix.transform_point3(axis);
            let foot = Vec3::new(top.x, space.lowest - DEEP, top.z);
            if Space::hides(top, foot, half, about) {
                return Err(Fault::Sight);
            }
        }
    }
    // then each shaft's axis: where that is too near, its skin is nearer still. A plain shaft (deep under the track,
    // and long) is tested along its axis alone, as wide as it is
    let mut points = Vec::new();
    let mut clearance = REACH;
    for p in &parts {
        if let Some((axis, half)) = kit.shafts.get(p.piece).copied().flatten() {
            let plain = p.piece >= FIRST_SHAFT;
            let wide = if plain {
                half * std::f32::consts::SQRT_2
            } else {
                0.0
            };
            let top = p.matrix.transform_point3(axis);
            let foot = p
                .matrix
                .transform_point3(Vec3::new(axis.x, kit.foot(p.piece), axis.z));
            let steps = ((top - foot).length() / SPACING).ceil().clamp(1.0, 4000.0);
            for k in 0..=steps as usize {
                let q = top.lerp(foot, k as f32 / steps);
                if q.y >= space.lowest - DEEP {
                    let d = space.point(q, own, wall, about, wide)?;
                    if plain {
                        clearance = clearance.min(d - wide);
                        points.push(q);
                    }
                }
            }
        }
    }
    clearance = clearance.min(clear(space, kit, about, &parts, own, wall, &mut points)?);
    Ok(Support {
        design,
        length,
        yaw,
        clearance,
        material,
        wall,
        to: None,
        pieces: parts,
        points,
    })
}

/// The level's triangles a beam from the holder at `place` could come to: those within the farthest wall's distance
/// that reach up to its height.
fn walls<'a>(space: &'a Space, place: &Mat4, kit: &Kit) -> Vec<&'a Tri> {
    let from = place.transform_point3(kit.joint + Vec3::Y * WALL_AT);
    let r = Vec3::splat(WALL_FAR);
    space
        .tris
        .iter()
        .filter(|t| !(from + r).cmplt(t.lo).any() && !(from - r).cmpgt(t.hi).any())
        .collect()
}

/// The nearest wall (scenery, or a pillar's side, facing the beam) of `near` from the holder towards `yaw`: its
/// distance, the point and the beam's direction. A pillar's side must be narrow: a wide steep face of the floors may be
/// one the ball rides.
fn wall(near: &[&Tri], place: &Mat4, kit: &Kit, yaw: f32) -> Option<(f32, Vec3, Vec3)> {
    let from = place.transform_point3(kit.joint + Vec3::Y * WALL_AT);
    let dir = (*place * Mat4::from_rotation_y(yaw.to_radians()))
        .transform_vector3(Vec3::X)
        .normalize_or_zero();
    let first = |from: Vec3| {
        let mut best: Option<(f32, &Tri)> = None;
        for t in near {
            if let Some(at) = ray_hit(&t.p, from, dir)
                && best.is_none_or(|b| at < b.0)
            {
                best = Some((at, t));
            }
        }
        best
    };
    let (at, t) = first(from)?;
    let aside = dir.cross(Vec3::Y).normalize_or_zero() * (PILLAR * 0.5);
    if t.ball
        && [aside, -aside]
            .iter()
            .any(|a| first(from + *a).is_some_and(|(far, s)| s.steep && (far - at).abs() < 1.5))
    {
        return None;
    }
    let n = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or_zero();
    ((!t.ball || t.steep)
        && (WALL_NEAR..=WALL_FAR).contains(&at)
        && n.dot(dir).abs() >= WALL_FACING
        && n.y.abs() < STEEP)
        .then(|| (at, from + dir * at, dir))
}

fn apart(a: &Support, b: &Support) -> bool {
    apart_but(a, b, None)
}

/// Whether two supports keep their distance, but for where both come to one holder (`shared`, its joint).
fn apart_but(a: &Support, b: &Support, shared: Option<Vec3>) -> bool {
    // (the points are many: the boxes first)
    let bounds = |s: &Support| {
        s.points.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(lo, hi), p| (lo.min(*p), hi.max(*p)),
        )
    };
    let ((alo, ahi), (blo, bhi)) = (bounds(a), bounds(b));
    let gap = BETWEEN + SLACK;
    if (alo - bhi).max_element() > gap || (blo - ahi).max_element() > gap {
        return true;
    }
    let near_b = |p: &Vec3| (blo - *p).max_element() <= gap && (*p - bhi).max_element() <= gap;
    let near_a = |p: &Vec3| (alo - *p).max_element() <= gap && (*p - ahi).max_element() <= gap;
    let off = |p: &Vec3| shared.is_none_or(|j| (*p - j).length() > 2.5);
    let bs: Vec<&Vec3> = b.points.iter().filter(|p| near_a(p) && off(p)).collect();
    !a.points
        .iter()
        .filter(|p| near_b(p) && off(p))
        .any(|p| bs.iter().any(|q| (**q - *p).length() < gap))
}

/// Looks for a support for every sack of the level; nothing when the module in use has no pieces (the original's).
pub fn plan(level: &Level, modules: &Modules) -> Plan {
    let began = std::time::Instant::now();
    let Some((model, module)) = modules.get(MODULE) else {
        return Plan::default();
    };
    let Some(kit) = Kit::new(module) else {
        return Plan::default();
    };
    let mut space = Space::new(level);
    let stand_ins: Vec<usize> = (0..level.objects.len())
        .filter(|&i| {
            crate::scene::module_of(level, i).is_some_and(|m| m.eq_ignore_ascii_case(MODULE))
        })
        .collect();
    let place_of = |i: usize| {
        level
            .objects
            .get(i)
            .map(|o| Mat4::from_cols_array_2d(&o.matrix))
            .filter(finite)
    };
    // each sack swings about its joint, which the module's file places
    for k in &mut space.keeps {
        if let Keep::Swing(joint, _, object) = k
            && let Some(m) = place_of(*object)
        {
            *joint = m.transform_point3(kit.joint);
        }
    }
    let mut plan = Plan {
        sacks: Vec::new(),
        took: 0.0,
        model,
        meshes: kit.meshes,
        nodes: kit.nodes,
    };
    // each sack's candidates are looked through on their own: all the sacks at once, on the machine's cores
    type Found = ([Option<Support>; ND], [Option<Fault>; ND]);
    let reach = LENGTHS
        .iter()
        .chain(&MORE_LENGTHS)
        .copied()
        .fold(WALL_FAR, f32::max)
        + 4.0;
    let search = |object: usize, lengths: &[f32], yaws: &[f32], to_walls: bool| -> Found {
        let mut found: [Option<Support>; ND] = Default::default();
        let mut faults = [None; ND];
        if let Some(place) =
            place_of(object).filter(|m| !space.holds(m.transform_point3(kit.joint)))
        {
            let about = space.about(place.transform_point3(kit.joint), reach);
            // a beam that is not clear is not clear longer either: the least length each direction failed at
            let mut beam_bad = vec![(f32::MAX, Fault::Ball); yaws.len()];
            for (design, material) in MATERIAL.iter().copied().enumerate().take(POST_PIECE.len()) {
                'design: for &length in lengths {
                    // (a gate is the same turned half round)
                    for (y, yaw) in yaws
                        .iter()
                        .enumerate()
                        .filter(|(_, y)| design != GATE || y.abs() <= 90.0)
                        // (the second look does not repeat the first)
                        .filter(|(_, y)| {
                            to_walls || !(LENGTHS.contains(&length) && YAWS.contains(y))
                        })
                    {
                        if let Some((_, f)) = beam_bad.get(y).filter(|b| length >= b.0) {
                            if let Some(x) = faults.get_mut(design) {
                                x.get_or_insert(*f);
                            }
                            continue;
                        }
                        // the bare beam first, once for every design: the thickest one, the rail tube
                        if let Err(f) = candidate(
                            &space, &kit, &about, &place, object, TUBE, BARE, *yaw, length, None,
                        ) {
                            if let (Some(b), Some(x)) =
                                (beam_bad.get_mut(y), faults.get_mut(design))
                            {
                                *b = (length, f);
                                x.get_or_insert(f);
                            }
                            continue;
                        }
                        match candidate(
                            &space, &kit, &about, &place, object, material, design, *yaw, length,
                            None,
                        ) {
                            Ok(s) => {
                                if let Some(f) = found.get_mut(design) {
                                    *f = Some(s);
                                }
                                break 'design;
                            }
                            Err(f) => {
                                if let Some(x) = faults.get_mut(design) {
                                    *x = Some(f);
                                }
                            }
                        }
                    }
                }
            }
            // the nearest wall that a clear beam reaches
            let near = if to_walls {
                walls(&space, &place, &kit)
            } else {
                Vec::new()
            };
            let mut walls: Vec<(f32, f32, Vec3, Vec3)> = (0..360)
                .step_by(WALL_STEP)
                .filter(|_| to_walls)
                .filter_map(|yaw| {
                    let yaw = yaw as f32 - 180.0;
                    wall(&near, &place, &kit, yaw).map(|(at, p, dir)| (at, yaw, p, dir))
                })
                .collect();
            walls.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (at, yaw, p, dir) in walls {
                match candidate(
                    &space,
                    &kit,
                    &about,
                    &place,
                    object,
                    IRON,
                    TO_WALL,
                    yaw,
                    at,
                    Some((p, dir)),
                ) {
                    Ok(s) => {
                        found[TO_WALL] = Some(s);
                        break;
                    }
                    Err(f) => faults[TO_WALL] = Some(f),
                }
            }
        }
        (found, faults)
    };
    let sacks: Vec<usize> = stand_ins.iter().copied().take(MAX_SACKS).collect();
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, 8);
    let share = sacks.len().div_ceil(threads).max(1);
    let searched: Vec<Found> = std::thread::scope(|scope| {
        let search = &search;
        let workers: Vec<_> = sacks
            .chunks(share)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|&o| search(o, &LENGTHS, &YAWS, true))
                        .collect::<Vec<Found>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .zip(sacks.chunks(share))
            // (a search that failed leaves its sacks as they are)
            .flat_map(|(w, part)| {
                w.join()
                    .unwrap_or_else(|_| part.iter().map(|_| Default::default()).collect())
            })
            .collect()
    });
    for (ordinal, (&object, (found, mut faults))) in sacks.iter().zip(searched).enumerate() {
        for (f, s) in faults.iter_mut().zip(&found) {
            if s.is_some() {
                *f = None;
            }
        }
        // one design for all: the first clear; varied: a wall where one is near, else the free designs in turn
        let pick = |order: &[usize],
                    placed: &dyn Fn(&Sack) -> Option<&Support>,
                    faults: &mut [Option<Fault>; ND]| {
            for &d in order {
                let Some(s) = found.get(d).and_then(|s| s.as_ref()) else {
                    continue;
                };
                if plan.sacks.iter().filter_map(placed).all(|o| apart(s, o)) {
                    return Some(s.clone());
                }
                if let Some(f) = faults.get_mut(d) {
                    *f = Some(Fault::Support);
                }
            }
            None
        };
        let free = POST_PIECE.len();
        let varied_order: Vec<usize> = std::iter::once(TO_WALL)
            .chain((0..free).map(|k| (k + ordinal) % free))
            .collect();
        let plain = pick(
            &[0, 2, 1, 4, 3, TO_WALL],
            &|s| s.plain.as_ref(),
            &mut faults,
        );
        let varied = pick(&varied_order, &|s| s.varied.as_ref(), &mut faults);
        plan.sacks.push(Sack {
            object,
            held: place_of(object).is_some_and(|m| space.holds(m.transform_point3(kit.joint))),
            plain,
            varied,
            faults,
        });
    }
    link_bare(&mut plan, &space, &kit, &place_of);
    // What is still bare is looked at once more, with longer beams and more directions: each such sack by itself, all
    // at once. What stands already stays as it stands, and a new support keeps its distance from it.
    let bare: Vec<usize> = (0..plan.sacks.len())
        .filter(|&i| {
            plan.sacks
                .get(i)
                .is_some_and(|s| !s.held && (s.plain.is_none() || s.varied.is_none()))
        })
        .collect();
    let lengths: Vec<f32> = LENGTHS.iter().chain(&MORE_LENGTHS).copied().collect();
    let yaws: Vec<f32> = YAWS.iter().chain(&MORE_YAWS).copied().collect();
    let more: Vec<Option<Found>> = std::thread::scope(|scope| {
        let (search, lengths, yaws, sacks) = (&search, &lengths, &yaws, &plan.sacks);
        let workers: Vec<_> = bare
            .iter()
            .map(|&i| {
                scope.spawn(move || sacks.get(i).map(|s| search(s.object, lengths, yaws, false)))
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().ok().flatten())
            .collect()
    });
    let free = POST_PIECE.len();
    for (&i, found) in bare.iter().zip(more) {
        let Some((found, _)) = found else {
            continue;
        };
        for set in [PLAIN, VARIED] {
            if plan.sacks.get(i).is_none_or(|s| in_set(s, set).is_some()) {
                continue;
            }
            let order: Vec<usize> = if set == PLAIN {
                vec![0, 2, 1, 4, 3]
            } else {
                (0..free).map(|k| (k + i) % free).collect()
            };
            // the shortest beam that keeps its distance from what stands (of equals, the design whose turn it is)
            let mut clear: Vec<&Support> = order
                .iter()
                .filter_map(|&d| found.get(d).and_then(|s| s.as_ref()))
                .filter(|s| {
                    plan.sacks
                        .iter()
                        .filter_map(|o| in_set(o, set))
                        .all(|o| apart(s, o))
                })
                .collect();
            clear.sort_by(|a, b| a.length.total_cmp(&b.length));
            let pick = clear.first().map(|s| (*s).clone());
            if let (Some(s), Some(sack)) = (pick, plan.sacks.get_mut(i)) {
                if let Some(f) = sack.faults.get_mut(s.design) {
                    *f = None;
                }
                *(if set == PLAIN {
                    &mut sack.plain
                } else {
                    &mut sack.varied
                }) = Some(s);
            }
        }
    }
    // (and its bare neighbours may now hang on it)
    link_bare(&mut plan, &space, &kit, &place_of);
    plan.took = began.elapsed().as_secs_f32();
    plan
}

/// Hangs each sack left bare on a cross-beam to its nearest neighbour that has a support (or hangs on one itself): a
/// group of sacks on one or two supports. In each set by itself.
fn link_bare(plan: &mut Plan, space: &Space, kit: &Kit, place_of: &dyn Fn(usize) -> Option<Mat4>) {
    let upright = |m: &Mat4| m.y_axis.truncate().normalize_or_zero().y >= LINK_UPRIGHT;
    for set in [PLAIN, VARIED] {
        let mut tried: Vec<(usize, usize)> = Vec::new();
        loop {
            let mut linked: Option<(usize, Support)> = None;
            let mut faults: Vec<(usize, Fault)> = Vec::new();
            'sacks: for (i, sack) in plan.sacks.iter().enumerate() {
                let bare = !sack.held && in_set(sack, set).is_none();
                let Some(place) = place_of(sack.object).filter(|m| bare && upright(m)) else {
                    continue;
                };
                let joint = place.transform_point3(kit.joint);
                let mut near: Vec<(f32, usize, Vec3, usize)> = plan
                    .sacks
                    .iter()
                    .enumerate()
                    .filter_map(|(j, other)| {
                        let end = place_of(other.object)
                            .filter(upright)?
                            .transform_point3(kit.joint);
                        let far = (end - joint).length();
                        let level = (end.y - joint.y).abs() <= LINK_LEVEL.min(far * 0.02);
                        (far <= LINK_FAR && level && !tried.contains(&(i, j))).then_some((
                            far,
                            j,
                            end,
                            in_set(other, set)?.material,
                        ))
                    })
                    .collect();
                near.sort_by(|a, b| a.0.total_cmp(&b.0));
                for (_, j, end, material) in near {
                    let Some(other) = plan.sacks.get(j) else {
                        continue;
                    };
                    tried.push((i, j));
                    match link(space, kit, &place, sack.object, other.object, end, material) {
                        Ok(s) => {
                            // (it comes to its neighbour's holder, as the neighbour's own support and other links do)
                            let free = plan.sacks.iter().all(|k| {
                                in_set(k, set).is_none_or(|o| {
                                    let shared =
                                        k.object == other.object || o.to == Some(other.object);
                                    apart_but(&s, o, shared.then_some(end))
                                })
                            });
                            if free {
                                linked = Some((i, s));
                                break 'sacks;
                            }
                            faults.push((i, Fault::Support));
                        }
                        Err(f) => faults.push((i, f)),
                    }
                }
            }
            for (i, f) in faults {
                if let Some(x) = plan.sacks.get_mut(i).and_then(|s| s.faults.get_mut(LINK)) {
                    *x = Some(f);
                }
            }
            let Some((i, s)) = linked else {
                break;
            };
            if let Some(sack) = plan.sacks.get_mut(i) {
                *(if set == PLAIN {
                    &mut sack.plain
                } else {
                    &mut sack.varied
                }) = Some(s);
            }
        }
    }
}

/// A sack's support in a set.
fn in_set(s: &Sack, set: u8) -> Option<&Support> {
    if set == PLAIN {
        s.plain.as_ref()
    } else {
        s.varied.as_ref()
    }
}

/// Which of the two sets a placement is of (`Placement::support`), and what the mod asks for (`set_mode`).
pub const PLAIN: u8 = 1;
pub const VARIED: u8 = 2;

impl Plan {
    /// The supports as things to draw: each piece with the sets it is in.
    pub fn placements(&self) -> Vec<Placement> {
        let mut out = Vec::new();
        for sack in &self.sacks {
            for (set, s) in [(PLAIN, &sack.plain), (VARIED, &sack.varied)] {
                for p in s.iter().flat_map(|s| &s.pieces) {
                    let Some(mesh) = self.meshes.get(p.piece).copied().flatten() else {
                        continue;
                    };
                    out.push(Placement {
                        model: FIRST_MODULE_MODEL + self.model,
                        mesh,
                        matrix: p.matrix
                            * self.nodes.get(p.piece).copied().unwrap_or(Mat4::IDENTITY),
                        marker: None,
                        object: sack.object,
                        body: None,
                        spin: None,
                        trafo_shadow: None,
                        sprite: None,
                        cloud: false,
                        support: set,
                    });
                }
            }
        }
        out
    }
}

/// What a check of a placed support found: its least distances to the floors' and rails' triangles themselves, to
/// the scenery (away from a bracket's wall) and, with a run, to the ball's recorded centres.
#[derive(Debug, Clone, Copy)]
pub struct Checked {
    pub floor: f32,
    pub scenery: f32,
    pub run: f32,
}

/// Measures a placed support again, another way than it was chosen: the pieces' meshes sampled twice as finely
/// against every floor, rail and scenery triangle of the level one by one (no grid, no lifted surfaces), and
/// against the ball's centres of recorded runs.
pub fn check(level: &Level, modules: &Modules, s: &Support, run: &[Vec3]) -> Checked {
    let mut out = Checked {
        floor: f32::MAX,
        scenery: f32::MAX,
        run: f32::MAX,
    };
    let Some(kit) = modules.get(MODULE).and_then(|(_, m)| Kit::new(m)) else {
        return out;
    };
    let space = Space::new(level);
    let mut points = Vec::new();
    for p in &s.pieces {
        for t in kit.tris.get(p.piece).into_iter().flatten() {
            let w = [
                p.matrix.transform_point3(t[0]),
                p.matrix.transform_point3(t[1]),
                p.matrix.transform_point3(t[2]),
            ];
            sample(w, SPACING * 0.5, space.lowest - DEEP, &mut points);
        }
    }
    let (lo, hi) = points.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    for t in &space.tris {
        let (tlo, thi) = (
            t.p[0].min(t.p[1]).min(t.p[2]),
            t.p[0].max(t.p[1]).max(t.p[2]),
        );
        if (tlo - hi).max_element() > 12.0 || (lo - thi).max_element() > 12.0 {
            continue;
        }
        for &q in &points {
            if (tlo - q).max_element() > 12.0 || (q - thi).max_element() > 12.0 {
                continue;
            }
            let d = (closest_point(&t.p, q) - q).length();
            if on_plate(t, q, s.wall, 0.5) {
                continue;
            }
            if t.ball {
                out.floor = out.floor.min(d);
            } else {
                out.scenery = out.scenery.min(d);
            }
        }
    }
    for b in run {
        if (lo - *b).max_element() > 12.0 || (*b - hi).max_element() > 12.0 {
            continue;
        }
        for q in &points {
            out.run = out.run.min((*q - *b).length());
        }
    }
    out
}

/// The least a placed support may measure in `check`: from the floors' and rails' triangles (the ball's size and
/// some air), from the scenery, and from the ball's centre in a recorded run (its radius and some air).
const CHECK_FLOOR: f32 = 2.0 * BALL_RADIUS;
const CHECK_SCENERY: f32 = 0.3;
const CHECK_RUN: f32 = BALL_RADIUS + 1.0;

/// A level loaded as the game loads it, its modules, and the supports planned for its sacks: what the game does
/// when a mod first asks for supports (`Game::show_supports`), and what the check measures.
type Planned = (
    crate::assets::LoadedLevel,
    crate::assets::Data,
    Modules,
    Plan,
);

fn planned(data: &crate::assets::Data, source: &crate::assets::Source) -> Result<Planned, String> {
    let data = data.for_source(source);
    let loaded = data.load(source)?;
    let data = data.with_parts(&loaded.level);
    let modules = crate::scene::load_modules(&loaded.level, &data);
    let plan = plan(&loaded.level, &modules);
    Ok((loaded, data, modules, plan))
}

/// `--supports-check geometry|runs`: plans the supports of the original levels 1 to 13 (with the copy) and of every shipped map,
/// measures each placed one again (`check`), with `runs` for the original levels also against the ball's way in
/// the runs of `hints`, and prints what stands where, what does not and why, by level and by design. An error when a placed
/// support fails a measure.
pub fn run(
    o: &crate::Options,
    data: &crate::assets::Data,
    hints: Option<&std::path::Path>,
) -> Result<(), String> {
    use crate::assets::Source;
    let mut sources: Vec<(String, Source, Vec<std::path::PathBuf>)> = Vec::new();
    if data.has_copy() {
        for n in 1..=13u32 {
            sources.push((
                format!("Level_{n:02}"),
                Source::Original(n),
                hints.map_or_else(Vec::new, |h| crate::levelstats::replays(h, n)),
            ));
        }
    }
    for file in [
        gamedata::resources::shipped_drahtseilakt_root(),
        gamedata::resources::shipped_community_root(),
    ]
    .iter()
    .flatten()
    .flat_map(|r| crate::audit::maps(r))
    {
        let name = file
            .file_stem()
            .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
        sources.push((
            name,
            if file.is_dir() {
                Source::Open(file)
            } else {
                Source::Nmo(file)
            },
            Vec::new(),
        ));
    }
    // per design: placed (varied set), placed (plain set), the least clearances
    let mut by_design = [(0usize, 0usize, f32::MAX, f32::MAX, f32::MAX); ND];
    let (mut sacks, mut bare, mut failed) = (0usize, [0usize; 2], 0usize);
    let (mut held, mut slowest) = (0usize, 0.0f32);
    for (name, source, replays) in sources {
        let began = std::time::Instant::now();
        let (loaded, data, modules, plan) = match planned(data, &source) {
            Ok(p) => p,
            Err(e) => {
                println!("{name}: {e}");
                continue;
            }
        };
        let level = &loaded.level;
        let took = began.elapsed().as_secs_f32();
        if plan.sacks.is_empty() {
            continue;
        }
        // the ball's centres in every recorded run of the level
        let mut way: Vec<Vec3> = Vec::new();
        let mut runs = 0usize;
        for r in replays {
            let mut lo = o.clone();
            lo.source = source.clone();
            lo.replay = Some(r);
            if let Ok(run) = crate::app::stats_run(&lo, &data, None) {
                way.extend(run.ticks.iter().map(|t| t.pos));
                runs = runs.saturating_add(1);
            }
        }
        let placed = |f: &dyn Fn(&Sack) -> bool| plan.sacks.iter().filter(|s| f(s)).count();
        println!(
            "{name}: {} sacks, {} on the level's own, supports {} varied / {} plain, {runs} runs ({} ball positions), \
             loaded and planned in {took:.2} s, planned in {:.2} s",
            plan.sacks.len(),
            placed(&|s| s.held),
            placed(&|s| s.varied.is_some()),
            placed(&|s| s.plain.is_some()),
            way.len(),
            plan.took
        );
        slowest = slowest.max(plan.took);
        for sack in &plan.sacks {
            sacks = sacks.saturating_add(1);
            let (object, at) = level
                .objects
                .get(sack.object)
                .map_or(("", [0.0; 3]), |o| (o.name.as_str(), o.position()));
            let object = format!(
                "{object} (sector {}, at {:.0} {:.0} {:.0})",
                crate::levelstats::sector_of(level, sack.object),
                at[0],
                at[1],
                at[2]
            );
            if sack.held {
                held = held.saturating_add(1);
                println!("  {object}: hangs on the level's own geometry");
                continue;
            }
            for (set, (label, s)) in [("varied", &sack.varied), ("plain", &sack.plain)]
                .into_iter()
                .enumerate()
            {
                let Some(s) = s else {
                    if let Some(b) = bare.get_mut(set) {
                        *b = b.saturating_add(1);
                    }
                    let why: Vec<String> = DESIGNS
                        .iter()
                        .zip(&sack.faults)
                        .map(|(d, f)| {
                            format!(
                                "{d} {}",
                                f.map_or("none near".into(), |f| format!("{f:?}").to_lowercase())
                            )
                        })
                        .collect();
                    println!("  {object} [{label}]: no support ({})", why.join(", "));
                    continue;
                };
                let c = check(level, &modules, s, &way);
                let show = |v: f32| {
                    if v > 12.0 {
                        "  >12".to_owned()
                    } else {
                        format!("{v:5.2}")
                    }
                };
                let ok = c.floor >= CHECK_FLOOR && c.scenery >= CHECK_SCENERY && c.run >= CHECK_RUN;
                if !ok {
                    failed = failed.saturating_add(1);
                }
                if let Some(d) = by_design.get_mut(s.design) {
                    if set == 0 {
                        d.0 = d.0.saturating_add(1);
                    } else {
                        d.1 = d.1.saturating_add(1);
                    }
                    d.2 = d.2.min(c.floor);
                    d.3 = d.3.min(c.scenery);
                    d.4 = d.4.min(c.run);
                }
                println!(
                    "  {object} [{label}]: {} {:.1} long at {:.0} deg; ball space {:.2}; checked: floors {} scenery {} runs {}{}",
                    DESIGNS.get(s.design).copied().unwrap_or(""),
                    s.length,
                    s.yaw,
                    s.clearance,
                    show(c.floor),
                    show(c.scenery),
                    show(c.run),
                    if ok { "" } else { "  FAILED" }
                );
            }
        }
    }
    println!(
        "supports: {sacks} sacks; {held} hang on their level's own geometry; without a support {} varied / {} plain; \
         the slowest level planned in {slowest:.2} s",
        bare[0], bare[1]
    );
    for (d, (varied, plain, floor, scenery, run)) in DESIGNS.iter().zip(by_design) {
        let show = |v: f32| {
            if v > 12.0 {
                ">12".to_owned()
            } else {
                format!("{v:.2}")
            }
        };
        println!(
            "  {d}: {varied} varied / {plain} plain; least distance to floors {} scenery {} recorded ball {}",
            show(floor),
            show(scenery),
            show(run)
        );
    }
    if failed > 0 {
        return Err(format!(
            "{failed} placed supports are too near the ball's space"
        ));
    }
    if sacks == 0 {
        // (every set of levels has sacks: none found means the models in use bring no supports, or no level loaded)
        return Err(format!(
            "no sack with a support to plan was found: the models in use are \"{}\" (the supports come with the game's \
             own, --models own), the copy {}",
            gamedata::packs::chosen(gamedata::packs::Kind::Models),
            if data.has_copy() {
                "is there"
            } else {
                "is missing"
            }
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use level::{Group, Material, Mesh, Object, Part};

    fn material() -> Material {
        Material {
            name: "m".into(),
            diffuse: [1.0; 4],
            ambient: [1.0; 3],
            emissive: [0.0; 3],
            specular: [0.0; 3],
            power: 0.0,
            texture: None,
            normal: None,
            parallax: 0.0,
            blend: None,
            alpha_cutoff: None,
            z_write: true,
            two_sided: false,
            unlit: false,
            texgen: None,
        }
    }

    /// A box from `lo` to `hi` as an object at `at`, with material 0 or none, in the given groups.
    fn add(
        level: &mut Level,
        name: &str,
        lo: [f32; 3],
        hi: [f32; 3],
        at: [f32; 3],
        with_material: bool,
        groups: &[&str],
    ) {
        let c = |k: usize| {
            [
                if k & 1 == 0 { lo[0] } else { hi[0] },
                if k & 2 == 0 { lo[1] } else { hi[1] },
                if k & 4 == 0 { lo[2] } else { hi[2] },
            ]
        };
        let mesh = level.meshes.len();
        level.meshes.push(Mesh {
            name: format!("{name}_Mesh"),
            positions: (0..8).map(c).collect(),
            parts: vec![Part {
                material: with_material.then_some(0),
                indices: vec![
                    0, 1, 3, 0, 3, 2, 4, 7, 5, 4, 6, 7, 0, 5, 1, 0, 4, 5, 2, 3, 7, 2, 7, 6, 0, 2,
                    6, 0, 6, 4, 1, 5, 7, 1, 7, 3,
                ],
            }],
            ..Mesh::default()
        });
        let object = level.objects.len();
        level.objects.push(Object {
            parent: None,
            name: name.into(),
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [at[0], at[1], at[2], 1.0],
            ],
            mesh: Some(mesh),
            visible: true,
            sprite: None,
        });
        for g in groups {
            match level.groups.iter_mut().find(|x| x.name == *g) {
                Some(x) => x.members.push(object),
                None => level.groups.push(Group {
                    name: (*g).into(),
                    members: vec![object],
                }),
            }
        }
    }

    /// The sack's module as our part has it: the joint 15 over the stand-in, and the five pieces (`supports` false:
    /// as the original's file, without them).
    fn modules(supports: bool) -> Modules {
        let mut m = Level::default();
        m.materials.push(material());
        add(
            &mut m,
            JOINT,
            [0.0; 3],
            [0.0; 3],
            [0.0, 15.0, 0.0],
            false,
            &[],
        );
        if supports {
            let g = format!("Supports_{MODULE}");
            let at = [0.0, 15.0, 0.0];
            let name = |k: usize| format!("{MODULE}_Support_{}", PIECES[k]);
            // a beam and a mount of each material, the four posts, the wall plate
            for k in 0..3 {
                add(
                    &mut m,
                    &name(k),
                    [0.0, 0.3, -0.2],
                    [1.0, 0.7, 0.2],
                    at,
                    true,
                    &[&g],
                );
                add(
                    &mut m,
                    &name(MOUNT + k),
                    [-0.8, 0.3, -1.5],
                    [0.8, 0.8, 1.5],
                    at,
                    true,
                    &[&g],
                );
            }
            add(
                &mut m,
                &name(6),
                [0.0, -240.0, -1.0],
                [2.0, 2.0, 1.0],
                at,
                true,
                &[&g],
            );
            add(
                &mut m,
                &name(7),
                [0.0, -240.0, -0.4],
                [0.8, 2.0, 0.4],
                at,
                true,
                &[&g],
            );
            add(
                &mut m,
                &name(8),
                [0.0, -240.0, -0.3],
                [0.6, 1.0, 0.3],
                at,
                true,
                &[&g],
            );
            add(
                &mut m,
                &name(9),
                [0.6, -240.0, -0.3],
                [1.2, 0.9, 0.3],
                at,
                true,
                &[&g],
            );
            add(
                &mut m,
                &name(WALL_PIECE),
                [-0.1, -0.3, -0.6],
                [0.0, 1.3, 0.6],
                at,
                true,
                &[&g],
            );
        }
        Modules {
            models: vec![m],
            names: vec![MODULE.into()],
        }
    }

    /// A track 6 wide along z with its top at 0, and a sack hanging over its middle (the stand-in 3 over the floor).
    fn track() -> Level {
        let mut level = Level::default();
        level.materials.push(material());
        add(
            &mut level,
            "Floor",
            [-3.0, -1.0, -60.0],
            [3.0, 0.0, 60.0],
            [0.0; 3],
            true,
            &["Phys_Floors"],
        );
        add(
            &mut level,
            "P_Modul_26_01",
            [-1.0, -2.0, -1.0],
            [1.0, 15.0, 1.0],
            [0.0, 3.0, 0.0],
            false,
            &[MODULE],
        );
        level
    }

    /// Every point of a support is outside the ball's space: more than the ball and the air from the floor, and
    /// over a floor more than the hop's height besides.
    fn assert_clear(level: &Level, modules: &Modules, s: &Support) {
        let c = check(level, modules, s, &[]);
        assert!(c.floor >= BALL_RADIUS + AIR, "{} from the floor", c.floor);
        for p in &s.points {
            let over_floor = p.x.abs() < 3.0 + BALL_RADIUS && p.z.abs() < 60.0 + BALL_RADIUS;
            assert!(
                !over_floor || p.y > 9.0 + BALL_RADIUS + AIR || p.y < -BALL_RADIUS - AIR,
                "{p} is in the ball's way"
            );
        }
    }

    #[test]
    fn a_sack_over_a_narrow_track_gets_a_post_beside_it() {
        let (level, modules) = (track(), modules(true));
        let plan = plan(&level, &modules);
        assert_eq!(plan.sacks.len(), 1);
        let sack = &plan.sacks[0];
        let (plain, varied) = (
            sack.plain.as_ref().expect("plain"),
            sack.varied.as_ref().expect("varied"),
        );
        // nothing is in the way: the first design, on the shortest beam, straight across the swing
        assert_eq!(DESIGNS[plain.design], "column");
        assert_eq!(plain.yaw.abs() % 180.0, 0.0);
        assert_clear(&level, &modules, plain);
        assert_clear(&level, &modules, varied);
        // drawn only in the set it is of, and never without the mod asking
        let placements = plan.placements();
        assert!(
            !placements.is_empty()
                && placements
                    .iter()
                    .all(|p| p.support != 0 && p.body.is_none())
        );
    }

    #[test]
    fn a_sack_in_the_middle_of_a_wide_floor_stays_as_it_is() {
        let mut level = track();
        add(
            &mut level,
            "Plaza",
            [-60.0, -1.0, -60.0],
            [60.0, 0.0, 60.0],
            [0.0; 3],
            true,
            &["Phys_Floors"],
        );
        let plan = plan(&level, &modules(true));
        assert!(plan.sacks[0].plain.is_none() && plan.sacks[0].varied.is_none());
        assert!(plan.placements().is_empty());
    }

    #[test]
    fn a_wall_near_by_takes_the_beam() {
        let mut level = track();
        // scenery (no floor, no rail): a tower 10 beside the track
        add(
            &mut level,
            "Tower",
            [10.0, -50.0, -4.0],
            [16.0, 40.0, 4.0],
            [0.0; 3],
            true,
            &[],
        );
        let modules = modules(true);
        let plan = plan(&level, &modules);
        let varied = plan.sacks[0].varied.as_ref().expect("varied");
        assert_eq!(DESIGNS[varied.design], "wall");
        assert!((varied.length - 10.0).abs() < 0.01 && varied.yaw == 0.0);
        assert_clear(&level, &modules, varied);
    }

    #[test]
    fn other_modules_keep_their_room() {
        let mut level = track();
        // a fan on a side floor at +x: its draught is no place for a post
        add(
            &mut level,
            "P_Modul_18_01",
            [-1.0, 0.0, -1.0],
            [1.0, 2.0, 1.0],
            [16.0, 0.0, 0.0],
            false,
            &["P_Modul_18"],
        );
        let plan = plan(&level, &modules(true));
        let s = plan.sacks[0].plain.as_ref().expect("plain");
        for p in &s.points {
            assert!(
                Vec3::new(p.x - 16.0, 0.0, p.z).length() >= FAN_RADIUS,
                "{p} is in the fan's draught"
            );
        }
        // and a second sack's swing is left free by the first one's support
        let mut level = track();
        add(
            &mut level,
            "P_Modul_26_02",
            [-1.0, -2.0, -1.0],
            [1.0, 15.0, 1.0],
            [0.0, 3.0, 12.0],
            false,
            &[MODULE],
        );
        let plan = super::plan(&level, &modules(true));
        assert_eq!(plan.sacks.len(), 2);
        let (a, b) = (
            plan.sacks[0].plain.as_ref().expect("first"),
            plan.sacks[1].plain.as_ref().expect("second"),
        );
        assert!(apart(a, b));
        for p in &a.points {
            let v = *p - Vec3::new(0.0, 18.0, 12.0);
            let out = (v.x / SWING_ACROSS).hypot(v.z / SWING_ALONG);
            assert!(
                v.length() >= SWING_REACH || v.y >= 0.0 || out >= -v.y,
                "{p} is in the other sack's swing"
            );
        }
    }

    #[test]
    fn a_sack_with_no_room_hangs_on_a_cross_beam_to_its_neighbour() {
        let mut level = track();
        // a second sack over the middle of a plaza beside the track: no post may stand near it
        add(
            &mut level,
            "Plaza",
            [-60.0, -1.0, 14.0],
            [60.0, 0.0, 134.0],
            [0.0; 3],
            true,
            &["Phys_Floors"],
        );
        add(
            &mut level,
            "P_Modul_26_02",
            [-1.0, -2.0, -1.0],
            [1.0, 15.0, 1.0],
            [0.0, 3.0, 26.0],
            false,
            &[MODULE],
        );
        let modules = modules(true);
        let plan = plan(&level, &modules);
        let (first, second) = (
            plan.sacks[0].varied.as_ref().expect("first"),
            plan.sacks[1].varied.as_ref().expect("second"),
        );
        assert_eq!(DESIGNS[second.design], "link");
        // level, from joint to joint, of the neighbour's material, with the mount of it
        assert!((second.length - 26.0).abs() < 0.01 && second.to == Some(plan.sacks[0].object));
        assert_eq!(second.material, first.material);
        assert!(
            second
                .pieces
                .iter()
                .any(|p| p.piece == MOUNT + first.material)
        );
        assert!(
            second
                .points
                .iter()
                .all(|p| p.y > 18.0 && p.y < 19.0 && p.x.abs() < 2.0)
        );
        assert_clear(&level, &modules, second);
        // a neighbour hanging lower takes no level beam: the sack is looked at once more, and stands on a support of
        // its own with a longer beam or one along its swing, clear as any; its neighbour's stays as it stood
        let (design, length, yaw) = (first.design, first.length, first.yaw);
        level.objects[3].matrix[3][1] = 1.0;
        let plan = super::plan(&level, &modules);
        let (first, second) = (
            plan.sacks[0].varied.as_ref().expect("first"),
            plan.sacks[1].varied.as_ref().expect("second"),
        );
        assert_eq!(
            (first.design, first.length, first.yaw),
            (design, length, yaw)
        );
        assert_ne!(DESIGNS[second.design], "link");
        assert!(second.length > 17.0 || MORE_YAWS.contains(&second.yaw));
        assert!(second.pieces.iter().all(|p| {
            let x = p.matrix.x_axis.truncate().normalize();
            x.y.abs() < 1e-4
        }));
        assert_clear(&level, &modules, second);
        assert!(apart(first, second));
    }

    #[test]
    fn a_pillar_of_the_floors_takes_a_wall_plate_on_its_side_only() {
        let mut level = track();
        // a tall pillar that is of the floors (as the levels' own are), 10 beside the track
        add(
            &mut level,
            "Pillar",
            [10.0, -50.0, -1.5],
            [13.0, 60.0, 1.5],
            [0.0; 3],
            true,
            &["Phys_Floors"],
        );
        let modules = modules(true);
        let plan = plan(&level, &modules);
        let varied = plan.sacks[0].varied.as_ref().expect("varied");
        assert_eq!(DESIGNS[varied.design], "wall");
        assert!((varied.length - 10.0).abs() < 0.01);
        let c = check(&level, &modules, varied, &[]);
        assert!(c.floor >= CHECK_FLOOR, "{c:?}");
        // a low one, whose top the ball could be on, takes none
        let mut level = track();
        add(
            &mut level,
            "Pillar",
            [10.0, -50.0, -1.5],
            [13.0, 16.0, 1.5],
            [0.0; 3],
            true,
            &["Phys_Floors"],
        );
        let plan = super::plan(&level, &modules);
        assert!(
            plan.sacks[0]
                .varied
                .as_ref()
                .is_none_or(|s| DESIGNS[s.design] != "wall")
        );
    }

    #[test]
    fn a_holder_at_the_level_s_own_beam_is_left_to_it() {
        let mut level = track();
        add(
            &mut level,
            "Arch",
            [-8.0, 18.2, -0.5],
            [8.0, 19.0, 0.5],
            [0.0; 3],
            true,
            &[],
        );
        let plan = plan(&level, &modules(true));
        assert!(
            plan.sacks[0].held && plan.sacks[0].plain.is_none() && plan.placements().is_empty()
        );
    }

    #[test]
    fn every_member_is_level_or_upright_and_of_one_material() {
        let plan = plan(&track(), &modules(true));
        let s = plan.sacks[0].varied.as_ref().expect("varied");
        for p in &s.pieces {
            // (the pieces stand as the holder does: no piece is tilted)
            assert!((p.matrix.y_axis.truncate().normalize() - Vec3::Y).length() < 1e-4);
        }
        let beam = s
            .pieces
            .iter()
            .find(|p| p.piece < MOUNT)
            .expect("a beam")
            .piece;
        assert!(s.pieces.iter().any(|p| p.piece == MOUNT + beam) && beam == s.material);
    }

    /// The check's path on the shipped maps with the game's own parts (no copy), up to the first map with a support:
    /// it finds the sacks, plans supports, and those placed pass the second measure.
    #[test]
    fn the_check_finds_the_shipped_maps_sacks() {
        let root = gamedata::resources::shipped_drahtseilakt_root().expect("the shipped levels");
        let data = crate::assets::Data::new(None);
        let (mut sacks, mut placed) = (0, 0);
        for file in crate::audit::maps(&root) {
            // (one map with a support placed is enough: a debug build plans slowly)
            if placed > 0 {
                break;
            }
            let source = if file.is_dir() {
                crate::assets::Source::Open(file.clone())
            } else {
                crate::assets::Source::Nmo(file.clone())
            };
            let (loaded, _, modules, plan) =
                planned(&data, &source).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            sacks += plan.sacks.len();
            for s in plan.sacks.iter().filter_map(|s| s.varied.as_ref()).take(2) {
                placed += 1;
                let c = check(&loaded.level, &modules, s, &[]);
                assert!(
                    c.floor >= CHECK_FLOOR && c.scenery >= CHECK_SCENERY,
                    "{}: {c:?}",
                    file.display()
                );
            }
        }
        assert!(
            sacks > 0 && placed > 0,
            "{sacks} sacks, {placed} supports checked"
        );
    }

    #[test]
    fn the_original_s_module_has_no_supports() {
        let plan = plan(&track(), &modules(false));
        assert!(plan.sacks.is_empty() && plan.placements().is_empty());
    }

    #[test]
    fn a_broken_stand_in_is_left_alone() {
        let mut level = track();
        level.objects[1].matrix[3][0] = f32::NAN;
        let plan = plan(&level, &modules(true));
        assert!(
            plan.sacks
                .iter()
                .all(|s| s.plain.is_none() && s.varied.is_none())
        );
    }
}
