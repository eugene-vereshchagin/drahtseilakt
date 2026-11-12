// Cameras.dll: the camera. The change goes to the host for this run only; the simulation never reads it.

use super::*;
use crate::exec::{CameraChange, CameraState};

/// A float pin's value, or the default when it is not linked or not a float.
fn float_or(v: Value, default: f32) -> f32 {
    match v {
        Value::Float(f) => f,
        _ => default,
    }
}

/// Get Current Camera: the camera object (none: the game's own) and its position.
pub(crate) fn get_current_camera(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let cam: CameraState = host.camera_get();
        set_pout(rt, b, "Current Camera", 0, Value::Obj(cam.object));
        set_pout(rt, b, "Position", 1, Value::Vec3(cam.position));
        out(rt, b, 0);
    }
    Flow::Done
}

/// Set As Active Camera: the target object becomes the view's camera (a target that is not an object is ignored).
pub(crate) fn set_as_active_camera(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0)
        && let Some(Ent::Object(i)) = target_ent(rt, b, host)
    {
        host.camera_set(CameraChange::Active(i));
        out(rt, b, 0);
    }
    Flow::Done
}

/// Set Clipping Planes: the near and far planes (defaults 0.1 and 200, as the original's camera). Only a near
/// above zero and a far above the near is applied; any other pair is ignored.
pub(crate) fn set_clipping_planes(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let near = float_or(pin(rt, b, "Near", 0, host), 0.1);
        let far = float_or(pin(rt, b, "Far", 1, host), 200.0);
        if near.is_finite() && far.is_finite() && near > 0.0 && far > near {
            host.camera_set(CameraChange::Clip { near, far });
        }
        out(rt, b, 0);
    }
    Flow::Done
}

/// Set FOV: the view's field of view, the Angle pin in radians across the 4:3 view (as InGameCam's 1.0123); it is
/// applied to the active view whatever the target, and an angle that is not finite is ignored.
pub(crate) fn set_fov(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let angle = float_or(pin(rt, b, "Angle", 0, host), f32::NAN);
        if angle.is_finite() {
            host.camera_set(CameraChange::Fov(angle.to_degrees()));
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::GET_CURRENT_CAMERA,
        name: "Get Current Camera",
        library: Library::Cameras,
        pins: &[],
        pouts: &["Current Camera", "Position"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: get_current_camera,
        event: None,
    },
    Proto {
        guid: proto::SET_AS_ACTIVE_CAMERA,
        name: "Set As Active Camera",
        library: Library::Cameras,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_as_active_camera,
        event: None,
    },
    Proto {
        guid: proto::SET_CLIPPING_PLANES,
        name: "Set Clipping Planes",
        library: Library::Cameras,
        pins: &["Near", "Far"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: set_clipping_planes,
        event: None,
    },
    Proto {
        guid: proto::SET_FOV,
        name: "Set FOV",
        library: Library::Cameras,
        pins: &["Angle"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_fov,
        event: None,
    },
];
