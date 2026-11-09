// The open format keeps a material's normal map (glTF's normalTexture, which the toolbox writes for remastered
// levels) and refuses one that is not among the level's textures (synthetic levels; no game data needed).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use level::{Level, Material, Mesh, Object, Part, Texture, TextureSource};
use std::path::PathBuf;

fn level() -> Level {
    let mut l = Level::default();
    for name in ["Wood_Metal.bmp", "Wood_Metal_normal.png"] {
        l.textures.push(Texture {
            name: name.into(),
            source: TextureSource::File(name.into()),
            color_key: None,
        });
    }
    l.materials.push(Material {
        name: "panel".into(),
        diffuse: [1.0; 4],
        ambient: [0.0; 3],
        emissive: [0.0; 3],
        specular: [0.0; 3],
        power: 0.0,
        texture: Some(0),
        normal: Some(1),
        parallax: 0.0,
        blend: None,
        alpha_cutoff: None,
        z_write: true,
        two_sided: false,
        unlit: false,
        texgen: None,
    });
    l.meshes.push(Mesh {
        name: "panel".into(),
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0; 2], [1.0, 0.0], [0.0, 1.0]],
        parts: vec![Part {
            material: Some(0),
            indices: vec![0, 1, 2],
        }],
        channels: Vec::new(),
        look: None,
    });
    l.objects.push(Object {
        name: "panel".into(),
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
    let d = std::env::temp_dir().join(format!(
        "drahtseilakt-normals-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_normal_map_survives_the_round_trip() {
    let d = dir("trip");
    level::open::export(&level(), &d).unwrap();
    let back = level::open::import(&d).unwrap();
    assert_eq!(back.materials[0].texture, Some(0));
    assert_eq!(back.materials[0].normal, Some(1));
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("level.gltf")).unwrap()).unwrap();
    assert_eq!(
        doc["materials"][0]["normalTexture"]["index"], 1,
        "glTF's own field, as Blender reads it"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_normal_map_that_is_no_texture_is_refused() {
    for (name, bad) in [
        ("past", serde_json::json!({"index": 9})),
        ("huge", serde_json::json!({"index": u64::MAX})),
        ("none", serde_json::json!({"scale": 1.0})),
        ("text", serde_json::json!("1")),
    ] {
        let d = dir(name);
        level::open::export(&level(), &d).unwrap();
        let path = d.join("level.gltf");
        let mut doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        doc["materials"][0]["normalTexture"] = bad;
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
        assert!(level::open::import(&d).is_err(), "{name}");
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn a_parallax_depth_survives_and_bad_ones_are_refused() {
    let d = dir("parallax");
    let mut l = level();
    l.materials[0].parallax = 0.04;
    level::open::export(&l, &d).unwrap();
    let back = level::open::import(&d).unwrap();
    assert!((back.materials[0].parallax - 0.04).abs() < 1e-6);
    let path = d.join("level.gltf");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for (name, depth, keep_normal) in [
        ("negative", serde_json::json!(-0.1), true),
        ("deep", serde_json::json!(5.0), true),
        ("text", serde_json::json!("0.04"), true),
        ("no map", serde_json::json!(0.04), false),
    ] {
        let mut doc = doc.clone();
        doc["materials"][0]["extras"]["drahtseilakt"]["parallax"] = depth;
        if !keep_normal {
            doc["materials"][0]
                .as_object_mut()
                .unwrap()
                .remove("normalTexture");
        }
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
        assert!(level::open::import(&d).is_err(), "{name}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
