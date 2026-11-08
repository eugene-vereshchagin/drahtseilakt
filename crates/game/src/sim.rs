// The simulation: ball, camera and game rules, advanced in fixed 60 Hz ticks from an input bit set. It knows
// nothing of windows or rendering, so a replay (the inputs of every tick) reproduces a run exactly, headless
// or on screen.
//
// The physics is IVP's (physics.rs); the rules around it (pushing, transformers, falling off, respawning) follow
// what the C game does, measured in its C reference build.

use crate::collide::{StaticMesh, Triangle};
use crate::proximity::{ACROSS, ALL_AXES, Proximity, Range};
use crate::physics::{GROUP_BALL, GROUP_FLOOR, Material, Rot, World};
use crate::scene::{ModuleBody, Modules, module_of};
use crate::volume::Obb;
use glam::{Mat3, Mat4, Quat, Vec3};
use level::Level;

pub const TICK: f32 = 1.0 / 60.0;
pub const BALL_RADIUS: f32 = 2.0;

/// Input of one tick, as bits (this is what replays store).
pub mod input {
    pub const UP: u8 = 1;
    pub const DOWN: u8 = 2;
    pub const LEFT: u8 = 4;
    pub const RIGHT: u8 = 8;
    /// Held with LEFT / RIGHT: turn the camera instead of the ball.
    pub const SHIFT: u8 = 16;
    /// Lift the camera for an overview.
    pub const LIFT: u8 = 32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BallKind {
    Wood,
    Stone,
    Paper,
}

impl BallKind {
    /// The original game's balls (its `Physicalize_GameBall` table, see docs/physics.md, section 3): the control
    /// force, mass, friction, elasticity and linear damping per second of the ball's material.
    fn params(self) -> Params {
        match self {
            BallKind::Wood => Params {
                force: 0.43,
                mass: 1.9,
                friction: 0.8,
                elasticity: 0.2,
                lin_damp: 0.9,
            },
            BallKind::Stone => Params {
                force: 0.92,
                mass: 10.0,
                friction: 0.5,
                elasticity: 0.1,
                lin_damp: 0.3,
            },
            BallKind::Paper => Params {
                force: 0.065,
                mass: 0.2,
                friction: 0.5,
                elasticity: 0.4,
                lin_damp: 1.5,
            },
        }
    }

    pub fn from_name(name: &str) -> Option<BallKind> {
        let n = name.to_ascii_lowercase();
        if n.contains("wood") {
            Some(BallKind::Wood)
        } else if n.contains("stone") {
            Some(BallKind::Stone)
        } else if n.contains("paper") {
            Some(BallKind::Paper)
        } else {
            None
        }
    }
}

/// Per-kind tuning (see `BallKind::params`).
#[derive(Clone, Copy)]
struct Params {
    force: f32,
    mass: f32,
    friction: f32,
    elasticity: f32,
    lin_damp: f32,
}

/// The camera's heading at the start and after a respawn, in quarter turns (see `Sim::new`).
const START_QUARTER: i32 = 1;

/// Ticks the ball spends out of the physics in a transformer: the C game removes the old ball's body three frames
/// after the transformer is under it and makes the new one 149 frames later.
const TRAFO_TICKS: u64 = 149;
/// Where a transformer holds the ball: its own (0, 3, 0), where the C game makes the new ball.
const TRAFO_HOLD: Vec3 = Vec3::new(0.0, 3.0, 0.0);
/// How fast the held ball is pulled to the hold point: a critically damped spring of this rate (per tick), as the C
/// game's ball moves (half way in 7 frames, 90% in 17).
const TRAFO_PULL: f32 = 0.23;
/// After a checkpoint, the C game takes the passed sector's objects out of the physics 44 frames later and makes the
/// next sector's 4 frames after that.
const SECTOR_END_TICKS: u64 = 44;
const SECTOR_BEGIN_TICKS: u64 = 48;
/// After a lost ball, the current sector's objects leave the physics one frame after the ball and come back at their
/// places four frames later.
const DEATH_SECTOR_END_TICKS: u64 = 62;
const DEATH_SECTOR_BEGIN_TICKS: u64 = 66;

/// After the ball's box reaches a depth-test cube, the C game stops the push at once, takes the falling ball out of the
/// physics 61 frames later (a life goes then), shows the new ball at the reset point 10 frames after that, and makes
/// it a body that rolls and answers the keys 245 frames after the loss (C game, level 2: lost at 331, the new body
/// falling from 577).
const DEATH_REMOVE_TICKS: u64 = 61;
const DEATH_SHOW_TICKS: u64 = 71;
const DEATH_RESPAWN_TICKS: u64 = 245;
/// The points counter waits while a ball is lost, and goes on this many ticks after the new ball, where it was.
const POINTS_RESUME_TICKS: u64 = 5;

/// The rotational damping of the original's balls (docs/physics.md, section 3).
const BALL_ROT_DAMP: f32 = 0.1;

#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    /// World angular velocity: the ball's spin, which friction with the floor changes.
    pub spin: Vec3,
    pub kind: BallKind,
    pub on_ground: bool,
}

impl Ball {
    /// The ball's material: the original's table for its kind.
    fn material(&self) -> Material {
        let p = self.kind.params();
        Material {
            friction: p.friction,
            elasticity: p.elasticity,
            mass: p.mass,
            lin_damp: p.lin_damp,
            rot_damp: BALL_ROT_DAMP,
        }
    }

}

/// The game camera, as the original's rig moves it (Camera.nmo's frames, Gameplay.nmo's two TT Set Dynamic Position
/// blocks, measured against the C game to 0.0015): the look-at point (Cam_Target) springs to the ball, 10 a second;
/// InGameCam springs to Cam_Pos, 22 behind the look-at point and 35 above it, with a force of 5 across and 0.8 up
/// (2 while Space lifts its goal 50 higher) and a damping of 0.5 across and 0.3 up. A view turn swings Cam_Pos round
/// a quarter in 250 ms, at an even rate.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Target heading in quarter turns, and the heading the view has turned to (radians).
    pub quarter: i32,
    pub yaw: f32,
    /// Space held: 1, else 0.
    pub lift: f32,
    pub position: Vec3,
    /// InGameCam's move in the last tick (the spring's memory of its speed).
    pub step: Vec3,
    pub target: Vec3,
}

/// The rig's values: Cam_Pos about Cam_Target, Cam_Target's force, InGameCam's forces and dampings (x, y, z), the
/// lift's goal and force up, and the view turn's time.
const CAM_BACK: f32 = 22.0;
const CAM_UP: f32 = 35.0;
const CAM_TARGET_FORCE: f32 = 10.0;
const CAM_FORCE: Vec3 = Vec3::new(5.0, 0.8, 5.0);
const CAM_DAMP: Vec3 = Vec3::new(0.5, 0.3, 0.5);
const CAM_LIFT: f32 = 50.0;
const CAM_LIFT_FORCE: f32 = 2.0;
const CAM_TURN_TICKS: f32 = 15.0;

impl Camera {
    /// Forward on the ground plane: where "up" pushes the ball.
    pub fn forward(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos())
    }
    /// The world axis "up" pushes along: the camera's heading in whole quarters, at once, even while the view is
    /// still turning (the original's four navigation forces are along world axes).
    pub fn push_forward(&self) -> Vec3 {
        match self.quarter.rem_euclid(4) {
            0 => Vec3::Z,
            1 => Vec3::X,
            2 => Vec3::NEG_Z,
            _ => Vec3::NEG_X,
        }
    }

    /// The world axis "right" pushes along: `push_forward` turned a quarter clockwise seen from above (left-handed,
    /// Y up).
    pub fn push_right(&self) -> Vec3 {
        let f = self.push_forward();
        Vec3::new(f.z, 0.0, -f.x)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Checkpoint(usize),
    ExtraPoint,
    ExtraLife,
    Transform(BallKind),
    Died,
    GameOver,
    Finished,
    /// Small balls of an extra point reaching the ball (each 20 points).
    ExtraHits(u32),
    /// A new ball after a lost one.
    NewBall,
    /// A hinge a heavy ball breaks (module 29's plate bridge): its script plays Misc_RopeTears.
    RopeTears,
}

/// What the rules need from the level, resolved once.
#[derive(Clone)]
struct Rules {
    start: Vec3,
    checkpoints: Vec<(usize, Vec3)>,
    /// The checkpoints in the order their group lists them, as replays of formats 1 to 3 were recorded (sim.rs
    /// `checkpoints_by_name`).
    checkpoints_listed: Vec<(usize, Vec3)>,
    resets: Vec<Vec3>,
    /// Each reset point's turn (its matrix without scale): a new ball takes its reset point's whole matrix.
    reset_turns: Vec<Mat3>,
    /// The reset points and their turns in plain byte order of their names, as replays of formats 1 to 3 were
    /// recorded (`checkpoints_by_name`; the same order as `resets` for every original level).
    resets_listed: Vec<Vec3>,
    reset_turns_listed: Vec<Mat3>,
    end: Option<Vec3>,
    extra_points: Vec<(usize, Vec3)>,
    extra_lives: Vec<(usize, Vec3)>,
    /// The transformers: object, position, the kind they make, world matrix (where their animation plays).
    trafos: Vec<(usize, Vec3, BallKind, Mat4)>,
    /// The level's depth-test cubes: the ball has fallen off when its box reaches one.
    depth_cubes: Vec<Obb>,
    /// Without depth-test cubes (a map that has none), the ball has fallen off this far below the lowest floor.
    death_y: f32,
}

/// A placed module's physics: its parts' bodies and its joints, in world terms.
#[derive(Clone, Debug, Default)]
struct ModuleInst {
    parts: Vec<usize>,
    joints: Vec<JointInst>,
    /// The part woken when the ball comes near: its body, the point measured from, and the distance.
    wake: Option<(usize, Vec3, Proximity)>,
    /// The forces made when it is woken.
    forces: Vec<ForceInst>,
    finish: Option<FinishInst>,
    updraft: Option<UpdraftInst>,
    swing: Option<SwingInst>,
    breaks: Option<BreakInst>,
}

/// A hinge a heavy ball breaks, placed (crate::modules::BreakSpec): the part measured to, the proximity, the kind
/// of ball and the joint (an index into the module's joints).
#[derive(Clone, Copy, Debug)]
struct BreakInst {
    near: usize,
    prox: Proximity,
    kind: BallKind,
    joint: usize,
}

/// A break while its module is in the physics: switched on by the module's wake-up; the physics' joint it
/// destroys, made when the module came in.
#[derive(Clone, Debug)]
struct BreakTrigger {
    at: BreakInst,
    joint: Option<usize>,
}

/// A swing placed (crate::modules::SwingSpec): the part, the world direction of its +push and the timing.
#[derive(Clone, Copy, Debug)]
struct SwingInst {
    body: usize,
    force: Vec3,
    phase: u64,
    pauses: bool,
    start: u64,
}

/// A swing while its module is in the physics: the tick it came in and the push now on.
#[derive(Clone, Debug)]
struct SwingRun {
    sector: Option<usize>,
    at: SwingInst,
    since: Option<u64>,
    /// The Sequencer's steps taken (crate::modules::SwingSpec): the step at the last coming in.
    steps: u64,
    /// The push on now: its direction (+1 or -1) and the physics' force.
    push: Option<(i8, usize)>,
}

impl SwingRun {
    fn new(sector: Option<usize>, at: SwingInst, now: bool) -> SwingRun {
        // the level's load took the Sequencer's first step
        let mut w = SwingRun { sector, at, since: None, steps: 1, push: None };
        if now {
            w.enter(0);
        }
        w
    }

    fn enter(&mut self, tick: u64) {
        self.since = Some(tick);
        self.steps += 1;
    }

    /// The push its script wants `age` ticks after the module came in: +1, -1 or none.
    fn wanted(&self, age: u64) -> i8 {
        let Some(t) = age.checked_sub(self.at.start) else { return 0 };
        let phase = t / self.at.phase.max(1);
        if self.at.pauses {
            [1, 0, -1, 0][(phase % 4) as usize]
        } else if (self.steps + phase) % 2 == 1 {
            1
        } else {
            -1
        }
    }

    fn stop(&mut self, world: &mut World, tick: u64) {
        if let Some(since) = self.since.take()
            && !self.at.pauses
        {
            self.steps += tick.saturating_sub(since + self.at.start) / self.at.phase.max(1);
        }
        if let Some((_, f)) = self.push.take() {
            world.remove_force(f);
        }
    }
}

/// An updraft placed (crate::modules::UpdraftSpec): the point the ball is measured to, the box it must reach, and
/// the force.
#[derive(Clone, Copy, Debug)]
struct UpdraftInst {
    point: Vec3,
    volume: Obb,
    value: f32,
}

/// An updraft while its module is in the physics: the script's two proximities (80 to switch it on, 7 to test the
/// boxes) and the force on the ball while it is in the volume.
#[derive(Clone, Debug)]
struct UpdraftTrigger {
    sector: Option<usize>,
    at: UpdraftInst,
    outer: Proximity,
    inner: Proximity,
    force: Option<usize>,
}

impl UpdraftTrigger {
    fn new(sector: Option<usize>, at: UpdraftInst) -> UpdraftTrigger {
        UpdraftTrigger {
            sector,
            at,
            outer: Proximity::new(80.0, ACROSS, 85.0, 100.0, (10, 60)),
            inner: Proximity::new(7.0, ACROSS, 12.0, 20.0, (1, 10)),
            force: None,
        }
    }

    /// Off: no tests and no force.
    fn stop(&mut self, world: &mut World) {
        self.outer.switch_off();
        self.inner.switch_off();
        if let Some(f) = self.force.take() {
            world.remove_force(f);
        }
    }
}

/// A force on a body: a world vector at a world point (crate::modules::ForceSpec, placed).
#[derive(Clone, Copy, Debug)]
struct ForceInst {
    body: usize,
    force: Vec3,
    point: Vec3,
}

/// The finish's trigger of a module (crate::modules::FinishSpec, placed): the part measured to, the distance, the
/// push, and the joints it breaks (indices into the module's joints).
#[derive(Clone, Debug)]
struct FinishInst {
    near: usize,
    prox: Proximity,
    force: ForceInst,
    breaks: Vec<usize>,
}

/// A finish trigger armed while its module is in the physics: the joints it breaks are those made when the module
/// came in.
#[derive(Clone, Debug)]
struct FinishTrigger {
    near: usize,
    prox: Proximity,
    force: ForceInst,
    breaks: Vec<usize>,
    /// The physics' joints to break, made when the module came into the physics.
    joints: Vec<usize>,
}

impl FinishTrigger {
    /// Switched on, with the physics' joints its module just made.
    fn arm(&mut self, made: &[Option<usize>]) {
        self.joints = self.breaks.iter().filter_map(|&k| made.get(k).copied().flatten()).collect();
        self.prox.switch_on();
    }
}

/// A module's wake-up on approach, armed while its module is in the physics (crate::modules::WakeSpec).
#[derive(Clone, Debug)]
struct WakeTrigger {
    sector: Option<usize>,
    body: usize,
    /// The break it switches on when it fires (an index into the breaks).
    then: Option<usize>,
    /// The forces made when it fires.
    forces: Vec<ForceInst>,
    near: Vec3,
    prox: Proximity,
}

/// A module's joint between two game bodies, its points and axes in the world.
#[derive(Clone, Copy, Debug)]
enum JointInst {
    Slider { a: usize, b: usize, p0: Vec3, p1: Vec3, limits: Option<(f32, f32)> },
    Hinge { a: usize, b: usize, anchor: Vec3, axis: Vec3 },
    Ball { a: usize, b: usize, anchor: Vec3 },
    Spring { a: usize, pa: Vec3, b: usize, pb: Vec3, length: f32, constant: f32, lin_damp: f32, glob_damp: f32 },
}

impl ModuleInst {
    /// The parts are made at their places and their joints attached, as the module's scripts do. Returns the physics'
    /// joint of each of the module's joints.
    fn enter(&self, world: &mut World) -> Vec<Option<usize>> {
        for &b in &self.parts {
            world.activate(b);
        }
        let mut made = Vec::new();
        for j in &self.joints {
            made.push(match *j {
                JointInst::Slider { a, b, p0, p1, limits } => world.add_slider(a, b, p0, p1, limits),
                JointInst::Hinge { a, b, anchor, axis } => world.add_hinge(a, b, anchor, axis),
                JointInst::Ball { a, b, anchor } => world.add_ball_joint(a, b, anchor),
                JointInst::Spring { a, pa, b, pb, length, constant, lin_damp, glob_damp } => {
                    world.add_spring(a, pa, b, pb, length, constant, lin_damp, glob_damp)
                }
            });
        }
        made
    }

    /// The parts leave the physics, their joints with them.
    fn leave(&self, world: &mut World) {
        for &b in &self.parts {
            world.deactivate(b);
        }
    }
}

/// The corners of FixCube, a unit cube; it collides with nothing, so only its being a body counts.
const FIXCUBE: [Vec3; 8] = [
    Vec3::new(-1.0, -1.0, -1.0),
    Vec3::new(1.0, -1.0, -1.0),
    Vec3::new(-1.0, 1.0, -1.0),
    Vec3::new(1.0, 1.0, -1.0),
    Vec3::new(-1.0, -1.0, 1.0),
    Vec3::new(1.0, -1.0, 1.0),
    Vec3::new(-1.0, 1.0, 1.0),
    Vec3::new(1.0, 1.0, 1.0),
];

