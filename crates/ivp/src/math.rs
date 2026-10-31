// Port of OpenBallance src/phys/phys_math.c (GPL-3.0; copyright of the upstream author and contributors, see the
// LICENSE of the repository). Vector, matrix and quaternion helpers of the IVP core, as the original game's
// physics_RT.dll computes them (docs/ivp_core.md sections 3 and 12). Function addresses in the original are kept
// in the comments.

/// A quaternion, as the upstream `IvpQuat`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

/// A rotation and a translation, as the upstream `IvpMatrix`: `r` columns are axes, `vv` is the offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    pub r: [[f64; 3]; 3],
    pub vv: [f64; 3],
}

impl Matrix {
    pub fn identity() -> Matrix {
        let mut r = [[0.0; 3]; 3];
        r[0][0] = 1.0;
        r[1][1] = 1.0;
        r[2][2] = 1.0;
        Matrix { r, vv: [0.0; 3] }
    }
}

/// Rotation to quaternion, as in `ivp_quat_to_matrix` (0x100190c0).
pub fn quat_to_matrix(q: &Quat) -> [[f64; 3]; 3] {
    let (x2, y2, z2) = (q.x + q.x, q.y + q.y, q.z + q.z);
    [
        [
            1.0 - (z2 * q.z + y2 * q.y),
            y2 * q.x - z2 * q.w,
            y2 * q.w + z2 * q.x,
        ],
        [
            z2 * q.w + y2 * q.x,
            1.0 - (z2 * q.z + x2 * q.x),
            z2 * q.y - x2 * q.w,
        ],
        [
            z2 * q.x - y2 * q.w,
            x2 * q.w + z2 * q.y,
            1.0 - (y2 * q.y + x2 * q.x),
        ],
    ]
}

/// Quaternion from a rotation, then normalised (`ivp_matrix_to_quat`, 0x100191b0).
pub fn matrix_to_quat(r: &[[f64; 3]; 3]) -> Quat {
    let mut q = Quat::default();
    let t = r[0][0] + r[1][1] + r[2][2];
    if t > 0.0 {
        let mut s = (t + 1.0).sqrt();
        q.w = s * 0.5;
        s = 0.5 / s;
        q.x = (r[2][1] - r[1][2]) * s;
        q.y = (r[0][2] - r[2][0]) * s;
        q.z = (r[1][0] - r[0][1]) * s;
    } else {
        let mut i = if r[0][0] < r[1][1] { 1 } else { 0 };
        if r[i][i] < r[2][2] {
            i = 2;
        }
        let j = (i + 1) % 3;
        let k = (j + 1) % 3;
        let mut v = [0.0f64; 3];
        let mut s = (r[i][i] - (r[k][k] + r[j][j]) + 1.0).sqrt();
        v[i] = s * 0.5;
        if s != 0.0 {
            s = 0.5 / s;
        }
        q.w = (r[k][j] - r[j][k]) * s;
        v[j] = (r[j][i] + r[i][j]) * s;
        v[k] = (r[k][i] + r[i][k]) * s;
        q.x = v[0];
        q.y = v[1];
        q.z = v[2];
    }
    quat_normalize(&mut q);
    q
}

/// Normalises when the length is not tiny (`ivp_quat_normalize`, 0x100194e0).
pub fn quat_normalize(q: &mut Quat) {
    let n2 = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
    if n2 > 1e-19 {
        let s = 1.0 / n2.sqrt();
        q.x *= s;
        q.y *= s;
        q.z *= s;
        q.w *= s;
    }
}

/// Newton iteration of 1/sqrt around 1 (`ivp_quat_normalize_iter`, 0x10019540).
pub fn quat_normalize_iter(q: &mut Quat) {
    let n = q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w;
    if (1.0 - n).abs() <= 1e-12 {
        return;
    }
    let mut y = 1.5 - 0.5 * n;
    while (1.0 - y * y * n).abs() > 1e-12 {
        y = (1.0 - y * y * n) * 0.5 + y;
    }
    q.x *= y;
    q.y *= y;
    q.z *= y;
    q.w *= y;
}

/// `r = a * b` (quaternion product, `ivp_quat_mul`, 0x1001e7c0).
pub fn quat_mul(a: &Quat, b: &Quat) -> Quat {
    Quat {
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    }
}

