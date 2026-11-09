// Port of OpenBallance src/phys/phys.c and the body parts of phys_core.c (ivp_object_init_core): the physics API the game
// uses. A body is an object with a core (its mass centre) and, for collision, a shape: a sphere (the ball, a loose
// sphere) or a compact surface (a convex box, a level's floors). Bodies are referred to by their core index.
//
// The game's pose convention is the object's origin; the core sits at the mass centre, so an object's origin is the
// core's position plus its rotated shift, which is the negated mass centre (`Core::shift`).

use crate::collision::ObjKind;
use crate::core::{Controller, MT_NOT_SIM, MT_STATIC, World};
use crate::glue::to_p3;
use crate::ledge::Surface;
use crate::math::{self, Matrix, Quat};

type V3 = [f64; 3];

/// A body's description (`PhysBodyDesc`): the game's entity id, fixedness, collision, the material, the mass, the
/// dampings, the no-collision group, and the mass centre override (`shift`, in object space; None takes the surface's
/// own centre).
#[derive(Clone, Copy, Debug)]
pub struct BodyDesc {
    pub entity: u32,
    /// The no-collision group: equal non-empty groups do not collide. All zero bytes is no group.
    pub group: [u8; 8],
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
            entity: 0,
            group: [0; 8],
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

/// The rotation turned by `w` (core axes, radians per second) for `dt` seconds, as a quaternion.
fn spin_turn(w: &[f64; 3], dt: f64) -> math::Quat {
    let (wx, wy, wz) = (w[0], w[1], w[2]);
    let speed = (wx * wx + wy * wy + wz * wz).sqrt();
    if speed < 1e-12 {
        return math::Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        };
    }
    let half = 0.5 * speed * dt;
    let k = half.sin() / speed;
    math::Quat {
        x: wx * k,
        y: wy * k,
        z: wz * k,
        w: half.cos(),
    }
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
    ((f64::from(v[0]) * f64::from(v[0])
        + f64::from(v[1]) * f64::from(v[1])
        + f64::from(v[2]) * f64::from(v[2]))
    .sqrt()) as f32
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
        self.cores[c].entity = desc.entity;
        if self.coll.groups.len() <= c {
            self.coll.groups.resize(c + 1, [0; 8]);
        }
        self.coll.groups[c] = desc.group;

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
        // the radius about the mass centre and the surface's deviation from it; a ball has neither (0x10009f00)
        let (radius_surface, radius_dev) = match &surface {
            Some(s) => s.radius_dev(mc),
            None => (0.0, 0.0),
        };
        {
            let core = &mut self.cores[c];
            core.rot_inertia = rot_inertia;
            core.mass = mass as f32;
            core.speed_damp = desc.lin_damp;
            core.rot_speed_damp = [desc.rot_damp; 3];
            core.upper_limit_radius = extra + radius_surface;
            // The rotation's share of the hull growth is the spin times this deviation (`calc_rotation_info`).
            core.max_surface_deviation = radius_dev;
            core.speed = [0.0; 3];
            core.rot_speed = [0.0; 3];
            core.movement_state = if desc.fixed { MT_STATIC } else { MT_NOT_SIM };
        }
        // the inverses, the equal-inertia flag and the inverse diameter (`calc_calc`), then the core ready for the
        // next PSI, as the upstream's init ends (0x10009f00)
        self.calc_calc(c);
        let next = self.env.time_of_next_psi - self.env.current_time;
        self.init_core_for_simulation(c, Self::inv_dt(next));
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
            // The next PSI's rotation follows from the spin, as the physics integrates it. An impact between PSIs
            // recomputes the spin from the rotation between the last and the next PSI (`core_sync`); with both equal
            // the spin would vanish, so the next rotation is the current one turned by the spin for one PSI.
            core.q_next = math::quat_mul(&s.rot, &spin_turn(&spin_cs, self.env.delta_psi));
        }
        self.wake_body(c);
    }

    /// An impulse at a world point of a body, applied at the next PSI (`ivp_body_async_push_ws`, with the wake).
    pub fn body_impulse(&mut self, c: usize, point_ws: V3, impulse: [f32; 3]) {
        self.wake_body(c);
        self.async_push_core_ws(c, point_ws, impulse);
    }

    /// A body-local impulse (`phys_impulse` with `local`): the point is in core space and the impulse is rotated by the
    /// pose at the current time, applied at the next PSI. The body is woken here, where the upstream's glue woke it
    /// before the call.
    pub fn body_impulse_local(&mut self, c: usize, point_cs: V3, impulse: [f32; 3]) {
        self.wake_body(c);
        let m = self.core_m_world_f_core_at(c, self.env.current_time);
        let imp_cs = math::rtmul(&m.r, &to_v3(impulse));
        // the upstream passes the impulse as the world impulse too, as it does here
        self.async_push_core(c, to_p3(point_cs), to_p3(imp_cs), impulse);
    }

    /// Sets the time factor the frame time is scaled by (`phys_set_time_factor`): 2 while playing, 0 paused. The factor
    /// is in thousandths of a second per millisecond, as the upstream keeps it.
    pub fn set_time_factor(&mut self, f: f32) {
        self.env.time_factor = f * 0.001;
    }

    /// The frame's physics step (`phys_frame`): the frame time is smoothed over the last frames, scaled by the time
    /// factor, and the simulation runs to that time. Returns the time reached. The write-back of the awake bodies'
    /// poses is the caller's (`body_pose`).
    pub fn frame(&mut self, delta_ms: f32) -> f64 {
        self.smoothed_ms = (delta_ms + 3.0 * self.smoothed_ms) * 0.25;
        let target = self.env.current_time + f64::from(self.smoothed_ms * self.env.time_factor);
        self.time_simulate_until(target);
        target
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
        let p = [
            [-10.0, 0.0, -10.0],
            [10.0, 0.0, -10.0],
            [10.0, 0.0, 10.0],
            [-10.0, 0.0, 10.0],
        ];
        let mut ledges = Vec::new();
        for tri in [[0, 1, 2], [0, 2, 3]] {
            ledges
                .push(crate::ledge::triangle(p[tri[0]], p[tri[1]], p[tri[2]]).expect("a triangle"));
        }
        Surface::compile(ledges).expect("a floor")
    }

    const IDENTITY: Quat = Quat {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    fn world() -> World {
        let mut w = World::new();
        w.time_start();
        w.set_gravity([0.0, -9.81, 0.0]);
        w
    }

    #[test]
    fn a_fixed_floor_is_a_static_body_with_its_surface() {
        let mut w = world();
        let f = w.add_body(
            BodyDesc {
                fixed: true,
                ..BodyDesc::default()
            },
            Shape::Surface(floor()),
            [0.0; 3],
            IDENTITY,
        );
        assert!(w.cores[f].unmovable);
        assert_eq!(w.body_pose(f).0, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_ball_has_no_surface_deviation_and_a_surface_has_its_own() {
        // The rotation's share of the hull growth is spin times the surface deviation: none for a ball (the upstream
        // leaves it 0), the surface's own for a polygon, never the core's initial 1000.
        let mut w = world();
        let f = w.add_body(
            BodyDesc {
                fixed: true,
                ..BodyDesc::default()
            },
            Shape::Surface(floor()),
            [0.0; 3],
            IDENTITY,
        );
        let ball = w.add_body(
            BodyDesc {
                mass: 1.0,
                ..BodyDesc::default()
            },
            Shape::Sphere(2.0),
            [0.0, 5.0, 0.0],
            IDENTITY,
        );
        assert_eq!(w.cores[ball].max_surface_deviation, 0.0);
        // a ball's inertias are equal, and its inverse diameter is that of its radius
        assert!(w.cores[ball].rot_inertias_equal);
        assert_eq!(w.cores[ball].inv_object_diameter, 0.25);
        let (_, dev) = floor().radius_dev(floor().mass_center);
        assert_eq!(w.cores[f].max_surface_deviation, dev);
        assert!(dev < 100.0);
    }

    #[test]
    fn a_ball_falls_onto_the_floor_and_rests_on_it() {
        let mut w = world();
        let _floor = w.add_body(
            BodyDesc {
                fixed: true,
                ..BodyDesc::default()
            },
            Shape::Surface(floor()),
            [0.0; 3],
            IDENTITY,
        );
        let radius = 1.0f32;
        let ball = w.add_body(
            BodyDesc {
                mass: 1.0,
                elasticity: 0.1,
                friction: 0.5,
                ..BodyDesc::default()
            },
            Shape::Sphere(radius),
            [0.0, 5.0, 0.0],
            IDENTITY,
        );
        // three seconds of falling and settling, in game ticks of 1/60 s
        for _ in 0..180 {
            w.advance(1.0 / 60.0);
        }
        let y = w.body_pose(ball).0[1];
        assert!((y - f64::from(radius)).abs() < 0.2, "resting height {y}");
        assert!(w.body_touching(ball), "the ball has a contact");
    }

    /// A ball rolling off the edge of a plate must fall: after it leaves the plate its upward speed stays small.
    #[test]
    fn a_ball_rolling_off_an_edge_does_not_launch() {
        let mut w = world();
        // a 20 x 20 plate at y = 0, centred on the origin
        let p = [
            [-10.0, 0.0, -10.0],
            [10.0, 0.0, -10.0],
            [10.0, 0.0, 10.0],
            [-10.0, 0.0, 10.0],
        ];
        let plate = Surface::compile(vec![
            crate::ledge::triangle(p[0], p[2], p[1]).unwrap(),
            crate::ledge::triangle(p[0], p[3], p[2]).unwrap(),
        ])
        .unwrap();
        let _f = w.add_body(
            BodyDesc {
                fixed: true,
                ..BodyDesc::default()
            },
            Shape::Surface(plate),
            [0.0; 3],
            IDENTITY,
        );
        let ball = w.add_body(
            BodyDesc::default(),
            Shape::Sphere(1.0),
            [5.0, 1.0, 0.0],
            IDENTITY,
        );
        w.set_body_state(
            ball,
            BodyState {
                pos: [5.0, 1.0, 0.0],
                rot: IDENTITY,
                vel: [3.0, 0.0, 0.0],
                spin: [0.0; 3],
            },
        );
        let mut max_up = 0.0f32;
        let mut max_x = 0.0f64;
        for _ in 0..180 {
            w.advance(1.0 / 60.0);
            let s = w.body_state(ball);
            max_x = max_x.max(s.pos[0]);
            if s.pos[0] > 10.5 {
                max_up = max_up.max(s.vel[1]);
            }
        }
        assert!(max_x > 10.5, "the ball left the plate: x {max_x}");
        assert!(
            max_up < 0.5,
            "the ball launched upward after the edge: vy {max_up}"
        );
    }

    /// The same off a box's top edge: a convex block, not a plate.
    #[test]
    fn a_ball_rolling_off_a_block_edge_does_not_launch() {
        let mut w = world();
        let mut corners = Vec::new();
        for &x in &[-10.0f64, 10.0] {
            for &y in &[0.0f64, 2.0] {
                for &z in &[-10.0f64, 10.0] {
                    corners.push([x, y, z]);
                }
            }
        }
        let block = Surface::compile(vec![crate::convex::hull(&corners).unwrap()]).unwrap();
        let _b = w.add_body(
            BodyDesc {
                fixed: true,
                ..BodyDesc::default()
            },
            Shape::Surface(block),
            [0.0; 3],
            IDENTITY,
        );
        let ball = w.add_body(
            BodyDesc::default(),
            Shape::Sphere(1.0),
            [5.0, 3.0, 0.0],
            IDENTITY,
        );
        w.set_body_state(
            ball,
            BodyState {
                pos: [5.0, 3.0, 0.0],
                rot: IDENTITY,
                vel: [3.0, 0.0, 0.0],
                spin: [0.0; 3],
            },
        );
        let mut max_up = 0.0f32;
        let mut max_x = 0.0f64;
        for _ in 0..180 {
            w.advance(1.0 / 60.0);
            let s = w.body_state(ball);
            max_x = max_x.max(s.pos[0]);
            if s.pos[0] > 10.5 {
                max_up = max_up.max(s.vel[1]);
            }
        }
        assert!(max_x > 10.5, "the ball left the block: x {max_x}");
        assert!(
            max_up < 0.5,
            "the ball launched upward after the edge: vy {max_up}"
        );
    }

    /// A ball falling onto a tilted triangle, its lowest corner under the centre, with a sideways speed. The contact
    /// with the edge at the foot of the slope must bring the ball to rest, not pump it: the port once pushed the ball
    /// up to 136 u/s here, where the upstream gives about 16.
    #[test]
    fn a_ball_falling_onto_a_tilted_triangle_does_not_explode() {
        let mut w = world();
        w.set_gravity([0.0, -20.0, 0.0]);
        let slope = Surface::compile(vec![
            crate::ledge::triangle([0.0, -2.0, -1.4], [2.5, -2.7, 1.1], [0.0, -2.0, 1.1])
                .expect("a triangle"),
        ])
        .expect("a slope");
        w.add_body(
            BodyDesc {
                fixed: true,
                ..BodyDesc::default()
            },
            Shape::Surface(slope),
            [0.0; 3],
            IDENTITY,
        );
        let desc = BodyDesc {
            mass: 1.9,
            friction: 0.8,
            elasticity: 0.2,
            lin_damp: 0.9,
            rot_damp: 0.1,
            ..BodyDesc::default()
        };
        let ball = w.add_body(desc, Shape::Sphere(2.0), [0.0; 3], IDENTITY);
        w.set_body_state(
            ball,
            BodyState {
                pos: [0.0; 3],
                rot: IDENTITY,
                vel: [-2.3, -2.7, 0.0],
                spin: [0.0; 3],
            },
        );
        let mut top = 0.0f32;
        for _ in 0..120 {
            w.advance(1.0 / 60.0);
            let v = w.body_state(ball).vel;
            top = top.max((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt());
        }
        assert!(top < 25.0, "the ball gained speed on the slope: {top}");
    }

    /// The game sets the ball's state every tick. An impact between two PSIs synchronises the core from the rotation
    /// between its last and next PSI (`core_sync`); the state set must keep that consistent with the spin, or the spin
    /// vanishes at the impact (the game lost a rolling ball's spin at a landing this way).
    #[test]
    fn a_state_set_keeps_the_spin_that_an_impact_synchronises_from() {
        let mut w = world();
        let ball = w.add_body(BodyDesc::default(), Shape::Sphere(2.0), [0.0; 3], IDENTITY);
        let spin = [0.0, 0.0, -6.0];
        w.set_body_state(
            ball,
            BodyState {
                pos: [0.0; 3],
                rot: IDENTITY,
                vel: [12.0, 0.0, 0.0],
                spin,
            },
        );
        w.advance(1.0 / 60.0);
        let s = w.body_state(ball);
        w.set_body_state(ball, s);
        w.core_sync(ball);
        let got = w.cores[ball].rot_speed;
        assert!(
            (got[2] + 6.0).abs() < 0.05,
            "the synchronised spin is {got:?}, expected the spin of -6 about z"
        );
    }
}
