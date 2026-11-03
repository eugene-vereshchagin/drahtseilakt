// Finding things in the player's own copy of the game: levels, the balls, textures. File names in levels are
// Windows names, so lookups ignore case.

use std::path::{Path, PathBuf};

/// The largest texture file of an open-format level read, and its largest side (the toolbox makes them up to 8K).
const IMAGE_MAX: u64 = 64 * 1024 * 1024;
const SIDE_MAX: u32 = 8192;

/// The folder of a level that keeps the game's files it uses (its modules, the balls, the transformer, its sky,
/// particles, shadow and sounds), by their paths in the player's copy; a level that has it plays without the copy's
/// level files (docs/drahtseilakt/open-format.md).
pub const GAME_FILES: &str = "game";

/// The largest game file a level folder may hold (a sound, a picture).
const GAME_FILE_MAX: u64 = 64 * 1024 * 1024;
/// The largest Virtools file read (the original's are a few MB, the community's largest maps tens).
const NMO_MAX: u64 = 256 * 1024 * 1024;
/// The most texture pixels a level's own game files may bring in all (1 GB of RGBA), counted from their headers.
const OVERLAY_PIXELS_MAX: u64 = 256 * 1024 * 1024;
static OVERLAY_PIXELS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The level's textures already counted (a texture decoded again, as a sky face is, counts once).
static OVERLAY_COUNTED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// The game files a level's own folder may stand in for: its parts (the modules, the balls, the transformer), its
/// sky, particles and shadow, and a run's sounds. The interface's files (the menus, the font, the level table, the
/// menu's sounds) are always the copy's: a stranger's level never changes them.
fn overlay_serves(relative: &str) -> bool {
    let l = relative.to_ascii_lowercase();
    let sound = l.strip_prefix("sounds/").is_some_and(|n| {
        !(n.starts_with("menu_") || n.starts_with("atari") || n.starts_with("music_highscore"))
    });
    l.starts_with("3d entities/ph/")
        || ["3d entities/balls", "3d entities/animtrafo"]
            .iter()
            .any(|p| l == *p || l.strip_prefix(p).is_some_and(|r| r.starts_with('.') || r.starts_with('/')))
        || l.starts_with("textures/sky/")
        || l.starts_with("textures/particles/")
        || l.strip_prefix("textures/").is_some_and(|n| n.starts_with("hardshadow."))
        || sound
}

/// A Virtools file of the copy read whole, through the cap every game file read has.
pub fn read_game_file(path: &Path) -> Option<Vec<u8>> {
    read_capped(path, NMO_MAX)
}

/// A file read whole, a plain one of at most `cap` bytes (what it says its length is does not count).
fn read_capped(path: &Path, cap: u64) -> Option<Vec<u8>> {
    use std::io::Read;
    if !std::fs::metadata(path).is_ok_and(|m| m.is_file()) {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path).ok()?.take(cap.saturating_add(1)).read_to_end(&mut bytes).ok()?;
    (u64::try_from(bytes.len()).ok()? <= cap).then_some(bytes)
}

#[derive(Clone, Debug)]
pub struct Data {
    /// The player's copy of the original, if there is one: the game starts and plays without it (decisions.md, "The
    /// game plays out of the box").
    pub root: Option<PathBuf>,
    /// The game files of the level being played (`<level>/game`), looked in before the copy.
    pub overlay: Option<PathBuf>,
    /// The level's parts (modules, balls, transformer, sky, particles, shadow) are ours before the copy's: the level
    /// is new content, made for our parts, so it plays the same with or without a copy. A level of the copy (or
    /// converted from one) plays with the copy's parts, as it was made.
    pub own_parts: bool,
}

/// Where a level comes from.
#[derive(Clone)]
pub enum Source {
    /// One of the original levels of the player's copy (1–13).
    Original(u32),
    /// A `.nmo` file: a community map, or any level file.
    Nmo(PathBuf),
    /// A level in the open format: a folder with level.gltf and level.bin.
    Open(PathBuf),
}

impl Source {
    /// The level is in the remastered folder (drawn with the remastered look).
    pub fn remastered(&self) -> bool {
        let root = crate::profile::dir(crate::profile::Kind::Remastered);
        matches!(self, Source::Open(dir) if dir.parent().is_some_and(|p| p == root))
    }
}

pub struct LoadedLevel {
    pub level: level::Level,
    pub name: String,
    pub hash: u64,
    pub warnings: Vec<String>,
}

impl Data {
    /// The game's files with the player's copy at `root`, or with none.
    pub fn new(root: Option<PathBuf>) -> Data {
        Data { root, overlay: None, own_parts: false }
    }

    /// Whether the player's copy is there at all.
    pub fn has_copy(&self) -> bool {
        self.root.is_some()
    }

    /// The game's files: the player's copy where one is found (`explicit`, `OPENBALLANCE_DATA`, the profile's
    /// `levels/original`, `./data`), else none; the game plays its own levels without it.
    pub fn find(explicit: Option<PathBuf>) -> Data {
        Data::locate(explicit).unwrap_or_else(|_| Data::new(None))
    }

    /// The player's copy in exactly the folder `root` (`--data DIR`): no other place is tried, a folder that is not a
    /// playable copy is refused.
    pub fn at(root: PathBuf) -> Result<Data, String> {
        if playable(&root) {
            Ok(Data::new(Some(root)))
        } else {
            Err(format!("{}: not a copy of Ballance (no 3D Entities/Level/Level_01.NMO in it)", root.display()))
        }
    }

