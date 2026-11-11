// physics_RT.dll: physicalized objects and forces.

use super::*;

/// The most pins of a Physicalize block that are looked at (a hostile file cannot make the scan long).
const MAX_PINS: usize = 512;

/// Reads a Physicalize block's pins: the fixed ones by place, the dynamic ones (`ball radius i`) by name.
fn read_spec(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> crate::PhysSpec {
    let f = |rt: &mut Runtime, host: &mut dyn Host, i: usize, def: f32| match b.pins.get(i) {
        Some(&p) => rt.read(p, host).as_float(),
        None => def,
    };
    let flag = |rt: &mut Runtime, host: &mut dyn Host, i: usize, def: bool| match b.pins.get(i) {
        Some(&p) => rt.read(p, host).as_bool(),
        None => def,
    };
    let count = |i: usize| usize::try_from(local(rt, b, i).as_int()).unwrap_or(0);
    let (convex_count, ball_count, concave_count) = (count(0), count(1), count(2));
    let shift = local(rt, b, 3).as_vec3();
    let mut radius = None;
    for &p in b.pins.iter().skip(11).take(MAX_PINS) {
        let is_radius = rt
            .graph
            .param(p)
            .is_some_and(|q| q.name.to_ascii_lowercase().starts_with("ball radius"));
        if is_radius {
            radius = Some(rt.read(p, host).as_float());
        }
    }
    let group = match b.pins.get(4) {
        Some(&p) => rt.read(p, host).as_string(),
        None => String::new(),
    };
    let surface = match b.pins.get(10) {
        Some(&p) => rt.read(p, host).as_string(),
        None => String::new(),
    };
    crate::PhysSpec {
        fixed: flag(rt, host, 0, false),
        friction: f(rt, host, 1, 0.7),
        elasticity: f(rt, host, 2, 0.4),
        mass: f(rt, host, 3, 1.0),
        group: group.chars().take(8).collect(),
        start_frozen: flag(rt, host, 5, false),
        collide: flag(rt, host, 6, true),
        auto_centre: flag(rt, host, 7, true),
        shift,
        lin_damp: f(rt, host, 8, 0.1),
        rot_damp: f(rt, host, 9, 0.1),
        surface,
        convex_count,
        ball_count,
        concave_count,
        radius,
    }
}

pub(crate) fn physicalize(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let on = input(rt, b, 0);
    let off = input(rt, b, 1);
    if on || off {
        if let Some(e) = target_ent(rt, b, host) {
            if on {
                let spec = read_spec(rt, b, host);
                host.physicalize_body(e, id, &spec);
            } else {
                host.physicalize(e, false);
            }
        }
        out(rt, b, usize::from(!on));
    }
    Flow::Done
}

/// A world vector from a vector pin and its referential pin (none: already world).
fn world_dir(
    rt: &mut Runtime,
    b: &Block,
    v: [f32; 3],
    name: &str,
    i: usize,
    host: &mut dyn Host,
) -> [f32; 3] {
    match referential(rt, b, name, i, host) {
        Some(m) => transform_dir(&m, v),
        None => v,
    }
}

pub(crate) fn set_physics_force(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let pos = pin(rt, b, "Position", 0, host).as_vec3();
        let dir = pin(rt, b, "Direction", 2, host).as_vec3();
        let value = pin(rt, b, "Force Value", 4, host).as_float();
        let at = match referential(rt, b, "Pos Referential", 1, host) {
            Some(m) => transform_point(&m, pos),
            None => pos,
        };
        let mut d = world_dir(rt, b, dir, "Direction Ref", 3, host);
        let l2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        d = if l2 <= 1e-4 {
            [1.0, 0.0, 0.0]
        } else {
            normalize(d)
        };
        let force = [d[0] * value, d[1] * value, d[2] * value];
        if let Some(e) = target_ent(rt, b, host) {
            host.physics_force(e, id, true, at, force);
        }
        out(rt, b, 0);
    }
    if input(rt, b, 1) {
        if let Some(e) = target_ent(rt, b, host) {
            host.physics_force(e, id, false, [0.0; 3], [0.0; 3]);
        }
        out(rt, b, 1);
    }
    Flow::Done
}

pub(crate) fn physics_impulse(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let pos = pin(rt, b, "Position", 0, host).as_vec3();
        let dir = pin(rt, b, "Direction", 2, host).as_vec3();
        let value = pin(rt, b, "Impulse", 4, host).as_float();
        let at = match referential(rt, b, "Referential", 1, host) {
            Some(m) => transform_point(&m, pos),
            None => pos,
        };
        let two_pos = local(rt, b, 0).as_bool();
        let d = if two_pos {
            // "2 pos instead of dir": the direction is the second point minus the first (both through their
            // referentials)
            let to = match referential(rt, b, "Direction Ref", 3, host) {
                Some(m) => transform_point(&m, dir),
                None => dir,
            };
            [to[0] - at[0], to[1] - at[1], to[2] - at[2]]
        } else {
            world_dir(rt, b, dir, "Direction Ref", 3, host)
        };
        let d = normalize(d);
        let value = if local(rt, b, 1).as_bool() {
            value * host.delta_ms() * 0.001
        } else {
            value
        };
        if let Some(e) = target_ent(rt, b, host) {
            host.physics_impulse(e, at, [d[0] * value, d[1] * value, d[2] * value]);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn physics_wakeup(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        if let Some(e) = target_ent(rt, b, host) {
            host.physics_wake(e);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::PHYSICALIZE,
        name: "Physicalize",
        library: Library::Physics,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: physicalize,
        event: None,
    },
    Proto {
        guid: proto::SET_PHYSICS_FORCE,
        name: "SetPhysicsForce",
        library: Library::Physics,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_physics_force,
        event: None,
    },
    Proto {
        guid: proto::PHYSICS_IMPULSE,
        name: "Physics Impulse",
        library: Library::Physics,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: physics_impulse,
        event: None,
    },
    Proto {
        guid: proto::PHYSICS_WAKEUP,
        name: "Physics WakeUp",
        library: Library::Physics,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Partial,
        layouts: &[],
        run: physics_wakeup,
        event: None,
    },
];
