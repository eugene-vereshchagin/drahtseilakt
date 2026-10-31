// Port of OpenBallance src/phys/phys_minimize.c (GPL-3.0; see the LICENSE of the repository): the minimize solver
// (docs/ivp_collision.md 3.9-3.10, 3.13, 5.5). It walks the closest features of two ledges (or a ball and a ledge) and
// writes the mindist's gap, normal and synapse features. Each converged case leaves the normal pointing from the second
// sorted synapse's object toward the first's, `len_numerator` the gap minus the extra radii, and
// `contact_dot_diff_center` the core offset along that normal.
//
// Return codes: 1 converged, 2 failed (a loop, too many steps, coincident points), 3 backside (status 5, and `pos`
// marks the point), 5 an impossible pairing the upstream asserts on.
//
// Two representation changes against the upstream, both invisible to the numbers. An edge is `Edge` (triangle, slot)
// within a ledge, and a feature carries the ledge's index. The upstream's loop check hashes pointers; here it hashes
// the (synapse, ledge, triangle, slot) identities, which is the same equality. The arithmetic keeps the upstream's
// f32 and f64 steps, where those steps are what the numbers depend on.

use crate::ledge::{Edge, Ledge, Surface};
use crate::math::{self, isqrt4, isqrt5, normalize4, normalize_len};
use crate::mindist::{Feat, Mindist, ObjCache, MDF_SORT, ST_BACKSIDE, ST_BALL, ST_EDGE, ST_FACE, ST_POINT};

type V3 = [f64; 3];
type P3 = [f32; 3];

fn dot3(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// `a - b` of two f32 points, the differences formed exactly and stored in f64 (`fsub_d`).
fn fsub_d(a: P3, b: P3) -> V3 {
    [f64::from(a[0]) - f64::from(b[0]), f64::from(a[1]) - f64::from(b[1]), f64::from(a[2]) - f64::from(b[2])]
}

/// `|q - p|^2` of two f32 points, summed in f64 and stored f32 (`flen2`).
fn flen2(q: P3, p: P3) -> f32 {
    let d = fsub_d(q, p);
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) as f32
}

fn all_inside(w: &[f32; 4]) -> bool {
    !w[0].is_sign_negative() && !w[1].is_sign_negative() && !w[2].is_sign_negative()
}

/// Object space to world (`0x10018d10`).
fn obj_to_world(c: &ObjCache, p: V3) -> V3 {
    let x = math::rmul(&c.r, &p);
    [x[0] + c.vv[0], x[1] + c.vv[1], x[2] + c.vv[2]]
}

/// World to object space (`0x10018ca0`).
fn world_to_obj(c: &ObjCache, w: V3) -> V3 {
    math::rtmul(&c.r, &[w[0] - c.vv[0], w[1] - c.vv[1], w[2] - c.vv[2]])
}

/// An f32 object-space point of `a`'s object, in `b`'s object space (`0x10020710`, the f32 matrix entries).
fn pos_other_space(e: Edge, a: &Info, b: &Info) -> V3 {
    let p = a.p(e);
    let mut w = [0.0f64; 3];
    for k in 0..3 {
        let s = (a.cache.r[k][0] as f32) * p[0] + (a.cache.r[k][1] as f32) * p[1] + (a.cache.r[k][2] as f32) * p[2];
        w[k] = f64::from(s) + a.cache.vv[k];
    }
    world_to_obj(&b.cache, w)
}

/// A point given in `a`'s object space, in `b`'s (`0x100208f0`, in f64).
fn fpos_other_space(p: P3, a: &Info, b: &Info) -> V3 {
    let w = obj_to_world(&a.cache, [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]);
    world_to_obj(&b.cache, w)
}

/// A direction in `a`'s object space, in `b`'s: `R_B^T R_A v` (`0x10020810`).
fn dir_other_space(v: V3, a: &Info, b: &Info) -> V3 {
    let w = math::rmul(&a.cache.r, &v);
    math::rtmul(&b.cache.r, &w)
}

/// The f64 differences of the object's own points. Shared by the cases below.
fn lerp3(a: V3, b: V3, t: f64) -> V3 {
    [a[0] * (1.0 - t) + b[0] * t, a[1] * (1.0 - t) + b[1] * t, a[2] * (1.0 - t) + b[2] * t]
}

/// The barycentric weights of X's projection in the triangle of `e` (`0x10020ad0`): the weights of the edges `e`,
/// next(e), prev(e) in the order the caller reads them, and the determinant. Only the four outputs are f32.
pub(crate) fn tri_qr_vals(l: &Ledge, e: Edge, x: V3) -> [f32; 4] {
    let t = &l.tris[e.tri];
    let (q0, q1, q2) = (l.points[t.start[0]], l.points[t.start[1]], l.points[t.start[2]]);
    let a = fsub_d(q0, q1);
    let b = fsub_d(q2, q1);
    let d = [x[0] - f64::from(q1[0]), x[1] - f64::from(q1[1]), x[2] - f64::from(q1[2])];
    let aa = dot3(a, a);
    let bb = dot3(b, b);
    let ab = dot3(a, b);
    let det = aa * bb - ab * ab;
    let da = dot3(d, a);
    let db = dot3(d, b);
    let w0 = da * bb - db * ab;
    let w2 = db * aa - da * ab;
    let w1 = det - w0 - w2;
    let w = [w0, w1, w2];
    let mut out = [0.0f32; 4];
    for k in 0..3 {
        out[k] = w[(e.slot + k + 2) % 3] as f32;
    }
    out[3] = det as f32;
    out
}

/// The edge-function values of `e` at X: (X - P).d and (N - X).d in f32 (`0x10020a10`).
fn kvals(l: &Ledge, e: Edge, x: V3) -> [f32; 2] {
    let p = l.p(e);
    let n = l.p(Ledge::next(e));
    let d = fsub_d(n, p);
    let a = [x[0] - f64::from(p[0]), x[1] - f64::from(p[1]), x[2] - f64::from(p[2])];
    let b = [f64::from(n[0]) - x[0], f64::from(n[1]) - x[1], f64::from(n[2]) - x[2]];
    [dot3(a, d) as f32, dot3(b, d) as f32]
}

/// The unscaled normal of the triangle of `e` (`0x10021280`): (N - P) x (prev - P), oriented (`hesse`).
fn hesse(l: &Ledge, e: Edge) -> V3 {
    let p = l.p(e);
    let n = l.p(Ledge::next(e));
    let r = l.p(Ledge::prev(e));
    let a = fsub_d(r, p);
    let b = fsub_d(n, p);
    cross3(b, a)
}

/// Squared distance to the infinite line of `e` (`0x10021420`).
fn line_dist2(l: &Ledge, e: Edge, x: V3) -> f64 {
    let p = l.p(e);
    let n = l.p(Ledge::next(e));
    let a = [x[0] - f64::from(p[0]), x[1] - f64::from(p[1]), x[2] - f64::from(p[2])];
    let d = fsub_d(n, p);
    let c = cross3(a, d);
    dot3(c, c) * (1.0 / dot3(d, d))
}

