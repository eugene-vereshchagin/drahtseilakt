// What to draw: the level's visible objects and the ball, from the simulation state, interpolated between the
// last two ticks so motion is smooth at any display rate.

use crate::render::{Instance, View};
use crate::sim::{BallKind, Sim};
use glam::{Mat4, Quat, Vec3};
use level::Level;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

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
    /// The lens the scripts set: a field of view in degrees and the clipping planes (none: the original's).
    pub fov: Option<f32>,
    pub clip: Option<(f32, f32)>,
}

impl Snapshot {
    pub fn of(sim: &Sim) -> Snapshot {
        let cam = sim.script_camera();
        // a camera object the scripts attached the view to: its place, looking along its own z
        let (eye, target) = match cam.active.and_then(|i| sim.script_object_matrix(i)) {
            Some(m) => (
                m.w_axis.truncate(),
                m.w_axis.truncate() + m.z_axis.truncate(),
            ),
            None => (sim.camera.position, sim.camera.target),
        };
        Snapshot {
            ball: sim.ball.position,
            rotation: sim.ball.rotation,
            kind: sim.drawn_ball(),
            trafo: sim.trafo_anim(),
            eye,
            target,
            time: sim.tick as f32 * crate::sim::TICK,
            fov: cam.fov,
            clip: cam.clip,
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
            fov: b.fov,
            clip: b.clip,
        }
    }
}

pub struct BallMeshes {
    pub wood: Option<usize>,
    pub stone: Option<usize>,
    pub paper: Option<usize>,
    /// Our own sticky ball (`Ball_Sticky_Mesh`): only our Balls part has it, not the original's Balls.nmo.
    pub sticky: Option<usize>,
    /// The boxes of the meshes (wood, stone, paper, sticky), a sphere's ±2 where a mesh is missing.
    pub boxes: [(Vec3, Vec3); crate::sim::BALL_KINDS.len()],
}

/// The tint the sticky ball is drawn with on the wooden ball's mesh when the balls in use (the copy's Balls.nmo) have no
/// sticky one: a tacky red, so it never passes for the wooden ball.
const STICKY_TINT: [f32; 4] = [1.0, 0.35, 0.5, 1.0];