    pub fn locate(explicit: Option<PathBuf>) -> Result<Data, String> {
        let candidates = explicit
            .into_iter()
            .chain(std::env::var_os("OPENBALLANCE_DATA").map(PathBuf::from))
            .chain([crate::profile::dir(crate::profile::Kind::Original), PathBuf::from("data")]);
        for root in candidates {
            if playable(&root) {
                return Ok(Data::new(Some(root)));
            }
        }
        Err(
            "game data not found: import your Ballance with --import DIR, pass --data DIR or set OPENBALLANCE_DATA \
             to your Ballance folder"
                .into(),
        )
    }

    /// A file of the copy by its path inside it (`/` between parts, any case). Names come from levels, which may be
    /// hostile: a part that is empty, `.` or `..`, or holds `\` or `:`, finds nothing (it could leave the copy).
    /// The folder of the level `source` names, when it is one (an open-format level, or an original the copy lacks
    /// played from its remastered export).
    pub fn level_dir(&self, source: &Source) -> Option<PathBuf> {
        match source {
            Source::Open(dir) => Some(dir.clone()),
            Source::Original(n) => match self.original(*n) {
                Some(Source::Open(dir)) => Some(dir),
                _ => None,
            },
            Source::Nmo(_) => None,
        }
    }

    /// The same copy for playing the level in `level_dir`: its own game files (if it has them) first.
    /// A file the level's own folder has of the files it may stand in for (its own sounds, say), never the copy's.
    pub fn level_own(&self, relative: &str) -> Option<PathBuf> {
        let o = self.overlay.as_deref()?;
        overlay_serves(relative).then(|| plain_path(o, relative)).flatten()
    }

    pub fn for_level(&self, level_dir: Option<&Path>) -> Data {
        // each level brings its own textures, within the budget again
        OVERLAY_PIXELS.store(0, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut counted) = OVERLAY_COUNTED.lock() {
            counted.clear();
        }
        Data {
            root: self.root.clone(),
            overlay: level_dir.map(|d| d.join(GAME_FILES)),
            own_parts: self.own_parts,
        }
    }

    /// The same files for playing `source`: its own folder's game files first (which parts come next is the loaded
    /// level's to say: `with_parts`).
    pub fn for_source(&self, source: &Source) -> Data {
        self.for_level(self.level_dir(source).as_deref())
    }

    /// The same files for the loaded `level`'s parts: ours before the copy's when the level says it is made for them
    /// (its settings' `"parts": "own"`, which our level kit writes), else the copy's first. Only the level's own
    /// marker decides, so a level made or exported before keeps the parts (and replays) it had.
    pub fn with_parts(&self, level: &level::Level) -> Data {
        Data { own_parts: level.settings.own_parts, ..self.clone() }
    }

    pub fn file(&self, relative: &str) -> Option<PathBuf> {
        let overlay = self.overlay.as_deref().filter(|_| overlay_serves(relative));
        if let Some(found) = overlay.and_then(|o| plain_path(o, relative)) {
            return Some(found);
        }
        self.copy_file(relative)
    }

    /// A file of the game's layout from its providers (never the level's own): our own files, the player's copy and
    /// the toolbox's remaster of it, in the order the interface setting gives (resources.rs). A level's parts (its
    /// modules, the balls, sky, particles, shadow) come from ours first for a new level, else from the copy first.
    fn copy_file(&self, relative: &str) -> Option<PathBuf> {
        use crate::resources::Provider;
        // our own files and the toolbox's remaster stand in for the interface's roles; ours also for a level's parts
        // (a level itself, a level's own texture or sound always comes from the copy)
        let role = crate::resources::role_of(relative).is_some();
        let part = crate::resources::is_part(relative);
        let order = if part && self.own_parts {
            [Provider::Own, Provider::Original, Provider::Own]
        } else if part && !role {
            Provider::Original.order()
        } else {
            crate::resources::in_use().order()
        };
        let found = order.into_iter().find_map(|p| match p {
            Provider::Original => self.copy_only(relative),
            Provider::Own if role || part => own_file(relative),
            Provider::Remastered if role => plain_path(&crate::resources::remastered_root(), relative),
            _ => None,
        });
        if found.is_none() {
            crate::resources::missing(relative);
        }
        found
    }

    /// Whether the player's copy itself has a file (no other provider asked).
    pub fn copy_has(&self, relative: &str) -> bool {
        self.copy_only(relative).is_some()
    }

    /// A file of the player's copy itself.
    fn copy_only(&self, relative: &str) -> Option<PathBuf> {
        relative.split('/').try_fold(self.root.clone()?, |dir, part| {
            let plain = !matches!(part, "" | "." | "..") && !part.contains(['\\', ':']);
            plain.then(|| find(&dir, part)).flatten()
        })
    }

    /// Where original level `n` is: its file in the copy, or, for a level the copy lacks (a copy without level 13),
    /// the player's own remastered export of it in levels/remastered/Level_NN.
    pub fn original(&self, n: u32) -> Option<Source> {
        if let Some(p) = self.file(&format!("3D Entities/Level/Level_{n:02}.NMO")) {
            return Some(Source::Nmo(p));
        }
        let dir = crate::profile::dir(crate::profile::Kind::Remastered).join(format!("Level_{n:02}"));
        dir.join("level.gltf").is_file().then_some(Source::Open(dir))
    }

