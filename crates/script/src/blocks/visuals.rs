// Visuals.dll: show and hide.

use super::*;

pub(crate) fn hide(rt: &mut Runtime, _id: Id, b: &Block, proto: Guid, host: &mut dyn Host) -> Flow {
    if input(rt, b, 0) {
        let t = rt.target(b, host);
        if let Some(e) = rt.ent(t) {
            host.set_visible(e, proto == proto::SHOW);
        } else if let Some(t) = t
            && matches!(rt.graph.objects.get(t).map(|o| o.kind), Some(ObjKind::Other))
            && let Some(e) = super::environment::ent2d_slot(rt, t)
        {
            // a 2D entity (the engine has no row for it): the run keeps the mark, the interface draws by it
            e.shown = Some(proto == proto::SHOW);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::HIDE,
        name: "Hide",
        library: Library::Visuals,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: hide,
        event: None,
    },
    Proto {
        guid: proto::SHOW,
        name: "Show",
        library: Library::Visuals,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: hide,
        event: None,
    },
];
