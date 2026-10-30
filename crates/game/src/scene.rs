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

pub fn instances(
    level: &Level,
    taken: &[usize],
    balls: &BallMeshes,
    s: &Snapshot,
) -> Vec<Instance> {
    let mut out: Vec<Instance> = level
        .objects
        .iter()
        .enumerate()
        .filter(|(i, o)| o.visible && !taken.contains(i))
        // A mesh without any material is an editor stand-in (the placeholders of start, checkpoints, modules):
        // the original never shows it, it replaces it with the module.
        .filter(|(_, o)| {
            o.mesh
                .and_then(|m| level.meshes.get(m))
                .is_some_and(|m| m.parts.iter().any(|p| p.material.is_some()))
        })
        .filter_map(|(_, o)| {
            o.mesh.map(|mesh| Instance {
                model: LEVEL_MODEL,
                mesh,
                matrix: Mat4::from_cols_array_2d(&o.matrix),
            })
        })
        .collect();
    let mesh = match s.kind {
        BallKind::Wood => balls.wood,
        BallKind::Stone => balls.stone,
        BallKind::Paper => balls.paper,
    };
    for mesh in 0..crate::sky::FACE_COUNT {
        out.push(Instance {
            model: SKY_MODEL,
            mesh,
            matrix: Mat4::from_translation(s.eye),
        });
    }
    if let Some(mesh) = mesh {
        out.push(Instance {
            model: BALLS_MODEL,
            mesh,
            matrix: Mat4::from_rotation_translation(s.rotation, s.ball),
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
