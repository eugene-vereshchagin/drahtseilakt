// The original's 2D interface: textured quads laid out in its 640×480 render context (homogeneous rectangles), and
// its texture font (Font_1 with the glyph table M_FontData_01, laid out as 2D Text does; docs/fonts.md). The 640×480
// layout sits in the middle of the window at 4:3, its left and right parts anchored to the window's edges, so any
// size and aspect ratio shows it whole.

use crate::assets::Data;

/// The render context the original lays its interface out in.
pub const CONTEXT_W: f32 = 640.0;
pub const CONTEXT_H: f32 = 480.0;

/// One textured quad in window pixels: its rectangle (left, top, right, bottom), texture coordinates (u0, v0, u1,
/// v1), the colours of its top and bottom edges, and the texture's name (loaded from the game's textures).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    pub top: [f32; 4],
    pub bottom: [f32; 4],
    pub texture: &'static str,
}

/// Which edge of the window an element keeps its distance to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Center: the menus' elements
pub enum Anchor {
    Left,
    Center,
    Right,
}

/// The window, and how the 640×480 context maps onto it.
#[derive(Clone, Copy, Debug)]
pub struct Screen {
    pub w: f32,
    pub h: f32,
}

impl Screen {
    /// Pixels a context pixel spans, and where the context's 4:3 frame begins.
    fn frame(self) -> (f32, f32, f32) {
        let s = (self.h / CONTEXT_H).min(self.w / CONTEXT_W).max(0.0);
        (
            (self.w - CONTEXT_W * s) / 2.0,
            (self.h - CONTEXT_H * s) / 2.0,
            s,
        )
    }

    /// A context x (pixels of 640) in the window.
    pub fn x(self, x: f32, anchor: Anchor) -> f32 {
        let (ox, _, s) = self.frame();
        match anchor {
            Anchor::Left => x * s,
            Anchor::Center => ox + x * s,
            Anchor::Right => self.w - (CONTEXT_W - x) * s,
        }
    }

    /// A context y (pixels of 480) in the window.
    pub fn y(self, y: f32) -> f32 {
        let (_, oy, s) = self.frame();
        oy + y * s
    }

    /// A rectangle in context pixels, in the window.
    pub fn rect(self, r: [f32; 4], anchor: Anchor) -> [f32; 4] {
        [
            self.x(r[0], anchor),
            self.y(r[1]),
            self.x(r[2], anchor),
            self.y(r[3]),
        ]
    }
}

/// A homogeneous rectangle (fractions of the context) in context pixels.
pub fn homogeneous(r: [f32; 4]) -> [f32; 4] {
    [
        r[0] * CONTEXT_W,
        r[1] * CONTEXT_H,
        r[2] * CONTEXT_W,
        r[3] * CONTEXT_H,
    ]
}

/// A texture font: its texture and glyph table (u, v, width, advance before, advance after, height; texture units).
#[derive(Clone, Debug)]
pub struct Font {
    pub texture: &'static str,
    tex_size: [f32; 2],
    glyphs: Vec<[f32; 6]>,
}

/// Alignment bits (2D Text): left, right, top, bottom; none of a pair centres.
pub const ALIGN_LEFT: u32 = 1;
pub const ALIGN_RIGHT: u32 = 2;
pub const ALIGN_TOP: u32 = 4;
pub const ALIGN_BOTTOM: u32 = 8;
/// Text properties: sizes relative to the render context, and word wrap.
pub const TEXT_SCREEN: u32 = 1;
pub const TEXT_WORDWRAP: u32 = 0x20;

/// How a 2D Text draws with a font (Set Font Properties and the 2D Text's pins and settings).
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub scale: [f32; 2],
    pub spacing: [f32; 2],
    pub align: u32,
    pub flags: u32,
    pub margins: [f32; 4],
    pub color: [f32; 4],
    /// The 2D Text block is version 2 or later: the letter spacing is a tenth of the space's advance times
    /// spacing.x, not pixels.
    pub v2: bool,
    /// The font's shadow (Set Font Properties' Shadow): drawn first, this many pixels off, in this colour.
    pub shadow: Option<([f32; 2], [f32; 4])>,
}

