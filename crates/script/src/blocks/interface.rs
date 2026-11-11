// Interface.dll: 2D text and frames.

use super::*;

pub(crate) fn text_display(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let t = rt.target(b, host);
    let name = ent_name(rt, t);
    if input(rt, b, 0) {
        let text = pin(rt, b, "Text", 4, host).as_string();
        host.text(&name, &text, true);
        out(rt, b, 0);
    }
    if input(rt, b, 1) {
        host.text(&name, "", false);
        out(rt, b, 1);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[Proto {
    guid: proto::TEXT_DISPLAY,
    name: "Text Display",
    library: Library::Interface,
    pins: &["", "", "", "", "Text"],
    pouts: &[],
    flags: flags::TARGET,
    standing: Standing::Partial,
    layouts: &[],
    run: text_display,
    event: None,
}];
