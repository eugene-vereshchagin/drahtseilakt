// Fuzz targets of the level readers (docs/drahtseilakt/fuzzing.md), on the stable fuzzer in crates/fuzzkit:
//   nmo_file    a whole .nmo/.cmo: container, object table, every scene decoder on every object, the level model;
//   nmo_chunk   one state chunk through every scene decoder;
//   open_level  the open format: level.gltf and level.bin;
//   script_json a converted map's behaviour graphs (the scene's `scripts` extras), read and run a few ticks.
// They are #[ignore]: `FUZZ_SECS=600 cargo test --release --offline -p level --test fuzz -- --ignored`. Seeds are
// synthetic files made here, plus the player's own files when present (read at run time, never committed). The
// committed repros in fuzz/regressions/ are replayed by the ordinary (not ignored) tests at the end.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use fuzzkit::{Kind, Target, test_name};
use level::{
    Blend, Channel, Group, Level, Material, Mesh, Object, Part, Settings, Sprite, TexGen, Texture,
};
use nmo::chunk::{Chunk, OPTION_FILE};
use nmo::{Document, class, scene};
use std::path::PathBuf;

// --- nmo ---------------------------------------------------------------------------------------------------------

/// Every decoder on every object, whatever its class.
fn decode_all(doc: &Document) {
    for o in &doc.objects {
        if let Some(c) = &o.chunk {
            decode_chunk(c);
        }
    }
}

/// What the renderer uploads as it is: an embedded picture of at least one pixel, its bytes exactly its size (a
/// zero-sized texture is a wgpu validation error, which panics).
fn check_pixels(w: u32, h: u32, rgba: &[u8]) {
    assert!(w >= 1 && h >= 1, "an embedded picture of {w}x{h}");
    assert_eq!(
        rgba.len() as u64,
        u64::from(w) * u64::from(h) * 4,
        "an embedded picture's bytes"
    );
}

fn decode_chunk(c: &Chunk) {
    if let Ok(t) = scene::texture(c)
        && let Some((w, h, rgba)) = &t.embedded
    {
        check_pixels(*w, *h, rgba);
    }
    let _ = scene::mesh(c);
    let _ = scene::mesh_channels(c, scene::mesh(c).map_or(64, |m| m.positions.len()));
    let _ = scene::material(c);
    let _ = scene::texture(c);
    let _ = scene::entity(c);
    let _ = scene::curve(c);
    let _ = scene::curve_point(c);
    let _ = scene::camera(c);
    let _ = scene::sprite(c);
    let _ = scene::entity_2d(c);
    let _ = scene::wave_sound(c);
    let _ = scene::material_effect(c);
    let _ = scene::parameter_word(c);
    let _ = scene::group(c);
    let _ = scene::data_array(c);
    let _ = scene::object_animation(c);
    let _ = scene::parameter_floats(c);
}

pub fn run_nmo_file(bytes: &[u8]) {
    let Ok(file) = nmo::read(bytes) else { return };
    if let Ok(doc) = Document::parse(&file) {
        decode_all(&doc);
    }
    if let Ok(loaded) = level::from_nmo::load_file(&file) {
        // the model is not much larger than the file it came from: no few words make megabytes
        let floats: usize = loaded
            .level
            .meshes
            .iter()
            .map(|m| {
                m.positions.len() * 3
                    + m.normals.len() * 3
                    + m.uvs.len() * 2
                    + m.channels.iter().map(|c| c.uvs.len() * 2).sum::<usize>()
                    + m.parts.iter().map(|p| p.indices.len()).sum::<usize>()
            })
            .sum();
        let input = file.header1.len() + file.data.len();
        assert!(
            floats * 4 <= 64 * input + (1 << 20),
            "a model of {floats} numbers from {input} bytes"
        );
        for t in &loaded.level.textures {
            if let level::TextureSource::Pixels {
                width,
                height,
                rgba,
            } = &t.source
            {
                check_pixels(*width, *height, rgba);
            }
        }
    }
}

pub fn run_nmo_chunk(bytes: &[u8]) {
    if let Ok(c) = Chunk::parse(bytes, nmo::Part::Data) {
        decode_chunk(&c);
    }
}

