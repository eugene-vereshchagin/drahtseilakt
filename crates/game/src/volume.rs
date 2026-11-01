// Oriented boxes and their overlap test, as the original's "Collision Box" checks use them: the ball has left the
// level when the box of its mesh overlaps one of the level's depth-test cubes (docs/gameplay_bbs.md, the box tests
// and their use in BallManager).

use glam::{Mat4, Vec3};
#[cfg(test)]
use glam::Mat3;

/// An oriented box: centre, unit axes and half extents along them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    pub center: Vec3,
    pub axes: [Vec3; 3],
    pub half: Vec3,
}

impl Obb {
    /// The box of local corners `min`..`max` under a world matrix, as `VxOBB` builds it: the centre transformed, the
    /// axes the matrix's normalised axes, the scale folded into the extents.
    pub fn from_box(min: Vec3, max: Vec3, m: &Mat4) -> Obb {
        let cols = [m.x_axis.truncate(), m.y_axis.truncate(), m.z_axis.truncate()];
        let len = cols.map(|c| c.length());
        let axes = [0, 1, 2].map(|i| if len[i] > 0.0 { cols[i] / len[i] } else { Vec3::ZERO });
        let ext = (max - min) * 0.5;
        Obb {
            center: m.transform_point3((max + min) * 0.5),
            axes,
            half: Vec3::new(ext.x * len[0], ext.y * len[1], ext.z * len[2]),
        }
    }

    /// A cube of half size `half` centred at `center`, turned by `rot` (tests).
    #[cfg(test)]
    pub fn cube(center: Vec3, half: f32, rot: Mat3) -> Obb {
        Obb {
            center,
            axes: [rot.x_axis, rot.y_axis, rot.z_axis],
            half: Vec3::splat(half),
        }
    }

    /// Whether two boxes overlap: the 15-axis separating-axis test with no epsilon, touching boxes overlapping
    /// (`OBBOBB`, in the original's order: A's axes, B's axes, then the cross axes).
    pub fn overlaps(&self, b: &Obb) -> bool {
        let a = self;
        let t = b.center - a.center;
        let (ae, be) = (a.half.to_array(), b.half.to_array());
        let mut r = [[0.0f32; 3]; 3];
        let mut ar = [[0.0f32; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = a.axes[i].dot(b.axes[j]);
                ar[i][j] = r[i][j].abs();
            }
        }
        let ta = [t.dot(a.axes[0]), t.dot(a.axes[1]), t.dot(a.axes[2])];
        for i in 0..3 {
            let rb = be[0] * ar[i][0] + be[1] * ar[i][1] + be[2] * ar[i][2];
            if ae[i] + rb < ta[i].abs() {
                return false;
            }
        }
        for j in 0..3 {
            let tb = t.dot(b.axes[j]);
            let ra = ae[0] * ar[0][j] + ae[1] * ar[1][j] + ae[2] * ar[2][j];
            if be[j] + ra < tb.abs() {
                return false;
            }
        }
        for i in 0..3 {
            let (i1, i2) = ((i + 1) % 3, (i + 2) % 3);
            for j in 0..3 {
                let (j1, j2) = ((j + 1) % 3, (j + 2) % 3);
                let proj = (ta[i2] * r[i1][j] - ta[i1] * r[i2][j]).abs();
                let rad = ae[i1] * ar[i2][j] + ae[i2] * ar[i1][j] + be[j1] * ar[i][j2] + be[j2] * ar[i][j1];
                if rad < proj {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    fn unit_at(p: Vec3) -> Obb {
        Obb::cube(p, 1.0, Mat3::IDENTITY)
    }

    #[test]
    fn boxes_apart_do_not_overlap_and_touching_ones_do() {
        let a = unit_at(Vec3::ZERO);
        assert!(!a.overlaps(&unit_at(Vec3::new(2.01, 0.0, 0.0))));
        assert!(a.overlaps(&unit_at(Vec3::new(2.0, 0.0, 0.0))), "touching counts");
        assert!(a.overlaps(&unit_at(Vec3::new(1.0, 1.0, 1.0))));
    }

    #[test]
    fn a_turned_box_reaches_further_along_its_diagonal() {
        // Turned 45 degrees about y, a unit cube reaches sqrt(2) along x.
        let turned = Obb::cube(Vec3::ZERO, 1.0, Mat3::from_quat(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)));
        assert!(turned.overlaps(&unit_at(Vec3::new(2.4, 0.0, 0.0))));
        assert!(!turned.overlaps(&unit_at(Vec3::new(2.5, 0.0, 0.0))));
    }

    #[test]
    fn a_scaled_matrix_scales_the_extents() {
        // A box of local corners -1..1 scaled by 10 in x and moved to x = 100 spans 90..110.
        let m = Mat4::from_scale_rotation_translation(Vec3::new(10.0, 1.0, 1.0), Quat::IDENTITY, Vec3::new(100.0, 0.0, 0.0));
        let b = Obb::from_box(Vec3::splat(-1.0), Vec3::splat(1.0), &m);
        assert_eq!(b.half, Vec3::new(10.0, 1.0, 1.0));
        assert!(b.overlaps(&unit_at(Vec3::new(89.0, 0.0, 0.0))));
        assert!(!b.overlaps(&unit_at(Vec3::new(88.9, 0.0, 0.0))));
    }
}
