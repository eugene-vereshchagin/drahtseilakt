// The open level format, version 1: a folder with `level.gltf` (glTF 2.0: meshes, materials, nodes with their
// matrices), `level.bin` (the binary buffer) and the textures as PNG files the caller writes. Group membership,
// which the game rules read, is stored in the scene's `extras`. glTF is right-handed and Virtools left-handed:
// Z is negated on the way out and back, which is exact, so a round trip reproduces the level bit for bit.
//
// Reading treats the folder as hostile (levels will be shared): sizes are capped, every accessor is checked
// against its buffer, and anything outside the subset written here is refused.

// `serde_json::Value` indexing by key never panics (a missing key reads as Null) and writes go to objects built
// just above; slices are only indexed through `chunks_exact`, whose chunks have the exact length.
#![allow(clippy::indexing_slicing)]

use crate::{
    Blend, Channel, Group, Level, Material, Mesh, Object, Part, TexGen, Texture, TextureSource,
};
use serde_json::{Value, json};
use std::path::Path;

pub const VERSION: u64 = 1;
/// The deepest parallax relief a material may ask for, in texture coordinates.
pub const MAX_PARALLAX: f64 = 0.25;
const MAX_JSON: u64 = 64 * 1024 * 1024;
const MAX_BIN: u64 = 1024 * 1024 * 1024;
/// The longest name a 2D frame keeps.
const MAX_NAME: usize = 1024;
const FLOAT: u64 = 5126;
const UINT: u64 = 5125;
/// glTF primitive modes: points (a mesh of vertices without faces) and triangles (the default).
const POINTS: u64 = 0;
const TRIANGLES: u64 = 4;

/// Negates the entries of a column-major matrix that mix Z with another axis: S·M·S with S = diag(1, 1, -1, 1).
fn flip_matrix(m: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = *m;
    for (c, col) in out.iter_mut().enumerate() {
        for (r, v) in col.iter_mut().enumerate() {
            if (c == 2) != (r == 2) {
                *v = -*v;
            }
        }
    }
    out
}

/// A number as JSON: itself when finite, else its name (`nan`, `inf`, `-inf`).
fn number_or_name(x: f32) -> Value {
    if x.is_finite() {
        json!(x)
    } else if x.is_nan() {
        json!("nan")
    } else if x > 0.0 {
        json!("inf")
    } else {
        json!("-inf")
    }
}

/// A number named by `number_or_name`.
fn name_to_number(v: &Value) -> Option<f32> {
    match v.as_str()? {
        "nan" => Some(f32::NAN),
        "inf" => Some(f32::INFINITY),
        "-inf" => Some(f32::NEG_INFINITY),
        _ => None,
    }
}

fn blend_name(b: Blend) -> &'static str {
    match b {
        Blend::Zero => "zero",
        Blend::One => "one",
        Blend::SrcColor => "src_color",
        Blend::InvSrcColor => "inv_src_color",
        Blend::SrcAlpha => "src_alpha",
        Blend::InvSrcAlpha => "inv_src_alpha",
        Blend::DestAlpha => "dest_alpha",
        Blend::InvDestAlpha => "inv_dest_alpha",
        Blend::DestColor => "dest_color",
        Blend::InvDestColor => "inv_dest_color",
    }
}

fn blend_from(s: &str) -> Option<Blend> {
    Some(match s {
        "zero" => Blend::Zero,
        "one" => Blend::One,
        "src_color" => Blend::SrcColor,
        "inv_src_color" => Blend::InvSrcColor,
        "src_alpha" => Blend::SrcAlpha,
        "inv_src_alpha" => Blend::InvSrcAlpha,
        "dest_alpha" => Blend::DestAlpha,
        "inv_dest_alpha" => Blend::InvDestAlpha,
        "dest_color" => Blend::DestColor,
        "inv_dest_color" => Blend::InvDestColor,
        _ => return None,
    })
}

/// A file name for a texture that is safe on every platform.
/// The prefix of an image `uri` naming the game's texture by name (`game:Track_Side`) instead of a file.
pub const GAME_URI: &str = "game:";

/// Whether `name` may be a `game:` texture's name: 1 to 128 characters, none a path separator, `:` or a control
/// character, not beginning with a dot (it is only ever compared with the game's texture names, never made a path).
pub fn game_texture_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= 128
        && !name.starts_with('.')
        && !name
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':') || c.is_control())
}

pub fn texture_file(index: usize, name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    format!("textures/{index:04}_{clean}.png")
}

/// Writes `level.gltf` and `level.bin` into `dir`. Texture images are referenced as `texture_file(i, name)`;
/// writing those PNG files is the caller's job (it has the pixels).
pub fn export(level: &Level, dir: &Path) -> Result<(), String> {
    let same: Vec<usize> = (0..level.textures.len()).collect();
    export_textures(level, dir, &level.textures, &same)
}

/// `export` with the textures given instead of the level's: `textures` are the images written (named by
/// `texture_file(i, name)`), and `remap[i]` is the image the level's texture `i` is, so that textures with the same
/// picture are one image. A texture without an entry in `remap` is dropped from the materials.
pub fn export_textures(
    level: &Level,
    dir: &Path,
    textures: &[Texture],
    remap: &[usize],
) -> Result<(), String> {
    export_slots(level, dir, textures, remap, &[])
}

