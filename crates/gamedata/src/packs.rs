// Texture packs and model packs (docs/drahtseilakt/packs.md): what the levels, their parts and the balls are drawn
// with, chosen in the extended mode's Textures and Models lines, as sound themes are (sound_themes.rs is the model).
//
// - A texture pack is a folder of pictures named by the textures' names (the copy's texture names, compared as
//   resources::texture_key compares them). Built in: `own` (assets/own/Textures and Track, our free textures),
//   `original` (the player's copy), `remastered` (a remastered level's own upscaled files, else the copy's). The
//   player's packs are the data folder's `textures/<pack>/`.
// - A model pack changes only the look of the parts and balls (the mesh drawn); the physics always uses the measured
//   shapes the level takes now (assets.rs, Data::part), so replays and play never change. Built in: `level` (the
//   level's own choice: no other look), `own` (assets/own/3D Entities), `original` (the copy's parts). The player's
//   packs are the data folder's `models/<pack>/`, laid out as assets/own/3D Entities (PH/<module>/, Balls/,
//   AnimTrafo/, each a part in the open format).
//
// A pack has a pack.txt (the shape of a sound theme's theme.txt: `name.<lang>`, `author`, `licence`, `fallback`).
// What the chosen pack lacks comes from its fallback, then the default (textures: ours, then the level's own; models:
// the level's own). The player's folders are untrusted (they are shared): plain names only, real folders and files
// (no links), a capped number of packs, a capped pack.txt, a capped number and size of files; their pictures and parts
// are decoded and checked through the caps of assets.rs.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The most packs of a kind taken (built-in and the player's together), the largest pack.txt and the longest key.
pub const MAX_PACKS: usize = 32;
const MAX_PACK_TXT: u64 = 4096;
const MAX_KEY: usize = 64;
/// The most pictures one texture pack's folder lists (more are passed over), and the largest picture file.
pub const MAX_PICTURES: usize = 4096;
pub const MAX_PICTURE_FILE: u64 = 32 * 1024 * 1024;
/// The longest chain of fallbacks followed.
const MAX_CHAIN: usize = 8;

/// The built-in packs' keys.
pub const OWN: &str = "own";
pub const ORIGINAL: &str = "original";
pub const REMASTERED: &str = "remastered";
/// The models' default: every level drawn with the parts it takes (no other look).
pub const LEVEL: &str = "level";
/// `fallback none`: nothing after this pack but the default's end (the level's own textures, or parts).
pub const NO_FALLBACK: &str = "none";

/// Which kind of pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Textures,
    Models,
}

impl Kind {
    /// The settings' key and the player's folder in the data folder.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Textures => "textures",
            Kind::Models => "models",
        }
    }

    /// The built-in packs, in the menu's order.
    pub fn builtin(self) -> &'static [&'static str] {
        match self {
            Kind::Textures => &[OWN, ORIGINAL, REMASTERED],
            Kind::Models => &[LEVEL, OWN, ORIGINAL],
        }
    }

    /// What is chosen when nothing is: for the game ours (textures, decisions.md "Own textures by default") and the
    /// level's own parts (models); `start` sets it. A program that only uses this library (the toolbox) reads the
    /// copy's textures, as before packs.
    fn library_default(self) -> &'static str {
        match self {
            Kind::Textures => ORIGINAL,
            Kind::Models => LEVEL,
        }
    }

    fn slot(self) -> usize {
        match self {
            Kind::Textures => 0,
            Kind::Models => 1,
        }
    }
}

