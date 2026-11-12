// The scene's look: Set Light Color (Lights.dll), Set Fog (WorldEnvironments.dll), TT Sky (TT_Gravity_RT.dll), and the
// 2D entity and pickable blocks. Every change goes to the host for this run only; the simulation never reads it.

use super::*;
use crate::exec::{Ent2d, EnvChange, FogSpec, MAX_ENTS_2D, SkySpec};

pub(crate) const SET_LIGHT_COLOR: Guid = Guid(0x3211_5590, 0x7895_1230);
pub(crate) const SET_FOG: Guid = Guid(0x151a_aaaa, 0xffd5_bdbc);
pub(crate) const TT_SKY: Guid = Guid(0x3669_1920, 0x3b26_1630);
pub(crate) const EDIT_2D_ENTITY: Guid = Guid(0x7fc5_4c91, 0x3f48_6670);
pub(crate) const SET_PICKABLE: Guid = Guid(0xf149_9052, 0xebe9_bbf1);

fn finite4(c: [f32; 4]) -> [f32; 4] {
    c.map(|x| {
        if x.is_finite() {
            x.clamp(0.0, 1.0)
        } else {
            0.0
        }
    })
}

/// Set Light Color: the colour of the target light (pin Color), by the light's name.
pub(crate) fn set_light_color(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let color = finite4(pin(rt, b, "Color", 0, host).as_vec4());
        let light = rt
            .target(b, host)
            .and_then(|t| rt.graph.objects.get(t))
            .map(|o| o.name.clone());
        if let Some(light) = light {
            host.env_set(EnvChange::LightColor { light, color });
        }
        out(rt, b, 0);
    }
    Flow::Done
}

/// Set Fog: pins Fog Mode (0 none, 1 exponential, 2 squared exponential, 3 linear), Fog Start, Fog End, Fog
/// Density, Fog Color. A mode that is not 1 to 3 switches the fog off; a linear fog needs an end past its start.
pub(crate) fn set_fog(rt: &mut Runtime, _id: Id, b: &Block, _p: Guid, host: &mut dyn Host) -> Flow {
    if input(rt, b, 0) {
        let mode = pin(rt, b, "Fog Mode", 0, host).as_int();
        let start = pin(rt, b, "Fog Start", 1, host).as_float();
        let end = pin(rt, b, "Fog End", 2, host).as_float();
        let density = pin(rt, b, "Fog Density", 3, host).as_float();
        let color = finite4(pin(rt, b, "Fog Color", 4, host).as_vec4());
        let ok = start.is_finite() && end.is_finite() && density.is_finite();
        let spec = match u8::try_from(mode) {
            Ok(m @ 1..=3) if ok && (m != 3 || end > start) && density >= 0.0 => Some(FogSpec {
                mode: m,
                start,
                end,
                density,
                color,
            }),
            _ => None,
        };
        host.env_set(EnvChange::Fog(spec));
        out(rt, b, 0);
    }
    Flow::Done
}

/// TT Sky: while on, a sky of Side Materials walls (local 0) around the camera; the materials are the pins after
/// the seven fixed ones (the sides, then the top's when local 1, then the bottom's when local 2). Off takes it away.
pub(crate) fn tt_sky(rt: &mut Runtime, _id: Id, b: &Block, _p: Guid, host: &mut dyn Host) -> Flow {
    if input(rt, b, 1) {
        host.env_set(EnvChange::Sky(None));
        out(rt, b, 1);
        return Flow::Done;
    }
    if input(rt, b, 0) {
        out(rt, b, 0);
    }
    let sides = usize::try_from(local(rt, b, 0).as_int())
        .unwrap_or(4)
        .clamp(4, 16);
    let top = local(rt, b, 1).as_bool();
    let bottom = local(rt, b, 2).as_bool();
    let mat = |rt: &mut Runtime, i: usize, host: &mut dyn Host| {
        let p = b.pins.get(i).copied()?;
        rt.read(p, host).as_obj()
    };
    let walls = (0..sides).map(|k| mat(rt, 7 + k, host)).collect();
    let bottom_at = 7 + sides + usize::from(top);
    let bottom = if bottom {
        mat(rt, bottom_at, host)
    } else {
        None
    };
    let spec = SkySpec {
        color: finite4(pin(rt, b, "Vertex Color", 1, host).as_vec4()),
        radius: pin(rt, b, "Radius", 3, host).as_float(),
        distortion: pin(rt, b, "Distortion", 0, host).as_float().clamp(0.0, 1.0),
        sides: walls,
        bottom,
    };
    host.env_set(EnvChange::Sky(Some(spec)));
    Flow::Continue
}