/// Squared distance to the start point of `e` (`0x100216f0`).
fn point_dist2(l: &Ledge, e: Edge, x: V3) -> f64 {
    let p = l.p(e);
    let a = [x[0] - f64::from(p[0]), x[1] - f64::from(p[1]), x[2] - f64::from(p[2])];
    dot3(a, a)
}

/// Squared distance to the segment of `e` (`0x10021740`).
fn seg_dist2(x: V3, l: &Ledge, e: Edge) -> f64 {
    let s = kvals(l, e, x);
    if s[0] >= 0.0 && s[1] >= 0.0 {
        return line_dist2(l, e, x);
    }
    if s[0] < 0.0 {
        return point_dist2(l, e, x);
    }
    point_dist2(l, Ledge::next(e), x)
}

/// The edge pair of an edge-edge case, both in the frame of the first object (`0x10020c60`).
#[derive(Clone, Copy)]
struct Kk<'a> {
    ik: Info<'a>,
    il: Info<'a>,
    k: Edge,
    l: Edge,
    l0: P3,
    l1: P3,
    k0: V3,
    k1: V3,
    kdir: V3,
    ldir: V3,
    cross: V3,
}

fn kk_setup<'a>(k: Edge, l: Edge, ik: Info<'a>, il: Info<'a>) -> Kk<'a> {
    let l0 = il.p(l);
    let l1 = il.p(Ledge::next(l));
    let k0 = pos_other_space(k, &ik, &il);
    let k1 = pos_other_space(Ledge::next(k), &ik, &il);
    let kd = fsub_d(ik.p(Ledge::next(k)), ik.p(k));
    let kdir = dir_other_space(kd, &ik, &il);
    let ldir = fsub_d(l1, l0);
    let cross = cross3(kdir, ldir);
    Kk {
        ik,
        il,
        k,
        l,
        l0,
        l1,
        k0,
        k1,
        kdir,
        ldir,
        cross,
    }
}

/// The parameters along the two edges: r[0..2] along K, r[2..4] along L, unscaled. Returns 0 in the parallel case,
/// where the parameters are sampled (`0x10063880`, `0x10020ecd` .. `0x10020fbe`).
fn kk_params(kk: &Kk, r: &mut [f32; 4]) -> i32 {
    if dot3(kk.cross, kk.cross) <= 1e-18 {
        const TBL: [f32; 11] = [-1.0, 0.5, 2.0, 0.0, 1.0, -0.001, 0.001, 0.999, 1.001, -1e-6, 1e-6];
        let mut best = 1e101;
        for &t in &TBL {
            let x = lerp3(kk.k0, kk.k1, f64::from(t));
            let d = line_dist2(kk.il.l, kk.l, x);
            if d < best {
                best = d;
                r[0] = t;
                r[1] = 1.0f32 - t;
                let v = kvals(kk.il.l, kk.l, x);
                r[2] = v[0];
                r[3] = v[1];
            }
        }
        let a0 = fpos_other_space(kk.l0, &kk.il, &kk.ik);
        let a1 = fpos_other_space(kk.l1, &kk.il, &kk.ik);
        for &t in TBL.iter().take(9) {
            let x = lerp3(a0, a1, f64::from(t));
            let d = line_dist2(kk.ik.l, kk.k, x);
            if d < best {
                best = d;
                r[2] = t;
                r[3] = 1.0f32 - t;
                let v = kvals(kk.ik.l, kk.k, x);
                r[0] = v[0];
                r[1] = v[1];
            }
        }
        return 0;
    }
    let l0 = [f64::from(kk.l0[0]), f64::from(kk.l0[1]), f64::from(kk.l0[2])];
    let l1 = [f64::from(kk.l1[0]), f64::from(kk.l1[1]), f64::from(kk.l1[2])];
    let nk = cross3(kk.kdir, kk.cross);
    let nl = cross3(kk.ldir, kk.cross);
    let a = dot3(l0, nk);
    let b = dot3(l1, nk);
    let c = dot3(kk.k0, nk);
    r[2] = ((a - c) * (a - b)) as f32;
    r[3] = ((c - b) * (a - b)) as f32;
    let f8 = dot3(nl, kk.k0);
    let f9 = dot3(nl, kk.k1);
    let f3 = dot3(l0, nl);
    let den = f8 - f9;
    r[0] = ((f8 - f3) * den) as f32;
    r[1] = ((f3 - f9) * den) as f32;
    1
}

/// The squared distance of two edges' segments in the edge-edge case (`0x10020da0`).
fn kk_qlen(kk: &Kk) -> f64 {
    let c2 = dot3(kk.cross, kk.cross);
    if c2 > 1e-24 {
        let l0 = [f64::from(kk.l0[0]), f64::from(kk.l0[1]), f64::from(kk.l0[2])];
        let d = dot3(kk.k0, kk.cross) - dot3(l0, kk.cross);
        return d * d / c2;
    }
    seg_dist2(kk.k0, kk.il.l, kk.l)
}

/// Like `kk_qlen` for two edges given directly (`qlen_kk`).
fn qlen_kk(k: Edge, l: Edge, ik: Info, il: Info) -> f64 {
    let kk = kk_setup(k, l, ik, il);
    let mut r = [0.0f32; 4];
    kk_params(&kk, &mut r);
    if r[2].is_sign_negative() || r[3].is_sign_negative() {
        let l0k = pos_other_space(l, &il, &ik);
        let l1k = pos_other_space(Ledge::next(l), &il, &ik);
        if !r[0].is_sign_negative() && !r[1].is_sign_negative() {
            if r[2] <= 0.0 {
                return seg_dist2(l0k, ik.l, k);
            }
            return seg_dist2(l1k, ik.l, k);
        }
        let mut m = seg_dist2(l0k, ik.l, k);
        let x = seg_dist2(kk.k0, il.l, l);
        if x < m {
            m = x;
        }
        let x = seg_dist2(l1k, ik.l, k);
        if x < m {
            m = x;
        }
        let x = seg_dist2(kk.k1, il.l, l);
        if x < m {
            m = x;
        }
        return m;
    }
    if r[0] <= 0.0 {
        return seg_dist2(kk.k0, il.l, l);
    }
    if r[1] > 0.0 {
        return kk_qlen(&kk);
    }
    seg_dist2(kk.k1, il.l, l)
}

