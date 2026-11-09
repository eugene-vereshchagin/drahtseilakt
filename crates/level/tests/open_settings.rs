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
            fog: Some(level::Fog {
                color: [0.5, 0.75, 1.0],
                color_down: [0.2, 0.3, 0.1],
                density: 0.0015,
                below: -40.0,
                depth: 150.0,
            }),
            sector_balls: vec![None, Some("stone".into()), None, Some("wood".into())],
            clouds: Some(level::Clouds {
                top: -60.0,
                thickness: 70.0,
                cover: 0.9,
                scale: 50.0,
                sun: [0.0, 0.6, 0.8],
                sun_color: [4.0, 3.0, 2.0],
                sky: [0.5, 0.6, 0.8],
                shadow: [0.4, 0.35, 0.5],
                haze: 0.001,
                under: 0.5,
                wind: [1.0, -2.0],
            }),
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
        serde_json::json!({"clouds": "fluffy"}),
        serde_json::json!({"clouds": {}}),
        serde_json::json!({"clouds": {"top": "high"}}),
        serde_json::json!({"clouds": {"top": 1e300}}),
        serde_json::json!({"clouds": {"top": 0, "sun": [0, 1]}}),
        serde_json::json!({"clouds": {"top": 0, "sun_color": [1, "a", 1]}}),
        serde_json::json!({"clouds": {"top": 0, "wind": [1, 2, 3]}}),
        serde_json::json!({"clouds": {"top": 0, "cover": [1]}}),
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
    doc["scenes"][0]["extras"]["drahtseilakt"]["settings"] = serde_json::json!({"fog": {"color": [0.5, 0.6, 2.0], "density": 5, "below": -20, "depth": 0}});
    std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
    let fog = level::open::import(&d).unwrap().settings.fog.unwrap();
    assert_eq!(fog.color, [0.5, 0.6, 1.0]);
    assert_eq!(fog.color_down, fog.color);
    assert_eq!(fog.density, 1.0);
    assert_eq!(fog.below, -20.0);
    assert_eq!(fog.depth, 0.01);
    let _ = std::fs::remove_dir_all(&d);
}

/// A cloud layer needs only its height: the rest has its defaults, values out of range are held to it, and a sun in
/// no direction keeps the default one.
#[test]
fn clouds_read_with_their_defaults_and_ranges() {
    let d = dir("clouds");
    level::open::export(&Level::default(), &d).unwrap();
    let path = d.join("level.gltf");
    let mut doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    doc["scenes"][0]["extras"]["drahtseilakt"]["settings"] = serde_json::json!({"clouds": {"top": -80, "cover": 3, "thickness": 0, "sun": [0, 0, 0], "sun_color": [100, 1, 1]}});
    std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
    let c = level::open::import(&d).unwrap().settings.clouds.unwrap();
    assert_eq!(c.top, -80.0);
    assert_eq!(c.cover, 1.0);
    assert_eq!(c.thickness, 1.0);
    assert_eq!(c.sun, level::Clouds::DEFAULT.sun);
    assert_eq!(c.sun_color, [64.0, 1.0, 1.0]);
    assert_eq!(c.wind, level::Clouds::DEFAULT.wind);
    assert_eq!(c.haze, level::Clouds::DEFAULT.haze);
    let _ = std::fs::remove_dir_all(&d);
}
