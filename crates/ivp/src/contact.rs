// Port of OpenBallance src/phys/phys_contact.c and the contact point records of phys_contact.h (GPL-3.0; see the LICENSE
// of the repository): the contact points (docs/ivp_contact.md 3, "CP"): the persistent feature pairs a mindist leaves
// behind, their per-update geometry (the four feature cases), their materials, the creation constants, and their
// destruction. The friction systems and the impact solver that consume the points are not ported yet.
//
// A point is in the arena `CollWorld::cps`. Each object keeps its points in a list in creation order (the newest last),
// which is the upstream's list walked from the head.

#![allow(dead_code)] // the friction and impact ports, next, are the consumers of these points

use crate::core::World;
use crate::ledge::{Edge, Ledge};
use crate::math;
use crate::mindist::{Feat, Mindist, ObjCache, ST_BALL, ST_EDGE, ST_FACE, ST_POINT};

type V3 = [f64; 3];
type P3 = [f32; 3];

/// The friction synapse of a point: its object, feature and status (`IvpFricSyn`, 0x14 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FricSyn {
    pub obj: usize,
    pub feat: Option<Feat>,
    pub typ: i16,
}

/// The per-update geometry of a point (`IvpContactInfo`, 2.7): the normal, the arms, the materials and the cores.
#[derive(Clone, Debug, Default)]
pub struct ContactInfo {
    /// The normal from object 0 toward object 1, in world space.
    pub n: P3,
    /// The contact point on object 0's surface, world space.
    pub cp_ws: V3,
    pub obj: [usize; 2],
    pub feat: [Option<Feat>; 2],
    pub mat_friction: [f32; 2],
    pub mat_elasticity: [f32; 2],
    pub elasticity: f32,
    /// Bit 8 set: the left feature (`IVP_CI_LEFT_FEATURE`); bit 9 is its inverse half.
    pub flags: u32,
    pub pushes: i16,
    pub inv_virt_mass: f32,
    pub virt_mass: f32,
    /// The movable cores of the two sides (None for a fixed side).
    pub core: [Option<usize>; 2],
    pub span: [P3; 2],
    pub cp_cs: [P3; 2],
    pub cross_cs: [P3; 2],
    /// The point's index in the system's solve (`index`), set by the solve.
    pub index: i32,
    /// The gap the solve reads (`t->gap`).
    pub gap: f32,
    /// The friction infos of the two sides in the system being solved.
    pub fi: [Option<usize>; 2],
    /// The velocity of the point's relative motion before the impact (the impact's output).
    pub rel: P3,
    /// The rescue speed of the point and its predicted gap at the next PSI (the impact system's).
    pub rescue: f32,
    pub pred_gap: f32,
}

/// A contact point (`IvpContactPoint`, 2.6): its two friction synapses, the spring state and the geometry.
#[derive(Clone, Debug, Default)]
pub struct ContactPoint {
    pub alive: bool,
    pub syn: [FricSyn; 2],
    pub inv_vm_no_dir: f32,
    pub s: [f32; 2],
    pub info: ContactInfo,
    pub mu: f32,
    pub inv_tri_det: f32,
    pub gap: f32,
    /// 1: the feature is to be rechecked on the next update (a new point).
    pub recheck: u8,
    pub keeper: u16,
    pub time: f64,
    /// The friction system this point belongs to.
    pub fs: Option<usize>,
    /// The warm-start key of the solver (`key`): positive after a push, negative after a pull.
    pub key: i32,
    /// The pressure (normal force times the step) and the energy bookkeeping of the friction.
    pub pressure: f32,
    pub old_energy: f32,
    pub destroyed: f32,
}

/// Contact distances (ivp_const.h): the gap a new point starts with, and the keeper's gap (f32 widened as the upstream).
pub const SET_FRICTION_DIST: f32 = f32::from_bits(0x3ca3_d70a);
pub const SET_KEEPER_DIST: f32 = f32::from_bits(0x3cbc_6a7f);
pub(crate) const C_1E19: f64 = 1e-19;

fn dotd(a: &P3, b: &P3) -> f64 {
    (f64::from(a[2]) * f64::from(b[2]) + f64::from(a[1]) * f64::from(b[1])) + f64::from(a[0]) * f64::from(b[0])
}

