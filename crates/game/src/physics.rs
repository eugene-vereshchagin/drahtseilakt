// Rigid bodies: the player's ball and the level's boxes. They fall under gravity, push one another and the level's
// physical geometry, and rub against it. The solver is sequential impulses: every contact gets a normal impulse
// that stops its penetration and reflects the approach by the pair's elasticity, and a friction impulse limited by
// the pair's friction times the normal impulse. Rolling without slipping is not written anywhere: a ball whose
// contact point is at rest turns because friction says so. Material values are the original game's (docs/physics.md,
// sections 3 and 4); the simulation steps are its 66 steps per second, split into substeps here.

use crate::collide::Collider;
use glam::{Mat3, Quat, Vec3};
use std::sync::Arc;

/// Substeps per tick (1/60 s). Boxes stacked on each other need more than one.
pub const SUBSTEPS: u32 = 6;
/// Steps per second of the original's own time (its "PSI"); its control forces are impulses per step.
pub const PSI_PER_SECOND: f32 = 66.0;
/// Gravity of the game (docs/physics.md, section 1).
pub const GRAVITY: f32 = 20.0;
/// Elasticity and friction of the level's floors and rails and of PH modules; a pair multiplies the two.
pub const SURFACE_ELASTICITY: f32 = 0.3;
pub const SURFACE_FRICTION: f32 = 0.7;

const ITERATIONS: u32 = 10;
/// Penetration left alone, and how fast a deeper one is pushed out of the way.
const SLOP: f32 = 0.005;
const PUSH_OUT: f32 = 0.2;
const MAX_PUSH: f32 = 3.0;
/// Approach speed below which a contact does not bounce.
const REST_SPEED: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Sphere {
        radius: f32,
    },
    /// Half the size along each local axis.
    Box {
        half: Vec3,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub shape: Shape,
    pub pos: Vec3,
    pub vel: Vec3,
    pub rot: Quat,
    /// World angular velocity.
    pub ang: Vec3,
    /// Zero for a body that does not move.
    pub inv_mass: f32,
    /// Inverse inertia along the body's own axes.
    pub inv_inertia: Vec3,
    pub friction: f32,
    pub elasticity: f32,
    pub lin_damp: f32,
    pub rot_damp: f32,
}

impl Body {
    /// A solid sphere: inertia 0.4 m r².
    pub fn sphere(
        radius: f32,
        mass: f32,
        friction: f32,
        elasticity: f32,
        lin_damp: f32,
        rot_damp: f32,
    ) -> Body {
        let inertia = 0.4 * mass * radius * radius;
        Body {
            shape: Shape::Sphere { radius },
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            rot: Quat::IDENTITY,
            ang: Vec3::ZERO,
            inv_mass: 1.0 / mass,
            inv_inertia: Vec3::splat(1.0 / inertia),
            friction,
            elasticity,
            lin_damp,
            rot_damp,
        }
    }

    /// A solid box: inertia m/3 (b² + c²) along each axis for half sizes b, c.
    pub fn cuboid(
        half: Vec3,
        mass: f32,
        friction: f32,
        elasticity: f32,
        lin_damp: f32,
        rot_damp: f32,
    ) -> Body {
        let (x, y, z) = (half.x * half.x, half.y * half.y, half.z * half.z);
        let i = Vec3::new(y + z, x + z, x + y) * (mass / 3.0);
        Body {
            shape: Shape::Box { half },
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            rot: Quat::IDENTITY,
            ang: Vec3::ZERO,
            inv_mass: 1.0 / mass,
            inv_inertia: Vec3::new(1.0 / i.x, 1.0 / i.y, 1.0 / i.z),
            friction,
            elasticity,
            lin_damp,
            rot_damp,
        }
    }

    fn inv_inertia_world(&self) -> Mat3 {
        let r = Mat3::from_quat(self.rot);
        r * Mat3::from_diagonal(self.inv_inertia) * r.transpose()
    }

    /// Velocity of the material point at `offset` from the centre.
    fn point_velocity(&self, offset: Vec3) -> Vec3 {
        self.vel + self.ang.cross(offset)
    }