impl Font {
    /// The game's font: `Font_1` with the glyph table `M_FontData_01` (Menu.nmo), as TT CreateFontEx builds it.
    pub fn game(data: &Data) -> Option<Font> {
        let table = data.data_array("3D Entities/Menu.nmo", "M_FontData_01")?;
        let mut glyphs = vec![[0.0f32; 6]; 256];
        let mut last = [0.0f32; 6];
        for (k, g) in glyphs.iter_mut().enumerate().take(255) {
            if let Some(row) = table.rows.get(k) {
                for (c, v) in last.iter_mut().enumerate() {
                    if let Some(nmo::scene::Cell::Float(f)) = row.get(c) {
                        *v = if f.is_finite() { *f } else { 0.0 };
                    }
                }
            }
            *g = last;
        }
        Some(Font {
            texture: "Font_1",
            tex_size: [512.0, 512.0],
            glyphs,
        })
    }

    fn g(&self, c: u8) -> [f32; 6] {
        self.glyphs.get(usize::from(c)).copied().unwrap_or_default()
    }

    /// `text` laid out in the context rectangle `rect` (pixels of 640×480) as 2D Text draws it: the glyph quads in
    /// context pixels.
    pub fn layout(&self, text: &[u8], rect: [f32; 4], st: &Style) -> Vec<Quad> {
        let r = [
            rect[0] + st.margins[0],
            rect[1] + st.margins[1],
            rect[2] - st.margins[2],
            rect[3] - st.margins[3],
        ];
        let (bw, bh) = if st.flags & TEXT_SCREEN != 0 {
            (CONTEXT_W, CONTEXT_H)
        } else {
            (self.tex_size[0], self.tex_size[1])
        };
        let sx = st.scale[0] * bw;
        let sy = st.scale[1] * bh;
        let sp = self.g(b' ');
        let space = sp[2] + sp[3] + sp[4];
        let ls = if st.v2 {
            space * st.spacing[0] * sx * 0.1
        } else {
            st.spacing[0]
        };
        let adv = space * sx + ls;
        let avail = r[2] - r[0];
        let lines = self.lines(text, sx, ls, adv, avail, st.flags);
        let gh = sy * self.g(0)[5];
        let line_h = gh + st.spacing[1];
        let total = lines.len() as f32 * line_h;
        let mut y = if st.align & ALIGN_TOP != 0 {
            r[1]
        } else if st.align & ALIGN_BOTTOM != 0 {
            r[3] - total
        } else {
            (r[1] + r[3]) / 2.0 - total / 2.0
        };
        let mut out = Vec::new();
        for (start, end, width) in lines {
            let x = if st.align & ALIGN_LEFT != 0 {
                r[0]
            } else if st.align & ALIGN_RIGHT != 0 {
                r[2] - width
            } else {
                (r[0] + r[2]) / 2.0 - width / 2.0
            };
            let line = text.get(start..end).unwrap_or_default();
            if let Some((off, color)) = st.shadow {
                self.line(
                    line,
                    x + off[0],
                    y + off[1],
                    sx,
                    gh,
                    adv,
                    ls,
                    color,
                    &mut out,
                );
            }
            self.line(line, x, y, sx, gh, adv, ls, st.color, &mut out);
            y += line_h;
        }
        out
    }

    /// Line breaking (docs/fonts.md 5.1): (start, end, width) of each line.
    fn lines(
        &self,
        text: &[u8],
        sx: f32,
        ls: f32,
        adv: f32,
        avail: f32,
        flags: u32,
    ) -> Vec<(usize, usize, f32)> {
        let mut out = Vec::new();
        let wrap = flags & (TEXT_WORDWRAP | 0x40) != 0;
        let mut p = 0;
        while p < text.len() {
            let start = p;
            let mut w = 0.0;
            let mut brk: Option<(usize, f32, usize)> = None;
            let mut in_word = true;
            let mut end = None;
            while let Some(&c) = text.get(p).filter(|&&c| c != b'\n') {
                if c == b' ' {
                    in_word = true;
                    brk = Some((p, w, p + 1));
                    w += adv;
                } else {
                    if in_word && p > start {
                        brk = Some((p, w, p));
                    }
                    in_word = false;
                    let g = self.g(c);
                    w += (g[3] + g[2] + g[4]) * sx + ls;
                }
                if wrap && w > avail {
                    match brk {
                        Some((e, bw, next)) if e > start => {
                            end = Some((e, bw));
                            p = next;
                        }
                        _ => {
                            let g = self.g(c);
                            if c != b' ' && p > start {
                                w -= (g[3] + g[2] + g[4]) * sx + ls;
                            } else {
                                p += 1;
                            }
                            end = Some((p, w));
                        }
                    }
                    break;
                }
                p += 1;
            }
            match end {
                Some((e, ew)) => out.push((start, e, ew)),
                None => {
                    out.push((start, p, w));
                    if text.get(p) == Some(&b'\n') {
                        p += 1;
                    }
                }
            }
        }
        out
    }