fn chunk_raw(c: &Chunk) -> Vec<u8> {
    let version = u32::from(c.data_version)
        | u32::from(c.class_id) << 8
        | u32::from(c.chunk_version) << 16
        | u32::from(c.options & OPTION_FILE) << 24;
    let mut out = version.to_le_bytes().to_vec();
    out.extend_from_slice(&(c.words.len() as u32).to_le_bytes());
    c.words
        .iter()
        .for_each(|w| out.extend_from_slice(&w.to_le_bytes()));
    out
}

/// A chunk of the given sections ([identifier, next, data...] linked).
fn chunk_of(sections: &[(u32, Vec<u32>)]) -> Chunk {
    let mut words = Vec::new();
    for (k, (id, data)) in sections.iter().enumerate() {
        let next = words.len() + 2 + data.len();
        words.push(*id);
        words.push(if k + 1 == sections.len() {
            0
        } else {
            next as u32
        });
        words.extend(data);
    }
    Chunk {
        data_version: 7,
        class_id: 0,
        chunk_version: 7,
        options: OPTION_FILE,
        words,
    }
}

/// Plausible small payloads under every identifier the decoders read.
fn synthetic_chunks() -> Vec<Chunk> {
    let f = |v: f32| v.to_bits();
    let small: Vec<u32> = (0..24)
        .map(|i| if i % 3 == 0 { 2 } else { f(i as f32 * 0.5) })
        .collect();
    let ids = [
        0x0010_0000u32,
        0x0002_0000,
        0x0001_0000,
        0x0000_4000,
        0x1000,
        0x002F_F000,
        0x4000,
        0xffc0_0000,
        0x1000_0000,
        0x0400_0000,
        0x0fc0_0000,
        0x0040_0000,
        0x40,
        0x000F_FFFF,
        0x2000,
    ];
    let mut out: Vec<Chunk> = ids
        .iter()
        .map(|&id| chunk_of(&[(id, small.clone())]))
        .collect();
    // a mesh: flags, 3 vertices, 1 face
    let mut verts = vec![0u32, 3];
    for i in 0..3 {
        verts.extend([f(i as f32), f(0.0), f(1.0)]);
    }
    out.push(chunk_of(&[
        (0x0010_0000, verts),
        (0x0001_0000, vec![1, 0, 1, 2, 0]),
        (0x2000, vec![1, 2, 3]),
    ]));
    out.push(chunk_of(&[(
        0x1000,
        (0..40).map(|i| f(i as f32 / 40.0)).collect(),
    )]));
    out
}

const CLASSES: [u32; 14] = [
    class::MESH,
    class::MATERIAL,
    class::TEXTURE,
    class::ENTITY_3D,
    class::OBJECT_3D,
    class::GROUP,
    class::DATA_ARRAY,
    class::CURVE,
    class::CAMERA,
    class::TARGET_CAMERA,
    class::SPRITE_3D,
    class::OBJECT_ANIMATION,
    class::LIGHT,
    class::LEVEL,
];

