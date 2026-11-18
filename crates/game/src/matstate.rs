// What a map's scripts changed of the level's materials and textures in this run (Set Diffuse, Set Texture, Set
// Blend Modes, ...). The level's own materials stay as loaded; this is a small overlay on top, part of the run
// state (it clones and rewinds with the run, and a new run starts without it). The renderer reads it each frame and
// rebuilds only the materials whose overlay changed (render.rs, `sync_materials`).
// Reason for the allow: material-state bookkeeping over textures already decoded and size-capped, with MAX_COPY_BYTES budget.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use crate::texpaint::{self, Pix};
use level::{Blend, Level, Material, TextureSource};
use script::{MatChange, TexChange, TexInfo, TexPaint};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// The most bytes of per-run texture copies a run may hold (the copies a script's pixel writes made).
pub const MAX_COPY_BYTES: usize = 16 << 20;

/// The game files the run's textures are read from when a script first writes one (set when a level is uploaded;
/// the pictures are the renderer's, the run's state only needs them for the copy a script writes into).
static READER: Mutex<Option<gamedata::assets::Data>> = Mutex::new(None);

/// Sets where textures are read from for a script's first write into one.
pub fn set_texture_reader(data: gamedata::assets::Data) {
    if let Ok(mut r) = READER.lock() {
        *r = Some(data);
    }
}

/// The ids of texture copies: each write takes a fresh one, so the renderer knows a copy by it (a rewind brings back
/// an older id; a new write after it takes one never used before).
static NEXT_GEN: AtomicU64 = AtomicU64::new(1);

/// A per-run copy of a level texture's pixels a script wrote into. Cloning it shares the pixels; a write copies them
/// first (`Arc::make_mut`), so a clone of the run made before the write keeps the picture as it was.
#[derive(Debug, Clone)]
pub struct TexCopy {
    pub pix: Pix,
    pub stamp: u64,
}

/// Where a level texture's pixels come from when nobody has written it yet.
#[derive(Debug)]
struct TexBase {
    embedded: Option<Pix>,
    source: Option<level::Texture>,
    loaded: OnceLock<Option<Pix>>,
}

impl TexBase {
    fn of(t: &level::Texture) -> TexBase {
        match &t.source {
            TextureSource::Pixels {
                width,
                height,
                rgba,
            } => TexBase {
                embedded: (*width > 0
                    && *height > 0
                    && *width <= MAX_TEXTURE_SIDE
                    && *height <= MAX_TEXTURE_SIDE
                    && rgba.len() == *width as usize * *height as usize * 4)
                    .then(|| Pix {
                        width: *width,
                        height: *height,
                        rgba: Arc::new(rgba.clone()),
                    }),
                source: None,
                loaded: OnceLock::new(),
            },
            TextureSource::None => TexBase {
                embedded: None,
                source: None,
                loaded: OnceLock::new(),
            },
            _ => TexBase {
                embedded: None,
                source: Some(t.clone()),
                loaded: OnceLock::new(),
            },
        }
    }

    fn pix(&self) -> Option<Pix> {
        if let Some(p) = &self.embedded {
            return Some(p.clone());
        }
        let t = self.source.as_ref()?;
        // only a picture that was read is kept: a read that failed (the game's files not known yet) is tried again
        if let Some(Some(p)) = self.loaded.get() {
            return Some(p.clone());
        }
        let data = READER.lock().ok()?.clone()?;
        let (w, h, rgba) = data.texture(t)?;
        let p = (w > 0
            && h > 0
            && w <= MAX_TEXTURE_SIDE
            && h <= MAX_TEXTURE_SIDE
            && rgba.len() == w as usize * h as usize * 4)
            .then(|| Pix {
                width: w,
                height: h,
                rgba: Arc::new(rgba),
            })?;
        let _ = self.loaded.set(Some(p.clone()));
        Some(p)
    }
}