/// The triangle's edges and the edges around the point of `e` (`FOR_RING`): each starts at a neighbour and ends at
/// the point of `e`, from prev(opp(prev(e))) round to prev(e).
fn ring(l: &Ledge, e: Edge) -> Vec<Edge> {
    let stop = Ledge::prev(e);
    let mut out = Vec::new();
    let mut g = Ledge::prev(l.opp(Ledge::prev(e)));
    // a valid ledge closes its ring; the bound only stops a corrupt one
    let bound = 3 * l.tris.len() + 3;
    loop {
        out.push(g);
        if g == stop || out.len() > bound {
            return out;
        }
        g = Ledge::prev(l.opp(g));
    }
}

/// The edge of the triangle of `f` (in walk order from `f`) nearest to X (`nearest_edge`).
fn nearest_edge(l: &Ledge, f: Edge, x: V3) -> Edge {
    let mut best = f;
    let mut bq = 1e101;
    let mut e = f;
    for _ in 0..3 {
        let q = seg_dist2(x, l, e);
        if q < bq {
            bq = q;
            best = e;
        }
        e = Ledge::next(e);
    }
    best
}

/// A solver's view of one of the mindist's objects: its cache and, for a polygon, its surface (`None` for a ball).
#[derive(Clone, Copy)]
pub struct Side<'a> {
    pub cache: ObjCache,
    pub surface: Option<&'a Surface>,
}

/// One synapse's feature for the case code: the synapse index (0 or 1), the ledge, the cache copy.
#[derive(Clone, Copy)]
struct Info<'a> {
    k: usize,
    feat: usize,
    cache: ObjCache,
    l: &'a Ledge,
}

impl<'a> Info<'a> {
    fn p(&self, e: Edge) -> P3 {
        self.l.p(e)
    }
}

/// The identity of a loop-check entry: the case tag and, where the upstream passes an edge, its synapse, ledge and
/// slot.
type Key = (u8, Option<(usize, usize, usize, usize)>);

fn key(i: &Info, tag: u8, e: Edge) -> Key {
    (tag, Some((i.k, i.feat, e.tri, e.slot)))
}

fn key_none(tag: u8) -> Key {
    (tag, None)
}

/// The minimize solver for one mindist (`IvpMinSolver`, 3.9.1): the mindist, its two sides, the loop countdown and the
/// loop hash, and the backside point.
pub struct Solver<'a, 'm> {
    md: &'m mut Mindist,
    sides: [Side<'a>; 2],
    count: i32,
    hash: Vec<(Key, Key)>,
    pos: V3,
}

impl<'a, 'm> Solver<'a, 'm> {
    /// A solver for `md` with its sides (synapse 0 and synapse 1 in that order), counting loop steps from `count`.
    pub fn new(md: &'m mut Mindist, sides: [Side<'a>; 2], count: i32) -> Self {
        Solver {
            md,
            sides,
            count,
            hash: Vec::new(),
            pos: [0.0; 3],
        }
    }

    /// The backside point of the last step that returned 3, in the status-5 synapse's object space.
    pub fn pos(&self) -> V3 {
        self.pos
    }

    /// The mindist the solver writes into, for the recalc loop's flag updates.
    pub fn mindist(&mut self) -> &mut Mindist {
        self.md
    }

