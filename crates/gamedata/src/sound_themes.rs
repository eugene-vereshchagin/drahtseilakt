// Sound themes (docs/drahtseilakt/resources.md): a theme is a folder of sounds and music by the names the game asks
// for (Sounds/<name>.ogg|.wav, Music/<name>.ogg|.wav, or a track Music/music.txt names). The game ships two in its
// assets (sounds/system: openly licensed recordings; sounds/neuro: our synthesized ones); the player may add more in
// the data folder's sounds/ — among them "original", which the toolbox makes from the player's copy and which is
// never shipped. The generated theme (neuro) plays unless the player chose another (owner's decision 2026-11-02);
// the extended mode chooses (`sound-theme NAME`), the classic mode plays the default. A sound the chosen theme
// lacks comes from its fallback theme (theme.txt `fallback NAME`, the system theme when none), then — for a level's
// own sounds, never for music — from the copy.
//
// The player's theme folders are untrusted (they are shared): only plain folder and file names, real folders and
// files (no links), a capped number of themes and a capped theme.txt; the sounds themselves are decoded through the
// audio layer's caps.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The most themes taken (shipped and the player's together), the largest theme.txt and the longest name.
const MAX_THEMES: usize = 32;
const MAX_THEME_TXT: u64 = 4096;
const MAX_NAME: usize = 64;
/// The theme every theme falls back to (and the one played when the default theme is not there).
pub const SYSTEM: &str = "system";
/// The theme played when the player has not chosen one: the generated theme (decisions.md, 2026-11-02).
pub const DEFAULT: &str = "neuro";
/// The theme the toolbox makes from the player's copy; while there is no such folder, choosing it plays the copy's
/// sounds directly.
pub const ORIGINAL: &str = "original";

/// A sound theme found.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Its folder's name, which the setting names.
    pub key: String,
    pub dir: PathBuf,
    /// Shipped with the game (not the player's).
    pub shipped: bool,
    /// Its name per language (`name.<lang>` lines).
    pub names: Vec<(String, String)>,
    /// The theme a sound it lacks comes from.
    pub fallback: Option<String>,
}

impl Theme {
    /// Its name in `lang` (en, ru, de, zh), else in English, else its key.
    pub fn name(&self, lang: &str) -> String {
        let find = |l: &str| {
            self.names
                .iter()
                .find(|(k, _)| k == l)
                .map(|(_, v)| v.clone())
        };
        find(lang)
            .or_else(|| find("en"))
            .unwrap_or_else(|| self.key.clone())
    }
}

/// A name a theme folder or a sound file may have: letters, digits, `_`, `-` and `.`, not beginning with a dot.
pub fn plain(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 96
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn real_dir(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir())
}

fn real_file(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file())
}

/// The shipped themes' folder (the game's assets), and the player's.
pub fn shipped_root() -> Option<PathBuf> {
    crate::resources::assets_root().map(|a| a.join("sounds"))
}

pub fn user_root() -> PathBuf {
    crate::profile::data_dir().join("sounds")
}

/// The themes in `root`: its plain-named real folders, by name.
fn themes_in(root: &Path, shipped: bool) -> Vec<Theme> {
    let Ok(d) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = d
        .flatten()
        .take(1024)
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|k| plain(k) && k.len() <= MAX_NAME && real_dir(&root.join(k)))
        .collect();
    keys.sort();
    keys.into_iter()
        .take(MAX_THEMES)
        .map(|key| {
            let dir = root.join(&key);
            let (names, fallback) = read_theme_txt(&dir.join("theme.txt"));
            Theme {
                key,
                dir,
                shipped,
                names,
                fallback,
            }
        })
        .collect()
}

/// A theme.txt's names and fallback; nothing from a file that is not a small plain one.
fn read_theme_txt(path: &Path) -> (Vec<(String, String)>, Option<String>) {
    use std::io::Read;
    let mut text = String::new();
    if real_file(path)
        && let Ok(f) = std::fs::File::open(path)
    {
        let _ = f.take(MAX_THEME_TXT).read_to_string(&mut text);
    }
    let mut names = Vec::new();
    let mut fallback = None;
    for line in text.lines().filter(|l| !l.starts_with('#')).take(64) {
        let Some((k, v)) = line.split_once(' ') else {
            continue;
        };
        let v: String = v
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_NAME)
            .collect();
        if let Some(lang) = k.strip_prefix("name.") {
            if matches!(lang, "en" | "ru" | "de" | "zh") && !v.is_empty() {
                names.push((lang.to_owned(), v));
            }
        } else if k == "fallback" && plain(&v) {
            fallback = Some(v);
        }
    }
    (names, fallback)
}

/// Every theme: the shipped ones, then the player's (a player's folder named like a shipped theme is left out).
pub fn scan() -> Vec<Theme> {
    let mut all = shipped_root()
        .map(|r| themes_in(&r, true))
        .unwrap_or_default();
    for t in themes_in(&user_root(), false) {
        if all.len() < MAX_THEMES && !all.iter().any(|s| s.key == t.key) {
            all.push(t);
        }
    }
    all
}

