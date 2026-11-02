// Where the game's own resources come from (docs/drahtseilakt/resources.md; decisions.md "Own content by
// default"). The game asks for a file of the original's layout ("3D Entities/Menu.nmo", "Sounds/Misc_Fall.wav",
// "Textures/Font_1.tga"); each such file plays a role (the menu's font, the fall's sound...). Three providers can
// fill a role: our own files shipped with the game (assets/own/, laid out as the original), the player's copy, and
// the toolbox's remaster of the copy (levels/remastered/.interface/). The player chooses which comes first with
// `interface own|original|remastered` (the extended mode's Interface switch). The default, and what the classic
// mode plays, is the copy's own interface when the copy has it, else ours (owner's decision 2026-11-03, while our
// own interface is redone).
// A role no provider fills is missing: logged once, never a crash.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

/// Who fills the interface's roles first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    /// Our own files, then the copy's for what we do not have yet.
    Own,
    /// The player's copy, then our own for what it lacks.
    Original,
    /// The toolbox's remaster of the copy, then the copy, then our own.
    Remastered,
}

impl Provider {
    pub const ALL: [Provider; 3] = [Provider::Own, Provider::Original, Provider::Remastered];

    /// Its name in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Provider::Own => "own",
            Provider::Original => "original",
            Provider::Remastered => "remastered",
        }
    }

    pub fn parse(s: &str) -> Option<Provider> {
        Provider::ALL.into_iter().find(|p| p.key() == s.trim())
    }

    /// The order the providers are asked in.
    pub fn order(self) -> [Provider; 3] {
        match self {
            Provider::Own => [Provider::Own, Provider::Original, Provider::Remastered],
            // the copy, and for a role it lacks our own; never the toolbox's remaster (classic plays the original)
            Provider::Original => [Provider::Original, Provider::Own, Provider::Original],
            Provider::Remastered => [Provider::Remastered, Provider::Original, Provider::Own],
        }
    }
}

/// What the game plays with now, and what the player chose (it takes effect on the next start: the interface's
/// files are loaded once).
static IN_USE: AtomicU8 = AtomicU8::new(0);
static CHOSEN: AtomicU8 = AtomicU8::new(0);

fn index(p: Provider) -> u8 {
    match p {
        Provider::Own => 0,
        Provider::Original => 1,
        Provider::Remastered => 2,
    }
}

fn of(i: u8) -> Provider {
    match i {
        0 => Provider::Own,
        2 => Provider::Remastered,
        _ => Provider::Original,
    }
}

/// Sets what the game plays with (at its start) and what is shown as chosen.
pub fn set_in_use(p: Provider) {
    IN_USE.store(index(p), Ordering::Relaxed);
}

pub fn in_use() -> Provider {
    of(IN_USE.load(Ordering::Relaxed))
}

pub fn set_chosen(p: Provider) {
    CHOSEN.store(index(p), Ordering::Relaxed);
}

pub fn chosen() -> Provider {
    of(CHOSEN.load(Ordering::Relaxed))
}

/// The settings' choice (`interface ...`), if the player made one.
pub fn from_settings() -> Option<Provider> {
    crate::profile::setting("interface").and_then(|s| Provider::parse(&s))
}

/// The interface played when the player chose none, and always in the classic mode: the copy's own when it has its
/// menu, else ours.
pub fn default_for(copy_has_menu: bool) -> Provider {
    if copy_has_menu {
        Provider::Original
    } else {
        Provider::Own
    }
}

/// The game's own assets folder (fonts, own files): `DRAHTSEILAKT_ASSETS`, else `assets/` beside the program or
/// three folders up (a build's target/release in the source tree).
pub fn assets_root() -> Option<PathBuf> {
    static ROOT: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let real = |p: &Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir());
        let from_env = std::env::var_os("DRAHTSEILAKT_ASSETS").map(PathBuf::from);
        let exe = std::env::current_exe().ok();
        let beside = exe
            .as_deref()
            .and_then(Path::parent)
            .map(|d| d.join("assets"));
        let tree = exe
            .as_deref()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(|d| d.join("assets"));
        from_env
            .into_iter()
            .chain(beside)
            .chain(tree)
            .find(|p| real(p))
    })
    .clone()
}

/// Our own files, laid out as the original: `own/` in the assets folder.
pub fn own_root() -> Option<PathBuf> {
    assets_root().map(|a| a.join("own"))
}

