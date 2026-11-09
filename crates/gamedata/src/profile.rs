// The game's own folders (docs/drahtseilakt/levels.md): its data folder, the settings file in it, and the `levels/`
// folders (the original game, community maps, remastered originals, open-format levels), each movable in the
// settings (`levels.<kind> PATH`). Shared by the game and the toolbox, so both find the same places.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Original,
    Community,
    Remastered,
    Open,
    /// Community remakes of the original levels (rotated, reversed, invisible): the original's geometry, so never
    /// shipped, played only from the player's files and with their copy of the original.
    Modified,
}

pub const KINDS: [Kind; 5] = [
    Kind::Original,
    Kind::Community,
    Kind::Remastered,
    Kind::Open,
    Kind::Modified,
];

impl Kind {
    pub fn folder(self) -> &'static str {
        match self {
            Kind::Original => "original",
            Kind::Community => "community",
            Kind::Remastered => "remastered",
            Kind::Open => "open",
            Kind::Modified => "modified",
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
    data_dir().join("levels").join(kind.folder())
}

/// Where a kind's levels are: the folder the settings name (`levels.<kind>`), else the default.
pub fn dir(kind: Kind) -> PathBuf {
    setting(&format!("levels.{}", kind.folder()))
        .filter(|p| !p.is_empty())
        .map_or_else(|| default_dir(kind), |p| resolve(&p))
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
