// The player's progress, shared by the game and the toolbox: the highscore lists (`highscores.txt`, one line a place,
// "level points name") and the levels unlocked (`unlocked.txt`, one level number a line) in the game's data folder,
// and the reading of the original game's `Database.tdb` (Virtools TT_DatabaseManager arrays) to take both over.
//
// Database.tdb: every byte `e` is `rotr3((-b) ^ 0xaf)`; decoded it is a run of records
//   name NUL, u32 size, then `size` bytes: i32 columns, i32 rows, i32 key column,
//   per column: name NUL, u32 type (1 int, 2 float, 3 string), then the cells column by column
//   (int/float 4 bytes little-endian, strings NUL-terminated).
// Used: `DB_Highscore_LvNN` (columns `Playername` string, `Points` int, ten rows) and `DB_Levelfreischaltung`
// (column `Freigeschaltet?`, one row a level, nonzero = unlocked). The file is a stranger's: every read is checked.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Places on a level's list, the longest name kept (bytes), levels a file may list.
pub const PLACES: usize = 10;
pub const NAME_MAX: usize = 16;
pub const LEVELS_MAX: usize = 64;
const LINES_MAX: usize = LEVELS_MAX * PLACES;
/// The largest highscores or unlocked file read (more is not ours).
pub const FILE_MAX: u64 = 64 * 1024;
/// The largest Database.tdb read, and the limits on one array of it.
pub const TDB_MAX: u64 = 4 * 1024 * 1024;
const COLS_MAX: usize = 64;
const ROWS_MAX: usize = 4096;

pub type List = Vec<(Vec<u8>, i64)>;

pub fn scores_path(data_dir: &Path) -> PathBuf {
    data_dir.join("highscores.txt")
}

pub fn unlocked_path(data_dir: &Path) -> PathBuf {
    data_dir.join("unlocked.txt")
}

/// The lists of a saved highscores file (index 0 is level 1), best first; lines that do not read as
/// "level points name" are skipped.
pub fn parse_scores(bytes: &[u8]) -> Vec<List> {
    let mut lists: Vec<List> = Vec::new();
    for line in bytes.split(|&c| c == b'\n').take(LINES_MAX) {
        let mut parts = line.splitn(3, |&c| c == b' ');
        let num = |p: Option<&[u8]>| {
            p.and_then(|p| std::str::from_utf8(p).ok())
                .map(str::trim)
                .map(str::to_owned)
        };
        let (Some(level), Some(points)) = (num(parts.next()), num(parts.next())) else {
            continue;
        };
        let (Ok(level), Ok(points)) = (level.parse::<usize>(), points.parse::<i64>()) else {
            continue;
        };
        if level == 0 || level > LEVELS_MAX {
            continue;
        }
        let name = parts.next().unwrap_or_default();
        let name = name.strip_suffix(b"\r").unwrap_or(name);
        if lists.len() < level {
            lists.resize(level, Vec::new());
        }
        if let Some(list) = lists.get_mut(level.saturating_sub(1))
            && list.len() < PLACES
        {
            list.push((name.get(..NAME_MAX).unwrap_or(name).to_vec(), points));
        }
    }
    for list in &mut lists {
        list.sort_by_key(|e| std::cmp::Reverse(e.1));
    }
    lists
}

/// The saved text of the lists.
pub fn scores_text(lists: &[List]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, list) in lists.iter().enumerate() {
        for (name, points) in list {
            out.extend_from_slice(format!("{} {points} ", i.saturating_add(1)).as_bytes());
            // the name in one line: Latin-1 bytes without line breaks
            out.extend(
                name.iter()
                    .map(|&c| if c == b'\n' || c == b'\r' { b' ' } else { c }),
            );
            out.push(b'\n');
        }
    }
    out
}

