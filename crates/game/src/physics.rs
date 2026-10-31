// The physics of the game: the player's ball, the level's loose boxes and balls, and the level's physical geometry.
// The dynamics are IVP's (crates/ivp, a port of OpenBallance's physics, GPL-3.0): the level's floors as one fixed
// compact surface, the boxes as convex surfaces, the balls as spheres, with the original game's material values
// (docs/physics.md, sections 3 and 4). A tick is one step of 1/60 s, which the physics runs as its own time steps of
// 1/66 s.

use crate::collide::Triangle;
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
/// The physics runs this many times faster than the frames while playing: the original's "Set Physics Globals" sets
/// the time factor to 2 (checked in the C game: its physics time advances 2 s for every second of frames).
const TIME_FACTOR: f64 = 2.0;

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

pub struct World {
    physics: IvpWorld,
    /// The IVP body of each of the game's bodies, in creation order; none while a body is taken out of the physics
    /// (the ball inside a transformer).
    handles: Vec<Option<usize>>,
    /// Poses after the last step, for drawing.
    pub bodies: Vec<Body>,
    /// Whether body 0 (the ball) is touching something.
    pub grounded: bool,
    /// The push controller of each body that has been pushed (see `set_push`).
    pushers: Vec<Option<usize>>,
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

fn ivp_quat(q: Quat) -> IvpQuat {
    IvpQuat {
        x: f64::from(q.x),
        y: f64::from(q.y),
        z: f64::from(q.z),
        w: f64::from(q.w),
    }
}

impl World {
    /// A world with the level's physical triangles as one fixed surface (a convex piece per triangle, as IVP keeps
    /// the level's floors).
    pub fn new(triangles: &[Triangle]) -> World {
        let mut physics = IvpWorld::new();
        physics.time_start();
        physics.set_gravity([0.0, -GRAVITY, 0.0]);
        let ledges: Vec<ledge::Ledge> = triangles
            .iter()
            .filter_map(|t| ledge::triangle(vec3_f64(t.a), vec3_f64(t.b), vec3_f64(t.c)))
            .collect();
        let mut world = World {
            physics,
            handles: Vec::new(),
            bodies: Vec::new(),
            grounded: false,
            pushers: Vec::new(),
        };
        if let Some(floor) = Surface::compile(ledges) {
            let desc = BodyDesc {
                fixed: true,
                friction: SURFACE_FRICTION,
                elasticity: SURFACE_ELASTICITY,
                ..BodyDesc::default()
            };
            world.physics.add_body(desc, Shape::Surface(floor), [0.0; 3], ivp_quat(Quat::IDENTITY));
        }
        world
    }

    fn add(&mut self, shape: Shape, m: Material, pos: Vec3, rot: Quat) -> usize {
        let desc = BodyDesc {
            // the floor is entity 0; the game's bodies follow in creation order, so the ball is entity 1
            entity: self.handles.len() as u32 + 1,
            mass: m.mass,
            friction: m.friction,
            elasticity: m.elasticity,
            lin_damp: m.lin_damp,
            rot_damp: m.rot_damp,
            ..BodyDesc::default()
        };
        let id = self.physics.add_body(desc, shape, vec3_f64(pos), ivp_quat(rot));
        self.handles.push(Some(id));
        self.bodies.push(Body { pos, rot });
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
    pub fn replace_ball(&mut self, i: usize, radius: f32, m: Material, pos: Vec3, rot: Quat) {
        if i >= self.handles.len() {
            return;
        }
        self.remove(i);
        let desc = BodyDesc {
            entity: i as u32 + 1,
            mass: m.mass,
            friction: m.friction,
            elasticity: m.elasticity,
            lin_damp: m.lin_damp,
            rot_damp: m.rot_damp,
            ..BodyDesc::default()
        };
        let id = self.physics.add_body(desc, Shape::Sphere(radius), vec3_f64(pos), ivp_quat(rot));
        self.handles[i] = Some(id);
        self.bodies[i] = Body { pos, rot };
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

    /// The player's ball, with its radius. By convention the first body.
    pub fn add_ball(&mut self, radius: f32, m: Material, pos: Vec3, rot: Quat) -> usize {
        self.add(Shape::Sphere(radius), m, pos, rot)
    }

    /// A box with the given half sizes: a convex surface of its eight corners. A box with positive half sizes always
    /// has a hull and a surface, so the two lookups cannot fail for the game's boxes.
    #[allow(clippy::expect_used)]
    pub fn add_box(&mut self, half: Vec3, m: Material, pos: Vec3, rot: Quat) -> usize {
        let mut corners = Vec::with_capacity(8);
        for &x in &[-1.0f64, 1.0] {
            for &y in &[-1.0f64, 1.0] {
                for &z in &[-1.0f64, 1.0] {
                    corners.push([x * f64::from(half.x), y * f64::from(half.y), z * f64::from(half.z)]);
                }
            }
        }
        let hull = ivp::convex::hull(&corners).expect("a box has a hull");
        let surface = Surface::compile(vec![hull]).expect("a box surface");
        self.add(Shape::Surface(surface), m, pos, rot)
    }

    /// A loose sphere.
    pub fn add_sphere(&mut self, radius: f32, m: Material, pos: Vec3, rot: Quat) -> usize {
        self.add(Shape::Sphere(radius), m, pos, rot)
    }

    /// The triangles of the level's floor near a point, as the physics sees them (the floor is body 0).
    pub fn floor_near(&self, p: Vec3, r: f32) -> Vec<[Vec3; 3]> {
        self.physics
            .surface_triangles_near(0, [f64::from(p.x), f64::from(p.y), f64::from(p.z)], f64::from(r))
            .into_iter()
            .map(|t| t.map(|v| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)))
            .collect()
    }

    /// The contact points the ball takes part in, for the watch log: those whose entities include the ball (entity 1).
    pub fn ball_contacts(&self) -> Vec<ivp::glue::ContactReadout> {
        self.physics.contacts_of(1)
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

    /// Advances the world by one tick. `push` is an impulse for one body (the player's push), at its centre.
    pub fn step(&mut self, push: Option<(usize, Vec3)>) {
        if let Some((i, impulse)) = push
            && let Some(&Some(id)) = self.handles.get(i)
        {
            let centre = self.physics.body_state(id).pos;
            self.physics
                .body_impulse(id, centre, [impulse.x, impulse.y, impulse.z]);
        }
        self.physics.advance(TIME_FACTOR * TICK);
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
        self.grounded = self.handles.first().copied().flatten().is_some_and(|id| self.physics.body_touching(id));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// A flat floor at y = 0, 200 units square.
    fn floor() -> Vec<Triangle> {
        let s = 100.0;
        let p = [
            Vec3::new(-s, 0.0, -s),
            Vec3::new(s, 0.0, -s),
            Vec3::new(s, 0.0, s),
            Vec3::new(-s, 0.0, s),
        ];
        let tri = |a, b, c| Triangle { a, b, c };
        vec![tri(p[0], p[2], p[1]), tri(p[0], p[3], p[2])]
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
        assert!((ratio - 0.06f32.sqrt()).abs() < 0.02, "rebound {after} from impact {before}, ratio {ratio}");
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
        assert!(contacts.iter().any(|c| c.entities.contains(&0)), "the floor is entity 0: {contacts:?}");
    }
}
