// The open format keeps what a converted map would otherwise lose: each object's parent (Set Parent in the map,
// which a script's moves follow) and the map's 2D frames; parents that point nowhere, frames that are malformed or
// too many refuse the level, and a loop of parents is broken as a map's is (synthetic levels; no game data needed).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use level::{Frame, Level, Material, Object};
use std::path::PathBuf;

fn object(name: &str, parent: Option<usize>) -> Object {
    Object {
        parent,
        name: name.into(),
        matrix: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [3.0, 4.0, 5.0, 1.0],
        ],
        mesh: None,
        visible: true,
        sprite: None,
    }
}

fn level() -> Level {
    let mut l = Level::default();
    l.materials.push(Material {
        name: "logo".into(),
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
        texgen: None,
        normal: None,
        parallax: 0.0,
    });
    l.objects = vec![
        object("base", None),
        object("arm", Some(0)),
        object("hand", Some(1)),
    ];
    l.frames = vec![
        Frame {
            name: "Signature".into(),
            rect: [500.0, 440.0, 640.0, 480.0],
            uv: [0.0, 0.0, 1.0, 0.5],
            z_order: -2,
            material: Some(0),
            visible: true,
        },
        Frame {
            name: "Hidden".into(),
            rect: [0.0, 0.0, 64.0, 64.0],
            uv: [0.0, 0.0, 1.0, 1.0],
            z_order: 3,
            material: None,
            visible: false,
        },
    ];
    l
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("drahtseilakt-open-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn parents_and_frames_round_trip() {
    let d = dir("parents-round");
    level::open::export(&level(), &d).unwrap();
    let back = level::open::import(&d).unwrap();
    let parents: Vec<_> = back.objects.iter().map(|o| o.parent).collect();
    assert_eq!(parents, vec![None, Some(0), Some(1)]);
    assert_eq!(back.frames, level().frames);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_matrix_that_is_not_finite_survives_and_gltf_keeps_numbers() {
    let mut l = level();
    l.objects[2].matrix[3] = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.0];
    let d = dir("parents-nan");
    level::open::export(&l, &d).unwrap();
    let j: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("level.gltf")).unwrap()).unwrap();
    assert!(
        j["nodes"][2]["matrix"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x.is_number())
    );
    let back = level::open::import(&d).unwrap();
    let [x, y, z, w] = back.objects[2].matrix[3];
    // z is negated on the way out and back (right- and left-handed): -inf stays -inf
    assert!(x.is_nan() && y == f32::INFINITY && z == f32::NEG_INFINITY && w == 1.0);
    assert_eq!(back.objects[0].matrix, l.objects[0].matrix);
    let _ = std::fs::remove_dir_all(&d);
    // a name that is no number refuses the level
    let r = edited("parents-nan-bad", |j| {
        j["nodes"][0]["extras"]["drahtseilakt"]["matrix"] = serde_json::json!(vec!["huge"; 16]);
    });
    assert!(r.is_err());
}

/// The level's JSON after `edit`.
fn edited(name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> Result<Level, String> {
    let d = dir(name);
    level::open::export(&level(), &d).unwrap();
    let path = d.join("level.gltf");
    let mut j: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    edit(&mut j);
    std::fs::write(&path, serde_json::to_string(&j).unwrap()).unwrap();
    let r = level::open::import(&d);
    let _ = std::fs::remove_dir_all(&d);
    r
}

#[test]
fn a_parent_that_is_no_other_object_refuses_the_level() {
    for bad in [
        serde_json::json!(3),
        serde_json::json!(1),
        serde_json::json!(-1),
        serde_json::json!("0"),
    ] {
        let r = edited("parents-bad", |j| {
            j["nodes"][1]["extras"]["drahtseilakt"]["parent"] = bad.clone();
        });
        assert!(r.is_err(), "{bad}");
    }
}

#[test]
fn a_loop_of_parents_is_broken() {
    let back = edited("parents-loop", |j| {
        j["nodes"][0]["extras"]["drahtseilakt"]["parent"] = serde_json::json!(2);
    })
    .unwrap();
    let parents: Vec<_> = back.objects.iter().map(|o| o.parent).collect();
    assert_eq!(parents, vec![None, Some(0), Some(1)]);
}

#[test]
fn malformed_or_too_many_frames_refuse_the_level() {
    let frames =
        |j: &mut serde_json::Value| j["scenes"][0]["extras"]["drahtseilakt"]["frames"].clone();
    for (name, edit) in [
        (
            "material",
            Box::new(|f: &mut serde_json::Value| f[0]["material"] = serde_json::json!(1))
                as Box<dyn Fn(&mut serde_json::Value)>,
        ),
        (
            "rect",
            Box::new(|f| f[0]["rect"] = serde_json::json!([1, 2, 3])),
        ),
        (
            "z",
            Box::new(|f| f[0]["z_order"] = serde_json::json!(1u64 << 40)),
        ),
        (
            "visible",
            Box::new(|f| f[0]["visible"] = serde_json::json!(1)),
        ),
        (
            "many",
            Box::new(|f| *f = serde_json::json!(vec![f[0].clone(); level::MAX_FRAMES + 1])),
        ),
    ] {
        let r = edited("frames-bad", |j| {
            let mut f = frames(j);
            edit(&mut f);
            j["scenes"][0]["extras"]["drahtseilakt"]["frames"] = f;
        });
        assert!(r.is_err(), "{name}");
    }
    // a level without frames (made before they were kept) has none
    let back = edited("frames-none", |j| {
        j["scenes"][0]["extras"]["drahtseilakt"]
            .as_object_mut()
            .map(|x| x.remove("frames"));
    })
    .unwrap();
    assert!(back.frames.is_empty());
}

#[test]
fn a_mesh_of_points_survives_and_points_beside_triangles_are_refused() {
    let mut l = level();
    l.meshes.push(level::Mesh {
        name: "Point Mesh".into(),
        positions: vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
        normals: vec![[0.0, 1.0, 0.0]; 2],
        uvs: vec![[0.0; 2]; 2],
        ..level::Mesh::default()
    });
    l.objects[0].mesh = Some(0);
    let d = dir("points");
    level::open::export(&l, &d).unwrap();
    let back = level::open::import(&d).unwrap();
    assert_eq!(back.meshes.len(), 1);
    assert_eq!(back.meshes[0].positions, l.meshes[0].positions);
    assert!(back.meshes[0].parts.is_empty());
    assert_eq!(back.objects[0].mesh, Some(0));
    let _ = std::fs::remove_dir_all(&d);
    let d = dir("points-bad");
    level::open::export(&l, &d).unwrap();
    let file = d.join("level.gltf");
    let mut j: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let p = j["meshes"][0]["primitives"][0].clone();
    j["meshes"][0]["primitives"] = serde_json::json!([p.clone(), p]);
    std::fs::write(&file, serde_json::to_string(&j).unwrap()).unwrap();
    assert!(level::open::import(&d).is_err());
    let _ = std::fs::remove_dir_all(&d);
}
