// The engine's level model: meshes, materials, textures, placed objects and named groups. Every source of
// levels — an original `.nmo`, a community map, later our open format — is loaded into this one model, so the
// renderer, physics and game rules never depend on a file format.
//
// Coordinates stay as Virtools has them: left-handed, Y up, units as in the original.

pub mod from_nmo;
pub mod open;
pub mod sectorballs;

/// The module a level object is named after: its name without the trailing `_NN` (digits, then any letters:
/// `P_Box_03` is `P_Box`, `P_Trafo_Stone_03a` is `P_Trafo_Stone`); a name without one is its own.
pub fn module_name(object: &str) -> &str {
    match object.rsplit_once('_') {
        Some((head, tail)) => {
            let digits = tail.trim_end_matches(|c: char| c.is_ascii_alphabetic());
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                head
            } else {
                object
            }
        }
        None => object,
    }
}

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
    /// What the original game gives a level from its level table and its rules by level number; an open-format
    /// level carries it itself, so it plays the same without the game's files.
    pub settings: Settings,
    /// The Virtools behaviour graphs a community map carries (scripts of its own), for the script runtime; a level
    /// in the open format has none.
    pub scripts: Option<std::sync::Arc<script::Graph>>,
}

/// A level's settings beyond its scene (the original's `AllLevel` table in Levelinit.nmo, and its first and last
/// levels' rules).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    /// The level's music theme (`Music_Theme_<n>_*`).
    pub music: Option<u32>,
    /// The colour of the level's light (0..1).
    pub light: Option<[f32; 3]>,
    /// The points the finish adds.
    pub level_bonus: Option<i64>,
    /// The level begins with the original's tutorial (level 1), which stops it before the ball comes.
    pub tutorial: bool,
    /// The game's last level: its finish plays the last music, and lightning flashes over it.
    pub last_level: bool,
    /// The hash of the file the level was converted from (replays recorded on that file play on this level).
    pub source_hash: Option<u64>,
    /// The level is made for the game's own parts (`"parts": "own"`): it takes ours before the copy's, so it plays the
    /// same with a copy or without one. Any other level takes the copy's parts first, as it was made.
    pub own_parts: bool,
    /// The air between the camera and what it sees: none in the original's levels (their look has none); our own
    /// levels set it so their columns and the world far below fade into the sky's haze. Only drawn.
    pub fog: Option<Fog>,
    /// A sea of cloud under the track, drawn as a volume (only drawn): none in the original's levels.
    pub clouds: Option<Clouds>,
    /// The ball each sector begins with (index 0 is sector 1), for a run begun at a sector (`--sector`): `wood`,
    /// `stone` or `paper`; none (or past the end) takes the level's start ball. A level run from its start ignores
    /// them: the ball carries its kind from the transformers.
    pub sector_balls: Vec<Option<String>>,
}

/// A level's cloud layer (only drawn, never the physics'): a slab of cumulus whose tops are at `top` and whose
/// bottom `thickness` under it, `cover` of it cloud (1 a closed sea, less broken banks with gaps), each cloud about
/// `scale` across, lit by a sun in the direction `sun` (towards it) of `sun_color`, the sky's light from above `sky`,
/// and in shadow `shadow` (the light bounced from below); far off they fade into the sky's haze, `haze` of it a unit of
/// distance (1 - e^(-haze * distance)). `under` is how much of the world under the layer the haze
/// there hides (1 none of it shows through a gap, 0 all of it), `wind` the drift (x, z, units a second). The colours
/// are linear light, 1 the brightness of white; the sun's may be brighter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clouds {
    pub top: f32,
    pub thickness: f32,
    pub cover: f32,
    pub scale: f32,
    pub sun: [f32; 3],
    pub sun_color: [f32; 3],
    pub sky: [f32; 3],
    pub shadow: [f32; 3],
    pub haze: f32,
    pub under: f32,
    pub wind: [f32; 2],
}

impl Clouds {
    /// The defaults of the settings a level may leave out (a white noon over a closed sea).
    pub const DEFAULT: Clouds = Clouds {
        top: 0.0,
        thickness: 60.0,
        cover: 0.85,
        scale: 60.0,
        sun: [0.3, 0.6, 0.74],
        sun_color: [3.0, 2.9, 2.7],
        sky: [0.55, 0.65, 0.85],
        shadow: [0.35, 0.38, 0.48],
        haze: 0.0005,
        under: 1.0,
        wind: [1.5, 0.6],
    };
}

