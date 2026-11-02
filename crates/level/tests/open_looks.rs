// The open format keeps a mesh's finer look (what the toolbox makes for remastered levels) through export and import,
// apart from the mesh the physics uses, and refuses looks that point at the level's own meshes, at nothing, or twice
// at one mesh (synthetic levels; no game data needed).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use level::{Level, Mesh, Object, Part};
use std::path::PathBuf;

fn triangle(name: &str, z: f32) -> Mesh {
    Mesh {
        name: name.into(),
        positions: vec![[0.0, 0.0, z], [1.0, 0.0, z], [0.0, 1.0, z]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0; 2]; 3],
        parts: vec![Part {
            material: None,
            indices: vec![0, 1, 2],
        }],
        channels: Vec::new(),
        look: None,
    }
}

/// Two meshes placed by two objects; the first has a finer look.
fn level() -> Level {
    let mut l = Level::default();
    let mut round = triangle("round", 0.0);
    round.look = Some(Box::new(triangle("round_look", 0.5)));
    l.meshes.push(round);
    l.meshes.push(triangle("flat", 2.0));
    for (k, name) in ["round", "flat"].into_iter().enumerate() {
        l.objects.push(Object {
            name: name.into(),
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            mesh: Some(k),
            visible: true,
            sprite: None,
        });
    }
    l
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("drahtseilakt-looks-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_look_survives_the_round_trip_beside_its_mesh() {
    let d = dir("trip");
    level::open::export(&level(), &d).unwrap();
    let back = level::open::import(&d).unwrap();
    // the level's own meshes only, the objects on them as before
    assert_eq!(back.meshes.len(), 2);
    assert_eq!(back.objects[0].mesh, Some(0));
    assert_eq!(back.objects[1].mesh, Some(1));
    let look = back.meshes[0].look.as_ref().expect("the look is kept");
    assert_eq!(look.name, "round_look");
    assert_eq!(look.positions[0], [0.0, 0.0, 0.5]);
    assert_eq!(
        back.meshes[0].positions[0],
        [0.0, 0.0, 0.0],
        "the mesh itself is unchanged"
    );
    assert!(back.meshes[1].look.is_none());
    let _ = std::fs::remove_dir_all(&d);
}

/// The exported level with its first mesh's look index replaced by `look`.
fn with_look(name: &str, look: serde_json::Value) -> PathBuf {
    let d = dir(name);
    level::open::export(&level(), &d).unwrap();
    let path = d.join("level.gltf");
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    doc["meshes"][0]["extras"]["drahtseilakt"]["look"] = look;
    std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
    d
}

#[test]
fn looks_that_point_wrong_are_refused() {
    // at the level's own mesh, at itself, past the meshes, and an extra mesh no one's look
    for (name, look) in [
        ("own", serde_json::json!(1)),
        ("self", serde_json::json!(0)),
        ("past", serde_json::json!(99)),
        ("huge", serde_json::json!(u64::MAX)),
    ] {
        let d = with_look(name, look);
        assert!(level::open::import(&d).is_err(), "{name}");
        let _ = std::fs::remove_dir_all(&d);
    }
    let d = with_look("twice", serde_json::json!(2));
    let path = d.join("level.gltf");
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    doc["meshes"][1]["extras"]["drahtseilakt"]["look"] = serde_json::json!(2);
    std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
    assert!(level::open::import(&d).is_err(), "one look for two meshes");
    let _ = std::fs::remove_dir_all(&d);
    // a mesh among the looks that is no mesh's look
    let d = with_look("orphan", serde_json::json!(2));
    let path = d.join("level.gltf");
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let extra = doc["meshes"][2].clone();
    doc["meshes"].as_array_mut().unwrap().push(extra);
    std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
    assert!(
        level::open::import(&d).is_err(),
        "an orphan among the looks"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(unix)]
#[test]
fn level_files_that_are_links_are_refused() {
    let d = dir("link");
    level::open::export(&level(), &d).unwrap();
    std::fs::rename(d.join("level.bin"), d.join("real.bin")).unwrap();
    std::os::unix::fs::symlink(d.join("real.bin"), d.join("level.bin")).unwrap();
    assert!(level::open::import(&d).is_err(), "a buffer that is a link");
    std::fs::remove_file(d.join("level.bin")).unwrap();
    std::os::unix::fs::symlink("/dev/zero", d.join("level.bin")).unwrap();
    assert!(
        level::open::import(&d).is_err(),
        "a buffer that is a device"
    );
    let _ = std::fs::remove_dir_all(&d);
}
