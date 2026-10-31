// Port of OpenBallance src/phys/phys_event.c (GPL-3.0; see the LICENSE of the repository): the mindist event solver
// (docs/ivp_collision.md 5.2-6). From the closest features the minimizer found, it finds the earliest time within
// [t_now, t_max] at which the pair reaches the collision distance (type 0x10 .. 0x42), or the features stop being valid
// (the 0x31, 0x32 and 0x41 codes). Each case is a functor of the two objects' poses at time t, and the search walks
// the 5 ms grid from t_now, refining a crossing with regula falsi.
//
// The poses come from the caller through `Motion`, since the body's motion lives with the glue. The sims cache a
// pose per grid slot exactly as the upstream does, and a body that is not simulated keeps its pose at t_now.
//
// Representation: the features are `Edge`s in the ledge of their synapse's surface, passed to the cases as the
// synapse's ledge reference. The functor's `eval` is the upstream's table of distance functors.

use crate::ledge::{Edge, Ledge, Surface};
use crate::math::{self, isqrt4, normalize4, Matrix};
use crate::mindist::{Mindist, ObjCache, ST_BALL, ST_EDGE, ST_FACE, ST_POINT};

type V3 = [f64; 3];
type P3 = [f32; 3];

/// Collision distances (phys_coll.h, 4.1): the f32 settings, widened where the upstream widens them.
pub const REAL_COLL_DIST: f32 = 0.001;
pub const COLL_DIST: f32 = 0.01;
/// 0x3a83126e: 0.1 times the minimum collision distance, rounded once.
pub const MIN_FRICTION_DIST: f32 = f32::from_bits(0x3a83_126e);

/// The core values an event reads (`IvpCore`, as f64 of the upstream's f32 fields).
#[derive(Clone, Copy, Debug, Default)]
pub struct CoreInfo {
    pub abs_omega: f64,
    pub current_speed: f64,
    pub max_surface_rot_speed: f64,
    pub upper_limit_radius: f64,
    pub inv_object_diameter: f64,
}

/// The poses of the bodies at a time, supplied by the glue (`ivp_body_m_world_f_object_at`, 0x10009d70).
pub trait Motion {
    fn matrix_at(&self, obj: usize, t: f64) -> Matrix;
}

/// One side of a mindist as the event solver sees it: the object, its cache at t_now, its core values, whether it
/// is simulated (a body that is not keeps its pose), and its surface.
#[derive(Clone, Copy)]
pub struct EventSide<'a> {
    pub obj: usize,
    pub cache: ObjCache,
    pub core: CoreInfo,
    pub simulated: bool,
    pub surface: Option<&'a Surface>,
}

/// The sim of one object (6.2): its pose at t_now and the poses on the grid, filled lazily.
struct Sim {
    obj: usize,
    m0: Matrix,
    core: CoreInfo,
    slots: [Option<Matrix>; 21],
}

impl Sim {
    fn new(s: &EventSide) -> Sim {
        let m0 = Matrix {
            r: s.cache.r,
            vv: s.cache.vv,
        };
        let mut slots = [None; 21];
        if !s.simulated {
            slots = [Some(m0); 21];
        }
        slots[0] = Some(m0);
        Sim {
            obj: s.obj,
            m0,
            core: s.core,
            slots,
        }
    }

    /// The pose on grid slot `i` at time `t`, computed on first use (`sim_slot`).
    fn slot(&mut self, motion: &dyn Motion, i: usize, t: f64) -> Matrix {
        if let Some(m) = self.slots[i] {
            return m;
        }
        let m = motion.matrix_at(self.obj, t);
        self.slots[i] = Some(m);
        m
    }
}

