// TT_ParticleSystems_RT.dll: particle emitters (docs/particles.md).

use super::*;
use crate::{EmitShape, EmitState, EmitterSpec};

pub(crate) fn point_particle_system(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let on = input(rt, b, 0);
    let off = input(rt, b, 1);
    let freeze = input(rt, b, 2);
    let e = rt.ent(b.owner);
    if on {
        if let Some(e) = e {
            host.particles(e, true);
        }
        rt.set_scratch(id, Scratch::Flag(true));
        out(rt, b, 0);
    }
    if off || freeze {
        if let Some(e) = e {
            host.particles(e, false);
        }
        rt.set_scratch(id, Scratch::None);
        out(rt, b, if off { 1 } else { 2 });
        return Flow::Done;
    }
    if matches!(rt.scratch(id), Scratch::Flag(true)) {
        Flow::Continue
    } else {
        Flow::Done
    }
}

/// The emitter's callback (docs/particles.md, section 8): deactivating the owning script takes the particles off
/// at once (the render callback is unregistered), activating it again brings back an emitter that was on; a
/// reset kills the particles.
pub(crate) fn particle_event(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    e: BlockEvent,
    host: &mut dyn Host,
) {
    let on = matches!(rt.scratch(id), Scratch::Flag(true));
    let Some(ent) = rt.ent(b.owner) else {
        return;
    };
    match e {
        BlockEvent::DeactivateScript | BlockEvent::Delete if on => host.particles(ent, false),
        BlockEvent::ActivateScript if on => host.particles(ent, true),
        _ => {}
    }
}

/// The planar and spherical emitters' pins and settings (docs/particles.md sections 2 and 11).
fn read_spec(rt: &mut Runtime, b: &Block, shape: EmitShape, host: &mut dyn Host) -> EmitterSpec {
    let f = |rt: &mut Runtime, n: &str, i: usize, host: &mut dyn Host| {
        pin(rt, b, n, i, host).as_float()
    };
    let n = |rt: &mut Runtime, name: &str, i: usize, host: &mut dyn Host| {
        pin(rt, b, name, i, host).as_int()
    };
    let color = |rt: &mut Runtime, i: usize, host: &mut dyn Host| match pin(rt, b, "Color", i, host)
    {
        Value::Color(c) => c,
        _ => [1.0; 4],
    };
    let texture = match pin(rt, b, "Texture", 27, host) {
        Value::Obj(Some(t)) => rt.graph.objects.get(t).map(|o| o.name.clone()),
        _ => None,
    };
    let pool = local(rt, b, 3).as_int();
    let max = n(rt, "Maximum Number", 10, host);
    EmitterSpec {
        shape,
        render: local(rt, b, 4).as_int(),
        src_blend: local(rt, b, 5).as_int(),
        dst_blend: local(rt, b, 6).as_int(),
        evolutions: local(rt, b, 8).as_int() as u32,
        variances: local(rt, b, 9).as_int() as u32,
        delay: f(rt, "Emission Delay", 0, host),
        yaw_var: f(rt, "Yaw Variance", 2, host),
        pitch_var: f(rt, "Pitch Variance", 3, host),
        speed: f(rt, "Speed", 4, host),
        speed_var: f(rt, "Speed Variance", 5, host),
        life: f(rt, "Lifespan", 8, host),
        life_var: f(rt, "Lifespan Variance", 9, host),
        max: if pool > 0 { max.min(pool) } else { max },
        emission: n(rt, "Emission", 11, host),
        emission_var: n(rt, "Emission Variance", 12, host),
        size0: f(rt, "Initial Size", 13, host),
        size0_var: f(rt, "Initial Size Variance", 14, host),
        size1: f(rt, "Ending Size", 15, host),
        size1_var: f(rt, "Ending Size Variance", 16, host),
        color0: color(rt, 23, host),
        color1: color(rt, 25, host),
        texture,
    }
}

fn shaped(rt: &mut Runtime, id: Id, b: &Block, shape: EmitShape, host: &mut dyn Host) -> Flow {
    let (on, off, freeze) = (input(rt, b, 0), input(rt, b, 1), input(rt, b, 2));
    // the state is kept in the scratch: 1 emitting, 2 frozen (bit flags, as the block's Activity local)
    let mut act = match rt.scratch(id) {
        Scratch::Index(a) => *a,
        _ => 0,
    };
    if freeze {
        act = if act & 2 != 0 {
            (act & !2) | 1
        } else {
            (act | 2) & !1
        };
        out(rt, b, 2);
    } else if on {
        act |= 1;
        out(rt, b, 0);
    } else if off {
        act = 0;
        out(rt, b, 1);
    }
    rt.set_scratch(id, Scratch::Index(act));
    let state = if act & 2 != 0 {
        EmitState::Frozen
    } else if act & 1 != 0 {
        EmitState::On
    } else {
        EmitState::Off
    };
    if let Some(e) = rt.ent(b.owner) {
        let spec = read_spec(rt, b, shape, host);
        host.emitter(id, e, &spec, state);
    }
    // off: nothing more to run for the block (the host lets live particles finish). Frozen stays active, where the
    // original's goes inactive until the next input: the host call repeats the same state, which is the same thing.
    if state != EmitState::Off {
        Flow::Continue
    } else {
        Flow::Done
    }
}

pub(crate) fn planar_particle_system(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    shaped(rt, id, b, EmitShape::Planar, host)
}

pub(crate) fn spherical_particle_system(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    shaped(rt, id, b, EmitShape::Spherical, host)
}

/// The shaped emitters' callback: deactivating or deleting the script takes the particles off at once; activating
/// it again brings back an emitter that was on; a reset forgets the state.
pub(crate) fn shaped_event(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    e: BlockEvent,
    host: &mut dyn Host,
) {
    if matches!(
        e,
        BlockEvent::DeactivateScript | BlockEvent::Delete | BlockEvent::Reset
    ) {
        if matches!(e, BlockEvent::Reset) {
            rt.set_scratch(id, Scratch::None);
        }
        if let Some(ent) = rt.ent(b.owner) {
            host.emitter(id, ent, &EmitterSpec::default(), EmitState::Gone);
        }
    }
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::POINT_PARTICLE_SYSTEM,
        name: "Point Particle System",
        library: Library::TtParticleSystems,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: point_particle_system,
        event: Some(particle_event),
    },
    Proto {
        guid: proto::PLANAR_PARTICLE_SYSTEM,
        name: "PlanarParticleSystem",
        library: Library::TtParticleSystems,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: planar_particle_system,
        event: Some(shaped_event),
    },
    Proto {
        guid: proto::SPHERICAL_PARTICLE_SYSTEM,
        name: "SphericalParticleSystem",
        library: Library::TtParticleSystems,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: spherical_particle_system,
        event: Some(shaped_event),
    },
];
