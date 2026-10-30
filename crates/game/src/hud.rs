// On-screen text for the prototype: a 5×7 pixel font written for this project (no game assets), drawn as
// solid quads over the frame, and the messages the game shows (points and lives, level complete, ball lost,
// game over). Replaced by the real HUD in phase 4.

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
pub fn overlay(sim: &Sim, width_px: u32, height_px: u32, ball_lost_at: Option<u64>) -> Vec<Rect> {
    let (w, h) = (width_px as f32, height_px as f32);
    let scale = (h / 240.0).round().max(2.0);
    let mut out = Vec::new();

    // Points, lives and sector in the top left corner, always.
    let hud = format!(
        "POINTS {}   LIVES {}   SECTOR {}",
        sim.points,
        sim.lives,
        sim.sector + 1
    );
    out.push(Rect {
        x: 0.0,
        y: 0.0,
        w: width(&hud, scale) + 2.0 * scale,
        h: 9.0 * scale,
        color: SHADE,
    });
    text(&mut out, &hud, scale, scale * 1.5, scale, WHITE);

    let centre = |out: &mut Vec<Rect>, s: &str, y: f32, scale: f32, color: [f32; 4]| {
        text(out, s, (w - width(s, scale)) / 2.0, y, scale, color);
    };
    let mid = h / 2.0;
    if sim.finished || sim.game_over {
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
        let band_bottom = line_top + small * 7.0 + scale * 4.0;
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
                sim.tick / 3600,
                (sim.tick % 3600) as f32 / 60.0
            );
            centre(&mut out, &time, line_top, small, WHITE);
        } else {
            centre(&mut out, "PRESS R TO TRY AGAIN", line_top, small, WHITE);
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