fn dot3(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn fsub_d(a: P3, b: P3) -> V3 {
    [f64::from(a[0]) - f64::from(b[0]), f64::from(a[1]) - f64::from(b[1]), f64::from(a[2]) - f64::from(b[2])]
}

fn to_v3(a: P3) -> V3 {
    [f64::from(a[0]), f64::from(a[1]), f64::from(a[2])]
}

/// `M p` for a pose (`0x1000f550`).
fn mpoint(m: &Matrix, p: V3) -> V3 {
    let x = math::rmul(&m.r, &p);
    [x[0] + m.vv[0], x[1] + m.vv[1], x[2] + m.vv[2]]
}

fn fmpoint(m: &Matrix, p: P3) -> V3 {
    mpoint(m, to_v3(p))
}

/// The steps of a search segment: `step * 200` truncated, at least one (`steps_of`, 0x10063b88). A value out of the
/// integer range is the x87 integer indefinite, which is below one.
fn steps_of(step: f64) -> i32 {
    let x = step * 200.0;
    let n = if x > -2_147_483_649.0 && x < 2_147_483_648.0 {
        x as i32
    } else {
        i32::MIN
    };
    n.max(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    PointPlane,
    DirDir,
    LineLine,
    Parallel,
    PointPoint,
    PlaneThrough,
    PointLine,
    EdgeTurn,
}

/// A distance functor (`Functor`): what the search evaluates between two poses, its speed bound and its vectors.
#[derive(Clone, Copy, Debug)]
struct Functor {
    kind: Kind,
    speed: f64,
    inv_speed: f64,
    a: V3,
    b: V3,
    c: V3,
    d: V3,
    h: f64,
    sign: f64,
    pf: P3,
    qf: P3,
}

impl Functor {
    fn new(kind: Kind, speed: f64) -> Functor {
        Functor {
            kind,
            speed,
            inv_speed: 1.0 / speed,
            a: [0.0; 3],
            b: [0.0; 3],
            c: [0.0; 3],
            d: [0.0; 3],
            h: 0.0,
            sign: 0.0,
            pf: [0.0; 3],
            qf: [0.0; 3],
        }
    }

    /// The functor's value for the two poses (`fx`).
    fn eval(&self, ma: &Matrix, mb: &Matrix) -> f64 {
        match self.kind {
            // 0x1002a9c0: (MA p - MB q) . (RB n)
            Kind::PointPlane => {
                let pa = mpoint(ma, self.a);
                let qb = mpoint(mb, self.c);
                let n = math::rmul(&mb.r, &self.b);
                dot3(fsub_v(pa, qb), n)
            }
            // 0x1002ae80: (RA d) . (RB n)
            Kind::DirDir => {
                let d = math::rmul(&ma.r, &self.a);
                let n = math::rmul(&mb.r, &self.b);
                dot3(d, n)
            }
            // 0x1002b530: sign ((MA pA - MB pB) . c) isqrt(|c|^2), c = (RA dA) x (RB dB)
            Kind::LineLine => {
                let pa = mpoint(ma, self.a);
                let pb = mpoint(mb, self.c);
                let da = math::rmul(&ma.r, &self.b);
                let db = math::rmul(&mb.r, &self.d);
                let c = cross3(da, db);
                self.sign * dot3(fsub_v(pa, pb), c) * isqrt4(dot3(c, c) as f32)
            }
            // 0x1002b620: |(RA dA) x (RB dB)|^2
            Kind::Parallel => {
                let da = math::rmul(&ma.r, &self.a);
                let db = math::rmul(&mb.r, &self.b);
                let c = cross3(da, db);
                dot3(c, c)
            }
            // 0x1002ab30: the signed projection s = 1.2 (d . u) when it is the smaller one, else the distance
            Kind::PointPoint => {
                let pa = mpoint(ma, self.a);
                let pb = mpoint(mb, self.b);
                let d = fsub_v(pb, pa);
                let s = 1.2000000476837158 * dot3(d, self.c);
                let d2 = dot3(d, d);
                if s * s.abs() < d2 {
                    s
                } else {
                    d2.sqrt()
                }
            }
            // 0x1002abc0: (M2 p2 - M1 p1) . (R2 v)
            Kind::PlaneThrough => {
                let p1 = mpoint(ma, self.a);
                let p2 = mpoint(mb, self.b);
                let v = math::rmul(&mb.r, &self.c);
                dot3(fsub_v(p2, p1), v)
            }
            // 0x1002ad90: min(|w|, w . (RB c) + h), w = (MB q - MA p) x (RB e)
            Kind::PointLine => {
                let p = fmpoint(ma, self.pf);
                let q = fmpoint(mb, self.qf);
                let e = math::rmul(&mb.r, &self.a);
                let c = math::rmul(&mb.r, &self.b);
                let w = cross3(fsub_v(q, p), e);
                let len = dot3(w, w).sqrt();
                let x = dot3(w, c) + self.h;
                if len < x {
                    len
                } else {
                    x
                }
            }
            // 0x1002ac40: (RA v) . normalize(((RB e) x d) x (RB e)), d = MA p - MB q
            Kind::EdgeTurn => {
                let p = mpoint(ma, self.a);
                let q = mpoint(mb, self.c);
                let e = math::rmul(&mb.r, &self.d);
                let v = math::rmul(&ma.r, &self.b);
                let d = fsub_v(p, q);
                let x = cross3(e, d);
                let mut y = cross3(x, e);
                math::normalize_d(&mut y);
                dot3(v, y)
            }
        }
    }
}

fn fsub_v(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The out-ring of an edge: the edges round its point, from opp(prev(e)) on, each step through opp(prev(g)). The
/// caller stops at `e` itself, as the upstream's loops do; the bound only stops a corrupt ledge.
fn out_ring(l: &Ledge, e: Edge) -> Vec<Edge> {
    let mut out = Vec::new();
    let mut g = l.opp(Ledge::prev(e));
    let bound = 3 * l.tris.len() + 3;
    loop {
        out.push(g);
        if g == e || out.len() > bound {
            return out;
        }
        g = l.opp(Ledge::prev(g));
    }
}

/// The unscaled normal of the triangle of `e`, from its edges (`tri_normal`, 0x10021280).
fn tri_normal(l: &Ledge, e: Edge) -> V3 {
    let a = fsub_d(l.p(Ledge::next(e)), l.p(e));
    let b = fsub_d(l.p(Ledge::prev(e)), l.p(e));
    cross3(a, b)
}

/// The result of one solve: the code (0 for none) and the time of the event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub code: u32,
    pub time: f64,
}

/// The solver state (`IvpEventSolver`) during one solve.
struct Ev<'a, 'm> {
    motion: &'m dyn Motion,
    sims: [Sim; 2],
    /// The ledge of each sim's feature (none for a ball).
    ledges: [Option<&'a Ledge>; 2],
    t_now: f64,
    t_max: f64,
    worst_approach: f64,
    worst_total: f64,
    code: u32,
    time: f64,
}

/// The convex-case constants (`Conv`).
#[derive(Clone, Copy)]
struct Conv {
    rotsum: f64,
    cd: f64,
    rcd: f64,
    eps: f64,
    xr: f64,
    len: f64,
}

impl<'a, 'm> Ev<'a, 'm> {
    /// The point of `e` in sim `i`'s ledge. Only the polygon sims are asked for points.
    fn pt(&self, i: usize, e: Edge) -> P3 {
        self.ledges[i].map_or([0.0; 3], |l| l.p(e))
    }

    /// The ledge of sim `i` (the caller only asks for polygons).
    fn ledge(&self, i: usize) -> &'a Ledge {
        // a sim without a ledge is never asked for its edges: the dispatch only runs polygon cases on polygon sims
        match self.ledges[i] {
            Some(l) => l,
            None => &EMPTY_LEDGE,
        }
    }

    fn fx(&mut self, f: &Functor, a: usize, b: usize, i: usize, t: f64) -> f64 {
        let ma = self.sims[a].slot(self.motion, i, t);
        let mb = self.sims[b].slot(self.motion, i, t);
        f.eval(&ma, &mb)
    }

    /// The functor at the start poses (`fx(F, sa->slot[0], sb->slot[0])`).
    fn fx0(&self, f: &Functor, a: usize, b: usize) -> f64 {
        f.eval(&self.sims[a].m0, &self.sims[b].m0)
    }

    /// The first time the functor drops to `target` or below, by the regula falsi between two grid points, with the
    /// poses computed fresh (`regula_falsi`, 0x10037790).
    fn regula_falsi(&self, f: &Functor, mut t_lo: f64, mut t_hi: f64, target: f64, mut f_lo: f64, mut f_hi: f64, a: usize, b: usize) -> f64 {
        let (oa, ob) = (self.sims[a].obj, self.sims[b].obj);
        let mut count = 0u32;
        loop {
            let mut t = t_lo + (t_hi - t_lo) * (target - f_lo) / (f_hi - f_lo);
            if count & 3 == 3 {
                if count > 0x40 {
                    return t_lo;
                }
                t += ((t_lo - t) + (t_hi - t)) * 0.375;
            }
            let ma = self.motion.matrix_at(oa, t);
            let mb = self.motion.matrix_at(ob, t);
            let fv = f.eval(&ma, &mb);
            if (fv - target).abs() < 1e-8 {
                return t;
            }
            count += 1;
            if fv < target {
                t_hi = t;
                f_hi = fv;
            } else {
                t_lo = t;
                f_lo = fv;
            }
        }
    }

    /// The first t with f(t) <= target, by conservative advancement on the 5 ms grid from slot `i` (`search_below`,
    /// 0x10037910). `t_max` is the bound; `f_start` the value at `t` when the caller has it.
    fn search_below(&mut self, f: &Functor, target: f64, t: f64, t_max: f64, i: usize, a: usize, b: usize, f_start: Option<f64>) -> Option<f64> {
        let mut fv = f_start.unwrap_or_else(|| self.fx0(f, a, b));
        if fv <= target {
            return Some(t);
        }
        let mut t = t;
        let mut i = i;
        loop {
            let dt_safe = (fv - target) * f.inv_speed;
            let d = t - t_max;
            if d + dt_safe > 0.0 {
                return None;
            }
            let mut step = 2.0 * dt_safe;
            // d rounded to f32 for this test, as the upstream does
            if f64::from(d as f32) + step > 0.0 {
                step = (t_max - t) + 1e-8;
            }
            let n = steps_of(step);
            i += n as usize;
            if i > 20 {
                return None;
            }
            let t_new = t + f64::from(n) * f64::from(0.005f32);
            let f_new = self.fx(f, a, b, i, t_new);
            if f_new > target {
                if i == 20 {
                    return None;
                }
                t = t_new;
                fv = f_new;
                continue;
            }
            let hit = self.regula_falsi(f, t, t_new, target, fv, f_new, a, b);
            if hit - t_max > 0.0 {
                return None;
            }
            return Some(hit);
        }
    }

    /// As `search_below`, with the branch for a start already within the target: `v2` is the close-in distance
    /// (`find_event`, 0x10037b30).
    fn find_event(&mut self, f: &Functor, target: f64, v2: f64, t_start: f64, t_max: f64, a: usize, b: usize, f_start: Option<f64>) -> Option<f64> {
        let f0 = f_start.unwrap_or_else(|| self.fx0(f, a, b));
        if f0 > target {
            return self.search_below(f, target, t_start, t_max, 0, a, b, Some(f0));
        }
        let inv = 1.0 / f.speed;
        if !(t_start - t_max < 0.0) {
            return None;
        }
        let mut t = t_start;
        let mut fv = f0;
        let mut i = 0usize;
        loop {
            let mut step = (fv - v2) * inv;
            if (t - t_max) + step > 0.0 {
                step = (t_max - t) + 1e-8;
            } else if step < 0.0 {
                step = 0.0;
            }
            let n = steps_of(step);
            i += n as usize;
            if i > 20 {
                return None;
            }
            let t_new = t + f64::from(n) * f64::from(0.005f32);
            let f_new = self.fx(f, a, b, i, t_new);
            if f_new > target {
                // out of the hit zone: look for the re-entry
                if t_new - t_max <= 0.0 {
                    return self.search_below(f, target, t_new, t_max, i, a, b, Some(f_new));
                }
                return None;
            }
            if f_new <= f0 {
                // not separating against the start (the upstream compares with f0, not f)
                return Some(t);
            }
            t = t_new;
            fv = f_new;
            if !(t_new - t_max < 0.0) {
                return None;
            }
        }
    }

    // ---- the ball cases (5.4), ball = sim 0, polygon = sim 1 ----

    /// BF (0x1002b370): the ball's centre against a face.
    fn ball_face(&mut self, e: Edge, md: &Mindist) {
        let r = f64::from(md.sum_extra_radius);
        let len = f64::from(md.len_numerator);
        let mut f = Functor::new(Kind::PointPlane, self.worst_approach);
        let l = self.ledge(1);
        f.b = tri_normal(l, e);
        normalize4(&mut f.b);
        f.c = to_v3(l.p(e));
        let f0 = r + len;
        if let Some(t) = self.find_event(&f, r + f64::from(COLL_DIST), 0.5 * r + f64::from(REAL_COLL_DIST), self.t_now, self.t_max, 0, 1, Some(f0)) {
            self.time = t;
            self.code = 0x20;
        }
    }

    /// BP (0x1002c260): the ball's centre against a point.
    fn ball_point(&mut self, e: Edge, md: &Mindist) {
        let r = f64::from(md.sum_extra_radius);
        let len = f64::from(md.len_numerator);
        let cd = f64::from(COLL_DIST);
        let pf = self.ledge(1).p(e);
        let mut f1 = Functor::new(Kind::PointPoint, self.worst_approach);
        f1.b = to_v3(pf);
        for k in 0..3 {
            f1.c[k] = f64::from(md.normal[k]) * -1.0;
        }
        if let Some(t) = self.find_event(&f1, r + cd, 0.5 * r + f64::from(REAL_COLL_DIST), self.t_now, self.t_max, 0, 1, None) {
            self.time = t;
            self.code = 0x10;
        }
        // leaving the point's region along an incident edge
        let dmax = (self.time - self.t_now) * self.worst_total + len;
        let pc = self.sims[1].core;
        let bcore = self.sims[0].core;
        let plen = (f64::from(pf[0]) * f64::from(pf[0]) + f64::from(pf[1]) * f64::from(pf[1]) + f64::from(pf[2]) * f64::from(pf[2])).sqrt();
        let speed = plen * pc.abs_omega + pc.current_speed + pc.abs_omega * dmax + bcore.current_speed;
        let m2 = if dmax * dmax < cd * cd { dmax * dmax } else { cd * cd };
        let k = -m2 * pc.inv_object_diameter;
        let mut f2 = Functor::new(Kind::PlaneThrough, speed);
        f2.b = to_v3(pf);
        let l = self.ledge(1);
        for g in out_ring(l, e) {
            let d = fsub_d(l.p(Ledge::next(g)), pf);
            let l2 = dot3(d, d);
            let is = isqrt4(l2 as f32);
            for j in 0..3 {
                f2.c[j] = d[j] * is;
            }
            let a = is * l2 * k;
            if let Some(t) = self.search_below(&f2, a, self.t_now, self.time, 0, 0, 1, None) {
                self.time = t;
                self.code = 0x11;
            }
            if g == e {
                break;
            }
        }
    }

    /// BK (0x1002d000): the ball's centre against an edge, and the two faces it turns into.
    fn ball_edge(&mut self, e: Edge, md: &Mindist) {
        let r = f64::from(md.sum_extra_radius);
        let cd = f64::from(COLL_DIST);
        let l = self.ledge(1);
        let pf = l.p(e);
        let qf = l.p(Ledge::next(e));
        let mut f1 = Functor::new(Kind::PointLine, self.worst_approach);
        f1.h = 0.5 * (r + cd);
        f1.qf = qf;
        f1.pf = pf;
        f1.a = fsub_d(qf, pf);
        normalize4(&mut f1.a);
        let sp = &self.sims[1].m0;
        let pw = fmpoint(sp, pf);
        let ew = math::rmul(&sp.r, &f1.a);
        let sb_vv = self.sims[0].m0.vv;
        let d = [pw[0] - sb_vv[0], pw[1] - sb_vv[1], pw[2] - sb_vv[2]];
        let x = cross3(d, ew);
        f1.b = math::rtmul(&sp.r, &x);
        math::normalize_d(&mut f1.b);
        if let Some(t) = self.find_event(&f1, r + cd, 0.9 * r + f64::from(REAL_COLL_DIST), self.t_now, self.t_max, 0, 1, None) {
            self.time = t;
            self.code = 0x30;
        }
        // entering either adjacent face
        let mut f2 = Functor::new(Kind::PointPlane, self.worst_total);
        let ee = [e, self.ledge(1).opp(e)];
        for ei in ee {
            let l = self.ledge(1);
            let qp = fsub_d(l.p(Ledge::next(ei)), l.p(ei));
            let rp = fsub_d(l.p(Ledge::prev(ei)), l.p(ei));
            let nt = cross3(qp, rp);
            f2.b = cross3(qp, nt);
            math::normalize_d(&mut f2.b);
            f2.c = to_v3(l.p(ei));
            if let Some(t) = self.search_below(&f2, -f64::from(MIN_FRICTION_DIST), self.t_now, self.time, 0, 0, 1, None) {
                self.time = t;
                self.code = 0x31;
            }
        }
    }

    /// The ball against a polygon (0x1002d4d0): the cases by the polygon's feature.
    fn ball_poly(&mut self, md: &Mindist) {
        let Some(feat) = md.syn[1].feat else { return };
        match md.syn[1].status {
            ST_POINT => self.ball_point(feat.edge, md),
            ST_EDGE => self.ball_edge(feat.edge, md),
            ST_FACE => self.ball_face(feat.edge, md),
            _ => {}
        }
    }

    /// Ball against ball (0x1002d5f0 / 0x1002c670).
    fn ball_ball(&mut self, md: &Mindist) {
        let r = f64::from(md.sum_extra_radius);
        let mut f = Functor::new(Kind::PointPoint, self.worst_approach);
        let vv_b = self.sims[1].m0.vv;
        let vv_a = self.sims[0].m0.vv;
        for k in 0..3 {
            f.c[k] = vv_b[k] - vv_a[k];
        }
        math::normalize_d(&mut f.c);
        if let Some(t) = self.find_event(&f, r + f64::from(COLL_DIST), 0.5 * r + f64::from(REAL_COLL_DIST), self.t_now, self.t_max, 0, 1, None) {
            self.time = t;
            self.code = 0x10;
        }
    }

    // ---- the convex cases (6.6-6.10); sim 0 is the first object, sim 1 the second ----

    /// PF (0x1002aed0): a vertex of A against a face of B, and an edge of A dipping below it.
    fn conv_pf(&mut self, c: &Conv, ea: Edge, eb: Edge) {
        let p = self.pt(0, ea);
        let mut f = Functor::new(Kind::PointPlane, self.worst_approach);
        f.a = to_v3(p);
        let lb = self.ledge(1);
        f.b = tri_normal(lb, eb);
        normalize4(&mut f.b);
        f.c = to_v3(lb.p(eb));
        let f_now = c.xr + c.len;
        if let Some(t) = self.find_event(&f, c.xr + c.cd, 0.5 * c.xr + c.rcd, self.t_now, self.time, 0, 1, Some(f_now)) {
            self.time = t;
            self.code = 0x20;
        }
        // an edge leaving P dips below the face plane
        let mut g = Functor::new(Kind::DirDir, c.rotsum + 1e-19);
        g.b = f.b;
        let mn = if c.len < c.cd { c.len } else { c.cd };
        let lim = (-(c.eps * self.sims[1].core.inv_object_diameter)) * (c.xr * f64::from(0.1f32) + mn) / c.cd;
        let nw = math::rmul(&self.sims[1].m0.r, &f.b);
        let na = math::rtmul(&self.sims[0].m0.r, &nw);
        let maxdev = (self.time - self.t_now) * (c.rotsum + 1e-19);
        let la = self.ledge(0);
        for ge in out_ring(la, ea) {
            let v = fsub_d(la.p(Ledge::next(ge)), p);
            let k = isqrt4(dot3(v, v) as f32);
            let c0 = dot3(na, v) * k;
            if c0 < maxdev {
                for j in 0..3 {
                    g.a[j] = v[j] * k;
                }
                if let Some(t) = self.search_below(&g, lim, self.t_now, self.time, 0, 0, 1, Some(c0)) {
                    self.time = t;
                    self.code = 0x21;
                }
            }
            if ge == ea {
                break;
            }
        }
    }

    /// PP (0x1002bcf0): a vertex of A against a vertex of B, and the edges leaving each one.
    fn conv_pp(&mut self, c: &Conv, ea: Edge, eb: Edge, md: &Mindist) {
        let pa = self.pt(0, ea);
        let pb = self.pt(1, eb);
        let mut pf = Functor::new(Kind::PointPoint, self.worst_approach);
        pf.a = to_v3(pa);
        pf.b = to_v3(pb);
        for k in 0..3 {
            pf.c[k] = -f64::from(md.normal[k]);
        }
        if let Some(t) = self.find_event(&pf, c.cd + c.xr, 0.5 * c.xr + c.rcd, self.t_now, self.t_max, 0, 1, None) {
            self.time = t;
            self.code = 0x10;
        }
        let maxd = (self.time - self.t_now) * self.worst_total + c.len;
        let la = dot3(to_v3(pa), to_v3(pa)).sqrt();
        let lb = dot3(to_v3(pb), to_v3(pb)).sqrt();
        let ca = self.sims[0].core;
        let cb = self.sims[1].core;
        let sa = la * ca.abs_omega + ca.current_speed;
        let sb = lb * cb.abs_omega + cb.current_speed;
        let spd_b = cb.abs_omega * maxd + sb + sa;
        let spd_a = ca.abs_omega * maxd + sb + sa;
        let r = if ca.upper_limit_radius > cb.upper_limit_radius { ca.upper_limit_radius } else { cb.upper_limit_radius };
        let m2 = if maxd * maxd < c.cd * c.cd { maxd * maxd } else { c.cd * c.cd };
        let k = -0.5 * m2 / r;
        // (edge in its object's ledge, that object's sim index, the other object's sim index, its start point, the
        // other point, speed)
        let sides = [(eb, 1usize, pb, pa, 0usize, spd_b), (ea, 0usize, pa, pb, 1usize, spd_a)];
        for (e0, own, p0, po, other, spd) in sides {
            let mut q = Functor::new(Kind::PlaneThrough, spd);
            q.a = to_v3(po);
            q.b = to_v3(p0);
            let l = self.ledge(own);
            for ge in out_ring(l, e0) {
                let v = fsub_d(l.p(Ledge::next(ge)), p0);
                let l2 = dot3(v, v);
                let ki = isqrt4(l2 as f32);
                for j in 0..3 {
                    q.c[j] = v[j] * ki;
                }
                let target = (l2 * ki) * k;
                // the functor's first pose is the point's object: sim `other` when Po is in it
                let (sa_i, sb_i) = if other == 0 { (0, 1) } else { (1, 0) };
                if let Some(t) = self.search_below(&q, target, self.t_now, self.time, 0, sa_i, sb_i, None) {
                    self.time = t;
                    self.code = 0x11;
                }
                if ge == e0 {
                    break;
                }
            }
        }
    }

    /// PK (0x1002c820): a vertex of A against an edge of B, and the edges turning toward it.
    fn conv_pk(&mut self, c: &Conv, ea: Edge, eb: Edge) {
        let p = self.pt(0, ea);
        let lb = self.ledge(1);
        let e0 = lb.p(eb);
        let e1 = lb.p(Ledge::next(eb));
        let mut e = fsub_d(e1, e0);
        math::normalize_d(&mut e);
        let mb = self.sims[1].m0;
        let ma = self.sims[0].m0;
        let wp = fmpoint(&ma, p);
        let we0 = fmpoint(&mb, e0);
        let we = math::rmul(&mb.r, &e);
        let d = fsub_v(we0, wp);
        let x = cross3(d, we);
        let mut rf = Functor::new(Kind::PointLine, self.worst_approach);
        rf.h = 0.5 * (c.cd + c.xr);
        rf.pf = p;
        rf.qf = e0;
        rf.a = e;
        rf.b = math::rtmul(&mb.r, &x);
        math::normalize_d(&mut rf.b);
        if let Some(t) = self.find_event(&rf, c.cd + c.xr, 0.9 * c.xr + c.rcd, self.t_now, self.t_max, 0, 1, None) {
            self.time = t;
            self.code = 0x30;
        }
        // P enters one of the two faces at the edge
        let mut f = Functor::new(Kind::PointPlane, self.worst_total);
        f.a = to_v3(p);
        let lb = self.ledge(1);
        let ce = [eb, lb.opp(eb)];
        for ei in ce {
            let pn = fsub_d(lb.p(Ledge::next(ei)), lb.p(ei));
            let pp = fsub_d(lb.p(Ledge::prev(ei)), lb.p(ei));
            let nrm = cross3(pn, pp);
            f.b = cross3(pn, nrm);
            math::normalize_d(&mut f.b);
            f.c = to_v3(lb.p(ei));
            if let Some(t) = self.search_below(&f, -c.eps, self.t_now, self.time, 0, 0, 1, None) {
                self.time = t;
                self.code = 0x31;
            }
        }
        // an edge leaving P turns toward the edge
        let mut dmin = c.len - (self.time - self.t_now) * self.worst_total;
        if dmin < 1e-8 {
            dmin = 1e-8;
        }
        let lp = dot3(to_v3(p), to_v3(p)).sqrt();
        let ca = self.sims[0].core;
        let cb = self.sims[1].core;
        let sa = lp * ca.abs_omega + ca.current_speed;
        // the upstream takes the second object's max surface rotation speed here (ported as is)
        let sb = cb.max_surface_rot_speed * cb.abs_omega + cb.current_speed;
        let mut t_f = Functor::new(Kind::EdgeTurn, (sb + sa) / dmin + c.rotsum);
        t_f.a = to_v3(p);
        t_f.c = to_v3(e0);
        t_f.d = e;
        let mn = if c.len < c.cd { c.len } else { c.cd };
        let target = 2.0 * cb.inv_object_diameter * mn * -0.30000001192092896;
        let la = self.ledge(0);
        for ge in out_ring(la, ea) {
            t_f.b = fsub_d(la.p(Ledge::next(ge)), p);
            math::normalize_d(&mut t_f.b);
            if let Some(t) = self.search_below(&t_f, target, self.t_now, self.time, 0, 0, 1, None) {
                self.time = t;
                self.code = 0x32;
            }
            if ge == ea {
                break;
            }
        }
    }

    /// KK (0x1002b690): two edges, their line distance, the parallel case and the four face turns.
    fn conv_kk(&mut self, c: &Conv, ea: Edge, eb: Edge, md: &Mindist) {
        let la = self.ledge(0);
        let lb = self.ledge(1);
        let mut da = fsub_d(la.p(Ledge::next(ea)), la.p(ea));
        normalize4(&mut da);
        let mut db = fsub_d(lb.p(Ledge::next(eb)), lb.p(eb));
        normalize4(&mut db);
        let wa = math::rmul(&self.sims[0].m0.r, &da);
        let wb = math::rmul(&self.sims[1].m0.r, &db);
        let cr = cross3(wa, wb);
        let nc = f64::from(md.normal[0]) * cr[0] + f64::from(md.normal[1]) * cr[1] + f64::from(md.normal[2]) * cr[2];
        let flag = if -nc > 0.0 { 0 } else { 1 };
        let sign = if flag == 0 { -1.0 } else { 1.0 };
        let mut h = Functor::new(Kind::LineLine, self.worst_approach);
        h.a = to_v3(la.p(ea));
        h.b = da;
        h.c = to_v3(lb.p(eb));
        h.d = db;
        h.sign = sign;
        if let Some(t) = self.find_event(&h, c.cd, c.rcd, self.t_now, self.time, 0, 1, None) {
            self.time = t;
            self.code = 0x40;
        }
        let mut j = Functor::new(Kind::Parallel, 2.0 * c.rotsum + 1e-19);
        j.a = da;
        j.b = db;
        if let Some(t) = self.search_below(&j, 1e-19, self.t_now, self.time, 0, 0, 1, None) {
            self.time = t;
            self.code = 0x41;
        }
        let mut k = Functor::new(Kind::DirDir, c.rotsum + 1e-19);
        let (plus, minus) = if flag == 1 { (1.0, -1.0) } else { (-1.0, 1.0) };
        // (face edge, direction, sign, sim of the direction, sim of the face, the face's object)
        let cases: [(Edge, V3, f64, usize, usize, usize); 4] = [
            (la.opp(ea), db, plus, 1, 0, 0),
            (ea, db, minus, 1, 0, 0),
            (lb.opp(eb), da, plus, 0, 1, 1),
            (eb, da, minus, 0, 1, 1),
        ];
        for (face, dir, sgn, sd, sf, own) in cases {
            for jx in 0..3 {
                k.a[jx] = dir[jx] * sgn;
            }
            let l = self.ledge(own);
            k.b = tri_normal(l, face);
            normalize4(&mut k.b);
            let target = -(c.eps * self.sims[sd].core.inv_object_diameter);
            if let Some(t) = self.search_below(&k, target, self.t_now, self.time, 0, sd, sf, None) {
                self.time = t;
                self.code = 0x42;
            }
        }
    }
}

/// A stand-in ledge for an unused sim (never read: the cases only ask polygon sims for their edges).
static EMPTY_LEDGE: Ledge = Ledge {
    points: Vec::new(),
    tris: Vec::new(),
};

/// The event solver (`ivp_event_solve`, table 0x1007632c): the earliest event of the mindist within
/// [t_now, t_max], by the sorted statuses. `sides` are the two synapses' objects in synapse order.
pub fn solve<'a>(md: &Mindist, sides: [EventSide<'a>; 2], motion: &dyn Motion, t_now: f64, t_max: f64, worst_approach: f64, worst_total: f64) -> Event {
    let ka = md.sorted(0);
    let kb = md.sorted(1);
    let sa = md.syn[ka].status;
    let sb = md.syn[kb].status;
    let none = Event { code: 0, time: t_max };
    let (sims_order, ledges): ([usize; 2], [Option<&Ledge>; 2]) = if sa == ST_BALL && sb == ST_BALL {
        ([0, 1], [None, None])
    } else if sa == ST_BALL && sb <= ST_FACE {
        ([0, 1], [None, feat_ledge(md, &sides, 1)])
    } else if sa <= sb && sb <= ST_FACE && !(sa == ST_EDGE && sb == ST_FACE) {
        ([ka, kb], [feat_ledge(md, &sides, ka), feat_ledge(md, &sides, kb)])
    } else {
        // the upstream asserts on this pairing (0x1002d850)
        return none;
    };
    let mut ev = Ev {
        motion,
        sims: [Sim::new(&sides[sims_order[0]]), Sim::new(&sides[sims_order[1]])],
        ledges,
        t_now,
        t_max,
        worst_approach,
        worst_total,
        code: 0,
        time: t_max,
    };
    let c = Conv {
        rotsum: 0.0,
        cd: f64::from(COLL_DIST),
        rcd: f64::from(REAL_COLL_DIST),
        eps: f64::from(MIN_FRICTION_DIST),
        xr: f64::from(md.sum_extra_radius),
        len: f64::from(md.len_numerator),
    };
    if sa == ST_BALL && sb == ST_BALL {
        ev.ball_ball(md);
    } else if sa == ST_BALL && sb <= ST_FACE {
        ev.ball_poly(md);
    } else {
        let mut c = c;
        c.rotsum = ev.sims[0].core.abs_omega + ev.sims[1].core.abs_omega;
        let (Some(fa), Some(fb)) = (md.syn[ka].feat, md.syn[kb].feat) else {
            return none;
        };
        match sa * 4 + sb {
            0 => ev.conv_pp(&c, fa.edge, fb.edge, md),
            1 => ev.conv_pk(&c, fa.edge, fb.edge),
            2 => ev.conv_pf(&c, fa.edge, fb.edge),
            5 => ev.conv_kk(&c, fa.edge, fb.edge, md),
            _ => {}
        }
    }
    Event {
        code: ev.code,
        time: ev.time,
    }
}

