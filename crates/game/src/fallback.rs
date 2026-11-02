// Text the game's font cannot draw: its font (Font_1) has Latin-1 only, so a community map named in Chinese or
// Cyrillic read as "????". Such text is drawn with the fonts shipped with the game (assets/fonts, Noto Sans and
// Noto Sans SC, SIL OFL): the glyphs it needs are rasterised once into an atlas the renderer takes as the named
// texture `Noto_Fallback`, and laid out as 2D Text lays out the game's font.

use crate::ui::Quad;
use ab_glyph::{Font, FontVec, GlyphId, PxScale, ScaleFont};
use std::collections::HashMap;
use std::sync::Mutex;

/// The atlas's texture name, side and the size its glyphs are drawn at.
pub const TEXTURE: &str = "Noto_Fallback";
const SIDE: u32 = 1024;
const PX: f32 = 48.0;
/// The largest font file read.
const FONT_MAX: u64 = 64 * 1024 * 1024;

/// One glyph in the atlas: its texels (u0, v0, u1, v1 in 0..1), its box against the pen at the baseline (x0, y0,
/// x1, y1 in pixels at PX) and its advance.
#[derive(Clone, Copy)]
struct Glyph {
    uv: [f32; 4],
    bounds: [f32; 4],
    advance: f32,
}

struct Atlas {
    fonts: Vec<FontVec>,
    rgba: Vec<u8>,
    glyphs: HashMap<char, Option<Glyph>>,
    /// The next free place: x, y of the shelf, the shelf's height.
    at: (u32, u32, u32),
    changed: bool,
    /// The last glyph found no room: the atlas starts again empty (the glyphs on screen are made again next frame).
    full: bool,
}

static ATLAS: Mutex<Option<Atlas>> = Mutex::new(None);

/// Whether `s` has characters the game's font lacks (beyond Latin-1).
pub fn needed(s: &str) -> bool {
    s.chars().any(|c| u32::from(c) > 0xFF)
}

fn load_fonts() -> Vec<FontVec> {
    let Some(dir) = gamedata::resources::assets_root().map(|a| a.join("fonts")) else {
        return Vec::new();
    };
    ["NotoSans-Regular.ttf", "NotoSansSC-Regular.otf"]
        .iter()
        .filter_map(|name| {
            let path = dir.join(name);
            let small = std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= FONT_MAX);
            let bytes = small.then(|| std::fs::read(&path).ok()).flatten()?;
            FontVec::try_from_vec(bytes).ok()
        })
        .collect()
}

impl Atlas {
    fn new() -> Atlas {
        Atlas {
            fonts: load_fonts(),
            rgba: vec![0; (SIDE * SIDE * 4) as usize],
            glyphs: HashMap::new(),
            at: (1, 1, 0),
            changed: false,
            full: false,
        }
    }

    /// The glyph for `c`, rasterised into the atlas the first time; None when no font has it or the atlas is full.
    fn glyph(&mut self, c: char) -> Option<Glyph> {
        if let Some(g) = self.glyphs.get(&c) {
            return *g;
        }
        let mut made = self.make(c);
        if made.is_none() && self.full {
            // no room left: the atlas starts again, and the glyph gets the first place in it
            self.rgba.fill(0);
            self.glyphs.clear();
            self.at = (1, 1, 0);
            self.full = false;
            self.changed = true;
            made = self.make(c);
        }
        if !self.full {
            self.glyphs.insert(c, made);
        }
        made
    }

