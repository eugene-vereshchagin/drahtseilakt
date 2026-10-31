// Port of OpenBallance src/phys/phys.c and the body parts of phys_core.c (ivp_object_init_core): the physics API the game
// uses. A body is an object with a core (its mass centre) and, for collision, a shape: a sphere (the ball, a loose
// sphere) or a compact surface (a convex box, a level's floors). Bodies are referred to by their core index.
//
// The game's pose convention is the object's origin; the core sits at the mass centre, so an object's origin is the
// core's position plus its rotated shift, which is the negated mass centre (`Core::shift`).

use crate::collision::ObjKind;
use crate::core::{Controller, World, MT_NOT_SIM, MT_STATIC};
use crate::ledge::Surface;
use crate::math::{self, Matrix, Quat};

type V3 = [f64; 3];

/// A body's description (`PhysBodyDesc`): fixedness, collision, the material, the mass, the dampings, and the mass
/// centre override (`shift`, in object space; None takes the surface's own centre).
#[derive(Clone, Copy, Debug)]
pub struct BodyDesc {
    pub fixed: bool,
    pub collide: bool,
    pub start_frozen: bool,
    pub mass: f32,
    pub friction: f32,
    pub elasticity: f32,
    pub lin_damp: f32,
    pub rot_damp: f32,
    pub shift: Option<[f32; 3]>,
}

impl Default for BodyDesc {
    fn default() -> Self {
        BodyDesc {
            fixed: false,
            collide: true,
            start_frozen: false,
            mass: 1.0,
            friction: 0.5,
            elasticity: 0.5,
            lin_damp: 0.0,
            rot_damp: 0.0,
            shift: None,
        }
    }
}

/// A body's collision shape.
#[derive(Clone, Debug)]
pub enum Shape {
    /// A sphere of this radius (the ball's extra radius is the radius itself).
    Sphere(f32),
    /// A compact surface, convex or concave.
    Surface(Surface),
}

/// The state the game reads and writes: the object origin, the rotation, the speed of the mass centre and the spin in
/// world axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyState {
    pub pos: [f64; 3],
    pub rot: Quat,
    pub vel: [f32; 3],
    pub spin: [f32; 3],
}

fn to_v3(v: [f32; 3]) -> V3 {
    [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]
}

fn len3(v: &[f32; 3]) -> f32 {
    ((f64::from(v[0]) * f64::from(v[0]) + f64::from(v[1]) * f64::from(v[1]) + f64::from(v[2]) * f64::from(v[2])).sqrt()) as f32
}

impl World {
    /// Sets the world's gravity (the controller of the first call; later calls change it). Gravity acts on the bodies
    /// created after it: a core attaches the gravity controller at creation.
    pub fn set_gravity(&mut self, g: [f32; 3]) {
        for c in self.controllers.iter_mut() {
            if let Controller::Gravity { g: old } = c {
                *old = g;
                return;
            }
        }
        self.add_controller(Controller::Gravity { g });
    }

    /// Makes the world's time start at zero, with the first PSI at the start (`phys_world_create`).
    pub fn time_start(&mut self) {
        self.time_init();
    }

