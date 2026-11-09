// The open format keeps a mesh's material channels and a material's TexGen through export and import, and refuses
// channel lists that are too long or point nowhere (synthetic levels; no game data needed).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use level::{Blend, Channel, Level, Material, Mesh, Object, Part, TexGen};
use std::path::PathBuf;

fn material(name: &str, texgen: Option<TexGen>) -> Material {
    Material {
        name: name.into(),
        diffuse: [1.0; 4],
        ambient: [0.0; 3],
        emissive: [0.0; 3],
        specular: [0.0; 3],
        power: 0.0,
        texture: None,
        blend: None,
        alpha_cutoff: None,
        z_write: true,
        two_sided: false,
        unlit: false,
        texgen,
        normal: None,
        parallax: 0.0,
    }
}

/// A triangle drawn with material 0 and a channel of material 1 (ZERO / SRCCOLOR, its own coordinates).
fn level() -> Level {
    let mut l = Level::default();
    l.materials.push(material("base", None));
    l.materials.push(material("env", Some(TexGen::Chrome)));
    l.meshes.push(Mesh {
        name: "dome".into(),
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0; 2]; 3],
        parts: vec![Part {
            material: Some(0),
            indices: vec![0, 1, 2],
        }],
        look: None,
        channels: vec![Channel {
            material: 1,
            blend: (Blend::Zero, Blend::SrcColor),
            uvs: vec![[0.5, 0.5], [0.25, 0.75], [1.0, 0.0]],
        }],
    });
    l.objects.push(Object {
        name: "dome".into(),
        matrix: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        mesh: Some(0),
        visible: true,
        sprite: None,
    });
    l
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("drahtseilakt-open-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn channels_and_texgen_round_trip() {
    let d = dir("round");
    level::open::export(&level(), &d).unwrap();
    let back = level::open::import(&d).unwrap();
    assert_eq!(back.meshes[0].channels, level().meshes[0].channels);
    assert_eq!(back.materials[1].texgen, Some(TexGen::Chrome));
    assert_eq!(back.materials[0].texgen, None);
    let _ = std::fs::remove_dir_all(&d);
}

/// Rewrites the exported level's mesh extras and imports it again.
fn import_with(name: &str, channels: serde_json::Value) -> Result<Level, String> {
    let d = dir(name);
    level::open::export(&level(), &d).unwrap();
    let path = d.join("level.gltf");
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    doc["meshes"][0]["extras"]["drahtseilakt"]["channels"] = channels;
    std::fs::write(&path, doc.to_string()).unwrap();
    let r = level::open::import(&d);
    let _ = std::fs::remove_dir_all(&d);
    r
}

#[test]
fn bad_channel_lists_are_refused() {
    let one = serde_json::json!({"material": 1, "blend": ["zero", "src_color"], "texcoord": 1});
    assert!(import_with("ok", serde_json::json!([one.clone()])).is_ok());
    let many: Vec<serde_json::Value> = (0..=nmo::scene::MAX_CHANNELS)
        .map(|_| one.clone())
        .collect();
    assert!(import_with("many", serde_json::json!(many)).is_err());
    let nowhere = serde_json::json!({"material": 9, "blend": ["zero", "src_color"], "texcoord": 1});
    assert!(import_with("material", serde_json::json!([nowhere])).is_err());
    let no_uv = serde_json::json!({"material": 1, "blend": ["zero", "src_color"], "texcoord": 7});
    assert!(import_with("uv", serde_json::json!([no_uv])).is_err());
    let no_blend = serde_json::json!({"material": 1, "blend": ["zero"], "texcoord": 1});
    assert!(import_with("blend", serde_json::json!([no_blend])).is_err());
}
