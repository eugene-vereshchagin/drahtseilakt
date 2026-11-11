// Logics.dll: flow (Nop, switches, sequencers, counters), timing, tests, arrays, groups, messages, progressions.

use super::*;

pub(crate) fn op(rt: &mut Runtime, _id: Id, b: &Block, _proto: Guid, host: &mut dyn Host) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    // The operation's GUID lies in the block's second and third locals (the first is a function pointer).
    let guid = Guid(
        local(rt, b, 1).as_int() as u32,
        local(rt, b, 2).as_int() as u32,
    );
    let a = pin(rt, b, "p1", 0, host);
    let c = pin(rt, b, "p2", 1, host);
    let out_guid = rt
        .pout(b, "res", 0)
        .and_then(|p| rt.graph.param(p))
        .map(|p| p.guid);
    let a_guid = rt
        .pin(b, "p1", 0)
        .and_then(|p| rt.graph.param(p))
        .map(|p| p.guid);
    let r = operation(rt, host, guid, out_guid, a_guid, &a, &c);
    set_pout(rt, b, "res", 0, r);
    out(rt, b, 0);
    Flow::Done
}

pub(crate) fn nop(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    let mut any = false;
    for i in 0..b.inputs.len() {
        any |= input(rt, b, i);
    }
    if any {
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn identity(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    input(rt, b, 0);
    for i in 0..b.pins.len() {
        let v = rt
            .pin(b, "", i)
            .map(|p| rt.read(p, host))
            .unwrap_or_default();
        if let Some(p) = b.pouts.get(i).copied() {
            rt.write(p, v);
        }
    }
    out(rt, b, 0);
    Flow::Done
}

pub(crate) fn parameter_selector(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    for i in 0..b.inputs.len() {
        if input(rt, b, i) {
            let v = b
                .pins
                .get(i)
                .copied()
                .map(|p| rt.read(p, host))
                .unwrap_or_default();
            set_pout(rt, b, "Selected", 0, v);
            out(rt, b, 0);
        }
    }
    Flow::Done
}

pub(crate) fn binary_switch(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let c = pin(rt, b, "Condition", 0, host).as_bool();
        out(rt, b, usize::from(!c));
    }
    Flow::Done
}

pub(crate) fn test(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let op = pin(rt, b, "Test", 0, host).as_int();
        let a = pin(rt, b, "A", 1, host);
        let c = pin(rt, b, "B", 2, host);
        out(rt, b, usize::from(!compare(op, &a, &c)));
    }
    Flow::Done
}

pub(crate) fn switch_on_parameter(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let t = pin(rt, b, "Test", 0, host);
        let mut hit = 0;
        for i in 1..b.pins.len() {
            let v = b
                .pins
                .get(i)
                .copied()
                .map(|p| rt.read(p, host))
                .unwrap_or_default();
            if v.same(&t) {
                hit = i;
                break;
            }
        }
        out(rt, b, hit);
    }
    Flow::Done
}

pub(crate) fn keep_active(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    // Reset stops it; In 1 keeps Out 1 firing every tick.
    if input(rt, b, 0) {
        rt.set_scratch(id, Scratch::Flag(false));
        out(rt, b, 0);
        return Flow::Done;
    }
    if input(rt, b, 1) {
        rt.set_scratch(id, Scratch::Flag(true));
    }
    if matches!(rt.scratch(id), Scratch::Flag(true)) {
        out(rt, b, 1);
        Flow::Continue
    } else {
        Flow::Done
    }
}

pub(crate) fn delayer(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let restart = input(rt, b, 0);
    let looped = input(rt, b, 1);
    let wait = pin(
        rt,
        b,
        if proto == proto::DELAYER {
            "Time to Wait"
        } else {
            "Duration"
        },
        0,
        host,
    )
    .as_float();
    let mut elapsed = match rt.scratch(id) {
        Scratch::Time(t) if !restart => *t,
        _ => 0.0,
    };
    if !restart && !looped && !matches!(rt.scratch(id), Scratch::Time(_)) {
        return Flow::Done;
    }
    if !restart {
        elapsed += host.delta_ms();
    }
    set_pout(rt, b, "Elapsed Time", 0, Value::Float(elapsed));
    set_pout(rt, b, "Delta Time", 1, Value::Float(host.delta_ms()));
    if elapsed >= wait {
        rt.set_scratch(id, Scratch::None);
        out(rt, b, 0);
        Flow::Done
    } else {
        rt.set_scratch(id, Scratch::Time(elapsed));
        if proto == proto::TIMER {
            out(rt, b, 1);
        }
        Flow::Continue
    }
}

