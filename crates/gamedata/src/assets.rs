// Finding things in the player's own copy of the game: levels, the balls, textures. File names in levels are
// Windows names, so lookups ignore case.

use std::path::{Path, PathBuf};

/// The largest texture file of an open-format level read, and its largest side (the toolbox makes them up to 4K).
const IMAGE_MAX: u64 = 64 * 1024 * 1024;
const SIDE_MAX: u32 = 4096;

/// The folder of a level that keeps the game's files it uses (its modules, the balls, the transformer, its sky,
/// particles, shadow and sounds), by their paths in the player's copy; a level that has it plays without the copy's
/// level files (docs/drahtseilakt/open-format.md).
pub const GAME_FILES: &str = "game";

/// The largest game file a level folder may hold (a sound, a picture).
const GAME_FILE_MAX: u64 = 64 * 1024 * 1024;
/// The largest Virtools file read (the original's are a few MB, the community's largest maps tens).
const NMO_MAX: u64 = 256 * 1024 * 1024;
/// The most texture memory a level's own game files may bring in all: 1 GiB, counted from their headers (a decoded
/// picture is 4 bytes a pixel; a compressed texture, gputex.rs, is its file's data).
const OVERLAY_BYTES_MAX: u64 = 1024 * 1024 * 1024;
/// A decoded picture's bytes a pixel.
const RGBA: u64 = 4;
/// The share of what is left of the budget one of a level's own textures may take: a larger one is drawn smaller
/// (without its largest mip levels) instead of taking the place of those that come after it.
const SHARE: u64 = 8;
/// The largest side a compressed texture is decoded at for an adapter that cannot draw it as it is (a 4K picture
/// would be 85 MB of RGBA there).
const DECODED_SIDE_MAX: u32 = 1024;
/// The compressed textures of the level already counted, by their data's hash: one picture many of its parts carry
/// is uploaded once (render.rs) and counted once.
static OVERLAY_HASHES: std::sync::Mutex<Vec<(CompressedKey, usize)>> =
    std::sync::Mutex::new(Vec::new());
static OVERLAY_BYTES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The level's textures already counted (a texture decoded again, as a sky face is, counts once).
static OVERLAY_COUNTED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());
/// The budgeted pictures already kept as a texture's pixels in the level or one of its parts: each further texture
/// that keeps a copy of one is charged again (one picture standing in for many textures holds many copies).
static STORED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

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
            .any(|p| {
                l == *p
                    || l.strip_prefix(p)
                        .is_some_and(|r| r.starts_with('.') || r.starts_with('/'))
            })
        || l.starts_with("textures/sky/")
        || l.starts_with("textures/particles/")
        || l.strip_prefix("textures/")
            .is_some_and(|n| n.starts_with("hardshadow."))
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
    std::fs::File::open(path)
        .ok()?
        .take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
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
        Data {
            root,
            overlay: None,
            own_parts: false,
        }
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
            Err(format!(
                "{}: not a copy of Ballance (no 3D Entities/Level/Level_01.NMO in it)",
                root.display()
            ))
        }
    }

    pub fn locate(explicit: Option<PathBuf>) -> Result<Data, String> {
        let candidates = explicit
            .into_iter()
            .chain(std::env::var_os("OPENBALLANCE_DATA").map(PathBuf::from))
            .chain([
                crate::profile::dir(crate::profile::Kind::Original),
                PathBuf::from("data"),
            ]);
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
        overlay_serves(relative)
            .then(|| plain_path(o, relative))
            .flatten()
    }

    pub fn for_level(&self, level_dir: Option<&Path>) -> Data {
        // each level brings its own textures, within the budget again
        OVERLAY_BYTES.store(0, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut hashes) = OVERLAY_HASHES.lock() {
            hashes.clear();
        }
        if let Ok(mut counted) = OVERLAY_COUNTED.lock() {
            counted.clear();
        }
        if let Ok(mut stored) = STORED.lock() {
            stored.clear();
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
        Data {
            own_parts: level.settings.own_parts,
            ..self.clone()
        }
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
            Provider::Remastered if role => {
                plain_path(&crate::resources::remastered_root(), relative)
            }
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
        relative
            .split('/')
            .try_fold(self.root.clone()?, |dir, part| {
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
        let dir =
            crate::profile::dir(crate::profile::Kind::Remastered).join(format!("Level_{n:02}"));
        dir.join("level.gltf")
            .is_file()
            .then_some(Source::Open(dir))
    }

    /// A game file the levels share (`relative` as "3D Entities/Balls.nmo"): the level's own copy of it in the open
    /// format when it has one (`<level>/game/3D Entities/Balls/`), else the copy's file.
    pub fn part(&self, relative: &str) -> Option<LoadedLevel> {
        let stem = relative
            .strip_suffix(".nmo")
            .or_else(|| relative.strip_suffix(".NMO"))
            .unwrap_or(relative);
        let overlay = self.overlay.as_deref().filter(|_| overlay_serves(stem));
        if let Some(dir) = overlay.and_then(|o| plain_path(o, stem)) {
            let mut part = load_open(&dir).ok()?;
            // a remastered original's parts are the copy's, remade; another level's are its maker's
            let maker = if self
                .overlay
                .as_deref()
                .is_some_and(|o| o.starts_with(remastered_levels()))
            {
                Maker::Copy
            } else {
                Maker::Stranger
            };
            self.put_pack_textures(&mut part.level, maker);
            // a stranger's own parts keep their look; a remaster's are the copy's, remade, and take the model pack's
            if maker == Maker::Copy {
                self.put_pack_looks(relative, &mut part.level, PartFrom::Remaster);
            }
            return Some(part);
        }
        // ours, in the open format (`assets/own/3D Entities/PH/P_Box/`), before or after the copy's file; either way
        // its textures that our texture set has by name are ours (`textures own`)
        let own = || {
            let root = crate::resources::own_root()?;
            load_open(&plain_path(&root, stem)?).ok()
        };
        let copy = || load_nmo(&self.copy_only(relative)?).ok();
        let own = || own().map(|p| (p, PartFrom::Own));
        let copy = || copy().map(|p| (p, PartFrom::Copy));
        let found = if self.own_parts {
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
        let Some((mut part, from)) = found else {
            crate::resources::missing(relative);
            return None;
        };
        self.put_pack_textures(&mut part.level, Maker::Copy);
        self.put_pack_looks(relative, &mut part.level, from);
        Some(part)
    }

    /// Puts the chosen model pack's look on a part (`Kind::Models`, packs.rs): each of its meshes drawn as the pack's
    /// mesh of the same object (else of the same name), the physics and the rules still using the part's own meshes.
    /// The packs are asked in their chain's order; the first that has the part at all gives its look. `level` (the
    /// default), or a built-in pack that is where the part already came from, leaves the part as it is.
    fn put_pack_looks(&self, relative: &str, part: &mut level::Level, from: PartFrom) {
        self.put_looks_of(
            relative,
            part,
            from,
            &crate::packs::chain(crate::packs::Kind::Models),
        );
    }

    /// `put_pack_looks` with the chain of model packs given.
    fn put_looks_of(
        &self,
        relative: &str,
        part: &mut level::Level,
        from: PartFrom,
        chain: &[crate::packs::Pack],
    ) {
        use crate::packs::{LEVEL, ORIGINAL, OWN};
        let stem = relative
            .strip_suffix(".nmo")
            .or_else(|| relative.strip_suffix(".NMO"))
            .unwrap_or(relative);
        for pack in chain {
            let look = match (pack.key.as_str(), &pack.dir) {
                (LEVEL, _) => return,
                (OWN, _) if from == PartFrom::Own => return,
                (ORIGINAL, _) if from == PartFrom::Copy => return,
                (OWN, _) => crate::resources::own_root()
                    .and_then(|r| plain_path(&r, stem))
                    .and_then(|d| load_open(&d).ok())
                    .map(|mut p| {
                        self.put_pack_textures(&mut p.level, Maker::Copy);
                        p.level
                    }),
                (ORIGINAL, _) => self
                    .copy_only(relative)
                    .and_then(|p| load_nmo(&p).ok())
                    .map(|mut p| {
                        self.put_pack_textures(&mut p.level, Maker::Copy);
                        p.level
                    }),
                (_, Some(dir)) => load_pack_part(dir, stem).map(|mut l| {
                    // the pack's own pictures are its maker's: never replaced
                    self.put_pack_textures(&mut l, Maker::Stranger);
                    l
                }),
                (_, None) => None,
            };
            if let Some(look) = look
                && attach_looks(part, look) > 0
            {
                return;
            }
        }
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
        if matches!(source, Source::Nmo(_)) {
            name_meant_textures(&mut loaded.level);
        }
        self.put_pack_textures(&mut loaded.level, maker);
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
                        None => format!(
                            "Level_{n:02}.NMO: no copy of the original (import it from the menu)"
                        ),
                    });
                }
            },
            Source::Nmo(p) => p.clone(),
            Source::Open(dir) => return self.load_open_level(dir),
        };
        let mut loaded = load_nmo(&path)?;
        let info = crate::package::info_of(&path);
        // Levels of the game name their sky, ball, music, light and bonus in the game's own table, not in the level
        // file; the level takes them, so an export carries them (the open format plays without the table).
        let s = self.settings(&loaded.name);
        // what the map's own level.txt says wins; the table (only there with a copy) fills what it leaves open
        loaded.level.sky = info.sky.or(loaded.level.sky.take()).or(s.sky);
        loaded.level.start_ball = loaded.level.start_ball.take().or(s.start_ball);
        loaded.level.settings = level::Settings {
            music: info.music.or(s.music),
            light: info.light.or(s.light),
            level_bonus: s.level_bonus,
            // the original's first level begins with its tutorial; its last one has the lightning and last music
            tutorial: loaded.name.eq_ignore_ascii_case("Level_01.NMO"),
            last_level: s.last,
            source_hash: Some(loaded.hash),
            own_parts: false,
            fog: None,
            clouds: None,
            sector_balls: loaded
                .name
                .strip_prefix("Level_")
                .and_then(|r| r.split('.').next())
                .and_then(|n| n.parse().ok())
                .map(level::sectorballs::original)
                .unwrap_or_default(),
        };
        Ok(loaded)
    }

    /// An open-format level. One made before levels carried their settings has none: a remastered original
    /// (levels/remastered/Level_NN) takes them from the copy's table as before, any other plays with the defaults
    /// and says so.
    fn load_open_level(&self, dir: &Path) -> Result<LoadedLevel, String> {
        let mut loaded = self.load_open_table(dir)?;
        // the level.txt beside the level names its sky, music and light (it wins over the level file's own)
        let info = crate::package::info_of(dir);
        loaded.level.sky = info.sky.or(loaded.level.sky.take());
        loaded.level.settings.music = info.music.or(loaded.level.settings.music);
        loaded.level.settings.light = info.light.or(loaded.level.settings.light);
        Ok(loaded)
    }

    fn load_open_table(&self, dir: &Path) -> Result<LoadedLevel, String> {
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
                    sector_balls: level::sectorballs::original(k),
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
        self.texture_with(t, &crate::packs::chain(crate::packs::Kind::Textures))
    }

    /// `texture` with the chain of texture packs given.
    fn texture_with(
        &self,
        t: &level::Texture,
        chain: &[crate::packs::Pack],
    ) -> Option<(u32, u32, Vec<u8>)> {
        let Some((w, h, mut rgba, keyed)) = self.texture_pixels(t, chain) else {
            // a texture nobody has (a map made for the copy's textures, played without it) is drawn plain: said once
            if !matches!(t.source, level::TextureSource::None) {
                static SAID: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
                if let Ok(mut said) = SAID.lock()
                    && said.len() < 256
                    && !said.contains(&t.name)
                {
                    said.push(t.name.clone());
                    eprintln!("missing texture: {} (drawn plain)", t.name);
                }
            }
            return None;
        };
        // the texture's transparent colour: those texels get alpha 0 (SetAlphaForTransparentColor); a player's pack's
        // picture has real alpha instead
        if let Some(key) = t.color_key.filter(|_| keyed) {
            for [r, g, b, a] in rgba.as_chunks_mut::<4>().0 {
                if [*r, *g, *b] == key {
                    *a = 0;
                }
            }
        }
        Some((w, h, rgba))
    }

    /// A texture's pixels, and whether its transparent colour applies to them (not to a player's pack's picture).
    fn texture_pixels(
        &self,
        t: &level::Texture,
        chain: &[crate::packs::Pack],
    ) -> Option<(u32, u32, Vec<u8>, bool)> {
        let keyed = |(w, h, px): (u32, u32, Vec<u8>)| (w, h, px, true);
        match &t.source {
            level::TextureSource::Pixels {
                width,
                height,
                rgba,
            } => Some((*width, *height, rgba.clone(), true)),
            // an open-format level's texture: a plain file of its folder, its size checked before it is decoded
            level::TextureSource::LevelFile(path) => {
                // a plain file that stays inside the level's folder (no link on the way leads out of it)
                let plain = std::fs::symlink_metadata(path)
                    .is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX)
                    && within_level(path);
                if !plain || !self.overlay_pixels(path, false) {
                    return None;
                }
                decode_capped(path).map(keyed)
            }
            // a texture a .nmo names: only the last part of its name, looked for in the game's texture folders (a
            // stranger's map may name any path there; it never reads one)
            level::TextureSource::File(name) => {
                let file = bare_name(name)?;
                let (rel, path) = self.texture_file_at(&file)?;
                // the level's sky, particles and shadow (the game's parts, not a level's or the interface's
                // textures): a player's texture pack has its own of them first
                let own = self.overlay.as_deref().is_some_and(|o| path.starts_with(o));
                if !own
                    && crate::resources::is_part(&rel)
                    && let Some((w, h, px)) = self.player_pack_picture(&file, chain)
                {
                    return Some((w, h, px, false));
                }
                // it may come from a level's game folder (a stranger's): its size checked before it is decoded
                if !std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX) {
                    return None;
                }
                if !self.overlay_pixels(&path, false) {
                    return None;
                }
                decode_capped(&path).map(keyed)
            }
            level::TextureSource::Game(name) => self.game_texture(name, chain),
            level::TextureSource::None => None,
        }
    }

    /// The game's texture `name` (an open-format level's `game:` image): the level's own `game/Textures/` first (a
    /// level folder's picture of that name wins), then the texture packs' chain, then ours, then the copy's. The name
    /// is only compared with the files' names (resources::texture_key), never made a path.
    fn game_texture(
        &self,
        name: &str,
        chain: &[crate::packs::Pack],
    ) -> Option<(u32, u32, Vec<u8>, bool)> {
        use crate::packs::{ORIGINAL, OWN};
        // each candidate: its path, whether it is a player's pack's (budgeted, real alpha), whether it is the copy's
        // (its transparent colour applies)
        let level_own = self
            .overlay
            .as_deref()
            .and_then(|o| plain_path(o, "Textures"));
        let mut tried: Vec<PathBuf> = Vec::new();
        let copy = |p: Option<PathBuf>| p.map(|p| (p, false, true));
        let candidates = level_own
            .and_then(|d| crate::packs::picture(&d, name))
            .map(|p| (p, false, true))
            .into_iter()
            .chain(chain.iter().filter_map(|p| match (p.key.as_str(), &p.dir) {
                (_, Some(dir)) => crate::packs::picture(dir, name).map(|p| (p, true, false)),
                (OWN, None) => crate::resources::own_texture(name).map(|p| (p, false, false)),
                (ORIGINAL, None) => copy(self.copy_picture(name)),
                _ => None,
            }))
            .chain(crate::resources::own_texture(name).map(|p| (p, false, false)))
            .chain(copy(self.copy_picture(name)));
        for (path, pack, keyed) in candidates {
            if tried.contains(&path) {
                continue;
            }
            tried.push(path.clone());
            if let Some((w, h, px)) = self.decode_budgeted(&path, pack) {
                return Some((w, h, px, keyed));
            }
        }
        None
    }

    /// The first picture of `name` among the player's texture packs that come before any built-in pack in the
    /// chain, decoded (a broken one is passed over for the next).
    fn player_pack_picture(
        &self,
        name: &str,
        chain: &[crate::packs::Pack],
    ) -> Option<(u32, u32, Vec<u8>)> {
        chain
            .iter()
            .map_while(|p| p.dir.clone())
            .filter_map(|dir| crate::packs::picture(&dir, name))
            .find_map(|path| self.decode_budgeted(&path, true))
    }

    /// The copy's own picture of the texture `name`, found by its name as texture_key compares names (in its
    /// Textures/ folder, Sky/ and Particles/).
    fn copy_picture(&self, name: &str) -> Option<PathBuf> {
        let dir = self.copy_only("Textures")?;
        crate::packs::picture(&dir, name)
    }

    /// A picture decoded through the caps: a plain file of at most IMAGE_MAX, within the pixel budget when it is a
    /// level's, the toolbox's or a player's pack's (`pack`: it comes from a player's pack, wherever that is).
    fn decode_budgeted(&self, path: &Path, pack: bool) -> Option<(u32, u32, Vec<u8>)> {
        let plain =
            std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX);
        if !plain || !self.overlay_pixels(path, pack) {
            return None;
        }
        decode_capped(path)
    }
}

