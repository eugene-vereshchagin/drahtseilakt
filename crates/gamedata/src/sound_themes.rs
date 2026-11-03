// Sound themes (docs/drahtseilakt/sound-themes.md): a theme is a folder of sounds and music by the names the game asks
// for (Sounds/<name>.ogg|.wav, Music/<name>.ogg|.wav), and tables naming more files per name (Music/music.txt,
// Sounds/sounds.txt): every name, a *role*, has a pool of files the game picks one from each time it plays it. The game
// ships two in its assets (sounds/system: openly licensed recordings; sounds/neuro: our synthesized ones); the player
// may add more in the data folder's sounds/ — among them "original", which the toolbox makes from the player's copy
// and which is never shipped. The generated theme (neuro) plays unless the player chose another (owner's decision
// 2026-11-02); the extended mode chooses (`sound-theme NAME`), the classic mode plays the default. A sound the chosen
// theme lacks comes from its fallback theme (theme.txt `fallback NAME`, the system theme when none; `fallback none`: no
// other theme, as the complete generated theme has), then — for a level's own sounds, never for music — from the copy.
//
// The player's theme folders are untrusted (they are shared): only plain folder and file names, real folders and
// files (no links), a capped number of themes, a capped theme.txt and capped tables (size, lines, files per role; a
// table over a cap or not UTF-8 is refused whole, a line naming anything but plain .ogg/.wav files of its own folder is
// passed over, a role the game does not know is ignored with a note); the sounds themselves are decoded through the
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
    parse_theme_txt(&text)
}

