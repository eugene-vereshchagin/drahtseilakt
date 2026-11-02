// Remastered levels (docs/drahtseilakt/levels.md, roadmap phase 7): the original levels converted to the open format
// in levels/remastered, with their textures made larger on the player's machine. A texture is upscaled by Real-ESRGAN
// when the player has installed it (`realesrgan-ncnn-vulkan` on the PATH: it adds detail), else by Lanczos with a
// light sharpening (larger and sharper, no new detail). The originals are kept beside (textures.original/), so a
// second run starts from them again. A colour-keyed texture gets its key as alpha first (its edges are blended when
// it grows, so the exact key colour would no longer match); the key stays in the level file, which keeps the alpha
// test on and lets the next run key the original again. The folders are the player's and may hold anything: links
// are never followed or written through, every file is written beside and moved into place, and a texture that
// cannot be read is left as it is and reported.

use crate::assets::{Data, Source};
use image::{RgbaImage, imageops};
use std::path::{Path, PathBuf};

/// The largest side a texture is made (GPU textures of 8K are common; the originals are 128 to 256).
const SIDE_MAX: u32 = 8192;
/// The largest level file and texture read.
const GLTF_MAX: u64 = 64 * 1024 * 1024;
const IMAGE_MAX: u64 = 64 * 1024 * 1024;

/// Converts every original level the copy has that is not in levels/remastered yet, then upscales the textures of
/// every level there by `factor`. Returns a line a level, and one for each texture left as it was.
pub fn run(data: &Data, factor: u32) -> Result<Vec<String>, String> {
    let root = crate::levels::dir(crate::levels::Kind::Remastered);
    let mut out = Vec::new();
    for n in 1..=crate::menu::LEVELS {
        let dir = root.join(format!("Level_{n:02}"));
        if std::fs::symlink_metadata(dir.join("level.gltf")).is_ok() {
            continue;
        }
        let Ok(loaded) = data.load(&Source::Original(n)) else {
            continue;
        };
        crate::assets::export(data, &loaded, &dir)?;
    }
    let mut levels: Vec<PathBuf> = std::fs::read_dir(&root)
        .map_err(|e| format!("{}: {e}", root.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| real_dir(p) && real_file(&p.join("level.gltf")))
        .collect();
    levels.sort();
    let esrgan = esrgan();
    for dir in levels {
        match level(&dir, factor, esrgan.as_deref(), &mut out) {
            Ok(n) => out.push(format!("{}: {n} textures x{factor}", dir.display())),
            Err(e) => out.push(format!("{}: not remastered: {e}", dir.display())),
        }
    }
    Ok(out)
}

/// A directory itself, not a link to one.
fn real_dir(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir())
}

/// A regular file itself, not a link to one.
fn real_file(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file())
}

/// Nothing there, or a regular file: a place a file may be written to (never a link, never a directory).
fn writable(p: &Path) -> bool {
    match std::fs::symlink_metadata(p) {
        Ok(m) => m.is_file(),
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    }
}

/// Writes `path` by writing its neighbour and moving it over (a link there is replaced, not written through; an
/// interruption leaves the old file).
fn replace(path: &Path, write: impl FnOnce(&Path) -> Result<(), String>) -> Result<(), String> {
    if !writable(path) {
        return Err(format!("{}: not a plain file", path.display()));
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".new");
    let tmp = PathBuf::from(tmp);
    if !writable(&tmp) {
        return Err(format!("{}: not a plain file", tmp.display()));
    }
    write(&tmp)?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A texture's file name as the level file gives it: a plain name under textures/ (no separator, drive colon, leading
/// dot or Windows device name).
fn texture_name(uri: &str) -> Option<&str> {
    let n = uri.strip_prefix("textures/")?;
    let stem = n.split('.').next().unwrap_or_default().to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.len() == 4);
    let plain = !n.is_empty()
        && !n.starts_with('.')
        && !n
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':') || c.is_control());
    (plain && !device).then_some(n)
}

/// Real-ESRGAN's program, if the player has it on the PATH.
fn esrgan() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "realesrgan-ncnn-vulkan.exe"
    } else {
        "realesrgan-ncnn-vulkan"
    };
    std::env::var_os("PATH").and_then(|p| {
        std::env::split_paths(&p)
            .map(|d| d.join(name))
            .find(|f| f.is_file())
    })
}

