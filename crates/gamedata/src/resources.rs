// Where the game's own resources come from (docs/drahtseilakt/resources.md; decisions.md "Own content by
// default"). The game asks for a file of the original's layout ("3D Entities/Menu.nmo", "Sounds/Misc_Fall.wav",
// "Textures/Font_1.tga"); each such file plays a role (the menu's font, the fall's sound...). Three providers can
// fill a role: our own files shipped with the game (assets/own/, laid out as the original), the player's copy, and
// the toolbox's remaster of the copy (levels/remastered/.interface/). The player chooses which comes first with
// `interface own|original|remastered` (the extended mode's Interface switch). The default, and what the classic
// mode plays, is the copy's own interface when the copy has it, else ours (owner's decision 2026-11-03, while our
// own interface is redone).
// A level's parts (modules, balls, sky, particles, shadow) are ours too: a new level takes ours first, a level of the
// copy the copy's first (assets.rs, Data::own_parts). A role no provider fills is missing: logged once, never a crash.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

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

static HAVE_COPY: AtomicBool = AtomicBool::new(true);

/// Whether the player's copy of the original exists (as last told by `set_copy`).
pub fn have_copy() -> bool {
    HAVE_COPY.load(Ordering::Relaxed)
}

/// Tells the interface choice whether the player's copy exists (before the choice is read).
pub fn set_copy(have: bool) {
    HAVE_COPY.store(have, Ordering::Relaxed);
}

/// Whether an interface can be chosen here: the copy's needs the copy, the toolbox's remaster of it needs the remaster.
/// A saved choice that cannot be used plays as ours (and is not offered by the menu).
pub fn usable(p: Provider) -> bool {
    match p {
        Provider::Own => true,
        Provider::Original => HAVE_COPY.load(Ordering::Relaxed),
        Provider::Remastered => {
            HAVE_COPY.load(Ordering::Relaxed)
                && std::fs::symlink_metadata(remastered_root()).is_ok_and(|m| m.is_dir())
        }
    }
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

/// The game's own assets folder (fonts, own files), in this order: `DRAHTSEILAKT_ASSETS`; `assets/` beside the
/// program; `../Resources/assets` (a macOS .app bundle); then, only as a developer's fallback, the source tree (three
/// folders up from a build's target/release, four from a test's target/release/deps, and last the path the program
/// was compiled in). One answer for the whole game, found once.
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
        // a test's program sits one folder deeper, in target/<profile>/deps: only there, never for an installed
        // program (whose folder four up could be anyone's)
        let named = |p: Option<&Path>, n: &str| p.and_then(Path::file_name).is_some_and(|f| f == n);
        let deps = exe
            .as_deref()
            .filter(|e| named(e.parent(), "deps") && named(e.ancestors().nth(3), "target"))
            .and_then(|e| e.ancestors().nth(4))
            .map(|d| d.join("assets"));
        let bundle = exe
            .as_deref()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(|d| d.join("Resources/assets"));
        let compiled = Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"));
        from_env
            .into_iter()
            .chain(beside)
            .chain(bundle)
            .chain(tree)
            .chain(deps)
            .chain(compiled)
            .find(|p| real(p))
    })
    .clone()
}

/// Our own files, laid out as the original: `own/` in the assets folder.
pub fn own_root() -> Option<PathBuf> {
    assets_root().map(|a| a.join("own"))
}

/// The levels shipped with the game, in the open format: `levels/` in the assets folder.
pub fn shipped_levels_root() -> Option<PathBuf> {
    assets_root().map(|a| a.join("levels"))
}

/// Whether a file of the original's layout is one of a level's parts, which our own files fill too (the modules, the
/// balls, the transformer, the skies, the particles and the ball's shadow): asked from ours first for a new level, from
/// the copy first for the copy's (assets.rs).
pub fn is_part(relative: &str) -> bool {
    let l = relative.to_ascii_lowercase();
    l.starts_with("3d entities/ph/")
        || l.starts_with("3d entities/balls.")
        || l.starts_with("3d entities/animtrafo.")
        || l.starts_with("textures/sky/")
        || l.starts_with("textures/particles/")
        || l.starts_with("textures/hardshadow.")
}

/// The toolbox's remaster of the copy's interface.
pub fn remastered_root() -> PathBuf {
    // found once: it is looked in on every file the copy lacks (the settings file need not be read each time)
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| crate::profile::dir(crate::profile::Kind::Remastered).join(".interface"))
        .clone()
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

