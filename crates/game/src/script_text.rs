// The text a map's scripts show (Text Display, 2D Text): boxes in the original's 640x480 frame, laid out here in the
// project's own 5x7 pixel font and drawn as rectangles under the interface (hud::overlay). A text that does not fit
// its box is shrunk, then cut. The layout is pure and never touches the simulation.

use crate::hud::{Rect, text};
use script::{MAX_TEXT_BOXES, MAX_TEXT_CHARS, TextSpec, text_align};

const FRAME_W: f32 = 640.0;
const FRAME_H: f32 = 480.0;
/// A line is 7 font pixels of glyph and 2 of leading; a glyph is 5 wide with 1 between.
const LINE: f32 = 9.0;
const ADVANCE: f32 = 6.0;
/// The smallest a font pixel gets (frame pixels) before the text is cut instead of shrunk further.
const MIN_PIXEL: f32 = 0.6;
/// The most glyphs drawn in a frame over all boxes (each is a few dozen rectangles).
const MAX_GLYPHS: usize = 2_048;

/// One laid out line: its text and its left, top in frame pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
}

/// A box laid out: the size of a font pixel in frame pixels and its lines.
#[derive(Debug, Clone, PartialEq)]
pub struct Laid {
    pub pixel: f32,
    pub lines: Vec<Line>,
}

fn cells(s: &str) -> usize {
    s.chars().count()
}

/// Breaks `text` into lines of at most `max` characters: at '\n', then at spaces, and inside a word that is
/// longer than a line.
fn wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut cur = String::new();
        for word in para.split(' ').filter(|w| !w.is_empty()) {
            let mut word = word.to_string();
            loop {
                let need = cells(&cur)
                    .saturating_add(usize::from(!cur.is_empty()))
                    .saturating_add(cells(&word));
                if need <= max {
                    if !cur.is_empty() {
                        cur.push(' ');
                    }
                    cur.push_str(&word);
                    break;
                }
                if !cur.is_empty() {
                    lines.push(std::mem::take(&mut cur));
                    continue;
                }
                // a word longer than the line: its head fills a line
                let head: String = word.chars().take(max.max(1)).collect();
                let rest: String = word.chars().skip(max.max(1)).collect();
                lines.push(head);
                word = rest;
                if word.is_empty() {
                    break;
                }
            }
        }
        lines.push(cur);
    }
    lines
}

/// Lays a box out: the wanted size first, shrunk while the text does not fit, then cut at the box's edge.
pub fn layout(spec: &TextSpec) -> Laid {
    let text: String = spec.text.chars().take(MAX_TEXT_CHARS).collect();
    let (w, h) = (spec.w.max(0.0), spec.h.max(0.0));
    let mut pixel = (spec.size / LINE).clamp(MIN_PIXEL, FRAME_H);
    let mut lines;
    loop {
        let per_line = (w / (ADVANCE * pixel)).floor().max(0.0) as usize;
        lines = wrap(&text, per_line);
        let fits = (lines.len() as f32) * LINE * pixel <= h
            && lines
                .iter()
                .all(|l| (cells(l) as f32) * ADVANCE * pixel <= w);
        if fits || pixel <= MIN_PIXEL {
            break;
        }
        pixel = (pixel * 0.9).max(MIN_PIXEL);
    }
    // cut: the lines that fit the height, each as long as fits the width
    let rows = if LINE * pixel > 0.0 {
        (h / (LINE * pixel)).floor() as usize
    } else {
        0
    };
    lines.truncate(rows);
    let per_line = (w / (ADVANCE * pixel)).floor().max(0.0) as usize;
    let total = lines.len() as f32 * LINE * pixel;
    let top = if spec.align & text_align::TOP != 0 {
        spec.y
    } else if spec.align & text_align::BOTTOM != 0 {
        spec.y + h - total
    } else {
        spec.y + (h - total) / 2.0
    };
    let lines = lines
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            let l: String = l.trim_end().chars().take(per_line).collect();
            let lw = cells(&l) as f32 * ADVANCE * pixel;
            let x = if spec.align & text_align::LEFT != 0 {
                spec.x
            } else if spec.align & text_align::RIGHT != 0 {
                spec.x + w - lw
            } else {
                spec.x + (w - lw) / 2.0
            };
            Line {
                text: l,
                x,
                y: top + i as f32 * LINE * pixel,
                w: lw,
            }
        })
        .collect();
    Laid { pixel, lines }
}