/// A theme.txt's names and fallback from its text (the fuzz targets' entry; `read_theme_txt` reads the file).
pub fn parse_theme_txt(text: &str) -> (Vec<(String, String)>, Option<String>) {
    let mut names = Vec::new();
    let mut fallback = None;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
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

/// The `fallback` of a theme complete on its own: no other theme's sounds are taken.
pub const NO_FALLBACK: &str = "none";

/// The folders of the themes a sound is looked for in, in order: the chosen theme's, its fallback's, the system
/// theme's; only the chosen theme's when its fallback is "none".
fn chain() -> Vec<PathBuf> {
    let themes = themes();
    let first = chosen();
    let mut keys = vec![first.clone()];
    let fallback = themes
        .iter()
        .find(|t| t.key == first)
        .and_then(|t| t.fallback.clone());
    match fallback.as_deref() {
        Some(NO_FALLBACK) => {}
        Some(f) => {
            keys.push(f.to_owned());
            keys.push(SYSTEM.to_owned());
        }
        None => keys.push(SYSTEM.to_owned()),
    }
    keys.dedup();
    keys.iter()
        .filter_map(|k| themes.iter().find(|t| &t.key == k))
        .map(|t| t.dir.clone())
        .collect()
}

/// The files of the sound `name`, in the order they are tried: the first of its pool in each theme of `chain` (a broken
/// one is passed over for the next).
pub fn files(name: &str) -> Vec<PathBuf> {
    if !plain(name) {
        return Vec::new();
    }
    chain().iter().filter_map(|d| in_theme(d, name)).collect()
}

/// A sound's first file in one theme folder: the file named like it (Sounds/ or Music/, Ogg first, then WAV), else the
/// first its tables name.
pub fn in_theme(dir: &Path, name: &str) -> Option<PathBuf> {
    pool_in_theme(dir, name).into_iter().next()
}

/// The most files a role's pool holds (more are ignored).
pub const MAX_POOL: usize = 16;

/// Every file one theme folder has for the role `name`, in order: the file named like it (Sounds/ or Music/, Ogg first,
/// then WAV), then those Sounds/sounds.txt and Music/music.txt name for it, each once, at most MAX_POOL.
pub fn pool_in_theme(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut pool: Vec<PathBuf> = Vec::new();
    if !plain(name) || !real_dir(dir) {
        return pool;
    }
    let named = ["Sounds", "Music"].into_iter().find_map(|folder| {
        let d = dir.join(folder);
        if !real_dir(&d) {
            return None;
        }
        ["ogg", "wav"]
            .into_iter()
            .map(|ext| d.join(format!("{name}.{ext}")))
            .find(|p| real_file(p))
    });
    pool.extend(named);
    for (folder, table) in [("Sounds", "sounds.txt"), ("Music", "music.txt")] {
        let d = dir.join(folder);
        for (role, files) in cached_table(&d, table) {
            if !role.eq_ignore_ascii_case(name) {
                continue;
            }
            for f in files {
                let p = d.join(f);
                if pool.len() < MAX_POOL && !pool.contains(&p) && real_file(&p) {
                    pool.push(p);
                }
            }
        }
    }
    pool
}

/// The pool of the role `name` the game picks from: the first theme's in the order `files` tries them that has the role
/// at all (a theme's pool is never mixed with its fallback's).
pub fn pool(name: &str) -> Vec<PathBuf> {
    chain()
        .iter()
        .map(|dir| pool_in_theme(dir, name))
        .find(|p| !p.is_empty())
        .unwrap_or_default()
}

/// The largest table read, and the most lines and roles in one.
const MAX_TABLE: usize = 16 * 1024;
const MAX_TABLE_LINES: usize = 512;
const MAX_ROLES: usize = 256;

/// The music roles beyond the parts a level plays, and the sounds the game plays or the original has (a table may
/// name them; any other name is ignored with a note).
pub const KNOWN_SOUNDS: &[&str] = &[
    "Misc_StartLevel",
    "Misc_Checkpoint",
    "Misc_Fall",
    "Misc_Lightning",
    "Misc_Trafo",
    "Misc_extraball",
    "Misc_RopeTears",
    "Misc_Ventilator",
    "Misc_UFO",
    "Misc_UFO_anim",
    "Extra_Life_Blob",
    "Extra_Start",
    "Extra_Hit",
    "Menu_click",
    "Menu_load",
    "Menu_atmo",
    "Menu_dong",
    "Menu_counter",
    "Music_thunder",
    "Music_EndCheckpoint",
    "Music_Final",
    "Music_LastFinal",
    "Music_Highscore",
    "Music_Atmo",
    "Roll_Paper",
    "Roll_Wood_Wood",
    "Roll_Wood_Stone",
    "Roll_Wood_Metal",
    "Roll_Stone_Wood",
    "Roll_Stone_Stone",
    "Roll_Stone_Metal",
    "Hit_Paper",
    "Hit_Wood_Wood",
    "Hit_Wood_Stone",
    "Hit_Wood_Metal",
    "Hit_Wood_Dome",
    "Hit_WoodenFlap",
    "Hit_Stone_Wood",
    "Hit_Stone_Stone",
    "Hit_Stone_Metal",
    "Hit_Stone_Kuppel",
    "Pieces_Paper",
    "Pieces_Wood",
    "Pieces_Stone",
];

/// A role the game knows: a sound of KNOWN_SOUNDS, an atmosphere part (Music_Atmo_1..3), a level theme
/// (Music_Theme_<n>, any of its parts) or one of its parts (Music_Theme_<n>_<1..3>), n from 1 to 99.
pub fn known_role(name: &str) -> bool {
    if KNOWN_SOUNDS.iter().any(|k| k.eq_ignore_ascii_case(name)) {
        return true;
    }
    // a number from 1 to `max`, without leading zeros
    let number = |s: &str, max: u32| {
        s.len() <= 2
            && !s.starts_with('0')
            && s.parse::<u32>().is_ok_and(|n| (1..=max).contains(&n))
    };
    let lower = name.to_ascii_lowercase();
    if let Some(part) = lower.strip_prefix("music_atmo_") {
        return number(part, 3);
    }
    let Some(rest) = lower.strip_prefix("music_theme_") else {
        return false;
    };
    match rest.split_once('_') {
        Some((n, part)) => number(n, 99) && number(part, 3),
        None => number(rest, 99),
    }
}

/// A parsed table: each role with its files, in the order given (a role on several lines gathers them), and notes on
/// what was passed over.
#[derive(Debug, Default, PartialEq)]
pub struct Table {
    pub roles: Vec<(String, Vec<String>)>,
    pub notes: Vec<String>,
}

/// Parses a table (`<role> <file> [<file> ...]` lines, `#` comments; the old one-file lines are the same format).
/// Fails closed: a table too large, with too many lines or not UTF-8 gives no roles at all; a line with a name that is
/// not plain, or a file that is not a plain .ogg/.wav name, is passed over whole; an unknown role is ignored.
pub fn parse_table(bytes: &[u8]) -> Table {
    let mut t = Table::default();
    if bytes.len() > MAX_TABLE {
        t.notes
            .push(format!("larger than {MAX_TABLE} bytes: not read"));
        return t;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        t.notes.push("not UTF-8: not read".to_owned());
        return t;
    };
    // a byte order mark, as some editors write at the start, is not part of the first role's name
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.lines().count() > MAX_TABLE_LINES {
        t.notes
            .push(format!("more than {MAX_TABLE_LINES} lines: not read"));
        return t;
    }
    let sound_file = |f: &str| {
        let lower = f.to_ascii_lowercase();
        plain(f) && (lower.ends_with(".ogg") || lower.ends_with(".wav")) && f.len() > 4
    };
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut words = line.split_whitespace();
        let Some(role) = words.next() else { continue };
        let files: Vec<&str> = words.collect();
        let n = i.saturating_add(1);
        if !plain(role) || role.len() > MAX_NAME {
            t.notes.push(format!("line {n}: not a plain role name"));
        } else if files.is_empty() {
            t.notes.push(format!("line {n}: {role} names no file"));
        } else if let Some(bad) = files.iter().find(|f| !sound_file(f)) {
            let bad: String = bad
                .chars()
                .filter(|c| !c.is_control())
                .take(MAX_NAME)
                .collect();
            t.notes.push(format!(
                "line {n}: {bad:?} is not a plain .ogg or .wav file name"
            ));
        } else if !known_role(role) {
            t.notes.push(format!(
                "line {n}: {role} is not a role the game plays: ignored"
            ));
        } else if let Some((_, pool)) = t
            .roles
            .iter_mut()
            .find(|(r, _)| r.eq_ignore_ascii_case(role))
        {
            for f in files {
                if pool.len() < MAX_POOL && !pool.iter().any(|p| p == f) {
                    pool.push(f.to_owned());
                }
            }
        } else if t.roles.len() < MAX_ROLES {
            let mut pool: Vec<String> = Vec::new();
            for f in files {
                if pool.len() < MAX_POOL && !pool.iter().any(|p| p == f) {
                    pool.push(f.to_owned());
                }
            }
            t.roles.push((role.to_owned(), pool));
        } else {
            t.notes.push(format!(
                "line {n}: more than {MAX_ROLES} roles: {role} ignored"
            ));
        }
    }
    t
}

