// own-pictures: finds the pictures of the original game that shipped levels carry and puts our own in their place.
// Each picture of a level (a `.nmo` map's saved textures, an open-format level's `textures.original/` files) is
// compared with every picture of the player's copy (gamedata::provenance); one that is the original's, pixel for
// pixel or rescaled, turned or recoloured a little, is replaced by our own picture of the same name (assets/own,
// made by assets/own/make), turned the same way and resized to its sides. A picture found only by its alpha (a fade
// or a shadow over the author's own colours) keeps its colours and gets our alpha. A map is patched in place with
// nmo::patch (only those pixels change); an open-format level's `textures.original/` file is rewritten, and the
// toolbox's `remaster-open` then makes its enlarged picture again. Without --write it only reports.
//
//   own-pictures --copy DIR [--copy DIR]... --own DIR [--write] PATH...
//
// PATH is a map file or a level folder. Exits non-zero when a picture of the original stays (no own picture of
// its name, or the result still matches).

use gamedata::provenance::{self, Hit, Match, Picture, Print};
use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

struct Options {
    copies: Vec<PathBuf>,
    own: Option<PathBuf>,
    write: bool,
    paths: Vec<PathBuf>,
}

fn options() -> Result<Options, String> {
    let mut o = Options {
        copies: Vec::new(),
        own: None,
        write: false,
        paths: Vec::new(),
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("--copy") => o
                .copies
                .push(args.next().ok_or("--copy needs a folder")?.into()),
            Some("--own") => o.own = Some(args.next().ok_or("--own needs a folder")?.into()),
            Some("--write") => o.write = true,
            Some(s) if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ => o.paths.push(a.into()),
        }
    }
    if o.copies.is_empty() || o.own.is_none() || o.paths.is_empty() {
        return Err(
            "usage: own-pictures --copy DIR [--copy DIR]... --own DIR [--write] PATH...".into(),
        );
    }
    Ok(o)
}

/// The picture turned one of the eight ways, as `provenance` turns its grids: out(x, y) = in(sx, sy), where (sx, sy)
/// is (y, x) when bit 0 is set, else (x, y), then mirrored on the source's axes, x by bit 1, y by bit 2.
fn turned(img: &RgbaImage, way: u8) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (ow, oh) = if way & 1 != 0 { (h, w) } else { (w, h) };
    RgbaImage::from_fn(ow, oh, |x, y| {
        let (mut sx, mut sy) = if way & 1 != 0 { (y, x) } else { (x, y) };
        if way & 2 != 0 {
            sx = w.saturating_sub(1).saturating_sub(sx);
        }
        if way & 4 != 0 {
            sy = h.saturating_sub(1).saturating_sub(sy);
        }
        *img.get_pixel(sx.min(w.saturating_sub(1)), sy.min(h.saturating_sub(1)))
    })
}

/// The way that undoes `way` (found by trying: eight ways, one small picture).
fn undo(way: u8) -> u8 {
    let probe = RgbaImage::from_fn(3, 2, |x, y| image::Rgba([x as u8, y as u8, 0, 255]));
    let there = turned(&probe, way);
    (0..8u8).find(|&w| turned(&there, w) == probe).unwrap_or(0)
}

/// Our own pictures by name (lower case, no extension): the first found under `dir`, shortest path first.
fn own_index(dir: &Path) -> std::collections::HashMap<String, PathBuf> {
    let mut files: Vec<PathBuf> = provenance::files_under(dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .collect();
    files.sort_by_key(|p| (p.components().count(), p.clone()));
    let mut out = std::collections::HashMap::new();
    for f in files {
        if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) {
            out.entry(stem.to_ascii_lowercase()).or_insert(f);
        }
    }
    out
}

