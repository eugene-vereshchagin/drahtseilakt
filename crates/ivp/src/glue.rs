// Port of the glue of OpenBallance src/phys/phys.c: the physics API the game calls beyond bodies, joints and the
// PSI itself. SetPhysicsForce's controller, the body's destruction, the game's listeners of impacts and contacts,
// the body-local velocity, and the readouts of the debugging overlay. Bodies are referred to by their core index;
// entities are the game's ids, kept with the body for the readouts.

use crate::collision::ObjKind;
use crate::core::{Controller, World, MT_MOVING};
use crate::impact::ImpactEvent;
use crate::math;

type V3 = [f64; 3];
type P3 = [f32; 3];

/// SetPhysicsForce's controller record (`PhysForce`): the body, the point in the body's core space and the world force.
#[derive(Clone, Debug)]
pub struct Force {
    /// The body the force acts on; None once the force is destroyed (its slot stays in the controller arena).
    pub body: Option<usize>,
    pub point_cs: V3,
    pub force_ws: P3,
}

/// The game's listeners (`phys_set_listeners`). The impact event is the post-collision event of an impact; a listener
/// may change its normal in place, and the change stays with the contact. The contact events come as a friction
/// contact is created (`begin`) and as it is deleted.
pub trait Listener {
    fn impact(&mut self, ev: &mut ImpactEvent);
    fn contact(&mut self, a: usize, b: usize, begin: bool);
}

/// The world's listener slot; empty until the game sets one.
#[derive(Default)]
pub struct Listeners(pub Option<Box<dyn Listener>>);

impl std::fmt::Debug for Listeners {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Listeners({})", if self.0.is_some() { "set" } else { "empty" })
    }
}

/// The body count, the awake count and the contact count (`phys_debug`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub bodies: u32,
    pub awake: u32,
    pub contacts: u32,
}

/// One body's readout (`phys_debug_body`): its entity, the object origin at the current time, whether it is awake and
/// fixed, its collision shape and the number of input faces of that shape (its ledges; 0 for a sphere). The upstream
/// counts its input faces the same way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyReadout {
    pub entity: u32,
    pub pos: V3,
    pub awake: bool,
    pub fixed: bool,
    pub shape: ObjKind,
    pub triangles: u32,
}

/// One core's readout (`phys_debug_core`): the mass centre's speed in world axes, the spin in core axes, and the
/// movement state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoreReadout {
    pub speed: P3,
    pub rot_speed: P3,
    pub movement_state: i8,
}

/// One contact point's readout (`phys_debug_contact`): the entities of its two bodies, their feature types, the gap,
/// the normal (from a toward b) and the point in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactReadout {
    pub entities: [u32; 2],
    pub types: [i16; 2],
    pub gap: f32,
    /// The normal pressure the friction system holds on the point.
    pub pressure: f32,
    pub normal: P3,
    pub point: V3,
}

fn to_v3(v: P3) -> V3 {
    [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]
}

