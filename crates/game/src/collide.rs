// Static collision geometry: the triangles of every object the level marks as physical, in world space, in a
// uniform grid so the ball only tests the triangles around it. The geometry is the original's, polygon for
// polygon (decisions.md, "Visual smoothing never touches physics").

use glam::Vec3;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    pub normal: Vec3,
}

#[derive(Clone)]
pub struct Collider {
    pub triangles: Vec<Triangle>,
    cells: HashMap<(i32, i32, i32), Vec<u32>>,
    cell: f32,
    pub min_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Contact {
    /// From the surface towards the ball's centre, unit length.
    pub normal: Vec3,
    pub depth: f32,
}

impl Collider {
    pub fn new(triangles: Vec<Triangle>) -> Collider {
        let cell = 8.0;
        let mut cells: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        let mut min_y = f32::INFINITY;
        for (i, t) in triangles.iter().enumerate() {
            let lo = t.a.min(t.b).min(t.c);
            let hi = t.a.max(t.b).max(t.c);
            min_y = min_y.min(lo.y);
            let (l, h) = (key(lo, cell), key(hi, cell));
            for x in l.0..=h.0 {
                for y in l.1..=h.1 {
                    for z in l.2..=h.2 {
                        cells.entry((x, y, z)).or_default().push(i as u32);
                    }
                }
            }
        }
        Collider {
            triangles,
            cells,
            cell,
            min_y: if min_y.is_finite() { min_y } else { 0.0 },
        }
    }

    /// Indices of the triangles whose cells touch a box around `p` of half-size `radius`.
    pub fn nearby(&self, p: Vec3, radius: f32) -> Vec<u32> {
        let r = Vec3::splat(radius);
        let (l, h) = (key(p - r, self.cell), key(p + r, self.cell));
        let mut seen: Vec<u32> = Vec::new();
        for x in l.0..=h.0 {
            for y in l.1..=h.1 {
                for z in l.2..=h.2 {
                    if let Some(list) = self.cells.get(&(x, y, z)) {
                        seen.extend_from_slice(list);
                    }
                }
            }
        }
        seen.sort_unstable();
        seen.dedup();
        seen
    }

    /// Every triangle the sphere touches, deepest first.
    pub fn contacts(&self, centre: Vec3, radius: f32) -> Vec<Contact> {
        let r = Vec3::splat(radius);
        let (l, h) = (key(centre - r, self.cell), key(centre + r, self.cell));
        let mut seen: Vec<u32> = Vec::new();
        for x in l.0..=h.0 {
            for y in l.1..=h.1 {
                for z in l.2..=h.2 {
                    if let Some(list) = self.cells.get(&(x, y, z)) {
                        seen.extend_from_slice(list);
                    }
                }
            }
        }
        seen.sort_unstable();
        seen.dedup();
        let mut out = Vec::new();
        for i in seen {
            let Some(t) = self.triangles.get(i as usize) else {
                continue;
            };
            let p = closest_point(centre, t.a, t.b, t.c);
            let d = centre - p;
            let dist = d.length();
            if dist < radius {
                let normal = if dist > 1e-6 { d / dist } else { t.normal };
                out.push(Contact {
                    normal,
                    depth: radius - dist,
                });
            }
        }
        out.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        out
    }
}

fn key(p: Vec3, cell: f32) -> (i32, i32, i32) {
    (
        (p.x / cell).floor() as i32,
        (p.y / cell).floor() as i32,
        (p.z / cell).floor() as i32,
    )
}

/// Closest point on a triangle to a point (Ericson, Real-Time Collision Detection, 5.1.5).
fn closest_point(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}
