// Where the game's own resources come from (docs/drahtseilakt/resources.md; decisions.md "Own content by
// default"). The game asks for a file of the original's layout ("3D Entities/Menu.nmo", "Sounds/Misc_Fall.wav",
// "Textures/Font_1.tga"); each such file plays a role (the menu's font, the fall's sound...). Three providers can
// fill a role: our own files shipped with the game (assets/own/, laid out as the original), the player's copy, and
// the toolbox's remaster of the copy (levels/remastered/.interface/). The player chooses which comes first with
// `interface own|original|remastered` (the extended mode's Interface switch); the classic mode plays the copy's.
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
            Provider::Original => [Provider::Original, Provider::Own, Provider::Remastered],
            Provider::Remastered => [Provider::Remastered, Provider::Original, Provider::Own],
        }
    }
}

/// What the game plays with now, and what the player chose (it takes effect on the next start: the interface's
/// files are loaded once).
static IN_USE: AtomicU8 = AtomicU8::new(1);
static CHOSEN: AtomicU8 = AtomicU8::new(1);

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

/// The settings' choice (`interface ...`), the copy's when none.
pub fn from_settings() -> Provider {
    crate::profile::setting("interface")
        .and_then(|s| Provider::parse(&s))
        .unwrap_or(Provider::Original)
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
}
