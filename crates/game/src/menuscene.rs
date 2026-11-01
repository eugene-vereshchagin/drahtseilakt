// The 3D scene behind the menus, as the original's MenuLevel.nmo makes it (docs/animation.md): its objects, its
// camera (Cam_MenuLevel) carried round a closed TCB curve (I_MenuLevel_Curve) once every 44 seconds by Position On
// Curve and looking at its target, and the four flames of its start. The stone ball's recorded roll is not played
// yet.

use crate::render::Instance;
use glam::{Mat4, Vec3};

/// One round of the curve (MenuLevel_Init's Bezier Progression).
pub const ROUND_MS: f32 = 44_000.0;
/// Samples per segment when the curve measures its length.
const SAMPLES: usize = 100;

/// A closed or open curve of TCB points, evaluated as CKCurve does (Hermite segments, by arc length).
#[derive(Debug, Clone)]
pub struct Curve {
    points: Vec<Vec3>,
    outs: Vec<Vec3>,
    ins: Vec<Vec3>,
    /// Arc length at each point, and the whole length.
    at: Vec<f32>,
    length: f32,
    open: bool,
}

fn hermite(p0: Vec3, t0: Vec3, p1: Vec3, t1: Vec3, s: f32) -> Vec3 {
    let (s2, s3) = (s * s, s * s * s);
    p0 * (2.0 * s3 - 3.0 * s2 + 1.0)
        + p1 * (-2.0 * s3 + 3.0 * s2)
        + t0 * (s3 - 2.0 * s2 + s)
        + t1 * (s3 - s2)
}

impl Curve {
    /// A curve through `points` (world positions) with their tension, continuity and bias. The tangents are
    /// CKCurve's (docs/animation.md, 3.2: its in and out tangents, which name continuity's sign the other way round
    /// from the usual Kochanek-Bartels; the game's curves have continuity 0). User tangents, straight segments and
    /// fitting are not used by the game's curves and are left out.
    pub fn new(points: Vec<Vec3>, tcb: &[[f32; 3]], open: bool) -> Curve {
        let n = points.len();
        let wrap = |i: isize| -> usize {
            if open {
                i.clamp(0, n as isize - 1) as usize
            } else {
                i.rem_euclid(n.max(1) as isize) as usize
            }
        };
        let mut outs = Vec::with_capacity(n);
        let mut ins = Vec::with_capacity(n);
        for i in 0..n {
            let p = points[i];
            let dp = if open && i == 0 {
                Vec3::ZERO
            } else {
                p - points[wrap(i as isize - 1)]
            };
            let dn = if open && i + 1 == n {
                Vec3::ZERO
            } else {
                points[wrap(i as isize + 1)] - p
            };
            let [t, c, b] = tcb.get(i).copied().unwrap_or([0.0; 3]);
            outs.push(
                ((1.0 - t) * (1.0 - c) * (1.0 - b) * dn + (1.0 - t) * (1.0 + c) * (1.0 + b) * dp)
                    / 2.0,
            );
            ins.push(
                ((1.0 - t) * (1.0 + c) * (1.0 - b) * dn + (1.0 - t) * (1.0 - c) * (1.0 + b) * dp)
                    / 2.0,
            );
        }
        let mut at = vec![0.0; n];
        let mut len = 0.0;
        let segments = if open { n.saturating_sub(1) } else { n };
        for i in 0..segments {
            let j = (i + 1) % n.max(1);
            at[i] = len;
            let mut prev = points[i];
            for k in 1..=SAMPLES {
                let q = hermite(
                    points[i],
                    outs[i],
                    points[j],
                    ins[j],
                    k as f32 / SAMPLES as f32,
                );
                len += q.distance(prev);
                prev = q;
            }
            if i + 1 != n {
                at[j] = len;
            }
        }
        Curve {
            points,
            outs,
            ins,
            at,
            length: len,
            open,
        }
    }

    /// The point at `step` (0..1 of the length; a closed curve wraps).
    pub fn pos(&self, step: f32) -> Vec3 {
        let n = self.points.len();
        if n < 2 {
            return self.points.first().copied().unwrap_or(Vec3::ZERO);
        }
        let step = if self.open {
            step.clamp(0.0, 1.0)
        } else {
            step.rem_euclid(1.0)
        };
        let l = step * self.length;
        let next = self
            .at
            .iter()
            .position(|&a| a > l)
            .unwrap_or(if self.open { n - 1 } else { 0 });
        let prev = if next == 0 {
            if self.open { 0 } else { n - 1 }
        } else {
            next - 1
        };
        let (l0, l1) = (
            self.at[prev],
            if next == 0 {
                self.length
            } else {
                self.at[next]
            },
        );
        let s = if l1 == l0 { 0.0 } else { (l - l0) / (l1 - l0) };
        hermite(
            self.points[prev],
            self.outs[prev],
            self.points[next],
            self.ins[next],
            s,
        )
    }
}