/// The largest side a script may make a texture (Change Texture Size).
pub const MAX_TEXTURE_SIDE: u32 = 2048;
/// The highest slot Set Current Slot records.
pub const MAX_SLOT: i32 = 255;
/// The sharpest specular power a script may set.
pub const MAX_POWER: f32 = 1000.0;

/// What a script set on one material; every field is empty until a script sets it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MatOverride {
    pub diffuse: Option<[f32; 4]>,
    pub emissive: Option<[f32; 3]>,
    pub specular: Option<([f32; 3], f32)>,
    pub ambient: Option<[f32; 3]>,
    pub texture: Option<Option<usize>>,
    pub blend: Option<Option<(Blend, Blend)>>,
    pub z_write: Option<bool>,
    /// Virtools' compare function of the depth test (1..8); the renderer's pipelines use it.
    pub z_func: Option<u8>,
    pub alpha_cutoff: Option<Option<f32>>,
    /// Virtools' compare function of the alpha test (1..8); 5, greater, is what the level's own use.
    pub alpha_func: Option<u8>,
    pub two_sided: Option<bool>,
}

/// What a script set on one texture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TexOverride {
    /// The size the script asked for, capped; the picture itself is not resampled.
    pub size: Option<(u32, u32)>,
    pub slot: i32,
}

#[derive(Debug, Clone, Default)]
pub struct MatState {
    /// By level material index; only materials a script touched. Bounded by the level's material count.
    pub over: BTreeMap<usize, MatOverride>,
    pub tex: BTreeMap<usize, TexOverride>,
    base: Arc<Vec<Material>>,
    /// Each level texture's name and size (0 when the file is not decoded here).
    textures: Arc<Vec<(String, u32, u32)>>,
    /// The pictures of the level's textures (read when a script first writes one).
    bases: Arc<Vec<TexBase>>,
    /// The slots after the first of the level's textures that have several, by texture index (Set Current Slot).
    slot_bases: Arc<BTreeMap<usize, Vec<TexBase>>>,
    /// The copies scripts wrote into, by level texture index. At most `MAX_COPY_BYTES` together; only textures the
    /// level has.
    pub written: BTreeMap<usize, TexCopy>,
}

fn blend_of(n: u8) -> Option<Blend> {
    Some(match n {
        1 => Blend::Zero,
        2 => Blend::One,
        3 => Blend::SrcColor,
        4 => Blend::InvSrcColor,
        5 => Blend::SrcAlpha,
        6 => Blend::InvSrcAlpha,
        7 => Blend::DestAlpha,
        8 => Blend::InvDestAlpha,
        9 => Blend::DestColor,
        10 => Blend::InvDestColor,
        _ => return None,
    })
}

