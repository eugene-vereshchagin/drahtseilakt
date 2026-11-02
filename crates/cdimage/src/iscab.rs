// InstallShield 6 archives as the game's CD has them (SETUP/DATA1.HDR with DATA1.CAB, DATA2.CAB, ...): the header
// file's cabinet descriptor lists the directories and the files (name, directory, flags, sizes, offset and volume),
// and each file's data sits in its volume as stored bytes or as chunks of raw deflate, each led by its 16-bit
// length. Read from the files the game's CD carries (docs/drahtseilakt/levels.md); older archive versions, files
// split across volumes and obfuscated files are refused, not guessed at.

use crate::{Error, Result, cstr_at, err, u16_at, u32_at, u64_at};

const SIGNATURE: &[u8; 4] = b"ISc(";
/// File flags.
const SPLIT: u16 = 1;
const OBFUSCATED: u16 = 2;
const COMPRESSED: u16 = 4;
const INVALID: u16 = 8;
/// A file descriptor's size in versions 6 and later.
const DESCRIPTOR: usize = 0x57;
/// Limits: directories and files listed, a name's length, a file's expanded size.
const DIRS_MAX: u32 = 10_000;
const FILES_MAX: u32 = 100_000;
const NAME_MAX: usize = 260;
pub const FILE_MAX: u64 = 64 * 1024 * 1024;
/// The most an import writes in all (the game is about 160 MB): many files pointing at one stretch of data cannot
/// fill a disk.
pub const TOTAL_MAX: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub index: u32,
    /// The directory as stored, parts separated by backslashes ("3D Entities\Level"); empty for the top.
    pub dir: String,
    pub name: String,
    pub flags: u16,
    pub expanded: u64,
    pub compressed: u64,
    /// Where its data begins in its volume file, and that volume's number (DATA<volume>.CAB).
    pub offset: u64,
    pub volume: u16,
}

/// A name every platform can write as an ordinary file: not empty, "." or "..", without separators, drive colons,
/// characters Windows refuses or control bytes, not ending in a dot or a space, and not one of Windows' device names
/// (CON, PRN, AUX, NUL, COM1-9, LPT1-9, with or without an extension).
fn plain(p: &str) -> bool {
    let stem = p.split('.').next().unwrap_or_default().to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.chars().last().is_some_and(|c| c.is_ascii_digit()));
    !p.is_empty()
        && !device
        && !p.ends_with(['.', ' '])
        && !p.chars().any(|c| {
            matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*') || c.is_control()
        })
}

impl File {
    /// A file that has data in a volume (not a placeholder, not one kept outside the archive).
    pub fn stored(&self) -> bool {
        self.flags & INVALID == 0 && self.offset != 0
    }