/// A plain (uncompressed) file of the given header 1, data, counts and included files.
fn plain_file(
    h1: &[u8],
    data: &[u8],
    managers: u32,
    objects: u32,
    included: &[nmo::IncludedFile],
) -> Vec<u8> {
    let fields: [u32; 14] = [
        0,
        0x1302_2002,
        8,
        0,
        0,
        h1.len() as u32,
        data.len() as u32,
        data.len() as u32,
        managers,
        objects,
        objects + 1,
        0,
        0x0201_0001,
        h1.len() as u32,
    ];
    let mut out = b"Nemo Fi\0".to_vec();
    fields
        .iter()
        .for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
    out.extend_from_slice(h1);
    out.extend_from_slice(data);
    for i in included {
        out.extend_from_slice(&(i.name.len() as u32).to_le_bytes());
        out.extend_from_slice(&i.name);
        out.extend_from_slice(&(i.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&i.data);
    }
    out
}

/// A synthetic file: one object of each class over the synthetic chunks.
fn synthetic_nmo() -> Vec<Vec<u8>> {
    let chunks = synthetic_chunks();
    let mut out = Vec::new();
    for round in 0..3usize {
        let (mut h1, mut data) = (Vec::new(), Vec::new());
        // one manager
        data.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(&7u32.to_le_bytes());
        let mut n = 0u32;
        for (k, c) in chunks.iter().enumerate() {
            let class = CLASSES[(k + round) % CLASSES.len()];
            let name = format!("P_Box_{k:02}");
            h1.extend_from_slice(&(k as u32 + 1).to_le_bytes());
            h1.extend_from_slice(&class.to_le_bytes());
            h1.extend_from_slice(&0u32.to_le_bytes());
            h1.extend_from_slice(&(name.len() as u32).to_le_bytes());
            h1.extend_from_slice(name.as_bytes());
            let raw = chunk_raw(c);
            data.extend_from_slice(&(raw.len() as u32).to_le_bytes());
            data.extend_from_slice(&raw);
            n += 1;
        }
        out.push(plain_file(&h1, &data, 1, n, &[]));
    }
    out
}

fn real_nmo() -> Vec<Vec<u8>> {
    let paths = fuzzkit::files_under(&fuzzkit::real_roots(), &["nmo", "cmo"], 24, 8 << 20);
    fuzzkit::read_all(&paths)
}

/// A real file as stored, and again written plain: the mutations then reach the object table and chunks directly
/// instead of the compressed stream.
fn nmo_seeds() -> Vec<Vec<u8>> {
    let mut out = synthetic_nmo();
    for bytes in real_nmo() {
        if let Ok(f) = nmo::read(&bytes) {
            let h = &f.header;
            out.push(plain_file(
                &f.header1,
                &f.data,
                h.manager_count,
                h.object_count,
                &f.included,
            ));
        }
        out.push(bytes);
    }
    out
}

fn chunk_seeds() -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = synthetic_chunks().iter().map(chunk_raw).collect();
    let mut seen = std::collections::HashSet::new();
    let mut per_class: std::collections::HashMap<u32, usize> = Default::default();
    for bytes in real_nmo() {
        let Ok(f) = nmo::read(&bytes) else { continue };
        let Ok(doc) = Document::parse(&f) else {
            continue;
        };
        for o in &doc.objects {
            let Some(c) = &o.chunk else { continue };
            let n = per_class.entry(o.class_id).or_default();
            if *n >= 60 || c.words.len() > 64 * 1024 {
                continue;
            }
            let raw = chunk_raw(c);
            if seen.insert(fuzzkit::fnv(&raw)) {
                *n += 1;
                out.push(raw);
            }
        }
    }
    out
}

#[test]
#[ignore]
fn fuzz_nmo_file() {
    let mut t = Target::new("nmo_file", test_name!("fuzz_nmo_file"), Kind::Binary);
    t.max_len = 16 << 20;
    fuzzkit::fuzz(t, nmo_seeds, run_nmo_file);
}

#[test]
#[ignore]
fn fuzz_nmo_chunk() {
    let t = Target::new("nmo_chunk", test_name!("fuzz_nmo_chunk"), Kind::Binary);
    fuzzkit::fuzz(t, chunk_seeds, run_nmo_chunk);
}

// --- the open format ---------------------------------------------------------------------------------------------

pub fn run_open_level(input: &[u8]) {
    let parts = fuzzkit::split_parts(input, 2);
    if let Ok(l) = level::open::import_bytes(&fuzzkit::scratch(), parts[0], parts[1]) {
        // no more vertex data than the buffer holds, twice over (accessors may be shared)
        let numbers: usize = l
            .meshes
            .iter()
            .flat_map(|m| std::iter::once(m).chain(m.look.as_deref()))
            .map(|m| {
                m.positions.len() * 3
                    + m.normals.len() * 3
                    + m.uvs.len() * 2
                    + m.channels.iter().map(|c| c.uvs.len() * 2).sum::<usize>()
                    + m.parts.iter().map(|p| p.indices.len()).sum::<usize>()
            })
            .sum();
        assert!(
            numbers * 4 <= 2 * parts[1].len() + (1 << 16),
            "{numbers} numbers from a buffer of {}",
            parts[1].len()
        );
        // what the game takes as it is: points it adds, a light colour, a theme number
        let s = &l.settings;
        assert!(
            s.level_bonus
                .is_none_or(|b| b.unsigned_abs() <= 1_000_000_000),
            "level bonus {:?}",
            s.level_bonus
        );
        assert!(s.music.is_none_or(|m| m <= 1000));
        assert!(
            s.light
                .is_none_or(|c| c.iter().all(|x| (0.0..=1.0).contains(x)))
        );
    }
}