/// Where a part's physics (its own meshes) came from, for the model packs: a built-in pack that is the same source
/// adds no look.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PartFrom {
    /// Ours (assets/own).
    Own,
    /// The player's copy.
    Copy,
    /// A remastered level's own folder (the copy's part, remade by the toolbox).
    Remaster,
}

/// The largest level.gltf and level.bin of a part in a player's model pack, and the most vertices and triangles its
/// meshes (with their looks) may have in all.
const PACK_JSON_MAX: u64 = 8 * 1024 * 1024;
const PACK_BIN_MAX: u64 = 64 * 1024 * 1024;
pub const PACK_VERTICES_MAX: usize = 1 << 20;
pub const PACK_TRIANGLES_MAX: usize = 1 << 21;

/// A part of a player's model pack (`stem` as "3D Entities/PH/P_Box", found in the pack folder as "PH/P_Box"): its
/// folder a plain path of real folders, its files capped, read as any open-format level is, then checked (counts of
/// vertices and triangles, every number finite). Anything wrong leaves the part out (said once): the next pack in the
/// chain gives it.
fn load_pack_part(dir: &Path, stem: &str) -> Option<level::Level> {
    let sub = stem.strip_prefix("3D Entities/").unwrap_or(stem);
    let folder = plain_path(dir, sub)?;
    let folder = std::path::absolute(&folder).ok()?;
    let file = |name: &str, cap: u64| plain_path(&folder, name).and_then(|p| read_capped(&p, cap));
    let checked = file("level.gltf", PACK_JSON_MAX)
        .ok_or_else(|| format!("no level.gltf of at most {PACK_JSON_MAX} bytes"))
        .and_then(|gltf| {
            let bin = file("level.bin", PACK_BIN_MAX)
                .ok_or_else(|| format!("no level.bin of at most {PACK_BIN_MAX} bytes"))?;
            level::open::import_bytes(&folder, &gltf, &bin)
        })
        .and_then(|l| check_pack_part(&l).map(|()| l));
    match checked {
        Ok(l) => Some(l),
        Err(e) => {
            pack_warning(&folder, &e);
            None
        }
    }
}

/// A model pack's part is within the caps: at most PACK_VERTICES_MAX vertices and PACK_TRIANGLES_MAX triangles in all
/// (its meshes and their looks), every position, normal and texture coordinate finite.
pub fn check_pack_part(l: &level::Level) -> Result<(), String> {
    let (mut vertices, mut triangles) = (0usize, 0usize);
    let looks = l
        .meshes
        .iter()
        .flat_map(|m| std::iter::once(m).chain(m.look.as_deref()));
    for m in looks {
        vertices = vertices.saturating_add(m.positions.len());
        triangles = m
            .parts
            .iter()
            .fold(triangles, |t, p| t.saturating_add(p.indices.len() / 3));
        let finite = m
            .positions
            .iter()
            .chain(&m.normals)
            .all(|v| v.iter().all(|x| x.is_finite()))
            && m.uvs.iter().all(|v| v.iter().all(|x| x.is_finite()));
        if !finite {
            return Err(format!("mesh {}: a number that is not finite", m.name));
        }
    }
    if vertices > PACK_VERTICES_MAX || triangles > PACK_TRIANGLES_MAX {
        return Err(format!(
            "{vertices} vertices and {triangles} triangles, over the caps ({PACK_VERTICES_MAX}, {PACK_TRIANGLES_MAX})"
        ));
    }
    Ok(())
}

/// Says once that a model pack's part was left out.
fn pack_warning(folder: &Path, why: &str) {
    static SAID: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());
    if let Ok(mut said) = SAID.lock()
        && !said.iter().any(|s| s == folder)
    {
        said.push(folder.to_path_buf());
        eprintln!("warning: model pack {}: {why}; left out", folder.display());
    }
}

