// The 3D scene behind the menus, as the original's MenuLevel.nmo makes it (docs/animation.md): its objects, its
// camera (Cam_MenuLevel) carried round a closed TCB curve (I_MenuLevel_Curve) once every 44 seconds by Position On
// Curve and looking at its target, the four flames of its start, and the stone ball rolling its recorded way (Record
// Anim, played by Ball_Stone Script's Play Animation 3D Entity once every 59.246 seconds, looping).
// Reason for the allow: the menu scene comes from nmo::scene values already bounds-checked by the nmo crate; the curve maths below work on its validated point lists.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use crate::render::Instance;
use glam::{Mat4, Vec3};

/// One round of the curve (MenuLevel_Init's Bezier Progression).
pub const ROUND_MS: f32 = 44_000.0;
/// One round of the stone ball's recorded roll (Ball_Stone Script's Play Animation 3D Entity: Duration 59246 ms).
pub const ROLL_MS: f64 = 59_246.0;
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
    /// The recorded animations and the scene objects they move.
    anims: Vec<(usize, Anim)>,
    /// The scene's own sky (the own scene's `Sky_*` objects, drawn round the camera); none: the level's sky.
    sky: Vec<usize>,
}

/// The original's lens: 0.95 across its 4:3 view (MenuLevel.nmo's Cam_MenuLevel), as the vertical field of view.
fn lens(fov: f32) -> f32 {
    2.0 * ((fov.clamp(0.05, 3.0) / 2.0).tan() * 0.75).atan()
}
const ORIGINAL_FOV: f32 = 0.95;
/// The tension of the original's camera curve at each of its points (docs/animation.md, 3.3).
const CURVE_TENSION: f32 = -0.6;

/// An object animation's linear keys (docs/animation.md 5.3): position, rotation (x, y, z, w) and scale by frame.
#[derive(Debug, Clone)]
struct Anim {
    length: f32,
    position: Vec<(f32, Vec3)>,
    rotation: Vec<(f32, glam::Vec4)>,
    scale: Vec<(f32, Vec3)>,
}

/// The keys around `t`: the segment's two values and how far `t` is into it (the first or last key outside them).
fn segment<T: Copy>(keys: &[(f32, T)], t: f32) -> Option<(T, T, f32)> {
    let (first, last) = (keys.first()?, keys.last()?);
    if t <= first.0 {
        return Some((first.1, first.1, 0.0));
    }
    if t >= last.0 {
        return Some((last.1, last.1, 0.0));
    }
    let hi = keys.partition_point(|k| k.0 < t).max(1);
    let (a, b) = (keys.get(hi - 1)?, keys.get(hi)?);
    let span = b.0 - a.0;
    Some((a.1, b.1, if span > 0.0 { (t - a.0) / span } else { 0.0 }))
}

/// VxMath's Slerp: the shorter way, linear when the two are close; not normalized.
fn slerp(q0: glam::Vec4, q1: glam::Vec4, f: f32) -> glam::Vec4 {
    let d = q0.dot(q1);
    let (d, sign) = if d < 0.0 { (-d, -1.0) } else { (d, 1.0) };
    let (k0, k1) = if 1.0 - d > 0.01 {
        let th = d.min(1.0).acos();
        (((1.0 - f) * th).sin() / th.sin(), (f * th).sin() / th.sin())
    } else {
        (1.0 - f, f)
    };
    q0 * k0 + q1 * (k1 * sign)
}

impl Anim {
    /// The object's world matrix at `step` (0 to 1) of the animation: the rotation's rows (VxQuaternion::ToMatrix)
    /// are the object's axes, each scaled, the position its fourth column.
    fn matrix(&self, step: f32) -> Option<Mat4> {
        let t = step * self.length;
        let lerp = |k: &[(f32, Vec3)]| segment(k, t).map(|(a, b, f)| a + (b - a) * f);
        let pos = lerp(&self.position)?;
        let scale = lerp(&self.scale).unwrap_or(Vec3::ONE);
        let q = segment(&self.rotation, t).map_or(glam::Vec4::W, |(a, b, f)| slerp(a, b, f));
        let n = q.length_squared();
        let s = if n > 0.0 { 2.0 / n } else { 0.0 };
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        let ax = Vec3::new(
            1.0 - s * (y * y + z * z),
            s * (x * y - w * z),
            s * (x * z + w * y),
        ) * scale.x;
        let ay = Vec3::new(
            s * (x * y + w * z),
            1.0 - s * (x * x + z * z),
            s * (y * z - w * x),
        ) * scale.y;
        let az = Vec3::new(
            s * (x * z - w * y),
            s * (y * z + w * x),
            1.0 - s * (x * x + y * y),
        ) * scale.z;
        Some(Mat4::from_cols(
            ax.extend(0.0),
            ay.extend(0.0),
            az.extend(0.0),
            pos.extend(1.0),
        ))
    }
}