/// A small level using every feature the format has.
fn synthetic_level() -> Level {
    let mut l = Level::default();
    let mut mesh = Mesh {
        name: "m".into(),
        positions: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        normals: vec![[0.0, 0.0, 1.0]; 4],
        uvs: vec![[0.0, 1.0]; 4],
        parts: vec![
            Part {
                material: Some(0),
                indices: vec![0, 1, 2],
            },
            Part {
                material: Some(1),
                indices: vec![1, 3, 2],
            },
        ],
        channels: vec![Channel {
            material: 1,
            blend: (Blend::SrcAlpha, Blend::One),
            uvs: vec![[0.5, 0.5]; 4],
        }],
        look: None,
    };
    let mut look = mesh.clone();
    look.name = "m_look".into();
    look.channels.clear();
    mesh.look = Some(Box::new(look));
    l.meshes.push(mesh);
    for k in 0..2 {
        l.textures.push(Texture {
            name: format!("t{k}"),
            source: level::TextureSource::File(format!("t{k}.bmp")),
            color_key: (k == 0).then_some([0, 0, 0]),
        });
        l.materials.push(Material {
            name: format!("mat{k}"),
            diffuse: [1.0, 0.5, 0.25, 1.0],
            ambient: [0.1; 3],
            emissive: [0.0; 3],
            specular: [0.5; 3],
            power: 10.0,
            texture: Some(k),
            normal: (k == 1).then_some(0),
            parallax: if k == 1 { 0.05 } else { 0.0 },
            blend: (k == 1).then_some((Blend::SrcAlpha, Blend::InvSrcAlpha)),
            alpha_cutoff: (k == 0).then_some(0.5),
            z_write: k == 0,
            two_sided: k == 1,
            unlit: k == 1,
            texgen: (k == 1).then_some(TexGen::Chrome),
        });
    }
    let id = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [1.0, 2.0, 3.0, 1.0],
    ];
    l.objects.push(Object {
        name: "P_Box_01".into(),
        matrix: id,
        mesh: Some(0),
        visible: true,
        sprite: None,
    });
    l.objects.push(Object {
        name: "Sprite".into(),
        matrix: id,
        mesh: None,
        visible: false,
        sprite: Some(Sprite {
            mode: 1,
            size: [1.0, 2.0],
            offset: [0.0, 0.5],
        }),
    });
    l.groups.push(Group {
        name: "Phys_Floors".into(),
        members: vec![0, 1],
    });
    l.sky = Some("A".into());
    l.start_ball = Some("Ball_Wood".into());
    l.settings = Settings {
        music: Some(2),
        light: Some([1.0, 0.9, 0.8]),
        level_bonus: Some(1000),
        tutorial: true,
        last_level: false,
        source_hash: Some(0x1234),
        own_parts: true,
        fog: None,
        clouds: None,
        sector_balls: vec![None, Some("paper".into())],
    };
    l
}

fn read_open(dir: &std::path::Path) -> Option<Vec<u8>> {
    let gltf = std::fs::read(dir.join("level.gltf")).ok()?;
    let bin = std::fs::read(dir.join("level.bin")).ok()?;
    Some(fuzzkit::join_parts(&[&gltf, &bin]))
}

fn open_seeds() -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let d = fuzzkit::scratch().join("seed");
    let _ = std::fs::create_dir_all(&d);
    if level::open::export(&synthetic_level(), &d).is_ok()
        && let Some(s) = read_open(&d)
    {
        out.push(s);
    }
    // the player's converted levels (open/ and remastered/ in the profile), small ones
    let mut roots = Vec::new();
    if std::env::var_os("FUZZ_NO_REAL").is_none()
        && let Some(home) = std::env::var_os("HOME")
    {
        let levels = PathBuf::from(home).join(".local/share/drahtseilakt/levels");
        roots.extend([levels.join("open"), levels.join("remastered")]);
    }
    for gltf in fuzzkit::files_under(&roots, &["gltf"], 12, 4 << 20) {
        let Some(dir) = gltf.parent() else { continue };
        if std::fs::metadata(dir.join("level.bin")).is_ok_and(|m| m.len() <= 8 << 20)
            && let Some(s) = read_open(dir)
        {
            out.push(s);
        }
    }
    out
}