/// `export_textures` with the further slots of textures (an animation's frames): each `(image, slots)` names the
/// images that are that image's slots after the first, in order (written in its extras).
pub fn export_slots(
    level: &Level,
    dir: &Path,
    textures: &[Texture],
    remap: &[usize],
    slots: &[(usize, Vec<usize>)],
) -> Result<(), String> {
    let tex = |t: Option<usize>| t.and_then(|i| remap.get(i).copied());
    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut push = |bytes: &[u8], target: u64, accessor: Value| -> usize {
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        views.push(json!({"buffer": 0, "byteOffset": bin.len(), "byteLength": bytes.len(), "target": target}));
        bin.extend_from_slice(bytes);
        let mut a = accessor;
        a["bufferView"] = json!(views.len().saturating_sub(1));
        accessors.push(a);
        accessors.len().saturating_sub(1)
    };

    let mut meshes = Vec::new();
    let mut mesh_index = Vec::new();
    for m in &level.meshes {
        let at = mesh_json(m, &mut push).map(|v| {
            meshes.push(v);
            meshes.len().saturating_sub(1)
        });
        mesh_index.push(at);
    }
    // a mesh's finer look is one more mesh after all the level's own, named in the mesh's extras
    for (m, at) in level.meshes.iter().zip(&mesh_index) {
        let (Some(look), Some(at)) = (&m.look, *at) else {
            continue;
        };
        let Some(v) = mesh_json(look, &mut push) else {
            continue;
        };
        meshes.push(v);
        let k = meshes.len().saturating_sub(1);
        if let Some(mesh) = meshes.get_mut(at) {
            mesh["extras"]["drahtseilakt"]["look"] = json!(k);
        }
    }

    let materials: Vec<Value> = level
        .materials
        .iter()
        .map(|m| {
            let mut pbr = json!({"baseColorFactor": m.diffuse, "metallicFactor": 0.0, "roughnessFactor": 1.0});
            if let Some(t) = tex(m.texture) {
                pbr["baseColorTexture"] = json!({"index": t});
            }
            let mut v = json!({
                "name": m.name,
                "pbrMetallicRoughness": pbr,
                "emissiveFactor": m.emissive,
                "doubleSided": m.two_sided,
                "extras": {"drahtseilakt": {
                    "ambient": m.ambient, "z_write": m.z_write, "specular": m.specular, "power": m.power,
                }},
            });
            match (m.blend, m.alpha_cutoff) {
                (Some((s, d)), _) => {
                    v["alphaMode"] = json!("BLEND");
                    v["extras"]["drahtseilakt"]["blend"] = json!([blend_name(s), blend_name(d)]);
                }
                (None, Some(c)) => {
                    v["alphaMode"] = json!("MASK");
                    v["alphaCutoff"] = json!(c);
                }
                (None, None) => v["alphaMode"] = json!("OPAQUE"),
            }
            if m.alpha_cutoff.is_some() && m.blend.is_some() {
                v["extras"]["drahtseilakt"]["alpha_cutoff"] = json!(m.alpha_cutoff);
            }
            if m.unlit {
                v["extensions"] = json!({"KHR_materials_unlit": {}});
            }
            if let Some(t) = tex(m.normal) {
                v["normalTexture"] = json!({"index": t});
                if m.parallax > 0.0 {
                    v["extras"]["drahtseilakt"]["parallax"] = json!(m.parallax);
                }
            }
            if let Some(g) = m.texgen {
                v["extras"]["drahtseilakt"]["texgen"] = json!(match g {
                    TexGen::Reflect => "reflect",
                    TexGen::Chrome => "chrome",
                });
            }
            v
        })
        .collect();

    let images: Vec<Value> = textures
        .iter()
        .enumerate()
        .map(|(i, t)| {
            // the game's texture by name stays a name (it is the packs', not the level's)
            let uri = match &t.source {
                TextureSource::Game(n) => format!("{GAME_URI}{n}"),
                _ => texture_file(i, &t.name),
            };
            let mut v = json!({"uri": uri, "name": t.name});
            if let Some(k) = t.color_key {
                v["extras"] = json!({"drahtseilakt": {"color_key": k}});
            }
            if let Some((_, more)) = slots.iter().find(|s| s.0 == i) {
                v["extras"]["drahtseilakt"]["slots"] = json!(more);
            }
            v
        })
        .collect();
    let textures: Vec<Value> = (0..textures.len())
        .map(|i| json!({"source": i, "sampler": 0}))
        .collect();

    let nodes: Vec<Value> = level
        .objects
        .iter()
        .map(|o| {
            let m = flip_matrix(&o.matrix);
            let flat: Vec<f32> = m.iter().flatten().copied().collect();
            // JSON has no number that is not finite: such a matrix (a map's object the game plays all the same) is
            // kept whole in the extras, its entries named, and glTF's has 0 for them
            let finite: Vec<f32> = flat.iter().map(|x| if x.is_finite() { *x } else { 0.0 }).collect();
            let mut v = json!({"name": o.name, "matrix": finite, "extras": {"drahtseilakt": {"visible": o.visible}}});
            if flat.iter().any(|x| !x.is_finite()) {
                v["extras"]["drahtseilakt"]["matrix"] =
                    json!(flat.iter().map(|x| number_or_name(*x)).collect::<Vec<_>>());
            }
            if let Some(s) = o.sprite {
                v["extras"]["drahtseilakt"]["sprite"] = json!({"mode": s.mode, "size": s.size, "offset": s.offset});
            }
            // the object it follows (Set Parent in the map); the matrix stays the world's, so it is no glTF child
            if let Some(p) = o.parent {
                v["extras"]["drahtseilakt"]["parent"] = json!(p);
            }
            if let Some(mesh) = o.mesh.and_then(|i| mesh_index.get(i).copied().flatten()) {
                v["mesh"] = json!(mesh);
            }
            v
        })
        .collect();
    let groups: Vec<Value> = level
        .groups
        .iter()
        .map(|g| json!({"name": g.name, "members": g.members}))
        .collect();

    let scripts = level.scripts.as_ref().map(|g| {
        crate::script_json::to_json(g, &|i| mesh_index.get(i).copied().flatten(), &|i| {
            remap.get(i).copied()
        })
    });
    let mut doc = json!({
        "asset": {"version": "2.0", "generator": "drahtseilakt"},
        "extensionsUsed": if level.materials.iter().any(|m| m.unlit) { json!(["KHR_materials_unlit"]) } else { json!([]) },
        "scene": 0,
        "scenes": [{"nodes": (0..nodes.len()).collect::<Vec<_>>(), "extras": {"drahtseilakt": {"format": VERSION, "groups": groups, "sky": level.sky, "start_ball": level.start_ball, "settings": settings_json(&level.settings)}}}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "textures": textures,
        "images": images,
        "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}],
        "buffers": [{"uri": "level.bin", "byteLength": bin.len()}],
        "bufferViews": views,
        "accessors": accessors,
    });
    if let Some(s) = scripts {
        doc["scenes"][0]["extras"]["drahtseilakt"]["scripts"] = s;
    }
    if !level.frames.is_empty() {
        let fin = |a: [f32; 4]| a.map(|x| if x.is_finite() { x } else { 0.0 });
        let frames: Vec<Value> = level
            .frames
            .iter()
            .take(crate::MAX_FRAMES)
            .map(|f| {
                json!({"name": f.name, "rect": fin(f.rect), "uv": fin(f.uv), "z_order": f.z_order,
                    "material": f.material, "visible": f.visible})
            })
            .collect();
        doc["scenes"][0]["extras"]["drahtseilakt"]["frames"] = json!(frames);
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("level.gltf"), text).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("level.bin"), &bin).map_err(|e| e.to_string())?;
    Ok(())
}

