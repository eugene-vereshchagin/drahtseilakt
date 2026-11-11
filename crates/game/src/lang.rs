// The menus' words. The own interface speaks ours (assets/own/Texts/<code>.txt, one `key = text` file a language, built in
// and overridden at start by files in the assets or the player's Texts folder; a new file adds a language): English, Russian, German,
// `language` (a key, then German, English, Spanish, Italian and French) in the chosen language, and ours for what the
// copy lacks (our additions, and the languages it has not). From the copy always: the key names of its table
// `all_keys` (the original's interface), and from base.cmo the default keys (DB_Options) and each level's default
// highscore list (DB_Highscore_LvNN). The copy's strings are Latin-1 bytes, held here as text.

use crate::assets::Data;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The languages of Language.nmo's columns, after the key column.
pub const COPY_LANGUAGES: [&str; 5] = ["de", "en", "es", "it", "fr"];

/// The built-in languages' files, in the Language page's order (assets/own/Texts/<code>.txt).
const BUILT_IN: [(&str, &str); 7] = [
    ("en", include_str!("../../../assets/own/Texts/en.txt")),
    ("ru", include_str!("../../../assets/own/Texts/ru.txt")),
    ("de", include_str!("../../../assets/own/Texts/de.txt")),
    ("zh", include_str!("../../../assets/own/Texts/zh.txt")),
    ("fr", include_str!("../../../assets/own/Texts/fr.txt")),
    ("es", include_str!("../../../assets/own/Texts/es.txt")),
    ("it", include_str!("../../../assets/own/Texts/it.txt")),
];

/// The most a language file may weigh; a larger one is ignored.
const FILE_MAX: u64 = 1 << 20;

/// One language's texts by key.
type Table = HashMap<String, String>;

/// Every language: its code, its own name (the file's `language name`) and its texts. English is first.
struct Tables {
    langs: Vec<(&'static str, &'static str)>,
    tables: Vec<Table>,
}

/// The texts of a `key = text` file; comments (`#`) and lines without `=` are skipped.
pub fn parse_table(text: &str) -> Table {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, t)| (k.trim().to_owned(), t.trim().to_owned()))
        .filter(|(k, t)| !k.is_empty() && !t.is_empty())
        .collect()
}

/// A language code a file may carry: short, lowercase ASCII letters, digits and `-`.
fn valid_code(code: &str) -> bool {
    (1..=8).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The built-in languages with the files of `dirs` laid over them, later folders winning: a file of a known code
/// replaces its texts key by key, a file of a new code adds a language (after the built-in ones, by code).
fn build(dirs: &[PathBuf]) -> Tables {
    let mut codes: Vec<String> = BUILT_IN.iter().map(|l| l.0.to_owned()).collect();
    let mut tables: Vec<Table> = BUILT_IN.iter().map(|l| parse_table(l.1)).collect();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut files: Vec<(String, PathBuf)> = entries
            .flatten()
            .filter_map(|e| {
                let path = e.path();
                let code = path.file_name()?.to_str()?.strip_suffix(".txt")?.to_owned();
                valid_code(&code).then_some((code, path))
            })
            .collect();
        // by name, so a run is the same everywhere
        files.sort();
        for (code, path) in files {
            let ok =
                std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= FILE_MAX);
            let Some(bytes) = ok.then(|| std::fs::read(&path).ok()).flatten() else {
                eprintln!("texts: {} not read", path.display());
                continue;
            };
            let table = parse_table(&String::from_utf8_lossy(&bytes));
            let i = match codes.iter().position(|c| *c == code) {
                Some(i) => i,
                None => {
                    codes.push(code);
                    tables.push(Table::new());
                    codes.len() - 1
                }
            };
            if let Some(t) = tables.get_mut(i) {
                t.extend(table);
            }
        }
    }
    let langs = codes
        .iter()
        .zip(&tables)
        .map(|(c, t)| {
            let name = t.get("language name").cloned().unwrap_or_else(|| c.clone());
            // one leak per language for the whole run: the list is built once
            let code: &'static str = Box::leak(c.clone().into_boxed_str());
            let name: &'static str = Box::leak(name.into_boxed_str());
            (code, name)
        })
        .collect();
    Tables { langs, tables }
}

