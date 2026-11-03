// The highscore lists, as the original keeps them: ten places a level, each a name and points, best first. They start
// from the copy's defaults (base.cmo's DB_Highscore_LvNN, lang.rs) and are saved in our own data folder
// (highscores.txt: one line a place, "level points name"), never in the game's files.

use crate::lang::PLACES;
use std::path::PathBuf;

/// The longest name kept, in bytes (the original's entry field holds about this many letters).
pub const NAME_MAX: usize = 16;
/// Levels a file may list, and lines read from it.
const LEVELS_MAX: usize = 64;
const LINES_MAX: usize = LEVELS_MAX * PLACES;
const FILE_MAX: u64 = 64 * 1024;

pub type List = Vec<(Vec<u8>, i64)>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    /// Index 0 is level 1.
    lists: Vec<List>,
}

impl Table {
    /// The saved lists, each level the file lacks taken from `defaults`.
    pub fn load(defaults: &[List]) -> Table {
        // a file larger than any we write is not ours: the defaults stand
        let small = std::fs::metadata(path()).is_ok_and(|m| m.len() <= FILE_MAX);
        let saved = if small {
            std::fs::read(path())
                .map(|b| Table::parse(&b))
                .unwrap_or_default()
        } else {
            Table::default()
        };
        Table::merged(defaults, saved)
    }

    fn merged(defaults: &[List], saved: Table) -> Table {
        let n = defaults.len().max(saved.lists.len());
        let lists = (0..n)
            .map(|i| match saved.lists.get(i) {
                Some(l) if !l.is_empty() => l.clone(),
                _ => defaults.get(i).cloned().unwrap_or_default(),
            })
            .collect();
        Table { lists }
    }

    /// Level `level`'s list (1 is the first level), best first.
    pub fn list(&self, level: u32) -> &[(Vec<u8>, i64)] {
        level
            .checked_sub(1)
            .and_then(|i| self.lists.get(i as usize))
            .map_or(&[], Vec::as_slice)
    }

    /// The place (0 the best) `points` would take on level `level`'s list, if any.
    pub fn place(&self, level: u32, points: i64) -> Option<usize> {
        let list = self.list(level);
        let at = list
            .iter()
            .position(|(_, p)| points > *p)
            .unwrap_or(list.len());
        (at < PLACES && level >= 1 && (level as usize) <= LEVELS_MAX).then_some(at)
    }

    /// Puts `name` with `points` on level `level`'s list; its place, if it made the list.
    pub fn insert(&mut self, level: u32, name: &[u8], points: i64) -> Option<usize> {
        let at = self.place(level, points)?;
        let i = (level - 1) as usize;
        if self.lists.len() <= i {
            self.lists.resize(i + 1, Vec::new());
        }
        let list = self.lists.get_mut(i)?;
        let name = name.get(..NAME_MAX).unwrap_or(name).to_vec();
        list.insert(at.min(list.len()), (name, points));
        list.truncate(PLACES);
        Some(at)
    }

    pub fn save(&self) {
        let path = path();
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Err(e) = std::fs::write(&path, self.to_text()) {
            eprintln!("could not save the highscores to {}: {e}", path.display());
        }
    }

    fn to_text(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (i, list) in self.lists.iter().enumerate() {
            for (name, points) in list {
                out.extend_from_slice(format!("{} {points} ", i + 1).as_bytes());
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

    /// A saved file; lines that do not read as "level points name" are skipped.
    pub(crate) fn parse(bytes: &[u8]) -> Table {
        let mut t = Table::default();
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
            if t.lists.len() < level {
                t.lists.resize(level, Vec::new());
            }
            if let Some(list) = t.lists.get_mut(level - 1)
                && list.len() < PLACES
            {
                list.push((name.get(..NAME_MAX).unwrap_or(name).to_vec(), points));
            }
        }
        for list in &mut t.lists {
            list.sort_by_key(|e| std::cmp::Reverse(e.1));
        }
        t
    }
}

fn path() -> PathBuf {
    crate::levels::data_dir().join("highscores.txt")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn defaults() -> Vec<List> {
        vec![
            (1..=10)
                .rev()
                .map(|k| (b"Mr. Default".to_vec(), k * 400))
                .collect(),
        ]
    }

    #[test]
    fn a_score_takes_its_place_and_the_last_leaves() {
        let mut t = Table::merged(&defaults(), Table::default());
        assert_eq!(t.place(1, 300), None, "below the tenth place");
        assert_eq!(t.insert(1, b"veresk", 3332), Some(2));
        let list = t.list(1);
        assert_eq!(list.len(), PLACES);
        assert_eq!(list[2], (b"veresk".to_vec(), 3332));
        assert_eq!(list[9].1, 800, "400 fell off");
        assert_eq!(
            t.place(2, 5000),
            Some(0),
            "a level without a list takes any score"
        );
        assert_eq!(t.place(0, 5000), None);
    }

    #[test]
    fn the_saved_text_reads_back() {
        let mut t = Table::merged(&defaults(), Table::default());
        t.insert(1, b"J\xfcrgen", 5000);
        let back = Table::merged(&defaults(), Table::parse(&t.to_text()));
        assert_eq!(back, t);
    }

    #[test]
    fn a_bad_file_is_read_as_far_as_it_reads() {
        let t = Table::parse(b"x y z\n1 100 a\n0 5 zero\n99 5 far\n1 notanumber b\n1 200 b c\n");
        assert_eq!(t.list(1), &[(b"b c".to_vec(), 200), (b"a".to_vec(), 100)]);
        assert!(t.list(99).is_empty());
    }
}
