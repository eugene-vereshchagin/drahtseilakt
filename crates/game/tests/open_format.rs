// The open format holds a level whole: each original level with a recorded run in the autopilot store (tests/replays/) is exported, and
// the run gives the same events on the .nmo and on its export. Needs the player's copy (OPENBALLANCE_DATA); without
// it the test says so and passes, as the game-data tests do.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The events of a headless replay: checkpoints, lost balls, transformations, the finish.
fn events(args: &[&OsStr]) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_drahtseilakt"))
        // a test must not write into the player's profile (dev logs of an unattended run)
        .env("DRAHTSEILAKT_DEV_LOGS", "0")
        .args(args)
        .output()
        .expect("the game runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let wanted = [
        "Checkpoint",
        "Died",
        "NewBall",
        "Transform",
        "Finished",
        "GameOver",
    ];
    text.lines()
        .filter(|l| wanted.iter().any(|e| l.contains(e)))
        .map(str::to_owned)
        .collect()
}

#[test]
fn recorded_runs_play_the_same_on_the_open_format() {
    let Some(data) = std::env::var_os("OPENBALLANCE_DATA") else {
        eprintln!("OPENBALLANCE_DATA not set: the open format's replay check is skipped");
        return;
    };
    // the recorded runs live in the autopilot store (DRAHTSEILAKT_AUTOPILOT, else drahtseilakt-tools/autopilot beside the repo)
    let store = std::env::var_os("DRAHTSEILAKT_AUTOPILOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../drahtseilakt-tools/autopilot")
        });
    let replays = store.join("tests/replays");
    if !replays.is_dir() {
        eprintln!(
            "no autopilot store at {}: the open format's replay check is skipped",
            store.display()
        );
        return;
    }
    let mut names: Vec<PathBuf> = std::fs::read_dir(&replays)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    names.sort();
    let tmp = std::env::temp_dir().join(format!("drahtseilakt-open-format-{}", std::process::id()));
    let mut checked = 0usize;
    for replay in names {
        let Some(n) = replay
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("Level_"))
            .and_then(|d| d.parse::<u32>().ok())
        else {
            continue;
        };
        let level = n.to_string();
        let dir = tmp.join(format!("Level_{n:02}"));
        let (d, l, r) = (data.as_os_str(), OsStr::new(&level), replay.as_os_str());
        let export = Command::new(env!("CARGO_BIN_EXE_drahtseilakt"))
            // a test must not write into the player's profile (dev logs of an unattended run)
            .env("DRAHTSEILAKT_DEV_LOGS", "0")
            .args([
                OsStr::new("--data"),
                d,
                OsStr::new("--level"),
                l,
                OsStr::new("--export"),
                dir.as_os_str(),
            ])
            .output()
            .expect("the export runs");
        assert!(
            export.status.success(),
            "export of level {n}: {}",
            String::from_utf8_lossy(&export.stderr)
        );
        let flags = |source: &OsStr, at: &OsStr| -> Vec<String> {
            events(&[
                OsStr::new("--data"),
                d,
                source,
                at,
                OsStr::new("--replay"),
                r,
                OsStr::new("--headless"),
            ])
        };
        let nmo = flags(OsStr::new("--level"), l);
        let open = flags(OsStr::new("--open"), dir.as_os_str());
        assert!(
            nmo.iter().any(|e| e.contains("Finished")),
            "level {n}: the run does not finish: {nmo:?}"
        );
        assert_eq!(nmo, open, "level {n}: the open format plays another run");
        checked = checked.saturating_add(1);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(checked > 0, "no recorded runs in {}", replays.display());
}