    /// The eight corners of a box in the world.
    fn corners(&self, half: Vec3) -> [Vec3; 8] {
        let mut out = [Vec3::ZERO; 8];
        for (k, c) in out.iter_mut().enumerate() {
            let s = Vec3::new(
                if k & 1 == 0 { -1.0 } else { 1.0 },
                if k & 2 == 0 { -1.0 } else { 1.0 },
                if k & 4 == 0 { -1.0 } else { 1.0 },
            );
            *c = self.pos + self.rot * (s * half);
        }
        out
    }
}

/// A contact: body `a` is pushed along `normal`, which points from `b` (or from the level, when `b` is None) to `a`.
#[derive(Clone, Copy, Debug)]
struct Contact {
    a: usize,
    b: Option<usize>,
    point: Vec3,
    normal: Vec3,
    depth: f32,
    friction: f32,
    elasticity: f32,
    /// Approach speed before the step, for the bounce.
    approach: f32,
    normal_impulse: f32,
    tangent_impulse: f32,
}

#[derive(Clone)]
pub struct World {
    pub bodies: Vec<Body>,
    pub statics: Arc<Collider>,
    /// Whether body 0 touched something it can stand on this step.
    pub grounded: bool,
}

impl World {
    pub fn new(statics: Arc<Collider>) -> World {
        World {
            bodies: Vec::new(),
            statics,
            grounded: false,
        }
    }

    /// Advances the world by one tick. `drive` is an acceleration added to body `index` during the tick (the
    /// player's push); the original's control force is an impulse per step of its own, which comes to the same.
    pub fn step(&mut self, dt: f32, drive: Option<(usize, Vec3)>) {
        let h = dt / SUBSTEPS as f32;
        self.grounded = false;
        for _ in 0..SUBSTEPS {
            self.substep(h, drive);
        }
    }

    fn substep(&mut self, h: f32, drive: Option<(usize, Vec3)>) {
        for (i, b) in self.bodies.iter_mut().enumerate() {
            if b.inv_mass == 0.0 {
                continue;
            }
            b.vel.y -= GRAVITY * h;
            if let Some((j, acc)) = drive {
                if i == j {
                    b.vel += acc * h;
                }
            }
        }
        let mut contacts = self.contacts();
        self.solve(&mut contacts, h);
        for b in self.bodies.iter_mut() {
            if b.inv_mass == 0.0 {
                continue;
            }
            b.vel *= (-b.lin_damp * h).exp();
            b.ang *= (-b.rot_damp * h).exp();
            b.pos += b.vel * h;
            b.rot = (Quat::from_scaled_axis(b.ang * h) * b.rot).normalize();
        }
    }

