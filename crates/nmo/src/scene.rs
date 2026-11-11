// Scene objects decoded from their chunks: meshes, materials, textures, 3D entities and groups. Field layouts
// were worked out from the files themselves (the original levels and the community maps) together with the
// identifier constants of the Virtools SDK; see docs/drahtseilakt/nmo-format.md.

use crate::chunk::{Chunk, Words};
use crate::document::reference;
use crate::error::{Error, Part, Result};

/// Caps against hostile counts; the largest original mesh has a few thousand vertices.
const MAX_VERTICES: usize = 1 << 20;
const MAX_FACES: usize = 1 << 21;
/// Material channels a mesh may have.
pub const MAX_CHANNELS: usize = 16;
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

/// A mesh's material channel: the mesh drawn again with another material, blended over itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub material: Option<usize>,
    /// Bit 0x1: active.
    pub flags: u32,
    pub src_blend: Blend,
    pub dest_blend: Blend,
    /// One pair per vertex, or empty when the channel shares the mesh's.
    pub uvs: Vec<[f32; 2]>,
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
            // Vertices that carry no data of their own (no positions, no normals, one colour, specular and uv for
            // all) cost the file nothing: a few words would make a million of them, some 20 MB a mesh. Such vertices
            // are no more than the words left; any other vertex carries at least one word per vertex.
            use vertex_flags::*;
            let bare = NO_POSITION | NO_NORMAL | SINGLE_UV | SINGLE_COLOR | SINGLE_SPECULAR;
            if flags & bare == bare && count > w.remaining() {
                return Err(malformed("vertex count"));
            }
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

/// A mesh's material channels (section 0x4000), apart from the mesh so that a bad one costs only the channels:
/// count, then per channel its material, flags, source and destination blend, and its texture coordinates (a count,
/// then that many pairs: none or one per vertex of the mesh's `vertices`).
pub fn mesh_channels(chunk: &Chunk, vertices: usize) -> Result<Vec<Channel>> {
    let mut out = Vec::new();
    if let Some(mut w) = chunk.section(0x0000_4000) {
        let n = w.u32()? as usize;
        if n > MAX_CHANNELS {
            return Err(malformed("channel count"));
        }
        for _ in 0..n {
            let material = reference(chunk, w.u32()?);
            let flags = w.u32()?;
            let src_blend = Blend::from_nibble(w.u32()?);
            let dest_blend = Blend::from_nibble(w.u32()?);
            let count = w.u32()? as usize;
            if count != 0 && count != vertices {
                return Err(malformed("channel uv count"));
            }
            let words = w.take(count.checked_mul(2).ok_or(malformed("channel uv data"))?)?;
            let uvs = words.as_chunks::<2>().0.iter().map(|c| uv2(c)).collect();
            out.push(Channel {
                material,
                flags,
                src_blend,
                dest_blend,
                uvs,
            });
        }
    }
    Ok(out)
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
    /// The colour (0xRRGGBB) its texels are transparent in, when it has one (CKTexture's transparent colour).
    pub transparent: Option<u32>,
}

pub fn texture(chunk: &Chunk) -> Result<Texture> {
    let mut tex = Texture {
        file_name: None,
        embedded: None,
        transparent: None,
    };
    let mut slot_count = 0u32;
    if let Some(mut w) = chunk.section(0x0001_0000) {
        let slots = w.u32()?;
        slot_count = slots;
        if slots > 0 {
            let name = w.string()?;
            if !name.is_empty() {
                tex.file_name = Some(name);
            }
        }
    }
    // The raw bitmaps (section 0x20000): a slot count, then the first slot's bits per pixel, width, height, the
    // channel masks (A, R, G, B), a compression word (0: none) and the channels one after another, each a whole
    // plane of width x height bytes behind its size in bytes, padded to a word: blue, green, red, alpha, rows bottom
    // up. (Found in the bytes of 8058 textures of 606 community maps: every one has four planes of exactly
    // width x height bytes and compression 0, and the copies of the game's own textures that maps carry are pixel
    // for pixel the game's pictures when read so.)
    if let Some(mut w) = chunk.section(0x0002_0000) {
        let slots = w.u32()?;
        if slots > 0 && w.remaining() >= 9 {
            let bpp = w.u32()?;
            let (width, height) = (w.u32()?, w.u32()?);
            let _masks = w.take(4)?;
            let compression = w.u32()?;
            // at least one pixel a side: a zero-sized picture is no picture (the renderer could not make a texture of it)
            let side = 1..=MAX_TEXTURE_SIDE;
            if bpp == 32 && compression == 0 && side.contains(&width) && side.contains(&height) {
                let pixels = (width as usize).saturating_mul(height as usize);
                if let Some(rgba) = unpack_planes(&mut w, width as usize, pixels) {
                    tex.embedded = Some((width, height, rgba));
                }
            }
        }
    }
    // The load options (section 0x2ff000): a packed word (0x100 transparent, 0x200 a video format follows), then,
    // by the section's size, the transparent colour, as CKTexture::Load reads it (C reference ck_3d.c).
    if let Some(mut w) = chunk.section(0x002F_F000) {
        let size = w.remaining().saturating_sub(1).saturating_mul(4);
        let mix = w.u32()?;
        let colour = match size {
            12 => Some(w.u32()?),
            8 if slot_count < 2 || mix & 0x200 == 0 => Some(w.u32()?),
            4 if mix & 0x200 == 0 && slot_count < 2 => Some(w.u32()?),
            _ => None,
        };
        if mix & 0x100 != 0 {
            tex.transparent = Some(colour.unwrap_or(0) & 0x00FF_FFFF);
        }
    }
    Ok(tex)
}