    /// A game file the levels share (`relative` as "3D Entities/Balls.nmo"): the level's own copy of it in the open
    /// format when it has one (`<level>/game/3D Entities/Balls/`), else the copy's file.
    pub fn part(&self, relative: &str) -> Option<LoadedLevel> {
        let stem = relative.strip_suffix(".nmo").or_else(|| relative.strip_suffix(".NMO")).unwrap_or(relative);
        let overlay = self.overlay.as_deref().filter(|_| overlay_serves(stem));
        if let Some(dir) = overlay.and_then(|o| plain_path(o, stem)) {
            let mut part = load_open(&dir).ok()?;
            // a remastered original's parts are the copy's, remade; another level's are its maker's
            let maker = if self.overlay.as_deref().is_some_and(|o| o.starts_with(remastered_levels())) {
                Maker::Copy
            } else {
                Maker::Stranger
            };
            self.put_own_textures(&mut part.level, maker);
            return Some(part);
        }
        // ours, in the open format (`assets/own/3D Entities/PH/P_Box/`), before or after the copy's file; either way
        // its textures that our texture set has by name are ours (`textures own`)
        let own = || {
            let root = crate::resources::own_root()?;
            load_open(&plain_path(&root, stem)?).ok()
        };
        let copy = || load_nmo(&self.copy_only(relative)?).ok();
        let mut part = if self.own_parts {
            own().or_else(copy)
        } else {
            copy().or_else(|| {
                let ours = own();
                // a level of the copy plays with the copy's parts: ours standing in for one the copy lacks or cannot
                // read is said once
                if ours.is_some() && self.has_copy() {
                    crate::resources::stand_in(relative);
                }
                ours
            })
        };
        match &mut part {
            Some(p) => self.put_own_textures(&mut p.level, Maker::Copy),
            None => crate::resources::missing(relative),
        }
        part
    }

    pub fn load(&self, source: &Source) -> Result<LoadedLevel, String> {
        let mut loaded = self.load_level(source)?;
        // ours in place of the original's textures (`textures own`): the copy's levels (and their remasters) by name,
        // a stranger's map only where it uses the copy's very textures
        let maker = match source {
            Source::Original(_) => Maker::Copy,
            Source::Open(_) if source.remastered() => Maker::Copy,
            Source::Nmo(_) | Source::Open(_) => Maker::Stranger,
        };
        self.put_own_textures(&mut loaded.level, maker);
        Ok(loaded)
    }

    fn load_level(&self, source: &Source) -> Result<LoadedLevel, String> {
        let path = match source {
            Source::Original(n) => match self.original(*n) {
                Some(Source::Nmo(p)) => p,
                Some(Source::Open(dir)) => return self.load_open_level(&dir),
                _ => {
                    return Err(match &self.root {
                        Some(root) => format!("Level_{n:02}.NMO is not in {}", root.display()),
                        None => format!("Level_{n:02}.NMO: no copy of the original (import it from the menu)"),
                    });
                }
            },
            Source::Nmo(p) => p.clone(),
            Source::Open(dir) => return self.load_open_level(dir),
        };
        let mut loaded = load_nmo(&path)?;
        // Levels of the game name their sky, ball, music, light and bonus in the game's own table, not in the level
        // file; the level takes them, so an export carries them (the open format plays without the table).
        let s = self.settings(&loaded.name);
        loaded.level.sky = loaded.level.sky.take().or(s.sky);
        loaded.level.start_ball = loaded.level.start_ball.take().or(s.start_ball);
        loaded.level.settings = level::Settings {
            music: s.music,
            light: s.light,
            level_bonus: s.level_bonus,
            // the original's first level begins with its tutorial; its last one has the lightning and last music
            tutorial: loaded.name.eq_ignore_ascii_case("Level_01.NMO"),
            last_level: s.last,
            source_hash: Some(loaded.hash),
            own_parts: false,
            fog: None,
            clouds: None,
        };
        Ok(loaded)
    }

    /// An open-format level. One made before levels carried their settings has none: a remastered original
    /// (levels/remastered/Level_NN) takes them from the copy's table as before, any other plays with the defaults
    /// and says so.
    fn load_open_level(&self, dir: &Path) -> Result<LoadedLevel, String> {
        let mut loaded = load_open(dir)?;
        if loaded.level.settings != level::Settings::default() {
            return Ok(loaded);
        }
        let n = Source::Open(dir.to_path_buf()).remastered().then(|| {
            let name = dir.file_name()?.to_str()?;
            let k: u32 = name.strip_prefix("Level_")?.parse().ok()?;
            (format!("Level_{k:02}") == name).then_some(k)
        });
        match n.flatten() {
            Some(k) => {
                let s = self.settings(&loaded.name);
                loaded.level.sky = loaded.level.sky.take().or(s.sky);
                loaded.level.start_ball = loaded.level.start_ball.take().or(s.start_ball);
                loaded.level.settings = level::Settings {
                    music: s.music,
                    light: s.light,
                    level_bonus: s.level_bonus,
                    tutorial: k == 1,
                    last_level: s.last,
                    source_hash: None,
                    own_parts: false,
                    fog: None,
                    clouds: None,
                };
                loaded.warnings.push(format!(
                    "{}: made before levels kept their settings; they come from the game's table (remaster it again)",
                    dir.display()
                ));
            }
            None => loaded.warnings.push(format!(
                "{}: the level keeps no settings (music, light, bonus); it plays with the defaults",
                dir.display()
            )),
        }
        Ok(loaded)
    }

    /// RGBA pixels of a texture: its own pixels, or its file from the game's texture folders.
    pub fn texture(&self, t: &level::Texture) -> Option<(u32, u32, Vec<u8>)> {
        let (w, h, mut rgba) = self.texture_pixels(t)?;
        // the texture's transparent colour: those texels get alpha 0 (SetAlphaForTransparentColor)
        if let Some(key) = t.color_key {
            for [r, g, b, a] in rgba.as_chunks_mut::<4>().0 {
                if [*r, *g, *b] == key {
                    *a = 0;
                }
            }
        }
        Some((w, h, rgba))
    }

