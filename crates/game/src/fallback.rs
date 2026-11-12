// Text in the fonts shipped with the game (assets/fonts, SIL OFL): the own interface's every text, and the text the
// copy's font cannot draw (its Font_1 has Latin-1 only, so Cyrillic and Chinese menus and the names of community maps
// read as "????"). The own interface's look follows the original's menu font: a squared technical face (Tektur)
// with small capitals for the lower case, a little wide and spaced; what Tektur lacks comes from Noto Sans, Chinese
// from Noto Sans SC. The glyphs a text needs are rasterised once into an atlas the renderer takes as the named
// texture `Noto_Fallback`, and laid out as 2D Text lays out the game's font (ui.rs): the same rectangles, alignment,
// word wrap, colour and shadow, so a screen's layout is the same in either font.

use crate::ui::{
    ALIGN_BOTTOM, ALIGN_LEFT, ALIGN_RIGHT, ALIGN_TOP, CONTEXT_H, Quad, Style, TEXT_SCREEN,
    TEXT_WORDWRAP,
};
use ab_glyph::{Font, FontVec, GlyphId, PxScale, ScaleFont, VariableFont};
use std::collections::HashMap;
use std::sync::Mutex;

/// The atlas's texture name, side and the size (pixels per em) its glyphs are drawn at.
pub const TEXTURE: &str = "Noto_Fallback";
const SIDE: u32 = 1024;
const EM: f32 = 44.0;
/// The largest font file read.
const FONT_MAX: u64 = 64 * 1024 * 1024;
/// The fonts, first come first: the interface's face, then Noto Sans for what it lacks, then Chinese.
const FONTS: [&str; 3] = [
    "Tektur-Variable.ttf",
    "NotoSans-Regular.ttf",
    "NotoSansSC-Regular.otf",
];
/// Tektur's weight and width (its variation axes): bold enough to carry as much ink as the original's labels.
const WEIGHT: f32 = 700.0;
const WIDTH: f32 = 100.0;
/// The capitals' height against the line, the small capitals' size against the capitals, the glyphs widened and
/// spaced (fractions of an em): measured on the original's main menu at 1280x720 (capitals 10 pixels high, its five
/// labels 45 to 124 pixels wide), Tektur so set draws them within 2 % of those widths.
const CAPS: f32 = 0.45;
const SMALL_CAPS: f32 = 0.8;
const WIDE: f32 = 1.7;
const TRACKING: f32 = 0.18;
/// The longest text laid out (characters): a label is short; more is cut.
const TEXT_MAX: usize = 512;

/// One glyph in the atlas: its texels (u0, v0, u1, v1 in 0..1), its box against the pen at the baseline (x0, y0,
/// x1, y1 in pixels at EM) and its advance.
#[derive(Clone, Copy)]
struct Glyph {
    uv: [f32; 4],
    bounds: [f32; 4],
    advance: f32,
}

struct Atlas {
    fonts: Vec<FontVec>,
    /// The first font's capital height in pixels at EM (what the line's size is measured by).
    cap: f32,
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
    FONTS
        .iter()
        .filter_map(|name| {
            let path = dir.join(name);
            let small = std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= FONT_MAX);
            let bytes = small.then(|| std::fs::read(&path).ok()).flatten()?;
            let mut font = FontVec::try_from_vec(bytes).ok()?;
            // a font without these axes keeps its own design
            font.set_variation(b"wght", WEIGHT);
            font.set_variation(b"wdth", WIDTH);
            Some(font)
        })
        .collect()
}

/// The scale that draws `font` at EM pixels per em (ab_glyph scales by the font's line height, not its em).
fn em_scale(font: &FontVec) -> PxScale {
    let per_em = font.units_per_em().unwrap_or(1000.0).max(1.0);
    PxScale::from(EM * font.height_unscaled() / per_em)
}