/// Spherical interpolation with the upstream's near-parallel branch (`ivp_quat_slerp`, 0x10019320).
pub fn quat_slerp(a: &Quat, b: &Quat, t: f64) -> Quat {
    let mut d = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
    let mut sign = 1.0;
    if d <= 0.0 {
        d = -d;
        sign = -1.0;
    }
    // The upstream compares with the single-precision constant 0.999f.
    if d < f64::from(0.999f32) {
        let th = d.acos();
        let k = 1.0 / (1.0 - d * d).sqrt();
        let ca = ((1.0 - t) * th).sin() * k;
        let cb = (t * th).sin() * sign * k;
        Quat {
            x: cb * b.x + ca * a.x,
            y: cb * b.y + ca * a.y,
            z: cb * b.z + ca * a.z,
            w: cb * b.w + ca * a.w,
        }
    } else {
        let x = (sign * b.x - a.x) * t + a.x;
        let y = (sign * b.y - a.y) * t + a.y;
        let z = (sign * b.z - a.z) * t + a.z;
        let w = (sign * b.w - a.w) * t + a.w;
        let n = (x * x + y * y + z * z + w * w) * 0.5;
        let mut s = 1.5 - n;
        s = (0.5 - s * s * n) + s;
        s = (0.5 - s * s * n) + s;
        Quat {
            x: x * s,
            y: y * s,
            z: z * s,
            w: w * s,
        }
    }
}

/// Rotation increment for angular velocity `w` over `dt`, third-order sine of the half angle per component
/// (`ivp_quat_from_rot_poly`, 0x10018f80).
pub fn quat_from_rot_poly(w: &[f32; 3], dt: f64) -> Quat {
    let h = dt * 0.5;
    let mut s = [0.0f64; 3];
    for i in 0..3 {
        s[i] = f64::from(w[i]) * h;
        s[i] -= s[i] * s[i] * s[i] * f64::from(1.0f32 / 6.0f32);
    }
    Quat {
        x: s[0],
        y: s[1],
        z: s[2],
        w: (1.0 - (s[0] * s[0] + s[1] * s[1] + s[2] * s[2])).sqrt(),
    }
}

/// Rotation increment with the exact sine, clamped to stay a rotation (`ivp_quat_from_rot_sin`, 0x10019010).
pub fn quat_from_rot_sin(w: &[f32; 3], dt: f64) -> Quat {
    let h = dt * 0.5;
    let mut s = [0.0f64; 3];
    for i in 0..3 {
        s[i] = (f64::from(w[i]) * h).sin();
    }
    let n = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
    if n > 1.0 {
        let k = 0.999_999_99 / n.sqrt();
        for v in s.iter_mut() {
            *v *= k;
        }
    }
    Quat {
        x: s[0],
        y: s[1],
        z: s[2],
        w: (1.0 - (s[0] * s[0] + s[1] * s[1] + s[2] * s[2])).sqrt(),
    }
}

/// Reciprocal square root from the upstream's bit-level initial guess and five Newton steps (0x1000e120).
pub fn rsqrt(n: f64) -> f64 {
    let hi = (n.to_bits() >> 32) as u32;
    let guess_hi = (0x7ff0_0000u32.wrapping_sub(hi)) / 2 + 0x1ff0_0000u32;
    let mut y = f64::from_bits(u64::from(guess_hi) << 32);
    for _ in 0..5 {
        y = (0.5 - 0.5 * n * y * y + 1.0) * y;
    }
    y
}

/// Normalises a double vector; false (unchanged) for a near-zero one (`ivp_normalize_d`, 0x1000e120).
pub fn normalize_d(v: &mut [f64; 3]) -> bool {
    let n = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if n < 1e-19 {
        return false;
    }
    let y = rsqrt(n);
    for c in v.iter_mut() {
        *c *= y;
    }
    true
}

/// Normalises a float vector and returns the length it had (`ivp_normalize_f`, 0x1000df30); 0 when near zero.
pub fn normalize_f(v: &mut [f32; 3]) -> f32 {
    let n = f64::from(v[0]) * f64::from(v[0])
        + f64::from(v[1]) * f64::from(v[1])
        + f64::from(v[2]) * f64::from(v[2]);
    if n < 1e-19 {
        return 0.0;
    }
    let y = rsqrt(n);
    for c in v.iter_mut() {
        *c = (f64::from(*c) * y) as f32;
    }
    (n * y) as f32
}