/// The original picture's name from its label (a file in the copy, or a texture inside one of its files).
fn original_name(label: &str) -> String {
    let name = match label.split_once(" [texture ") {
        Some((_, inside)) => inside.split('"').nth(1).unwrap_or_default().to_owned(),
        None => Path::new(label)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    };
    // "Rail_Environment.bmp" and "Rail_Environment.Copy006" are both Rail_Environment
    name.split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Our picture in the shipped one's place: turned as the shipped one lies, resized to its sides; a match in the
/// alpha alone keeps the shipped colours. A shipped picture with no transparency stays without.
fn replacement(
    shipped: &Picture,
    hit: &Hit,
    original: &Print,
    own: &RgbaImage,
    alpha_ours: bool,
) -> Result<Picture, String> {
    let way = match hit.how {
        Match::Near { way, .. } => way,
        _ => 0,
    };
    let own = turned(own, undo(way));
    let own = image::imageops::resize(
        &own,
        shipped.width,
        shipped.height,
        image::imageops::FilterType::Lanczos3,
    );
    let own_alpha_drawn = {
        let mut a = own.pixels().map(|p| p.0[3]);
        let first = a.next().unwrap_or(255);
        a.any(|v| v != first)
    };
    let shipped_opaque = shipped.rgba.chunks_exact(4).all(|p| p.get(3) == Some(&255));
    let mut rgba = Vec::with_capacity(shipped.rgba.len());
    for (s, o) in shipped.rgba.chunks_exact(4).zip(own.pixels()) {
        let o = o.0;
        let px = if hit.alpha_only {
            if !own_alpha_drawn {
                return Err("matched by its alpha, and our picture has no alpha of its own".into());
            }
            [
                s.first().copied().unwrap_or(0),
                s.get(1).copied().unwrap_or(0),
                s.get(2).copied().unwrap_or(0),
                o[3],
            ]
        } else {
            let alpha = if shipped_opaque {
                255
            } else if alpha_ours {
                // made ours in an earlier pass (the picture matched one original by its alpha, another by colour)
                s.get(3).copied().unwrap_or(255)
            } else if own_alpha_drawn || original.alpha.is_some() {
                // the original's alpha came with it: ours goes with ours
                o[3]
            } else {
                // the author's own alpha over the original's colours
                s.get(3).copied().unwrap_or(255)
            };
            [o[0], o[1], o[2], alpha]
        };
        rgba.extend(px);
    }
    Ok(Picture {
        width: shipped.width,
        height: shipped.height,
        rgba,
    })
}

/// The copy match that is surest, if any, for one picture.
fn best_hit<'a>(p: &Print, originals: &'a [(String, Print)]) -> Option<(&'a str, &'a Print, Hit)> {
    originals
        .iter()
        .filter_map(|(l, o)| provenance::compare_detail(p, o).map(|h| (l.as_str(), o, h)))
        .filter(|(_, _, h)| h.how.is_copy())
        .min_by_key(|(_, _, h)| match h.how {
            Match::Exact => 0,
            Match::Near { bits, .. } => bits.saturating_add(1),
            Match::Flat { .. } => u32::MAX,
        })
}

struct Run<'a> {
    originals: &'a [(String, Print)],
    own: &'a std::collections::HashMap<String, PathBuf>,
    write: bool,
    left: usize,
}