pub(crate) fn to_p3(v: V3) -> P3 {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

impl World {
    /// SetPhysicsForce (`phys_force_create`): a force acting on a body every PSI while it is awake, at a point. The
    /// point is in core space when `local`; otherwise it is a world point, taken into core space now. Returns the
    /// controller's index, for `force_destroy`.
    pub fn force_create(&mut self, c: usize, force_ws: P3, point: V3, local: bool) -> usize {
        let point_cs = if local {
            point
        } else {
            let m = self.core_m_world_f_core_at(c, self.env.current_time);
            let d = [point[0] - m.vv[0], point[1] - m.vv[1], point[2] - m.vv[2]];
            math::rtmul(&m.r, &d)
        };
        let ctrl = self.add_controller(Controller::Force(Force {
            body: Some(c),
            point_cs,
            force_ws,
        }));
        self.core_add_controller(c, ctrl);
        ctrl
    }

    /// Changes the force of a SetPhysicsForce, from the next PSI on: the game steers its ball this way, one force per
    /// PSI, without creating a controller every step.
    pub fn force_set(&mut self, ctrl: usize, force_ws: P3) {
        if let Some(Controller::Force(f)) = self.controllers.get_mut(ctrl) {
            f.force_ws = force_ws;
        }
    }

    /// Destroys a SetPhysicsForce (`phys_force_destroy`): it leaves its body. A destroyed force, or one whose body is
    /// gone, does nothing.
    pub fn force_destroy(&mut self, ctrl: usize) {
        let Some(Controller::Force(f)) = self.controllers.get_mut(ctrl) else {
            return;
        };
        let Some(c) = f.body.take() else {
            return;
        };
        self.core_remove_controller(c, ctrl);
    }

    /// The force's simulation (`force_simulate`): every PSI, the force is pushed on its body's core at its point.
    pub(crate) fn force_simulate(&mut self, c: usize, f: &Force) {
        let m = self.core_m_world_f_core_at(c, self.env.current_time);
        let f_cs = math::rtmul(&m.r, &to_v3(f.force_ws));
        self.async_push_core(c, to_p3(f.point_cs), to_p3(f_cs), f.force_ws);
    }

    /// Sets the game's listeners (`phys_set_listeners`); None removes them.
    pub fn set_listener(&mut self, l: Option<Box<dyn Listener>>) {
        self.listeners.0 = l;
    }

    /// Hands an impact event to the listener, if there is one. The listener is taken out while it runs, so it cannot
    /// reach the world.
    pub(crate) fn notify_impact(&mut self, ev: &mut ImpactEvent) {
        if let Some(mut l) = self.listeners.0.take() {
            l.impact(ev);
            self.listeners.0 = Some(l);
        }
    }

    /// Tells the listener that a friction contact between two bodies was created or deleted.
    pub(crate) fn notify_contact(&mut self, a: usize, b: usize, begin: bool) {
        if let Some(mut l) = self.listeners.0.take() {
            l.contact(a, b, begin);
            self.listeners.0 = Some(l);
        }
    }

    /// Destroys a body (`phys_body_destroy`): its forces and joints go, its contacts are forgotten, its collision object
    /// is disabled, and its core leaves the revive list, its controllers and its unit. The index stays reserved (the core
    /// is flagged `removed`), so the other bodies keep their indices.
    pub fn body_destroy(&mut self, c: usize) {
        if self.cores.get(c).is_none_or(|k| k.removed) {
            return;
        }
        for ctrl in 0..self.controllers.len() {
            if matches!(self.controllers[ctrl], Controller::Force(Force { body: Some(b), .. }) if b == c) {
                self.force_destroy(ctrl);
            }
        }
        for j in 0..self.joints.len() {
            let jo = &self.joints[j];
            if jo.alive && (jo.a == Some(c) || jo.b == Some(c)) {
                self.joint_destroy(j);
            }
        }
        self.object_remove_contacts(c, true);
        self.disable_collision_detection(c);
        if self.cores[c].in_revive_list {
            self.revive.retain(|&r| r != c);
            self.cores[c].in_revive_list = false;
        }
        for ctrl in self.cores[c].controllers.clone() {
            self.core_remove_controller(c, ctrl);
        }
        let su = self.cores[c].unit;
        self.units[su].cores.retain(|&k| k != c);
        if self.units[su].cores.is_empty() {
            self.unit_list_remove(su);
        } else {
            self.units[su].changed = true;
        }
        self.coll.objs[c] = Default::default();
        self.coll.hulls[c] = Default::default();
        let core = &mut self.cores[c];
        core.removed = true;
        core.movement_state = crate::core::MT_NOT_SIM;
    }

    /// The bodies in the order the C body list has them: the newest first. Removed bodies are skipped.
    fn body_indices(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.cores.len()).rev().filter(|&c| !self.cores[c].removed)
    }

    /// The body count, the awake count and the contact count (`phys_debug`).
    pub fn debug_counts(&self) -> Counts {
        let mut k = Counts::default();
        for c in self.body_indices() {
            k.bodies += 1;
            if self.cores[c].movement_state == MT_MOVING {
                k.awake += 1;
            }
        }
        k.contacts = self.coll.cps.iter().filter(|p| p.alive && p.fs.is_some()).count() as u32;
        k
    }

    /// The i-th body's readout (`phys_debug_body`), in the order of `debug_counts`.
    pub fn debug_body(&self, i: usize) -> Option<BodyReadout> {
        let c = self.body_indices().nth(i)?;
        let shape = self.coll.objs.get(c).and_then(|o| o.kind).unwrap_or(ObjKind::Ball);
        let triangles = self
            .coll
            .objs
            .get(c)
            .and_then(|o| o.surface.as_ref())
            .map_or(0, |s| s.ledges.len() as u32);
        Some(BodyReadout {
            entity: self.cores[c].entity,
            pos: self.body_pose(c).0,
            awake: self.cores[c].movement_state == MT_MOVING,
            fixed: self.cores[c].unmovable,
            shape,
            triangles,
        })
    }

    /// The i-th body's core readout (`phys_debug_core`), in the order of `debug_counts`.
    pub fn debug_core(&self, i: usize) -> Option<CoreReadout> {
        let c = self.body_indices().nth(i)?;
        let core = &self.cores[c];
        Some(CoreReadout {
            speed: core.speed,
            rot_speed: core.rot_speed,
            movement_state: core.movement_state,
        })
    }

    /// The i-th contact point's readout (`phys_debug_contact`), over the live points that belong to a friction system.
    pub fn debug_contact(&self, i: usize) -> Option<ContactReadout> {
        let p = self
            .coll
            .cps
            .iter()
            .filter(|p| p.alive && p.fs.is_some())
            .nth(i)?;
        Some(ContactReadout {
            entities: [self.cores[p.syn[0].obj].entity, self.cores[p.syn[1].obj].entity],
            types: [p.syn[0].typ, p.syn[1].typ],
            gap: p.gap,
            pressure: p.pressure,
            normal: p.info.n,
            point: p.info.cp_ws,
        })
    }

    /// The post-collision events since the last call (`PhysImpactEvent`, what PhysicsCollDetection listens to): the
    /// two objects and their relative velocity at the impact. Taking them empties the list.
    pub fn take_impacts(&mut self) -> Vec<([usize; 2], P3)> {
        self.coll.impact_events.drain(..).map(|e| (e.obj, e.rel)).collect()
    }

    /// The objects an object has friction contacts with now (one entry per contact point), as PhysicsContinuousContact
    /// counts them.
    pub fn friction_partners(&self, obj: usize) -> Vec<usize> {
        self.coll
            .cps
            .iter()
            .filter(|p| p.alive && p.fs.is_some())
            .filter_map(|p| match (p.syn[0].obj, p.syn[1].obj) {
                (a, b) if a == obj => Some(b),
                (a, b) if b == obj => Some(a),
                _ => None,
            })
            .collect()
    }

    /// The contact points of one body, in one pass over the points (`debug_contact` for each would be quadratic).
    pub fn contacts_of(&self, entity: u32) -> Vec<ContactReadout> {
        self.coll
            .cps
            .iter()
            .filter(|p| p.alive && p.fs.is_some())
            .filter_map(|p| {
                let entities = [self.cores[p.syn[0].obj].entity, self.cores[p.syn[1].obj].entity];
                (entities[0] == entity || entities[1] == entity).then(|| ContactReadout {
                    entities,
                    types: [p.syn[0].typ, p.syn[1].typ],
                    gap: p.gap,
            pressure: p.pressure,
                    normal: p.info.n,
                    point: p.info.cp_ws,
                })
            })
            .collect()
    }

    /// The triangles of a body's surface that have a corner within `r` of `p` (for diagnosing contacts; the upstream
    /// has no such readout).
    pub fn surface_triangles_near(&self, obj: usize, p: V3, r: f64) -> Vec<[V3; 3]> {
        let Some(surface) = self.coll.objs.get(obj).and_then(|o| o.surface.as_ref()) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for l in &surface.ledges {
            for t in 0..l.tris.len() {
                let e = crate::ledge::Edge { tri: t, slot: 0 };
                let corners = [l.p(e), l.p(crate::ledge::Ledge::next(e)), l.p(crate::ledge::Ledge::prev(e))];
                let c: Vec<V3> = corners.iter().map(|v| to_v3(*v)).collect();
                let near = c.iter().any(|v| {
                    let d = [v[0] - p[0], v[1] - p[1], v[2] - p[2]];
                    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= r * r
                });
                if near {
                    out.push([c[0], c[1], c[2]]);
                }
            }
        }
        out
    }

    /// The collision counters (`phys_debug_collision`): live mindists, mindists created, OV rechecks, watcher checks,
    /// impacts, and the tree split guards of the compiled surfaces (the upstream counts them globally).
    pub fn debug_collision(&self) -> [u32; 6] {
        let guards: u32 = self
            .coll
            .objs
            .iter()
            .filter_map(|o| o.surface.as_ref())
            .map(|s| s.split_guards)
            .sum();
        [
            self.coll.md_created - self.coll.md_deleted,
            self.coll.md_created,
            self.coll.ov_rechecks,
            self.coll.watcher_checks,
            self.coll.impacts,
            guards,
        ]
    }

    /// The simulated time in seconds (`phys_time`).
    pub fn time(&self) -> f64 {
        self.env.current_time
    }

    /// The velocity of a world point fixed to a body, at the pose of the last PSI (`phys_body_velocity_at`).
    pub fn body_velocity_at(&self, c: usize, p_ws: V3) -> P3 {
        let core = &self.cores[c];
        let r = [
            p_ws[0] - core.m_world_f_core.vv[0],
            p_ws[1] - core.m_world_f_core.vv[1],
            p_ws[2] - core.m_world_f_core.vv[2],
        ];
        let pc = math::rtmul(&core.m_world_f_core.r, &r);
        self.surface_speed(c, &to_p3(pc))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::body::{BodyDesc, Shape};
    use crate::ledge::Surface;
    use std::cell::RefCell;
    use std::rc::Rc;

    const IDENTITY: crate::math::Quat = crate::math::Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// A floor of two triangles at height 0 spanning [-10, 10] in x and z.
    fn floor() -> Surface {
        let p = [[-10.0, 0.0, -10.0], [10.0, 0.0, -10.0], [10.0, 0.0, 10.0], [-10.0, 0.0, 10.0]];
        let mut ledges = Vec::new();
        for tri in [[0, 1, 2], [0, 2, 3]] {
            ledges.push(crate::ledge::triangle(p[tri[0]], p[tri[1]], p[tri[2]]).expect("a triangle"));
        }
        Surface::compile(ledges).expect("a floor")
    }

    fn ball(w: &mut World, entity: u32, pos: [f64; 3]) -> usize {
        let desc = BodyDesc { entity, ..BodyDesc::default() };
        w.add_body(desc, Shape::Sphere(1.0), pos, IDENTITY)
    }

    fn weightless() -> World {
        let mut w = World::new();
        w.time_start();
        w.set_gravity([0.0; 3]);
        w
    }

    fn speed(w: &World, c: usize) -> f32 {
        let v = w.body_state(c).vel;
        (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
    }

    #[test]
    fn a_force_accelerates_its_body_until_it_is_destroyed() {
        let mut w = weightless();
        let b = ball(&mut w, 1, [0.0, 5.0, 0.0]);
        let f = w.force_create(b, [2.0, 0.0, 0.0], [0.0, 5.0, 0.0], false);
        w.advance(1.0);
        // the upstream pushes the force as an impulse at every PSI, so a second of 66 PSIs gives 66 F / m
        let v = speed(&w, b);
        let psis = 1.0 / w.env.delta_psi;
        assert!((f64::from(v) - 2.0 * psis).abs() < 0.02 * psis, "66 pushes of 2, got {v}");
        w.force_destroy(f);
        w.advance(1.0);
        assert!((speed(&w, b) - v).abs() < 1e-3, "a destroyed force pushes no more");
        w.force_destroy(f);
    }

    #[test]
    fn a_local_force_point_spins_the_body_as_a_world_one_does() {
        let mut world = weightless();
        let a = ball(&mut world, 1, [0.0, 0.0, 0.0]);
        world.force_create(a, [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], false);
        world.advance(0.5);
        let mut local = weightless();
        let b = ball(&mut local, 1, [0.0, 0.0, 0.0]);
        local.force_create(b, [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], true);
        local.advance(0.5);
        let (sa, sb) = (world.body_state(a).spin, local.body_state(b).spin);
        for k in 0..3 {
            assert!((sa[k] - sb[k]).abs() < 1e-3, "spin {k}: world {} local {}", sa[k], sb[k]);
        }
        assert!(sa[1].abs() > 0.1, "the force at a lever arm spins the ball: {sa:?}");
    }

    #[test]
    fn a_local_impulse_at_the_origin_is_a_speed_change_of_its_mass() {
        let mut w = weightless();
        let b = ball(&mut w, 1, [0.0, 0.0, 0.0]);
        w.body_impulse_local(b, [0.0; 3], [3.0, 0.0, 0.0]);
        w.advance(1.0 / 66.0);
        let v = w.body_state(b).vel;
        assert!((v[0] - 3.0).abs() < 1e-3, "an impulse of 3 on mass 1: {v:?}");
    }

    #[test]
    fn a_destroyed_body_leaves_the_counts_and_its_index_stays_reserved() {
        let mut w = weightless();
        let a = ball(&mut w, 7, [0.0, 0.0, 0.0]);
        let b = ball(&mut w, 9, [4.0, 0.0, 0.0]);
        let f = w.force_create(a, [1.0, 0.0, 0.0], [0.0; 3], true);
        assert_eq!(w.debug_counts().bodies, 2);
        w.body_destroy(a);
        w.body_destroy(a);
        assert_eq!(w.debug_counts().bodies, 1);
        assert_eq!(w.debug_body(0).map(|r| r.entity), Some(9));
        assert!(w.debug_body(1).is_none());
        w.advance(0.5);
        assert!(speed(&w, b) == 0.0, "the other body is untouched");
        w.force_destroy(f);
    }

    #[test]
    fn a_ball_dropped_on_a_floor_reports_its_contacts_and_impacts() {
        #[derive(Default)]
        struct Log {
            impacts: Vec<[usize; 2]>,
            begins: u32,
            ends: u32,
        }
        struct Rec(Rc<RefCell<Log>>);
        impl Listener for Rec {
            fn impact(&mut self, ev: &mut ImpactEvent) {
                self.0.borrow_mut().impacts.push(ev.order);
            }
            fn contact(&mut self, _a: usize, _b: usize, begin: bool) {
                let mut l = self.0.borrow_mut();
                if begin {
                    l.begins += 1;
                } else {
                    l.ends += 1;
                }
            }
        }
        let log = Rc::new(RefCell::new(Log::default()));
        let mut w = World::new();
        w.time_start();
        w.set_gravity([0.0, -9.8, 0.0]);
        w.set_listener(Some(Box::new(Rec(log.clone()))));
        let f = w.add_body(BodyDesc { fixed: true, ..BodyDesc::default() }, Shape::Surface(floor()), [0.0; 3], IDENTITY);
        let b = ball(&mut w, 3, [0.0, 2.0, 0.0]);
        w.advance(2.0);
        let log = log.borrow();
        assert!(log.begins >= 1, "a contact begins when the ball lands");
        assert!(!log.impacts.is_empty(), "the landing is an impact");
        for o in &log.impacts {
            assert!(o.contains(&f) && o.contains(&b), "the impact's order is the two bodies: {o:?}");
        }
        assert_eq!(w.coll.impact_events.len(), log.impacts.len());
        assert!(w.debug_counts().contacts >= 1, "the ball rests on the floor");
        let c = w.debug_contact(0).expect("a readout for the contact");
        assert!(c.entities.contains(&3), "the readout names the ball's entity: {c:?}");
    }

    #[test]
    fn the_frame_runs_the_smoothed_frame_time_scaled_by_the_factor() {
        let mut w = weightless();
        ball(&mut w, 1, [0.0; 3]);
        w.set_time_factor(0.5);
        let t0 = w.time();
        let reached = w.frame(16.0);
        // the smoothed frame time of the first frame is a quarter of its own: 4 ms, scaled by the factor 0.5 in
        // thousandths of a second per millisecond
        assert!((reached - (t0 + f64::from(4.0_f32 * 0.5 * 0.001))).abs() < 1e-9, "reached {reached}");
        assert!((w.time() - reached).abs() < 1e-9, "the simulation ends at the time reached");
    }

    #[test]
    fn the_readouts_count_the_awake_bodies_and_their_shapes() {
        let mut w = weightless();
        ball(&mut w, 5, [0.0; 3]);
        w.add_body(BodyDesc { fixed: true, ..BodyDesc::default() }, Shape::Surface(floor()), [0.0; 3], IDENTITY);
        let counts = w.debug_counts();
        assert_eq!(counts.bodies, 2);
        assert_eq!(counts.awake, 1, "the fixed floor never wakes");
        let readout = w.debug_body(1).expect("the ball is the newest body's neighbour");
        assert_eq!(readout.entity, 5);
        let floor_readout = w.debug_body(0);
        assert!(floor_readout.is_some_and(|r| r.fixed && r.triangles == 2), "floor readout: {floor_readout:?}");
        assert_eq!(w.debug_core(1).map(|c| c.speed), Some([0.0; 3]));
        assert!(w.debug_collision()[1] >= w.debug_collision()[0]);
    }
}
