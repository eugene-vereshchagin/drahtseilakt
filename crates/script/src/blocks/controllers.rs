// Controllers.dll: keyboard and mouse.

use super::*;

pub(crate) fn key_event(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    // Keys other than the game's are not in a run's input: the event never fires.
    if input(rt, b, 1) {
        return Flow::Done;
    }
    input(rt, b, 0);
    Flow::Continue
}

pub(super) const PROTOS: &[Proto] = &[Proto {
    guid: proto::KEY_EVENT,
    name: "Key Event",
    library: Library::Controllers,
    pins: &[],
    pouts: &[],
    flags: 0,
    standing: Standing::Partial,
    layouts: &[],
    run: key_event,
    event: None,
}];