impl MenuScene {
    /// MenuLevel.nmo from the player's copy, if it is there and has its camera and curve.
    pub fn load(data: &crate::assets::Data) -> Option<MenuScene> {
        let path = data.file("3D Entities/MenuLevel.nmo")?;
        let bytes = gamedata::assets::read_game_file(&path)?;
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
        // each animation with its object in the scene, found by name
        let anims = doc
            .of_class(nmo::class::OBJECT_ANIMATION)
            .filter_map(|(_, o)| {
                nmo::scene::object_animation(o.chunk.as_ref()?)
                    .ok()
                    .flatten()
            })
            .filter_map(|a| {
                let name = doc.get(a.entity?)?.name_lossy();
                let at = level.objects.iter().position(|o| o.name == name)?;
                let anim = Anim {
                    length: a.length,
                    position: a
                        .position
                        .into_iter()
                        .map(|(t, v)| (t, Vec3::from(v)))
                        .collect(),
                    rotation: a
                        .rotation
                        .into_iter()
                        .map(|(t, q)| (t, glam::Vec4::from(q)))
                        .collect(),
                    scale: a
                        .scale
                        .into_iter()
                        .map(|(t, v)| (t, Vec3::from(v)))
                        .collect(),
                };
                Some((at, anim))
            })
            .collect();
        let effects = crate::particles::Effects::menu(&level);
        // a sane lens (between about 3 and 170 degrees), a curve of finite points and length
        let fov_y = lens(cam.fov);
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
            anims,
            sky: Vec::new(),
        })
    }

    /// The own interface's scene in the open format (assets/own/3D Entities/MenuLevel, made by
    /// make_menu_scene.py): the camera's closed curve through `I_MenuLevel_Curve_0..`, looking at
    /// `Cam_MenuLevel_Target`, with the original's lens and tension; its own sky in the `Sky_*` objects.
    pub fn load_open(dir: &std::path::Path) -> Option<MenuScene> {
        let level = gamedata::assets::load_open(dir).ok()?.level;
        let at = |name: &str| level.object(name).map(|(_, o)| Vec3::from(o.position()));
        let target = at("Cam_MenuLevel_Target")?;
        let points: Vec<Vec3> = (0..)
            .map_while(|k| at(&format!("I_MenuLevel_Curve_{k}")))
            .take(64)
            .collect();
        if points.len() < 2 || !points.iter().all(|p| p.is_finite()) || !target.is_finite() {
            return None;
        }
        let tcb = vec![[CURVE_TENSION, 0.0, 0.0]; points.len()];
        let sky = level
            .objects
            .iter()
            .filter(|o| o.name.starts_with("Sky_"))
            .filter_map(|o| o.mesh)
            .collect();
        Some(MenuScene {
            curve: Curve::new(points, &tcb, false),
            target,
            fov_y: lens(ORIGINAL_FOV),
            effects: crate::particles::Effects::menu(&level),
            level,
            model: None,
            anims: Vec::new(),
            sky,
        })
    }

    /// What the frame draws `ms` into the menu: the scene's objects and the sky about the camera, the flames, and
    /// the view (the window and the screenshots draw the menu the same way).
    pub fn frame(
        &self,
        ms: f64,
    ) -> (
        Vec<Instance>,
        Vec<crate::particles::Batch>,
        crate::render::View,
    ) {
        let (eye, target) = self.camera((ms % f64::from(ROUND_MS)) as f32);
        let mut inst = self.instances(ms);
        let at_eye = Mat4::from_translation(eye);
        if let Some(model) = self.model.filter(|_| !self.sky.is_empty()) {
            inst.extend(
                self.sky
                    .iter()
                    .map(|&mesh| Instance::plain(model, mesh, at_eye)),
            );
            let particles = self.effects.batches(eye, target);
            let view = crate::render::View {
                eye,
                target,
                fov_y: self.fov_y,
                near: 0.5,
                far: 3000.0,
            };
            return (inst, particles, view);
        }
        for mesh in 0..crate::sky::faces() {
            inst.push(Instance::plain(
                crate::scene::SKY_MODEL,
                mesh,
                Mat4::from_translation(eye),
            ));
        }
        let particles = self.effects.batches(eye, target);
        (
            inst,
            particles,
            crate::render::View {
                eye,
                target,
                fov_y: self.fov_y,
                near: 0.5,
                far: 3000.0,
            },
        )
    }

    /// The camera at `ms` into the menu: where it is and what it looks at.
    pub fn camera(&self, ms: f32) -> (Vec3, Vec3) {
        (self.curve.pos(ms / ROUND_MS), self.target)
    }

    /// The scene's objects to draw (visible, with a material) `ms` into the menu, the animated ones where their
    /// animations have them then. Only Record Anim is played by the original (on the stone ball), so every
    /// animation here takes its duration.
    pub fn instances(&self, ms: f64) -> Vec<Instance> {
        let step = ((ms % ROLL_MS) / ROLL_MS) as f32;
        let Some(model) = self.model else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (i, o) in self.level.objects.iter().enumerate() {
            let Some(mesh) = o.mesh else { continue };
            let moved = self
                .anims
                .iter()
                .find(|(at, _)| *at == i)
                .and_then(|(_, a)| a.matrix(step))
                .filter(|m| m.is_finite());
            let lit = self
                .level
                .meshes
                .get(mesh)
                .is_some_and(|m| m.parts.iter().any(|p| p.material.is_some()));
            let sky = self.sky.contains(&mesh) && o.name.starts_with("Sky_");
            if o.visible && lit && o.sprite.is_none() && !sky {
                let m = moved.unwrap_or_else(|| Mat4::from_cols_array_2d(&o.matrix));
                out.push(Instance::plain(model, mesh, m));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Record Anim's first key puts the stone ball where MenuLevel.nmo saved it (docs/animation.md 5.2), the
    /// rotation's rows read as its axes; its later keys move it. Needs the player's copy (OPENBALLANCE_DATA).
    #[test]
    #[allow(clippy::expect_used)]
    fn the_stone_ball_starts_where_it_was_saved() {
        if std::env::var_os("OPENBALLANCE_DATA").is_none() {
            return;
        }
        let Ok(data) = crate::assets::Data::locate(None) else {
            return;
        };
        let Some(scene) = MenuScene::load(&data) else {
            return;
        };
        let (at, anim) = scene
            .anims
            .iter()
            .find(|(at, _)| scene.level.objects[*at].name == "I_Ball_Stone")
            .expect("Record Anim");
        let saved = Mat4::from_cols_array_2d(&scene.level.objects[*at].matrix);
        let start = anim.matrix(0.0).expect("its keys");
        assert!(start.abs_diff_eq(saved, 1e-3), "{start:?} vs {saved:?}");
        let later = anim.matrix(0.5).expect("its keys");
        assert!(
            later.w_axis.truncate().distance(saved.w_axis.truncate()) > 1.0,
            "it moves"
        );
    }

    /// The own interface's scene (assets/own/3D Entities/MenuLevel, made by make_menu_scene.py) loads as the game
    /// reads it: its camera's closed curve round the target at the original's radius and height, its own sky's
    /// five faces drawn round the camera and not with the scene, the four flames of its start.
    #[test]
    #[allow(clippy::expect_used)]
    fn the_own_menu_scene_loads_with_its_camera_sky_and_flames() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/own/3D Entities/MenuLevel");
        let scene = MenuScene::load_open(&dir).expect("the own menu scene");
        assert_eq!(scene.sky.len(), crate::sky::FACE_COUNT);
        assert!("ABCD".chars().all(|c| {
            scene
                .level
                .object(&format!("FourFlames_Flame_{c}"))
                .is_some()
        }));
        let (eye, target) = scene.camera(0.0);
        assert!(target.length() < 1e-3, "{target}");
        assert!(
            (Vec3::new(eye.x, 0.0, eye.z).length() - 95.0).abs() < 1.0
                && (eye.y - 40.0).abs() < 1e-3,
            "{eye}"
        );
        let mut scene = scene;
        scene.model = Some(9);
        let inst = scene.instances(0.0);
        assert!(inst.len() > 5 && inst.iter().all(|i| !scene.sky.contains(&i.mesh)));
        let (all, _, view) = scene.frame(0.0);
        assert_eq!(all.len(), inst.len() + crate::sky::FACE_COUNT);
        assert!((view.fov_y - lens(ORIGINAL_FOV)).abs() < 1e-6);
    }

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
