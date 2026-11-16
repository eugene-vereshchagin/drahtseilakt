// Whether a picture is one of the original game's: the fingerprints that find a shipped level's texture among the
// original's pictures, pixel for pixel (exact) or as the same picture rescaled, recoloured a little, turned or
// mirrored (near). Nothing of the original may ship (CLAUDE.md, "Own content by default"); the check that uses this
// is `crates/gamedata/tests/shipped_originals.rs`, run against the player's copy. The pictures are gathered from
// image files and from the textures saved inside `.nmo` / `.cmo` files, read by our own reader (crates/nmo).

use std::path::{Path, PathBuf};

/// The side of the grey thumbnail the perceptual hash is made of.
const GRID: usize = 32;
/// The side of the colour thumbnail compared beside the hash.
const THUMB: usize = 8;
/// The most bits two perceptual hashes may differ in and still be one picture (of 63).
pub const NEAR_BITS: u32 = 10;
/// The largest mean difference of two colour thumbnails (0..255 per channel) for one picture.
pub const NEAR_COLOUR: f32 = 14.0;
/// Below this spread of brightness (0..255) a picture is flat: a plain colour or a soft gradient, nothing anyone
/// drew; its hash is noise, so it is told apart only by its colours.
pub const FLAT_SPREAD: f32 = 1.5;
/// A picture file read for its fingerprint is at most this large.
const FILE_MAX: u64 = 256 * 1024 * 1024;

/// A picture as RGBA bytes.
#[derive(Clone, Debug)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// One layer of a picture (its colours, or its alpha) as it is known.
#[derive(Clone, Debug)]
pub struct Layer {
    /// FNV-1a over the sides and the layer's bytes.
    pub exact: u64,
    /// The perceptual hash of the layer as it is and turned or mirrored (the eight ways a square can lie).
    pub hashes: [u64; 8],
    /// The small thumbnails in the same eight ways (RGB or alpha, 0..255).
    pub thumbs: [Vec<f32>; 8],
    /// The spread of its brightness (standard deviation on the grey grid, 0..255).
    pub spread: f32,
}

/// What a picture is known by: its colours (alpha left out: the original's BMPs have none, a map's copy of them a
/// full one) and, when its alpha is not one value, its alpha on its own (a shadow or a glow is drawn in its alpha
/// over a plain colour).
#[derive(Clone, Debug)]
pub struct Print {
    pub width: u32,
    pub height: u32,
    pub colour: Layer,
    pub alpha: Option<Layer>,
}

/// How a shipped picture matches one of the original's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Match {
    /// The same sides and the same colours (or the same drawn alpha), pixel for pixel.
    Exact,
    /// The same picture rescaled, recompressed or turned: hash bits that differ, mean thumbnail difference, and
    /// the way the shipped picture lies (`turned`: bit 0 transposed, bit 1 mirrored left to right, bit 2 top to
    /// bottom; the shipped picture so turned is the original).
    Near { bits: u32, colour: f32, way: u8 },
    /// Both are flat (a plain colour or a soft gradient) of about the same colour: nothing drawn, not a copy of
    /// anything anyone could own; reported, not refused.
    Flat { colour: f32 },
}

impl Match {
    /// Whether the match means the picture is the original's (a flat one does not).
    pub fn is_copy(&self) -> bool {
        !matches!(self, Match::Flat { .. })
    }
}

