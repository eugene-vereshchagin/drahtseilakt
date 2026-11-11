// Visuals.dll: show and hide.

use super::*;

pub(crate) fn hide(rt: &mut Runtime, _id: Id, b: &Block, proto: Guid, host: &mut dyn Host) -> Flow {
    if input(rt, b, 0) {
        if let Some(e) = target_ent(rt, b, host) {
            host.set_visible(e, proto == proto::SHOW);
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
