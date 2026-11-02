// A disc image: a cue sheet naming its raw bin, a raw bin (2352-byte sectors, mode 1 or mode 2 form 1), or a plain
// iso (2048-byte sectors). What is read from it is the 2048 bytes of user data of each sector.

use crate::{Error, Result, err};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// User data a sector holds.
pub const SECTOR: usize = 2048;
const RAW: u64 = 2352;
const SYNC: [u8; 12] = [
    0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0,
];
/// The largest cue sheet read, and the largest read from the image at once.
const CUE_MAX: u64 = 64 * 1024;
pub const READ_MAX: usize = 256 * 1024 * 1024;

pub struct Image {
    file: std::fs::File,
    /// Bytes a sector takes in the file, and where its user data begins.
    stride: u64,
    data_at: u64,
    sectors: u64,
}

impl Image {
    /// Opens a `.cue` (its first FILE, which must sit next to it), a raw `.bin` or an `.iso`; the format is told by
    /// the file itself: the sync pattern of raw sectors, and the ISO 9660 volume descriptor at sector 16.
    pub fn open(path: &Path) -> Result<Image> {
        let is_cue = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("cue"));
        let path = if is_cue {
            cue_file(path)?
        } else {
            path.to_path_buf()
        };
        let mut file =
            std::fs::File::open(&path).map_err(|e| Error(format!("{}: {e}", path.display())))?;
        let len = file
            .metadata()
            .map_err(|e| Error(format!("{}: {e}", path.display())))?
            .len();
        let mut head = [0u8; 16];
        let raw = len % RAW == 0
            && file.seek(SeekFrom::Start(16 * RAW)).is_ok()
            && file.read_exact(&mut head).is_ok()
            && head.get(..12) == Some(&SYNC[..]);
        let (stride, data_at) = match (raw, head.get(15)) {
            (true, Some(1)) => (RAW, 16),
            // mode 2 form 1: an 8-byte subheader before the data
            (true, Some(2)) => (RAW, 24),
            (true, _) => {
                return err(format!(
                    "{}: a sector mode this reader does not know",
                    path.display()
                ));
            }
            (false, _) if len.checked_rem(SECTOR as u64) == Some(0) => (SECTOR as u64, 0),
            _ => {
                return err(format!(
                    "{}: not a disc image (its size fits no sector size)",
                    path.display()
                ));
            }
        };
        let sectors = len.checked_div(stride).unwrap_or(0);
        let mut img = Image {
            file,
            stride,
            data_at,
            sectors,
        };
        let pvd = img.read_sectors(16, 1)?;
        if pvd.get(1..6) != Some(b"CD001") {
            return err(format!("{}: no ISO 9660 file system", path.display()));
        }
        Ok(img)
    }

    pub fn sectors(&self) -> u64 {
        self.sectors
    }

    /// The user data of `count` sectors from `lba`.
    pub fn read_sectors(&mut self, lba: u64, count: usize) -> Result<Vec<u8>> {
        let end = lba
            .checked_add(count as u64)
            .ok_or(Error("sector range".into()))?;
        let bytes = count.checked_mul(SECTOR).filter(|&n| n <= READ_MAX);
        let (Some(bytes), true) = (bytes, end <= self.sectors) else {
            return err(format!(
                "sectors {lba}..{end} past the image's {}",
                self.sectors
            ));
        };
        let mut out = vec![0u8; bytes];
        for (sector, chunk) in (lba..end).zip(out.chunks_mut(SECTOR)) {
            // within the file: sector < sectors = length / stride
            let at = sector
                .checked_mul(self.stride)
                .and_then(|a| a.checked_add(self.data_at))
                .ok_or(Error("sector offset".into()))?;
            self.file
                .seek(SeekFrom::Start(at))
                .map_err(|e| Error(format!("seek: {e}")))?;
            self.file
                .read_exact(chunk)
                .map_err(|e| Error(format!("read: {e}")))?;
        }
        Ok(out)
    }

    /// `size` bytes of the extent beginning at sector `lba`.
    pub fn read(&mut self, lba: u64, size: u64) -> Result<Vec<u8>> {
        let size = usize::try_from(size).map_err(|_| Error("size".into()))?;
        let mut out = self.read_sectors(lba, size.div_ceil(SECTOR))?;
        out.truncate(size);
        Ok(out)
    }
}