/// The toolbox's remaster of the copy's interface.
pub fn remastered_root() -> PathBuf {
    // found once: it is looked in on every file the copy lacks (the settings file need not be read each time)
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| crate::profile::dir(crate::profile::Kind::Remastered).join(".interface"))
        .clone()
}

/// Which textures the levels, their parts and the balls are drawn with (`textures own|original`, the extended mode's
/// Textures switch). Our own stand in for the original's texture of the same name, whichever level asks for it (an
/// original level, a community map, a part); a texture we have none for comes from the copy as before. The default,
/// and what the classic mode plays, is ours (owner's decision 2026-11-03, decisions.md "Own textures by default").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureSet {
    Own,
    Original,
}

impl TextureSet {
    pub const ALL: [TextureSet; 2] = [TextureSet::Own, TextureSet::Original];

    /// Its name in the settings file and on the command line.
    pub fn key(self) -> &'static str {
        match self {
            TextureSet::Own => "own",
            TextureSet::Original => "original",
        }
    }

    pub fn parse(s: &str) -> Option<TextureSet> {
        TextureSet::ALL.into_iter().find(|t| t.key() == s.trim())
    }
}

/// The textures drawn now (they are read as each level loads, so a change applies to the next level). The game sets
/// them at its start (ours by default); a program that only uses this library, such as the toolbox remastering the
/// copy, reads the copy's unless it sets them.
static TEXTURES: AtomicU8 = AtomicU8::new(0);

pub fn set_textures(t: TextureSet) {
    TEXTURES.store(u8::from(t == TextureSet::Own), Ordering::Relaxed);
}

pub fn textures() -> TextureSet {
    if TEXTURES.load(Ordering::Relaxed) == 0 {
        TextureSet::Original
    } else {
        TextureSet::Own
    }
}

/// The settings' choice (`textures ...`), if the player made one.
pub fn textures_from_settings() -> Option<TextureSet> {
    crate::profile::setting("textures").and_then(|s| TextureSet::parse(&s))
}

/// How a texture's name is compared: its file name without the folder and the extension, in lower case, without
/// `_`, `-` and spaces (the balls' module names its textures "BallWood.bmp", the copy's file is "Ball_Wood.bmp").
pub fn texture_key(name: &str) -> String {
    let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = match file.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.len() <= 4 => stem,
        _ => file,
    };
    stem.chars()
        .filter(|c| !matches!(c, '_' | '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The most own textures looked at (the folder ships about sixty).
const OWN_TEXTURES_MAX: usize = 4096;

/// Our own textures by their key: the PNG files of `dir` (plain files only; a link or a folder is passed over).
pub fn texture_index(dir: &Path) -> std::collections::HashMap<String, PathBuf> {
    let mut index = std::collections::HashMap::new();
    let Ok(listed) = std::fs::read_dir(dir) else { return index };
    for entry in listed.flatten().take(OWN_TEXTURES_MAX) {
        let path = entry.path();
        let plain = std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file());
        let png = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("png"));
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let key = texture_key(name);
        if plain && png && !key.is_empty() {
            index.entry(key).or_insert(path);
        }
    }
    index
}

/// Our own texture standing in for the original's texture `name` (a texture's name or its file's), if we have one:
/// `own/Textures/<name>.png` among the game's assets. The name only picks among our files; it never makes a path.
pub fn own_texture(name: &str) -> Option<PathBuf> {
    static INDEX: std::sync::OnceLock<std::collections::HashMap<String, PathBuf>> = std::sync::OnceLock::new();
    let index = INDEX.get_or_init(|| own_root().map(|r| texture_index(&r.join("Textures"))).unwrap_or_default());
    index.get(&texture_key(name)).cloned()
}

