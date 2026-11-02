// ISO 9660: the primary volume descriptor's root directory and the directory records under it, enough to find a
// file by its path (case-insensitive, the ";1" version suffix dropped). Records never cross a sector; a zero length
// ends a sector's records.

use crate::image::{Image, SECTOR};
use crate::{Error, Result, err, u32_at};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub lba: u32,
    pub size: u32,
    pub dir: bool,
}

/// The largest directory read, and the deepest path followed.
const DIR_MAX: u32 = 4 * 1024 * 1024;
const DEPTH_MAX: usize = 32;

/// The root directory's extent, from the volume descriptor at sector 16.
fn root(img: &mut Image) -> Result<Entry> {
    let pvd = img.read_sectors(16, 1)?;
    if pvd.first() != Some(&1) {
        return err("sector 16 is not a primary volume descriptor");
    }
    let rec = pvd.get(156..190).ok_or(Error("volume descriptor".into()))?;
    Ok(record(rec)?.0)
}

/// One directory record, and whether it is the "." or ".." record (names 0 and 1; the root's own record is 0).
fn record(r: &[u8]) -> Result<(Entry, bool)> {
    let len = usize::from(*r.first().ok_or(Error("record".into()))?);
    let name_len = usize::from(*r.get(32).ok_or(Error("record too short".into()))?);
    let name_end = 33usize.checked_add(name_len).ok_or(Error("name".into()))?;
    if len < 34 || name_end > len || r.len() < len {
        return err("directory record out of bounds");
    }
    let raw = r
        .get(33..name_end)
        .ok_or(Error("name past the record".into()))?;
    let dot = raw == [0] || raw == [1];
    let name: String = raw.iter().map(|&c| char::from(c)).collect();
    let name = name
        .split(';')
        .next()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_owned();
    let flags = *r.get(25).ok_or(Error("record".into()))?;
    let e = Entry {
        name,
        lba: u32_at(r, 2)?,
        size: u32_at(r, 10)?,
        dir: flags & 2 != 0,
    };
    Ok((e, dot))
}

/// The entries of a directory.
pub fn list(img: &mut Image, dir: &Entry) -> Result<Vec<Entry>> {
    if !dir.dir || dir.size > DIR_MAX {
        return err(format!("{}: not a directory this reader takes", dir.name));
    }
    let data = img.read(u64::from(dir.lba), u64::from(dir.size))?;
    let mut out = Vec::new();
    for sector in data.chunks(SECTOR) {
        let mut at = 0usize;
        while let Some(&len) = sector.get(at) {
            if len == 0 {
                break;
            }
            let end = at
                .checked_add(usize::from(len))
                .ok_or(Error("record".into()))?;
            let rec = sector
                .get(at..end)
                .ok_or(Error("directory record crosses its sector".into()))?;
            let (e, dot) = record(rec)?;
            // an associated file (flag 4, Mac resource forks) is not the file of that name
            let associated = rec.get(25).is_some_and(|f| f & 4 != 0);
            if !dot && !associated {
                out.push(e);
            }
            at = end;
        }
    }
    Ok(out)
}

/// The file or directory at `path` ("SETUP/DATA1.HDR"), compared without case.
pub fn find(img: &mut Image, path: &str) -> Result<Option<Entry>> {
    let mut at = root(img)?;
    for (depth, part) in path.split('/').filter(|p| !p.is_empty()).enumerate() {
        if depth >= DEPTH_MAX {
            return err("path too deep");
        }
        let Some(next) = list(img, &at)?
            .into_iter()
            .find(|e| e.name.eq_ignore_ascii_case(part))
        else {
            return Ok(None);
        };
        at = next;
    }
    Ok(Some(at))
}

/// A file's bytes.
pub fn read(img: &mut Image, e: &Entry) -> Result<Vec<u8>> {
    if e.dir {
        return err(format!("{} is a directory", e.name));
    }
    img.read(u64::from(e.lba), u64::from(e.size))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    fn rec(name: &[u8], lba: u32, size: u32, dir: bool) -> Vec<u8> {
        let mut r = vec![0u8; 33];
        r[2..6].copy_from_slice(&lba.to_le_bytes());
        r[10..14].copy_from_slice(&size.to_le_bytes());
        r[25] = if dir { 2 } else { 0 };
        r[32] = name.len() as u8;
        r.extend_from_slice(name);
        if r.len() % 2 == 1 {
            r.push(0);
        }
        r[0] = r.len() as u8;
        r
    }

    /// Root at 18 with SETUP/ at 19 holding DATA1.HDR (at sector 17, 5 bytes).
    fn image(dir: &std::path::Path) -> Image {
        let path = dir.join("t.iso");
        let mut sectors = vec![vec![0u8; SECTOR]; 20];
        sectors[16][0] = 1;
        sectors[16][1..6].copy_from_slice(b"CD001");
        let root = rec(&[0], 18, 2048, true);
        sectors[16][156..156 + root.len()].copy_from_slice(&root);
        let mut r = rec(&[0], 18, 2048, true);
        r.extend(rec(&[1], 18, 2048, true));
        r.extend(rec(b"SETUP", 19, 2048, true));
        sectors[18][..r.len()].copy_from_slice(&r);
        let s = rec(b"DATA1.HDR;1", 17, 5, false);
        sectors[19][..s.len()].copy_from_slice(&s);
        sectors[17][..5].copy_from_slice(b"hello");
        std::fs::write(&path, sectors.concat()).unwrap();
        Image::open(&path).unwrap()
    }

    #[test]
    fn a_file_is_found_by_its_path_without_case() {
        let d = std::env::temp_dir().join(format!("cdimage-iso-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let mut img = image(&d);
        let e = find(&mut img, "setup/data1.hdr").unwrap().unwrap();
        assert_eq!((e.name.as_str(), e.size, e.dir), ("DATA1.HDR", 5, false));
        assert_eq!(read(&mut img, &e).unwrap(), b"hello");
        assert_eq!(find(&mut img, "SETUP/NONE").unwrap(), None);
        let setup = find(&mut img, "SETUP").unwrap().unwrap();
        assert!(read(&mut img, &setup).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn bad_records_are_refused() {
        assert!(record(&[10; 10]).is_err(), "shorter than a record");
        let mut r = rec(b"X", 0, 0, false);
        r[32] = 200;
        assert!(record(&r).is_err(), "name past the record");
    }
}
