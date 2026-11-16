// One picture, one look (open-format.md, a material's `original`): every material of every own-built level we ship
// (assets/levels/drahtseilakt/, the ones levelkit.py made) either stands for one of the original's textures, so that
// the texture pack draws it as it draws the parts and balls, or is declared ours alone. The levels are local files
// (not in the repository): without them there is nothing to check, and the test says so.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use gamedata::resources::is_original_texture;
use std::path::Path;

/// What is wrong with one level's materials (empty: nothing).
fn faults(gltf: &serde_json::Value) -> Vec<String> {
    let images: Vec<&str> = gltf["images"]
        .as_array()
        .map(|a| a.iter().map(|i| i["uri"].as_str().unwrap_or("")).collect())
        .unwrap_or_default();
    let mut out = Vec::new();
    for m in gltf["materials"].as_array().into_iter().flatten() {
        let name = m["name"].as_str().unwrap_or("?");
        let x = &m["extras"]["drahtseilakt"];
        if let Some(o) = x.get("original") {
            match o["texture"].as_str() {
                Some(t) if is_original_texture(t) => {}
                _ => out.push(format!(
                    "{name}: stands for {o}, not one of the original's textures"
                )),
            }
            continue;
        }
        if x["own_only"].as_bool() == Some(true) {
            continue;
        }
        let Some(t) = m["pbrMetallicRoughness"]["baseColorTexture"]["index"].as_u64() else {
            continue; // a colour, no picture
        };
        let uri = gltf["textures"][t as usize]["source"]
            .as_u64()
            .and_then(|s| images.get(s as usize).copied())
            .unwrap_or("");
        if uri.strip_prefix("game:").is_some_and(is_original_texture) {
            continue;
        }
        out.push(format!(
            "{name} ({uri}): neither stands for one of the original's textures nor is own-only (levelkit.ORIGINAL_OF)"
        ));
    }
    out
}

#[test]
fn every_own_level_material_maps_to_the_original_or_is_own_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/levels/drahtseilakt");
    let Ok(read) = std::fs::read_dir(&root) else {
        eprintln!(
            "{}: no shipped levels here; nothing checked",
            root.display()
        );
        return;
    };
    let mut checked = 0;
    let mut all = Vec::new();
    for e in read.flatten() {
        let path = e.path().join("level.gltf");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let gltf: serde_json::Value = serde_json::from_str(&text).unwrap();
        // ours: made by levelkit.py (a community map's remaster is the converter's, its textures the map's)
        let own = gltf["asset"]["generator"]
            .as_str()
            .is_some_and(|g| g.starts_with("drahtseilakt levelkit.py"));
        if !own {
            continue;
        }
        checked += 1;
        all.extend(
            faults(&gltf)
                .into_iter()
                .map(|f| format!("{}: {f}", e.path().display())),
        );
        // and the game reads what it says
        let (level, _) = level::open::import_read(&e.path()).unwrap();
        assert!(
            level
                .originals
                .iter()
                .all(|o| is_original_texture(&o.texture)),
            "{}",
            e.path().display()
        );
    }
    eprintln!("{checked} own-built levels checked");
    assert!(all.is_empty(), "{}", all.join("\n"));
}

#[test]
fn a_material_without_a_mapping_is_found() {
    let gltf = serde_json::json!({
        "images": [{"uri": "textures/Track_Top_A.png"}, {"uri": "game:Wood_Plain"}, {"uri": "game:Track_Side"}],
        "textures": [{"source": 0}, {"source": 1}, {"source": 2}],
        "materials": [
            {"name": "mapped", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}},
             "extras": {"drahtseilakt": {"original": {"texture": "Floor_Top_Borderless"}}}},
            {"name": "own", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}},
             "extras": {"drahtseilakt": {"own_only": true}}},
            {"name": "wood", "pbrMetallicRoughness": {"baseColorTexture": {"index": 1}}},
            {"name": "colour"},
            {"name": "side", "pbrMetallicRoughness": {"baseColorTexture": {"index": 2}}},
            {"name": "wrong", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}},
             "extras": {"drahtseilakt": {"original": {"texture": "Track_Top"}}}},
        ],
    });
    let f = faults(&gltf);
    assert_eq!(f.len(), 2, "{f:?}");
    assert!(f[0].starts_with("side ") && f[1].starts_with("wrong:"));
}