#[test]
#[ignore]
fn fuzz_open_level() {
    let mut t = Target::new("open_level", test_name!("fuzz_open_level"), Kind::Text);
    t.max_len = 16 << 20;
    t.dict = &[
        "\"accessors\"",
        "\"bufferViews\"",
        "\"byteOffset\"",
        "\"byteLength\"",
        "\"byteStride\"",
        "\"count\"",
        "\"componentType\"",
        "\"VEC2\"",
        "\"VEC3\"",
        "\"SCALAR\"",
        "5126",
        "5125",
        "\"TEXCOORD_1\"",
        "\"look\"",
        "\"channels\"",
        "\"parallax\"",
        "\"normalTexture\"",
        "\"BLEND\"",
        "\"MASK\"",
        "\"matrix\"",
        "\"members\"",
        "\"sprite\"",
        "\"settings\"",
        "textures/",
        "\"textures/../x\"",
        "null",
        "true",
        "false",
        "[]",
        "{}",
    ];
    fuzzkit::fuzz(t, open_seeds, run_open_level);
}

// --- script_json -------------------------------------------------------------------------------------------------

/// A host with nothing in it: the scripts run against it and ask for what they like.
struct Nothing;

impl script::Host for Nothing {
    fn delta_ms(&self) -> f32 {
        16.0
    }
    fn matrix(&self, _: script::Ent) -> Option<[[f32; 4]; 4]> {
        Some([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
    fn initial_matrix(&self, e: script::Ent) -> Option<[[f32; 4]; 4]> {
        self.matrix(e)
    }
    fn set_matrix(&mut self, _: script::Ent, _: [[f32; 4]; 4]) {}
    fn set_visible(&mut self, _: script::Ent, _: bool) {}
    fn bounds(&self, _: script::Ent) -> Option<([f32; 3], [f32; 3])> {
        Some(([-1.0; 3], [1.0; 3]))
    }
    fn delete(&mut self, _: script::Ent) {}
    fn physicalize(&mut self, _: script::Ent, _: bool) {}
    fn particles(&mut self, _: script::Ent, _: bool) {}
    fn mesh_of(&self, _: script::Ent) -> Option<usize> {
        None
    }
    fn group_members(&self, _: usize) -> Vec<usize> {
        vec![0, 1, 2]
    }
    fn game_cell(&self, _: &str, _: usize, _: usize) -> Option<script::Value> {
        None
    }
    fn message(&mut self, _: &str, _: Option<script::Ent>) {}
    fn sound(&mut self, _: &str, _: bool) {}
    fn text(&mut self, _: &str, _: &str, _: bool) {}
    fn note(&mut self, _: &str) {}
}

/// The graphs from JSON text, then run: a graph that is accepted must not stall, nor grow without bound.
pub fn run_script_json(input: &[u8]) {
    let Ok(j) = serde_json::from_slice::<serde_json::Value>(input) else {
        return;
    };
    let Ok(g) = level::script_json::from_json(&j, [8, 4, 4, 4, 4]) else {
        return;
    };
    let mut rt = script::Runtime::new(std::sync::Arc::new(g));
    let mut host = Nothing;
    for _ in 0..4 {
        rt.tick(&mut host);
    }
    let ids: Vec<usize> = rt.graph.scripts.iter().copied().take(8).collect();
    for id in ids {
        let _ = rt.read(id, &mut host);
        rt.write(id, script::Value::Int(1));
    }
}

fn script_seeds() -> Vec<Vec<u8>> {
    use script::{Array, Block, Graph, Guid, Link, Node, ObjInfo, ObjKind, Op, Param, Value};
    let obj = |n: &str, kind| ObjInfo {
        name: n.into(),
        class: 0,
        kind,
    };
    let objects = vec![
        obj("Script", ObjKind::Behavior),
        obj("Sub", ObjKind::Behavior),
        obj("In", ObjKind::Other),
        obj("Out", ObjKind::Other),
        obj("Link", ObjKind::Other),
        obj("A", ObjKind::Other),
        obj("B", ObjKind::Other),
        obj("Op", ObjKind::Other),
        obj("Thing", ObjKind::Entity(0)),
    ];
    let root = Block {
        name: "Script".into(),
        flags: 2,
        sub: vec![1],
        links: vec![4],
        inputs: vec![2],
        outputs: vec![3],
        ..Block::default()
    };
    let sub = Block {
        name: "Sub".into(),
        proto: Some(Guid(7, 8)),
        pins: vec![5],
        pouts: vec![6],
        ops: vec![7],
        ..Block::default()
    };
    let param = |n: &str, v: Value, dest: Vec<usize>| Param {
        name: n.into(),
        guid: script::types::INT,
        value: v,
        destinations: dest,
        ..Param::default()
    };
    let g = Graph::assemble(
        objects,
        vec![
            (0, Node::Block(root)),
            (1, Node::Block(sub)),
            (
                4,
                Node::Link(Link {
                    delay: 2,
                    src: 2,
                    dst: 3,
                }),
            ),
            (5, Node::Param(param("A", Value::Int(2), vec![6]))),
            (6, Node::Param(param("B", Value::Str("x".into()), vec![]))),
            (
                7,
                Node::Op(Op {
                    guid: script::ops::op::ADD,
                    inputs: [Some(5), Some(5)],
                    output: Some(6),
                }),
            ),
        ],
        vec![Array {
            name: "T".into(),
            columns: vec![("c".into(), 1)],
            rows: vec![vec![Value::Int(1)], vec![Value::Vec3([1.0, 2.0, 3.0])]],
            key: Some(0),
        }],
        vec![(0, Some(8))],
    );
    let mut out = Vec::new();
    if let Ok(g) = g {
        let j = level::script_json::to_json(&g, &Some, &Some);
        out.push(serde_json::to_vec(&j).unwrap_or_default());
    }
    // the player's converted levels' scripts, when present
    if std::env::var_os("FUZZ_NO_REAL").is_none()
        && let Some(home) = std::env::var_os("HOME")
    {
        let levels = PathBuf::from(home).join(".local/share/drahtseilakt/levels");
        for gltf in fuzzkit::files_under(
            &[levels.join("open"), levels.join("remastered")],
            &["gltf"],
            12,
            4 << 20,
        ) {
            if let Ok(bytes) = std::fs::read(gltf)
                && let Ok(doc) = serde_json::from_slice::<serde_json::Value>(&bytes)
                && let s = &doc["scenes"][0]["extras"]["drahtseilakt"]["scripts"]
                && !s.is_null()
            {
                out.push(serde_json::to_vec(s).unwrap_or_default());
            }
        }
    }
    out
}

#[test]
#[ignore]
fn fuzz_script_json() {
    let mut t = Target::new("script_json", test_name!("fuzz_script_json"), Kind::Text);
    t.max_len = 1 << 20;
    t.dict = &[
        "\"objects\"",
        "\"nodes\"",
        "\"arrays\"",
        "\"scripts\"",
        "\"block\"",
        "\"link\"",
        "\"io\"",
        "\"param\"",
        "\"op\"",
        "\"sub\"",
        "\"dest\"",
        "\"source\"",
        "\"links\"",
        "\"inputs\"",
        "\"outputs\"",
        "\"pins\"",
        "\"pouts\"",
        "\"locals\"",
        "\"ops\"",
        "\"target\"",
        "\"proto\"",
        "\"entity\"",
        "\"group\"",
        "\"array\"",
        "[\"s\",",
        "[\"o\",",
        "[\"i\",",
        "[\"m\",",
        "null",
        "[]",
        "0",
        "1",
        "99999999",
    ];
    fuzzkit::fuzz(t, script_seeds, run_script_json);
}

// --- committed repros --------------------------------------------------------------------------------------------

#[test]
fn fuzz_regressions() {
    fuzzkit::regressions("nmo_file", run_nmo_file);
    fuzzkit::regressions("nmo_chunk", run_nmo_chunk);
    fuzzkit::regressions("open_level", run_open_level);
    fuzzkit::regressions("script_json", run_script_json);
}

/// The synthetic seeds are what they mean to be (a seed the reader refuses would fuzz only its first checks).
#[test]
fn fuzz_seeds_are_read() {
    for s in synthetic_nmo() {
        let f = nmo::read(&s).unwrap();
        let doc = Document::parse(&f).unwrap();
        assert!(doc.objects.iter().all(|o| o.chunk.is_some()));
        level::from_nmo::load_file(&f).unwrap();
    }
    let open = open_seeds();
    let p = fuzzkit::split_parts(&open[0], 2);
    let l = level::open::import_bytes(&fuzzkit::scratch(), p[0], p[1]).unwrap();
    assert_eq!(l.meshes.len(), 1);
    assert!(l.meshes[0].look.is_some());
    // the script seed is read by the reader (else the target fuzzes only its first checks)
    let seeds = script_seeds();
    let j: serde_json::Value = serde_json::from_slice(&seeds[0]).unwrap();
    assert!(level::script_json::from_json(&j, [8, 4, 4, 4, 4]).is_ok());
}