/// The textures of the original's levels and parts, by name (its `Textures` folder; the names only, which every
/// community map uses): what a map's texture that carries no picture and names no file can be meant to be.
const LEVEL_TEXTURES: [&str; 53] = [
    "Ball_LightningSphere1",
    "Ball_LightningSphere2",
    "Ball_LightningSphere3",
    "Ball_Paper",
    "Ball_Stone",
    "Ball_Wood",
    "Brick",
    "Column_beige",
    "Column_beige_fade",
    "Column_blue",
    "Dome",
    "DomeEnvironment",
    "DomeShadow",
    "E_Holzbeschlag",
    "ExtraBall",
    "ExtraParticle",
    "FloorGlow",
    "Floor_Side",
    "Floor_Top_Border",
    "Floor_Top_Borderless",
    "Floor_Top_Checkpoint",
    "Floor_Top_Flat",
    "Floor_Top_Profil",
    "Floor_Top_ProfilFlat",
    "HardShadow",
    "Laterne_Glas",
    "Laterne_Schatten",
    "Laterne_Verlauf",
    "Metal_stained",
    "Modul03_Floor",
    "Modul03_Wall",
    "Modul11_13_Wood",
    "Modul11_Wood",
    "Modul15",
    "Modul16",
    "Modul18",
    "Modul18_Gitter",
    "Modul30_d_Seiten",
    "Rail_Environment",
    "SkyLayer",
    "Stick_Bottom",
    "Stick_Stripes",
    "Tower_Roof",
    "Trafo_Environment",
    "Trafo_FlashField",
    "Trafo_Shadow_Big",
    "Wood_Metal",
    "Wood_MetalStripes",
    "Wood_Misc",
    "Wood_Nailed",
    "Wood_Old",
    "Wood_Panel",
    "Wood_Plain",
];

/// The game's texture a map's texture means when it carries no pixels and names no file, if its name says so
/// (`materials`: the names of the materials that draw with it, for a texture without a name):
/// - the texture's name is the game's texture's (`Floor_Top_Profil`, also with an editor's copy suffix after a dot);
/// - it is that name cut to the eight letters of a DOS file name, as an exporter left it (`METAL_ST`, `RAIL_ENV`,
///   with copy marks after it: `METAL_STCC`, `RAIL_ENV000`): the game's texture beginning so, the shortest of them
///   when several do (`WOOD_MET` is `Wood_Metal`, not `Wood_MetalStripes`; `FLOOR_TO` the plain `Floor_Top_Flat`);
/// - it has no name, and a material drawing with it is named after a game's texture (`_aaaSkyLayer`): that one,
///   the longest name found in the material's.
///
/// A guess where the cut name fits several; the map has nothing better, and the original draws such a texture as
/// none at all. The answer is one of the game's own names, never the map's text.
pub fn meant_texture(name: &str, materials: &[&str]) -> Option<&'static str> {
    let stem = name.split('.').next().unwrap_or(name).trim();
    if stem.is_empty() {
        return materials
            .iter()
            .filter_map(|m| {
                let m = texture_key(m);
                LEVEL_TEXTURES
                    .iter()
                    .filter(|t| m.contains(&texture_key(t)))
                    .max_by_key(|t| t.len())
            })
            .max_by_key(|t| t.len())
            .copied();
    }
    let key = texture_key(stem);
    if let Some(t) = LEVEL_TEXTURES.iter().find(|t| texture_key(t) == key) {
        return Some(t);
    }
    // a DOS name: eight characters, then nothing but an editor's copy marks
    let lower = stem.to_ascii_lowercase();
    let (short, marks) = (lower.get(..8)?, lower.get(8..)?);
    if !marks.bytes().all(|b| b.is_ascii_digit() || b == b'c') {
        return None;
    }
    LEVEL_TEXTURES
        .iter()
        .filter(|t| t.to_ascii_lowercase().starts_with(short))
        .min_by_key(|t| t.len())
        .copied()
}

/// The most own textures looked at (the folder ships about sixty).
const OWN_TEXTURES_MAX: usize = 4096;

/// Our own textures by their key: the PNG files of `dir` (plain files only; a link or a folder is passed over).
pub fn texture_index(dir: &Path) -> std::collections::HashMap<String, PathBuf> {
    let mut index = std::collections::HashMap::new();
    let Ok(listed) = std::fs::read_dir(dir) else {
        return index;
    };
    for entry in listed.flatten().take(OWN_TEXTURES_MAX) {
        let path = entry.path();
        let plain = std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file());
        let png = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("png"));
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let key = texture_key(name);
        if plain && png && !key.is_empty() {
            index.entry(key).or_insert(path);
        }
    }
    index
}

