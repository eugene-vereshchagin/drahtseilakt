// The engine's level model: meshes, materials, textures, placed objects and named groups. Every source of
// levels — an original `.nmo`, a community map, later our open format — is loaded into this one model, so the
// renderer, physics and game rules never depend on a file format.
//
// Coordinates stay as Virtools has them: left-handed, Y up, units as in the original.

pub mod from_nmo;
pub mod open;

#[derive(Debug, Clone, Default)]
pub struct Level {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub objects: Vec<Object>,
    pub groups: Vec<Group>,
    /// Letter of the sky (`Sky_<letter>_*.bmp` in the game's textures), when the level names one.
    pub sky: Option<String>,
    /// Name of the ball the level starts with (`Ball_Wood`, ...), when the level names one.
    pub start_ball: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Triangles grouped by material.
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Default)]
pub struct Part {
    pub material: Option<usize>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blend {
    Zero,
    One,
    SrcColor,
    InvSrcColor,
    SrcAlpha,
    InvSrcAlpha,
    DestAlpha,
    InvDestAlpha,
    DestColor,
    InvDestColor,
}

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    /// RGBA, 0..1.
    pub diffuse: [f32; 4],
    pub ambient: [f32; 3],
    pub emissive: [f32; 3],
    pub texture: Option<usize>,
    /// Blending against the frame, used when `blend` is set.
    pub blend: Option<(Blend, Blend)>,
    /// Pixels with alpha at or below this are discarded (alpha test), when set.
    pub alpha_cutoff: Option<f32>,
    pub z_write: bool,
    pub two_sided: bool,
    /// Drawn with its own colour only, without lighting (skies).
    pub unlit: bool,
}

#[derive(Debug, Clone)]
pub enum TextureSource {
    /// A file name, looked up in the game's texture folders.
    File(String),
    /// Pixels stored with the level: width, height, RGBA.
    Pixels {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    /// Neither: drawn untextured.
    None,
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub name: String,
    pub source: TextureSource,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub name: String,
    /// Column-major 4x4 world matrix (columns: X axis, Y axis, Z axis, position).
    pub matrix: [[f32; 4]; 4],
    pub mesh: Option<usize>,
    pub visible: bool,
}

impl Object {
    pub fn position(&self) -> [f32; 3] {
        let [x, y, z, _] = self.matrix[3];
        [x, y, z]
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub members: Vec<usize>,
}

impl Level {
    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(name))
    }

    /// The objects of a group, in the group's order.
    pub fn members<'a>(&'a self, name: &str) -> impl Iterator<Item = (usize, &'a Object)> + 'a {
        let members = self
            .group(name)
            .map(|g| g.members.clone())
            .unwrap_or_default();
        members
            .into_iter()
            .filter_map(move |i| self.objects.get(i).map(|o| (i, o)))
    }

    pub fn object(&self, name: &str) -> Option<(usize, &Object)> {
        self.objects
            .iter()
            .enumerate()
            .find(|(_, o)| o.name.eq_ignore_ascii_case(name))
    }

    /// Names of the groups an object belongs to.
    pub fn groups_of(&self, object: usize) -> impl Iterator<Item = &str> {
        self.groups
            .iter()
            .filter(move |g| g.members.contains(&object))
            .map(|g| g.name.as_str())
    }
}