/// The ledge a synapse's feature lies in, from its object's surface.
fn feat_ledge<'a>(md: &Mindist, sides: &[EventSide<'a>; 2], k: usize) -> Option<&'a Ledge> {
    let f = md.syn[k].feat?;
    sides[k].surface?.ledges.get(f.ledge)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::convex;
    use crate::ledge::Surface;
    use crate::mindist::{Feat, Synapse};
    use crate::minimize::{Side, Solver};

    /// A unit cube in its object space, as a one-ledge surface.
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

    fn identity() -> [[f64; 3]; 3] {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    fn cache(vv: V3) -> ObjCache {
        ObjCache {
            r: identity(),
            vv,
            core_pos: vv,
        }
    }

    fn core(speed: f64) -> CoreInfo {
        CoreInfo {
            abs_omega: 0.0,
            current_speed: speed,
            max_surface_rot_speed: 0.0,
            upper_limit_radius: 0.866,
            inv_object_diameter: 1.0,
        }
    }

    /// Object 0 stays put; object 1 moves along x with the given velocity from x = 2.
    struct Line {
        vx: f64,
    }

    impl Motion for Line {
        fn matrix_at(&self, obj: usize, t: f64) -> Matrix {
            let vv = if obj == 0 { [0.0; 3] } else { [2.0 + self.vx * t, 0.0, 0.0] };
            Matrix { r: identity(), vv }
        }
    }

    /// The mindist of the two cubes at gap 1, solved to its closest features as the recalc does.
    fn gap_one_mindist(cube: &Surface) -> Mindist {
        let mut md = Mindist::default();
        let feat = Feat {
            ledge: 0,
            edge: Edge { tri: 0, slot: 0 },
        };
        md.syn[0] = Synapse {
            obj: 0,
            feat: Some(feat),
            status: ST_POINT,
        };
        md.syn[1] = Synapse {
            obj: 1,
            feat: Some(feat),
            status: ST_POINT,
        };
        let sides = [
            Side {
                cache: cache([0.0; 3]),
                surface: Some(cube),
            },
            Side {
                cache: cache([2.0, 0.0, 0.0]),
                surface: Some(cube),
            },
        ];
        for _ in 0..8 {
            let rc = {
                let mut s = Solver::new(&mut md, sides, 20);
                let rc = s.step();
                if rc == 3 {
                    s.fix_backside();
                }
                rc
            };
            if rc == 1 || rc == 2 {
                break;
            }
        }
        md
    }

    fn event_sides<'a>(cube: &'a Surface, vx: f64) -> [EventSide<'a>; 2] {
        [
            EventSide {
                obj: 0,
                cache: cache([0.0; 3]),
                core: core(0.0),
                simulated: false,
                surface: Some(cube),
            },
            EventSide {
                obj: 1,
                cache: cache([2.0, 0.0, 0.0]),
                core: core(vx.abs()),
                simulated: true,
                surface: Some(cube),
            },
        ]
    }

    #[test]
    fn an_approaching_cube_meets_the_other_at_the_gap_time() {
        let cube = unit_cube();
        let md = gap_one_mindist(&cube);
        assert!((md.len_numerator - 1.0).abs() < 1e-4, "gap {}", md.len_numerator);
        // the gap closes at speed 10 to the collision distance 0.01: t = 0.99 / 10. The search covers 20 grid steps
        // of 5 ms from t_now, so the window is 0.1 s
        let ev = solve(&md, event_sides(&cube, -10.0), &Line { vx: -10.0 }, 0.0, 0.1, 10.0, 10.0);
        assert!(ev.code != 0, "an event is found");
        assert!((ev.time - 0.099).abs() < 0.005, "time {}", ev.time);
    }

    #[test]
    fn a_cube_moving_away_has_no_event_in_the_psi() {
        let cube = unit_cube();
        let md = gap_one_mindist(&cube);
        let ev = solve(&md, event_sides(&cube, 10.0), &Line { vx: 10.0 }, 0.0, 0.1, 10.0, 10.0);
        assert_eq!(ev.code, 0, "no event: {ev:?}");
    }
}