    /// The file's place as path parts, each a plain name: none that is empty, "." or "..", or holds a separator,
    /// a drive colon or a NUL. `None` for any other.
    pub fn parts(&self) -> Option<Vec<String>> {
        let mut parts: Vec<String> = self
            .dir
            .split('\\')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        parts.push(self.name.clone());
        parts.iter().all(|p| plain(p)).then_some(parts)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cab {
    pub files: Vec<File>,
}

/// The archive's version from its header word: 6 and later share one layout.
fn major(version: u32) -> u32 {
    match version >> 24 {
        1 => (version >> 12) & 0xf,
        2 | 4 => {
            let v = version & 0xffff;
            if v == 0 { 0 } else { v / 100 }
        }
        _ => 0,
    }
}

/// The header file (DATA1.HDR): its cabinet descriptor's directories and files.
pub fn parse(hdr: &[u8]) -> Result<Cab> {
    if hdr.get(..4) != Some(&SIGNATURE[..]) {
        return err("not an InstallShield archive header");
    }
    let version = major(u32_at(hdr, 4)?);
    if version < 6 {
        return err(format!(
            "InstallShield {version} archives are not read yet (6 and later are)"
        ));
    }
    let at = |base: usize, off: u32| base.checked_add(off as usize).ok_or(Error("offset".into()));
    let desc = u32_at(hdr, 0x0c)? as usize;
    let table_off = u32_at(hdr, at(desc, 0x0c)?)?;
    let dirs = u32_at(hdr, at(desc, 0x1c)?)?;
    let files = u32_at(hdr, at(desc, 0x28)?)?;
    let descriptors_off = u32_at(hdr, at(desc, 0x2c)?)?;
    if dirs > DIRS_MAX || files > FILES_MAX {
        return err(format!(
            "{dirs} directories and {files} files: more than an archive of a game holds"
        ));
    }
    let table = at(desc, table_off)?;
    let dir_names = (0..dirs)
        .map(|i| {
            let slot = at(table, i.checked_mul(4).ok_or(Error("directory".into()))?)?;
            cstr_at(hdr, at(table, u32_at(hdr, slot)?)?, NAME_MAX)
        })
        .collect::<Result<Vec<String>>>()?;
    let descriptors = at(table, descriptors_off)?;
    let mut out = Vec::with_capacity(files.min(4096) as usize);
    for index in 0..files {
        let p = (index as usize)
            .checked_mul(DESCRIPTOR)
            .and_then(|o| descriptors.checked_add(o))
            .ok_or(Error("file descriptor".into()))?;
        let flags = u16_at(hdr, p.saturating_add(0))?;
        let name_at = at(table, u32_at(hdr, p.saturating_add(0x3a))?)?;
        let dir = usize::from(u16_at(hdr, p.saturating_add(0x3e))?);
        out.push(File {
            index,
            dir: dir_names.get(dir).cloned().unwrap_or_default(),
            name: if flags & INVALID != 0 {
                String::new()
            } else {
                cstr_at(hdr, name_at, NAME_MAX)?
            },
            flags,
            expanded: u64_at(hdr, p.saturating_add(2))?,
            compressed: u64_at(hdr, p.saturating_add(10))?,
            offset: u64_at(hdr, p.saturating_add(18))?,
            volume: u16_at(hdr, p.saturating_add(0x55))?,
        });
    }
    Ok(Cab { files: out })
}

/// A file's bytes from its volume file's contents: stored, or inflated chunk by chunk; they must come to exactly
/// its expanded size.
pub fn extract(f: &File, volume: &[u8]) -> Result<Vec<u8>> {
    if f.flags & (SPLIT | OBFUSCATED) != 0 {
        return err(format!(
            "{}: split across volumes or obfuscated, not read yet",
            f.name
        ));
    }
    if !f.stored() {
        return err(format!("{}: has no data in the archive", f.name));
    }
    if f.expanded > FILE_MAX {
        return err(format!(
            "{}: {} bytes, more than a game file",
            f.name, f.expanded
        ));
    }
    let start = usize::try_from(f.offset).map_err(|_| Error("offset".into()))?;
    let len = usize::try_from(if f.flags & COMPRESSED != 0 {
        f.compressed
    } else {
        f.expanded
    })
    .map_err(|_| Error("size".into()))?;
    let end = start.checked_add(len).ok_or(Error("size".into()))?;
    let raw = volume
        .get(start..end)
        .ok_or(Error(format!("{}: past the end of its volume", f.name)))?;
    let expanded = usize::try_from(f.expanded).map_err(|_| Error("size".into()))?;
    if f.flags & COMPRESSED == 0 {
        return Ok(raw.to_vec());
    }
    let mut out = Vec::with_capacity(expanded.min(64 * 1024 * 1024));
    let mut at = 0usize;
    while at < raw.len() {
        let n = usize::from(u16_at(raw, at)?);
        let from = at.checked_add(2).ok_or(Error("chunk".into()))?;
        let to = from.checked_add(n).ok_or(Error("chunk".into()))?;
        let chunk = raw
            .get(from..to)
            .ok_or(Error(format!("{}: a chunk past the file's data", f.name)))?;
        let room = expanded.saturating_sub(out.len());
        let part = miniz_oxide::inflate::decompress_to_vec_with_limit(chunk, room)
            .map_err(|e| Error(format!("{}: bad compressed data ({:?})", f.name, e.status)))?;
        out.extend_from_slice(&part);
        at = to;
    }
    if out.len() != expanded {
        return err(format!(
            "{}: {} bytes inflated, {expanded} expected",
            f.name,
            out.len()
        ));
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    /// A header with one directory ("3D Entities\Level") and the files given (name, flags, expanded, compressed,
    /// offset), in the layout DATA1.HDR has.
    pub fn header(files: &[(&str, u16, u64, u64, u64)]) -> Vec<u8> {
        let desc = 0x200usize;
        let table_off = 0x100u32;
        let table = desc + table_off as usize;
        let mut h = vec![0u8; table + 0x40 + files.len() * (DESCRIPTOR + 32)];
        h[..4].copy_from_slice(SIGNATURE);
        h[4..8].copy_from_slice(&0x0100_600cu32.to_le_bytes());
        h[0x0c..0x10].copy_from_slice(&(desc as u32).to_le_bytes());
        let put32 =
            |h: &mut Vec<u8>, at: usize, v: u32| h[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put32(&mut h, desc + 0x0c, table_off);
        put32(&mut h, desc + 0x1c, 1);
        put32(&mut h, desc + 0x28, files.len() as u32);
        let names = 0x40usize;
        put32(&mut h, desc + 0x2c, names as u32 + files.len() as u32 * 32);
        put32(&mut h, table, 8);
        h[table + 8..table + 8 + 18].copy_from_slice(b"3D Entities\\Level\0");
        for (i, (name, flags, exp, comp, off)) in files.iter().enumerate() {
            let n = table + names + i * 32;
            h[n..n + name.len()].copy_from_slice(name.as_bytes());
            let p = table + names + files.len() * 32 + i * DESCRIPTOR;
            h[p..p + 2].copy_from_slice(&flags.to_le_bytes());
            h[p + 2..p + 10].copy_from_slice(&exp.to_le_bytes());
            h[p + 10..p + 18].copy_from_slice(&comp.to_le_bytes());
            h[p + 18..p + 26].copy_from_slice(&off.to_le_bytes());
            put32(&mut h, p + 0x3a, (names + i * 32) as u32);
            h[p + 0x55..p + 0x57].copy_from_slice(&2u16.to_le_bytes());
        }
        h
    }

    /// `data` as the archive keeps it: raw deflate chunks, each led by its length.
    fn chunks(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for part in data.chunks(1000) {
            let z = miniz_oxide::deflate::compress_to_vec(part, 6);
            out.extend_from_slice(&(z.len() as u16).to_le_bytes());
            out.extend_from_slice(&z);
        }
        out
    }

    #[test]
    fn files_are_listed_and_extracted() {
        let data: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
        let packed = chunks(&data);
        let mut volume = vec![0u8; 0x200];
        volume.extend_from_slice(&packed);
        volume.extend_from_slice(b"plain");
        let plain_at = 0x200 + packed.len() as u64;
        let h = header(&[
            ("Level_01.NMO", COMPRESSED, 5000, packed.len() as u64, 0x200),
            ("readme.txt", 0, 5, 5, plain_at),
            ("", INVALID, 0, 0, 0),
        ]);
        let cab = parse(&h).unwrap();
        assert_eq!(cab.files.len(), 3);
        let f = &cab.files[0];
        assert_eq!(f.parts().unwrap(), ["3D Entities", "Level", "Level_01.NMO"]);
        assert_eq!(extract(f, &volume).unwrap(), data);
        assert_eq!(extract(&cab.files[1], &volume).unwrap(), b"plain");
        assert!(!cab.files[2].stored());
        assert!(extract(&cab.files[2], &volume).is_err());
    }

    #[test]
    fn bad_archives_and_files_are_refused() {
        assert!(parse(b"MSCF....").is_err(), "not InstallShield");
        let mut old = header(&[]);
        old[4..8].copy_from_slice(&0x0100_500cu32.to_le_bytes());
        assert!(parse(&old).is_err(), "version 5");
        let mut many = header(&[]);
        many[0x200 + 0x28..0x200 + 0x2c].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&many).is_err(), "too many files");
        let h = header(&[("a", COMPRESSED, 10, 50, 0x10)]);
        let f = parse(&h).unwrap().files[0].clone();
        assert!(extract(&f, &[0u8; 40]).is_err(), "past the volume");
        assert!(extract(&f, &[0xffu8; 100]).is_err(), "not deflate");
        let mut big = f.clone();
        big.expanded = FILE_MAX + 1;
        assert!(extract(&big, &[0u8; 100]).is_err());
        let evil = File {
            dir: "..\\..".into(),
            ..f.clone()
        };
        assert_eq!(evil.parts(), None);
        let drive = File {
            name: "C:x".into(),
            ..f.clone()
        };
        assert_eq!(drive.parts(), None);
        for bad in ["NUL", "com1.wav", "x.", "x ", "a?b", "a\u{1}b", "..."] {
            let f = File {
                name: bad.into(),
                ..f.clone()
            };
            assert_eq!(f.parts(), None, "{bad}");
        }
        let ok = File {
            name: "Sound_Ball.wav".into(),
            ..f
        };
        assert!(ok.parts().is_some());
    }

    #[test]
    fn a_short_inflate_is_an_error() {
        let packed = chunks(b"abc");
        let mut volume = vec![0u8; 0x10];
        volume.extend_from_slice(&packed);
        let h = header(&[("a", COMPRESSED, 4, packed.len() as u64, 0x10)]);
        let f = &parse(&h).unwrap().files[0];
        assert!(
            extract(f, &volume).is_err(),
            "3 bytes where 4 were promised"
        );
    }
}
