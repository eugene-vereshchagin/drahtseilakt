// Reads every .nmo / .cmo of the player's copy (OPENBALLANCE_DATA) and of the map collection
// (OPENBALLANCE_MAPS). Each must read completely; skipped when the variables are not set, since game data is
// never part of the repository's tests.

// The hostile-input lints guard library code; a test that panics is a failing test, which is what we want.
#![allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::unwrap_used
)]

use std::path::{Path, PathBuf};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("nmo") || e.eq_ignore_ascii_case("cmo"))
        {
            out.push(path);
        }
    }
}

fn check(var: &str) {
    let Ok(root) = std::env::var(var) else {
        eprintln!("{var} not set; skipped");
        return;
    };
    let mut files = Vec::new();
    collect(Path::new(&root), &mut files);
    assert!(
        !files.is_empty(),
        "{var}={root}: no .nmo / .cmo files found"
    );
    let failures: Vec<String> = files
        .iter()
        .filter_map(|f| {
            match std::fs::read(f)
                .map_err(|e| e.to_string())
                .and_then(|b| nmo::read(&b).map(|_| ()).map_err(|e| e.to_string()))
            {
                Ok(()) => None,
                Err(e) => Some(format!("{}: {e}", f.display())),
            }
        })
        .collect();
    eprintln!("{var}: {} files, {} failed", files.len(), failures.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn game_files_read() {
    check("OPENBALLANCE_DATA");
}

#[test]
fn map_files_read() {
    check("OPENBALLANCE_MAPS");
}
