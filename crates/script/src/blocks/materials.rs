// Materials.dll: materials and textures.

use super::*;
use crate::exec::{MatChange, TexChange, TexPaint};

/// Set 3DSprite Mode: the target 3D sprite's orientation mode (pin Mode: 0 faces the view, 1 and 2 turn about the world
/// x and y, 3 keeps the sprite's own axes; anything else is ignored).
pub(crate) fn set_3dsprite_mode(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let mode = pin(rt, b, "Mode", 0, host).as_int();
        if let (Some(e), Ok(m @ 0..=3)) = (target_ent(rt, b, host), u32::try_from(mode)) {
            host.sprite_mode(e, m);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn change_texture_video_format(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        out(rt, b, 0);
    }
    Flow::Done
}

/// The level material the block's target is (the script's owner when the block has no target of its own).
fn target_material(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> Option<usize> {
    let t = rt.target(b, host)?;
    match rt.graph.objects.get(t)?.kind {
        ObjKind::Material(m) => Some(m),
        _ => None,
    }
}

fn target_texture(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> Option<usize> {
    let t = rt.target(b, host)?;
    texture_of(rt, Some(t))
}

fn texture_of(rt: &Runtime, id: Option<Id>) -> Option<usize> {
    match rt.graph.objects.get(id?)?.kind {
        ObjKind::Texture(t) => Some(t),
        _ => None,
    }
}

fn material_of(rt: &Runtime, id: Option<Id>) -> Option<usize> {
    match rt.graph.objects.get(id?)?.kind {
        ObjKind::Material(m) => Some(m),
        _ => None,
    }
}

/// A saved enum or count as a small number (blend modes, compare functions): 0 when out of range.
fn small(v: &Value) -> u8 {
    u8::try_from(v.as_int()).unwrap_or(0)
}

fn rgb(v: &Value) -> [f32; 3] {
    let c = v.as_vec4();
    [c[0], c[1], c[2]]
}

/// Runs a block that changes its target material by one `MatChange`, built from the block's pins.
fn change_material(
    rt: &mut Runtime,
    b: &Block,
    host: &mut dyn Host,
    make: impl FnOnce(&mut Runtime, &mut dyn Host) -> MatChange,
) -> Flow {
    if input(rt, b, 0) {
        if let Some(m) = target_material(rt, b, host) {
            let change = make(rt, host);
            host.material_set(m, change);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn set_diffuse(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    change_material(rt, b, host, |rt, host| MatChange::Diffuse {
        color: pin(rt, b, "Diffuse Color", 0, host).as_vec4(),
        keep_alpha: pin(rt, b, "Keep Alpha Component", 1, host).as_bool(),
    })
}

pub(crate) fn set_emissive(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    change_material(rt, b, host, |rt, host| {
        MatChange::Emissive(rgb(&pin(rt, b, "Emissive Color", 0, host)))
    })
}

pub(crate) fn set_ambient(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    change_material(rt, b, host, |rt, host| {
        MatChange::Ambient(rgb(&pin(rt, b, "Ambient Color", 0, host)))
    })
}

pub(crate) fn set_specular(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    change_material(rt, b, host, |rt, host| MatChange::Specular {
        color: rgb(&pin(rt, b, "Specular Color", 0, host)),
        power: pin(rt, b, "Specular Power", 1, host).as_float(),
    })
}

/// Set Texture: the material's texture (a texture that is not one of the level's clears it). Perspective Correct and
/// Address Mode are accepted; the renderer always wraps and corrects.
pub(crate) fn set_texture(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    change_material(rt, b, host, |rt, host| {
        let t = pin(rt, b, "Texture", 0, host).as_obj();
        MatChange::Texture(texture_of(rt, t))
    })
}

pub(crate) fn set_blend_modes(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    change_material(rt, b, host, |rt, host| MatChange::Blend {
        on: pin(rt, b, "Blend", 0, host).as_bool(),
        src: small(&pin(rt, b, "Source Blend", 1, host)),
        dst: small(&pin(rt, b, "Destination Blend", 2, host)),
    })
}

pub(crate) fn set_z_buffer(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    change_material(rt, b, host, |rt, host| MatChange::ZBuffer {
        write: pin(rt, b, "Z Buffer Write", 0, host).as_bool(),
        func: small(&pin(rt, b, "Z Compare Function", 1, host)),
    })
}

pub(crate) fn set_alpha_test(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    change_material(rt, b, host, |rt, host| MatChange::AlphaTest {
        on: pin(rt, b, "Alpha Test Enable", 0, host).as_bool(),
        func: small(&pin(rt, b, "Alpha Test Function", 1, host)),
        reference: u8::try_from(
            pin(rt, b, "Alpha Reference Value", 2, host)
                .as_int()
                .clamp(0, 255),
        )
        .unwrap_or(0),
    })
}

/// Set Material: the target object's material "Material to replace" (all of its mesh's materials when that pin is
/// empty) takes the state of the other material. The engine's materials are shared by every object that uses them,
/// so the change is level-wide, not for this object alone.
pub(crate) fn set_material(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let to_obj = pin(rt, b, "Material", 0, host).as_obj();
        let from_obj = pin(rt, b, "Material to replace", 1, host).as_obj();
        let (to, from) = (material_of(rt, to_obj), material_of(rt, from_obj));
        if let Some(to) = to {
            let replaced = match from {
                Some(f) => vec![f],
                None => target_ent(rt, b, host)
                    .and_then(|e| host.mesh_of(e))
                    .map(|m| host.mesh_materials(m))
                    .unwrap_or_default(),
            };
            for f in replaced.into_iter().take(MAX_REPLACED) {
                host.material_set(f, MatChange::CopyFrom(to));
            }
        }
        out(rt, b, 0);
    }
    Flow::Done
}

/// The most materials one Set Material block replaces at once.
const MAX_REPLACED: usize = 64;

/// Change Texture Size: the engine records the size (capped at 2048 x 2048); the picture is not resampled.
pub(crate) fn change_texture_size(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        if let Some(t) = target_texture(rt, b, host) {
            let size = pin(rt, b, "Size", 0, host).as_vec2();
            let clear = pin(rt, b, "Clear", 1, host).as_bool();
            let side = |f: f32| {
                if f.is_finite() && f > 0.0 {
                    f as u32
                } else {
                    0
                }
            };
            host.texture_set(
                t,
                TexChange::Size {
                    width: side(size[0]),
                    height: side(size[1]),
                    clear,
                },
            );
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) fn set_current_slot(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        if let Some(t) = target_texture(rt, b, host) {
            let slot = pin(rt, b, "Slot Index", 0, host).as_int();
            host.texture_set(t, TexChange::Slot(slot));
        }
        out(rt, b, 0);
    }
    Flow::Done
}

/// Get Texture Info: "Exist" when the target is a level texture (with its file name, and, past the declared two,
/// width, height and slot where the block has outputs for them), else "Empty".
pub(crate) fn get_texture_info(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let info = target_texture(rt, b, host).and_then(|t| host.texture_info(t));
        match info {
            Some(i) => {
                set_pout(rt, b, "FileName", 0, Value::Str(i.file));
                set_pout(rt, b, "Priority", 1, Value::Int(0));
                if b.pouts.len() > 2 {
                    set_pout(rt, b, "Width", 2, Value::Int(i.width as i32));
                    set_pout(rt, b, "Height", 3, Value::Int(i.height as i32));
                    set_pout(rt, b, "Slot", 4, Value::Int(i.slot));
                }
                out(rt, b, 0);
            }
            None => out(rt, b, 1),
        }
    }
    Flow::Done
}

/// Pixels one unit of the tick's work pays for.
const PIXELS_PER_UNIT: u64 = 64;

fn pay_pixels(rt: &mut Runtime, pixels: u64) {
    let _ = rt.pay(pixels / PIXELS_PER_UNIT + 1);
}

/// Blend Textures: the target texture takes Texture 1 * (1 - Blend Factor) + Texture 2 * Blend Factor (the factor is
/// clamped to 0..1).
pub(crate) fn blend_textures(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let target = target_texture(rt, b, host);
        let a = pin(rt, b, "Texture 1", 0, host).as_obj();
        let c = pin(rt, b, "Texture 2", 1, host).as_obj();
        let f = pin(rt, b, "Blend Factor", 2, host).as_float();
        if let (Some(t), Some(a), Some(c)) = (target, texture_of(rt, a), texture_of(rt, c)) {
            let factor = if f.is_finite() {
                f.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let n = host.texture_paint(t, TexPaint::Blend { a, b: c, factor });
            pay_pixels(rt, n);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

/// Write In Texture: pin UV is the pixel (x, y), Size the side of the square written (a pen texture, when given, is
/// stretched to it), Color and Write Mode as `game::texpaint` defines them.
pub(crate) fn write_in_texture(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        let target = target_texture(rt, b, host);
        let uv = pin(rt, b, "UV", 0, host).as_vec2();
        let color = pin(rt, b, "Color", 1, host).as_vec4();
        let size = pin(rt, b, "Size", 2, host).as_int();
        let pen = pin(rt, b, "Pen Texture", 3, host).as_obj();
        let mode = pin(rt, b, "Write Mode", 4, host).as_int();
        if let Some(t) = target {
            let pen = texture_of(rt, pen);
            let n = host.texture_paint(
                t,
                TexPaint::Write {
                    uv,
                    color,
                    size,
                    pen,
                    mode,
                },
            );
            pay_pixels(rt, n);
        }
        out(rt, b, 0);
    }
    Flow::Done
}

macro_rules! proto {
    ($guid:ident, $name:expr, $pins:expr, $pouts:expr, $flags:expr, $standing:ident, $run:expr) => {
        Proto {
            guid: proto::$guid,
            name: $name,
            library: Library::Materials,
            pins: $pins,
            pouts: $pouts,
            flags: $flags,
            standing: Standing::$standing,
            layouts: &[],
            run: $run,
            event: None,
        }
    };
}

pub(super) const PROTOS: &[Proto] = &[
    proto!(
        CHANGE_TEXTURE_SIZE,
        "Change Texture Size",
        &["Size", "Clear"],
        &[],
        flags::TARGET,
        Full,
        change_texture_size
    ),
    proto!(
        SET_TEXTURE,
        "Set Texture",
        &["Texture", "Perspective Correct", "Address Mode"],
        &[],
        flags::TARGET,
        Full,
        set_texture
    ),
    proto!(
        SET_DIFFUSE,
        "Set Diffuse",
        &["Diffuse Color", "Keep Alpha Component"],
        &[],
        flags::TARGET,
        Full,
        set_diffuse
    ),
    proto!(
        SET_EMISSIVE,
        "Set Emissive",
        &["Emissive Color"],
        &[],
        flags::TARGET,
        Full,
        set_emissive
    ),
    proto!(
        SET_SPECULAR,
        "Set Specular",
        &["Specular Color", "Specular Power"],
        &[],
        flags::TARGET,
        Full,
        set_specular
    ),
    proto!(
        SET_AMBIENT,
        "Set Ambient",
        &["Ambient Color"],
        &[],
        flags::TARGET,
        Full,
        set_ambient
    ),
    proto!(
        SET_MATERIAL,
        "Set Material",
        &["Material", "Material to replace"],
        &[],
        flags::TARGET,
        Partial,
        set_material
    ),
    proto!(
        SET_BLEND_MODES,
        "Set Blend Modes",
        &[
            "Blend",
            "Source Blend",
            "Destination Blend",
            "Texture Blend"
        ],
        &[],
        flags::TARGET,
        Full,
        set_blend_modes
    ),
    proto!(
        SET_MATERIAL_Z_BUFFER,
        "Set Material Z Buffer",
        &["Z Buffer Write", "Z Compare Function"],
        &[],
        flags::TARGET,
        Full,
        set_z_buffer
    ),
    proto!(
        SET_ALPHA_TEST,
        "Set Alpha Test",
        &[
            "Alpha Test Enable",
            "Alpha Test Function",
            "Alpha Reference Value"
        ],
        &[],
        flags::TARGET,
        Full,
        set_alpha_test
    ),
    proto!(
        SET_CURRENT_SLOT,
        "Set Current Slot",
        &["Slot Index"],
        &[],
        flags::TARGET,
        Full,
        set_current_slot
    ),
    proto!(
        GET_TEXTURE_INFO,
        "Get Texture Info",
        &[],
        &["FileName", "Priority"],
        flags::TARGET,
        Full,
        get_texture_info
    ),
    proto!(
        SET_3DSPRITE_MODE,
        "Set 3DSprite Mode",
        &["Mode"],
        &[],
        flags::TARGET,
        Full,
        set_3dsprite_mode
    ),
    proto!(
        BLEND_TEXTURES,
        "Blend Textures",
        &["Texture 1", "Texture 2", "Blend Factor"],
        &[],
        flags::TARGET,
        Full,
        blend_textures
    ),
    proto!(
        WRITE_IN_TEXTURE,
        "Write In Texture",
        &["UV", "Color", "Size", "Pen Texture", "Write Mode"],
        &[],
        flags::TARGET,
        Full,
        write_in_texture
    ),
    proto!(
        CHANGE_TEXTURE_VIDEO_FORMAT,
        "Change Texture Video Format",
        &[],
        &[],
        0,
        Full,
        change_texture_video_format
    ),
];
