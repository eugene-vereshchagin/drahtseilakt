// The 3D texts a map's scripts show (3D Text): glyph quads of the shipped fonts (fallback.rs) laid out on the local
// XY plane of the entity the block targets, with the entity's world matrix, so the text follows it when it moves. They
// go to the renderer as particle batches: world-space vertices, depth-tested, alpha-blended, unlit. The layout is
// pure and never touches the simulation.

use crate::fallback;
use crate::particles::{Batch, Blend, Vertex};
use crate::scripting::Scripts;
use crate::ui::{ALIGN_BOTTOM, ALIGN_LEFT, ALIGN_RIGHT, ALIGN_TOP, Style, TEXT_WORDWRAP};
use glam::{Mat4, Vec3};

/// A 3D text on an entity: the block that shows it, the entity's row and what to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Text3d {
    pub key: script::Id,
    pub row: usize,
    pub spec: script::Text3dSpec,
}

/// The 3D texts a run shows, by block: at most `MAX_TEXT3D` of them, `MAX_TEXT3D_CHARS` characters together. It is
/// part of the run (it clones and rewinds with it) and is only read by the renderer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Texts {
    pub items: Vec<Text3d>,
}

impl Texts {
    /// Shows, changes or takes away the text of block `key`. A text that does not fit the character budget is cut,
    /// one that finds no slot is left out. Only an object of the level or a script's own carries text (not the ball).
    pub fn set(&mut self, key: usize, on: Option<(script::Ent, script::Text3dSpec)>) {
        let at = self.items.iter().position(|t| t.key == key);
        let Some((script::Ent::Object(row), mut spec)) = on else {
            if let Some(i) = at {
                self.items.remove(i);
            }
            return;
        };
        let others: usize = self
            .items
            .iter()
            .filter(|t| t.key != key)
            .map(|t| t.spec.text.chars().count())
            .sum();
        let room = script::MAX_TEXT3D_CHARS.saturating_sub(others);
        spec.text = spec
            .text
            .chars()
            .take(room.min(script::MAX_TEXT_CHARS))
            .collect();
        match at {
            Some(i) => {
                if let Some(slot) = self.items.get_mut(i) {
                    *slot = Text3d { key, row, spec };
                }
            }
            None if self.items.len() < script::MAX_TEXT3D => {
                self.items.push(Text3d { key, row, spec });
            }
            None => {}
        }
    }
}

/// How many font pixels (32 to a line at scale 1) make one world unit: a line at scale 1 is half a unit high (the
/// maps set scales of 10 for texts a few units tall).
const PX_PER_UNIT: f32 = 64.0;
/// The box of an entity without a mesh (world units): wide enough for any line, so that only a newline breaks one.
/// The entity's origin is where the alignment puts the text (centred on it, or starting, ending, hanging from it).
const BARE_BOX: [f32; 2] = [1024.0, 256.0];
/// The most quads drawn in a frame over all texts (a quad is six vertices).
const MAX_QUADS: usize = 4_096;

