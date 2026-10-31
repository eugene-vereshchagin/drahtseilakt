// Port of OpenBallance src/phys/phys_convex.c (GPL-3.0; see the LICENSE of the repository): convex hull ledges (docs/
// ivp_collision.md 1.6.3). The upstream runs qhull and then fixes its sliver facets; this port keeps the upstream's own
// replacement builder: an incremental hull in f64, coplanar facets merged into convex polygons (collinear boundary
// vertices dropped), each polygon fanned into outward-wound triangles, twins linked, and each triangle's pierce partner
// the most anti-parallel one. Only the order of triangles and points (tie breaking) can differ from qhull's.

use crate::ledge::{self, Ledge};

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(u: V3, v: V3) -> V3 {
    [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
}

fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}

fn dist2(a: V3, b: V3) -> f64 {
    let d = sub(a, b);
    dot(d, d)
}

/// A facet of the hull: its vertices (indices into the input), its outward unit normal and the plane offset.
#[derive(Clone, Copy, Debug)]
struct Face {
    v: [usize; 3],
    n: V3,
    d: f64,
    alive: bool,
}

fn face_plane(p: &[V3], f: &mut Face) {
    let a = p[f.v[0]];
    let n = cross(sub(p[f.v[1]], a), sub(p[f.v[2]], a));
    let l = len(n);
    let n = if l > 0.0 { [n[0] / l, n[1] / l, n[2] / l] } else { n };
    f.n = n;
    f.d = dot(n, a);
}

fn face_dist(f: &Face, q: V3) -> f64 {
    f.n[0] * q[0] + f.n[1] * q[1] + f.n[2] * q[2] - f.d
}

fn new_face(p: &[V3], a: usize, b: usize, c: usize) -> Face {
    let mut f = Face {
        v: [a, b, c],
        n: [0.0; 3],
        d: 0.0,
        alive: true,
    };
    face_plane(p, &mut f);
    f
}

/// Every directed edge of the alive faces has exactly one reverse.
fn manifold(faces: &[Face]) -> bool {
    for f in faces.iter().filter(|f| f.alive) {
        for k in 0..3 {
            let (a, b) = (f.v[k], f.v[(k + 1) % 3]);
            let twins: usize = faces
                .iter()
                .filter(|g| g.alive)
                .map(|g| (0..3).filter(|&m| g.v[m] == b && g.v[(m + 1) % 3] == a).count())
                .sum();
            if twins != 1 {
                return false;
            }
        }
    }
    true
}