/// The parts of the module placed by level object `i` (its stand-in), as bodies out of the physics until their sector,
/// and the module's joints. Each object of the module is at the stand-in's matrix times its matrix in the module file;
/// a part's collision meshes are scaled by its scale. None when the module file lacks a part or a frame.
fn module_parts(
    world: &mut World,
    level: &Level,
    i: usize,
    spec: &crate::modules::ModuleSpec,
    file: &Level,
    fixcube: Option<usize>,
    bodies: &mut Vec<ModuleBody>,
) -> Option<ModuleInst> {
    use crate::modules::{FIXCUBE, ForceSpec, JointSpec};
    let place = glam::Mat4::from_cols_array_2d(&level.objects.get(i)?.matrix);
    let at = |name: &str| -> Option<glam::Mat4> {
        let o = file.objects.iter().find(|o| o.name == name)?;
        Some(place * glam::Mat4::from_cols_array_2d(&o.matrix))
    };
    let mut inst = ModuleInst::default();
    let mut named: Vec<(&str, usize)> = Vec::new();
    for part in spec.parts {
        let world_m = at(part.object)?;
        let (pos, rot, scale) = pose_and_scale(&world_m.to_cols_array_2d())?;
        let pieces: Vec<Vec<Vec3>> = part
            .meshes
            .iter()
            .filter_map(|m| file.meshes.iter().find(|x| x.name == *m))
            .map(|m| m.positions.iter().map(|p| Vec3::from(*p) * scale).collect())
            .collect();
        let body = world.add_part(
            &pieces,
            part.material,
            pos,
            rot,
            part.fixed,
            part.group,
            part.start_frozen,
            part.collide,
            part.shift,
            false,
        )?;
        inst.parts.push(body);
        named.push((part.object, body));
        bodies.push(ModuleBody {
            placeholder: i,
            object: part.object.to_owned(),
            body,
            scale,
        });
    }
    let body = |n: &str| -> Option<usize> {
        if n == FIXCUBE {
            fixcube
        } else {
            named.iter().find(|p| p.0 == n).map(|p| p.1)
        }
    };
    // a point given in a frame object (the frame's whole matrix, as Virtools' Transform), and a frame's Z axis
    let point = |frame: &str, p: [f32; 3]| at(frame).map(|m| m.transform_point3(Vec3::from(p)));
    if let Some(w) = spec.wake {
        // every module's wake-up measures across (x and z), whatever the height
        let prox = Proximity::new(w.distance, ACROSS, w.exact.0, w.exact.1, w.frames);
        inst.wake = Some((body(w.part)?, at(w.near)?.w_axis.truncate(), prox));
    }
    // A force's direction turned by its frame and normalised (a near-zero one is +x), times its value.
    let force = |f: &ForceSpec| -> Option<ForceInst> {
        let dir = at(f.dir_frame)?.transform_vector3(Vec3::from(f.dir));
        let dir = if dir.length_squared() <= 1e-4 { Vec3::X } else { dir.normalize() };
        Some(ForceInst { body: body(f.part)?, force: dir * f.value, point: point(f.pos_frame, f.pos)? })
    };
    for f in spec.forces {
        inst.forces.push(force(f)?);
    }
    for j in spec.joints {
        inst.joints.push(match *j {
            JointSpec::Slider { a, b, from, to, limits, lo, hi } => JointInst::Slider {
                a: body(a)?,
                b: body(b)?,
                p0: point(from, [0.0; 3])?,
                p1: point(to, [0.0; 3])?,
                limits: limits.then_some((lo, hi)),
            },
            JointSpec::Hinge { a, b, frame } => JointInst::Hinge {
                a: body(a)?,
                b: body(b)?,
                anchor: point(frame, [0.0; 3])?,
                axis: at(frame)?.z_axis.truncate().normalize_or_zero(),
            },
            JointSpec::Ball { a, b, frame, pos } => JointInst::Ball {
                a: body(a)?,
                b: body(b)?,
                anchor: point(frame, pos)?,
            },
            JointSpec::Spring { a, pos_a, frame_a, b, pos_b, frame_b, length, constant, lin_damp, glob_damp } => {
                JointInst::Spring {
                    a: body(a)?,
                    pa: point(frame_a, pos_a)?,
                    b: body(b)?,
                    pb: point(frame_b, pos_b)?,
                    length,
                    constant,
                    lin_damp,
                    glob_damp,
                }
            }
        });
    }
    if let Some(u) = &spec.updraft {
        let o = file.objects.iter().find(|o| o.name == u.volume)?;
        let mesh = file.meshes.get(o.mesh?)?;
        let (lo, hi) = mesh.positions.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| {
            (lo.min(Vec3::from(*p)), hi.max(Vec3::from(*p)))
        });
        inst.updraft = Some(UpdraftInst {
            point: at(u.near)?.w_axis.truncate(),
            volume: Obb::from_box(lo, hi, &at(u.volume)?),
            value: u.value,
        });
    }
    if let Some(w) = &spec.swing {
        let dir = at(w.dir_frame)?.transform_vector3(Vec3::from(w.dir));
        let dir = if dir.length_squared() <= 1e-4 { Vec3::X } else { dir.normalize() };
        inst.swing = Some(SwingInst {
            body: body(w.part)?,
            force: dir * w.value,
            phase: u64::from(w.phase),
            pauses: w.pauses,
            start: u64::from(w.start),
        });
    }
    if let Some(b) = &spec.breaks {
        let joint = spec
            .joints
            .iter()
            .position(|j| matches!(j, JointSpec::Hinge { frame, .. } if *frame == b.frame))?;
        inst.breaks = Some(BreakInst {
            near: body(b.near)?,
            prox: Proximity::new(b.distance, ALL_AXES, b.exact.0, b.exact.1, b.frames),
            kind: BallKind::from_name(b.ball)?,
            joint,
        });
    }
    if let Some(f) = &spec.finish {
        // The joints broken are the hinges at the named frames.
        let breaks = spec
            .joints
            .iter()
            .enumerate()
            .filter(|(_, j)| matches!(j, JointSpec::Hinge { frame, .. } if f.breaks.contains(frame)))
            .map(|(k, _)| k)
            .collect();
        // the finish's proximity: tested every frame within 5, every 10 frames beyond 10
        let prox = Proximity::new(f.distance, ALL_AXES, 5.0, 10.0, (1, 10));
        inst.finish = Some(FinishInst { near: body(f.near)?, prox, force: force(&f.force)?, breaks });
    }
    Some(inst)
}

/// The ball inside a transformer: out of the physics until `release`, then a new ball of its new kind at `at`.
#[derive(Clone, Copy, Debug)]
struct TrafoHold {
    start: u64,
    release: u64,
    at: Vec3,
    /// Where the ball was taken.
    from: Vec3,
    /// The kind it was, drawn until the new ball comes.
    old: BallKind,
}

/// A transformer's animation (see `trafo.rs`): the transformer's matrix and the tick it took the ball.
#[derive(Clone, Copy, Debug)]
pub struct TrafoAnim {
    pub at: Mat4,
    pub start: u64,
    /// The kind of ball it makes.
    pub kind: BallKind,
}

/// The ball's smoke after its lightning (Ball_LightningSphere's Delayer: 2500 ms).
const SMOKE_AFTER_NEW_BALL: u64 = 150;

#[derive(Clone)]
pub struct Sim {
    rules: Rules,
    world: World,
    /// The loose bodies (boxes and loose balls): (level object index, body index in the world).
    loose: Vec<(usize, usize)>,
    /// How deep each loose object (box or loose ball, by level object index) was placed in the level's static
    /// geometry (collide::start_depth), and how far it was lifted out of it (`rest_out_of`); read by the checks of
    /// our levels and the original's.
    #[cfg_attr(not(test), allow(dead_code))]
    pub start_depths: Vec<(usize, f32, f32)>,
    pub ball: Ball,
    pub camera: Camera,
    pub tick: u64,
    pub lives: u32,
    /// The last ball was kept by god-mode's endless lives (the mods' `lives.endless`): the run is helped.
    pub lives_spared: bool,
    pub points: i64,
    pub sector: usize,
    pub finished: bool,
    /// The tick the level was finished at (the run's time; the world goes on after it).
    pub finished_at: u64,
    /// The points finishing the level adds (the level table's LevelBonus).
    pub level_bonus: i64,
    pub game_over: bool,
    /// Objects taken (extra points and lives) are hidden by the renderer.
    pub taken: Vec<usize>,
    /// Level objects out of sight, by index: the members of a sector (groups `Sector_NN`) while it is not in play.
    /// The original shows a sector's objects when it comes in (Activate Sector's Show) and hides them when it goes
    /// (Deactivate Sector's Hide, or its module's script taking down its instance), so a passed sector's swings and
    /// boxes are gone, not hanging still where a ball would fall through them.
    pub unseen: Vec<bool>,
    /// The sector of each level object (`None`: in no sector).
    object_sectors: Vec<Option<usize>>,
    pub events: Vec<(u64, Event)>,
    prev_input: u8,
    /// The ball inside a transformer, if it is in one.
    hold: Option<TrafoHold>,
    /// The transformer animation playing, if one is.
    trafo_anim: Option<TrafoAnim>,
    /// The tick the ball fell off the level, until the new ball is in.
    dying: Option<u64>,
    /// The arrows that pushed at the last tick, to wake the ball when they change.
    push_keys: u8,
    /// The body indices of each sector's objects (sector 1 first).
    sectors: Vec<Vec<usize>>,
    /// Each sector's modules.
    module_sectors: Vec<Vec<ModuleInst>>,
    /// The modules' wake-ups on approach.
    wake_triggers: Vec<WakeTrigger>,
    /// The finish's triggers (the balloon's); the level's end gate counts only when there is none.
    finishes: Vec<FinishTrigger>,
    /// The fans' updrafts.
    updrafts: Vec<UpdraftTrigger>,
    /// The swings' and sacks' pushes.
    swings: Vec<SwingRun>,
    /// The extra points' pickups (TT Extra), when the module file gives their small balls.
    extras: Vec<crate::extra::Extra>,
    /// The hinges heavy balls break.
    breaks: Vec<BreakTrigger>,
    /// The modules' parts that are bodies, for drawing.
    module_bodies: Vec<ModuleBody>,
    /// The box of each kind's ball mesh (wood, stone, paper), for the depth-test cubes.
    ball_boxes: [(Vec3, Vec3); 3],
    /// The balls' pieces (pieces.rs), those lying about now (their kind, piece and body), and for each kind (wood,
    /// stone, paper) the tick they burst and the tick their fade began (20 s on, or the ball was lost).
    pieces: crate::pieces::Pieces,
    flying: Vec<(BallKind, usize, usize)>,
    pieces_burst: [Option<u64>; 3],
    pieces_fade: [Option<u64>; 3],
    /// The pieces go 20 s after their burst (Fadeout Manager): off only to play a replay recorded before it (formats 1
    /// and 2), where they lay until the ball was lost.
    pub pieces_expire: bool,
    /// The old ball bursts in a transformer: off only to play a replay recorded before the bursts (format 1), as
    /// their bodies, however far, change the physics' course.
    pub bursts: bool,
    /// The checkpoints follow their names: off only to play a replay recorded before (formats 1 to 3), when they
    /// followed their group's list, and a level that lists them out of order (11, 12) never opened some sectors.
    pub checkpoints_by_name: bool,
    /// The points counter's ticks counted, and the tick it goes on from after a lost ball.
    points_clock: u64,
    points_resume: u64,
    /// The tick the depth-test cubes' Group Iterator started (with the new ball), and the ball's pose at the end of
    /// the last tick, which is what the original's test sees (its scripts run before its physics in a frame).
    depth_from: u64,
    prev_pose: (Vec3, Quat),
    /// Sector changes due: at a tick, a sector's objects or modules leave or enter the physics.
    sector_changes: Vec<SectorChange>,
    /// The mods' rules on the run's events (modrt.rs): cloned, rewound and replayed with the run.
    pub mods: crate::modrt::SimMods,
    /// What the ball touched at the last tick (for the mods): whether, and whether something steep.
    touching: (bool, bool),
}

/// A change of a sector's bodies due at a tick: its objects or its modules, entering or leaving the physics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SectorChange {
    tick: u64,
    sector: usize,
    enter: bool,
    modules: bool,
}

const PICK_RADIUS: f32 = 3.5;
/// How near a transformer takes the ball.
pub const TRAFO_REACH: f32 = PICK_RADIUS + 1.0;
/// Physics time from the level's load to its first sector's objects: 594 frames at time factor 1 (the C game,
/// level 1).
const LEVEL_INTRO_SECONDS: f64 = 594.0 / 60.0;
/// Physics time between a sector's objects and the ball at a level start (the C game): 180 frames at time factor 2
/// (levels 2 to 12), or four frames at time factor 1 when the tutorial stops the level first (level 1).
const START_OBJECTS_SECONDS: f64 = 180.0 / 30.0;
const START_OBJECTS_SECONDS_TUTORIAL: f64 = 4.0 / 60.0;
const GATE_RADIUS: f32 = 7.0;
/// The points a level starts with (the Energy array's StartPoints), and the counter that takes one every half
/// second (its Timefactor, 500 ms): measured in the C game on level 2, the points go down at ticks 32, 62, 92...
const START_POINTS: i64 = 1000;
/// The points each life left adds to a finished level's score (the Energy array's LifeBonus).
const LIFE_BONUS: i64 = 200;
const POINTS_FROM: u64 = 32;
const POINTS_EVERY: u64 = 30;

/// Ticks from a new ball to the depth-test cubes' first test: fitted to the C game, where a ball leaving level 2 on
/// the same route is lost at tick 331 (the iterator's round is 4 ticks there, so this is known modulo 4).
const DEPTH_PHASE: u64 = 3;

impl Sim {
    /// A run of a level without its modules' physics (their placeholders are solid).
    #[cfg(test)]
    pub fn new(level: &Level, start_ball: BallKind) -> Sim {
        Sim::with_modules(level, start_ball, &Modules::default(), false)
    }

    /// A run of a level, with the physics of the modules the port has (crate::modules) from their loaded files.
    /// `tutorial`: the level starts with the original's tutorial (level 1), which stops the level before the ball.
    #[cfg(test)]
    pub fn with_modules(level: &Level, start_ball: BallKind, modules: &Modules, tutorial: bool) -> Sim {
        Sim::with_rules(level, start_ball, modules, tutorial, crate::replay::VERSION)
    }