pub(crate) fn chrono(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        rt.set_scratch(id, Scratch::Time(0.0));
        out(rt, b, 0);
    }
    if input(rt, b, 1) {
        let t = match rt.scratch(id) {
            Scratch::Time(t) => *t,
            _ => 0.0,
        };
        set_pout(rt, b, "Elapsed Time", 0, Value::Float(t));
        rt.set_scratch(id, Scratch::None);
        out(rt, b, 1);
        return Flow::Done;
    }
    match rt.scratch(id) {
        Scratch::Time(t) => {
            let t = *t + host.delta_ms();
            rt.set_scratch(id, Scratch::Time(t));
            set_pout(rt, b, "Elapsed Time", 0, Value::Float(t));
            Flow::Continue
        }
        _ => Flow::Done,
    }
}

pub(crate) fn get_delta_time(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        set_pout(rt, b, "Delta Time", 0, Value::Float(host.delta_ms()));
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn per_second(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let x = pin(rt, b, "X", 0, host).as_float();
        set_pout(rt, b, "Y", 0, Value::Float(x * host.delta_ms() / 1000.0));
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn counter(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let start = input(rt, b, 0);
    let looped = input(rt, b, 1);
    if !start && !looped {
        return Flow::Done;
    }
    let count = pin(rt, b, "Count", 0, host).as_int();
    let first = pin(rt, b, "Start Index", 1, host).as_int();
    let step = pin(rt, b, "Step", 2, host).as_int().max(1);
    let i = if start {
        0
    } else {
        match rt.scratch(id) {
            Scratch::Index(i) => *i,
            _ => 0,
        }
    };
    if i >= i64::from(count) {
        rt.set_scratch(id, Scratch::None);
        out(rt, b, 0);
    } else {
        set_pout(
            rt,
            b,
            "Value",
            0,
            Value::Int(first.saturating_add((i as i32).saturating_mul(step))),
        );
        rt.set_scratch(id, Scratch::Index(i.saturating_add(1)));
        out(rt, b, 1);
    }
    Flow::Done
}

pub(crate) fn binary_memory(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        rt.set_scratch(id, Scratch::Flag(false));
        set_pout(rt, b, "State", 0, Value::Bool(false));
        out(rt, b, 0);
    }
    if input(rt, b, 1) {
        rt.set_scratch(id, Scratch::Flag(true));
        set_pout(rt, b, "State", 0, Value::Bool(true));
        out(rt, b, 1);
    }
    Flow::Done
}

pub(crate) fn streaming_event(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    for i in 0..b.inputs.len() {
        if input(rt, b, i) {
            out(rt, b, i);
        }
    }
    Flow::Done
}

pub(crate) fn wait_message(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 1) {
        rt.set_scratch(id, Scratch::None);
        return Flow::Done;
    }
    let started = input(rt, b, 0);
    if started {
        rt.set_scratch(id, Scratch::Received(false));
    }
    if !matches!(rt.scratch(id), Scratch::Received(_)) {
        return Flow::Done;
    }
    let name = pin(rt, b, "Message", 0, host).as_string();
    if !started && rt.received(&name, b.owner) {
        rt.set_scratch(id, Scratch::None);
        out(rt, b, 0);
        return Flow::Done;
    }
    Flow::Continue
}