/// `more` taken into `into`: per level the entries of both (one that is in both once), best first, the top
/// `PLACES` kept. Nothing in `into` is lost that stays among the top.
pub fn merge_scores(into: &mut Vec<List>, more: &[List]) {
    if into.len() < more.len() {
        into.resize(more.len(), Vec::new());
    }
    for (list, add) in into.iter_mut().zip(more) {
        for e in add {
            let e = (e.0.get(..NAME_MAX).unwrap_or(&e.0).to_vec(), e.1);
            if !list.contains(&e) {
                list.push(e);
            }
        }
        list.sort_by_key(|e| std::cmp::Reverse(e.1));
        list.truncate(PLACES);
    }
}

/// The level numbers of an unlocked file (one a line, 1 to `LEVELS_MAX`; other lines skipped).
pub fn parse_unlocked(bytes: &[u8]) -> BTreeSet<u32> {
    bytes
        .split(|&c| c == b'\n')
        .take(LEVELS_MAX.saturating_mul(4))
        .filter_map(|l| std::str::from_utf8(l).ok()?.trim().parse::<u32>().ok())
        .filter(|&n| n >= 1 && n as usize <= LEVELS_MAX)
        .collect()
}

pub fn unlocked_text(levels: &BTreeSet<u32>) -> String {
    levels.iter().map(|n| format!("{n}\n")).collect()
}

/// A plain file read up to `cap` bytes; empty when there is none, an error when it is not a plain file or too large.
fn read_capped(path: &Path, cap: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
        Ok(m) if !m.is_file() => return Err(format!("{}: not a plain file", path.display())),
        Ok(_) => {}
    }
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    f.take(cap.saturating_add(1))
        .read_to_end(&mut out)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if out.len() as u64 > cap {
        return Err(format!("{}: larger than {cap} bytes", path.display()));
    }
    Ok(out)
}

/// The levels unlocked in the data folder (none when the file is missing or unreadable).
pub fn read_unlocked(data_dir: &Path) -> BTreeSet<u32> {
    read_capped(&unlocked_path(data_dir), FILE_MAX)
        .map(|b| parse_unlocked(&b))
        .unwrap_or_default()
}

/// What the original game's file holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// Levels unlocked (1-based), ascending.
    pub unlocked: Vec<u32>,
    /// Index 0 is level 1; levels the file lacks have an empty list.
    pub scores: Vec<List>,
}

fn decode(e: u8) -> u8 {
    0u8.wrapping_sub(e.rotate_left(3) ^ 0xaf)
}

struct Cur<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Cur<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).ok_or("size overflow")?;
        let s = self.b.get(self.at..end).ok_or("the file is cut off")?;
        self.at = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let s: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| "the file is cut off")?;
        Ok(u32::from_le_bytes(s))
    }
    fn i32(&mut self) -> Result<i32, String> {
        self.u32().map(|v| v as i32)
    }
    /// A NUL-terminated string without its NUL.
    fn cstr(&mut self) -> Result<&'a [u8], String> {
        let rest = self.b.get(self.at..).unwrap_or_default();
        let n = rest
            .iter()
            .position(|&c| c == 0)
            .ok_or("a string is not ended")?;
        let s = self.take(n)?;
        self.take(1)?;
        Ok(s)
    }
}

enum Col {
    Int(Vec<i32>),
    Str(Vec<Vec<u8>>),
}

/// One array's columns by name.
fn read_array(blk: &[u8]) -> Result<Vec<(Vec<u8>, Col)>, String> {
    let mut c = Cur { b: blk, at: 0 };
    let (ncols, nrows, key) = (c.i32()?, c.i32()?, c.i32()?);
    let ncols = usize::try_from(ncols).map_err(|_| "negative column count")?;
    let nrows = usize::try_from(nrows).map_err(|_| "negative row count")?;
    if ncols > COLS_MAX || nrows > ROWS_MAX || key < -1 || key >= ncols as i32 {
        return Err("an array's size is out of range".to_owned());
    }
    let mut heads = Vec::new();
    for _ in 0..ncols {
        let name = c.cstr()?.to_vec();
        let ty = c.u32()?;
        if !(1..=3).contains(&ty) {
            return Err("unknown column type".to_owned());
        }
        heads.push((name, ty));
    }
    let mut cols = Vec::new();
    for (name, ty) in heads {
        let col = if ty == 3 {
            let mut v = Vec::new();
            for _ in 0..nrows {
                v.push(c.cstr()?.to_vec());
            }
            Col::Str(v)
        } else {
            let mut v = Vec::new();
            for _ in 0..nrows {
                v.push(c.i32()?);
            }
            Col::Int(v)
        };
        cols.push((name, col));
    }
    Ok(cols)
}