/// The level's settings as the scene's extras keep them (only what it has).
fn settings_json(s: &crate::Settings) -> Value {
    let mut v = json!({"tutorial": s.tutorial, "last_level": s.last_level});
    if let Some(m) = s.music {
        v["music"] = json!(m);
    }
    if let Some(l) = s.light {
        v["light"] = json!(l);
    }
    if let Some(b) = s.level_bonus {
        v["level_bonus"] = json!(b);
    }
    if let Some(h) = s.source_hash {
        v["source_hash"] = json!(format!("{h:016x}"));
    }
    if s.own_parts {
        v["parts"] = json!("own");
    }
    if s.sector_balls.iter().any(Option::is_some) {
        v["sector_balls"] = json!(s.sector_balls);
    }
    if let Some(f) = s.fog {
        v["fog"] = json!({"color": f.color, "color_down": f.color_down, "density": f.density, "below": f.below,
            "depth": f.depth});
    }
    if let Some(c) = s.clouds {
        v["clouds"] = json!({"top": c.top, "thickness": c.thickness, "cover": c.cover, "scale": c.scale, "sun": c.sun,
            "sun_color": c.sun_color, "sky": c.sky, "shadow": c.shadow, "haze": c.haze, "under": c.under, "wind": c.wind});
    }
    v
}

/// A number of a settings object: finite, held to lo..hi; `None` when it is missing (null).
fn number(o: &Value, what: &str, k: &str, lo: f32, hi: f32) -> Result<Option<f32>, String> {
    match &o[k] {
        Value::Null => Ok(None),
        x => {
            let x = x
                .as_f64()
                .ok_or_else(|| format!("settings: {what}'s {k} is not a number"))?
                as f32;
            if x.is_finite() {
                Ok(Some(x.clamp(lo, hi)))
            } else {
                Err(format!("settings: {what}'s {k} is not finite"))
            }
        }
    }
}

/// N numbers of a settings object, each finite and held to lo..hi; `None` when they are missing (null).
fn numbers<const N: usize>(
    o: &Value,
    what: &str,
    k: &str,
    lo: f32,
    hi: f32,
) -> Result<Option<[f32; N]>, String> {
    if o[k].is_null() {
        return Ok(None);
    }
    let bad = || format!("settings: {what}'s {k} is not {N} numbers");
    let a = o[k].as_array().filter(|a| a.len() == N).ok_or_else(bad)?;
    let mut out = [0.0f32; N];
    for (slot, x) in out.iter_mut().zip(a) {
        let x = x.as_f64().ok_or_else(bad)? as f32;
        if !x.is_finite() {
            return Err(format!("settings: {what}'s {k} is not finite"));
        }
        *slot = x.clamp(lo, hi);
    }
    Ok(Some(out))
}

/// The cloud layer's settings: its tops' height is needed, the rest has defaults ([`crate::Clouds::DEFAULT`]).
fn clouds_from(c: &Value) -> Result<crate::Clouds, String> {
    if !c.is_object() {
        return Err("settings: clouds is not an object".into());
    }
    let d = crate::Clouds::DEFAULT;
    let what = "clouds";
    let top = number(c, what, "top", -1.0e6, 1.0e6)?.ok_or("settings: clouds has no top")?;
    // the direction to the sun: a zero vector (nowhere) keeps the default
    let sun = numbers::<3>(c, what, "sun", -1.0, 1.0)?
        .map(|s| {
            let l = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            if l > 1e-3 {
                [s[0] / l, s[1] / l, s[2] / l]
            } else {
                d.sun
            }
        })
        .unwrap_or(d.sun);
    Ok(crate::Clouds {
        top,
        thickness: number(c, what, "thickness", 1.0, 1.0e4)?.unwrap_or(d.thickness),
        cover: number(c, what, "cover", 0.0, 1.0)?.unwrap_or(d.cover),
        scale: number(c, what, "scale", 1.0, 1.0e4)?.unwrap_or(d.scale),
        sun,
        sun_color: numbers(c, what, "sun_color", 0.0, 64.0)?.unwrap_or(d.sun_color),
        sky: numbers(c, what, "sky", 0.0, 16.0)?.unwrap_or(d.sky),
        shadow: numbers(c, what, "shadow", 0.0, 16.0)?.unwrap_or(d.shadow),
        haze: number(c, what, "haze", 0.0, 1.0)?.unwrap_or(d.haze),
        under: number(c, what, "under", 0.0, 1.0)?.unwrap_or(d.under),
        wind: numbers(c, what, "wind", -1.0e3, 1.0e3)?.unwrap_or(d.wind),
    })
}

/// The most a level's music theme and bonus may be (the original's are 1 to 5 and up to 20000).
const MAX_MUSIC: u64 = 1000;
const MAX_BONUS: i64 = 1_000_000_000;