pub(crate) fn send_message_to_group(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let name = pin(rt, b, "Message", 0, host).as_string();
        let dest = pin(
            rt,
            b,
            if proto == proto::SEND_MESSAGE {
                "Dest"
            } else {
                "Group"
            },
            1,
            host,
        )
        .as_obj();
        let to_group = dest.and_then(|d| rt.graph.objects.get(d)).map(|o| o.kind);
        match to_group {
            Some(ObjKind::Group(g)) => {
                for m in host.group_members(g) {
                    let gid = rt
                        .graph
                        .objects
                        .iter()
                        .position(|o| o.kind == ObjKind::Entity(m));
                    rt.send(&name, gid);
                    host.message(&name, Some(Ent::Object(m)));
                }
            }
            Some(ObjKind::Entity(_)) | Some(ObjKind::Ball) => {
                rt.send(&name, dest);
                let e = rt.ent(dest);
                host.message(&name, e);
            }
            _ => {
                rt.send(&name, None);
                host.message(&name, None);
            }
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn get_cell(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    let row = pin(rt, b, "Row Index", 0, host).as_int();
    let col = pin(rt, b, "Column Index", 1, host).as_int();
    let (Ok(row), Ok(col)) = (usize::try_from(row), usize::try_from(col)) else {
        out(rt, b, 1);
        return Flow::Done;
    };
    let t = rt.target(b, host);
    let cell = match array_of(rt, t) {
        Some(a) => {
            if proto == proto::SET_CELL {
                let v = pin(rt, b, "Value", 2, host);
                let slot = rt
                    .graph
                    .arrays
                    .get(a)
                    .and_then(|arr| arr.rows.get(row))
                    .and_then(|r| r.get(col))
                    .is_some();
                if slot {
                    // the arrays live in the shared graph; a map's own table is edited in the runtime's copy
                    rt.set_cell(a, row, col, v);
                }
                slot.then_some(Value::None)
            } else {
                rt.cell(a, row, col)
            }
        }
        None => {
            let name = ent_name(rt, t);
            if proto == proto::SET_CELL {
                None
            } else {
                host.game_cell(&name, row, col)
            }
        }
    };
    match cell {
        Some(v) => {
            if proto == proto::GET_CELL {
                set_pout(rt, b, "Cell Value", 0, v);
            }
            out(rt, b, 0);
        }
        None => out(rt, b, 1),
    }
    Flow::Done
}

pub(crate) fn get_key_row(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    let t = rt.target(b, host);
    let Some(a) = array_of(rt, t) else {
        out(rt, b, 1);
        return Flow::Done;
    };
    let row = if proto == proto::GET_ROW {
        usize::try_from(pin(rt, b, "Row Index", 0, host).as_int()).ok()
    } else {
        let key = pin(rt, b, "Key Value", 0, host);
        rt.rows(a)
            .iter()
            .position(|r| r.first().is_some_and(|c| c.same(&key)))
    };
    match row.and_then(|r| rt.rows(a).get(r).cloned()) {
        Some(cells) => {
            for (i, c) in cells.into_iter().enumerate() {
                if let Some(p) = b.pouts.get(i).copied() {
                    rt.write(p, c);
                }
            }
            out(rt, b, 0);
        }
        None => out(rt, b, 1),
    }
    Flow::Done
}

pub(crate) fn add_row(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let t = rt.target(b, host);
        if let Some(a) = array_of(rt, t) {
            let mut row = Vec::new();
            for i in 0..b.pins.len() {
                row.push(
                    b.pins
                        .get(i)
                        .copied()
                        .map(|p| rt.read(p, host))
                        .unwrap_or_default(),
                );
            }
            let unique = local(rt, b, 0).as_int();
            let present = usize::try_from(unique).ok().is_some_and(|k| {
                rt.rows(a)
                    .iter()
                    .any(|r| r.get(k).zip(row.get(k)).is_some_and(|(x, y)| x.same(y)))
            });
            if present {
                out(rt, b, 1);
            } else {
                rt.add_row(a, row);
                out(rt, b, 0);
            }
        } else {
            out(rt, b, 0);
        }
    }
    Flow::Done
}

pub(crate) fn clear_array(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let t = rt.target(b, host);
        if let Some(a) = array_of(rt, t) {
            rt.clear_rows(a);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn iterator(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let start = input(rt, b, 0);
    let looped = input(rt, b, 1);
    if !start && !looped {
        return Flow::Done;
    }
    let t = rt.target(b, host);
    let rows = array_of(rt, t).map(|a| rt.rows(a).len()).unwrap_or(0) as i64;
    let first = i64::from(pin(rt, b, "Starting Row", 0, host).as_int());
    let last = i64::from(pin(rt, b, "Ending Row", 1, host).as_int());
    let last = if last < 0 {
        rows.saturating_sub(1)
    } else {
        last.min(rows.saturating_sub(1))
    };
    let backward = pin(rt, b, "Backward", 2, host).as_bool();
    let i = if start {
        if backward { last } else { first }
    } else {
        match rt.scratch(id) {
            Scratch::Index(i) => *i,
            _ => first,
        }
    };
    let done = if backward { i < first } else { i > last };
    if done || i < 0 {
        rt.set_scratch(id, Scratch::None);
        out(rt, b, 0);
        return Flow::Done;
    }
    set_pout(rt, b, "Row Index", 0, Value::Int(i as i32));
    let cells = array_of(rt, t)
        .and_then(|a| rt.rows(a).get(i as usize).cloned())
        .unwrap_or_default();
    for (k, c) in cells.into_iter().enumerate() {
        if let Some(p) = b.pouts.get(k.saturating_add(1)).copied() {
            rt.write(p, c);
        }
    }
    rt.set_scratch(
        id,
        Scratch::Index(if backward {
            i.saturating_sub(1)
        } else {
            i.saturating_add(1)
        }),
    );
    out(rt, b, 1);
    Flow::Done
}

pub(crate) fn group_iterator(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let start = input(rt, b, 0);
    let looped = input(rt, b, 1);
    if !start && !looped {
        return Flow::Done;
    }
    let g = pin(
        rt,
        b,
        if proto == proto::GROUP_ITERATOR {
            "Group"
        } else {
            "Collection"
        },
        0,
        host,
    )
    .as_obj();
    let members: Vec<Id> = match g.and_then(|x| rt.graph.objects.get(x)).map(|o| o.kind) {
        Some(ObjKind::Group(gi)) => host
            .group_members(gi)
            .into_iter()
            .filter_map(|m| {
                rt.graph
                    .objects
                    .iter()
                    .position(|o| o.kind == ObjKind::Entity(m))
            })
            .collect(),
        _ => Vec::new(),
    };
    // The index lives where the original keeps it: in output parameter 1 (Index) when the block has one, else in
    // local 0, so it survives as theirs does and other readers see it.
    let slot = b.pouts.get(1).or_else(|| b.locals.first()).copied();
    let i = if start {
        0
    } else {
        slot.and_then(|p| rt.values.get(p))
            .map_or(0, |v| v.as_int().saturating_add(1))
    };
    match usize::try_from(i)
        .ok()
        .and_then(|k| members.get(k).copied())
    {
        Some(m) => {
            set_pout(rt, b, "Element", 0, Value::Obj(Some(m)));
            if let Some(p) = slot {
                rt.write(p, Value::Int(i));
            }
            out(rt, b, 1);
        }
        None => out(rt, b, 0),
    }
    Flow::Done
}

pub(crate) fn is_in_group(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let o = pin(rt, b, "Object", 0, host).as_obj();
        let g = pin(rt, b, "Group", 1, host).as_obj();
        let e = rt.ent(o);
        let hit = match (e, g.and_then(|x| rt.graph.objects.get(x)).map(|o| o.kind)) {
            (Some(Ent::Object(i)), Some(ObjKind::Group(gi))) => host.group_members(gi).contains(&i),
            _ => false,
        };
        out(rt, b, usize::from(!hit));
    }
    Flow::Done
}

pub(crate) fn add_to_group(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let g = pin(rt, b, "Group", 0, host).as_obj();
        let t = rt.target(b, host);
        if let (Some(ObjKind::Group(gi)), Some(Ent::Object(i))) = (
            g.and_then(|x| rt.graph.objects.get(x)).map(|o| o.kind),
            rt.ent(t),
        ) {
            if proto == proto::ADD_TO_GROUP {
                host.group_add(gi, i);
            } else {
                host.group_remove(gi, i);
            }
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn group_clear(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let t = rt.target(b, host);
        if let Some(ObjKind::Group(gi)) = t.and_then(|x| rt.graph.objects.get(x)).map(|o| o.kind) {
            host.group_clear(gi);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn linear_progression(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    // In starts at A; each Loop In moves on by the tick (or by one frame when not time based) and
    // gives the value between A and B; at the end B and Out. The Bezier one's curve is a 2D curve the
    // runtime does not decode: it is taken as the straight line a new block has.
    let start = input(rt, b, 0);
    let looped = input(rt, b, 1);
    if !start && !looped {
        return Flow::Done;
    }
    let duration = pin(rt, b, "Duration", 0, host).as_float();
    let a = pin(rt, b, "A", 1, host).as_float();
    let c = pin(rt, b, "B", 2, host).as_float();
    let time_based = !matches!(local(rt, b, 0), Value::Bool(false));
    let (elapsed, last) = match rt.scratch(id) {
        Scratch::Progress { elapsed, last } if !start => (
            *elapsed + if time_based { host.delta_ms() } else { 1.0 },
            *last,
        ),
        _ => (0.0, a),
    };
    let progress = if duration > 0.0 {
        (elapsed / duration).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let value = a + (c - a) * progress;
    set_pout(rt, b, "Elapsed Time", 0, Value::Float(elapsed));
    set_pout(rt, b, "Value", 1, Value::Float(value));
    set_pout(rt, b, "Delta", 2, Value::Float(value - last));
    set_pout(rt, b, "Progression", 3, Value::Float(progress));
    if progress >= 1.0 {
        rt.set_scratch(id, Scratch::None);
        out(rt, b, 0);
    } else {
        rt.set_scratch(
            id,
            Scratch::Progress {
                elapsed,
                last: value,
            },
        );
        out(rt, b, 1);
    }
    Flow::Done
}

pub(crate) fn interpolator(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let a = pin(rt, b, "A", 0, host);
        let c = pin(rt, b, "B", 1, host);
        let t = pin(rt, b, "Value", 2, host).as_float();
        let mix = |x: f32, y: f32| x + (y - x) * t;
        let v = match (&a, &c) {
            (Value::Vec3(x), Value::Vec3(y)) => {
                Value::Vec3([mix(x[0], y[0]), mix(x[1], y[1]), mix(x[2], y[2])])
            }
            (Value::Vec2(x), Value::Vec2(y)) => Value::Vec2([mix(x[0], y[0]), mix(x[1], y[1])]),
            (Value::Color(x), Value::Color(y)) => Value::Color([
                mix(x[0], y[0]),
                mix(x[1], y[1]),
                mix(x[2], y[2]),
                mix(x[3], y[3]),
            ]),
            (Value::Int(x), Value::Int(y)) => Value::Int(mix(*x as f32, *y as f32) as i32),
            _ => Value::Float(mix(a.as_float(), c.as_float())),
        };
        set_pout(rt, b, "C", 0, v);
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn get_component(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let parts: Vec<f32> = match pin(rt, b, "Variable", 0, host) {
            Value::Vec3(v) => v.to_vec(),
            Value::Vec2(v) => v.to_vec(),
            Value::Color(v) => v.to_vec(),
            v @ Value::Raw(_) => v.as_vec3().to_vec(),
            v => vec![v.as_float()],
        };
        for (i, x) in parts.into_iter().enumerate() {
            if let Some(p) = b.pouts.get(i).copied() {
                rt.write(p, Value::Float(x));
            }
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn set_component(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let mut c = [0.0f32; 4];
        for (i, slot) in c.iter_mut().enumerate() {
            if let Some(p) = b.pins.get(i).copied() {
                *slot = rt.read(p, host).as_float();
            }
        }
        let kind = rt
            .pout(b, "Variable", 0)
            .and_then(|p| rt.graph.param(p))
            .map(|p| p.guid);
        let v = match kind {
            Some(types::VECTOR2) => Value::Vec2([c[0], c[1]]),
            Some(types::COLOR) => Value::Color(c),
            _ if b.pins.len() == 2 => Value::Vec2([c[0], c[1]]),
            _ if b.pins.len() >= 4 => Value::Color(c),
            _ => Value::Vec3([c[0], c[1], c[2]]),
        };
        set_pout(rt, b, "Variable", 0, v);
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn random(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let lo = pin(rt, b, "Min", 0, host);
        let hi = pin(rt, b, "Max", 1, host);
        let v = match (&lo, &hi) {
            (Value::Vec3(x), Value::Vec3(y)) => {
                let mut r = [0.0f32; 3];
                for (k, slot) in r.iter_mut().enumerate() {
                    let (x, y) = (
                        x.get(k).copied().unwrap_or(0.0),
                        y.get(k).copied().unwrap_or(0.0),
                    );
                    *slot = x + (y - x) * rt.random();
                }
                Value::Vec3(r)
            }
            (Value::Int(x), Value::Int(y)) => {
                let (x, y) = (*x as f32, *y as f32);
                Value::Int((x + (y - x) * rt.random()) as i32)
            }
            _ => {
                let (x, y) = (lo.as_float(), hi.as_float());
                Value::Float(x + (y - x) * rt.random())
            }
        };
        set_pout(rt, b, "Rand", 0, v);
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn random_switch(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        // one output by its coefficient's share of the sum
        let mut coefs = Vec::new();
        for i in 0..b.outputs.len().min(64) {
            let c = b
                .pins
                .get(i)
                .copied()
                .map_or(1.0, |p| rt.read(p, host).as_float());
            coefs.push(if c.is_finite() { c.max(0.0) } else { 0.0 });
        }
        let sum: f32 = coefs.iter().sum();
        let mut left = rt.random() * sum;
        let mut hit = 0usize;
        for (i, c) in coefs.iter().enumerate() {
            hit = i;
            if left < *c {
                break;
            }
            left -= *c;
        }
        out(rt, b, hit);
    }
    Flow::Done
}

pub(crate) fn remove_row(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let t = rt.target(b, host);
        let row = usize::try_from(pin(rt, b, "Row Index", 0, host).as_int()).ok();
        let removed = match (array_of(rt, t), row) {
            (Some(a), Some(r)) => rt.remove_row(a, r),
            _ => false,
        };
        out(rt, b, usize::from(!removed));
    }
    Flow::Done
}

pub(crate) fn row_search(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let t = rt.target(b, host);
        let col = usize::try_from(pin(rt, b, "Column", 0, host).as_int()).ok();
        let first = usize::try_from(pin(rt, b, "First Search Row", 1, host).as_int()).unwrap_or(0);
        let op = pin(rt, b, "Operation", 2, host).as_int();
        let reference = pin(rt, b, "Reference Value", 3, host);
        let found = match (array_of(rt, t), col) {
            (Some(a), Some(col)) => rt
                .rows(a)
                .iter()
                .enumerate()
                .skip(first)
                .find(|(_, r)| r.get(col).is_some_and(|c| compare(op, c, &reference)))
                .map(|(i, _)| i),
            _ => None,
        };
        match found {
            Some(i) => {
                set_pout(rt, b, "First Row Found", 0, Value::Int(i as i32));
                out(rt, b, 0);
            }
            None => out(rt, b, 1),
        }
    }
    Flow::Done
}

pub(crate) fn get_nearest_in_group(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let g = pin(rt, b, "Group", 0, host).as_obj();
        let p = pin(rt, b, "Position", 1, host).as_vec3();
        let at = match referential(rt, b, "Referential", 2, host) {
            Some(rm) => transform_point(&rm, p),
            None => p,
        };
        let mut best: Option<(usize, f32)> = None;
        if let Some(ObjKind::Group(gi)) = g.and_then(|x| rt.graph.objects.get(x)).map(|o| o.kind) {
            for m in host.group_members(gi) {
                if let Some(mm) = host.matrix(Ent::Object(m)) {
                    let d = distance_sq([mm[3][0], mm[3][1], mm[3][2]], at);
                    if best.is_none_or(|(_, bd)| d < bd) {
                        best = Some((m, d));
                    }
                }
            }
        }
        let nearest = best.and_then(|(m, _)| {
            rt.graph
                .objects
                .iter()
                .position(|o| o.kind == ObjKind::Entity(m))
        });
        set_pout(rt, b, "Nearest Object", 0, Value::Obj(nearest));
        if let Some((_, d)) = best {
            set_pout(rt, b, "Distance", 1, Value::Float(d.sqrt()));
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::OP,
        name: "Op",
        library: Library::Logics,
        pins: &["p1", "p2"],
        pouts: &["res"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: op,
        event: None,
    },
    Proto {
        guid: proto::NOP,
        name: "Nop",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: flags::VAR_INPUTS,
        standing: Standing::Full,
        layouts: &[],
        run: nop,
        event: None,
    },
    Proto {
        guid: proto::WAIT_FOR_ALL,
        name: "Wait For All",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: flags::VAR_INPUTS,
        standing: Standing::Full,
        layouts: &[],
        run: nop,
        event: None,
    },
    Proto {
        guid: proto::IDENTITY,
        name: "Identity",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: flags::VAR_PINS | flags::VAR_POUTS,
        standing: Standing::Full,
        layouts: &[],
        run: identity,
        event: None,
    },
    Proto {
        guid: proto::PARAMETER_SELECTOR,
        name: "Parameter Selector",
        library: Library::Logics,
        pins: &[],
        pouts: &["Selected"],
        flags: flags::VAR_INPUTS,
        standing: Standing::Full,
        layouts: &[],
        run: parameter_selector,
        event: None,
    },
    Proto {
        guid: proto::BINARY_SWITCH,
        name: "Binary Switch",
        library: Library::Logics,
        pins: &["Condition"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: binary_switch,
        event: None,
    },
    Proto {
        guid: proto::TEST,
        name: "Test",
        library: Library::Logics,
        pins: &["Test", "A", "B"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: test,
        event: None,
    },
    Proto {
        guid: proto::SWITCH_ON_PARAMETER,
        name: "Switch On Parameter",
        library: Library::Logics,
        pins: &["Test"],
        pouts: &[],
        flags: flags::VAR_PINS,
        standing: Standing::Full,
        layouts: &[],
        run: switch_on_parameter,
        event: None,
    },
    Proto {
        guid: proto::KEEP_ACTIVE,
        name: "Keep Active",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: keep_active,
        event: None,
    },
    Proto {
        guid: proto::DELAYER,
        name: "Delayer",
        library: Library::Logics,
        pins: &["Time to Wait"],
        pouts: &["Elapsed Time", "Delta Time"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: delayer,
        event: None,
    },
    Proto {
        guid: proto::TIMER,
        name: "Timer",
        library: Library::Logics,
        pins: &["Duration"],
        pouts: &["Elapsed Time", "Delta Time"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: delayer,
        event: None,
    },
    Proto {
        guid: proto::CHRONO,
        name: "Chrono",
        library: Library::Logics,
        pins: &[],
        pouts: &["Elapsed Time"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: chrono,
        event: None,
    },
    Proto {
        guid: proto::GET_DELTA_TIME,
        name: "Get Delta Time",
        library: Library::Logics,
        pins: &[],
        pouts: &["Delta Time"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: get_delta_time,
        event: None,
    },
    Proto {
        guid: proto::PER_SECOND,
        name: "Per Second",
        library: Library::Logics,
        pins: &["X"],
        pouts: &["Y"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: per_second,
        event: None,
    },
    Proto {
        guid: proto::COUNTER,
        name: "Counter",
        library: Library::Logics,
        pins: &["Count", "Start Index", "Step"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: counter,
        event: None,
    },
    Proto {
        guid: proto::BINARY_MEMORY,
        name: "Binary Memory",
        library: Library::Logics,
        pins: &[],
        pouts: &["State"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: binary_memory,
        event: None,
    },
    Proto {
        guid: proto::STREAMING_EVENT,
        name: "Streaming Event",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: flags::VAR_INPUTS,
        standing: Standing::Full,
        layouts: &[],
        run: streaming_event,
        event: None,
    },
    Proto {
        guid: proto::SEQUENCER,
        name: "Sequencer",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: flags::VAR_INPUTS,
        standing: Standing::Full,
        layouts: &[],
        run: streaming_event,
        event: None,
    },
    Proto {
        guid: proto::WAIT_MESSAGE,
        name: "Wait Message",
        library: Library::Logics,
        pins: &["Message"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: wait_message,
        event: None,
    },
    Proto {
        guid: proto::SEND_MESSAGE_TO_GROUP,
        name: "Send Message To Group",
        library: Library::Logics,
        pins: &["Message", "Group"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: send_message_to_group,
        event: None,
    },
    Proto {
        guid: proto::SEND_MESSAGE,
        name: "Send Message",
        library: Library::Logics,
        pins: &["Message", "Dest"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: send_message_to_group,
        event: None,
    },
    Proto {
        guid: proto::GET_CELL,
        name: "Get Cell",
        library: Library::Logics,
        pins: &["Row Index", "Column Index"],
        pouts: &["Cell Value"],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: get_cell,
        event: None,
    },
    Proto {
        guid: proto::SET_CELL,
        name: "Set Cell",
        library: Library::Logics,
        pins: &["Row Index", "Column Index", "Value"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: get_cell,
        event: None,
    },
    Proto {
        guid: proto::GET_KEY_ROW,
        name: "Get Key Row",
        library: Library::Logics,
        pins: &["Key Value"],
        pouts: &[],
        flags: flags::TARGET | flags::VAR_POUTS,
        standing: Standing::Full,
        layouts: &[],
        run: get_key_row,
        event: None,
    },
    Proto {
        guid: proto::GET_ROW,
        name: "Get Row",
        library: Library::Logics,
        pins: &["Row Index"],
        pouts: &[],
        flags: flags::TARGET | flags::VAR_POUTS,
        standing: Standing::Full,
        layouts: &[],
        run: get_key_row,
        event: None,
    },
    Proto {
        guid: proto::ADD_ROW,
        name: "Add Row",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET | flags::VAR_PINS,
        standing: Standing::Full,
        layouts: &[],
        run: add_row,
        event: None,
    },
    Proto {
        guid: proto::CLEAR_ARRAY,
        name: "Clear Array",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: clear_array,
        event: None,
    },
    Proto {
        guid: proto::ITERATOR,
        name: "Iterator",
        library: Library::Logics,
        pins: &["Starting Row", "Ending Row", "Backward"],
        pouts: &["Row Index"],
        flags: flags::TARGET | flags::VAR_POUTS,
        standing: Standing::Full,
        layouts: &[],
        run: iterator,
        event: None,
    },
    Proto {
        guid: proto::GROUP_ITERATOR,
        name: "Group Iterator",
        library: Library::Logics,
        pins: &["Group"],
        pouts: &["Element", "Index"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: group_iterator,
        event: None,
    },
    Proto {
        guid: proto::COLLECTION_ITERATOR,
        name: "Collection Iterator",
        library: Library::Logics,
        pins: &["Collection"],
        pouts: &["Element", "Index"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: group_iterator,
        event: None,
    },
    Proto {
        guid: proto::IS_IN_GROUP,
        name: "Is In Group",
        library: Library::Logics,
        pins: &["Object", "Group"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: is_in_group,
        event: None,
    },
    Proto {
        guid: proto::PROXIMITY,
        name: "Proximity",
        library: Library::Logics,
        pins: &["Distance", "Object A", "Object B"],
        pouts: &["Current Distance"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: super::tt_toolbox::tt_scaleable_proximity,
        event: None,
    },
    Proto {
        guid: proto::ADD_TO_GROUP,
        name: "Add To Group",
        library: Library::Logics,
        pins: &["Group"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: add_to_group,
        event: None,
    },
    Proto {
        guid: proto::REMOVE_FROM_GROUP,
        name: "Remove From Group",
        library: Library::Logics,
        pins: &["Group"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: add_to_group,
        event: None,
    },
    Proto {
        guid: proto::GROUP_CLEAR,
        name: "Group Clear",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: group_clear,
        event: None,
    },
    Proto {
        guid: proto::LINEAR_PROGRESSION,
        name: "Linear Progression",
        library: Library::Logics,
        pins: &["Duration", "A", "B"],
        pouts: &["Elapsed Time", "Value", "Delta", "Progression"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: linear_progression,
        event: None,
    },
    Proto {
        guid: proto::BEZIER_PROGRESSION,
        name: "Bezier Progression",
        library: Library::Logics,
        pins: &["Duration", "A", "B"],
        pouts: &["Elapsed Time", "Value", "Delta", "Progression"],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: linear_progression,
        event: None,
    },
    Proto {
        guid: proto::INTERPOLATOR,
        name: "Interpolator",
        library: Library::Logics,
        pins: &["A", "B", "Value"],
        pouts: &["C"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: interpolator,
        event: None,
    },
    Proto {
        guid: proto::GET_COMPONENT,
        name: "Get Component",
        library: Library::Logics,
        pins: &["Variable"],
        pouts: &[],
        flags: flags::VAR_POUTS,
        standing: Standing::Full,
        layouts: &[],
        run: get_component,
        event: None,
    },
    Proto {
        guid: proto::SET_COMPONENT,
        name: "Set Component",
        library: Library::Logics,
        pins: &[],
        pouts: &["Variable"],
        flags: flags::VAR_PINS,
        standing: Standing::Full,
        layouts: &[],
        run: set_component,
        event: None,
    },
    Proto {
        guid: proto::RANDOM,
        name: "Random",
        library: Library::Logics,
        pins: &["Min", "Max"],
        pouts: &["Rand"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: random,
        event: None,
    },
    Proto {
        guid: proto::RANDOM_SWITCH,
        name: "Random Switch",
        library: Library::Logics,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: random_switch,
        event: None,
    },
    Proto {
        guid: proto::REMOVE_ROW,
        name: "Remove Row",
        library: Library::Logics,
        pins: &["Row Index"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: remove_row,
        event: None,
    },
    Proto {
        guid: proto::ROW_SEARCH,
        name: "Row Search",
        library: Library::Logics,
        pins: &["Column", "First Search Row", "Operation", "Reference Value"],
        pouts: &["First Row Found"],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: row_search,
        event: None,
    },
    Proto {
        guid: proto::GET_NEAREST_IN_GROUP,
        name: "Get Nearest In Group",
        library: Library::Logics,
        pins: &["Group", "Position", "Referential"],
        pouts: &["Nearest Object", "Distance"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: get_nearest_in_group,
        event: None,
    },
];
