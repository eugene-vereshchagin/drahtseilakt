// What to draw: the level's visible objects and the ball, from the simulation state, interpolated between the
// last two ticks so motion is smooth at any display rate.

use crate::render::{Instance, View};
use crate::sim::{BallKind, Sim};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use glam::{Mat4, Quat, Vec3};
use level::Level;

pub const LEVEL_MODEL: usize = 0;
pub const BALLS_MODEL: usize = 1;
pub const SKY_MODEL: usize = 2;

/// The state the renderer interpolates.
#[derive(Clone, Copy)]
pub struct Snapshot {
    pub ball: Vec3,
    pub rotation: Quat,
    /// The ball to draw (none for a moment in a transformer).
    pub kind: Option<BallKind>,
    /// A transformer animation playing: the transformer's matrix, the animation's age in ticks and the kind it makes.
    pub trafo: Option<(Mat4, f32, BallKind)>,
    pub eye: Vec3,
    pub target: Vec3,
    /// The run's time in seconds, for the scripts' turning objects.
    pub time: f32,
}

impl Snapshot {
    pub fn of(sim: &Sim) -> Snapshot {
        Snapshot {
            ball: sim.ball.position,
            rotation: sim.ball.rotation,
            kind: sim.drawn_ball(),
            trafo: sim.trafo_anim(),
            eye: sim.camera.position,
            target: sim.camera.target,
            time: sim.tick as f32 * crate::sim::TICK,
        }
    }

    pub fn lerp(a: &Snapshot, b: &Snapshot, t: f32) -> Snapshot {
        Snapshot {
            ball: a.ball.lerp(b.ball, t),
            rotation: a.rotation.slerp(b.rotation, t),
            kind: b.kind,
            trafo: match (a.trafo, b.trafo) {
                (Some((_, x, _)), Some((m, y, k))) => Some((m, x + (y - x) * t, k)),
                (_, b) => b,
            },
            eye: a.eye.lerp(b.eye, t),
            target: a.target.lerp(b.target, t),
            time: a.time + (b.time - a.time) * t,
        }
    }
}

pub struct BallMeshes {
    pub wood: Option<usize>,
    pub stone: Option<usize>,
    pub paper: Option<usize>,
    /// The boxes of the three meshes (wood, stone, paper), a sphere's ±2 where a mesh is missing.
    pub boxes: [(Vec3, Vec3); 3],
}

impl BallMeshes {
    pub fn find(balls: &Level) -> BallMeshes {
        let mesh = |name: &str| {
            balls
                .meshes
                .iter()
                .position(|m| m.name.eq_ignore_ascii_case(name))
        };
        let (wood, stone, paper) = (
            mesh("Ball_Wood_Mesh"),
            mesh("Ball_Stone_HighRes_Mesh").or_else(|| mesh("Ball_Stone_Mesh")),
            mesh("Ball_Paper_Mesh"),
        );
        let bounds = |m: Option<usize>| {
            let sphere = (Vec3::splat(-2.0), Vec3::splat(2.0));
            let Some(m) = m.and_then(|i| balls.meshes.get(i)).filter(|m| !m.positions.is_empty()) else {
                return sphere;
            };
            m.positions.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| {
                (lo.min(Vec3::from(*p)), hi.max(Vec3::from(*p)))
            })
        };
        BallMeshes {
            wood,
            stone,
            paper,
            boxes: [bounds(wood), bounds(stone), bounds(paper)],
        }
    }
}

/// Groups the original shows with particles (flames, sparkles): their placeholders are shown as translucent markers
/// when the copy lacks the module that makes the particles (the app drops them when the flames are drawn).
const MARKERS: [(&str, [f32; 4]); 3] = [
    ("PC_Checkpoints", [1.0, 0.45, 0.85, 0.45]),
    ("PS_Levelstart", [1.0, 0.45, 0.85, 0.35]),
    ("P_Extra_Point", [0.9, 0.9, 1.0, 0.7]),
];

