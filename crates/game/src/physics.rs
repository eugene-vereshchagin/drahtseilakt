// The physics of the game: the player's ball, the level's loose boxes and balls, and the level's physical geometry.
// The dynamics are IVP's (crates/ivp, a port of OpenBallance's physics, GPL-3.0): each of the level's physical
// objects as a fixed body of its own, as the original physicalizes them, the boxes as convex surfaces, the balls as spheres, with the original game's material values
// (docs/physics.md, sections 3 and 4). A tick is one step of 1/60 s, which the physics runs as its own time steps of
// 1/66 s.
// Reason for the allow: physics glue over the validated level model and fixed tables.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use crate::collide::StaticMesh;
use glam::{Quat, Vec3};
use ivp::body::{BodyDesc, BodyState, Shape};
use ivp::core::World as IvpWorld;
use ivp::ledge::{self, Surface};
use ivp::math::Quat as IvpQuat;

/// Steps per second of the original's own time (its "PSI"); its control forces are impulses per step.
/// Gravity of the game (docs/physics.md, section 1).
pub const GRAVITY: f32 = 20.0;
/// Elasticity and friction of the level's floors and rails and of PH modules (`Physicalize_Floors`).
pub const SURFACE_ELASTICITY: f32 = 0.3;
pub const SURFACE_FRICTION: f32 = 0.7;
/// One tick, in seconds.
const TICK: f64 = 1.0 / 60.0;

/// The physics time of one frame as the original's physics manager takes it: the frame time in milliseconds as a
/// float, times the time factor over a thousand in float (the original's level 1 played at 60 frames a second:
/// 16.666666 ms * 0.002 = 0.033333335 s at time factor 2, one float step over 1/30). Measured on the original
/// (2026-11-22, calibration runs on levels 1 to 4): with this step the frame ends keep their place on the PSI grid
/// over a whole run, as the original's do (a key pressed in a frame that ends just after a PSI misses that PSI on
/// level 3); the smoothed step 0.033333331 drifts by 0.0005 PSI in 2400 frames and gives that PSI the push.
fn frame_step(time_factor: f64) -> f64 {
    let frame_ms = (TICK * 1000.0) as f32;
    let factor = time_factor as f32 * 0.001f32;
    f64::from(frame_ms * factor)
}
/// The physics runs this many times faster than the frames while playing: the original's "Set Physics Globals" sets
/// the time factor to 2 (checked in the C game: its physics time advances 2 s for every second of frames).
pub const TIME_FACTOR: f64 = 2.0;

/// The collision group of the player's ball and of the floor stoppers (docs/physics.md 3.4).
pub const GROUP_BALL: &str = "Ball";
/// The collision group of the level's floors and rails.
pub const GROUP_FLOOR: &str = "Floor";

/// A group name as the physics keeps it: its first 8 bytes, NUL padded.
fn group(name: &str) -> [u8; 8] {
    let mut g = [0u8; 8];
    for (d, s) in g.iter_mut().zip(name.bytes()) {
        *d = s;
    }
    g
}

/// The material of a body, as the original's tables give it.
#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub friction: f32,
    pub elasticity: f32,
    pub mass: f32,
    pub lin_damp: f32,
    pub rot_damp: f32,
}

/// What the rest of the game needs of a body: where it is and how it is turned.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub pos: Vec3,
    pub rot: Quat,
}

/// The full state of a body that the simulation reads and writes.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub pos: Vec3,
    pub vel: Vec3,
    pub rot: Quat,
    pub spin: Vec3,
}

/// A body's sound material, as the original's attributes give it: the hit ("Coll Detection ID": 1 stone, 2 wood,
/// 3 metal, 4 dome) and the roll ("Continuous Contact ID": 1 stone, 2 wood, 3 metal); 0 for none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SoundIds {
    pub hit: u8,
    pub roll: u8,
}

/// What the ball met at the last step, for its sounds: the impacts (the other body's hit material and the
/// relative speed) and the materials of the bodies it has friction contacts with, one per contact point.
#[derive(Clone, Debug, Default)]
pub struct BallSounds {
    pub impacts: Vec<(u8, f32)>,
    pub rolls: Vec<u8>,
}

#[derive(Clone)]
pub struct World {
    physics: IvpWorld,
    /// The sound material of each game body.
    sounds: Vec<SoundIds>,
    /// What the ball met at the last step.
    pub ball_sounds: BallSounds,
    /// The IVP body of each of the game's bodies, in creation order; none while a body is taken out of the physics
    /// (the ball inside a transformer).
    handles: Vec<Option<usize>>,
    /// Poses after the last step, for drawing.
    pub bodies: Vec<Body>,
    /// Whether body 0 (the ball) is touching something.
    pub grounded: bool,
    /// The push controller of each body that has been pushed (see `set_push`).
    pushers: Vec<Option<usize>>,
    /// The gravity and time factor in use: the original's unless a mod's rule changes them (modrt.rs).
    gravity: f32,
    gravity_xz: [f32; 2],
    time_factor: f64,
    /// The IVP bodies of the level's physical objects, with whether the player's ball collides with them.
    statics: Vec<(usize, bool, SoundIds)>,
    /// How each game body is made, for the ones that come and go (a sector's objects); none for the ball.
    spawns: Vec<Option<Spawn>>,
    /// For each mesh given to `new`, the IVP body made of it (none: no usable triangles).
    baked_of_mesh: Vec<Option<usize>>,
    /// The level objects (rows of the entity table) whose body is one of the baked statics, by row (`tag_baked`).
    baked_rows: std::collections::BTreeMap<usize, usize>,
    /// The fixed bodies scripts made at run time, by handle (slots are never reused): see `add_static_*`.
    script_statics: Vec<Option<ScriptStatic>>,
    /// What is needed to make a baked body again somewhere else (a script moved its object), by IVP body.
    baked_info: std::collections::BTreeMap<usize, BakedInfo>,
}

/// A level object's baked fixed body, kept so that it can follow its object.
#[derive(Clone)]
struct BakedInfo {
    surface: std::sync::Arc<Surface>,
    pos: Vec3,
    rot: Quat,
    group: [u8; 8],
}

/// A fixed body made by a script: kept whole so that it can be put somewhere else (the body is made again, at the
/// same place in `statics`).
#[derive(Clone)]
struct ScriptStatic {
    id: usize,
    shape: Shape,
    pos: Vec3,
    rot: Quat,
    friction: f32,
    elasticity: f32,
    group: [u8; 8],
    ball_collides: bool,
}

/// What the physics needs to make a game body again at its start: the original physicalizes a sector's objects when
/// the sector begins and again after a lost ball, at their places in the level.
#[derive(Clone, Debug)]
struct Spawn {
    shape: Shape,
    m: Material,
    pos: Vec3,
    rot: Rot,
    /// The mass centre in the body's frame (`None`: the shape's own, as "Auto Mass Center").
    shift: Option<[f32; 3]>,
    group: [u8; 8],
    fixed: bool,
    /// Made asleep (a module's movable part waits to be touched).
    frozen: bool,
    /// In the collision system at all (FixCube is not).
    collide: bool,
}

