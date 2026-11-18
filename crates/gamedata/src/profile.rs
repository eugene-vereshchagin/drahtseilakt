// The game's own folders (docs/drahtseilakt/levels.md): its data folder, the settings file in it, and the `levels/`
// folders (the original game, community maps, remastered originals, open-format levels), each movable in the
// settings (`levels.<kind> PATH`). Shared by the game and the toolbox, so both find the same places.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Original,
    Community,
    Remastered,
    Open,
    /// Community remakes of the original levels (rotated, reversed, invisible): the original's geometry, so never
    /// shipped, played only from the player's files and with their copy of the original.
    Modified,
    /// The player's downloaded levels: community `.nmo` maps and open-format levels (owner's decision 2026-11-09).
    /// The earlier `community` and `open` folders are still read next to it.
    Downloaded,
}

/// Every folder the game reads levels from (the first of them the original's). `Community` and `Open` are the
/// earlier names of what is now `Downloaded`: still read, no longer made.
pub const KINDS: [Kind; 6] = [
    Kind::Original,
    Kind::Downloaded,
    Kind::Community,
    Kind::Open,
    Kind::Remastered,
    Kind::Modified,
];

/// The folders whose levels are the player's downloaded ones: the new folder, then the two it replaces.
pub const DOWNLOADED: [Kind; 3] = [Kind::Downloaded, Kind::Community, Kind::Open];

impl Kind {
    pub fn folder(self) -> &'static str {
        match self {
            Kind::Original => "ballance-original",
            Kind::Community => "community",
            Kind::Remastered => "ballance-remaster",
            Kind::Open => "open",
            Kind::Modified => "modified",
            Kind::Downloaded => "user-levels",
        }
    }

    /// The name the folder had before the layout of 2026-11-12 (`original`, `remastered`, `downloaded`): the settings
    /// key of that name still counts, and the folder is moved to the new name once (`migrate_in`).
    pub fn legacy_folder(self) -> Option<&'static str> {
        match self {
            Kind::Original => Some("original"),
            Kind::Remastered => Some("remastered"),
            Kind::Downloaded => Some("downloaded"),
            _ => None,
        }
    }
}

/// The folder of the running program, when it is a portable copy: a file `portable.txt` lies beside it. The player's
/// data is then in `data/` there, and nothing outside the folder is touched.
pub fn portable_root() -> Option<PathBuf> {
    static ROOT: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        std::fs::symlink_metadata(dir.join("portable.txt"))
            .is_ok_and(|m| m.is_file())
            .then_some(dir)
    })
    .clone()
}

/// The game's data folder: `data/` beside the program when it is portable (`portable.txt` beside it), else
/// XDG_DATA_HOME or ~/.local/share on Linux, %APPDATA% on Windows,
/// ~/Library/Application Support on macOS; the current folder when none is known.
pub fn data_dir() -> PathBuf {
    if let Some(root) = portable_root() {
        return root.join("data");
    }
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let base = if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        // a relative XDG_DATA_HOME is ignored, as the XDG spec says
        var("XDG_DATA_HOME")
            .filter(|p| p.is_absolute())
            .or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("drahtseilakt")
}

/// Where a kind's folder is by default.
pub fn default_dir(kind: Kind) -> PathBuf {
    migrate_once();
    data_dir().join("levels").join(kind.folder())
}

/// Where a kind's levels are: the folder the settings name (`levels.<kind>`, or by the kind's earlier name), else the
/// default.
pub fn dir(kind: Kind) -> PathBuf {
    migrate_once();
    let text = settings_text();
    let named = |name: &str| setting_in(&text, &format!("levels.{name}")).filter(|p| !p.is_empty());
    named(kind.folder())
        .or_else(|| kind.legacy_folder().and_then(named))
        .map_or_else(|| default_dir(kind), |p| resolve(&p))
}

/// The folders renamed in the player's `levels/` by the layout of 2026-11-12: (earlier name, kind). The three folders
/// for the player's own levels all become `user-levels/`.
const RENAMED: [(&str, Kind); 5] = [
    ("original", Kind::Original),
    ("remastered", Kind::Remastered),
    ("downloaded", Kind::Downloaded),
    ("community", Kind::Downloaded),
    ("open", Kind::Downloaded),
];

/// Moves the player's earlier level folders to the current names, once per run (a log line for each move):
/// `original` to `ballance-original`, `remastered` to `ballance-remaster`, and `downloaded`, `community` and `open` into
/// `user-levels` (their entries one by one; an entry whose name is taken there stays where it is and is still read).
/// Only the default places under the data folder, and not for a kind the settings place elsewhere. Nothing is ever
/// deleted or overwritten.
fn migrate_once() {
    static DONE: std::sync::Once = std::sync::Once::new();
    DONE.call_once(|| {
        let text = settings_text();
        let placed = |k: Kind, old: &str| {
            [k.folder(), old]
                .iter()
                .any(|n| setting_in(&text, &format!("levels.{n}")).is_some_and(|p| !p.is_empty()))
        };
        let root = data_dir().join("levels");
        for line in migrate_in(&root, placed) {
            println!("{line}");
        }
    });
}

