// Finding things in the player's own copy of the game: levels, the balls, textures. File names in levels are
// Windows names, so lookups ignore case.

use std::path::{Path, PathBuf};

/// The largest texture file of an open-format level read, and its largest side (the toolbox makes them up to 8K).
const IMAGE_MAX: u64 = 64 * 1024 * 1024;
const SIDE_MAX: u32 = 8192;

/// Where the toolbox keeps its remade copy of a game file the levels share (the balls, the modules, the
/// transformer; `relative` as "3D Entities/PH/P_Ball_Wood.nmo"): levels/remastered/.shared/ and the file's path
/// without its extension, in lower case (the game's names do not agree on case), a folder in the open format.
pub fn remade_dir(relative: &str) -> PathBuf {
    let stem = Path::new(&relative.to_lowercase()).with_extension("");
    crate::profile::dir(crate::profile::Kind::Remastered).join(".shared").join(stem)
}

/// Gives a game file's level what the toolbox remade of it, when there is a remade copy that matches it: each
/// mesh's finer look, and each texture's larger file. The level's own meshes stay as they are, so the physics does
/// not change. A remade copy that does not match (another copy of the game, an older toolbox) is left unused.
pub fn graft(level: &mut level::Level, remade: &Path) {
    graft_within(level, &crate::profile::dir(crate::profile::Kind::Remastered), remade);
}

/// [`graft`] with the remade copy inside `root`: real folders from `root` down, never through a link, and its parts
/// small (a part's buffer and its textures' pixels are capped, the textures of all parts together too).
pub fn graft_within(level: &mut level::Level, root: &Path, remade: &Path) {
    if !real_folders(root, &remade.join("textures")) {
        return;
    }
    let small = std::fs::symlink_metadata(remade.join("level.bin")).is_ok_and(|m| m.is_file() && m.len() <= PART_MAX);
    if !small {
        return;
    }
    let Ok(r) = level::open::import(remade) else { return };
    let written: Vec<&mut level::Mesh> = level.meshes.iter_mut().filter(|m| level::open::written(m)).collect();
    let same = written.len() == r.meshes.len() && written.iter().zip(&r.meshes).all(|(m, rm)| m.name == rm.name);
    let textures = level.textures.len() == r.textures.len()
        && level.textures.iter().zip(&r.textures).all(|(t, rt)| t.name == rt.name);
    if !same || !textures {
        return;
    }
    for (m, rm) in written.into_iter().zip(r.meshes) {
        m.look = rm.look;
    }
    for (t, rt) in level.textures.iter_mut().zip(r.textures) {
        if let level::TextureSource::File(p) = &rt.source
            && take_pixels(p)
        {
            t.source = rt.source;
        }
    }
}

/// The largest buffer of a remade part, and the most texture pixels all remade parts may bring (1 GB of RGBA).
const PART_MAX: u64 = 64 * 1024 * 1024;
const PIXELS_MAX: u64 = 256 * 1024 * 1024;
static PIXELS_TAKEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A remade texture may be used: its file is a plain one and its pixels (read from its header) still fit in what the
/// remade parts may bring together; they are counted as taken.
fn take_pixels(path: &str) -> bool {
    use std::sync::atomic::Ordering;
    let plain = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX);
    let Some((w, h)) = plain
        .then(|| image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_dimensions().ok())
        .flatten()
    else {
        return false;
    };
    let n = u64::from(w).saturating_mul(u64::from(h));
    let mut taken = PIXELS_TAKEN.load(Ordering::Relaxed);
    loop {
        let Some(next) = taken.checked_add(n).filter(|&t| t <= PIXELS_MAX) else {
            return false;
        };
        match PIXELS_TAKEN.compare_exchange_weak(taken, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return true,
            Err(now) => taken = now,
        }
    }
}

/// Every folder from `root` down to `dir` is a real folder (no link on the way), `dir` inside `root`.
fn real_folders(root: &Path, dir: &Path) -> bool {
    let Ok(rest) = dir.strip_prefix(root) else { return false };
    let mut at = root.to_path_buf();
    std::fs::symlink_metadata(&at).is_ok_and(|m| m.is_dir())
        && rest.components().all(|c| {
            at.push(c);
            matches!(c, std::path::Component::Normal(_)) && std::fs::symlink_metadata(&at).is_ok_and(|m| m.is_dir())
        })
}

pub struct Data {
    pub root: PathBuf,
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
    /// The original level this is: one of the copy's, or the player's remastered export of one
    /// (levels/remastered/Level_NN), which plays with that level's settings.
    pub fn original_number(&self) -> Option<u32> {
        match self {
            Source::Original(n) => Some(*n),
            Source::Open(dir) => self.remastered().then(|| {
                let name = dir.file_name()?.to_str()?;
                name.get(..6)?.eq_ignore_ascii_case("Level_").then(|| name.get(6..)?.parse().ok())?
            })?,
            Source::Nmo(_) => None,
        }
    }

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
                return Ok(Data { root });
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
    pub fn file(&self, relative: &str) -> Option<PathBuf> {
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

    /// A game file the levels share (`relative` as "3D Entities/Balls.nmo"), with what the toolbox remade of it when
    /// `remastered` (a remastered level is played).
    pub fn part(&self, relative: &str, remastered: bool) -> Option<LoadedLevel> {
        let mut loaded = load_nmo(&self.file(relative)?).ok()?;
        if remastered {
            graft(&mut loaded.level, &remade_dir(relative));
        }
        Some(loaded)
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
        // Levels of the game name their sky and ball in the game's own table, not in the level file.
        let s = self.settings(&loaded.name);
        loaded.level.sky = loaded.level.sky.take().or(s.sky);
        loaded.level.start_ball = loaded.level.start_ball.take().or(s.start_ball);
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
                let plain = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() <= IMAGE_MAX);
                if !plain {
                    return None;
                }
                let mut reader = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?;
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(SIDE_MAX);
                limits.max_image_height = Some(SIDE_MAX);
                reader.limits(limits);
                let img = reader.decode().ok()?.to_rgba8();
                Some((img.width(), img.height(), img.into_raw()))
            }
            level::TextureSource::File(name) => {
                let file = Path::new(name).file_name()?.to_str()?.to_owned();
                let path = ["Textures", "Textures/Sky", "Textures/Particles"]
                    .iter()
                    .find_map(|dir| self.file(&format!("{dir}/{file}")))?;
                let img = image::open(path).ok()?.to_rgba8();
                Some((img.width(), img.height(), img.into_raw()))
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
        let row = table
            .rows
            .iter()
            .find(|r| {
                // a level file's name, or an export's folder named after it (Level_13 for Level_13.NMO)
                text(r, "Levelfile").is_some_and(|f| {
                    f.eq_ignore_ascii_case(file_name)
                        || Path::new(&f).file_stem().is_some_and(|s| s.eq_ignore_ascii_case(file_name))
                })
            })
            .or_else(|| table.rows.first());
        let Some(row) = row else { return none };
        Settings {
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
pub fn export(data: &Data, loaded: &LoadedLevel, dir: &Path) -> Result<(), String> {
    level::open::export(&loaded.level, dir)?;
    let mut missing = 0usize;
    for (i, t) in loaded.level.textures.iter().enumerate() {
        let path = dir.join(level::open::texture_file(i, &t.name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        match data.texture(t) {
            Some((w, h, rgba)) => {
                image::save_buffer(&path, &rgba, w, h, image::ExtendedColorType::Rgba8)
                    .map_err(|e| e.to_string())?
            }
            None => missing = missing.saturating_add(1),
        }
    }
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
