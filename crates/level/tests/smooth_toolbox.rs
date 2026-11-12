// The rounding the game does at load (`level::smooth`) is the toolbox's: for a shipped level that carries looks the
// toolbox made, rounding its meshes again gives the same looks. Skipped when that level is not on this machine
// (levels are local only); set SMOOTH_LEVEL to a level folder to test another.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;

#[test]
fn rounding_again_gives_the_toolbox_looks() {
    let dir = std::env::var_os("SMOOTH_LEVEL").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../drahtseilakt/assets/levels/01-Map-260801"),
        PathBuf::from,
    );
    let Ok((level, _)) = level::open::import_read(&dir) else {
        eprintln!("skipped: no level at {}", dir.display());
        return;
    };
    let (mut same, mut with_look) = (0, 0);
    for m in &level.meshes {
        let Some(look) = m.look.as_deref() else {
            assert!(level::smooth::mesh(m).is_none(), "{}: a look the toolbox left out", m.name);
            continue;
        };
        with_look += 1;
        let again = level::smooth::mesh(m).unwrap_or_else(|| panic!("{}: no look made", m.name));
        assert_eq!(again.positions, look.positions, "{}", m.name);
        assert_eq!(again.normals, look.normals, "{}", m.name);
        assert_eq!(again.uvs, look.uvs, "{}", m.name);
        assert_eq!(
            again.parts.iter().map(|p| (&p.indices, p.material)).collect::<Vec<_>>(),
            look.parts.iter().map(|p| (&p.indices, p.material)).collect::<Vec<_>>()
        );
        same += 1;
    }
    eprintln!("{same} of {with_look} looks identical");
    assert_eq!(same, with_look);
}

#[test]
fn hostile_meshes_never_panic() {
    use level::{Level, Mesh, Part};
    let mut l = Level::default();
    // indices out of range, nothing finite, one huge degenerate mesh
    l.meshes.push(Mesh {
        name: "bad".into(),
        positions: vec![[f32::NAN, 0.0, 0.0], [f32::INFINITY, 1.0, 0.0], [0.0, 1e38, 1.0]],
        normals: vec![],
        uvs: vec![],
        parts: vec![Part { material: None, indices: vec![0, 1, 2, 7, 8, 9, 0, 0, 0, 1] }],
        channels: vec![],
        look: None,
    });
    l.meshes.push(Mesh {
        name: "big".into(),
        positions: vec![[0.0; 3]; 3],
        normals: vec![],
        uvs: vec![],
        parts: vec![Part { material: None, indices: vec![0, 1, 2].repeat(level::smooth::MAX_SOURCE_TRIANGLES + 1) }],
        channels: vec![],
        look: None,
    });
    let _ = level::smooth::add_looks(&mut l);
    assert!(l.meshes[1].look.is_none());
}
