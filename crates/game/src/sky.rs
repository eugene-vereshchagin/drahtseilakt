// The sky: a box of five textures around the camera (Ballance has no top face; the zenith is the clear
// colour, taken from the top edge of the front texture).

use level::{Level, Material, Mesh, Part, Texture, TextureSource};

pub const SIZE: f32 = 1200.0;
pub const FACE_COUNT: usize = 5;

/// Faces as (texture suffix, four corners on the unit cube, counter-clockwise from bottom left as seen from inside).
const FACES: [(&str, [[f32; 3]; 4]); FACE_COUNT] = [
    (
        "Front",
        [
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ],
    ),
    (
        "Right",
        [
            [1.0, -1.0, 1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [1.0, 1.0, 1.0],
        ],
    ),
    (
        "Back",
        [
            [1.0, -1.0, -1.0],
            [-1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
        ],
    ),
    (
        "Left",
        [
            [-1.0, -1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [-1.0, 1.0, -1.0],
        ],
    ),
    (
        "Down",
        [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, -1.0, 1.0],
            [-1.0, -1.0, 1.0],
        ],
    ),
];

/// A sky as a level model: one mesh per face, unlit, not writing depth.
pub fn model(letter: &str) -> Level {
    let mut sky = Level::default();
    for (i, (face, corners)) in FACES.iter().enumerate() {
        sky.textures.push(Texture {
            name: format!("Sky_{letter}_{face}"),
            source: TextureSource::File(format!("Sky_{letter}_{face}.bmp")),
            color_key: None,
        });
        sky.materials.push(Material {
            name: format!("Sky_{face}"),
            diffuse: [1.0; 4],
            ambient: [0.0; 3],
            emissive: [0.0; 3],
            specular: [0.0; 3],
            power: 0.0,
            texture: Some(i),
            blend: None,
            alpha_cutoff: None,
            z_write: false,
            two_sided: true,
            unlit: true,
            texgen: None,
        });
        sky.meshes.push(Mesh {
            name: format!("Sky_{face}"),
            channels: Vec::new(),
            positions: corners
                .iter()
                .map(|c| [c[0] * SIZE, c[1] * SIZE, c[2] * SIZE])
                .collect(),
            normals: vec![[0.0, 1.0, 0.0]; 4],
            // Texture rows run top to bottom; the inset keeps edge texels from wrapping around.
            uvs: vec![
                [0.002, 0.998],
                [0.998, 0.998],
                [0.998, 0.002],
                [0.002, 0.002],
            ],
            parts: vec![Part {
                material: Some(i),
                indices: vec![0, 1, 2, 0, 2, 3],
            }],
        });
    }
    sky
}