fn p3(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn vec3_f64(v: Vec3) -> [f64; 3] {
    [f64::from(v.x), f64::from(v.y), f64::from(v.z)]
}

fn vec3(v: [f64; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

fn quat(q: IvpQuat) -> Quat {
    Quat::from_xyzw(q.x as f32, q.y as f32, q.z as f32, q.w as f32)
}

/// A body's rotation: a level object's matrix without its scale, which the physics turns into its quaternion as the
/// original's glue does (`ivp_matrix_to_quat` in double precision), or a quaternion for a body the game makes itself.
#[derive(Clone, Copy, Debug)]
pub enum Rot {
    Basis(glam::Mat3),
    Quat(Quat),
}

impl From<Quat> for Rot {
    fn from(q: Quat) -> Rot {
        Rot::Quat(q)
    }
}

impl Rot {
    /// The rotation for drawing and for the game's own geometry.
    pub fn quat(self) -> Quat {
        match self {
            Rot::Basis(m) => Quat::from_mat3(&m).normalize(),
            Rot::Quat(q) => q,
        }
    }

    fn ivp(self) -> IvpQuat {
        match self {
            Rot::Basis(m) => {
                // the rows of the rotation are the matrix's axes' components (its columns are the axes)
                let c = [m.x_axis, m.y_axis, m.z_axis];
                let r: [[f64; 3]; 3] =
                    std::array::from_fn(|row| std::array::from_fn(|col| f64::from(c[col][row])));
                ivp::math::matrix_to_quat(&r)
            }
            Rot::Quat(q) => ivp_quat(q),
        }
    }
}

fn ivp_quat(q: Quat) -> IvpQuat {
    IvpQuat {
        x: f64::from(q.x),
        y: f64::from(q.y),
        z: f64::from(q.z),
        w: f64::from(q.w),
    }
}

impl World {
    /// A world with each of the level's physical objects as a fixed body: a compact surface of a convex piece per
    /// triangle in the object's frame, at the object's pose (the original's concave physicalize).
    pub fn new(statics: &[StaticMesh]) -> World {
        let mut physics = IvpWorld::new();
        physics.time_start();
        // Research knob: start the physics clock this many PSIs (1/66 s) late, to line our PSI grid up with another
        // engine's when comparing traces (the original's phase depends on when its physics timer was reset).
        if let Some(phase) = std::env::var("DRAHTSEILAKT_PSI_PHASE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|p| (0.0..1.0).contains(p))
        {
            physics.advance(phase / 66.0);
        }
        // where the physics' random numbers and its sleep countdown stand when the original's level starts
        physics.rng = ivp::core::Rng::at_level_start();
        physics.env.sleep_countdown = ivp::core::LEVEL_START_SLEEP_COUNTDOWN;
        physics.set_gravity([0.0, -GRAVITY, 0.0]);
        let mut world = World {
            physics,
            sounds: Vec::new(),
            ball_sounds: BallSounds::default(),
            handles: Vec::new(),
            bodies: Vec::new(),
            grounded: false,
            pushers: Vec::new(),
            gravity: GRAVITY,
            gravity_xz: [0.0, 0.0],
            time_factor: TIME_FACTOR,
            statics: Vec::new(),
            spawns: Vec::new(),
            baked_of_mesh: Vec::new(),
            baked_rows: std::collections::BTreeMap::new(),
            script_statics: Vec::new(),
            baked_info: std::collections::BTreeMap::new(),
        };
        for mesh in statics {
            let ledges: Vec<ledge::Ledge> = mesh
                .triangles
                .iter()
                .filter_map(|t| ledge::triangle(vec3_f64(t.a), vec3_f64(t.b), vec3_f64(t.c)))
                .collect();
            let Some(surface) = Surface::compile(ledges) else {
                world.baked_of_mesh.push(None);
                continue;
            };
            let desc = BodyDesc {
                fixed: true,
                friction: SURFACE_FRICTION,
                elasticity: SURFACE_ELASTICITY,
                group: group(mesh.group),
                ..BodyDesc::default()
            };
            let kept = std::sync::Arc::new(surface.clone());
            let id = world.physics.add_body(
                desc,
                Shape::Surface(surface),
                vec3_f64(mesh.pos),
                mesh.rot.ivp(),
            );
            world.baked_info.insert(
                id,
                BakedInfo {
                    surface: kept,
                    pos: mesh.pos,
                    rot: mesh.rot.quat(),
                    group: group(mesh.group),
                },
            );
            world.baked_of_mesh.push(Some(id));
            world
                .statics
                .push((id, mesh.group != GROUP_BALL, mesh.sound));
        }
        world
    }

    /// Says which level object each mesh given to `new` was made of (`rows[k]` for mesh `k`), so that a script's
    /// Physicalize / Unphysicalize of that object finds its baked body.
    pub fn tag_baked(&mut self, rows: &[usize]) {
        self.baked_rows.clear();
        for (&row, id) in rows.iter().zip(&self.baked_of_mesh) {
            if let Some(id) = *id {
                self.baked_rows.insert(row, id);
            }
        }
    }

    /// Whether the level's object `row` still has its baked fixed body.
    pub fn has_baked(&self, row: usize) -> bool {
        self.baked_rows.contains_key(&row)
    }

    /// Takes the baked body of object `row` out of the physics (a script's Unphysicalize of a level object). The
    /// ball's floor lookup stops seeing it. False when there was none.
    pub fn remove_baked(&mut self, row: usize) -> bool {
        let Some(id) = self.baked_rows.remove(&row) else {
            return false;
        };
        self.statics.retain(|s| s.0 != id);
        self.baked_info.remove(&id);
        self.physics.body_destroy(id);
        true
    }

    /// The rows that still have their baked body, in row order.
    pub fn baked_row_list(&self) -> Vec<usize> {
        self.baked_rows.keys().copied().collect()
    }

    /// Carries the baked body of object `row` along with its object, which a script moved by `delta` (the new world
    /// matrix times the inverse of the old). The body is made again in the same slot of the statics.
    pub fn move_baked(&mut self, row: usize, delta: glam::Mat4) {
        let Some(&old) = self.baked_rows.get(&row) else {
            return;
        };
        let Some(mut info) = self.baked_info.remove(&old) else {
            return;
        };
        let basis = glam::Mat3::from_cols(
            delta.x_axis.truncate().normalize_or_zero(),
            delta.y_axis.truncate().normalize_or_zero(),
            delta.z_axis.truncate().normalize_or_zero(),
        );
        let turn = Quat::from_mat3(&basis).normalize();
        let pos = delta.transform_point3(info.pos);
        if !(pos.is_finite() && turn.is_finite()) {
            self.baked_info.insert(old, info);
            return;
        }
        info.pos = pos;
        info.rot = (turn * info.rot).normalize();
        self.physics.body_destroy(old);
        let desc = BodyDesc {
            fixed: true,
            friction: SURFACE_FRICTION,
            elasticity: SURFACE_ELASTICITY,
            group: info.group,
            ..BodyDesc::default()
        };
        let id = self.physics.add_body(
            desc,
            Shape::Surface((*info.surface).clone()),
            vec3_f64(info.pos),
            ivp_quat(info.rot),
        );
        if let Some(e) = self.statics.iter_mut().find(|x| x.0 == old) {
            e.0 = id;
        }
        self.baked_rows.insert(row, id);
        self.baked_info.insert(id, info);
    }

    /// Puts game body `i` (a script's movable body) at another place at rest, as the original does when a script
    /// sets the position of a physicalized object: the body is made again there, awake.
    pub fn teleport(&mut self, i: usize, pos: Vec3, rot: Quat) {
        if !matches!(self.handles.get(i), Some(Some(_))) {
            return;
        }
        let Some(Some(s)) = self.spawns.get_mut(i) else {
            return;
        };
        s.pos = pos;
        s.rot = Rot::Quat(rot);
        s.frozen = false;
        self.remove(i);
        self.activate(i);
    }

    /// A fixed body made at run time from triangles in the object's own frame (scale baked in), at `pos`/`rot`
    /// (a script's concave Physicalize). Returns its handle; none when no triangle is usable.
    pub fn add_static_triangles(
        &mut self,
        triangles: &[[Vec3; 3]],
        pos: Vec3,
        rot: Quat,
        friction: f32,
        elasticity: f32,
        group_name: &str,
    ) -> Option<usize> {
        let ledges: Vec<ledge::Ledge> = triangles
            .iter()
            .filter_map(|t| ledge::triangle(vec3_f64(t[0]), vec3_f64(t[1]), vec3_f64(t[2])))
            .collect();
        let surface = Surface::compile(ledges)?;
        Some(self.place_static(surface, pos, rot, friction, elasticity, group_name))
    }

    /// A fixed body made at run time from the hull of `points` (a script's convex Physicalize with Fixed).
    pub fn add_static_hull(
        &mut self,
        points: &[Vec3],
        pos: Vec3,
        rot: Quat,
        friction: f32,
        elasticity: f32,
        group_name: &str,
    ) -> Option<usize> {
        let pts: Vec<[f64; 3]> = points.iter().map(|&p| vec3_f64(p)).collect();
        let surface = Surface::compile(vec![ivp::convex::hull(&pts)?])?;
        Some(self.place_static(surface, pos, rot, friction, elasticity, group_name))
    }

    /// A fixed sphere made at run time (a script's Physicalize of a ball shape with Fixed = TRUE).
    pub fn add_static_sphere(
        &mut self,
        radius: f32,
        pos: Vec3,
        rot: Quat,
        friction: f32,
        elasticity: f32,
        group_name: &str,
    ) -> Option<usize> {
        if !(radius.is_finite() && radius > 0.0) {
            return None;
        }
        let mut s = ScriptStatic {
            id: 0,
            shape: Shape::Sphere(radius),
            pos,
            rot,
            friction,
            elasticity,
            group: group(group_name),
            ball_collides: group_name != GROUP_BALL,
        };
        s.id = self.make_static_body(&s);
        self.statics
            .push((s.id, s.ball_collides, SoundIds::default()));
        self.script_statics.push(Some(s));
        Some(self.script_statics.len() - 1)
    }

    fn make_static_body(&mut self, s: &ScriptStatic) -> usize {
        let desc = BodyDesc {
            fixed: true,
            friction: s.friction,
            elasticity: s.elasticity,
            group: s.group,
            ..BodyDesc::default()
        };
        self.physics
            .add_body(desc, s.shape.clone(), vec3_f64(s.pos), ivp_quat(s.rot))
    }

    fn place_static(
        &mut self,
        surface: Surface,
        pos: Vec3,
        rot: Quat,
        friction: f32,
        elasticity: f32,
        group_name: &str,
    ) -> usize {
        let mut s = ScriptStatic {
            id: 0,
            shape: Shape::Surface(surface),
            pos,
            rot,
            friction,
            elasticity,
            group: group(group_name),
            ball_collides: group_name != GROUP_BALL,
        };
        s.id = self.make_static_body(&s);
        self.statics
            .push((s.id, s.ball_collides, SoundIds::default()));
        self.script_statics.push(Some(s));
        self.script_statics.len() - 1
    }

    /// Takes a script's fixed body out of the physics; the ball's floor lookup stops seeing it.
    pub fn remove_static(&mut self, h: usize) {
        if let Some(s) = self.script_statics.get_mut(h).and_then(Option::take) {
            self.statics.retain(|x| x.0 != s.id);
            self.physics.body_destroy(s.id);
        }
    }

    /// Puts a script's fixed body at another place (its object was moved): made again in the same slot of the
    /// level's statics, so the order of everything stays.
    pub fn move_static(&mut self, h: usize, pos: Vec3, rot: Quat) {
        let Some(mut s) = self.script_statics.get_mut(h).and_then(Option::take) else {
            return;
        };
        self.physics.body_destroy(s.id);
        s.pos = pos;
        s.rot = rot;
        let old = s.id;
        s.id = self.make_static_body(&s);
        if let Some(e) = self.statics.iter_mut().find(|x| x.0 == old) {
            e.0 = s.id;
        }
        self.script_statics[h] = Some(s);
    }

    /// Where a script's fixed body is (none once removed).
    #[cfg(test)]
    pub fn static_pose(&self, h: usize) -> Option<(Vec3, Quat)> {
        let s = self.script_statics.get(h)?.as_ref()?;
        Some((s.pos, s.rot))
    }

    /// How many fixed bodies there are: the level's baked ones and the scripts'.
    #[cfg(test)]
    pub fn static_count(&self) -> usize {
        self.statics.len()
    }

    /// A new game body from its spawn, made in the physics now when `active`, else only placed (a sector's object
    /// before its sector).
    fn add_spawn(&mut self, spawn: Spawn, active: bool) -> usize {
        self.sounds.push(SoundIds::default());
        self.handles.push(None);
        self.bodies.push(Body {
            pos: spawn.pos,
            rot: spawn.rot.quat(),
        });
        self.spawns.push(Some(spawn));
        let i = self.handles.len() - 1;
        if active {
            self.activate(i);
        }
        i
    }

    /// Makes body `i` in the physics at its start, if it is not there (the original's "physicalize").
    pub fn activate(&mut self, i: usize) {
        if !matches!(self.handles.get(i), Some(None)) {
            return;
        }
        let Some(Some(s)) = self.spawns.get(i).cloned() else {
            return;
        };
        let desc = BodyDesc {
            fixed: s.fixed,
            start_frozen: s.frozen,
            collide: s.collide,
            shift: s.shift,
            group: s.group,
            // the level's objects are entity 0; the game's bodies follow in creation order, so the ball is entity 1
            entity: i as u32 + 1,
            mass: s.m.mass,
            friction: s.m.friction,
            elasticity: s.m.elasticity,
            lin_damp: s.m.lin_damp,
            rot_damp: s.m.rot_damp,
        };
        let id = self
            .physics
            .add_body(desc, s.shape, vec3_f64(s.pos), s.rot.ivp());
        self.handles[i] = Some(id);
        self.bodies[i] = Body {
            pos: s.pos,
            rot: s.rot.quat(),
        };
    }

    /// Takes body `i` out of the physics and puts it back at its start (a sector's object when the sector ends).
    pub fn deactivate(&mut self, i: usize) {
        self.remove(i);
        if let Some(Some(s)) = self.spawns.get(i) {
            self.bodies[i] = Body {
                pos: s.pos,
                rot: s.rot.quat(),
            };
        }
    }

    /// Runs the physics for `seconds` of its own time in steps of 1/60 s (a frame at time factor 1, half a frame at 2).
    pub fn run(&mut self, seconds: f64) {
        let frames = (seconds * 60.0).round() as u32;
        for _ in 0..frames {
            self.physics.advance(1.0 / 60.0);
        }
    }

    /// A slot for the player's ball before it is made (`replace_ball` makes it). By convention the first body.
    pub fn reserve_ball(&mut self, pos: Vec3, rot: Quat) -> usize {
        self.sounds.push(SoundIds::default());
        self.handles.push(None);
        self.bodies.push(Body { pos, rot });
        self.spawns.push(None);
        self.handles.len() - 1
    }

    /// Takes a body out of the physics, as the original's "dephysicalize": its body, its push and its contacts go.
    pub fn remove(&mut self, i: usize) {
        if let Some(id) = self.handles.get_mut(i).and_then(Option::take) {
            self.physics.body_destroy(id);
        }
        if let Some(p) = self.pushers.get_mut(i) {
            *p = None;
        }
    }

    /// Puts a new ball in place of body `i`, at rest, as the original physicalizes a new ball after a death or in a
    /// transformer: the old body is destroyed and a new one made with the new material. The game index and the entity
    /// number stay the same.
    pub fn replace_ball(
        &mut self,
        i: usize,
        radius: f32,
        m: Material,
        pos: Vec3,
        rot: impl Into<Rot>,
    ) {
        if i >= self.handles.len() {
            return;
        }
        let rot = rot.into();
        self.remove(i);
        let desc = BodyDesc {
            entity: i as u32 + 1,
            group: group(GROUP_BALL),
            mass: m.mass,
            friction: m.friction,
            elasticity: m.elasticity,
            lin_damp: m.lin_damp,
            rot_damp: m.rot_damp,
            ..BodyDesc::default()
        };
        let id = self
            .physics
            .add_body(desc, Shape::Sphere(radius), vec3_f64(pos), rot.ivp());
        self.handles[i] = Some(id);
        self.bodies[i] = Body {
            pos,
            rot: rot.quat(),
        };
    }

    /// The player's push on a body, as the original steers its ball: a force that the physics applies at every PSI
    /// at the body's centre while the body is awake. The controller is made the first time and only its force changes
    /// after that.
    pub fn set_push(&mut self, i: usize, force: Vec3) {
        let Some(&Some(id)) = self.handles.get(i) else {
            return;
        };
        if self.pushers.len() <= i {
            self.pushers.resize(i + 1, None);
        }
        let f = [force.x, force.y, force.z];
        match self.pushers[i] {
            Some(ctrl) => self.physics.force_set(ctrl, f),
            None => {
                if f != [0.0; 3] {
                    self.pushers[i] = Some(self.physics.force_create(id, f, [0.0; 3], true));
                }
            }
        }
    }

    /// The player's ball, with its radius, in group Ball, made now. By convention the first body (tests).
    #[cfg(test)]
    pub fn add_ball(&mut self, radius: f32, m: Material, pos: Vec3, rot: Quat) -> usize {
        let i = self.reserve_ball(pos, rot);
        self.replace_ball(i, radius, m, pos, rot);
        i
    }

    /// The original's "Physics WakeUp": a sleeping body comes back into the simulation, an awake one restarts its
    /// sleep timers.
    pub fn wake(&mut self, i: usize) {
        if let Some(&Some(id)) = self.handles.get(i) {
            self.physics.wake_body(id);
        }
    }

    /// A box with the given half sizes: the hull of its eight corners (tests).
    #[cfg(test)]
    #[allow(clippy::expect_used)]
    pub fn add_box(&mut self, half: Vec3, m: Material, pos: Vec3, rot: Quat) -> usize {
        let mut corners = Vec::with_capacity(8);
        for x in [-half.x, half.x] {
            for y in [-half.y, half.y] {
                for z in [-half.z, half.z] {
                    corners.push(Vec3::new(x, y, z));
                }
            }
        }
        self.add_convex(&corners, m, pos, rot, false, true)
            .expect("a box has a hull")
    }

    /// A loose sphere, its mass centre its centre; made in the physics now when `active`.
    pub fn add_sphere(
        &mut self,
        radius: f32,
        m: Material,
        pos: Vec3,
        rot: impl Into<Rot>,
        active: bool,
    ) -> usize {
        self.add_sphere_in(radius, m, pos, rot, active, "")
    }

    /// A loose sphere in the collision group `group_name` (a script's ball body): bodies of one named group do not
    /// collide with each other.
    pub fn add_sphere_in(
        &mut self,
        radius: f32,
        m: Material,
        pos: Vec3,
        rot: impl Into<Rot>,
        active: bool,
        group_name: &str,
    ) -> usize {
        let spawn = Spawn {
            shape: Shape::Sphere(radius),
            m,
            pos,
            rot: rot.into(),
            shift: None,
            group: group(group_name),
            fixed: false,
            frozen: false,
            collide: true,
        };
        self.add_spawn(spawn, active)
    }

    /// A loose convex body: the hull of `points` in its own frame, its mass centre at its origin (the original's
    /// sector objects are physicalized without "Auto Mass Center"). None when the points span no volume.
    pub fn add_convex(
        &mut self,
        points: &[Vec3],
        m: Material,
        pos: Vec3,
        rot: impl Into<Rot>,
        fixed: bool,
        active: bool,
    ) -> Option<usize> {
        let pts: Vec<[f64; 3]> = points.iter().map(|&p| vec3_f64(p)).collect();
        let hull = ivp::convex::hull(&pts)?;
        let surface = Surface::compile(vec![hull])?;
        let spawn = Spawn {
            shape: Shape::Surface(surface),
            m,
            pos,
            rot: rot.into(),
            shift: Some([0.0; 3]),
            group: [0; 8],
            fixed,
            frozen: false,
            collide: true,
        };
        Some(self.add_spawn(spawn, active))
    }

    /// A script's movable convex body (Physicalize): the hull of `points` in the object's own frame, made in the
    /// physics now. `shift` is the mass centre in that frame, none for the hull's own ("Auto Mass Center").
    /// None when the points span no volume.
    #[allow(clippy::too_many_arguments)]
    pub fn add_scripted_convex(
        &mut self,
        points: &[Vec3],
        m: Material,
        pos: Vec3,
        rot: Quat,
        frozen: bool,
        collide: bool,
        shift: Option<[f32; 3]>,
        group_name: &str,
    ) -> Option<usize> {
        let pts: Vec<[f64; 3]> = points.iter().map(|&p| vec3_f64(p)).collect();
        let surface = Surface::compile(vec![ivp::convex::hull(&pts)?])?;
        let spawn = Spawn {
            shape: Shape::Surface(surface),
            m,
            pos,
            rot: rot.into(),
            shift,
            group: group(group_name),
            fixed: false,
            frozen,
            collide,
        };
        Some(self.add_spawn(spawn, true))
    }

    /// A module's part (or FixCube): one convex piece per point set, in its own frame, its mass centre at its origin,
    /// with the module's values. None when no piece spans a volume.
    #[allow(clippy::too_many_arguments)]
    pub fn add_part(
        &mut self,
        pieces: &[Vec<Vec3>],
        m: Material,
        pos: Vec3,
        rot: impl Into<Rot>,
        fixed: bool,
        group_name: &'static str,
        frozen: bool,
        collide: bool,
        shift: [f32; 3],
        active: bool,
    ) -> Option<usize> {
        let hulls: Vec<ledge::Ledge> = pieces
            .iter()
            .filter_map(|p| {
                let pts: Vec<[f64; 3]> = p.iter().map(|&v| vec3_f64(v)).collect();
                ivp::convex::hull(&pts)
            })
            .collect();
        let surface = Surface::compile(hulls)?;
        let spawn = Spawn {
            shape: Shape::Surface(surface),
            m,
            pos,
            rot: rot.into(),
            shift: Some(shift),
            group: group(group_name),
            fixed,
            frozen,
            collide,
        };
        Some(self.add_spawn(spawn, active))
    }

    /// The IVP bodies of two game bodies, both in the physics.
    fn pair(&self, a: usize, b: usize) -> Option<(usize, usize)> {
        match (self.handles.get(a), self.handles.get(b)) {
            (Some(&Some(ia)), Some(&Some(ib))) => Some((ia, ib)),
            _ => None,
        }
    }

    /// The IVP body of a game body, when it is in the physics.
    pub fn body_ivp(&self, i: usize) -> Option<usize> {
        self.handles.get(i).copied().flatten()
    }

    /// The IVP body of a script's fixed body (its handle from `add_static_*`).
    pub fn static_ivp(&self, h: usize) -> Option<usize> {
        self.script_statics.get(h)?.as_ref().map(|s| s.id)
    }

    /// The IVP body of a level object's baked fixed body.
    pub fn baked_ivp(&self, row: usize) -> Option<usize> {
        self.baked_rows.get(&row).copied()
    }

    /// A slider between two IVP bodies (the ids `body_ivp`, `static_ivp` and `baked_ivp` give): the same joint
    /// `add_slider` makes, for bodies that are not both game bodies (a fixed body and a movable one).
    pub fn add_slider_ivp(
        &mut self,
        a: usize,
        b: usize,
        p0: Vec3,
        p1: Vec3,
        limits: Option<(f32, f32)>,
    ) -> Option<usize> {
        let (lim, lo, hi) = limits.map_or((false, -1.0, 1.0), |(lo, hi)| (true, lo, hi));
        self.physics
            .slider_create(Some(a), Some(b), p3(p0), p3(p1), lim, lo, hi)
    }

    /// A slider of body `a` to body `b` along the world axis from `p0` to `p1`, limited to lo..hi along it when
    /// asked (Set Physics Slider). None when either body is not in the physics.
    pub fn add_slider(
        &mut self,
        a: usize,
        b: usize,
        p0: Vec3,
        p1: Vec3,
        limits: Option<(f32, f32)>,
    ) -> Option<usize> {
        let (ia, ib) = self.pair(a, b)?;
        let (lim, lo, hi) = limits.map_or((false, -1.0, 1.0), |(lo, hi)| (true, lo, hi));
        self.physics
            .slider_create(Some(ia), Some(ib), p3(p0), p3(p1), lim, lo, hi)
    }

    /// A hinge of body `a` to body `b` at a world anchor about a world axis (Set Physics Hinge).
    pub fn add_hinge(&mut self, a: usize, b: usize, anchor: Vec3, axis: Vec3) -> Option<usize> {
        let (ia, ib) = self.pair(a, b)?;
        self.physics
            .hinge_create(Some(ia), Some(ib), p3(anchor), p3(axis))
    }

    /// A ball joint of body `a` to body `b` at a world anchor (Set Physics Ball Joint).
    pub fn add_ball_joint(&mut self, a: usize, b: usize, anchor: Vec3) -> Option<usize> {
        let (ia, ib) = self.pair(a, b)?;
        self.physics
            .balljoint_create(Some(ia), Some(ib), p3(anchor))
    }

    /// A spring between world anchors on bodies `a` and `b` (Set Physics Spring): rest length, constant, and the
    /// linear and global dampings.
    #[allow(clippy::too_many_arguments)]
    pub fn add_spring(
        &mut self,
        a: usize,
        pa: Vec3,
        b: usize,
        pb: Vec3,
        length: f32,
        constant: f32,
        lin_damp: f32,
        glob_damp: f32,
    ) -> Option<usize> {
        let (ia, ib) = self.pair(a, b)?;
        Some(self.physics.spring_create(
            Some(ia),
            p3(pa),
            Some(ib),
            p3(pb),
            length,
            constant,
            lin_damp,
            glob_damp,
        ))
    }

    /// A constant force on body `i` at a world point (SetPhysicsForce): `force` is a world vector, given to the body
    /// at every PSI while it is awake. None when the body is not in the physics. Returns the force's handle.
    pub fn add_force(&mut self, i: usize, force: Vec3, point: Vec3) -> Option<usize> {
        let Some(&Some(id)) = self.handles.get(i) else {
            return None;
        };
        Some(
            self.physics
                .force_create(id, p3(force), vec3_f64(point), false),
        )
    }

    /// Removes a force made by `add_force` (SetPhysicsForce's Destroy).
    pub fn remove_force(&mut self, ctrl: usize) {
        self.physics.force_destroy(ctrl);
    }

    /// Gives game body `i` a sound material.
    pub fn set_sound(&mut self, i: usize, ids: SoundIds) {
        if let Some(s) = self.sounds.get_mut(i) {
            *s = ids;
        }
    }

    /// The sound material of an IVP body.
    fn sound_of(&self, id: usize) -> SoundIds {
        if let Some(s) = self.statics.iter().find(|s| s.0 == id) {
            return s.2;
        }
        let i = self.handles.iter().position(|h| *h == Some(id));
        i.and_then(|i| self.sounds.get(i))
            .copied()
            .unwrap_or_default()
    }

    /// The ball's impacts and contacts of the step just taken (every impact is taken, so the list does not grow).
    fn listen_ball(&mut self) {
        let impacts = self.physics.take_impacts();
        let Some(&Some(ball)) = self.handles.first() else {
            self.ball_sounds = BallSounds::default();
            return;
        };
        let mut s = BallSounds::default();
        for (obj, rel) in impacts {
            let other = match obj {
                [a, b] if a == ball => b,
                [a, b] if b == ball => a,
                _ => continue,
            };
            let speed = (rel[0] * rel[0] + rel[1] * rel[1] + rel[2] * rel[2]).sqrt();
            s.impacts.push((self.sound_of(other).hit, speed));
        }
        s.rolls = self
            .physics
            .friction_partners(ball)
            .into_iter()
            .map(|o| self.sound_of(o).roll)
            .collect();
        self.ball_sounds = s;
    }

    /// The physics' own time, in seconds.
    pub fn time(&self) -> f64 {
        self.physics.time()
    }

    /// Removes a joint made by `add_slider`, `add_hinge` or `add_ball_joint` (a joint's Destroy input).
    pub fn remove_joint(&mut self, j: usize) {
        self.physics.joint_destroy(j);
    }

    /// The triangles near a point of the level's physical objects that the player's ball collides with, in world
    /// space, as the physics sees them.
    pub fn floor_near(&self, p: Vec3, r: f32) -> Vec<[Vec3; 3]> {
        let mut out = Vec::new();
        for &(id, _, _) in self.statics.iter().filter(|s| s.1) {
            let (pos, rot) = self.physics.body_pose(id);
            let (pos, rot) = (vec3(pos), quat(rot));
            let local = rot.inverse() * (p - pos);
            for t in self
                .physics
                .surface_triangles_near(id, vec3_f64(local), f64::from(r))
            {
                out.push(t.map(|v| pos + rot * vec3(v)));
            }
        }
        out
    }

    /// The movement state of a body's core (the upstream's 0 moving .. 8 not simulated), for the session log.
    pub fn movement_state(&self, i: usize) -> Option<i32> {
        let id = (*self.handles.get(i)?)?;
        self.physics
            .cores
            .get(id)
            .map(|c| i32::from(c.movement_state))
    }

    /// A mod's gravity (down, then sideways along the world's x and z) and time factor (modrt.rs SimOut): set only
    /// when they change.
    pub fn set_rules(&mut self, gravity: f32, side: [f32; 2], time_factor: f32) {
        if gravity != self.gravity || side != self.gravity_xz {
            self.gravity = gravity;
            self.gravity_xz = side;
            self.physics.set_gravity([side[0], -gravity, side[1]]);
        }
        let t = f64::from(time_factor);
        if t != self.time_factor {
            self.time_factor = t;
        }
    }

    /// The contact points the ball takes part in, for the watch log: those whose entities include the ball (entity 1).
    pub fn ball_contacts(&self) -> Vec<ivp::glue::ContactReadout> {
        self.physics.contacts_of(1)
    }

    /// The contact points one body (game index) takes part in, for tracing (entities are game indices + 1).
    pub fn body_contacts(&self, i: usize) -> Vec<ivp::glue::ContactReadout> {
        self.physics.contacts_of(i as u32 + 1)
    }

    /// The physics' world-wide state as text (the time, the shared random generator, the sleep test's countdown,
    /// how many units are simulated or frozen), for the state dump (`DRAHTSEILAKT_DUMP_STATE`, app.rs).
    pub fn dump_world(&self) -> String {
        let p = &self.physics;
        format!(
            "time {:.6} time_code {} rng {:?} sleep_countdown {} active {} frozen {} revive {} joints {} gravity {} {:?}",
            p.time(),
            p.time_code,
            p.rng,
            p.env.sleep_countdown,
            p.active.len(),
            p.frozen.len(),
            p.revive.len(),
            p.joints.len(),
            self.gravity,
            self.gravity_xz
        )
    }

    /// Body `i`'s whole physics core, its unit and its controllers as text, and its contact points, for the state
    /// dump; `None` while it is out of the physics.
    pub fn dump_body(&self, i: usize) -> Option<String> {
        let id = (*self.handles.get(i)?)?;
        let c = self.physics.cores.get(id)?;
        let mut s = format!("core {c:?}");
        if let Some(u) = self.physics.units.get(c.unit) {
            s.push_str(&format!("\n      unit {u:?}"));
        }
        for &k in &c.controllers {
            if let Some(ctl) = self.physics.controllers.get(k) {
                s.push_str(&format!("\n      controller {ctl:?}"));
            }
        }
        for cp in self.body_contacts(i) {
            s.push_str(&format!(
                "\n      contact types {:?} gap {:.5} pressure {:.5} normal {:?} point {:?}",
                cp.types, cp.gap, cp.pressure, cp.normal, cp.point
            ));
        }
        Some(s)
    }

    pub fn state(&self, i: usize) -> Option<State> {
        let id = (*self.handles.get(i)?)?;
        let s: BodyState = self.physics.body_state(id);
        Some(State {
            pos: vec3(s.pos),
            vel: Vec3::new(s.vel[0], s.vel[1], s.vel[2]),
            rot: quat(s.rot),
            spin: Vec3::new(s.spin[0], s.spin[1], s.spin[2]),
        })
    }

    /// An impulse on body `i` at a world point (Physics Impulse), applied at the next PSI; the body is woken.
    pub fn impulse_at(&mut self, i: usize, point: Vec3, impulse: Vec3) {
        if let Some(&Some(id)) = self.handles.get(i) {
            self.physics
                .body_impulse(id, vec3_f64(point), [impulse.x, impulse.y, impulse.z]);
        }
    }

    /// Advances the world by one tick. `push` is an impulse for one body (the player's push), at its centre.
    pub fn step(&mut self, push: Option<(usize, Vec3)>) {
        if let Some((i, impulse)) = push
            && let Some(&Some(id)) = self.handles.get(i)
        {
            let centre = self.physics.body_state(id).pos;
            self.physics
                .body_impulse(id, centre, [impulse.x, impulse.y, impulse.z]);
        }
        self.physics.advance(frame_step(self.time_factor));
        self.listen_ball();
        for (i, id) in self.handles.iter().enumerate() {
            let Some(id) = *id else {
                continue;
            };
            let (pos, rot) = self.physics.body_pose(id);
            self.bodies[i] = Body {
                pos: vec3(pos),
                rot: quat(rot),
            };
        }
        self.grounded = self
            .handles
            .first()
            .copied()
            .flatten()
            .is_some_and(|id| self.physics.body_touching(id));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::collide::Triangle;

    /// A flat floor at y = 0, 200 units square.
    fn floor() -> Vec<StaticMesh> {
        let s = 100.0;
        let p = [
            Vec3::new(-s, 0.0, -s),
            Vec3::new(s, 0.0, -s),
            Vec3::new(s, 0.0, s),
            Vec3::new(-s, 0.0, s),
        ];
        let tri = |a, b, c| Triangle { a, b, c };
        vec![StaticMesh {
            triangles: vec![tri(p[0], p[2], p[1]), tri(p[0], p[3], p[2])],
            pos: Vec3::ZERO,
            rot: Quat::IDENTITY.into(),
            group: GROUP_FLOOR,
            sound: SoundIds::default(),
        }]
    }

    fn crate_material() -> Material {
        Material {
            friction: 0.7,
            elasticity: 0.3,
            mass: 1.0,
            lin_damp: 0.1,
            rot_damp: 0.1,
        }
    }

    fn wood() -> Material {
        Material {
            friction: 0.8,
            elasticity: 0.2,
            mass: 1.9,
            lin_damp: 0.9,
            rot_damp: 0.1,
        }
    }

    fn paper() -> Material {
        Material {
            friction: 0.5,
            elasticity: 0.4,
            mass: 0.2,
            lin_damp: 1.5,
            rot_damp: 0.1,
        }
    }

    fn steps(w: &mut World, n: usize, push: Option<(usize, Vec3)>) {
        for _ in 0..n {
            w.step(push);
        }
    }

    #[test]
    fn a_floor_with_its_own_pose_holds_the_ball_where_it_is_in_the_world() {
        // The floor's triangles in its own frame, the body moved up by 10 and turned a quarter about y: the ball
        // rests one radius above y = 10.
        let mut f = floor();
        f[0].pos = Vec3::new(0.0, 10.0, 0.0);
        f[0].rot = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2).into();
        let mut w = World::new(&f);
        let ball = w.add_ball(2.0, wood(), Vec3::new(3.0, 15.0, 4.0), Quat::IDENTITY);
        steps(&mut w, 180, None);
        let s = w.state(ball).unwrap();
        assert!((s.pos.y - 12.0).abs() < 0.05, "the ball is at {:?}", s.pos);
        let near = w.floor_near(Vec3::new(0.0, 10.0, 0.0), 200.0);
        assert!(
            near.iter().flatten().all(|v| (v.y - 10.0).abs() < 1e-4),
            "the triangles in the world: {near:?}"
        );
    }

    #[test]
    fn a_floor_stopper_stops_a_box_but_not_the_ball() {
        // A wall 4 high across x = 10, in the ball's group: the box pushed into it stops, the ball rolls through.
        let mut statics = floor();
        let w = |y: f32, z: f32| Vec3::new(10.0, y, z);
        statics.push(StaticMesh {
            triangles: vec![
                Triangle {
                    a: w(0.0, -50.0),
                    b: w(4.0, 50.0),
                    c: w(4.0, -50.0),
                },
                Triangle {
                    a: w(0.0, -50.0),
                    b: w(0.0, 50.0),
                    c: w(4.0, 50.0),
                },
            ],
            pos: Vec3::ZERO,
            rot: Quat::IDENTITY.into(),
            group: GROUP_BALL,
            sound: SoundIds::default(),
        });
        let mut world = World::new(&statics);
        let ball = world.add_ball(2.0, wood(), Vec3::new(0.0, 2.0, 8.0), Quat::IDENTITY);
        let bx = world.add_box(
            Vec3::splat(1.0),
            crate_material(),
            Vec3::new(0.0, 1.0, -8.0),
            Quat::IDENTITY,
        );
        for _ in 0..240 {
            world.set_push(ball, Vec3::X * 0.43);
            world.set_push(bx, Vec3::X * 0.3);
            world.step(None);
        }
        let (b, x) = (world.state(ball).unwrap().pos, world.state(bx).unwrap().pos);
        assert!(b.x > 14.0, "the ball did not pass the stopper: {b:?}");
        assert!(x.x < 10.0, "the box went through the stopper: {x:?}");
        assert!(
            world
                .floor_near(Vec3::new(10.0, 2.0, 0.0), 60.0)
                .iter()
                .all(|t| t.iter().all(|v| v.y == 0.0)),
            "the stopper is not the ball's floor"
        );
    }

    #[test]
    fn a_level_matrix_becomes_the_same_rotation_as_its_quaternion() {
        // A quarter turn about y given as an object's matrix: the physics' quaternion is that turn, from the matrix
        // in double precision as the original's glue takes it.
        let turn = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let q = Rot::Basis(glam::Mat3::from_quat(turn)).ivp();
        let h = std::f64::consts::FRAC_1_SQRT_2;
        assert!(
            (q.x).abs() < 1e-6
                && (q.y - h).abs() < 1e-6
                && (q.z).abs() < 1e-6
                && (q.w - h).abs() < 1e-6,
            "{q:?}"
        );
        assert!(
            Rot::Basis(glam::Mat3::from_quat(turn))
                .quat()
                .angle_between(turn)
                < 1e-5
        );
    }

    #[test]
    fn a_box_rests_on_the_floor() {
        let mut w = World::new(&floor());
        let b = w.add_box(
            Vec3::splat(2.5),
            crate_material(),
            Vec3::new(0.0, 8.0, 0.0),
            Quat::IDENTITY,
        );
        steps(&mut w, 240, None);
        let s = w.state(b).unwrap();
        assert!((s.pos.y - 2.5).abs() < 0.05, "box rests at {}", s.pos.y);
        assert!(s.vel.length() < 0.05, "box still moves at {:?}", s.vel);
    }

    #[test]
    fn a_box_does_not_sink_into_the_floor_when_it_lands_tilted() {
        let mut w = World::new(&floor());
        let tilt = Quat::from_rotation_z(0.6) * Quat::from_rotation_x(0.3);
        let b = w.add_box(
            Vec3::splat(2.5),
            crate_material(),
            Vec3::new(0.0, 12.0, 0.0),
            tilt,
        );
        steps(&mut w, 300, None);
        let s = w.state(b).unwrap();
        // The box lands on a corner and then settles on a face (a box balanced on a corner tips over): its centre
        // ends between a face's height (2.5) and the corner's (about 4).
        assert!(s.pos.y > 1.0, "box sank to {}", s.pos.y);
        assert!(s.pos.y < 4.0, "box floats at {}", s.pos.y);
    }

    #[test]
    fn crates_made_overlapping_side_by_side_do_not_stop_the_world() {
        // Two crates (cubes of 5) made 4 apart along x, so each one's corners lie exactly on the other's edges along
        // x, as a level's crates set too close make them; they land on the floor. The contact of a corner on an edge
        // at no distance once had no normal: the impact gave the crates NaN speeds, their NaN event times stopped the
        // physics clock, and every body went on in a straight line from its last step, unsteered — a ball in the air
        // floated at a constant speed. The ball, falling far away, must keep falling as gravity says.
        let mut w = World::new(&floor());
        for x in [0.0, 4.0] {
            w.add_box(
                Vec3::splat(2.5),
                crate_material(),
                Vec3::new(x, 2.7, 0.0),
                Quat::IDENTITY,
            );
        }
        let ball = w.add_ball(2.0, wood(), Vec3::new(60.0, 150.0, 0.0), Quat::IDENTITY);
        let mut ys = Vec::new();
        for _ in 0..120 {
            w.step(None);
            let s = w.state(ball).unwrap();
            assert!(
                s.pos.is_finite() && s.vel.is_finite(),
                "the ball is at {:?}, speed {:?}",
                s.pos,
                s.vel
            );
            ys.push(s.pos.y);
        }
        // the fall speeds up every tick (the drag of a wooden ball is far from cancelling gravity at these speeds)
        for k in 2..ys.len() {
            let (d0, d1) = (ys[k - 1] - ys[k - 2], ys[k] - ys[k - 1]);
            assert!(
                d1 < d0 - 1e-4,
                "tick {k}: the ball moves {d0} then {d1}: it floats at a constant speed"
            );
        }
        for (i, b) in w.bodies.iter().enumerate() {
            assert!(b.pos.is_finite(), "body {i} is at {:?}", b.pos);
        }
    }

    #[test]
    fn a_wooden_ball_pushes_a_box_and_the_box_stays_down() {
        let mut w = World::new(&floor());
        let bx = w.add_box(
            Vec3::splat(2.5),
            crate_material(),
            Vec3::new(6.0, 2.5, 0.0),
            Quat::IDENTITY,
        );
        let ball = w.add_ball(2.0, wood(), Vec3::new(-6.0, 2.0, 0.0), Quat::IDENTITY);
        for _ in 0..240 {
            w.step(Some((ball, Vec3::X * 1.5)));
        }
        let b = w.state(bx).unwrap();
        assert!(b.pos.x > 8.0, "the box was not pushed: {:?}", b.pos);
        assert!(b.pos.y < 3.0, "the box left the floor: {:?}", b.pos);
    }

    #[test]
    fn a_paper_ball_does_not_move_a_box_that_a_wooden_one_moves() {
        // The push is the original's control force per step (its column "Force", docs/physics.md section 3), times
        // 66 steps per second of its time, per tick: the impulse the sim gives, which does not depend on the mass.
        let run = |m: Material, force: f32| {
            let mut w = World::new(&floor());
            let bx = w.add_box(
                Vec3::splat(2.5),
                crate_material(),
                Vec3::new(6.0, 2.5, 0.0),
                Quat::IDENTITY,
            );
            let ball = w.add_ball(2.0, m, Vec3::new(-6.0, 2.0, 0.0), Quat::IDENTITY);
            for _ in 0..180 {
                w.set_push(ball, Vec3::X * force);
                w.step(None);
            }
            w.state(bx).unwrap().pos.x - 6.0
        };
        let by_wood = run(wood(), 0.43);
        let by_paper = run(paper(), 0.065);
        assert!(
            by_wood > by_paper + 0.5,
            "wood moved the box {by_wood}, paper {by_paper}"
        );
    }

    #[test]
    fn a_wooden_ball_barely_bounces() {
        let mut w = World::new(&floor());
        let ball = w.add_ball(2.0, wood(), Vec3::new(0.0, 20.0, 0.0), Quat::IDENTITY);
        let mut before = 0.0f32;
        let mut after = 0.0f32;
        for _ in 0..240 {
            let v = w.state(ball).unwrap().vel.y;
            w.step(None);
            let now = w.state(ball).unwrap().vel.y;
            if v < -1.0 && now > 0.0 {
                before = -v;
                after = now;
                break;
            }
        }
        assert!(before > 0.0, "the ball never landed");
        // The impact restitutes at sqrt(e) (docs/ivp_contact.md 6): the pair's elasticity is 0.2 × 0.3 = 0.06, and
        // sqrt(0.06) = 0.245 of the impact speed comes back.
        let ratio = after / before;
        assert!(
            (ratio - 0.06f32.sqrt()).abs() < 0.02,
            "rebound {after} from impact {before}, ratio {ratio}"
        );
    }

    #[test]
    fn stacked_and_pushed_boxes_stay_on_the_floor_and_do_not_pass_through_each_other() {
        let mut w = World::new(&floor());
        let a = w.add_box(
            Vec3::splat(2.5),
            crate_material(),
            Vec3::new(-4.0, 2.5, 0.0),
            Quat::IDENTITY,
        );
        let b = w.add_box(
            Vec3::splat(2.5),
            crate_material(),
            Vec3::new(4.0, 2.5, 0.0),
            Quat::IDENTITY,
        );
        let c = w.add_box(
            Vec3::splat(2.5),
            crate_material(),
            Vec3::new(4.0, 7.5, 0.0),
            Quat::IDENTITY,
        );
        let ball = w.add_ball(2.0, wood(), Vec3::new(-16.0, 2.0, 0.0), Quat::IDENTITY);
        // the push of the wood ball in the game: its force 0.43 times 66 / 60 per tick (sim.rs)
        for _ in 0..480 {
            w.step(Some((ball, Vec3::X * 0.473)));
        }
        let (sa, sb, sc) = (
            w.state(a).unwrap(),
            w.state(b).unwrap(),
            w.state(c).unwrap(),
        );
        for (name, s) in [("a", sa), ("b", sb), ("c", sc)] {
            assert!(
                s.pos.y > 1.5 && s.pos.y < 9.0,
                "box {name} is at {:?}",
                s.pos
            );
        }
        // Boxes are 5 wide: two centres closer than 5 on the same level mean one of them went into the other.
        let gap = |p: Vec3, q: Vec3| (p - q).abs().max_element();
        assert!(
            gap(sa.pos, sb.pos) >= 4.9 || (sa.pos.y - sb.pos.y).abs() >= 4.9,
            "a and b overlap: {:?} {:?}",
            sa.pos,
            sb.pos
        );
    }

    /// A ball resting on the floor touches it: its contacts name the ball's entity, 1, and the floor's, 0.
    #[test]
    fn a_resting_ball_reports_its_contacts_with_entity_ids() {
        let mut w = World::new(&floor());
        w.add_ball(2.0, wood(), Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
        for _ in 0..120 {
            w.step(None);
        }
        let contacts = w.ball_contacts();
        assert!(!contacts.is_empty(), "the ball on the floor has contacts");
        assert!(contacts.iter().all(|c| c.entities.contains(&1)));
        assert!(
            contacts.iter().any(|c| c.entities.contains(&0)),
            "the floor is entity 0: {contacts:?}"
        );
    }
}
