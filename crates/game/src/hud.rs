// The in-game interface: the original's points counter and life balls (`interface`, laid out as the C game shows
// them: the Gameplay scripts' 2D frames over Button01_special, the points in the game font), the "ball lost" message
// in the game font as its other texts (`ball_lost`), and the prototype's messages in a 5×7 pixel font written for
// this project (level complete and game over without the game's font; the debug line).

use crate::sim::Sim;

const GLYPH_W: usize = 5;
const GLYPH_H: usize = 7;

/// Rows of a glyph, top to bottom; `#` is a lit pixel.
fn glyph(c: char) -> [&'static str; GLYPH_H] {
    match c.to_ascii_uppercase() {
        'A' => [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'B' => [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
        'C' => [
            ".###.", "#...#", "#....", "#....", "#....", "#...#", ".###.",
        ],
        'D' => [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
        'E' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
        'F' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
        'G' => [
            ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####",
        ],
        'H' => [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'I' => [
            ".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        'J' => [
            "..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
        'K' => [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
        'L' => [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
        'M' => [
            "#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#",
        ],
        'N' => [
            "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
        ],
        'O' => [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'P' => [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
        'Q' => [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
        'R' => [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
        'S' => [
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
        'T' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'U' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'V' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
        'W' => [
            "#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#",
        ],
        'X' => [
            "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
        ],
        'Y' => [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' => [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        '0' => [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
        '1' => [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        '2' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
        '3' => [
            "#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###.",
        ],
        '4' => [
            "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
        ],
        '5' => [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
        '6' => [
            "..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###.",
        ],
        '7' => [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
        '8' => [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
        '9' => [
            ".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##..",
        ],
        ':' => [
            ".....", "..#..", ".....", ".....", ".....", "..#..", ".....",
        ],
        '-' => [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
        '.' => [
            ".....", ".....", ".....", ".....", ".....", "..#..", ".....",
        ],
        '!' => [
            "..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#..",
        ],
        '/' => [
            "....#", "....#", "...#.", "..#..", ".#...", "#....", "#....",
        ],
        ',' => [
            ".....", ".....", ".....", ".....", "..#..", "..#..", ".#...",
        ],
        '%' => [
            "##..#", "##..#", "...#.", "..#..", ".#...", "#..##", "#..##",
        ],
        '_' => [
            ".....", ".....", ".....", ".....", ".....", ".....", "#####",
        ],
        '\\' => [
            "#....", "#....", ".#...", "..#..", "...#.", "....#", "....#",
        ],
        '(' => [
            "...#.", "..#..", ".#...", ".#...", ".#...", "..#..", "...#.",
        ],
        ')' => [
            ".#...", "..#..", "...#.", "...#.", "...#.", "..#..", ".#...",
        ],
        '>' => [
            "#....", ".#...", "..#..", "...#.", "..#..", ".#...", "#....",
        ],
        '~' => [
            ".....", ".....", ".#..#", "#.##.", ".....", ".....", ".....",
        ],
        '\'' => [
            "..#..", "..#..", ".....", ".....", ".....", ".....", ".....",
        ],
        '?' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#..",
        ],
        '+' => [
            ".....", "..#..", "..#..", "#####", "..#..", "..#..", ".....",
        ],
        '=' => [
            ".....", ".....", "#####", ".....", "#####", ".....", ".....",
        ],
        '[' => [
            ".###.", ".#...", ".#...", ".#...", ".#...", ".#...", ".###.",
        ],
        ']' => [
            ".###.", "...#.", "...#.", "...#.", "...#.", "...#.", ".###.",
        ],
        _ => [
            ".....", ".....", ".....", ".....", ".....", ".....", ".....",
        ],
    }
}

/// A coloured rectangle in pixels of the target, top left origin.
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
}

/// Lays out one line of text with its top left corner at (x, y) in pixels, `scale` pixels per font pixel.
pub fn text(out: &mut Vec<Rect>, s: &str, x: f32, y: f32, scale: f32, color: [f32; 4]) {
    let advance = (GLYPH_W as f32 + 1.0) * scale;
    for (n, c) in s.chars().enumerate() {
        let gx = x + n as f32 * advance;
        for (row, line) in glyph(c).iter().enumerate() {
            for (col, px) in line.bytes().enumerate() {
                if px == b'#' {
                    out.push(Rect {
                        x: gx + col as f32 * scale,
                        y: y + row as f32 * scale,
                        w: scale,
                        h: scale,
                        color,
                    });
                }
            }
        }
    }
}

/// Width in pixels of a line of text.
pub fn width(s: &str, scale: f32) -> f32 {
    s.chars().count() as f32 * (GLYPH_W as f32 + 1.0) * scale
}

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const SHADE: [f32; 4] = [0.0, 0.0, 0.0, 0.55];
const GOLD: [f32; 4] = [1.0, 0.85, 0.3, 1.0];

/// What the player sees over the frame, for the current state of the run.
/// `end_screen`: the original's end screen is drawn (endscreen.rs), so no banner for a finished level.
pub fn overlay(
    sim: &Sim,
    width_px: u32,
    height_px: u32,
    debug: bool,
    end_screen: bool,
) -> Vec<Rect> {
    let (w, h) = (width_px as f32, height_px as f32);
    let scale = (h / 240.0).round().max(2.0);
    // The map's own text goes first: under everything the game itself shows.
    let mut out = match &sim.scripts {
        Some(sc) if !sc.text.is_empty() => crate::script_text::rects(&sc.text, w, h),
        _ => Vec::new(),
    };

    // The sector and the tick in the top left corner, in debug mode.
    let hud = format!("SECTOR {}   TICK {}", sim.sector + 1, sim.tick);
    if debug {
        out.push(Rect {
            x: 0.0,
            y: 0.0,
            w: width(&hud, scale) + 2.0 * scale,
            h: 9.0 * scale,
            color: SHADE,
        });
        text(&mut out, &hud, scale, scale * 1.5, scale, WHITE);
    }

    let centre = |out: &mut Vec<Rect>, s: &str, y: f32, scale: f32, color: [f32; 4]| {
        text(out, s, (w - width(s, scale)) / 2.0, y, scale, color);
    };
    let mid = h / 2.0;
    if sim.finished && end_screen {
        // drawn by endscreen.rs
    } else if sim.finished || sim.game_over {
        // A banner with a title and one line under it, centred on the screen.
        let (title, color) = if sim.finished {
            ("LEVEL COMPLETE", GOLD)
        } else {
            ("GAME OVER", [1.0, 0.45, 0.45, 1.0])
        };
        let big = scale * 3.0;
        let small = scale * 1.5;
        let title_top = mid - big * 3.5;
        let line_top = title_top + big * 7.0 + scale * 3.0;
        let band_top = title_top - scale * 4.0;
        // a finished level shows its time and the score's parts on four lines
        let lines = if sim.finished { 5.0 } else { 1.0 };
        let keys_top = line_top + (small * 7.0 + scale * 3.0) * lines;
        let band_bottom = keys_top + scale * 7.0 + scale * 4.0;
        out.push(Rect {
            x: 0.0,
            y: band_top,
            w,
            h: band_bottom - band_top,
            color: SHADE,
        });
        centre(&mut out, title, title_top, big, color);
        if sim.finished {
            let time = format!(
                "TIME {:02}:{:05.2}",
                sim.finished_at / 3600,
                (sim.finished_at % 3600) as f32 / 60.0
            );
            centre(&mut out, &time, line_top, small, WHITE);
            let step = small * 7.0 + scale * 3.0;
            let parts = [
                format!("TIME POINTS {}", sim.points),
                format!("LEVEL BONUS {}", sim.level_bonus),
                format!("EXTRA LIVES {}", 200 * i64::from(sim.lives)),
                format!("SCORE {}", sim.score()),
            ];
            for (k, p) in parts.iter().enumerate() {
                let color = if k == 3 { GOLD } else { WHITE };
                centre(&mut out, p, line_top + step * (k + 1) as f32, small, color);
            }
            centre(
                &mut out,
                "ENTER NEXT LEVEL   ESC MENU",
                keys_top,
                scale,
                WHITE,
            );
        } else {
            centre(&mut out, "PRESS R TO TRY AGAIN", line_top, small, WHITE);
            centre(&mut out, "ESC MENU", keys_top, scale, WHITE);
        }
    }
    out
}

/// Ticks the "ball lost" message shows, from the loss: it fades in, stays and fades out before the new ball rolls.
const LOST_TICKS: u64 = 120;
const LOST_FADE_IN: u64 = 15;
const LOST_FADE_OUT: u64 = 30;
/// Where the message sits in the 640×480 context (homogeneous): centred, over the middle where the camera keeps the
/// falling ball.
const LOST_RECT: [f32; 4] = [0.0, 0.30, 1.0, 0.38];

/// The message's opacity `age` ticks after the loss (0 when it is not shown): linear fades, as the Gameplay scripts'
/// texts fade (an Interpolator on the font's colour over a Linear Progression).
fn lost_alpha(age: u64) -> f32 {
    if age >= LOST_TICKS {
        0.0
    } else if age < LOST_FADE_IN {
        (age + 1) as f32 / LOST_FADE_IN as f32
    } else {
        ((LOST_TICKS - age) as f32 / LOST_FADE_OUT as f32).min(1.0)
    }
}

/// The "ball lost" message as the game's texts are drawn: the interface's face in white with the shadow its fonts
/// have (2 off at 120 degrees, black at one half), the size of the points counter's digits, in the 4:3 context so
/// any window shows it alike.
pub fn ball_lost(
    sim: &Sim,
    lost_at: Option<u64>,
    text: &str,
    font: Option<&crate::ui::Face>,
    screen: crate::ui::Screen,
) -> Vec<crate::ui::Quad> {
    use crate::ui::{Anchor, Style, TEXT_SCREEN, homogeneous, place};
    let Some(at) = lost_at.filter(|_| !sim.finished && !sim.game_over) else {
        return Vec::new();
    };
    let a = lost_alpha(sim.tick.saturating_sub(at));
    let Some(f) = font.filter(|_| a > 0.0 && sim.tick >= at) else {
        return Vec::new();
    };
    let style = Style {
        scale: [0.8, 0.9],
        spacing: [1.5, 1.0],
        align: 0,
        flags: TEXT_SCREEN,
        margins: [0.0; 4],
        color: [1.0, 1.0, 1.0, a],
        v2: true,
        shadow: Some(([1.0, 1.732_05], [0.0, 0.0, 0.0, 0.502 * a])),
    };
    place(
        f.layout(text, homogeneous(LOST_RECT), &style),
        screen,
        Anchor::Center,
    )
}

/// A map's 2D frame as it is drawn: its rectangle in the 640 x 480 context, the texture's part, the material's
/// colour, and the level's texture it shows (kept by the renderer under `ui::FRAME_TEXTURES[slot]`, the frame's
/// place in the list).
#[derive(Debug, Clone, PartialEq)]
pub struct LevelFrame {
    /// The 2D entity's name (a script finds it by it: Edit 2D Entity).
    pub name: String,
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    pub color: [f32; 4],
    pub texture: usize,
    /// Drawn as the file has it (false: the file hides it, and only a script's Show draws it).
    pub shown: bool,
    /// Turn about the rectangle's centre, in degrees (Edit 2D Entity's rotation).
    pub angle: f32,
}

/// The frames of a level that can be drawn, in drawing order (by Z order, then as the file lists them): those
/// (shown or hidden by the file: a script may show one) with a material that has a texture and a rectangle with an area, as the original's render context draws
/// its 2D entities (C reference render.c). A frame without a texture is left out: a plain rectangle is what a
/// script of the map fills or fades, and would only cover the picture.
pub fn level_frames(level: &level::Level) -> Vec<LevelFrame> {
    let mut frames: Vec<(i32, LevelFrame)> = level
        .frames
        .iter()
        .filter_map(|f| {
            let m = level.materials.get(f.material?)?;
            let [l, t, r, b] = f.rect;
            let sane = f.rect.iter().chain(&f.uv).all(|v| v.is_finite()) && r > l && b > t;
            sane.then_some((
                f.z_order,
                LevelFrame {
                    name: f.name.clone(),
                    rect: f.rect,
                    uv: f.uv,
                    color: m.diffuse,
                    // a texture the level has (a material may name one it does not)
                    texture: m.texture.filter(|t| level.textures.get(*t).is_some())?,
                    shown: f.visible,
                    angle: 0.0,
                },
            ))
        })
        .collect();
    frames.sort_by_key(|f| f.0);
    frames
        .into_iter()
        .map(|f| f.1)
        .take(crate::ui::FRAME_TEXTURES.len())
        .collect()
}

/// Edit 2D Entity's frames over the level's: a frame a script gave a position or a size (values up to 2 are fractions of
/// the 640 x 480 frame, larger ones pixels) moves or resizes; what it did not edit stays as the file has it, and a
/// size that is nothing hides the frame. At most `FRAME_TEXTURES` frames, so this is bounded by the caller's list.
pub fn edited_frames(frames: &[LevelFrame], edits: &[(String, script::Ent2d)]) -> Vec<LevelFrame> {
    let px = |v: f32, whole: f32| if v.abs() <= 2.0 { v * whole } else { v };
    frames
        .iter()
        .filter_map(|f| {
            let Some((_, e)) = edits.iter().find(|(n, _)| *n == f.name) else {
                return Some(f.clone());
            };
            let mut r = f.rect;
            let (w, h) = (r[2] - r[0], r[3] - r[1]);
            let (mut x, mut y, mut w, mut h) = (r[0], r[1], w, h);
            if let Some([px_, py_]) = e.pos {
                if px_.is_finite() {
                    x = px(px_, 640.0);
                }
                if py_.is_finite() {
                    y = px(py_, 480.0);
                }
            }
            if let Some([sw, sh]) = e.size {
                if sw.is_finite() {
                    w = px(sw, 640.0);
                }
                if sh.is_finite() {
                    h = px(sh, 480.0);
                }
            }
            r = [x, y, x + w, y + h];
            // the engine's rotation is in radians; one that is not a number is none
            let angle = e
                .rotation
                .filter(|a| a.is_finite())
                .map_or(f.angle, f32::to_degrees);
            (w > 0.0 && h > 0.0 && r.iter().all(|v| v.is_finite())).then(|| LevelFrame {
                rect: r,
                angle,
                shown: e.shown.unwrap_or(f.shown),
                ..f.clone()
            })
        })
        .collect()
}

/// The quads of a level's frames: a frame keeps its distance to the edge of the window it is nearest to, as the
/// interface's own parts do.
fn frame_quads(frames: &[LevelFrame], screen: crate::ui::Screen) -> Vec<crate::ui::Quad> {
    use crate::ui::{Anchor, CONTEXT_W, FRAME_TEXTURES};
    frames
        .iter()
        .zip(FRAME_TEXTURES)
        .filter(|(f, _)| f.shown)
        .map(|(f, texture)| {
            let middle = (f.rect[0] + f.rect[2]) / 2.0;
            let anchor = if middle < CONTEXT_W / 3.0 {
                Anchor::Left
            } else if middle > CONTEXT_W * 2.0 / 3.0 {
                Anchor::Right
            } else {
                Anchor::Center
            };
            crate::ui::Quad {
                rect: screen.rect(f.rect, anchor),
                uv: f.uv,
                top: f.color,
                bottom: f.color,
                texture,
                angle: f.angle,
            }
        })
        .collect()
}

/// The points box and the life balls as the original draws them (rectangles and texture parts of its 2D frames, as
/// the C game places them; the points in Font_Punktezähler's style). One ball per life; each further one 0.0387 of
/// the context to the left, the end piece after the last.
pub fn interface(
    sim: &Sim,
    font: Option<&crate::ui::Face>,
    screen: crate::ui::Screen,
) -> Vec<crate::ui::Quad> {
    use crate::ui::{Anchor, Quad, homogeneous};
    const SPECIAL: &str = "Button01_special";
    let quad = |r: [f32; 4], uv: [f32; 4], anchor: Anchor| Quad {
        angle: 0.0,
        rect: screen.rect(homogeneous(r), anchor),
        uv,
        top: WHITE,
        bottom: WHITE,
        texture: SPECIAL,
    };
    // the map's own frames first: the game's interface is drawn over them
    let mut out = frame_quads(&sim.frames(), screen);
    // on the end screen the counter's points and the lives move over to the tally
    let tally = crate::endscreen::of(sim);
    let lives = tally.map_or(sim.lives, |t| t.lives).min(40) as usize;
    let points = tally.map_or(sim.points, |t| t.counter);
    let first = 0.910_90_f32;
    for k in 0..lives {
        let x = first - k as f32 * 0.0387;
        out.push(quad(
            [x, 0.885_40, x + 0.043_75, 0.943_73],
            [0.0667, 0.5294, 0.1765, 0.6392],
            Anchor::Right,
        ));
    }
    // with no ball left the end piece sits where a first ball would end (C game: 0.9288)
    let end = first - (lives as f32 - 1.0) * 0.0387 - 0.0208;
    out.push(quad(
        [end, 0.881_30, end + 0.023_44, 0.943_80],
        [0.0040, 0.5255, 0.0600, 0.6410],
        Anchor::Right,
    ));
    out.push(quad(
        [0.890_63, 0.85, 0.981_25, 0.977_09],
        [0.1843, 0.4667, 0.4078, 0.7020],
        Anchor::Right,
    ));
    out.push(quad(
        [0.015, 0.87, 0.235, 0.98],
        [0.3255, 0.7300, 1.0, 0.9804],
        Anchor::Left,
    ));
    if let Some(f) = font {
        let style = crate::ui::Style {
            scale: [0.8, 0.9],
            spacing: [1.5, 1.0],
            align: crate::ui::ALIGN_RIGHT,
            flags: crate::ui::TEXT_SCREEN,
            margins: [2.0; 4],
            color: WHITE,
            v2: true,
            shadow: None,
        };
        let digits = f.layout(
            &points.max(0).to_string(),
            homogeneous([0.05, 0.884_17, 0.203_75, 0.934_17]),
            &style,
        );
        out.extend(crate::ui::place(digits, screen, Anchor::Left));
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn edit_2d_entity_moves_and_resizes_a_frame_by_name() {
        let f = |name: &str| LevelFrame {
            name: name.into(),
            rect: [0.0, 0.0, 64.0, 48.0],
            uv: [0.0, 0.0, 1.0, 1.0],
            color: [1.0; 4],
            texture: 0,
            shown: true,
            angle: 0.0,
        };
        let frames = vec![f("a"), f("b")];
        let edit = script::Ent2d {
            pos: Some([0.5, 100.0]),
            size: Some([0.25, f32::NAN]),
            ..Default::default()
        };
        let out = edited_frames(&frames, &[("a".into(), edit)]);
        assert_eq!(
            out[0].rect,
            [320.0, 100.0, 480.0, 148.0],
            "fractions and pixels, a NaN part kept"
        );
        assert_eq!(out[1], frames[1], "an unnamed frame stays");
        let none = script::Ent2d {
            size: Some([0.0, 0.0]),
            ..Default::default()
        };
        assert_eq!(
            edited_frames(&frames, &[("b".into(), none)]).len(),
            1,
            "no size hides it"
        );
    }

    #[test]
    fn edit_2d_entity_rotates_a_frame_and_show_draws_one_the_file_hides() {
        let hidden = LevelFrame {
            name: "h".into(),
            rect: [100.0, 100.0, 200.0, 140.0],
            uv: [0.0, 0.0, 1.0, 1.0],
            color: [1.0; 4],
            texture: 0,
            shown: false,
            angle: 0.0,
        };
        let screen = crate::ui::Screen { w: 640.0, h: 480.0 };
        assert!(frame_quads(std::slice::from_ref(&hidden), screen).is_empty());
        let edit = script::Ent2d {
            rotation: Some(std::f32::consts::FRAC_PI_2),
            shown: Some(true),
            ..Default::default()
        };
        let out = edited_frames(std::slice::from_ref(&hidden), &[("h".into(), edit)]);
        assert!(out[0].shown);
        assert!((out[0].angle - 90.0).abs() < 1e-4, "radians become degrees");
        let q = frame_quads(&out, screen);
        assert_eq!(q.len(), 1);
        assert!((q[0].angle - 90.0).abs() < 1e-4);
        // a Hide over a shown frame, and a rotation that is not a number
        let shown = LevelFrame {
            shown: true,
            ..hidden
        };
        let edit = script::Ent2d {
            rotation: Some(f32::NAN),
            shown: Some(false),
            ..Default::default()
        };
        let out = edited_frames(&[shown], &[("h".into(), edit)]);
        assert!(frame_quads(&out, screen).is_empty());
        assert_eq!(out[0].angle, 0.0);
    }

    /// A map's frames as the original's render context draws them: shown ones with a material, by Z order; here
    /// only those with a texture, each anchored to the edge of the window it is nearest to.
    #[test]
    fn a_maps_frames_are_drawn_by_z_order_where_the_file_put_them() {
        let material = |texture: Option<usize>| level::Material {
            name: "m".into(),
            diffuse: [1.0, 0.5, 0.25, 1.0],
            ambient: [1.0; 3],
            emissive: [0.0; 3],
            specular: [0.0; 3],
            power: 0.0,
            texture,
            normal: None,
            parallax: 0.0,
            blend: None,
            alpha_cutoff: None,
            z_write: true,
            two_sided: false,
            unlit: false,
            texgen: None,
        };
        let frame =
            |rect: [f32; 4], z_order: i32, material: Option<usize>, visible: bool| level::Frame {
                name: "f".into(),
                rect,
                uv: [0.0, 0.0, 1.0, 1.0],
                z_order,
                material,
                visible,
            };
        let texture = level::Texture {
            name: "t".into(),
            source: level::TextureSource::None,
            color_key: None,
        };
        let level = level::Level {
            textures: vec![texture],
            materials: vec![material(Some(0)), material(None), material(Some(7))],
            frames: vec![
                frame([560.0, 0.0, 640.0, 60.0], 2, Some(0), true),
                frame([0.0, 0.0, 80.0, 60.0], 1, Some(0), true),
                frame([0.0, 0.0, 80.0, 60.0], 0, Some(0), false),
                frame([0.0, 0.0, 80.0, 60.0], 0, Some(1), true),
                frame([0.0, 0.0, 80.0, 60.0], 0, Some(2), true),
                frame([0.0, 0.0, 80.0, 60.0], 0, Some(9), true),
                frame([0.0, 0.0, 80.0, 60.0], 0, None, true),
                frame([80.0, 0.0, 0.0, 60.0], 0, Some(0), true),
                frame([0.0, f32::NAN, 80.0, 60.0], 0, Some(0), true),
            ],
            ..Default::default()
        };
        let frames = level_frames(&level);
        // the frame the file hides is kept (a script may show it), but is not drawn until then
        assert_eq!(frames.len(), 3);
        assert!(!frames[0].shown && frames[1].shown);
        assert_eq!(frames[1].rect, [0.0, 0.0, 80.0, 60.0]);
        assert_eq!(frames[2].color, [1.0, 0.5, 0.25, 1.0]);
        let frames: Vec<LevelFrame> = frames.into_iter().filter(|f| f.shown).collect();
        // a window twice as wide as 4:3 needs: the left frame at the left edge, the right one at the right edge
        let screen = crate::ui::Screen {
            w: 2560.0,
            h: 960.0,
        };
        let quads = frame_quads(&frames, screen);
        assert_eq!(quads[0].rect, [0.0, 0.0, 160.0, 120.0]);
        assert_eq!(quads[1].rect, [2400.0, 0.0, 2560.0, 120.0]);
        assert_eq!(quads[0].texture, crate::ui::FRAME_TEXTURES[0]);
        // never more frames than there are slots
        let many = level::Level {
            frames: vec![frame([0.0, 0.0, 8.0, 8.0], 0, Some(0), true); 40],
            ..level.clone()
        };
        assert_eq!(level_frames(&many).len(), crate::ui::FRAME_TEXTURES.len());
    }

    #[test]
    fn the_ball_lost_message_fades_in_stays_and_fades_out() {
        assert!(lost_alpha(0) > 0.0 && lost_alpha(0) < 0.2);
        assert_eq!(lost_alpha(LOST_FADE_IN - 1), 1.0);
        assert_eq!(lost_alpha(LOST_TICKS - LOST_FADE_OUT), 1.0);
        let out: Vec<f32> = (LOST_TICKS - LOST_FADE_OUT..LOST_TICKS)
            .map(lost_alpha)
            .collect();
        assert!(out.windows(2).all(|w| w[1] < w[0]) && out.iter().all(|&a| a > 0.0));
        assert_eq!(lost_alpha(LOST_TICKS), 0.0);
        assert_eq!(lost_alpha(u64::MAX), 0.0);
    }
}