/// Puts the meshes of `look` (a part of a model pack) on the meshes of `base` (the part the game plays with) as their
/// looks: a base mesh takes the look of the pack's object of the same name as the first object placing it (moved into
/// that object's frame), else of the pack's mesh of the same name. The look's materials and textures are added to the
/// base's. Returns how many meshes took a look; with none, `base` is left as it was.
pub fn attach_looks(base: &mut level::Level, look: level::Level) -> usize {
    type Pair = (usize, usize, Option<[[f32; 4]; 4]>);
    let mut pairs: Vec<Pair> = Vec::new();
    for (bm, _) in base.meshes.iter().enumerate() {
        // every object placing the mesh must find the same look in the same place (a mesh shared by several objects
        // has one look); objects that disagree leave the mesh its own look
        let mut placed = base
            .objects
            .iter()
            .filter(|o| o.mesh == Some(bm))
            .peekable();
        let by_object = placed.peek().is_some().then(|| {
            let mut agreed: Option<(usize, [[f32; 4]; 4])> = None;
            for b in placed {
                let l = look
                    .objects
                    .iter()
                    .find(|o| o.mesh.is_some() && o.name.eq_ignore_ascii_case(&b.name))?;
                let this = (l.mesh?, relative_matrix(&b.matrix, &l.matrix)?);
                match agreed {
                    None => agreed = Some(this),
                    Some((m, r)) if m == this.0 && near(&r, &this.1) => {}
                    Some(_) => return None,
                }
            }
            agreed
        });
        let shared = base.objects.iter().filter(|o| o.mesh == Some(bm)).count() > 1;
        let by_object = by_object.flatten();
        // a shared mesh is never matched by its name alone (its objects found no common look)
        let shared_disagree = shared && by_object.is_none();
        let name = base
            .meshes
            .get(bm)
            .map(|m| m.name.as_str())
            .unwrap_or_default();
        let found = by_object.map(|(lm, m)| (lm, Some(m))).or_else(|| {
            if shared_disagree {
                return None;
            }
            look.meshes
                .iter()
                .position(|m| !name.is_empty() && m.name.eq_ignore_ascii_case(name))
                .map(|lm| (lm, None))
        });
        if let Some((lm, m)) = found {
            pairs.push((bm, lm, m));
        }
    }
    if pairs.is_empty() {
        return 0;
    }
    let (t0, m0) = (base.textures.len(), base.materials.len());
    // the looks made first: a part none of whose looks could be moved into place is left as it was
    let mut ready: Vec<(usize, level::Mesh)> = Vec::new();
    for (bm, lm, matrix) in pairs {
        let Some(src) = look.meshes.get(lm) else {
            continue;
        };
        let mut mesh = src.look.as_deref().unwrap_or(src).clone();
        mesh.look = None;
        for p in &mut mesh.parts {
            p.material = p.material.and_then(|i| i.checked_add(m0));
        }
        for c in &mut mesh.channels {
            c.material = c.material.saturating_add(m0);
        }
        if matrix.is_none_or(|m| transform_mesh(&mut mesh, &m)) {
            ready.push((bm, mesh));
        }
    }
    if ready.is_empty() {
        return 0;
    }
    base.textures.extend(look.textures.iter().cloned());
    base.materials
        .extend(look.materials.iter().cloned().map(|mut m| {
            m.texture = m.texture.and_then(|t| t.checked_add(t0));
            m.normal = m.normal.and_then(|t| t.checked_add(t0));
            m
        }));
    let mut n = 0usize;
    for (bm, mesh) in ready {
        if let Some(b) = base.meshes.get_mut(bm) {
            b.look = Some(Box::new(mesh));
            n = n.saturating_add(1);
        }
    }
    attach_dressing(base, &look, m0);
    n
}

/// The groups of a part whose members are drawn only, put together by the game where they fit (the sacks' supports:
/// `Supports_P_Modul_26`, crates/game/src/supports.rs).
pub const DRESSING: &str = "Supports_";

/// Brings the look's dressing (its `Supports_` groups: objects no rule and no physics knows, with their meshes) into
/// `base`, whose materials already hold the look's from `m0` on. The look's part is made on the base's frames, so
/// the objects keep their places. A group `base` has itself stays as it is.
fn attach_dressing(base: &mut level::Level, look: &level::Level, m0: usize) {
    for g in &look.groups {
        let is_dressing = g
            .name
            .get(..DRESSING.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(DRESSING));
        if !is_dressing || base.group(&g.name).is_some() {
            continue;
        }
        let mut members = Vec::new();
        for o in g.members.iter().filter_map(|&i| look.objects.get(i)) {
            let Some(src) = o.mesh.and_then(|m| look.meshes.get(m)) else {
                continue;
            };
            let mut mesh = src.look.as_deref().unwrap_or(src).clone();
            mesh.look = None;
            for p in &mut mesh.parts {
                p.material = p.material.and_then(|i| i.checked_add(m0));
            }
            for c in &mut mesh.channels {
                c.material = c.material.saturating_add(m0);
            }
            let mut object = o.clone();
            object.mesh = Some(base.meshes.len());
            object.visible = false;
            base.meshes.push(mesh);
            members.push(base.objects.len());
            base.objects.push(object);
        }
        base.groups.push(level::Group {
            name: g.name.clone(),
            members,
        });
    }
}

/// A matrix of the level model (column-major) as glam's.
fn mat(m: &[[f32; 4]; 4]) -> glam::Mat4 {
    glam::Mat4::from_cols_array_2d(m)
}

/// The matrix taking the look's object frame `look` into the base object's frame `base` (base⁻¹ · look; both affine),
/// none when the base's cannot be inverted or the result is not finite; the identity exactly when it is one within
/// float noise (no move needed).
fn relative_matrix(base: &[[f32; 4]; 4], look: &[[f32; 4]; 4]) -> Option<[[f32; 4]; 4]> {
    let inv = affine_inverse(base)?;
    let out = mat(&inv).mul_mat4(&mat(look));
    // a flattened move (the look's frame singular) would leave its normals unmoved: refused like the base's
    if !out.is_finite() || affine_inverse(&out.to_cols_array_2d()).is_none() {
        return None;
    }
    Some(if out.abs_diff_eq(glam::Mat4::IDENTITY, 1e-6) {
        IDENTITY
    } else {
        out.to_cols_array_2d()
    })
}

/// Two matrices the same within float noise.
fn near(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> bool {
    let (a, b) = (mat(a), mat(b));
    let scale = a
        .to_cols_array()
        .iter()
        .chain(&b.to_cols_array())
        .fold(1.0f32, |m, x| m.max(x.abs()));
    a.abs_diff_eq(b, 1e-4 * scale)
}

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// The inverse of an affine matrix, none when it is not affine, cannot be inverted (a flattened frame) or is not finite.
fn affine_inverse(m: &[[f32; 4]; 4]) -> Option<[[f32; 4]; 4]> {
    let m = glam::DMat4::from_cols_array_2d(&m.map(|c| c.map(f64::from)));
    let affine = m.row(3).abs_diff_eq(glam::DVec4::W, 1e-9);
    let det = m.determinant();
    if !affine || !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    let inv = m.inverse().as_mat4();
    inv.is_finite().then(|| inv.to_cols_array_2d())
}

/// Moves a mesh by an affine matrix: its positions, and its normals by the inverse transpose (normalised again).
/// False, the mesh left half moved, when the matrix cannot be inverted or a moved position is not finite (finite
/// numbers moved far enough overflow): the look is not taken.
fn transform_mesh(mesh: &mut level::Mesh, m: &[[f32; 4]; 4]) -> bool {
    if *m == IDENTITY {
        return true;
    }
    let Some(inv) = affine_inverse(m) else {
        return false;
    };
    let mm = mat(m);
    for p in &mut mesh.positions {
        let moved = mm.transform_point3(glam::Vec3::from(*p));
        if !moved.is_finite() {
            return false;
        }
        *p = moved.to_array();
    }
    let normal = glam::Mat3::from_mat4(mat(&inv)).transpose();
    for n in &mut mesh.normals {
        let t = normal.mul_vec3(glam::Vec3::from(*n));
        if let Some(t) = t.try_normalize() {
            *n = t.to_array();
        }
    }
    true
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
        self.texture_file_at(file).map(|(_, p)| p)
    }

    /// The same, with the path in the game's layout it was found at (`Textures/Sky/Sky_A_Front.bmp`).
    fn texture_file_at(&self, file: &str) -> Option<(String, PathBuf)> {
        ["Textures", "Textures/Sky", "Textures/Particles"]
            .iter()
            .find_map(|dir| {
                let rel = format!("{dir}/{file}");
                self.file(&rel).map(|p| (rel, p))
            })
    }

    /// Puts the chosen texture pack's pictures in place of the level's textures (packs.rs, `Kind::Textures`), as
    /// pixels, when the level is loaded (so the choice holds for everything drawn or exported from it). Which of a
    /// level's textures a pack may replace (by name):
    /// - a texture a `.nmo` names by file: unless the level's own folder brings the file (by the file's name, or the
    ///   texture's name when no such file is found at all);
    /// - pixels a `.nmo` carries: of the copy's levels and parts always; of a stranger's map only when they are the
    ///   copy's own texture of that name, pixel for pixel (a map's own pictures stay its own);
    /// - an open-format level's files: only in a remaster of the copy's level.
    ///
    /// The packs are asked in their chain's order: a player's pack and `own` give their picture of the name; `original`
    /// keeps a texture that is the copy's as it is (a remaster's file takes the copy's picture of its name); `remastered`
    /// keeps a remaster's own file. A pack without the name, or whose picture does not decode, passes to the next; the
    /// end of the chain keeps the level's own. A pack's picture has real alpha: the transparent colour is dropped.
    /// The game's textures by name (`game:` images) are resolved when drawn (`game_texture`).
    fn put_pack_textures(&self, level: &mut level::Level, maker: Maker) {
        self.put_textures_of(
            level,
            maker,
            &crate::packs::chain(crate::packs::Kind::Textures),
        );
    }

    /// `put_pack_textures` with the chain of texture packs given.
    fn put_textures_of(
        &self,
        level: &mut level::Level,
        maker: Maker,
        chain: &[crate::packs::Pack],
    ) {
        use crate::packs::{ORIGINAL, OWN, REMASTERED};
        dedupe_game_textures(level);
        // the copy's textures as they are: nothing to do (the classic look of the copy, and the toolbox's default)
        if chain.first().is_none_or(|p| p.key == ORIGINAL) && !matches!(maker, Maker::Copy) {
            return;
        }
        for t in &mut level.textures {
            // the names a pack's picture is looked for under, or none when the texture is not one a pack replaces
            let names: Vec<String> = match &t.source {
                level::TextureSource::File(name) => match bare_name(name) {
                    Some(file) => match self.texture_file(&file) {
                        Some(found)
                            if self
                                .overlay
                                .as_deref()
                                .is_some_and(|o| found.starts_with(o)) =>
                        {
                            Vec::new()
                        }
                        Some(_) => vec![file],
                        None => vec![file, t.name.clone()],
                    },
                    None => Vec::new(),
                },
                level::TextureSource::Pixels { .. } => vec![t.name.clone()],
                level::TextureSource::LevelFile(_) if maker == Maker::Copy => vec![t.name.clone()],
                // a stranger's texture that names no file and carries no pixels: the pack's picture of its name, if any
                level::TextureSource::None if maker == Maker::Stranger && !t.name.is_empty() => {
                    vec![t.name.clone()]
                }
                level::TextureSource::LevelFile(_)
                | level::TextureSource::Game(_)
                | level::TextureSource::None => Vec::new(),
            };
            if names.is_empty() {
                continue;
            }
            let remaster_file = matches!(t.source, level::TextureSource::LevelFile(_));
            // a stranger's pixels are replaced only when they are the copy's: asked once, only when a pack has a picture
            let mut copys: Option<bool> = None;
            let mut replaced = None;
            for pack in chain {
                // a player's pack's pictures are budgeted wherever the pack is; the copy's picture keeps the texture's
                // transparent colour (a remaster's texture carries the copy's)
                let from_pack = pack.dir.is_some();
                let keep_key = pack.key == ORIGINAL && pack.dir.is_none();
                let path = match (pack.key.as_str(), &pack.dir) {
                    (_, Some(dir)) => names.iter().find_map(|n| crate::packs::picture(dir, n)),
                    (OWN, None) => names.iter().find_map(|n| crate::resources::own_texture(n)),
                    (ORIGINAL, None) if remaster_file => self.copy_picture(&t.name),
                    // the copy's texture: kept as it is
                    (ORIGINAL, None) => break,
                    (REMASTERED, None) if remaster_file => break,
                    _ => None,
                };
                let Some(path) = path else { continue };
                if let level::TextureSource::Pixels {
                    width,
                    height,
                    rgba,
                } = &t.source
                    && maker == Maker::Stranger
                    && !*copys.get_or_insert_with(|| {
                        self.is_copys_texture(&t.name, *width, *height, rgba)
                    })
                {
                    break;
                }
                if let Some(px) = self.decode_budgeted(&path, from_pack)
                    && self.charge_stored(&path, from_pack, px.0, px.1)
                {
                    replaced = Some((px, keep_key));
                    break;
                }
            }
            if let Some(((width, height, rgba), keep_key)) = replaced {
                t.source = level::TextureSource::Pixels {
                    width,
                    height,
                    rgba,
                };
                if !keep_key {
                    t.color_key = None;
                }
            }
        }
    }

    /// Whether pixels a map carries under `name` are the copy's own texture of that name (its file in Textures/,
    /// decoded; compared by size and hash, each of the copy's files decoded once).
    fn is_copys_texture(&self, name: &str, width: u32, height: u32, rgba: &[u8]) -> bool {
        type Seen = std::collections::HashMap<PathBuf, Option<(u32, u32, u64)>>;
        static SEEN: std::sync::Mutex<Option<Seen>> = std::sync::Mutex::new(None);
        let Some(file) = bare_name(name) else {
            return false;
        };
        let has_ext = file.rsplit_once('.').is_some_and(|(_, e)| e.len() <= 4);
        let names = if has_ext {
            vec![file]
        } else {
            vec![format!("{file}.bmp"), format!("{file}.tga"), file]
        };
        let Some(path) = names.iter().find_map(|n| self.copy_has_texture(n)) else {
            return false;
        };
        let Ok(mut seen) = SEEN.lock() else {
            return false;
        };
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
        std::fs::metadata(&path)
            .is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX)
            .then_some(path)
    }
}