    /// A run of a level by the rules of a replay format (crate::replay), so that a replay plays as it was recorded:
    /// format 5 made the loose objects (crates, paper balls, domes) of their module file's object, as the original
    /// does; before, of the level's stand-in. `tutorial` as for `with_modules`.
    pub fn with_rules(level: &Level, start_ball: BallKind, modules: &Modules, tutorial: bool, format: u32) -> Sim {
        // A placeholder whose module is loaded is not solid itself: the module is made of its parts when the port
        // has its physics, and an original module without an entry has none. A placeholder whose module file is
        // missing (a map's own module) stays solid.
        let loaded = |i: usize| module_of(level, i).filter(|n| modules.get(n).is_some());
        let made: Vec<usize> = (0..level.objects.len())
            .filter(|&i| loaded(i).is_some_and(|n| crate::modules::spec(n).is_some()))
            .collect();
        let not_solid: Vec<usize> = (0..level.objects.len()).filter(|&i| loaded(i).is_some()).collect();
        let statics = static_meshes(level, &not_solid);
        let pos = |name: &str| -> Vec<(usize, Vec3)> {
            level
                .members(name)
                .map(|(i, o)| (i, Vec3::from(o.position())))
                .collect()
        };
        let start = pos("PS_Levelstart")
            .first()
            .map(|p| p.1)
            .unwrap_or(Vec3::ZERO);
        let mut trafos = Vec::new();
        for g in &level.groups {
            if let Some(rest) = g.name.strip_prefix("P_Trafo_")
                && let Some(kind) = BallKind::from_name(rest)
            {
                for &m in &g.members {
                    if let Some(o) = level.objects.get(m) {
                        trafos.push((m, Vec3::from(o.position()), kind, Mat4::from_cols_array_2d(&o.matrix)));
                    }
                }
            }
        }
        let rules = Rules {
            start,
            // in the order of their names (PC_TwoFlames_01, _02, ...), as the original numbers its sectors: a level's
            // group need not list them so (level 11 lists _01, _02, _04, _05, _03; level 12 _02 ... _06, _01, _07)
            checkpoints_listed: pos("PC_Checkpoints"),
            checkpoints: {
                let mut c = pos("PC_Checkpoints");
                c.sort_by(|a, b| {
                    let name = |i: usize| level.objects.get(i).map_or("", |o| o.name.as_str());
                    name_order(name(a.0), name(b.0))
                });
                c
            },
            resets: sorted_by_name(level, "PR_Resetpoints"),
            reset_turns: sorted_turns_by_name(level, "PR_Resetpoints"),
            resets_listed: by_bytes(level, "PR_Resetpoints").into_iter().map(|o| Vec3::from(o.position())).collect(),
            reset_turns_listed: by_bytes(level, "PR_Resetpoints").into_iter().map(turn_of).collect(),
            end: pos("PE_Levelende").first().map(|p| p.1),
            extra_points: pos("P_Extra_Point"),
            extra_lives: pos("P_Extra_Life"),
            trafos,
            depth_cubes: depth_cubes(level),
            death_y: statics
                .iter()
                .flat_map(StaticMesh::world_triangles)
                .flat_map(|t| [t.a.y, t.b.y, t.c.y])
                .fold(f32::INFINITY, f32::min)
                - 60.0,
        };
        // The camera starts looking along +X. The original sets it from the shared camera object (Camera.nmo,
        // Cam_Orient): its axis points along +X at every start, which the C game shows at the level 1 start. The
        // earlier guess from the floors turned it 180 degrees.
        let quarter = START_QUARTER;
        let yaw = quarter as f32 * std::f32::consts::FRAC_PI_2;
        // The ball appears at the first reset point (its centre), else just above the start.
        let spawn = rules
            .resets
            .first()
            .copied()
            .unwrap_or(start + Vec3::Y * BALL_RADIUS);
        let turn = rules.reset_turns.first().copied().unwrap_or(Mat3::IDENTITY);
        let ball = Ball {
            position: spawn,
            velocity: Vec3::ZERO,
            rotation: Rot::Basis(turn).quat(),
            spin: Vec3::ZERO,
            kind: start_ball,
            on_ground: false,
        };
        let mut world = World::new(&statics);
        // Body 0 is the ball; it is made after the first sector's objects (below).
        let ball_body = world.reserve_ball(ball.position, ball.rotation);
        debug_assert_eq!(ball_body, 0);
        // The sector of each level object (groups Sector_01, Sector_02, ...): a sector's objects are in the physics
        // only while it is the current one; an object in no sector always is.
        let sector_of = |i: usize| -> Option<usize> {
            level.groups.iter().find_map(|g| sector_number(&g.name).filter(|_| g.members.contains(&i)))
        };
        let object_sectors: Vec<Option<usize>> = (0..level.objects.len()).map(sector_of).collect();
        // Loose objects that move, and the domes: boxes, the loose balls and P_Dome (their material table in
        // docs/physics.md, section 3).
        let mut loose = Vec::new();
        let mut start_depths = Vec::new();
        let static_triangles: Vec<Triangle> = statics.iter().flat_map(StaticMesh::world_triangles).collect();
        let mut sectors: Vec<Vec<usize>> = Vec::new();
        let mut unsectored = Vec::new();
        for (i, o) in level.objects.iter().enumerate() {
            let Some(mesh) = o.mesh.and_then(|m| level.meshes.get(m)) else {
                continue;
            };
            let name = o.name.to_ascii_lowercase();
            // As the original physicalizes them: the pose without the scale, the mesh's points scaled.
            let Some((pos, rot, scale)) = pose_and_scale(&o.matrix) else {
                continue;
            };
            // The original makes the object of its module file (`P_Dome_MF` of PH/P_Dome.nmo, with the stand-in's
            // matrix) and physicalizes that one's mesh, which is not the stand-in's: the dome's is rounder (264
            // points against 73), 0.12 higher off the floor and 0.12 taller; the paper ball's a crumpled 315 points
            // against an 18-point ball. A level without the module file keeps its stand-in.
            let mesh = match module_mesh(level, i, modules) {
                Some(m) if format >= 5 => m,
                _ => mesh,
            };
            let points: Vec<Vec3> = mesh.positions.iter().map(|p| Vec3::from(*p) * scale).collect();
            // A loose object made inside a floor or a wall (deeper than the original's levels ever have one) is
            // lifted out onto it before it is made (`rest_out_of`): the physics' penetration recovery would shove
            // it out, and the player's ball touching it while it does is thrown away at hundreds of units a second.
            let is_box = level.members("P_Box").any(|(k, _)| k == i);
            let sphere = name
                .strip_prefix("p_ball_")
                .and_then(|_| BallKind::from_name(&name))
                .filter(|k| *k != BallKind::Paper)
                .map(|_| BALL_RADIUS * scale.x);
            let mut pos = pos;
            if is_box || name.starts_with("p_ball_") {
                let (depth, lift) = rest_out_of(&static_triangles, &points, sphere, pos, rot.quat());
                if lift > 0.0 {
                    eprintln!("{}: made {depth:.2} inside the floors, lifted {lift:.2} onto them", o.name);
                }
                pos.y += lift;
                start_depths.push((i, depth, lift));
            }
            let sector = sector_of(i);
            // Every object is made at the level start (below); here it is only placed.
            let active = false;
            let body = if level.members("P_Dome").any(|(k, _)| k == i) {
                // A dome is a fixed bumper.
                let material = Material {
                    friction: 0.2,
                    elasticity: 0.8,
                    mass: 0.2,
                    lin_damp: 0.1,
                    rot_damp: 0.1,
                };
                world.add_convex(&points, material, pos, rot, true, active)
            } else if level.members("P_Box").any(|(k, _)| k == i) {
                let material = Material {
                    friction: 0.7,
                    elasticity: 0.3,
                    mass: 1.0,
                    lin_damp: 0.1,
                    rot_damp: 0.1,
                };
                world.add_convex(&points, material, pos, rot, false, active)
            } else if let Some(kind) = name
                .strip_prefix("p_ball_")
                .and_then(|_| BallKind::from_name(&name))
            {
                let (mass, friction, elasticity, lin) = match kind {
                    BallKind::Wood => (2.0, 0.6, 0.2, 0.6),
                    BallKind::Stone => (10.0, 0.7, 0.1, 0.2),
                    BallKind::Paper => (0.2, 0.5, 0.4, 1.5),
                };
                let material = Material {
                    friction,
                    elasticity,
                    mass,
                    lin_damp: lin,
                    rot_damp: 0.1,
                };
                // The wooden and the stone ball are spheres of radius 2 (times the object's scale); the paper ball
                // has no radius in the table and is the hull of its mesh.
                match kind {
                    BallKind::Paper => world.add_convex(&points, material, pos, rot, false, active),
                    _ => Some(world.add_sphere(BALL_RADIUS * scale.x, material, pos, rot, active)),
                }
            } else {
                continue;
            };
            let Some(body) = body else {
                continue;
            };
            // its sound material from its groups; a dome sounds as one (hit material 4)
            let mut sound = sound_ids(level, i);
            if level.members("P_Dome").any(|(k, _)| k == i) {
                sound.hit = 4;
            }
            world.set_sound(body, sound);
            match sector {
                Some(n) => {
                    if sectors.len() <= n {
                        sectors.resize(n + 1, Vec::new());
                    }
                    sectors[n].push(body);
                }
                None => unsectored.push(body),
            }
            loose.push((i, body));
        }
        // The level start as the C game runs it at level 1: the physics runs at time factor 1 with only the level's
        // geometry, then the first sector's objects are made (at 9.90 s of its time), then the ball four frames later
        // (9.9667 s). The time puts the steps of the physics where the original's are.
        // The fixed anchor of the modules' joints (the original's FixCube: fixed, out of the collision system, at the
        // origin).
        let fixcube = world.add_part(
            &[FIXCUBE.to_vec()],
            Material {
                friction: 0.7,
                elasticity: 0.4,
                mass: 1.0,
                lin_damp: 0.1,
                rot_damp: 0.1,
            },
            Vec3::ZERO,
            Quat::IDENTITY,
            true,
            "",
            false,
            false,
            [0.0; 3],
            true,
        );
        // The modules' parts: each placeholder's module, made from the module file at the placeholder's place.
        let mut module_sectors: Vec<Vec<ModuleInst>> = Vec::new();
        let mut unsectored_modules = Vec::new();
        let mut module_bodies = Vec::new();
        for &i in &made {
            let Some(name) = module_of(level, i) else { continue };
            let (Some(spec), Some((_, file))) = (crate::modules::spec(name), modules.get(name)) else {
                continue;
            };
            let Some(inst) = module_parts(&mut world, level, i, spec, file, fixcube, &mut module_bodies) else {
                continue;
            };
            match sector_of(i) {
                Some(n) => {
                    if module_sectors.len() <= n {
                        module_sectors.resize_with(n + 1, Vec::new);
                    }
                    module_sectors[n].push(inst);
                }
                None => unsectored_modules.push(inst),
            }
        }
        world.run(LEVEL_INTRO_SECONDS);
        for &b in unsectored.iter().chain(sectors.first().map(Vec::as_slice).unwrap_or(&[])) {
            world.activate(b);
        }
        // The modules come one frame after the sector's objects, then the ball.
        let (frame, until_ball) = if tutorial {
            (1.0 / 60.0, START_OBJECTS_SECONDS_TUTORIAL)
        } else {
            (1.0 / 30.0, START_OBJECTS_SECONDS)
        };
        world.run(frame);
        // The wake-ups on approach and the finish triggers, armed now for the modules that are in the physics from
        // the start.
        let mut wake_triggers = Vec::new();
        let mut finishes = Vec::new();
        let mut updrafts = Vec::new();
        let mut swings = Vec::new();
        let mut breaks = Vec::new();
        let sectored = module_sectors.iter().enumerate().flat_map(|(n, v)| v.iter().map(move |m| (Some(n), m)));
        for (sector, inst) in unsectored_modules.iter().map(|m| (None, m)).chain(sectored) {
            let now = sector.is_none_or(|n| n == 0);
            let joints = if now { inst.enter(&mut world) } else { Vec::new() };
            if let Some(at) = inst.swing {
                swings.push(SwingRun::new(sector, at, now));
            }
            if let Some(u) = inst.updraft {
                let mut t = UpdraftTrigger::new(sector, u);
                if now {
                    t.outer.switch_on();
                }
                updrafts.push(t);
            }
            let then = inst.breaks.map(|at| {
                let joint = joints.get(at.joint).copied().flatten();
                breaks.push(BreakTrigger { at, joint });
                breaks.len() - 1
            });
            if let Some((body, near, mut prox)) = inst.wake {
                if now {
                    prox.switch_on();
                }
                wake_triggers.push(WakeTrigger { sector, body, then, forces: inst.forces.clone(), near, prox });
            }
            if let Some(f) = &inst.finish {
                let mut t = FinishTrigger {
                    near: f.near,
                    prox: f.prox,
                    force: f.force,
                    breaks: f.breaks.clone(),
                    joints: Vec::new(),
                };
                if now {
                    t.arm(&joints);
                }
                finishes.push(t);
            }
        }
        // The extra points' small balls start at the module's frames 1..6, around each stand-in.
        let mut extras = Vec::new();
        if let Some((_, file)) = modules.get("P_Extra_Point") {
            let local: Vec<Vec3> = (1..=6)
                .filter_map(|k| file.objects.iter().find(|o| o.name == format!("P_Extra_Point_Frame{k}")))
                .map(|o| Vec3::from(o.position()))
                .collect();
            if local.len() == 6 {
                for &(i, q) in &rules.extra_points {
                    let rot = level.objects.get(i).and_then(|o| pose_and_scale(&o.matrix)).map_or(Quat::IDENTITY, |p| p.1.quat());
                    extras.push(crate::extra::Extra::new(i, q, rot, &local));
                }
            }
        }
        world.run(until_ball - frame);
        world.replace_ball(0, BALL_RADIUS, ball.material(), ball.position, Rot::Basis(turn));
        let mut camera = Camera {
            quarter,
            yaw,
            lift: 0.0,
            position: Vec3::ZERO,
            step: Vec3::ZERO,
            target: ball.position,
        };
        camera.position = camera_goal(&camera, ball.position);
        Sim {
            rules,
            world,
            loose,
            start_depths,
            ball,
            camera,
            tick: 0,
            lives: 3,
            lives_spared: false,
            // the points counter takes its first point as the level starts (C game, level 2: 999 when the ball comes)
            points: START_POINTS - 1,
            sector: 0,
            finished: false,
            finished_at: 0,
            level_bonus: 0,
            game_over: false,
            taken: Vec::new(),
            // the first sector is in play from the start, as its objects are in the physics (above)
            unseen: object_sectors.iter().map(|s| s.is_some_and(|n| n != 0)).collect(),
            object_sectors,
            events: Vec::new(),
            prev_input: 0,
            hold: None,
            trafo_anim: None,
            dying: None,
            push_keys: 0,
            sectors,
            wake_triggers,
            finishes,
            updrafts,
            swings,
            extras,
            breaks,
            module_sectors,
            module_bodies,
            ball_boxes: [(Vec3::splat(-BALL_RADIUS), Vec3::splat(BALL_RADIUS)); 3],
            pieces: crate::pieces::Pieces::default(),
            flying: Vec::new(),
            pieces_burst: [None; 3],
            pieces_fade: [None; 3],
            pieces_expire: true,
            bursts: true,
            checkpoints_by_name: true,
            depth_from: DEPTH_PHASE,
            points_clock: 0,
            points_resume: 0,
            prev_pose: (ball.position, ball.rotation),
            sector_changes: Vec::new(),
            mods: crate::modrt::SimMods::default(),
            touching: (false, false),
        }
    }

    /// The number of sectors (at least 1).
    pub fn sector_count(&self) -> usize {
        self.rules.resets.len().max(self.sectors.len()).max(self.module_sectors.len()).max(1)
    }

    /// Begins the run elsewhere than the level start, for testing a place: `sector` (0 is the first) becomes the
    /// current one, as if its checkpoint had just been passed, and a new ball of `kind` (the current one if `None`)
    /// is made at rest at `at` (the sector's reset point if `None`), in the start view.
    pub fn start_from(&mut self, sector: usize, kind: Option<BallKind>, at: Option<Vec3>) {
        let sector = sector.min(self.sector_count() - 1);
        if sector != self.sector {
            let tick = self.tick;
            for (sector, enter) in [(self.sector, false), (sector, true)] {
                for modules in [false, true] {
                    self.sector_changes.push(SectorChange { tick, sector, enter, modules });
                }
            }
            self.apply_sector_changes();
            self.sector = sector;
        }
        if let Some(k) = kind {
            self.ball.kind = k;
        }
        self.respawn();
        if let Some(p) = at {
            self.ball.position = p;
            self.world.replace_ball(0, BALL_RADIUS, self.ball.material(), p, Rot::Quat(self.ball.rotation));
            self.camera.position = camera_goal(&self.camera, p);
            self.camera.target = p;
        }
    }

    /// Throws the ball: `dv` added to its velocity at the next step (tests).
    #[cfg(test)]
    pub(crate) fn throw_ball(&mut self, dv: Vec3) {
        let mass = self.ball.kind.params().mass;
        self.world.impulse_at(0, self.ball.position, dv * mass);
    }

    /// Puts the ball somewhere at rest, as a new ball (tests).
    #[cfg(test)]
    fn put_ball(&mut self, pos: Vec3) {
        self.ball.position = pos;
        self.ball.velocity = Vec3::ZERO;
        self.world.replace_ball(0, BALL_RADIUS, self.ball.material(), pos, self.ball.rotation);
    }

    /// The rigid bodies: index 0 is the ball, then the loose ones. Each is (level object index, body index).
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn loose(&self) -> &[(usize, usize)] {
        &self.loose
    }

    /// One tick, with the mods' rules around it (modrt.rs): the start's and the tick's before it, the events' after.
    pub fn step(&mut self, input: u8) {
        if self.mods.is_empty() || self.game_over || self.finished {
            self.step_run(input);
            return;
        }
        let before = self.events.len();
        if self.tick == 0 {
            self.fire_mods(gamedata::mods::Event::Start, input);
        }
        self.touching = self.touch();
        self.fire_mods(gamedata::mods::Event::Tick, input);
        let out = self.mods.out;
        self.world.set_rules(out.gravity, out.time_factor);
        self.step_run(input);
        let events: Vec<Event> = self.events.iter().skip(before).map(|e| e.1).collect();
        for e in events {
            use gamedata::mods::Event as M;
            let m = match e {
                Event::Checkpoint(_) => M::Sector,
                Event::Died => M::Lost,
                Event::NewBall => M::NewBall,
                Event::Transform(_) => M::Transform,
                Event::ExtraPoint | Event::ExtraLife => M::Extra,
                Event::Finished => M::Finish,
                _ => continue,
            };
            self.fire_mods(m, input);
        }
    }

    /// The run's values a mod reads (gamedata::mods::Value; the view's are 0 here).
    pub fn mod_value(&self, v: gamedata::mods::Value, input: u8) -> f32 {
        use gamedata::mods::Value as V;
        let b = &self.ball;
        let bit = |m: u8| if input & m != 0 { 1.0 } else { 0.0 };
        match v {
            V::BallX => b.position.x,
            V::BallY => b.position.y,
            V::BallZ => b.position.z,
            V::BallSpeed => b.velocity.length(),
            V::BallVy => b.velocity.y,
            V::BallKind => match b.kind {
                BallKind::Wood => 0.0,
                BallKind::Stone => 1.0,
                BallKind::Paper => 2.0,
            },
            V::BallTouching => {
                if self.touching.0 {
                    1.0
                } else {
                    0.0
                }
            }
            V::BallWall => {
                if self.touching.1 {
                    1.0
                } else {
                    0.0
                }
            }
            V::RunTick => self.tick as f32,
            V::RunTime => self.tick as f32 / 60.0,
            V::RunSector => (self.sector + 1) as f32,
            V::RunLives => self.lives as f32,
            V::RunPoints => self.points as f32,
            V::InputForward => bit(input::UP),
            V::InputBackward => bit(input::DOWN),
            V::InputLeft => bit(input::LEFT),
            V::InputRight => bit(input::RIGHT),
            V::InputRotation => bit(input::SHIFT),
            V::InputOverview => bit(input::LIFT),
            _ => 0.0,
        }
    }

    fn fire_mods(&mut self, event: gamedata::mods::Event, input: u8) {
        let mut mods = std::mem::take(&mut self.mods);
        mods.fire(event, &|v| self.mod_value(v, input));
        self.mods = mods;
    }

    /// The directions from what the ball touches to its centre (one per contact near enough to be one).
    fn contact_normals(&self) -> Vec<Vec3> {
        let p = self.ball.position;
        self.world
            .ball_contacts()
            .iter()
            .filter_map(|c| {
                let q = Vec3::new(c.point[0] as f32, c.point[1] as f32, c.point[2] as f32);
                let d = p - q;
                (d.length() < BALL_RADIUS * 1.5)
                    .then(|| d.normalize_or_zero())
                    .filter(|n| *n != Vec3::ZERO)
            })
            .collect()
    }

    fn touch(&self) -> (bool, bool) {
        let n = self.contact_normals();
        (!n.is_empty(), n.iter().any(|n| n.y < 0.5))
    }

    /// The sticky ball (a mod's `ball.stick`): what it touches holds it, against gravity, and a push into a wall
    /// turns up along it, so a wall can be climbed. `push` is the arrows' direction, `force` the ball's push.
    fn sticky_force(&self, push: Vec3, force: f32) -> Vec3 {
        let k = self.mods.out.stick;
        if k <= 0.0 || self.dying.is_some() || self.hold.is_some() {
            return push * force;
        }
        let normals = self.contact_normals();
        if normals.is_empty() {
            return push * force;
        }
        let n = normals.iter().copied().sum::<Vec3>().normalize_or_zero();
        // the ball's weight as the physics' impulses count it (one a PSI, 66 a second of its time)
        let weight = self.ball.kind.params().mass * self.mods.out.gravity / 66.0;
        let hold = -n * weight * k * 1.5 + Vec3::Y * weight * k * (1.0 - n.y.max(0.0));
        let into = -push.dot(n);
        let mut along = push - n * push.dot(n);
        if into > 0.0 {
            along += (Vec3::Y - n * n.y).normalize_or_zero() * into * k;
        }
        along * force + hold
    }

    /// The extra points and lives left in the current sector, and how many it has: (points, all, lives, all).
    pub fn extras_left(&self) -> (u32, u32, u32, u32) {
        let count = |list: &[(usize, Vec3)]| {
            let here: Vec<usize> = list
                .iter()
                .filter(|(i, _)| {
                    self.object_sectors.get(*i).copied().flatten() == Some(self.sector)
                })
                .map(|(i, _)| *i)
                .collect();
            let left = here.iter().filter(|i| !self.taken.contains(i)).count();
            (left as u32, here.len() as u32)
        };
        let (p, pa) = count(&self.rules.extra_points);
        let (l, la) = count(&self.rules.extra_lives);
        (p, pa, l, la)
    }

