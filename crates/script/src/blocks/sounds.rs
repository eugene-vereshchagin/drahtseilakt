// Sounds.dll: wave sounds.

use super::*;

pub(crate) fn wave_player(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let t = rt.target(b, host);
    let name = ent_name(rt, t);
    if input(rt, b, 0) {
        host.sound(&name, true);
        out(rt, b, 0);
    }
    if input(rt, b, 1) {
        host.sound(&name, false);
        out(rt, b, 1);
    }
    if input(rt, b, 2) {
        out(rt, b, 2);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[Proto {
    guid: proto::WAVE_PLAYER,
    name: "Wave Player",
    library: Library::Sounds,
    pins: &[],
    pouts: &[],
    flags: flags::TARGET,
    standing: Standing::Partial,
    layouts: &[],
    run: wave_player,
    event: None,
}];