/// The incremental hull (`hull3d`): faces with outward normals, or None for flat or degenerate input.
fn hull3d(p: &[V3], eps: f64) -> Option<Vec<Face>> {
    let n = p.len();
    let mut i0 = 0;
    for i in 1..n {
        if p[i][0] < p[i0][0] {
            i0 = i;
        }
    }
    let mut i1 = None;
    let mut best = 0.0;
    for i in 0..n {
        if dist2(p[i], p[i0]) > best {
            best = dist2(p[i], p[i0]);
            i1 = Some(i);
        }
    }
    let i1 = i1?;
    if best <= eps * eps {
        return None;
    }
    let e = sub(p[i1], p[i0]);
    let el = len(e);
    let mut i2 = None;
    best = 0.0;
    for i in 0..n {
        let l = len(cross(sub(p[i], p[i0]), e)) / el;
        if l > best {
            best = l;
            i2 = Some(i);
        }
    }
    let i2 = i2?;
    if best <= eps {
        return None;
    }
    let mut base = new_face(p, i0, i1, i2);
    base.alive = true;
    let mut i3 = None;
    best = 0.0;
    for i in 0..n {
        if face_dist(&base, p[i]).abs() > best {
            best = face_dist(&base, p[i]).abs();
            i3 = Some(i);
        }
    }
    let i3 = i3?;
    if best <= eps {
        return None;
    }

    let mut faces: Vec<Face> = Vec::new();
    if face_dist(&base, p[i3]) > 0.0 {
        faces.push(new_face(p, i0, i2, i1));
        faces.push(new_face(p, i0, i1, i3));
        faces.push(new_face(p, i1, i2, i3));
        faces.push(new_face(p, i2, i0, i3));
    } else {
        faces.push(new_face(p, i0, i1, i2));
        faces.push(new_face(p, i0, i3, i1));
        faces.push(new_face(p, i1, i3, i2));
        faces.push(new_face(p, i2, i3, i0));
    }
    for i in 0..n {
        if i == i0 || i == i1 || i == i2 || i == i3 {
            continue;
        }
        let visible = |f: &Face| f.alive && face_dist(f, p[i]) > eps;
        if !faces.iter().any(visible) {
            continue;
        }
        // the horizon: edges of visible faces whose reverse is not on a visible face
        let mut horizon: Vec<(usize, usize)> = Vec::new();
        for (fi, f) in faces.iter().enumerate() {
            if !visible(f) {
                continue;
            }
            for k in 0..3 {
                let (a, b) = (f.v[k], f.v[(k + 1) % 3]);
                let inner = faces.iter().enumerate().any(|(gi, g)| {
                    gi != fi && visible(g) && (0..3).any(|m| g.v[m] == b && g.v[(m + 1) % 3] == a)
                });
                if !inner {
                    horizon.push((a, b));
                }
            }
        }
        for f in faces.iter_mut() {
            if visible(f) {
                f.alive = false;
            }
        }
        for (a, b) in horizon {
            faces.push(new_face(p, a, b, i));
        }
        // compact the dead faces away now and then
        if faces.len() > 4 * n + 64 {
            faces.retain(|f| f.alive);
        }
    }
    faces.retain(|f| f.alive);
    if manifold(&faces) {
        Some(faces)
    } else {
        None
    }
}

/// A convex polygon of merged coplanar facets: its boundary loop (outward winding) and its normal.
#[derive(Clone, Debug)]
struct Poly {
    v: Vec<usize>,
    nrm: V3,
}

/// The facets grouped into polygons (edge-adjacent faces within the tolerance of the group's first face's plane), with
/// their boundary loops. None when a loop is not simple (`merge_polys`).
fn merge_polys(p: &[V3], faces: &[Face], tol: f64) -> Option<Vec<Poly>> {
    let nf = faces.len();
    let mut grp: Vec<Option<usize>> = vec![None; nf];
    let mut ng = 0;
    for s in 0..nf {
        if grp[s].is_some() {
            continue;
        }
        let seed = faces[s];
        let mut stack = vec![s];
        grp[s] = Some(ng);
        while let Some(fi) = stack.pop() {
            let f = faces[fi];
            for k in 0..3 {
                let (a, b) = (f.v[k], f.v[(k + 1) % 3]);
                for g in 0..nf {
                    if grp[g].is_some() {
                        continue;
                    }
                    let gf = faces[g];
                    let adj = (0..3).any(|m| gf.v[m] == b && gf.v[(m + 1) % 3] == a);
                    if !adj {
                        continue;
                    }
                    let c = dot(gf.n, seed.n);
                    let cop = c > 1.0 - 1e-9 && (0..3).all(|m| face_dist(&seed, p[gf.v[m]]).abs() <= tol);
                    if !cop {
                        continue;
                    }
                    grp[g] = Some(ng);
                    stack.push(g);
                }
            }
        }
        ng += 1;
    }

    let mut polys = Vec::with_capacity(ng);
    for g in 0..ng {
        // boundary edges of the group, in the faces' winding
        let mut be: Vec<(usize, usize)> = Vec::new();
        let mut seed = None;
        for f in 0..nf {
            if grp[f] != Some(g) {
                continue;
            }
            if seed.is_none() {
                seed = Some(f);
            }
            for k in 0..3 {
                let (a, b) = (faces[f].v[k], faces[f].v[(k + 1) % 3]);
                let inner = (0..nf).any(|h| {
                    h != f
                        && grp[h] == Some(g)
                        && (0..3).any(|m| faces[h].v[m] == b && faces[h].v[(m + 1) % 3] == a)
                });
                if !inner {
                    be.push((a, b));
                }
            }
        }
        let seed = seed?;
        let nbe = be.len();
        let start = be.first()?.0;
        let mut cur = start;
        let mut v = Vec::with_capacity(nbe);
        loop {
            let nexts: Vec<usize> = be.iter().filter(|e| e.0 == cur).map(|e| e.1).collect();
            if nexts.len() != 1 || v.len() >= nbe {
                return None;
            }
            v.push(cur);
            cur = nexts[0];
            if cur == start {
                break;
            }
        }
        if v.len() != nbe {
            return None;
        }
        polys.push(Poly {
            v,
            nrm: faces[seed].n,
        });
    }
    Some(polys)
}