/// The roles the interface's files play, by the original's path (a texture by its file name in Textures/, a sound
/// in Sounds/); the list resources.md describes.
pub const ROLES: &[(&str, &str)] = &[
    (
        "menu: font and its glyph table, buttons, texts",
        "3D Entities/Menu.nmo",
    ),
    ("menu: the 3D scene behind it", "3D Entities/MenuLevel.nmo"),
    ("texts in the copy's languages", "3D Entities/Language.nmo"),
    ("menu texts and the level table's defaults", "base.cmo"),
    (
        "level table: sky, start ball, music, light, bonus",
        "3D Entities/Levelinit.nmo",
    ),
    ("balls, their pieces and shadows", "3D Entities/Balls.nmo"),
    ("transformer animation", "3D Entities/AnimTrafo.nmo"),
    ("game font texture", "Textures/Font_1.tga"),
    (
        "button: special (points box, frames)",
        "Textures/Button01_special.tga",
    ),
    ("button: chosen", "Textures/Button01_select.tga"),
    ("button: not chosen", "Textures/Button01_deselect.tga"),
    ("ball shadow", "Textures/HardShadow.bmp"),
    (
        "particles: flames",
        "Textures/Particles/Particle_Flames.bmp",
    ),
    ("particles: smoke", "Textures/Particles/Particle_Smoke.bmp"),
    (
        "particles: extra point",
        "Textures/Particles/ExtraParticle.bmp",
    ),
    ("particles: extra ball", "Textures/Particles/ExtraBall.bmp"),
    (
        "particles: extra life",
        "Textures/Particles/P_Extra_Life_Particle.bmp",
    ),
    ("sound: menu click", "Sounds/Menu_click.wav"),
    ("sound: menu load", "Sounds/Menu_load.wav"),
    ("sound: level start", "Sounds/Misc_StartLevel.wav"),
    ("sound: checkpoint", "Sounds/Misc_Checkpoint.wav"),
    ("sound: ball lost", "Sounds/Misc_Fall.wav"),
    ("sound: new ball", "Sounds/Misc_Lightning.wav"),
    ("sound: transformer", "Sounds/Misc_Trafo.wav"),
    ("sound: extra life", "Sounds/Extra_Life_Blob.wav"),
    ("sound: extra ball", "Sounds/Misc_extraball.wav"),
    ("sound: extra point", "Sounds/Extra_Start.wav"),
    ("sound: extra point hit", "Sounds/Extra_Hit.wav"),
    ("sound: rope tears", "Sounds/Misc_RopeTears.wav"),
    ("sound: fan", "Sounds/Misc_Ventilator.wav"),
    ("sound: thunder", "Sounds/Music_thunder.wav"),
];

/// The role a file of the original's layout plays, if it is one of the interface's.
pub fn role_of(relative: &str) -> Option<&'static str> {
    ROLES
        .iter()
        .find(|(_, path)| path.eq_ignore_ascii_case(relative))
        .map(|(role, _)| *role)
}

/// Says once that a role has no provider (the game goes on without it).
pub fn missing(relative: &str) {
    static SAID: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let Some(role) = role_of(relative) else {
        return;
    };
    if let Ok(mut said) = SAID.lock()
        && !said.iter().any(|s| s == relative)
    {
        said.push(relative.to_owned());
        eprintln!("missing: {role} ({relative}) — no own file, no copy");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choice_reads_and_orders() {
        assert_eq!(Provider::parse(" own "), Some(Provider::Own));
        assert_eq!(Provider::parse("nonsense"), None);
        assert_eq!(Provider::Own.order()[0], Provider::Own);
        assert_eq!(Provider::Original.order()[0], Provider::Original);
        assert_eq!(
            Provider::Remastered.order(),
            [Provider::Remastered, Provider::Original, Provider::Own]
        );
        assert_eq!(role_of("sounds/misc_fall.WAV"), Some("sound: ball lost"));
        assert_eq!(role_of("3D Entities/Level/Level_01.NMO"), None);
    }

    #[test]
    fn a_texture_is_known_by_its_name_whatever_its_spelling() {
        assert_eq!(texture_key("BallWood.bmp"), texture_key("Ball_Wood.BMP"));
        assert_eq!(texture_key("Textures/Floor_Side.bmp"), "floorside");
        assert_eq!(texture_key("C:\\Ballance\\Textures\\Floor_Side"), "floorside");
        assert_eq!(texture_key("floor_top_Checkpoint"), "floortopcheckpoint");
        assert_eq!(texture_key("Floor.Top.Strange"), "floor.top.strange");
        assert_eq!(TextureSet::parse(" original "), Some(TextureSet::Original));
        assert_eq!(TextureSet::parse("remastered"), None);
    }

    #[test]
    fn own_textures_are_plain_png_files_of_the_folder() {
        let dir = std::env::temp_dir().join(format!("drahtseilakt-own-tex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Wood_Old.png")).ok();
        std::fs::write(dir.join("Floor_Side.png"), b"x").ok();
        std::fs::write(dir.join("Button01_select.tga"), b"x").ok();
        let index = texture_index(&dir);
        assert!(index.contains_key("floorside"));
        // a folder named like a texture, and a file that is not a PNG, are not ours
        assert!(!index.contains_key("woodold"));
        assert!(!index.contains_key("button01select"));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.join("Floor_Side.png"), dir.join("Brick.png")).ok();
            assert!(!texture_index(&dir).contains_key("brick"));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