/// The triangle normal (P1 - P0) x (P2 - P0) of an edge's triangle, object space (`tri_normal`, 0x10021280).
pub(crate) fn tri_normal(l: &Ledge, e: Edge) -> V3 {
    let p0 = l.p(e);
    let p1 = l.p(Ledge::next(e));
    let p2 = l.p(Ledge::prev(e));
    let a = [f64::from(p1[0]) - f64::from(p0[0]), f64::from(p1[1]) - f64::from(p0[1]), f64::from(p1[2]) - f64::from(p0[2])];
    let b = [f64::from(p2[0]) - f64::from(p0[0]), f64::from(p2[1]) - f64::from(p0[1]), f64::from(p2[2]) - f64::from(p0[2])];
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// A unit vector in f32, unchanged below 1e-19 (`normize_f`, 0x1000e030).
fn normize_f(v: &mut P3) {
    let n = dotd(v, v);
    if n < C_1E19 {
        return;
    }
    let y = math::rsqrt(n);
    for c in v.iter_mut() {
        *c = (f64::from(*c) * y) as f32;
    }
}

/// The world point of an edge's start, in f64 (`world_point`, 0x100217d0).
fn world_point(c: &ObjCache, l: &Ledge, e: Edge) -> V3 {
    let p = l.p(e);
    let x = math::rmul(&c.r, &[f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]);
    [x[0] + c.vv[0], x[1] + c.vv[1], x[2] + c.vv[2]]
}

pub(crate) fn left_feature(t: &mut ContactInfo) {
    t.flags = (t.flags & !0x200) | 0x100;
}

/// POINT or BALL against POINT or BALL (`point_point`, 0x1001f050).
fn point_point(cp: &mut ContactPoint, p0: V3, p1: V3, t: &mut ContactInfo) {
    let mut d = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let len = math::normalize_len(&mut d);
    cp.gap = len as f32;
    for k in 0..3 {
        t.n[k] = d[k] as f32;
    }
    let n = t.n;
    let (ax, az): (f32, f32) = if n[0] * n[0] < 0.9 { (1.0, 0.0) } else { (0.0, 1.0) };
    t.span[0] = [n[1] * az, n[2] * ax - n[0] * az, 0.0 - n[1] * ax];
    normize_f(&mut t.span[0]);
    t.cp_ws = p0;
}

/// POINT or BALL against EDGE (`point_edge`, 0x1001f160).
fn point_edge(cp: &mut ContactPoint, p0: V3, l: &Ledge, g1: Edge, c1: &ObjCache, t: &mut ContactInfo) {
    let a = world_point(c1, l, g1);
    let b = world_point(c1, l, Ledge::next(g1));
    let mut e = [0.0f32; 3];
    let mut r = [0.0f32; 3];
    for k in 0..3 {
        e[k] = (b[k] - a[k]) as f32;
        r[k] = (p0[k] - a[k]) as f32;
    }
    let inv_len = math::isqrt5(f64::from(e[2]) * f64::from(e[2]) + f64::from(e[1]) * f64::from(e[1]) + f64::from(e[0]) * f64::from(e[0]));
    let mut c = [
        r[2] * e[1] - e[2] * r[1],
        e[2] * r[0] - r[2] * e[0],
        r[1] * e[0] - e[1] * r[0],
    ];
    let dist = dotd(&c, &c).sqrt() * inv_len;
    cp.gap = dist as f32;
    if dist * dist <= C_1E19 {
        c = [1.0, 0.0, 0.0];
    } else {
        let k = inv_len / dist;
        for v in c.iter_mut() {
            *v = (f64::from(*v) * k) as f32;
        }
    }
    t.n = [c[2] * e[1] - c[1] * e[2], e[2] * c[0] - c[2] * e[0], c[1] * e[0] - e[1] * c[0]];
    normize_f(&mut t.n);
    t.cp_ws = p0;
    t.span[0] = c;
    if cp.recheck == 1 {
        cp.recheck = 0;
        let u = (f64::from(e[2]) * f64::from(r[2]) + f64::from(r[1]) * f64::from(e[1]) + f64::from(r[0]) * f64::from(e[0])) * inv_len * inv_len;
        if u < 0.0 || !(u <= 1.0) {
            left_feature(t);
        }
    }
}

/// POINT or BALL against TRIANGLE (`point_triangle`, 0x1001ee70).
fn point_triangle(cp: &mut ContactPoint, p0: V3, l: &Ledge, g1: Edge, c1: &ObjCache, t: &mut ContactInfo) {
    t.cp_ws = p0;
    let d = [p0[0] - c1.vv[0], p0[1] - c1.vv[1], p0[2] - c1.vv[2]];
    let p_os = math::rtmul(&c1.r, &d);
    if cp.recheck == 1 {
        cp.recheck = 0;
        let w = crate::minimize::tri_qr_vals(l, g1, p_os);
        if w[0].is_sign_negative() || w[1].is_sign_negative() || w[2].is_sign_negative() {
            left_feature(t);
        }
    }
    let nn = tri_normal(l, g1);
    let inv = f64::from(cp.inv_tri_det);
    let n_os = [nn[0] * inv, nn[1] * inv, nn[2] * inv];
    let n_ws = math::rmul(&c1.r, &n_os);
    let p0v = l.p(g1);
    let p1v = l.p(Ledge::next(g1));
    let q0 = p0v.map(f64::from);
    cp.gap = ((p_os[2] * n_os[2] + p_os[1] * n_os[1] + p_os[0] * n_os[0]) - ((q0[1] * n_os[1] + q0[0] * n_os[0]) + q0[2] * n_os[2])) as f32;
    for k in 0..3 {
        t.n[k] = (n_ws[k] * -1.0) as f32;
    }
    let e = [p1v[0] - p0v[0], p1v[1] - p0v[1], p1v[2] - p0v[2]];
    t.span[0] = math::rmul(&c1.r, &e.map(f64::from)).map(|x| x as f32);
}

/// EDGE against EDGE (`edge_edge`, 0x1001f3b0).
fn edge_edge(cp: &mut ContactPoint, l0: &Ledge, g0: Edge, c0: &ObjCache, l1: &Ledge, g1: Edge, c1: &ObjCache, t: &mut ContactInfo) {
    let a0 = world_point(c0, l0, g0);
    let a1 = world_point(c0, l0, Ledge::next(g0));
    let b0 = world_point(c1, l1, g1);
    let b1 = world_point(c1, l1, Ledge::next(g1));
    let mut eb = [b1[0] - b0[0], b1[1] - b0[1], b1[2] - b0[2]];
    let mut ea = [a1[0] - a0[0], a1[1] - a0[1], a1[2] - a0[2]];
    math::normalize4(&mut eb);
    math::normalize4(&mut ea);
    let c = [ea[1] * eb[2] - ea[2] * eb[1], ea[2] * eb[0] - ea[0] * eb[2], ea[0] * eb[1] - ea[1] * eb[0]];
    let cc = c[0] * c[0] + c[1] * c[1] + c[2] * c[2];
    if cc <= 1e-24 {
        // parallel edges (0x100634e8): the keeper distance, an arbitrary normal
        left_feature(t);
        cp.gap = SET_KEEPER_DIST;
        t.cp_ws = a0;
        t.n = [1.0, 0.0, 0.0];
        t.span[0] = [0.0, 1.0, 0.0];
        return;
    }
    let na = [ea[1] * c[2] - ea[2] * c[1], ea[2] * c[0] - ea[0] * c[2], ea[0] * c[1] - ea[1] * c[0]];
    let nb = [eb[1] * c[2] - eb[2] * c[1], eb[2] * c[0] - eb[0] * c[2], eb[0] * c[1] - eb[1] * c[0]];
    let d3 = |a: V3, b: V3| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let tb = (d3(na, b0) - d3(na, a0)) * (1.0 / (d3(na, b0) - d3(na, b1)));
    let ta = (d3(nb, a0) - d3(nb, b0)) * (1.0 / (d3(nb, a0) - d3(nb, a1)));
    let mut pa = [0.0; 3];
    let mut pb = [0.0; 3];
    let mut dd = [0.0; 3];
    for k in 0..3 {
        pa[k] = (1.0 - ta) * a0[k] + ta * a1[k];
        pb[k] = (1.0 - tb) * b0[k] + tb * b1[k];
        dd[k] = pb[k] - pa[k];
    }
    let len = (dd[0] * dd[0] + dd[1] * dd[1] + dd[2] * dd[2]).sqrt();
    if len <= C_1E19 {
        cp.gap = 0.0;
        let k = 1.0 / cc.sqrt();
        for i in 0..3 {
            t.n[i] = (c[i] * k) as f32;
        }
    } else {
        cp.gap = len as f32;
        let k = 1.0 / len;
        for i in 0..3 {
            t.n[i] = (dd[i] * k) as f32;
        }
    }
    for i in 0..3 {
        t.span[0][i] = ea[i] as f32;
    }
    t.cp_ws = pa;
    if cp.recheck == 1 {
        cp.recheck = 0;
        if tb < 0.0 || tb > 1.0 || ta < 0.0 || ta > 1.0 {
            left_feature(t);
        }
    }
}

/// The velocity of a core's point (`ivp_core_point_velocity`, 0x1000bf90): the rotation's cross product in f32, the
/// rotation to world in f64, and the core's speed added.
pub fn core_point_velocity(world_r: &[[f64; 3]; 3], p_cs: &P3, v: &P3, w: &P3) -> P3 {
    let x = [w[1] * p_cs[2] - w[2] * p_cs[1], w[2] * p_cs[0] - w[0] * p_cs[2], w[0] * p_cs[1] - w[1] * p_cs[0]];
    let d = math::rmul(world_r, &[f64::from(x[0]), f64::from(x[1]), f64::from(x[2])]);
    [(d[0] + f64::from(v[0])) as f32, (d[1] + f64::from(v[1])) as f32, (d[2] + f64::from(v[2])) as f32]
}

/// The worst-case virtual mass at a core-space point (`ivp_worst_vm`, 0x1000c200).
pub fn worst_vm(inv_rot_inertia: &P3, inv_mass: f32, p: &P3) -> f64 {
    let a = (p[1] * p[1] + p[2] * p[2]) * inv_rot_inertia[0];
    let b = (p[0] * p[0] + p[2] * p[2]) * inv_rot_inertia[1];
    let c = (p[0] * p[0] + p[1] * p[1]) * inv_rot_inertia[2];
    let mut m = a;
    if b > m {
        m = b;
    }
    if c > m {
        m = c;
    }
    1.0 / (f64::from(m) + f64::from(inv_mass))
}

impl World {
    /// The start point of a feature's edge in its object's ledge: the point identity the upstream compares.
    fn feat_point(&self, obj: usize, f: Feat) -> Option<usize> {
        let l = self.coll.objs.get(obj)?.surface.as_ref()?.ledges.get(f.ledge)?;
        l.tris.get(f.edge.tri).map(|t| t.start[f.edge.slot])
    }

    /// Whether a point's synapse matches a mindist synapse: the same status and feature in the same ledge (`syn_match`,
    /// 0x1001d840). An edge matches the edge itself or its twin; a face, its triangle; a ball, any ball.
    fn syn_match(&self, cs: &FricSyn, ms_status: i16, ms_feat: Option<Feat>) -> bool {
        if cs.typ != ms_status {
            return false;
        }
        match (cs.typ, cs.feat, ms_feat) {
            (ST_BALL, _, _) => true,
            (_, Some(a), Some(b)) => {
                if a.ledge != b.ledge {
                    return false;
                }
                match cs.typ {
                    ST_POINT => self.feat_point(cs.obj, a) == self.feat_point(cs.obj, b),
                    ST_EDGE => {
                        let Some(l) = self.coll.objs.get(cs.obj).and_then(|o| o.surface.as_ref()).and_then(|s| s.ledges.get(a.ledge)) else {
                            return false;
                        };
                        b.edge == a.edge || b.edge == l.opp(a.edge)
                    }
                    ST_FACE => a.edge.tri == b.edge.tri,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// The mindist's features either way round against the point's (`cp_matches`, 0x1001d910).
    fn cp_matches(&self, cp: &ContactPoint, md: &Mindist) -> bool {
        let (m0, m1) = (&md.syn[0], &md.syn[1]);
        let direct = cp.syn[0].obj == m0.obj
            && cp.syn[1].obj == m1.obj
            && self.syn_match(&cp.syn[0], m0.status, m0.feat)
            && self.syn_match(&cp.syn[1], m1.status, m1.feat);
        if direct {
            return true;
        }
        cp.syn[0].obj == m1.obj
            && cp.syn[1].obj == m0.obj
            && self.syn_match(&cp.syn[0], m1.status, m1.feat)
            && self.syn_match(&cp.syn[1], m0.status, m0.feat)
    }
}

impl World {
    /// A new contact point from a mindist's sorted synapses (`cp_new`, 0x1001a8b0).
    fn cp_new(&mut self, md: &Mindist) -> usize {
        let mut cp = ContactPoint {
            alive: true,
            time: self.env.current_time,
            recheck: 1,
            gap: SET_FRICTION_DIST,
            keeper: 20,
            ..ContactPoint::default()
        };
        for k in 0..2 {
            let s = md.syn[md.sorted(k)];
            cp.syn[k] = FricSyn {
                obj: s.obj,
                feat: s.feat,
                typ: s.status,
            };
        }
        if cp.syn[1].typ == ST_FACE {
            if let (Some(f), Some(surface)) = (cp.syn[1].feat, self.coll.objs[cp.syn[1].obj].surface.as_ref()) {
                if let Some(l) = surface.ledges.get(f.ledge) {
                    let n = tri_normal(l, f.edge);
                    cp.inv_tri_det = (1.0 / (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()) as f32;
                }
            }
        }
        let idx = match self.coll.free_cps.pop() {
            Some(i) => {
                self.coll.cps[i] = cp;
                i
            }
            None => {
                self.coll.cps.push(cp);
                self.coll.cps.len() - 1
            }
        };
        for k in 0..2 {
            let obj = self.coll.cps[idx].syn[k].obj;
            self.coll.objs[obj].fric.push((idx, k));
        }
        idx
    }

    /// The contact point of a mindist: the one the object already has for the same features, or a new one
    /// (`ivp_cp_find_or_create`, 0x10020030). Returns (index, created).
    pub(crate) fn cp_find_or_create(&mut self, md: &Mindist) -> (usize, bool) {
        let o1 = md.syn[1].obj;
        let found = self.coll.objs[md.syn[0].obj]
            .fric
            .iter()
            .rev()
            .map(|&(cp, _)| cp)
            .find(|&cp| {
                let p = &self.coll.cps[cp];
                (p.syn[0].obj == o1 || p.syn[1].obj == o1) && self.cp_matches(p, md)
            });
        match found {
            Some(cp) => (cp, false),
            None => (self.cp_new(md), true),
        }
    }

    /// The geometry of a point from the current poses (`ivp_cp_update`, 0x1001f860): the normal, the point, the gap,
    /// the arms, the cores' contributions and the virtual mass; the spring integrated over the time since the last
    /// update. Nothing is written to the cores.
    pub(crate) fn cp_update(&mut self, idx: usize) {
        let mut cp = std::mem::take(&mut self.coll.cps[idx]);
        let o0 = cp.syn[0].obj;
        let o1 = cp.syn[1].obj;
        let c0 = self.obj_cache(o0);
        let c1 = self.obj_cache(o1);
        let r0_extra = self.coll.objs[o0].extra_radius;
        let r1_extra = self.coll.objs[o1].extra_radius;
        let mut t = std::mem::take(&mut cp.info);
        t.pushes = 0;
        t.flags &= !0x3ff;
        let l0 = cp.syn[0].feat.and_then(|f| self.coll.objs[o0].surface.as_ref()?.ledges.get(f.ledge));
        let l1 = cp.syn[1].feat.and_then(|f| self.coll.objs[o1].surface.as_ref()?.ledges.get(f.ledge));
        let mut skip_second = false;
        let p0: V3 = match cp.syn[0].typ {
            ST_POINT => match (l0, cp.syn[0].feat) {
                (Some(l), Some(f)) => world_point(&c0, l, f.edge),
                _ => [0.0; 3],
            },
            ST_BALL => {
                cp.recheck = 1;
                c0.vv
            }
            ST_EDGE => {
                if let (Some(a), Some(b), Some(fa), Some(fb)) = (l0, l1, cp.syn[0].feat, cp.syn[1].feat) {
                    edge_edge(&mut cp, a, fa.edge, &c0, b, fb.edge, &c1, &mut t);
                }
                skip_second = true;
                [0.0; 3]
            }
            _ => [0.0; 3],
        };
        if !skip_second {
            match cp.syn[1].typ {
                ST_POINT => {
                    if let (Some(l), Some(f)) = (l1, cp.syn[1].feat) {
                        let p1 = world_point(&c1, l, f.edge);
                        point_point(&mut cp, p0, p1, &mut t);
                    }
                }
                ST_EDGE => {
                    if let (Some(l), Some(f)) = (l1, cp.syn[1].feat) {
                        point_edge(&mut cp, p0, l, f.edge, &c1, &mut t);
                    }
                }
                ST_FACE => {
                    if let (Some(l), Some(f)) = (l1, cp.syn[1].feat) {
                        point_triangle(&mut cp, p0, l, f.edge, &c1, &mut t);
                    }
                }
                ST_BALL => point_point(&mut cp, p0, c1.vv, &mut t),
                _ => {}
            }
        }

        // the tail: the extra radii, the penetration (dropped when negative), the second arm
        let r0 = r0_extra;
        for k in 0..3 {
            t.cp_ws[k] = f64::from(t.n[k]) * f64::from(r0) + t.cp_ws[k];
        }
        cp.gap -= r0 + r1_extra;
        if cp.gap < 0.0 {
            cp.gap = 0.0;
        }
        normize_f(&mut t.span[0]);
        let n = t.n;
        let v0 = t.span[0];
        t.span[1] = [n[1] * v0[2] - v0[1] * n[2], v0[0] * n[2] - n[0] * v0[2], n[0] * v0[1] - n[1] * v0[0]];
        let mut vel = [[0.0f32; 3]; 2];
        let mut inv = [0.0f64; 2];
        let objs = [o0, o1];
        for k in 0..2 {
            let core = &self.cores[objs[k]];
            if core.unmovable {
                t.cp_cs[k] = [0.0; 3];
                t.cross_cs[k] = [0.0; 3];
                t.core[k] = None;
                continue;
            }
            let r = core.m_world_f_core.r;
            let d = [t.cp_ws[0] - core.m_world_f_core.vv[0], t.cp_ws[1] - core.m_world_f_core.vv[1], t.cp_ws[2] - core.m_world_f_core.vv[2]];
            let cs_d = math::rtmul(&r, &d);
            let cs: P3 = [cs_d[0] as f32, cs_d[1] as f32, cs_d[2] as f32];
            let ncs_d = math::rtmul(&r, &[f64::from(n[0]), f64::from(n[1]), f64::from(n[2])]);
            let ncs: P3 = [ncs_d[0] as f32, ncs_d[1] as f32, ncs_d[2] as f32];
            t.cp_cs[k] = cs;
            let cr = [cs[1] * ncs[2] - cs[2] * ncs[1], cs[2] * ncs[0] - cs[0] * ncs[2], ncs[1] * cs[0] - ncs[0] * cs[1]];
            t.cross_cs[k] = cr;
            vel[k] = core_point_velocity(&core.m_world_f_core.r, &cs, &core.speed, &core.rot_speed);
            let ii = core.inv_rot_inertia;
            inv[k] = ((f64::from(cr[2] * ii[2]) * f64::from(cr[2]) + f64::from(cr[1] * ii[1]) * f64::from(cr[1]))
                + f64::from(cr[0] * ii[0]) * f64::from(cr[0]))
                + f64::from(core.inv_mass);
            t.core[k] = Some(objs[k]);
        }
        // the virtual mass: the inverses add for two movable cores (f32, as the upstream)
        let inv0 = inv[0] as f32;
        let inv1 = inv[1] as f32;
        t.inv_virt_mass = if t.core[1].is_some() { inv1 + inv0 } else { inv0 };
        let vrel = if t.core[1].is_some() {
            [vel[0][0] - vel[1][0], vel[0][1] - vel[1][1], vel[0][2] - vel[1][2]]
        } else {
            vel[0]
        };
        t.virt_mass = 1.0 / t.inv_virt_mass;
        let dt = self.env.current_time - cp.time;
        cp.time = self.env.current_time;
        cp.s[0] = (f64::from(cp.s[0]) - dotd(&vrel, &t.span[0]) * dt) as f32;
        cp.s[1] = (f64::from(cp.s[1]) - dotd(&vrel, &t.span[1]) * dt) as f32;
        cp.info = t;
        self.coll.cps[idx] = cp;
    }

    /// The materials of a point's two sides, and the point's product of frictions (`ivp_cp_read_materials`, 0x10024040).
    /// Ballance's triangles carry no material index, so each side uses its object's friction and elasticity.
    pub(crate) fn cp_read_materials(&mut self, idx: usize) {
        let syn = self.coll.cps[idx].syn;
        let mut f = [0.0f64; 2];
        let mut e = [0.0f64; 2];
        let mut t = std::mem::take(&mut self.coll.cps[idx].info);
        for k in 0..2 {
            let o = &self.coll.objs[syn[k].obj];
            f[k] = f64::from(o.friction);
            e[k] = f64::from(o.elasticity);
            t.mat_friction[k] = f[k] as f32;
            t.mat_elasticity[k] = e[k] as f32;
            t.obj[k] = syn[k].obj;
            t.feat[k] = syn[k].feat;
        }
        t.elasticity = (e[0] * e[1]) as f32;
        self.coll.cps[idx].info = t;
        self.coll.cps[idx].mu = (f[1] * f[0]) as f32;
    }

    /// The inverse of the virtual mass without the direction (`ivp_cp_init_constants`, 0x1001d3d0): the creation
    /// constant, from the movable side's worst-case mass.
    pub(crate) fn cp_init_constants(&mut self, idx: usize) {
        let cp = &self.coll.cps[idx];
        let (o0, o1) = (cp.syn[0].obj, cp.syn[1].obj);
        let (cs0, cs1) = (cp.info.cp_cs[0], cp.info.cp_cs[1]);
        let (f0, f1) = (self.cores[o0].unmovable, self.cores[o1].unmovable);
        let vm = |w: &World, o: usize, p: &P3| worst_vm(&w.cores[o].inv_rot_inertia, w.cores[o].inv_mass, p);
        let inv = if f0 {
            1.0 / vm(self, o1, &cs1)
        } else if f1 {
            1.0 / vm(self, o0, &cs0)
        } else {
            let m0 = vm(self, o0, &cs0);
            let m1 = vm(self, o1, &cs1);
            1.0 / ((m1 * m0) / (m1 + m0))
        };
        self.coll.cps[idx].inv_vm_no_dir = inv as f32;
    }

    /// Removes a point from both objects' lists and frees it (`ivp_cp_destroy`, 0x1001c230).
    pub(crate) fn cp_destroy(&mut self, idx: usize) {
        let syn = self.coll.cps[idx].syn;
        self.notify_contact(syn[0].obj, syn[1].obj, false);
        for s in syn {
            self.coll.objs[s.obj].fric.retain(|&(cp, _)| cp != idx);
        }
        self.coll.cps[idx].alive = false;
        self.coll.free_cps.push(idx);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::convex;
    use crate::ledge::Surface;

    fn unit_cube() -> Surface {
        let mut p = Vec::new();
        for &x in &[0.0, 1.0] {
            for &y in &[0.0, 1.0] {
                for &z in &[0.0, 1.0] {
                    p.push([x, y, z]);
                }
            }
        }
        Surface::compile(vec![convex::hull(&p).expect("a cube has a hull")]).expect("a surface")
    }

    #[test]
    fn point_on_a_face_has_the_face_normal_and_gap() {
        // a point on the top face of a box, at height 0.25 above the surface: the normal points out of the face
        let l = convex::hull(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0]]).expect("a cube");
        let surface = unit_cube();
        assert!(surface.ledges_within_radius([0.5; 3], 0.1).contains(&0));
        let c1 = ObjCache {
            r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            vv: [0.0; 3],
            core_pos: [0.0; 3],
        };
        // find the first triangle on the top face (z = 1) and a point above it
        let face = (0..l.tris.len())
            .map(|t| Edge { tri: t, slot: 0 })
            .find(|&e| l.p(e)[2] == 1.0 && l.p(Ledge::next(e))[2] == 1.0 && l.p(Ledge::prev(e))[2] == 1.0)
            .expect("a top triangle");
        // the creation sets the inverse of the triangle normal's length (cp_new)
        let nn = tri_normal(&l, face);
        let mut cp = ContactPoint {
            recheck: 0,
            inv_tri_det: (1.0 / (nn[0] * nn[0] + nn[1] * nn[1] + nn[2] * nn[2]).sqrt()) as f32,
            ..ContactPoint::default()
        };
        let mut t = ContactInfo::default();
        point_triangle(&mut cp, [0.25, 0.25, 1.25], &l, face, &c1, &mut t);
        // the gap is the height above the top face
        assert!((cp.gap - 0.25).abs() < 1e-3, "gap {}", cp.gap);
        assert!(t.n[2].abs() > 0.99, "normal {:?}", t.n);
    }

    #[test]
    fn the_init_constant_of_a_fixed_side_uses_the_movable_one() {
        let inv = [0.0f32; 3];
        let w = worst_vm(&inv, 2.0, &[0.0, 0.0, 0.0]);
        assert!((w - 0.5).abs() < 1e-9);
    }
}
