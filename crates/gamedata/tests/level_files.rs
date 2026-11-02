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
    let data = Data::new(Some(copy.clone())).for_level(Some(&level));
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
    let plain = Data::new(Some(copy));
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
    let data = Data::new(Some(copy)).for_level(Some(&level));
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
    let data = Data::new(Some(copy.join("3D Entities"))).for_level(Some(&level));
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
    let data = Data::new(Some(copy.clone())).for_level(Some(&level));
    let got = data
        .part("3D Entities/Balls.nmo")
        .expect("the level's own balls");
    assert_eq!(got.level.objects[0].name, "Ball_Wood");
    // the copy has none (our own balls may stand in for them, as for any part)
    assert!(
        !Data::new(Some(copy)).copy_has("3D Entities/Balls.nmo"),
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

#[test]
fn a_level_folder_never_stands_in_for_the_interface() {
    let r = root("interface");
    let (copy, level) = (r.join("copy"), r.join("level"));
    for rel in [
        "3D Entities/Menu.nmo",
        "3D Entities/Levelinit.nmo",
        "3D Entities/Language.nmo",
        "Sounds/Menu_click.wav",
        "Sounds/Hit.wav",
        "3D Entities/PH/P_Box.nmo",
        "Textures/Sky/Sky_A_Front.bmp",
    ] {
        for (base, what) in [(&copy, "copy"), (&level.join("game"), "level")] {
            let p = base.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, what).unwrap();
        }
    }
    let data = Data::new(Some(copy)).for_level(Some(&level));
    let from = |rel: &str| std::fs::read(data.file(rel).unwrap()).unwrap();
    // the interface's files are the copy's, whatever a level folder holds
    for rel in [
        "3D Entities/Menu.nmo",
        "3D Entities/Levelinit.nmo",
        "3D Entities/Language.nmo",
        "Sounds/Menu_click.wav",
    ] {
        assert_eq!(from(rel), b"copy", "{rel}");
    }
    // the level's own parts, sky and sounds are its own
    for rel in [
        "Sounds/Hit.wav",
        "3D Entities/PH/P_Box.nmo",
        "Textures/Sky/Sky_A_Front.bmp",
    ] {
        assert_eq!(from(rel), b"level", "{rel}");
    }
    let _ = std::fs::remove_dir_all(&r);
}

#[test]
fn an_open_level_without_settings_says_so() {
    let r = root("settings");
    let (copy, level) = (r.join("copy"), r.join("Somewhere"));
    std::fs::create_dir_all(&copy).unwrap();
    level::open::export(&level::Level::default(), &level).unwrap();
    let data = Data::new(Some(copy));
    let loaded = data
        .load(&gamedata::assets::Source::Open(level.clone()))
        .unwrap();
    assert!(
        loaded.warnings.iter().any(|w| w.contains("no settings")),
        "{:?}",
        loaded.warnings
    );
    let _ = std::fs::remove_dir_all(&r);
}

#[cfg(unix)]
#[test]
fn copying_a_game_file_never_writes_through_a_hard_link() {
    let r = root("hardlink");
    let (src, game, victim) = (r.join("Hit.wav"), r.join("game"), r.join("victim.txt"));
    std::fs::write(&src, b"sound").unwrap();
    std::fs::write(&victim, b"keep").unwrap();
    std::fs::create_dir_all(game.join("Sounds")).unwrap();
    // a stranger's folder with the copy's neighbour already there, hard-linked to someone's file
    std::fs::hard_link(&victim, game.join("Sounds/Hit.wav.new")).unwrap();
    std::fs::hard_link(&victim, game.join("Sounds/Hit.wav")).unwrap();
    gamedata::assets::copy_into(&src, &game, "Sounds/Hit.wav").unwrap();
    assert_eq!(std::fs::read(&victim).unwrap(), b"keep");
    assert_eq!(
        std::fs::read(game.join("Sounds/Hit.wav")).unwrap(),
        b"sound"
    );
    let _ = std::fs::remove_dir_all(&r);
}