impl Atlas {
    fn new() -> Atlas {
        let fonts = load_fonts();
        let cap = fonts
            .first()
            .and_then(|f| f.outline_glyph(f.glyph_id('H').with_scale(em_scale(f))))
            .map_or(EM * 0.7, |o| o.px_bounds().height())
            .max(1.0);
        Atlas {
            fonts,
            cap,
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
        let scale = em_scale(font);
        let scaled = font.as_scaled(scale);
        let id = font.glyph_id(c);
        let advance = scaled.h_advance(id);
        let glyph = id.with_scale(scale);
        let Some(outline) = font.outline_glyph(glyph) else {
            // a blank (a space of its own kind): an advance only
            return Some(Glyph {
                uv: [0.0; 4],
                bounds: [0.0; 4],
                advance,
            });
        };
        let b = outline.px_bounds();
        let (w, h) = (
            (b.width().ceil() as u32).saturating_add(1),
            (b.height().ceil() as u32).saturating_add(1),
        );
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
        // Tektur's D has the same square box as its O at menu sizes (7 pixels): the D is drawn with its right corners
        // cut well off and its left corners square, so that it reads as a D (see `d_cover`)
        let first = std::ptr::eq(font, &self.fonts[0]);
        let own_d = c == 'D' && first;
        let mut put = |gx: u32, gy: u32, cover: f32| {
            let (px, py) = (x + gx, y + gy);
            let i = ((py * SIDE + px) * 4) as usize;
            if let Some(texel) = rgba.get_mut(i..i + 4) {
                let a = (cover.clamp(0.0, 1.0) * 255.0) as u8;
                texel.copy_from_slice(&[255, 255, 255, a]);
            }
        };
        if own_d {
            for gy in 0..h {
                for gx in 0..w {
                    put(gx, gy, d_cover(gx, gy, b.width(), b.height()));
                }
            }
        } else {
            outline.draw(&mut put);
        }
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

/// The coverage of texel (gx, gy) by a D of w by h pixels with stems 0.24 of the height wide: square on the left, the right corners cut
/// at 45 degrees by 0.30 of the height (outer outline and counter, the wall a stem thick), 6 by 6 samples a texel.
fn d_cover(gx: u32, gy: u32, w: f32, h: f32) -> f32 {
    let (k, t) = (0.30 * h, 0.24 * h);
    let inside = |x: f32, y: f32| -> bool {
        // outer outline: the box less the two right corners
        let (dx, dy) = (w - x, y.min(h - y));
        if !(0.0..=w).contains(&x) || !(0.0..=h).contains(&y) || dx + dy < k {
            return false;
        }
        // counter: the box inset by t, its right corners cut a stem inside the outer cut
        let (cx, cy) = (x - t, y - t);
        let (cdx, cdy) = (w - t - x, y.min(h - y) - t);
        let counter = cx > 0.0 && cy > 0.0 && cdx > 0.0 && cdy > 0.0 && cdx + cdy > k - t * 0.4142;
        !counter
    };
    let n = 6;
    let mut hits = 0;
    for sy in 0..n {
        for sx in 0..n {
            if inside(
                gx as f32 + (sx as f32 + 0.5) / n as f32,
                gy as f32 + (sy as f32 + 0.5) / n as f32,
            ) {
                hits += 1;
            }
        }
    }
    hits as f32 / (n * n) as f32
}

/// A character as drawn: the glyph of its capital at small-capital size for a lower-case letter that has one capital
/// (as the original's font draws them), else its own at full size.
fn shape(c: char) -> (char, f32) {
    if c.is_lowercase() {
        let mut up = c.to_uppercase();
        if let (Some(u), None) = (up.next(), up.next())
            && u != c
        {
            return (u, SMALL_CAPS);
        }
    }
    (c, 1.0)
}

/// A Chinese, Japanese or Korean character: a line may break before or after it without a space.
fn cjk(c: char) -> bool {
    matches!(u32::from(c), 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

/// One character placed: its glyph, its size and its advance in context pixels.
struct Placed {
    c: char,
    glyph: Option<Glyph>,
    size: f32,
    advance: f32,
}

/// The characters of `text` with their glyphs, `kx` context pixels across per atlas pixel.
fn place_chars(atlas: &mut Atlas, text: &str, kx: f32) -> Vec<Placed> {
    text.chars()
        .take(TEXT_MAX)
        .filter(|&c| c != '\r')
        .map(|c| {
            if c == '\n' {
                return Placed {
                    c,
                    glyph: None,
                    size: 1.0,
                    advance: 0.0,
                };
            }
            let (g, size) = shape(c);
            let glyph = atlas.glyph(g);
            let advance = glyph.map_or(0.0, |g| (g.advance + TRACKING * EM) * size * kx);
            Placed {
                c,
                glyph,
                size,
                advance,
            }
        })
        .collect()
}

/// The lines (start, end, width) of placed characters `avail` wide: a break at a newline, and with `wrap` at the last
/// space or Chinese character that keeps a line inside, else in the word.
fn lines(chars: &[Placed], avail: f32, wrap: bool) -> Vec<(usize, usize, f32)> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let (mut w, mut end, mut next) = (0.0f32, chars.len(), chars.len());
        let mut brk: Option<(usize, f32, usize)> = None;
        for (i, p) in chars.iter().enumerate().skip(start) {
            if p.c == '\n' {
                (end, next) = (i, i + 1);
                break;
            }
            if p.c == ' ' {
                brk = Some((i, w, i + 1));
            } else if cjk(p.c) && i > start {
                brk = Some((i, w, i));
            }
            if wrap && w + p.advance > avail && i > start {
                (end, next) = match brk {
                    Some((e, _, n)) if e > start => (e, n),
                    _ => (i, i),
                };
                break;
            }
            w += p.advance;
        }
        let width: f32 = chars
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .map(|p| p.advance)
            .sum();
        // a line's trailing space does not count to its width
        let trail: f32 = chars
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .rev()
            .take_while(|p| p.c == ' ')
            .map(|p| p.advance)
            .sum();
        out.push((start, end, width - trail));
        start = next.max(start + 1);
    }
    out
}

/// `text` as 2D Text with `st` would draw it in the game's font, in the shipped fonts: in the context rectangle
/// `rect` (pixels of 640×480), its line `st.scale[1]` of the context high over 16 (a glyph cell of the game font).
/// Characters no shipped font has are left out.
pub fn styled(text: &str, rect: [f32; 4], st: &Style) -> Vec<Quad> {
    let base = if st.flags & TEXT_SCREEN != 0 {
        CONTEXT_H
    } else {
        512.0
    };
    let height = st.scale[1] * base / 16.0;
    let widen = if st.scale[1] > 0.0 {
        st.scale[0] / st.scale[1]
    } else {
        1.0
    };
    let r = [
        rect[0] + st.margins[0],
        rect[1] + st.margins[1],
        rect[2] - st.margins[2],
        rect[3] - st.margins[3],
    ];
    draw(text, r, height, widen, st)
}

fn draw(text: &str, r: [f32; 4], height: f32, widen: f32, st: &Style) -> Vec<Quad> {
    let Ok(mut guard) = ATLAS.lock() else {
        return Vec::new();
    };
    let atlas = guard.get_or_insert_with(Atlas::new);
    if atlas.fonts.is_empty() || !(height.is_finite() && height > 0.0) {
        return Vec::new();
    }
    // context pixels an atlas pixel spans: the capitals CAPS of the line
    let k = CAPS * height / atlas.cap;
    let kx = k * WIDE * widen.clamp(0.25, 4.0);
    let chars = place_chars(atlas, text, kx);
    let wrap = st.flags & (TEXT_WORDWRAP | 0x40) != 0;
    let lines = lines(&chars, r[2] - r[0], wrap);
    let line_h = height + st.spacing[1];
    let total = lines.len() as f32 * line_h;
    let mut top = if st.align & ALIGN_TOP != 0 {
        r[1]
    } else if st.align & ALIGN_BOTTOM != 0 {
        r[3] - total
    } else {
        (r[1] + r[3]) / 2.0 - total / 2.0
    };
    let mut out = Vec::new();
    for (start, end, width) in lines {
        let x0 = if st.align & ALIGN_LEFT != 0 {
            r[0]
        } else if st.align & ALIGN_RIGHT != 0 {
            r[2] - width
        } else {
            (r[0] + r[2]) / 2.0 - width / 2.0
        };
        // the capitals a little above the middle of the line, where the game font's sit in their cells (measured
        // on the original's menu: a fifth of a capital higher)
        let baseline = top + height / 2.0 + atlas.cap * k * 0.3;
        let line = chars.get(start..end).unwrap_or_default();
        let mut passes = Vec::new();
        if let Some((off, color)) = st.shadow {
            passes.push((off, color));
        }
        passes.push(([0.0, 0.0], st.color));
        for (off, color) in passes {
            let mut x = x0 + off[0];
            for p in line {
                if let Some(g) = p.glyph.filter(|g| g.uv[2] > g.uv[0]) {
                    let (sx, sy) = (kx * p.size, k * p.size);
                    let y = baseline + off[1];
                    out.push(Quad {
                        angle: 0.0,
                        rect: [
                            x + g.bounds[0] * sx,
                            y + g.bounds[1] * sy,
                            x + g.bounds[2] * sx,
                            y + g.bounds[3] * sy,
                        ],
                        uv: g.uv,
                        top: color,
                        bottom: color,
                        texture: TEXTURE,
                    });
                }
                x += p.advance;
            }
        }
        top += line_h;
    }
    out
}

/// The shipped fonts were found: the own interface can draw its texts.
pub fn available() -> bool {
    ATLAS
        .lock()
        .is_ok_and(|mut g| !g.get_or_insert_with(Atlas::new).fonts.is_empty())
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

    #[test]
    fn lower_case_is_drawn_as_small_capitals() {
        assert_eq!(shape('a'), ('A', SMALL_CAPS));
        assert_eq!(shape('ж'), ('Ж', SMALL_CAPS));
        assert_eq!(shape('A'), ('A', 1.0));
        assert_eq!(shape('ß'), ('ß', 1.0), "two capitals: its own glyph");
        assert_eq!(shape('中'), ('中', 1.0));
    }

    fn placed(s: &str) -> Vec<Placed> {
        s.chars()
            .map(|c| Placed {
                c,
                glyph: None,
                size: 1.0,
                advance: 10.0,
            })
            .collect()
    }

    #[test]
    fn lines_wrap_at_spaces_and_between_chinese_characters() {
        let l = lines(&placed("ab cd ef"), 55.0, true);
        assert_eq!(
            l.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(),
            [(0, 5), (6, 8)]
        );
        assert_eq!(l[0].2, 50.0);
        let l = lines(&placed("确定退出游戏吗"), 35.0, true);
        assert_eq!(
            l.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(),
            [(0, 3), (3, 6), (6, 7)]
        );
        let l = lines(&placed("one\ntwo"), 1000.0, false);
        assert_eq!(l.len(), 2);
        assert_eq!(
            lines(&placed("toolong"), 20.0, false).len(),
            1,
            "no wrap: one line"
        );
        assert!(lines(&placed(""), 20.0, true).is_empty());
    }
}
