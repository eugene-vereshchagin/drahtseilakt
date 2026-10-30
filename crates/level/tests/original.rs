// Loads the original levels from the player's copy (OPENBALLANCE_DATA) and checks what the game rules rely
// on: a start, an end, checkpoints, sectors and floors. Skipped when the variable is not set.

#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

#[test]
fn original_levels_have_their_structure() {
    let Ok(root) = std::env::var("OPENBALLANCE_DATA") else {
        eprintln!("OPENBALLANCE_DATA not set; skipped");
        return;
    };
    for n in 1..=12 {
        let path = format!("{root}/3D Entities/Level/Level_{n:02}.NMO");
        let loaded = level::from_nmo::load(&std::fs::read(&path).unwrap()).unwrap();
        let l = &loaded.level;
        assert!(loaded.warnings.is_empty(), "{path}: {:?}", loaded.warnings);
        for g in [
            "PS_Levelstart",
            "PE_Levelende",
            "PC_Checkpoints",
            "PR_Resetpoints",
            "Phys_Floors",
            "Sector_01",
        ] {
            assert!(l.members(g).count() > 0, "{path}: group {g} empty");
        }
        let triangles: usize = l
            .meshes
            .iter()
            .flat_map(|m| &m.parts)
            .map(|p| p.indices.len() / 3)
            .sum();
        eprintln!(
            "Level_{n:02}: {} objects, {} meshes, {} triangles, {} materials, {} textures, {} groups",
            l.objects.len(),
            l.meshes.len(),
            triangles,
            l.materials.len(),
            l.textures.len(),
            l.groups.len()
        );
    }
}
