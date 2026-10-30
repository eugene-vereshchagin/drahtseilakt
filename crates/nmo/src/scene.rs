// Scene objects decoded from their chunks: meshes, materials, textures, 3D entities and groups. Field layouts
// were worked out from the files themselves (the original levels and the community maps) together with the
// identifier constants of the Virtools SDK; see docs/drahtseilakt/nmo-format.md.

use crate::chunk::Chunk;
use crate::document::reference;
use crate::error::{Error, Part, Result};

/// Caps against hostile counts; the largest original mesh has a few thousand vertices.
const MAX_VERTICES: usize = 1 << 20;
const MAX_FACES: usize = 1 << 21;
const MAX_TEXTURE_SIDE: u32 = 8_192;

fn malformed(what: &'static str) -> Error {
    Error::Malformed {
        part: Part::Data,
        what,
    }
}

fn missing(what: &'static str) -> Error {
    Error::Malformed {
        part: Part::Data,
        what,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    /// Empty when the file stores none (the original levels): the reader of the mesh computes them.
    pub normals: Vec<[f32; 3]>,
    /// One pair per vertex; a mesh saved with a single UV repeats it.
    pub uvs: Vec<[f32; 2]>,
    /// Vertex indices and the index into `materials`.
    pub faces: Vec<([u16; 3], u16)>,
    /// Material object indices; `None` for an empty slot.
    pub materials: Vec<Option<usize>>,
}

mod vertex_flags {
    pub const SINGLE_COLOR: u32 = 0x1;
    pub const SINGLE_SPECULAR: u32 = 0x2;
    pub const NO_NORMAL: u32 = 0x4;
    pub const SINGLE_UV: u32 = 0x8;
    pub const NO_POSITION: u32 = 0x10;
}

pub fn mesh(chunk: &Chunk) -> Result<Mesh> {
    let mut mesh = Mesh {
        positions: vec![],
        normals: vec![],
        uvs: vec![],
        faces: vec![],
        materials: vec![],
    };

    if let Some(mut w) = chunk.section(0x0010_0000) {
        let n = w.u32()? as usize;
        if n > w.remaining() {
            return Err(malformed("material count"));
        }
        for _ in 0..n {
            mesh.materials.push(reference(chunk, w.u32()?));
            w.u32()?;
        }
    }

    if let Some(mut w) = chunk.section(0x0002_0000) {
        let count = w.u32()? as usize;
        if count > MAX_VERTICES {
            return Err(malformed("vertex count"));
        }
        if count > 0 {
            let flags = w.u32()?;
            let _size = w.u32()?;
            let vec3s = |w: &mut crate::chunk::Words<'_>| -> Result<Vec<[f32; 3]>> {
                let words = w.take(count.checked_mul(3).ok_or(malformed("vertex data"))?)?;
                Ok(words.chunks_exact(3).map(xyz).collect())
            };
            mesh.positions = if flags & vertex_flags::NO_POSITION == 0 {
                vec3s(&mut w)?
            } else {
                vec![[0.0; 3]; count]
            };
            for flag in [vertex_flags::SINGLE_COLOR, vertex_flags::SINGLE_SPECULAR] {
                w.take(if flags & flag != 0 { 1 } else { count })?;
            }
            if flags & vertex_flags::NO_NORMAL == 0 {
                mesh.normals = vec3s(&mut w)?;
            }
            mesh.uvs = if flags & vertex_flags::SINGLE_UV != 0 {
                let uv = w.take(2)?;
                vec![uv2(uv); count]
            } else {
                let words = w.take(count.checked_mul(2).ok_or(malformed("uv data"))?)?;
                words.chunks_exact(2).map(uv2).collect()
            };
        }
    }

    if let Some(mut w) = chunk.section(0x0001_0000) {
        let count = w.u32()? as usize;
        if count > MAX_FACES {
            return Err(malformed("face count"));
        }
        let words = w.take(count.checked_mul(2).ok_or(malformed("face data"))?)?;
        for f in words.chunks_exact(2) {
            let (a, b) = match f {
                [a, b] => (*a, *b),
                _ => return Err(malformed("face")),
            };
            let idx = [a as u16, (a >> 16) as u16, b as u16];
            if idx.iter().any(|&i| usize::from(i) >= mesh.positions.len()) {
                return Err(malformed("face index"));
            }
            mesh.faces.push((idx, (b >> 16) as u16));
        }
    }
    Ok(mesh)
}