fn fnv(h: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(h, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The mean of each cell when the picture is cut into `n` x `n` cells (area averaging; a picture smaller than the
/// grid gives each cell the pixels under it), RGBA.
fn cells(p: &Picture, n: usize) -> Vec<[f32; 4]> {
    let (w, h) = (p.width as usize, p.height as usize);
    if w == 0 || h == 0 || p.rgba.len() < w.saturating_mul(h).saturating_mul(4) {
        return vec![[0.0; 4]; n.saturating_mul(n)];
    }
    // the pixels of cell k along a side of `len`: from k*len/n up to (k+1)*len/n, at least one
    let span = |k: usize, len: usize| -> (usize, usize) {
        let a = k
            .saturating_mul(len)
            .checked_div(n)
            .unwrap_or(0)
            .min(len.saturating_sub(1));
        let b = k
            .saturating_add(1)
            .saturating_mul(len)
            .checked_div(n)
            .unwrap_or(0);
        (a, b.max(a.saturating_add(1)).min(len))
    };
    let mut out = Vec::with_capacity(n.saturating_mul(n));
    for cy in 0..n {
        let (y0, y1) = span(cy, h);
        for cx in 0..n {
            let (x0, x1) = span(cx, w);
            let mut sum = [0f64; 4];
            let mut count = 0u64;
            for y in y0..y1 {
                let row = y.saturating_mul(w);
                let from = row.saturating_add(x0).saturating_mul(4);
                let to = row.saturating_add(x1).saturating_mul(4);
                for px in p.rgba.get(from..to).unwrap_or_default().chunks_exact(4) {
                    for (acc, &v) in sum.iter_mut().zip(px) {
                        *acc += f64::from(v);
                    }
                    count = count.saturating_add(1);
                }
            }
            let c = count.max(1) as f64;
            out.push(sum.map(|v| (v / c) as f32));
        }
    }
    out
}

/// The cell grid of an `n` x `n` square turned or mirrored: way 0..8 (the bits: transpose, mirror x, mirror y).
fn turned<T: Copy + Default>(grid: &[T], n: usize, way: u8) -> Vec<T> {
    let mut out = vec![T::default(); grid.len()];
    let last = n.saturating_sub(1);
    for y in 0..n {
        for x in 0..n {
            let (mut sx, mut sy) = if way & 1 != 0 { (y, x) } else { (x, y) };
            if way & 2 != 0 {
                sx = last.saturating_sub(sx);
            }
            if way & 4 != 0 {
                sy = last.saturating_sub(sy);
            }
            let from = sy.saturating_mul(n).saturating_add(sx);
            let to = y.saturating_mul(n).saturating_add(x);
            if let (Some(v), Some(slot)) = (grid.get(from).copied(), out.get_mut(to)) {
                *slot = v;
            }
        }
    }
    out
}

/// The perceptual hash of a grey `GRID` x `GRID` grid: the lowest 8 x 8 frequencies of its cosine transform (the
/// constant one left out), each a bit set when above their median.
fn phash(grey: &[f32]) -> u64 {
    let n = GRID;
    let cos: Vec<f32> = (0..8usize)
        .flat_map(|u| {
            (0..n).map(move |x| {
                (std::f32::consts::PI * u as f32 * (2.0 * x as f32 + 1.0) / (2.0 * n as f32)).cos()
            })
        })
        .collect();
    let at = |g: &[f32], i: usize| g.get(i).copied().unwrap_or(0.0);
    let mut freq = [0f32; 64];
    for (k, f) in freq.iter_mut().enumerate() {
        let (u, v) = (k / 8, k % 8);
        let mut s = 0f32;
        for y in 0..n {
            let cy = at(&cos, v.saturating_mul(n).saturating_add(y));
            for x in 0..n {
                let cx = at(&cos, u.saturating_mul(n).saturating_add(x));
                s += at(grey, y.saturating_mul(n).saturating_add(x)) * cx * cy;
            }
        }
        *f = s;
    }
    let mut rest: Vec<f32> = freq.iter().skip(1).copied().collect();
    rest.sort_by(f32::total_cmp);
    let median = rest.get(rest.len() / 2).copied().unwrap_or(0.0);
    freq.iter()
        .enumerate()
        .skip(1)
        .filter(|(_, f)| **f > median)
        .fold(0u64, |h, (k, _)| {
            h | 1u64.checked_shl(k as u32).unwrap_or(0)
        })
}

/// One layer's prints from its grey grid, its thumbnail (cells of `k` values) and its exact hash.
fn layer(exact: u64, grey: &[f32], thumb: &[f32], k: usize) -> Layer {
    let mean = grey.iter().sum::<f32>() / grey.len().max(1) as f32;
    let spread = (grey.iter().map(|g| (g - mean) * (g - mean)).sum::<f32>()
        / grey.len().max(1) as f32)
        .sqrt();
    let cells: Vec<&[f32]> = thumb.chunks_exact(k.max(1)).collect();
    let mut hashes = [0u64; 8];
    let mut thumbs: [Vec<f32>; 8] = Default::default();
    for way in 0..8u8 {
        let w = usize::from(way);
        if let Some(h) = hashes.get_mut(w) {
            *h = phash(&turned(grey, GRID, way));
        }
        if let Some(t) = thumbs.get_mut(w) {
            *t = turned(&cells, THUMB, way).concat();
        }
    }
    Layer {
        exact,
        hashes,
        thumbs,
        spread,
    }
}

/// The fingerprints of a picture.
pub fn print(p: &Picture) -> Print {
    let sides = fnv(
        fnv(0xcbf2_9ce4_8422_2325, &p.width.to_le_bytes()),
        &p.height.to_le_bytes(),
    );
    let (mut exact_rgb, mut exact_a) = (sides, sides);
    let mut alpha_values = (u8::MAX, u8::MIN);
    for px in p.rgba.chunks_exact(4) {
        exact_rgb = fnv(exact_rgb, px.get(..3).unwrap_or_default());
        let a = px.get(3).copied().unwrap_or(u8::MAX);
        exact_a = fnv(exact_a, &[a]);
        alpha_values = (alpha_values.0.min(a), alpha_values.1.max(a));
    }
    let grid = cells(p, GRID);
    let thumb = cells(p, THUMB);
    let grey: Vec<f32> = grid
        .iter()
        .map(|c| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2])
        .collect();
    let rgb: Vec<f32> = thumb.iter().flat_map(|c| [c[0], c[1], c[2]]).collect();
    let colour = layer(exact_rgb, &grey, &rgb, 3);
    // an alpha of one value everywhere draws nothing of its own
    let alpha = (alpha_values.0 != alpha_values.1).then(|| {
        let a: Vec<f32> = grid.iter().map(|c| c[3]).collect();
        let t: Vec<f32> = thumb.iter().map(|c| c[3]).collect();
        layer(exact_a, &a, &t, 1)
    });
    Print {
        width: p.width,
        height: p.height,
        colour,
        alpha,
    }
}

/// How one layer of a shipped picture matches the same layer of an original one.
fn compare_layers(same_sides: bool, shipped: &Layer, original: &Layer) -> Option<Match> {
    let flat = |l: &Layer| l.spread < FLAT_SPREAD;
    if same_sides && shipped.exact == original.exact {
        // the same plain colour pixel for pixel is still nothing anyone drew
        return Some(if flat(shipped) {
            Match::Flat { colour: 0.0 }
        } else {
            Match::Exact
        });
    }
    let diff = |way: usize| -> f32 {
        let (Some(a), Some(b)) = (shipped.thumbs.get(way), original.thumbs.first()) else {
            return f32::MAX;
        };
        if a.len() != b.len() || a.is_empty() {
            return f32::MAX;
        }
        a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>() / a.len() as f32
    };
    if flat(shipped) || flat(original) {
        // a flat layer is only like another flat one, by its colours
        if flat(shipped) && flat(original) {
            let c = diff(0);
            return (c <= NEAR_COLOUR / 2.0).then_some(Match::Flat { colour: c });
        }
        return None;
    }
    let base = original.hashes.first().copied().unwrap_or(0);
    (0..8)
        .filter_map(|way| {
            let bits = (shipped.hashes.get(way).copied().unwrap_or(0) ^ base).count_ones();
            let c = diff(way);
            (bits <= NEAR_BITS && c <= NEAR_COLOUR).then_some(Match::Near {
                bits,
                colour: c,
                way: way as u8,
            })
        })
        .min_by_key(rank)
}

/// The order of matches, the surest first.
fn rank(m: &Match) -> u32 {
    match m {
        Match::Exact => 0,
        Match::Near { bits, .. } => bits.saturating_add(1),
        Match::Flat { .. } => u32::MAX,
    }
}

/// A match and the layer it was found in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub how: Match,
    /// Found in the alpha alone (the colours are not the original's): a shadow or a fade drawn over own colours.
    pub alpha_only: bool,
}

/// Whether a shipped picture is one of the original's, and how: by its colours, or by its drawn alpha (a flat
/// alpha match is no match: a plain colour with a plain alpha is nothing).
pub fn compare(shipped: &Print, original: &Print) -> Option<Match> {
    compare_detail(shipped, original).map(|h| h.how)
}

/// `compare`, telling which layer matched.
pub fn compare_detail(shipped: &Print, original: &Print) -> Option<Hit> {
    let same = shipped.width == original.width && shipped.height == original.height;
    let by_colour = compare_layers(same, &shipped.colour, &original.colour);
    let by_alpha = match (&shipped.alpha, &original.alpha) {
        (Some(a), Some(b)) => compare_layers(same, a, b).filter(Match::is_copy),
        _ => None,
    };
    let colour_copy = by_colour.filter(Match::is_copy);
    match (colour_copy, by_alpha) {
        (Some(c), Some(a)) => Some(Hit {
            how: if rank(&a) < rank(&c) { a } else { c },
            alpha_only: false,
        }),
        (Some(c), None) => Some(Hit {
            how: c,
            alpha_only: false,
        }),
        (None, Some(a)) => Some(Hit {
            how: a,
            alpha_only: true,
        }),
        (None, None) => by_colour.map(|how| Hit {
            how,
            alpha_only: false,
        }),
    }
}

/// A picture found in a folder: where (the file, and inside a `.nmo` the texture object and slot) and its pixels.
#[derive(Clone, Debug)]
pub struct Found {
    pub file: PathBuf,
    /// Inside a Virtools file: the texture object's index, its name and the slot (0 the first).
    pub inside: Option<(usize, String, usize)>,
    pub picture: Picture,
}

impl Found {
    pub fn label(&self) -> String {
        match &self.inside {
            Some((i, name, slot)) => {
                format!("{} [texture {i} {name:?} slot {slot}]", self.file.display())
            }
            None => self.file.display().to_string(),
        }
    }
}

const IMAGE_EXTS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "tga"];
const VIRTOOLS_EXTS: [&str; 3] = ["nmo", "cmo", "vmo"];