    fn step_run(&mut self, input: u8) {
        if self.game_over {
            return;
        }
        if self.finished {
            // The level is over, the world goes on: the balloon carries the ball away, unsteered, the camera looking
            // after it from where it stopped.
            self.tick += 1;
            self.world.set_push(0, Vec3::ZERO);
            self.world.step(None);
            if let Some(s) = self.world.state(0) {
                self.ball.position = s.pos;
                self.ball.velocity = s.vel;
                self.ball.rotation = s.rot;
                self.ball.spin = s.spin;
            }
            self.camera.target += (self.ball.position - self.camera.target) * CAM_TARGET_FORCE * TICK;
            return;
        }
        self.tick += 1;
        let pressed = input & !self.prev_input;
        self.prev_input = input;

        // Camera turns: Shift + Left / Right, a quarter per press.
        if input & input::SHIFT != 0 {
            if pressed & input::LEFT != 0 {
                self.camera.quarter -= 1;
            }
            if pressed & input::RIGHT != 0 {
                self.camera.quarter += 1;
            }
        }
        // a quarter in 250 ms at an even rate (the turn's Bezier progression is linear)
        let goal_yaw = self.camera.quarter as f32 * std::f32::consts::FRAC_PI_2;
        let rate = std::f32::consts::FRAC_PI_2 / CAM_TURN_TICKS;
        self.camera.yaw += (goal_yaw - self.camera.yaw).clamp(-rate, rate);
        self.camera.lift = if input & input::LIFT != 0 { 1.0 } else { 0.0 };

        // The push: the original's "Ball Navigation" has one force per arrow, each along a world axis chosen by the
        // camera's heading (Left / Right only when Shift is not held). Arrows held together add their forces, so a
        // diagonal push is the force times the square root of two.
        let push_keys = if input & input::SHIFT == 0 {
            input & (input::UP | input::DOWN | input::LEFT | input::RIGHT)
        } else {
            input & (input::UP | input::DOWN)
        };
        let mut push = Vec3::ZERO;
        if input & input::UP != 0 {
            push += self.camera.push_forward();
        }
        if input & input::DOWN != 0 {
            push -= self.camera.push_forward();
        }
        if input & input::SHIFT == 0 {
            if input & input::RIGHT != 0 {
                push += self.camera.push_right();
            }
            if input & input::LEFT != 0 {
                push -= self.camera.push_right();
            }
        }

        // The push is the original's control force, a SetPhysicsForce: the physics applies an impulse of the force at
        // every PSI (66 a second of its time), while the ball is awake. The ball's force is its material's.
        // The physics owns the ball's state; the game only reads it back. Only a new ball (a respawn, the end of a
        // transformer) puts a new body in, as the original does.
        // A ball that has fallen off is no longer steered.
        let push = if self.dying.is_some() { Vec3::ZERO } else { push };
        // Every change of the arrows (a press or a release) wakes the ball, as the original's navigation runs
        // "Physics WakeUp" (measured in the C game): a ball left alone falls asleep, and a force does nothing to a
        // sleeping body; a ball resting against a slope while pushed rolls back once the push is let go.
        if self.push_keys != push_keys && self.dying.is_none() && self.hold.is_none() {
            self.world.wake(0);
        }
        self.push_keys = push_keys;
        let p = self.ball.kind.params();
        let force = if self.mods.out.stick > 0.0 {
            self.sticky_force(push, p.force)
        } else {
            push * p.force
        };
        self.world.set_push(0, force);
        self.prev_pose = (self.ball.position, self.ball.rotation);
        self.world.step(None);
        // a kind's pieces start to fade 20 s after their burst, and go when their fade is over (in the order they were
        // made, so kinds going on the same tick leave the physics as they always did)
        let mut gone = [false; 3];
        let (tick, expire) = (self.tick, self.pieces_expire);
        for ((burst, fade), go) in self.pieces_burst.iter_mut().zip(&mut self.pieces_fade).zip(&mut gone) {
            if let (Some(b), None) = (*burst, *fade)
                && expire
                && tick >= b + crate::pieces::LIE_TICKS
            {
                *fade = Some(tick);
            }
            if fade.is_some_and(|f| tick >= f + crate::pieces::FADE_TICKS) {
                *go = true;
                *fade = None;
                *burst = None;
            }
        }
        if gone.contains(&true) {
            let world = &mut self.world;
            self.flying.retain(|&(kind, _, body)| {
                let go = gone.get(crate::pieces::index(kind)).copied().unwrap_or(false);
                if go {
                    world.remove(body);
                }
                !go
            });
        }
        if let Some(h) = self.hold {
            // the old ball bursts as it is hidden in the transformer
            if self.tick == h.start + crate::trafo::BEGIN + crate::trafo::OLD_BALL_HIDDEN as u64 && self.bursts {
                self.burst(h.old, self.ball.position);
            }
            // Out of the physics, the ball is drawn moving to the transformer's hold point.
            let t = (self.tick - h.start) as f32 * TRAFO_PULL;
            self.ball.position = h.at + (h.from - h.at) * (1.0 + t) * (-t).exp();
            if self.tick >= h.release {
                self.hold = None;
                self.ball.position = h.at;
                self.ball.velocity = Vec3::ZERO;
                self.ball.spin = Vec3::ZERO;
                self.world.replace_ball(0, BALL_RADIUS, self.ball.material(), h.at, self.ball.rotation);
                self.drop_ball_forces();
            }
        } else if let Some(s) = self.world.state(0) {
            self.ball.position = s.pos;
            self.ball.velocity = s.vel;
            self.ball.rotation = s.rot;
            self.ball.spin = s.spin;
        }
        self.ball.on_ground = self.hold.is_none() && self.world.grounded;

        self.follow_camera();
        self.apply_rules();
        if self.trafo_anim.is_some_and(|a| self.tick as f32 > (a.start + crate::trafo::BEGIN) as f32 + crate::trafo::END) {
            self.trafo_anim = None;
        }
        // The points counter, every 30 ticks from tick 32, waiting while a ball is lost.
        if self.tick > POINTS_FROM - POINTS_EVERY && self.dying.is_none() && self.tick >= self.points_resume {
            self.points_clock += 1;
            if self.points_clock.is_multiple_of(POINTS_EVERY) && self.points > 0 {
                self.points -= 1;
            }
        }
    }

    /// The rig's tick, in the original's order: InGameCam springs to Cam_Pos about the look-at point as it was, then
    /// the look-at point springs to the ball (TT Set Dynamic Position: next = now + last move × damping + (goal −
    /// now) × force × dt).
    fn follow_camera(&mut self) {
        rig_step(&mut self.camera, self.ball.position);
    }

    fn apply_rules(&mut self) {
        let p = self.ball.position;
        let near = |q: Vec3, r: f32| p.distance_squared(q) < r * r;

        // The extra points as TT Extra runs them: switched on within 80, 100 points when one fires, 20 a small ball.
        let mut extra_events = Vec::new();
        for e in &mut self.extras {
            let on = e.fired() || near(e.center(), 80.0);
            match e.tick(p, on) {
                Some(crate::extra::ExtraEvent::Activated) => extra_events.push((e.object, None)),
                Some(crate::extra::ExtraEvent::Hits(n)) => extra_events.push((e.object, Some(n))),
                None => {}
            }
        }
        for (i, hits) in extra_events {
            match hits {
                None => {
                    self.taken.push(i);
                    self.points += crate::extra::POINTS_ACTIVATED;
                    self.events.push((self.tick, Event::ExtraPoint));
                }
                Some(n) => {
                    self.points += crate::extra::POINTS_HIT * i64::from(n);
                    self.events.push((self.tick, Event::ExtraHits(n)));
                }
            }
        }
        for &(i, q) in &self.rules.extra_points {
            if self.extras.is_empty() && !self.taken.contains(&i) && near(q, PICK_RADIUS) {
                self.taken.push(i);
                self.points += 100;
                self.events.push((self.tick, Event::ExtraPoint));
            }
        }
        for &(i, q) in &self.rules.extra_lives {
            if !self.taken.contains(&i) && near(q, PICK_RADIUS) {
                self.taken.push(i);
                self.lives += 1;
                self.events.push((self.tick, Event::ExtraLife));
            }
        }
        for &(_, q, kind, at) in &self.rules.trafos {
            if self.hold.is_none() && self.ball.kind != kind && near(q, TRAFO_REACH) {
                // The original takes the ball out of the physics and, after a while, makes a ball of the new kind at
                // rest above the transformer.
                let old = self.ball.kind;
                self.ball.kind = kind;
                self.world.remove(0);
                self.ball.velocity = Vec3::ZERO;
                self.ball.spin = Vec3::ZERO;
                self.hold = Some(TrafoHold {
                    start: self.tick,
                    release: self.tick + TRAFO_TICKS,
                    at: q + TRAFO_HOLD,
                    from: p,
                    old,
                });
                self.trafo_anim = Some(TrafoAnim { at, start: self.tick, kind });
                self.events.push((self.tick, Event::Transform(kind)));
            }
        }
        // Checkpoints in order: reaching the next one opens the next sector.
        if let Some(&(_, q)) = self.checkpoints().get(self.sector)
            && near(q, GATE_RADIUS)
        {
            self.sector += 1;
            self.events
                .push((self.tick, Event::Checkpoint(self.sector)));
            // The passed sector's objects leave the physics, then the next sector's are made at their places, its
            // modules one frame later.
            self.sector_change(SECTOR_END_TICKS, self.sector - 1, false);
            self.sector_change(SECTOR_BEGIN_TICKS, self.sector, true);
        }
        if let Some(end) = self.rules.end
            && self.finishes.is_empty()
            && self.sector >= self.checkpoints().len()
            && near(end, GATE_RADIUS)
        {
            self.finished = true;
            self.finished_at = self.tick;
            self.events.push((self.tick, Event::Finished));
        }
        self.apply_sector_changes();
        self.check_wake_triggers();
        self.check_breaks();
        self.run_swings();
        self.check_updrafts();
        self.check_finishes();
        if let Some(d) = self.dying {
            if self.tick == d + DEATH_REMOVE_TICKS {
                self.world.remove(0);
                // Lives are balls: the last ball lost ends the run (in practice the last one is kept).
                if self.mods.out.endless && self.lives <= 1 {
                    self.lives_spared = true;
                } else {
                    self.lives = self.lives.saturating_sub(1);
                }
                if self.lives == 0 {
                    self.game_over = true;
                    self.events.push((self.tick, Event::GameOver));
                    return;
                }
            }
            if self.tick == d + DEATH_SHOW_TICKS {
                self.place_ball();
                self.events.push((self.tick, Event::NewBall));
            }
            if self.tick >= d + DEATH_RESPAWN_TICKS {
                self.dying = None;
                self.make_ball();
                self.points_resume = self.tick + POINTS_RESUME_TICKS;
            }
        } else if self.hold.is_none() && self.fell_off() {
            self.die();
        }
    }

    /// Whether the ball has left the level: its mesh box reaches a depth-test cube (the original's `BallManager`
    /// test), or, on a level without them, it is far below the floors. As the original's Group Iterator, one cube
    /// is tested a tick, in the group's order, with a tick without a test after the last one (N + 1 ticks a round,
    /// from the new ball on), against the ball where it was at the end of the last tick.
    fn fell_off(&self) -> bool {
        let cubes = &self.rules.depth_cubes;
        if cubes.is_empty() {
            return self.ball.position.y < self.rules.death_y;
        }
        let Some(age) = self.tick.checked_sub(self.depth_from) else { return false };
        let Some(cube) = cubes.get((age % (cubes.len() as u64 + 1)) as usize) else { return false };
        let (lo, hi) = self.ball_boxes[self.ball.kind as usize];
        let (pos, rot) = self.prev_pose;
        cube.overlaps(&Obb::from_box(lo, hi, &glam::Mat4::from_rotation_translation(rot, pos)))
    }

    fn die(&mut self) {
        self.events.push((self.tick, Event::Died));
        // the pieces lying about fade out and go (Ball_ResetPieces_<kind>)
        for (kind, _, _) in &self.flying {
            let k = crate::pieces::index(*kind);
            if let Some(f @ None) = self.pieces_fade.get_mut(k) {
                *f = Some(self.tick);
            }
        }
        self.dying = Some(self.tick);
        // The current sector starts again: its objects leave the physics and are made again at their places.
        self.sector_change(DEATH_SECTOR_END_TICKS, self.sector, false);
        self.sector_change(DEATH_SECTOR_BEGIN_TICKS, self.sector, true);
    }

    /// Schedules a sector's objects to enter or leave the physics `after` ticks from now, and its modules with them
    /// (entering one tick later).
    fn sector_change(&mut self, after: u64, sector: usize, enter: bool) {
        let tick = self.tick + after;
        self.sector_changes.push(SectorChange {
            tick,
            sector,
            enter,
            modules: false,
        });
        self.sector_changes.push(SectorChange {
            tick: if enter { tick + 1 } else { tick },
            sector,
            enter,
            modules: true,
        });
    }

    /// The boxes of the ball meshes (wood, stone, paper) from the game's Balls.nmo; the depth-test cubes are tested
    /// against the box of the ball's mesh, as the original does.
    pub fn set_ball_boxes(&mut self, boxes: [(Vec3, Vec3); 3]) {
        self.ball_boxes = boxes;
    }

    /// The balls' pieces, for the bursts in the transformers.
    pub fn set_pieces(&mut self, pieces: crate::pieces::Pieces) {
        self.pieces = pieces;
    }

    /// Where the ball's smoke puffs this tick, if it does: at the ball when a transformer bursts the old one, and at
    /// the new ball 2.5 s after its lightning (Ball_LightningSphere starts BallParticle_Frame script then), as at the
    /// level start.
    pub fn smoke(&self) -> Option<Vec3> {
        let hidden = crate::trafo::BEGIN + crate::trafo::OLD_BALL_HIDDEN as u64;
        let burst = self.hold.is_some_and(|h| self.tick == h.start + hidden);
        let new_ball = self.tick == 1
            || self.tick.checked_sub(SMOKE_AFTER_NEW_BALL).is_some_and(|t| {
                self.events.iter().rev().take(64).any(|&(at, e)| at == t && e == Event::NewBall)
            });
        (burst || new_ball).then_some(self.ball.position)
    }

    /// The pieces lying about: each one's mesh in the balls' model, its body and how opaque it is.
    pub fn pieces_view(&self) -> Vec<(usize, usize, f32)> {
        let alpha = |kind: BallKind| {
            let fade = self.pieces_fade.get(crate::pieces::index(kind)).copied().flatten();
            fade.map_or(1.0, |f| crate::pieces::fade(self.tick.saturating_sub(f)))
        };
        self.flying
            .iter()
            .filter_map(|&(kind, p, body)| self.pieces.of(kind).get(p).map(|piece| (piece.mesh, body, alpha(kind))))
            .collect()
    }

    /// The old ball bursts at `at` into its pieces (Ball_Explosion_<kind>): the pieces of that kind lying about are
    /// taken up again, set about the ball, made bodies and pushed along their own up.
    fn burst(&mut self, kind: BallKind, at: Vec3) {
        let again: Vec<usize> = self.flying.iter().filter(|f| f.0 == kind).map(|f| f.2).collect();
        for body in again {
            self.world.remove(body);
        }
        self.flying.retain(|f| f.0 != kind);
        let k = crate::pieces::index(kind);
        if let (Some(at), Some(fade)) = (self.pieces_burst.get_mut(k), self.pieces_fade.get_mut(k)) {
            *at = Some(self.tick);
            *fade = None;
        }
        let b = crate::pieces::burst(kind);
        let mut rng = crate::pieces::Rng::new(self.tick);
        let pieces: Vec<crate::pieces::Piece> = self.pieces.of(kind).to_vec();
        for (k, piece) in pieces.iter().enumerate() {
            let (pos, rot) = self.pieces.pose(kind, piece, at);
            let m = Material {
                friction: rng.range(b.friction),
                elasticity: b.elasticity,
                mass: rng.range(b.mass),
                lin_damp: b.lin_damp,
                rot_damp: b.rot_damp,
            };
            let pieces = [piece.hull.clone()];
            let made = self.world.add_part(&pieces, m, pos, rot, false, GROUP_BALL, false, true, [0.0; 3], true);
            let Some(body) = made else { continue };
            let up = rot * Vec3::Y;
            self.world.impulse_at(body, pos + rot * b.point, up * rng.range(b.impulse));
            self.flying.push((kind, k, body));
        }
    }

