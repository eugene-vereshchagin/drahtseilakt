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
    let data = Data {
        root: copy,
        overlay: None,
    }
    .for_level(Some(&level));
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
    let data = Data {
        root: copy,
        overlay: None,
    };
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
    assert_eq!(std::fs::read(game.join("Sounds/Hit.wav")).unwrap(), b"sound");
    let _ = std::fs::remove_dir_all(&r);
}

/// A map's texture names as a `.nmo` gives them (raw strings, any path a stranger wrote): only their last part is
/// ever looked for, in the game's texture folders; an absolute or `..` path never reads a file elsewhere, neither to
/// draw nor to export.
#[test]
fn a_maps_texture_name_never_reads_a_path() {
    let r = root("texnames");
    let (copy, outside) = (r.join("copy"), r.join("outside"));
    std::fs::create_dir_all(copy.join("Textures")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let px = |c: [u8; 4]| image::RgbaImage::from_pixel(2, 2, image::Rgba(c));
    px([200, 0, 0, 255]).save(copy.join("Textures/Inside.png")).unwrap();
    px([0, 200, 0, 255]).save(outside.join("Secret.png")).unwrap();
    let data = Data {
        root: copy.clone(),
        overlay: None,
    };
    let named = |name: String| level::Texture {
        name: "T".into(),
        source: level::TextureSource::File(name),
        color_key: None,
    };
    let secret = outside.join("Secret.png");
    let names = [
        secret.to_string_lossy().into_owned(),
        "../outside/Secret.png".to_owned(),
        "Textures/../../outside/Secret.png".to_owned(),
        "C:\\Pictures\\..\\Secret.png".to_owned(),
        "..".to_owned(),
        String::new(),
    ];
    for n in &names {
        assert!(data.texture(&named(n.clone())).is_none(), "{n} read a file");
    }
    // a path's last part is a texture of the game's folders
    let (_, _, rgba) = data.texture(&named(format!("{}/Inside.png", outside.display()))).unwrap();
    assert_eq!(&rgba[..4], &[200, 0, 0, 255]);
    let (_, _, rgba) = data.texture(&named("D:\\Maps\\Inside.png".into())).unwrap();
    assert_eq!(&rgba[..4], &[200, 0, 0, 255]);

    // the same names in a level exported to the open format: only the copy's texture is written
    let mut lvl = level::Level::default();
    lvl.textures = names.iter().cloned().map(named).collect();
    lvl.textures.push(named("/x/Inside.png".into()));
    let loaded = gamedata::assets::LoadedLevel {
        level: lvl,
        name: "Map.nmo".into(),
        hash: 0,
        warnings: Vec::new(),
    };
    let out = r.join("export");
    gamedata::assets::export(&data, &loaded, &out).unwrap();
    let written: Vec<_> = std::fs::read_dir(out.join("textures")).unwrap().flatten().collect();
    assert_eq!(written.len(), 1, "only the copy's texture");
    let img = image::open(written[0].path()).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(0, 0).0, [200, 0, 0, 255]);
    let _ = std::fs::remove_dir_all(&r);
}

/// An open-format level read from a relative folder still knows its textures as its own files (by full path).
#[cfg(unix)]
#[test]
fn an_open_levels_textures_are_its_own_files() {
    let r = root("openown");
    let out = r.join("lvl");
    let mut lvl = level::Level::default();
    lvl.textures.push(level::Texture {
        name: "A".into(),
        source: level::TextureSource::Pixels { width: 1, height: 1, rgba: vec![1, 2, 3, 255] },
        color_key: None,
    });
    level::open::export(&lvl, &out).unwrap();
    std::fs::create_dir_all(out.join("textures")).unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]))
        .save(out.join(level::open::texture_file(0, "A")))
        .unwrap();
    // the folder named relatively to where the program runs
    let cwd = std::env::current_dir().unwrap();
    let mut rel = PathBuf::new();
    for _ in cwd.components().skip(1) {
        rel.push("..");
    }
    let rel = rel.join(out.strip_prefix("/").unwrap_or(&out));
    let back = level::open::import(&rel).unwrap();
    match &back.textures[0].source {
        level::TextureSource::LevelFile(p) => assert!(p.is_absolute() && p.starts_with(&out)),
        other => panic!("{other:?}"),
    }
    let _ = std::fs::remove_dir_all(&r);
}