/// A level's fog (only drawn, never the physics'): what is far from the camera fades into the air's colour, `color`
/// seen towards the horizon and `color_down` seen straight down (the world far below), `density` of it a unit of
/// distance (1 - e^(-density * distance)), and what lies below `below` fades into it as well, wholly at `depth` under
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fog {
    pub color: [f32; 3],
    pub color_down: [f32; 3],
    pub density: f32,
    pub below: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Triangles grouped by material.
    pub parts: Vec<Part>,
    /// Passes over the whole mesh with other materials, in order.
    pub channels: Vec<Channel>,
    /// What is drawn instead of this mesh, when a level carries a finer look (the toolbox's rounder shapes); the
    /// physics and the rules always use the mesh itself.
    pub look: Option<Box<Mesh>>,
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
    /// The highlight's colour and sharpness (the fixed-function specular term; none with a black colour or power 0).
    pub specular: [f32; 3],
    pub power: f32,
    pub texture: Option<usize>,
    /// A normal map over the base texture (tangent space, green up as glTF has it): the relief the toolbox gives a
    /// remastered level. Only drawn; none in the original's levels.
    pub normal: Option<usize>,
    /// The depth of the relief drawn with parallax, in texture coordinates (0: none): the normal map's alpha is then
    /// a height (1 the top), as the toolbox makes it for masonry. Only drawn.
    pub parallax: f32,
    /// Blending against the frame, used when `blend` is set.
    pub blend: Option<(Blend, Blend)>,
    /// Pixels with alpha at or below this are discarded (alpha test), when set.
    pub alpha_cutoff: Option<f32>,
    pub z_write: bool,
    pub two_sided: bool,
    /// Drawn with its own colour only, without lighting (skies).
    pub unlit: bool,
    /// Texture coordinates generated from the view instead of the mesh's (the material's TexGen effect).
    pub texgen: Option<TexGen>,
}

/// How a material's texture coordinates are made from the view (Virtools' TexGen types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexGen {
    /// From the reflection of the view about the normal (type 2).
    Reflect,
    /// From the normal as seen from the view (type 3).
    Chrome,
}

/// A further pass of a mesh with another material, blended over it (a Virtools material channel).
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub material: usize,
    /// Source and destination blend.
    pub blend: (Blend, Blend),
    /// One pair per vertex.
    pub uvs: Vec<[f32; 2]>,
}

#[derive(Debug, Clone)]
pub enum TextureSource {
    /// A file name, looked up in the game's texture folders. From a `.nmo` it is the file's raw name (a stranger's
    /// map may name anything there): only its last part is ever used.
    File(String),
    /// A file of an open-format level's own `textures/` folder: its full path, made by the reader from the level's
    /// folder (made absolute) and a plain name it checked. Only this source names a path to read.
    LevelFile(std::path::PathBuf),
    /// The game's texture of this name (an open-format level's image `game:<name>`): looked up by name through the
    /// texture packs (docs/drahtseilakt/packs.md), never read as a path. Our levels name the textures they share
    /// this way instead of carrying copies.
    Game(String),
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
    /// Texels of this colour (RGB) are transparent: the original drops them with an alpha test.
    pub color_key: Option<[u8; 3]>,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub name: String,
    /// Column-major 4x4 world matrix (columns: X axis, Y axis, Z axis, position).
    pub matrix: [[f32; 4]; 4],
    pub mesh: Option<usize>,
    pub visible: bool,
    /// A sprite (CKSprite3D): its mesh is a unit quad (0..1 in x and y) that is turned and sized each frame.
    pub sprite: Option<Sprite>,
}

/// How a sprite is placed each frame (the original's CKSprite3D): `mode` 0 faces the view, 1 turns about the world x,
/// 2 about the world y, 3 keeps its own axes; the quad is `size` wide and high, shifted by `offset` (-1..1 of half
/// its size) in its plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    pub mode: u32,
    pub size: [f32; 2],
    pub offset: [f32; 2],
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

#[cfg(test)]
mod tests {
    #[test]
    fn a_placeholder_names_its_module_with_or_without_a_letter() {
        assert_eq!(super::module_name("P_Box_03"), "P_Box");
        assert_eq!(super::module_name("P_Trafo_Stone_03a"), "P_Trafo_Stone");
        assert_eq!(super::module_name("P_Modul_18_10a"), "P_Modul_18");
        assert_eq!(
            super::module_name("P_Trafo_Stone_ab"),
            "P_Trafo_Stone_ab",
            "letters alone are no number"
        );
        assert_eq!(super::module_name("Floor"), "Floor");
    }
}
