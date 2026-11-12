// Loading a level from a Virtools file into the engine's model. Objects that fail to decode are skipped and
// reported, so one broken object in a community map costs that object, not the level.

use crate::{
    Blend, Channel, Group, Level, Material, Mesh, Object, Part, Sprite, TexGen, Texture,
    TextureSource,
};
use nmo::{Document, class, scene};
use std::collections::HashMap;

pub struct Loaded {
    pub level: Level,
    /// Objects that could not be decoded, as readable messages.
    pub warnings: Vec<String>,
}

pub fn load(bytes: &[u8]) -> Result<Loaded, nmo::Error> {
    load_file(&nmo::read(bytes)?)
}

/// A level from a file whose container is already read.
pub fn load_file(file: &nmo::File) -> Result<Loaded, nmo::Error> {
    let doc = Document::parse(file)?;
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
                let source_of = |embedded: Option<(u32, u32, Vec<u8>)>, file_name: Option<Vec<u8>>| match (embedded, file_name) {
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
                let source = source_of(t.embedded, t.file_name);
                textures.insert(i, level.textures.len());
                if !t.slots.is_empty() {
                    let more = t
                        .slots
                        .into_iter()
                        .map(|s| source_of(s.embedded, s.file_name))
                        .collect();
                    level.texture_slots.push((level.textures.len(), more));
                }
                level.textures.push(Texture {
                    name: name(o),
                    source,
                    color_key: t
                        .transparent
                        .map(|c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]),
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
                let mut mat = material(name(o), &m, &textures);
                // TexGen (effect 1, or 2 with a referential, used as the view): its type is the parameter's value
                let effect = scene::material_effect(chunk).unwrap_or_else(|e| {
                    warn(o, e);
                    None
                });
                if let Some((1 | 2, Some(p))) = effect {
                    let kind = doc
                        .get(p)
                        .filter(|po| PARAMETER_CLASSES.contains(&po.class_id))
                        .and_then(|po| po.chunk.as_ref())
                        .and_then(|c| scene::parameter_word(c).ok().flatten());
                    mat.texgen = match kind {
                        Some(2) => Some(TexGen::Reflect),
                        Some(3) => Some(TexGen::Chrome),
                        _ => None,
                    };
                }
                level.materials.push(mat);
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
                // a bad channel section costs only the channels
                let channels = scene::mesh_channels(chunk, m.positions.len()).unwrap_or_else(|e| {
                    warn(o, e);
                    Vec::new()
                });
                level.meshes.push(mesh(name(o), m, &channels, &materials));
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
        class::SPRITE_3D,
    ];
    let mut objects = HashMap::new();
    let mut parents: Vec<(usize, usize)> = Vec::new();
    for (i, o) in doc.objects.iter().enumerate() {
        if !entity_classes.contains(&o.class_id) {
            continue;
        }
        let Some(chunk) = &o.chunk else { continue };
        match scene::entity(chunk) {
            Ok(e) => {
                let [x, y, z, p] = e.matrix;
                let hidden = chunk.section(0x4).is_some() || chunk.section(0x18).is_some();
                // A sprite is drawn as a quad of its own: a unit square with its texture rectangle and material.
                let (mut mesh_index, mut sprite) =
                    (e.mesh.and_then(|m| meshes.get(&m).copied()), None);
                if o.class_id == class::SPRITE_3D {
                    match scene::sprite(chunk) {
                        Ok(s) => {
                            let [u0, v0, u1, v1] = s.uv;
                            mesh_index = Some(level.meshes.len());
                            level.meshes.push(Mesh {
                                channels: Vec::new(),
                                look: None,
                                name: format!("{}_Sprite", name(o)),
                                positions: vec![
                                    [0.0, 0.0, 0.0],
                                    [0.0, 1.0, 0.0],
                                    [1.0, 1.0, 0.0],
                                    [1.0, 0.0, 0.0],
                                ],
                                normals: vec![[0.0, 0.0, -1.0]; 4],
                                uvs: vec![[u0, v1], [u0, v0], [u1, v0], [u1, v1]],
                                parts: vec![Part {
                                    material: s.material.and_then(|m| materials.get(&m).copied()),
                                    indices: vec![0, 1, 2, 0, 2, 3],
                                }],
                            });
                            sprite = Some(Sprite {
                                mode: s.mode,
                                size: s.size,
                                offset: s.offset,
                            });
                        }
                        Err(err) => warn(o, err),
                    }
                }
                objects.insert(i, level.objects.len());
                if let Some(p) = e.parent {
                    parents.push((level.objects.len(), p));
                }
                level.objects.push(Object {
                    parent: None,
                    name: name(o),
                    matrix: [
                        [x[0], x[1], x[2], 0.0],
                        [y[0], y[1], y[2], 0.0],
                        [z[0], z[1], z[2], 0.0],
                        [p[0], p[1], p[2], 1.0],
                    ],
                    mesh: mesh_index,
                    visible: !hidden,
                    sprite,
                });
            }
            Err(e) => warn(o, e),
        }
    }

    // Parents, now that every object has its index; a parent that is no object here (or a loop) is dropped, the
    // matrices being world ones already.
    for (child, p) in parents {
        if let Some(&par) = objects.get(&p)
            && par != child
        {
            level.objects[child].parent = Some(par);
        }
    }
    let mut parent_of: Vec<Option<usize>> = level.objects.iter().map(|o| o.parent).collect();
    break_parent_loops(&mut parent_of);
    for (o, p) in level.objects.iter_mut().zip(parent_of) {
        o.parent = p;
    }

    // The 2D frames (class 27; a 2D sprite carries a picture of its own and is left out): the rectangle in pixels of
    // the 640 x 480 render context, whichever way the file has it.
    for (_, o) in doc.of_class(class::ENTITY_2D) {
        if level.frames.len() >= crate::MAX_FRAMES {
            break;
        }
        let Some(chunk) = &o.chunk else { continue };
        match scene::entity_2d(chunk) {
            Ok(e) => {
                let [l, t, r, b] = e.rect;
                let rect = if e.homogeneous() {
                    [l * 640.0, t * 480.0, r * 640.0, b * 480.0]
                } else {
                    e.rect
                };
                let uv = if e.use_source() {
                    e.source
                } else {
                    [0.0, 0.0, 1.0, 1.0]
                };
                level.frames.push(crate::Frame {
                    name: name(o),
                    rect,
                    uv,
                    z_order: e.z_order,
                    material: e.material.and_then(|m| materials.get(&m).copied()),
                    visible: chunk.section(0x4).is_none() && chunk.section(0x18).is_none(),
                });
            }
            Err(e) => warn(o, e),
        }
    }

    let mut group_indices = HashMap::new();
    for (i, o) in doc.of_class(class::GROUP) {
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
        if level.groups.len() > group_indices.len() {
            group_indices.insert(i, level.groups.len().saturating_sub(1));
        }
    }
    // The map's own scripts, if any, with each file object's meaning to the engine.
    let kind = |i: usize| -> script::ObjKind {
        if let Some(&e) = objects.get(&i) {
            return script::ObjKind::Entity(e);
        }
        if let Some(&m) = meshes.get(&i) {
            return script::ObjKind::Mesh(m);
        }
        if let Some(&m) = materials.get(&i) {
            return script::ObjKind::Material(m);
        }
        if let Some(&t) = textures.get(&i) {
            return script::ObjKind::Texture(t);
        }
        if let Some(g) = group_indices.get(&i) {
            return script::ObjKind::Group(*g);
        }
        script::ObjKind::Other
    };
    let graph = script::Graph::from_document(doc, &kind);
    if !graph.scripts.is_empty() {
        level.scripts = Some(std::sync::Arc::new(graph));
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
        specular: rgb(m.specular),
        power: if m.specular_power.is_finite() {
            m.specular_power.clamp(0.0, 1000.0)
        } else {
            0.0
        },
        texture: m.texture.and_then(|t| textures.get(&t).copied()),
        blend: m
            .alpha_blend
            .then(|| (blend(m.src_blend), blend(m.dest_blend))),
        alpha_cutoff: m.alpha_test.then(|| f32::from(m.alpha_ref) / 255.0),
        z_write: m.z_write,
        two_sided: m.two_sided,
        unlit: false,
        texgen: None,
        normal: None,
        parallax: 0.0,
    }
}

/// Parameter classes (parameter in 2, out 3, local 45, parameter 46, as the game's files have them): what a material
/// effect's setting may be.
const PARAMETER_CLASSES: [u32; 4] = [2, 3, 45, 46];

fn mesh(
    name: String,
    m: scene::Mesh,
    channels: &[scene::Channel],
    materials: &HashMap<usize, usize>,
) -> Mesh {
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
    // the active channels whose material is here; a channel without its own coordinates uses the mesh's
    let channels = channels
        .iter()
        .filter(|c| c.flags & 1 != 0)
        .filter_map(|c| {
            let material = c.material.and_then(|i| materials.get(&i).copied())?;
            let uvs = if c.uvs.len() == m.positions.len() {
                c.uvs.clone()
            } else {
                uvs.clone()
            };
            Some(Channel {
                material,
                blend: (blend(c.src_blend), blend(c.dest_blend)),
                uvs,
            })
        })
        .collect();
    Mesh {
        name,
        positions: m.positions,
        normals,
        uvs,
        parts,
        channels,
        look: None,
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


/// Breaks every loop in a parent table by dropping the parent of the loop's lowest-numbered object (a parent that is
/// no object counts as none). One walk over each chain, each object visited once: linear in the table, whatever the
/// chains' length (a hostile file may chain a hundred thousand objects).
fn break_parent_loops(parent: &mut [Option<usize>]) {
    let n = parent.len();
    // 0 not seen, 1 on the chain being walked, 2 settled
    let mut state = vec![0u8; n];
    let mut path: Vec<usize> = Vec::new();
    for start in 0..n {
        if state[start] != 0 {
            continue;
        }
        path.clear();
        let mut at = start;
        loop {
            state[at] = 1;
            path.push(at);
            let Some(p) = parent[at].filter(|&p| p < n) else {
                parent[at] = None;
                break;
            };
            match state[p] {
                2 => break,
                1 => {
                    let from = path.iter().position(|&x| x == p).unwrap_or(0);
                    if let Some(&lowest) = path[from..].iter().min() {
                        parent[lowest] = None;
                    }
                    break;
                }
                _ => at = p,
            }
        }
        for &x in &path {
            state[x] = 2;
        }
    }
}

#[cfg(test)]
mod loop_tests {
    use super::break_parent_loops;

    #[test]
    fn loops_are_broken_and_chains_kept() {
        // 0 -> 1 -> 2 -> 0 (a loop), 3 -> 1 (a tail into it), 4 -> 5 (a chain), 6 -> itself
        let mut p = vec![Some(1), Some(2), Some(0), Some(1), Some(5), None, Some(6)];
        break_parent_loops(&mut p);
        assert_eq!(p, [None, Some(2), Some(0), Some(1), Some(5), None, None]);
    }

    #[test]
    fn a_hundred_thousand_long_chain_and_loop_are_fast() {
        let n = 100_000;
        let t = std::time::Instant::now();
        // a chain: i -> i + 1, kept whole
        let mut chain: Vec<Option<usize>> = (0..n).map(|i| (i + 1 < n).then_some(i + 1)).collect();
        let before = chain.clone();
        break_parent_loops(&mut chain);
        assert_eq!(chain, before);
        // the same chain closed into a loop: broken once, at object 0
        let mut ring: Vec<Option<usize>> = (0..n).map(|i| Some((i + 1) % n)).collect();
        break_parent_loops(&mut ring);
        assert_eq!(ring[0], None);
        assert!(ring[1..].iter().all(Option::is_some));
        // and the loop reached from the far end of a long tail, the tail in reverse order
        let mut tail: Vec<Option<usize>> = (0..n).map(|i| Some(i.saturating_sub(1))).collect();
        break_parent_loops(&mut tail);
        assert_eq!(tail[0], None);
        assert!(t.elapsed().as_secs_f32() < 2.0, "took {:?}", t.elapsed());
    }
}
