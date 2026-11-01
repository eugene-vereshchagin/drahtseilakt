// The in-game interface: the original's points counter and life balls (`interface`, laid out as the C game shows
// them: the Gameplay scripts' 2D frames over Button01_special, the points in the game font), and the prototype's
// messages in a 5×7 pixel font written for this project (level complete, ball lost, game over; the debug line).

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
    ball_lost_at: Option<u64>,
    debug: bool,
    end_screen: bool,
) -> Vec<Rect> {
    let (w, h) = (width_px as f32, height_px as f32);
    let scale = (h / 240.0).round().max(2.0);
    let mut out = Vec::new();

    // The sector and the tick in the top left corner, in debug mode.
    let hud = format!(
        "SECTOR {}   TICK {}",
        sim.sector + 1,
        sim.tick
    );
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
            centre(&mut out, "ENTER NEXT LEVEL   ESC MENU", keys_top, scale, WHITE);
        } else {
            centre(&mut out, "PRESS R TO TRY AGAIN", line_top, small, WHITE);
            centre(&mut out, "ESC MENU", keys_top, scale, WHITE);
        }
    } else if let Some(at) = ball_lost_at.filter(|&at| sim.tick.saturating_sub(at) < 120) {
        let _ = at;
        centre(
            &mut out,
            "BALL LOST",
            mid - scale * 4.0,
            scale * 2.0,
            [1.0, 0.45, 0.45, 1.0],
        );
    }
    out
}

/// The points box and the life balls as the original draws them (rectangles and texture parts of its 2D frames, as
/// the C game places them; the points in Font_Punktezähler's style). One ball per life; each further one 0.0387 of
/// the context to the left, the end piece after the last.
pub fn interface(sim: &Sim, font: Option<&crate::ui::Font>, screen: crate::ui::Screen) -> Vec<crate::ui::Quad> {
    use crate::ui::{Anchor, Quad, homogeneous};
    const SPECIAL: &str = "Button01_special";
    let quad = |r: [f32; 4], uv: [f32; 4], anchor: Anchor| Quad {
        rect: screen.rect(homogeneous(r), anchor),
        uv,
        top: WHITE,
        bottom: WHITE,
        texture: SPECIAL,
    };
    let mut out = Vec::new();
    // on the end screen the counter's points and the lives move over to the tally
    let tally = crate::endscreen::of(sim);
    let lives = tally.map_or(sim.lives, |t| t.lives).min(40) as usize;
    let points = tally.map_or(sim.points, |t| t.counter);
    let first = 0.910_90_f32;
    for k in 0..lives {
        let x = first - k as f32 * 0.0387;
        out.push(quad([x, 0.885_40, x + 0.043_75, 0.943_73], [0.0667, 0.5294, 0.1765, 0.6392], Anchor::Right));
    }
    // with no ball left the end piece sits where a first ball would end (C game: 0.9288)
    let end = first - (lives as f32 - 1.0) * 0.0387 - 0.0208;
    out.push(quad([end, 0.881_30, end + 0.023_44, 0.943_80], [0.0040, 0.5255, 0.0600, 0.6410], Anchor::Right));
    out.push(quad([0.890_63, 0.85, 0.981_25, 0.977_09], [0.1843, 0.4667, 0.4078, 0.7020], Anchor::Right));
    out.push(quad([0.015, 0.87, 0.235, 0.98], [0.3255, 0.7300, 1.0, 0.9804], Anchor::Left));
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
        let digits = f.layout(points.max(0).to_string().as_bytes(), homogeneous([0.05, 0.884_17, 0.203_75, 0.934_17]), &style);
        out.extend(crate::ui::place(digits, screen, Anchor::Left));
    }
    out
}
