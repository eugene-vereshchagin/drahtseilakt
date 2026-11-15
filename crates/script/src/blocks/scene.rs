// Blocks that change the scene itself: objects made and deleted (Object Create, Delete Dynamic Objects), the
// hierarchy (Set Parent), scripts attached to objects (Attach Script) and the
// physics slider (Set Physics Slider). The effects are the run's: the entity table, the graph, the physics queue;
// each is capped (docs/drahtseilakt/scripts.md, "Scene changes").

use super::*;
use crate::exec::SliderSpec;

/// Object Create (Narratives, `FUN_25681720`): pins Class, Name; local Dynamic; pOut the object. The object is made
/// in the run (docs: `Runtime::create_object`); Out comes whether or not one was made, as the original's.
pub(crate) fn object_create(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        out(rt, b, 0);
        let class = pin(rt, b, "Class", 0, host).as_int();
        let name = pin(rt, b, "Name", 1, host).as_string();
        let dynamic = b.locals.is_empty() || local(rt, b, 0).as_bool();
        let name: String = name.chars().take(MAX_NAME).collect();
        if let Ok(class) = u32::try_from(class)
            && let Some(o) = rt.create_object(class, &name, dynamic, host)
        {
            set_pout(rt, b, "Object", 0, Value::Obj(Some(o)));
        }
    }
    Flow::Done
}

/// The longest name a created object gets.
const MAX_NAME: usize = 256;

/// Delete Dynamic Objects (Narratives, `FUN_256810f0`): everything the scripts made dynamic is deleted. (The
/// original's context destroys all its dynamic objects; the C oracle tracks none.)
pub(crate) fn delete_dynamic_objects(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        rt.delete_dynamic(host);
        out(rt, b, 0);
    }
    Flow::Done
}

/// Set Parent (3DTransfo, `0x25002a70`, C: `bb_set_parent`): the target gets the Parent pin's entity as its
/// parent, keeping its world place; a parent that is not an entity unparents; itself or a descendant is refused
/// (the table checks the chain). No target entity: nothing, not even Out.
pub(crate) fn set_parent(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(e) = target_ent(rt, b, host) else {
        return Flow::Done;
    };
    input(rt, b, 0);
    out(rt, b, 0);
    let parent = pin(rt, b, "Parent", 0, host).as_obj();
    let parent = match rt.ent(parent) {
        Some(p) if p == e => return Flow::Done,
        Some(p @ Ent::Object(_)) => Some(p),
        _ => None,
    };
    if !host.set_parent(e, parent) {
        host.note("script Set Parent refused (a loop, or the engine keeps no hierarchy for it)");
    }
    Flow::Done
}

/// Attach Script (Narratives): pins Dynamic?, Script, Activate?, Reset?; target the object; pOut Created Script.
pub(crate) fn attach_script(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let dynamic = pin(rt, b, "Dynamic?", 0, host).as_bool();
        let script = pin(rt, b, "Script", 1, host).as_obj();
        let activate = pin(rt, b, "Activate?", 2, host).as_bool();
        let reset = pin(rt, b, "Reset?", 3, host).as_bool();
        let target = rt.target(b, host);
        let made = match (script, target) {
            (Some(s), Some(t)) => rt.attach_script(s, t, dynamic, activate, reset, host),
            _ => None,
        };
        if let Some(c) = made {
            set_pout(rt, b, "Created Script", 0, Value::Obj(Some(c)));
        }
        out(rt, b, 0);
    }
    Flow::Done
}

/// The world place of an entity pin.
fn place_of(
    rt: &mut Runtime,
    b: &Block,
    i: usize,
    name: &str,
    host: &mut dyn Host,
) -> Option<[f32; 3]> {
    let o = pin(rt, b, name, i, host).as_obj();
    let e = rt.ent(o)?;
    let m = host.matrix(e)?;
    Some([m[3][0], m[3][1], m[3][2]])
}

/// Set Physics Slider (Physics, `FUN_10005d90` and `FUN_10005f10`, C: `cmd_slider`): on Create the slider of this
/// block joins the target's body and Object2's along the axis from the first point to the second (world places
/// when the block runs), limited in metres when asked; on Destroy it is removed. A missing target, Object2 or
/// point makes none. Out1 follows Create, Out2 Destroy.
pub(crate) fn set_physics_slider(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let t = target_ent(rt, b, host);
        let o2 = pin(rt, b, "Object2", 0, host).as_obj();
        let o2 = rt.ent(o2);
        let p0 = place_of(rt, b, 1, "Axis first Point", host);
        let p1 = place_of(rt, b, 2, "Axis second Point", host);
        let limited = pin(rt, b, "Limitations (meter)", 3, host).as_bool();
        let lo = pin(rt, b, "Lower Limit", 4, host).as_float();
        let hi = pin(rt, b, "Upper Limit", 5, host).as_float();
        if let (Some(t), Some(o2), Some(p0), Some(p1)) = (t, o2, p0, p1) {
            let spec = SliderSpec {
                p0,
                p1,
                limits: limited.then_some((lo, hi)),
            };
            host.physics_slider(t, o2, id, Some(spec));
        }
        out(rt, b, 0);
    }
    if input(rt, b, 1) {
        let t = target_ent(rt, b, host);
        let o2 = pin(rt, b, "Object2", 0, host).as_obj();
        if let (Some(t), Some(o2)) = (t, rt.ent(o2)) {
            host.physics_slider(t, o2, id, None);
        }
        out(rt, b, 1);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::OBJECT_CREATE,
        name: "Object Create",
        library: Library::Narratives,
        pins: &["Class", "Name"],
        pouts: &["Object"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: object_create,
        event: None,
    },
    Proto {
        guid: proto::DELETE_DYNAMIC_OBJECTS,
        name: "Delete Dynamic Objects",
        library: Library::Narratives,
        pins: &["Scene"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: delete_dynamic_objects,
        event: None,
    },
    Proto {
        guid: proto::SET_PARENT,
        name: "Set Parent",
        library: Library::Transfo3d,
        pins: &["Parent"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_parent,
        event: None,
    },
    Proto {
        guid: proto::ATTACH_SCRIPT,
        name: "Attach Script",
        library: Library::Narratives,
        pins: &["Dynamic?", "Script", "Activate?", "Reset?"],
        pouts: &["Created Script"],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: attach_script,
        event: None,
    },
    Proto {
        guid: proto::SET_PHYSICS_SLIDER,
        name: "Set Physics Slider",
        library: Library::Physics,
        pins: &[
            "Object2",
            "Axis first Point",
            "Axis second Point",
            "Limitations (meter)",
            "Lower Limit",
            "Upper Limit",
        ],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_physics_slider,
        event: None,
    },
];
