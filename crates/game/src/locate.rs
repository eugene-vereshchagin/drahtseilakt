// Finds where the ball is in each frame of a video of the original game (a walkthrough), for the test player to
// follow: for every frame, the level is drawn from candidate places of the ball with the in-game camera behind it
// (22 back, 35 up, turned in quarters), and the candidates whose pictures match the frame best (their edges and
// their coarse grey levels) are taken. The search starts at the level start and keeps several likely ways at
// once, each going on near where it was heading; the view's quarter is searched too (the player turns it), so the
// turns are found along with the way. For testing only; it reads the player's own frames and never goes online.

use crate::render::{Renderer, View};
use glam::Vec3;

/// Frames a second, of the video and of `--frames`: often enough that the ball goes only a few units between two
/// (a long straight looks much the same from anywhere along it, so only nearness tells its places apart).
pub const RATE: u32 = 4;
/// The size the frames and the candidates are compared at (4:3, as the original's view).
const W: u32 = 128;
const H: u32 = 96;
/// Candidates: floor points within this of the last place, at least this far apart.
const REACH: f32 = 12.0;
const GRID: f32 = 3.0;
/// What a place's distance from where the ball was heading and a turn of the view cost in likeness: the ball
/// rarely changes its speed much between two frames, and the view turns seldom.
const MOVE_COST: f32 = 0.004;
/// Below this likeness the track is taken as lost and searched for within `WIDE`.
const LOST: f32 = 0.35;
const WIDE: f32 = 150.0;
const TURN_COST: f32 = 0.03;
/// The coarse grey levels' share of the likeness (the rest is the edges'): the edges place the view finely, the
/// grey levels tell the level's parts apart.
const GREY: f32 = 0.3;
/// How far above or below its last place a ball's next place may be (it drops off steps).
const DROP: f32 = 30.0;
/// The largest frame read (either side).
const MAX_FRAME: u32 = 4096;
/// A track's first line (its columns).
pub const TRACK_HEAD: &str = "# frame x y z quarter likeness";
/// A fix this alike is taken as sure (for following a track), and one this far from the last sure one as a slip.
pub const SURE: f32 = 0.5;
pub const JUMP: f32 = 30.0;
/// The ways kept at each frame.
const BEAM: usize = 6;

/// A frame's features, without the HUD at the bottom and the corner watermark at the top right: the gradient
/// magnitude of its grey levels (its edges) and its grey levels in blocks of 4 × 4, each part normalised to mean 0
/// and length 1 and weighted so that the whole has length 1.
pub fn edges(rgba: &[u8], w: u32, h: u32) -> Vec<f32> {
    let grey = |x: u32, y: u32| -> f32 {
        let i = ((y * w + x) * 4) as usize;
        rgba.get(i..i + 3).map_or(0.0, |p| {
            0.3 * f32::from(p[0]) + 0.59 * f32::from(p[1]) + 0.11 * f32::from(p[2])
        })
    };
    let mut out = vec![0.0f32; (w * h) as usize];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            // the HUD and the tutorial's text panels (the bottom third) and the watermark (top right) are not the level
            if y > h * 68 / 100 || (y < h / 8 && x > w * 7 / 8) {
                continue;
            }
            let gx = grey(x + 1, y) - grey(x - 1, y);
            let gy = grey(x, y + 1) - grey(x, y - 1);
            out[(y * w + x) as usize] = (gx * gx + gy * gy).sqrt();
        }
    }
    normalise(&mut out);
    // the coarse grey levels too (where sky, floor and void are), in blocks of 4 × 4
    let (bw, bh) = (w / 4, h / 4);
    let mut coarse = vec![0.0f32; (bw * bh) as usize];
    for by in 0..bh {
        for bx in 0..bw {
            if by * 4 > h * 68 / 100 || (by * 4 < h / 8 && bx * 4 > w * 7 / 8) {
                continue;
            }
            let mut sum = 0.0;
            for y in 0..4 {
                for x in 0..4 {
                    sum += grey(bx * 4 + x, by * 4 + y);
                }
            }
            coarse[(by * bw + bx) as usize] = sum;
        }
    }
    normalise(&mut coarse);
    for v in &mut out {
        *v *= (1.0 - GREY).sqrt();
    }
    out.extend(coarse.iter().map(|v| v * GREY.sqrt()));
    out
}