/// The progress in the bytes of a Database.tdb. An error when it is not one (cut off, out of range, none of the
/// arrays); never a panic.
pub fn read_tdb(bytes: &[u8]) -> Result<Progress, String> {
    if bytes.len() as u64 > TDB_MAX {
        return Err("Database.tdb is too large".to_owned());
    }
    let d: Vec<u8> = bytes.iter().map(|&e| decode(e)).collect();
    let mut c = Cur { b: &d, at: 0 };
    let mut p = Progress::default();
    let mut found = false;
    while c.at < d.len() {
        let name = c.cstr()?;
        let size = c.u32()? as usize;
        let blk = c.take(size)?;
        if name == b"DB_Levelfreischaltung" {
            let cols = read_array(blk)?;
            let Some((_, Col::Int(flags))) = cols.into_iter().next() else {
                return Err("DB_Levelfreischaltung has no flag column".to_owned());
            };
            p.unlocked = flags
                .iter()
                .enumerate()
                .filter(|&(i, &f)| f != 0 && i < LEVELS_MAX)
                .map(|(i, _)| (i as u32).saturating_add(1))
                .collect();
            found = true;
        } else if let Some(n) = name.strip_prefix(b"DB_Highscore_Lv") {
            let level: usize = std::str::from_utf8(n)
                .ok()
                .and_then(|s| s.parse().ok())
                .filter(|&l| (1..=LEVELS_MAX).contains(&l))
                .ok_or("a highscore array has a bad level number")?;
            let cols = read_array(blk)?;
            let (mut names, mut points) = (None, None);
            for (cn, col) in cols {
                match (cn.as_slice(), col) {
                    (b"Playername", Col::Str(v)) => names = Some(v),
                    (b"Points", Col::Int(v)) => points = Some(v),
                    _ => {}
                }
            }
            let (Some(names), Some(points)) = (names, points) else {
                return Err("a highscore array lacks Playername or Points".to_owned());
            };
            let mut list: List = names
                .into_iter()
                .zip(points)
                .map(|(n, p)| (n.get(..NAME_MAX).unwrap_or(&n).to_vec(), i64::from(p)))
                .collect();
            list.sort_by_key(|e| std::cmp::Reverse(e.1));
            list.truncate(PLACES);
            if p.scores.len() < level {
                p.scores.resize(level, Vec::new());
            }
            if let Some(slot) = p.scores.get_mut(level.saturating_sub(1)) {
                *slot = list;
            }
            found = true;
        }
    }
    if !found {
        return Err("no highscores or unlocked levels in this file".to_owned());
    }
    Ok(p)
}

/// What taking over changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    /// Levels newly unlocked, and levels whose list gained entries.
    pub unlocked: usize,
    pub levels_with_scores: usize,
    /// Entries newly on a list.
    pub scores: usize,
}

