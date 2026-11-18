// The sky: a box of textures around the camera, named by the level (`Sky_<name>_<Face>`). Ballance's skies have
// five faces and no top (the zenith is the clear colour, taken from the top edge of the front texture); a sky that
// brings a sixth, `Top`, face (our photographic skies) is drawn closed.
// Reason for the allow: sky geometry from fixed tables.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use std::sync::atomic::{AtomicUsize, Ordering};

use level::{Level, Material, Mesh, Part, Texture, TextureSource};

pub const SIZE: f32 = 1200.0;
/// The faces of a sky without a top (Ballance's).
pub const FACE_COUNT: usize = 5;
/// The longest sky name taken from a level (the letters A..L of the original's, or a name such as `Mountains`).
pub const NAME_MAX: usize = 32;

/// How many faces the loaded sky has (5, or 6 with its top): the frame draws that many meshes of the sky model.
static FACES_DRAWN: AtomicUsize = AtomicUsize::new(FACE_COUNT);

/// Faces as (texture suffix, four corners on the unit cube, counter-clockwise from bottom left as seen from inside);
/// the last, `Top`, only for a sky that has it (its bottom edge on the front side).
const FACES: [(&str, [[f32; 3]; 4]); FACE_COUNT + 1] = [
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
    (
        "Top",
        [
            [-1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
        ],
    ),
];

/// Whether a level's sky name may name texture files: letters, digits, `_` and `-`, at most [`NAME_MAX`]. A level
/// is a stranger's file; any other name (a path, an empty one) is taken as no sky named.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= NAME_MAX
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Our sky standing in for a letter of the original's that our texture set has no face for, keeping the mood of the
/// original's: the dusk and night skies of D, I and K become our night over water, E and F the stormy cloud deck,
/// the warm G and L our sunset, the brown-and-blue H the desert, the pale J the clouds. None for A, B and C (ours) and
/// any other name. Skies without land only: the original's levels have no land of their own, and a mesh under the
/// track would show through it.
pub fn stand_in(letter: &str) -> Option<&'static str> {
    Some(match letter {
        "D" | "I" | "K" => "Night",
        "E" | "F" => "Storm",
        "G" | "L" => "Sunset",
        "H" => "Desert",
        "J" => "Clouds",
        _ => return None,
    })
}

/// The sky a level plays under: its own letter, or, when the chosen texture packs have no face for it (ours has
/// A, B, C and the named skies only), [`stand_in`] for it. The copy's and the remaster's packs keep the letter.
pub fn resolve(letter: &str, served: bool) -> String {
    match stand_in(letter) {
        Some(ours) if !served => ours.to_owned(),
        _ => letter.to_owned(),
    }
}

/// The texture file of a sky's face, as the game's textures name it (ours may be PNG or JPEG: assets.rs finds them).
pub fn face_file(name: &str, face: &str) -> String {
    format!("Sky_{name}_{face}.bmp")
}

/// A sky chosen on the command line (`--sky NAME`) for every level of this run, in place of the level's own.
static OVERRIDE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Sets the run's sky (once, at the start).
pub fn set_override(name: String) {
    let _ = OVERRIDE.set(name);
}

/// The run's sky, when one was chosen.
pub fn override_name() -> Option<String> {
    OVERRIDE.get().cloned()
}

/// The sky model uploaded has `count` faces (5, or 6 with its top).
pub fn set_faces(count: usize) {
    FACES_DRAWN.store(count.clamp(FACE_COUNT, FACE_COUNT + 1), Ordering::Relaxed);
}

/// How many faces of the sky model the frame draws.
pub fn faces() -> usize {
    FACES_DRAWN.load(Ordering::Relaxed)
}

/// A sky as a level model: one mesh per face (with the top when `top`), unlit, not writing depth. Its uploader tells
/// the frame how many faces it has ([`set_faces`]).
pub fn model(letter: &str, top: bool) -> Level {
    let mut sky = Level::default();
    let count = if top { FACE_COUNT + 1 } else { FACE_COUNT };
    for (i, (face, corners)) in FACES.iter().take(count).enumerate() {
        sky.textures.push(Texture {
            name: format!("Sky_{letter}_{face}"),
            source: TextureSource::File(face_file(letter, face)),
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
            normal: None,
            parallax: 0.0,
        });
        sky.meshes.push(Mesh {
            name: format!("Sky_{face}"),
            channels: Vec::new(),
            look: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_names_are_plain_words() {
        for ok in ["A", "L", "Mountains", "Sea_2", "my-sky"] {
            assert!(valid_name(ok), "{ok}");
        }
        for bad in [
            "",
            "../x",
            "a/b",
            "a\\b",
            "C:",
            "sky name",
            &"x".repeat(NAME_MAX + 1),
        ] {
            assert!(!valid_name(bad), "{bad}");
        }
    }

    #[test]
    fn letters_without_a_face_get_one_of_our_skies() {
        // no land sky: an original level has no land of its own
        let land = ["Mountains", "Forest", "Mist"];
        for l in ["D", "E", "F", "G", "H", "I", "J", "K", "L"] {
            let ours = stand_in(l).unwrap_or_default();
            assert!(valid_name(ours) && !land.contains(&ours), "{l}");
            assert_eq!(resolve(l, false), ours);
            // a pack that serves the letter (the copy's, the remaster's) keeps it
            assert_eq!(resolve(l, true), l);
        }
        for l in ["A", "B", "C", "Sea", "Z"] {
            assert_eq!(resolve(l, false), l);
        }
    }

    #[test]
    fn the_top_face_closes_the_box() {
        assert_eq!(model("A", false).meshes.len(), FACE_COUNT);
        let closed = model("Mountains", true);
        assert_eq!(closed.meshes.len(), FACE_COUNT + 1);
        assert!(
            closed.meshes[FACE_COUNT]
                .positions
                .iter()
                .all(|p| (p[1] - SIZE).abs() < 1e-3)
        );
        assert_eq!(closed.textures[FACE_COUNT].name, "Sky_Mountains_Top");
    }
}