/// Whether vertex `i` of the loop is a corner, not collinear with its loop neighbours (`loop_corner`).
fn loop_corner(p: &[V3], pl: &Poly, i: usize, tol: f64) -> bool {
    let n = pl.v.len();
    let a = p[pl.v[(i + n - 1) % n]];
    let b = p[pl.v[i]];
    let c = p[pl.v[(i + 1) % n]];
    let w = sub(c, a);
    let lw = len(w);
    // the distance of b from the line a-c
    lw > 0.0 && len(cross(sub(b, a), w)) / lw > tol
}

/// A sliver polygon: its area is small against its perimeter (`sliver`, 0x1003a1e0 / 0x1003a2d0).
fn sliver(p: &[V3], pl: &Poly) -> bool {
    let mut a_sum = 0.0;
    let mut l2 = 0.0;
    let v0 = p[pl.v[0]];
    let n = pl.v.len();
    for k in 0..n {
        let a = p[pl.v[k]];
        let b = p[pl.v[(k + 1) % n]];
        let d = sub(b, a);
        let c = cross(d, sub(a, v0));
        a_sum += dot(pl.nrm, c);
        l2 += dot(d, d);
    }
    if a_sum < 0.0 {
        a_sum = -a_sum;
    }
    a_sum < l2.sqrt() * 0.005
}

/// The triangle partners of the fanned triangles: for each triangle in list order without a partner, the triangle
/// with the most negative dot product below -1e-6 (the first on ties); both are set, overwriting the old partner.
fn assign_pierce(nrm: &[V3]) -> Vec<usize> {
    let n = nrm.len();
    let mut pierce: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        if pierce[i].is_some() {
            continue;
        }
        let mut best = None;
        let mut bv = -1e-6;
        for j in 0..n {
            let d = dot(nrm[i], nrm[j]);
            if d < bv {
                bv = d;
                best = Some(j);
            }
        }
        // no valid partner: the triangle is its own
        let best = best.unwrap_or(i);
        pierce[i] = Some(best);
        pierce[best] = Some(i);
    }
    // every index was set in the loop above
    pierce.into_iter().map(|p| p.unwrap_or(0)).collect()
}