    /// One line's glyphs from (x, y) (docs/fonts.md 5.3): snapped to whole pixels, half-texel inset.
    #[allow(clippy::too_many_arguments)]
    fn line(
        &self,
        s: &[u8],
        x: f32,
        y: f32,
        sx: f32,
        gh: f32,
        adv: f32,
        ls: f32,
        color: [f32; 4],
        out: &mut Vec<Quad>,
    ) {
        let mut x = (x + 0.5).floor();
        let y = (y + 0.5).floor();
        let ls = (ls + 0.5).floor();
        let (du, dv) = (0.25 / self.tex_size[0], 0.25 / self.tex_size[1]);
        for &c in s {
            if c == b' ' {
                x += adv;
                continue;
            }
            let g = self.g(c);
            if g[5] == 0.0 {
                continue;
            }
            let x0 = x + sx * g[3];
            let x1 = x0 + sx * g[2];
            out.push(Quad {
                rect: [(x0 + 0.5).floor(), y, (x1 + 0.5).floor(), y + gh],
                uv: [g[0] + du, g[1] + dv, g[0] + g[2], g[1] + g[5]],
                top: color,
                bottom: color,
                texture: self.texture,
            });
            x = x1 + sx * g[4] + ls;
        }
    }
}

/// The font the interface's texts are drawn in: the copy's game font (the original's interface), or the own one in
/// the shipped fonts (fallback.rs). Either draws a text as 2D Text with a style would; the game font hands what it
/// cannot draw (beyond Latin-1: Russian, Chinese) to the shipped fonts.
pub enum Face {
    Game(Font),
    Own,
}

impl Face {
    /// The interface's face: the own one, or for the original's (or its remaster) the copy's font when it has one.
    /// None when neither can draw (no copy font and no shipped fonts).
    pub fn new(data: &Data, provider: gamedata::resources::Provider) -> Option<Face> {
        let game = (provider != gamedata::resources::Provider::Own)
            .then(|| Font::game(data))
            .flatten();
        match game {
            Some(f) => Some(Face::Game(f)),
            None => crate::fallback::available().then_some(Face::Own),
        }
    }

    /// `text` laid out in the context rectangle `rect` as 2D Text with `st` draws it: glyph quads in context pixels.
    pub fn layout(&self, text: &str, rect: [f32; 4], st: &Style) -> Vec<Quad> {
        match self {
            Face::Game(f) if !crate::fallback::needed(text) => {
                let bytes: Vec<u8> = text
                    .chars()
                    .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
                    .collect();
                f.layout(&bytes, rect, st)
            }
            _ => crate::fallback::styled(text, rect, st),
        }
    }

    /// The own interface's face.
    pub fn own(&self) -> bool {
        matches!(self, Face::Own)
    }
}

/// The own interface's picture behind its menus and loading screen (assets/own/Textures/Menu_Background.png).
pub const BACKDROP: &str = "Menu_Background";
/// Its aspect ratio, as drawn by assets/own/make.
const BACKDROP_ASPECT: f32 = 16.0 / 9.0;

/// The own interface's picture over the whole window, cut to its shape (never stretched).
pub fn backdrop(screen: Screen) -> Quad {
    let aspect = if screen.h > 0.0 {
        screen.w / screen.h
    } else {
        BACKDROP_ASPECT
    };
    let uv = if aspect >= BACKDROP_ASPECT {
        let v = (1.0 - BACKDROP_ASPECT / aspect) / 2.0;
        [0.0, v, 1.0, 1.0 - v]
    } else {
        let u = (1.0 - aspect / BACKDROP_ASPECT) / 2.0;
        [u, 0.0, 1.0 - u, 1.0]
    };
    Quad {
        rect: [0.0, 0.0, screen.w, screen.h],
        uv,
        top: [1.0; 4],
        bottom: [1.0; 4],
        texture: BACKDROP,
    }
}