/// The value of the block's pin of exactly this name (no index fallback: Edit 2D Entity's pins are the ones its
/// author ticked).
fn named(rt: &mut Runtime, b: &Block, name: &str, host: &mut dyn Host) -> Option<Value> {
    let p = b.pins.iter().copied().find(|&p| {
        rt.graph
            .param(p)
            .is_some_and(|q| q.name.eq_ignore_ascii_case(name))
    })?;
    Some(rt.read(p, host))
}

fn finite2(v: [f32; 2]) -> Option<[f32; 2]> {
    (v[0].is_finite() && v[1].is_finite()).then_some(v)
}

/// Edit 2D Entity: the ticked parts of the target 2D entity, by the names of the pins the block carries (Position
/// X and Y, Size X and Y, Rotation). The run keeps them (at most `MAX_ENTS_2D` entities) and a 2D Text on the
/// entity lays itself out inside them; the engine has no picture of the entity itself.
pub(crate) fn edit_2d_entity(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let t = rt.target(b, host);
        let pair = |rt: &mut Runtime, x: &str, y: &str, host: &mut dyn Host| {
            let (vx, vy) = (named(rt, b, x, host), named(rt, b, y, host));
            (vx.is_some() || vy.is_some()).then(|| {
                [
                    vx.map_or(f32::NAN, |v| v.as_float()),
                    vy.map_or(f32::NAN, |v| v.as_float()),
                ]
            })
        };
        let pos = pair(rt, "Position X", "Position Y", host);
        let size = pair(rt, "Size X", "Size Y", host);
        let rotation = named(rt, b, "Rotation", host).map(|v| v.as_float());
        if let Some(t) = t {
            let at = rt.ents2d.iter().position(|(k, _)| *k == t);
            let slot = match at {
                Some(i) => rt.ents2d.get_mut(i).map(|(_, e)| e),
                None if rt.ents2d.len() < MAX_ENTS_2D => {
                    rt.ents2d.push((t, Ent2d::default()));
                    rt.ents2d.last_mut().map(|(_, e)| e)
                }
                None => None,
            };
            if let Some(e) = slot {
                // a half-given pair keeps the other half
                let merge = |old: Option<[f32; 2]>, new: Option<[f32; 2]>| {
                    new.map(|n| {
                        let o = old.unwrap_or([0.0; 2]);
                        [
                            if n[0].is_finite() { n[0] } else { o[0] },
                            if n[1].is_finite() { n[1] } else { o[1] },
                        ]
                    })
                    .and_then(finite2)
                    .or(old)
                };
                e.pos = merge(e.pos, pos);
                e.size = merge(e.size, size);
                if let Some(r) = rotation.filter(|r| r.is_finite()) {
                    e.rotation = Some(r);
                }
            }
        }
        out(rt, b, 0);
    }
    Flow::Done
}

/// Set Pickable: whether a render object can be picked by the mouse; the player has no picking, so it only passes on.
pub(crate) fn set_pickable(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    _host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: SET_LIGHT_COLOR,
        name: "Set Light Color",
        library: Library::Lights,
        pins: &["Color"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_light_color,
        event: None,
    },
    Proto {
        guid: SET_FOG,
        name: "Set Fog",
        library: Library::WorldEnvironments,
        pins: &[
            "Fog Mode",
            "Fog Start",
            "Fog End",
            "Fog Density",
            "Fog Color",
        ],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: set_fog,
        event: None,
    },
    Proto {
        guid: TT_SKY,
        name: "TT Sky",
        library: Library::TtGravity,
        pins: &[
            "Distortion",
            "Vertex Color",
            "Orientation Object",
            "Radius",
            "Quadratic SideFaces?",
            "or SideFace-Heigth",
            "Y-Position of Sky",
        ],
        pouts: &[],
        flags: flags::VAR_PINS,
        standing: Standing::Partial,
        layouts: &[],
        run: tt_sky,
        event: None,
    },
    Proto {
        guid: EDIT_2D_ENTITY,
        name: "Edit 2D Entity",
        library: Library::Interface,
        pins: &[],
        pouts: &[],
        flags: flags::TARGET | flags::VAR_PINS,
        standing: Standing::Partial,
        layouts: &[],
        run: edit_2d_entity,
        event: None,
    },
    Proto {
        guid: SET_PICKABLE,
        name: "Set Pickable",
        library: Library::Visuals,
        pins: &["Pickable"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: set_pickable,
        event: None,
    },
];
