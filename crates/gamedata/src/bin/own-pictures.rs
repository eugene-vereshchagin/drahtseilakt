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
        let bytes = read_plain(path)?;
        let mut new = Vec::new();
        // the pictures and the patch come from the same bytes: nothing read twice
        for found in provenance::pictures_in_bytes(path, &bytes) {
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
            let png = f.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"));
            for found in provenance::pictures_in(&f) {
                if !png {
                    // only a .png is rewritten as one; any other file of the original's stays for a person
                    if let Some((orig, _, hit)) =
                        best_hit(&provenance::print(&found.picture), self.originals)
                    {
                        println!("LEFT {}: {hit:?} of {orig}, not a .png", found.label());
                        self.left += 1;
                    }
                    continue;
                }
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

/// The most a map file read here may hold (the patcher refuses far less).
const MAP_MAX: u64 = 256 * 1024 * 1024;

/// A map's bytes, only from a plain file: a symbolic link or anything else is refused, not followed.
fn read_plain(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let fail = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    let meta = std::fs::symlink_metadata(path).map_err(|e| fail(&e))?;
    if meta.file_type().is_symlink() {
        return Err(fail(&"a symbolic link; not followed"));
    }
    if !meta.is_file() {
        return Err(fail(&"not a plain file"));
    }
    let file = gamedata::package::open_plain(path, &meta).map_err(|e| fail(&e))?;
    let mut bytes = Vec::new();
    file.take(MAP_MAX.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| fail(&e))?;
    if u64::try_from(bytes.len()).map_or(true, |n| n > MAP_MAX) {
        return Err(fail(&"too large"));
    }
    Ok(bytes)
}

/// Names for the file written beside `path` before it moves into place: hidden, in the same folder, and
/// unpredictable (a per-process random key hashed with the time and the attempt).
fn temp_names(path: &Path) -> impl Iterator<Item = PathBuf> + '_ {
    use std::hash::{BuildHasher, Hasher};
    let key = std::collections::hash_map::RandomState::new();
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let stem = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    (0..32u32).map(move |attempt| {
        let mut h = key.build_hasher();
        h.write_u32(attempt);
        h.write_u32(std::process::id());
        if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            h.write_u128(t.as_nanos());
        }
        dir.join(format!(".{stem}.{:016x}.own-pictures.tmp", h.finish()))
    })
}

/// Replaces the plain file `path` with `bytes`: written to a new file beside it (never one that exists, never
/// through a link), with the old file's permissions, synced, then moved into place. On any failure the original
/// stays as it was and the new file is removed.
fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    replace_file_via(path, bytes, temp_names(path))
}