impl Data {
    /// A texture from the level's own game files still fits the budget of their pixels (read from its header, then
    /// counted as taken); any other texture always does.
    fn overlay_pixels(&self, path: &Path, pack: bool) -> bool {
        if !self.budgeted(path, pack) {
            return true;
        }
        if OVERLAY_COUNTED
            .lock()
            .is_ok_and(|c| c.iter().any(|p| p == path))
        {
            return true;
        }
        let Some((w, h)) = image::ImageReader::open(path)
            .ok()
            .and_then(|r| r.with_guessed_format().ok())
            .and_then(|r| r.into_dimensions().ok())
        else {
            return false;
        };
        if !charge(
            u64::from(w)
                .saturating_mul(u64::from(h))
                .saturating_mul(RGBA),
        ) {
            return false;
        }
        if let Ok(mut counted) = OVERLAY_COUNTED.lock() {
            counted.push(path.to_path_buf());
        }
        true
    }

    /// The files of a level's own folder, of the toolbox's interface and of the player's packs (`pack`, or by where
    /// they are) share the budget; the copy's and ours do not.
    fn budgeted(&self, path: &Path, pack: bool) -> bool {
        pack || self.overlay.as_deref().is_some_and(|o| path.starts_with(o))
            || path.starts_with(crate::resources::remastered_root())
            || crate::packs::in_player_packs(path)
    }

    /// A budgeted picture decoded as `width` × `height` is kept as one more texture's pixels: the first copy was
    /// counted when it was decoded, each further one is charged again (one picture a pack has for one name may stand in
    /// for any number of the level's and its parts' textures of that name, each keeping a copy). False: over the budget,
    /// the texture keeps its own.
    fn charge_stored(&self, path: &Path, pack: bool, width: u32, height: u32) -> bool {
        if !self.budgeted(path, pack) {
            return true;
        }
        let Ok(mut stored) = STORED.lock() else {
            return false;
        };
        if !stored.iter().any(|p| p == path) {
            stored.push(path.to_path_buf());
            return true;
        }
        charge(
            u64::from(width)
                .saturating_mul(u64::from(height))
                .saturating_mul(RGBA),
        )
    }
}

/// Takes `n` bytes from the level's budget, if they fit.
fn charge(n: u64) -> bool {
    use std::sync::atomic::Ordering;
    let mut taken = OVERLAY_BYTES.load(Ordering::Relaxed);
    loop {
        let Some(next) = taken.checked_add(n).filter(|&t| t <= OVERLAY_BYTES_MAX) else {
            return false;
        };
        match OVERLAY_BYTES.compare_exchange_weak(taken, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return true,
            Err(now) => taken = now,
        }
    }
}

/// What one more texture may take of the level's budget: a share of what is left.
fn fair_share() -> u64 {
    OVERLAY_BYTES_MAX.saturating_sub(OVERLAY_BYTES.load(std::sync::atomic::Ordering::Relaxed))
        / SHARE
}

/// A level's own file stays inside the level: `path` as the system resolves it (every link on the way followed)
/// lies under its level's folder resolved the same way (the folder above the `textures/` it is named in). A link
/// anywhere below that folder which leads out of it is refused; the level's folder itself may be reached by one.
fn within_level(path: &Path) -> bool {
    let Some(level) = path.parent().and_then(Path::parent) else {
        return false;
    };
    match (std::fs::canonicalize(level), std::fs::canonicalize(path)) {
        (Ok(level), Ok(file)) => file.starts_with(&level) && file != level,
        _ => false,
    }
}

/// A picture file's size from its header.
fn picture_size(path: &Path) -> Option<(u32, u32)> {
    image::ImageReader::open(path)
        .ok()
        .and_then(|r| r.with_guessed_format().ok())
        .and_then(|r| r.into_dimensions().ok())
}

/// What a picture takes halved `shrink` times, as RGBA.
fn shrunk_bytes(w: u32, h: u32, shrink: u32) -> u64 {
    let side = |s: u32| u64::from(s.checked_shr(shrink).unwrap_or(0).max(1));
    side(w).saturating_mul(side(h)).saturating_mul(RGBA)
}

/// A level's own picture file planned: decoded when loaded, its transparent colour made alpha 0, then halved
/// `shrink` times.
pub struct PicturePlan {
    path: PathBuf,
    key: Option<[u8; 3]>,
    shrink: u32,
}

/// A texture chosen and counted, not yet read: `texture_plan` makes the plans one after another (the budget is
/// taken in the level's order, so a level looks the same each time), `load` reads them, on any thread.
pub enum TexturePlan {
    /// Pixels already there (a small picture of the copy, of ours or of a pack).
    Pixels(u32, u32, Vec<u8>),
    Picture(PicturePlan),
    /// A level's compressed texture, read without its `skip` largest levels; `decode`: as RGBA (no BC on the card).
    /// The file read must be the one planned (`seen`: the plain file looked at; `header`: what it said). When it
    /// turns out damaged, the picture beside it is drawn instead (`fallback`: no larger than what was counted).
    Compressed {
        path: PathBuf,
        seen: std::fs::Metadata,
        header: crate::gputex::Header,
        skip: usize,
        decode: bool,
        fallback: Option<PicturePlan>,
    },
}

impl PicturePlan {
    fn load(self) -> Option<crate::gputex::TextureData> {
        let (mut w, mut h, mut rgba) = decode_capped(&self.path)?;
        if let Some(key) = self.key {
            for [r, g, b, a] in rgba.as_chunks_mut::<4>().0 {
                if [*r, *g, *b] == key {
                    *a = 0;
                }
            }
        }
        for _ in 0..self.shrink {
            if w <= 1 && h <= 1 {
                break;
            }
            rgba = crate::gputex::halve(w, h, &rgba)?;
            (w, h) = ((w / 2).max(1), (h / 2).max(1));
        }
        crate::gputex::TextureData::from_rgba(w, h, rgba)
    }
}

/// The compressed texture planned, read: the very file looked at (never through a link put there since), its header
/// still what the plan was made from, the whole of it checked.
fn load_compressed(
    path: &Path,
    seen: &std::fs::Metadata,
    header: &crate::gputex::Header,
    skip: usize,
    decode: bool,
) -> Result<crate::gputex::TextureData, String> {
    use std::io::Read;
    let cap = crate::gputex::FILE_MAX;
    let mut bytes = Vec::new();
    crate::package::open_plain(path, seen)
        .and_then(|f| f.take(cap.saturating_add(1)).read_to_end(&mut bytes))
        .map_err(|e| e.to_string())?;
    if u64::try_from(bytes.len()).map_or(true, |n| n > cap) {
        return Err("too big".into());
    }
    if crate::gputex::header(&bytes)? != *header {
        return Err("changed since it was looked at".into());
    }
    let t = crate::gputex::texture(bytes, skip)?;
    if decode {
        crate::gputex::decoded(&t).ok_or_else(|| "does not decode".to_owned())
    } else {
        Ok(t)
    }
}

impl TexturePlan {
    /// The texture with its mip levels, as the renderer uploads it.
    pub fn load(self) -> Option<crate::gputex::TextureData> {
        match self {
            TexturePlan::Pixels(w, h, rgba) => crate::gputex::TextureData::from_rgba(w, h, rgba),
            TexturePlan::Picture(p) => p.load(),
            TexturePlan::Compressed {
                path,
                seen,
                header,
                skip,
                decode,
                fallback,
            } => match load_compressed(&path, &seen, &header, skip, decode) {
                Ok(t) => Some(t),
                Err(e) => {
                    let instead = if fallback.is_some() {
                        "its picture is drawn instead, smaller"
                    } else {
                        "drawn plain"
                    };
                    eprintln!(
                        "{}: {e} ({instead}; remaster the level again)",
                        path.display()
                    );
                    fallback.and_then(PicturePlan::load)
                }
            },
        }
    }
}