/// The level's settings from the scene's extras: none there is the default; one there that is malformed refuses the
/// level.
fn settings_from(v: &Value) -> Result<crate::Settings, String> {
    if v.is_null() {
        return Ok(crate::Settings::default());
    }
    if !v.is_object() {
        return Err("settings: not an object".into());
    }
    let flag = |k: &str| -> Result<bool, String> {
        match &v[k] {
            Value::Null => Ok(false),
            Value::Bool(b) => Ok(*b),
            _ => Err(format!("settings: {k} is not true or false")),
        }
    };
    let music = match &v["music"] {
        Value::Null => None,
        m => Some(
            m.as_u64()
                .filter(|&m| m <= MAX_MUSIC)
                .and_then(|m| u32::try_from(m).ok())
                .ok_or("settings: music is not a theme number")?,
        ),
    };
    let light = match &v["light"] {
        Value::Null => None,
        l => {
            let a = l
                .as_array()
                .filter(|a| a.len() == 3)
                .ok_or("settings: light is not three numbers")?;
            let c = |i: usize| -> Result<f32, String> {
                let x = a
                    .get(i)
                    .and_then(Value::as_f64)
                    .ok_or("settings: light is not three numbers")? as f32;
                if x.is_finite() {
                    Ok(x.clamp(0.0, 1.0))
                } else {
                    Err("settings: light is not finite".into())
                }
            };
            Some([c(0)?, c(1)?, c(2)?])
        }
    };
    let level_bonus = match &v["level_bonus"] {
        Value::Null => None,
        b => Some(
            b.as_i64()
                // unsigned_abs: i64::MIN has no abs(), which wrapped to itself and passed the check
                .filter(|b| b.unsigned_abs() <= MAX_BONUS.unsigned_abs())
                .ok_or("settings: level_bonus is not a number of points")?,
        ),
    };
    let source_hash = match &v["source_hash"] {
        Value::Null => None,
        h => {
            let s = h
                .as_str()
                .filter(|s| s.len() == 16)
                .ok_or("settings: source_hash is not 16 hex digits")?;
            Some(
                u64::from_str_radix(s, 16)
                    .map_err(|_| "settings: source_hash is not 16 hex digits".to_owned())?,
            )
        }
    };
    let own_parts = match &v["parts"] {
        Value::Null => false,
        Value::String(p) if p == "own" => true,
        Value::String(p) if p == "copy" => false,
        _ => return Err("settings: parts is not \"own\" or \"copy\"".into()),
    };
    let fog = match &v["fog"] {
        Value::Null => None,
        f => {
            if !f.is_object() {
                return Err("settings: fog is not an object".into());
            }
            let num = |k: &str, lo: f32, hi: f32| -> Result<f32, String> {
                let x = f[k]
                    .as_f64()
                    .ok_or_else(|| format!("settings: fog's {k} is not a number"))?
                    as f32;
                if x.is_finite() {
                    Ok(x.clamp(lo, hi))
                } else {
                    Err(format!("settings: fog's {k} is not finite"))
                }
            };
            let colour = |k: &str| -> Result<[f32; 3], String> {
                let a = f[k]
                    .as_array()
                    .filter(|a| a.len() == 3)
                    .ok_or_else(|| format!("settings: fog's {k} is not three numbers"))?;
                let mut out = [0.0f32; 3];
                for (o, x) in out.iter_mut().zip(a) {
                    let x = x
                        .as_f64()
                        .ok_or_else(|| format!("settings: fog's {k} is not three numbers"))?
                        as f32;
                    if !x.is_finite() {
                        return Err(format!("settings: fog's {k} is not finite"));
                    }
                    *o = x.clamp(0.0, 1.0);
                }
                Ok(out)
            };
            let color = colour("color")?;
            // a fog without its colour seen downwards (missing or null) has one colour all round
            let color_down = if f["color_down"].is_null() {
                color
            } else {
                colour("color_down")?
            };
            Some(crate::Fog {
                color,
                color_down,
                density: num("density", 0.0, 1.0)?,
                below: num("below", -1.0e6, 1.0e6)?,
                depth: num("depth", 0.01, 1.0e6)?,
            })
        }
    };
    let clouds = match &v["clouds"] {
        Value::Null => None,
        c => Some(clouds_from(c)?),
    };
    let sector_balls = match &v["sector_balls"] {
        Value::Null => Vec::new(),
        Value::Array(a) if a.len() <= 256 => a
            .iter()
            .map(|b| match b {
                Value::Null => Ok(None),
                Value::String(s) if ["wood", "stone", "paper", "sticky"].contains(&s.as_str()) => {
                    Ok(Some(s.clone()))
                }
                _ => Err("settings: sector_balls holds wood, stone, paper, sticky or null"),
            })
            .collect::<Result<_, _>>()?,
        _ => return Err("settings: sector_balls is not a list".into()),
    };
    Ok(crate::Settings {
        sector_balls,
        clouds,
        fog,
        own_parts,
        music,
        light,
        level_bonus,
        tutorial: flag("tutorial")?,
        last_level: flag("last_level")?,
        source_hash,
    })
}

/// A mesh the open format writes: one with vertices (an empty one is left out, so the meshes read back are the
/// written ones, in their order). A mesh of vertices without faces (a particle emitter's points) is written as one
/// primitive of points.
pub fn written(m: &Mesh) -> bool {
    !m.positions.is_empty()
}

/// One mesh as glTF, its data pushed into the buffer; None for a mesh without faces.
fn mesh_json(m: &Mesh, push: &mut impl FnMut(&[u8], u64, Value) -> usize) -> Option<Value> {
    let n = m.positions.len();
    if !written(m) {
        return None;
    }
    let parts: Vec<&Part> = m.parts.iter().filter(|p| !p.indices.is_empty()).collect();
    let pos: Vec<f32> = m
        .positions
        .iter()
        .flat_map(|p| [p[0], p[1], -p[2]])
        .collect();
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in pos.chunks_exact(3) {
        for k in 0..3 {
            let v = p.get(k).copied().unwrap_or(0.0);
            if let (Some(l), Some(h)) = (lo.get_mut(k), hi.get_mut(k)) {
                *l = l.min(v);
                *h = h.max(v);
            }
        }
    }
    let p_acc = push(
        &f32_bytes(&pos),
        34962,
        json!({"componentType": FLOAT, "count": n, "type": "VEC3", "min": lo, "max": hi}),
    );
    let nor: Vec<f32> = (0..n)
        .flat_map(|i| {
            let v = m.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
            [v[0], v[1], -v[2]]
        })
        .collect();
    let n_acc = push(
        &f32_bytes(&nor),
        34962,
        json!({"componentType": FLOAT, "count": n, "type": "VEC3"}),
    );
    let uv: Vec<f32> = (0..n)
        .flat_map(|i| m.uvs.get(i).copied().unwrap_or([0.0; 2]))
        .collect();
    let t_acc = push(
        &f32_bytes(&uv),
        34962,
        json!({"componentType": FLOAT, "count": n, "type": "VEC2"}),
    );
    // each channel's coordinates as one more TEXCOORD set, named in the mesh's extras
    let mut attributes = json!({"POSITION": p_acc, "NORMAL": n_acc, "TEXCOORD_0": t_acc});
    let mut channels = Vec::new();
    for (k, c) in (1usize..).zip(&m.channels) {
        let uv: Vec<f32> = (0..n)
            .flat_map(|i| c.uvs.get(i).copied().unwrap_or([0.0; 2]))
            .collect();
        let acc = push(
            &f32_bytes(&uv),
            34962,
            json!({"componentType": FLOAT, "count": n, "type": "VEC2"}),
        );
        attributes[format!("TEXCOORD_{k}")] = json!(acc);
        channels.push(json!({"material": c.material, "blend": [blend_name(c.blend.0), blend_name(c.blend.1)], "texcoord": k}));
    }
    let mut prims = Vec::new();
    for p in parts {
        // Mirroring Z reverses the winding; swap two corners to keep faces facing the same way.
        let idx: Vec<u32> = p
            .indices
            .chunks_exact(3)
            .flat_map(|t| [t[0], t[2], t[1]])
            .collect();
        let bytes: Vec<u8> = idx.iter().flat_map(|i| i.to_le_bytes()).collect();
        let i_acc = push(
            &bytes,
            34963,
            json!({"componentType": UINT, "count": idx.len(), "type": "SCALAR"}),
        );
        let mut prim = json!({"attributes": attributes.clone(), "indices": i_acc});
        if let Some(mat) = p.material {
            prim["material"] = json!(mat);
        }
        prims.push(prim);
    }
    if prims.is_empty() {
        // vertices without faces: glTF's points (mode 0), no indices and no material
        prims.push(json!({"attributes": attributes, "mode": POINTS}));
    }
    let mut mesh = json!({"name": m.name, "primitives": prims});
    if !channels.is_empty() {
        mesh["extras"] = json!({"drahtseilakt": {"channels": channels}});
    }
    Some(mesh)
}

