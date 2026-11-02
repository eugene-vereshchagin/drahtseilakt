// The copy's own words for the menus: Language.nmo's table `language` (a key, then German, English, Spanish, Italian
// and French) in the chosen language, the key names of its table `all_keys`, and from base.cmo the default keys
// (DB_Options) and each level's default highscore list (DB_Highscore_LvNN). Strings stay bytes in the game's
// Latin-1, as its font draws them.

use crate::assets::Data;
use std::collections::HashMap;

/// The languages of Language.nmo's columns, after the key column.
pub const LANGUAGES: [&str; 5] = ["de", "en", "es", "it", "fr"];

/// Highscore places a level keeps.
pub const PLACES: usize = 10;

#[derive(Clone, Debug, Default)]
pub struct Strings {
    text: HashMap<Vec<u8>, Vec<u8>>,
    /// Key names by the row DB_Options keeps for a key.
    keys: Vec<Vec<u8>>,
    /// The rows of the keys for forward, backward, left, right, rotation and overview, and Invert Rotation.
    pub controls: [usize; 6],
    pub invert: bool,
    /// Each level's default highscore list (index 0 is level 1), best first.
    pub highscores: Vec<crate::highscore::List>,
}

impl Strings {
    /// The copy's strings in `language` (one of LANGUAGES; English when the copy lacks it).
    pub fn load(data: &Data, language: &str) -> Strings {
        let column = LANGUAGES.iter().position(|&l| l == language).unwrap_or(1) + 1;
        let mut s = Strings {
            controls: [68, 69, 70, 71, 39, 53],
            ..Strings::default()
        };
        let cell = |row: &[nmo::scene::Cell], c: usize| match row.get(c) {
            Some(nmo::scene::Cell::String(b)) => Some(b.clone()),
            _ => None,
        };
        for (name, a) in data.data_arrays("3D Entities/Language.nmo", |n| {
            n == "language" || n == "all_keys"
        }) {
            for row in &a.rows {
                // a missing translation falls back to English, then German
                let text = [column, 2, 1]
                    .into_iter()
                    .filter_map(|c| cell(row, c))
                    .find(|t| !t.is_empty());
                match name.as_str() {
                    "language" => {
                        if let (Some(k), Some(t)) = (cell(row, 0), text) {
                            s.text.insert(k, t);
                        }
                    }
                    _ => s.keys.push(text.unwrap_or_default()),
                }
            }
        }
        let wanted = |n: &str| n == "DB_Options" || n.starts_with("DB_Highscore_Lv");
        let mut lists: Vec<(u32, crate::highscore::List)> = Vec::new();
        for (name, a) in data.data_arrays("base.cmo", wanted) {
            if name == "DB_Options" {
                let Some(row) = a.rows.first() else { continue };
                let int = |col: &str| match a.column(col).and_then(|c| row.get(c)) {
                    Some(nmo::scene::Cell::Int(v)) => usize::try_from(*v).ok(),
                    _ => None,
                };
                let cols = [
                    "Key Forward",
                    "Key Backward",
                    "Key Left",
                    "Key Right",
                    "Key Rotate Cam",
                    "Key Lift Cam",
                ];
                for (slot, col) in s.controls.iter_mut().zip(cols) {
                    if let Some(v) = int(col) {
                        *slot = v;
                    }
                }
                s.invert = int("Invert Cam Rotation?").is_some_and(|v| v != 0);
            } else if let Some(n) = name
                .strip_prefix("DB_Highscore_Lv")
                .and_then(|n| n.parse::<u32>().ok())
            {
                let (Some(pn), Some(pp)) = (a.column("Playername"), a.column("Points")) else {
                    continue;
                };
                let mut list: crate::highscore::List = a
                    .rows
                    .iter()
                    .filter_map(|r| {
                        let points = match r.get(pp) {
                            Some(nmo::scene::Cell::Int(v)) => i64::from(*v),
                            Some(nmo::scene::Cell::Float(v)) if v.is_finite() => *v as i64,
                            _ => return None,
                        };
                        Some((cell(r, pn)?, points))
                    })
                    .collect();
                list.sort_by_key(|e| std::cmp::Reverse(e.1));
                list.truncate(PLACES);
                lists.push((n, list));
            }
        }
        lists.sort_by_key(|(n, _)| *n);
        for (n, list) in lists {
            let i = n.saturating_sub(1) as usize;
            if i < 64 {
                if s.highscores.len() <= i {
                    s.highscores.resize(i + 1, Vec::new());
                }
                if let Some(slot) = s.highscores.get_mut(i) {
                    *slot = list;
                }
            }
        }
        s
    }

    /// The text of `key` (Language.nmo's first column), or `fallback` when the copy lacks it.
    pub fn get(&self, key: &str, fallback: &str) -> Vec<u8> {
        self.text
            .get(key.as_bytes())
            .cloned()
            .unwrap_or_else(|| fallback.as_bytes().to_vec())
    }

    /// The name of the key DB_Options keeps as `row`.
    pub fn key(&self, row: usize) -> Vec<u8> {
        self.keys.get(row).cloned().unwrap_or_else(|| b"?".to_vec())
    }
}

/// The language the menus speak: the settings' choice, else the system's (LANG), else English.
pub fn chosen(setting: Option<&str>) -> String {
    let system = std::env::var("LANG")
        .ok()
        .and_then(|l| l.get(..2).map(str::to_ascii_lowercase));
    [setting.map(str::to_owned), system]
        .into_iter()
        .flatten()
        .find(|l| LANGUAGES.contains(&l.as_str()))
        .unwrap_or_else(|| "en".to_owned())
}

/// Latin-1 bytes as text for the prototype's font.
pub fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_key_falls_back() {
        let s = Strings::default();
        assert_eq!(s.get("Menu_Main Button Start", "Start"), b"Start");
        assert_eq!(s.key(3), b"?");
    }

    #[test]
    fn latin1_keeps_umlauts() {
        assert_eq!(latin1(b"Zur\xfcck"), "Zurück");
    }
}