/// Quads in context pixels placed in the window with `anchor`.
pub fn place(quads: Vec<Quad>, screen: Screen, anchor: Anchor) -> Vec<Quad> {
    quads
        .into_iter()
        .map(|q| Quad {
            rect: screen.rect(q.rect, anchor),
            ..q
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A font whose every glyph is `w` wide with no side bearings, 1/16 high.
    fn font(w: f32) -> Font {
        Font {
            texture: "Font_1",
            tex_size: [512.0, 512.0],
            glyphs: vec![[0.0, 0.0, w, 0.0, 0.0, 0.0625]; 256],
        }
    }

    #[test]
    fn right_aligned_text_ends_at_the_margin_and_is_centred_vertically() {
        let f = font(15.0 / 512.0);
        let st = Style {
            scale: [1.0, 0.9],
            spacing: [0.0, 1.0],
            align: ALIGN_RIGHT,
            flags: 0,
            margins: [2.0; 4],
            color: [1.0; 4],
            v2: false,
            shadow: None,
        };
        let q = f.layout(b"953", [32.0, 424.4, 130.4, 448.4], &st);
        assert_eq!(q.len(), 3);
        // 512 × 0.9 / 16 = 28.8 high, 29.8 a line; centred about 436.4
        assert_eq!(q[0].rect[1], (436.4f32 - 14.9 + 0.5).floor());
        assert!((q[2].rect[2] - 128.0).abs() <= 1.0, "{:?}", q[2].rect);
    }

    /// The C game draws a level button's label ("Level 2", GameFont_03) and the points (Font_Punktezähler) with
    /// these glyph quads (DUMPQUADS of the C build; x less its -0.25 pixel tweak).
    #[test]
    fn the_game_font_lays_out_as_in_the_c_game() {
        let Some(data) =
            std::env::var_os("OPENBALLANCE_DATA").and_then(|_| Data::locate(None).ok())
        else {
            return;
        };
        let f = Font::game(&data).expect("M_FontData_01");
        let st = Style {
            scale: [0.35, 0.4],
            spacing: [0.0, 0.0],
            align: 0,
            flags: TEXT_SCREEN,
            margins: [2.0; 4],
            color: [1.0; 4],
            v2: true,
            shadow: None,
        };
        let q = f.layout(b"Level 2", homogeneous([0.4031, 0.15, 0.5969, 0.2]), &st);
        let first = q.first().expect("glyphs");
        let last = q.last().expect("glyphs");
        assert_eq!(
            (first.rect[0], first.rect[1], first.rect[3]),
            (297.0, 78.0, 90.0)
        );
        assert!(
            (first.uv[0] - 0.75635).abs() < 1e-4 && (first.uv[1] - 0.25049).abs() < 1e-4,
            "{:?}",
            first.uv
        );
        assert_eq!((last.rect[0], last.rect[2]), (337.0, 343.0));
        let points = Style {
            scale: [0.8, 0.9],
            spacing: [1.5, 1.0],
            align: ALIGN_RIGHT,
            flags: TEXT_SCREEN,
            margins: [2.0; 4],
            color: [1.0; 4],
            v2: true,
            shadow: None,
        };
        let q = f.layout(
            b"953",
            homogeneous([0.05, 0.884_17, 0.203_75, 0.934_17]),
            &points,
        );
        let x: Vec<[f32; 2]> = q.iter().map(|g| [g.rect[0], g.rect[2]]).collect();
        assert_eq!(x, [[67.0, 82.0], [89.0, 104.0], [111.0, 125.0]]);
        assert_eq!(q[0].rect[1], 422.0);
    }

    #[test]
    fn the_context_keeps_its_edges_on_a_wide_window() {
        let s = Screen {
            w: 1920.0,
            h: 720.0,
        };
        assert_eq!(s.x(0.0, Anchor::Left), 0.0);
        assert_eq!(s.x(640.0, Anchor::Right), 1920.0);
        assert_eq!(s.x(320.0, Anchor::Center), 960.0);
        assert_eq!(s.y(480.0), 720.0);
    }
}
