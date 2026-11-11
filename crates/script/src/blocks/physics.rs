// physics_RT.dll: physicalized objects and forces.

use super::*;

pub(crate) fn physicalize(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let on = input(rt, b, 0);
    let off = input(rt, b, 1);
    if on || off {
        if let Some(e) = target_ent(rt, b, host) {
            host.physicalize(e, on);
        }
        out(rt, b, usize::from(!on));
    }
    Flow::Done
}

pub(crate) fn set_physics_force(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        host.note("script sets a physics force (not applied yet)");
        out(rt, b, 0);
    }
    if input(rt, b, 1) {
        out(rt, b, 1);
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
        standing: Standing::Partial,
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
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: set_physics_force,
        event: None,
    },
];