impl Run<'_> {
    /// Our picture for one shipped picture that is the original's; `None` when it is not the original's, or
    /// (counted as left) when there is no own picture of its name or the result still matches. A picture that
    /// matches one original by its colours and another by its alpha is replaced in two passes.
    fn own_for(&mut self, label: &str, shipped: &Picture) -> Option<Picture> {
        let mut current: Option<Picture> = None;
        let mut alpha_ours = false;
        for _ in 0..3 {
            let now = current.as_ref().unwrap_or(shipped);
            let Some((orig_label, orig, hit)) = best_hit(&provenance::print(now), self.originals)
            else {
                return current;
            };
            let name = original_name(orig_label);
            let Some(own_path) = self.own.get(&name) else {
                println!("LEFT {label}: {hit:?} of {orig_label}, and no own picture named {name}");
                self.left += 1;
                return None;
            };
            let made = image::open(own_path)
                .map_err(|e| e.to_string())
                .and_then(|own| replacement(now, &hit, orig, &own.to_rgba8(), alpha_ours));
            match made {
                Ok(p) => {
                    println!(
                        "{} {label}: {hit:?} of {orig_label} -> {}",
                        if self.write {
                            "REPLACED"
                        } else {
                            "WOULD REPLACE"
                        },
                        own_path.display()
                    );
                    alpha_ours |= hit.alpha_only;
                    current = Some(p);
                }
                Err(e) => {
                    println!("LEFT {label}: {e}");
                    self.left += 1;
                    return None;
                }
            }
        }
        if let Some(p) = &current
            && let Some((still, _, h)) = best_hit(&provenance::print(p), self.originals)
        {
            println!("LEFT {label}: ours still matches {still} ({h:?})");
            self.left += 1;
            return None;
        }
        current
    }

    fn map(&mut self, path: &Path) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut new = Vec::new();
        for found in provenance::pictures_in(path) {
            let Some((object, _, slot)) = &found.inside else {
                continue;
            };
            if *object == usize::MAX {
                // a file the map carries after its data (the editor's thumbnail): not a texture
                if best_hit(&provenance::print(&found.picture), self.originals).is_some() {
                    println!("LEFT {}: an included file", found.label());
                    self.left += 1;
                }
                continue;
            }
            if let Some(p) = self.own_for(&found.label(), &found.picture) {
                new.push((*object, *slot, p));
            }
        }
        if new.is_empty() || !self.write {
            return Ok(());
        }
        let pictures: Vec<nmo::patch::NewPicture<'_>> = new
            .iter()
            .map(|(object, slot, p)| nmo::patch::NewPicture {
                object: *object,
                slot: *slot,
                width: p.width,
                height: p.height,
                rgba: &p.rgba,
            })
            .collect();
        let out = nmo::patch::replace_pictures(&bytes, &pictures)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        replace_file(path, &out)
    }

    fn folder(&mut self, dir: &Path) -> Result<(), String> {
        let originals_dir = dir.join("textures.original");
        let mut replaced = std::collections::HashSet::new();
        for f in provenance::files_under(&originals_dir) {
            for found in provenance::pictures_in(&f) {
                if let Some(p) = self.own_for(&found.label(), &found.picture) {
                    replaced.insert(f.file_name().map(|n| n.to_owned()));
                    if self.write {
                        let img = RgbaImage::from_raw(p.width, p.height, p.rgba)
                            .ok_or("picture of the wrong size")?;
                        let mut png = Vec::new();
                        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                            .map_err(|e| e.to_string())?;
                        replace_file(&f, &png)?;
                    }
                }
            }
        }
        // the folder's other pictures (the enlarged ones, the parts' own) are made from textures.original/ by the
        // toolbox; one that matches with no plain original replaced here is left for a person to look at
        for f in provenance::files_under(dir) {
            if f.starts_with(&originals_dir) {
                continue;
            }
            for found in provenance::pictures_in(&f) {
                let Some((orig, _, hit)) =
                    best_hit(&provenance::print(&found.picture), self.originals)
                else {
                    continue;
                };
                let derived = f.parent().is_some_and(|p| p == dir.join("textures"))
                    && replaced
                        .contains(&f.with_extension("png").file_name().map(|n| n.to_owned()));
                if derived {
                    println!(
                        "REMAKE {}: from its replaced original (toolbox remaster-open)",
                        f.display()
                    );
                } else {
                    println!("LEFT {}: {:?} of {orig}", found.label(), hit);
                    self.left += 1;
                }
            }
        }
        Ok(())
    }
}

/// Writes beside and moves into place, so a failed write never leaves half a file.
fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".own-pictures.tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn main() -> ExitCode {
    let o = match options() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let originals = provenance::prints_under(&o.copies);
    let own = own_index(o.own.as_deref().unwrap_or(Path::new(".")));
    println!(
        "{} pictures in the copy, {} own pictures",
        originals.len(),
        own.len()
    );
    let mut run = Run {
        originals: &originals,
        own: &own,
        write: o.write,
        left: 0,
    };
    for path in &o.paths {
        let r = if path.is_dir() {
            run.folder(path)
        } else {
            run.map(path)
        };
        if let Err(e) = r {
            eprintln!("{e}");
            run.left += 1;
        }
    }
    if run.left > 0 {
        eprintln!("{} pictures of the original left", run.left);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
