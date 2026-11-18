// Where the autopilot store is: the replays, recorded runs and the autopilot's data (routes, zones, learned table)
// live in a repository of their own beside this one, not in it (decisions.md, "Replays and the autopilot's data live
// in a separate autopilot store"). Only developer commands and tests read it; the game itself never needs it, and
// everything that does reads it through `root()` and goes without when it is absent.

use std::path::PathBuf;

/// The environment variable that names the store.
pub const ENV: &str = "DRAHTSEILAKT_AUTOPILOT";

/// The store: `DRAHTSEILAKT_AUTOPILOT`, else `drahtseilakt-autopilot` beside the repository (the working directory is its
/// root for the developer commands). None when there is no such folder.
pub fn root() -> Option<PathBuf> {
    let dir = match std::env::var_os(ENV) {
        Some(v) => PathBuf::from(v),
        None => std::env::current_dir()
            .ok()?
            .join("../drahtseilakt-autopilot"),
    };
    dir.is_dir().then_some(dir)
}

/// The store for a test, found from the sources' own place so the working directory does not matter. Prints why a
/// test is skipped when it is absent, so CI (which has none) passes.
#[cfg(test)]
pub fn for_test(what: &str) -> Option<PathBuf> {
    let dir = match std::env::var_os(ENV) {
        Some(v) => PathBuf::from(v),
        None => {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../drahtseilakt-autopilot")
        }
    };
    if dir.is_dir() {
        Some(dir)
    } else {
        eprintln!(
            "{ENV} not set and no drahtseilakt-autopilot beside the repository: {what} is skipped"
        );
        None
    }
}