/// The local box a text is laid out in: the entity's mesh bounds on X and Y (centre, size), else a default box.
fn local_box(bounds: Option<(Vec3, Vec3)>, align: u8) -> ([f32; 2], [f32; 2]) {
    match bounds {
        Some((lo, hi)) if hi.x - lo.x > 1e-4 && hi.y - lo.y > 1e-4 => (
            [(lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0],
            [hi.x - lo.x, hi.y - lo.y],
        ),
        _ => {
            let [w, h] = BARE_BOX;
            let x = if align & 1 != 0 {
                w / 2.0
            } else if align & 2 != 0 {
                -w / 2.0
            } else {
                0.0
            };
            let y = if align & 4 != 0 {
                -h / 2.0
            } else if align & 8 != 0 {
                h / 2.0
            } else {
                0.0
            };
            ([x, y], BARE_BOX)
        }
    }
}

/// The vertices (two triangles a glyph) of one text with the entity's world matrix `world`: the box (`centre`, `size`
/// in local units) is laid out as 2D Text lays out its frame, 1 world unit = `PX_PER_UNIT` font pixels.
pub fn vertices(t: &Text3d, centre: [f32; 2], size: [f32; 2], world: Mat4, out: &mut Vec<Vertex>) {
    let s = &t.spec;
    let (w, h) = (size[0] * PX_PER_UNIT, size[1] * PX_PER_UNIT);
    let mut align = 0;
    for (bit, a) in [
        (1u8, ALIGN_LEFT),
        (2, ALIGN_RIGHT),
        (4, ALIGN_TOP),
        (8, ALIGN_BOTTOM),
    ] {
        if s.align & bit != 0 {
            align |= a;
        }
    }
    let style = Style {
        scale: s.scale,
        spacing: [0.0; 2],
        align,
        flags: TEXT_WORDWRAP,
        margins: [
            s.margins[0] + s.offset[0],
            s.margins[1] + s.offset[1],
            s.margins[2] - s.offset[0],
            s.margins[3] - s.offset[1],
        ],
        color: s.color,
        v2: true,
        shadow: None,
    };
    let quads = fallback::styled(&s.text, [0.0, 0.0, w, h], &style);
    let place = |x: f32, y: f32| -> [f32; 3] {
        // pixels (y down) to the box's local plane (y up), then the world
        let lx = centre[0] + (x - w / 2.0) / PX_PER_UNIT;
        let ly = centre[1] - (y - h / 2.0) / PX_PER_UNIT;
        world.transform_point3(Vec3::new(lx, ly, 0.0)).to_array()
    };
    for q in quads {
        let [x0, y0, x1, y1] = q.rect;
        let [u0, v0, u1, v1] = q.uv;
        let c = q.top;
        let v = |x: f32, y: f32, u: f32, vv: f32| Vertex {
            pos: place(x, y),
            uv: [u, vv],
            color: c,
        };
        let (a, b, cc, d) = (
            v(x0, y0, u0, v0),
            v(x1, y0, u1, v0),
            v(x1, y1, u1, v1),
            v(x0, y1, u0, v1),
        );
        out.extend([a, b, cc, a, cc, d]);
    }
}

/// The run's 3D texts as one batch of the font atlas (none when no text shows).
pub fn batches(sc: &Scripts) -> Vec<Batch> {
    let mut vertices = Vec::new();
    for t in &sc.text3d.items {
        if sc.ents.is_hidden(t.row) {
            continue;
        }
        let Some(world) = sc.ents.world(t.row) else {
            continue;
        };
        let bounds = sc.ents.local_bounds(t.row).flatten();
        let (centre, size) = local_box(bounds, t.spec.align);
        // an entity with a material tints its text by the material's diffuse colour, as the scripts have it now
        let tint = sc
            .ents
            .get(t.row)
            .and_then(|e| e.materials.first())
            .and_then(|&m| sc.mats.effective(m))
            .map_or([1.0; 4], |m| m.diffuse);
        let mut shown = t.clone();
        for (c, k) in shown.spec.color.iter_mut().zip(tint) {
            *c *= k;
        }
        vertices_limited(&shown, centre, size, world, &mut vertices);
    }
    if vertices.is_empty() {
        return Vec::new();
    }
    vec![Batch {
        texture: Some(fallback::TEXTURE),
        blend: Blend::Alpha,
        lines: false,
        vertices,
    }]
}

fn vertices_limited(
    t: &Text3d,
    centre: [f32; 2],
    size: [f32; 2],
    world: Mat4,
    out: &mut Vec<Vertex>,
) {
    let mut mine = Vec::new();
    vertices(t, centre, size, world, &mut mine);
    let room = (MAX_QUADS * 6).saturating_sub(out.len());
    mine.truncate(room);
    out.extend(mine);
}

/// Adds the run's 3D texts to the frame's particle batches and uploads the font atlas when glyphs were added.
pub fn add(sc: Option<&Scripts>, particles: &mut Vec<Batch>, r: &mut crate::render::Renderer) {
    let Some(sc) = sc.filter(|s| !s.text3d.items.is_empty()) else {
        return;
    };
    particles.extend(batches(sc));
    if let Some((aw, ah, rgba)) = fallback::take_changed() {
        r.add_named_texture(fallback::TEXTURE, aw, ah, rgba);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use script::{MAX_TEXT3D, MAX_TEXT3D_CHARS, Text3dSpec};

    fn text(s: &str) -> Text3d {
        Text3d {
            key: 1,
            row: 0,
            spec: Text3dSpec {
                text: s.into(),
                align: 0,
                scale: [1.0; 2],
                margins: [0.0; 4],
                offset: [0.0; 2],
                bold: false,
                color: [1.0; 4],
            },
        }
    }

    #[test]
    fn a_text_follows_its_entity_and_stays_in_its_plane() {
        if !fallback::available() {
            return;
        }
        let t = text("AB");
        let mut at_origin = Vec::new();
        vertices(&t, [0.0; 2], [20.0, 4.0], Mat4::IDENTITY, &mut at_origin);
        assert!(!at_origin.is_empty() && at_origin.len() % 6 == 0);
        assert!(at_origin.iter().all(|v| v.pos[2] == 0.0));
        let moved = Mat4::from_translation(Vec3::new(5.0, 6.0, 7.0));
        let mut there = Vec::new();
        vertices(&t, [0.0; 2], [20.0, 4.0], moved, &mut there);
        for (a, b) in at_origin.iter().zip(&there) {
            assert_eq!(
                [a.pos[0] + 5.0, a.pos[1] + 6.0, a.pos[2] + 7.0],
                b.pos,
                "the quads follow the matrix"
            );
        }
    }

    fn spec(s: &str) -> Text3dSpec {
        text(s).spec
    }

    fn on(row: usize, s: &str) -> Option<(script::Ent, Text3dSpec)> {
        Some((script::Ent::Object(row), spec(s)))
    }

    #[test]
    fn state_is_kept_per_block() {
        let mut t = Texts::default();
        t.set(10, on(1, "one"));
        t.set(11, on(2, "two"));
        t.set(10, on(1, "uno"));
        assert_eq!(t.items.len(), 2);
        assert_eq!(t.items[0].spec.text, "uno");
        assert_eq!(t.items[1].row, 2);
        t.set(10, None);
        assert_eq!(t.items.len(), 1);
        assert_eq!(t.items[0].key, 11);
        // the ball carries no text
        t.set(12, Some((script::Ent::Ball, spec("x"))));
        assert_eq!(t.items.len(), 1);
    }

    #[test]
    fn the_caps_hold() {
        let mut t = Texts::default();
        for k in 0..100 {
            t.set(k, on(k, "x"));
        }
        assert_eq!(t.items.len(), MAX_TEXT3D);
        // a changed text of a shown block still takes its place
        t.set(5, on(5, "changed"));
        assert_eq!(t.items.len(), MAX_TEXT3D);
        let mut t = Texts::default();
        for k in 0..8 {
            t.set(k, on(k, &"y".repeat(1000)));
        }
        let total: usize = t.items.iter().map(|i| i.spec.text.chars().count()).sum();
        assert_eq!(total, MAX_TEXT3D_CHARS);
        assert_eq!(
            t.items.len(),
            8,
            "the text past the budget is cut to nothing, not dropped"
        );
        assert!(MAX_QUADS * 6 >= MAX_TEXT3D_CHARS);
    }

    #[test]
    fn a_rewind_restores_the_texts() {
        let mut t = Texts::default();
        t.set(1, on(3, "saved"));
        let saved = t.clone();
        t.set(1, on(3, "later"));
        t.set(2, on(4, "new"));
        assert_ne!(t, saved);
        t = saved.clone();
        assert_eq!(t, saved);
        assert_eq!(t.items.len(), 1);
        assert_eq!(t.items[0].spec.text, "saved");
    }
}