/// `R v` (`ivp_rmul`, 0x1000f6f0).
pub fn rmul(r: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [
        r[0][0] * v[0] + r[0][1] * v[1] + r[0][2] * v[2],
        r[1][0] * v[0] + r[1][1] * v[1] + r[1][2] * v[2],
        r[2][0] * v[0] + r[2][1] * v[1] + r[2][2] * v[2],
    ]
}

/// `R^T v` (`ivp_rtmul`, 0x1000f760).
pub fn rtmul(r: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [
        r[0][0] * v[0] + r[1][0] * v[1] + r[2][0] * v[2],
        r[0][1] * v[0] + r[1][1] * v[1] + r[2][1] * v[2],
        r[0][2] * v[0] + r[1][2] * v[1] + r[2][2] * v[2],
    ]
}

/// `A B` for rotations (`ivp_mat_rr`).
pub fn mat_rr(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut t = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            t[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    t
}

/// `A B^T` for rotations (`ivp_mat_rrt`, 0x1000f140).
pub fn mat_rrt(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut t = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            t[i][j] = a[i][0] * b[j][0] + a[i][1] * b[j][1] + a[i][2] * b[j][2];
        }
    }
    t
}

/// `C = A B` for transforms (`ivp_mat_mul`, 0x1000ec90).
pub fn mat_mul(a: &Matrix, b: &Matrix) -> Matrix {
    let r = mat_rr(&a.r, &b.r);
    let mut vv = rmul(&a.r, &b.vv);
    for k in 0..3 {
        vv[k] += a.vv[k];
    }
    Matrix { r, vv }
}

/// `R' = R^T`, `vv' = -(R^T vv)` (`ivp_mat_inverse`, 0x1000f2c0).
pub fn mat_inverse(m: &Matrix) -> Matrix {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = m.r[j][i];
        }
    }
    let mut vv = rmul(&r, &m.vv);
    for v in vv.iter_mut() {
        *v = -*v;
    }
    Matrix { r, vv }
}

/// General inverse by cofactors, `None` when `|det| < eps` (`ivp_mat_inverse_general`, 0x1000e8f0).
pub fn mat_inverse_general(m: &Matrix, eps: f64) -> Option<Matrix> {
    let a = &m.r;
    let mut c = [[0.0f64; 3]; 3];
    c[0][0] = a[1][1] * a[2][2] - a[1][2] * a[2][1];
    c[0][1] = a[0][2] * a[2][1] - a[0][1] * a[2][2];
    c[0][2] = a[0][1] * a[1][2] - a[0][2] * a[1][1];
    c[1][0] = a[1][2] * a[2][0] - a[1][0] * a[2][2];
    c[1][1] = a[0][0] * a[2][2] - a[0][2] * a[2][0];
    c[1][2] = a[0][2] * a[1][0] - a[0][0] * a[1][2];
    c[2][0] = a[1][0] * a[2][1] - a[1][1] * a[2][0];
    c[2][1] = a[0][1] * a[2][0] - a[0][0] * a[2][1];
    c[2][2] = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    let det = a[0][0] * c[0][0] + a[0][1] * c[1][0] + a[0][2] * c[2][0];
    if det.abs() < eps {
        return None;
    }
    let id = 1.0 / det;
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = c[i][j] * id;
        }
    }
    let mut vv = rmul(&r, &m.vv);
    for v in vv.iter_mut() {
        *v = -*v;
    }
    Some(Matrix { r, vv })
}