/// Modules whose own systems draw what they show (the extra points' balls, the finish's balloon or UFO), so their
/// `All_` group is not shown as a whole.
const OWN_DRAWING: [&str; 2] = ["P_Extra_Point", "PE_Balloon"];

/// One drawn thing of the level: a level mesh, a module mesh at a placeholder, or a marker.
pub struct Placement {
    pub model: usize,
    pub mesh: usize,
    pub matrix: Mat4,
    pub marker: Option<[f32; 4]>,
    /// The level object it stands for (hidden when the object is taken).
    pub object: usize,
    /// A rigid body of the physics world that moves this placement: `matrix` is then relative to the body.
    pub body: Option<usize>,
    /// A turn its script gives it all the time: about this axis of its own frame, at this many radians a second.
    pub spin: Option<(Vec3, f32)>,
    /// The shadow of a transformer making this kind of ball.
    pub trafo_shadow: Option<BallKind>,
    /// A sprite: `matrix` is then its place, and the quad is turned and sized each frame (`sprite_matrix`).
    pub sprite: Option<level::Sprite>,
    /// The level's cloud layer (its `SkyLayer`), which the Graphics page's Clouds switch hides.
    pub cloud: bool,
}

/// The Clouds switch (the Graphics page): the original's `Skylayer ein?`.
static CLOUDS: AtomicBool = AtomicBool::new(true);

pub fn set_clouds(on: bool) {
    CLOUDS.store(on, Ordering::Relaxed);
}

/// The cloud layer's look. Its texture is grey clouds on black and its material has no blending, yet the original
/// shows clouds over the sky, not a black floor: it is drawn added to what is behind it (black adds nothing), unlit
/// and without hiding what comes after it.
pub fn cloud_material(level: &mut Level) {
    let clouds: Vec<usize> = level
        .objects
        .iter()
        .filter(|o| o.name == "SkyLayer")
        .filter_map(|o| o.mesh)
        .filter_map(|m| level.meshes.get(m))
        .flat_map(|m| m.parts.iter().filter_map(|p| p.material))
        .collect();
    for i in clouds {
        if let Some(m) = level.materials.get_mut(i) {
            m.blend = Some((level::Blend::One, level::Blend::One));
            m.z_write = false;
            m.unlit = true;
        }
    }
}

/// The field of view the player chose, degrees across 4:3 (the extended mode's); 0 for the original's.
static FOV: AtomicU32 = AtomicU32::new(0);

pub fn set_fov(degrees: Option<u32>) {
    FOV.store(degrees.unwrap_or(0), Ordering::Relaxed);
}

/// The matrix that draws a sprite's unit quad (0..1 in x and y) for a view from `eye` toward `target`, as the
/// original's CKSprite3D places it (C reference ck_sprite3d_quad): its axes face the view (mode 0), turn about the
/// world x (1) or y (2), or stay its own (3); the quad spans its size along them, from (offset - 1) / 2 of it.
pub fn sprite_matrix(place: &Mat4, s: &level::Sprite, eye: Vec3, target: Vec3) -> Mat4 {
    let dir = (target - eye).normalize_or(Vec3::Z);
    let right = Vec3::Y.cross(dir).normalize_or(Vec3::X);
    let up = dir.cross(right).normalize_or(Vec3::Y);
    let (x, y, z) = match s.mode {
        0 => (right, up, dir),
        1 => {
            let d = Vec3::new(0.0, -up.z, up.y).normalize_or(Vec3::Z);
            (Vec3::X, Vec3::new(0.0, d.z, -d.y).normalize_or(Vec3::Y), d)
        }
        2 => {
            let r = Vec3::new(dir.z, 0.0, -dir.x).normalize_or(Vec3::X);
            (r, Vec3::Y, Vec3::new(-r.z, 0.0, r.x).normalize_or(Vec3::Z))
        }
        _ => (
            place.x_axis.truncate().normalize_or(Vec3::X),
            place.y_axis.truncate().normalize_or(Vec3::Y),
            place.z_axis.truncate().normalize_or(Vec3::Z),
        ),
    };
    let (rt, upv) = (x * s.size[0], y * s.size[1]);
    let origin = place.w_axis.truncate() + rt * ((s.offset[0] - 1.0) * 0.5) + upv * ((s.offset[1] - 1.0) * 0.5);
    Mat4::from_cols(rt.extend(0.0), upv.extend(0.0), z.extend(0.0), origin.extend(1.0))
}

