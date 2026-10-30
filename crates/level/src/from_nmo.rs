// Loading a level from a Virtools file into the engine's model. Objects that fail to decode are skipped and
// reported, so one broken object in a community map costs that object, not the level.

use crate::{Blend, Group, Level, Material, Mesh, Object, Part, Texture, TextureSource};
use nmo::{Document, class, scene};
use std::collections::HashMap;

pub struct Loaded {
    pub level: Level,
    /// Objects that could not be decoded, as readable messages.
    pub warnings: Vec<String>,
}

pub fn load(bytes: &[u8]) -> Result<Loaded, nmo::Error> {
    let file = nmo::read(bytes)?;
    let doc = Document::parse(&file)?;
    Ok(build(&doc))
}

fn name(o: &nmo::Object) -> String {
    o.name_lossy()
}

fn build(doc: &Document) -> Loaded {
    let mut level = Level::default();
    let mut warnings = Vec::new();
    let mut warn = |o: &nmo::Object, e: nmo::Error| warnings.push(format!("{}: {e}", name(o)));

    // First pass: give every texture, material and mesh its index in the model.
    let mut textures = HashMap::new();
    for (i, o) in doc.of_class(class::TEXTURE) {
        let Some(chunk) = &o.chunk else { continue };
        match scene::texture(chunk) {
            Ok(t) => {
                let source = match (t.embedded, t.file_name) {
                    (Some((width, height, rgba)), _) => TextureSource::Pixels {
                        width,
                        height,
                        rgba,
                    },
                    (None, Some(file)) => {
                        TextureSource::File(String::from_utf8_lossy(&file).into_owned())
                    }
                    (None, None) => TextureSource::None,
                };
                textures.insert(i, level.textures.len());
                level.textures.push(Texture {
                    name: name(o),
                    source,
                });
            }
            Err(e) => warn(o, e),
        }
    }

    let mut materials = HashMap::new();
    for (i, o) in doc.of_class(class::MATERIAL) {
        let Some(chunk) = &o.chunk else { continue };
        match scene::material(chunk) {
            Ok(m) => {
                materials.insert(i, level.materials.len());
                level.materials.push(material(name(o), &m, &textures));
            }
            Err(e) => warn(o, e),
        }
    }

    let mut meshes = HashMap::new();
    for (i, o) in doc.of_class(class::MESH) {
        let Some(chunk) = &o.chunk else { continue };
        match scene::mesh(chunk) {
            Ok(m) => {
                meshes.insert(i, level.meshes.len());
                level.meshes.push(mesh(name(o), m, &materials));
            }
            Err(e) => warn(o, e),
        }
    }

    // Placed objects: every 3D entity class carries a world matrix.
    let entity_classes = [
        class::OBJECT_3D,
        class::ENTITY_3D,
        class::CAMERA,
        class::TARGET_CAMERA,
        class::LIGHT,
        class::TARGET_LIGHT,
    ];
    let mut objects = HashMap::new();
    for (i, o) in doc.objects.iter().enumerate() {
        if !entity_classes.contains(&o.class_id) {
            continue;
        }
        let Some(chunk) = &o.chunk else { continue };
        match scene::entity(chunk) {
            Ok(e) => {
                let [x, y, z, p] = e.matrix;
                let hidden = chunk.section(0x4).is_some() || chunk.section(0x18).is_some();
                objects.insert(i, level.objects.len());
                level.objects.push(Object {
                    name: name(o),
                    matrix: [
                        [x[0], x[1], x[2], 0.0],
                        [y[0], y[1], y[2], 0.0],
                        [z[0], z[1], z[2], 0.0],
                        [p[0], p[1], p[2], 1.0],
                    ],
                    mesh: e.mesh.and_then(|m| meshes.get(&m).copied()),
                    visible: !hidden,
                });
            }
            Err(e) => warn(o, e),
        }
    }

    for (_, o) in doc.of_class(class::GROUP) {
        let Some(chunk) = &o.chunk else { continue };
        match scene::group(chunk) {
            Ok(members) => level.groups.push(Group {
                name: name(o),
                members: members
                    .iter()
                    .filter_map(|m| objects.get(m).copied())
                    .collect(),
            }),
            Err(e) => warn(o, e),
        }
    }

    Loaded { level, warnings }
}

