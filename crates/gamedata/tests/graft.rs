// A game file's level takes the looks and textures of the toolbox's remade copy only when the copy matches it, and
// its own meshes never change (the physics uses them). Synthetic levels; no game data needed.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use level::{Level, Mesh, Part, Texture, TextureSource};

fn mesh(name: &str, faces: bool) -> Mesh {
    Mesh {
        name: name.into(),
        positions: if faces {
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        } else {
            Vec::new()
        },
        normals: if faces {
            vec![[0.0, 0.0, 1.0]; 3]
        } else {
            Vec::new()
        },
        uvs: if faces { vec![[0.0; 2]; 3] } else { Vec::new() },
        parts: vec![Part {
            material: None,
            indices: if faces { vec![0, 1, 2] } else { Vec::new() },
        }],
        channels: Vec::new(),
        look: None,
    }
}

/// A part file: an empty mesh (the open format leaves it out), then two meshes, and one texture.
fn part() -> Level {
    let mut l = Level::default();
    l.meshes.push(mesh("empty", false));
    l.meshes.push(mesh("ball", true));
    l.meshes.push(mesh("ring", true));
    l.textures.push(Texture {
        name: "Ball_Wood.bmp".into(),
        source: TextureSource::File("Ball_Wood.bmp".into()),
        color_key: None,
    });
    l
}

fn root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("drahtseilakt-graft-{}", std::process::id()))
}

/// A remade part's folder inside the test's root, with its textures folder and a texture file of 4x4.
fn dir(name: &str) -> std::path::PathBuf {
    let d = root().join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("textures")).unwrap();
    let png = d.join(level::open::texture_file(0, "Ball_Wood.bmp"));
    image::save_buffer(&png, &[255u8; 64], 4, 4, image::ExtendedColorType::Rgba8).unwrap();
    d
}

#[test]
fn a_matching_remade_copy_gives_its_looks_and_textures() {
    let d = dir("match");
    let mut remade = part();
    let mut look = mesh("ball_look", true);
    look.positions[0] = [0.0, 0.0, 0.25];
    remade.meshes[1].look = Some(Box::new(look));
    level::open::export(&remade, &d).unwrap();
    let mut l = part();
    gamedata::assets::graft_within(&mut l, &root(), &d);
    assert_eq!(
        l.meshes[1].look.as_ref().unwrap().positions[0],
        [0.0, 0.0, 0.25]
    );
    assert_eq!(
        l.meshes[1].positions[0],
        [0.0, 0.0, 0.0],
        "the mesh itself is unchanged"
    );
    assert!(l.meshes[0].look.is_none() && l.meshes[2].look.is_none());
    // the texture now names the remade copy's file
    assert!(
        matches!(&l.textures[0].source, TextureSource::File(p) if p.starts_with(d.to_str().unwrap()))
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(unix)]
#[test]
fn a_remade_copy_reached_through_a_link_is_left_unused() {
    let d = dir("linked_target");
    let mut remade = part();
    remade.meshes[1].look = Some(Box::new(mesh("ball_look", true)));
    level::open::export(&remade, &d).unwrap();
    let link = root().join("linked");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&d, &link).unwrap();
    let mut l = part();
    gamedata::assets::graft_within(&mut l, &root(), &link);
    assert!(l.meshes.iter().all(|m| m.look.is_none()));
    let _ = std::fs::remove_file(&link);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_remade_copy_that_does_not_match_is_left_unused() {
    let d = dir("other");
    let mut remade = part();
    remade.meshes[2].name = "something else".into();
    remade.meshes[1].look = Some(Box::new(mesh("ball_look", true)));
    level::open::export(&remade, &d).unwrap();
    let mut l = part();
    gamedata::assets::graft_within(&mut l, &root(), &d);
    assert!(l.meshes.iter().all(|m| m.look.is_none()));
    assert!(matches!(&l.textures[0].source, TextureSource::File(p) if p == "Ball_Wood.bmp"));
    // and none at all is no harm, nor one outside the root
    gamedata::assets::graft_within(&mut l, &root(), &root().join("none"));
    gamedata::assets::graft_within(&mut l, &root().join("other"), &d);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_path_from_a_level_never_leaves_the_copy() {
    let root = root().join("copy");
    std::fs::create_dir_all(root.join("3D Entities/PH")).unwrap();
    std::fs::write(root.join("3D Entities/PH/P_Box.nmo"), b"").unwrap();
    std::fs::write(root.join("outside.nmo"), b"").unwrap();
    let data = gamedata::assets::Data {
        root: root.join("3D Entities"),
    };
    assert!(data.file("PH/P_Box.nmo").is_some());
    assert!(data.file("ph/p_box.NMO").is_some(), "any case");
    for bad in [
        "../outside.nmo",
        "PH/../../outside.nmo",
        "PH/./P_Box.nmo",
        "PH//P_Box.nmo",
        "PH\\P_Box.nmo",
        "C:/x",
    ] {
        assert!(data.file(bad).is_none(), "{bad}");
    }
    let _ = std::fs::remove_dir_all(&root);
}