    /// The transformers: where each takes a ball (one within `TRAFO_REACH` of it) and the kind it makes.
    pub fn trafos(&self) -> impl Iterator<Item = (Vec3, BallKind)> + '_ {
        self.rules.trafos.iter().map(|&(_, q, kind, _)| (q, kind))
    }

    /// The transformer animation playing: the transformer's matrix, the animation's age in ticks and the kind it
    /// makes.
    pub fn trafo_anim(&self) -> Option<(Mat4, f32, BallKind)> {
        self.trafo_anim
            .map(|a| (a.at, self.tick as f32 - (a.start + crate::trafo::BEGIN) as f32, a.kind))
    }

    /// The ball to draw: in a transformer the old kind until the old ball is hidden, then none until the new one.
    pub fn drawn_ball(&self) -> Option<BallKind> {
        match self.hold {
            Some(h) if self.tick >= h.start + crate::trafo::BEGIN + crate::trafo::OLD_BALL_HIDDEN as u64 => None,
            Some(h) => Some(h.old),
            None => Some(self.ball.kind),
        }
    }

    /// What to draw of the extra points.
    pub fn extra_views(&self) -> Vec<crate::extra::ExtraView> {
        self.extras.iter().map(crate::extra::Extra::view).collect()
    }

    /// The fans' updrafts, for the test player: where each is measured to, the top of its volume and how far across
    /// from its point it takes the ball (the inner proximity, within the volume).
    pub fn updrafts(&self) -> Vec<(Vec3, f32, f32)> {
        self.updrafts
            .iter()
            .map(|u| {
                let v = &u.at.volume;
                let reach = |f: fn(Vec3) -> f32| (0..3).map(|i| f(v.axes[i]).abs() * v.half[i]).sum::<f32>();
                let across = reach(|a| a.x).min(reach(|a| a.z)).min(7.0);
                (u.at.point, v.center.y + reach(|a| a.y), across)
            })
            .collect()
    }

    /// Whether an updraft lifts the ball now.
    pub fn in_updraft(&self) -> bool {
        self.updrafts.iter().any(|u| u.force.is_some())
    }

    /// The fans in the physics now (their points), for their sound.
    pub fn fans(&self) -> Vec<Vec3> {
        self.updrafts.iter().filter(|u| u.outer.on || u.inner.on).map(|u| u.at.point).collect()
    }

    /// The score of a finished level, as the original's end screen counts it: the points left (time points), the
    /// level bonus and 200 for each life (C game: 950 + 100 + 3 × 200 = 1650 on level 1).
    pub fn score(&self) -> i64 {
        self.points + self.level_bonus + LIFE_BONUS * i64::from(self.lives)
    }

    /// The checkpoints in the order the sectors pass them.
    fn checkpoints(&self) -> &[(usize, Vec3)] {
        if self.checkpoints_by_name { &self.rules.checkpoints } else { &self.rules.checkpoints_listed }
    }

    /// The checkpoints in the order the sectors pass them, then the level's end.
    pub fn stages(&self) -> Vec<Vec3> {
        self.checkpoints().iter().map(|c| c.1).chain(self.rules.end).collect()
    }

    /// Where the transformers are.
    pub fn transformers(&self) -> Vec<Vec3> {
        self.rules.trafos.iter().map(|t| t.1).collect()
    }

    /// Whether the ball is falling to its loss (lost, not yet replaced).
    pub fn is_dying(&self) -> bool {
        self.dying.is_some()
    }

    /// Whether a transformer holds the ball (out of the physics, made over into another kind).
    pub fn is_held(&self) -> bool {
        self.hold.is_some()
    }

    /// Whether the checkpoint at level object `object` has been passed.
    pub fn checkpoint_passed(&self, object: usize) -> bool {
        self.checkpoints().iter().position(|c| c.0 == object).is_some_and(|k| k < self.sector)
    }

    /// Whether the last checkpoint has been passed (the level's last stage), and where the level ends.
    pub fn last_stage(&self) -> (bool, Option<Vec3>) {
        (self.sector >= self.checkpoints().len(), self.rules.end)
    }

    /// The modules' parts that the physics moves, for drawing.
    pub fn module_bodies(&self) -> &[ModuleBody] {
        &self.module_bodies
    }

    /// The modules' wake-ups on approach: a part is woken, once, when the ball comes within its distance.
    fn check_wake_triggers(&mut self) {
        let ball = self.ball.position;
        for w in &mut self.wake_triggers {
            if w.prox.frame(ball, w.near) == Some(Range::Enter) {
                w.prox.switch_off();
                if let Some(b) = w.then.and_then(|i| self.breaks.get_mut(i)) {
                    b.at.prox.switch_on();
                }
                self.world.wake(w.body);
                for f in &w.forces {
                    self.world.add_force(f.body, f.force, f.point);
                }
            }
        }
    }

    /// The hinges heavy balls break: the ball within reach of the part, and of the kind the script tests for.
    fn check_breaks(&mut self) {
        let (ball, kind) = (self.ball.position, self.ball.kind);
        let alive = self.hold.is_none() && self.dying.is_none();
        let tick = self.tick;
        for b in &mut self.breaks {
            let Some(near) = self.world.bodies.get(b.at.near).map(|x| x.pos) else { continue };
            if b.at.prox.frame(ball, near) == Some(Range::Enter) && alive && kind == b.at.kind {
                b.at.prox.switch_off();
                if let Some(j) = b.joint.take() {
                    self.world.remove_joint(j);
                }
                self.events.push((tick, Event::RopeTears));
            }
        }
    }

    /// The swings' and sacks' pushes, switched by their scripts' delayers; a push along + wakes the part.
    fn run_swings(&mut self) {
        let tick = self.tick;
        for w in &mut self.swings {
            let Some(since) = w.since else { continue };
            let want = w.wanted(tick - since);
            if w.push.map(|p| p.0) == Some(want) || (want == 0 && w.push.is_none()) {
                continue;
            }
            if let Some((_, f)) = w.push.take() {
                self.world.remove_force(f);
            }
            if want != 0 {
                let Some(origin) = self.world.bodies.get(w.at.body).map(|b| b.pos) else { continue };
                if want > 0 {
                    self.world.wake(w.at.body);
                }
                if let Some(f) = self.world.add_force(w.at.body, w.at.force * f32::from(want), origin) {
                    w.push = Some((want, f));
                }
            }
        }
    }

    /// The fans' updrafts, as module 18's script runs them: the outer proximity switches the inner one on and off;
    /// at each inner test while within it, the ball's box against the fan's volume makes or removes the force.
    fn check_updrafts(&mut self) {
        let ball = self.ball.position;
        let (lo, hi) = self.ball_boxes[self.ball.kind as usize];
        let ball_box = Obb::from_box(lo, hi, &Mat4::from_rotation_translation(self.ball.rotation, ball));
        let alive = self.hold.is_none() && self.dying.is_none();
        for u in &mut self.updrafts {
            match u.outer.frame(ball, u.at.point) {
                Some(Range::Enter) => u.inner.switch_on(),
                Some(Range::Exit) => {
                    u.inner.switch_off();
                    if let Some(f) = u.force.take() {
                        self.world.remove_force(f);
                    }
                }
                _ => {}
            }
            if u.inner.frame(ball, u.at.point) != Some(Range::In) {
                continue;
            }
            let inside = alive && u.at.volume.overlaps(&ball_box);
            match (inside, u.force) {
                (true, None) => u.force = self.world.add_force(0, Vec3::Y * u.at.value, ball),
                (false, Some(f)) => {
                    self.world.remove_force(f);
                    u.force = None;
                }
                _ => {}
            }
        }
    }

    /// The finish's triggers: the ball within reach of the balloon's platform pushes its box away, breaks its bridge
    /// and finishes the level, as its script does. The ball is measured to the part where the physics has it.
    fn check_finishes(&mut self) {
        let ball = self.ball.position;
        let mut fired = None;
        for t in &mut self.finishes {
            let Some(near) = self.world.bodies.get(t.near) else { continue };
            if t.prox.frame(ball, near.pos) == Some(Range::Enter) {
                t.prox.switch_off();
                fired = Some((t.force, t.joints.clone()));
                break;
            }
        }
        if let Some((force, joints)) = fired {
            self.world.add_force(force.body, force.force, force.point);
            for j in joints {
                self.world.remove_joint(j);
            }
            self.finished = true;
            self.finished_at = self.tick;
            self.events.push((self.tick, Event::Finished));
        }
    }

    /// The sector changes due at this tick, in the order they were made.
    fn apply_sector_changes(&mut self) {
        let tick = self.tick;
        let due: Vec<SectorChange> = self.sector_changes.iter().copied().filter(|c| c.tick == tick).collect();
        self.sector_changes.retain(|c| c.tick > tick);
        for c in due {
            if c.modules {
                let insts = self.module_sectors.get(c.sector).cloned().unwrap_or_default();
                for inst in &insts {
                    if c.enter {
                        let joints = inst.enter(&mut self.world);
                        if let Some(f) = &inst.finish
                            && let Some(t) = self.finishes.iter_mut().find(|t| t.near == f.near)
                        {
                            t.arm(&joints);
                        }
                        if let Some(b) = &inst.breaks
                            && let Some(t) = self.breaks.iter_mut().find(|t| t.at.near == b.near)
                        {
                            t.joint = joints.get(b.joint).copied().flatten();
                        }
                    } else {
                        inst.leave(&mut self.world);
                        if let Some(b) = &inst.breaks
                            && let Some(t) = self.breaks.iter_mut().find(|t| t.at.near == b.near)
                        {
                            t.at.prox.switch_off();
                        }
                        if let Some(f) = &inst.finish
                            && let Some(t) = self.finishes.iter_mut().find(|t| t.near == f.near)
                        {
                            t.prox.switch_off();
                        }
                    }
                }
                for w in self.wake_triggers.iter_mut().filter(|w| w.sector == Some(c.sector)) {
                    if c.enter {
                        w.prox.switch_on();
                    } else {
                        w.prox.switch_off();
                    }
                }
                for w in self.swings.iter_mut().filter(|w| w.sector == Some(c.sector)) {
                    if c.enter {
                        w.enter(tick);
                    } else {
                        w.stop(&mut self.world, tick);
                    }
                }
                for u in self.updrafts.iter_mut().filter(|u| u.sector == Some(c.sector)) {
                    if c.enter {
                        u.outer.switch_on();
                    } else {
                        u.stop(&mut self.world);
                    }
                }
                continue;
            }
            for &b in self.sectors.get(c.sector).map(Vec::as_slice).unwrap_or(&[]) {
                if c.enter {
                    self.world.activate(b);
                } else {
                    self.world.deactivate(b);
                }
            }
            for (u, _) in self.unseen.iter_mut().zip(&self.object_sectors).filter(|(_, s)| **s == Some(c.sector)) {
                *u = !c.enter;
            }
        }
    }

    /// The scripts' forces on the ball went with its old body; a new ball starts without them.
    fn drop_ball_forces(&mut self) {
        for u in &mut self.updrafts {
            u.force = None;
        }
    }

    /// A new ball at the current reset point, at rest, and the camera in its start view.
    fn respawn(&mut self) {
        self.place_ball();
        self.make_ball();
    }

    /// The new ball's body at the place it was put, at rest.
    fn make_ball(&mut self) {
        self.world.replace_ball(0, BALL_RADIUS, self.ball.material(), self.ball.position, Rot::Quat(self.ball.rotation));
        self.drop_ball_forces();
        // the new ball starts the depth-test cubes' iterator again
        self.depth_from = self.tick + DEPTH_PHASE;
        self.prev_pose = (self.ball.position, self.ball.rotation);
    }

    /// The new ball put at the current reset point (shown, not yet a body), and the camera in its start view.
    fn place_ball(&mut self) {
        let (resets, turns) = if self.checkpoints_by_name {
            (&self.rules.resets, &self.rules.reset_turns)
        } else {
            (&self.rules.resets_listed, &self.rules.reset_turns_listed)
        };
        let spawn = resets.get(self.sector).copied().unwrap_or(self.rules.start + Vec3::Y * BALL_RADIUS);
        let turn = turns.get(self.sector).copied().unwrap_or(Mat3::IDENTITY);
        self.ball.position = spawn;
        self.ball.rotation = Rot::Basis(turn).quat();
        self.ball.velocity = Vec3::ZERO;
        self.ball.spin = Vec3::ZERO;
        self.hold = None;
        self.trafo_anim = None;
        // The rig comes back as it was at the start (TT Restore IC on Cam_MF, put at the reset point): the start
        // heading, not lifted, the look-at point at the ball; InGameCam springs there from where it was, its speed
        // forgotten.
        self.camera.quarter = START_QUARTER;
        self.camera.yaw = START_QUARTER as f32 * std::f32::consts::FRAC_PI_2;
        self.camera.lift = 0.0;
        self.camera.step = Vec3::ZERO;
        self.camera.target = self.ball.position;
    }
}

/// Deeper than this in the level's static geometry a loose object is lifted out before it is made. The original's
/// levels place every loose object at most 0.031 deep (the paper balls of level 10; all others touch or are clear),
/// so none of theirs is moved; our levels' builder refuses anything deeper than 0.05 (levelkit.py, check_clear).
pub const SUNK_DEPTH: f32 = 0.1;

/// How deep a loose object (its points about `pos`, or a sphere of `sphere`'s radius) starts in the static
/// `triangles`, and how far up it is moved so that it rests on them instead (0 when it is not deeper than
/// SUNK_DEPTH, or no lift up to four times its reach frees it).
fn rest_out_of(triangles: &[Triangle], points: &[Vec3], sphere: Option<f32>, pos: Vec3, rot: Quat) -> (f32, f32) {
    let world: Vec<Vec3> = points.iter().map(|p| pos + rot * *p).collect();
    let (middle, reach) = match sphere {
        Some(r) => (pos, r),
        None => crate::collide::Shape::Hull(&world).bounds(),
    };
    let top = 4.0 * reach;
    let near: Vec<Triangle> = triangles
        .iter()
        .filter(|t| {
            let lo = t.a.min(t.b).min(t.c);
            let hi = t.a.max(t.b).max(t.c);
            lo.x < middle.x + reach && hi.x > middle.x - reach && lo.z < middle.z + reach && hi.z > middle.z - reach
                && lo.y < middle.y + reach + top && hi.y > middle.y - reach
        })
        .copied()
        .collect();
    let depth_at = |lift: f32, scratch: &mut Vec<Vec3>| -> f32 {
        let up = Vec3::Y * lift;
        let s = match sphere {
            Some(r) => crate::collide::Shape::Sphere(pos + up, r),
            None => {
                scratch.clear();
                scratch.extend(world.iter().map(|p| *p + up));
                crate::collide::Shape::Hull(scratch)
            }
        };
        crate::collide::start_depth(&s, &near)
    };
    let mut scratch = Vec::new();
    let depth = depth_at(0.0, &mut scratch);
    if depth <= SUNK_DEPTH {
        return (depth, 0.0);
    }
    // up in steps until it is clear, then halved down to where it just rests
    let step = 0.05;
    let mut below = 0.0;
    let mut lift = step;
    while lift <= top {
        if depth_at(lift, &mut scratch) <= 0.0 {
            for _ in 0..12 {
                let mid = (below + lift) * 0.5;
                if depth_at(mid, &mut scratch) <= 0.0 {
                    lift = mid;
                } else {
                    below = mid;
                }
            }
            // a hair over, so that it settles onto the floor rather than starting in it
            return (depth, lift + 0.005);
        }
        below = lift;
        lift += step;
    }
    (depth, 0.0)
}

/// The mesh of the module file's object (`<module>_MF`) that the original makes in place of a loose object's
/// stand-in (a crate, a paper ball, a dome), if the module file is loaded and has it.
fn module_mesh<'a>(level: &Level, i: usize, modules: &'a Modules) -> Option<&'a level::Mesh> {
    let name = level::module_name(&level.objects.get(i)?.name);
    let (_, file) = modules.get(name)?;
    let root = format!("{name}_MF");
    let o = file.objects.iter().find(|o| o.name.eq_ignore_ascii_case(&root))?;
    file.meshes.get(o.mesh?)
}

/// The most sectors a level has (the original's have up to 8; hints take sectors 1 to 64 as well).
pub const MAX_SECTORS: usize = 64;

/// The sector a group `Sector_NN` names (0 is the first), when NN is 1 to MAX_SECTORS: a map's absurd number (a
/// billion) would make the sectors' lists that long.
fn sector_number(group: &str) -> Option<usize> {
    let n: usize = group.strip_prefix("Sector_")?.parse().ok()?;
    (1..=MAX_SECTORS).contains(&n).then(|| n - 1)
}

/// The sector of the level object nearest to `p` among those in a sector (groups `Sector_NN`; 0 is the first), or
/// `None` if no object is in one.
pub fn sector_near(level: &Level, p: Vec3) -> Option<usize> {
    level
        .groups
        .iter()
        .filter_map(|g| {
            let n = sector_number(&g.name)?;
            let d = g
                .members
                .iter()
                .filter_map(|&i| level.objects.get(i))
                .map(|o| p.distance_squared(Vec3::from(o.position())))
                .fold(f32::INFINITY, f32::min);
            Some((d, n))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, n)| n)
}

/// One tick of the camera rig toward the ball at `ball` (see `Camera`).
fn rig_step(c: &mut Camera, ball: Vec3) {
    let goal = camera_goal(c, c.target);
    let force = Vec3::new(CAM_FORCE.x, if c.lift > 0.5 { CAM_LIFT_FORCE } else { CAM_FORCE.y }, CAM_FORCE.z);
    let next = c.position + c.step * CAM_DAMP + (goal - c.position) * force * TICK;
    c.step = next - c.position;
    c.position = next;
    c.target += (ball - c.target) * CAM_TARGET_FORCE * TICK;
}

/// Cam_Pos about a look-at point: 22 behind it along the view's heading and 35 above, 50 more while Space is held.
fn camera_goal(cam: &Camera, at: Vec3) -> Vec3 {
    at - cam.forward() * CAM_BACK + Vec3::Y * (CAM_UP + CAM_LIFT * cam.lift)
}

/// The boxes of the level's depth-test cubes (group `DepthTestCubes`): each member's mesh box under its matrix.
fn depth_cubes(level: &Level) -> Vec<Obb> {
    level
        .members("DepthTestCubes")
        .filter_map(|(_, o)| {
            let mesh = o.mesh.and_then(|i| level.meshes.get(i))?;
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            for p in &mesh.positions {
                min = min.min(Vec3::from(*p));
                max = max.max(Vec3::from(*p));
            }
            (min.x <= max.x).then(|| Obb::from_box(min, max, &glam::Mat4::from_cols_array_2d(&o.matrix)))
        })
        .collect()
}

/// The turns (matrices without scale) of a group's members, sorted by name as `sorted_by_name`.
fn sorted_turns_by_name(level: &Level, group: &str) -> Vec<Mat3> {
    let mut v: Vec<(String, Mat3)> = level
        .members(group)
        .map(|(_, o)| (o.name.clone(), turn_of(o)))
        .collect();
    v.sort_by(|a, b| name_order(&a.0, &b.0));
    v.into_iter().map(|x| x.1).collect()
}

/// An object's turn: its matrix without scale.
fn turn_of(o: &level::Object) -> Mat3 {
    match pose_and_scale(&o.matrix) {
        Some((_, Rot::Basis(m), _)) => m,
        _ => Mat3::IDENTITY,
    }
}

/// A group's members in plain byte order of their names, as reset points were taken before replay format 4.
fn by_bytes<'a>(level: &'a Level, group: &str) -> Vec<&'a level::Object> {
    let mut v: Vec<&level::Object> = level.members(group).map(|(_, o)| o).collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