/// The channel planes of a raw bitmap (blue, green, red, alpha; each its size in bytes, then its bytes padded to a
/// word) to RGBA bytes. Every plane has exactly `pixels` bytes, or the picture is refused (`None`); a bitmap that
/// ends after its red plane, or whose alpha plane is empty, is opaque. Nothing is allocated before the three
/// colour planes are known to be in the file, so a picture is never larger than four times the words that hold it.
fn unpack_planes(w: &mut Words<'_>, width: usize, pixels: usize) -> Option<Vec<u8>> {
    let mut plane = |optional: bool| -> Option<Option<&[u32]>> {
        if optional && w.remaining() == 0 {
            return Some(None);
        }
        let size = w.u32().ok()? as usize;
        if optional && size == 0 {
            return Some(None);
        }
        if size != pixels {
            return None;
        }
        w.take(size.div_ceil(4)).ok().map(Some)
    };
    let blue = plane(false)??;
    let green = plane(false)??;
    let red = plane(false)??;
    let alpha = plane(true)?;
    // the byte of pixel `i` in a plane: the file is little-endian, so the first byte of a word is its lowest
    let byte = |plane: &[u32], i: usize| -> u8 {
        let word = plane.get(i / 4).copied().unwrap_or(0).to_le_bytes();
        word.get(i % 4).copied().unwrap_or(0)
    };
    // the file's first row is the picture's bottom one: the rows are turned over, so that the first is the top
    let mut rgba = Vec::with_capacity(pixels.checked_mul(4)?);
    for row in (0..pixels.checked_div(width)?).rev() {
        let start = row.checked_mul(width)?;
        for i in start..start.checked_add(width)? {
            rgba.extend([
                byte(red, i),
                byte(green, i),
                byte(blue, i),
                alpha.map_or(0xFF, |a| byte(a, i)),
            ]);
        }
    }
    Some(rgba)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Row-major world matrix as Virtools stores it: three axis rows, then the position.
    pub matrix: [[f32; 3]; 4],
    /// The mesh drawn for this entity (3D objects only).
    pub mesh: Option<usize>,
    pub flags: u32,
    /// The entity it is a child of (an index into the document's objects), when the file names one.
    pub parent: Option<usize>,
}

pub fn entity(chunk: &Chunk) -> Result<Entity> {
    let mut w = chunk.section(0x0010_0000).ok_or(missing("entity matrix"))?;
    let first = w.u32()?;
    let flags = w.u32()?;
    let m: Vec<[f32; 3]> = w.take(12)?.chunks_exact(3).map(xyz).collect();
    let row = |i: usize| m.get(i).copied().unwrap_or_default();
    // after the matrix: 0x10000 an object to skip, 0x20000 the parent (CKRenderObject's save order)
    // (a short chunk simply has none: a missing word is no parent, never a lost object)
    if first & 0x10000 != 0 {
        let _ = w.u32();
    }
    let parent = if first & 0x20000 != 0 {
        w.u32().ok().and_then(|r| reference(chunk, r))
    } else {
        None
    };
    let mesh = match chunk.section(0x4000) {
        Some(mut w) => reference(chunk, w.u32()?),
        None => None,
    };
    Ok(Entity {
        parent,
        matrix: [row(0), row(1), row(2), row(3)],
        mesh,
        flags,
    })
}

/// A 3D curve (CKCurve, class 43; section 0xffc00000 of data version 5 and later): its control points in order,
/// the fitting coefficient, and whether it is open.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub points: Vec<usize>,
    pub fitting: f32,
    pub open: bool,
}

/// Control points a curve may have.
const MAX_CURVE_POINTS: usize = 4096;

pub fn curve(chunk: &Chunk) -> Result<Option<Curve>> {
    if chunk.data_version < 5 {
        return Ok(None);
    }
    let Some(mut w) = chunk.section(0xffc0_0000) else {
        return Ok(None);
    };
    let n = w.u32()? as usize;
    if n > MAX_CURVE_POINTS || n > w.remaining() {
        return Err(malformed("curve point count"));
    }
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        if let Some(r) = reference(chunk, w.u32()?) {
            points.push(r);
        }
    }
    let fitting = w.f32()?;
    if !fitting.is_finite() {
        return Err(malformed("curve fitting"));
    }
    w.u32()?; // the step count of its line mesh
    let open = w.u32()? != 0;
    Ok(Some(Curve {
        points,
        fitting,
        open,
    }))
}

/// A curve's control point (CKCurvePoint, class 36; section 0x10000000 of data version 5 and later): TCB or
/// its own tangents, whether the segment leaving it is straight, tension, continuity and bias, and its saved in
/// and out tangents (curve-local).
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePoint {
    pub tcb: bool,
    pub linear: bool,
    pub tcb_values: [f32; 3],
    pub tangents: [[f32; 3]; 2],
}

pub fn curve_point(chunk: &Chunk) -> Result<Option<CurvePoint>> {
    if chunk.data_version < 5 {
        return Ok(None);
    }
    let Some(mut w) = chunk.section(0x1000_0000) else {
        return Ok(None);
    };
    w.u32()?; // the curve
    let tcb = w.u32()? == 0;
    let linear = w.u32()? != 0;
    let mut f = || -> Result<f32> {
        let v = w.f32()?;
        if v.is_finite() {
            Ok(v)
        } else {
            Err(malformed("curve point value"))
        }
    };
    let tcb_values = [f()?, f()?, f()?];
    let tangents = [[f()?, f()?, f()?], [f()?, f()?, f()?]];
    Ok(Some(CurvePoint {
        tcb,
        linear,
        tcb_values,
        tangents,
    }))
}

/// An object animation (CKObjectAnimation, class 15; section 0x4000000 of data version 1 and later,
/// docs/animation.md 5.2): the entity it moves, its length in frames, and the keys of its linear position,
/// rotation (quaternion x, y, z, w) and scale controllers. Other controllers (TCB, Bezier, scale axis, morph) are
/// skipped by their size; merged animations are left out.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectAnimation {
    pub entity: Option<usize>,
    pub flags: u32,
    pub length: f32,
    pub position: Vec<(f32, [f32; 3])>,
    pub rotation: Vec<(f32, [f32; 4])>,
    pub scale: Vec<(f32, [f32; 3])>,
}

/// Keys one controller may have.
const MAX_KEYS: usize = 1 << 20;
/// Controllers one animation may list.
const MAX_CONTROLLERS: usize = 64;