/// The data file a cue sheet names: its first FILE line, a plain file name in the cue's own folder (a path that
/// leads elsewhere is refused).
fn cue_file(cue: &Path) -> Result<PathBuf> {
    let small = std::fs::metadata(cue).is_ok_and(|m| m.len() <= CUE_MAX);
    if !small {
        return err(format!("{}: not a cue sheet (too big)", cue.display()));
    }
    let text = std::fs::read(cue).map_err(|e| Error(format!("{}: {e}", cue.display())))?;
    let text = String::from_utf8_lossy(&text);
    let name = text
        .lines()
        .map(|l| l.trim_start_matches('\u{feff}').trim())
        .find_map(|l| {
            let (word, rest) = l.split_once(char::is_whitespace)?;
            word.eq_ignore_ascii_case("FILE").then(|| rest.trim_start())
        })
        .and_then(|rest| match rest.strip_prefix('"') {
            Some(q) => q.split_once('"').map(|(n, _)| n),
            None => rest.split_whitespace().next(),
        })
        .ok_or(Error(format!(
            "{}: no FILE in the cue sheet",
            cue.display()
        )))?;
    let plain =
        !name.is_empty() && !name.contains(['/', '\\', ':', '\0']) && name != "." && name != "..";
    if !plain {
        return err(format!(
            "{}: the cue sheet names a file elsewhere ({name})",
            cue.display()
        ));
    }
    Ok(cue.with_file_name(name))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    /// A disc of `n` sectors with the volume descriptor's signature at 16, raw (mode 1) or plain.
    pub fn disc(dir: &Path, name: &str, raw: bool, fill: impl Fn(u64) -> Vec<u8>) -> PathBuf {
        let mut out = Vec::new();
        for s in 0..20u64 {
            let mut data = fill(s);
            data.resize(SECTOR, 0);
            if s == 16 {
                data[0] = 1;
                data[1..6].copy_from_slice(b"CD001");
            }
            if raw {
                out.extend_from_slice(&SYNC);
                out.extend_from_slice(&[0, 0, 0, 1]);
                out.extend_from_slice(&data);
                out.extend_from_slice(&[0u8; 288]);
            } else {
                out.extend_from_slice(&data);
            }
        }
        let p = dir.join(name);
        std::fs::write(&p, out).unwrap();
        p
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cdimage-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn raw_plain_and_cue_read_the_same_data() {
        let d = tmp("img");
        let fill = |s: u64| vec![s as u8; 4];
        let bin = disc(&d, "a.bin", true, fill);
        let iso = disc(&d, "a.iso", false, fill);
        std::fs::write(
            d.join("a.cue"),
            "FILE \"a.bin\" BINARY\n  TRACK 01 MODE1/2352\n",
        )
        .unwrap();
        for p in [bin, iso, d.join("a.cue")] {
            let mut img = Image::open(&p).unwrap();
            assert_eq!(img.sectors(), 20);
            assert_eq!(&img.read(3, 6).unwrap(), &[3, 3, 3, 3, 0, 0]);
            assert!(img.read_sectors(19, 2).is_err(), "past the end");
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_cue_that_points_elsewhere_or_a_non_image_is_refused() {
        let d = tmp("cue");
        std::fs::write(d.join("b.cue"), "FILE \"../secret.bin\" BINARY\n").unwrap();
        assert!(Image::open(&d.join("b.cue")).is_err());
        std::fs::write(d.join("c.cue"), "TRACK 01\n").unwrap();
        assert!(Image::open(&d.join("c.cue")).is_err());
        std::fs::write(d.join("odd.iso"), [0u8; 1000]).unwrap();
        assert!(Image::open(&d.join("odd.iso")).is_err());
        let no_fs = disc(&d, "nofs.iso", false, |_| Vec::new());
        let mut bytes = std::fs::read(&no_fs).unwrap();
        bytes[16 * SECTOR + 1] = b'X';
        std::fs::write(&no_fs, bytes).unwrap();
        assert!(Image::open(&no_fs).is_err(), "no volume descriptor");
        let _ = std::fs::remove_dir_all(&d);
    }
}