    fn texture_pixels(&self, t: &level::Texture) -> Option<(u32, u32, Vec<u8>)> {
        match &t.source {
            level::TextureSource::Pixels {
                width,
                height,
                rgba,
            } => Some((*width, *height, rgba.clone())),
            // an open-format level's texture: a plain file of its folder, its size checked before it is decoded
            level::TextureSource::LevelFile(path) => {
                // its folder a real one too (a link there would lead out of the level's folder)
                let folder = path.parent().is_some_and(|d| std::fs::symlink_metadata(d).is_ok_and(|m| m.is_dir()));
                let plain = folder && std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX);
                if !plain || !self.overlay_pixels(path) {
                    return None;
                }
                decode_capped(path)
            }
            // a texture a .nmo names: only the last part of its name, looked for in the game's texture folders (a
            // stranger's map may name any path there; it never reads one)
            level::TextureSource::File(name) => {
                let path = self.texture_file(&bare_name(name)?)?;
                // it may come from a level's game folder (a stranger's): its size checked before it is decoded
                if !std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX) {
                    return None;
                }
                if !self.overlay_pixels(&path) {
                    return None;
                }
                decode_capped(&path)
            }
            level::TextureSource::None => None,
        }
    }
}

/// Who made a level's (or part's) textures, for putting ours in their place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Maker {
    /// The copy: an original level, a part of the copy, or a remaster of one.
    Copy,
    /// Anyone else: a community map, an open-format level of a player's.
    Stranger,
}

/// The last part of a texture's name as a `.nmo` gives it (after any `/` or `\\`), if it is a plain file name.
fn bare_name(name: &str) -> Option<String> {
    let last = name.rsplit(['/', '\\']).next()?;
    (!matches!(last, "" | "." | "..") && !last.contains(':')).then(|| last.to_owned())
}

/// The folder of the remastered levels (a remastered original's parts are the copy's, remade).
fn remastered_levels() -> PathBuf {
    crate::profile::dir(crate::profile::Kind::Remastered)
}

impl Data {
    /// The file of a texture by its bare file name in the game's texture folders: the level's own first, then the
    /// copy's.
    fn texture_file(&self, file: &str) -> Option<PathBuf> {
        ["Textures", "Textures/Sky", "Textures/Particles"].iter().find_map(|dir| self.file(&format!("{dir}/{file}")))
    }

    /// Puts our own textures in place of the original's (resources.rs, `textures own`), as pixels, when the level is
    /// loaded (so the choice holds for everything drawn or exported from it):
    /// - a texture a `.nmo` names by file: ours of that file's name, unless the level's own folder brings the file;
    ///   by the texture's name only when no such file is found at all;
    /// - pixels a `.nmo` carries: of the copy's levels and parts, ours of the texture's name; of a stranger's map only
    ///   when they are the copy's own texture of that name, pixel for pixel (a map's own pictures stay its own);
    /// - an open-format level's files: replaced only in a remaster of the copy's level, by name.
    ///
    /// Ours have real alpha: the original's transparent colour is not applied to them.
    fn put_own_textures(&self, level: &mut level::Level, maker: Maker) {
        if crate::resources::textures() != crate::resources::TextureSet::Own {
            return;
        }
        for t in &mut level.textures {
            let own = match &t.source {
                level::TextureSource::File(name) => match bare_name(name) {
                    Some(file) => match self.texture_file(&file) {
                        Some(found) if self.overlay.as_deref().is_some_and(|o| found.starts_with(o)) => None,
                        Some(_) => crate::resources::own_texture(&file),
                        None => crate::resources::own_texture(&file).or_else(|| crate::resources::own_texture(&t.name)),
                    },
                    None => None,
                },
                level::TextureSource::Pixels { width, height, rgba } => {
                    let ours = crate::resources::own_texture(&t.name);
                    match maker {
                        Maker::Copy => ours,
                        Maker::Stranger => ours.filter(|_| self.is_copys_texture(&t.name, *width, *height, rgba)),
                    }
                }
                level::TextureSource::LevelFile(_) if maker == Maker::Copy => crate::resources::own_texture(&t.name),
                level::TextureSource::LevelFile(_) | level::TextureSource::None => None,
            };
            let Some(path) = own else { continue };
            if !std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX) {
                continue;
            }
            if let Some((width, height, rgba)) = decode_capped(&path) {
                t.source = level::TextureSource::Pixels { width, height, rgba };
                t.color_key = None;
            }
        }
    }

    /// Whether pixels a map carries under `name` are the copy's own texture of that name (its file in Textures/,
    /// decoded; compared by size and hash, each of the copy's files decoded once).
    fn is_copys_texture(&self, name: &str, width: u32, height: u32, rgba: &[u8]) -> bool {
        type Seen = std::collections::HashMap<PathBuf, Option<(u32, u32, u64)>>;
        static SEEN: std::sync::Mutex<Option<Seen>> = std::sync::Mutex::new(None);
        let Some(file) = bare_name(name) else { return false };
        let has_ext = file.rsplit_once('.').is_some_and(|(_, e)| e.len() <= 4);
        let names = if has_ext { vec![file] } else { vec![format!("{file}.bmp"), format!("{file}.tga"), file] };
        let Some(path) = names.iter().find_map(|n| self.copy_has_texture(n)) else { return false };
        let Ok(mut seen) = SEEN.lock() else { return false };
        let seen = seen.get_or_insert_with(Default::default);
        let copy = *seen.entry(path.clone()).or_insert_with(|| {
            let (w, h, px) = decode_capped(&path)?;
            Some((w, h, crate::fnv1a(&px)))
        });
        copy == Some((width, height, crate::fnv1a(rgba)))
    }

    /// The copy's own file in Textures/ (not a level's folder's).
    fn copy_has_texture(&self, file: &str) -> Option<PathBuf> {
        let path = self.copy_only(&format!("Textures/{file}"))?;
        std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX).then_some(path)
    }
}