/// What tells one compressed texture from another when the level's budget counts them: the data's hash as the header
/// gives it (not yet checked, and a hash anyone can forge), the format, the size and the file's length. Files alike
/// in all of it are counted once: the renderer keeps one texture for them, whatever each really holds.
type CompressedKey = (u64, crate::gputex::Format, u32, u32, u64);

impl Data {
    /// The plan for a texture the renderer is about to load; `bc`: the card draws BC-compressed textures. A level's
    /// own picture (an open-format level's file) with a compressed texture beside it (`name.dtx` by `name.png`, the
    /// toolbox's) is planned as that; any other texture is its pixels as `texture` gives them.
    pub fn texture_plan(&self, t: &level::Texture, bc: bool) -> Option<TexturePlan> {
        let level::TextureSource::LevelFile(path) = &t.source else {
            return self
                .texture(t)
                .map(|(w, h, rgba)| TexturePlan::Pixels(w, h, rgba));
        };
        let plan = self.level_file_plan(path, t.color_key, bc);
        if plan.is_none() {
            eprintln!("missing texture: {} (drawn plain)", t.name);
        }
        plan
    }

    fn level_file_plan(&self, path: &Path, key: Option<[u8; 3]>, bc: bool) -> Option<TexturePlan> {
        // a plain file that stays inside the level's folder (no link on the way leads out of it)
        let picture = std::fs::symlink_metadata(path)
            .ok()
            .filter(|m| m.is_file() && m.len() <= IMAGE_MAX && within_level(path))
            .map(|m| m.len());
        let size = picture.and_then(|_| picture_size(path));
        let picture_plan = |shrink: u32| PicturePlan {
            path: path.to_path_buf(),
            key,
            shrink,
        };
        if let Some(plan) = self.compressed_plan(
            &path.with_extension("dtx"),
            picture,
            size,
            bc,
            &picture_plan,
        ) {
            return Some(plan);
        }
        picture?;
        // a picture of the level's game files over its share of the budget is drawn smaller
        let mut shrink = 0u32;
        if self.budgeted(path, false) {
            let (w, h) = size?;
            let share = fair_share();
            while shrunk_bytes(w, h, shrink) > share
                && shrink < 16
                && shrunk_bytes(w, h, shrink) > RGBA
            {
                shrink = shrink.saturating_add(1);
            }
            if !charge(shrunk_bytes(w, h, shrink)) {
                return None;
            }
        }
        Some(TexturePlan::Picture(picture_plan(shrink)))
    }

    /// The compressed texture at `path`, if it is one made from the picture beside it (`picture`: that file's
    /// length, `size`: its sides; none: the level carries only the compressed one): its header read and checked, the
    /// levels chosen that fit its share of the budget (a level's own game files) or the adapter (no BC: decoded, at
    /// most DECODED_SIDE_MAX a side; a texture that cannot be made that small is left to its picture).
    fn compressed_plan(
        &self,
        path: &Path,
        picture: Option<u64>,
        size: Option<(u32, u32)>,
        bc: bool,
        picture_plan: &dyn Fn(u32) -> PicturePlan,
    ) -> Option<TexturePlan> {
        use std::io::Read;
        let seen = std::fs::symlink_metadata(path)
            .ok()
            .filter(|m| m.is_file() && m.len() <= crate::gputex::FILE_MAX && within_level(path))?;
        let mut head = [0u8; crate::gputex::HEADER_LEN];
        crate::package::open_plain(path, &seen)
            .ok()?
            .read_exact(&mut head)
            .ok()?;
        let h = crate::gputex::header(&head).ok()?;
        if h.file_len() != seen.len() || picture.is_some_and(|p| p != h.source_len) {
            return None;
        }
        let droppable = h.droppable();
        let mut skip = 0usize;
        let side = |skip: usize| h.mips.get(skip).map_or(0, |m| m.width.max(m.height));
        // what the levels from `skip` on take: their blocks, or their pixels when decoded
        let bytes = |skip: usize| -> u64 {
            if bc {
                u64::try_from(h.bytes_from(skip)).unwrap_or(u64::MAX)
            } else {
                h.mips.iter().skip(skip).fold(0u64, |n, m| {
                    n.saturating_add(
                        u64::from(m.width)
                            .saturating_mul(u64::from(m.height))
                            .saturating_mul(RGBA),
                    )
                })
            }
        };
        while !bc && skip < droppable && side(skip) > DECODED_SIDE_MAX {
            skip = skip.saturating_add(1);
        }
        if !bc && side(skip) > DECODED_SIDE_MAX {
            return None;
        }
        // the picture drawn should the data turn out damaged: halved until it takes no more than what is counted
        // here for the compressed texture
        let fallback = |skip: usize| {
            let (w, h) = size?;
            let shrink = (0..16u32).find(|s| shrunk_bytes(w, h, *s) <= bytes(skip).max(RGBA))?;
            Some(picture_plan(shrink))
        };
        let mut instead = None;
        if self.budgeted(path, false) {
            let key: CompressedKey = (h.data_hash, h.format, h.width, h.height, seen.len());
            let known = OVERLAY_HASHES
                .lock()
                .ok()
                .and_then(|k| k.iter().find(|(k, _)| *k == key).map(|(_, s)| *s));
            match known {
                // the same texture again: drawn as the first of it is, and the renderer keeps one for both (no
                // picture instead: nothing was counted for this one)
                Some(first) => skip = first,
                None => {
                    let share = fair_share();
                    while skip < droppable && bytes(skip) > share {
                        skip = skip.saturating_add(1);
                    }
                    if !charge(bytes(skip)) {
                        return None;
                    }
                    if let Ok(mut k) = OVERLAY_HASHES.lock() {
                        k.push((key, skip));
                    }
                    instead = fallback(skip);
                }
            }
        } else {
            instead = fallback(skip);
        }
        Some(TexturePlan::Compressed {
            path: path.to_path_buf(),
            seen,
            header: h,
            skip,
            decode: !bc,
            fallback: instead,
        })
    }
}

/// A stranger's `.nmo` map's textures that carry no picture and name no file, where the name says which of the
/// game's textures is meant (resources::meant_texture): made the game's texture of that name, found like any other
/// (the texture packs, ours, the copy's). The others stay without a picture.
fn name_meant_textures(level: &mut level::Level) {
    // the materials drawing with each texture, gathered in one pass (a hostile map may hold very many of both);
    // a few names say all there is to say
    const NAMES: usize = 8;
    let mut users: Vec<Vec<&str>> = vec![Vec::new(); level.textures.len()];
    for m in &level.materials {
        if let Some(u) = m.texture.and_then(|t| users.get_mut(t))
            && u.len() < NAMES
        {
            u.push(m.name.as_str());
        }
    }
    let meant: Vec<Option<&'static str>> = level
        .textures
        .iter()
        .zip(&users)
        .map(|(t, users)| {
            let bare = matches!(t.source, level::TextureSource::None) && !users.is_empty();
            bare.then(|| crate::resources::meant_texture(&t.name, users))
                .flatten()
        })
        .collect();
    for (t, name) in level.textures.iter_mut().zip(meant) {
        if let Some(name) = name {
            t.source = level::TextureSource::Game(name.to_owned());
        }
    }
}

/// Points every material at the first of the level's `game:` textures of each name (as texture names compare), so a
/// level naming one game texture many times draws it once (the others are left in the list, unused, and kept as
/// they are by an export).
fn dedupe_game_textures(level: &mut level::Level) {
    let mut first: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut to: Vec<Option<usize>> = Vec::with_capacity(level.textures.len());
    for (i, t) in level.textures.iter().enumerate() {
        to.push(match &t.source {
            level::TextureSource::Game(name) => {
                let k = *first
                    .entry(crate::resources::texture_key(name))
                    .or_insert(i);
                (k != i).then_some(k)
            }
            _ => None,
        });
    }
    if to.iter().all(Option::is_none) {
        return;
    }
    let map = |t: &mut Option<usize>| {
        if let Some(k) = t.and_then(|i| to.get(i).copied().flatten()) {
            *t = Some(k);
        }
    };
    for m in &mut level.materials {
        map(&mut m.texture);
        map(&mut m.normal);
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
        nmo::scene::parameter_floats(doc.get(index)?.chunk.as_ref()?)
            .ok()
            .flatten()
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
        let own = table.rows.iter().position(|r| {
            // a level file's name, or an export's folder named after it (Level_13 for Level_13.NMO)
            text(r, "Levelfile").is_some_and(|f| {
                f.eq_ignore_ascii_case(file_name)
                    || Path::new(&f)
                        .file_stem()
                        .is_some_and(|s| s.eq_ignore_ascii_case(file_name))
            })
        });
        let last = own.is_some_and(|i| i.checked_add(1) == Some(table.rows.len()));
        let Some(row) = own
            .and_then(|i| table.rows.get(i))
            .or_else(|| table.rows.first())
        else {
            return none;
        };
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
                Some(nmo::scene::Cell::Object(Some(p))) => self
                    .parameter_floats("3D Entities/Levelinit.nmo", *p)
                    .and_then(|v| light_of(&v)),
                _ => None,
            },
        }
    }
}