/// Objects numbered by name (PR_Resetpoint_9, PR_Resetpoint_10): by the name before the number, then by the number
/// as a number (compared digit by digit, so any length of number orders right), then by the whole name.
fn name_order(a: &str, b: &str) -> std::cmp::Ordering {
    fn split(s: &str) -> (usize, &str) {
        let stem = s.trim_end_matches(|c: char| c.is_ascii_digit());
        let digits = s.get(stem.len()..).unwrap_or("").trim_start_matches('0');
        (stem.len(), digits)
    }
    let ((sa, da), (sb, db)) = (split(a), split(b));
    let stem = |s: &str, n: usize| s.get(..n).unwrap_or("").to_owned();
    stem(a, sa)
        .cmp(&stem(b, sb))
        .then_with(|| da.len().cmp(&db.len()))
        .then_with(|| da.cmp(db))
        .then_with(|| a.cmp(b))
}

fn sorted_by_name(level: &Level, group: &str) -> Vec<Vec3> {
    let mut v: Vec<(String, Vec3)> = level
        .members(group)
        .map(|(_, o)| (o.name.clone(), Vec3::from(o.position())))
        .collect();
    v.sort_by(|a, b| name_order(&a.0, &b.0));
    v.into_iter().map(|x| x.1).collect()
}

/// An object's pose without its scale, and the scale (the lengths of its matrix's axes), as the original's physicalize
/// splits them. None for a mirrored or degenerate matrix, which has no rotation.
fn pose_and_scale(matrix: &[[f32; 4]; 4]) -> Option<(Vec3, Rot, Vec3)> {
    let mat = glam::Mat4::from_cols_array_2d(matrix);
    let axes = [mat.x_axis.truncate(), mat.y_axis.truncate(), mat.z_axis.truncate()];
    let scale = Vec3::new(axes[0].length(), axes[1].length(), axes[2].length());
    if scale.min_element() <= 1e-6 {
        return None;
    }
    let unscaled = Mat3::from_cols(axes[0] / scale.x, axes[1] / scale.y, axes[2] / scale.z);
    if unscaled.determinant() <= 0.0 {
        return None;
    }
    Some((mat.w_axis.truncate(), Rot::Basis(unscaled), scale))
}

/// The physical objects of the level, one mesh each, as the original physicalizes them: the mesh's points scaled by
/// the object's scale in its own frame, the pose without the scale (an object in two solid groups is made once).
/// A level object's sound material from its groups (`Sound_HitID_0N`, `Sound_RollID_0N`), which the original's
/// level init turns into the "Coll Detection ID" and "Continuous Contact ID" attributes.
fn sound_ids(level: &Level, i: usize) -> crate::physics::SoundIds {
    let id = |prefix: &str| {
        level
            .groups_of(i)
            .find_map(|g| g.strip_prefix(prefix).and_then(|n| n.parse::<u8>().ok()))
            .unwrap_or(0)
    };
    crate::physics::SoundIds { hit: id("Sound_HitID_"), roll: id("Sound_RollID_") }
}