impl Data {
    /// A texture from the level's own game files still fits the budget of their pixels (read from its header, then
    /// counted as taken); any other texture always does.
    fn overlay_pixels(&self, path: &Path) -> bool {
        use std::sync::atomic::Ordering;
        // the files of a level's own folder and of the toolbox's interface share the budget; the copy's do not
        let budgeted = self.overlay.as_deref().is_some_and(|o| path.starts_with(o))
            || path.starts_with(crate::resources::remastered_root());
        if !budgeted {
            return true;
        }
        if OVERLAY_COUNTED.lock().is_ok_and(|c| c.iter().any(|p| p == path)) {
            return true;
        }
        let Some((w, h)) = image::ImageReader::open(path)
            .ok()
            .and_then(|r| r.with_guessed_format().ok())
            .and_then(|r| r.into_dimensions().ok())
        else {
            return false;
        };
        let n = u64::from(w).saturating_mul(u64::from(h));
        let mut taken = OVERLAY_PIXELS.load(Ordering::Relaxed);
        loop {
            let Some(next) = taken.checked_add(n).filter(|&t| t <= OVERLAY_PIXELS_MAX) else {
                return false;
            };
            match OVERLAY_PIXELS.compare_exchange_weak(taken, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => {
                    if let Ok(mut counted) = OVERLAY_COUNTED.lock() {
                        counted.push(path.to_path_buf());
                    }
                    return true;
                }
                Err(now) => taken = now,
            }
        }
    }
}

/// A level's settings from the game's own table (`AllLevel` in Levelinit.nmo): the sky and the starting ball.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Letter of the sky textures (`Sky_<letter>_Front.bmp`, ...).
    pub sky: Option<String>,
    pub start_ball: Option<String>,
    /// The level's music theme (`Music_Theme_<n>_1..3`).
    pub music: Option<u32>,
    /// The points a finished level adds to the score.
    pub level_bonus: Option<i64>,
    /// The colour of the level's light (Gameplay_Sky sets Light_Ingame to it).
    pub light: Option<[f32; 3]>,
    /// The level is the table's last row (its own row, not the first one a map without a row takes): the game's
    /// last level (Gameplay_Blitz and the last music test CurrentLevel against the rows).
    pub last: bool,
}

impl Data {
    /// The data array `name` of the game file `relative`.
    pub fn data_array(&self, relative: &str, name: &str) -> Option<nmo::scene::DataArray> {
        let bytes = read_capped(&self.file(relative)?, NMO_MAX)?;
        let file = nmo::read(&bytes).ok()?;
        let doc = nmo::Document::parse(&file).ok()?;
        doc.of_class(nmo::class::DATA_ARRAY)
            .find(|(_, o)| o.name.eq_ignore_ascii_case(name.as_bytes()))
            .and_then(|(_, o)| o.chunk.as_ref())
            .and_then(|c| nmo::scene::data_array(c).ok())
    }

    /// The value of the parameter object at `index` of the game file `relative`, as floats.
    pub fn parameter_floats(&self, relative: &str, index: usize) -> Option<Vec<f32>> {
        let bytes = read_capped(&self.file(relative)?, NMO_MAX)?;
        let file = nmo::read(&bytes).ok()?;
        let doc = nmo::Document::parse(&file).ok()?;
        nmo::scene::parameter_floats(doc.get(index)?.chunk.as_ref()?).ok().flatten()
    }

    /// Every data array of the game file `relative` that `wanted` accepts by name, the file read once.
    pub fn data_arrays(
        &self,
        relative: &str,
        wanted: impl Fn(&str) -> bool,
    ) -> Vec<(String, nmo::scene::DataArray)> {
        let Some(bytes) = self.file(relative).and_then(|p| read_capped(&p, NMO_MAX)) else {
            return Vec::new();
        };
        let Ok(file) = nmo::read(&bytes) else {
            return Vec::new();
        };
        let Ok(doc) = nmo::Document::parse(&file) else {
            return Vec::new();
        };
        doc.of_class(nmo::class::DATA_ARRAY)
            .filter_map(|(_, o)| {
                let name = String::from_utf8_lossy(&o.name).into_owned();
                if !wanted(&name) {
                    return None;
                }
                let array = o
                    .chunk
                    .as_ref()
                    .and_then(|c| nmo::scene::data_array(c).ok())?;
                Some((name, array))
            })
            .collect()
    }

    /// How many levels the game's table has (the rows of `AllLevel` in Levelinit.nmo): the last of them is the one
    /// the scripts treat as the last level (its finish music, its lightning).
    pub fn level_count(&self) -> Option<u32> {
        let table = self.data_array("3D Entities/Levelinit.nmo", "AllLevel")?;
        u32::try_from(table.rows.len()).ok()
    }

