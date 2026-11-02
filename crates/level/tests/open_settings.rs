// An open-format level carries its settings (what the original takes from its level table and its rules by level
// number) through export and import, and refuses malformed ones (synthetic levels; no game data needed).

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use level::{Level, Settings};
use std::path::PathBuf;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "drahtseilakt-settings-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn settings_survive_the_round_trip() {
    let d = dir("trip");
    let l = Level {
        settings: Settings {
            music: Some(3),
            light: Some([1.0, 0.5, 0.25]),
            level_bonus: Some(2000),
            tutorial: true,
            last_level: true,
            source_hash: Some(0x0123_4567_89ab_cdef),
        },
        ..Level::default()
    };
    level::open::export(&l, &d).unwrap();
    assert_eq!(level::open::import(&d).unwrap().settings, l.settings);
    // a level without settings reads back with none
    level::open::export(&Level::default(), &d).unwrap();
    assert_eq!(
        level::open::import(&d).unwrap().settings,
        Settings::default()
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn malformed_settings_are_refused() {
    let d = dir("bad");
    level::open::export(&Level::default(), &d).unwrap();
    let path = d.join("level.gltf");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for bad in [
        serde_json::json!("x"),
        serde_json::json!({"music": -1}),
        serde_json::json!({"music": 99999}),
        serde_json::json!({"light": [1, 2]}),
        serde_json::json!({"light": ["a", 0, 0]}),
        serde_json::json!({"level_bonus": 1e30}),
        serde_json::json!({"tutorial": "yes"}),
        serde_json::json!({"source_hash": "xyz"}),
        serde_json::json!({"source_hash": "0123456789abcdef0"}),
    ] {
        let mut doc = doc.clone();
        doc["scenes"][0]["extras"]["drahtseilakt"]["settings"] = bad.clone();
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
        assert!(level::open::import(&d).is_err(), "{bad}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
