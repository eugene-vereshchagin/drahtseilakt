// A level folder's own game files (`<level>/game`) are found before the copy's, never through a link or out of the
// folder, and a path from a level never leaves the copy (synthetic folders; no game data needed).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use gamedata::assets::Data;
use std::path::PathBuf;

fn root(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("drahtseilakt-files-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_level_folders_own_files_come_first() {
    let r = root("first");
    let (copy, level) = (r.join("copy"), r.join("level"));
    std::fs::create_dir_all(copy.join("Sounds")).unwrap();
    std::fs::create_dir_all(level.join("game/Sounds")).unwrap();
    std::fs::write(copy.join("Sounds/Hit.wav"), b"copy").unwrap();
    std::fs::write(copy.join("Sounds/Only.wav"), b"copy").unwrap();
    std::fs::write(level.join("game/Sounds/Hit.wav"), b"level").unwrap();
    let data = Data {
        root: copy.clone(),
        overlay: None,
    }
    .for_level(Some(&level));
    assert_eq!(
        std::fs::read(data.file("Sounds/hit.WAV").unwrap()).unwrap(),
        b"level",
        "any case"
    );
    assert_eq!(
        std::fs::read(data.file("Sounds/Only.wav").unwrap()).unwrap(),
        b"copy",
        "else the copy's"
    );
    let plain = Data {
        root: copy,
        overlay: None,
    };
    assert_eq!(
        std::fs::read(plain.file("Sounds/Hit.wav").unwrap()).unwrap(),
        b"copy"
    );
    let _ = std::fs::remove_dir_all(&r);
}

#[cfg(unix)]
#[test]
fn a_link_in_a_level_folder_leads_nowhere() {
    let r = root("link");
    let (copy, level, outside) = (r.join("copy"), r.join("level"), r.join("outside"));
    std::fs::create_dir_all(&copy).unwrap();
    std::fs::create_dir_all(level.join("game/Textures")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("Hit.wav"), b"outside").unwrap();
    std::os::unix::fs::symlink(&outside, level.join("game/Sounds")).unwrap();
    std::os::unix::fs::symlink(outside.join("Hit.wav"), level.join("game/Textures/A.bmp")).unwrap();
    let data = Data {
        root: copy,
        overlay: None,
    }
    .for_level(Some(&level));
    assert!(data.file("Sounds/Hit.wav").is_none(), "a linked folder");
    assert!(data.file("Textures/A.bmp").is_none(), "a linked file");
    let _ = std::fs::remove_dir_all(&r);
}

#[test]
fn a_path_from_a_level_never_leaves_the_copy() {
    let r = root("paths");
    let copy = r.join("copy");
    std::fs::create_dir_all(copy.join("3D Entities/PH")).unwrap();
    std::fs::write(copy.join("3D Entities/PH/P_Box.nmo"), b"").unwrap();
    std::fs::write(r.join("outside.nmo"), b"").unwrap();
    let level = r.join("level");
    std::fs::create_dir_all(level.join("game")).unwrap();
    let data = Data {
        root: copy.join("3D Entities"),
        overlay: None,
    }
    .for_level(Some(&level));
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
    let _ = std::fs::remove_dir_all(&r);
}

#[test]
fn a_part_comes_from_the_level_folder_in_the_open_format() {
    let r = root("part");
    let (copy, level) = (r.join("copy"), r.join("level"));
    std::fs::create_dir_all(&copy).unwrap();
    let mut part = level::Level::default();
    part.objects.push(level::Object {
        name: "Ball_Wood".into(),
        matrix: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        mesh: None,
        visible: true,
        sprite: None,
    });
    level::open::export(&part, &level.join("game/3D Entities/Balls")).unwrap();
    let data = Data {
        root: copy.clone(),
        overlay: None,
    }
    .for_level(Some(&level));
    let got = data
        .part("3D Entities/Balls.nmo")
        .expect("the level's own balls");
    assert_eq!(got.level.objects[0].name, "Ball_Wood");
    assert!(
        Data {
            root: copy,
            overlay: None
        }
        .part("3D Entities/Balls.nmo")
        .is_none(),
        "none in the copy"
    );
    let _ = std::fs::remove_dir_all(&r);
}

#[cfg(unix)]
#[test]
fn a_game_file_is_copied_in_never_through_a_link() {
    let r = root("copy_into");
    let (game, outside) = (r.join("game"), r.join("outside"));
    std::fs::create_dir_all(&game).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let src = r.join("Hit.wav");
    std::fs::write(&src, b"sound").unwrap();
    gamedata::assets::copy_into(&src, &game, "Sounds/Hit.wav").unwrap();
    assert_eq!(
        std::fs::read(game.join("Sounds/Hit.wav")).unwrap(),
        b"sound"
    );
    std::os::unix::fs::symlink(&outside, game.join("Textures")).unwrap();
    assert!(
        gamedata::assets::copy_into(&src, &game, "Textures/A.bmp").is_err(),
        "a linked folder"
    );
    std::os::unix::fs::symlink(outside.join("x"), game.join("Sounds/B.wav")).unwrap();
    assert!(
        gamedata::assets::copy_into(&src, &game, "Sounds/B.wav").is_err(),
        "a link in its place"
    );
    assert!(
        gamedata::assets::copy_into(&src, &game, "../x.wav").is_err(),
        "out of the folder"
    );
    assert!(
        std::fs::read_dir(&outside).unwrap().next().is_none(),
        "nothing written outside"
    );
    let _ = std::fs::remove_dir_all(&r);
}
