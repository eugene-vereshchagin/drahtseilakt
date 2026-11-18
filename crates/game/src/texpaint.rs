// The pixel work of Blend Textures and Write In Texture on a per-run copy of a level texture (matstate.rs keeps the
// copies). Pure functions over RGBA buffers; nothing here reads a file or the clock.
// Reason for the allow: texture painting over images already decoded and size-capped by the image crate.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use std::sync::Arc;

/// A picture: RGBA, 4 bytes a pixel, row by row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct Pix {
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<Vec<u8>>,
}

impl Pix {
    /// The pixel at (x, y), the picture stretched over `of` (the size of what it is drawn into) by nearest sampling.
    fn sample(&self, x: u32, y: u32, of: (u32, u32)) -> [u8; 4] {
        let sx = (u64::from(x) * u64::from(self.width) / u64::from(of.0.max(1))) as usize;
        let sy = (u64::from(y) * u64::from(self.height) / u64::from(of.1.max(1))) as usize;
        let at = (sy * self.width as usize + sx) * 4;
        match self.rgba.get(at..at + 4) {
            Some([r, g, b, a]) => [*r, *g, *b, *a],
            _ => [0; 4],
        }
    }
}

/// The side of the largest square Write In Texture writes.
pub const MAX_PEN: i32 = 256;

/// Blend Textures: a * (1 - factor) + b * factor in every channel, at the size `(w, h)` (both sources stretched to it).
pub fn blend(w: u32, h: u32, a: &Pix, b: &Pix, factor: f32) -> Vec<u8> {
    let f = if factor.is_finite() {
        factor.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h {
        for x in 0..w {
            let pa = a.sample(x, y, (w, h));
            let pb = b.sample(x, y, (w, h));
            for c in 0..4 {
                let v = f32::from(pa[c]) * (1.0 - f) + f32::from(pb[c]) * f;
                out.push(v.round().clamp(0.0, 255.0) as u8);
            }
        }
    }
    out
}

/// Write In Texture: a square of `size` pixels (1 ..= MAX_PEN) centred on the pixel `uv`, clipped to the picture, of
/// `color` (RGBA, 0..1), or of the `pen` picture stretched to the square (its own colours and alpha). Write Mode:
/// 0 replaces all four channels; 1 (also any other value) replaces the colour and keeps the texture's alpha;
/// 2 mixes by the written alpha; 3 adds the colour (clamped). Returns how many pixels it wrote.
pub fn write(
    rgba: &mut [u8],
    (w, h): (u32, u32),
    uv: [f32; 2],
    color: [f32; 4],
    size: i32,
    pen: Option<&Pix>,
    mode: i32,
) -> u64 {
    if !uv[0].is_finite() || !uv[1].is_finite() || w == 0 || h == 0 {
        return 0;
    }
    let size = size.clamp(1, MAX_PEN);
    let side = size as u32;
    let x0 = uv[0].floor() as i64 - i64::from((size - 1) / 2);
    let y0 = uv[1].floor() as i64 - i64::from((size - 1) / 2);
    let col = color.map(|c| {
        if c.is_finite() {
            (c.clamp(0.0, 1.0) * 255.0).round() as u8
        } else {
            0
        }
    });
    let mut n = 0u64;
    for dy in 0..side {
        for dx in 0..side {
            let (x, y) = (x0 + i64::from(dx), y0 + i64::from(dy));
            if x < 0 || y < 0 || x >= i64::from(w) || y >= i64::from(h) {
                continue;
            }
            let src = match pen {
                Some(p) => p.sample(dx, dy, (side, side)),
                None => col,
            };
            let at = (y as usize * w as usize + x as usize) * 4;
            let Some(px) = rgba.get_mut(at..at + 4) else {
                continue;
            };
            match mode {
                0 => px.copy_from_slice(&src),
                2 => {
                    let a = f32::from(src[3]) / 255.0;
                    for c in 0..3 {
                        let v = f32::from(px[c]) * (1.0 - a) + f32::from(src[c]) * a;
                        px[c] = v.round().clamp(0.0, 255.0) as u8;
                    }
                }
                3 => {
                    for c in 0..3 {
                        px[c] = px[c].saturating_add(src[c]);
                    }
                }
                _ => px[..3].copy_from_slice(&src[..3]),
            }
            n += 1;
        }
    }
    n
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn flat(w: u32, h: u32, c: [u8; 4]) -> Pix {
        Pix {
            width: w,
            height: h,
            rgba: Arc::new(c.repeat((w * h) as usize)),
        }
    }

    #[test]
    fn blend_mixes_and_stretches() {
        let a = flat(2, 2, [0, 0, 0, 255]);
        let b = flat(1, 1, [200, 100, 50, 255]);
        let r = blend(4, 4, &a, &b, 0.5);
        assert_eq!(r.len(), 4 * 4 * 4);
        assert_eq!(&r[..4], &[100, 50, 25, 255]);
        assert_eq!(blend(4, 4, &a, &b, 0.0), a_stretched(4));
        assert_eq!(&blend(1, 1, &a, &b, 1.0)[..], &[200, 100, 50, 255]);
        // a factor that is no number or past the range is held to it
        assert_eq!(blend(1, 1, &a, &b, f32::NAN), blend(1, 1, &a, &b, 0.0));
        assert_eq!(blend(1, 1, &a, &b, 9.0), blend(1, 1, &a, &b, 1.0));
    }

    fn a_stretched(n: u32) -> Vec<u8> {
        [0, 0, 0, 255].repeat((n * n) as usize)
    }

    #[test]
    fn write_clips_and_counts() {
        let mut px = vec![10u8; 4 * 4 * 4];
        let n = write(
            &mut px,
            (4, 4),
            [0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            3,
            None,
            1,
        );
        // a 3 x 3 square centred on (0, 0): only 2 x 2 of it is inside; alpha is kept in mode 1
        assert_eq!(n, 4);
        assert_eq!(&px[..4], &[255, 0, 0, 10]);
        assert_eq!(&px[(4 + 1) * 4..(4 + 1) * 4 + 4], &[255, 0, 0, 10]);
        assert_eq!(&px[2 * 4..2 * 4 + 4], &[10; 4]);
        assert_eq!(
            write(&mut px, (4, 4), [f32::NAN, 0.0], [1.0; 4], 1, None, 0),
            0
        );
        assert_eq!(write(&mut px, (4, 4), [50.0, 0.0], [1.0; 4], 1, None, 0), 0);
        write(
            &mut px,
            (4, 4),
            [3.0, 3.0],
            [0.0, 1.0, 0.0, 0.5],
            1,
            None,
            0,
        );
        assert_eq!(&px[(3 * 4 + 3) * 4..], &[0, 255, 0, 128]);
    }

    #[test]
    fn write_with_a_pen_stamps_it() {
        let mut px = vec![0u8; 4 * 4 * 4];
        let pen = flat(1, 1, [9, 8, 7, 255]);
        assert_eq!(
            write(&mut px, (4, 4), [1.0, 1.0], [0.0; 4], 2, Some(&pen), 0),
            4
        );
        assert_eq!(&px[(4 + 1) * 4..(4 + 1) * 4 + 4], &[9, 8, 7, 255]);
        // the largest pen is held
        let mut big = vec![0u8; 512 * 512 * 4];
        assert_eq!(
            write(
                &mut big,
                (512, 512),
                [200.0, 200.0],
                [1.0; 4],
                i32::MAX,
                None,
                0
            ),
            256 * 256
        );
    }
}
