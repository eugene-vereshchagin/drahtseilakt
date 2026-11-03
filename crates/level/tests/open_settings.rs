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
            own_parts: true,
            fog: Some(level::Fog { color: [0.5, 0.75, 1.0], color_down: [0.2, 0.3, 0.1], density: 0.0015, below: -40.0, depth: 150.0 }),
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
        // found by review while fuzzing: i64::MIN's abs() wrapped to itself in a release build and passed the cap
        serde_json::json!({"level_bonus": i64::MIN}),
        serde_json::json!({"level_bonus": -1_000_000_001i64}),
        serde_json::json!({"tutorial": "yes"}),
        serde_json::json!({"source_hash": "xyz"}),
        serde_json::json!({"source_hash": "0123456789abcdef0"}),
        serde_json::json!({"fog": "thick"}),
        serde_json::json!({"fog": {"color": [1, 1], "density": 0.1, "below": 0, "depth": 10}}),
        serde_json::json!({"fog": {"color": [1, 1, 1, 1], "density": 0.1, "below": 0, "depth": 10}}),
        serde_json::json!({"fog": {"color": [1, "a", 1], "density": 0.1, "below": 0, "depth": 10}}),
        serde_json::json!({"fog": {"color": [1, 1, 1], "below": 0, "depth": 10}}),
        serde_json::json!({"fog": {"color": [1, 1, 1], "density": 0.1, "depth": 10}}),
        serde_json::json!({"fog": {"color": [1, 1, 1], "density": 0.1, "below": 0}}),
        // finite as JSON, infinite once it is a 32-bit float
        serde_json::json!({"fog": {"color": [1, 1, 1], "density": 0.1, "below": 1e300, "depth": 10}}),
        serde_json::json!({"fog": {"color": [1, 1, 1], "color_down": [0, 0], "density": 0.1, "below": 0, "depth": 1}}),
    ] {
        let mut doc = doc.clone();
        doc["scenes"][0]["extras"]["drahtseilakt"]["settings"] = bad.clone();
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
        assert!(level::open::import(&d).is_err(), "{bad}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// A fog without its colour seen downwards has one colour all round, and values out of range are held to it.
#[test]
fn a_fog_reads_with_its_defaults_and_ranges() {
    let d = dir("fog");
    level::open::export(&Level::default(), &d).unwrap();
    let path = d.join("level.gltf");
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    doc["scenes"][0]["extras"]["drahtseilakt"]["settings"] =
        serde_json::json!({"fog": {"color": [0.5, 0.6, 2.0], "density": 5, "below": -20, "depth": 0}});
    std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
    let fog = level::open::import(&d).unwrap().settings.fog.unwrap();
    assert_eq!(fog.color, [0.5, 0.6, 1.0]);
    assert_eq!(fog.color_down, fog.color);
    assert_eq!(fog.density, 1.0);
    assert_eq!(fog.below, -20.0);
    assert_eq!(fog.depth, 0.01);
    let _ = std::fs::remove_dir_all(&d);
}