fn xyz(c: &[u32]) -> [f32; 3] {
    let f = |i: usize| c.get(i).copied().map(f32::from_bits).unwrap_or(0.0);
    [f(0), f(1), f(2)]
}

fn uv2(c: &[u32]) -> [f32; 2] {
    let f = |i: usize| c.get(i).copied().map(f32::from_bits).unwrap_or(0.0);
    [f(0), f(1)]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    Zero = 1,
    One,
    SrcColor,
    InvSrcColor,
    SrcAlpha,
    InvSrcAlpha,
    DestAlpha,
    InvDestAlpha,
    DestColor,
    InvDestColor,
    SrcAlphaSat,
}

impl Blend {
    fn from_nibble(n: u32) -> Blend {
        match n {
            1 => Blend::Zero,
            3 => Blend::SrcColor,
            4 => Blend::InvSrcColor,
            5 => Blend::SrcAlpha,
            6 => Blend::InvSrcAlpha,
            7 => Blend::DestAlpha,
            8 => Blend::InvDestAlpha,
            9 => Blend::DestColor,
            10 => Blend::InvDestColor,
            11 => Blend::SrcAlphaSat,
            _ => Blend::One,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// Colours as ARGB words.
    pub diffuse: u32,
    pub ambient: u32,
    pub specular: u32,
    pub emissive: u32,
    pub specular_power: f32,
    pub texture: Option<usize>,
    /// Virtools texture blend mode (1 decal, 2 modulate, 3 decal alpha, 4 modulate alpha, ...).
    pub texture_blend: u32,
    pub src_blend: Blend,
    pub dest_blend: Blend,
    pub alpha_blend: bool,
    pub alpha_test: bool,
    /// Alpha comparison (Virtools VXCMPFUNC) and reference, used when `alpha_test` is set.
    pub alpha_func: u32,
    pub alpha_ref: u8,
    pub z_write: bool,
    pub two_sided: bool,
}

pub fn material(chunk: &Chunk) -> Result<Material> {
    let mut w = chunk.section(0x1000).ok_or(missing("material data"))?;
    let diffuse = w.u32()?;
    let ambient = w.u32()?;
    let specular = w.u32()?;
    let emissive = w.u32()?;
    let specular_power = w.f32()?;
    let texture = reference(chunk, w.u32()?);
    let _border = w.u32()?;
    let modes = w.u32()?;
    let flags = w.u32()?;
    let nib = |shift: u32| modes.checked_shr(shift).unwrap_or(0) & 0xF;
    Ok(Material {
        diffuse,
        ambient,
        specular,
        emissive,
        specular_power,
        texture,
        texture_blend: nib(0),
        src_blend: Blend::from_nibble(nib(12)),
        dest_blend: Blend::from_nibble(nib(16)),
        two_sided: flags & 0x1 != 0,
        z_write: flags & 0x2 != 0,
        alpha_blend: flags & 0x8 != 0,
        alpha_test: flags & 0x10 != 0,
        alpha_func: (flags >> 16) & 0xFF,
        alpha_ref: (flags >> 24) as u8,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// The file name of the first slot, looked up in the game's texture folders.
    pub file_name: Option<Vec<u8>>,
    /// Pixels saved inside the file, if any: width, height and RGBA bytes.
    pub embedded: Option<(u32, u32, Vec<u8>)>,
}

pub fn texture(chunk: &Chunk) -> Result<Texture> {
    let mut tex = Texture {
        file_name: None,
        embedded: None,
    };
    if let Some(mut w) = chunk.section(0x0001_0000) {
        let slots = w.u32()?;
        if slots > 0 {
            let name = w.string()?;
            if !name.is_empty() {
                tex.file_name = Some(name);
            }
        }
    }
    if let Some(mut w) = chunk.section(0x0002_0000) {
        let slots = w.u32()?;
        if slots > 0 && w.remaining() >= 9 {
            let bpp = w.u32()?;
            let (width, height) = (w.u32()?, w.u32()?);
            let masks = [w.u32()?, w.u32()?, w.u32()?, w.u32()?];
            let _ = w.u32()?;
            let count = w.u32()? as usize;
            if bpp == 32 && width <= MAX_TEXTURE_SIDE && height <= MAX_TEXTURE_SIDE {
                let pixels = (width as usize).saturating_mul(height as usize);
                if count == pixels {
                    let data = w.take(count)?;
                    tex.embedded = Some((width, height, unpack_pixels(data, masks)));
                }
            }
        }
    }
    Ok(tex)
}

/// 32-bit pixels with channel masks (A, R, G, B) to RGBA bytes.
fn unpack_pixels(data: &[u32], [a, r, g, b]: [u32; 4]) -> Vec<u8> {
    let channel = |p: u32, mask: u32| -> u8 {
        if mask == 0 {
            return 0xFF;
        }
        ((p & mask) >> mask.trailing_zeros()) as u8
    };
    data.iter()
        .flat_map(|&p| [channel(p, r), channel(p, g), channel(p, b), channel(p, a)])
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Row-major world matrix as Virtools stores it: three axis rows, then the position.
    pub matrix: [[f32; 3]; 4],
    /// The mesh drawn for this entity (3D objects only).
    pub mesh: Option<usize>,
    pub flags: u32,
}

pub fn entity(chunk: &Chunk) -> Result<Entity> {
    let mut w = chunk.section(0x0010_0000).ok_or(missing("entity matrix"))?;
    let _ = w.u32()?;
    let flags = w.u32()?;
    let m: Vec<[f32; 3]> = w.take(12)?.chunks_exact(3).map(xyz).collect();
    let row = |i: usize| m.get(i).copied().unwrap_or_default();
    let mesh = match chunk.section(0x4000) {
        Some(mut w) => reference(chunk, w.u32()?),
        None => None,
    };
    Ok(Entity {
        matrix: [row(0), row(1), row(2), row(3)],
        mesh,
        flags,
    })
}

pub fn group(chunk: &Chunk) -> Result<Vec<usize>> {
    let Some(mut w) = chunk.section(0x000F_FFFF) else {
        return Ok(vec![]);
    };
    let n = w.u32()? as usize;
    let refs = w.take(n)?;
    Ok(refs.iter().filter_map(|&r| reference(chunk, r)).collect())
}

/// A cell of a data array (a Virtools table such as `AllLevel`, which lists each level's sky and ball).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Int(i32),
    Float(f32),
    String(Vec<u8>),
    /// An object, or a parameter object holding the value (by index in the file).
    Object(Option<usize>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataArray {
    /// Column names as stored.
    pub columns: Vec<Vec<u8>>,
    pub rows: Vec<Vec<Cell>>,
}

impl DataArray {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(name.as_bytes()))
    }
}

const MAX_COLUMNS: usize = 1_024;

pub fn data_array(chunk: &Chunk) -> Result<DataArray> {
    let mut w = chunk.section(0x1000).ok_or(missing("data array columns"))?;
    let n = w.u32()? as usize;
    if n > MAX_COLUMNS {
        return Err(malformed("column count"));
    }
    let mut columns = Vec::with_capacity(n);
    let mut kinds = Vec::with_capacity(n);
    for _ in 0..n {
        columns.push(w.string()?);
        let kind = w.u32()?;
        if kind == 5 {
            w.take(2)?; // parameter type GUID
        }
        kinds.push(kind);
    }
    let mut rows = Vec::new();
    if let Some(mut w) = chunk.section(0x2000) {
        let count = w.u32()? as usize;
        if count > w.remaining() {
            return Err(malformed("row count"));
        }
        for _ in 0..count {
            let mut row = Vec::with_capacity(kinds.len());
            for &kind in &kinds {
                row.push(match kind {
                    1 => Cell::Int(w.u32()? as i32),
                    2 => Cell::Float(w.f32()?),
                    3 => Cell::String(w.string()?),
                    _ => Cell::Object(reference(chunk, w.u32()?)),
                });
            }
            rows.push(row);
        }
    }
    Ok(DataArray { columns, rows })
}