/// Mean 0 and length 1.
fn normalise(v: &mut [f32]) {
    let n = v.len().max(1) as f32;
    let mean = v.iter().sum::<f32>() / n;
    for x in v.iter_mut() {
        *x -= mean;
    }
    let len = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for x in v.iter_mut() {
        *x /= len;
    }
}

/// How alike two edge maps are (their correlation, -1..1).
pub fn likeness(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// The camera the original puts behind the ball for view quarter `q` (Sim's camera at rest).
pub fn camera(ball: Vec3, q: i32) -> (Vec3, Vec3) {
    camera_at(ball, q as f32 * std::f32::consts::FRAC_PI_2)
}

/// The camera behind the ball looking along `yaw` (radians; quarter q is q·π/2).
pub fn camera_at(ball: Vec3, yaw: f32) -> (Vec3, Vec3) {
    let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    (ball - forward * 22.0 + Vec3::Y * 35.0, ball)
}

/// One frame's answer: where the ball is, the view's quarter, and how alike the best picture was.
#[derive(Clone, Copy, Debug)]
pub struct Fix {
    pub ball: Vec3,
    pub quarter: i32,
    pub score: f32,
}

/// The candidate places near `last` (ball centres over floor points), thinned to `GRID`: within `reach` across,
/// and below it as far as a ball falls in a frame or two (it drops off steps and onto lower floors).
pub fn candidates(floor: &[Vec3], last: Vec3, reach: f32) -> Vec<Vec3> {
    let mut out: Vec<Vec3> = Vec::new();
    for &f in floor {
        let c = f + Vec3::Y * crate::sim::BALL_RADIUS;
        let across = Vec3::new(c.x - last.x, 0.0, c.z - last.z).length();
        if across <= reach
            && (c.y - last.y).abs() <= reach.max(DROP)
            && out.iter().all(|o| o.distance(c) >= GRID)
        {
            out.push(c);
        }
    }
    out
}

/// Draws the level from `view` (the ball at the given place) at W × H and returns its features.
pub type Draw<'a> = dyn FnMut(&View, Vec3) -> Option<Vec<f32>> + 'a;

/// One way the ball may have gone, as far as the frames so far: where it is, its last step, how alike all its
/// pictures were less what its moves cost, and the way it came from.
#[derive(Clone, Copy)]
struct Way {
    fix: Fix,
    step: Vec3,
    total: f32,
    /// The way it came from (its index in the last frame's kept ways).
    from: Option<usize>,
}

/// Follows the ball through `frames` (edge maps, `RATE` a second), from `start` with view quarter `quarter`. It
/// keeps the `BEAM` likeliest ways at every frame, not only the best: a place that looks best in one frame (a long
/// straight looks the same along it) is often not on the way that fits the frames after it.
pub fn track(
    frames: &[Vec<f32>],
    floor: &[Vec3],
    start: Vec3,
    quarter: i32,
    draw: &mut Draw<'_>,
) -> Result<Vec<Fix>, String> {
    // a picture that could not be drawn stops the search (every place would look alike)
    let failed = std::cell::Cell::new(false);
    let rate = RATE as f32;
    let mut ways = vec![Way {
        fix: Fix {
            ball: start,
            quarter,
            score: 0.0,
        },
        step: Vec3::ZERO,
        total: 0.0,
        from: None,
    }];
    // every frame's kept ways, to go back along the best at the end
    let mut kept: Vec<Vec<Way>> = Vec::new();
    for frame in frames {
        let mut next: Vec<Way> = Vec::new();
        let look =
            |w: &Way, from: usize, p: Vec3, q: i32, next: &mut Vec<Way>, draw: &mut Draw<'_>| {
                let (eye, _) = camera(p, q);
                // the in-game camera trails a moving ball: its eye by speed/5, its aim by speed/12 (Sim's follow rates)
                let lag = w.step * rate;
                let view = View {
                    eye: eye - lag / 5.0,
                    target: p - lag / 12.0,
                    fov_y: crate::scene::fov_y(),
                };
                let Some(picture) = draw(&view, p) else {
                    failed.set(true);
                    return;
                };
                let s = likeness(frame, &picture);
                let cost = MOVE_COST * p.distance(w.fix.ball + w.step)
                    + if q != w.fix.quarter { TURN_COST } else { 0.0 };
                let step = if s >= LOST {
                    p - w.fix.ball
                } else {
                    Vec3::ZERO
                };
                let way = Way {
                    fix: Fix {
                        ball: p,
                        quarter: q,
                        score: s,
                    },
                    step,
                    total: w.total + s - cost,
                    from: Some(from),
                };
                // one way for each place and quarter: the likeliest that reaches it
                match next
                    .iter_mut()
                    .find(|o| o.fix.quarter == q && o.fix.ball.distance(p) < GRID / 2.0)
                {
                    Some(o) if o.total >= way.total => {}
                    Some(o) => *o = way,
                    None => next.push(way),
                }
            };
        for (i, w) in ways.iter().enumerate() {
            let ahead = w.fix.ball + w.step;
            let mut places = candidates(floor, ahead, REACH);
            for p in candidates(floor, w.fix.ball, REACH / 2.0) {
                if places.iter().all(|o| o.distance(p) >= GRID) {
                    places.push(p);
                }
            }
            for p in places {
                for q in [w.fix.quarter, w.fix.quarter - 1, w.fix.quarter + 1] {
                    look(w, i, p, q, &mut next, draw);
                }
            }
        }
        // lost (a poor match everywhere): look over a wide area round the likeliest way, coarsely
        let best = |n: &[Way]| n.iter().copied().max_by(|a, b| a.total.total_cmp(&b.total));
        let likeliest = ways
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total.total_cmp(&b.1.total));
        if best(&next).is_none_or(|b| b.fix.score < LOST)
            && let Some((i, w)) = likeliest
        {
            for p in candidates(floor, w.fix.ball, WIDE) {
                if p.distance(w.fix.ball) >= REACH {
                    look(w, i, p, w.fix.quarter, &mut next, draw);
                }
            }
        }
        next.sort_by(|a, b| b.total.total_cmp(&a.total));
        next.truncate(BEAM);
        // then finer, round each kept place
        let coarse = next.clone();
        for c in &coarse {
            let Some(from) = c.from else { continue };
            let Some(w) = ways.get(from).copied() else {
                continue;
            };
            for dx in [-1.0f32, 0.0, 1.0] {
                for dz in [-1.0f32, 0.0, 1.0] {
                    if dx != 0.0 || dz != 0.0 {
                        look(
                            &w,
                            from,
                            c.fix.ball + Vec3::new(dx, 0.0, dz) * (GRID / 3.0),
                            c.fix.quarter,
                            &mut next,
                            draw,
                        );
                    }
                }
            }
        }
        next.sort_by(|a, b| b.total.total_cmp(&a.total));
        next.truncate(BEAM);
        if failed.get() {
            return Err("a picture of the level could not be read back from the GPU".into());
        }
        if next.is_empty() {
            break;
        }
        kept.push(next.clone());
        ways = next;
    }
    // back along the likeliest way
    let mut out = Vec::with_capacity(kept.len());
    let mut at = Some(0);
    for frame in kept.iter().rev() {
        let Some(w) = at.and_then(|i| frame.get(i)) else {
            break;
        };
        out.push(w.fix);
        at = w.from;
    }
    out.reverse();
    Ok(out)
}

