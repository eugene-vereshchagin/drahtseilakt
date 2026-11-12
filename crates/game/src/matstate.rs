// What a map's scripts changed of the level's materials and textures in this run (Set Diffuse, Set Texture, Set
// Blend Modes, ...). The level's own materials stay as loaded; this is a small overlay on top, part of the run
// state (it clones and rewinds with the run, and a new run starts without it). The renderer reads it each frame and
// rebuilds only the materials whose overlay changed (render.rs, `sync_materials`).

use level::{Blend, Level, Material, TextureSource};
use script::{MatChange, TexChange, TexInfo};
use std::collections::BTreeMap;
use std::sync::Arc;

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
    /// Virtools' compare function (1..8); recorded, the renderer's depth test stays "less or equal".
    pub z_func: Option<u8>,
    pub alpha_cutoff: Option<Option<f32>>,
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
        MatState {
            over: BTreeMap::new(),
            tex: BTreeMap::new(),
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
            MatChange::AlphaTest { on, reference, .. } => {
                cur.alpha_cutoff = Some(on.then(|| f32::from(reference) / 255.0));
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
            TexChange::Slot(s) => cur.slot = s.clamp(-1, MAX_SLOT),
        }
        self.tex.insert(texture, cur);
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
