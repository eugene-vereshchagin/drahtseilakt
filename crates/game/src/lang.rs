// The menus' words. The own interface speaks ours (assets/own/Texts/menu.txt, built in): English, Russian, German,
// Simplified Chinese, French, Spanish and Italian. The original's interface speaks its copy's, Language.nmo's table
// `language` (a key, then German, English, Spanish, Italian and French) in the chosen language, and ours for what the
// copy lacks (our additions, and the languages it has not). From the copy always: the key names of its table
// `all_keys` (the original's interface), and from base.cmo the default keys (DB_Options) and each level's default
// highscore list (DB_Highscore_LvNN). The copy's strings are Latin-1 bytes, held here as text.

use crate::assets::Data;
use std::collections::HashMap;

/// The languages of Language.nmo's columns, after the key column.
pub const COPY_LANGUAGES: [&str; 5] = ["de", "en", "es", "it", "fr"];

/// The menus' languages, in the Language page's order, each by its own name; the columns of our texts.
pub const LANGUAGES: [(&str, &str); 7] = [
    ("en", "English"),
    ("ru", "Русский"),
    ("de", "Deutsch"),
    ("zh", "简体中文"),
    ("fr", "Français"),
    ("es", "Español"),
    ("it", "Italiano"),
];

/// Our texts (built in): a key, then a column for each of LANGUAGES.
const OURS: &str = include_str!("../../../assets/own/Texts/menu.txt");

/// Highscore places a level keeps.
pub const PLACES: usize = 10;

#[derive(Clone, Debug, Default)]
pub struct Strings {
    /// The copy's texts (the original's interface in one of the copy's languages), by key.
    text: HashMap<String, String>,
    /// Key names by the row DB_Options keeps for a key (the copy's, for the original's interface).
    keys: Vec<String>,
    /// The rows of the keys for forward, backward, left, right, rotation and overview, and Invert Rotation.
    pub controls: [usize; 6],
    pub invert: bool,
    /// Each level's default highscore list (index 0 is level 1), best first.
    pub highscores: Vec<crate::highscore::List>,
    /// The language's column in LANGUAGES (0, English, when none).
    lang: usize,
}

/// Our text of `key` in the language of column `lang`, English when that column is empty.
fn ours(key: &str, lang: usize) -> Option<&'static str> {
    OURS.lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split(" | ").map(str::trim))
        .find_map(|mut cols| {
            (cols.next()? == key).then(|| {
                let texts: Vec<&str> = cols.collect();
                texts
                    .get(lang)
                    .filter(|t| !t.is_empty())
                    .or(texts.first())
                    .copied()
            })
        })
        .flatten()
}

impl Strings {
    /// The menus' words in `language` (one of LANGUAGES; English when not): the copy's (`copy`, the original's
    /// interface) where it has that language, else ours; the copy's default keys and highscores either way.
    pub fn load(data: &Data, language: &str, copy: bool) -> Strings {
        let mut s = Strings {
            controls: [68, 69, 70, 71, 39, 53],
            lang: LANGUAGES.iter().position(|l| l.0 == language).unwrap_or(0),
            ..Strings::default()
        };
        let column = COPY_LANGUAGES
            .iter()
            .position(|&l| l == language)
            .map(|c| c + 1);
        let cell = |row: &[nmo::scene::Cell], c: usize| match row.get(c) {
            Some(nmo::scene::Cell::String(b)) => Some(latin1(b)),
            _ => None,
        };
        let tables = |n: &str| n == "language" || n == "all_keys";
        let arrays = if copy {
            data.data_arrays("3D Entities/Language.nmo", tables)
        } else {
            Vec::new()
        };
        for (name, a) in arrays {
            for row in &a.rows {
                // a missing translation falls back to English, then German
                let text = [column.unwrap_or(2), 2, 1]
                    .into_iter()
                    .filter_map(|c| cell(row, c))
                    .find(|t| !t.is_empty());
                match name.as_str() {
                    // the menus' texts only in a language the copy has
                    "language" if column.is_some() => {
                        if let (Some(k), Some(t)) = (cell(row, 0), text) {
                            s.text.insert(k, t);
                        }
                    }
                    "language" => {}
                    _ => s.keys.push(text.unwrap_or_default()),
                }
            }
        }
        // the own interface's highscore lists are ours; the copy's default keys count either way
        let wanted = |n: &str| n == "DB_Options" || (copy && n.starts_with("DB_Highscore_Lv"));
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
                        let name = match r.get(pn) {
                            Some(nmo::scene::Cell::String(b)) => b.clone(),
                            _ => return None,
                        };
                        Some((name, points))
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
        if !copy {
            s.own_highscores();
        }
        s
    }

    /// The own interface's default highscore list of every original level: ten names, 3000 points down to 750.
    fn own_highscores(&mut self) {
        const NAMES: [&str; PLACES] = [
            "Ada", "Boris", "Clara", "Dmitri", "Emil", "Fang", "Greta", "Hugo", "Iris", "Jun",
        ];
        let list: crate::highscore::List = NAMES
            .iter()
            .zip((0..).map(|k| 3000 - 250 * k))
            .map(|(n, p)| (n.as_bytes().to_vec(), p))
            .collect();
        self.highscores = vec![list; crate::menu::LEVELS as usize];
    }