fn replace_file_via(
    path: &Path,
    bytes: &[u8],
    names: impl Iterator<Item = PathBuf>,
) -> Result<(), String> {
    use std::io::Write;
    let fail = |p: &Path, e: &dyn std::fmt::Display| format!("{}: {e}", p.display());
    let meta = std::fs::symlink_metadata(path).map_err(|e| fail(path, &e))?;
    if !meta.file_type().is_file() {
        return Err(fail(path, &"not a plain file; left as it is"));
    }
    for tmp in names {
        // create_new is O_CREAT | O_EXCL: an existing name, a link among them, is never opened
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(fail(&tmp, &e)),
        };
        let written = file
            .set_permissions(meta.permissions())
            .and_then(|()| file.write_all(bytes))
            .and_then(|()| file.sync_all())
            .map_err(|e| fail(&tmp, &e));
        drop(file);
        let moved = written.and_then(|()| {
            // the target must still be the plain file looked at, not a link swapped in meanwhile
            match std::fs::symlink_metadata(path) {
                Ok(m) if m.file_type().is_file() => {
                    std::fs::rename(&tmp, path).map_err(|e| fail(path, &e))
                }
                Ok(_) => Err(fail(path, &"no longer a plain file; left as it is")),
                Err(e) => Err(fail(path, &e)),
            }
        });
        if moved.is_err() {
            let _ = std::fs::remove_file(&tmp);
            return moved;
        }
        // the move itself made durable where the system allows syncing a folder
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty())
            && let Ok(d) = std::fs::File::open(dir)
        {
            let _ = d.sync_all();
        }
        return Ok(());
    }
    Err(fail(path, &"no free name for the new file beside it"))
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "drahtseilakt-own-pictures-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn entries(d: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn a_file_is_replaced_whole_and_nothing_is_left_beside_it() {
        let d = scratch("replace");
        let f = d.join("map.nmo");
        std::fs::write(&f, b"old").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o640)).unwrap();
        }
        replace_file(&f, b"new bytes").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"new bytes");
        assert_eq!(entries(&d), vec!["map.nmo".to_owned()]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o640, "the permissions stay");
        }
        // two names from one call differ
        let names: Vec<PathBuf> = temp_names(&f).take(2).collect();
        assert_ne!(names[0], names[1]);
        assert_eq!(names[0].parent(), Some(d.as_path()));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_failure_leaves_the_original_intact() {
        let d = scratch("fail");
        let f = d.join("map.nmo");
        std::fs::write(&f, b"old").unwrap();
        // the new file cannot be made: nothing changes
        let r = replace_file_via(&f, b"new", std::iter::once(d.join("missing/x.tmp")));
        assert!(r.is_err());
        assert_eq!(std::fs::read(&f).unwrap(), b"old");
        // no free name: nothing changes
        std::fs::write(d.join("taken.tmp"), b"someone's").unwrap();
        let r = replace_file_via(&f, b"new", std::iter::once(d.join("taken.tmp")));
        assert!(r.is_err());
        assert_eq!(std::fs::read(&f).unwrap(), b"old");
        assert_eq!(std::fs::read(d.join("taken.tmp")).unwrap(), b"someone's");
        // a folder in the file's place is refused
        let r = replace_file(&d.join("missing"), b"new");
        assert!(r.is_err());
        std::fs::create_dir(d.join("sub")).unwrap();
        assert!(replace_file(&d.join("sub"), b"new").is_err());
        assert!(d.join("sub").is_dir());
        assert_eq!(
            entries(&d),
            vec![
                "map.nmo".to_owned(),
                "sub".to_owned(),
                "taken.tmp".to_owned()
            ]
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn links_are_never_followed() {
        let d = scratch("links");
        let f = d.join("map.nmo");
        let victim = d.join("victim");
        std::fs::write(&f, b"old").unwrap();
        std::fs::write(&victim, b"victim").unwrap();
        // a link where the new file would go is skipped, its target untouched
        let planted = d.join("planted.tmp");
        std::os::unix::fs::symlink(&victim, &planted).unwrap();
        let r = replace_file_via(
            &f,
            b"new",
            [planted.clone(), d.join("free.tmp")].into_iter(),
        );
        assert!(r.is_ok(), "{r:?}");
        assert_eq!(std::fs::read(&f).unwrap(), b"new");
        assert_eq!(std::fs::read(&victim).unwrap(), b"victim");
        assert!(
            std::fs::symlink_metadata(&planted)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        // only a link where the new file would go: refused
        let r = replace_file_via(&f, b"newer", std::iter::once(planted.clone()));
        assert!(r.is_err());
        assert_eq!(std::fs::read(&f).unwrap(), b"new");
        assert_eq!(std::fs::read(&victim).unwrap(), b"victim");
        // PATH a link: refused, read or written, and the link's target untouched
        let link = d.join("link.nmo");
        std::os::unix::fs::symlink(&victim, &link).unwrap();
        assert!(read_plain(&link).is_err());
        assert!(replace_file(&link, b"new").is_err());
        assert_eq!(std::fs::read(&victim).unwrap(), b"victim");
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(read_plain(&f).unwrap(), b"new");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