/// The extra points' meshes (the P_Extra_Point module): the centre sprite, the six small balls' sprites and the floor
/// glow, with the floor's matrix in the module.
pub struct ExtraModel {
    pub model: usize,
    center: Option<(usize, level::Sprite)>,
    balls: Vec<(usize, level::Sprite)>,
    floor: Option<(usize, Mat4)>,
}

impl ExtraModel {
    /// From the loaded modules, when they have the extra points' module.
    pub fn new(modules: &Modules) -> Option<ExtraModel> {
        let (index, file) = modules.get("P_Extra_Point")?;
        let sprite = |name: &str| {
            let o = file.objects.iter().find(|o| o.name == name)?;
            Some((o.mesh?, o.sprite?))
        };
        let floor = file
            .objects
            .iter()
            .find(|o| o.name == "P_Extra_Point_Floor")
            .and_then(|o| Some((o.mesh?, Mat4::from_cols_array_2d(&o.matrix))));
        Some(ExtraModel {
            model: FIRST_MODULE_MODEL + index,
            center: sprite("P_Extra_Point_Ball0"),
            balls: (1..=6).filter_map(|k| sprite(&format!("P_Extra_Point_Ball{k}"))).collect(),
            floor,
        })
    }
}

/// Module objects a script turns, and how: module 18's rotor turns about its own y at -15 radians a second (its
/// Particle Script: Rotate by Per Second(-15)).
const SPINNING: [(&str, Vec3, f32); 1] = [("P_Modul_18_Rotor", Vec3::Y, -15.0)];

/// Module models (`3D Entities/PH/<name>.nmo`), uploaded after the fixed models, with their names.
#[derive(Default, Clone)]
pub struct Modules {
    pub models: Vec<Level>,
    pub names: Vec<String>,
}

impl Modules {
    /// The model of a module by its name (`P_Modul_34`).
    pub fn get(&self, name: &str) -> Option<(usize, &Level)> {
        let i = self.names.iter().position(|n| n.eq_ignore_ascii_case(name))?;
        Some((i, self.models.get(i)?))
    }
}

/// A module part that the physics moves: the placeholder's level object, the part's object in the module file, and
/// its body.
#[derive(Debug, Clone)]
pub struct ModuleBody {
    pub placeholder: usize,
    pub object: String,
    pub body: usize,
    /// The part's scale, which the body's pose leaves out.
    pub scale: Vec3,
}

pub const FIRST_MODULE_MODEL: usize = 3;

fn has_material(level: &Level, mesh: usize) -> bool {
    level
        .meshes
        .get(mesh)
        .is_some_and(|m| m.parts.iter().any(|p| p.material.is_some()))
}

/// The module a level object stands in for: a visible mesh without any material and without a marker group is an
/// editor stand-in, which the original replaces with its module (named by the object without its `_NN`).
pub fn module_of(level: &Level, i: usize) -> Option<&str> {
    let o = level.objects.get(i)?;
    let mesh = o.mesh?;
    if !o.visible || has_material(level, mesh) {
        return None;
    }
    let marker = level
        .groups_of(i)
        .any(|g| MARKERS.iter().any(|m| m.0.eq_ignore_ascii_case(g)));
    (!marker).then(|| level::module_name(&o.name))
}