/// Gives each mesh the finer look its extras name and takes the looks out of the level's meshes. The looks must all
/// come after the level's own meshes, each the look of one mesh only, and have no look of their own; anything else is
/// refused.
fn attach_looks(meshes: &mut Vec<Mesh>, looks: &[Option<u64>]) -> Result<(), String> {
    let wanted: Vec<(usize, usize)> = looks
        .iter()
        .enumerate()
        .filter_map(|(i, k)| k.map(|k| usize::try_from(k).map(|k| (i, k))))
        .collect::<Result<_, _>>()
        .map_err(|_| "a look's index is out of range".to_owned())?;
    let Some(first) = wanted.iter().map(|&(_, k)| k).min() else {
        return Ok(());
    };
    let mut seen = vec![false; meshes.len()];
    for &(i, k) in &wanted {
        if k >= meshes.len() || i >= first || k < first {
            return Err("a mesh's look must be one of the meshes after the level's own".into());
        }
        match seen.get_mut(k) {
            Some(s) if !*s => *s = true,
            _ => return Err("a look is the look of two meshes".into()),
        }
    }
    let extra: Vec<Mesh> = meshes.split_off(first);
    if seen.iter().skip(first).any(|s| !s) {
        return Err("a mesh after the level's own is no mesh's look".into());
    }
    let mut extra: Vec<Option<Mesh>> = extra.into_iter().map(Some).collect();
    for (i, k) in wanted {
        let look = k
            .checked_sub(first)
            .and_then(|j| extra.get_mut(j))
            .and_then(Option::take);
        if let (Some(m), Some(look)) = (meshes.get_mut(i), look) {
            m.look = Some(Box::new(look));
        }
    }
    Ok(())
}

fn f32_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// A level file: a regular file itself (never a link, a device or a pipe), read up to `cap` bytes (what it says its
/// length is does not count; a file that grows while read is refused).
fn read_capped(path: &Path, cap: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let plain = std::fs::symlink_metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .is_file();
    if !plain {
        return Err(format!("{}: not a plain file", path.display()));
    }
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if u64::try_from(bytes.len()).map_or(true, |n| n > cap) {
        return Err(format!(
            "{}: above the limit of {cap} bytes",
            path.display()
        ));
    }
    Ok(bytes)
}

struct Reader<'a> {
    doc: &'a Value,
    bin: &'a [u8],
    /// Bytes still to be read through accessors. glTF lets accessors be shared, so a level.gltf naming one large
    /// accessor from many meshes would copy the buffer again and again (gigabytes from a small file); what is read in
    /// all stays within twice the buffer (what the exporter writes reads each byte once).
    budget: std::cell::Cell<usize>,
}

impl Reader<'_> {
    /// The bytes and element count of an accessor, after checking it against its view and the buffer.
    fn accessor(
        &self,
        index: u64,
        component: u64,
        kind: &str,
        width: usize,
    ) -> Result<(&[u8], usize), String> {
        let a = self.doc["accessors"]
            .get(index as usize)
            .ok_or("accessor out of range")?;
        if a["componentType"].as_u64() != Some(component) || a["type"].as_str() != Some(kind) {
            return Err(format!("accessor {index}: unsupported layout"));
        }
        let count = a["count"].as_u64().ok_or("accessor without count")? as usize;
        let view = self.doc["bufferViews"]
            .get(a["bufferView"].as_u64().ok_or("accessor without view")? as usize)
            .ok_or("buffer view out of range")?;
        if view.get("byteStride").is_some()
            || a.get("byteOffset").is_some_and(|o| o.as_u64() != Some(0))
        {
            return Err(format!(
                "accessor {index}: strided or offset data is not used by this format"
            ));
        }
        let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize;
        let length = view["byteLength"].as_u64().ok_or("view without length")? as usize;
        let needed = count.checked_mul(width).ok_or("accessor size overflows")?;
        if needed > length {
            return Err(format!("accessor {index}: larger than its view"));
        }
        let left = self
            .budget
            .get()
            .checked_sub(needed)
            .ok_or("accessors read more than twice the buffer")?;
        self.budget.set(left);
        let end = offset.checked_add(needed).ok_or("view overflows")?;
        let bytes = self
            .bin
            .get(offset..end)
            .ok_or(format!("accessor {index}: outside the buffer"))?;
        Ok((bytes, count))
    }

    fn floats<const N: usize>(&self, index: u64, kind: &str) -> Result<Vec<[f32; N]>, String> {
        let width = N.saturating_mul(4);
        let (bytes, count) = self.accessor(index, FLOAT, kind, width)?;
        let mut out = Vec::with_capacity(count);
        for chunk in bytes.chunks_exact(width.max(1)) {
            let mut v = [0.0f32; N];
            for (k, f) in chunk.chunks_exact(4).enumerate() {
                if let (Some(slot), Ok(b)) = (v.get_mut(k), <[u8; 4]>::try_from(f)) {
                    *slot = f32::from_le_bytes(b);
                }
            }
            out.push(v);
        }
        Ok(out)
    }
}

