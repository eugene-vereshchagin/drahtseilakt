// The level's physical geometry: what the level's physical groups (floors, rails, modules) are made of, one mesh per
// object as the original physicalizes them. The watch measures the ball's distance to their triangles.

use glam::{Quat, Vec3};

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
    pub rot: Quat,
}

impl StaticMesh {
    /// The triangles in world space.
    pub fn world_triangles(&self) -> impl Iterator<Item = Triangle> + '_ {
        self.triangles.iter().map(|t| Triangle {
            a: self.pos + self.rot * t.a,
            b: self.pos + self.rot * t.b,
            c: self.pos + self.rot * t.c,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closest_point_is_inside_on_an_edge_or_a_corner() {
        let t = [Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 4.0)];
        assert_eq!(closest_point(&t, Vec3::new(1.0, 3.0, 1.0)), Vec3::new(1.0, 0.0, 1.0));
        assert_eq!(closest_point(&t, Vec3::new(2.0, -1.0, -5.0)), Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(closest_point(&t, Vec3::new(-3.0, 2.0, -3.0)), Vec3::ZERO);
        assert_eq!(closest_point(&t, Vec3::new(4.0, 0.0, 4.0)), Vec3::new(2.0, 0.0, 2.0));
    }
}