/// The themes as the game found them at its start, and the one chosen.
struct State {
    themes: Vec<Theme>,
    chosen: String,
    /// What the player chose, kept as it is for the settings even when that theme is not there this time; none when
    /// they never chose (then the default plays, and a later default is followed).
    wanted: Option<String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// The theme that plays for the player's choice `wanted` among `themes`: the chosen one when it is there ("original"
/// always: it plays the copy's sounds until the toolbox has made its folder), else the default, else the system theme.
fn playing(themes: &[Theme], wanted: Option<&str>) -> String {
    let found = |k: &str| themes.iter().any(|t| t.key == k);
    match wanted {
        Some(w) if found(w) || w == ORIGINAL => w.to_owned(),
        _ if found(DEFAULT) => DEFAULT.to_owned(),
        _ => SYSTEM.to_owned(),
    }
}

/// Finds the themes and takes the player's choice (the setting's; none when they never chose one).
pub fn start(wanted: Option<&str>) {
    let themes = scan();
    let chosen = playing(&themes, wanted);
    if let Ok(mut s) = STATE.lock() {
        *s = Some(State {
            themes,
            chosen,
            wanted: wanted.map(str::to_owned),
        });
    }
}

/// Plays the player's choice again (none: the default) without choosing anything — the mode switch: the classic
/// mode plays the default, the extended mode the saved choice.
pub fn restore(wanted: Option<&str>) {
    if let Ok(mut s) = STATE.lock()
        && let Some(s) = s.as_mut()
    {
        s.chosen = playing(&s.themes, wanted);
        s.wanted = wanted.map(str::to_owned);
    }
}

/// The themes found at the start.
pub fn themes() -> Vec<Theme> {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.themes.clone()))
        .unwrap_or_default()
}

/// The theme chosen (its key).
pub fn chosen() -> String {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.chosen.clone()))
        .unwrap_or_else(|| DEFAULT.to_owned())
}

/// Chooses another theme (it plays from the next sound loaded; the game forgets the ones loaded).
pub fn choose(key: &str) {
    if let Ok(mut s) = STATE.lock()
        && let Some(s) = s.as_mut()
        && (s.themes.iter().any(|t| t.key == key) || key == ORIGINAL)
    {
        s.chosen = key.to_owned();
        s.wanted = Some(key.to_owned());
    }
}

/// The theme the player chose, for the settings: kept even when it was not found this time (a player's theme folder
/// moved away for a while is not forgotten); none when they never chose one.
pub fn wanted() -> Option<String> {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().and_then(|s| s.wanted.clone()))
}

/// The copy's own sounds play: "original" chosen while the toolbox has not made its folder.
pub fn plays_copy() -> bool {
    let c = chosen();
    c == ORIGINAL && !themes().iter().any(|t| t.key == ORIGINAL)
}

/// The keys that can be chosen, in order: the themes found, and "original" when it has no folder yet.
pub fn choices() -> Vec<String> {
    let mut keys: Vec<String> = themes().into_iter().map(|t| t.key).collect();
    if !keys.iter().any(|k| k == ORIGINAL) {
        keys.push(ORIGINAL.to_owned());
    }
    keys
}

/// The files of the sound `name`, in the order they are tried: the chosen theme's, its fallback's, the system
/// theme's (a broken one is passed over for the next).
pub fn files(name: &str) -> Vec<PathBuf> {
    if !plain(name) {
        return Vec::new();
    }
    let themes = themes();
    let first = chosen();
    let mut keys = vec![first.clone()];
    if let Some(f) = themes
        .iter()
        .find(|t| t.key == first)
        .and_then(|t| t.fallback.clone())
    {
        keys.push(f);
    }
    keys.push(SYSTEM.to_owned());
    keys.dedup();
    keys.iter()
        .filter_map(|k| themes.iter().find(|t| &t.key == k))
        .filter_map(|t| in_theme(&t.dir, name))
        .collect()
}

/// A sound's file in one theme folder: Sounds/ or Music/, Ogg first, then WAV, then the track music.txt names.
pub fn in_theme(dir: &Path, name: &str) -> Option<PathBuf> {
    if !plain(name) || !real_dir(dir) {
        return None;
    }
    for folder in ["Sounds", "Music"] {
        if !real_dir(&dir.join(folder)) {
            continue;
        }
        for ext in ["ogg", "wav"] {
            let p = dir.join(folder).join(format!("{name}.{ext}"));
            if real_file(&p) {
                return Some(p);
            }
        }
    }
    let music = dir.join("Music");
    let table = music_table(&music);
    let file = &table.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?.1;
    let p = music.join(file);
    real_file(&p).then_some(p)
}

