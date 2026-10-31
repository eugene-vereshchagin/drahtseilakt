// The simulation: ball, camera and game rules, advanced in fixed 60 Hz ticks from an input bit set. It knows
// nothing of windows or rendering, so a replay (the inputs of every tick) reproduces a run exactly, headless
// or on screen.
//
// The physics is written from scratch and tuned by hand for now; phase 3 measures the original under Wine
// and fits these numbers to reference trajectories.

use crate::collide::Triangle;
use crate::physics::{Material, World};
use glam::{Mat3, Quat, Vec3};
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

/// The game camera: behind the ball, turned in quarter turns, optionally lifted.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Target heading in quarter turns, and the smoothed heading in radians.
    pub quarter: i32,
    pub yaw: f32,
    pub lift: f32,
    pub position: Vec3,
    pub target: Vec3,
}

impl Camera {
    /// Forward on the ground plane: where "up" pushes the ball.
    pub fn forward(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos())
    }
    pub fn right(&self) -> Vec3 {
        // Left-handed, Y up: right is forward turned a quarter clockwise seen from above.
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
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
}

/// What the rules need from the level, resolved once.
#[derive(Clone)]
struct Rules {
    start: Vec3,
    checkpoints: Vec<(usize, Vec3)>,
    resets: Vec<Vec3>,
    end: Option<Vec3>,
    extra_points: Vec<(usize, Vec3)>,
    extra_lives: Vec<(usize, Vec3)>,
    trafos: Vec<(usize, Vec3, BallKind)>,
    death_y: f32,
}

/// The ball inside a transformer: out of the physics until `release`, then a new ball of its new kind at `at`.
#[derive(Clone, Copy, Debug)]
struct TrafoHold {
    release: u64,
    at: Vec3,
}

pub struct Sim {
    rules: Rules,
    world: World,
    /// The loose bodies (boxes and loose balls): (level object index, body index in the world).
    loose: Vec<(usize, usize)>,
    pub ball: Ball,
    pub camera: Camera,
    pub tick: u64,
    pub lives: u32,
    pub points: i64,
    pub sector: usize,
    pub finished: bool,
    pub game_over: bool,
    /// Objects taken (extra points and lives) are hidden by the renderer.
    pub taken: Vec<usize>,
    pub events: Vec<(u64, Event)>,
    prev_input: u8,
    /// The ball inside a transformer, if it is in one.
    hold: Option<TrafoHold>,
}

const PICK_RADIUS: f32 = 3.5;
const GATE_RADIUS: f32 = 7.0;