pub fn object_animation(chunk: &Chunk) -> Result<Option<ObjectAnimation>> {
    if chunk.data_version < 1 {
        return Ok(None);
    }
    let Some(mut w) = chunk.section(0x0400_0000) else {
        return Ok(None);
    };
    w.take(7)?; // a vector and four unused floats
    let flags = w.u32()?;
    let entity = reference(chunk, w.u32()?);
    let length = w.f32()?;
    if !length.is_finite() || length < 0.0 {
        return Err(malformed("animation length"));
    }
    if flags & 0x80 != 0 {
        w.take(3)?; // merge factor and the two merged animations
    }
    let mut anim = ObjectAnimation {
        entity,
        flags,
        length,
        position: Vec::new(),
        rotation: Vec::new(),
        scale: Vec::new(),
    };
    for _ in 0..MAX_CONTROLLERS {
        let kind = w.u32()?;
        if kind == 0 {
            return Ok(Some(anim));
        }
        let size = w.u32()? as usize;
        let block = w.take(size)?;
        // the key's words: time and three values (position, scale) or four (rotation)
        let width = match kind {
            0x637c_4301 | 1 | 0x654a_3a04 | 4 => 4,
            0x49ed_4002 | 2 => 5,
            _ => continue,
        };
        let (&count, keys) = block.split_first().ok_or(malformed("animation keys"))?;
        let count = count as usize;
        if count > MAX_KEYS || count.checked_mul(width) != Some(keys.len()) {
            return Err(malformed("animation key count"));
        }
        let mut finite = true;
        let mut read = |k: &[u32]| -> Vec<f32> {
            let v: Vec<f32> = k.iter().map(|&x| f32::from_bits(x)).collect();
            finite &= v.iter().all(|x| x.is_finite());
            v
        };
        for k in keys.chunks_exact(width) {
            let v = read(k);
            match (kind, v.as_slice()) {
                (0x637c_4301 | 1, &[t, x, y, z]) => anim.position.push((t, [x, y, z])),
                (0x654a_3a04 | 4, &[t, x, y, z]) => anim.scale.push((t, [x, y, z])),
                (_, &[t, x, y, z, q]) => anim.rotation.push((t, [x, y, z, q])),
                _ => {}
            }
        }
        if !finite {
            return Err(malformed("animation key"));
        }
    }
    Err(malformed("animation controller count"))
}

/// A camera's lens (section 0xfc00000 of data version 5 and later: projection, field of view, orthographic zoom,
/// aspect, near and far planes) and, for a target camera (class 35), its target (section 0x10000000).
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub target: Option<usize>,
}

pub fn camera(chunk: &Chunk) -> Result<Camera> {
    let mut cam = Camera {
        fov: 0.5,
        near: 1.0,
        far: 4000.0,
        target: None,
    };
    if chunk.data_version >= 5
        && let Some(mut w) = chunk.section(0x0fc0_0000)
    {
        w.u32()?;
        let fov = w.f32()?;
        w.u32()?;
        w.u32()?;
        let (near, far) = (w.f32()?, w.f32()?);
        if [fov, near, far].iter().all(|v| v.is_finite()) {
            (cam.fov, cam.near, cam.far) = (fov, near, far);
        }
    }
    if u32::from(chunk.class_id) == crate::class::TARGET_CAMERA
        && let Some(mut w) = chunk.section(0x1000_0000)
    {
        cam.target = reference(chunk, w.u32()?);
    }
    Ok(cam)
}

/// A CKSprite3D's own part (section 0x400000): how it turns toward the view (0 facing it, 1 about its x, 2 about
/// its y, 3 as placed), its size (stored as half), the quad's offset in its plane (-1..1 of the half size), the
/// texture rectangle (u0, v0, u1, v1) and its material.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub mode: u32,
    pub size: [f32; 2],
    pub offset: [f32; 2],
    pub uv: [f32; 4],
    pub material: Option<usize>,
}

/// The sprite's part, or the constructor's values (2 × 2, no offset, the whole texture) when the file has none.
pub fn sprite(chunk: &Chunk) -> Result<Sprite> {
    let mut s = Sprite {
        mode: 0,
        size: [2.0; 2],
        offset: [0.0; 2],
        uv: [0.0, 0.0, 1.0, 1.0],
        material: None,
    };
    let Some(mut w) = chunk.section(0x0040_0000) else {
        return Ok(s);
    };
    s.mode = w.u32()?;
    let finite = |v: f32| {
        if v.is_finite() {
            Ok(v)
        } else {
            Err(malformed("sprite value"))
        }
    };
    s.size = [finite(w.f32()? * 2.0)?, finite(w.f32()? * 2.0)?];
    s.offset = [finite(w.f32()?)?, finite(w.f32()?)?];
    for v in &mut s.uv {
        *v = finite(w.f32()?)?;
    }
    s.material = reference(chunk, w.u32()?);
    Ok(s)
}

/// A 2D entity (CK2dEntity, classes 27 to 29; section 0x10f000, C reference `load_2dentity`): a rectangle of the
/// screen drawn over the scene with a material. The rectangle is left, top, right, bottom: fractions of the
/// view when `homogeneous`, else pixels of the 640 x 480 render context. `source` is the part of the texture it
/// shows (u0, v0, u1, v1), used when `use_source`.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity2d {
    pub flags: u32,
    pub rect: [f32; 4],
    pub source: [f32; 4],
    pub z_order: i32,
    pub material: Option<usize>,
}

impl Entity2d {
    pub const USE_SOURCE: u32 = 0x20;
    pub const HOMOGENEOUS: u32 = 0x200;

    pub fn homogeneous(&self) -> bool {
        self.flags & Self::HOMOGENEOUS != 0
    }

    pub fn use_source(&self) -> bool {
        self.flags & Self::USE_SOURCE != 0
    }
}