    /// The settings row whose level file matches `file_name`; a community map without one takes level 1's.
    pub fn settings(&self, file_name: &str) -> Settings {
        let none = Settings {
            sky: None,
            start_ball: None,
            music: None,
            level_bonus: None,
            light: None,
            last: false,
        };
        let Some(table) = self.data_array("3D Entities/Levelinit.nmo", "AllLevel") else {
            return none;
        };
        let text = |row: &Vec<nmo::scene::Cell>, col: &str| -> Option<String> {
            match table.column(col).and_then(|c| row.get(c)) {
                Some(nmo::scene::Cell::String(s)) => Some(String::from_utf8_lossy(s).into_owned()),
                _ => None,
            }
        };
        let own = table
            .rows
            .iter()
            .position(|r| {
                // a level file's name, or an export's folder named after it (Level_13 for Level_13.NMO)
                text(r, "Levelfile").is_some_and(|f| {
                    f.eq_ignore_ascii_case(file_name)
                        || Path::new(&f).file_stem().is_some_and(|s| s.eq_ignore_ascii_case(file_name))
                })
            });
        let last = own.is_some_and(|i| i.checked_add(1) == Some(table.rows.len()));
        let Some(row) = own.and_then(|i| table.rows.get(i)).or_else(|| table.rows.first()) else { return none };
        Settings {
            last,
            sky: text(row, "Sky").and_then(|s| s.strip_prefix("Sky_").map(str::to_owned)),
            start_ball: text(row, "StartBall"),
            music: match table.column("Music").and_then(|c| row.get(c)) {
                Some(nmo::scene::Cell::Int(n)) => u32::try_from(*n).ok(),
                _ => None,
            },
            level_bonus: match table.column("LevelBonus").and_then(|c| row.get(c)) {
                Some(nmo::scene::Cell::Int(n)) => Some(i64::from(*n)),
                _ => None,
            },
            light: match table.column("Light").and_then(|c| row.get(c)) {
                Some(nmo::scene::Cell::Object(Some(p))) => {
                    self.parameter_floats("3D Entities/Levelinit.nmo", *p).and_then(|v| light_of(&v))
                }
                _ => None,
            },
        }
    }
}

/// A copy counts when it has the first level, not only the folder: a half-made import is not a game.
fn playable(root: &Path) -> bool {
    ["3D Entities", "Level", "Level_01.NMO"].iter().try_fold(root.to_path_buf(), |d, p| find(&d, p)).is_some()
}

/// One of our own files by its path in the original's layout; a picture the original has as `.bmp` or `.tga` may be
/// ours as `.png` (our textures are made as PNG).
fn own_file(relative: &str) -> Option<PathBuf> {
    let root = crate::resources::own_root()?;
    plain_path(&root, relative).or_else(|| {
        let l = relative.to_ascii_lowercase();
        let stem = relative.get(..relative.len().checked_sub(4)?)?;
        // ours are PNG, the photographic skies JPEG (a photograph in PNG would be several times the size)
        (l.ends_with(".bmp") || l.ends_with(".tga"))
            .then(|| plain_path(&root, &format!("{stem}.png")).or_else(|| plain_path(&root, &format!("{stem}.jpg"))))
            .flatten()
    })
}

pub fn load_open(dir: &Path) -> Result<LoadedLevel, String> {
    // its textures are named by their whole paths: a folder given relative to where the game was started would make
    // them relative ones, which are taken for the game's own texture names
    let dir = &std::path::absolute(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (level, bytes) = level::open::import_read(dir)?;
    Ok(LoadedLevel {
        level,
        name: dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        hash: crate::fnv1a(&bytes),
        warnings: Vec::new(),
    })
}

/// Writes a level in the open format into `dir`, with its textures as PNG files (from the player's copy).
/// Exports a level into `dir` in the open format with its textures, and with the game files it uses (its modules,
/// the balls, the transformer, its sky, particles, shadow and sounds) in `dir/game`, so that it plays without the
/// copy's level files.
pub fn export(data: &Data, loaded: &LoadedLevel, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let missing = export_level(data, &loaded.level, dir)?;
    let game = dir.join(GAME_FILES);
    let mut parts = 0usize;
    for rel in parts_used(data, &loaded.level) {
        let Some(part) = data.part(&rel) else { continue };
        let stem = rel.strip_suffix(".nmo").or_else(|| rel.strip_suffix(".NMO")).unwrap_or(&rel);
        real_folders(dir, &game.join(stem))?;
        export_level(data, &part.level, &game.join(stem))?;
        parts = parts.saturating_add(1);
    }
    let mut files = 0usize;
    real_folders(dir, &game)?;
    for rel in files_used(data, &loaded.level) {
        let Some(src) = data.file(&rel) else { continue };
        copy_into(&src, &game, &rel)?;
        files = files.saturating_add(1);
    }
    println!(
        "{}: {parts} modules and parts, {files} game files (sky, particles, shadow, sounds)",
        game.display()
    );
    println!(
        "{} written: {} objects, {} meshes, {} materials, {} textures ({missing} not found), {} groups",
        dir.display(),
        loaded.level.objects.len(),
        loaded.level.meshes.len(),
        loaded.level.materials.len(),
        loaded.level.textures.len(),
        loaded.level.groups.len()
    );
    Ok(())
}

/// Writes one level (or part) in the open format with its textures; returns how many textures were not found.
fn export_level(data: &Data, level: &level::Level, dir: &Path) -> Result<usize, String> {
    // the level's files are made anew: one there is removed first (a link or a hard link would be written through)
    for name in ["level.gltf", "level.bin"] {
        fresh(&dir.join(name))?;
    }
    level::open::export(level, dir)?;
    real_folders(dir, &dir.join("textures"))?;
    let mut missing = 0usize;
    for (i, t) in level.textures.iter().enumerate() {
        let path = dir.join(level::open::texture_file(i, &t.name));
        // read first: the texture may be this very file (a level exported into its own folder); a texture not found
        // leaves what is there
        match data.texture(t) {
            Some((w, h, rgba)) => {
                fresh(&path)?;
                image::save_buffer(&path, &rgba, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?
            }
            None => missing = missing.saturating_add(1),
        }
    }
    Ok(missing)
}

/// Makes the folders from `root` (a real folder) down to `dir` one at a time, each a real folder: a link on the way is
/// refused, never followed.
fn real_folders(root: &Path, dir: &Path) -> Result<(), String> {
    if !std::fs::symlink_metadata(root).is_ok_and(|m| m.is_dir()) {
        return Err(format!("{}: not a folder", root.display()));
    }
    let rest = dir.strip_prefix(root).map_err(|_| format!("{}: outside {}", dir.display(), root.display()))?;
    let mut at = root.to_path_buf();
    for part in rest.components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err(format!("{}: not a plain path", dir.display()));
        }
        at.push(part);
        match std::fs::symlink_metadata(&at) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err(format!("{}: not a folder", at.display())),
            Err(_) => std::fs::create_dir(&at).map_err(|e| format!("{}: {e}", at.display()))?,
        }
    }
    Ok(())
}

