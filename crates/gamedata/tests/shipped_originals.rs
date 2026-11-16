// No shipped level carries a picture of the original game: every picture under assets/levels (the open-format
// levels' files, the textures saved inside the community maps' .nmo files) is compared with every picture of the
// player's copy (its image files and the textures saved in its .nmo files), pixel for pixel and by a perceptual hash
// that sees through rescaling, turning, small colour changes and a fade drawn in the alpha (gamedata::provenance).
// A match fails the test; `own-pictures` (crates/gamedata/src/bin) puts our own pictures in their place. Our own
// parts and mods (assets/own, assets/mods) are compared too and their look-alikes reported, not refused: the
// interface is drawn after the original's on purpose (an atlas of the same layout reads as near).
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
    let levels = provenance::prints_under(&[assets.join("levels")]);
    // the levels' pictures are gitignored: a checkout without them has nothing to check
    if levels.is_empty() {
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
    // the check sees a copy: the original's drawn pictures match themselves exactly
    assert!(
        originals
            .iter()
            .any(|(_, o)| provenance::compare(o, o) == Some(Match::Exact))
    );
    let ours = provenance::prints_under(&[assets.join("own"), assets.join("mods")]);
    for (s, o, h) in provenance::matches(&ours, &originals) {
        if h.how.is_copy() {
            eprintln!("look-alike of our own (reported, not refused): {h:?}: {s} = {o}");
        }
    }
    let found = provenance::matches(&levels, &originals);
    for (s, o, h) in &found {
        eprintln!("{h:?}: {s} = {o}");
    }
    let hits: Vec<_> = found.iter().filter(|(_, _, h)| h.how.is_copy()).collect();
    eprintln!(
        "{} pictures of the shipped levels checked against {} of the original's: {} copies, {} flat look-alikes",
        levels.len(),
        originals.len(),
        hits.len(),
        found.len() - hits.len()
    );
    assert!(
        hits.is_empty(),
        "shipped pictures that are the original's (own-pictures replaces them):\n{}",
        hits.iter()
            .map(|(s, o, h)| format!("  {h:?}: {s} = {o}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