/// Our own texture standing in for the original's texture `name` (a texture's name or its file's), if we have one:
/// `own/Textures/<name>.png` among the game's assets, or the track's own `own/Track/<name>.png`. The name only picks among our files; it never makes a path.
pub fn own_texture(name: &str) -> Option<PathBuf> {
    static INDEX: std::sync::OnceLock<std::collections::HashMap<String, PathBuf>> =
        std::sync::OnceLock::new();
    // the stand-ins for the original's textures, and the track's own (our levels name them: open-format.md, `game:`)
    let index = INDEX.get_or_init(|| {
        let Some(root) = own_root() else {
            return Default::default();
        };
        let mut index = texture_index(&root.join("Textures"));
        for (k, v) in texture_index(&root.join("Track")) {
            index.entry(k).or_insert(v);
        }
        index
    });
    index
        .get(&texture_key(name))
        // a map's duplicates keep the picture's name with an editor's suffix (`Laterne_Verlauf.tga.003`,
        // `Stick_Bottom.001`): the picture is the part before the first dot
        .or_else(|| {
            let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
            let (base, _) = file.split_once('.').filter(|(b, _)| !b.is_empty())?;
            index.get(&texture_key(base))
        })
        .cloned()
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

/// Says once that our part stands in for one the copy lacks or cannot read, in a level made for the copy's parts.
pub fn stand_in(relative: &str) {
    static SAID: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    if let Ok(mut said) = SAID.lock()
        && !said.iter().any(|s| s == relative)
    {
        said.push(relative.to_owned());
        eprintln!(
            "warning: {relative}: the copy lacks it or it does not read; our own part stands in"
        );
    }
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

    /// The names the community's maps carry (the 605-map audit): whole, cut to a DOS name, copied, or none.
    #[test]
    fn a_texture_without_a_picture_is_meant_by_its_name() {
        let none: [&str; 0] = [];
        assert_eq!(
            meant_texture("Floor_Top_Profil", &none),
            Some("Floor_Top_Profil")
        );
        assert_eq!(
            meant_texture("Floor_Top_ProfilFlat.Copy", &none),
            Some("Floor_Top_ProfilFlat")
        );
        assert_eq!(meant_texture("METAL_ST", &none), Some("Metal_stained"));
        assert_eq!(meant_texture("METAL_STCC", &none), Some("Metal_stained"));
        assert_eq!(
            meant_texture("METAL_ST.Copy.Copy", &none),
            Some("Metal_stained")
        );
        assert_eq!(
            meant_texture("RAIL_ENV000", &none),
            Some("Rail_Environment")
        );
        assert_eq!(meant_texture("Wood_Met", &none), Some("Wood_Metal"));
        assert_eq!(meant_texture("WOOD_PAN", &none), Some("Wood_Panel"));
        assert_eq!(meant_texture("FLOOR_SI", &none), Some("Floor_Side"));
        assert_eq!(meant_texture("FLOOR_TO", &none), Some("Floor_Top_Flat"));
        assert_eq!(
            meant_texture("", &["Core", "_aaaSkyLayer001"]),
            Some("SkyLayer")
        );
        // a name of its own, a short one, a cut name with more than copy marks after it, no name and no hint
        assert_eq!(meant_texture("Venti", &none), None);
        assert_eq!(meant_texture("Wood", &none), None);
        assert_eq!(meant_texture("METAL_STUFF", &none), None);
        assert_eq!(meant_texture("", &["03 - Default"]), None);
        assert_eq!(meant_texture("", &none), None);
        // names that are no ASCII do not cut inside a letter
        assert_eq!(meant_texture("水面水面水面", &none), None);
    }

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
        assert!(is_part("3D Entities/PH/P_Box.nmo") && is_part("Textures/Sky/Sky_A_Front.bmp"));
        assert!(is_part("Textures/HardShadow.bmp") && is_part("3D Entities/Balls.nmo"));
        assert!(
            !is_part("3D Entities/Level/Level_01.NMO") && !is_part("Textures/Button01_select.tga")
        );
    }

    #[test]
    fn a_texture_is_known_by_its_name_whatever_its_spelling() {
        assert_eq!(texture_key("BallWood.bmp"), texture_key("Ball_Wood.BMP"));
        assert_eq!(texture_key("Textures/Floor_Side.bmp"), "floorside");
        assert_eq!(
            texture_key("C:\\Ballance\\Textures\\Floor_Side"),
            "floorside"
        );
        assert_eq!(texture_key("floor_top_Checkpoint"), "floortopcheckpoint");
        assert_eq!(texture_key("Floor.Top.Strange"), "floor.top.strange");
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
