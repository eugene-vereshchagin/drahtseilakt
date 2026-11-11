// Narratives.dll: scripts and objects (activate, deactivate, execute, copy, delete, initial conditions).

use super::*;

pub(crate) fn activate_script(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let script = pin(
            rt,
            b,
            "Script",
            if proto == proto::DEACTIVATE_SCRIPT {
                0
            } else {
                1
            },
            host,
        )
        .as_obj();
        if let Some(s) = script
            && rt.graph.block(s).is_some()
        {
            if proto == proto::DEACTIVATE_SCRIPT {
                rt.activate(s, false, false, 0);
            } else {
                let reset = pin(rt, b, "Reset ?", 0, host).as_bool();
                rt.activate(s, true, reset, 0);
            }
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn activate_object(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let o = pin(rt, b, "Object", 0, host).as_obj();
        if let Some(o) = o {
            let reset = pin(rt, b, "Reset Scripts", 3, host).as_bool()
                || pin(rt, b, "Reset ?", 1, host).as_bool();
            rt.activate_object(o, proto == proto::ACTIVATE_OBJECT, reset);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn object_delete(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let o = pin(rt, b, "Object", 0, host).as_obj();
        match o.and_then(|x| rt.graph.objects.get(x)).map(|i| i.kind) {
            Some(ObjKind::Entity(_)) | Some(ObjKind::Ball) => {
                if let Some(e) = rt.ent(o) {
                    host.delete(e);
                }
            }
            Some(ObjKind::Group(gi)) => {
                // deleting a group deletes the group object only; its members stay (as the original)
                host.note(&format!("script deletes group {} (members kept)", gi));
            }
            Some(ObjKind::Behavior) => {
                if let Some(x) = o {
                    rt.activate(x, false, false, 0);
                }
            }
            _ => {}
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn restore_ic(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        if let Some(e) = target_ent(rt, b, host)
            && let Some(m) = host.initial_matrix(e)
        {
            host.set_matrix(e, m);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn set_ic(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    // Looks only, or initial conditions kept as loaded: accepted, nothing to do in this engine.
    if input(rt, b, 0) {
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn object_copy(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let o = pin(rt, b, "Original", 1, host).as_obj();
        let copy = o.and_then(|o| rt.copy_object(o, host));
        if copy.is_none() {
            host.note(&format!(
                "script copies {} (no copy made; the copy is nothing)",
                ent_name(rt, o)
            ));
        }
        set_pout(rt, b, "Copy", 0, Value::Obj(copy));
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::ACTIVATE_SCRIPT,
        name: "Activate Script",
        library: Library::Narratives,
        pins: &["Reset ?", "Script"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: activate_script,
        event: None,
    },
    Proto {
        guid: proto::EXECUTE_SCRIPT,
        name: "Execute Script",
        library: Library::Narratives,
        pins: &["Reset ?", "Script"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: activate_script,
        event: None,
    },
    Proto {
        guid: proto::DEACTIVATE_SCRIPT,
        name: "Deactivate Script",
        library: Library::Narratives,
        pins: &["Script"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: activate_script,
        event: None,
    },
    Proto {
        guid: proto::ACTIVATE_OBJECT,
        name: "Activate Object",
        library: Library::Narratives,
        pins: &["Object", "Reset ?", "", "Reset Scripts"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: activate_object,
        event: None,
    },
    Proto {
        guid: proto::DEACTIVATE_OBJECT,
        name: "Deactivate Object",
        library: Library::Narratives,
        pins: &["Object", "Reset ?", "", "Reset Scripts"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: activate_object,
        event: None,
    },
    Proto {
        guid: proto::OBJECT_DELETE,
        name: "Object Delete",
        library: Library::Narratives,
        pins: &["Object"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: object_delete,
        event: None,
    },
    Proto {
        guid: proto::RESTORE_IC,
        name: "Restore IC",
        library: Library::Narratives,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: restore_ic,
        event: None,
    },
    Proto {
        guid: proto::SET_IC,
        name: "Set IC",
        library: Library::Narratives,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: set_ic,
        event: None,
    },
    Proto {
        guid: proto::OBJECT_COPY,
        name: "Object Copy",
        library: Library::Narratives,
        pins: &["", "Original"],
        pouts: &["Copy"],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: object_copy,
        event: None,
    },
];
