// No shipped level carries a picture of the original game: every picture under assets/levels (the open-format
// levels' files, the textures saved inside the community maps' .nmo files) and assets/own is compared with every
// picture of the player's copy (its image files and the textures saved in its .nmo files), pixel for pixel and by
// a perceptual hash that sees through rescaling, turning and small colour changes (gamedata::provenance).
// Needs the copy (OPENBALLANCE_DATA; more copies, e.g. one with level 13, in DRAHTSEILAKT_ORIGINAL_COPIES as a
// path list) and the shipped levels' files, which are not in git; skipped without them.

#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use gamedata::provenance::{self, Match};
use std::path::PathBuf;

#[test]
fn no_shipped_level_carries_a_picture_of_the_original() {
    let Some(copy) = std::env::var_os("OPENBALLANCE_DATA") else {
        eprintln!("OPENBALLANCE_DATA not set; skipped");
        return;
    };
    let mut copies = vec![PathBuf::from(copy)];
    if let Some(more) = std::env::var_os("DRAHTSEILAKT_ORIGINAL_COPIES") {
        copies.extend(std::env::split_paths(&more));
    }
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let shipped_roots: Vec<PathBuf> = ["levels", "own", "mods"]
        .iter()
        .map(|d| assets.join(d))
        .collect();
    let shipped = provenance::prints_under(&shipped_roots);
    // the levels' pictures are gitignored: a checkout without them has nothing to check
    if !shipped.iter().any(|(l, _)| l.contains("/levels/")) {
        eprintln!(
            "no shipped level pictures under {}; skipped",
            assets.display()
        );
        return;
    }
    let originals = provenance::prints_under(&copies);
    assert!(
        originals.len() > 300,
        "only {} pictures in the copy {copies:?}: not a copy of the game",
        originals.len()
    );
    let found = provenance::matches(&shipped, &originals);
    for (s, o, m) in &found {
        eprintln!("{m:?}: {s} = {o}");
    }
    let copies: Vec<_> = found.iter().filter(|(_, _, h)| h.how.is_copy()).collect();
    eprintln!(
        "{} shipped pictures checked against {} of the original's: {} copies, {} flat look-alikes",
        shipped.len(),
        originals.len(),
        copies.len(),
        found.len() - copies.len()
    );
    assert!(
        copies.is_empty(),
        "shipped pictures that are the original's:\n{}",
        copies
            .iter()
            .map(|(s, o, m)| format!("  {m:?}: {s} = {o}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    // the check sees a copy: the original's own pictures match themselves exactly
    let (label, first) = &originals[0];
    assert_eq!(
        provenance::compare(first, first),
        Some(Match::Exact),
        "{label}"
    );
}