/// Nothing at `path` any more: a plain file there is removed (so the new one is a file of its own, never written
/// through a link or a hard link); anything else there is refused.
fn fresh(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Err(_) => Ok(()),
        Ok(m) if m.is_file() || m.file_type().is_symlink() => {
            std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))
        }
        Ok(_) => Err(format!("{}: not a plain file", path.display())),
    }
}

/// A light colour from a parameter's floats: the first three, each finite and clamped to 0..1 (a NaN would pass a clamp
/// as it is, and the copy's Levelinit.nmo may be a level folder's own).
fn light_of(v: &[f32]) -> Option<[f32; 3]> {
    let c = |i: usize| v.get(i).copied().filter(|x| x.is_finite()).map(|x| x.clamp(0.0, 1.0));
    Some([c(0)?, c(1)?, c(2)?])
}

/// A picture decoded with its sides capped at SIDE_MAX, as RGBA.
fn decode_capped(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = read_capped(path, IMAGE_MAX)?;
    let ext = path.extension().and_then(|e| e.to_str());
    decode_image(&bytes, ext)
}

/// A picture's bytes decoded as `decode_capped` does (the format by the file's extension, else by its contents), its
/// sides capped at SIDE_MAX, as RGBA: the fuzz targets' entry.
pub fn decode_image(bytes: &[u8], ext: Option<&str>) -> Option<(u32, u32, Vec<u8>)> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes));
    if let Some(format) = ext.and_then(image::ImageFormat::from_extension) {
        reader.set_format(format);
    }
    let mut reader = reader.with_guessed_format().ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(SIDE_MAX);
    limits.max_image_height = Some(SIDE_MAX);
    reader.limits(limits);
    let img = reader.decode().ok()?.to_rgba8();
    Some((img.width(), img.height(), img.into_raw()))
}

/// The shared parts a level uses, by their paths in the copy: the balls, the transformer, and the module of every
/// object named after one (`P_Box_03` is P_Box, `PC_TwoFlames_01` the flames), as the copy names their files.
pub fn parts_used(data: &Data, level: &level::Level) -> Vec<String> {
    let mut out = vec!["3D Entities/Balls.nmo".to_owned(), "3D Entities/AnimTrafo.nmo".to_owned()];
    let Some(ph) = data.file("3D Entities/PH") else { return out };
    let Ok(listed) = std::fs::read_dir(&ph) else { return out };
    let mut names: Vec<String> = listed
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|n| n.to_ascii_lowercase().ends_with(".nmo"))
        .filter(|n| {
            let stem = n.get(..n.len().saturating_sub(4)).unwrap_or_default();
            level.objects.iter().any(|o| level::module_name(&o.name).eq_ignore_ascii_case(stem))
        })
        .take(1024)
        .collect();
    names.sort();
    out.extend(names.into_iter().map(|n| format!("3D Entities/PH/{n}")));
    out
}

/// The game files a level uses besides its parts, by their paths in the copy: its sky's six faces, the particles'
/// textures, the ball's shadow, and the sounds a run plays (not the menu's, nor other levels' music themes).
pub fn files_used(data: &Data, level: &level::Level) -> Vec<String> {
    let listed = |rel: &str| -> Vec<String> {
        let Some(dir) = data.file(rel) else { return Vec::new() };
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().to_str().map(str::to_owned))
            .take(4096)
            .collect();
        v.sort();
        v
    };
    let mut out = Vec::new();
    if let Some(letter) = &level.sky {
        let prefix = format!("sky_{}_", letter.to_ascii_lowercase());
        out.extend(
            listed("Textures/Sky")
                .into_iter()
                .filter(|n| n.to_ascii_lowercase().starts_with(&prefix))
                .map(|n| format!("Textures/Sky/{n}")),
        );
    }
    out.extend(listed("Textures/Particles").into_iter().map(|n| format!("Textures/Particles/{n}")));
    out.extend(
        listed("Textures")
            .into_iter()
            .filter(|n| n.to_ascii_lowercase().starts_with("hardshadow."))
            .map(|n| format!("Textures/{n}")),
    );
    let theme = level.settings.music.map(|m| format!("music_theme_{m}_"));
    out.extend(
        listed("Sounds")
            .into_iter()
            .filter(|n| {
                let l = n.to_ascii_lowercase();
                let interface = l.starts_with("menu_") || l.starts_with("atari") || l.starts_with("music_highscore");
                let other_theme = l.starts_with("music_theme_") && theme.as_ref().is_none_or(|t| !l.starts_with(t));
                l.ends_with(".wav") && !interface && !other_theme
            })
            .map(|n| format!("Sounds/{n}")),
    );
    out
}

