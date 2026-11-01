// What to draw: the level's visible objects and the ball, from the simulation state, interpolated between the
// last two ticks so motion is smooth at any display rate.

use crate::render::{Instance, View};
use crate::sim::{BallKind, Sim};
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
    pub kind: BallKind,
    pub eye: Vec3,
    pub target: Vec3,
}

impl Snapshot {
    pub fn of(sim: &Sim) -> Snapshot {
        Snapshot {
            ball: sim.ball.position,
            rotation: sim.ball.rotation,
            kind: sim.ball.kind,
            eye: sim.camera.position,
            target: sim.camera.target,
        }
    }

    pub fn lerp(a: &Snapshot, b: &Snapshot, t: f32) -> Snapshot {
        Snapshot {
            ball: a.ball.lerp(b.ball, t),
            rotation: a.rotation.slerp(b.rotation, t),
            kind: b.kind,
            eye: a.eye.lerp(b.eye, t),
            target: a.target.lerp(b.target, t),
        }
    }
}

pub struct BallMeshes {
    pub wood: Option<usize>,
    pub stone: Option<usize>,
    pub paper: Option<usize>,
}

impl BallMeshes {
    pub fn find(balls: &Level) -> BallMeshes {
        let mesh = |name: &str| {
            balls
                .meshes
                .iter()
                .position(|m| m.name.eq_ignore_ascii_case(name))
        };
        BallMeshes {
            wood: mesh("Ball_Wood_Mesh"),
            stone: mesh("Ball_Stone_HighRes_Mesh").or_else(|| mesh("Ball_Stone_Mesh")),
            paper: mesh("Ball_Paper_Mesh"),
        }
    }
}

/// Groups the original shows with particles (flames, sparkles), which this engine does not draw yet: their
/// placeholders are shown as translucent markers so the player sees where they are. Temporary, until phase 4.
const MARKERS: [(&str, [f32; 4]); 3] = [
    ("PC_Checkpoints", [1.0, 0.45, 0.85, 0.45]),
    ("PS_Levelstart", [1.0, 0.45, 0.85, 0.35]),
    ("P_Extra_Point", [0.9, 0.9, 1.0, 0.7]),
];

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
}

/// Module models (`3D Entities/PH/<name>.nmo`), uploaded after the fixed models, with their names.
#[derive(Default)]
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

/// A placeholder's module: its object name without the trailing `_NN` (`P_Box_03` → `P_Box`).
fn module_name(object: &str) -> &str {
    match object.rsplit_once('_') {
        Some((head, tail)) if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) => head,
        _ => object,
    }
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
    (!marker).then(|| module_name(&o.name))
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
        let Some(path) = data.file(&format!("3D Entities/PH/{name}.nmo")) else {
            continue;
        };
        if let Ok(m) = crate::assets::load_nmo(&path) {
            modules.models.push(m.level);
            modules.names.push(name.to_owned());
        }
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
        if !o.visible {
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
            });
            continue;
        }
        let Some((index, module)) = module_of(level, i).and_then(|n| modules.get(n)) else {
            continue;
        };
        // The module's textured meshes. Hidden ones are shown by the original's scripts, which we do not run; a box
        // is a body that exists from the start, so its mesh is shown whatever the file says. A part the physics
        // moves is drawn at its body, scaled by its own scale.
        let body = boxes.iter().find(|b| b.0 == i).map(|b| b.1);
        for mo in &module.objects {
            let part = parts.iter().find(|p| p.placeholder == i && p.object == mo.name);
            if !(mo.visible || body.is_some() || part.is_some()) {
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
            });
        }
    }
    out
}

pub fn instances(
    placements: &[Placement],
    taken: &[usize],
    bodies: &[crate::physics::Body],
    balls: &BallMeshes,
    s: &Snapshot,
) -> Vec<Instance> {
    let mut out: Vec<Instance> = placements
        .iter()
        .filter(|p| !taken.contains(&p.object))
        .map(|p| Instance {
            model: p.model,
            mesh: p.mesh,
            matrix: match p.body.and_then(|b| bodies.get(b)) {
                Some(b) => Mat4::from_rotation_translation(b.rot, b.pos) * p.matrix,
                None => p.matrix,
            },
            marker: p.marker,
        })
        .collect();
    for mesh in 0..crate::sky::FACE_COUNT {
        out.push(Instance {
            model: SKY_MODEL,
            mesh,
            matrix: Mat4::from_translation(s.eye),
            marker: None,
        });
    }
    let mesh = match s.kind {
        BallKind::Wood => balls.wood,
        BallKind::Stone => balls.stone,
        BallKind::Paper => balls.paper,
    };
    if let Some(mesh) = mesh {
        out.push(Instance {
            model: BALLS_MODEL,
            mesh,
            matrix: Mat4::from_rotation_translation(s.rotation, s.ball),
            marker: None,
        });
    }
    out
}

pub fn view(s: &Snapshot) -> View {
    View {
        eye: s.eye,
        target: s.target,
        fov_y: 58f32.to_radians(),
    }
}
