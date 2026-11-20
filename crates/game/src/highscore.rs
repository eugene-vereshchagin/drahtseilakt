// The highscore lists, as the original keeps them: ten places a level, each a name and points, best first. They start
// from the copy's defaults (base.cmo's DB_Highscore_LvNN, lang.rs) and are saved in our own data folder
// (highscores.txt: one line a place, "level points name"), never in the game's files.

use crate::lang::PLACES;
use std::path::PathBuf;

use gamedata::progress::{FILE_MAX, LEVELS_MAX};
/// The longest name kept, in bytes (the original's entry field holds about this many letters). The file format
/// itself (reading, writing, merging) is in gamedata::progress, which the toolbox uses to take over the original's
/// highscores.
pub use gamedata::progress::{List, NAME_MAX};

const _: () = assert!(PLACES == gamedata::progress::PLACES);

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
        let i = usize::try_from(level.saturating_sub(1)).ok()?;
        if self.lists.len() <= i {
            self.lists.resize(i.saturating_add(1), Vec::new());
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
        gamedata::progress::scores_text(&self.lists)
    }

    /// A saved file; lines that do not read as "level points name" are skipped.
    pub(crate) fn parse(bytes: &[u8]) -> Table {
        Table {
            lists: gamedata::progress::parse_scores(bytes),
        }
    }
}

fn path() -> PathBuf {
    gamedata::progress::scores_path(&crate::levels::data_dir())
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
    fn absurd_lines_are_skipped() {
        let mut text =
            b"18446744073709551615 5 huge\n0 5 zero\n1 99999999999999999999 big\n".to_vec();
        text.extend(b"\xff\xfe 5 raw\n1 -9223372036854775808 low\n\n \n1\n");
        text.extend(std::iter::repeat_n(b'x', 1 << 20));
        let t = Table::parse(&text);
        assert!(t.list(1).len() <= PLACES);
        assert!(t.list(u32::MAX).is_empty());
        let mut t = Table::default();
        assert_eq!(t.insert(0, b"x", 1), None);
        assert_eq!(t.insert(u32::MAX, b"x", 1), None);
    }

    #[test]
    fn a_bad_file_is_read_as_far_as_it_reads() {
        let t = Table::parse(b"x y z\n1 100 a\n0 5 zero\n99 5 far\n1 notanumber b\n1 200 b c\n");
        assert_eq!(t.list(1), &[(b"b c".to_vec(), 200), (b"a".to_vec(), 100)]);
        assert!(t.list(99).is_empty());
    }
}