/// Whether the player's copy is there. The `original` packs are its files, so without it they are not offered (the
/// default is that there is one: a program that only uses this library, the toolbox, works on a copy).
static HAVE_COPY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Tells the packs whether the player's copy exists (before `start`).
pub fn set_copy(have: bool) {
    HAVE_COPY.store(have, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the remastered export holds a level (levels/remastered/<Level>/level.gltf): the `remastered` textures are
/// its files, so there is nothing to choose without one.
pub fn remaster_exists() -> bool {
    let Ok(read) = std::fs::read_dir(crate::profile::dir(crate::profile::Kind::Remastered)) else {
        return false;
    };
    read.flatten()
        .take(64)
        .any(|e| e.path().join("level.gltf").is_file())
}

/// Whether a built-in pack can be used here: the copy's needs the copy, the remaster's needs a remaster (a pack with
/// neither would draw every part untextured).
fn builtin_usable(kind: Kind, key: &str) -> bool {
    match key {
        ORIGINAL => HAVE_COPY.load(std::sync::atomic::Ordering::Relaxed),
        REMASTERED if kind == Kind::Textures => remaster_exists(),
        _ => true,
    }
}

/// A pack found.
#[derive(Clone, Debug, PartialEq)]
pub struct Pack {
    /// Its folder's name (a built-in pack's key), which the settings name.
    pub key: String,
    /// The player's pack's folder; none for a built-in one.
    pub dir: Option<PathBuf>,
    /// Its name per language (`name.<lang>` lines).
    pub names: Vec<(String, String)>,
    /// What it lacks comes from this pack (`none`: from no other pack).
    pub fallback: Option<String>,
}

impl Pack {
    fn builtin(kind: Kind, key: &str) -> Pack {
        // the remaster's pictures are its own level's: everything else is the copy's
        let fallback = (kind == Kind::Textures && key == REMASTERED).then(|| ORIGINAL.to_owned());
        Pack {
            key: key.to_owned(),
            dir: None,
            names: Vec::new(),
            fallback,
        }
    }

    /// Its name in `lang` (en, ru, de, zh), else in English, else its key.
    pub fn name(&self, lang: &str) -> String {
        let find = |l: &str| {
            self.names
                .iter()
                .find(|(k, _)| k == l)
                .map(|(_, v)| v.clone())
        };
        find(lang).or_else(|| find("en")).unwrap_or_else(|| {
            crate::package::clean_noted(&self.key, crate::sound_themes::NAME_SHOWN).0
        })
    }

    /// One of the game's own packs (not a folder of the player's).
    pub fn is_builtin(&self) -> bool {
        self.dir.is_none()
    }
}

/// A name a pack folder or a file in one may have: letters, digits, `_`, `-` and `.`, not beginning with a dot (the
/// sound themes' rule).
pub fn plain(name: &str) -> bool {
    crate::sound_themes::plain(name)
}

fn real_dir(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir())
}

fn real_file(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file())
}

/// The player's packs of a kind: the data folder's `textures/` or `models/`.
pub fn user_root(kind: Kind) -> PathBuf {
    crate::profile::data_dir().join(kind.key())
}

/// A pack.txt's names and fallback from its text: the shape of a sound theme's theme.txt (the fuzz targets' entry).
pub fn parse_pack_txt(text: &str) -> (Vec<(String, String)>, Option<String>) {
    crate::sound_themes::parse_theme_txt(text)
}

/// A pack.txt read: nothing from a file that is not a small plain one.
fn read_pack_txt(path: &Path) -> (Vec<(String, String)>, Option<String>) {
    use std::io::Read;
    let mut text = String::new();
    if real_file(path)
        && let Ok(f) = std::fs::File::open(path)
    {
        let _ = f.take(MAX_PACK_TXT).read_to_string(&mut text);
    }
    parse_pack_txt(&text)
}

/// The packs in `root`: its plain-named real folders, by name, at most MAX_PACKS (a folder named like a built-in pack
/// is left out: the built-in one is that name).
pub fn packs_in(kind: Kind, root: &Path) -> Vec<Pack> {
    let Ok(d) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = d
        .flatten()
        .take(1024)
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|k| plain(k) && k.len() <= MAX_KEY && real_dir(&root.join(k)))
        .filter(|k| !kind.builtin().contains(&k.as_str()) && k != NO_FALLBACK)
        .collect();
    keys.sort();
    keys.into_iter()
        .take(MAX_PACKS)
        .map(|key| {
            let dir = root.join(&key);
            let (names, fallback) = read_pack_txt(&dir.join("pack.txt"));
            Pack {
                key,
                dir: Some(dir),
                names,
                fallback,
            }
        })
        .collect()
}

/// Every pack of a kind: the built-in ones, then the player's.
pub fn scan(kind: Kind) -> Vec<Pack> {
    let mut all: Vec<Pack> = kind
        .builtin()
        .iter()
        .filter(|k| builtin_usable(kind, k))
        .map(|k| Pack::builtin(kind, k))
        .collect();
    for p in packs_in(kind, &user_root(kind)) {
        if all.len() < MAX_PACKS {
            all.push(p);
        }
    }
    all
}