/// The hull ledge of a point set, duplicates removed by the caller (`ivp_ledge_convex_hull`, 0x1003a750).
pub fn hull(p: &[V3]) -> Option<Ledge> {
    let n = p.len();
    let mut lo = [1e300f64; 3];
    let mut hi = [-1e300f64; 3];
    for pt in p {
        for k in 0..3 {
            if pt[k] < lo[k] {
                lo[k] = pt[k];
            }
            if pt[k] > hi[k] {
                hi[k] = pt[k];
            }
        }
    }
    let mut ext = 0.0f64;
    let mut mag = 0.0f64;
    for k in 0..3 {
        if hi[k] - lo[k] > ext {
            ext = hi[k] - lo[k];
        }
        if lo[k].abs() > mag {
            mag = lo[k].abs();
        }
        if hi[k].abs() > mag {
            mag = hi[k].abs();
        }
    }
    // tolerances: the f32 resolution of the input, relative to its size
    let eps = (ext + mag) * 1e-9;
    let tol = (ext + mag) * 2e-7;
    let faces = hull3d(p, eps)?;

    let mut polys = match merge_polys(p, &faces, tol) {
        Some(pl) => pl,
        // not simple: keep the hull's own triangles, one polygon each
        None => faces
            .iter()
            .map(|f| Poly {
                v: f.v.to_vec(),
                nrm: f.n,
            })
            .collect(),
    };

    // a sliver polygon drops one vertex (neither of the two farthest apart) and the hull is rebuilt without it
    let mut corner = vec![false; n];
    for pl in &polys {
        for i in 0..pl.v.len() {
            if loop_corner(p, pl, i, tol) {
                corner[pl.v[i]] = true;
            }
        }
    }
    for pl in polys.iter_mut() {
        pl.v.retain(|&v| corner[v]);
    }
    for pl in &polys {
        if pl.v.len() < 3 || !sliver(p, pl) {
            continue;
        }
        let mut a = 0;
        let mut best = -1.0;
        for i in 0..pl.v.len() {
            let d = dist2(p[pl.v[i]], p[pl.v[0]]);
            if d > best {
                best = d;
                a = i;
            }
        }
        let mut b = 0;
        best = -1.0;
        for i in 0..pl.v.len() {
            let d = dist2(p[pl.v[i]], p[pl.v[a]]);
            if d > best {
                best = d;
                b = i;
            }
        }
        let Some(drop) = (0..pl.v.len()).find(|&i| i != a && i != b).map(|i| pl.v[i]) else {
            continue;
        };
        let q: Vec<V3> = p
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != drop)
            .map(|(_, &pt)| pt)
            .collect();
        return match q.len() {
            3 => ledge::triangle(q[0], q[1], q[2]),
            m if m < 3 => None,
            _ => hull(&q),
        };
    }

    // fan every polygon (outward winding follows the loop), the list in reverse creation order
    let ntri: usize = polys.iter().map(|pl| if pl.v.len() >= 3 { pl.v.len() - 2 } else { 0 }).sum();
    let mut tri: Vec<[usize; 3]> = vec![[0; 3]; ntri];
    let mut t = ntri;
    for pl in &polys {
        for i in 1..pl.v.len().saturating_sub(1) {
            t -= 1;
            tri[t] = [pl.v[0], pl.v[i], pl.v[i + 1]];
        }
    }
    if !(4..=4095).contains(&ntri) {
        return None;
    }
    let nrm: Vec<V3> = tri
        .iter()
        .map(|t| {
            let mut f = new_face(p, t[0], t[1], t[2]);
            f.alive = true;
            f.n
        })
        .collect();
    let pierce = assign_pierce(&nrm);
    let pts: Vec<[f32; 3]> = p
        .iter()
        .map(|q| [q[0] as f32, q[1] as f32, q[2] as f32])
        .collect();
    ledge::generate(&pts, &tri, &pierce)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn cube(lo: f64, hi: f64) -> Vec<V3> {
        let mut v = Vec::new();
        for &x in &[lo, hi] {
            for &y in &[lo, hi] {
                for &z in &[lo, hi] {
                    v.push([x, y, z]);
                }
            }
        }
        v
    }

    /// Every edge of every triangle has its twin in the ledge, with the endpoints reversed.
    fn twins_are_closed(l: &Ledge) -> bool {
        l.tris.iter().enumerate().all(|(ti, t)| {
            (0..3).all(|k| {
                let o = t.opposite[k];
                let a = t.start[k];
                let b = t.start[(k + 1) % 3];
                let ot = &l.tris[o.tri];
                let oa = ot.start[o.slot];
                let ob = ot.start[(o.slot + 1) % 3];
                oa == b && ob == a && o.tri != ti
            })
        })
    }

    #[test]
    fn cube_hull_is_a_closed_twelve_triangle_ledge() {
        let l = hull(&cube(-1.0, 1.0)).expect("a cube has a hull");
        assert_eq!(l.tris.len(), 12);
        assert!(twins_are_closed(&l));
    }

    #[test]
    fn tetrahedron_has_four_triangles() {
        let p = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let l = hull(&p).expect("a tetrahedron has a hull");
        assert_eq!(l.tris.len(), 4);
        assert!(twins_are_closed(&l));
    }

    #[test]
    fn flat_input_has_no_hull() {
        let p = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]];
        assert!(hull(&p).is_none());
    }
}