/// Draws candidates with `r` at W × H and reads them back as edges.
pub struct Offscreen {
    target: wgpu::Texture,
    view: wgpu::TextureView,
    buf: wgpu::Buffer,
    row: u32,
    w: u32,
    h: u32,
}

impl Offscreen {
    pub fn new(r: &Renderer) -> Offscreen {
        Offscreen::sized(r, W, H)
    }

    /// An offscreen picture of `w` × `h`.
    pub fn sized(r: &Renderer, w: u32, h: u32) -> Offscreen {
        let target = r.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("locate"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: r.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let row = (4 * w).div_ceil(256) * 256;
        let buf = r.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("locate readback"),
            size: u64::from(row) * u64::from(h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Offscreen {
            target,
            view,
            buf,
            row,
            w,
            h,
        }
    }

    /// The picture of `inst` from `view`, as features (none if it could not be read back).
    pub fn edges(
        &self,
        r: &mut Renderer,
        view: &View,
        inst: &[crate::render::Instance],
    ) -> Option<Vec<f32>> {
        let px = self.pixels(r, view, inst, &[])?;
        Some(edges(&px, self.w, self.h))
    }

    /// The picture of `inst` and `quads` (2D, over it) from `view`, as RGBA bytes (none if it could not be read
    /// back from the GPU).
    pub fn pixels(
        &self,
        r: &mut Renderer,
        view: &View,
        inst: &[crate::render::Instance],
        quads: &[crate::ui::Quad],
    ) -> Option<Vec<u8>> {
        let (w, h) = (self.w, self.h);
        r.draw(&self.view, w, h, view, inst, &[]);
        r.draw_ui(&self.view, w, h, quads);
        let mut enc = r.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        r.queue.submit([enc.finish()]);
        let slice = self.buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let polled = r.device.poll(wgpu::PollType::wait_indefinitely()).is_ok();
        let mut pixels = Vec::with_capacity((4 * w * h) as usize);
        let mut whole = false;
        if polled && let Ok(mapped) = slice.get_mapped_range() {
            whole = (0..h).all(|y| {
                let start = (y * self.row) as usize;
                mapped
                    .get(start..start + (4 * w) as usize)
                    .map(|row| pixels.extend_from_slice(row))
                    .is_some()
            });
        }
        self.buf.unmap();
        whole.then_some(pixels)
    }
}

/// A frame file (JPEG) as edges at W × H; a wide frame is cut to 4:3 in its middle first.
pub fn frame_edges(path: &std::path::Path) -> Option<Vec<f32>> {
    // a video's frame, not a huge picture (its copies below are not held to the decoder's limits)
    let (fw, fh) = image::image_dimensions(path).ok()?;
    if fw > MAX_FRAME || fh > MAX_FRAME {
        return None;
    }
    let img = image::open(path).ok()?.to_rgba8();
    let (fw, fh) = img.dimensions();
    let cw = (fh.saturating_mul(4) / 3).min(fw);
    let x0 = (fw - cw) / 2;
    let cut = image::imageops::crop_imm(&img, x0, 0, cw, fh).to_image();
    let small = image::imageops::resize(&cut, W, H, image::imageops::FilterType::Triangle);
    Some(edges(small.as_raw(), W, H))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_most_like_itself() {
        let mut a = vec![0u8; (W * H * 4) as usize];
        for (i, px) in a.chunks_exact_mut(4).enumerate() {
            let v = if (i as u32 % W) / 8 % 2 == 0 { 200 } else { 30 };
            px.copy_from_slice(&[v, v, v, 255]);
        }
        let ea = edges(&a, W, H);
        assert!((likeness(&ea, &ea) - 1.0).abs() < 1e-3);
        let mut b = a.clone();
        b.rotate_left(4 * 3);
        assert!(likeness(&ea, &edges(&b, W, H)) < 0.99);
    }
}