/// The packs as the game found them at its start, the one chosen, and what the player chose.
struct State {
    packs: Vec<Pack>,
    chosen: String,
}

static STATE: Mutex<[Option<State>; 2]> = Mutex::new([None, None]);

/// Finds the packs of a kind and chooses `wanted` (the setting's or the command line's) when it is there, else
/// `default` (a player's pack moved away for a while plays the default, and the setting keeps its name).
pub fn start(kind: Kind, wanted: Option<&str>, default: &str) {
    let packs = scan(kind);
    let found = |k: &str| packs.iter().any(|p| p.key == k);
    if let Some(w) = wanted
        && !found(w)
        && kind.builtin().contains(&w)
    {
        eprintln!(
            "{} pack {w}: not available here (it needs the player's copy or a remaster), using {default}",
            kind.key()
        );
    }
    let chosen = match wanted {
        Some(w) if found(w) => w.to_owned(),
        _ if found(default) => default.to_owned(),
        _ => kind.library_default().to_owned(),
    };
    if let Ok(mut s) = STATE.lock()
        && let Some(slot) = s.get_mut(kind.slot())
    {
        *slot = Some(State { packs, chosen });
    }
}

/// The packs found at the start (the built-in ones before it).
pub fn packs(kind: Kind) -> Vec<Pack> {
    STATE
        .lock()
        .ok()
        .and_then(|s| {
            s.get(kind.slot())
                .and_then(|s| s.as_ref().map(|s| s.packs.clone()))
        })
        .unwrap_or_else(|| {
            kind.builtin()
                .iter()
                .filter(|k| builtin_usable(kind, k))
                .map(|k| Pack::builtin(kind, k))
                .collect()
        })
}

/// The pack chosen (its key).
pub fn chosen(kind: Kind) -> String {
    STATE
        .lock()
        .ok()
        .and_then(|s| {
            s.get(kind.slot())
                .and_then(|s| s.as_ref().map(|s| s.chosen.clone()))
        })
        .unwrap_or_else(|| kind.library_default().to_owned())
}

/// Chooses another pack (it applies from the next level loaded); a key that is not among the packs found is ignored.
pub fn choose(kind: Kind, key: &str) {
    if let Ok(mut s) = STATE.lock()
        && let Some(Some(s)) = s.get_mut(kind.slot())
        && s.packs.iter().any(|p| p.key == key)
    {
        s.chosen = key.to_owned();
    }
}

/// The pack `by` steps after the chosen one (the menu's arrows go round them).
pub fn next(kind: Kind, by: i32) -> String {
    let all = packs(kind);
    let now = chosen(kind);
    let at = all.iter().position(|p| p.key == now).unwrap_or(0);
    let n = i64::try_from(all.len().max(1)).unwrap_or(1);
    let at = i64::try_from(at).unwrap_or(0);
    let i = usize::try_from(at.saturating_add(i64::from(by)).rem_euclid(n)).unwrap_or(0);
    all.get(i)
        .map_or_else(|| kind.library_default().to_owned(), |p| p.key.clone())
}

/// The packs asked in order, from `packs` and the chosen key: the chosen one, then each fallback (a player's pack
/// without one falls back to the default, `own` textures or the level's own models; `none` and the built-in packs end
/// the chain), each once, at most MAX_CHAIN. A fallback naming no pack found ends the chain there.
pub fn chain_of(kind: Kind, packs: &[Pack], first: &str) -> Vec<Pack> {
    let mut out: Vec<Pack> = Vec::new();
    let mut key = Some(first.to_owned());
    while let Some(k) = key.take() {
        if out.len() >= MAX_CHAIN || out.iter().any(|p| p.key == k) {
            break;
        }
        let Some(p) = packs.iter().find(|p| p.key == k) else {
            break;
        };
        key = match p.fallback.as_deref() {
            Some(NO_FALLBACK) => None,
            Some(f) => Some(f.to_owned()),
            None if p.is_builtin() => None,
            None => Some(match kind {
                Kind::Textures => OWN.to_owned(),
                Kind::Models => LEVEL.to_owned(),
            }),
        };
        out.push(p.clone());
    }
    out
}

/// The chosen pack's chain.
pub fn chain(kind: Kind) -> Vec<Pack> {
    chain_of(kind, &packs(kind), &chosen(kind))
}