fn ext_of(p: &Path) -> Option<String> {
    p.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
}

/// Every picture and Virtools file under `root` (files only, links not followed), sorted.
pub fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let Ok(t) = e.file_type() else { continue };
            let path = e.path();
            if t.is_dir() {
                stack.push(path);
            } else if t.is_file()
                && ext_of(&path).is_some_and(|x| {
                    IMAGE_EXTS.contains(&x.as_str()) || VIRTOOLS_EXTS.contains(&x.as_str())
                })
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// The pictures one file holds: an image file's own, or every picture saved in a Virtools file's textures (each
/// slot) and in the files it carries. A file that cannot be read holds none (the reader refuses a malformed one).
pub fn pictures_in(path: &Path) -> Vec<Found> {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Vec::new();
    };
    if !meta.is_file() || meta.len() > FILE_MAX {
        return Vec::new();
    }
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let ext = ext_of(path);
    if ext.as_deref().is_some_and(|x| IMAGE_EXTS.contains(&x)) {
        return crate::assets::decode_image(&bytes, ext.as_deref())
            .map(|(width, height, rgba)| Found {
                file: path.to_path_buf(),
                inside: None,
                picture: Picture {
                    width,
                    height,
                    rgba,
                },
            })
            .into_iter()
            .collect();
    }
    pictures_in_virtools(path, &bytes)
}