/// Merges `p` into the data folder `dir`: unlocked levels are a union, the highscore lists keep the top places of
/// both. Nothing is lowered. Files are written whole (temporary file, then renamed).
pub fn take_over(p: &Progress, dir: &Path) -> Result<Taken, String> {
    let mut taken = Taken::default();
    let mut have = parse_unlocked(&read_capped(&unlocked_path(dir), FILE_MAX)?);
    let before = have.len();
    have.extend(p.unlocked.iter().copied());
    taken.unlocked = have.len().saturating_sub(before);

    let mut lists = parse_scores(&read_capped(&scores_path(dir), FILE_MAX)?);
    let old = lists.clone();
    merge_scores(&mut lists, &p.scores);
    for (i, l) in lists.iter().enumerate() {
        let was = old.get(i).map_or(&[][..], Vec::as_slice);
        let new = l.iter().filter(|e| !was.contains(e)).count();
        taken.scores = taken.scores.saturating_add(new);
        if new > 0 {
            taken.levels_with_scores = taken.levels_with_scores.saturating_add(1);
        }
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if taken.unlocked > 0 {
        write_whole(&unlocked_path(dir), unlocked_text(&have).as_bytes())?;
    }
    if taken.scores > 0 {
        write_whole(&scores_path(dir), &scores_text(&lists))?;
    }
    Ok(taken)
}

fn write_whole(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads the `Database.tdb` in an original game's folder (the name's case as found) and takes its progress over
/// into the data folder `dir`.
pub fn import_from_game(game: &Path, dir: &Path) -> Result<(Progress, Taken), String> {
    let file = std::fs::read_dir(game)
        .map_err(|e| format!("{}: {e}", game.display()))?
        .flatten()
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("Database.tdb")
        })
        .map(|e| e.path())
        .ok_or_else(|| format!("{}: no Database.tdb", game.display()))?;
    let bytes = read_capped(&file, TDB_MAX)?;
    let p = read_tdb(&bytes)?;
    let t = take_over(&p, dir)?;
    Ok((p, t))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn sample(name: &str) -> Option<Vec<u8>> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../ref")
            .join(name)
            .join("Database.tdb");
        std::fs::read(p).ok()
    }

    fn encode(b: u8) -> u8 {
        ((0u8.wrapping_sub(b)) ^ 0xaf).rotate_right(3)
    }

    /// A small file of our own: level 1 list of two, levels 1 and 3 unlocked.
    fn made() -> Vec<u8> {
        let mut d = Vec::new();
        let mut array = |name: &str, cols: &[(&str, u32)], rows: u32, cells: &[u8]| {
            d.extend(name.bytes());
            d.push(0);
            let mut b = Vec::new();
            b.extend((cols.len() as i32).to_le_bytes());
            b.extend((rows as i32).to_le_bytes());
            b.extend((-1i32).to_le_bytes());
            for (n, t) in cols {
                b.extend(n.bytes());
                b.push(0);
                b.extend(t.to_le_bytes());
            }
            b.extend(cells);
            d.extend((b.len() as u32).to_le_bytes());
            d.extend(b);
        };
        let mut cells = b"J\xfcrgen\0Bo\0".to_vec();
        cells.extend(500i32.to_le_bytes());
        cells.extend(900i32.to_le_bytes());
        array(
            "DB_Highscore_Lv01",
            &[("Playername", 3), ("Points", 1)],
            2,
            &cells,
        );
        let mut flags = Vec::new();
        for f in [1i32, 0, 1] {
            flags.extend(f.to_le_bytes());
        }
        array(
            "DB_Levelfreischaltung",
            &[("Freigeschaltet?", 1)],
            3,
            &flags,
        );
        d.iter().map(|&b| encode(b)).collect()
    }

    #[test]
    fn a_made_file_reads() {
        let p = read_tdb(&made()).unwrap();
        assert_eq!(p.unlocked, vec![1, 3]);
        assert_eq!(
            p.scores,
            vec![vec![(b"Bo".to_vec(), 900), (b"J\xfcrgen".to_vec(), 500)]]
        );
    }

    #[test]
    fn the_original_files_read() {
        let mut seen = 0;
        for (dir, unlocked) in [
            ("Ballance", vec![1, 2]),
            ("Ballance-13", (1..=12).collect()),
        ] {
            let Some(b) = sample(dir) else { continue };
            seen += 1;
            let p = read_tdb(&b).unwrap();
            assert_eq!(p.unlocked, unlocked, "{dir}");
            assert_eq!(p.scores.len(), 12);
            let l1: Vec<i64> = p.scores[0].iter().map(|e| e.1).collect();
            assert_eq!(
                l1,
                [4000, 3600, 3332, 3200, 2800, 2400, 2000, 1600, 1200, 800]
            );
            assert_eq!(p.scores[0][2].0, b"veresk");
            assert_eq!(p.scores[0][0].0, b"Mr. Default");
            let l2: Vec<i64> = p.scores[1].iter().map(|e| e.1).collect();
            assert_eq!(
                l2,
                [4000, 3600, 3200, 2800, 2400, 2000, 1600, 1200, 800, 400]
            );
            assert!(p.scores.iter().all(|l| l.len() == PLACES));
        }
        if seen == 0 {
            eprintln!("no ref/ sample files here; real-file checks skipped");
        }
    }

    /// Offsets where a record ends (a cut there is a shorter, valid file).
    fn boundaries(enc: &[u8]) -> Vec<usize> {
        let d: Vec<u8> = enc.iter().map(|&e| decode(e)).collect();
        let mut c = Cur { b: &d, at: 0 };
        let mut v = vec![0];
        while c.at < d.len() {
            c.cstr().unwrap();
            let n = c.u32().unwrap() as usize;
            c.take(n).unwrap();
            v.push(c.at);
        }
        v
    }

    fn every_cut_errors_or_is_a_boundary(b: &[u8]) {
        let ends = boundaries(b);
        for n in 0..b.len() {
            let r = read_tdb(&b[..n]);
            if !ends.contains(&n) {
                assert!(r.is_err(), "cut at {n} of {}", b.len());
            }
        }
    }

    #[test]
    fn truncations_are_errors() {
        every_cut_errors_or_is_a_boundary(&made());
        if let Some(b) = sample("Ballance") {
            every_cut_errors_or_is_a_boundary(&b);
        }
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        let mut x = 12345u32;
        for len in [0usize, 1, 5, 17, 300, 5000] {
            let junk: Vec<u8> = (0..len)
                .map(|_| {
                    x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                    (x >> 24) as u8
                })
                .collect();
            let _ = read_tdb(&junk);
        }
        // flipped bytes of a good file
        let good = made();
        for i in 0..good.len() {
            let mut b = good.clone();
            b[i] ^= 0x55;
            let _ = read_tdb(&b);
        }
        // a record claiming 4 GB, and absurd counts
        let mut d = b"DB_Levelfreischaltung\0".to_vec();
        d.extend(u32::MAX.to_le_bytes());
        let enc: Vec<u8> = d.iter().map(|&b| encode(b)).collect();
        assert!(read_tdb(&enc).is_err());
        let mut d = b"DB_Levelfreischaltung\0".to_vec();
        d.extend(12u32.to_le_bytes());
        for v in [i32::MAX, i32::MAX, -1] {
            d.extend(v.to_le_bytes());
        }
        let enc: Vec<u8> = d.iter().map(|&b| encode(b)).collect();
        assert!(read_tdb(&enc).is_err());
    }

    #[test]
    fn taking_over_only_adds() {
        let dir = std::env::temp_dir().join(format!("progress-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(unlocked_path(&dir), "5\n1\n").unwrap();
        std::fs::write(scores_path(&dir), "1 700 mine\n1 100 low\n").unwrap();
        let p = read_tdb(&made()).unwrap();
        let t = take_over(&p, &dir).unwrap();
        assert_eq!(
            t,
            Taken {
                unlocked: 1,
                levels_with_scores: 1,
                scores: 2
            }
        );
        assert_eq!(read_unlocked(&dir), BTreeSet::from([1, 3, 5]));
        let l = parse_scores(&std::fs::read(scores_path(&dir)).unwrap());
        let pts: Vec<i64> = l[0].iter().map(|e| e.1).collect();
        assert_eq!(pts, [900, 700, 500, 100]);
        // again: nothing changes
        assert_eq!(take_over(&p, &dir).unwrap(), Taken::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn merge_keeps_the_top_ten_and_never_duplicates() {
        let full: List = (1..=10).map(|k| (b"a".to_vec(), k * 100)).collect();
        let mut into = vec![full.clone()];
        merge_scores(
            &mut into,
            &[vec![(b"b".to_vec(), 50), (b"c".to_vec(), 1500)]],
        );
        assert_eq!(into[0].len(), PLACES);
        assert_eq!(into[0][0], (b"c".to_vec(), 1500));
        assert_eq!(into[0][9].1, 200);
        merge_scores(&mut into, &[full]);
        assert_eq!(into[0].len(), PLACES);
    }
}