/// Upscales one level's textures; returns how many. A texture that cannot be read or written is reported in `notes`
/// and left as it is.
fn level(
    dir: &Path,
    factor: u32,
    esrgan: Option<&Path>,
    notes: &mut Vec<String>,
) -> Result<usize, String> {
    let gltf = dir.join("level.gltf");
    let small = real_file(&gltf) && std::fs::metadata(&gltf).is_ok_and(|m| m.len() <= GLTF_MAX);
    if !small {
        return Err("level.gltf missing, a link, or too big".into());
    }
    let text = std::fs::read_to_string(&gltf).map_err(|e| e.to_string())?;
    let doc: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let (textures, originals) = (dir.join("textures"), dir.join("textures.original"));
    if !real_dir(&textures) {
        return Err("textures/ missing or a link".into());
    }
    match std::fs::symlink_metadata(&originals) {
        Ok(m) if !m.is_dir() => return Err("textures.original is not a folder".into()),
        Ok(_) => {}
        Err(_) => {
            std::fs::create_dir(&originals).map_err(|e| format!("{}: {e}", originals.display()))?
        }
    }
    let tmp = dir.join(".remaster-tmp");
    let mut done = 0;
    for img in doc
        .get("images")
        .and_then(|i| i.as_array())
        .into_iter()
        .flatten()
    {
        let Some(name) = img
            .get("uri")
            .and_then(|u| u.as_str())
            .and_then(texture_name)
        else {
            continue;
        };
        let (path, original) = (textures.join(name), originals.join(name));
        let key = img
            .pointer("/extras/drahtseilakt/color_key")
            .and_then(key_of);
        let one = || -> Result<(), String> {
            // the original kept once, written beside and moved in, so an interruption never leaves half of it
            if !real_file(&original) {
                if !real_file(&path) {
                    return Err("missing or a link".into());
                }
                replace(&original, |t| {
                    std::fs::copy(&path, t).map(drop).map_err(|e| e.to_string())
                })?;
            }
            let source = load(&original)?;
            let source = match key {
                Some(k) => keyed(source, k),
                None => source,
            };
            let big = match esrgan {
                Some(prog) => esrgan_upscale(prog, &source, factor, &tmp)
                    .unwrap_or_else(|_| lanczos(&source, factor)),
                None => lanczos(&source, factor),
            };
            // the file keeps its own format (what the level file names)
            let format = image::ImageFormat::from_path(&path).map_err(|e| e.to_string())?;
            replace(&path, |t| {
                let img = image::DynamicImage::ImageRgba8(big);
                let img = if format == image::ImageFormat::Jpeg {
                    image::DynamicImage::ImageRgb8(img.to_rgb8())
                } else {
                    img
                };
                img.save_with_format(t, format).map_err(|e| e.to_string())
            })
        };
        match one() {
            Ok(()) => done += 1,
            Err(e) => notes.push(format!("{}: left as it was: {e}", path.display())),
        }
    }
    if real_dir(&tmp) {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    Ok(done)
}

fn key_of(v: &serde_json::Value) -> Option<[u8; 3]> {
    let a = v.as_array()?;
    let c = |i: usize| {
        a.get(i)
            .and_then(|x| x.as_u64())
            .and_then(|x| u8::try_from(x).ok())
    };
    Some([c(0)?, c(1)?, c(2)?])
}

/// A texture, its size checked before it is decoded.
fn load(path: &Path) -> Result<RgbaImage, String> {
    let small = real_file(path) && std::fs::metadata(path).is_ok_and(|m| m.len() <= IMAGE_MAX);
    if !small {
        return Err(format!("{}: missing, a link or too big", path.display()));
    }
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(SIDE_MAX);
    limits.max_image_height = Some(SIDE_MAX);
    reader.limits(limits);
    Ok(reader
        .decode()
        .map_err(|e| format!("{}: {e}", path.display()))?
        .to_rgba8())
}

/// The key colour made transparent, and the transparent texels given their opaque neighbours' colour, so that the
/// colour of the key never bleeds into the edges when the texture grows.
fn keyed(mut img: RgbaImage, key: [u8; 3]) -> RgbaImage {
    for p in img.pixels_mut() {
        if p.0[..3] == key {
            p.0[3] = 0;
        }
    }
    let (w, h) = img.dimensions();
    for _ in 0..8 {
        let src = img.clone();
        let mut changed = false;
        for y in 0..h {
            for x in 0..w {
                if src.get_pixel(x, y).0[3] != 0 {
                    continue;
                }
                let (mut sum, mut n) = ([0u32; 3], 0u32);
                for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                    if nx < 0 || ny < 0 || nx >= i64::from(w) || ny >= i64::from(h) {
                        continue;
                    }
                    let q = src.get_pixel(nx as u32, ny as u32).0;
                    // an opaque texel, or one filled in an earlier pass
                    if q[3] != 0 || q[..3] != key {
                        for c in 0..3 {
                            sum[c] += u32::from(q[c]);
                        }
                        n += 1;
                    }
                }
                if n > 0 {
                    let p = img.get_pixel_mut(x, y);
                    for c in 0..3 {
                        p.0[c] = (sum[c] / n) as u8;
                    }
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    img
}

/// The target size: `factor` times, the longer side at most SIDE_MAX.
fn target(w: u32, h: u32, factor: u32) -> (u32, u32) {
    let f = factor.clamp(1, 32).min(SIDE_MAX / w.max(h).max(1)).max(1);
    (w * f, h * f)
}

/// Lanczos resampling and a light unsharp mask.
fn lanczos(img: &RgbaImage, factor: u32) -> RgbaImage {
    let (w, h) = target(img.width(), img.height(), factor);
    let big = imageops::resize(img, w, h, imageops::FilterType::Lanczos3);
    imageops::unsharpen(&big, 0.6 * (w as f32 / img.width().max(1) as f32).sqrt(), 2)
}

/// Real-ESRGAN by 4 at a time (its model's scale), then Lanczos to the exact size; its files in `tmp`, a folder of
/// the level's own.
fn esrgan_upscale(
    prog: &Path,
    img: &RgbaImage,
    factor: u32,
    tmp: &Path,
) -> Result<RgbaImage, String> {
    let (w, h) = target(img.width(), img.height(), factor);
    match std::fs::symlink_metadata(tmp) {
        Ok(m) if !m.is_dir() => {
            return Err("the remaster's temporary folder is not a folder".into());
        }
        Ok(_) => {}
        Err(_) => std::fs::create_dir(tmp).map_err(|e| e.to_string())?,
    }
    let (input, output) = (tmp.join("in.png"), tmp.join("out.png"));
    let mut cur = img.clone();
    while (cur.width() < w || cur.height() < h) && cur.width().max(cur.height()) * 4 <= SIDE_MAX {
        replace(&input, |t| {
            cur.save_with_format(t, image::ImageFormat::Png)
                .map_err(|e| e.to_string())
        })?;
        if !writable(&output) {
            return Err("Real-ESRGAN's output is not a plain file".into());
        }
        let ok = std::process::Command::new(prog)
            .arg("-i")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .args(["-s", "4", "-n", "realesrgan-x4plus"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            return Err("Real-ESRGAN failed".into());
        }
        cur = load(&output)?;
    }
    Ok(if cur.dimensions() == (w, h) {
        cur
    } else {
        imageops::resize(&cur, w, h, imageops::FilterType::Lanczos3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_becomes_alpha_and_its_colour_does_not_bleed() {
        let mut img = RgbaImage::new(4, 1);
        for (x, c) in [[255, 0, 255], [10, 20, 30], [10, 20, 30], [255, 0, 255]]
            .into_iter()
            .enumerate()
        {
            img.put_pixel(x as u32, 0, image::Rgba([c[0], c[1], c[2], 255]));
        }
        let k = keyed(img, [255, 0, 255]);
        assert_eq!(k.get_pixel(0, 0).0, [10, 20, 30, 0]);
        assert_eq!(k.get_pixel(1, 0).0, [10, 20, 30, 255]);
    }

    #[test]
    fn texture_names_are_plain() {
        assert_eq!(
            texture_name("textures/0001_Floor.png"),
            Some("0001_Floor.png")
        );
        for bad in [
            "textures/../x.png",
            "textures/a/b.png",
            "x.png",
            "textures/.hidden",
            "textures/CON.png",
            "textures/c:x",
        ] {
            assert_eq!(texture_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_size_grows_by_the_factor_up_to_8k() {
        assert_eq!(target(128, 64, 8), (1024, 512));
        assert_eq!(target(4096, 4096, 8), (8192, 8192));
        assert_eq!(target(8192, 8192, 8), (8192, 8192));
    }
}