/// A path inside `root` by its parts (`/` between them, any case), every step a real folder or file of `root`'s
/// own: a part that is empty, `.` or `..`, holds `\\` or `:`, or is a link finds nothing (a level folder is the
/// player's or a stranger's, and must not lead out of itself).
fn plain_path(root: &Path, relative: &str) -> Option<PathBuf> {
    if !std::fs::symlink_metadata(root).is_ok_and(|m| m.is_dir()) {
        return None;
    }
    relative.split('/').try_fold(root.to_path_buf(), |dir, part| {
        if matches!(part, "" | "." | "..") || part.contains(['\\', ':']) {
            return None;
        }
        // the exact name first, else the first in name order of those that match in any case (not the folder's order)
        let mut found: Vec<std::fs::DirEntry> = std::fs::read_dir(&dir)
            .ok()?
            .flatten()
            .take(4096)
            .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))
            .collect();
        found.sort_by_key(|e| (e.file_name().to_string_lossy() != part, e.file_name()));
        let entry = found.into_iter().next()?;
        let kind = entry.file_type().ok()?;
        (!kind.is_symlink()).then(|| entry.path())
    })
}

/// Copies a game file into a level's game folder `root` at `relative` (its path in the copy): the source a plain file
/// of at most GAME_FILE_MAX, `root` a real folder, the folders under it made one by one and never links, the file
/// written beside and moved into place.
pub fn copy_into(src: &Path, root: &Path, relative: &str) -> Result<(), String> {
    use std::io::Read;
    if !std::fs::metadata(src).is_ok_and(|m| m.is_file()) {
        return Err(format!("{}: not a plain file", src.display()));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(src)
        .and_then(|f| f.take(GAME_FILE_MAX.saturating_add(1)).read_to_end(&mut bytes))
        .map_err(|e| format!("{}: {e}", src.display()))?;
    if u64::try_from(bytes.len()).map_or(true, |n| n > GAME_FILE_MAX) {
        return Err(format!("{}: too big", src.display()));
    }
    if !std::fs::symlink_metadata(root).is_ok_and(|m| m.is_dir()) {
        return Err(format!("{}: not a folder", root.display()));
    }
    let parts: Vec<&str> = relative.split('/').collect();
    let Some((name, folders)) = parts.split_last() else { return Err("no file name".into()) };
    let plain = |p: &str| !matches!(p, "" | "." | "..") && !p.contains(['\\', ':']);
    if !parts.iter().all(|p| plain(p)) {
        return Err(format!("{relative}: not a plain path"));
    }
    let mut at = root.to_path_buf();
    for f in folders {
        at.push(f);
        match std::fs::symlink_metadata(&at) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err(format!("{}: not a folder", at.display())),
            Err(_) => std::fs::create_dir(&at).map_err(|e| format!("{}: {e}", at.display()))?,
        }
    }
    let dst = at.join(name);
    let tmp = at.join(format!("{name}.new"));
    for p in [&dst, &tmp] {
        if std::fs::symlink_metadata(p).is_ok_and(|m| !m.is_file()) {
            return Err(format!("{}: not a plain file", p.display()));
        }
    }
    // a file left at the neighbour's place is removed (a hard link there would be written through) and a new one made
    if std::fs::symlink_metadata(&tmp).is_ok() {
        std::fs::remove_file(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    }
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| format!("{}: {e}", tmp.display()))?;
        f.write_all(&bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    }
    std::fs::rename(&tmp, &dst).map_err(|e| format!("{}: {e}", dst.display()))
}

pub fn load_nmo(path: &Path) -> Result<LoadedLevel, String> {
    let bytes = read_capped(path, NMO_MAX)
        .ok_or_else(|| format!("{}: unreadable, or over {NMO_MAX} bytes", path.display()))?;
    let loaded = level::from_nmo::load(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(LoadedLevel {
        level: loaded.level,
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        hash: crate::fnv1a(&bytes),
        warnings: loaded.warnings,
    })
}

/// A directory entry by name, ignoring case.
pub fn find(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_maps_pixels_are_the_copys_only_when_equal_to_its_file() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-copytex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Textures")).unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([9, 8, 7, 255])).save(root.join("Textures/Wood_Old.bmp")).unwrap();
        let data = Data::new(Some(root.clone()));
        let same = [9u8, 8, 7, 255].repeat(4);
        let other = [9u8, 8, 6, 255].repeat(4);
        assert!(data.is_copys_texture("Wood_Old", 2, 2, &same), "by the name without its extension");
        assert!(data.is_copys_texture("wood_old.BMP", 2, 2, &same), "any case");
        assert!(!data.is_copys_texture("Wood_Old", 2, 2, &other), "a map's own picture under the name");
        assert!(!data.is_copys_texture("Wood_Old", 1, 4, &same), "another size");
        assert!(!data.is_copys_texture("Brick", 2, 2, &same), "a name the copy lacks");
        assert!(!data.is_copys_texture("../Textures/Wood_Old.bmp/..", 2, 2, &same));
        assert_eq!(bare_name("C:\\maps\\x.bmp").as_deref(), Some("x.bmp"));
        assert_eq!(bare_name("/a/b/../x.png").as_deref(), Some("x.png"));
        assert_eq!(bare_name("a/.."), None);
        assert_eq!(bare_name("x:y.bmp"), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Found by review while fuzzing: a NaN in the level table's light passed the clamp and lit the level with it.
    #[test]
    fn a_light_colour_is_three_finite_numbers() {
        assert_eq!(light_of(&[0.5, 2.0, -1.0, 9.0]), Some([0.5, 1.0, 0.0]));
        assert_eq!(light_of(&[0.5, f32::NAN, 0.0]), None);
        assert_eq!(light_of(&[f32::INFINITY, 0.0, 0.0]), None);
        assert_eq!(light_of(&[0.5, 0.5]), None);
    }
}
