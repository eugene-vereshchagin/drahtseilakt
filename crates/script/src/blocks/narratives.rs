// Narratives.dll: scripts and objects (activate, deactivate, execute, copy, delete, initial conditions).

use super::*;

/// Activate Script (docs/drahtseilakt/scripts.md, "Stage 0"): pin 0 Reset?, pins 1.. the scripts (an older layout:
/// pin 0 the script, pin 1 Reset?, default true); local 0 Awake Object (default true) wakes a sleeping owner.
/// Each script gets an activation request, applied at the start of the next tick. Out at once.
pub(crate) fn activate_script(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    out(rt, b, 0);
    let old = super::find(proto).is_some_and(|p| p.layout(rt, b) != p.pins);
    let pin_at =
        |rt: &mut Runtime, i: usize, host: &mut dyn Host| b.pins.get(i).map(|&p| rt.read(p, host));
    let (reset, scripts) = if old {
        let reset = pin_at(rt, 1, host).is_none_or(|v| v.as_bool());
        (reset, vec![pin_at(rt, 0, host).and_then(|v| v.as_obj())])
    } else {
        let reset = pin_at(rt, 0, host).is_none_or(|v| v.as_bool());
        let scripts = (1..b.pins.len())
            .map(|i| pin_at(rt, i, host).and_then(|v| v.as_obj()))
            .collect();
        (reset, scripts)
    };
    let awake = old || b.locals.is_empty() || local(rt, b, 0).as_bool();
    for s in scripts.into_iter().flatten() {
        if rt.graph.block(s).is_none() {
            continue;
        }
        rt.request(s, true, reset);
        if awake {
            rt.wake_owner(s, host);
        }
    }
    Flow::Done
}

/// Deactivate Script: every pin is a script; each gets a deactivation request for the next tick. Out at once.
pub(crate) fn deactivate_script(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    out(rt, b, 0);
    for i in 0..b.pins.len() {
        let s = b.pins.get(i).copied().map(|p| rt.read(p, host).as_obj());
        if let Some(Some(s)) = s
            && rt.graph.block(s).is_some()
        {
            rt.request(s, false, false);
        }
    }
    Flow::Done
}

/// Execute Script: pin 0 Reset? (default false), pin 1 the script; local 0 Awake Object. On In the script is
/// activated and run at once inside this frame; Out fires in the first later tick in which the script is no longer
/// active (the block waits until then).
pub(crate) fn execute_script(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let script = match b.pins.get(1) {
        Some(&p) => rt.read(p, host).as_obj(),
        None => None,
    }
    .filter(|&s| rt.graph.block(s).is_some());
    let Some(s) = script else {
        input(rt, b, 0);
        out(rt, b, 0);
        return Flow::Done;
    };
    if input(rt, b, 0) {
        let reset = match b.pins.first() {
            Some(&p) => rt.read(p, host).as_bool(),
            None => false,
        };
        if b.locals.is_empty() || local(rt, b, 0).as_bool() {
            rt.wake_owner(s, host);
        }
        if !rt.execute_now(s, reset, host) {
            out(rt, b, 0);
            return Flow::Done;
        }
        return Flow::Continue;
    }
    if rt.is_active(s) {
        Flow::Continue
    } else {
        out(rt, b, 0);
        Flow::Done
    }
}

/// Activate Object / Deactivate Object: the object wakes in the scene (its scripts asked to activate, reset if
/// asked) or falls asleep (its scripts are not run until it wakes).
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
            let on = proto == proto::ACTIVATE_OBJECT;
            let reset = on
                && (pin(rt, b, "Reset Scripts", 3, host).as_bool()
                    || pin(rt, b, "Reset ?", 1, host).as_bool());
            rt.activate_object(o, on, reset, host);
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
                // the object's scripts go with it: the manager no longer runs them
                if let Some(x) = o {
                    rt.delete_object(x, host);
                }
            }
            Some(ObjKind::Group(gi)) => {
                // deleting a group deletes the group object only; its members stay (as the original)
                host.note(&format!("script deletes group {} (members kept)", gi));
            }
            Some(ObjKind::Behavior) => {
                if let Some(x) = o {
                    rt.delete_behavior(x, host);
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
        flags: flags::VAR_PINS,
        standing: Standing::Full,
        layouts: &[Layout {
            first_pin_not: types::BOOL,
            pins: &["Script", "Reset ?"],
        }],
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
        run: execute_script,
        event: None,
    },
    Proto {
        guid: proto::DEACTIVATE_SCRIPT,
        name: "Deactivate Script",
        library: Library::Narratives,
        pins: &["Script"],
        pouts: &[],
        flags: flags::VAR_PINS,
        standing: Standing::Full,
        layouts: &[],
        run: deactivate_script,
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
        standing: Standing::Full,
        layouts: &[],
        run: object_copy,
        event: None,
    },
];