    /// The contacts of this substep, at the current positions.
    fn contacts(&mut self) -> Vec<Contact> {
        let mut out = Vec::new();
        let statics = Arc::clone(&self.statics);
        for i in 0..self.bodies.len() {
            let b = self.bodies[i];
            if b.inv_mass == 0.0 {
                continue;
            }
            let (fric_s, elas_s) = (SURFACE_FRICTION, SURFACE_ELASTICITY);
            let mut push = |point: Vec3, normal: Vec3, depth: f32, other: Option<usize>| {
                let (friction, elasticity) = match other {
                    Some(j) => (
                        b.friction * self.bodies[j].friction,
                        b.elasticity * self.bodies[j].elasticity,
                    ),
                    None => (b.friction * fric_s, b.elasticity * elas_s),
                };
                out.push(Contact {
                    a: i,
                    b: other,
                    point,
                    normal,
                    depth,
                    friction,
                    elasticity,
                    approach: 0.0,
                    normal_impulse: 0.0,
                    tangent_impulse: 0.0,
                });
            };
            match b.shape {
                Shape::Sphere { radius } => {
                    for c in statics.contacts(b.pos, radius).into_iter().take(4) {
                        push(b.pos - c.normal * radius, c.normal, c.depth, None);
                    }
                }
                Shape::Box { half } => {
                    for corner in b.corners(half) {
                        box_vs_level(&statics, b.pos, corner, &mut push);
                    }
                }
            }
        }
        // Body against body: the ball and boxes, boxes and boxes.
        for i in 0..self.bodies.len() {
            for j in i + 1..self.bodies.len() {
                let (a, b) = (self.bodies[i], self.bodies[j]);
                if a.inv_mass == 0.0 && b.inv_mass == 0.0 {
                    continue;
                }
                let friction = a.friction * b.friction;
                let elasticity = a.elasticity * b.elasticity;
                match (a.shape, b.shape) {
                    (Shape::Sphere { radius }, Shape::Box { half }) => {
                        if let Some((point, normal, depth)) = sphere_vs_box(a.pos, radius, &b, half)
                        {
                            out.push(pair(i, j, point, normal, depth, friction, elasticity));
                        }
                    }
                    (Shape::Box { half }, Shape::Sphere { radius }) => {
                        if let Some((point, normal, depth)) = sphere_vs_box(b.pos, radius, &a, half)
                        {
                            out.push(pair(j, i, point, normal, depth, friction, elasticity));
                        }
                    }
                    (Shape::Box { half: ha }, Shape::Box { half: hb }) => {
                        for c in a.corners(ha) {
                            if let Some((normal, depth)) = corner_in_box(c, &b, hb) {
                                out.push(pair(i, j, c, normal, depth, friction, elasticity));
                            }
                        }
                        for c in b.corners(hb) {
                            if let Some((normal, depth)) = corner_in_box(c, &a, ha) {
                                out.push(pair(j, i, c, normal, depth, friction, elasticity));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        for c in &out {
            if c.a == 0 && c.normal.y > 0.3 {
                self.grounded = true;
            }
        }
        out
    }

    /// Sequential impulses: normal impulses that push apart and bounce, then friction within the cone.
    fn solve(&mut self, contacts: &mut [Contact], h: f32) {
        for c in contacts.iter_mut() {
            let (va, vb) = self.velocities(c);
            c.approach = (va - vb).dot(c.normal);
        }
        for _ in 0..ITERATIONS {
            for c in contacts.iter_mut() {
                // Normal: reach the separating speed the bounce and the penetration ask for.
                let rest = if c.approach < -REST_SPEED {
                    -c.elasticity * c.approach
                } else {
                    0.0
                };
                let bias = ((c.depth - SLOP).max(0.0) * PUSH_OUT / h).min(MAX_PUSH);
                let target = rest.max(bias);
                let relative = self.relative_velocity(c);
                let k = self.effective_mass(c, c.normal);
                if k <= 0.0 {
                    continue;
                }
                let delta = (target - relative.dot(c.normal)) / k;
                let new = (c.normal_impulse + delta).max(0.0);
                let applied = new - c.normal_impulse;
                c.normal_impulse = new;
                self.apply(c, c.normal * applied);

                // Friction: stop the sliding, up to friction times the normal impulse.
                let relative = self.relative_velocity(c);
                let tangent_velocity = relative - c.normal * relative.dot(c.normal);
                let speed = tangent_velocity.length();
                if speed < 1e-6 {
                    continue;
                }
                let t = tangent_velocity / speed;
                let kt = self.effective_mass(c, t);
                if kt <= 0.0 {
                    continue;
                }
                let limit = c.friction * c.normal_impulse;
                let new_t = (c.tangent_impulse - speed / kt).clamp(-limit, limit);
                let applied_t = new_t - c.tangent_impulse;
                c.tangent_impulse = new_t;
                self.apply(c, t * applied_t);
            }
        }
    }

    fn velocities(&self, c: &Contact) -> (Vec3, Vec3) {
        let a = &self.bodies[c.a];
        let va = a.point_velocity(c.point - a.pos);
        let vb = match c.b {
            Some(j) => {
                let b = &self.bodies[j];
                b.point_velocity(c.point - b.pos)
            }
            None => Vec3::ZERO,
        };
        (va, vb)
    }

    fn relative_velocity(&self, c: &Contact) -> Vec3 {
        let (va, vb) = self.velocities(c);
        va - vb
    }

    /// Effective mass of the pair along a unit direction at the contact point.
    fn effective_mass(&self, c: &Contact, d: Vec3) -> f32 {
        let term = |body: &Body, point: Vec3| -> f32 {
            if body.inv_mass == 0.0 {
                return 0.0;
            }
            let r = point - body.pos;
            let rn = r.cross(d);
            body.inv_mass + d.dot(body.inv_inertia_world().mul_vec3(rn).cross(r))
        };
        let a = term(&self.bodies[c.a], c.point);
        let b = c.b.map(|j| term(&self.bodies[j], c.point)).unwrap_or(0.0);
        a + b
    }

    /// Applies an impulse at the contact point: to `a` along +impulse, to `b` against it.
    fn apply(&mut self, c: &Contact, impulse: Vec3) {
        apply_to(&mut self.bodies[c.a], c.point, impulse);
        if let Some(j) = c.b {
            apply_to(&mut self.bodies[j], c.point, -impulse);
        }
    }
}

fn apply_to(b: &mut Body, point: Vec3, impulse: Vec3) {
    if b.inv_mass == 0.0 {
        return;
    }
    b.vel += impulse * b.inv_mass;
    let r = point - b.pos;
    b.ang += b.inv_inertia_world().mul_vec3(r.cross(impulse));
}

fn pair(
    a: usize,
    b: usize,
    point: Vec3,
    normal: Vec3,
    depth: f32,
    friction: f32,
    elasticity: f32,
) -> Contact {
    Contact {
        a,
        b: Some(b),
        point,
        normal,
        depth,
        friction,
        elasticity,
        approach: 0.0,
        normal_impulse: 0.0,
        tangent_impulse: 0.0,
    }
}

/// A corner of a box against the level: the corner is below a triangle that it lies over, on the far side from
/// the box's centre.
fn box_vs_level(
    statics: &Collider,
    centre: Vec3,
    corner: Vec3,
    push: &mut impl FnMut(Vec3, Vec3, f32, Option<usize>),
) {
    for i in statics.nearby(corner, 2.0) {
        let Some(t) = statics.triangles.get(i as usize) else {
            continue;
        };
        let mut n = t.normal;
        if (centre - t.a).dot(n) < 0.0 {
            n = -n;
        }
        let s = (corner - t.a).dot(n);
        if s >= 0.0 {
            continue;
        }
        let projected = corner - n * s;
        if point_in_triangle(projected, t.a, t.b, t.c) {
            push(corner, n, -s, None);
        }
    }
}

/// Whether a point projected on a triangle's plane lies inside it.
fn point_in_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> bool {
    let n = (b - a).cross(c - a);
    let side = |u: Vec3, v: Vec3| (v - u).cross(p - u).dot(n);
    let (s1, s2, s3) = (side(a, b), side(b, c), side(c, a));
    (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0)
}

/// Sphere against a box: the contact normal from the box to the sphere, its point and the depth.
fn sphere_vs_box(centre: Vec3, radius: f32, boxed: &Body, half: Vec3) -> Option<(Vec3, Vec3, f32)> {
    let inv = boxed.rot.inverse();
    let l = inv * (centre - boxed.pos);
    let q = l.clamp(-half, half);
    let d = l - q;
    let dist = d.length();
    if dist > 1e-6 {
        if dist >= radius {
            return None;
        }
        let normal = boxed.rot * (d / dist);
        return Some((boxed.pos + boxed.rot * q, normal, radius - dist));
    }
    // The centre is inside the box: leave by the nearest face.
    let slack = half - l.abs();
    let axis = if slack.x <= slack.y && slack.x <= slack.z {
        Vec3::X
    } else if slack.y <= slack.z {
        Vec3::Y
    } else {
        Vec3::Z
    };
    let sign = if l.dot(axis) < 0.0 { -1.0 } else { 1.0 };
    let face = (half * axis).length();
    let normal = boxed.rot * (axis * sign);
    Some((centre, normal, radius + face - l.dot(axis).abs()))
}

/// A corner inside a box: the normal from the box outward through its nearest face, and the depth.
fn corner_in_box(corner: Vec3, boxed: &Body, half: Vec3) -> Option<(Vec3, f32)> {
    let l = boxed.rot.inverse() * (corner - boxed.pos);
    if l.abs().cmpgt(half).any() {
        return None;
    }
    let slack = half - l.abs();
    let (axis, depth) = if slack.x <= slack.y && slack.x <= slack.z {
        (Vec3::X, slack.x)
    } else if slack.y <= slack.z {
        (Vec3::Y, slack.y)
    } else {
        (Vec3::Z, slack.z)
    };
    let sign = if l.dot(axis) < 0.0 { -1.0 } else { 1.0 };
    Some((boxed.rot * (axis * sign), depth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::{Collider, Triangle};
    use crate::sim::TICK;

    /// A flat floor at y = 0, 200 units square, as two triangles.
    fn floor() -> Arc<Collider> {
        let s = 100.0;
        let p = [
            Vec3::new(-s, 0.0, -s),
            Vec3::new(s, 0.0, -s),
            Vec3::new(s, 0.0, s),
            Vec3::new(-s, 0.0, s),
        ];
        let tri = |a, b, c| Triangle {
            a,
            b,
            c,
            normal: Vec3::Y,
        };
        Arc::new(Collider::new(vec![
            tri(p[0], p[2], p[1]),
            tri(p[0], p[3], p[2]),
        ]))
    }

    fn ball(world: &mut World, at: Vec3) -> usize {
        let mut b = Body::sphere(2.0, 1.9, 0.8, 0.2, 0.9, 0.1);
        b.pos = at;
        world.bodies.push(b);
        world.bodies.len() - 1
    }

    fn boxed(world: &mut World, at: Vec3) -> usize {
        let mut b = Body::cuboid(Vec3::splat(2.5), 1.0, 0.7, 0.3, 0.1, 0.1);
        b.pos = at;
        world.bodies.push(b);
        world.bodies.len() - 1
    }

    #[test]
    fn a_box_rests_on_the_floor() {
        let mut w = World::new(floor());
        let i = boxed(&mut w, Vec3::new(0.0, 8.0, 0.0));
        for _ in 0..240 {
            w.step(TICK, None);
        }
        let b = w.bodies[i];
        assert!((b.pos.y - 2.5).abs() < 0.05, "box rests at {}", b.pos.y);
        assert!(b.vel.length() < 0.05, "box still moves at {:?}", b.vel);
    }

    #[test]
    fn a_ball_pushes_a_box_and_the_box_stays_down() {
        let mut w = World::new(floor());
        let box_i = boxed(&mut w, Vec3::new(6.0, 2.5, 0.0));
        let ball_i = ball(&mut w, Vec3::new(-6.0, 2.0, 0.0));
        let drive = Some((ball_i, Vec3::X * 40.0));
        for _ in 0..240 {
            w.step(TICK, drive);
        }
        let b = w.bodies[box_i];
        assert!(b.pos.x > 8.0, "the box was not pushed: {:?}", b.pos);
        assert!(b.pos.y < 3.0, "the box left the floor: {:?}", b.pos);
    }

    #[test]
    fn a_light_paper_ball_does_not_move_a_box() {
        let mut heavy = World::new(floor());
        let box_h = boxed(&mut heavy, Vec3::new(6.0, 2.5, 0.0));
        let wood = ball(&mut heavy, Vec3::new(-6.0, 2.0, 0.0));
        let mut paper = World::new(floor());
        let box_p = boxed(&mut paper, Vec3::new(6.0, 2.5, 0.0));
        let light = {
            let mut b = Body::sphere(2.0, 0.2, 0.5, 0.4, 1.5, 0.1);
            b.pos = Vec3::new(-6.0, 2.0, 0.0);
            paper.bodies.push(b);
            paper.bodies.len() - 1
        };
        // The same push (an acceleration), so the difference is the mass only.
        for _ in 0..120 {
            heavy.step(TICK, Some((wood, Vec3::X * 20.0)));
            paper.step(TICK, Some((light, Vec3::X * 20.0)));
        }
        let moved_by_wood = heavy.bodies[box_h].pos.x - 6.0;
        let moved_by_paper = paper.bodies[box_p].pos.x - 6.0;
        assert!(
            moved_by_wood > moved_by_paper + 0.5,
            "wood {moved_by_wood}, paper {moved_by_paper}"
        );
    }
}