fn unit(c: f32) -> f32 {
    if c.is_finite() {
        c.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

impl MatState {
    pub fn new(level: &Level) -> MatState {
        // the slot entries kept in all are bounded (each holds its own source)
        let mut room = level::MAX_SLOT_ENTRIES;
        MatState {
            over: BTreeMap::new(),
            tex: BTreeMap::new(),
            written: BTreeMap::new(),
            bases: Arc::new(level.textures.iter().map(TexBase::of).collect()),
            slot_bases: Arc::new(
                level
                    .texture_slots
                    .iter()
                    .filter(|(t, _)| *t < level.textures.len())
                    .filter_map(|(t, more)| {
                        let own = level.textures.get(*t)?;
                        let n = more.len().min(nmo::scene::MAX_SLOTS - 1).min(room);
                        room -= n;
                        let bases = more
                            .iter()
                            .take(n)
                            .map(|source| {
                                TexBase::of(&level::Texture {
                                    source: source.clone(),
                                    ..own.clone()
                                })
                            })
                            .collect();
                        Some((*t, bases))
                    })
                    .collect(),
            ),
            base: Arc::new(level.materials.clone()),
            textures: Arc::new(
                level
                    .textures
                    .iter()
                    .map(|t| match &t.source {
                        TextureSource::Pixels { width, height, .. } => {
                            (t.name.clone(), *width, *height)
                        }
                        _ => (t.name.clone(), 0, 0),
                    })
                    .collect(),
            ),
        }
    }

    /// Whether no script changed anything (the common case: nothing to do per frame).
    pub fn is_empty(&self) -> bool {
        self.over.is_empty()
    }

    /// The material as it is drawn now: the level's with the scripts' changes over it.
    pub fn effective(&self, index: usize) -> Option<Material> {
        let mut m = self.base.get(index)?.clone();
        if let Some(o) = self.over.get(&index) {
            if let Some(d) = o.diffuse {
                m.diffuse = d;
            }
            if let Some(e) = o.emissive {
                m.emissive = e;
            }
            if let Some((c, p)) = o.specular {
                m.specular = c;
                m.power = p;
            }
            if let Some(a) = o.ambient {
                m.ambient = a;
            }
            if let Some(t) = o.texture {
                m.texture = t;
            }
            if let Some(b) = o.blend {
                m.blend = b;
            }
            if let Some(z) = o.z_write {
                m.z_write = z;
            }
            if let Some(c) = o.alpha_cutoff {
                m.alpha_cutoff = c;
            }
            if let Some(s) = o.two_sided {
                m.two_sided = s;
            }
        }
        Some(m)
    }

    /// The depth test's and the alpha test's compare functions of the material as drawn now.
    pub fn funcs(&self, index: usize) -> (u8, u8) {
        let o = self.over.get(&index);
        (
            o.and_then(|o| o.z_func)
                .unwrap_or(crate::render::Z_FUNC_DEFAULT),
            o.and_then(|o| o.alpha_func)
                .unwrap_or(crate::render::ALPHA_FUNC_DEFAULT),
        )
    }

    pub fn apply(&mut self, material: usize, change: MatChange) {
        if material >= self.base.len() {
            return;
        }
        let mut cur = self.over.get(&material).cloned().unwrap_or_default();
        match change {
            MatChange::Diffuse { color, keep_alpha } => {
                let old = self.effective(material).map_or(1.0, |m| m.diffuse[3]);
                let [r, g, b, a] = color.map(unit);
                cur.diffuse = Some([r, g, b, if keep_alpha { old } else { a }]);
            }
            MatChange::Emissive(c) => cur.emissive = Some(c.map(unit)),
            MatChange::Specular { color, power } => {
                let p = if power.is_finite() {
                    power.clamp(0.0, MAX_POWER)
                } else {
                    0.0
                };
                cur.specular = Some((color.map(unit), p));
            }
            MatChange::Ambient(c) => cur.ambient = Some(c.map(unit)),
            MatChange::Texture(t) => {
                if t.is_some_and(|t| t >= self.textures.len()) {
                    return;
                }
                cur.texture = Some(t);
            }
            MatChange::CopyFrom(from) => {
                let Some(m) = self.effective(from) else {
                    return;
                };
                cur = MatOverride {
                    diffuse: Some(m.diffuse),
                    emissive: Some(m.emissive),
                    specular: Some((m.specular, m.power)),
                    ambient: Some(m.ambient),
                    texture: Some(m.texture),
                    blend: Some(m.blend),
                    z_write: Some(m.z_write),
                    z_func: self.over.get(&from).and_then(|o| o.z_func),
                    alpha_cutoff: Some(m.alpha_cutoff),
                    alpha_func: self.over.get(&from).and_then(|o| o.alpha_func),
                    two_sided: Some(m.two_sided),
                };
            }
            MatChange::Blend { on, src, dst } => {
                if !on {
                    cur.blend = Some(None);
                } else if let (Some(s), Some(d)) = (blend_of(src), blend_of(dst)) {
                    cur.blend = Some(Some((s, d)));
                } else {
                    return;
                }
            }
            MatChange::ZBuffer { write, func } => {
                cur.z_write = Some(write);
                if (1..=8).contains(&func) {
                    cur.z_func = Some(func);
                }
            }
            MatChange::AlphaTest {
                on,
                reference,
                func,
            } => {
                cur.alpha_cutoff = Some(on.then(|| f32::from(reference) / 255.0));
                if (1..=8).contains(&func) {
                    cur.alpha_func = Some(func);
                }
            }
        }
        if self.over.get(&material) != Some(&cur) {
            self.over.insert(material, cur);
        }
    }

    pub fn apply_texture(&mut self, texture: usize, change: TexChange) {
        if texture >= self.textures.len() {
            return;
        }
        let mut cur = self.tex.get(&texture).cloned().unwrap_or_default();
        match change {
            TexChange::Size { width, height, .. } => {
                if width > 0 && height > 0 {
                    cur.size = Some((width.min(MAX_TEXTURE_SIDE), height.min(MAX_TEXTURE_SIDE)));
                }
            }
            TexChange::Slot(s) => {
                cur.slot = s.clamp(-1, MAX_SLOT);
                self.show_slot(texture, cur.slot);
            }
        }
        self.tex.insert(texture, cur);
    }

    /// Set Current Slot on a texture that has several slots: the slot's picture is what the texture draws (as the
    /// copy a script wrote, so the renderer uploads it the same way; a write after it paints that picture); slot 0
    /// gives the level's own back. A slot the texture does not have, or whose picture cannot be read, changes nothing.
    fn show_slot(&mut self, texture: usize, slot: i32) {
        let Some(more) = self.slot_bases.get(&texture) else {
            return;
        };
        let Ok(k) = usize::try_from(slot) else {
            return;
        };
        if k == 0 {
            self.written.remove(&texture);
            return;
        }
        let Some(pix) = more.get(k - 1).and_then(TexBase::pix) else {
            return;
        };
        if self
            .written
            .get(&texture)
            .is_some_and(|c| Arc::ptr_eq(&c.pix.rgba, &pix.rgba))
        {
            return;
        }
        let held = self.written.get(&texture).map_or(0, |c| c.pix.rgba.len());
        if self.copy_bytes() - held + pix.rgba.len() > MAX_COPY_BYTES {
            return;
        }
        let stamp = NEXT_GEN.fetch_add(1, Ordering::Relaxed);
        self.written.insert(texture, TexCopy { pix, stamp });
    }

    /// How many materials the level has.
    pub fn material_count(&self) -> usize {
        self.base.len()
    }

    /// The pixels of a level texture as drawn now: a script's copy, else the level's picture.
    fn pixels(&self, texture: usize) -> Option<Pix> {
        match self.written.get(&texture) {
            Some(c) => Some(c.pix.clone()),
            None => self.bases.get(texture)?.pix(),
        }
    }

    /// The bytes the copies hold.
    pub fn copy_bytes(&self) -> usize {
        self.written.values().map(|c| c.pix.rgba.len()).sum()
    }

    /// A script writes pixels of a level texture (Blend Textures, Write In Texture): the first write makes the
    /// run's copy of it (past `MAX_COPY_BYTES` of copies, or for a picture that is not there, nothing is written).
    /// Returns the pixels written, which the block pays for from the tick's work.
    pub fn paint(&mut self, target: usize, paint: TexPaint) -> u64 {
        if target >= self.textures.len() {
            return 0;
        }
        let (pix, n) = match paint {
            TexPaint::Blend { a, b, factor } => {
                let (Some(pa), Some(pb)) = (self.pixels(a), self.pixels(b)) else {
                    return 0;
                };
                // the target's own size; a target nobody has takes the first texture's
                let (w, h) = self
                    .pixels(target)
                    .map_or((pa.width, pa.height), |t| (t.width, t.height));
                let rgba = texpaint::blend(w, h, &pa, &pb, factor);
                let n = u64::from(w) * u64::from(h);
                (
                    Pix {
                        width: w,
                        height: h,
                        rgba: Arc::new(rgba),
                    },
                    n,
                )
            }
            TexPaint::Write {
                uv,
                color,
                size,
                pen,
                mode,
            } => {
                let pen = pen.and_then(|p| self.pixels(p));
                // a copy this run already has is written in place (when no clone of the run shares it)
                let had = self.written.remove(&target);
                let old_gen = had.as_ref().map(|c| c.stamp);
                let Some(mut cur) = had.map(|c| c.pix).or_else(|| self.pixels(target)) else {
                    return 0;
                };
                let dims = (cur.width, cur.height);
                let n = texpaint::write(
                    Arc::make_mut(&mut cur.rgba).as_mut_slice(),
                    dims,
                    uv,
                    color,
                    size,
                    pen.as_ref(),
                    mode,
                );
                if n == 0 {
                    // nothing changed: the copy stays as it was
                    if let Some(stamp) = old_gen {
                        self.written.insert(target, TexCopy { pix: cur, stamp });
                    }
                    return 0;
                }
                (cur, n)
            }
        };
        // a blend that gives the picture the copy already is (a factor that did not move) changes nothing: the copy keeps
        // its stamp and the renderer does not upload it again
        if self
            .written
            .get(&target)
            .is_some_and(|c| c.pix.rgba == pix.rgba)
        {
            return n;
        }
        let bytes = pix.rgba.len();
        let held = self.written.get(&target).map_or(0, |c| c.pix.rgba.len());
        if self.copy_bytes() - held + bytes > MAX_COPY_BYTES {
            return 0;
        }
        let stamp = NEXT_GEN.fetch_add(1, Ordering::Relaxed);
        self.written.insert(target, TexCopy { pix, stamp });
        n
    }

    pub fn texture_info(&self, texture: usize) -> Option<TexInfo> {
        let (name, w, h) = self.textures.get(texture)?;
        let t = self.tex.get(&texture);
        let (width, height) = t.and_then(|t| t.size).unwrap_or((*w, *h));
        Some(TexInfo {
            file: name.clone(),
            width,
            height,
            slot: t.map_or(0, |t| t.slot),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn material(d: [f32; 4]) -> Material {
        Material {
            name: String::new(),
            diffuse: d,
            ambient: [0.0; 3],
            emissive: [0.0; 3],
            specular: [0.0; 3],
            power: 0.0,
            texture: None,
            normal: None,
            parallax: 0.0,
            blend: None,
            alpha_cutoff: None,
            z_write: true,
            two_sided: false,
            unlit: false,
            texgen: None,
        }
    }

    fn state() -> MatState {
        MatState {
            base: Arc::new(vec![
                material([0.5, 0.5, 0.5, 0.7]),
                material([1.0, 0.0, 0.0, 1.0]),
            ]),
            textures: Arc::new(vec![("a.bmp".into(), 64, 32), ("b.bmp".into(), 0, 0)]),
            ..MatState::default()
        }
    }

    fn flat(w: u32, h: u32, c: [u8; 4]) -> Pix {
        Pix {
            width: w,
            height: h,
            rgba: Arc::new(c.repeat((w * h) as usize)),
        }
    }

    /// A state with three textures of known pictures (black 4 x 4, white 4 x 4, and one 2048 x 2048).
    fn painted() -> MatState {
        let embedded = |p: Pix| TexBase {
            embedded: Some(p),
            source: None,
            loaded: OnceLock::new(),
        };
        MatState {
            textures: Arc::new(vec![
                ("a".into(), 4, 4),
                ("b".into(), 4, 4),
                ("big".into(), 2048, 2048),
                ("big2".into(), 2048, 2048),
                ("gone".into(), 0, 0),
            ]),
            bases: Arc::new(vec![
                embedded(flat(4, 4, [0, 0, 0, 255])),
                embedded(flat(4, 4, [200, 100, 0, 255])),
                embedded(flat(2048, 2048, [1, 2, 3, 4])),
                embedded(flat(2048, 2048, [1, 2, 3, 4])),
                TexBase {
                    embedded: None,
                    source: None,
                    loaded: OnceLock::new(),
                },
            ]),
            ..MatState::default()
        }
    }

    fn put(x: [f32; 2]) -> TexPaint {
        TexPaint::Write {
            uv: x,
            color: [1.0, 1.0, 1.0, 1.0],
            size: 1,
            pen: None,
            mode: 1,
        }
    }

    #[test]
    fn set_current_slot_shows_the_slot_s_picture_of_a_texture_with_several() {
        let embedded = |p: Pix| TexBase {
            embedded: Some(p),
            source: None,
            loaded: OnceLock::new(),
        };
        let mut s = painted();
        s.slot_bases = Arc::new(BTreeMap::from([(
            0,
            vec![
                embedded(flat(2, 2, [9, 9, 9, 255])),
                embedded(flat(2, 2, [7, 7, 7, 255])),
            ],
        )]));
        s.apply_texture(0, TexChange::Slot(2));
        assert_eq!(s.written[&0].pix.rgba[..4], [7, 7, 7, 255]);
        assert_eq!(s.texture_info(0).unwrap().slot, 2);
        let stamp = s.written[&0].stamp;
        s.apply_texture(0, TexChange::Slot(2));
        assert_eq!(
            s.written[&0].stamp, stamp,
            "the same picture is not made again"
        );
        s.apply_texture(0, TexChange::Slot(1));
        assert_eq!(s.written[&0].pix.rgba[..4], [9, 9, 9, 255]);
        // a slot the texture does not have, or none (-1), leaves the picture
        s.apply_texture(0, TexChange::Slot(3));
        s.apply_texture(0, TexChange::Slot(-1));
        assert_eq!(s.written[&0].pix.rgba[..4], [9, 9, 9, 255]);
        // slot 0 is the level's own picture again; a texture of one slot takes no copy
        s.apply_texture(0, TexChange::Slot(0));
        assert!(s.written.is_empty());
        s.apply_texture(1, TexChange::Slot(1));
        assert!(s.written.is_empty());
        // a rewind (a clone before) keeps its own
        let mut t = s.clone();
        s.apply_texture(0, TexChange::Slot(1));
        t.apply_texture(0, TexChange::Slot(0));
        assert!(t.written.is_empty() && s.written.contains_key(&0));
    }

    #[test]
    fn a_write_makes_a_copy_and_a_clone_before_it_keeps_the_picture() {
        let mut s = painted();
        assert!(s.written.is_empty());
        let before = s.clone();
        assert_eq!(s.paint(0, put([1.0, 1.0])), 1);
        let c = &s.written[&0];
        assert_eq!(
            &c.pix.rgba[(4 + 1) * 4..(4 + 1) * 4 + 4],
            &[255, 255, 255, 255]
        );
        assert!(
            before.written.is_empty(),
            "the clone made before the write has no copy"
        );
        // a clone after the write shares the pixels; writing again leaves that clone as it was
        let stamp = c.stamp;
        let mid = s.clone();
        assert!(Arc::ptr_eq(
            &mid.written[&0].pix.rgba,
            &s.written[&0].pix.rgba
        ));
        assert_eq!(s.paint(0, put([2.0, 2.0])), 1);
        assert_ne!(s.written[&0].stamp, stamp, "a changed copy has a new stamp");
        assert_eq!(mid.written[&0].stamp, stamp);
        assert_eq!(
            &mid.written[&0].pix.rgba[(2 * 4 + 2) * 4..(2 * 4 + 2) * 4 + 4],
            &[0, 0, 0, 255]
        );
        assert_eq!(
            &s.written[&0].pix.rgba[(2 * 4 + 2) * 4..(2 * 4 + 2) * 4 + 4],
            &[255, 255, 255, 255]
        );
        // a write that changes nothing keeps the copy and its stamp
        let now = s.written[&0].stamp;
        assert_eq!(s.paint(0, put([90.0, 1.0])), 0);
        assert_eq!(s.written[&0].stamp, now);
        // rewind: back to the clone
        s = before;
        assert!(s.written.is_empty());
    }

    #[test]
    fn blend_writes_the_mix_into_the_target() {
        let mut s = painted();
        assert_eq!(
            s.paint(
                0,
                TexPaint::Blend {
                    a: 0,
                    b: 1,
                    factor: 0.5
                }
            ),
            16
        );
        assert_eq!(&s.written[&0].pix.rgba[..4], &[100, 50, 0, 255]);
        // a blend that gives the picture the copy already is leaves the copy (and its stamp) alone
        let blend = TexPaint::Blend {
            a: 1,
            b: 1,
            factor: 0.3,
        };
        assert_eq!(s.paint(0, blend), 16);
        let stamp = s.written[&0].stamp;
        assert_eq!(s.paint(0, blend), 16);
        assert_eq!(s.written[&0].stamp, stamp);
        // the picture the level has is untouched: blending from factor 0 gives texture 1 back
        assert_eq!(
            s.paint(
                1,
                TexPaint::Blend {
                    a: 1,
                    b: 1,
                    factor: 0.0
                }
            ),
            16
        );
        assert_eq!(&s.written[&1].pix.rgba[..4], &[200, 100, 0, 255]);
        // a texture nobody has cannot be blended from
        assert_eq!(
            s.paint(
                0,
                TexPaint::Blend {
                    a: 4,
                    b: 1,
                    factor: 0.5
                }
            ),
            0
        );
        assert_eq!(s.paint(4, put([0.0, 0.0])), 0);
    }

    #[test]
    fn copies_are_capped() {
        let mut s = painted();
        // 2048 x 2048 x 4 = 16 MiB: one such copy fills the cap
        assert_eq!(s.paint(2, put([0.0, 0.0])), 1);
        assert_eq!(s.copy_bytes(), MAX_COPY_BYTES);
        // no second copy fits, but the one held is written again
        assert_eq!(s.paint(3, put([0.0, 0.0])), 0);
        assert_eq!(s.paint(0, put([0.0, 0.0])), 0);
        assert_eq!(s.written.len(), 1);
        assert_eq!(s.paint(2, put([1.0, 0.0])), 1);
        // a texture the level does not have
        assert_eq!(s.paint(99, put([0.0, 0.0])), 0);
        assert_eq!(s.written.len(), 1);
    }

    #[test]
    fn every_change_shows_in_the_effective_material() {
        let mut s = state();
        assert!(s.is_empty());
        s.apply(
            0,
            MatChange::Diffuse {
                color: [1.0, 0.0, 0.0, 0.2],
                keep_alpha: true,
            },
        );
        assert_eq!(s.effective(0).unwrap().diffuse, [1.0, 0.0, 0.0, 0.7]);
        s.apply(
            0,
            MatChange::Diffuse {
                color: [0.0, 1.0, 0.0, 0.2],
                keep_alpha: false,
            },
        );
        assert_eq!(s.effective(0).unwrap().diffuse, [0.0, 1.0, 0.0, 0.2]);
        s.apply(0, MatChange::Emissive([0.1, 0.2, 0.3]));
        s.apply(0, MatChange::Ambient([0.4, 0.5, 0.6]));
        s.apply(
            0,
            MatChange::Specular {
                color: [1.0; 3],
                power: 20.0,
            },
        );
        s.apply(0, MatChange::Texture(Some(1)));
        s.apply(
            0,
            MatChange::Blend {
                on: true,
                src: 5,
                dst: 6,
            },
        );
        s.apply(
            0,
            MatChange::ZBuffer {
                write: false,
                func: 4,
            },
        );
        s.apply(
            0,
            MatChange::AlphaTest {
                on: true,
                func: 7,
                reference: 51,
            },
        );
        let m = s.effective(0).unwrap();
        assert_eq!(m.emissive, [0.1, 0.2, 0.3]);
        assert_eq!(m.ambient, [0.4, 0.5, 0.6]);
        assert_eq!((m.specular, m.power), ([1.0; 3], 20.0));
        assert_eq!(m.texture, Some(1));
        assert_eq!(m.blend, Some((Blend::SrcAlpha, Blend::InvSrcAlpha)));
        assert!(!m.z_write);
        assert_eq!(s.over[&0].z_func, Some(4));
        assert_eq!(s.funcs(0), (4, 7));
        assert_eq!(
            s.funcs(1),
            (4, 5),
            "an untouched material keeps the level's tests"
        );
        assert!((m.alpha_cutoff.unwrap() - 0.2).abs() < 1e-6);
        s.apply(
            0,
            MatChange::Blend {
                on: false,
                src: 0,
                dst: 0,
            },
        );
        s.apply(
            0,
            MatChange::AlphaTest {
                on: false,
                func: 7,
                reference: 0,
            },
        );
        s.apply(0, MatChange::Texture(None));
        let m = s.effective(0).unwrap();
        assert_eq!((m.blend, m.alpha_cutoff, m.texture), (None, None, None));
        // the other material is untouched
        assert_eq!(s.effective(1).unwrap().diffuse, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn set_material_copies_the_whole_state() {
        let mut s = state();
        s.apply(1, MatChange::Texture(Some(0)));
        s.apply(
            1,
            MatChange::Blend {
                on: true,
                src: 2,
                dst: 2,
            },
        );
        s.apply(0, MatChange::CopyFrom(1));
        let m = s.effective(0).unwrap();
        assert_eq!(m.diffuse, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(m.texture, Some(0));
        assert_eq!(m.blend, Some((Blend::One, Blend::One)));
    }

    #[test]
    fn a_clone_is_a_snapshot_and_rewind_restores_it() {
        let mut s = state();
        s.apply(0, MatChange::Emissive([1.0; 3]));
        let saved = s.clone();
        s.apply(1, MatChange::Emissive([0.5; 3]));
        s.apply(0, MatChange::Emissive([0.0; 3]));
        s = saved;
        assert_eq!(s.over.len(), 1);
        assert_eq!(s.effective(0).unwrap().emissive, [1.0; 3]);
        assert_eq!(s.effective(1).unwrap().emissive, [0.0; 3]);
    }

    #[test]
    fn caps_hold() {
        let mut s = state();
        // a material or texture the level does not have is ignored; the table never grows past the level's own
        for i in 2..10_000 {
            s.apply(i, MatChange::Emissive([1.0; 3]));
            s.apply_texture(i, TexChange::Slot(1));
        }
        assert!(s.over.is_empty() && s.tex.is_empty());
        s.apply(0, MatChange::Texture(Some(9)));
        assert!(s.over.is_empty());
        s.apply(
            0,
            MatChange::Specular {
                color: [9.0, f32::NAN, -1.0],
                power: 1e9,
            },
        );
        assert_eq!(s.over[&0].specular, Some(([1.0, 0.0, 0.0], MAX_POWER)));
        s.apply(
            0,
            MatChange::Blend {
                on: true,
                src: 99,
                dst: 5,
            },
        );
        assert_eq!(s.over[&0].blend, None);
        s.apply_texture(
            0,
            TexChange::Size {
                width: 1 << 20,
                height: 8,
                clear: false,
            },
        );
        assert_eq!(s.texture_info(0).unwrap().width, MAX_TEXTURE_SIDE);
        assert_eq!(s.texture_info(0).unwrap().height, 8);
        s.apply_texture(0, TexChange::Slot(i32::MAX));
        assert_eq!(s.texture_info(0).unwrap().slot, MAX_SLOT);
        assert_eq!(s.texture_info(5), None);
    }
}