/// `migrate_once` on one `levels/` folder; `skip(kind, earlier name)` leaves a folder alone. Returns the log lines.
pub fn migrate_in(root: &Path, skip: impl Fn(Kind, &str) -> bool) -> Vec<String> {
    let mut log = Vec::new();
    let plain_dir = |p: &Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_dir());
    for (old, kind) in RENAMED {
        let new = kind.folder();
        if skip(kind, old) {
            continue;
        }
        let (from, to) = (root.join(old), root.join(new));
        if !plain_dir(&from) {
            continue;
        }
        if std::fs::symlink_metadata(&to).is_err() {
            match std::fs::rename(&from, &to) {
                Ok(()) => log.push(format!("levels: moved {old}/ to {new}/")),
                Err(e) => log.push(format!("levels: could not move {old}/ to {new}/: {e}")),
            }
            continue;
        }
        // the target exists: only the player's own folders merge, entry by entry
        if kind != Kind::Downloaded || !plain_dir(&to) {
            log.push(format!("levels: {old}/ stays, {new}/ exists already"));
            continue;
        }
        let Ok(read) = std::fs::read_dir(&from) else {
            continue;
        };
        let mut names: Vec<_> = read.flatten().map(|e| e.file_name()).collect();
        names.sort();
        for name in names {
            if name == "README.txt" {
                continue;
            }
            let shown = name.to_string_lossy();
            if std::fs::symlink_metadata(to.join(&name)).is_ok() {
                log.push(format!(
                    "levels: {old}/{shown} stays, {new}/{shown} exists already"
                ));
            } else {
                match std::fs::rename(from.join(&name), to.join(&name)) {
                    Ok(()) => log.push(format!("levels: moved {old}/{shown} to {new}/")),
                    Err(e) => log.push(format!("levels: could not move {old}/{shown}: {e}")),
                }
            }
        }
    }
    log
}

/// A folder named in the settings: `~/` is the home folder, and a relative path is taken from the data folder (never
/// from wherever the game was started).
pub fn resolve(p: &str) -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let path = match (p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")), home) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(p),
    };
    if path.is_absolute() {
        path
    } else {
        data_dir().join(path)
    }
}

/// The player's settings file: one `name value` a line (`sound 0.8`, `music 0.5`).
pub fn settings_path() -> PathBuf {
    data_dir().join("settings.txt")
}

/// The settings file's text: a file larger than any we write is not ours and is ignored; bytes that are not UTF-8
/// are read as replacement characters, so the other lines still count.
pub fn settings_text() -> String {
    const MAX: u64 = 64 * 1024;
    let small = std::fs::metadata(settings_path()).is_ok_and(|m| m.len() <= MAX);
    let bytes = if small {
        std::fs::read(settings_path()).unwrap_or_default()
    } else {
        Vec::new()
    };
    settings_from_bytes(&bytes)
}

/// The settings file's bytes as text (the fuzz targets' entry; `settings_text` reads the file).
pub fn settings_from_bytes(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

/// A value of the settings file ("key value" lines).
pub fn setting(key: &str) -> Option<String> {
    setting_in(&settings_text(), key)
}

/// A value of the settings' text.
pub fn setting_in(text: &str, key: &str) -> Option<String> {
    text.lines()
        .filter_map(|l| l.split_once(' '))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.trim().to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn earlier_folders_move_once_and_nothing_is_lost() {
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for d in [
            "original",
            "remastered",
            "downloaded/A",
            "community/B",
            "community/A",
            "open/C",
            "modified",
        ] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        std::fs::write(root.join("original/base.cmo"), "x").unwrap();
        let log = migrate_in(&root, |_, _| false);
        assert!(root.join("ballance-original/base.cmo").is_file());
        assert!(root.join("ballance-remaster").is_dir() && !root.join("original").exists());
        // downloaded became user-levels; community and open merged in; the clashing A stays in community
        for d in [
            "user-levels/A",
            "user-levels/B",
            "user-levels/C",
            "community/A",
            "modified",
        ] {
            assert!(root.join(d).is_dir(), "{d}");
        }
        assert!(
            log.iter().any(|l| l.contains("community/A stays")),
            "{log:?}"
        );
        // a second run changes nothing
        assert!(
            migrate_in(&root, |_, _| false)
                .iter()
                .all(|l| l.contains("stays"))
        );
        // a folder the settings place elsewhere is left
        std::fs::create_dir_all(root.join("original")).unwrap();
        std::fs::remove_dir_all(root.join("ballance-original")).unwrap();
        migrate_in(&root, |k, _| k == Kind::Original);
        assert!(root.join("original").is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }
}
