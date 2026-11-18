// Reading and writing the game's own small files safely: a plain file only (never a link, a device or a pipe), read
// up to a cap whatever it says its length is, and written whole or not at all, never through a link. Replays,
// settings, reports and the audit's tables go through these.

use std::path::{Path, PathBuf};

/// The largest replay file read (a player's run, a run a hint names).
pub const MAX_REPLAY_FILE: u64 = 4 * 1024 * 1024;

/// A plain file itself (never a link, a device or a pipe), read up to `cap` bytes
/// whatever it says its length is.
pub fn read_plain(path: &Path, cap: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let plain = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file());
    if !plain {
        return Err(format!("{}: not a plain file", path.display()));
    }
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if u64::try_from(bytes.len()).map_or(true, |n| n > cap) {
        return Err(format!("{}: more than {cap} bytes", path.display()));
    }
    Ok(bytes)
}

/// Whether a folder on the way to a file written here is a link: the folder itself and those above it,
/// up to the current folder for a relative path, or within the home folder for an absolute one (above that the
/// system's own layout is no concern of ours).
fn linked_folder(dir: &Path) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    for d in dir.ancestors() {
        if d.as_os_str().is_empty() {
            break;
        }
        if dir.is_absolute()
            && !home
                .as_ref()
                .is_some_and(|h| d.starts_with(h) && d != h.as_path())
            && d != dir
        {
            break;
        }
        if std::fs::symlink_metadata(d).is_ok_and(|m| m.file_type().is_symlink()) {
            return Some(d.to_path_buf());
        }
    }
    None
}

/// Writes `text` as the file at `path` (settings, reports, tables, the files a tool adds to): never through a link (the file, or a folder on the way: `linked_folder`), and whole or not at all:
/// written to a new file beside it, synced, and renamed into place.
pub fn write_plain(path: &Path, text: &str) -> Result<(), String> {
    use std::io::Write as _;
    let shown = |p: &Path| p.display().to_string();
    if std::fs::symlink_metadata(path).is_ok_and(|m| !m.is_file()) {
        return Err(format!("{}: not a plain file, not written", shown(path)));
    }
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    if let Some(l) = linked_folder(&dir) {
        return Err(format!("{}: a link on the way, not written", shown(&l)));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", shown(&dir)))?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        // a leftover of an earlier run (a file or a link) is removed itself, never followed
        if std::fs::symlink_metadata(&tmp).is_ok() {
            std::fs::remove_file(&tmp)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", shown(path))
    })
}

/// The text of a plain file, read up to `cap` bytes; an error when it is missing, too long, not UTF-8 or not a
/// plain file.
pub fn read_utf8(path: &Path, cap: u64) -> Result<String, String> {
    String::from_utf8(read_plain(path, cap)?).map_err(|_| format!("{}: not UTF-8", path.display()))
}

/// The text of a plain file added to (empty when there is none yet), read up to `cap` bytes.
pub fn read_text(path: &Path, cap: u64) -> Result<String, String> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
        Ok(_) => {}
    }
    String::from_utf8(read_plain(path, cap)?).map_err(|_| format!("{}: not UTF-8", path.display()))
}