fn pictures_in_virtools(path: &Path, bytes: &[u8]) -> Vec<Found> {
    let mut out = Vec::new();
    let Ok(file) = nmo::read(bytes) else {
        return out;
    };
    for inc in &file.included {
        let name = String::from_utf8_lossy(&inc.name).into_owned();
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !ext.as_deref().is_some_and(|x| IMAGE_EXTS.contains(&x)) {
            continue;
        }
        if let Some((width, height, rgba)) = crate::assets::decode_image(&inc.data, ext.as_deref())
        {
            out.push(Found {
                file: path.to_path_buf(),
                inside: Some((usize::MAX, name, 0)),
                picture: Picture {
                    width,
                    height,
                    rgba,
                },
            });
        }
    }
    let Ok(doc) = nmo::Document::parse(&file) else {
        return out;
    };
    for (i, o) in doc.of_class(nmo::class::TEXTURE) {
        let Some(chunk) = &o.chunk else { continue };
        let Ok(t) = nmo::scene::texture(chunk) else {
            continue;
        };
        let slots = std::iter::once(t.embedded).chain(t.slots.into_iter().map(|s| s.embedded));
        for (slot, embedded) in slots.enumerate() {
            if let Some((width, height, rgba)) = embedded {
                out.push(Found {
                    file: path.to_path_buf(),
                    inside: Some((i, o.name_lossy(), slot)),
                    picture: Picture {
                        width,
                        height,
                        rgba,
                    },
                });
            }
        }
    }
    out
}

