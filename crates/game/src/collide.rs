// The level's physical geometry: what the level's physical groups (floors, rails, modules) are made of, one mesh per
// object as the original physicalizes them. The watch measures the ball's distance to their triangles.

use crate::physics::Rot;
use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

/// One physical object of the level as the original physicalizes it: its mesh's triangles in the object's own
/// frame, scaled by the object's scale, and the object's pose without the scale. The physics makes a fixed body of it.
#[derive(Debug, Clone)]
pub struct StaticMesh {
    pub triangles: Vec<Triangle>,
    pub pos: Vec3,
    pub rot: Rot,
    /// The collision group: objects of the same non-empty group do not collide (docs/physics.md 3.4).
    pub group: &'static str,
    /// Its sound material (crate::physics::SoundIds).
    pub sound: crate::physics::SoundIds,
}

impl StaticMesh {
    /// The triangles in world space.
    pub fn world_triangles(&self) -> impl Iterator<Item = Triangle> + '_ {
        let rot = self.rot.quat();
        self.triangles.iter().map(move |t| Triangle {
            a: self.pos + rot * t.a,
            b: self.pos + rot * t.b,
            c: self.pos + rot * t.c,
        })
    }
}

/// The point of triangle `t` closest to `p` (Ericson, Real-Time Collision Detection, 5.1.5).
pub fn closest_point(t: &[Vec3; 3], p: Vec3) -> Vec3 {
    let [a, b, c] = *t;
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
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

/// A loose body's shape for `start_depth`: a sphere (its radius) or a convex hull (its points), in world space.
pub enum Shape<'a> {
    Sphere(Vec3, f32),
    Hull(&'a [Vec3]),
}

impl Shape<'_> {
    /// Its middle and how far it reaches from it.
    pub fn bounds(&self) -> (Vec3, f32) {
        match self {
            Shape::Sphere(c, r) => (*c, *r),
            Shape::Hull(points) if !points.is_empty() => {
                let middle = points.iter().copied().sum::<Vec3>() / points.len() as f32;
                (middle, points.iter().map(|p| p.distance(middle)).fold(0.0, f32::max))
            }
            Shape::Hull(_) => (Vec3::ZERO, 0.0),
        }
    }
}

/// How deep a body reaches into the level's static triangles: the largest overlap with any one triangle, measured
/// along the axis that separates them best (0 when it is clear of all). A sphere's is exact; a hull's is tested on
/// the axes that separate a convex body from a triangle (the triangle's normal, the direction from the hull's
/// middle to the triangle's nearest point, the world's and the triangle's edges crossed with the world's axes), so it
/// is exact for a box standing square and at most a little too deep for a round hull touching a triangle's edge.
/// Used to find a loose object made inside a floor or a wall (it would be shoved out by the physics' penetration
/// recovery, violently against the player's ball).
pub fn start_depth(shape: &Shape, triangles: &[Triangle]) -> f32 {
    let (middle, reach) = shape.bounds();
    if reach <= 0.0 {
        return 0.0;
    }
    let mut deepest = 0.0f32;
    for t in triangles {
        let tri = [t.a, t.b, t.c];
        let near = closest_point(&tri, middle);
        if near.distance(middle) >= reach {
            continue;
        }
        let depth = match shape {
            Shape::Sphere(c, r) => {
                // behind the triangle's face, over it: the centre is inside the solid
                let n = (t.b - t.a).cross(t.c - t.a).normalize_or_zero();
                let d = c.distance(near);
                let behind = (*c - t.a).dot(n) < 0.0 && d < 1e-4 + (*c - t.a).dot(n).abs();
                if behind { r + d } else { r - d }
            }
            Shape::Hull(points) => hull_triangle_overlap(points, &tri, middle - near),
        };
        deepest = deepest.max(depth);
    }
    deepest
}

/// The least overlap of a convex hull's points and a triangle over the axes that separate them (0 or less: apart).
fn hull_triangle_overlap(points: &[Vec3], tri: &[Vec3; 3], toward: Vec3) -> f32 {
    let normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
    let mut axes = vec![normal, toward];
    for e in [tri[1] - tri[0], tri[2] - tri[1], tri[0] - tri[2]] {
        for a in [Vec3::X, Vec3::Y, Vec3::Z] {
            axes.push(e.cross(a));
        }
    }
    axes.extend([Vec3::X, Vec3::Y, Vec3::Z]);
    let mut least = f32::INFINITY;
    for axis in axes {
        let Some(axis) = axis.try_normalize() else { continue };
        let (lo, hi) = points.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
            let d = p.dot(axis);
            (lo.min(d), hi.max(d))
        });
        let (tlo, thi) = tri.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
            let d = p.dot(axis);
            (lo.min(d), hi.max(d))
        });
        least = least.min((hi - tlo).min(thi - lo));
        if least <= 0.0 {
            return 0.0;
        }
    }
    least
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closest_point_is_inside_on_an_edge_or_a_corner() {
        let t = [
            Vec3::ZERO,
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 4.0),
        ];
        assert_eq!(
            closest_point(&t, Vec3::new(1.0, 3.0, 1.0)),
            Vec3::new(1.0, 0.0, 1.0)
        );
        assert_eq!(
            closest_point(&t, Vec3::new(2.0, -1.0, -5.0)),
            Vec3::new(2.0, 0.0, 0.0)
        );
        assert_eq!(closest_point(&t, Vec3::new(-3.0, 2.0, -3.0)), Vec3::ZERO);
        assert_eq!(
            closest_point(&t, Vec3::new(4.0, 0.0, 4.0)),
            Vec3::new(2.0, 0.0, 2.0)
        );
    }

    #[test]
    fn a_body_s_depth_in_the_floor() {
        let floor = [
            Triangle { a: Vec3::new(-9.0, 0.0, -9.0), b: Vec3::new(-9.0, 0.0, 9.0), c: Vec3::new(9.0, 0.0, 9.0) },
            Triangle { a: Vec3::new(-9.0, 0.0, -9.0), b: Vec3::new(9.0, 0.0, 9.0), c: Vec3::new(9.0, 0.0, -9.0) },
        ];
        let sphere = |y: f32| start_depth(&Shape::Sphere(Vec3::new(1.0, y, 1.0), 2.0), &floor);
        assert_eq!(sphere(2.0), 0.0, "resting on it");
        assert_eq!(sphere(2.5), 0.0, "over it");
        assert!((sphere(0.5) - 1.5).abs() < 1e-5);
        // the centre under the face: in the solid
        assert!((sphere(-0.5) - 2.5).abs() < 1e-5);
        let cube = |y: f32| {
            let corners: Vec<Vec3> = (0..8)
                .map(|k| Vec3::new([-2.5, 2.5][k & 1], y + [-2.5, 2.5][(k >> 1) & 1], [-2.5, 2.5][k >> 2]))
                .collect();
            start_depth(&Shape::Hull(&corners), &floor)
        };
        assert_eq!(cube(2.5), 0.0, "standing on it");
        assert!((cube(2.0) - 0.5).abs() < 1e-5);
    }
}
