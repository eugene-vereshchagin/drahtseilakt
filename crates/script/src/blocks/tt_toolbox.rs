// TT_Toolbox_RT.dll: the game's own toolbox blocks (proximity, timer, initial conditions).

use super::*;

pub(crate) fn tt_scaleable_proximity(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let off = proto == proto::TT_SCALEABLE_PROXIMITY && input(rt, b, 1);
    if off {
        rt.set_scratch(id, Scratch::None);
        return Flow::Done;
    }
    let started = input(rt, b, 0);
    if started {
        rt.set_scratch(
            id,
            Scratch::Proximity {
                inside: None,
                wait: 0,
            },
        );
    }
    let Scratch::Proximity { inside, wait } = rt.scratch(id).clone() else {
        return Flow::Done;
    };
    if wait > 0 {
        rt.set_scratch(
            id,
            Scratch::Proximity {
                inside,
                wait: wait.saturating_sub(1),
            },
        );
        return Flow::Continue;
    }
    let distance = pin(rt, b, "Distance", 0, host).as_float();
    let a = pin(
        rt,
        b,
        if proto == proto::PROXIMITY {
            "Object A"
        } else {
            "ObjectA"
        },
        1,
        host,
    )
    .as_obj();
    let c = pin(
        rt,
        b,
        if proto == proto::PROXIMITY {
            "Object B"
        } else {
            "ObjectB"
        },
        2,
        host,
    )
    .as_obj();
    let pos = |rt: &Runtime, host: &dyn Host, o: Option<Id>| {
        rt.ent(o)
            .and_then(|e| host.matrix(e))
            .map(|m| [m[3][0], m[3][1], m[3][2]])
    };
    let (Some(pa), Some(pc)) = (pos(rt, host, a), pos(rt, host, c)) else {
        return Flow::Continue;
    };
    let d2 = distance_sq(pa, pc);
    set_pout(rt, b, "Current Distance", 0, Value::Float(d2.sqrt()));
    let now = d2 <= distance * distance;
    // The test runs every N ticks, N going from the minimum frame delay at the near exactness to the
    // maximum at the far one (both compared squared), as the original's.
    let mut next_wait = 0i64;
    if proto == proto::TT_SCALEABLE_PROXIMITY {
        let near = pin(rt, b, "Exactness min. Distance", 4, host).as_float();
        let far = pin(rt, b, "Exactness max. Distance", 5, host).as_float();
        let fmin = pin(rt, b, "Minimum Framedelay", 6, host).as_float();
        let fmax = pin(rt, b, "Maximum Framedelay", 7, host).as_float();
        let (n2, f2) = (near * near, far * far);
        let t = if f2 > n2 {
            ((d2 - n2) / (f2 - n2)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        next_wait = ((fmin + (fmax - fmin) * t).floor().max(1.0) as i64).saturating_sub(1);
    }
    rt.set_scratch(
        id,
        Scratch::Proximity {
            inside: Some(now),
            wait: next_wait,
        },
    );
    match (inside, now) {
        (Some(false) | None, true) => out(rt, b, 0),
        (Some(true), false) => out(rt, b, 1),
        _ => {
            if proto == proto::PROXIMITY {
                out(rt, b, 2);
            }
        }
    }
    if proto == proto::PROXIMITY {
        Flow::Done
    } else {
        Flow::Continue
    }
}

pub(crate) fn tt_timer(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    // On/Reset, Pause, Play, Off: a clock that counts the ticks' time while it plays.
    let (mut elapsed, mut running, mut on) = match rt.scratch(id) {
        Scratch::Clock { elapsed, running } => (*elapsed, *running, true),
        _ => (0.0, false, false),
    };
    if input(rt, b, 0) {
        (elapsed, running, on) = (0.0, true, true);
        out(rt, b, 0);
    } else if on && running {
        elapsed += host.delta_ms();
    }
    if input(rt, b, 1) {
        running = false;
        out(rt, b, 1);
    }
    if input(rt, b, 2) {
        running = on;
        out(rt, b, 2);
    }
    if input(rt, b, 3) {
        on = false;
        out(rt, b, 3);
    }
    if on {
        set_pout(rt, b, "Elapsed Time", 0, Value::Float(elapsed));
        rt.set_scratch(id, Scratch::Clock { elapsed, running });
        Flow::Continue
    } else {
        rt.set_scratch(id, Scratch::None);
        Flow::Done
    }
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::TT_SCALEABLE_PROXIMITY,
        name: "TT Scaleable Proximity",
        library: Library::TtToolbox,
        pins: &[
            "Distance",
            "ObjectA",
            "ObjectB",
            "",
            "Exactness min. Distance",
            "Exactness max. Distance",
            "Minimum Framedelay",
            "Maximum Framedelay",
        ],
        pouts: &["Current Distance"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: tt_scaleable_proximity,
        event: None,
    },
    Proto {
        guid: proto::TT_RESTORE_IC,
        name: "TT Restore IC",
        library: Library::TtToolbox,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: super::narratives::restore_ic,
        event: None,
    },
    Proto {
        guid: proto::TT_TIMER,
        name: "TT_Timer",
        library: Library::TtToolbox,
        pins: &[],
        pouts: &["Elapsed Time"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: tt_timer,
        event: None,
    },
];