/// The rectangles of all the boxes in a window of `width` x `height` pixels: the 640x480 frame in the middle at
/// 4:3, as the interface's own.
pub fn rects(boxes: &[(script::Id, TextSpec)], width: f32, height: f32) -> Vec<Rect> {
    let s = (width / FRAME_W).min(height / FRAME_H).max(0.0);
    let (ox, oy) = ((width - FRAME_W * s) / 2.0, (height - FRAME_H * s) / 2.0);
    let mut out = Vec::new();
    let mut glyphs = 0usize;
    for (_, spec) in boxes.iter().take(MAX_TEXT_BOXES) {
        let laid = layout(spec);
        for line in &laid.lines {
            glyphs = glyphs.saturating_add(cells(&line.text));
            if glyphs > MAX_GLYPHS {
                return out;
            }
            let (x, y, px) = (ox + line.x * s, oy + line.y * s, laid.pixel * s);
            // a dark shadow one font pixel down and right, so the text reads on a bright sky too
            let shadow = [0.0, 0.0, 0.0, spec.color[3] * 0.7];
            let d = px.max(1.0);
            text(&mut out, &line.text, x + d, y + d, px, shadow);
            if spec.bold {
                text(&mut out, &line.text, x + px * 0.6 + d, y + d, px, shadow);
            }
            text(&mut out, &line.text, x, y, px, spec.color);
            if spec.bold {
                text(&mut out, &line.text, x + px * 0.6, y, px, spec.color);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(text: &str, x: f32, y: f32, w: f32, h: f32, size: f32, align: u8) -> TextSpec {
        TextSpec {
            text: text.into(),
            x,
            y,
            w,
            h,
            size,
            bold: false,
            color: [1.0; 4],
            align,
        }
    }

    #[test]
    fn boxes_fit_their_frame_whatever_the_text() {
        let long = "word ".repeat(300);
        let cases = [
            spec("short", 10.0, 10.0, 320.0, 32.0, 20.0, text_align::LEFT),
            spec(&long, 5.0, 5.0, 200.0, 40.0, 40.0, 0),
            spec(
                &long,
                100.0,
                100.0,
                50.0,
                12.0,
                300.0,
                text_align::RIGHT | text_align::BOTTOM,
            ),
            spec(
                &"W".repeat(1000),
                0.0,
                0.0,
                640.0,
                480.0,
                200.0,
                text_align::TOP,
            ),
            spec("a\n\n\n\n\nb\nc\nd", 600.0, 440.0, 40.0, 40.0, 32.0, 0),
            spec("tiny box", 1.0, 1.0, 3.0, 3.0, 20.0, 0),
            spec("zero", 1.0, 1.0, 0.0, 0.0, 20.0, 0),
        ];
        for c in &cases {
            let laid = layout(c);
            for l in &laid.lines {
                let eps = 0.01;
                assert!(
                    l.x >= c.x - eps && l.x + l.w <= c.x + c.w + eps,
                    "{l:?} outside {c:?}"
                );
                assert!(
                    l.y >= c.y - eps && l.y + LINE * laid.pixel <= c.y + c.h + eps,
                    "{l:?} outside {c:?}"
                );
            }
            assert!(laid.lines.iter().all(|l| cells(&l.text) <= MAX_TEXT_CHARS));
        }
        assert!(layout(&cases[0]).lines.len() == 1);
        // the 1000 W: shrunk first (the box is large), only then cut
        assert!(layout(&cases[3]).pixel < 200.0 / LINE);
    }

    #[test]
    fn a_text_that_fits_keeps_its_size_and_is_placed_by_its_alignment() {
        let left = layout(&spec(
            "hi",
            10.0,
            10.0,
            100.0,
            50.0,
            18.0,
            text_align::LEFT | text_align::TOP,
        ));
        assert_eq!(left.pixel, 2.0);
        assert_eq!((left.lines[0].x, left.lines[0].y), (10.0, 10.0));
        let right = layout(&spec(
            "hi",
            10.0,
            10.0,
            100.0,
            50.0,
            18.0,
            text_align::RIGHT | text_align::BOTTOM,
        ));
        assert_eq!(right.lines[0].x, 10.0 + 100.0 - 24.0);
        assert_eq!(right.lines[0].y, 10.0 + 50.0 - 18.0);
    }

    #[test]
    fn the_glyph_and_box_caps_hold() {
        let boxes: Vec<_> = (0..200)
            .map(|i| (i, spec(&"x".repeat(1024), 0.0, 0.0, 640.0, 480.0, 9.0, 0)))
            .collect();
        let r = rects(&boxes, 640.0, 480.0);
        assert!(r.len() < MAX_GLYPHS * 36);
    }
}