    /// Creates a body at an object origin `pos` with orientation `rot`: its core, its mass and inertia from the shape, its
    /// collision object (when it collides), and its wake (unless fixed or started frozen). Returns the body's index.
    pub fn add_body(&mut self, desc: BodyDesc, shape: Shape, pos: [f64; 3], rot: Quat) -> usize {
        // the mass centre of the shape, in object space
        let mc: [f32; 3] = match desc.shift {
            Some(s) => s,
            None => match &shape {
                Shape::Sphere(_) => [0.0; 3],
                Shape::Surface(s) => s.mass_center,
            },
        };
        // the core is at the mass centre: its world position is the origin moved by the rotated centre
        let r = math::quat_to_matrix(&rot);
        let rmc = math::rmul(&r, &to_v3(mc));
        let core_pos = [pos[0] + rmc[0], pos[1] + rmc[1], pos[2] + rmc[2]];
        let body = self.cores.len();
        let c = self.core_init(body, rot, core_pos, desc.fixed);
        self.ensure_object_pub(c);
        // the object origin is the core moved back by the rotated centre: the shift is the negated centre
        self.cores[c].shift = [-mc[0], -mc[1], -mc[2]];

        let (kind, extra, surface) = match &shape {
            Shape::Sphere(radius) => (ObjKind::Ball, *radius, None),
            Shape::Surface(s) => (ObjKind::Polygon, 0.0, Some(s.clone())),
        };
        // inertia: a sphere's 0.4 r^2 per unit mass, a surface's own unit inertia
        let mut mass = f64::from(desc.mass);
        if mass < 1e-8 {
            mass = 1.0;
        }
        let unit_inertia: [f32; 3] = match &shape {
            Shape::Sphere(radius) => {
                let i = (f64::from(*radius) * f64::from(*radius) * 0.4) as f32;
                [i, i, i]
            }
            Shape::Surface(s) => s.rotation_inertia,
        };
        let mut rot_inertia = [0.0f32; 3];
        for i in 0..3 {
            rot_inertia[i] = (f64::from(unit_inertia[i]) * 1.0 * mass) as f32;
        }
        // no axis below 3% of the inertia's length
        let floor = len3(&rot_inertia) * 0.03;
        for v in rot_inertia.iter_mut() {
            if *v < floor {
                *v = floor;
            }
        }
        let radius_surface = match &surface {
            Some(s) => {
                let (rad, _) = s.radius_dev(mc);
                rad
            }
            None => 0.0,
        };
        {
            let core = &mut self.cores[c];
            core.rot_inertia = rot_inertia;
            core.inv_rot_inertia = [
                (1.0 / f64::from(rot_inertia[0])) as f32,
                (1.0 / f64::from(rot_inertia[1])) as f32,
                (1.0 / f64::from(rot_inertia[2])) as f32,
            ];
            core.mass = mass as f32;
            core.inv_mass = (1.0 / mass) as f32;
            core.speed_damp = desc.lin_damp;
            core.rot_speed_damp = [desc.rot_damp; 3];
            core.upper_limit_radius = extra + radius_surface;
            core.speed = [0.0; 3];
            core.rot_speed = [0.0; 3];
            core.movement_state = if desc.fixed { MT_STATIC } else { MT_NOT_SIM };
        }
        self.set_object_geometry(c, kind, surface, extra);
        self.set_object_material(c, desc.friction, desc.elasticity);
        if desc.collide && (kind == ObjKind::Ball || self.coll.objs[c].surface.is_some()) {
            self.enable_collision_detection(c);
        }
        if !desc.fixed && !desc.start_frozen {
            self.wake_body(c);
        }
        c
    }

    /// Ensures a body's index is valid in the collision tables.
    fn ensure_object_pub(&mut self, c: usize) {
        self.ensure_object(c);
    }

    /// Wakes a body: it comes back into simulation at the next PSI (`ivp_body_wake`).
    pub fn wake_body(&mut self, c: usize) {
        self.ensure_in_simulation(c);
    }

    /// The object's pose at the current time: its origin and its rotation (the game's convention).
    pub fn body_pose(&self, c: usize) -> (V3, Quat) {
        let m = self.cores[c].object_pose_at(self.env.current_time);
        (m.vv, math::matrix_to_quat(&m.r))
    }

    /// The object's state: pose, the mass centre's velocity and the spin in world axes.
    pub fn body_state(&self, c: usize) -> BodyState {
        let core = &self.cores[c];
        let (pos, rot) = self.body_pose(c);
        let r = core.m_world_f_core.r;
        let spin = math::rmul(&r, &to_v3(core.rot_speed));
        BodyState {
            pos,
            rot,
            vel: core.speed,
            spin: [spin[0] as f32, spin[1] as f32, spin[2] as f32],
        }
    }