/// A copy counts when it has the first level, not only the folder: a half-made import is not a game.
fn playable(root: &Path) -> bool {
    ["3D Entities", "Level", "Level_01.NMO"]
        .iter()
        .try_fold(root.to_path_buf(), |d, p| find(&d, p))
        .is_some()
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
            .then(|| {
                plain_path(&root, &format!("{stem}.png"))
                    .or_else(|| plain_path(&root, &format!("{stem}.jpg")))
            })
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
pub fn export(
    data: &Data,
    loaded: &LoadedLevel,
    dir: &Path,
    with_game_files: bool,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let missing = export_level(data, &loaded.level, dir)?;
    if with_game_files {
        let game = dir.join(GAME_FILES);
        let mut parts = 0usize;
        for rel in parts_used(data, &loaded.level) {
            let Some(part) = data.part(&rel) else {
                continue;
            };
            let stem = rel
                .strip_suffix(".nmo")
                .or_else(|| rel.strip_suffix(".NMO"))
                .unwrap_or(&rel);
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
    }
    let unique = std::fs::read_dir(dir.join("textures"))
        .map(|d| d.flatten().count())
        .unwrap_or(0);
    println!(
        "{} written: {} objects, {} meshes, {} materials, {} textures ({unique} pictures written, {} not found), {} groups",
        dir.display(),
        loaded.level.objects.len(),
        loaded.level.meshes.len(),
        loaded.level.materials.len(),
        loaded.level.textures.len(),
        missing.len(),
        loaded.level.groups.len()
    );
    if !missing.is_empty() {
        println!("not found: {}", missing.join(", "));
    }
    Ok(())
}

/// Writes one level (or part) in the open format with its textures; returns the names of the textures not found.
/// Textures with the same picture (and the same transparent colour) are written once and shared by the materials
/// that use them: a map may carry thousands of texture objects of a few dozen pictures.
fn export_level(data: &Data, level: &level::Level, dir: &Path) -> Result<Vec<String>, String> {
    // the level's files are made anew: one there is removed first (a link or a hard link would be written through)
    for name in ["level.gltf", "level.bin"] {
        fresh(&dir.join(name))?;
    }
    real_folders(dir, &dir.join("textures"))?;
    // read first: the texture may be this very file (a level exported into its own folder); a texture not found
    // leaves what is there
    type Key = (u32, u32, u64, Option<[u8; 3]>);
    type Picture = (u32, u32, Vec<u8>);
    let mut keep: Vec<level::Texture> = Vec::new();
    let mut pictures: Vec<Option<Picture>> = Vec::new();
    let mut to: Vec<usize> = Vec::with_capacity(level.textures.len());
    let mut seen: std::collections::HashMap<Key, Vec<usize>> = std::collections::HashMap::new();
    let mut by_name: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut missing: Vec<String> = Vec::new();
    for t in &level.textures {
        // the game's texture by name stays a name (level.gltf says `game:<name>`): no file of the level's
        let named = match &t.source {
            level::TextureSource::Game(n) => Some(format!("game:{n}")),
            _ => None,
        };
        let found = if named.is_some() {
            None
        } else {
            data.texture(t)
        };
        let add = |keep: &mut Vec<level::Texture>,
                   pictures: &mut Vec<Option<Picture>>,
                   pic: Option<Picture>| {
            keep.push(level::Texture {
                name: t.name.clone(),
                source: match &t.source {
                    level::TextureSource::Game(_) => t.source.clone(),
                    _ => level::TextureSource::None,
                },
                color_key: t.color_key,
            });
            pictures.push(pic);
            keep.len() - 1
        };
        let idx = match (named, found) {
            (Some(n), _) => match by_name.get(&n) {
                Some(&k) => k,
                None => {
                    let k = add(&mut keep, &mut pictures, None);
                    by_name.insert(n, k);
                    k
                }
            },
            (None, Some((w, h, rgba))) => {
                let key: Key = (w, h, crate::fnv1a(&rgba), t.color_key);
                let same = seen.get(&key).and_then(|c| {
                    c.iter().copied().find(|&k| {
                        pictures
                            .get(k)
                            .and_then(Option::as_ref)
                            .is_some_and(|p| p.2 == rgba)
                    })
                });
                match same {
                    Some(k) => k,
                    None => {
                        let k = add(&mut keep, &mut pictures, Some((w, h, rgba)));
                        seen.entry(key).or_default().push(k);
                        k
                    }
                }
            }
            (None, None) => {
                let n = format!("missing:{}", t.name);
                match by_name.get(&n) {
                    Some(&k) => k,
                    None => {
                        missing.push(t.name.clone());
                        let k = add(&mut keep, &mut pictures, None);
                        by_name.insert(n, k);
                        k
                    }
                }
            }
        };
        to.push(idx);
    }
    level::open::export_textures(level, dir, &keep, &to)?;
    // pictures of an earlier export into this folder (numbered as ours are) would stay as stale files: all are read
    // by now, so they go
    let old: Vec<PathBuf> = std::fs::read_dir(dir.join("textures"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.ends_with(".png")
                && n.get(..5).is_some_and(|h| {
                    h.ends_with('_') && h.bytes().take(4).all(|b| b.is_ascii_digit())
                })
        })
        .map(|e| e.path())
        .take(65536)
        .collect();
    for p in old {
        fresh(&p)?;
    }
    for (i, (t, pic)) in keep.iter().zip(&pictures).enumerate() {
        if let Some((w, h, rgba)) = pic {
            let path = dir.join(level::open::texture_file(i, &t.name));
            fresh(&path)?;
            image::save_buffer(&path, rgba, *w, *h, image::ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())?;
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
    let rest = dir
        .strip_prefix(root)
        .map_err(|_| format!("{}: outside {}", dir.display(), root.display()))?;
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
    let c = |i: usize| {
        v.get(i)
            .copied()
            .filter(|x| x.is_finite())
            .map(|x| x.clamp(0.0, 1.0))
    };
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
    let mut out = vec![
        "3D Entities/Balls.nmo".to_owned(),
        "3D Entities/AnimTrafo.nmo".to_owned(),
    ];
    let Some(ph) = data.file("3D Entities/PH") else {
        return out;
    };
    let Ok(listed) = std::fs::read_dir(&ph) else {
        return out;
    };
    let mut names: Vec<String> = listed
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|n| n.to_ascii_lowercase().ends_with(".nmo"))
        .filter(|n| {
            let stem = n.get(..n.len().saturating_sub(4)).unwrap_or_default();
            (0..level.objects.len()).any(|i| level.module_of(i).eq_ignore_ascii_case(stem))
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
        let Some(dir) = data.file(rel) else {
            return Vec::new();
        };
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
    out.extend(
        listed("Textures/Particles")
            .into_iter()
            .map(|n| format!("Textures/Particles/{n}")),
    );
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
                let interface = l.starts_with("menu_")
                    || l.starts_with("atari")
                    || l.starts_with("music_highscore");
                let other_theme = l.starts_with("music_theme_")
                    && theme.as_ref().is_none_or(|t| !l.starts_with(t));
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
    relative
        .split('/')
        .try_fold(root.to_path_buf(), |dir, part| {
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
        .and_then(|f| {
            f.take(GAME_FILE_MAX.saturating_add(1))
                .read_to_end(&mut bytes)
        })
        .map_err(|e| format!("{}: {e}", src.display()))?;
    if u64::try_from(bytes.len()).map_or(true, |n| n > GAME_FILE_MAX) {
        return Err(format!("{}: too big", src.display()));
    }
    if !std::fs::symlink_metadata(root).is_ok_and(|m| m.is_dir()) {
        return Err(format!("{}: not a folder", root.display()));
    }
    let parts: Vec<&str> = relative.split('/').collect();
    let Some((name, folders)) = parts.split_last() else {
        return Err("no file name".into());
    };
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
        f.write_all(&bytes)
            .map_err(|e| format!("{}: {e}", tmp.display()))?;
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
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-copytex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Textures")).unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([9, 8, 7, 255]))
            .save(root.join("Textures/Wood_Old.bmp"))
            .unwrap();
        let data = Data::new(Some(root.clone()));
        let same = [9u8, 8, 7, 255].repeat(4);
        let other = [9u8, 8, 6, 255].repeat(4);
        assert!(
            data.is_copys_texture("Wood_Old", 2, 2, &same),
            "by the name without its extension"
        );
        assert!(
            data.is_copys_texture("wood_old.BMP", 2, 2, &same),
            "any case"
        );
        assert!(
            !data.is_copys_texture("Wood_Old", 2, 2, &other),
            "a map's own picture under the name"
        );
        assert!(
            !data.is_copys_texture("Wood_Old", 1, 4, &same),
            "another size"
        );
        assert!(
            !data.is_copys_texture("Brick", 2, 2, &same),
            "a name the copy lacks"
        );
        assert!(!data.is_copys_texture("../Textures/Wood_Old.bmp/..", 2, 2, &same));
        assert_eq!(bare_name("C:\\maps\\x.bmp").as_deref(), Some("x.bmp"));
        assert_eq!(bare_name("/a/b/../x.png").as_deref(), Some("x.png"));
        assert_eq!(bare_name("a/.."), None);
        assert_eq!(bare_name("x:y.bmp"), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn player_pack(key: &str, dir: &Path, fallback: Option<&str>) -> crate::packs::Pack {
        crate::packs::Pack {
            key: key.into(),
            dir: Some(dir.to_path_buf()),
            names: Vec::new(),
            fallback: fallback.map(str::to_owned),
        }
    }

    fn builtin(key: &str) -> crate::packs::Pack {
        crate::packs::Pack {
            key: key.into(),
            dir: None,
            names: Vec::new(),
            fallback: None,
        }
    }

    fn pixels(name: &str, rgba: [u8; 4]) -> level::Texture {
        level::Texture {
            name: name.into(),
            source: level::TextureSource::Pixels {
                width: 1,
                height: 1,
                rgba: rgba.to_vec(),
            },
            color_key: Some([0, 0, 0]),
        }
    }

    fn first_pixel(t: &level::Texture) -> Option<[u8; 4]> {
        match &t.source {
            level::TextureSource::Pixels { rgba, .. } => {
                rgba.get(..4).and_then(|p| p.try_into().ok())
            }
            _ => None,
        }
    }

    /// The tests that take from the level's pixel budget (a global, as the game has one level at a time) run one at a
    /// time.
    static BUDGET: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn budget_lock() -> std::sync::MutexGuard<'static, ()> {
        BUDGET
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Found by review: one pack picture standing in for many textures kept a copy for each, beyond the budget. Each
    /// further copy is charged; over the budget the textures keep their own.
    #[test]
    fn each_copy_of_a_pack_picture_is_charged() {
        let _lock = budget_lock();
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-texpack-uses-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]))
            .save(root.join("Brick.png"))
            .unwrap();
        let data = Data::new(None).for_level(None);
        // room for three copies of the 2 × 2 picture: the first counted when decoded, two more as copies
        OVERLAY_BYTES.store(
            OVERLAY_BYTES_MAX - 12 * RGBA,
            std::sync::atomic::Ordering::Relaxed,
        );
        let mut level = level::Level {
            textures: (0..6).map(|_| pixels("Brick", [9, 9, 9, 255])).collect(),
            ..Default::default()
        };
        data.put_textures_of(
            &mut level,
            Maker::Copy,
            &[player_pack("p", &root, Some("none"))],
        );
        let replaced = level
            .textures
            .iter()
            .filter(|t| first_pixel(t) == Some([10, 20, 30, 255]))
            .count();
        assert_eq!(replaced, 3);
        let _ = Data::new(None).for_level(None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The owner's remastered level drew its boxes plain white: the parts' 4K pictures were past the budget. A
    /// texture over its share is drawn smaller instead, and a compressed one beside a picture is taken first, counted
    /// once however many parts carry it, and refused when it is not that picture's.
    #[test]
    fn a_level_texture_over_its_share_is_drawn_smaller_never_plain() {
        use crate::gputex::{Format, write};
        let _lock = budget_lock();
        let root = std::env::temp_dir().join(format!("drahtseilakt-plan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let textures = root.join(GAME_FILES).join("textures");
        std::fs::create_dir_all(&textures).unwrap();
        let png = textures.join("0000_a.png");
        image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 0, 255, 255]))
            .save(&png)
            .unwrap();
        let data = Data::new(None).for_level(Some(&root));
        let t = level::Texture {
            name: "a".into(),
            source: level::TextureSource::LevelFile(png.clone()),
            color_key: Some([255, 0, 255]),
        };
        let left = |bytes: u64| {
            OVERLAY_BYTES.store(
                OVERLAY_BYTES_MAX - bytes,
                std::sync::atomic::Ordering::Relaxed,
            )
        };
        // room for it eight times over: whole; for a 16th of that: halved twice, keyed
        left(64 * 64 * 4 * SHARE);
        let whole = data.texture_plan(&t, true).unwrap().load().unwrap();
        assert_eq!(whole.size(), (64, 64));
        left(64 * 64 * 4 * SHARE / 16);
        let small = data.texture_plan(&t, true).unwrap().load().unwrap();
        assert_eq!(small.size(), (16, 16));
        assert_eq!(small.data.get(..4), Some(&[255, 0, 255, 0][..]));
        // a compressed texture of that picture beside it: taken, whole while it fits, without its top level else
        let block = {
            let mut b = [0u8; 16];
            b[0] = 0x40;
            b
        };
        let levels: Vec<Vec<u8>> = [256usize, 64, 16, 4, 1, 1, 1]
            .iter()
            .map(|n| block.repeat(*n))
            .collect();
        let len = std::fs::metadata(&png).unwrap().len();
        let dtx = textures.join("0000_a.dtx");
        std::fs::write(&dtx, write(Format::Bc7, 64, 64, (len, 0), &levels).unwrap()).unwrap();
        let _ = data.for_level(Some(&root));
        let c = data.texture_plan(&t, true).unwrap().load().unwrap();
        assert_eq!(
            (c.format, c.size(), c.mips.len()),
            (Format::Bc7, (64, 64), 7)
        );
        // the same file again is not counted again
        let taken = OVERLAY_BYTES.load(std::sync::atomic::Ordering::Relaxed);
        assert!(data.texture_plan(&t, true).is_some());
        assert_eq!(
            OVERLAY_BYTES.load(std::sync::atomic::Ordering::Relaxed),
            taken
        );
        let _ = data.for_level(Some(&root));
        left(256 * 16 * SHARE / 2);
        let c = data.texture_plan(&t, true).unwrap().load().unwrap();
        assert_eq!((c.size(), c.mips.len()), ((32, 32), 6));
        // no BC on the card: decoded
        let _ = data.for_level(Some(&root));
        let c = data.texture_plan(&t, false).unwrap().load().unwrap();
        assert_eq!((c.format, c.size()), (Format::Rgba, (64, 64)));
        // made from another picture (its length differs), or damaged: the picture itself
        std::fs::write(
            &dtx,
            write(Format::Bc7, 64, 64, (len + 1, 0), &levels).unwrap(),
        )
        .unwrap();
        let c = data.texture_plan(&t, true).unwrap().load().unwrap();
        assert_eq!(c.format, Format::Rgba);
        // damaged data: found when it is read, and the picture drawn instead, no larger than what was counted
        // (4 x 4 blocks of 16 bytes: a byte a pixel, so the 64 x 64 picture at 32 x 32)
        let mut bad = write(Format::Bc7, 64, 64, (len, 0), &levels).unwrap();
        *bad.last_mut().unwrap() ^= 1;
        std::fs::write(&dtx, &bad).unwrap();
        let _ = data.for_level(Some(&root));
        let c = data.texture_plan(&t, true).unwrap().load().unwrap();
        assert_eq!((c.format, c.size()), (Format::Rgba, (32, 32)));
        // a second file that says the same of itself is counted once but has no picture instead
        assert!(data.texture_plan(&t, true).unwrap().load().is_none());
        // found by review: files forged to one hash were counted once whatever their size. Another size or length
        // is another texture, counted again
        let _ = data.for_level(Some(&root));
        let good = write(Format::Bc7, 64, 64, (len, 0), &levels).unwrap();
        std::fs::write(&dtx, &good).unwrap();
        assert!(data.texture_plan(&t, true).is_some());
        let taken = OVERLAY_BYTES.load(std::sync::atomic::Ordering::Relaxed);
        let mut small = write(Format::Bc7, 32, 32, (len, 0), &levels[1..]).unwrap();
        small[32..40].copy_from_slice(&good[32..40]);
        std::fs::write(&dtx, &small).unwrap();
        assert!(data.texture_plan(&t, true).is_some());
        assert!(OVERLAY_BYTES.load(std::sync::atomic::Ordering::Relaxed) > taken);
        // found by review: the file is read as it was planned. Changed since (another header), it is not taken
        std::fs::write(&dtx, &good).unwrap();
        let _ = data.for_level(Some(&root));
        let plan = data.texture_plan(&t, true).unwrap();
        let other = write(Format::Bc7, 64, 64, (len, 1), &levels).unwrap();
        std::fs::write(&dtx, &other).unwrap();
        assert_eq!(plan.load().map(|c| c.format), Some(Format::Rgba));
        // found by review: without BC a texture that cannot be made small (its halves are not whole blocks) is left
        // to its picture, never decoded whole
        let block1 = [0xffu8, 0xff, 0, 0, 0, 0, 0, 0];
        let (mut w, mut chain) = (2044u32, Vec::new());
        loop {
            chain.push(block1.repeat((w.div_ceil(4) * w.div_ceil(4)) as usize));
            if w == 1 {
                break;
            }
            w = (w / 2).max(1);
        }
        std::fs::write(
            &dtx,
            write(Format::Bc1, 2044, 2044, (len, 0), &chain).unwrap(),
        )
        .unwrap();
        let _ = data.for_level(Some(&root));
        assert!(matches!(
            data.texture_plan(&t, true),
            Some(TexturePlan::Compressed { .. })
        ));
        assert!(matches!(
            data.texture_plan(&t, false),
            Some(TexturePlan::Picture(_))
        ));
        // found by review: a textures folder that is a link out of the level is refused (the picture and the
        // compressed texture alike)
        #[cfg(unix)]
        {
            std::fs::write(&dtx, &good).unwrap();
            let away = root.join("away");
            std::fs::rename(&textures, &away).unwrap();
            let outside =
                std::env::temp_dir().join(format!("drahtseilakt-plan-out-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&outside);
            std::fs::rename(&away, &outside).unwrap();
            std::os::unix::fs::symlink(&outside, &textures).unwrap();
            assert!(data.texture_plan(&t, true).is_none());
            assert!(data.texture(&t).is_none());
            let _ = std::fs::remove_dir_all(&outside);
        }
        let _ = Data::new(None).for_level(None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Found by review: a player's pack's sky face took the original's transparent colour; a pack's picture has real
    /// alpha, the copy's file keeps the key.
    #[test]
    fn a_pack_sky_face_keeps_its_alpha() {
        let _lock = budget_lock();
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-texpack-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("copy/Textures/Sky")).unwrap();
        std::fs::create_dir_all(root.join("pack")).unwrap();
        let key = image::Rgba([10, 20, 30, 255]);
        image::RgbaImage::from_pixel(1, 1, key)
            .save(root.join("copy/Textures/Sky/Sky_Z_Front.bmp"))
            .unwrap();
        image::RgbaImage::from_pixel(1, 1, key)
            .save(root.join("pack/Sky_Z_Front.png"))
            .unwrap();
        let data = Data::new(Some(root.join("copy"))).for_level(None);
        let face = level::Texture {
            name: "Sky_Z_Front".into(),
            source: level::TextureSource::File("Sky_Z_Front.bmp".into()),
            color_key: Some([10, 20, 30]),
        };
        let alpha = |chain: &[crate::packs::Pack]| {
            data.texture_with(&face, chain)
                .and_then(|(_, _, px)| px.get(3).copied())
        };
        assert_eq!(
            alpha(&[player_pack("p", &root.join("pack"), None)]),
            Some(255)
        );
        assert_eq!(
            alpha(&[]),
            Some(0),
            "the copy's own face keeps its transparent colour"
        );
        let _ = Data::new(None).for_level(None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Found by review: a look moved far enough overflowed to infinities, and a flattened look frame left its normals
    /// unmoved; both are refused, and the part keeps its own look.
    #[test]
    fn a_look_that_cannot_be_moved_is_refused() {
        let far = [
            [1e30, 0.0, 0.0, 0.0],
            [0.0, 1e30, 0.0, 0.0],
            [0.0, 0.0, 1e30, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut mesh = quad("q", 1e10);
        assert!(!transform_mesh(&mut mesh, &far));
        let flat = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        assert_eq!(relative_matrix(&IDENTITY, &flat), None);
        let base = || level::Level {
            meshes: vec![quad("Box", 1.0)],
            materials: vec![material(None)],
            objects: vec![object("P_Box_MF", 0, [0.0; 3])],
            ..Default::default()
        };
        let mut look = level::Level {
            meshes: vec![quad("Other", 1e10)],
            materials: vec![material(None)],
            objects: vec![object("P_Box_MF", 0, [0.0; 3])],
            ..Default::default()
        };
        look.objects[0].matrix = far;
        let mut b = base();
        assert_eq!(attach_looks(&mut b, look), 0);
        assert!(
            b.meshes[0].look.is_none() && b.materials.len() == 1,
            "left as it was"
        );
        let mut look = base();
        look.objects[0].matrix = flat;
        look.meshes[0].name = "Other".into();
        let mut b = base();
        assert_eq!(attach_looks(&mut b, look), 0);
    }

    /// A player's texture pack replaces the copy's textures it has by name (any spelling), passes a broken or
    /// oversized picture over for the next pack, never touches a stranger's own pixels, and stops at `original`.
    #[test]
    fn a_texture_pack_replaces_by_name_and_fails_closed() {
        let _lock = budget_lock();
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-texpack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = (root.join("a"), root.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]))
            .save(a.join("wood_old.png"))
            .unwrap();
        // a picture that does not decode, and one too large to be read: both passed over for pack b's
        std::fs::write(a.join("Brick.png"), b"not a png").unwrap();
        image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 1, 1, 255]))
            .save(b.join("Brick.png"))
            .unwrap();
        image::RgbaImage::from_pixel(1, 1, image::Rgba([2, 2, 2, 255]))
            .save(b.join("Metal.png"))
            .unwrap();
        let data = Data::new(None);
        let chain = [
            player_pack("a", &a, Some("b")),
            player_pack("b", &b, Some("none")),
        ];
        let mut level = level::Level {
            textures: vec![
                pixels("Wood_Old", [9, 9, 9, 255]),
                pixels("Brick", [9, 9, 9, 255]),
                pixels("Floor", [9, 9, 9, 255]),
            ],
            ..Default::default()
        };
        data.put_textures_of(&mut level, Maker::Copy, &chain);
        assert_eq!(first_pixel(&level.textures[0]), Some([10, 20, 30, 255]));
        assert!(
            level.textures[0].color_key.is_none(),
            "a pack's picture has real alpha"
        );
        assert_eq!(
            first_pixel(&level.textures[1]),
            Some([1, 1, 1, 255]),
            "the broken picture falls to the fallback"
        );
        assert_eq!(
            first_pixel(&level.textures[2]),
            Some([9, 9, 9, 255]),
            "a name no pack has is kept"
        );
        // a stranger's own pixels (there is no copy to compare with): never replaced
        let mut map = level::Level {
            textures: vec![pixels("Metal", [5, 5, 5, 255])],
            ..Default::default()
        };
        data.put_textures_of(&mut map, Maker::Stranger, &chain);
        assert_eq!(first_pixel(&map.textures[0]), Some([5, 5, 5, 255]));
        // `original` first keeps the copy's texture as it is, whatever comes after
        let mut copy = level::Level {
            textures: vec![pixels("Metal", [5, 5, 5, 255])],
            ..Default::default()
        };
        data.put_textures_of(
            &mut copy,
            Maker::Copy,
            &[builtin("original"), player_pack("b", &b, None)],
        );
        assert_eq!(first_pixel(&copy.textures[0]), Some([5, 5, 5, 255]));
        // a picture over the size cap is not read
        let big = root.join("big");
        std::fs::create_dir_all(&big).unwrap();
        std::fs::File::create(big.join("Metal.png"))
            .unwrap()
            .set_len(IMAGE_MAX + 1)
            .unwrap();
        let mut copy = level::Level {
            textures: vec![pixels("Metal", [5, 5, 5, 255])],
            ..Default::default()
        };
        data.put_textures_of(
            &mut copy,
            Maker::Copy,
            &[player_pack("big", &big, Some("none"))],
        );
        assert_eq!(first_pixel(&copy.textures[0]), Some([5, 5, 5, 255]));
        let _ = std::fs::remove_dir_all(&root);
    }

    fn quad(name: &str, scale: f32) -> level::Mesh {
        level::Mesh {
            name: name.into(),
            positions: vec![[0.0, 0.0, 0.0], [scale, 0.0, 0.0], [0.0, scale, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs: vec![[0.0, 0.0]; 3],
            parts: vec![level::Part {
                material: Some(0),
                indices: vec![0, 1, 2],
            }],
            ..Default::default()
        }
    }

    fn object(name: &str, mesh: usize, at: [f32; 3]) -> level::Object {
        let mut matrix = IDENTITY;
        matrix[3] = [at[0], at[1], at[2], 1.0];
        level::Object {
            parent: None,
            name: name.into(),
            matrix,
            mesh: Some(mesh),
            visible: true,
            sprite: None,
        }
    }

    fn material(texture: Option<usize>) -> level::Material {
        level::Material {
            name: "m".into(),
            diffuse: [1.0; 4],
            ambient: [0.0; 3],
            emissive: [0.0; 3],
            specular: [0.0; 3],
            power: 0.0,
            texture,
            normal: None,
            parallax: 0.0,
            blend: None,
            alpha_cutoff: None,
            z_write: true,
            two_sided: false,
            unlit: false,
            texgen: None,
        }
    }

    /// A look goes on the mesh of the object of the same name, moved into that object's frame, with its own materials
    /// and textures added; the part's own meshes (the physics') are untouched.
    #[test]
    fn looks_go_on_by_object_and_leave_the_physics() {
        let mut base = level::Level {
            meshes: vec![quad("Box", 1.0), quad("Hidden", 1.0)],
            materials: vec![material(None)],
            objects: vec![
                object("P_Box_MF", 0, [5.0, 0.0, 0.0]),
                object("Collision", 1, [0.0, 0.0, 0.0]),
            ],
            ..Default::default()
        };
        let before: Vec<Vec<[f32; 3]>> = base.meshes.iter().map(|m| m.positions.clone()).collect();
        let look = level::Level {
            meshes: vec![quad("Other", 2.0), quad("Hidden", 3.0)],
            materials: vec![material(Some(0))],
            textures: vec![pixels("T", [1, 2, 3, 255])],
            // the pack's object one unit further along x: its look is moved by that in the base's frame
            objects: vec![object("p_box_mf", 0, [6.0, 0.0, 0.0])],
            ..Default::default()
        };
        assert_eq!(attach_looks(&mut base, look), 2);
        let after: Vec<Vec<[f32; 3]>> = base.meshes.iter().map(|m| m.positions.clone()).collect();
        assert_eq!(before, after, "the physics' meshes stay as they were");
        let l0 = base.meshes[0].look.as_deref().unwrap();
        assert_eq!(
            l0.positions,
            vec![[1.0, 0.0, 0.0], [3.0, 0.0, 0.0], [1.0, 2.0, 0.0]]
        );
        assert_eq!(l0.parts[0].material, Some(1));
        assert_eq!(base.materials[1].texture, Some(0));
        assert_eq!(base.textures.len(), 1);
        // matched by the mesh's name where no object matches, unmoved
        assert_eq!(
            base.meshes[1].look.as_deref().unwrap().positions[1],
            [3.0, 0.0, 0.0]
        );
    }

    #[test]
    fn a_shared_mesh_takes_a_look_only_when_its_objects_agree() {
        let make = || level::Level {
            meshes: vec![quad("Wall", 1.0)],
            materials: vec![material(None)],
            objects: vec![
                object("Wall01", 0, [0.0, 0.0, 0.0]),
                object("Wall02", 0, [4.0, 0.0, 0.0]),
            ],
            ..Default::default()
        };
        let look = |second: [f32; 3]| level::Level {
            meshes: vec![quad("Wall", 2.0)],
            materials: vec![material(None)],
            objects: vec![
                object("Wall01", 0, [0.0, 0.0, 0.0]),
                object("Wall02", 0, second),
            ],
            ..Default::default()
        };
        let mut agree = make();
        assert_eq!(attach_looks(&mut agree, look([4.0, 0.0, 0.0])), 1);
        let mut disagree = make();
        assert_eq!(attach_looks(&mut disagree, look([9.0, 0.0, 0.0])), 0);
        assert!(
            disagree.meshes[0].look.is_none() && disagree.materials.len() == 1,
            "left as it was"
        );
    }

    #[test]
    fn the_relative_matrix_inverts_rotations_and_scales() {
        // base: turned 90 degrees about y and scaled by 2, at (1, 2, 3)
        let base = [
            [0.0, 0.0, -2.0, 0.0],
            [0.0, 2.0, 0.0, 0.0],
            [2.0, 0.0, 0.0, 0.0],
            [1.0, 2.0, 3.0, 1.0],
        ];
        assert_eq!(relative_matrix(&base, &base), Some(IDENTITY));
        let inv = affine_inverse(&base).unwrap();
        let mut mesh = quad("q", 1.0);
        transform_mesh(&mut mesh, &base);
        transform_mesh(&mut mesh, &inv);
        for (p, q) in mesh.positions.iter().zip(quad("q", 1.0).positions) {
            assert!(
                p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-5),
                "{p:?} {q:?}"
            );
        }
        assert!(mesh.normals.iter().all(|n| (n[2] - 1.0).abs() < 1e-5));
        let flat = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        assert_eq!(
            affine_inverse(&flat),
            None,
            "a flattened frame cannot be inverted"
        );
    }

    /// A player's model pack part: read only through plain folders and capped files, checked; anything wrong is left
    /// out (the next pack gives the part).
    #[test]
    fn hostile_model_pack_parts_are_left_out() {
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-modelpack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let good = level::Level {
            meshes: vec![quad("Box", 1.0)],
            materials: vec![material(None)],
            objects: vec![object("P_Box_MF", 0, [0.0; 3])],
            ..Default::default()
        };
        let part = root.join("pack/PH/P_Box");
        std::fs::create_dir_all(&part).unwrap();
        level::open::export(&good, &part).unwrap();
        let pack = root.join("pack");
        assert!(load_pack_part(&pack, "3D Entities/PH/P_Box").is_some());
        assert!(
            load_pack_part(&pack, "3D Entities/PH/../PH/P_Box").is_none(),
            "no `..`"
        );
        assert!(
            load_pack_part(&pack, "3D Entities/PH/P_Ball").is_none(),
            "a part the pack lacks"
        );
        // a number that is not finite, and too many vertices
        let mut nan = good.clone();
        nan.meshes[0].positions[1][0] = f32::NAN;
        assert!(check_pack_part(&nan).is_err());
        let mut huge = good.clone();
        huge.meshes[0].positions = vec![[0.0; 3]; PACK_VERTICES_MAX + 1];
        assert!(check_pack_part(&huge).is_err());
        assert!(check_pack_part(&good).is_ok());
        // a level.bin over the cap is not read
        let big = root.join("big/PH/P_Box");
        std::fs::create_dir_all(&big).unwrap();
        level::open::export(&good, &big).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(big.join("level.bin"))
            .unwrap()
            .set_len(PACK_BIN_MAX + 1)
            .unwrap();
        assert!(load_pack_part(&root.join("big"), "3D Entities/PH/P_Box").is_none());
        #[cfg(unix)]
        {
            // a linked part folder, or a linked level.gltf, finds nothing
            std::fs::create_dir_all(root.join("linked/PH")).unwrap();
            std::os::unix::fs::symlink(&part, root.join("linked/PH/P_Box")).unwrap();
            assert!(load_pack_part(&root.join("linked"), "3D Entities/PH/P_Box").is_none());
            let file_link = root.join("filelink/PH/P_Box");
            std::fs::create_dir_all(&file_link).unwrap();
            std::os::unix::fs::symlink(part.join("level.gltf"), file_link.join("level.gltf"))
                .unwrap();
            std::fs::copy(part.join("level.bin"), file_link.join("level.bin")).unwrap();
            assert!(load_pack_part(&root.join("filelink"), "3D Entities/PH/P_Box").is_none());
        }
        // through the chain: a broken pack passes to the next, `level` ends it with no look
        let data = Data::new(None);
        let mut base = good.clone();
        let chain = [
            player_pack("big", &root.join("big"), None),
            player_pack("pack", &pack, None),
        ];
        data.put_looks_of(
            "3D Entities/PH/P_Box.nmo",
            &mut base,
            PartFrom::Copy,
            &chain,
        );
        assert!(base.meshes[0].look.is_some());
        let mut base = good.clone();
        data.put_looks_of(
            "3D Entities/PH/P_Box.nmo",
            &mut base,
            PartFrom::Copy,
            &[builtin("level"), player_pack("pack", &pack, None)],
        );
        assert!(base.meshes[0].look.is_none());
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