/// The folders whose language files override the built-in ones: the game's assets, then the player's data folder.
fn override_dirs() -> Vec<PathBuf> {
    if cfg!(test) {
        // tests run on the built-in texts, whatever the machine holds
        return Vec::new();
    }
    [
        gamedata::resources::assets_root().map(|r| r.join("own").join("Texts")),
        Some(gamedata::profile::data_dir().join("Texts")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| build(&override_dirs()))
}

/// The menus' languages, in the Language page's order: code and own name. The built-in ones, then any the player's
/// files add.
pub fn languages() -> &'static [(&'static str, &'static str)] {
    &tables().langs
}

/// The keys of the language `code` (built-in text plus overrides), for the tests.
#[cfg(test)]
pub fn table_keys(code: &str) -> Vec<String> {
    keys_of(tables(), code)
}

#[cfg(test)]
fn keys_of(t: &Tables, code: &str) -> Vec<String> {
    t.langs
        .iter()
        .position(|l| l.0 == code)
        .and_then(|i| t.tables.get(i))
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default()
}

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
    /// The language's place in languages() (0, English, when none).
    lang: usize,
}

/// Our text of `key` in the language at place `lang` of `languages()`, English when that language lacks it.
fn ours(key: &str, lang: usize) -> Option<&'static str> {
    let t = tables();
    t.tables
        .get(lang)
        .and_then(|m| m.get(key))
        .or_else(|| t.tables.first().and_then(|m| m.get(key)))
        .map(String::as_str)
}

impl Strings {
    /// The menus' words in `language` (one of languages(); English when not): the copy's (`copy`, the original's
    /// interface) where it has that language, else ours; the copy's default keys and highscores either way.
    pub fn load(data: &Data, language: &str, copy: bool) -> Strings {
        let mut s = Strings {
            controls: [68, 69, 70, 71, 39, 53],
            lang: languages()
                .iter()
                .position(|l| l.0 == language)
                .unwrap_or(0),
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

    /// Our text of `key` in `language` (the key itself when there is none), for text made outside the menus' language.
    pub fn ours_for(key: &str, language: &str) -> String {
        let lang = languages()
            .iter()
            .position(|l| l.0 == language)
            .unwrap_or(0);
        ours(key, lang).unwrap_or(key).to_owned()
    }

    /// Our words in `language`, without a copy (the layout tests).
    #[cfg(test)]
    pub fn ours_in(language: &str) -> Strings {
        let mut s = Strings {
            controls: [68, 69, 70, 71, 39, 53],
            lang: languages()
                .iter()
                .position(|l| l.0 == language)
                .unwrap_or(0),
            ..Strings::default()
        };
        s.own_highscores();
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

    /// The menus' language (its code in languages()).
    pub fn language(&self) -> &'static str {
        languages().get(self.lang).map_or("en", |l| l.0)
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
        .find(|l| languages().iter().any(|k| k.0 == l))
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
    }

    #[test]
    fn every_language_file_has_the_keys_of_english() {
        let t = build(&[]);
        let en = keys_of(&t, "en");
        assert!(en.len() > 200);
        let mut missing = Vec::new();
        for (code, _) in &t.langs {
            let has = keys_of(&t, code);
            for k in &en {
                if !has.contains(k) {
                    missing.push(format!("{code}: {k}"));
                }
            }
            for k in &has {
                if !en.contains(k) {
                    missing.push(format!("{code}: {k} (not in English)"));
                }
            }
        }
        assert!(missing.is_empty(), "keys missing:\n{}", missing.join("\n"));
        // English holds a text for every key, and each file names its language
        for (code, name) in &t.langs {
            assert!(
                !name.is_empty() && *name != *code,
                "{code} has no language name"
            );
        }
    }

    #[test]
    fn a_file_overrides_a_language_and_adds_one() {
        let dir = std::env::temp_dir().join(format!("drahtseilakt-texts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("de.txt"),
            "# fix\nMenu_Main Button Start = Los = geht's\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("pt.txt"),
            "language name = Português\nlevel = Nível {n}\n",
        )
        .unwrap();
        std::fs::write(dir.join("Bad Name.txt"), "language name = x\n").unwrap();
        let t = build(std::slice::from_ref(&dir));
        let _ = std::fs::remove_dir_all(&dir);
        let de = t.langs.iter().position(|l| l.0 == "de").unwrap();
        assert_eq!(t.tables[de]["Menu_Main Button Start"], "Los = geht's");
        assert_eq!(t.tables[de]["Menu_Main Button Options"], "Optionen");
        assert_eq!(t.langs.len(), 8);
        assert_eq!(t.langs[7], ("pt", "Português"));
        assert_eq!(t.tables[7]["level"], "Nível {n}");
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
