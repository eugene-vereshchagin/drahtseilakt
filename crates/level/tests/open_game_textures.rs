// The open format's `game:` images (open-format.md): a texture named as the game's, resolved through the texture packs
// and never read as a path. Written and read back as a name; a name that could be a path is refused (synthetic levels;
// no game data needed).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use level::{Level, Material, Mesh, Object, Part, Texture, TextureSource};

fn level() -> Level {
    let mut l = Level::default();
    l.textures.push(Texture {
        name: "Track_Side".into(),
        source: TextureSource::Game("Track_Side".into()),
        color_key: None,
    });
    l.materials.push(Material {
        name: "side".into(),
        diffuse: [1.0; 4],
        ambient: [0.0; 3],
        emissive: [0.0; 3],
        specular: [0.0; 3],
        power: 0.0,
        texture: Some(0),
        normal: None,
        parallax: 0.0,
        blend: None,
        alpha_cutoff: None,
        z_write: true,
        two_sided: false,
        unlit: false,
        texgen: None,
    });
    l.meshes.push(Mesh {
        name: "side".into(),
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
        parent: None,
        name: "side".into(),
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

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "drahtseilakt-game-tex-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_game_texture_is_kept_as_a_name() {
    let d = dir("round");
    level::open::export(&level(), &d).unwrap();
    let text = std::fs::read_to_string(d.join("level.gltf")).unwrap();
    assert!(text.contains("\"game:Track_Side\""));
    let (back, _) = level::open::import_read(&d).unwrap();
    assert!(matches!(&back.textures[0].source, TextureSource::Game(n) if n == "Track_Side"));
    assert!(!d.join("textures").exists(), "no file is written for it");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_game_texture_that_could_be_a_path_is_refused() {
    let d = dir("bad");
    level::open::export(&level(), &d).unwrap();
    let text = std::fs::read_to_string(d.join("level.gltf")).unwrap();
    let gltf = text.clone().into_bytes();
    let bin = std::fs::read(d.join("level.bin")).unwrap();
    assert!(level::open::import_bytes(&d, &gltf, &bin).is_ok());
    let long = "x".repeat(129);
    for bad in [
        "game:",
        "game:../x",
        "game:a/b",
        "game:a\\\\b",
        "game:C:x",
        "game:.hidden",
        "game:a\\u0007b",
        &format!("game:{long}"),
    ] {
        let t = text.replace("\"game:Track_Side\"", &format!("\"{bad}\""));
        assert!(
            level::open::import_bytes(&d, t.as_bytes(), &bin).is_err(),
            "{bad}"
        );
    }
    // a name with spaces or other letters is a name
    for good in ["game:Track Side", "game:Дерево"] {
        let t = text.replace("\"game:Track_Side\"", &format!("\"{good}\""));
        assert!(
            level::open::import_bytes(&d, t.as_bytes(), &bin).is_ok(),
            "{good}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn what_a_material_stands_for_in_the_original_is_kept_and_checked() {
    let d = dir("original");
    let mut l = level();
    l.originals.push(level::Original {
        material: 0,
        texture: "Rail_Environment".into(),
        texgen: Some(level::TexGen::Chrome),
    });
    level::open::export(&l, &d).unwrap();
    let (back, _) = level::open::import_read(&d).unwrap();
    assert_eq!(back.originals, l.originals);
    let text = std::fs::read_to_string(d.join("level.gltf")).unwrap();
    let bin = std::fs::read(d.join("level.bin")).unwrap();
    let said = "\"texture\": \"Rail_Environment\"";
    assert!(text.contains(said), "{text}");
    // a name that could be a path, or an unknown way of drawing it: the level is refused
    for bad in ["\"texture\": \"../x\"", "\"texture\": 7"] {
        let t = text.replace(said, bad);
        assert!(
            level::open::import_bytes(&d, t.as_bytes(), &bin).is_err(),
            "{bad}"
        );
    }
    let t = text.replace("\"texgen\": \"chrome\"", "\"texgen\": \"mirror\"");
    assert!(level::open::import_bytes(&d, t.as_bytes(), &bin).is_err());
    let _ = std::fs::remove_dir_all(&d);
}