/// A texture's colour key from the image's extras: three bytes, or none.
fn color_key_of(v: &serde_json::Value) -> Option<[u8; 3]> {
    let a = v.as_array()?;
    if a.len() != 3 {
        return None;
    }
    let mut k = [0u8; 3];
    for (o, c) in k.iter_mut().zip(a) {
        *o = u8::try_from(c.as_u64()?).ok()?;
    }
    Some(k)
}

/// The 2D frames of the scene's extras (none when absent); a malformed one, more than `MAX_FRAMES` or a material that
/// is not there refuses the level.
fn frames_from(v: &Value, materials: usize) -> Result<Vec<crate::Frame>, String> {
    if v.is_null() {
        return Ok(Vec::new());
    }
    let list = v.as_array().ok_or("frames must be a list")?;
    if list.len() > crate::MAX_FRAMES {
        return Err("too many frames".into());
    }
    let bad = || "a frame is malformed".to_owned();
    let four = |a: &Value| -> Result<[f32; 4], String> {
        let a = a.as_array().filter(|a| a.len() == 4).ok_or_else(bad)?;
        let mut out = [0.0f32; 4];
        for (o, x) in out.iter_mut().zip(a) {
            *o = x.as_f64().ok_or_else(bad)? as f32;
            if !o.is_finite() {
                return Err(bad());
            }
        }
        Ok(out)
    };
    list.iter()
        .map(|f| {
            let name = f["name"].as_str().ok_or_else(bad)?;
            if name.len() > MAX_NAME {
                return Err(bad());
            }
            let material = match &f["material"] {
                Value::Null => None,
                m => Some(
                    m.as_u64()
                        .and_then(|m| usize::try_from(m).ok())
                        .filter(|&m| m < materials)
                        .ok_or("a frame's material is not there")?,
                ),
            };
            Ok(crate::Frame {
                name: name.to_owned(),
                rect: four(&f["rect"])?,
                uv: four(&f["uv"])?,
                z_order: f["z_order"]
                    .as_i64()
                    .and_then(|z| i32::try_from(z).ok())
                    .ok_or_else(bad)?,
                material,
                visible: f["visible"].as_bool().ok_or_else(bad)?,
            })
        })
        .collect()
}

/// A sprite's placement from the object's extras, if it is one and its numbers are sound.
fn sprite_of(v: &serde_json::Value) -> Option<crate::Sprite> {
    let pair = |k: &str| -> Option<[f32; 2]> {
        let a = v[k].as_array()?;
        let x = a.first()?.as_f64()? as f32;
        let y = a.get(1)?.as_f64()? as f32;
        (x.is_finite() && y.is_finite()).then_some([x, y])
    };
    Some(crate::Sprite {
        mode: u32::try_from(v["mode"].as_u64()?).ok()?,
        size: pair("size")?,
        offset: pair("offset")?,
    })
}