fn argb(c: u32) -> [f32; 4] {
    let ch = |shift: u32| f32::from(c.checked_shr(shift).unwrap_or(0) as u8) / 255.0;
    [ch(16), ch(8), ch(0), ch(24)]
}

fn rgb(c: u32) -> [f32; 3] {
    let [r, g, b, _] = argb(c);
    [r, g, b]
}

fn blend(b: scene::Blend) -> Blend {
    match b {
        scene::Blend::Zero => Blend::Zero,
        scene::Blend::One | scene::Blend::SrcAlphaSat => Blend::One,
        scene::Blend::SrcColor => Blend::SrcColor,
        scene::Blend::InvSrcColor => Blend::InvSrcColor,
        scene::Blend::SrcAlpha => Blend::SrcAlpha,
        scene::Blend::InvSrcAlpha => Blend::InvSrcAlpha,
        scene::Blend::DestAlpha => Blend::DestAlpha,
        scene::Blend::InvDestAlpha => Blend::InvDestAlpha,
        scene::Blend::DestColor => Blend::DestColor,
        scene::Blend::InvDestColor => Blend::InvDestColor,
    }
}

fn material(name: String, m: &scene::Material, textures: &HashMap<usize, usize>) -> Material {
    Material {
        name,
        diffuse: argb(m.diffuse),
        ambient: rgb(m.ambient),
        emissive: rgb(m.emissive),
        texture: m.texture.and_then(|t| textures.get(&t).copied()),
        blend: m
            .alpha_blend
            .then(|| (blend(m.src_blend), blend(m.dest_blend))),
        alpha_cutoff: m.alpha_test.then(|| f32::from(m.alpha_ref) / 255.0),
        z_write: m.z_write,
        two_sided: m.two_sided,
    }
}

fn mesh(name: String, m: scene::Mesh, materials: &HashMap<usize, usize>) -> Mesh {
    let mut parts: Vec<Part> = Vec::new();
    for (idx, slot) in &m.faces {
        let material = m
            .materials
            .get(usize::from(*slot))
            .copied()
            .flatten()
            .and_then(|i| materials.get(&i).copied());
        let part = match parts.iter_mut().position(|p| p.material == material) {
            Some(p) => p,
            None => {
                parts.push(Part {
                    material,
                    indices: vec![],
                });
                parts.len().saturating_sub(1)
            }
        };
        if let Some(p) = parts.get_mut(part) {
            p.indices.extend(idx.iter().map(|&i| u32::from(i)));
        }
    }
    let normals = if m.normals.len() == m.positions.len() {
        m.normals
    } else {
        smooth_normals(&m.positions, &m.faces)
    };
    let uvs = if m.uvs.len() == m.positions.len() {
        m.uvs
    } else {
        vec![[0.0; 2]; m.positions.len()]
    };
    Mesh {
        name,
        positions: m.positions,
        normals,
        uvs,
        parts,
    }
}

/// Vertex normals as the area-weighted average of the faces around each vertex.
fn smooth_normals(positions: &[[f32; 3]], faces: &[([u16; 3], u16)]) -> Vec<[f32; 3]> {
    let mut acc = vec![[0.0f32; 3]; positions.len()];
    let p = |i: u16| positions.get(usize::from(i)).copied().unwrap_or_default();
    for (idx, _) in faces {
        let [a, b, c] = idx.map(p);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for i in idx {
            if let Some(s) = acc.get_mut(usize::from(*i)) {
                *s = [s[0] + n[0], s[1] + n[1], s[2] + n[2]];
            }
        }
    }
    acc.into_iter()
        .map(|[x, y, z]| {
            let len = (x * x + y * y + z * z).sqrt();
            if len > 1e-12 {
                [x / len, y / len, z / len]
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect()
}