/// A theme's music.txt (`<name> <file>` lines), read once per folder.
fn music_table(music: &Path) -> Vec<(String, String)> {
    use std::io::Read;
    static TABLES: Mutex<Vec<(PathBuf, Vec<(String, String)>)>> = Mutex::new(Vec::new());
    if let Ok(t) = TABLES.lock()
        && let Some((_, table)) = t.iter().find(|(p, _)| p == music)
    {
        return table.clone();
    }
    let path = music.join("music.txt");
    let mut text = String::new();
    if real_dir(music)
        && real_file(&path)
        && let Ok(f) = std::fs::File::open(&path)
    {
        let _ = f.take(64 * 1024).read_to_string(&mut text);
    }
    let table: Vec<(String, String)> = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once(' '))
        .map(|(a, b)| (a.trim().to_owned(), b.trim().to_owned()))
        .filter(|(a, b)| plain(a) && plain(b) && (b.ends_with(".ogg") || b.ends_with(".wav")))
        .take(256)
        .collect();
    if let Ok(mut t) = TABLES.lock()
        && t.len() < MAX_THEMES
    {
        t.push((music.to_path_buf(), table.clone()));
    }
    table
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn root(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("drahtseilakt-themes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn themes_are_plain_real_folders_with_their_names() {
        let r = root("scan");
        std::fs::create_dir_all(r.join("calm/Sounds")).unwrap();
        std::fs::write(
            r.join("calm/theme.txt"),
            "name.en Calm\nname.ru Тихая\nfallback system\nname.xx no\n",
        )
        .unwrap();
        std::fs::create_dir_all(r.join(".hidden")).unwrap();
        std::fs::create_dir_all(r.join("with space")).unwrap();
        std::fs::write(r.join("a-file"), b"").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(r.join("calm"), r.join("linked")).unwrap();
        let t = themes_in(&r, false);
        assert_eq!(
            t.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(),
            ["calm"]
        );
        assert_eq!(t[0].name("ru"), "Тихая");
        assert_eq!(t[0].name("zh"), "Calm");
        assert_eq!(t[0].fallback.as_deref(), Some("system"));
        std::fs::remove_dir_all(&r).unwrap();
    }

    #[test]
    fn a_sound_is_found_by_plain_name_only_and_never_through_a_link() {
        let r = root("files");
        std::fs::create_dir_all(r.join("t/Sounds")).unwrap();
        std::fs::create_dir_all(r.join("t/Music")).unwrap();
        std::fs::write(r.join("t/Sounds/Misc_Fall.wav"), b"x").unwrap();
        std::fs::write(r.join("t/Sounds/Misc_Fall.ogg"), b"x").unwrap();
        std::fs::write(r.join("t/Music/piece.ogg"), b"x").unwrap();
        std::fs::write(
            r.join("t/Music/music.txt"),
            "Music_Theme_1_1 piece.ogg\nBad ../x.ogg\n",
        )
        .unwrap();
        let t = r.join("t");
        assert_eq!(
            in_theme(&t, "Misc_Fall"),
            Some(t.join("Sounds/Misc_Fall.ogg")),
            "Ogg first"
        );
        assert_eq!(
            in_theme(&t, "Music_Theme_1_1"),
            Some(t.join("Music/piece.ogg"))
        );
        assert_eq!(in_theme(&t, "Bad"), None);
        assert_eq!(in_theme(&t, "../t/Sounds/Misc_Fall"), None);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc/hostname", t.join("Sounds/Linked.wav")).unwrap();
            assert_eq!(in_theme(&t, "Linked"), None);
        }
        std::fs::remove_dir_all(&r).unwrap();
    }

    #[test]
    fn the_shipped_themes_hold_what_the_game_asks_for() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sounds");
        let t = themes_in(&assets, true);
        let keys: Vec<&str> = t.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys, ["neuro", "system"]);
        let system = assets.join("system");
        for name in [
            "Misc_StartLevel",
            "Misc_Checkpoint",
            "Roll_Wood_Wood",
            "Music_Theme_1_1",
            "Music_Final",
        ] {
            assert!(in_theme(&system, name).is_some(), "system {name}");
        }
        assert!(in_theme(&assets.join("neuro"), "Misc_Fall").is_some());
        // the generated theme has its own level music, every part of the five themes, and its own end music
        for name in ["Music_EndCheckpoint", "Music_Final", "Music_LastFinal"] {
            assert!(
                in_theme(&assets.join("neuro"), name).is_some(),
                "neuro {name}"
            );
        }
        for n in 1..=5 {
            for part in 1..=3 {
                let name = format!("Music_Theme_{n}_{part}");
                assert!(
                    in_theme(&assets.join("neuro"), &name).is_some(),
                    "neuro {name}"
                );
            }
        }
        assert!(t.iter().all(|t| !t.names.is_empty()));
        // the generated theme plays unless the player chose another; a choice not found falls back to it
        assert_eq!(playing(&t, None), DEFAULT);
        assert_eq!(playing(&t, Some("system")), "system");
        assert_eq!(playing(&t, Some("gone")), DEFAULT);
        assert_eq!(playing(&t, Some(ORIGINAL)), ORIGINAL);
        let only_system: Vec<Theme> = t.iter().filter(|t| t.key == SYSTEM).cloned().collect();
        assert_eq!(playing(&only_system, None), SYSTEM);
    }
}