fn static_meshes(level: &Level, made: &[usize]) -> Vec<StaticMesh> {
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for g in &level.groups {
        // Floors and rails are group Floor. Floor stoppers are group Ball, the player ball's: they stop boxes and
        // loose balls but not the ball (docs/physics.md, section 3.4).
        let group = match g.name.to_ascii_lowercase().as_str() {
            "phys_floors" | "phys_floorrails" => GROUP_FLOOR,
            "phys_floorstopper" => GROUP_BALL,
            // Modules are solid for now; their own behaviour comes with phase 4. Boxes are bodies (see `Sim::new`).
            n if n.starts_with("p_modul") => "",
            _ => continue,
        };
        for &m in &g.members {
            if seen.contains(&m) || made.contains(&m) {
                continue;
            }
            seen.push(m);
            let Some(o) = level.objects.get(m) else {
                continue;
            };
            let Some(mesh) = o.mesh.and_then(|i| level.meshes.get(i)) else {
                continue;
            };
            // A mirrored or degenerate matrix has no rotation; such an object keeps its world triangles at the origin.
            let mat = glam::Mat4::from_cols_array_2d(&o.matrix);
            let (pos, rot, local): (Vec3, Rot, Vec<Vec3>) = match pose_and_scale(&o.matrix) {
                Some((pos, rot, scale)) => (pos, rot, mesh.positions.iter().map(|p| Vec3::from(*p) * scale).collect()),
                None => (
                    Vec3::ZERO,
                    Rot::Quat(Quat::IDENTITY),
                    mesh.positions.iter().map(|p| mat.transform_point3(Vec3::from(*p))).collect(),
                ),
            };
            let mut triangles = Vec::new();
            for part in &mesh.parts {
                for t in part.indices.as_chunks::<3>().0 {
                    let v = |k: usize| {
                        t.get(k)
                            .and_then(|&i| local.get(i as usize))
                            .copied()
                            .unwrap_or_default()
                    };
                    let (a, b, c) = (v(0), v(1), v(2));
                    if (b - a).cross(c - a).length_squared() < 1e-12 {
                        continue;
                    }
                    triangles.push(Triangle { a, b, c });
                }
            }
            if !triangles.is_empty() {
                out.push(StaticMesh {
                    triangles,
                    pos,
                    rot,
                    group,
                    sound: sound_ids(level, m),
                });
            }
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use level::{Group, Level, Material, Mesh, Object, Part};

    /// A flat 200×200 floor at y = 0 in Phys_Floors, a start at the origin, a reset point a ball's radius up.
    pub(crate) fn flat_level() -> Level {
        let s = 100.0;
        let floor = Mesh {
            name: "floor".into(),
            channels: Vec::new(),
            look: None,
            positions: vec![[-s, 0.0, -s], [s, 0.0, -s], [s, 0.0, s], [-s, 0.0, s]],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            uvs: vec![[0.0; 2]; 4],
            parts: vec![Part {
                material: Some(0),
                indices: vec![0, 2, 1, 0, 3, 2],
            }],
        };
        let at = |x: f32, y: f32, z: f32| {
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [x, y, z, 1.0],
            ]
        };
        let object = |name: &str, mesh: Option<usize>, m| Object {
            name: name.into(),
            matrix: m,
            mesh,
            visible: true,
            sprite: None,
        };
        Level {
            meshes: vec![floor],
            materials: vec![Material {
                name: "m".into(),
                diffuse: [1.0; 4],
                ambient: [0.0; 3],
                emissive: [0.0; 3],
                specular: [0.0; 3],
                power: 0.0,
                texture: None,
                blend: None,
                alpha_cutoff: None,
                z_write: true,
                two_sided: false,
                unlit: false,
                texgen: None,
                normal: None,
                parallax: 0.0,
            }],
            textures: vec![],
            objects: vec![
                object("floor", Some(0), at(0.0, 0.0, 0.0)),
                object("PS_FourFlames_01", None, at(0.0, 0.0, 0.0)),
                object("PR_Resetpoint_01", None, at(0.0, BALL_RADIUS, 0.0)),
            ],
            groups: vec![
                Group {
                    name: "Phys_Floors".into(),
                    members: vec![0],
                },
                Group {
                    name: "PS_Levelstart".into(),
                    members: vec![1],
                },
                Group {
                    name: "PR_Resetpoints".into(),
                    members: vec![2],
                },
            ],
            sky: None,
            start_ball: None,
            settings: level::Settings::default(),
        }
    }

    /// The run's mods of a mod.txt (all on, at their defaults).
    fn mods_of(text: &str) -> crate::modrt::SimMods {
        let found = vec![gamedata::mods::Mod {
            key: "m".into(),
            factory: true,
            parsed: gamedata::mods::parse_mod_txt(text),
        }];
        crate::modrt::Mods::load(found, |_| Some("on".into()))
            .running(true)
            .0
    }

    #[test]
    fn a_mods_gravity_and_time_change_the_fall_and_replay_the_same() {
        let mut level = flat_level();
        level.objects[2].matrix[3][1] = 30.0;
        let fall = |mods: &crate::modrt::SimMods| {
            let mut sim = Sim::new(&level, BallKind::Wood);
            sim.mods = mods.clone();
            for _ in 0..30 {
                sim.step(0);
            }
            sim.ball.position.y
        };
        let classic = fall(&crate::modrt::SimMods::default());
        let moon_text = "name.en Moon\non start\ndo world.gravity 4\n";
        let moon = fall(&mods_of(moon_text));
        assert!(
            moon > classic + 3.0,
            "low gravity falls slower: {moon} against {classic}"
        );
        assert_eq!(
            fall(&mods_of(moon_text)),
            moon,
            "the same mod plays the same run"
        );
        let slow = fall(&mods_of("name.en Slow\non start\ndo world.time 0.5\n"));
        assert!(
            slow > classic + 3.0,
            "a slower physics falls less in as many ticks: {slow} against {classic}"
        );
    }

    #[test]
    fn the_sticky_ball_climbs_a_wall_it_is_pushed_into() {
        let forward = Sim::new(&flat_level(), BallKind::Wood)
            .camera
            .push_forward()
            .normalize();
        let side = Vec3::Y.cross(forward).normalize();
        let base = forward * 8.0;
        let corner = |a: f32, h: f32| (base + side * a + Vec3::Y * h).to_array();
        let mut level = flat_level();
        level.meshes.push(Mesh {
            name: "wall".into(),
            channels: Vec::new(),
            look: None,
            positions: vec![
                corner(-30.0, -1.0),
                corner(30.0, -1.0),
                corner(30.0, 60.0),
                corner(-30.0, 60.0),
            ],
            normals: vec![(-forward).to_array(); 4],
            uvs: vec![[0.0; 2]; 4],
            parts: vec![Part {
                material: Some(0),
                indices: vec![0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2],
            }],
        });
        level.objects.push(Object {
            name: "wall".into(),
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            mesh: Some(1),
            visible: true,
            sprite: None,
        });
        level.groups[0].members.push(3);
        let climb = |mods: crate::modrt::SimMods| {
            let mut sim = Sim::new(&level, BallKind::Wood);
            sim.mods = mods;
            let mut top = 0.0f32;
            for _ in 0..600 {
                sim.step(input::UP);
                top = top.max(sim.ball.position.y);
            }
            top
        };
        let plain = climb(crate::modrt::SimMods::default());
        let sticky = climb(mods_of("name.en Sticky\non tick\ndo ball.stick 1\n"));
        assert!(
            plain < BALL_RADIUS + 1.0,
            "the plain ball stays down: {plain}"
        );
        assert!(
            sticky > BALL_RADIUS + 5.0,
            "the sticky ball climbs: {sticky}"
        );
    }

    #[test]
    fn forward_rolls_the_ball_forward_on_the_floor() {
        let level = flat_level();
        let mut sim = Sim::new(&level, BallKind::Wood);
        let forward = sim.camera.forward();
        for _ in 0..120 {
            sim.step(input::UP);
        }
        let moved = sim.ball.position - Vec3::new(0.0, BALL_RADIUS, 0.0);
        assert!(
            moved.dot(forward) > 5.0,
            "ball moved {moved:?}, forward {forward:?}"
        );
        assert!(
            (sim.ball.position.y - BALL_RADIUS).abs() < 0.05,
            "ball left the floor: {:?}",
            sim.ball.position
        );
        assert!(sim.ball.on_ground);
    }

    #[test]
    fn a_rolling_ball_turns_the_way_it_rolls() {
        let level = flat_level();
        let mut sim = Sim::new(&level, BallKind::Wood);
        // the start settles and the push builds up its speed before the rolling is judged
        for _ in 0..90 {
            sim.step(input::UP);
        }
        let forward = sim.camera.forward();
        // Rolling without slipping on a floor, the spin is up × velocity / radius: about the axis to the right of the
        // travel, at speed / radius.
        let v = sim.ball.velocity;
        let rolling = Vec3::Y.cross(v) / BALL_RADIUS;
        assert!(v.dot(forward) > 5.0, "the ball does not roll forward: {v:?}");
        assert!(
            (sim.ball.spin - rolling).length() < 0.1 * rolling.length(),
            "spin {:?}, rolling without slipping {rolling:?}",
            sim.ball.spin
        );
        // The pose turns about the spin: the rotation over one tick has the spin's axis.
        let before = sim.ball.rotation;
        sim.step(input::UP);
        let (axis, angle) = (sim.ball.rotation * before.inverse()).to_axis_angle();
        let axis = if angle > std::f32::consts::PI { -axis } else { axis };
        assert!(
            axis.dot(sim.ball.spin.normalize()) > 0.99,
            "turned about {axis:?}, spin {:?}",
            sim.ball.spin
        );
    }

    #[test]
    fn paper_carries_less_momentum_than_wood_and_stone() {
        let p = |k: BallKind| {
            let q = k.params();
            q.force / q.mass
        };
        // Speed reached on the same push, and momentum per unit speed (mass): paper is the lightest.
        assert!(BallKind::Paper.params().mass < BallKind::Wood.params().mass);
        assert!(BallKind::Wood.params().mass < BallKind::Stone.params().mass);
        assert!(p(BallKind::Paper) > p(BallKind::Stone));
    }

    /// The rig's steady lag behind a ball rolling at a constant speed v along the view: the look-at point v·(1/10 −
    /// dt), InGameCam v·(1 − 0.5)/5 more: about 0.183·v in all at 60 Hz (the C game's rig, measured).
    #[test]
    fn the_camera_lags_behind_a_rolling_ball_as_the_original() {
        let mut c =
            Camera { quarter: 0, yaw: 0.0, lift: 0.0, position: Vec3::ZERO, step: Vec3::ZERO, target: Vec3::ZERO };
        c.position = camera_goal(&c, Vec3::ZERO);
        let v = 10.0;
        let mut ball = Vec3::ZERO;
        for _ in 0..1200 {
            ball += c.forward() * v * TICK;
            rig_step(&mut c, ball);
        }
        let lag = (camera_goal(&c, ball) - c.position).dot(c.forward());
        assert!((lag - 0.183 * v).abs() < 0.01, "lag {lag}");
        assert!((c.position.y - (ball.y + CAM_UP)).abs() < 1e-3);
    }

    #[test]
    fn a_respawn_brings_the_camera_back_to_the_start_view() {
        let level = flat_level();
        // Turn the camera away and lift it, then fall off.
        let mut sim = Sim::new(&level, BallKind::Paper);
        for _ in 0..120 {
            sim.step(input::SHIFT | input::RIGHT);
        }
        for _ in 0..120 {
            sim.step(input::LIFT);
        }
        assert_ne!((sim.camera.quarter, sim.camera.lift), (START_QUARTER, 0.0));
        sim.put_ball(Vec3::new(500.0, 10.0, 0.0));
        for _ in 0..600 {
            sim.step(0);
            if sim.lives < 3 {
                break;
            }
        }
        for _ in 0..DEATH_RESPAWN_TICKS {
            sim.step(0);
        }
        assert_eq!(sim.camera.quarter, START_QUARTER);
        assert!((sim.camera.yaw - START_QUARTER as f32 * std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert_eq!(sim.camera.lift, 0.0);
    }

    #[test]
    fn a_transformer_makes_a_new_heavier_ball_after_holding_it() {
        // A stone transformer right under the start: the ball is taken out of the physics, held, and comes back as a
        // stone ball at rest above the transformer, as the C game does (old body gone, new one TRAFO_TICKS later).
        let mut level = flat_level();
        let trafo = level.objects.len();
        level.objects.push(Object {
            name: "P_Trafo_Stone_01".into(),
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            mesh: None,
            visible: true,
            sprite: None,
        });
        level.groups.push(Group {
            name: "P_Trafo_Stone".into(),
            members: vec![trafo],
        });
        let mut sim = Sim::new(&level, BallKind::Wood);
        sim.step(0);
        assert!(sim.events.contains(&(1, Event::Transform(BallKind::Stone))));
        assert!(sim.world.state(0).is_none(), "the ball left the physics");
        // The old wooden ball is drawn until a tick before the new one, and the animation plays from the next tick.
        assert_eq!(sim.drawn_ball(), Some(BallKind::Wood));
        assert_eq!(sim.trafo_anim().map(|a| a.1), Some(-1.0));
        for _ in 1..TRAFO_TICKS {
            sim.step(0);
            assert!(sim.world.state(0).is_none());
        }
        assert_eq!(sim.drawn_ball(), None, "the old ball is hidden");
        sim.step(0);
        assert_eq!(sim.drawn_ball(), Some(BallKind::Stone));
        let s = sim.world.state(0).expect("a new ball");
        assert_eq!(sim.ball.kind, BallKind::Stone);
        assert!((s.pos - TRAFO_HOLD).length() < 0.1, "made at {:?}", s.pos);
        for _ in 0..10 {
            sim.step(0);
        }
        assert!(sim.trafo_anim().is_none(), "the animation is over");

        // The stone ball is heavier: the same push gives it less than half the wooden ball's speed (C game, level 1:
        // 3.1 against 6.9 after 30 ticks).
        for _ in 0..60 {
            sim.step(0);
        }
        let mut wood = Sim::new(&flat_level(), BallKind::Wood);
        for _ in 0..60 {
            wood.step(0);
        }
        for _ in 0..30 {
            sim.step(input::DOWN);
            wood.step(input::DOWN);
        }
        let (vs, vw) = (sim.ball.velocity.length(), wood.ball.velocity.length());
        assert!(vs > 1.0 && vs < 0.6 * vw, "stone {vs}, wood {vw}");
    }

    #[test]
    fn a_ball_that_reaches_a_depth_cube_falls_unsteered_then_comes_back() {
        // A depth-test cube whose top is 10 below the floor, beside it: the ball runs off the floor's edge.
        let mut level = flat_level();
        let mesh = level.meshes.len();
        let mut corners = Vec::new();
        for x in [-1.0f32, 1.0] {
            for y in [-1.0f32, 1.0] {
                for z in [-1.0f32, 1.0] {
                    corners.push([x, y, z]);
                }
            }
        }
        level.meshes.push(Mesh {
            name: "cube".into(),
            channels: Vec::new(),
            look: None,
            positions: corners,
            normals: vec![[0.0, 1.0, 0.0]; 8],
            uvs: vec![[0.0; 2]; 8],
            parts: vec![],
        });
        let cube = level.objects.len();
        level.objects.push(Object {
            name: "Quader01".into(),
            // 400 wide, 100 deep, top at y = -10
            matrix: [
                [200.0, 0.0, 0.0, 0.0],
                [0.0, 50.0, 0.0, 0.0],
                [0.0, 0.0, 200.0, 0.0],
                [0.0, -60.0, 0.0, 1.0],
            ],
            mesh: Some(mesh),
            visible: false,
            sprite: None,
        });
        level.groups.push(Group {
            name: "DepthTestCubes".into(),
            members: vec![cube],
        });
        let mut sim = Sim::new(&level, BallKind::Wood);
        sim.put_ball(Vec3::new(110.0, 5.0, 0.0));
        let mut died = None;
        for _ in 0..300 {
            sim.step(input::UP);
            if died.is_none() && sim.events.iter().any(|e| e.1 == Event::Died) {
                died = Some((sim.tick, sim.ball.position, sim.ball.velocity));
                break;
            }
        }
        let (t, p, v) = died.expect("the ball fell into the cube");
        // The ball's box (half size 2) reaches the cube's top at y = -10.
        assert!(p.y <= -8.0 && p.y > -9.0, "died at {p:?}");
        // Unsteered from there: with the push held, the speed along it only decays (it would grow by about 15 in
        // 30 ticks if the push still worked).
        for _ in 0..30 {
            sim.step(input::UP);
        }
        assert!(sim.ball.velocity.x < v.x, "the falling ball was still pushed: {:?} from {v:?}", sim.ball.velocity);
        while sim.tick < t + DEATH_REMOVE_TICKS {
            sim.step(0);
        }
        assert!(sim.world.state(0).is_none(), "the ball left the physics");
        while sim.tick < t + DEATH_RESPAWN_TICKS {
            sim.step(0);
        }
        assert!(sim.world.state(0).is_some());
        assert!(sim.ball.position.distance(Vec3::new(0.0, BALL_RADIUS, 0.0)) < 0.5);
    }

    #[test]
    fn two_arrows_push_with_both_forces_along_world_axes() {
        // Each arrow is a force of its own along a world axis: Up and Right together push diagonally with the force
        // times sqrt(2), so the ball gets about sqrt(2) times as fast as with Up alone.
        let level = flat_level();
        let run = |input: u8| {
            let mut sim = Sim::new(&level, BallKind::Wood);
            for _ in 0..60 {
                sim.step(0);
            }
            for _ in 0..30 {
                sim.step(input);
            }
            sim.ball.velocity * Vec3::new(1.0, 0.0, 1.0)
        };
        let (up, diag) = (run(input::UP), run(input::UP | input::RIGHT));
        let ratio = diag.length() / up.length();
        assert!((ratio - 2f32.sqrt()).abs() < 0.05, "diagonal {diag:?} against up {up:?}: {ratio}");
        let along = diag.normalize();
        assert!((along.x - along.z.abs()).abs() < 0.02, "not diagonal: {along:?}");
    }

    #[test]
    fn a_camera_turn_changes_the_push_axis_at_once() {
        let mut cam = Sim::new(&flat_level(), BallKind::Wood).camera;
        assert_eq!(cam.push_forward(), Vec3::X);
        cam.quarter += 1;
        // the view is still where it was, the push already along the next axis
        assert!((cam.forward() - Vec3::X).length() < 1e-6);
        assert_eq!(cam.push_forward(), Vec3::NEG_Z);
        assert_eq!(cam.push_right(), Vec3::NEG_X);
        cam.quarter = -1;
        assert_eq!(cam.push_forward(), Vec3::NEG_X);
    }

    #[test]
    fn a_ball_that_fell_asleep_wakes_when_pushed() {
        // Left alone, the ball's core goes calm and is frozen; the original's navigation wakes it on every press.
        let level = flat_level();
        let mut sim = Sim::new(&level, BallKind::Wood);
        for _ in 0..1200 {
            sim.step(0);
        }
        assert_eq!(sim.world.movement_state(0), Some(8), "the ball is asleep after 20 s");
        let start = sim.ball.position;
        for _ in 0..30 {
            sim.step(input::UP);
        }
        assert!(sim.ball.position.distance(start) > 1.0, "the push did not move a sleeping ball");
    }

    /// A unit cube mesh (corners at -1..1) added to the level, and an object of it at `at` in the given groups.
    fn add_cube(level: &mut Level, name: &str, at: Vec3, groups: &[&str]) -> usize {
        let mesh = level.meshes.len();
        let mut corners = Vec::new();
        for x in [-1.0f32, 1.0] {
            for y in [-1.0f32, 1.0] {
                for z in [-1.0f32, 1.0] {
                    corners.push([x, y, z]);
                }
            }
        }
        level.meshes.push(Mesh {
            name: "cube".into(),
            channels: Vec::new(),
            look: None,
            positions: corners,
            normals: vec![[0.0, 1.0, 0.0]; 8],
            uvs: vec![[0.0; 2]; 8],
            parts: vec![],
        });
        let obj = level.objects.len();
        level.objects.push(Object {
            name: name.into(),
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [at.x, at.y, at.z, 1.0],
            ],
            mesh: Some(mesh),
            visible: true,
            sprite: None,
        });
        for g in groups {
            match level.groups.iter_mut().find(|x| x.name == *g) {
                Some(x) => x.members.push(obj),
                None => level.groups.push(Group {
                    name: (*g).into(),
                    members: vec![obj],
                }),
            }
        }
        obj
    }

    /// A convex mesh: the box from `lo` to `hi`.
    fn box_mesh(name: &str, lo: Vec3, hi: Vec3) -> Mesh {
        let mut corners = Vec::new();
        for x in [lo.x, hi.x] {
            for y in [lo.y, hi.y] {
                for z in [lo.z, hi.z] {
                    corners.push([x, y, z]);
                }
            }
        }
        Mesh {
            name: name.into(),
            channels: Vec::new(),
            look: None,
            positions: corners,
            normals: vec![[0.0, 1.0, 0.0]; 8],
            uvs: vec![[0.0; 2]; 8],
            parts: vec![],
        }
    }

    /// The original physicalizes a dome (any loose object) as its module file's object `P_Dome_MF`, not as the
    /// level's stand-in (the dome of level 1: the stand-in 3.55 high and coarse, the module's 3.68 and round). Here
    /// the stand-in is a slab the ball rolls over and the module's object a wall it cannot climb: format 5 meets
    /// the wall, a replay of format 4 rolls over the slab as it was recorded.
    #[test]
    fn a_dome_is_made_of_its_module_file_s_object() {
        let mut level = flat_level();
        let probe = Sim::new(&level, BallKind::Wood);
        let forward = probe.camera.forward().with_y(0.0).normalize();
        let at = forward * 15.0;
        let stand_in = add_cube(&mut level, "P_Dome_01", at, &["P_Dome", "Sector_01"]);
        let mesh = level.objects[stand_in].mesh.expect("a mesh");
        level.meshes[mesh] = box_mesh("P_Dome_01_Mesh", Vec3::new(-6.6, 0.0, -6.6), Vec3::new(6.6, 0.05, 6.6));
        level.objects[stand_in].matrix[3] = [at.x, 0.0, at.z, 1.0];
        let mut file = Level::default();
        file.meshes.push(box_mesh("P_Dome_MF_Mesh", Vec3::new(-6.6, 0.0, -6.6), Vec3::new(6.6, 8.0, 6.6)));
        file.objects.push(Object {
            name: "P_Dome_MF".into(),
            matrix: Mat4::IDENTITY.to_cols_array_2d(),
            mesh: Some(0),
            visible: true,
            sprite: None,
        });
        let modules = Modules {
            models: vec![file],
            names: vec!["P_Dome".into()],
        };
        let along = |format: u32| {
            let mut sim = Sim::with_rules(&level, BallKind::Wood, &modules, false, format);
            for _ in 0..360 {
                sim.step(input::UP);
            }
            sim.ball.position.dot(forward)
        };
        let new = along(5);
        assert!(new < 15.0 - 6.6, "the ball got past the module's wall: {new}");
        let old = along(4);
        assert!(old > 15.0 + 6.6, "a format 4 run did not roll over the stand-in: {old}");
    }

    /// A loose ball placed 1.5 into the floor (a level's mistake: our level 1 had them, up to 2.4 deep) is lifted
    /// out onto it before it is made; the ball rolled into it pushes it away instead of being thrown off the level by
    /// the physics' penetration recovery (seen at 740 units a second before the lift).
    #[test]
    fn a_loose_ball_made_in_the_floor_is_lifted_out_onto_it() {
        let mut level = flat_level();
        let probe = Sim::new(&level, BallKind::Wood);
        let forward = probe.camera.forward().with_y(0.0).normalize();
        let at = probe.ball.position.with_y(0.5) + forward * 12.0;
        let stone = add_cube(&mut level, "P_Ball_Stone_01", at, &["P_Ball_Stone", "Sector_01"]);
        let mut sim = Sim::new(&level, BallKind::Wood);
        let &(i, depth, lift) = sim.start_depths.first().expect("the loose ball measured");
        assert_eq!(i, stone);
        assert!((depth - 1.5).abs() < 0.01 && (lift - 1.5).abs() < 0.02, "{depth} deep, lifted {lift}");
        let body = sim.loose[0].1;
        let mut fastest = (0.0f32, 0.0f32);
        for _ in 0..240 {
            sim.step(input::UP);
            fastest.0 = fastest.0.max(sim.ball.velocity.length());
            fastest.1 = fastest.1.max(sim.world.state(body).map_or(0.0, |s| s.vel.length()));
        }
        assert!(fastest.0 < 15.0 && fastest.1 < 15.0, "thrown: ball {}, loose ball {}", fastest.0, fastest.1);
        let moved = sim.world.state(body).map_or(Vec3::ZERO, |s| s.pos) - at;
        assert!(moved.dot(forward) > 1.0, "the ball pushed it on: {moved}");
    }

    #[test]
    fn a_sector_s_objects_are_in_the_physics_only_while_it_is_current() {
        // A box in sector 1 and one in sector 2, and the first checkpoint on the ball's way.
        let mut level = flat_level();
        let o1 = add_cube(&mut level, "P_Box_01", Vec3::new(-30.0, 1.0, 0.0), &["P_Box", "Sector_01"]);
        let o2 = add_cube(&mut level, "P_Box_02", Vec3::new(-30.0, 1.0, 20.0), &["P_Box", "Sector_02"]);
        let cp = add_cube(&mut level, "PC_TwoFlames_01", Vec3::new(0.0, 0.0, 0.0), &["PC_Checkpoints"]);
        let mut sim = Sim::new(&level, BallKind::Wood);
        let (b1, b2) = (sim.loose[0].1, sim.loose[1].1);
        assert!(sim.world.state(b1).is_some(), "the first sector's box is in the physics");
        assert!(sim.world.state(b2).is_none(), "the second sector's box is not yet");
        // and they are seen only while their sector is in play (an object in no sector always)
        assert!(!sim.unseen[o1] && sim.unseen[o2] && !sim.unseen[cp], "{:?}", sim.unseen);
        // the ball starts on the checkpoint: the sector changes at once
        sim.step(0);
        let t = sim.tick;
        assert!(sim.events.contains(&(t, Event::Checkpoint(1))));
        while sim.tick < t + SECTOR_END_TICKS {
            sim.step(0);
        }
        assert!(sim.world.state(b1).is_none(), "the passed sector's box left");
        assert!(sim.world.state(b2).is_none());
        assert!(sim.unseen[o1], "the passed sector's box is gone from sight too");
        while sim.tick < t + SECTOR_BEGIN_TICKS {
            sim.step(0);
        }
        assert!(sim.world.state(b2).is_some(), "the next sector's box came");
        assert!(!sim.unseen[o2] && !sim.unseen[cp]);
    }

    #[test]
    fn checkpoints_go_by_their_names_whatever_order_their_group_lists() {
        // as level 11 lists them: the second first, the first (under the ball) after it
        let mut level = flat_level();
        let far = add_cube(&mut level, "PC_TwoFlames_02", Vec3::new(0.0, 0.0, 300.0), &["PC_Checkpoints"]);
        let near = add_cube(&mut level, "PC_TwoFlames_01", Vec3::new(0.0, 0.0, 0.0), &["PC_Checkpoints"]);
        let mut sim = Sim::new(&level, BallKind::Wood);
        sim.step(0);
        assert!(sim.events.contains(&(sim.tick, Event::Checkpoint(1))), "the first by name is passed");
        assert!(sim.checkpoint_passed(near) && !sim.checkpoint_passed(far));
        // a replay of format 3 or older keeps the group's order: the far one is first, the ball not at it
        let mut old = Sim::new(&level, BallKind::Wood);
        old.checkpoints_by_name = false;
        old.step(0);
        assert!(!old.events.iter().any(|e| matches!(e.1, Event::Checkpoint(_))));
    }

    #[test]
    fn numbered_names_go_by_their_numbers() {
        use std::cmp::Ordering::Less;
        assert_eq!(name_order("PR_Resetpoint_9", "PR_Resetpoint_10"), Less);
        assert_eq!(name_order("PC_TwoFlames_03", "PC_TwoFlames_04"), Less);
        assert_eq!(name_order("PC_TwoFlames_01", "PC_TwoFlames_1"), Less, "equal numbers: by the whole name");
        assert_eq!(name_order("PC_TwoFlames_02", "PC_TwoFlames_1"), std::cmp::Ordering::Greater);
        assert_eq!(name_order("X_1", "X_99999999999999999999999"), Less, "numbers longer than any integer");
        assert_eq!(name_order("X_00000000000000000000000009", "X_10"), Less);
        assert_eq!(name_order("A_2", "B_1"), Less, "the stem first");
        assert_eq!(name_order("PC_TwoFlames", "PC_TwoFlames_01"), Less);
    }

    /// Level 1's finish with the player's copy (skipped without OPENBALLANCE_DATA): rolled onto the balloon's
    /// platform, the ball finishes the level and the balloon carries it away along +x at about 1.1 units per 20
    /// ticks, the ball 0.72 behind the platform's centre, as in the C game (docs/physics.md, the balloon).
    #[test]
    fn the_balloon_finishes_level_1_and_carries_the_ball_away() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(1)).expect("level 1").level;
        let modules = crate::scene::load_modules(&level, &data);
        let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, true);
        sim.start_from(3, None, Some(Vec3::new(704.0, -9.0, -873.7)));
        for t in 0..400 {
            sim.step(if t < 120 { input::UP } else { 0 });
            if sim.finished {
                break;
            }
        }
        assert!(sim.finished, "not finished, the ball at {:?}", sim.ball.position);
        let platform = sim.finishes[0].near;
        let p0 = sim.world.bodies[platform].pos;
        for _ in 0..200 {
            sim.step(0);
        }
        let p1 = sim.world.bodies[platform].pos;
        let speed = (p1.x - p0.x) / 10.0;
        assert!((speed - 1.1).abs() < 0.3, "the platform went {:?} -> {:?}", p0, p1);
        let behind = sim.ball.position - p1;
        assert!((behind.x + 0.72).abs() < 0.2 && behind.y.abs() < 0.5, "the ball at {behind:?} from the platform");
    }

    /// Level 2's first fan with the player's copy (skipped without OPENBALLANCE_DATA): its updraft lifts a paper
    /// ball to the top of its volume (62.8) and holds it there, its box's top touching it; a wooden ball stays down.
    #[test]
    fn a_fan_holds_a_paper_ball_at_the_top_of_its_updraft() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(2)).expect("level 2").level;
        let modules = crate::scene::load_modules(&level, &data);
        let at = Vec3::new(1016.67, 49.0, -366.74);
        let height = |kind| {
            let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, false);
            sim.start_from(2, Some(kind), Some(at));
            let mut ys = Vec::new();
            for t in 0..600 {
                sim.step(0);
                if t >= 300 {
                    ys.push(sim.ball.position.y);
                }
            }
            let lo = ys.iter().copied().fold(f32::MAX, f32::min);
            let hi = ys.iter().copied().fold(f32::MIN, f32::max);
            (lo, hi)
        };
        let (lo, hi) = height(BallKind::Paper);
        assert!(lo > 63.5 && hi < 66.0, "paper ball between {lo} and {hi}");
        let (_, hi) = height(BallKind::Wood);
        assert!(hi < 49.0, "wooden ball up to {hi}");
    }

    /// Level 8's sack (sector 3) and swing (sector 5) with the player's copy (skipped without OPENBALLANCE_DATA):
    /// their scripts' pushes swing them as in the C game, which moved them by these z offsets at these frames after
    /// their sector came in (the C build with the checkpoints moved under the ball).
    #[test]
    fn level_8_s_sack_and_swing_swing_as_in_the_c_game() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(8)).expect("level 8").level;
        let modules = crate::scene::load_modules(&level, &data);
        let cases: [(usize, f32, &[(u64, f32)]); 2] = [
            (2, 572.15, &[(135, 1.86), (255, -5.31), (510, 7.26), (600, -7.54)]),
            (4, 865.12, &[(30, 2.53), (105, -5.13), (180, 6.33), (480, -4.57)]),
        ];
        for (sector, x, samples) in cases {
            let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, false);
            sim.start_from(sector, None, None);
            let body = sim
                .swings
                .iter()
                .find(|w| w.since.is_some() && (sim.world.bodies[w.at.body].pos.x - x).abs() < 1.0)
                .expect("the part")
                .at
                .body;
            let z0 = sim.world.bodies[body].pos.z;
            let mut t = 0;
            for &(frame, dz) in samples {
                while t < frame {
                    sim.step(0);
                    t += 1;
                }
                let got = sim.world.bodies[body].pos.z - z0;
                assert!((got - dz).abs() < 0.15, "part at x {x}, frame {frame}: {got} against {dz}");
            }
        }
    }

    /// Level 2's plate bridge (module 29) with the player's copy (skipped without OPENBALLANCE_DATA): a stone ball
    /// on plate 06 breaks the hinge between plates 06 and 07 and falls with the plates; a wooden ball only bends
    /// the bridge (about 3.7).
    #[test]
    fn a_stone_ball_breaks_the_plate_bridge_and_a_wooden_one_does_not() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(2)).expect("level 2").level;
        let modules = crate::scene::load_modules(&level, &data);
        let drop = |kind| {
            let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, false);
            let plate = sim.breaks.first().expect("module 29").at.near;
            let p = sim.world.bodies[plate].pos;
            let sector = sector_near(&level, p).expect("its sector");
            sim.start_from(sector, Some(kind), Some(p + Vec3::Y * 3.0));
            // the deepest the plate went before the ball was lost (the sector's objects then come back)
            let y0 = sim.world.bodies[plate].pos.y;
            let mut deepest = 0.0f32;
            for _ in 0..240 {
                sim.step(0);
                if sim.dying.is_some() {
                    break;
                }
                deepest = deepest.max(y0 - sim.world.bodies[plate].pos.y);
            }
            (deepest, sim.breaks[0].joint.is_none())
        };
        let (stone, broken) = drop(BallKind::Stone);
        assert!(broken && stone > 6.0, "under the stone ball: broken {broken}, the plate fell {stone}");
        // the wooden ball only bends the bridge
        let (wood, broken) = drop(BallKind::Wood);
        assert!(!broken && wood < 5.0, "under the wooden ball: broken {broken}, the plate fell {wood}");
    }

    /// A short route off level 2 with the player's copy (skipped without OPENBALLANCE_DATA): the ball is lost at
    /// tick 331, as in the C game on the same input, where the depth-test cubes are tested one a frame; the new ball
    /// is a body from tick 577 (C: y 10.33 there, 10.07 at 581), and at tick 600 the points are 988 with 2 lives,
    /// the counter having waited while the ball was lost.
    #[test]
    fn a_ball_leaving_level_2_is_lost_on_the_c_game_s_tick() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(2)).expect("level 2").level;
        let modules = crate::scene::load_modules(&level, &data);
        let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, false);
        let route: [(u64, u8); 9] = [(1, 0), (40, 1), (140, 0), (170, 8), (230, 0), (260, 2), (330, 0), (360, 4), (400, 0)];
        let input = |t: u64| route.iter().rev().find(|r| r.0 <= t).map_or(0, |r| r.1);
        while sim.tick < 400 && !sim.events.iter().any(|e| e.1 == Event::Died) {
            sim.step(input(sim.tick + 1));
        }
        let died = sim.events.iter().find(|e| e.1 == Event::Died).map(|e| e.0);
        assert_eq!(died, Some(331));
        let mut heights = Vec::new();
        while sim.tick < 600 {
            sim.step(input(sim.tick + 1));
            if sim.tick == 577 || sim.tick == 581 {
                heights.push(sim.ball.position.y);
            }
        }
        assert!((heights[0] - 10.33).abs() < 0.02 && (heights[1] - 10.07).abs() < 0.03, "{heights:?}");
        assert_eq!((sim.points, sim.lives), (988, 2));
    }

    /// Level 1's module 34 with the player's copy (skipped without OPENBALLANCE_DATA): a wooden ball pushed into
    /// the box under the stone moves it to x 148.88 and stalls, the stone staying up, as in the C game (its ball
    /// brought back next to the box by a moved reset point); a stone ball pushes the box out and the stone drops.
    #[test]
    fn module_34_s_box_gives_way_to_a_stone_ball_only() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(1)).expect("level 1").level;
        let modules = crate::scene::load_modules(&level, &data);
        let push = |kind| {
            let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, true);
            sim.start_from(1, Some(kind), Some(Vec3::new(132.0, -7.0, -563.03)));
            let part = |n: &str| sim.module_bodies.iter().find(|b| b.object == n).map(|b| b.body).expect(n);
            let (kiste, stein) = (part("P_Modul_34_Kiste"), part("P_Modul_34_Schiebestein"));
            for t in 0..300 {
                sim.step(if t < 120 { input::UP } else { 0 });
            }
            (sim.world.bodies[kiste].pos.x, sim.world.bodies[stein].pos.y)
        };
        let (x, y) = push(BallKind::Wood);
        assert!((x - 148.88).abs() < 0.2 && (y + 2.50).abs() < 0.05, "wooden ball: box at x {x}, stone at y {y}");
        let (x, y) = push(BallKind::Stone);
        assert!(x > 155.0 && y < -6.0, "stone ball: box at x {x}, stone at y {y}");
    }

    /// Level 1's start with the player's copy (skipped without OPENBALLANCE_DATA): the ball dropped onto the floor
    /// hits it, and rolling on it touches a material the original's sound scripts know (its groups).
    #[test]
    fn the_ball_meets_sound_materials_on_level_1() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(1)).expect("level 1").level;
        let modules = crate::scene::load_modules(&level, &data);
        let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, true);
        sim.start_from(0, None, Some(sim.ball.position + Vec3::Y * 6.0));
        let (mut hits, mut rolls) = (Vec::new(), Vec::new());
        for t in 0..240 {
            sim.step(if t > 60 { input::UP } else { 0 });
            hits.extend(sim.world.ball_sounds.impacts.iter().map(|i| i.0).filter(|&m| m > 0));
            rolls.extend(sim.world.ball_sounds.rolls.iter().copied().filter(|&m| m > 0));
        }
        assert!(!hits.is_empty(), "no hit on a known material");
        assert!(!rolls.is_empty(), "no rolling on a known material");
    }

    /// Level 1's first extra point with the player's copy (skipped without OPENBALLANCE_DATA): the ball at its
    /// centre fires it (100 points); its six small balls fly away for a second and then reach the ball, 20 points
    /// each, as in the C game with an extra point moved onto the ball: no hit 60 frames after firing, five by 120,
    /// all six by 200.
    #[test]
    fn an_extra_point_gives_its_points_as_in_the_c_game() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(1)).expect("level 1").level;
        let modules = crate::scene::load_modules(&level, &data);
        let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, true);
        assert!(!sim.extras.is_empty(), "the module's small balls");
        let at = sim.extras[0].center();
        sim.start_from(0, None, Some(at));
        let mut fired = None;
        let mut hits = Vec::new();
        for _ in 0..300 {
            sim.step(0);
            for (t, e) in sim.events.iter() {
                match e {
                    Event::ExtraPoint if fired.is_none() => fired = Some(*t),
                    Event::ExtraHits(n) if *t == sim.tick => hits.extend(std::iter::repeat_n(*t, *n as usize)),
                    _ => {}
                }
            }
        }
        let f = fired.expect("fired");
        let by = |d: u64| hits.iter().filter(|&&t| t <= f + d).count();
        eprintln!("fired {f} hits {:?}", hits.iter().map(|t| t - f).collect::<Vec<_>>());
        assert_eq!((by(60), by(200)), (0, 6));
        assert!(by(120) >= 4, "{} by 120", by(120));
    }

    /// Found by review while fuzzing a level played: a map's group "Sector_1000000000" made the list of sectors a
    /// billion long (gigabytes, or a capacity overflow for larger numbers). A sector number beyond MAX_SECTORS puts
    /// its objects in no sector.
    #[test]
    fn an_absurd_sector_number_is_no_sector() {
        let mut level = flat_level();
        add_cube(&mut level, "P_Box_01", Vec3::new(-30.0, 1.0, 0.0), &["P_Box", "Sector_1000000000"]);
        add_cube(&mut level, "P_Box_02", Vec3::new(-30.0, 1.0, 20.0), &["P_Box", "Sector_18446744073709551615"]);
        add_cube(&mut level, "P_Box_03", Vec3::new(-30.0, 1.0, 40.0), &["P_Box", "Sector_02"]);
        let sim = Sim::new(&level, BallKind::Wood);
        assert_eq!(sim.sector_count(), 2);
        assert_eq!(sector_near(&level, Vec3::new(-30.0, 1.0, 0.0)), Some(1));
    }

    #[test]
    fn a_run_can_begin_at_a_later_sector_and_place() {
        let mut level = flat_level();
        add_cube(&mut level, "P_Box_01", Vec3::new(-30.0, 1.0, 0.0), &["P_Box", "Sector_01"]);
        add_cube(&mut level, "P_Box_02", Vec3::new(-30.0, 1.0, 20.0), &["P_Box", "Sector_02"]);
        let mut sim = Sim::new(&level, BallKind::Wood);
        let (b1, b2) = (sim.loose[0].1, sim.loose[1].1);
        let at = Vec3::new(-20.0, 3.0, 20.0);
        assert_eq!(sector_near(&level, at), Some(1));
        sim.start_from(1, Some(BallKind::Stone), Some(at));
        assert_eq!((sim.sector, sim.ball.kind), (1, BallKind::Stone));
        assert!(sim.world.state(b1).is_none(), "the first sector's box left");
        assert!(sim.world.state(b2).is_some(), "the chosen sector's box is in");
        assert!((sim.world.state(0).expect("a ball").pos - at).length() < 1e-4);
    }

    #[test]
    fn a_lost_ball_puts_the_sector_s_objects_back() {
        let mut level = flat_level();
        let start = Vec3::new(-30.0, 1.0, 0.0);
        add_cube(&mut level, "P_Box_01", start, &["P_Box", "Sector_01"]);
        let mut sim = Sim::new(&level, BallKind::Wood);
        let b = sim.loose[0].1;
        // push the box away (it fell asleep before the ball came), then lose the ball
        sim.world.wake(b);
        for _ in 0..60 {
            sim.world.set_push(b, Vec3::X * 0.5);
            sim.step(0);
        }
        sim.world.set_push(b, Vec3::ZERO);
        let moved = sim.world.state(b).unwrap().pos;
        assert!(moved.distance(start) > 3.0, "the box did not move: {moved:?}");
        sim.put_ball(Vec3::new(500.0, 10.0, 0.0));
        let mut died = None;
        for _ in 0..600 {
            sim.step(0);
            if sim.lives < 3 {
                died = Some(sim.tick);
                break;
            }
        }
        let d = died.expect("the ball fell");
        while sim.tick < d + DEATH_SECTOR_BEGIN_TICKS {
            sim.step(0);
        }
        let back = sim.world.state(b).unwrap().pos;
        assert!(back.distance(start) < 0.1, "the box is at {back:?}, its place is {start:?}");
    }

    #[test]
    fn a_module_is_made_of_its_parts_at_its_stand_in_and_its_stone_slides() {
        // Module 34 as a file: a box and a stone (cubes of half size 1) one above the other, and the slider's frames
        // on a vertical axis through the stone. Its stand-in in sector 1, turned a quarter about y, 20 along x.
        let mut file = flat_level();
        file.objects.clear();
        file.groups.clear();
        let kiste = add_cube(&mut file, "P_Modul_34_Kiste", Vec3::new(-7.0, 1.0, 0.0), &[]);
        let stein = add_cube(&mut file, "P_Modul_34_Schiebestein", Vec3::new(-7.0, 3.0, 0.0), &[]);
        file.meshes[file.objects[kiste].mesh.unwrap()].name = "P_Modul_34_Kiste_Mesh".into();
        file.meshes[file.objects[stein].mesh.unwrap()].name = "P_Modul_34_Schiebestein_Mesh".into();
        for (name, y) in [("P_Modul_34_Slider_Frame01", 6.0), ("P_Modul_34_Slider_Frame02", -6.0)] {
            let o = add_cube(&mut file, name, Vec3::new(-7.0, y, 0.0), &[]);
            file.objects[o].mesh = None;
        }
        let mut level = flat_level();
        let stand_in = add_cube(&mut level, "P_Modul_34_01", Vec3::ZERO, &["P_Modul_34", "Sector_01"]);
        level.objects[stand_in].matrix = [
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [20.0, 0.0, 0.0, 1.0],
        ];
        let mut modules = Modules::default();
        modules.models.push(file);
        modules.names.push("P_Modul_34".into());
        let mut sim = Sim::with_modules(&level, BallKind::Wood, &modules, false);
        let parts = sim.module_bodies().to_vec();
        assert_eq!(parts.len(), 2);
        // the stand-in's matrix times the part's: (-7, 1, 0) turned a quarter about y is (0, 1, 7), moved to x = 20
        let k = sim.world.state(parts[0].body).expect("the box is made").pos;
        assert!((k - Vec3::new(20.0, 1.0, 7.0)).length() < 1e-4, "box at {k:?}");
        // the ball is within 50 of the stone: the module's script wakes it, once
        sim.step(0);
        assert_ne!(sim.world.movement_state(parts[1].body), Some(8), "the stone was not woken");
        // take the box away: the stone drops along its slider onto the floor, straight down
        sim.world.deactivate(parts[0].body);
        for _ in 0..120 {
            sim.step(0);
        }
        let st = sim.world.state(parts[1].body).unwrap().pos;
        assert!((st - Vec3::new(20.0, 1.0, 7.0)).length() < 0.05, "stone at {st:?}");
    }

    #[test]
    fn same_input_same_run() {
        let level = flat_level();
        let run = || {
            let mut sim = Sim::new(&level, BallKind::Stone);
            for t in 0..600u32 {
                let input = match t / 60 % 4 {
                    0 => input::UP,
                    1 => input::UP | input::RIGHT,
                    2 => input::SHIFT | input::LEFT,
                    _ => input::DOWN,
                };
                sim.step(input);
            }
            (
                sim.ball.position,
                sim.ball.velocity,
                sim.camera.yaw,
                sim.tick,
            )
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn falling_off_costs_a_life_and_respawns() {
        let level = flat_level();
        let mut sim = Sim::new(&level, BallKind::Paper);
        sim.put_ball(Vec3::new(500.0, 10.0, 0.0));
        for _ in 0..600 {
            sim.step(0);
            if sim.lives < 3 {
                break;
            }
        }
        assert_eq!(sim.lives, 2);
        assert!(sim.events.iter().any(|e| e.1 == Event::Died));
        for _ in 0..DEATH_RESPAWN_TICKS {
            sim.step(0);
        }
        assert!(sim.ball.position.distance(Vec3::new(0.0, BALL_RADIUS, 0.0)) < 1.0);
    }

    #[test]
    fn last_ball_lost_ends_the_run() {
        let level = flat_level();
        let mut sim = Sim::new(&level, BallKind::Wood);
        for lost in 1..=3 {
            sim.put_ball(Vec3::new(500.0, 10.0, 0.0));
            for _ in 0..600 {
                sim.step(0);
                if sim.events.iter().filter(|e| e.1 == Event::Died).count() >= lost && sim.dying.is_none() {
                    break;
                }
            }
        }
        assert_eq!(sim.lives, 0);
        assert!(sim.game_over, "three balls lost, the run must be over");
    }

    #[test]
    fn god_modes_endless_lives_keep_the_last_ball() {
        let level = flat_level();
        let mut sim = Sim::new(&level, BallKind::Wood);
        sim.mods = mods_of("name.en God\non tick\ndo lives.endless 1\n");
        for lost in 1..=5 {
            sim.put_ball(Vec3::new(500.0, 10.0, 0.0));
            for _ in 0..600 {
                sim.step(0);
                if sim.events.iter().filter(|e| e.1 == Event::Died).count() >= lost && sim.dying.is_none() {
                    break;
                }
            }
        }
        assert!(!sim.game_over && sim.lives == 1, "five balls lost, the last one kept: {} lives", sim.lives);
        assert!(sim.lives_spared, "the run says it was helped");
        assert!(sim.mods.modded(), "and is modded");
    }

    /// Level 1's dome (P_Dome_01, sector 2) with the player's copy (skipped without OPENBALLANCE_DATA): the wooden
    /// ball at rest 6 in front of it and pushed straight at it climbs it and rolls over, as in the original (run
    /// under Wine with the start moved there) and in the C reference, where it is 7.85 high 5 ticks after hitting
    /// it and 10.71 high over its top at z -507.9. Made of the level's stand-in (format 4), the coarse, lower dome
    /// stopped it.
    #[test]
    fn level_1_dome_is_climbed_as_in_the_original() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(1)).expect("level 1").level;
        let modules = crate::scene::load_modules(&level, &data);
        let run = |format: u32| {
            let mut sim = Sim::with_rules(&level, BallKind::Wood, &modules, level.settings.tutorial, format);
            sim.start_from(1, None, Some(Vec3::new(74.79, 8.0, -494.0)));
            let mut top = (0.0f32, 0.0f32);
            for t in 1..=560 {
                sim.step(if t >= 367 { input::RIGHT } else { 0 });
                let p = sim.ball.position;
                if p.y > top.0 {
                    top = (p.y, p.z);
                }
            }
            (top, sim.ball.position)
        };
        let ((y, z), _) = run(5);
        assert!(y > 10.5 && (z + 507.8).abs() < 1.5, "over the dome's top at y {y}, z {z}");
        let ((y, _), p) = run(4);
        assert!(y < 9.0 && p.z > -502.0, "format 4: up to y {y}, at {p:?}");
    }

    /// Level 10's first sector with the player's copy (skipped without OPENBALLANCE_DATA): the way on from the spur
    /// at the hinged bridge (P_Modul_37_01), where a wooden ball is too light to tip it. Rammed by the wooden ball, the
    /// pusher (P_Modul_01_01) clears its pegs from the loop's west side enough to pass; the rails behind lead to the
    /// stone transformer P_Trafo_Stone_01; back round the loop the stone ball tips the bridge, and over it the ball
    /// rolls on south (hints/Level_10.txt; the inputs of hints/Level_10-s1-bridge.replay, from the spur as wood).
    #[test]
    fn level_10_s_bridge_is_crossed_after_the_stone_transformer_behind_the_pusher() {
        let Some(data) = std::env::var_os("OPENBALLANCE_DATA").and_then(|_| crate::assets::Data::locate(None).ok())
        else {
            return;
        };
        let level = data.load(&crate::assets::Source::Original(10)).expect("level 10").level;
        let modules = crate::scene::load_modules(&level, &data);
        let run = crate::replay::Replay::parse(include_str!("../../../hints/Level_10-s1-bridge.replay")).expect("replay");
        let start = run.start.clone().expect("a run from the spur");
        let mut sim = Sim::with_rules(&level, BallKind::Wood, &modules, false, run.version);
        sim.start_from(start.sector - 1, start.ball.as_deref().and_then(BallKind::from_name), start.at.map(Vec3::from));
        let mut stone_at = None;
        for t in 1..=run.end {
            sim.step(run.input_at(t));
            if stone_at.is_none() && sim.ball.kind == BallKind::Stone {
                stone_at = Some(sim.ball.position);
            }
        }
        let trafo = Vec3::new(331.7, -9.6, -89.6);
        assert!(
            stone_at.is_some_and(|p| (p - trafo).length() < TRAFO_REACH + 1.0),
            "made stone at {stone_at:?}, not at P_Trafo_Stone_01"
        );
        let p = sim.ball.position;
        assert_eq!(sim.lives, 3, "a ball was lost");
        // south of the gap the bridge spans (z -78 to -65 at x 291.5), on the floor
        assert!(p.z > -60.0 && (p.x - 291.5).abs() < 2.0 && p.y > -9.0, "the ball at {p:?}");
    }

}