/// `dir` as a full path with `.` and `..` worked out by name (no link followed): what the level's own files are known by.
pub fn full_path(dir: &Path) -> std::io::Result<std::path::PathBuf> {
    let mut out = std::path::PathBuf::new();
    for c in std::path::absolute(dir)?.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

/// Reads an open level from `dir`. Texture images become `TextureSource::LevelFile` with the full path.
pub fn import(dir: &Path) -> Result<Level, String> {
    import_read(dir).map(|(level, _)| level)
}

/// Reads an open level from `dir`, and gives the bytes it was read from (level.gltf, then level.bin), so that what
/// a level is known by is what was played.
pub fn import_read(dir: &Path) -> Result<(Level, Vec<u8>), String> {
    // the level's own files are known by their full path (a relative one would read like a texture's bare name)
    let dir = &full_path(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let text = read_capped(&dir.join("level.gltf"), MAX_JSON)?;
    import_from(dir, text, || read_capped(&dir.join("level.bin"), MAX_BIN))
}

/// Reads an open level from the bytes of its level.gltf and level.bin (the fuzz targets' entry: no files), its
/// textures named as files of `dir`.
pub fn import_bytes(dir: &Path, gltf: &[u8], bin: &[u8]) -> Result<Level, String> {
    if u64::try_from(gltf.len()).map_or(true, |n| n > MAX_JSON)
        || u64::try_from(bin.len()).map_or(true, |n| n > MAX_BIN)
    {
        return Err("above the size limits".into());
    }
    import_from(dir, gltf.to_vec(), || Ok(bin.to_vec())).map(|(level, _)| level)
}

fn import_from(
    dir: &Path,
    text: Vec<u8>,
    read_bin: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<(Level, Vec<u8>), String> {
    let doc: Value = serde_json::from_slice(&text).map_err(|e| format!("level.gltf: {e}"))?;
    if doc["asset"]["version"].as_str() != Some("2.0") {
        return Err("level.gltf: not glTF 2.0".into());
    }
    let format = doc["scenes"][0]["extras"]["drahtseilakt"]["format"].as_u64();
    if format != Some(VERSION) {
        return Err(format!(
            "level.gltf: open level format {format:?}, this game reads {VERSION}"
        ));
    }
    let uri = doc["buffers"][0]["uri"].as_str().ok_or("no buffer")?;
    if uri != "level.bin" {
        return Err("the buffer must be level.bin next to level.gltf".into());
    }
    let bin = read_bin()?;
    let r = Reader {
        doc: &doc,
        bin: &bin,
        budget: std::cell::Cell::new(bin.len().saturating_mul(2)),
    };
    let arr = |key: &str| doc[key].as_array().cloned().unwrap_or_default();
    let mut level = Level::default();

    for (i, img) in arr("images").iter().enumerate() {
        let uri = img["uri"].as_str().ok_or("image without uri")?;
        // the game's texture by name: no file of the level's
        if let Some(name) = uri.strip_prefix(GAME_URI) {
            if !game_texture_name(name) {
                return Err(format!(
                    "image {i}: not a plain name of the game's textures"
                ));
            }
            level.textures.push(Texture {
                name: img["name"].as_str().unwrap_or(name).to_owned(),
                source: TextureSource::Game(name.to_owned()),
                color_key: color_key_of(&img["extras"]["drahtseilakt"]["color_key"]),
            });
            continue;
        }
        // A texture is a file of the level's textures/ folder, by a plain name (no further folders: a link among them
        // could lead out of the level's folder).
        let plain = uri.strip_prefix("textures/").is_some_and(|n| {
            !n.is_empty()
                && !n.starts_with('.')
                && !n
                    .chars()
                    .any(|c| matches!(c, '/' | '\\' | ':') || c.is_control())
        });
        if !plain {
            return Err(format!(
                "image {i}: not a plain file of the level's textures/ folder"
            ));
        }
        level.textures.push(Texture {
            name: img["name"].as_str().unwrap_or(uri).to_owned(),
            source: TextureSource::LevelFile(dir.join(uri)),
            color_key: color_key_of(&img["extras"]["drahtseilakt"]["color_key"]),
        });
    }
    // a texture's further slots: other images of the level, at most MAX_SLOTS - 1 of them
    for (i, img) in arr("images").iter().enumerate() {
        let listed = &img["extras"]["drahtseilakt"]["slots"];
        if listed.is_null() {
            continue;
        }
        let bad = || format!("image {i}: its slots are not other images of the level");
        let list = listed
            .as_array()
            .filter(|l| l.len() < nmo::scene::MAX_SLOTS)
            .ok_or_else(bad)?;
        let mut sources = Vec::with_capacity(list.len());
        for k in list {
            let k = k
                .as_u64()
                .and_then(|k| usize::try_from(k).ok())
                .filter(|&k| k != i)
                .ok_or_else(bad)?;
            sources.push(level.textures.get(k).ok_or_else(bad)?.source.clone());
        }
        level.texture_slots.push((i, sources));
    }
    let tex_image: Vec<Option<usize>> = arr("textures")
        .iter()
        .map(|t| t["source"].as_u64().map(|s| s as usize))
        .collect();

    for m in arr("materials") {
        let pbr = &m["pbrMetallicRoughness"];
        let x = &m["extras"]["drahtseilakt"];
        let f4 = |v: &Value, d: [f32; 4]| -> [f32; 4] {
            let mut out = d;
            for (k, slot) in out.iter_mut().enumerate() {
                if let Some(f) = v.get(k).and_then(Value::as_f64) {
                    *slot = f as f32;
                }
            }
            out
        };
        let rgb = |v: &Value| {
            let [a, b, c, _] = f4(v, [0.0; 4]);
            [a, b, c]
        };
        let blend = match m["alphaMode"].as_str() {
            Some("BLEND") => {
                let pair = x["blend"].as_array().cloned().unwrap_or_default();
                let s = pair
                    .first()
                    .and_then(Value::as_str)
                    .and_then(blend_from)
                    .unwrap_or(Blend::SrcAlpha);
                let d = pair
                    .get(1)
                    .and_then(Value::as_str)
                    .and_then(blend_from)
                    .unwrap_or(Blend::InvSrcAlpha);
                Some((s, d))
            }
            _ => None,
        };
        let alpha_cutoff = match m["alphaMode"].as_str() {
            Some("MASK") => Some(m["alphaCutoff"].as_f64().unwrap_or(0.5) as f32),
            _ => x["alpha_cutoff"].as_f64().map(|v| v as f32),
        };
        // a normal map named must be one of the level's textures: one that is not is refused, not dropped
        let normal = match &m["normalTexture"] {
            Value::Null => None,
            n => {
                let image = n["index"]
                    .as_u64()
                    .and_then(|t| tex_image.get(usize::try_from(t).ok()?).copied().flatten());
                match image.filter(|&i| i < level.textures.len()) {
                    Some(i) => Some(i),
                    None => {
                        return Err(
                            "a material's normal map is not one of the level's textures".into()
                        );
                    }
                }
            }
        };
        // a parallax depth needs a normal map (its alpha is the height) and a sane depth; anything else is refused
        let parallax = match &x["parallax"] {
            Value::Null => 0.0,
            p => match p
                .as_f64()
                .filter(|p| p.is_finite() && *p > 0.0 && *p <= MAX_PARALLAX)
            {
                Some(p) if normal.is_some() => p as f32,
                _ => {
                    return Err(
                        "a material's parallax depth is not a depth over its normal map".into(),
                    );
                }
            },
        };
        level.materials.push(Material {
            name: m["name"].as_str().unwrap_or_default().to_owned(),
            normal,
            parallax,
            diffuse: f4(&pbr["baseColorFactor"], [1.0; 4]),
            ambient: rgb(&x["ambient"]),
            emissive: rgb(&m["emissiveFactor"]),
            specular: rgb(&x["specular"]),
            power: x["power"]
                .as_f64()
                .filter(|p| p.is_finite())
                .map_or(0.0, |p| p.clamp(0.0, 1000.0) as f32),
            texture: pbr["baseColorTexture"]["index"]
                .as_u64()
                .and_then(|t| tex_image.get(t as usize).copied().flatten()),
            blend,
            alpha_cutoff,
            z_write: x["z_write"].as_bool().unwrap_or(blend.is_none()),
            two_sided: m["doubleSided"].as_bool().unwrap_or(false),
            unlit: m["extensions"].get("KHR_materials_unlit").is_some(),
            texgen: match x["texgen"].as_str() {
                Some("reflect") => Some(TexGen::Reflect),
                Some("chrome") => Some(TexGen::Chrome),
                _ => None,
            },
        });
    }

    // each mesh's finer look, by its index among the meshes (they come after all the level's own)
    let mut looks: Vec<Option<u64>> = Vec::new();
    for m in arr("meshes") {
        let prims = m["primitives"].as_array().cloned().unwrap_or_default();
        let first = prims.first().ok_or("mesh without primitives")?;
        let attr = &first["attributes"];
        let get = |k: &str| attr[k].as_u64().ok_or(format!("mesh without {k}"));
        let positions: Vec<[f32; 3]> = r
            .floats::<3>(get("POSITION")?, "VEC3")?
            .into_iter()
            .map(|p| [p[0], p[1], -p[2]])
            .collect();
        let normals: Vec<[f32; 3]> = r
            .floats::<3>(get("NORMAL")?, "VEC3")?
            .into_iter()
            .map(|p| [p[0], p[1], -p[2]])
            .collect();
        let uvs = r.floats::<2>(get("TEXCOORD_0")?, "VEC2")?;
        let mut parts = Vec::new();
        for p in &prims {
            if p["attributes"] != *attr {
                return Err("primitives of one mesh must share their vertices".into());
            }
            // a mesh of points has its vertices only: one primitive, no indices, no material
            if p.get("mode").is_some_and(|m| *m != json!(TRIANGLES)) {
                if p["mode"] != json!(POINTS)
                    || prims.len() != 1
                    || p.get("indices").is_some()
                    || p.get("material").is_some()
                {
                    return Err(
                        "a primitive must be triangles, or the points of a mesh alone".into(),
                    );
                }
                continue;
            }
            let (bytes, count) = r.accessor(
                p["indices"].as_u64().ok_or("primitive without indices")?,
                UINT,
                "SCALAR",
                4,
            )?;
            if count % 3 != 0 {
                return Err("index count is not a multiple of three".into());
            }
            let idx: Vec<u32> = bytes
                .chunks_exact(4)
                .map(|b| <[u8; 4]>::try_from(b).map(u32::from_le_bytes).unwrap_or(0))
                .collect();
            if idx.iter().any(|&i| i as usize >= positions.len()) {
                return Err("index out of range".into());
            }
            let indices = idx
                .chunks_exact(3)
                .flat_map(|t| [t[0], t[2], t[1]])
                .collect();
            let material = p["material"]
                .as_u64()
                .map(|m| m as usize)
                .filter(|&m| m < level.materials.len());
            parts.push(Part { material, indices });
        }
        if normals.len() != positions.len() || uvs.len() != positions.len() {
            return Err("vertex attributes of different lengths".into());
        }
        let mut channels = Vec::new();
        let listed = m["extras"]["drahtseilakt"]["channels"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if listed.len() > nmo::scene::MAX_CHANNELS {
            return Err("too many channels on a mesh".into());
        }
        for c in &listed {
            let material = c["material"]
                .as_u64()
                .map(|v| v as usize)
                .filter(|&v| v < level.materials.len())
                .ok_or("channel without a material")?;
            let b = |k: usize| {
                c["blend"]
                    .get(k)
                    .and_then(Value::as_str)
                    .and_then(blend_from)
            };
            let (Some(s), Some(d)) = (b(0), b(1)) else {
                return Err("channel without its blend".into());
            };
            let k = c["texcoord"]
                .as_u64()
                .ok_or("channel without texture coordinates")?;
            let acc = attr[format!("TEXCOORD_{k}")]
                .as_u64()
                .ok_or("channel coordinates missing")?;
            let uvs = r.floats::<2>(acc, "VEC2")?;
            if uvs.len() != positions.len() {
                return Err("channel coordinates of another length".into());
            }
            channels.push(Channel {
                material,
                blend: (s, d),
                uvs,
            });
        }
        looks.push(m["extras"]["drahtseilakt"]["look"].as_u64());
        level.meshes.push(Mesh {
            name: m["name"].as_str().unwrap_or_default().to_owned(),
            positions,
            normals,
            uvs,
            parts,
            channels,
            look: None,
        });
    }
    attach_looks(&mut level.meshes, &looks)?;

    for n in arr("nodes") {
        let mut m = [[0.0f32; 4]; 4];
        // the whole matrix of the extras, when it has entries that are not finite; glTF's otherwise
        let whole = &n["extras"]["drahtseilakt"]["matrix"];
        let (flat, named) = match whole.as_array() {
            Some(a) => (a, true),
            None => (n["matrix"].as_array().ok_or("node without matrix")?, false),
        };
        if flat.len() != 16 {
            return Err("node matrix must have 16 numbers".into());
        }
        for (k, v) in flat.iter().enumerate() {
            if let Some(slot) = m.get_mut(k / 4).and_then(|c| c.get_mut(k % 4)) {
                *slot = match v.as_f64() {
                    Some(x) => x as f32,
                    None if named => name_to_number(v).ok_or("matrix entry is not a number")?,
                    None => return Err("matrix entry is not a number".into()),
                };
            }
        }
        level.objects.push(Object {
            parent: None,
            name: n["name"].as_str().unwrap_or_default().to_owned(),
            matrix: flip_matrix(&m),
            mesh: n["mesh"]
                .as_u64()
                .map(|i| i as usize)
                .filter(|&i| i < level.meshes.len()),
            visible: n["extras"]["drahtseilakt"]["visible"]
                .as_bool()
                .unwrap_or(true),
            sprite: sprite_of(&n["extras"]["drahtseilakt"]["sprite"]),
        });
    }
    // the objects' parents, once every object is there: one that is no other object refuses the level, a loop is
    // broken as a map's is
    let count = level.objects.len();
    let mut parents: Vec<Option<usize>> = Vec::with_capacity(count);
    for (i, n) in arr("nodes").iter().enumerate() {
        let p = &n["extras"]["drahtseilakt"]["parent"];
        parents.push(if p.is_null() {
            None
        } else {
            let p = p
                .as_u64()
                .and_then(|p| usize::try_from(p).ok())
                .filter(|&p| p < count && p != i)
                .ok_or("a node's parent is no other node")?;
            Some(p)
        });
    }
    crate::from_nmo::break_parent_loops(&mut parents);
    for (o, p) in level.objects.iter_mut().zip(parents) {
        o.parent = p;
    }

    let groups = doc["scenes"][0]["extras"]["drahtseilakt"]["groups"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for g in groups {
        let members = g["members"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_u64)
                    .map(|i| i as usize)
                    .filter(|&i| i < level.objects.len())
                    .collect()
            })
            .unwrap_or_default();
        level.groups.push(Group {
            name: g["name"].as_str().unwrap_or_default().to_owned(),
            members,
        });
    }
    let x = &doc["scenes"][0]["extras"]["drahtseilakt"];
    level.sky = x["sky"].as_str().map(str::to_owned);
    level.start_ball = x["start_ball"].as_str().map(str::to_owned);
    level.settings = settings_from(&x["settings"])?;
    level.frames = frames_from(&x["frames"], level.materials.len())?;
    if !x["scripts"].is_null() {
        let counts = [
            level.objects.len(),
            level.groups.len(),
            level.meshes.len(),
            level.materials.len(),
            level.textures.len(),
        ];
        level.scripts = Some(std::sync::Arc::new(crate::script_json::from_json(
            &x["scripts"],
            counts,
        )?));
    }
    let mut bytes = text;
    bytes.extend_from_slice(&bin);
    Ok((level, bytes))
}