impl Sim {
    pub fn new(level: &Level, start_ball: BallKind) -> Sim {
        let triangles = collision_triangles(level);
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
                        trafos.push((m, Vec3::from(o.position()), kind));
                    }
                }
            }
        }
        let rules = Rules {
            start,
            checkpoints: pos("PC_Checkpoints"),
            resets: sorted_by_name(level, "PR_Resetpoints"),
            end: pos("PE_Levelende").first().map(|p| p.1),
            extra_points: pos("P_Extra_Point"),
            extra_lives: pos("P_Extra_Life"),
            trafos,
            death_y: triangles
                .iter()
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
        let ball = Ball {
            position: spawn,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            spin: Vec3::ZERO,
            kind: start_ball,
            on_ground: false,
        };
        let mut world = World::new(&triangles);
        // Body 0 is the ball.
        let ball_body = world.add_ball(BALL_RADIUS, ball.material(), ball.position, ball.rotation);
        debug_assert_eq!(ball_body, 0);
        // Loose objects that move: boxes and the loose balls (their material table in docs/physics.md, section 3).
        let mut loose = Vec::new();
        for (i, o) in level.objects.iter().enumerate() {
            let Some(mesh) = o.mesh.and_then(|m| level.meshes.get(m)) else {
                continue;
            };
            let name = o.name.to_ascii_lowercase();
            let m = Mat3::from_cols(
                Vec3::new(o.matrix[0][0], o.matrix[0][1], o.matrix[0][2]),
                Vec3::new(o.matrix[1][0], o.matrix[1][1], o.matrix[1][2]),
                Vec3::new(o.matrix[2][0], o.matrix[2][1], o.matrix[2][2]),
            );
            let (pos, rot) = (Vec3::from(o.position()), Quat::from_mat3(&m).normalize());
            let body = if level.members("P_Box").any(|(k, _)| k == i) {
                let half = mesh
                    .positions
                    .iter()
                    .fold(Vec3::ZERO, |h, p| h.max(Vec3::from(*p).abs()));
                let material = Material {
                    friction: 0.7,
                    elasticity: 0.3,
                    mass: 1.0,
                    lin_damp: 0.1,
                    rot_damp: 0.1,
                };
                world.add_box(half, material, pos, rot)
            } else if let Some(kind) = name
                .strip_prefix("p_ball_")
                .and_then(|_| BallKind::from_name(&name))
            {
                let radius = mesh
                    .positions
                    .iter()
                    .fold(0.0f32, |r, p| r.max(Vec3::from(*p).length()));
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
                world.add_sphere(radius, material, pos, rot)
            } else {
                continue;
            };
            loose.push((i, body));
        }
        let mut camera = Camera {
            quarter,
            yaw,
            lift: 0.0,
            position: Vec3::ZERO,
            target: ball.position,
        };
        camera.position = camera_goal(&camera, ball.position);
        Sim {
            rules,
            world,
            loose,
            ball,
            camera,
            tick: 0,
            lives: 3,
            points: 1000,
            sector: 0,
            finished: false,
            game_over: false,
            taken: Vec::new(),
            events: Vec::new(),
            prev_input: 0,
            hold: None,
        }
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

    pub fn step(&mut self, input: u8) {
        if self.finished || self.game_over {
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
        let goal_yaw = self.camera.quarter as f32 * std::f32::consts::FRAC_PI_2;
        self.camera.yaw += (goal_yaw - self.camera.yaw) * (1.0 - (-TICK * 8.0f32).exp());
        let lift_goal = if input & input::LIFT != 0 { 1.0 } else { 0.0 };
        self.camera.lift += (lift_goal - self.camera.lift) * (1.0 - (-TICK * 6.0f32).exp());

        // The push: arrows relative to the camera (Left / Right only when Shift is not held).
        let mut push = Vec3::ZERO;
        if input & input::UP != 0 {
            push += self.camera.forward();
        }
        if input & input::DOWN != 0 {
            push -= self.camera.forward();
        }
        if input & input::SHIFT == 0 {
            if input & input::RIGHT != 0 {
                push += self.camera.right();
            }
            if input & input::LEFT != 0 {
                push -= self.camera.right();
            }
        }
        let push = push.normalize_or_zero();

        // The push is the original's control force, a SetPhysicsForce: the physics applies an impulse of the force at
        // every PSI (66 a second of its time), while the ball is awake. The ball's force is its material's.
        // The physics owns the ball's state; the game only reads it back. Only a new ball (a respawn, the end of a
        // transformer) puts a new body in, as the original does.
        let p = self.ball.kind.params();
        self.world.set_push(0, push * p.force);
        self.world.step(None);
        if let Some(h) = self.hold {
            // Out of the physics, the ball is drawn moving to the transformer's hold point.
            self.ball.position += (h.at - self.ball.position) * (1.0 - (-TICK * 6.0f32).exp());
            if self.tick >= h.release {
                self.hold = None;
                self.ball.position = h.at;
                self.ball.velocity = Vec3::ZERO;
                self.ball.spin = Vec3::ZERO;
                self.world.replace_ball(0, BALL_RADIUS, self.ball.material(), h.at, self.ball.rotation);
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
        if self.tick.is_multiple_of(30) && self.points > 0 {
            self.points -= 1;
        }
    }

    fn follow_camera(&mut self) {
        let goal = camera_goal(&self.camera, self.ball.position);
        let k = 1.0 - (-TICK * 5.0f32).exp();
        self.camera.position += (goal - self.camera.position) * k;
        self.camera.target +=
            (self.ball.position - self.camera.target) * (1.0 - (-TICK * 12.0f32).exp());
    }

    fn apply_rules(&mut self) {
        let p = self.ball.position;
        let near = |q: Vec3, r: f32| p.distance_squared(q) < r * r;

        for &(i, q) in &self.rules.extra_points {
            if !self.taken.contains(&i) && near(q, PICK_RADIUS) {
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
        for &(_, q, kind) in &self.rules.trafos {
            if self.hold.is_none() && self.ball.kind != kind && near(q, PICK_RADIUS + 1.0) {
                // The original takes the ball out of the physics and, after a while, makes a ball of the new kind at
                // rest above the transformer.
                self.ball.kind = kind;
                self.world.remove(0);
                self.ball.velocity = Vec3::ZERO;
                self.ball.spin = Vec3::ZERO;
                self.hold = Some(TrafoHold {
                    release: self.tick + TRAFO_TICKS,
                    at: q + TRAFO_HOLD,
                });
                self.events.push((self.tick, Event::Transform(kind)));
            }
        }
        // Checkpoints in order: reaching the next one opens the next sector.
        if let Some(&(_, q)) = self.rules.checkpoints.get(self.sector)
            && near(q, GATE_RADIUS)
        {
            self.sector += 1;
            self.events
                .push((self.tick, Event::Checkpoint(self.sector)));
        }
        if let Some(end) = self.rules.end
            && self.sector >= self.rules.checkpoints.len()
            && near(end, GATE_RADIUS)
        {
            self.finished = true;
            self.events.push((self.tick, Event::Finished));
        }
        if p.y < self.rules.death_y {
            self.die();
        }
    }

    fn die(&mut self) {
        self.events.push((self.tick, Event::Died));
        // Lives are balls: the last ball lost ends the run.
        self.lives = self.lives.saturating_sub(1);
        if self.lives == 0 {
            self.game_over = true;
            self.events.push((self.tick, Event::GameOver));
            return;
        }
        let spawn = self
            .rules
            .resets
            .get(self.sector)
            .copied()
            .unwrap_or(self.rules.start + Vec3::Y * BALL_RADIUS);
        self.ball.position = spawn;
        self.ball.velocity = Vec3::ZERO;
        self.ball.spin = Vec3::ZERO;
        self.hold = None;
        self.world
            .replace_ball(0, BALL_RADIUS, self.ball.material(), spawn, self.ball.rotation);
        // The camera comes back as it was at the start: the start heading, not lifted, no turn in progress.
        self.camera.quarter = START_QUARTER;
        self.camera.yaw = START_QUARTER as f32 * std::f32::consts::FRAC_PI_2;
        self.camera.lift = 0.0;
        self.camera.position = camera_goal(&self.camera, self.ball.position);
        self.camera.target = self.ball.position;
    }
}

fn camera_goal(cam: &Camera, ball: Vec3) -> Vec3 {
    let back = -cam.forward();
    // The original's in-game camera sits 22 back and 35 up from the ball, looking at its centre (C reference,
    // InGameCam at level 1: offset (-22, +35, 0)); Space lifts it further.
    let (dist, height) = (22.0 - 12.0 * cam.lift, 35.0 + 38.0 * cam.lift);
    ball + back * dist + Vec3::Y * height
}

fn sorted_by_name(level: &Level, group: &str) -> Vec<Vec3> {
    let mut v: Vec<(String, Vec3)> = level
        .members(group)
        .map(|(_, o)| (o.name.clone(), Vec3::from(o.position())))
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v.into_iter().map(|x| x.1).collect()
}

/// Triangles of the physical objects, in world space.
fn collision_triangles(level: &Level) -> Vec<Triangle> {
    let mut out = Vec::new();
    for g in &level.groups {
        // The player ball (group Ball) does not collide with objects of its own group: floor stoppers are such
        // objects, so they stop boxes and loose balls but not the ball (docs/physics.md, section 3.4).
        let solid = match g.name.to_ascii_lowercase().as_str() {
            "phys_floors" | "phys_floorrails" => true,
            "phys_floorstopper" => false,
            // Modules are solid for now; their own behaviour comes with phase 4. Boxes are bodies (see `Sim::new`).
            n if n.starts_with("p_modul") => true,
            _ => false,
        };
        if !solid {
            continue;
        }
        for &m in &g.members {
            let Some(o) = level.objects.get(m) else {
                continue;
            };
            let Some(mesh) = o.mesh.and_then(|i| level.meshes.get(i)) else {
                continue;
            };
            let mat = glam::Mat4::from_cols_array_2d(&o.matrix);
            let world: Vec<Vec3> = mesh
                .positions
                .iter()
                .map(|p| mat.transform_point3(Vec3::from(*p)))
                .collect();
            for part in &mesh.parts {
                for t in part.indices.as_chunks::<3>().0 {
                    let v = |k: usize| {
                        t.get(k)
                            .and_then(|&i| world.get(i as usize))
                            .copied()
                            .unwrap_or_default()
                    };
                    let (a, b, c) = (v(0), v(1), v(2));
                    let n = (b - a).cross(c - a);
                    if n.length_squared() < 1e-12 {
                        continue;
                    }
                    out.push(Triangle { a, b, c });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use level::{Group, Level, Material, Mesh, Object, Part};

    /// A flat 200×200 floor at y = 0 in Phys_Floors, a start at the origin, a reset point a ball's radius up.
    fn flat_level() -> Level {
        let s = 100.0;
        let floor = Mesh {
            name: "floor".into(),
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
        };
        Level {
            meshes: vec![floor],
            materials: vec![Material {
                name: "m".into(),
                diffuse: [1.0; 4],
                ambient: [0.0; 3],
                emissive: [0.0; 3],
                texture: None,
                blend: None,
                alpha_cutoff: None,
                z_write: true,
                two_sided: false,
                unlit: false,
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
        }
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
        });
        level.groups.push(Group {
            name: "P_Trafo_Stone".into(),
            members: vec![trafo],
        });
        let mut sim = Sim::new(&level, BallKind::Wood);
        sim.step(0);
        assert!(sim.events.contains(&(1, Event::Transform(BallKind::Stone))));
        assert!(sim.world.state(0).is_none(), "the ball left the physics");
        for _ in 1..TRAFO_TICKS {
            sim.step(0);
            assert!(sim.world.state(0).is_none());
        }
        sim.step(0);
        let s = sim.world.state(0).expect("a new ball");
        assert_eq!(sim.ball.kind, BallKind::Stone);
        assert!((s.pos - TRAFO_HOLD).length() < 0.1, "made at {:?}", s.pos);

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
                if sim.events.iter().filter(|e| e.1 == Event::Died).count() >= lost {
                    break;
                }
            }
        }
        assert_eq!(sim.lives, 0);
        assert!(sim.game_over, "three balls lost, the run must be over");
    }

    #[test]
    fn a_wooden_ball_barely_bounces() {
        let level = flat_level();
        let mut sim = Sim::new(&level, BallKind::Wood);
        sim.put_ball(Vec3::new(0.0, 20.0, 0.0));
        let mut impact = 0.0f32;
        let mut rebound = 0.0f32;
        let mut was_falling = false;
        for _ in 0..240 {
            let before = sim.ball.velocity.y;
            sim.step(0);
            if before < -1.0 && sim.ball.velocity.y > 0.0 {
                // The first contact: speed before and after reflection.
                impact = impact.max(-before);
                rebound = rebound.max(sim.ball.velocity.y);
                was_falling = true;
                break;
            }
        }
        assert!(was_falling, "the ball never landed");
        // The impact restitutes at sqrt(e) (docs/ivp_contact.md 6): the pair's elasticity is 0.2 × 0.3 = 0.06, and
        // sqrt(0.06) = 0.245 of the impact speed comes back.
        assert!(
            (rebound / impact - 0.06f32.sqrt()).abs() < 0.02,
            "rebound {rebound} from impact {impact}"
        );
    }
}