/// The fingerprints of every picture under the roots, read on all cores (decoding the shipped 4K pictures one
/// by one would take minutes). Each picture's pixels are dropped once its prints are made.
pub fn prints_under(roots: &[PathBuf]) -> Vec<(String, Print)> {
    let files: Vec<PathBuf> = roots.iter().flat_map(|r| files_under(r)).collect();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 16);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut out: Vec<(usize, String, Print)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                s.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(f) = files.get(k) else { break };
                        for found in pictures_in(f) {
                            mine.push((k, found.label(), print(&found.picture)));
                        }
                    }
                    mine
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    out.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    out.into_iter().map(|(_, l, p)| (l, p)).collect()
}

/// Every shipped picture that is one of the original's: (shipped, original, how), the best match of each.
pub fn matches(
    shipped: &[(String, Print)],
    originals: &[(String, Print)],
) -> Vec<(String, String, Hit)> {
    let mut out = Vec::new();
    for (label, p) in shipped {
        let best = originals
            .iter()
            .filter_map(|(ol, op)| compare_detail(p, op).map(|h| (ol, h)))
            .min_by_key(|(_, h)| rank(&h.how));
        if let Some((ol, m)) = best {
            out.push((label.clone(), ol.clone(), m));
        }
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    /// A picture with something drawn in it (rings and stripes), seeded.
    fn drawn(w: u32, h: u32, seed: u32) -> Picture {
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
                let s = seed as f32;
                let r = ((fx * 7.0 + s).sin() * 0.5 + 0.5) * 255.0;
                let g = (((fx - 0.3) * (fx - 0.3) + (fy - 0.6) * (fy - 0.6)).sqrt() * (12.0 + s))
                    .cos()
                    * 127.0
                    + 128.0;
                let b = if ((fy * (5.0 + s)) as u32).is_multiple_of(2) {
                    40.0
                } else {
                    220.0
                };
                rgba.extend([r as u8, g as u8, b as u8, 255]);
            }
        }
        Picture {
            width: w,
            height: h,
            rgba,
        }
    }

    fn scaled(p: &Picture, w: u32, h: u32) -> Picture {
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let sx = x * p.width / w;
                let sy = y * p.height / h;
                let i = ((sy * p.width + sx) * 4) as usize;
                rgba.extend_from_slice(&p.rgba[i..i + 4]);
            }
        }
        Picture {
            width: w,
            height: h,
            rgba,
        }
    }

    #[test]
    fn the_same_pixels_are_exact_whatever_the_alpha() {
        let a = drawn(64, 64, 1);
        let mut b = a.clone();
        b.rgba.chunks_exact_mut(4).for_each(|px| px[3] = 7);
        assert_eq!(compare(&print(&b), &print(&a)), Some(Match::Exact));
    }

    #[test]
    fn a_rescaled_turned_or_slightly_recoloured_copy_is_near() {
        let a = drawn(128, 128, 2);
        let big = scaled(&a, 512, 512);
        assert!(matches!(
            compare(&print(&big), &print(&a)),
            Some(Match::Near { .. })
        ));
        // mirrored left to right
        let mut m = a.clone();
        for y in 0..128usize {
            for x in 0..128usize {
                let (i, j) = ((y * 128 + x) * 4, (y * 128 + 127 - x) * 4);
                m.rgba[i..i + 4].copy_from_slice(&a.rgba[j..j + 4]);
            }
        }
        assert!(matches!(
            compare(&print(&m), &print(&a)),
            Some(Match::Near { .. })
        ));
        let mut tinted = a.clone();
        tinted
            .rgba
            .chunks_exact_mut(4)
            .for_each(|px| px[0] = px[0].saturating_add(8));
        assert!(matches!(
            compare(&print(&tinted), &print(&a)),
            Some(Match::Near { .. })
        ));
    }

    #[test]
    fn different_pictures_do_not_match() {
        for (s, o) in [(3, 4), (5, 9), (1, 6)] {
            let (a, b) = (drawn(64, 64, s), drawn(64, 64, o));
            assert_eq!(compare(&print(&a), &print(&b)), None, "{s} {o}");
        }
        // a flat picture is no copy of a drawn one, and two flat ones of one colour are only flat
        let flat = Picture {
            width: 16,
            height: 16,
            rgba: [90u8, 90, 90, 255].repeat(256),
        };
        assert_eq!(compare(&print(&flat), &print(&drawn(16, 16, 1))), None);
        let mut other = flat.clone();
        other.rgba[0] = 91;
        assert!(!compare(&print(&other), &print(&flat)).unwrap().is_copy());
    }

    #[test]
    fn a_shadow_drawn_in_its_alpha_matches_by_its_alpha() {
        // black everywhere, the picture in the alpha
        let shade = |p: &Picture| Picture {
            width: p.width,
            height: p.height,
            rgba: p
                .rgba
                .chunks_exact(4)
                .flat_map(|px| [0, 0, 0, px[1]])
                .collect(),
        };
        let a = shade(&drawn(64, 64, 2));
        assert_eq!(compare(&print(&a), &print(&a)), Some(Match::Exact));
        let big = shade(&scaled(&drawn(64, 64, 2), 256, 256));
        assert!(matches!(
            compare(&print(&big), &print(&a)),
            Some(Match::Near { .. })
        ));
        // another shadow on the same black is only a flat look-alike
        let mut other = drawn(64, 64, 2);
        other.rgba.chunks_exact_mut(4).for_each(|px| px[1] = px[0]);
        let other = shade(&other);
        assert!(!compare(&print(&other), &print(&a)).unwrap().is_copy());
    }

    #[test]
    fn odd_sides_and_empty_pictures_do_not_panic() {
        for (w, h) in [(1, 1), (1, 300), (300, 1), (33, 7)] {
            let p = drawn(w, h, 1);
            let _ = compare(&print(&p), &print(&drawn(32, 32, 1)));
        }
        let empty = Picture {
            width: 0,
            height: 0,
            rgba: Vec::new(),
        };
        let _ = print(&empty);
        let short = Picture {
            width: 10,
            height: 10,
            rgba: vec![1, 2, 3],
        };
        let _ = print(&short);
    }
}
