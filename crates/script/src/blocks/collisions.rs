// Collisions.dll: intersection tests.

use super::*;

pub(crate) fn box_box_intersection(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let a = pin(rt, b, "Entity 0", 0, host).as_obj();
        let c = pin(rt, b, "Entity 1", 1, host).as_obj();
        let (ea, ec) = (rt.ent(a), rt.ent(c));
        let hit = match (
            ea.and_then(|e| host.bounds(e)),
            ec.and_then(|e| host.bounds(e)),
        ) {
            (Some((amin, amax)), Some((cmin, cmax))) => amin
                .iter()
                .zip(amax.iter())
                .zip(cmin.iter().zip(cmax.iter()))
                .all(|((a0, a1), (c0, c1))| a0 <= c1 && c0 <= a1),
            _ => false,
        };
        out(rt, b, usize::from(!hit));
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[Proto {
    guid: proto::BOX_BOX_INTERSECTION,
    name: "Box Box Intersection",
    library: Library::Collisions,
    pins: &["Entity 0", "Entity 1"],
    pouts: &[],
    flags: 0,
    standing: Standing::Full,
    layouts: &[],
    run: box_box_intersection,
    event: None,
}];