/// Loads every module the level's stand-ins name, once each (a missing or unreadable file is left out).
pub fn load_modules(level: &Level, data: &crate::assets::Data) -> Modules {
    let mut modules = Modules::default();
    let mut tried: Vec<String> = Vec::new();
    for i in 0..level.objects.len() {
        let Some(name) = module_of(level, i) else { continue };
        if tried.iter().any(|t| t == name) {
            continue;
        }
        tried.push(name.to_owned());
        if let Some(m) = data.part(&format!("3D Entities/PH/{name}.nmo")) {
            modules.models.push(m.level);
            modules.names.push(name.to_owned());
        }
    }
    // the extra points' module, for where its small balls start (its stand-ins are drawn as markers)
    if level.group("P_Extra_Point").is_some()
        && let Some(m) = data.part("3D Entities/PH/P_Extra_Point.nmo")
    {
        modules.models.push(m.level);
        modules.names.push("P_Extra_Point".to_owned());
    }
    modules
}

pub fn placements(
    level: &Level,
    modules: &Modules,
    boxes: &[(usize, usize)],
    parts: &[ModuleBody],
) -> Vec<Placement> {
    let mut out = Vec::new();
    for (i, o) in level.objects.iter().enumerate() {
        let Some(mesh) = o.mesh else { continue };
        // the cloud layer is hidden in the file: Gameplay.nmo shows it when the options have clouds (instances())
        let cloud = o.name == "SkyLayer";
        if !o.visible && !cloud {
            continue;
        }
        let matrix = Mat4::from_cols_array_2d(&o.matrix);
        if has_material(level, mesh) {
            out.push(Placement {
                model: LEVEL_MODEL,
                mesh,
                matrix,
                marker: None,
                object: i,
                body: None,
                spin: None,
                trafo_shadow: None,
                sprite: None,
                cloud,
            });
            continue;
        }
        // A mesh without any material is an editor stand-in: the original replaces it with its module.
        if let Some(color) = level.groups_of(i).find_map(|g| {
            MARKERS
                .iter()
                .find(|m| m.0.eq_ignore_ascii_case(g))
                .map(|m| m.1)
        }) {
            out.push(Placement {
                model: LEVEL_MODEL,
                mesh,
                matrix,
                marker: Some(color),
                object: i,
                body: None,
                spin: None,
                trafo_shadow: None,
                sprite: None,
                cloud: false,
            });
            continue;
        }
        let Some((index, module)) = module_of(level, i).and_then(|n| modules.get(n)) else {
            continue;
        };
        // The module's textured meshes. A module file hides everything; the original shows the module's `All_<name>`
        // group when its sector comes in (CKGroup's Show shows each member), so its members are drawn, apart from
        // modules whose own systems draw them. A box is a body that exists from the start, so its mesh is shown
        // whatever the file says. A part the physics moves is drawn at its body, scaled by its own scale.
        let body = boxes.iter().find(|b| b.0 == i).map(|b| b.1);
        let name = &modules.names[index];
        let all = (!OWN_DRAWING.iter().any(|m| m.eq_ignore_ascii_case(name)))
            .then(|| module.group(&format!("All_{name}")))
            .flatten();
        for (k, mo) in module.objects.iter().enumerate() {
            let part = parts.iter().find(|p| p.placeholder == i && p.object == mo.name);
            let in_all = all.is_some_and(|g| g.members.contains(&k));
            let spin = SPINNING.iter().find(|s| s.0.eq_ignore_ascii_case(&mo.name)).map(|s| (s.1, s.2));
            if !(mo.visible || in_all || body.is_some() || part.is_some()) {
                continue;
            }
            let Some(mm) = mo.mesh.filter(|&m| has_material(module, m)) else {
                continue;
            };
            // A body moves its module: the matrix is then relative to the body, which starts at the placeholder.
            let local = Mat4::from_cols_array_2d(&mo.matrix);
            let (matrix, body) = match part {
                Some(p) => (Mat4::from_scale(p.scale), Some(p.body)),
                None if body.is_some() => (local, body),
                None => (matrix * local, None),
            };
            out.push(Placement {
                model: FIRST_MODULE_MODEL + index,
                mesh: mm,
                matrix,
                marker: None,
                object: i,
                body,
                spin,
                trafo_shadow: modules.names[index]
                    .strip_prefix("P_Trafo_")
                    .filter(|_| mo.name.ends_with("_Shadow"))
                    .and_then(BallKind::from_name),
                sprite: mo.sprite,
                cloud: false,
            });
        }
    }
    out
}

