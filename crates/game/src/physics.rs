// The physics of the game: the player's ball, the level's loose boxes and balls, and the level's physical geometry.
// Rigid-body dynamics come from rapier (Apache-2.0, a library we depend on, not code we copied). What is ours here:
// the level as a static triangle mesh, the original game's material values (docs/physics.md, sections 3 and 4) with
// pair friction and elasticity as products, and the one-step-per-tick contract that keeps replays reproducible.

use crate::collide::Triangle;
use glam::{Quat, Vec3};
use rapier3d::prelude::*;

/// Steps per second of the original's own time (its "PSI"); its control forces are impulses per step.
pub const PSI_PER_SECOND: f32 = 66.0;
/// Gravity of the game (docs/physics.md, section 1).
pub const GRAVITY: f32 = 20.0;
/// Elasticity and friction of the level's floors and rails and of PH modules (`Physicalize_Floors`).
pub const SURFACE_ELASTICITY: f32 = 0.3;
pub const SURFACE_FRICTION: f32 = 0.7;
/// Solver iterations per step; one step is one tick (1/60 s).
const SOLVER_ITERATIONS: usize = 8;
const TICK: f32 = 1.0 / 60.0;

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
    physics: PhysicsWorld,
    handles: Vec<RigidBodyHandle>,
    colliders: Vec<ColliderHandle>,
    /// Poses after the last step, for drawing.
    pub bodies: Vec<Body>,
    /// Whether body 0 (the ball) is touching something.
    pub grounded: bool,
}

fn vector(v: Vec3) -> Vector {
    Vector::new(v.x, v.y, v.z)
}

fn vec3(v: Vector) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn quat(r: Rotation) -> Quat {
    Quat::from_xyzw(r.x, r.y, r.z, r.w)
}

fn rotation(q: Quat) -> Rotation {
    Rotation::from_xyzw(q.x, q.y, q.z, q.w)
}

/// Pair coefficients are products of the two materials' values, as in the original.
fn coefficients(c: ColliderBuilder, friction: f32, elasticity: f32) -> ColliderBuilder {
    c.friction(friction)
        .restitution(elasticity)
        .friction_combine_rule(CoefficientCombineRule::Multiply)
        .restitution_combine_rule(CoefficientCombineRule::Multiply)
}

impl World {
    /// A world with the level's physical triangles as one fixed mesh.
    pub fn new(triangles: &[Triangle]) -> World {
        let mut physics = PhysicsWorld::new();
        physics.gravity = Vector::new(0.0, -GRAVITY, 0.0);
        physics.integration_parameters.dt = TICK;
        physics.integration_parameters.num_solver_iterations = SOLVER_ITERATIONS;
        let mut world = World {
            physics,
            handles: Vec::new(),
            colliders: Vec::new(),
            bodies: Vec::new(),
            grounded: false,
        };
        let mut vertices = Vec::with_capacity(triangles.len() * 3);
        let mut indices = Vec::with_capacity(triangles.len());
        for t in triangles {
            let k = vertices.len() as u32;
            vertices.extend([vector(t.a), vector(t.b), vector(t.c)]);
            indices.push([k, k + 1, k + 2]);
        }
        if let Ok(mesh) = ColliderBuilder::trimesh(vertices, indices) {
            let collider = coefficients(mesh, SURFACE_FRICTION, SURFACE_ELASTICITY);
            world.physics.colliders.insert(collider);
        }
        world
    }

    fn add(&mut self, shape: ColliderBuilder, m: Material, pos: Vec3, rot: Quat) -> usize {
        let body = RigidBodyBuilder::dynamic()
            .pose(Pose::from_parts(vector(pos), rotation(rot)))
            .linear_damping(m.lin_damp)
            .angular_damping(m.rot_damp);
        let collider = coefficients(shape, m.friction, m.elasticity).mass(m.mass);
        let (handle, col) = self.physics.insert(body, collider);
        self.handles.push(handle);
        self.colliders.push(col);
        self.bodies.push(Body { pos, rot });
        self.handles.len() - 1
    }

    /// The player's ball, radius 2 in every case. By convention the first body.
    pub fn add_ball(&mut self, radius: f32, m: Material, pos: Vec3, rot: Quat) -> usize {
        self.add(ColliderBuilder::ball(radius), m, pos, rot)
    }

    /// A box with the given half sizes.
    pub fn add_box(&mut self, half: Vec3, m: Material, pos: Vec3, rot: Quat) -> usize {
        self.add(ColliderBuilder::cuboid(half.x, half.y, half.z), m, pos, rot)
    }

    /// A loose sphere.
    pub fn add_sphere(&mut self, radius: f32, m: Material, pos: Vec3, rot: Quat) -> usize {
        self.add(ColliderBuilder::ball(radius), m, pos, rot)
    }

    pub fn state(&self, i: usize) -> Option<State> {
        let b = self.physics.bodies.get(*self.handles.get(i)?)?;
        Some(State {
            pos: vec3(b.translation()),
            vel: vec3(b.linvel()),
            rot: quat(*b.rotation()),
            spin: vec3(b.angvel()),
        })
    }

    /// Puts a body where the simulation says it is. The ball's state is owned by the simulation, so a respawn or a
    /// change of ball takes effect here, before the step.
    pub fn set_state(&mut self, i: usize, s: State) {
        let Some(&h) = self.handles.get(i) else {
            return;
        };
        if let Some(b) = self.physics.bodies.get_mut(h) {
            b.set_position(Pose::from_parts(vector(s.pos), rotation(s.rot)), true);
            b.set_linvel(vector(s.vel), true);
            b.set_angvel(vector(s.spin), true);
        }
    }

    /// Advances the world by one tick. `push` is an impulse for one body (the player's push).
    pub fn step(&mut self, push: Option<(usize, Vec3)>) {
        if let Some((i, impulse)) = push
            && let Some(b) = self
                .handles
                .get(i)
                .and_then(|&h| self.physics.bodies.get_mut(h))
        {
            b.apply_impulse(vector(impulse), true);
        }
        self.physics.step();
        for (i, &h) in self.handles.iter().enumerate() {
            if let Some(b) = self.physics.bodies.get(h) {
                self.bodies[i] = Body {
                    pos: vec3(b.translation()),
                    rot: quat(*b.rotation()),
                };
            }
        }
        self.grounded = self.colliders.first().is_some_and(|&c| {
            self.physics
                .narrow_phase
                .contact_pairs_with(c)
                .any(|p| p.has_any_active_contact())
        });
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
        // The lowest corner of the tilted box sits on the floor: the centre is above half the diagonal's reach.
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
                w.step(Some((ball, Vec3::X * force * PSI_PER_SECOND / 60.0)));
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
        // Pair elasticity 0.2 × 0.3 = 0.06: a few percent of the impact, not the rubber bounce of before (0.6).
        assert!(after < 0.2 * before, "rebound {after} from impact {before}");
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
        for _ in 0..480 {
            w.step(Some((ball, Vec3::X * 1.0)));
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
}