/// The menu's scene: its model (uploaded by the caller), its objects to draw, the camera's way and target.
pub struct MenuScene {
    pub level: level::Level,
    pub model: Option<usize>,
    curve: Curve,
    target: Vec3,
    /// The camera's vertical field of view (the file gives the horizontal one of its 4:3 view).
    pub fov_y: f32,
    pub effects: crate::particles::Effects,
}

impl MenuScene {
    /// MenuLevel.nmo from the player's copy, if it is there and has its camera and curve.
    pub fn load(data: &crate::assets::Data) -> Option<MenuScene> {
        let path = data.file("3D Entities/MenuLevel.nmo")?;
        let bytes = std::fs::read(&path).ok()?;
        let file = nmo::read(&bytes).ok()?;
        let doc = nmo::Document::parse(&file).ok()?;
        let chunk = |i: usize| doc.get(i).and_then(|o| o.chunk.as_ref());
        let position = |i: usize| {
            chunk(i)
                .and_then(|c| nmo::scene::entity(c).ok())
                .map(|e| Vec3::from(e.matrix[3]))
        };
        let (_, cam_obj) = doc.of_class(nmo::class::TARGET_CAMERA).next()?;
        let cam = nmo::scene::camera(cam_obj.chunk.as_ref()?).ok()?;
        let target = position(cam.target?)?;
        let (_, curve_obj) = doc.of_class(nmo::class::CURVE).next()?;
        let c = nmo::scene::curve(curve_obj.chunk.as_ref()?).ok()??;
        let mut points = Vec::new();
        let mut tcb = Vec::new();
        for &p in &c.points {
            points.push(position(p)?);
            let cp = chunk(p).and_then(|ch| nmo::scene::curve_point(ch).ok().flatten());
            tcb.push(cp.map_or([0.0; 3], |cp| cp.tcb_values.map(|v| v.clamp(-1.0, 1.0))));
        }
        let level = crate::assets::load_nmo(&path).ok()?.level;
        let effects = crate::particles::Effects::menu(&level);
        // a sane lens (between about 3 and 170 degrees), a curve of finite points and length
        let fov_y = 2.0 * ((cam.fov.clamp(0.05, 3.0) / 2.0).tan() * 0.75).atan();
        if !points.iter().all(|p| p.is_finite()) || !target.is_finite() {
            return None;
        }
        Some(MenuScene {
            curve: Curve::new(points, &tcb, c.open),
            target,
            fov_y,
            level,
            model: None,
            effects,
        })
    }

    /// The camera at `ms` into the menu: where it is and what it looks at.
    pub fn camera(&self, ms: f32) -> (Vec3, Vec3) {
        (self.curve.pos(ms / ROUND_MS), self.target)
    }

    /// The scene's objects to draw (visible, with a material).
    pub fn instances(&self) -> Vec<Instance> {
        let Some(model) = self.model else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for o in &self.level.objects {
            let Some(mesh) = o.mesh else { continue };
            let lit = self
                .level
                .meshes
                .get(mesh)
                .is_some_and(|m| m.parts.iter().any(|p| p.material.is_some()));
            if o.visible && lit && o.sprite.is_none() {
                out.push(Instance::plain(
                    model,
                    mesh,
                    Mat4::from_cols_array_2d(&o.matrix),
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menu's curve (docs/animation.md, 3.3): four points round a circle of radius about 95, TCB tension -0.6;
    /// its length is 595.27, and it starts at the first point.
    #[test]
    fn the_menu_curve_measures_as_the_original() {
        let p = vec![
            Vec3::new(0.477, 40.0, -94.955),
            Vec3::new(95.432, 40.0, 0.954),
            Vec3::new(-0.954, 40.0, 95.433),
            Vec3::new(-94.955, 40.0, -1.431),
        ];
        let c = Curve::new(p.clone(), &[[-0.6, 0.0, 0.0]; 4], false);
        assert!((c.length - 595.27).abs() < 0.5, "{}", c.length);
        assert!(c.pos(0.0).distance(p[0]) < 1e-3);
        assert!(c.pos(1.0).distance(p[0]) < 1e-3);
        let r = Vec3::new(c.pos(0.125).x, 0.0, c.pos(0.125).z).length();
        assert!((r - 95.0).abs() < 8.0, "{r}");
    }
}
