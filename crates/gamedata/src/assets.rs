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

pub struct Data {
    pub root: PathBuf,
    /// The game files of the level being played (`<level>/game`), looked in before the copy.
    pub overlay: Option<PathBuf>,
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
    pub fn locate(explicit: Option<PathBuf>) -> Result<Data, String> {
        let candidates = explicit
            .into_iter()
            .chain(std::env::var_os("OPENBALLANCE_DATA").map(PathBuf::from))
            .chain([crate::profile::dir(crate::profile::Kind::Original), PathBuf::from("data")]);
        // a copy counts when it has the first level, not only the folder: a half-made import is not a game
        let playable = |root: &Path| {
            ["3D Entities", "Level", "Level_01.NMO"].iter().try_fold(root.to_path_buf(), |d, p| find(&d, p)).is_some()
        };
        for root in candidates {
            if playable(&root) {
                return Ok(Data { root, overlay: None });
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
    pub fn for_level(&self, level_dir: Option<&Path>) -> Data {
        Data {
            root: self.root.clone(),
            overlay: level_dir.map(|d| d.join(GAME_FILES)),
        }
    }

    pub fn file(&self, relative: &str) -> Option<PathBuf> {
        if let Some(found) = self.overlay.as_deref().and_then(|o| plain_path(o, relative)) {
            return Some(found);
        }
        self.copy_file(relative)
    }

    /// A file of the copy itself (never the level's own).
    fn copy_file(&self, relative: &str) -> Option<PathBuf> {
        relative.split('/').try_fold(self.root.clone(), |dir, part| {
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
        if let Some(dir) = self.overlay.as_deref().and_then(|o| plain_path(o, stem)) {
            return load_open(&dir).ok();
        }
        load_nmo(&self.copy_file(relative)?).ok()
    }

    pub fn load(&self, source: &Source) -> Result<LoadedLevel, String> {
        let path = match source {
            Source::Original(n) => match self.original(*n) {
                Some(Source::Nmo(p)) => p,
                Some(Source::Open(dir)) => return load_open(&dir),
                _ => return Err(format!("Level_{n:02}.NMO is not in {}", self.root.display())),
            },
            Source::Nmo(p) => p.clone(),
            Source::Open(dir) => return load_open(dir),
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
        };
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
            level::TextureSource::File(name) if Path::new(name).is_absolute() => {
                let path = Path::new(name);
                // its folder a real one too (a link there would lead out of the level's folder)
                let folder = path.parent().is_some_and(|d| std::fs::symlink_metadata(d).is_ok_and(|m| m.is_dir()));
                let plain = folder && std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX);
                if !plain {
                    return None;
                }
                decode_capped(path)
            }
            level::TextureSource::File(name) => {
                let file = Path::new(name).file_name()?.to_str()?.to_owned();
                let path = ["Textures", "Textures/Sky", "Textures/Particles"]
                    .iter()
                    .find_map(|dir| self.file(&format!("{dir}/{file}")))?;
                // it may come from a level's game folder (a stranger's): its size checked before it is decoded
                if !std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX) {
                    return None;
                }
                decode_capped(&path)
            }
            level::TextureSource::None => None,
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
        let bytes = std::fs::read(self.file(relative)?).ok()?;
        let file = nmo::read(&bytes).ok()?;
        let doc = nmo::Document::parse(&file).ok()?;
        doc.of_class(nmo::class::DATA_ARRAY)
            .find(|(_, o)| o.name.eq_ignore_ascii_case(name.as_bytes()))
            .and_then(|(_, o)| o.chunk.as_ref())
            .and_then(|c| nmo::scene::data_array(c).ok())
    }

    /// The value of the parameter object at `index` of the game file `relative`, as floats.
    pub fn parameter_floats(&self, relative: &str, index: usize) -> Option<Vec<f32>> {
        let bytes = std::fs::read(self.file(relative)?).ok()?;
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
        let Some(bytes) = self.file(relative).and_then(|p| std::fs::read(p).ok()) else {
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
                Some(nmo::scene::Cell::Object(Some(p))) => self.parameter_floats("3D Entities/Levelinit.nmo", *p).and_then(|v| {
                    let c = |i: usize| v.get(i).copied().map(|x| x.clamp(0.0, 1.0));
                    Some([c(0)?, c(1)?, c(2)?])
                }),
                _ => None,
            },
        }
    }
}

pub fn load_open(dir: &Path) -> Result<LoadedLevel, String> {
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
    let missing = export_level(data, &loaded.level, dir)?;
    let game = dir.join(GAME_FILES);
    let mut parts = 0usize;
    for rel in parts_used(data, &loaded.level) {
        let Some(part) = data.part(&rel) else { continue };
        let stem = rel.strip_suffix(".nmo").or_else(|| rel.strip_suffix(".NMO")).unwrap_or(&rel);
        export_level(data, &part.level, &game.join(stem))?;
        parts = parts.saturating_add(1);
    }
    let mut files = 0usize;
    std::fs::create_dir_all(&game).map_err(|e| format!("{}: {e}", game.display()))?;
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
    level::open::export(level, dir)?;
    let mut missing = 0usize;
    for (i, t) in level.textures.iter().enumerate() {
        let path = dir.join(level::open::texture_file(i, &t.name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        match data.texture(t) {
            Some((w, h, rgba)) => {
                image::save_buffer(&path, &rgba, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?
            }
            None => missing = missing.saturating_add(1),
        }
    }
    Ok(missing)
}

/// A picture decoded with its sides capped at SIDE_MAX, as RGBA.
fn decode_capped(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let mut reader = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?;
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
        let entry = std::fs::read_dir(&dir)
            .ok()?
            .flatten()
            .take(4096)
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
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
    std::fs::write(&tmp, &bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &dst).map_err(|e| format!("{}: {e}", dst.display()))
}

pub fn load_nmo(path: &Path) -> Result<LoadedLevel, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
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
