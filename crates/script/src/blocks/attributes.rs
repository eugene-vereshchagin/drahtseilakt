// Logics.dll's attribute blocks (Has, Set, Remove Attribute, Objects With Attribute Iterator): an object carries
// attributes, each a number (the file's attribute index, the same in the object's saved list and in a block's
// Attribute pin) with a value. Semantics: docs/gameplay_bbs.md, "Attributes".

use super::*;

/// The attribute a block's pin names: the file's attribute index; -1 (none) for a pin without a number.
fn attribute_of(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> i32 {
    match pin(rt, b, "Attribute", 0, host) {
        Value::Str(_) | Value::None => -1,
        v => v.as_int(),
    }
}

/// The target when it is a behavioural object of the graph.
fn carrier(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> Option<Id> {
    let t = rt.target(b, host)?;
    let o = rt.graph.objects.get(t)?;
    types::class_derives(o.class, 19).then_some(t)
}

pub(crate) fn has_attribute(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    let attr = attribute_of(rt, b, host);
    let value = carrier(rt, b, host).and_then(|t| rt.attribute(t, attr).cloned());
    match value {
        Some(v) => {
            out(rt, b, 0);
            if !matches!(v, Value::None) && !b.pouts.is_empty() {
                set_pout(rt, b, "Attribute Value", 0, v);
            }
        }
        None => out(rt, b, 1),
    }
    Flow::Done
}

pub(crate) fn set_attribute(
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
    let Some(t) = carrier(rt, b, host) else {
        return Flow::Done;
    };
    let attr = attribute_of(rt, b, host);
    let value = if b.pins.get(1).is_some() {
        Some(pin(rt, b, "Attribute Value", 1, host))
    } else {
        None
    };
    rt.set_attribute(t, attr, value);
    Flow::Done
}

pub(crate) fn remove_attribute(
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
    if let Some(t) = carrier(rt, b, host) {
        let attr = attribute_of(rt, b, host);
        rt.remove_attribute(t, attr);
    }
    Flow::Done
}

pub(crate) fn objects_with_attribute(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let attr = attribute_of(rt, b, host);
    let start = input(rt, b, 0);
    if !start {
        input(rt, b, 1);
    }
    // the index lives in local 0, as the original keeps it (a block without one keeps it in its scratch)
    let last = match b.locals.first() {
        Some(&l) => rt.values.get(l).map_or(0, |v| v.as_int()),
        None => match rt.scratch(id) {
            Scratch::Index(k) => i32::try_from(*k).unwrap_or(0),
            _ => 0,
        },
    };
    let idx = if start { 0 } else { last.saturating_add(1) };
    let found = usize::try_from(idx)
        .ok()
        .and_then(|i| rt.with_attribute(attr, i))
        .map(|(o, v)| (o, v.clone()));
    let Some((o, v)) = found else {
        out(rt, b, 0);
        return Flow::Done;
    };
    set_pout(rt, b, "Object", 0, Value::Obj(Some(o)));
    match b.locals.first() {
        Some(&l) => rt.write(l, Value::Int(idx)),
        None => rt.set_scratch(id, Scratch::Index(i64::from(idx))),
    }
    if !matches!(v, Value::None) && b.pouts.len() > 1 {
        set_pout(rt, b, "Attribute Value", 1, v);
    }
    out(rt, b, 1);
    Flow::Done
}

const fn decl(
    guid: Guid,
    name: &'static str,
    pins: &'static [&'static str],
    pouts: &'static [&'static str],
    flags: u32,
    run: Run,
) -> Proto {
    Proto {
        guid,
        name,
        library: Library::Logics,
        pins,
        pouts,
        flags,
        standing: Standing::Full,
        layouts: &[],
        run,
        event: None,
    }
}

pub(crate) const PROTOS: &[Proto] = &[
    decl(
        proto::HAS_ATTRIBUTE,
        "Has Attribute",
        &["Attribute"],
        &["Attribute Value"],
        flags::TARGET,
        has_attribute,
    ),
    decl(
        proto::SET_ATTRIBUTE,
        "Set Attribute",
        &["Attribute", "Attribute Value"],
        &[],
        flags::TARGET,
        set_attribute,
    ),
    decl(
        proto::REMOVE_ATTRIBUTE,
        "Remove Attribute",
        &["Attribute"],
        &[],
        flags::TARGET,
        remove_attribute,
    ),
    decl(
        proto::OBJECTS_WITH_ATTRIBUTE_ITERATOR,
        "Objects With Attribute Iterator",
        &["Attribute"],
        &["Object", "Attribute Value"],
        0,
        objects_with_attribute,
    ),
];