/// The 2D entity's part, or the constructor's values (no rectangle, the whole texture, no material) when the file
/// has none. A plain frame (class 27) of an older file names its material in section 0x200000.
pub fn entity_2d(chunk: &Chunk) -> Result<Entity2d> {
    let mut e = Entity2d {
        flags: 0,
        rect: [0.0; 4],
        source: [0.0, 0.0, 1.0, 1.0],
        z_order: 0,
        material: None,
    };
    let frame = u32::from(chunk.class_id) == crate::class::ENTITY_2D;
    if let Some(mut w) = chunk.section(0x0010_f000) {
        let saved = w.u32()?;
        // the bits that say what follows are not the entity's flags
        e.flags = saved & 0xfff8_f7ff;
        let four = |w: &mut Words<'_>| -> Result<[f32; 4]> {
            let mut v = [0.0f32; 4];
            for x in &mut v {
                *x = w.f32()?;
                if !x.is_finite() {
                    return Err(malformed("2D entity rectangle"));
                }
            }
            Ok(v)
        };
        e.rect = four(&mut w)?;
        if saved & 0x1_0000 != 0 {
            e.source = four(&mut w)?;
        }
        if saved & 0x2_0000 != 0 {
            e.z_order = w.u32()? as i32;
        }
        if saved & 0x4_0000 != 0 {
            e.material = reference(chunk, w.u32()?);
        }
    }
    if frame && let Some(mut w) = chunk.section(0x0020_0000) {
        e.material = reference(chunk, w.u32()?);
    }
    Ok(e)
}

/// A wave sound (CKWaveSound, class 25; C reference `ck_sound_load_chunk`): the file it plays (CKSound's section
/// 0x1000, or 0x100000 when present) and, from data version 3, its flags (0x8 loops), gain, pan and pitch.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveSound {
    pub file_name: Option<Vec<u8>>,
    pub looped: bool,
    pub gain: f32,
    pub pan: f32,
    pub pitch: f32,
}

/// A sound's file name is a path; longer than this is no name.
const MAX_SOUND_NAME: usize = 1_024;

pub fn wave_sound(chunk: &Chunk) -> Result<WaveSound> {
    let mut s = WaveSound {
        file_name: None,
        looped: false,
        gain: 1.0,
        pan: 0.0,
        pitch: 1.0,
    };
    let named = |name: Vec<u8>| -> Result<Option<Vec<u8>>> {
        if name.len() > MAX_SOUND_NAME {
            return Err(malformed("sound file name"));
        }
        Ok((!name.is_empty()).then_some(name))
    };
    if let Some(mut w) = chunk.section(0x1000) {
        w.u32()?;
        s.file_name = named(w.string()?)?;
    }
    if let Some(mut w) = chunk.section(0x0010_0000) {
        s.file_name = named(w.string()?)?;
    }
    if chunk.data_version >= 3
        && let Some(mut w) = chunk.section(0x0040_0000)
    {
        let flags = w.u32()?;
        w.u32()?; // priority
        let (gain, pan, pitch) = (w.f32()?, w.f32()?, w.f32()?);
        if ![gain, pan, pitch].iter().all(|v| v.is_finite()) {
            return Err(malformed("sound value"));
        }
        s.looped = flags & 0x8 != 0;
        (s.gain, s.pan, s.pitch) = (gain, pan, pitch);
    }
    Ok(s)
}

/// A material's effect (section 0x10000), apart from the material so that a bad one costs only the effect: its
/// kind (1 texture coordinate generation, 2 the same with a referential) and the parameter object holding its
/// setting.
pub fn material_effect(chunk: &Chunk) -> Result<Option<(u32, Option<usize>)>> {
    let Some(mut e) = chunk.section(0x0001_0000) else {
        return Ok(None);
    };
    let param = reference(chunk, e.u32()?);
    Ok(Some((e.u32()?, param)))
}

/// A parameter object's value as one word (section 0x40: type GUID, a word, the size in bytes, the value), when it
/// is four bytes.
pub fn parameter_word(chunk: &Chunk) -> Result<Option<u32>> {
    let Some(mut w) = chunk.section(0x40) else {
        return Ok(None);
    };
    w.take(2)?;
    w.u32()?;
    let size = w.u32()?;
    Ok(if size == 4 { Some(w.u32()?) } else { None })
}

/// A parameter object's value as floats (a colour, a vector), when it is 1 to 16 whole words and every one a finite
/// number.
pub fn parameter_floats(chunk: &Chunk) -> Result<Option<Vec<f32>>> {
    let Some(mut w) = chunk.section(0x40) else {
        return Ok(None);
    };
    w.take(2)?;
    w.u32()?;
    let size = w.u32()? as usize;
    if size == 0 || !size.is_multiple_of(4) || size > 64 {
        return Ok(None);
    }
    let v: Vec<f32> = w
        .take(size / 4)?
        .iter()
        .map(|&x| f32::from_bits(x))
        .collect();
    Ok(v.iter().all(|x| x.is_finite()).then_some(v))
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
    /// Each column's kind as stored: 1 integer, 2 float, 3 string, 4 object, 5 parameter.
    pub kinds: Vec<u32>,
    pub rows: Vec<Vec<Cell>>,
    /// The key column (identifier `0x4000`: sort order, sort column, key column), if the array has one.
    pub key_column: Option<usize>,
}

impl DataArray {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(name.as_bytes()))
    }
}

/// The most attributes one object is read with (the run caps them lower).
pub const MAX_ATTRIBUTES: usize = 256;

