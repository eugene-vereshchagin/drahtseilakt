// 3DTransfo.dll: positions, orientations, rotations, scale, world matrices.

use super::*;

pub(crate) fn set_position(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let p = pin(rt, b, "Position", 0, host).as_vec3();
        let r = referential(rt, b, "Referential", 1, host);
        if let Some(e) = target_ent(rt, b, host)
            && let Some(mut m) = host.matrix(e)
        {
            let world = match r {
                Some(rm) => transform_point(&rm, p),
                None => p,
            };
            m[3] = [world[0], world[1], world[2], 1.0];
            host.set_matrix(e, m);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn set_orientation(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let dir = pin(rt, b, "Dir", 0, host).as_vec3();
        let up = pin(rt, b, "Up", 1, host).as_vec3();
        let r = referential(rt, b, "Referential", 3, host);
        if let Some(e) = target_ent(rt, b, host)
            && let Some(mut m) = host.matrix(e)
        {
            let (dir, up) = match r {
                Some(rm) => (transform_dir(&rm, dir), transform_dir(&rm, up)),
                None => (dir, up),
            };
            let z = normalize(dir);
            let x = normalize(cross(up, z));
            let y = cross(z, x);
            let s = scale_of(&m);
            m[0] = [x[0] * s[0], x[1] * s[0], x[2] * s[0], 0.0];
            m[1] = [y[0] * s[1], y[1] * s[1], y[2] * s[1], 0.0];
            m[2] = [z[0] * s[2], z[1] * s[2], z[2] * s[2], 0.0];
            host.set_matrix(e, m);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn rotate(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let axis = pin(rt, b, "Axis of Rotation", 0, host).as_vec3();
        let angle = pin(rt, b, "Angle of Rotation", 1, host).as_float();
        let r = referential(rt, b, "Referential of the Axis", 2, host);
        if let Some(e) = target_ent(rt, b, host)
            && let Some(m) = host.matrix(e)
        {
            let axis = normalize(match r {
                Some(rm) => transform_dir(&rm, axis),
                None => axis,
            });
            let pos = m[3];
            let mut rotated = mat_mul(&rotation(axis, angle), &m);
            rotated[3] = pos;
            host.set_matrix(e, rotated);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn set_world_matrix(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        if let Some(m) = pin(rt, b, "World Matrix", 0, host).as_matrix()
            && let Some(e) = target_ent(rt, b, host)
        {
            host.set_matrix(e, m);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn set_euler_orientation(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let x = pin(rt, b, "X", 0, host).as_float();
        let y = pin(rt, b, "Y", 1, host).as_float();
        let z = pin(rt, b, "Z", 2, host).as_float();
        let r = referential(rt, b, "Referential", 4, host);
        if let Some(e) = target_ent(rt, b, host)
            && let Some(m) = host.matrix(e)
        {
            // the turn about X first, then Y, then Z, in the referential's axes (the world's without one)
            let mut rot = mat_mul(
                &rotation([0.0, 0.0, 1.0], z),
                &mat_mul(&rotation([0.0, 1.0, 0.0], y), &rotation([1.0, 0.0, 0.0], x)),
            );
            if let Some(rm) = r {
                let mut frame = rm;
                for (col, len) in frame.iter_mut().zip(scale_of(&rm)).take(3) {
                    if len > 1e-12 {
                        *col = col.map(|v| v / len);
                    }
                }
                frame[3] = [0.0, 0.0, 0.0, 1.0];
                rot = mat_mul(&frame, &rot);
            }
            let s = scale_of(&m);
            let mut next = m;
            for ((dst, src), len) in next.iter_mut().zip(rot.iter()).zip(s).take(3) {
                *dst = [src[0] * len, src[1] * len, src[2] * len, 0.0];
            }
            host.set_matrix(e, next);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn scale(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let v = pin(rt, b, "Scaling Vector", 0, host).as_vec3();
        let absolute = pin(rt, b, "Absolute", 2, host).as_bool();
        if let Some(e) = target_ent(rt, b, host)
            && let Some(mut m) = host.matrix(e)
        {
            let s = scale_of(&m);
            for ((col, want), len) in m.iter_mut().zip(v).zip(s).take(3) {
                let k = if absolute {
                    if len > 1e-12 { want / len } else { 1.0 }
                } else {
                    want
                };
                *col = [col[0] * k, col[1] * k, col[2] * k, 0.0];
            }
            host.set_matrix(e, m);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::SET_POSITION,
        name: "Set Position",
        library: Library::Transfo3d,
        pins: &["Position", "Referential"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_position,
        event: None,
    },
    Proto {
        guid: proto::SET_ORIENTATION,
        name: "Set Orientation",
        library: Library::Transfo3d,
        pins: &["Dir", "Up", "", "Referential"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_orientation,
        event: None,
    },
    Proto {
        guid: proto::ROTATE,
        name: "Rotate",
        library: Library::Transfo3d,
        pins: &[
            "Axis of Rotation",
            "Angle of Rotation",
            "Referential of the Axis",
        ],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: rotate,
        event: None,
    },
    Proto {
        guid: proto::SET_WORLD_MATRIX,
        name: "Set World Matrix",
        library: Library::Transfo3d,
        pins: &["World Matrix"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_world_matrix,
        event: None,
    },
    Proto {
        guid: proto::SET_EULER_ORIENTATION,
        name: "Set Euler Orientation",
        library: Library::Transfo3d,
        pins: &["X", "Y", "Z", "", "Referential"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_euler_orientation,
        event: None,
    },
    Proto {
        guid: proto::SCALE,
        name: "Scale",
        library: Library::Transfo3d,
        pins: &["Scaling Vector", "", "Absolute"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Partial,
        layouts: &[],
        run: scale,
        event: None,
    },
];
