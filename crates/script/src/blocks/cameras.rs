// Cameras.dll: the camera.

use super::*;

pub(crate) fn get_current_camera(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        set_pout(rt, b, "Current Camera", 0, Value::Obj(None));
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[Proto {
    guid: proto::GET_CURRENT_CAMERA,
    name: "Get Current Camera",
    library: Library::Cameras,
    pins: &[],
    pouts: &["Current Camera"],
    flags: 0,
    standing: Standing::Partial,
    layouts: &[],
    run: get_current_camera,
    event: None,
}];