/// The attributes a behavioural object carries (identifier `0x11`, docs/ck-runtime.md, "Attributes"): the file's
/// attribute index and the parameter object holding its value, in the order saved. The layout is the count *n*,
/// *n* parameter references, the count again, the attribute manager's GUID (two words), then *n* attribute indices.
/// Nothing here can fail the object or the file: a short, inconsistent or oversized section gives no attributes.
pub fn attributes(chunk: &Chunk) -> Vec<(u32, Option<usize>)> {
    let read = || -> Option<Vec<(u32, Option<usize>)>> {
        let mut w = chunk.section(0x11)?;
        let n = w.u32().ok()? as usize;
        // n references, the count, two GUID words and n indices must all be there
        if n == 0 || n > MAX_ATTRIBUTES || n.checked_mul(2)?.checked_add(3)? > w.remaining() {
            return None;
        }
        let refs: Vec<u32> = (0..n).map(|_| w.u32().ok()).collect::<Option<_>>()?;
        if w.u32().ok()? as usize != n {
            return None;
        }
        w.u32().ok()?;
        w.u32().ok()?;
        let idx: Vec<u32> = (0..n).map(|_| w.u32().ok()).collect::<Option<_>>()?;
        Some(
            idx.into_iter()
                .zip(refs)
                .map(|(i, r)| (i, reference(chunk, r)))
                .collect(),
        )
    };
    read().unwrap_or_default()
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
    // a missing or short 0x4000 means no key column; a negative one (-1) too
    let key_column = chunk.section(0x4000).and_then(|mut w| {
        w.u32().ok()?;
        w.u32().ok()?;
        usize::try_from(w.u32().ok()? as i32).ok()
    });
    Ok(DataArray {
        columns,
        kinds,
        rows,
        key_column,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;
    use crate::chunk::OPTION_FILE;

    /// A chunk of one identifier's section holding `data`.
    fn chunk(sections: &[(u32, Vec<u32>)]) -> Chunk {
        let mut words = Vec::new();
        for (k, (id, data)) in sections.iter().enumerate() {
            let next = words.len() + 2 + data.len();
            words.push(*id);
            words.push(if k + 1 == sections.len() {
                0
            } else {
                next as u32
            });
            words.extend(data);
        }
        Chunk {
            data_version: 7,
            class_id: 0,
            chunk_version: 7,
            options: OPTION_FILE,
            words,
        }
    }

    /// An array's key column comes from identifier 0x4000 (sort order, sort column, key column); -1, a short
    /// section or none at all mean no key column, and never fail the array.
    #[test]
    fn a_data_array_keeps_its_key_column() {
        let key = u32::from_le_bytes(*b"Key\0");
        let columns = vec![2, 4, key, 1, 4, key, 1];
        let rows = vec![1, 10, 11];
        let with = |extra: Option<Vec<u32>>| {
            let mut s = vec![(0x1000, columns.clone()), (0x2000, rows.clone())];
            if let Some(e) = extra {
                s.push((0x4000, e));
            }
            data_array(&chunk(&s)).map(|a| (a.rows.len(), a.key_column))
        };
        assert_eq!(with(Some(vec![0, 0, 1])).ok(), Some((1, Some(1))));
        assert_eq!(with(Some(vec![0, 0, u32::MAX])).ok(), Some((1, None)));
        assert_eq!(with(Some(vec![0, 0])).ok(), Some((1, None)));
        assert_eq!(with(None).ok(), Some((1, None)));
    }

    /// Attributes (identifier 0x11): count, references, count, manager GUID, indices; anything off gives none.
    #[test]
    fn attributes_are_read_defensively() {
        let good = vec![2, 0x385, 0x386, 2, 0x3d24_2466, 0, 0x29, 0x41];
        let read = |w: Vec<u32>| attributes(&chunk(&[(0x11, w)]));
        assert_eq!(
            read(good.clone()),
            vec![(0x29, Some(0x385)), (0x41, Some(0x386))]
        );
        assert_eq!(read(vec![1, u32::MAX, 1, 7, 0, 5]), vec![(5, None)]);
        // short, a count that disagrees, no attributes, a count beyond what is there or beyond the cap
        assert!(read(good[..7].to_vec()).is_empty());
        let mut bad = good.clone();
        bad[3] = 3;
        assert!(read(bad).is_empty());
        assert!(read(vec![0]).is_empty());
        assert!(read(vec![u32::MAX, 1, 2, 3]).is_empty());
        assert!(read(vec![1_000_000, 1, 1, 0, 0, 5]).is_empty());
        assert!(attributes(&chunk(&[(0x10, good)])).is_empty());
    }

    /// Record Anim's layout (docs/animation.md 5.2): linear position, rotation and scale, then the end of the list;
    /// a TCB controller between them is skipped by its size.
    #[test]
    fn an_object_animation_is_read_and_bad_ones_refused() {
        let f = f32::to_bits;
        let head = |rest: &[u32]| {
            let mut v = vec![0, 0, 0, 0, 0, 0, 0, 0, 5, f(100.0)];
            v.extend_from_slice(rest);
            chunk(&[(0x0400_0000, v)])
        };
        let ok = head(&[
            0x637c_4301,
            9,
            2,
            f(0.0),
            f(1.0),
            f(2.0),
            f(3.0),
            f(100.0),
            f(4.0),
            f(5.0),
            f(6.0),
            0x347e_4a01,
            3,
            1,
            0,
            0,
            0x49ed_4002,
            6,
            1,
            f(0.0),
            f(0.0),
            f(0.0),
            f(0.0),
            f(1.0),
            0x654a_3a04,
            5,
            1,
            f(0.0),
            f(1.0),
            f(1.0),
            f(1.0),
            0,
        ]);
        let a = object_animation(&ok).expect("animation").expect("its part");
        assert_eq!((a.entity, a.length), (Some(5), 100.0));
        assert_eq!(
            a.position,
            vec![(0.0, [1.0, 2.0, 3.0]), (100.0, [4.0, 5.0, 6.0])]
        );
        assert_eq!(a.rotation, vec![(0.0, [0.0, 0.0, 0.0, 1.0])]);
        assert_eq!(a.scale, vec![(0.0, [1.0; 3])]);
        let disagree = head(&[0x637c_4301, 9, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(
            object_animation(&disagree).is_err(),
            "count and size disagree"
        );
        assert!(
            object_animation(&head(&[0x637c_4301, 5, 1, f(f32::NAN), 0, 0, 0, 0])).is_err(),
            "not a number"
        );
        assert!(
            object_animation(&head(&[0x637c_4301, 1000, 1])).is_err(),
            "cut short"
        );
        assert!(
            object_animation(&head(&[0x1234, 0])).is_err(),
            "no end to the list"
        );
        assert_eq!(
            object_animation(&chunk(&[(0x4, vec![])])).expect("none"),
            None
        );
    }

    #[test]
    fn a_sprite_is_read_and_one_without_its_part_takes_the_defaults() {
        let f = f32::to_bits;
        let c = chunk(&[(
            0x0040_0000,
            vec![
                2,
                f(0.25),
                f(0.5),
                f(0.1),
                f(-0.2),
                f(0.0),
                f(0.5),
                f(1.0),
                f(0.75),
                9,
            ],
        )]);
        let s = sprite(&c).expect("sprite");
        assert_eq!(s.mode, 2);
        assert_eq!(s.size, [0.5, 1.0]);
        assert_eq!(s.offset, [0.1, -0.2]);
        assert_eq!(s.uv, [0.0, 0.5, 1.0, 0.75]);
        assert_eq!(s.material, Some(9));
        let d = sprite(&chunk(&[(0x4, vec![])])).expect("defaults");
        assert_eq!((d.size, d.uv), ([2.0; 2], [0.0, 0.0, 1.0, 1.0]));
    }

    /// Level 1's dome as saved: one channel with material 6, active, ZERO / SRCCOLOR, one UV pair per vertex.
    #[test]
    fn a_mesh_s_material_channels_are_read_and_bad_ones_refused() {
        let f = f32::to_bits;
        let verts = (
            0x0002_0000,
            vec![2, 0x10 | 0x4 | 0x1 | 0x2, 0, 0, 0, 0, 0, 0, 0],
        );
        let ok = chunk(&[
            (
                0x0000_4000,
                vec![1, 6, 0x0200_0001, 1, 3, 2, f(0.5), f(0.25), f(0.75), f(1.0)],
            ),
            verts.clone(),
        ]);
        let m = mesh(&ok).expect("mesh");
        assert_eq!(m.positions.len(), 2);
        let chans = mesh_channels(&ok, 2).expect("channels");
        let c = chans.first().expect("a channel");
        assert_eq!(
            (c.material, c.flags, c.src_blend, c.dest_blend),
            (Some(6), 0x0200_0001, Blend::Zero, Blend::SrcColor)
        );
        assert_eq!(c.uvs, [[0.5, 0.25], [0.75, 1.0]]);
        // no coordinates of its own: allowed
        assert!(mesh_channels(&chunk(&[(0x0000_4000, vec![1, 6, 1, 1, 3, 0])]), 2).is_ok());
        // a count that is neither 0 nor the vertex count, data cut short, too many channels; the mesh stays
        assert!(
            mesh_channels(
                &chunk(&[(0x0000_4000, vec![1, 6, 1, 1, 3, 1, f(0.5), f(0.5)])]),
                2
            )
            .is_err()
        );
        assert!(
            mesh_channels(&chunk(&[(0x0000_4000, vec![1, 6, 1, 1, 3, 2, f(0.5)])]), 2).is_err()
        );
        let many = chunk(&[(0x0000_4000, vec![u32::MAX]), verts]);
        assert!(mesh_channels(&many, 2).is_err());
        assert!(mesh(&many).is_ok());
    }

    /// A curve, its points and a target camera as MenuLevel.nmo saves them, and broken ones refused.
    #[test]
    fn curves_points_and_cameras_are_read_and_bad_ones_refused() {
        let f = f32::to_bits;
        let v10 = |sections: &[(u32, Vec<u32>)]| Chunk {
            data_version: 10,
            ..chunk(sections)
        };
        let c = v10(&[(0xffc0_0000, vec![2, 7, 8, f(0.0), 100, 0])]);
        assert_eq!(
            curve(&c).expect("curve"),
            Some(Curve {
                points: vec![7, 8],
                fitting: 0.0,
                open: false
            })
        );
        assert!(
            curve(&v10(&[(0xffc0_0000, vec![5000])])).is_err(),
            "too many points"
        );
        assert!(
            curve(&v10(&[(0xffc0_0000, vec![2, 7, 8])])).is_err(),
            "cut short"
        );
        assert!(
            curve(&v10(&[(0xffc0_0000, vec![1, 7, f(f32::NAN), 100, 0])])).is_err(),
            "fitting not a number"
        );
        let old = Chunk {
            data_version: 4,
            ..chunk(&[(0xffc0_0000, vec![0, f(0.0), 1, 0])])
        };
        assert_eq!(
            curve(&old).expect("old"),
            None,
            "data version 4: another layout"
        );
        let p = v10(&[(
            0x1000_0000,
            vec![3, 0, 0, f(-0.6), f(0.0), f(0.0), 0, 0, 0, 0, 0, 0],
        )]);
        let cp = curve_point(&p).expect("point").expect("its part");
        assert!(cp.tcb && !cp.linear && cp.tcb_values == [-0.6, 0.0, 0.0]);
        assert!(curve_point(&v10(&[(0x1000_0000, vec![3, 0, 0, f(f32::INFINITY)])])).is_err());
        let cam = Chunk {
            class_id: 35,
            ..v10(&[
                (0x0fc0_0000, vec![0, f(0.95), f(1.0), 0, f(20.0), f(550.0)]),
                (0x1000_0000, vec![4]),
            ])
        };
        assert_eq!(
            camera(&cam).expect("camera"),
            Camera {
                fov: 0.95,
                near: 20.0,
                far: 550.0,
                target: Some(4)
            }
        );
        assert!(
            camera(&Chunk {
                class_id: 35,
                ..v10(&[(0x0fc0_0000, vec![0, f(0.95)])])
            })
            .is_err(),
            "cut short"
        );
    }

    /// The dome's material effect (TexGen, its parameter object 4) and that parameter's value (3, Chrome).
    #[test]
    fn a_material_effect_and_a_parameter_word_are_read() {
        let mat = chunk(&[(0x1000, vec![0; 9]), (0x0001_0000, vec![4, 1])]);
        assert_eq!(material_effect(&mat).expect("effect"), Some((1, Some(4))));
        assert_eq!(
            material_effect(&chunk(&[(0x1000, vec![0; 9])])).expect("plain"),
            None
        );
        assert!(material_effect(&chunk(&[(0x1000, vec![0; 9]), (0x0001_0000, vec![4])])).is_err());
        let par = chunk(&[(0x40, vec![0x7dbc_28f5, 0x3f16_1e80, 1, 4, 3])]);
        assert_eq!(parameter_word(&par).expect("parameter"), Some(3));
        assert_eq!(
            parameter_word(&chunk(&[(0x40, vec![0, 0, 1, 8, 1, 2])])).expect("wide"),
            None
        );
        assert!(parameter_word(&chunk(&[(0x40, vec![0, 0, 1, 4])])).is_err());
    }

    #[test]
    fn a_broken_sprite_is_refused() {
        let short = chunk(&[(0x0040_0000, vec![0, 0])]);
        assert!(sprite(&short).is_err());
        let nan = chunk(&[(
            0x0040_0000,
            vec![0, f32::NAN.to_bits(), 0, 0, 0, 0, 0, 0, 0, 0],
        )]);
        assert!(sprite(&nan).is_err());
    }

    #[test]
    fn a_texture_s_transparent_colour_is_read_as_its_options_say() {
        let with = chunk(&[
            (0x0001_0000, vec![1, 0]),
            (0x002F_F000, vec![0x101, 0xFF00_0000, 0, 0]),
        ]);
        // no file name slot words here: the 0x10000 section above is only a slot count of 1 and an empty string
        assert_eq!(texture(&with).expect("texture").transparent, Some(0));
        let magenta = chunk(&[(0x002F_F000, vec![0x100, 0x00FF_00FF])]);
        assert_eq!(
            texture(&magenta).expect("texture").transparent,
            Some(0x00FF_00FF)
        );
        let none = chunk(&[(0x002F_F000, vec![0x001, 0x00FF_00FF])]);
        assert_eq!(texture(&none).expect("texture").transparent, None);
    }

    /// A raw bitmap as a file holds it: masks, no compression, then a plane a channel, each behind its size in bytes.
    fn raw_bitmap(w: u32, h: u32, planes: &[&[u8]]) -> Vec<u32> {
        let mut data = vec![1, 32, w, h];
        data.extend([0xFF00_0000, 0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0]);
        for plane in planes {
            data.push(plane.len() as u32);
            data.extend(plane.chunks(4).map(|c| {
                let mut word = [0u8; 4];
                word[..c.len()].copy_from_slice(c);
                u32::from_le_bytes(word)
            }));
        }
        data
    }

    fn embedded(data: Vec<u32>) -> Option<(u32, u32, Vec<u8>)> {
        texture(&chunk(&[(0x0002_0000, data)]))
            .expect("texture")
            .embedded
    }

    /// Fuzzing found it (nmo_chunk): an embedded picture 8192 wide and 0 high (no pixels, so the count matched) was
    /// kept, and the renderer cannot make a texture of no size. A side of 0 is no embedded picture.
    #[test]
    fn an_embedded_picture_without_pixels_is_none() {
        let pictured = |w: u32, h: u32| {
            let plane = vec![0x10u8; (w * h) as usize];
            embedded(raw_bitmap(w, h, &[&plane, &[0x20], &[0x40], &[0x80]]))
        };
        assert_eq!(pictured(8192, 0), None);
        assert_eq!(pictured(0, 5), None);
        assert_eq!(pictured(0, 0), None);
        assert_eq!(pictured(1, 1), Some((1, 1, vec![0x40, 0x20, 0x10, 0x80])));
    }

    /// The layout of the community maps' own pictures (606 maps, 8058 textures): the channels are whole planes one
    /// after another, blue, green, red, alpha, each of width x height bytes with rows bottom up and padded to a
    /// word; not packed pixels (read as those, a frame came out four times across in three bands and a blank one).
    #[test]
    fn an_embedded_picture_is_read_from_its_channel_planes() {
        // 3 x 2, so a plane of 6 bytes ends inside a word: the padding is skipped, not read as the next plane
        let blue = [1, 2, 3, 4, 5, 6];
        let green = [11, 12, 13, 14, 15, 16];
        let red = [21, 22, 23, 24, 25, 26];
        let alpha = [31, 32, 33, 34, 35, 36];
        let (w, h, rgba) =
            embedded(raw_bitmap(3, 2, &[&blue, &green, &red, &alpha])).expect("a picture");
        assert_eq!((w, h), (3, 2));
        // the file's first row is the bottom one: the second row's pixels (3, 4, 5) come first, each left to right
        let order = [3u8, 4, 5, 0, 1, 2];
        let expected: Vec<u8> = order
            .iter()
            .flat_map(|i| [21 + i, 11 + i, 1 + i, 31 + i])
            .collect();
        assert_eq!(rgba, expected);
        assert_eq!(rgba.get(..4), Some(&[24, 14, 4, 34][..]));

        // no alpha plane (the bitmap ends, or the plane is empty): opaque
        let opaque: Vec<u8> = order
            .iter()
            .flat_map(|i| [21 + i, 11 + i, 1 + i, 0xFF])
            .collect();
        let ended = embedded(raw_bitmap(3, 2, &[&blue, &green, &red])).expect("a picture");
        assert_eq!(ended.2, opaque);
        let empty = embedded(raw_bitmap(3, 2, &[&blue, &green, &red, &[]])).expect("a picture");
        assert_eq!(empty.2, opaque);
    }

    /// A hostile bitmap is no picture, and never a panic or a buffer of the size it claims: a plane of another size
    /// than width x height, planes cut short, a compressed bitmap, another depth, a size no file could hold.
    #[test]
    fn a_broken_embedded_picture_is_none() {
        let p = [7u8; 6];
        assert_eq!(embedded(raw_bitmap(3, 2, &[&p, &p[..5], &p, &p])), None);
        assert_eq!(embedded(raw_bitmap(3, 2, &[&p, &p, &[7u8; 7], &p])), None);
        assert_eq!(embedded(raw_bitmap(3, 2, &[&p, &p, &p, &p[..3]])), None);
        assert_eq!(embedded(raw_bitmap(3, 2, &[&p, &p])), None);
        assert_eq!(embedded(raw_bitmap(3, 2, &[])), None);
        // a plane that says its size and has no bytes behind it
        let mut cut = raw_bitmap(3, 2, &[&p, &p]);
        cut.push(6);
        cut.push(0x0707_0707);
        assert_eq!(embedded(cut), None);
        let mut compressed = raw_bitmap(3, 2, &[&p, &p, &p, &p]);
        compressed[8] = 1;
        assert_eq!(embedded(compressed), None);
        let mut deep = raw_bitmap(3, 2, &[&p, &p, &p, &p]);
        deep[1] = 24;
        assert_eq!(embedded(deep), None);
        // 8192 x 8192 claimed over a handful of words: refused before anything is allocated
        let mut huge = raw_bitmap(8192, 8192, &[]);
        huge.push(8192 * 8192);
        huge.extend([0; 16]);
        assert_eq!(embedded(huge), None);
        assert_eq!(embedded(raw_bitmap(8193, 1, &[&p, &p, &p, &p])), None);
        // only a header
        assert_eq!(embedded(vec![1, 32, 3]), None);
    }

    /// Found by review while fuzzing: a mesh whose vertices carry no data of their own (no positions, no normals, one
    /// colour, one specular colour and one uv for all) made up to a million vertices out of a handful of words, about
    /// 20 MB each; a file of many such meshes asked for gigabytes. Such vertices are no more than the words left.
    #[test]
    fn vertices_without_data_of_their_own_are_bounded_by_the_words() {
        use vertex_flags::*;
        let all = NO_POSITION | NO_NORMAL | SINGLE_UV | SINGLE_COLOR | SINGLE_SPECULAR;
        let made = |count: u32, flags: u32, extra: usize| {
            let mut data = vec![count, flags, 0, 0, 0, 0, 0];
            data.extend(std::iter::repeat_n(0, extra));
            mesh(&chunk(&[(0x0002_0000, data)])).map(|m| m.positions.len())
        };
        assert!(made(MAX_VERTICES as u32, all, 0).is_err());
        assert!(made(1000, all, 0).is_err());
        assert_eq!(made(4, all, 0).ok(), Some(4));
        // positions in the file: as many vertices as they hold
        assert!(made(1000, all & !NO_POSITION, 3000).is_ok());
        assert!(made(1000, all & !NO_POSITION, 2000).is_err());
    }

    /// The 2D entity's part as the C reference's `load_2dentity` reads it: the saved flags say what follows the
    /// rectangle (0x10000 a source rectangle, 0x20000 the Z order, 0x40000 the material).
    #[test]
    fn a_2d_frame_is_read_and_bad_ones_refused() {
        let f = f32::to_bits;
        let rect = [f(0.0), f(0.0), f(80.0), f(60.0)];
        let mut data = vec![0x0007_0220];
        data.extend(rect);
        data.extend([f(0.25), f(0.5), f(0.75), f(1.0)]);
        data.extend([(-3i32) as u32, 7]);
        let e = entity_2d(&chunk(&[(0x0010_f000, data.clone())])).unwrap();
        assert_eq!(e.rect, [0.0, 0.0, 80.0, 60.0]);
        assert_eq!(e.source, [0.25, 0.5, 0.75, 1.0]);
        assert_eq!((e.z_order, e.material), (-3, Some(7)));
        assert!(e.homogeneous() && e.use_source());
        // the bits that say what follows are not kept as flags
        assert_eq!(e.flags & 0x0007_0800, 0);
        // nothing optional: the whole texture, no material
        let mut plain = vec![0x200];
        plain.extend(rect);
        let e = entity_2d(&chunk(&[(0x0010_f000, plain.clone())])).unwrap();
        assert_eq!(
            (e.source, e.z_order, e.material),
            ([0.0, 0.0, 1.0, 1.0], 0, None)
        );
        // a plain frame's material of an older file; not read for a sprite
        let mut old = chunk(&[(0x0010_f000, plain.clone()), (0x0020_0000, vec![9])]);
        assert_eq!(entity_2d(&old).unwrap().material, None);
        old.class_id = 27;
        assert_eq!(entity_2d(&old).unwrap().material, Some(9));
        // no part at all: the constructor's values
        assert_eq!(entity_2d(&chunk(&[(0x1, vec![])])).unwrap().rect, [0.0; 4]);
        // cut short anywhere, or a rectangle that is no number
        for n in 0..data.len() {
            assert!(entity_2d(&chunk(&[(0x0010_f000, data[..n].to_vec())])).is_err());
        }
        let mut nan = plain;
        nan[2] = f(f32::NAN);
        assert!(entity_2d(&chunk(&[(0x0010_f000, nan)])).is_err());
    }

    /// A wave sound as the C reference's `ck_sound_load_chunk` reads it: CKSound's file name, the later section's
    /// over it, then flags (0x8 loops), priority, gain, pan and pitch.
    #[test]
    fn a_wave_sound_is_read_and_bad_ones_refused() {
        let f = f32::to_bits;
        let name = |s: &str| {
            let mut v = vec![s.len() as u32];
            let mut bytes = s.as_bytes().to_vec();
            bytes.resize(s.len().div_ceil(4) * 4, 0);
            v.extend(
                bytes
                    .chunks(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])),
            );
            v
        };
        let mut base = vec![0];
        base.extend(name("a.wav"));
        let settings = vec![0x8, f(0.5), f(0.85), f(0.0), f(1.0)];
        let s = wave_sound(&chunk(&[
            (0x1000, base.clone()),
            (0x0040_0000, settings.clone()),
        ]))
        .unwrap();
        assert_eq!(s.file_name.as_deref(), Some(&b"a.wav"[..]));
        assert!(s.looped);
        assert_eq!((s.gain, s.pan, s.pitch), (0.85, 0.0, 1.0));
        // the later section's name wins; no settings: the constructor's
        let s = wave_sound(&chunk(&[
            (0x1000, base.clone()),
            (0x0010_0000, name("b.wav")),
        ]))
        .unwrap();
        assert_eq!(s.file_name.as_deref(), Some(&b"b.wav"[..]));
        assert!(!s.looped && s.gain == 1.0);
        // settings of a data version before 3 are not read
        let mut early = chunk(&[(0x0040_0000, settings.clone())]);
        early.data_version = 2;
        assert!(!wave_sound(&early).unwrap().looped);
        // a name longer than its section, an endless one, settings cut short or no numbers
        assert!(wave_sound(&chunk(&[(0x1000, vec![0, 400, 1])])).is_err());
        let mut long = vec![0];
        long.extend(name(&"x".repeat(2000)));
        assert!(wave_sound(&chunk(&[(0x1000, long)])).is_err());
        assert!(wave_sound(&chunk(&[(0x0040_0000, settings[..3].to_vec())])).is_err());
        let mut inf = settings;
        inf[2] = f(f32::INFINITY);
        assert!(wave_sound(&chunk(&[(0x0040_0000, inf)])).is_err());
    }
}