    /// Puts a body where the state says, with its velocities, and wakes it. The pose becomes the last PSI's, so the
    /// next PSI starts from it.
    pub fn set_body_state(&mut self, c: usize, s: BodyState) {
        let now = self.env.current_time;
        let r = math::quat_to_matrix(&s.rot);
        let rmc = math::rmul(&r, &to_v3(self.cores[c].shift.map(|v| -v)));
        let core_pos = [s.pos[0] + rmc[0], s.pos[1] + rmc[1], s.pos[2] + rmc[2]];
        {
            let core = &mut self.cores[c];
            core.q_last = s.rot;
            core.q_next = s.rot;
            core.pos_last = core_pos;
            core.delta_pos = [0.0; 3];
            core.time_of_last_psi = now;
            core.m_world_f_core = Matrix { r, vv: core_pos };
            core.speed = s.vel;
            // the spin in world axes becomes the core's rotation speed
            let spin_cs = math::rtmul(&r, &to_v3(s.spin));
            core.rot_speed = [spin_cs[0] as f32, spin_cs[1] as f32, spin_cs[2] as f32];
        }
        self.wake_body(c);
    }

    /// An impulse at a world point of a body, applied at the next PSI (`ivp_body_async_push_ws`, with the wake).
    pub fn body_impulse(&mut self, c: usize, point_ws: V3, impulse: [f32; 3]) {
        self.wake_body(c);
        self.async_push_core_ws(c, point_ws, impulse);
    }

    /// Whether a body has a contact point right now (the ball's grounded test: a point of its friction system).
    pub fn body_touching(&self, c: usize) -> bool {
        self.coll.objs.get(c).is_some_and(|o| !o.fric.is_empty())
    }

    /// Advances the physics to `now + dt` (seconds), running the PSIs and the mindist events in between
    /// (`phys_frame`, the time manager's loop).
    pub fn advance(&mut self, dt: f64) {
        let target = self.env.current_time + dt;
        self.time_simulate_until(target);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    
    /// A floor of two triangles at height 0 spanning [-10, 10] in x and z, as one compact surface.
    fn floor() -> Surface {
        let p = [[-10.0, 0.0, -10.0], [10.0, 0.0, -10.0], [10.0, 0.0, 10.0], [-10.0, 0.0, 10.0]];
        let mut ledges = Vec::new();
        for tri in [[0, 1, 2], [0, 2, 3]] {
            ledges.push(crate::ledge::triangle(p[tri[0]], p[tri[1]], p[tri[2]]).expect("a triangle"));
        }
        Surface::compile(ledges).expect("a floor")
    }

    const IDENTITY: Quat = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    fn world() -> World {
        let mut w = World::new();
        w.time_start();
        w.set_gravity([0.0, -9.81, 0.0]);
        w
    }

    #[test]
    fn a_fixed_floor_is_a_static_body_with_its_surface() {
        let mut w = world();
        let f = w.add_body(BodyDesc { fixed: true, ..BodyDesc::default() }, Shape::Surface(floor()), [0.0; 3], IDENTITY);
        assert!(w.cores[f].unmovable);
        assert_eq!(w.body_pose(f).0, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_ball_falls_onto_the_floor_and_rests_on_it() {
        let mut w = world();
        let _floor = w.add_body(BodyDesc { fixed: true, ..BodyDesc::default() }, Shape::Surface(floor()), [0.0; 3], IDENTITY);
        let radius = 1.0f32;
        let ball = w.add_body(BodyDesc { mass: 1.0, elasticity: 0.1, friction: 0.5, ..BodyDesc::default() }, Shape::Sphere(radius), [0.0, 5.0, 0.0], IDENTITY);
        // three seconds of falling and settling, in game ticks of 1/60 s
        for _ in 0..180 {
            w.advance(1.0 / 60.0);
        }
        let y = w.body_pose(ball).0[1];
        assert!((y - f64::from(radius)).abs() < 0.2, "resting height {y}");
        assert!(w.body_touching(ball), "the ball has a contact");
    }
}