impl BallMeshes {
    pub fn find(balls: &Level) -> BallMeshes {
        let mesh = |name: &str| {
            balls
                .meshes
                .iter()
                .position(|m| m.name.eq_ignore_ascii_case(name))
        };
        let (wood, stone, paper, sticky) = (
            mesh("Ball_Wood_Mesh"),
            mesh("Ball_Stone_HighRes_Mesh").or_else(|| mesh("Ball_Stone_Mesh")),
            mesh("Ball_Paper_Mesh"),
            mesh("Ball_Sticky_Mesh"),
        );
        let bounds = |m: Option<usize>| {
            let sphere = (Vec3::splat(-2.0), Vec3::splat(2.0));
            let Some(m) = m
                .and_then(|i| balls.meshes.get(i))
                .filter(|m| !m.positions.is_empty())
            else {
                return sphere;
            };
            m.positions.iter().fold(
                (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                |(lo, hi), p| (lo.min(Vec3::from(*p)), hi.max(Vec3::from(*p))),
            )
        };
        BallMeshes {
            wood,
            stone,
            paper,
            sticky,
            boxes: [
                bounds(wood),
                bounds(stone),
                bounds(paper),
                bounds(sticky.or(wood)),
            ],
        }
    }

    /// The mesh a ball of `kind` is drawn with, and its tint if any (the sticky ball on the wooden ball's mesh when the
    /// balls in use have no sticky one).
    pub fn of(&self, kind: BallKind) -> Option<(usize, Option<[f32; 4]>)> {
        match kind {
            BallKind::Wood => self.wood.map(|m| (m, None)),
            BallKind::Stone => self.stone.map(|m| (m, None)),
            BallKind::Paper => self.paper.map(|m| (m, None)),
            BallKind::Sticky => self
                .sticky
                .map(|m| (m, None))
                .or_else(|| self.wood.map(|m| (m, Some(STICKY_TINT)))),
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

/// Whether a group is one the original shows with particles, not with a module's meshes.
pub fn is_marker_group(group: &str) -> bool {
    MARKERS.iter().any(|m| m.0.eq_ignore_ascii_case(group))
}

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
    /// A piece of a sack's support (supports.rs): the sets it is drawn in (`supports::PLAIN`, `VARIED`), 0 for
    /// everything else.
    pub support: u8,
}

/// Which supports are drawn (the `render.supports` action of a view mod): none, `supports::PLAIN` or `VARIED`.
static SUPPORTS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn set_supports(set: u8) {
    SUPPORTS.store(set, Ordering::Relaxed);
}

/// The Clouds switch (the Graphics page): the original's `Skylayer ein?`.
static CLOUDS: AtomicBool = AtomicBool::new(true);

pub fn set_clouds(on: bool) {
    CLOUDS.store(on, Ordering::Relaxed);
}

/// The level has a sea of cloud drawn as a volume (clouds.rs), which takes the place of its flat cloud layer.
static VOLUME_CLOUDS: AtomicBool = AtomicBool::new(false);

pub fn set_volume_clouds(on: bool) {
    VOLUME_CLOUDS.store(on, Ordering::Relaxed);
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
    let origin = place.w_axis.truncate()
        + rt * ((s.offset[0] - 1.0) * 0.5)
        + upv * ((s.offset[1] - 1.0) * 0.5);
    Mat4::from_cols(
        rt.extend(0.0),
        upv.extend(0.0),
        z.extend(0.0),
        origin.extend(1.0),
    )
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
            balls: (1..=6)
                .filter_map(|k| sprite(&format!("P_Extra_Point_Ball{k}")))
                .collect(),
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
        let i = self
            .names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name))?;
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

/// The module a level object stands in for, which the original makes in its place. The original's level init goes
/// through the groups named after its modules and makes the module at every member, whatever the member is called,
/// looks like or whether it is hidden (a map made in Blender gives its stand-ins the part's own mesh and materials,
/// or hides them); so a member of a module's group is a stand-in. An object in no such group is one only as the
/// original's levels have them: a visible mesh without any material, named after its module (`P_Box_03`).
pub fn module_of(level: &Level, i: usize) -> Option<&str> {
    let o = level.objects.get(i)?;
    let mesh = o.mesh?;
    if let Some(group) = level.module_group(i) {
        return (!is_marker_group(group)).then_some(group);
    }
    // the finish: the original's init makes the balloon at the member of PE_Levelende, hidden or not (the invisible
    // maps hide every object of the level, the balloon's stand-in too, and still end at the balloon)
    if level
        .groups_of(i)
        .any(|g| g.eq_ignore_ascii_case("PE_Levelende"))
    {
        return Some("PE_Balloon");
    }
    if !o.visible || has_material(level, mesh) {
        return None;
    }
    let marker = level.groups_of(i).any(is_marker_group);
    (!marker).then(|| level::module_name(&o.name))
}

/// Whether a group is one of the game's own moving modules (`P_Modul_` and a number), the ones left solid as their
/// stand-ins when their file is missing. A group of any other name (a mod's `P_Modul_Portal`) is nothing to the
/// original's level init: its members are never made physical.
pub fn is_game_module(group: &str) -> bool {
    group
        .get(..8)
        .filter(|p| p.eq_ignore_ascii_case("P_Modul_"))
        .and_then(|_| group.get(8..))
        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether a level object is level geometry the physics makes a fixed body of (a floor, a rail, a stopper).
fn is_solid_geometry(level: &Level, i: usize) -> bool {
    level.groups_of(i).any(|g| {
        ["Phys_Floors", "Phys_FloorRails", "Phys_FloorStopper"]
            .iter()
            .any(|p| p.eq_ignore_ascii_case(g))
    })
}

/// The colliders' overlay (`--show-colliders`): the floors, rails and stoppers a map hides are drawn as translucent
/// markers, so that what stops the ball unseen can be looked at.
static SHOW_COLLIDERS: AtomicBool = AtomicBool::new(false);

pub fn set_show_colliders(on: bool) {
    SHOW_COLLIDERS.store(on, Ordering::Relaxed);
}

/// The overlay's colours: what the ball runs into, and what only stops boxes and loose balls.
const HIDDEN_FLOOR: [f32; 4] = [1.0, 0.15, 0.1, 0.45];
const HIDDEN_STOPPER: [f32; 4] = [1.0, 0.8, 0.1, 0.3];

/// Loads every module the level's stand-ins name, once each (a missing or unreadable file is left out).
pub fn load_modules(level: &Level, data: &crate::assets::Data) -> Modules {
    let mut modules = Modules::default();
    let mut tried: Vec<String> = Vec::new();
    for i in 0..level.objects.len() {
        let Some(name) = module_of(level, i) else {
            continue;
        };
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
        let matrix = Mat4::from_cols_array_2d(&o.matrix);
        // a stand-in is never drawn itself, hidden or not: its module is (below)
        // (only when its module is there: a stand-in whose module nobody has, a mod's or a missing file's, is all
        // there is to see at its place, and is drawn as the object it is)
        let stand_in = module_of(level, i).is_some_and(|n| modules.get(n).is_some());
        if !o.visible && !cloud && !stand_in {
            // (the colliders' overlay shows a hidden floor, rail or stopper)
            if SHOW_COLLIDERS.load(Ordering::Relaxed) && is_solid_geometry(level, i) {
                let stopper = level.groups_of(i).all(|g| {
                    !g.eq_ignore_ascii_case("Phys_Floors")
                        && !g.eq_ignore_ascii_case("Phys_FloorRails")
                });
                out.push(Placement {
                    model: LEVEL_MODEL,
                    mesh,
                    matrix,
                    marker: Some(if stopper {
                        HIDDEN_STOPPER
                    } else {
                        HIDDEN_FLOOR
                    }),
                    object: i,
                    body: None,
                    spin: None,
                    trafo_shadow: None,
                    sprite: None,
                    cloud: false,
                    support: 0,
                });
            }
            continue;
        }
        // A floor, rail or stopper is drawn even without a material, as the original draws any visible object
        // (plain, with the default material): the ball runs on it, so it must be seen.
        // The same for a module's stand-in left solid because its file is missing (sim.rs, `static_objects`).
        let plain_geometry = !stand_in
            && !has_material(level, mesh)
            && (is_solid_geometry(level, i) || level.module_group(i).is_some_and(is_game_module));
        // (a start's, a checkpoint's or an extra point's stand-in is replaced too, whatever it looks like: below)
        let marked = level.groups_of(i).any(is_marker_group);
        if !stand_in && !marked && (has_material(level, mesh) || plain_geometry) {
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
                support: 0,
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
                support: 0,
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
            let part = parts
                .iter()
                .find(|p| p.placeholder == i && p.object == mo.name);
            let in_all = all.is_some_and(|g| g.members.contains(&k));
            let spin = SPINNING
                .iter()
                .find(|s| s.0.eq_ignore_ascii_case(&mo.name))
                .map(|s| (s.1, s.2));
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
                support: 0,
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
    let Some(g) = level.group("Shadow") else {
        return Vec::new();
    };
    g.members
        .iter()
        .filter_map(|&i| {
            let o = level.objects.get(i).filter(|o| o.visible)?;
            let mesh = o.mesh?;
            let m = Mat4::from_cols_array_2d(&o.matrix);
            let pts = level
                .meshes
                .get(mesh)?
                .positions
                .iter()
                .map(|&p| m.transform_point3(Vec3::from(p)));
            let (min, max) = pts.fold(
                (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                |(a, b), p| (a.min(p), b.max(p)),
            );
            (min.x <= max.x).then_some(ShadowReceiver {
                mesh,
                matrix: m,
                min,
                max,
            })
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
        let over = f.min.x <= ball.x + r
            && f.max.x >= ball.x - r
            && f.min.z <= ball.z + r
            && f.max.z >= ball.z - r;
        if over && f.min.y <= ball.y + r && (ball.y - r) - f.max.y <= 20.0 {
            out.push(Instance {
                shadow: Some([ball.x, ball.z, side]),
                ..Instance::plain(LEVEL_MODEL, f.mesh, f.matrix)
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn instances(
    placements: &[Placement],
    taken: &[usize],
    unseen: &[bool],
    bodies: &[crate::physics::Body],
    balls: &BallMeshes,
    trafo: Option<&crate::trafo::TrafoModel>,
    extras: (&[crate::extra::ExtraView], Option<&ExtraModel>),
    s: &Snapshot,
    pieces: &[(usize, usize, f32)],
    scripted: Option<&crate::entities::Entities>,
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
                    let mut i =
                        Instance::plain(m.model, *mesh, sprite_matrix(&place, sp, s.eye, s.target));
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
                    let mut i =
                        Instance::plain(m.model, *mesh, sprite_matrix(&at, sp, s.eye, s.target));
                    i.unlit = true;
                    out.push(i);
                }
            }
        }
    }
    let hidden = |object: usize| extra_model.is_some() && views.iter().any(|v| v.object == object);
    let clouds = CLOUDS.load(Ordering::Relaxed) && !VOLUME_CLOUDS.load(Ordering::Relaxed);
    let supports = SUPPORTS.load(Ordering::Relaxed);
    out.extend(
        placements
            .iter()
            .filter(|p| p.support == 0 || p.support & supports != 0)
            .filter(|p| {
                !taken.contains(&p.object) && !unseen.get(p.object).copied().unwrap_or(false)
            })
            .filter(|p| !scripted.is_some_and(|e| e.is_hidden(p.object)))
            .filter(|p| !(p.marker.is_some() && hidden(p.object)))
            .filter(|p| clouds || !p.cloud)
            .map(|p| Instance {
                model: p.model,
                mesh: p.mesh,
                matrix: match (p.body.and_then(|b| bodies.get(b)), p.spin, &p.sprite) {
                    (Some(b), _, _) => Mat4::from_rotation_translation(b.rot, b.pos) * p.matrix,
                    (None, Some((axis, rate)), _) => {
                        p.matrix * Mat4::from_axis_angle(axis, rate * s.time)
                    }
                    (None, None, Some(sp)) => {
                        let mode = scripted.and_then(|e| e.sprite_mode(p.object));
                        let sp = level::Sprite {
                            mode: mode.unwrap_or(sp.mode),
                            ..*sp
                        };
                        sprite_matrix(&p.matrix, &sp, s.eye, s.target)
                    }
                    (None, None, None) => {
                        scripted.map_or(p.matrix, |e| e.draw_matrix(p.object, p.matrix))
                    }
                },
                marker: p.marker,
                uv_offset: [0.0; 2],
                unlit: p.sprite.is_some(),
                shadow: None,
                swaps: scripted.map_or_else(Vec::new, |e| e.swaps(p.object).to_vec()),
                // a transformer's shadow fades while its kind's animation plays (Set Diffuse on its material)
                diffuse: match (p.trafo_shadow, s.trafo) {
                    (Some(k), Some((_, age, kind))) if k == kind => {
                        crate::trafo::shadow_alpha(age).map(|a| [1.0, 1.0, 1.0, a])
                    }
                    _ => None,
                },
            }),
    );
    // what scripts copied or made: drawn from the entity table, capped per frame (entities.rs)
    if let Some(e) = scripted {
        for (_, row, world) in e.drawn_extra() {
            if let Some(mesh) = row.mesh {
                out.push(Instance {
                    swaps: row.swaps.clone(),
                    ..Instance::plain(LEVEL_MODEL, mesh, world)
                });
            }
        }
    }
    for mesh in 0..crate::sky::faces() {
        out.push(Instance {
            model: SKY_MODEL,
            mesh,
            matrix: Mat4::from_translation(s.eye),
            marker: None,
            uv_offset: [0.0; 2],
            diffuse: None,
            unlit: false,
            shadow: None,
            swaps: Vec::new(),
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
            swaps: Vec::new(),
        });
    }
    if let Some((mesh, tint)) = s.kind.and_then(|k| balls.of(k)) {
        out.push(Instance {
            model: BALLS_MODEL,
            mesh,
            matrix: Mat4::from_rotation_translation(s.rotation, s.ball),
            marker: None,
            uv_offset: [0.0; 2],
            diffuse: tint,
            unlit: false,
            shadow: None,
            swaps: Vec::new(),
        });
    }
    out
}

/// The ball's view-only looks (mods `ball.deform`, `decoys`; Interference, DeformedWB): the wooden ball's drawn matrix
/// squeezed and stretched at random about its centre (the wood only, never the physics' round ball), and `decoys`
/// translucent copies wandering around it. `t` is the real time in seconds; nothing here touches the run.
pub fn ball_looks(
    inst: &mut Vec<Instance>,
    snap: &Snapshot,
    balls: &BallMeshes,
    t: f32,
    deform: f32,
    decoys: u8,
) {
    let Some((mesh, _)) = snap.kind.and_then(|k| balls.of(k)) else {
        return;
    };
    if deform > 0.0 && snap.kind == Some(BallKind::Wood) {
        if let Some(i) = inst.iter_mut().rev().find(|i| {
            i.mesh == mesh
                && i.model == BALLS_MODEL
                && (i.matrix.w_axis.truncate() - snap.ball).length() < 1e-3
        }) {
            let w =
                |k: f32| (t * (1.3 + k) + k * 5.1).sin() * (t * (0.7 + 0.4 * k) + k * 2.3).cos();
            let s = Vec3::new(
                1.0 + 0.25 * deform * w(0.0),
                1.0 + 0.25 * deform * w(1.0),
                1.0 + 0.25 * deform * w(2.0),
            );
            i.matrix = Mat4::from_translation(snap.ball)
                * Mat4::from_scale(s)
                * Mat4::from_quat(snap.rotation);
        }
    }
    for n in 0..decoys.min(8) {
        let k = f32::from(n);
        let a = t * (0.35 + 0.11 * k) + k * 2.4;
        let r = 4.0 + 0.5 * k;
        let at = snap.ball + Vec3::new(a.cos() * r, (a * 1.7 + k).sin() * 1.5, a.sin() * r);
        inst.push(Instance {
            model: BALLS_MODEL,
            mesh,
            matrix: Mat4::from_rotation_translation(snap.rotation * Quat::from_rotation_y(a), at),
            marker: None,
            uv_offset: [0.0; 2],
            diffuse: Some([1.0, 1.0, 1.0, 0.3]),
            unlit: false,
            shadow: None,
            swaps: Vec::new(),
        });
    }
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
        d => fov_from_degrees(d as f32),
    }
}

/// The vertical field of view for `degrees` across the 4:3 view (the extended mode's convention).
pub fn fov_from_degrees(degrees: f32) -> f32 {
    2.0 * ((degrees.to_radians() / 2.0).tan() * 0.75).atan()
}

/// The inverse of `fov_from_degrees`.
pub fn fov_degrees(fov_y: f32) -> f32 {
    (2.0 * ((fov_y / 2.0).tan() / 0.75).atan()).to_degrees()
}

pub fn view(s: &Snapshot) -> View {
    View {
        eye: s.eye,
        target: s.target,
        fov_y: s.fov.map_or_else(fov_y, fov_from_degrees),
        near: s.clip.map_or(0.5, |c| c.0),
        far: s.clip.map_or(3000.0, |c| c.1),
    }
}