    fn make(&mut self, c: char) -> Option<Glyph> {
        let font = self.fonts.iter().find(|f| f.glyph_id(c) != GlyphId(0))?;
        let scaled = font.as_scaled(PxScale::from(PX));
        let id = font.glyph_id(c);
        let advance = scaled.h_advance(id);
        let glyph = id.with_scale(PX);
        let Some(outline) = font.outline_glyph(glyph) else {
            // a blank (a space of its own kind): an advance only
            return Some(Glyph {
                uv: [0.0; 4],
                bounds: [0.0; 4],
                advance,
            });
        };
        let b = outline.px_bounds();
        let (w, h) = ((b.width().ceil() as u32).saturating_add(1), (b.height().ceil() as u32).saturating_add(1));
        if w.saturating_add(2) > SIDE || h.saturating_add(2) > SIDE {
            // a glyph larger than the whole atlas never fits: no use starting it again for it
            return None;
        }
        let (mut x, mut y, mut row) = self.at;
        if x + w + 1 > SIDE {
            (x, y, row) = (1, y + row + 1, 0);
        }
        if y + h + 1 > SIDE {
            self.full = true;
            return None;
        }
        let rgba = &mut self.rgba;
        outline.draw(|gx, gy, cover| {
            let (px, py) = (x + gx, y + gy);
            let i = ((py * SIDE + px) * 4) as usize;
            if let Some(texel) = rgba.get_mut(i..i + 4) {
                let a = (cover.clamp(0.0, 1.0) * 255.0) as u8;
                texel.copy_from_slice(&[255, 255, 255, a]);
            }
        });
        self.at = (x + w + 1, y, row.max(h));
        self.changed = true;
        let s = SIDE as f32;
        Some(Glyph {
            uv: [
                x as f32 / s,
                y as f32 / s,
                (x + w) as f32 / s,
                (y + h) as f32 / s,
            ],
            bounds: [b.min.x, b.min.y, b.min.x + w as f32, b.min.y + h as f32],
            advance,
        })
    }
}

/// `text` in one line in the context rectangle `rect` (pixels of 640×480), its glyphs `height` high (the game font's
/// line height), centred or aligned as `align` (ui's ALIGN_ bits), in `color`. Characters no shipped font has are
/// left out.
pub fn layout(text: &str, rect: [f32; 4], height: f32, align: u32, color: [f32; 4]) -> Vec<Quad> {
    let Ok(mut guard) = ATLAS.lock() else {
        return Vec::new();
    };
    let atlas = guard.get_or_insert_with(Atlas::new);
    let Some(font) = atlas.fonts.first() else {
        return Vec::new();
    };
    let scaled = font.as_scaled(PxScale::from(PX));
    let (ascent, descent) = (scaled.ascent(), scaled.descent());
    let k = height / (ascent - descent).max(1.0);
    let glyphs: Vec<Glyph> = text.chars().filter_map(|c| atlas.glyph(c)).collect();
    let width: f32 = glyphs.iter().map(|g| g.advance * k).sum();
    let mut x = if align & crate::ui::ALIGN_LEFT != 0 {
        rect[0]
    } else if align & crate::ui::ALIGN_RIGHT != 0 {
        rect[2] - width
    } else {
        (rect[0] + rect[2]) / 2.0 - width / 2.0
    };
    let baseline = (rect[1] + rect[3]) / 2.0 - height / 2.0 + ascent * k;
    let mut out = Vec::new();
    for g in glyphs {
        if g.uv[2] > g.uv[0] {
            out.push(Quad {
                rect: [
                    x + g.bounds[0] * k,
                    baseline + g.bounds[1] * k,
                    x + g.bounds[2] * k,
                    baseline + g.bounds[3] * k,
                ],
                uv: g.uv,
                top: color,
                bottom: color,
                texture: TEXTURE,
            });
        }
        x += g.advance * k;
    }
    out
}

/// The atlas's pixels when glyphs were added since the last time (the renderer uploads them).
pub fn take_changed() -> Option<(u32, u32, Vec<u8>)> {
    let mut guard = ATLAS.lock().ok()?;
    let atlas = guard.as_mut()?;
    if !atlas.changed {
        return None;
    }
    atlas.changed = false;
    Some((SIDE, SIDE, atlas.rgba.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_text_is_told_apart() {
        assert!(!needed("Level_01 é"));
        assert!(needed("失衡空间站"));
        assert!(needed("Уровень"));
    }
}
