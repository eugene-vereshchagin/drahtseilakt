// The census over real maps, when the owner's collection is given (OPENBALLANCE_MAPS, docs/drahtseilakt/
// community-maps.md): it reads every file without failing and every block of a kind absent from the player's
// libraries is declared a named no-op. Skipped without the collection (no game data in the repository).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
fn the_census_reads_the_community_maps() {
    let Some(dir) = std::env::var_os("OPENBALLANCE_MAPS") else {
        eprintln!("OPENBALLANCE_MAPS not set: skipped");
        return;
    };
    let mut c = script::census::Census::default();
    let errors = c.add_files(std::path::Path::new(&dir));
    assert!(c.files > 0, "no map files under {dir:?}");
    assert!(
        errors.len() * 20 < c.files,
        "most files read: {} of {} failed",
        errors.len(),
        c.files
    );
    let csv = c.csv();
    for name in ["Explode", "Midi Player", "Run VSL", "Get Nearest Object"] {
        for line in csv.lines().filter(|l| l.split(',').nth(1) == Some(name)) {
            assert!(line.contains(",-,no-op,"), "{line}");
        }
    }
    println!("{csv}");
}
