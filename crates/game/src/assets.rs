// Finding things in the player's own copy of the game: levels, the balls, textures. File names in levels are
// Windows names, so lookups ignore case.

use std::path::{Path, PathBuf};

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
            .chain([crate::levels::dir(crate::levels::Kind::Original), PathBuf::from("data")]);
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

    pub fn file(&self, relative: &str) -> Option<PathBuf> {
        relative
            .split('/')
            .try_fold(self.root.clone(), |dir, part| find(&dir, part))
    }

    pub fn load(&self, source: &Source) -> Result<LoadedLevel, String> {
        let path = match source {
            Source::Original(n) => self
                .file(&format!("3D Entities/Level/Level_{n:02}.NMO"))
                .ok_or_else(|| format!("Level_{n:02}.NMO is not in {}", self.root.display()))?,
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
            for px in rgba.chunks_exact_mut(4) {
                if px[..3] == key {
                    px[3] = 0;
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
            level::TextureSource::File(name) if Path::new(name).is_absolute() => {
                let img = image::open(name).ok()?.to_rgba8();
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

    /// The settings row whose level file matches `file_name`; a community map without one takes level 1's.
    pub fn settings(&self, file_name: &str) -> Settings {
        let none = Settings {
            sky: None,
            start_ball: None,
            music: None,
            level_bonus: None,
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
            .find(|r| text(r, "Levelfile").is_some_and(|f| f.eq_ignore_ascii_case(file_name)))
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
        }
    }
}

pub fn load_open(dir: &Path) -> Result<LoadedLevel, String> {
    let level = level::open::import(dir)?;
    let mut bytes = std::fs::read(dir.join("level.gltf")).map_err(|e| e.to_string())?;
    bytes.extend(std::fs::read(dir.join("level.bin")).map_err(|e| e.to_string())?);
    Ok(LoadedLevel {
        level,
        name: dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        hash: crate::replay::fnv1a(&bytes),
        warnings: Vec::new(),
    })
}

/// Writes a level in the open format into `dir`, with its textures as PNG files (from the player's copy).
pub fn export(data: &Data, loaded: &LoadedLevel, dir: &Path) -> Result<(), String> {
    level::open::export(&loaded.level, dir)?;
    let mut missing = 0;
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
            None => missing += 1,
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
        hash: crate::replay::fnv1a(&bytes),
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