    /// The info of synapse `k`: its feature, ledge and cache. None when the synapse has no polygon behind it.
    fn info(&self, k: usize) -> Option<Info<'a>> {
        let feat = self.md.syn[k].feat?;
        let surface = self.sides[k].surface?;
        let l = surface.ledges.get(feat.ledge)?;
        Some(Info {
            k,
            feat: feat.ledge,
            cache: self.sides[k].cache,
            l,
        })
    }

    /// The edge a synapse holds (its feature's edge).
    fn edge_of(&self, k: usize) -> Option<Edge> {
        self.md.syn[k].feat.map(|f| f.edge)
    }

    /// Records the feature of `i` on edge `e` with the status (`set`).
    fn set(&mut self, i: &Info, e: Edge, st: i16) {
        self.md.syn[i.k].feat = Some(Feat {
            ledge: i.feat,
            edge: e,
        });
        self.md.syn[i.k].status = st;
    }

    /// Makes `i` the first sorted synapse (`make_first`).
    fn make_first(&mut self, i: &Info) {
        if i.k != self.md.sel() {
            self.md.flags ^= MDF_SORT;
        }
    }

    fn store_n(&mut self, n: V3) {
        self.md.normal = [n[0] as f32, n[1] as f32, n[2] as f32];
    }

    /// The core offset along the normal (`dot_diff`).
    fn dot_diff(&mut self, a: &ObjCache, b: &ObjCache) {
        let d = [a.core_pos[0] - b.core_pos[0], a.core_pos[1] - b.core_pos[1], a.core_pos[2] - b.core_pos[2]];
        let n = self.md.normal;
        self.md.contact_dot_diff_center = (d[0] * f64::from(n[0]) + d[1] * f64::from(n[1]) + d[2] * f64::from(n[2])) as f32;
    }

    /// The loop check of a step (`LOOP_CHECK`): counts the step, and from the countdown on remembers the pairs; true
    /// when the pair was seen, or the hash is full (the caller fails the solve).
    fn looped(&mut self, a: Key, b: Key) -> bool {
        self.count -= 1;
        if self.count >= 0 {
            return false;
        }
        let (hi, lo) = if a > b { (a, b) } else { (b, a) };
        if self.hash.iter().rev().any(|&(h, l)| h == hi && l == lo) {
            return true;
        }
        if self.hash.len() > 0xff {
            return true;
        }
        self.hash.push((hi, lo));
        false
    }

    // ---- the cases ----

    /// The point of PF's face test: the point of `pa` inside the triangle `f` (`PF`, 0x10030c60).
    fn pf(&mut self, p: Edge, f: Edge, a: Info, b: Info) -> i32 {
        let x = pos_other_space(p, &a, &b);
        let w = tri_qr_vals(b.l, f, x);
        if !all_inside(&w) {
            let k = nearest_edge(b.l, f, x);
            return self.pk(p, k, a, b);
        }
        self.pf_core(p, x, f, a, b)
    }

    /// PF core (`0x10030f80`): the point P over the face F (x is P in B's space).
    fn pf_core(&mut self, p: Edge, x: V3, f: Edge, a: Info, b: Info) -> i32 {
        if self.looped(key(&a, 0, p), key(&b, 2, f)) {
            return 2;
        }
        let mut h = hesse(b.l, f);
        normalize4(&mut h);
        let n_w = math::rmul(&b.cache.r, &h);
        let mut n_a = math::rtmul(&a.cache.r, &n_w);
        let f0 = b.p(f);
        let dist = dot3(h, x) - (h[0] * f64::from(f0[0]) + h[1] * f64::from(f0[1]) + h[2] * f64::from(f0[2]));
        self.md.len_numerator = dist as f32;
        self.store_n(n_w);
        if self.md.len_numerator < 0.0 {
            for c in n_a.iter_mut() {
                *c *= -1.0;
            }
        }
        self.md.len_numerator = self.md.len_numerator - self.md.sum_extra_radius;
        self.dot_diff(&a.cache, &b.cache);
        let pp = a.p(p);
        let mut best = 0.0;
        let mut best_e: Option<Edge> = None;
        for g in ring(a.l, p) {
            let q = a.p(g);
            let qp = fsub_d(q, pp);
            let proj = qp[0] * n_a[0] + qp[1] * n_a[1] + qp[2] * n_a[2];
            if proj < 0.0 {
                let val = isqrt4(flen2(q, pp)) * proj;
                if val < best {
                    best = val;
                    best_e = Some(g);
                }
            }
        }
        let Some(best_e) = best_e else {
            self.set(&a, p, ST_POINT);
            self.set(&b, f, ST_FACE);
            if f64::from(self.md.len_numerator) + f64::from(self.md.sum_extra_radius) >= 0.0 {
                return 1;
            }
            self.pos = x;
            self.set(&b, f, ST_BACKSIDE);
            return 3;
        };
        let xq = pos_other_space(best_e, &a, &b);
        let w = tri_qr_vals(b.l, f, xq);
        if all_inside(&w) {
            return self.pf_core(best_e, xq, f, a, b);
        }
        let fk = [f, Ledge::next(f), Ledge::next(Ledge::next(f))];
        let mut nneg = 0;
        let mut kneg = 0;
        for k in 0..3 {
            if w[k] < 0.0 {
                nneg += 1;
                kneg = k;
            }
        }
        if nneg == 1 {
            return self.kk(best_e, fk[kneg], a, b);
        }
        let mut pick: Option<Edge> = None;
        let mut bq = 1e101;
        for k in 0..3 {
            if !(w[k] <= 0.0) {
                continue;
            }
            let q = qlen_kk(best_e, fk[k], a, b);
            if q < bq {
                bq = q;
                pick = Some(fk[k]);
            }
        }
        self.kk(best_e, pick.unwrap_or(fk[kneg]), a, b)
    }

    /// PK entry (`0x10032da0`): the point P against the edge K.
    fn pk(&mut self, p: Edge, k: Edge, a: Info, b: Info) -> i32 {
        let x = pos_other_space(p, &a, &b);
        let uv = kvals(b.l, k, x);
        if uv[0] < 0.0 {
            return self.pp(p, k, a, b);
        }
        if uv[1] < 0.0 {
            return self.pp(p, Ledge::next(k), a, b);
        }
        self.pk_core(p, k, a, b)
    }

    /// A vector perpendicular to d (`0x1000f870`).
    fn any_perpendicular(d: V3) -> V3 {
        let a = [d[0].abs(), d[1].abs(), d[2].abs()];
        let mut ax = [0.0; 3];
        let idx = if a[0] <= a[1] && a[0] <= a[2] {
            0
        } else if a[1] <= a[2] {
            1
        } else {
            2
        };
        ax[idx] = 1.0;
        cross3(d, ax)
    }

    /// PK core (`0x10032e60`): the point P against the edge K, when the point projects onto the edge.
    fn pk_core(&mut self, p: Edge, k: Edge, a: Info, b: Info) -> i32 {
        if self.looped(key(&a, 0, p), key(&b, 1, k)) {
            return 2;
        }
        let x = pos_other_space(p, &a, &b);
        let k0 = b.p(k);
        let d = fsub_d(b.p(Ledge::next(k)), k0);
        let t1 = fsub_d(b.p(Ledge::prev(k)), k0);
        let t2 = fsub_d(b.p(Ledge::prev(b.l.opp(k))), k0);
        let xs = [x[0] - f64::from(k0[0]), x[1] - f64::from(k0[1]), x[2] - f64::from(k0[2])];
        let n1 = cross3(d, t1);
        let n2 = cross3(t2, d);
        let s1 = dot3(xs, n1);
        let s2 = dot3(xs, n2);
        let w1 = tri_qr_vals(b.l, k, x);
        let w2 = tri_qr_vals(b.l, b.l.opp(k), x);
        if w1[0] > 0.0 {
            if w2[0] <= 0.0 || s2 <= 0.0 {
                return self.pf(p, k, a, b);
            }
            return self.pf(p, b.l.opp(k), a, b);
        }
        if w2[0] > 0.0 {
            return self.pf(p, b.l.opp(k), a, b);
        }
        if s1 < 0.0 && s2 < 0.0 {
            self.set(&a, p, ST_POINT);
            self.set(&b, k, ST_BACKSIDE);
            self.pos = x;
            return 3;
        }
        let c = cross3(d, xs);
        let id = 1.0 / dot3(d, d);
        let q = dot3(c, c) * id;
        if q <= 1e-19 {
            // exact contact on the edge line: the upstream stores an object-space normal and walks the neighbours
            // with it (3.13.R2.2); treated as converged, the normal rotated to world (9.3)
            let mut pv = Self::any_perpendicular(d);
            normalize4(&mut pv);
            let n_w = math::rmul(&b.cache.r, &pv);
            self.md.len_numerator = -self.md.sum_extra_radius;
            self.store_n(n_w);
            self.dot_diff(&a.cache, &b.cache);
            self.set(&a, p, ST_POINT);
            self.set(&b, k, ST_EDGE);
            return 1;
        }
        let si = isqrt5(q);
        self.md.len_numerator = (si * q - f64::from(self.md.sum_extra_radius)) as f32;
        let dc = cross3(d, c);
        let n_w = math::rmul(&b.cache.r, &dc);
        let n = [n_w[0] * (-si * id), n_w[1] * (-si * id), n_w[2] * (-si * id)];
        self.store_n(n);
        self.dot_diff(&a.cache, &b.cache);
        // is a neighbour of P closer (n_w, unscaled, points from P toward the edge)?
        let n_a = math::rtmul(&a.cache.r, &n_w);
        let mut best = q * 1e-12;
        let mut best_e: Option<Edge> = None;
        let pp = a.p(p);
        for g in ring(a.l, p) {
            let qv = a.p(g);
            let qp = fsub_d(qv, pp);
            let proj = qp[0] * n_a[0] + qp[1] * n_a[1] + qp[2] * n_a[2];
            if proj > 0.0 {
                let val = isqrt4(flen2(qv, pp)) * proj;
                if val > best {
                    best = val;
                    best_e = Some(g);
                }
            }
        }
        let Some(best_e) = best_e else {
            self.set(&a, p, ST_POINT);
            self.set(&b, k, ST_EDGE);
            return 1;
        };
        if best < 1e-8 {
            // nearly parallel: avoid flip-flopping
            let kk = kk_setup(k, best_e, b, a);
            let mut r = [0.0f32; 4];
            kk_params(&kk, &mut r);
            if r[2] < 0.0 {
                self.set(&a, p, ST_POINT);
                self.set(&b, k, ST_EDGE);
                return 1;
            }
        }
        self.kk(best_e, k, a, b)
    }

    /// PP (`0x10032350`): two points.
    fn pp(&mut self, pa: Edge, pb: Edge, a: Info, b: Info) -> i32 {
        if self.looped(key(&a, 0, pa), key(&b, 0, pb)) {
            return 2;
        }
        let wa = obj_to_world(&a.cache, fsub_point(a.p(pa)));
        let wb = obj_to_world(&b.cache, fsub_point(b.p(pb)));
        let xa_b = world_to_obj(&b.cache, wa);
        let xb_a = world_to_obj(&a.cache, wb);
        let d = [wa[0] - wb[0], wa[1] - wb[1], wa[2] - wb[2]];
        let q = dot3(d, d);
        if q <= 1e-12 {
            return 2;
        }
        let si = isqrt5(q);
        self.md.len_numerator = (si * q - f64::from(self.md.sum_extra_radius)) as f32;
        self.store_n([d[0] * si, d[1] * si, d[2] * si]);
        self.dot_diff(&a.cache, &b.cache);
        let mut best = 0.0;
        let mut best_e: Option<(usize, Edge)> = None;
        let ps = [pa, pb];
        let is = [a, b];
        let other = [xb_a, xa_b];
        for k in 0..2 {
            let p = is[k].p(ps[k]);
            let v = [other[k][0] - f64::from(p[0]), other[k][1] - f64::from(p[1]), other[k][2] - f64::from(p[2])];
            let base = f64::from(p[0]) * v[0] + f64::from(p[1]) * v[1] + f64::from(p[2]) * v[2];
            for g in ring(is[k].l, ps[k]) {
                let qv = is[k].p(g);
                let proj = (f64::from(qv[0]) * v[0] + f64::from(qv[1]) * v[1] + f64::from(qv[2]) * v[2]) - base;
                if proj > 0.0 {
                    let val = isqrt4(flen2(qv, p)) * proj;
                    if val > best {
                        best = val;
                        best_e = Some((k, g));
                    }
                }
            }
        }
        if let Some((side, be)) = best_e {
            let si_ = is[side];
            let oi = is[side ^ 1];
            let uv = kvals(si_.l, be, other[side]);
            if uv[1] > 0.0 {
                if uv[0] >= 0.0 {
                    self.make_first(&oi);
                    return self.pk_core(ps[side ^ 1], be, oi, si_);
                }
                self.make_first(&si_);
                return self.pp(be, ps[side ^ 1], si_, oi);
            }
        }
        self.set(&a, pa, ST_POINT);
        self.set(&b, pb, ST_POINT);
        1
    }

    /// KK entry (`0x10031440`): two edges.
    fn kk(&mut self, ka: Edge, kb: Edge, a: Info, b: Info) -> i32 {
        let kk = kk_setup(ka, kb, a, b);
        let mut r = [0.0f32; 4];
        kk_params(&kk, &mut r);
        if r[2] >= 0.0 && r[3] >= 0.0 {
            if r[0] < 0.0 {
                return self.pk(ka, kb, a, b);
            }
            if r[1] >= 0.0 {
                return self.kk_core(ka, kb, &kk, r, a, b);
            }
            return self.pk(Ledge::next(ka), kb, a, b);
        }
        if r[0] >= 0.0 && r[1] >= 0.0 {
            self.make_first(&b);
            let first = if r[2] >= 0.0 { Ledge::next(kb) } else { kb };
            return self.pk(first, ka, b, a);
        }
        // both out of range
        let pa = if r[0] < r[1] { ka } else { Ledge::next(ka) };
        let qa = if pa == ka { Ledge::next(ka) } else { ka };
        let pb = if r[2] < r[3] { kb } else { Ledge::next(kb) };
        let qb = if pb == kb { Ledge::next(kb) } else { kb };
        let x = pos_other_space(pa, &a, &b);
        let uv = kvals(b.l, kb, x);
        if uv[0] >= 0.0 && uv[1] >= 0.0 {
            return self.pk(pa, kb, a, b);
        }
        let x2 = pos_other_space(pb, &b, &a);
        let uv2 = kvals(a.l, ka, x2);
        if uv2[0] >= 0.0 && uv2[1] >= 0.0 {
            self.make_first(&b);
            return self.pk(pb, ka, b, a);
        }
        if uv[0] * r[2] < 0.0 {
            // r[2], not r[3] (3.13.R2.1)
            return self.pp(pa, qb, a, b);
        }
        if uv2[0] * r[0] < 0.0 {
            self.make_first(&b);
            return self.pp(pb, qa, b, a);
        }
        self.pp(pa, pb, a, b)
    }

    /// KK core (`0x100317a0`): two edges whose parameters are inside their ranges, then the four faces around them.
    fn kk_core(&mut self, ka: Edge, kb: Edge, kk: &Kk, r: [f32; 4], a: Info, b: Info) -> i32 {
        if self.looped(key(&a, 1, ka), key(&b, 1, kb)) {
            return 2;
        }
        let c = kk.cross;
        let sf = ((f64::from(kk.l0[0]) - kk.k0[0]) * c[0] + (f64::from(kk.l0[1]) - kk.k0[1]) * c[1] + (f64::from(kk.l0[2]) - kk.k0[2]) * c[2]) as f32;
        let neg: i32 = i32::from(sf.is_sign_negative());
        let inv = isqrt5(dot3(c, c));
        let n_w = math::rmul(&b.cache.r, &c);
        let n_a = math::rtmul(&a.cache.r, &n_w);
        self.md.len_numerator = ((f64::from(sf) * inv).abs() - f64::from(self.md.sum_extra_radius)) as f32;
        let kf = (neg as f32 - 0.5f32) * 2.0f32;
        let kscale = f64::from(kf) * inv;
        self.store_n([n_w[0] * kscale, n_w[1] * kscale, n_w[2] * kscale]);
        self.dot_diff(&a.cache, &b.cache);
        let kb0_a = fpos_other_space(kk.l0, &b, &a);
        let kb1_a = fpos_other_space(kk.l1, &b, &a);

        // the four faces: (triangle, the other edge, its start point, the owner, the other info, the vector)
        struct Face<'b> {
            tri: Edge,
            other: Edge,
            point: V3,
            owner: Info<'b>,
            other_info: Info<'b>,
            vec: V3,
        }
        let faces = [
            Face {
                tri: a.l.opp(ka),
                other: kb,
                point: kb0_a,
                owner: b,
                other_info: a,
                vec: n_a,
            },
            Face {
                tri: ka,
                other: b.l.opp(kb),
                point: kb1_a,
                owner: b,
                other_info: a,
                vec: n_a,
            },
            Face {
                tri: b.l.opp(kb),
                other: ka,
                point: kk.k0,
                owner: a,
                other_info: b,
                vec: c,
            },
            Face {
                tri: kb,
                other: a.l.opp(ka),
                point: kk.k1,
                owner: a,
                other_info: b,
                vec: c,
            },
        ];
        let mut hs = [[0.0f64; 3]; 4];
        let mut flag = [0i32; 4];
        for i in 0..4 {
            hs[i] = hesse(faces[i].owner.l, faces[i].tri);
            let v = dot3(hs[i], faces[i].vec) as f32;
            flag[i] = i32::from(v.is_sign_negative()) ^ (i as i32 >> 1) ^ neg;
        }
        let mut best = -4e-12;
        let mut pick: Option<usize> = None;
        let mut pickj = 0usize;
        let mut pw = [0.0f32; 4];
        for i in 0..4 {
            let j = (flag[i] ^ i as i32 ^ neg) as usize;
            let kx = (i as i32 ^ neg) as usize;
            let v = [
                faces[kx].point[0] - faces[kx ^ 1].point[0],
                faces[kx].point[1] - faces[kx ^ 1].point[1],
                faces[kx].point[2] - faces[kx ^ 1].point[2],
            ];
            let dot = dot3(v, hs[i]);
            if dot < 0.0 {
                let hf = isqrt4(dot3(hs[i], hs[i]) as f32);
                let cc = isqrt4(dot3(v, v) as f32) * dot * hf;
                if cc < best {
                    let w = tri_qr_vals(faces[i].owner.l, faces[i].tri, faces[j].point);
                    if w[0] > 0.0 {
                        best = cc;
                        pick = Some(i);
                        pickj = j;
                        pw = w;
                    }
                }
            }
        }
        let Some(pick) = pick else {
            self.set(&a, ka, ST_EDGE);
            self.set(&b, kb, ST_EDGE);
            if flag[0] + flag[1] == 2 {
                self.set(&a, ka, ST_BACKSIDE);
                self.set(&b, kb, ST_FACE);
                self.pos = lerp3(kb0_a, kb1_a, f64::from(r[2]) / (f64::from(r[2]) + f64::from(r[3])));
                return 3;
            }
            if flag[2] + flag[3] == 2 {
                self.set(&a, ka, ST_FACE);
                self.set(&b, kb, ST_BACKSIDE);
                self.pos = lerp3(kk.k0, kk.k1, f64::from(r[0]) / (f64::from(r[0]) + f64::from(r[1])));
                return 3;
            }
            return 1;
        };
        let tri = faces[pick].tri;
        let other = faces[pickj].other;
        let oi = faces[pickj].other_info;
        let ow = faces[pick].owner;
        self.make_first(&oi);
        if all_inside(&pw) {
            return self.pf_core(other, faces[pickj].point, tri, oi, ow);
        }
        if pw[2] >= 0.0 {
            return self.kk(other, Ledge::next(tri), oi, ow);
        }
        if pw[1] >= 0.0 {
            return self.kk(other, Ledge::prev(tri), oi, ow);
        }
        // the upstream passes a stack address as the second call's first edge (3.13.R2.4); `other` is meant
        let q1 = qlen_kk(other, Ledge::next(tri), oi, ow);
        let q2 = qlen_kk(other, ow.l.opp(Ledge::prev(tri)), oi, ow);
        let next = if q2 < q1 { Ledge::prev(tri) } else { Ledge::next(tri) };
        self.kk(other, next, oi, ow)
    }

    /// FF init (`0x10019aa0`): the brute-force restart for face-face.
    fn ff(&mut self, fa: Edge, fb: Edge, a: Info, b: Info) -> i32 {
        let ea = |i: usize| Edge { tri: fa.tri, slot: i };
        let eb = |j: usize| Edge { tri: fb.tri, slot: j };
        let mut best = 1e101;
        for i in 0..3 {
            for j in 0..3 {
                let pa = fobj_world(&a, ea(i));
                let pb = fobj_world(&b, eb(j));
                let d = [pa[0] - pb[0], pa[1] - pb[1], pa[2] - pb[2]];
                if dot3(d, d) < best {
                    best = dot3(d, d);
                    self.set(&a, ea(i), ST_POINT);
                    self.set(&b, eb(j), ST_POINT);
                }
            }
        }
        // the two faces in turn: B's face against A's points, then A's face against B's points
        let faces = [b, a];
        let pts = [a, b];
        let ft = [fb, fa];
        let pt = [fa, fb];
        for f in 0..2 {
            let face = Edge { tri: ft[f].tri, slot: 0 };
            let mut n = hesse(faces[f].l, face);
            normalize4(&mut n);
            let p0 = faces[f].p(face);
            let dd = -(n[0] * f64::from(p0[0]) + n[1] * f64::from(p0[1]) + n[2] * f64::from(p0[2]));
            for i in 0..3 {
                let e = Edge { tri: pt[f].tri, slot: i };
                let x = pos_other_space(e, &pts[f], &faces[f]);
                let w = tri_qr_vals(faces[f].l, face, x);
                if !all_inside(&w) {
                    continue;
                }
                let mut q = dot3(n, x) + dd;
                q *= q;
                if q * 1.000000000001 < best {
                    best = q;
                    self.set(&pts[f], e, ST_POINT);
                    self.set(&faces[f], face, ST_FACE);
                }
            }
        }
        for i in 0..3 {
            for j in 0..3 {
                let kk = kk_setup(ea(i), eb(j), a, b);
                let mut r = [0.0f32; 4];
                kk_params(&kk, &mut r);
                if r.iter().any(|v| v.is_sign_negative()) {
                    continue;
                }
                let q = kk_qlen(&kk);
                if q * 1.000000000001 < best {
                    best = q;
                    self.set(&a, ea(i), ST_EDGE);
                    self.set(&b, eb(j), ST_EDGE);
                }
            }
        }
        let (mut first, mut second) = (a, b);
        if self.md.syn[second.k].status == ST_POINT && self.md.syn[first.k].status != ST_POINT {
            first = b;
            second = a;
        }
        self.make_first(&first);
        let sa = self.md.syn[first.k].status;
        let sb = self.md.syn[second.k].status;
        let (Some(ea_e), Some(eb_e)) = (self.edge_of(first.k), self.edge_of(second.k)) else {
            return 2;
        };
        if sa == ST_POINT && sb == ST_POINT {
            return self.pp(ea_e, eb_e, first, second);
        }
        if sa == ST_POINT && sb == ST_EDGE {
            return self.pk(ea_e, eb_e, first, second);
        }
        if sa == ST_POINT && sb == ST_FACE {
            return self.pf(ea_e, eb_e, first, second);
        }
        if sa == ST_EDGE && sb == ST_EDGE {
            return self.kk(ea_e, eb_e, first, second);
        }
        2
    }

    /// BF (`0x10030d50`, no loop check): the ball's centre over the face of `e`.
    fn bf(&mut self, bi: &ObjCache, e: Edge, i: &Info) -> i32 {
        let x = world_to_obj(&i.cache, bi.vv);
        let w = tri_qr_vals(i.l, e, x);
        if !all_inside(&w) {
            let k = nearest_edge(i.l, e, x);
            return self.bk(bi, k, i);
        }
        let mut n = hesse(i.l, e);
        normalize4(&mut n);
        let n_w = math::rmul(&i.cache.r, &n);
        self.store_n(n_w);
        self.set(i, e, ST_FACE);
        let p = i.p(e);
        let h = dot3(n, x) - (n[0] * f64::from(p[0]) + n[1] * f64::from(p[1]) + n[2] * f64::from(p[2]));
        if h >= 0.0 {
            self.md.len_numerator = (h - f64::from(self.md.sum_extra_radius)) as f32;
            self.dot_diff(bi, &i.cache);
            return 1;
        }
        self.set(i, e, ST_BACKSIDE);
        self.pos = x;
        3
    }

    /// BK core (`0x10032910`): the ball's centre over the edge `e`'s region.
    fn bk_core(&mut self, bi: &ObjCache, e: Edge, i: &Info) -> i32 {
        if self.looped(key_none(3), key(i, 1, e)) {
            return 2;
        }
        let x = world_to_obj(&i.cache, bi.vv);
        let p = i.p(e);
        let d = fsub_d(i.p(Ledge::next(e)), p);
        let r = fsub_d(i.p(Ledge::prev(e)), p);
        let sv = fsub_d(i.p(Ledge::prev(i.l.opp(e))), p);
        let xs = [x[0] - f64::from(p[0]), x[1] - f64::from(p[1]), x[2] - f64::from(p[2])];
        let n1 = cross3(d, r);
        let n2 = cross3(sv, d);
        let h1 = dot3(xs, n1);
        let h2 = dot3(xs, n2);
        let w1 = tri_qr_vals(i.l, e, x);
        let w2 = tri_qr_vals(i.l, i.l.opp(e), x);
        if w1[0] <= 0.0 {
            if w2[0] <= 0.0 {
                // in the edge's region
                if h1 < -1e-19 && h2 < -1e-19 {
                    self.pos = x;
                    self.set(i, e, ST_BACKSIDE);
                    return 3;
                }
                let c = cross3(d, xs);
                let id2 = 1.0 / dot3(d, d);
                let q = dot3(c, c) * id2;
                let is = isqrt5(q);
                self.md.len_numerator = (is * q - f64::from(self.md.sum_extra_radius)) as f32;
                let dc = cross3(d, c);
                let wv = math::rmul(&i.cache.r, &dc);
                let n = [-(is * id2) * wv[0], -(is * id2) * wv[1], -(is * id2) * wv[2]];
                self.store_n(n);
                self.dot_diff(bi, &i.cache);
                self.set(i, e, ST_EDGE);
                return 1;
            }
            return self.bf(bi, i.l.opp(e), i);
        }
        if w2[0] <= 0.0 || h2 <= 0.0 {
            return self.bf(bi, e, i);
        }
        self.bf(bi, i.l.opp(e), i)
    }

    /// BK (`0x10032860`): the ball's centre against the edge `e`.
    fn bk(&mut self, bi: &ObjCache, e: Edge, i: &Info) -> i32 {
        let x = world_to_obj(&i.cache, bi.vv);
        let ab = kvals(i.l, e, x);
        if ab[0] < 0.0 {
            return self.bp(bi, e, i);
        }
        if ab[1] < 0.0 {
            return self.bp(bi, Ledge::next(e), i);
        }
        self.bk_core(bi, e, i)
    }

    /// BP (`0x10032050`): the ball's centre against the point of `e`.
    fn bp(&mut self, bi: &ObjCache, e: Edge, i: &Info) -> i32 {
        if self.looped(key_none(3), key(i, 0, e)) {
            return 2;
        }
        let pw = obj_to_world(&i.cache, fsub_point(i.p(e)));
        let x = world_to_obj(&i.cache, bi.vv);
        let mut v = [bi.vv[0] - pw[0], bi.vv[1] - pw[1], bi.vv[2] - pw[2]];
        let len = normalize_len(&mut v);
        self.store_n(v);
        self.md.len_numerator = (len - f64::from(self.md.sum_extra_radius)) as f32;
        self.dot_diff(bi, &i.cache);
        let p = i.p(e);
        let xd = [x[0] - f64::from(p[0]), x[1] - f64::from(p[1]), x[2] - f64::from(p[2])];
        let c0 = f64::from(p[0]) * xd[0] + f64::from(p[1]) * xd[1] + f64::from(p[2]) * xd[2];
        let mut best: Option<Edge> = None;
        let mut bestv = 0.0;
        for g in ring(i.l, e) {
            let n = i.p(g);
            let f = (f64::from(n[0]) * xd[0] + f64::from(n[1]) * xd[1] + f64::from(n[2]) * xd[2]) - c0;
            if f > 0.0 {
                let wv = isqrt4(flen2(n, p)) * f;
                if wv > bestv {
                    bestv = wv;
                    best = Some(g);
                }
            }
        }
        if let Some(b) = best {
            let ab = kvals(i.l, b, x);
            if ab[1] > 0.0 {
                return if ab[0] >= 0.0 { self.bk(bi, b, i) } else { self.bp(bi, b, i) };
            }
        }
        self.set(i, e, ST_POINT);
        1
    }

    /// BB (`0x1001a370`): two balls.
    fn bb(&mut self) -> i32 {
        let ca = self.sides[0].cache;
        let cb = self.sides[1].cache;
        let v = [
            (ca.vv[0] - cb.vv[0]) as f32,
            (ca.vv[1] - cb.vv[1]) as f32,
            (ca.vv[2] - cb.vv[2]) as f32,
        ];
        let q = f64::from(v[0]) * f64::from(v[0]) + f64::from(v[1]) * f64::from(v[1]) + f64::from(v[2]) * f64::from(v[2]);
        let inv = if q.abs() <= f64::from(1e-19f32) { 1.0 } else { isqrt5(q) };
        self.md.len_numerator = (inv * q - f64::from(self.md.sum_extra_radius)) as f32;
        for k in 0..3 {
            self.md.normal[k] = (f64::from(v[k]) * inv) as f32;
        }
        self.dot_diff(&ca, &cb);
        1
    }

    /// One step of the solver (`ivp_minimize_step`, the table 0x10075ee0): dispatches on the two sorted statuses.
    pub fn step(&mut self) -> i32 {
        let mut sa = self.md.syn[self.md.sorted(0)].status;
        let sb = self.md.syn[self.md.sorted(1)].status;
        if sa == ST_BALL && sb == ST_BALL {
            return self.bb();
        }
        if sb == ST_BALL {
            // the upstream asserts here (0x1001a1f0) and flips the pair
            self.md.flags ^= MDF_SORT;
            sa = ST_BALL;
        }
        if sa == ST_BALL {
            // the ball is synapse 0 (0x1001a220)
            let bi = self.sides[0].cache;
            let Some(i) = self.info(1) else { return 2 };
            let Some(e) = self.edge_of(1) else { return 2 };
            return match self.md.syn[1].status {
                ST_POINT => self.bp(&bi, e, &i),
                ST_EDGE => self.bk(&bi, e, &i),
                ST_FACE => self.bf(&bi, e, &i),
                _ => 2,
            };
        }
        if (sa == ST_EDGE && sb == ST_FACE) || (sa == ST_FACE && sb == ST_EDGE) {
            // the upstream asserts on this pairing (0x1001a6a0)
            return 5;
        }
        if sa > sb {
            // (KP, FP) of 0x1001a350
            self.md.flags ^= MDF_SORT;
        }
        // 0x10019ff0: the sorted pair
        let ka = self.md.sorted(0);
        let kb = self.md.sorted(1);
        let (Some(ia), Some(ib)) = (self.info(ka), self.info(kb)) else { return 2 };
        let (Some(ea), Some(eb)) = (self.edge_of(ka), self.edge_of(kb)) else { return 2 };
        let code = self.md.syn[ka].status * 4 + self.md.syn[kb].status;
        match code {
            0 => self.pp(ea, eb, ia, ib),
            1 => self.pk(ea, eb, ia, ib),
            2 => self.pf(ea, eb, ia, ib),
            5 => self.kk(ea, eb, ia, ib),
            _ => self.ff(ea, eb, ia, ib),
        }
    }

    /// The pierce walk (`0x1001a5c0`): from the pierce triangle of the synapse's triangle to the one facing `pos`.
    fn pierce_walk(&self, i: &Info) -> Option<Edge> {
        let cur = self.edge_of(i.k)?;
        let n = i.l.tris.len();
        let mut visited = vec![false; n];
        let mut e = Edge {
            tri: i.l.tris[cur.tri].pierce,
            slot: 0,
        };
        loop {
            visited[e.tri] = true;
            let w = tri_qr_vals(i.l, e, self.pos);
            let mut ek = e;
            let mut next: Option<Edge> = None;
            for k in 0..3 {
                if next.is_some() {
                    break;
                }
                if w[k] <= 0.0 && !visited[i.l.opp(ek).tri] {
                    next = Some(i.l.opp(ek));
                }
                ek = Ledge::next(ek);
            }
            match next {
                Some(n) => e = n,
                None => break,
            }
        }
        Some(e)
    }

    /// Moves the backside synapse onto the face the walk reaches (`0x10019790`).
    pub fn fix_backside(&mut self) {
        let a = self.md.sorted(0);
        let k = if self.md.syn[a].status == ST_BACKSIDE { a } else { self.md.sorted(1) };
        let Some(i) = self.info(k) else { return };
        let Some(e) = self.pierce_walk(&i) else { return };
        self.md.syn[k].feat = Some(Feat {
            ledge: i.feat,
            edge: e,
        });
        self.md.syn[k].status = ST_FACE;
    }
}

