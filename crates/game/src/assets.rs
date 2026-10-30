// Finding things in the player's own copy of the game: levels, the balls, textures. File names in levels are
// Windows names, so lookups ignore case.

use std::path::{Path, PathBuf};

pub struct Data {
    pub root: PathBuf,
}

/// Where a level comes from.
pub enum Source {
    /// One of the original levels of the player's copy (1–13).
    Original(u32),
    /// A `.nmo` file: a community map, or any level file.
    Nmo(PathBuf),
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
            .chain([PathBuf::from("data")]);
        for root in candidates {
            if find(&root, "3D Entities").is_some() {
                return Ok(Data { root });
            }
        }
        Err(
            "game data not found: pass --data DIR or set OPENBALLANCE_DATA to your Ballance folder"
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
        };
        load_nmo(&path)
    }

    /// RGBA pixels of a texture: its own pixels, or its file from the game's texture folders.
    pub fn texture(&self, t: &level::Texture) -> Option<(u32, u32, Vec<u8>)> {
        match &t.source {
            level::TextureSource::Pixels {
                width,
                height,
                rgba,
            } => Some((*width, *height, rgba.clone())),
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
}

impl Data {
    /// The settings row whose level file matches `file_name`; a community map without one takes level 1's.
    pub fn settings(&self, file_name: &str) -> Settings {
        let none = Settings {
            sky: None,
            start_ball: None,
        };
        let Some(path) = self.file("3D Entities/Levelinit.nmo") else {
            return none;
        };
        let Ok(bytes) = std::fs::read(path) else {
            return none;
        };
        let Ok(file) = nmo::read(&bytes) else {
            return none;
        };
        let Ok(doc) = nmo::Document::parse(&file) else {
            return none;
        };
        let table = doc
            .of_class(nmo::class::DATA_ARRAY)
            .find(|(_, o)| o.name.eq_ignore_ascii_case(b"AllLevel"))
            .and_then(|(_, o)| o.chunk.as_ref())
            .and_then(|c| nmo::scene::data_array(c).ok());
        let Some(table) = table else { return none };
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
        }
    }
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
fn find(dir: &Path, name: &str) -> Option<PathBuf> {
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