/// The pictures a texture pack's folder has, by their texture's key: its plain image files (PNG, JPEG, BMP, TGA, each a
/// real file of at most MAX_PICTURE_FILE), at its top and in `Sky/` and `Particles/` (real folders), at most
/// MAX_PICTURES. The first in name order wins a key two files share.
pub fn picture_index(dir: &Path) -> std::collections::HashMap<String, PathBuf> {
    let mut index = std::collections::HashMap::new();
    if !real_dir(dir) {
        return index;
    }
    let mut taken = 0usize;
    for sub in ["", "Sky", "Particles"] {
        let d = if sub.is_empty() {
            dir.to_path_buf()
        } else {
            dir.join(sub)
        };
        if !real_dir(&d) {
            continue;
        }
        let Ok(listed) = std::fs::read_dir(&d) else {
            continue;
        };
        let mut names: Vec<String> = listed
            .flatten()
            .take(MAX_PICTURES)
            .filter_map(|e| e.file_name().to_str().map(str::to_owned))
            .filter(|n| plain(n))
            .collect();
        names.sort();
        for name in names {
            if taken >= MAX_PICTURES {
                return index;
            }
            let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
            if !matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg" | "bmp" | "tga")) {
                continue;
            }
            let path = d.join(&name);
            let small = std::fs::symlink_metadata(&path)
                .is_ok_and(|m| m.is_file() && m.len() <= MAX_PICTURE_FILE);
            let key = crate::resources::texture_key(&name);
            if small && !key.is_empty() {
                taken = taken.saturating_add(1);
                index.entry(key).or_insert(path);
            }
        }
    }
    index
}

/// A texture pack's picture for the texture `name` (a texture's name or its file's), indexed once per folder.
pub fn picture(dir: &Path, name: &str) -> Option<PathBuf> {
    type Indexes = std::collections::HashMap<PathBuf, std::collections::HashMap<String, PathBuf>>;
    static INDEXES: Mutex<Option<Indexes>> = Mutex::new(None);
    let key = crate::resources::texture_key(name);
    let mut all = INDEXES.lock().ok()?;
    let all = all.get_or_insert_with(Default::default);
    all.entry(dir.to_path_buf())
        .or_insert_with(|| picture_index(dir))
        .get(&key)
        .cloned()
}

/// The settings' choice of a kind, if the player made one.
pub fn from_settings(kind: Kind) -> Option<String> {
    crate::profile::setting(kind.key()).filter(|k| plain(k) && k.len() <= MAX_KEY)
}

