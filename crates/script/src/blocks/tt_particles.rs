// TT_ParticleSystems_RT.dll: particle emitters (docs/particles.md).

use super::*;

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
        event: None,
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
        run: point_particle_system,
        event: None,
    },
];