/// An orthonormal basis whose column `k` is the normalised `axis` (`ivp_basis`, 0x1000eb60).
pub fn basis(axis: &[f64; 3], k: usize) -> [[f64; 3]; 3] {
    let mut a = *axis;
    normalize_d(&mut a);
    let mut v = [a[1], a[2] - a[0], -a[1]];
    if !normalize_d(&mut v) {
        v = [a[2], -a[2], a[1] - a[0]];
        normalize_d(&mut v);
    }
    // v × a
    let u = [
        v[1] * a[2] - v[2] * a[1],
        v[2] * a[0] - v[0] * a[2],
        v[0] * a[1] - v[1] * a[0],
    ];
    let (k1, k2) = ((k + 1) % 3, (k + 2) % 3);
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        r[i][k] = a[i];
        r[i][k1] = u[i];
        r[i][k2] = v[i];
    }
    r
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn quaternion_round_trip_through_a_rotation() {
        let q = quat_normalize_then(Quat {
            x: 0.2,
            y: -0.4,
            z: 0.1,
            w: 0.9,
        });
        let back = matrix_to_quat(&quat_to_matrix(&q));
        // q and -q are the same rotation.
        let same =
            close(back.x, q.x) && close(back.y, q.y) && close(back.z, q.z) && close(back.w, q.w);
        let flipped = close(back.x, -q.x)
            && close(back.y, -q.y)
            && close(back.z, -q.z)
            && close(back.w, -q.w);
        assert!(same || flipped, "{q:?} came back as {back:?}");
    }

    fn quat_normalize_then(mut q: Quat) -> Quat {
        quat_normalize(&mut q);
        q
    }

    #[test]
    fn inverse_transform_undoes_the_transform() {
        let r = quat_to_matrix(&quat_normalize_then(Quat {
            x: 0.1,
            y: 0.5,
            z: -0.3,
            w: 0.8,
        }));
        let m = Matrix {
            r,
            vv: [3.0, -1.0, 2.0],
        };
        let id = mat_mul(&m, &mat_inverse(&m));
        for i in 0..3 {
            for j in 0..3 {
                let want = if i == j { 1.0 } else { 0.0 };
                assert!(close(id.r[i][j], want), "r[{i}][{j}] = {}", id.r[i][j]);
            }
            assert!(close(id.vv[i], 0.0), "vv[{i}] = {}", id.vv[i]);
        }
    }

    #[test]
    fn general_inverse_agrees_with_the_transpose_for_rotations() {
        let m = Matrix {
            r: quat_to_matrix(&quat_normalize_then(Quat {
                x: 0.3,
                y: 0.0,
                z: 0.2,
                w: 0.9,
            })),
            vv: [1.0, 2.0, 3.0],
        };
        let a = mat_inverse(&m);
        let b = mat_inverse_general(&m, 1e-12).expect("a rotation is invertible");
        for i in 0..3 {
            assert!(close(a.vv[i], b.vv[i]));
            for j in 0..3 {
                assert!(close(a.r[i][j], b.r[i][j]));
            }
        }
    }

    #[test]
    fn rsqrt_is_accurate() {
        for n in [1e-6, 0.5, 1.0, 2.0, 1234.5] {
            let y = rsqrt(n);
            assert!(
                (y - 1.0 / n.sqrt()).abs() / (1.0 / n.sqrt()) < 1e-9,
                "rsqrt({n}) = {y}"
            );
        }
    }

    #[test]
    fn basis_is_orthonormal_with_the_axis_in_column_k() {
        let axis = [0.3, -0.7, 0.2];
        for k in 0..3 {
            let r = basis(&axis, k);
            let mut a = axis;
            normalize_d(&mut a);
            for i in 0..3 {
                assert!(close(r[i][k], a[i]), "column {k} is not the axis");
            }
            let p = mat_rrt(&r, &r);
            for i in 0..3 {
                for j in 0..3 {
                    assert!(
                        close(p[i][j], if i == j { 1.0 } else { 0.0 }),
                        "not orthonormal at {i}{j}"
                    );
                }
            }
        }
    }

    #[test]
    fn slerp_halfway_between_identity_and_a_quarter_turn() {
        let id = Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        };
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let quarter = Quat {
            x: 0.0,
            y: 0.0,
            z: s,
            w: s,
        };
        let half = quat_slerp(&id, &quarter, 0.5);
        let eighth = Quat {
            x: 0.0,
            y: 0.0,
            z: (std::f64::consts::FRAC_PI_8).sin(),
            w: (std::f64::consts::FRAC_PI_8).cos(),
        };
        assert!(
            close(half.z, eighth.z) && close(half.w, eighth.w),
            "{half:?}"
        );
    }
}