/// A floor that takes the ball's shadow (the level's `Shadow` group): its mesh, placement and world box.
pub struct ShadowReceiver {
    pub mesh: usize,
    pub matrix: Mat4,
    pub min: Vec3,
    pub max: Vec3,
}

/// The level's shadow receivers: the visible members of its `Shadow` group with a mesh.
pub fn shadow_receivers(level: &Level) -> Vec<ShadowReceiver> {
    let Some(g) = level.group("Shadow") else { return Vec::new() };
    g.members
        .iter()
        .filter_map(|&i| {
            let o = level.objects.get(i).filter(|o| o.visible)?;
            let mesh = o.mesh?;
            let m = Mat4::from_cols_array_2d(&o.matrix);
            let pts = level.meshes.get(mesh)?.positions.iter().map(|&p| m.transform_point3(Vec3::from(p)));
            let (min, max) = pts.fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(a, b), p| (a.min(p), b.max(p)));
            (min.x <= max.x).then_some(ShadowReceiver { mesh, matrix: m, min, max })
        })
        .collect()
}

/// The ball's blob shadow (TT Simple Shadow, Ball_Shadow: HardShadow, size scale 1.3, maximum height 20): a pass
/// over every receiver under the ball (its box meets the ball's in x and z, its top at most 20 below the ball's
/// bottom), a square of the ball's width times 1.3 centred under it.
pub fn shadows(receivers: &[ShadowReceiver], ball: Vec3, out: &mut Vec<Instance>) {
    let r = crate::sim::BALL_RADIUS;
    let side = 2.0 * r * 1.3;
    for f in receivers {
        let over = f.min.x <= ball.x + r && f.max.x >= ball.x - r && f.min.z <= ball.z + r && f.max.z >= ball.z - r;
        if over && f.min.y <= ball.y + r && (ball.y - r) - f.max.y <= 20.0 {
            out.push(Instance { shadow: Some([ball.x, ball.z, side]), ..Instance::plain(LEVEL_MODEL, f.mesh, f.matrix) });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn instances(
    placements: &[Placement],
    taken: &[usize],
    bodies: &[crate::physics::Body],
    balls: &BallMeshes,
    trafo: Option<&crate::trafo::TrafoModel>,
    extras: (&[crate::extra::ExtraView], Option<&ExtraModel>),
    s: &Snapshot,
    pieces: &[(usize, usize, f32)],
) -> Vec<Instance> {
    let (views, extra_model) = extras;
    let mut out: Vec<Instance> = Vec::new();
    // The extra points with their module: the centre sprite and the floor glow until one fires, its small balls
    // while they fly; their stand-ins' markers are then not drawn.
    if let Some(m) = extra_model {
        for v in views {
            let place = Mat4::from_rotation_translation(v.rot, v.center);
            if !v.fired {
                if let Some((mesh, sp)) = &m.center {
                    let mut i = Instance::plain(m.model, *mesh, sprite_matrix(&place, sp, s.eye, s.target));
                    i.unlit = true;
                    out.push(i);
                }
                if let Some((mesh, local)) = &m.floor
                    && !v.flying
                {
                    out.push(Instance::plain(m.model, *mesh, place * *local));
                }
            }
            for &(k, pos) in &v.balls {
                if let Some((mesh, sp)) = m.balls.get(k) {
                    let at = Mat4::from_translation(pos);
                    let mut i = Instance::plain(m.model, *mesh, sprite_matrix(&at, sp, s.eye, s.target));
                    i.unlit = true;
                    out.push(i);
                }
            }
        }
    }
    let hidden = |object: usize| extra_model.is_some() && views.iter().any(|v| v.object == object);
    let clouds = CLOUDS.load(Ordering::Relaxed);
    out.extend(placements
        .iter()
        .filter(|p| !taken.contains(&p.object) && !(p.marker.is_some() && hidden(p.object)))
        .filter(|p| clouds || !p.cloud)
        .map(|p| Instance {
            model: p.model,
            mesh: p.mesh,
            matrix: match (p.body.and_then(|b| bodies.get(b)), p.spin, &p.sprite) {
                (Some(b), _, _) => Mat4::from_rotation_translation(b.rot, b.pos) * p.matrix,
                (None, Some((axis, rate)), _) => p.matrix * Mat4::from_axis_angle(axis, rate * s.time),
                (None, None, Some(sp)) => sprite_matrix(&p.matrix, sp, s.eye, s.target),
                (None, None, None) => p.matrix,
            },
            marker: p.marker,
            uv_offset: [0.0; 2],
            unlit: p.sprite.is_some(),
            shadow: None,
            // a transformer's shadow fades while its kind's animation plays (Set Diffuse on its material)
            diffuse: match (p.trafo_shadow, s.trafo) {
                (Some(k), Some((_, age, kind))) if k == kind => crate::trafo::shadow_alpha(age).map(|a| [1.0, 1.0, 1.0, a]),
                _ => None,
            },
        })
        );
    for mesh in 0..crate::sky::FACE_COUNT {
        out.push(Instance {
            model: SKY_MODEL,
            mesh,
            matrix: Mat4::from_translation(s.eye),
            marker: None,
            uv_offset: [0.0; 2],
            diffuse: None,
            unlit: false,
            shadow: None,
        });
    }
    if let (Some(model), Some((at, age, _))) = (trafo, s.trafo) {
        model.instances(&at, age, &mut out);
    }
    // the burst balls' pieces at their bodies, fading after the ball is lost
    for &(mesh, body, alpha) in pieces {
        let Some(b) = bodies.get(body) else { continue };
        out.push(Instance {
            model: BALLS_MODEL,
            mesh,
            matrix: Mat4::from_rotation_translation(b.rot, b.pos),
            marker: None,
            uv_offset: [0.0; 2],
            diffuse: (alpha < 1.0).then_some([1.0, 1.0, 1.0, alpha]),
            unlit: false,
            shadow: None,
        });
    }
    let mesh = match s.kind {
        Some(BallKind::Wood) => balls.wood,
        Some(BallKind::Stone) => balls.stone,
        Some(BallKind::Paper) => balls.paper,
        None => None,
    };
    if let Some(mesh) = mesh {
        out.push(Instance {
            model: BALLS_MODEL,
            mesh,
            matrix: Mat4::from_rotation_translation(s.rotation, s.ball),
            marker: None,
            uv_offset: [0.0; 2],
            diffuse: None,
            unlit: false,
            shadow: None,
        });
    }
    out
}

/// The in-game camera's field of view: InGameCam's (Camera.nmo), 1.0123 radians across its 4:3 view, so
/// vertically 2·atan(tan(1.0123 / 2) · 3/4) ≈ 45°; a wider window sees more at the sides.
pub fn original_fov_y() -> f32 {
    2.0 * ((1.012_291_f32 / 2.0).tan() * 0.75).atan()
}

/// The field of view drawn: the player's choice in the extended mode, else the original's.
pub fn fov_y() -> f32 {
    match FOV.load(Ordering::Relaxed) {
        0 => original_fov_y(),
        d => 2.0 * (((d as f32).to_radians() / 2.0).tan() * 0.75).atan(),
    }
}

pub fn view(s: &Snapshot) -> View {
    View {
        eye: s.eye,
        target: s.target,
        fov_y: fov_y(),
    }
}
