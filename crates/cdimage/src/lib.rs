// Reading the player's own game CD: a disc image (a cue sheet with its raw bin, or a plain iso), its ISO 9660
// file system, and the InstallShield 6 archives the game's files are packed in (docs/drahtseilakt/levels.md,
// "The original game: copied by default"). Every byte comes from a file the player gave us and is treated as
// hostile: reads are bounds-checked, sizes and counts capped, anything unexpected is an error, nothing panics.

pub mod image;
pub mod iscab;
pub mod iso;

pub use image::Image;

/// What went wrong, in words for the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn err<T>(what: impl Into<String>) -> Result<T> {
    Err(Error(what.into()))
}

/// Little-endian fields of a byte slice, every read checked.
pub(crate) fn u16_at(b: &[u8], at: usize) -> Result<u16> {
    let end = at.checked_add(2).ok_or(Error("offset".into()))?;
    let s = b
        .get(at..end)
        .ok_or(Error(format!("field at {at} past the end")))?;
    Ok(u16::from_le_bytes([
        s.first().copied().unwrap_or(0),
        s.get(1).copied().unwrap_or(0),
    ]))
}

pub(crate) fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    let end = at.checked_add(4).ok_or(Error("offset".into()))?;
    let s: [u8; 4] = b
        .get(at..end)
        .and_then(|s| s.try_into().ok())
        .ok_or(Error(format!("field at {at} past the end")))?;
    Ok(u32::from_le_bytes(s))
}

pub(crate) fn u64_at(b: &[u8], at: usize) -> Result<u64> {
    let end = at.checked_add(8).ok_or(Error("offset".into()))?;
    let s: [u8; 8] = b
        .get(at..end)
        .and_then(|s| s.try_into().ok())
        .ok_or(Error(format!("field at {at} past the end")))?;
    Ok(u64::from_le_bytes(s))
}

/// The NUL-terminated string at `at` (at most `max` bytes), as Latin-1 text.
pub(crate) fn cstr_at(b: &[u8], at: usize, max: usize) -> Result<String> {
    let rest = b
        .get(at..)
        .ok_or(Error(format!("string at {at} past the end")))?;
    let rest = rest.get(..max.min(rest.len())).unwrap_or(rest);
    let end = rest
        .iter()
        .position(|&c| c == 0)
        .ok_or(Error(format!("string at {at} not terminated")))?;
    Ok(rest
        .get(..end)
        .unwrap_or_default()
        .iter()
        .map(|&c| char::from(c))
        .collect())
}
