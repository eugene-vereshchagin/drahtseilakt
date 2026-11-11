// The time played per level (a mod's `playtime`, docs/drahtseilakt/mods.md): statistics kept in the data folder's
// `playtime.txt`, one line `<level file name> <seconds>` per level. Only a mod that asks for it adds to it; nothing
// of a run depends on it. The file is read as hostile input like any other: capped in size and lines, plain names
// only, numbers finite; a line that is not one is skipped.

use std::path::Path;

/// The most levels kept, the longest name, the largest file read.
const MAX_LEVELS: usize = 4096;
const MAX_NAME: usize = 96;
const MAX_FILE: u64 = 256 * 1024;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayTimes {
    levels: Vec<(String, f64)>,
    /// Added to since it was last written.
    pub dirty: bool,
}

fn plain(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl PlayTimes {
    pub fn parse(text: &str) -> PlayTimes {
        let mut t = PlayTimes::default();
        for line in text.lines().take(MAX_LEVELS) {
            let mut f = line.split_whitespace();
            let (Some(name), Some(secs)) = (f.next(), f.next()) else {
                continue;
            };
            if let (true, Ok(s)) = (plain(name), secs.parse::<f64>())
                && s.is_finite()
                && s >= 0.0
                && t.levels.len() < MAX_LEVELS
                && !t.levels.iter().any(|l| l.0 == name)
            {
                t.levels.push((name.to_owned(), s.min(1e9)));
            }
        }
        t
    }

    pub fn load(path: &Path) -> PlayTimes {
        match std::fs::metadata(path) {
            Ok(m) if m.is_file() && m.len() <= MAX_FILE => std::fs::read_to_string(path)
                .map(|s| PlayTimes::parse(&s))
                .unwrap_or_default(),
            _ => PlayTimes::default(),
        }
    }

    pub fn to_text(&self) -> String {
        self.levels
            .iter()
            .map(|(n, s)| format!("{n} {s:.2}\n"))
            .collect()
    }

    pub fn save(&mut self, path: &Path) {
        if self.dirty && std::fs::write(path, self.to_text()).is_ok() {
            self.dirty = false;
        }
    }

    /// `secs` more played on the level (a level name that is not plain, or one too many levels, counts for nothing).
    pub fn add(&mut self, level: &str, secs: f32) {
        if !secs.is_finite() || secs <= 0.0 || !plain(level) {
            return;
        }
        let secs = f64::from(secs.min(1.0));
        if let Some(l) = self.levels.iter_mut().find(|l| l.0 == level) {
            l.1 = (l.1 + secs).min(1e9);
        } else if self.levels.len() < MAX_LEVELS {
            self.levels.push((level.to_owned(), secs));
        } else {
            return;
        }
        self.dirty = true;
    }

    pub fn level(&self, level: &str) -> f32 {
        self.levels
            .iter()
            .find(|l| l.0 == level)
            .map_or(0.0, |l| l.1 as f32)
    }

    pub fn total(&self) -> f32 {
        self.levels.iter().map(|l| l.1).sum::<f64>() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_adds_per_level_and_round_trips() {
        let mut t = PlayTimes::default();
        t.add("Level_01.NMO", 0.5);
        t.add("Level_01.NMO", 0.5);
        t.add("Level_02.NMO", 5.0); // one call counts a second at most
        assert!(t.dirty);
        assert_eq!(
            (t.level("Level_01.NMO"), t.level("Level_09.NMO")),
            (1.0, 0.0)
        );
        assert!((t.total() - 2.0).abs() < 1e-6);
        let back = PlayTimes::parse(&t.to_text());
        assert_eq!(back.level("Level_02.NMO"), 1.0);
    }

    #[test]
    fn hostile_lines_count_for_nothing() {
        let t = PlayTimes::parse(
            "../x 5\nok nan\nok -3\nok inf\nfine 7\nfine 9\n\u{0}bad 1\njustone\n",
        );
        assert_eq!((t.level("fine"), t.level("ok"), t.total()), (7.0, 0.0, 7.0));
        let mut u = PlayTimes::default();
        u.add("a b", 1.0);
        u.add("x", f32::NAN);
        u.add("x", -1.0);
        assert!(!u.dirty);
    }
}