/// A folder's table (`file` in it), read once per folder; its notes are printed when it is first read.
fn cached_table(folder: &Path, file: &str) -> Vec<(String, Vec<String>)> {
    use std::io::Read;
    type Cache = Vec<(PathBuf, Vec<(String, Vec<String>)>)>;
    static TABLES: Mutex<Cache> = Mutex::new(Vec::new());
    let path = folder.join(file);
    if let Ok(t) = TABLES.lock()
        && let Some((_, table)) = t.iter().find(|(p, _)| *p == path)
    {
        return table.clone();
    }
    let mut bytes = Vec::new();
    if real_dir(folder)
        && real_file(&path)
        && let Ok(f) = std::fs::File::open(&path)
    {
        // one byte more than the cap: a larger table is seen as such and refused
        let _ = f
            .take((MAX_TABLE as u64).saturating_add(1))
            .read_to_end(&mut bytes);
    }
    let table = if bytes.is_empty() {
        Table::default()
    } else {
        parse_table(&bytes)
    };
    for note in &table.notes {
        eprintln!("sound theme table {}: {note}", path.display());
    }
    if let Ok(mut t) = TABLES.lock()
        && t.len() < 2 * MAX_THEMES
    {
        t.push((path, table.roles.clone()));
    }
    table.roles
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

    #[test]
    fn a_table_gives_each_role_a_pool_and_keeps_the_old_format() {
        let t = parse_table(
            b"# a comment\nMusic_Theme_1_1 a.ogg\r\nMusic_Atmo_1\ta.ogg  b.WAV a.ogg\n\nmusic_atmo_1 c.ogg\n\
              Music_Atmo d.ogg\nMusic_Theme_4 e.ogg\nHit_Wood_Wood h1.ogg h2.ogg\n",
        );
        assert!(t.notes.is_empty(), "{:?}", t.notes);
        let pool = |r: &str| {
            t.roles
                .iter()
                .find(|(k, _)| k == r)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        // the old one-file line, a pool on one line and on several (a role's case does not matter), the groups
        assert_eq!(pool("Music_Theme_1_1"), ["a.ogg"]);
        assert_eq!(pool("Music_Atmo_1"), ["a.ogg", "b.WAV", "c.ogg"]);
        assert_eq!(pool("Music_Atmo"), ["d.ogg"]);
        assert_eq!(pool("Music_Theme_4"), ["e.ogg"]);
        assert_eq!(pool("Hit_Wood_Wood"), ["h1.ogg", "h2.ogg"]);
        // a byte order mark at the start is no part of the first role
        let bom = parse_table("\u{feff}Music_Final f.ogg\n".as_bytes());
        assert!(bom.notes.is_empty(), "{:?}", bom.notes);
        assert_eq!(
            bom.roles,
            [("Music_Final".to_owned(), vec!["f.ogg".to_owned()])]
        );
        assert_eq!(t.roles.len(), 5);
    }

    #[test]
    fn a_hostile_table_is_refused_or_passed_over() {
        // lines naming anything but plain .ogg/.wav files of the folder are passed over whole, unknown roles ignored
        let t = parse_table(
            b"Music_Final ../x.ogg\nMusic_Final /etc/passwd.ogg\nMusic_Final a\\b.ogg\nMusic_Final .hidden.ogg\n\
              Music_Final ok.ogg evil.exe\nMusic_Final C:x.ogg\nMusic_Final .ogg\nMusic_Final\n../role ok.ogg\n\
              Music_Theme_1_4 ok.ogg\nMusic_Theme_0 ok.ogg\nMusic_Atmo_01 ok.ogg\nMy_Sound ok.ogg\n\
              Music_Final \x1b[31m.ogg\nMusic_LastFinal fine.ogg\n",
        );
        assert_eq!(
            t.roles,
            [("Music_LastFinal".to_owned(), vec!["fine.ogg".to_owned()])]
        );
        assert_eq!(t.notes.len(), 14, "{:#?}", t.notes);
        assert!(
            t.notes.iter().all(|n| !n.contains('\x1b')),
            "no control characters echoed"
        );
        // a pool is capped
        let many: String = (0..40).map(|i| format!("Hit_Paper p{i}.ogg\n")).collect();
        assert_eq!(parse_table(many.as_bytes()).roles[0].1.len(), MAX_POOL);
        // too large, too many lines, not UTF-8: nothing at all
        let large = format!("Music_Final a.ogg\n{}", "#".repeat(MAX_TABLE));
        assert!(parse_table(large.as_bytes()).roles.is_empty());
        let long = "Music_Final a.ogg\n".to_owned() + &"\n".repeat(MAX_TABLE_LINES);
        assert!(parse_table(long.as_bytes()).roles.is_empty());
        assert!(parse_table(b"Music_Final a.ogg\n\xff\xfe").roles.is_empty());
        assert_eq!(parse_table(b""), Table::default());
        // roles past the cap are ignored with a note: the theme's known roles are fewer, so they repeat in case
        let roles: String = (1..=99)
            .flat_map(|n| (1..=3).map(move |k| format!("Music_Theme_{n}_{k} a.ogg\n")))
            .collect();
        let t = parse_table(roles.as_bytes());
        assert_eq!(t.roles.len(), MAX_ROLES);
        assert_eq!(t.notes.len(), 297 - 256, "{:?}", t.notes.first());
        assert!(t.notes.iter().all(|n| n.contains("more than 256 roles")));
    }

    #[test]
    fn a_theme_folder_gives_a_pool_of_real_files() {
        let r = root("pool");
        let t = r.join("t");
        std::fs::create_dir_all(t.join("Sounds")).unwrap();
        std::fs::create_dir_all(t.join("Music")).unwrap();
        for f in [
            "Sounds/Hit_Paper.wav",
            "Sounds/paper2.ogg",
            "Music/a1.ogg",
            "Music/a2.ogg",
        ] {
            std::fs::write(t.join(f), b"x").unwrap();
        }
        std::fs::write(
            t.join("Sounds/sounds.txt"),
            "Hit_Paper paper2.ogg missing.ogg Hit_Paper.wav\n",
        )
        .unwrap();
        std::fs::write(
            t.join("Music/music.txt"),
            "Music_Atmo_1 a1.ogg\nMusic_Atmo_1 a2.ogg linked.ogg\nMusic_Final paper2.ogg\n",
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(t.join("Music/a1.ogg"), t.join("Music/linked.ogg")).unwrap();
        // the file named like the role first, then the tables' (each once, only real files of the table's folder)
        assert_eq!(
            pool_in_theme(&t, "Hit_Paper"),
            [t.join("Sounds/Hit_Paper.wav"), t.join("Sounds/paper2.ogg")]
        );
        assert_eq!(
            pool_in_theme(&t, "Music_Atmo_1"),
            [t.join("Music/a1.ogg"), t.join("Music/a2.ogg")]
        );
        assert!(
            pool_in_theme(&t, "Music_Final").is_empty(),
            "a table names its own folder's files"
        );
        assert_eq!(in_theme(&t, "Music_Atmo_1"), Some(t.join("Music/a1.ogg")));
        assert!(
            known_role("Music_Theme_12_3")
                && known_role("music_atmo_2")
                && !known_role("Music_Atmo_4")
        );
        std::fs::remove_dir_all(&r).unwrap();
    }
}