    /// The text of `key`: the copy's (Language.nmo's first column), else ours, else `fallback`.
    pub fn get(&self, key: &str, fallback: &str) -> String {
        self.text
            .get(key)
            .cloned()
            .or_else(|| ours(key, self.lang).map(str::to_owned))
            .unwrap_or_else(|| fallback.to_owned())
    }

    /// One of our texts in the menus' language (the key itself when there is none).
    pub fn ours(&self, key: &str) -> String {
        ours(key, self.lang).unwrap_or(key).to_owned()
    }

    /// The copy's (or our) text of `key` with `{n}` as `n`; a copy's text without the mark gets the number after it
    /// ("Highscore Level " + 3).
    pub fn numbered(&self, key: &str, n: u32) -> String {
        let t = self.get(key, "{n}");
        if t.contains("{n}") {
            t.replace("{n}", &n.to_string())
        } else {
            format!("{t}{n}")
        }
    }

    /// The name of a key: the copy's for its row (the original's interface), else ours by winit's `name`.
    pub fn key(&self, row: Option<usize>, name: &str) -> Option<String> {
        row.and_then(|r| self.keys.get(r))
            .filter(|k| !k.is_empty())
            .cloned()
            .or_else(|| ours(&format!("key {name}"), self.lang).map(str::to_owned))
    }

    /// The menus' language (its code in LANGUAGES).
    pub fn language(&self) -> &'static str {
        LANGUAGES.get(self.lang).map_or("en", |l| l.0)
    }
}

/// The system's language from the POSIX locale variables (LC_ALL, LC_MESSAGES, LANG: "ru_RU.UTF-8" is "ru"), if
/// one is set; Windows and macOS set none for a program started from its desktop.
fn system_language() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.is_empty())
        .and_then(|l| l.get(..2).map(str::to_ascii_lowercase))
}

/// The language the menus speak: the settings' choice, else the system's, else English.
pub fn chosen(setting: Option<&str>) -> String {
    [setting.map(str::to_owned), system_language()]
        .into_iter()
        .flatten()
        .find(|l| LANGUAGES.iter().any(|k| k.0 == l))
        .unwrap_or_else(|| "en".to_owned())
}

/// The languages our own texts (docs, the level folders' READMEs) are written in.
pub const DOCS_LANGUAGES: [&str; 4] = ["en", "ru", "de", "zh"];

/// The language of our own texts: the settings' choice, else the system's (LANG), else English.
pub fn docs_language(setting: Option<&str>) -> String {
    [setting.map(str::to_owned), system_language()]
        .into_iter()
        .flatten()
        .find(|l| DOCS_LANGUAGES.contains(&l.as_str()))
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
        assert_eq!(s.get("no such key", "Start"), "Start");
        assert_eq!(
            s.get("Menu_Main Button Start", "x"),
            "Start",
            "ours when the copy has none"
        );
        assert_eq!(s.key(Some(3), "KeyQ"), None);
        assert_eq!(s.key(Some(3), "Space").as_deref(), Some("Space"));
    }

    #[test]
    fn our_texts_have_every_language() {
        let ru = Strings {
            lang: 1,
            ..Strings::default()
        };
        assert_eq!(ru.get("Menu_Main Button Options", ""), "Настройки");
        assert_eq!(ru.numbered("level", 3), "Уровень 3");
        let zh = Strings {
            lang: 3,
            ..Strings::default()
        };
        assert_eq!(
            zh.numbered("Highscore Title (+ SPACE!)", 2),
            "第 2 关最高分"
        );
        assert_eq!(zh.language(), "zh");
        // every line has a key and seven texts, none empty in English
        for l in OURS
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        {
            let cols: Vec<&str> = l.split(" | ").collect();
            assert_eq!(cols.len(), 1 + LANGUAGES.len(), "{l}");
            assert!(cols.iter().all(|c| !c.trim().is_empty()), "{l}");
        }
    }

    #[test]
    fn a_copy_text_without_the_mark_gets_the_number_after_it() {
        let mut s = Strings::default();
        s.text.insert(
            "Highscore Title (+ SPACE!)".into(),
            "Highscore Level ".into(),
        );
        assert_eq!(
            s.numbered("Highscore Title (+ SPACE!)", 4),
            "Highscore Level 4"
        );
    }

    #[test]
    fn the_own_highscores_are_ten_a_level_best_first() {
        let mut s = Strings::default();
        s.own_highscores();
        assert_eq!(s.highscores.len(), crate::menu::LEVELS as usize);
        let first = &s.highscores[0];
        assert_eq!(first.len(), PLACES);
        assert!(first.windows(2).all(|w| w[0].1 > w[1].1));
    }

    #[test]
    fn latin1_keeps_umlauts() {
        assert_eq!(latin1(b"Zur\xfcck"), "Zurück");
    }
}
