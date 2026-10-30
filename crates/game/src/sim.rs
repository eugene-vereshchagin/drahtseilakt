// The simulation: ball, camera and game rules, advanced in fixed 60 Hz ticks from an input bit set. It knows
// nothing of windows or rendering, so a replay (the inputs of every tick) reproduces a run exactly, headless
// or on screen.
//
// The physics is written from scratch and tuned by hand for now; phase 3 measures the original under Wine
// and fits these numbers to reference trajectories.

use crate::collide::{Collider, Surface, Triangle};
use glam::{Quat, Vec3};
use level::Level;

pub const TICK: f32 = 1.0 / 60.0;
const SUBSTEPS: u32 = 4;
pub const BALL_RADIUS: f32 = 2.0;
const GRAVITY: f32 = 20.0;

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
    /// Mass, push acceleration, linear damping per second, bounce, rolling friction.
    fn params(self) -> (f32, f32, f32, f32) {
        match self {
            BallKind::Wood => (14.0, 0.35, 0.25, 0.6),
            BallKind::Stone => (10.0, 0.30, 0.10, 0.8),
            BallKind::Paper => (24.0, 1.80, 0.40, 0.4),
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

#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub kind: BallKind,
    pub on_ground: bool,
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

pub struct Sim {
    collider: Collider,
    rules: Rules,
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
}

const PICK_RADIUS: f32 = 3.5;
const GATE_RADIUS: f32 = 7.0;

impl Sim {
    pub fn new(level: &Level, start_ball: BallKind) -> Sim {
        let collider = Collider::new(collision_triangles(level));
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
            if let Some(rest) = g.name.strip_prefix("P_Trafo_") {
                if let Some(kind) = BallKind::from_name(rest) {
                    for &m in &g.members {
                        if let Some(o) = level.objects.get(m) {
                            trafos.push((m, Vec3::from(o.position()), kind));
                        }
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
            death_y: collider.min_y - 60.0,
        };
        // The camera starts looking the way out of the start: where the physical floor around the start leads
        // (the mean direction to floor vertices in a ring around it), in quarter turns as the original camera
        // turns. The start object's axes do not tell this reliably (they agree in 7 of 13 levels). Without a
        // clear way out: +X, the most common in the original levels.
        let ring = collider
            .triangles
            .iter()
            .flat_map(|t| [t.a, t.b, t.c])
            .fold(Vec3::ZERO, |acc, v| {
                let d = Vec3::new(v.x - start.x, 0.0, v.z - start.z);
                let len = d.length();
                if (14.0..45.0).contains(&len) && (v.y - start.y).abs() < 20.0 {
                    acc + d / len
                } else {
                    acc
                }
            });
        let start_axis = if ring.length() > 1.0 { ring } else { Vec3::X };
        let quarter =
            (start_axis.x.atan2(start_axis.z) / std::f32::consts::FRAC_PI_2).round() as i32;
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
            kind: start_ball,
            on_ground: false,
        };
        let mut camera = Camera {
            quarter,
            yaw,
            lift: 0.0,
            position: Vec3::ZERO,
            target: ball.position,
        };
        camera.position = camera_goal(&camera, ball.position);
        Sim {
            collider,
            rules,
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
        }
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

        let (accel, damping, bounce, roll) = self.ball.kind.params();
        let dt = TICK / SUBSTEPS as f32;
        for _ in 0..SUBSTEPS {
            self.ball.velocity += push * accel * dt;
            self.ball.velocity.y -= GRAVITY * dt;
            self.ball.velocity *= 1.0 - damping * dt;
            self.ball.position += self.ball.velocity * dt;
            self.resolve(bounce, roll, dt);
        }
        // Rolling look: spin about the axis perpendicular to motion and the ground.
        let horizontal = Vec3::new(self.ball.velocity.x, 0.0, self.ball.velocity.z);
        let axis = Vec3::Y.cross(horizontal);
        if axis.length_squared() > 1e-8 {
            let angle = horizontal.length() * TICK / BALL_RADIUS;
            // Left-handed coordinates turn the other way round.
            self.ball.rotation =
                (Quat::from_axis_angle(axis.normalize(), -angle) * self.ball.rotation).normalize();
        }

        self.follow_camera();
        self.apply_rules();
        if self.tick.is_multiple_of(30) && self.points > 0 {
            self.points -= 1;
        }
    }

    fn resolve(&mut self, bounce: f32, roll: f32, dt: f32) {
        self.ball.on_ground = false;
        for _ in 0..4 {
            let contacts = self.collider.contacts(self.ball.position, BALL_RADIUS);
            let Some(c) = contacts.first() else { break };
            self.ball.position += c.normal * c.depth;
            let vn = self.ball.velocity.dot(c.normal);
            if vn < 0.0 {
                let e = if -vn > 4.0 { bounce } else { 0.0 };
                self.ball.velocity -= c.normal * vn * (1.0 + e);
            }
            if c.normal.y > 0.3 {
                self.ball.on_ground = true;
            }
            // Rolling resistance along the surface.
            let tangential = self.ball.velocity - c.normal * self.ball.velocity.dot(c.normal);
            let factor = match c.surface {
                Surface::Rail => roll * 0.7,
                _ => roll,
            };
            self.ball.velocity -= tangential * (factor * dt).min(1.0);
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
            if self.ball.kind != kind && near(q, PICK_RADIUS + 1.0) {
                self.ball.kind = kind;
                self.events.push((self.tick, Event::Transform(kind)));
            }
        }
        // Checkpoints in order: reaching the next one opens the next sector.
        if let Some(&(_, q)) = self.rules.checkpoints.get(self.sector) {
            if near(q, GATE_RADIUS) {
                self.sector += 1;
                self.events
                    .push((self.tick, Event::Checkpoint(self.sector)));
            }
        }
        if let Some(end) = self.rules.end {
            if self.sector >= self.rules.checkpoints.len() && near(end, GATE_RADIUS) {
                self.finished = true;
                self.events.push((self.tick, Event::Finished));
            }
        }
        if p.y < self.rules.death_y {
            self.die();
        }
    }

    fn die(&mut self) {
        self.events.push((self.tick, Event::Died));
        if self.lives == 0 {
            self.game_over = true;
            self.events.push((self.tick, Event::GameOver));
            return;
        }
        self.lives -= 1;
        let spawn = self
            .rules
            .resets
            .get(self.sector)
            .copied()
            .unwrap_or(self.rules.start + Vec3::Y * BALL_RADIUS);
        self.ball.position = spawn;
        self.ball.velocity = Vec3::ZERO;
        self.camera.position = camera_goal(&self.camera, self.ball.position);
        self.camera.target = self.ball.position;
    }
}

fn camera_goal(cam: &Camera, ball: Vec3) -> Vec3 {
    let back = -cam.forward();
    let (dist, height) = (22.0 - 12.0 * cam.lift, 12.0 + 38.0 * cam.lift);
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
        let surface = match g.name.to_ascii_lowercase().as_str() {
            "phys_floors" => Surface::Floor,
            "phys_floorrails" => Surface::Rail,
            "phys_floorstopper" => Surface::Stopper,
            n if n.starts_with("phys_") => Surface::Wood,
            // Modules and boxes are solid for now; their own behaviour comes with phase 4.
            n if n.starts_with("p_modul") || n == "p_box" => Surface::Floor,
            _ => continue,
        };
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
                for t in part.indices.chunks_exact(3) {
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
                    out.push(Triangle {
                        a,
                        b,
                        c,
                        normal: n.normalize(),
                        surface,
                    });
                }
            }
        }
    }
    out
}