/// Whether a path is inside one of the player's pack folders (their pictures share the level's pixel budget).
pub fn in_player_packs(path: &Path) -> bool {
    [Kind::Textures, Kind::Models]
        .iter()
        .any(|k| path.starts_with(user_root(*k)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn pack(key: &str, fallback: Option<&str>) -> Pack {
        Pack {
            key: key.into(),
            dir: Some(PathBuf::from(key)),
            names: Vec::new(),
            fallback: fallback.map(str::to_owned),
        }
    }

    #[test]
    fn a_pack_txt_reads_as_a_theme_txt() {
        let (names, fallback) = parse_pack_txt(
            "\u{feff}# Stone: rough stone everywhere\nname.en Stone\nname.ru Камень\nname.xx Nope\nauthor Jane\n\
             licence CC0 1.0\nfallback original\n",
        );
        assert_eq!(
            names,
            vec![
                ("en".to_owned(), "Stone".to_owned()),
                ("ru".to_owned(), "Камень".to_owned())
            ]
        );
        assert_eq!(fallback.as_deref(), Some("original"));
        // a fallback that is not a plain name is not taken; control characters are dropped from names
        let (names, fallback) = parse_pack_txt("name.de A\u{7}B\nfallback ../x\n");
        assert_eq!(names, vec![("de".to_owned(), "AB".to_owned())]);
        assert_eq!(fallback, None);
        let long = format!("name.en {}\n", "x".repeat(500));
        assert_eq!(parse_pack_txt(&long).0[0].1.chars().count(), 16);
        // a longer name is cut at a word, ending in the ellipsis; a name of 16 letters stays whole
        let (names, _) = parse_pack_txt("name.en Wonderful big pack of things\nname.ru Ш\n");
        assert_eq!(names[0].1, "Wonderful big…");
        let (names, _) = parse_pack_txt(&format!("name.de {}\n", "W".repeat(16)));
        assert_eq!(names[0].1, "W".repeat(16));
        let (cut_text, cut) = crate::package::clean_noted(&"汉".repeat(20), 16);
        assert!(cut && cut_text.chars().count() <= 16);
        assert!(!crate::package::clean_noted(&"汉".repeat(16), 16).1);
        assert!(crate::package::clean_noted(&"x y ".repeat(30), 40).1);
    }

    #[test]
    fn the_chain_follows_fallbacks_once_and_ends() {
        let t = Kind::Textures;
        let mut all: Vec<Pack> = t.builtin().iter().map(|k| Pack::builtin(t, k)).collect();
        all.push(pack("a", Some("b")));
        all.push(pack("b", None));
        all.push(pack("c", Some("none")));
        all.push(pack("loop1", Some("loop2")));
        all.push(pack("loop2", Some("loop1")));
        all.push(pack("lost", Some("nowhere")));
        let keys = |first: &str| {
            chain_of(t, &all, first)
                .into_iter()
                .map(|p| p.key)
                .collect::<Vec<_>>()
        };
        assert_eq!(keys("a"), ["a", "b", "own"]);
        assert_eq!(keys("c"), ["c"]);
        assert_eq!(keys("loop1"), ["loop1", "loop2"]);
        assert_eq!(keys("lost"), ["lost"]);
        assert_eq!(keys("own"), ["own"]);
        assert_eq!(keys("remastered"), ["remastered", "original"]);
        assert_eq!(keys("unknown"), Vec::<String>::new());
        let m = Kind::Models;
        let mut models: Vec<Pack> = m.builtin().iter().map(|k| Pack::builtin(m, k)).collect();
        models.push(pack("round", None));
        let keys = chain_of(m, &models, "round")
            .into_iter()
            .map(|p| p.key)
            .collect::<Vec<_>>();
        assert_eq!(keys, ["round", "level"]);
    }

    #[test]
    fn hostile_pack_folders_are_passed_over() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-packs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("good/Sky")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::create_dir_all(root.join("own")).unwrap();
        std::fs::create_dir_all(root.join("with space")).unwrap();
        std::fs::write(root.join("afile"), b"x").unwrap();
        std::fs::write(root.join("good/pack.txt"), "name.en Good\nfallback none\n").unwrap();
        std::fs::write(root.join("good/Floor_Side.png"), b"x").unwrap();
        std::fs::write(root.join("good/Sky/Sky_A_Front.jpg"), b"x").unwrap();
        std::fs::write(root.join("good/notes.txt"), b"x").unwrap();
        std::fs::write(root.join("good/.Wood_Old.png"), b"x").unwrap();
        std::fs::create_dir_all(root.join("good/Brick.png")).unwrap();
        // a huge pack.txt is read only to its cap
        std::fs::create_dir_all(root.join("big")).unwrap();
        std::fs::write(
            root.join("big/pack.txt"),
            format!("{}\nname.en Late\n", "#".repeat(10_000)),
        )
        .unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("good"), root.join("linked")).unwrap();
            std::os::unix::fs::symlink(
                root.join("good/Floor_Side.png"),
                root.join("good/Metal.png"),
            )
            .unwrap();
            std::os::unix::fs::symlink("/", root.join("good/Particles")).unwrap();
        }
        let found = packs_in(Kind::Textures, &root);
        let keys: Vec<&str> = found.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(
            keys,
            ["big", "good"],
            "no hidden, built-in, spaced, file or linked folder"
        );
        assert_eq!(found[0].names, Vec::<(String, String)>::new());
        assert_eq!(found[1].name("ru"), "Good");
        assert_eq!(found[1].fallback.as_deref(), Some("none"));
        let index = picture_index(&root.join("good"));
        assert!(index.contains_key("floorside") && index.contains_key("skyafront"));
        for k in ["notes", "woodold", "brick", "metal"] {
            assert!(!index.contains_key(k), "{k}");
        }
        assert_eq!(index.len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn many_pack_folders_are_capped() {
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-packs-many-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for i in 0..50 {
            std::fs::create_dir_all(root.join(format!("p{i:02}"))).unwrap();
        }
        assert_eq!(packs_in(Kind::Models, &root).len(), MAX_PACKS);
        let _ = std::fs::remove_dir_all(&root);
    }
}