/// The point (f32) of an object as f64 for the world transform.
fn fsub_point(p: P3) -> V3 {
    [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]
}

/// The world point of an edge's start in `i`'s object.
fn fobj_world(i: &Info, e: Edge) -> V3 {
    obj_to_world(&i.cache, fsub_point(i.p(e)))
}


#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::convex;
    use crate::ledge::Surface;
    use crate::mindist::{Mindist, Synapse};

    /// A unit cube in its object space (the corner at the origin), as a one-ledge surface. Its place in the world
    /// is the object's cache (`cache_at`), not the points.
    fn unit_cube() -> Surface {
        let mut p = Vec::new();
        for &x in &[0.0, 1.0] {
            for &y in &[0.0, 1.0] {
                for &z in &[0.0, 1.0] {
                    p.push([x, y, z]);
                }
            }
        }
        let l = convex::hull(&p).expect("a cube has a hull");
        Surface::compile(vec![l]).expect("a surface")
    }

    fn cache_at(o: V3) -> ObjCache {
        ObjCache {
            r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            vv: o,
            core_pos: o,
        }
    }

    /// Runs the solver the way the mindist recalc does: steps, with the backside fix, until it converges.
    fn solve(a: &Surface, oa: V3, b: &Surface, ob: V3) -> (i32, Mindist) {
        let mut md = Mindist::default();
        md.syn[0] = Synapse {
            obj: 0,
            feat: Some(Feat {
                ledge: 0,
                edge: Edge { tri: 0, slot: 0 },
            }),
            status: ST_POINT,
            ..Synapse::default()
        };
        md.syn[1] = Synapse {
            obj: 1,
            feat: Some(Feat {
                ledge: 0,
                edge: Edge { tri: 0, slot: 0 },
            }),
            status: ST_POINT,
            ..Synapse::default()
        };
        let sides = [
            Side {
                cache: cache_at(oa),
                surface: Some(a),
            },
            Side {
                cache: cache_at(ob),
                surface: Some(b),
            },
        ];
        let mut rc = 2;
        for _ in 0..8 {
            let mut s = Solver::new(&mut md, sides, 20);
            rc = s.step();
            if rc == 3 {
                s.fix_backside();
            }
            if rc == 1 {
                break;
            }
            if rc == 2 {
                break;
            }
            drop(s);
        }
        (rc, md)
    }

    #[test]
    fn two_cubes_side_by_side_are_one_apart() {
        let a = unit_cube();
        let b = unit_cube();
        let (rc, md) = solve(&a, [0.0; 3], &b, [2.0, 0.0, 0.0]);
        assert_eq!(rc, 1, "the solve converges");
        assert!((md.len_numerator - 1.0).abs() < 1e-4, "gap {}", md.len_numerator);
        let n = md.normal;
        assert!((n[0].abs() - 1.0).abs() < 1e-4 && n[1].abs() < 1e-4 && n[2].abs() < 1e-4, "normal {n:?}");
    }

    #[test]
    fn two_cubes_corner_to_corner_are_sqrt2_apart() {
        let a = unit_cube();
        let b = unit_cube();
        let (rc, md) = solve(&a, [0.0; 3], &b, [2.0, 2.0, 0.0]);
        assert_eq!(rc, 1, "the solve converges");
        let want = 2f32.sqrt();
        assert!((md.len_numerator - want).abs() < 1e-3 * want, "gap {} want {want}", md.len_numerator);
    }

    #[test]
    fn a_cube_above_another_is_its_height_away() {
        let a = unit_cube();
        let b = unit_cube();
        let (rc, md) = solve(&a, [0.0; 3], &b, [0.25, 0.25, 1.5]);
        assert_eq!(rc, 1, "the solve converges");
        assert!((md.len_numerator - 0.5).abs() < 1e-3, "gap {}", md.len_numerator);
        assert!((md.normal[2].abs() - 1.0).abs() < 1e-3, "normal {:?}", md.normal);
    }
}

