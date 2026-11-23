// Port of OpenBallance src/phys/phys_mindist.c (GPL-3.0; see the LICENSE of the repository): the mindist manager
// (docs/ivp_collision.md 4). A mindist is created for a pair of objects, kept EXACT (its closest features are solved and
// its event is on the time list), INVALID (its features failed to solve), or HULL (it waits on the objects' hull
// listeners while the gap is large). The per-PSI passes re-solve the EXACT ones and the critical ones take their events.
//
// The mindists live in an arena (`CollWorld::mds`), their lists are index links, and the synapses are referred to by
// `syn_ref`. The object cache and the poses come from the core (`Core::object_pose_at`). The contact system hooks in
// through `contact_inter_penetration` (a recalc that failed) and `contact_do_impact` (a mindist whose gap closed).

use crate::collision::{CollKind, ObjKind};
use crate::core::{Core, World};
use crate::event::{
    self, COLL_DIST, CoreInfo, EventSide, MIN_FRICTION_DIST, Motion, REAL_COLL_DIST,
};
use crate::hull::{ListenerKind, encode_listener};
use crate::ledge::{Edge, NodeKind, Surface};
use crate::math::Matrix;
use crate::mindist::{
    Feat, MD_EXACT, MD_HULL, MD_HULL_RECURSIVE, MD_INVALID, MDF_COLL_DIST, MDF_COLL_DIST_SHIFT,
    MDF_COLL_TYPE, MDF_FUNCTION, MDF_RECALC, MDF_STATUS, Mindist, ObjCache, ST_BALL, ST_POINT,
    Synapse, syn_ref,
};
use crate::minimize::{Side, Solver};

/// The threshold speed the upstream calls IVP_SPEED_AFTER_KEEPER (phys_coll.h).
const SPEED_AFTER_KEEPER: f32 = 0.50503469;

/// The poses of the cores at a time, for the event solver.
struct CorePoses<'a> {
    cores: &'a [Core],
}

impl Motion for CorePoses<'_> {
    fn matrix_at(&self, obj: usize, t: f64) -> Matrix {
        match self.cores.get(obj) {
            Some(c) => c.object_pose_at(t),
            None => Matrix::identity(),
        }
    }
}

/// The core position of an object at a time (`ivp_core_pos_at`, 0x10012470).
pub(crate) fn core_pos_at(c: &Core, t: f64) -> [f64; 3] {
    let dt = t - c.time_of_last_psi;
    let mut out = [0.0; 3];
    for k in 0..3 {
        out[k] = c.pos_last[k] + f64::from(c.delta_pos[k]) * dt;
    }
    out
}

fn core_info(c: &Core) -> CoreInfo {
    CoreInfo {
        abs_omega: f64::from(c.abs_omega),
        current_speed: f64::from(c.current_speed),
        max_surface_rot_speed: f64::from(c.max_surface_rot_speed),
        upper_limit_radius: f64::from(c.upper_limit_radius),
        inv_object_diameter: f64::from(c.inv_object_diameter),
    }
}

/// Pushes synapse `s` at the head of a synapse list (`syn_push`).
fn syn_push(mds: &mut [Mindist], head: &mut Option<usize>, s: usize) {
    mds[s / 2].syn[s % 2].prev = None;
    mds[s / 2].syn[s % 2].next = *head;
    if let Some(h) = *head {
        mds[h / 2].syn[h % 2].prev = Some(s);
    }
    *head = Some(s);
}

/// Takes synapse `s` out of a synapse list (`syn_unlink`).
fn syn_unlink(mds: &mut [Mindist], head: &mut Option<usize>, s: usize) {
    let (prev, next) = (mds[s / 2].syn[s % 2].prev, mds[s / 2].syn[s % 2].next);
    match prev {
        Some(p) => mds[p / 2].syn[p % 2].next = next,
        None => {
            if *head == Some(s) {
                *head = next;
            }
        }
    }
    if let Some(n) = next {
        mds[n / 2].syn[n % 2].prev = prev;
    }
    mds[s / 2].syn[s % 2].next = None;
    mds[s / 2].syn[s % 2].prev = None;
}

/// Pushes mindist `m` at the head of the manager's list (`md_push`).
fn md_push(mds: &mut [Mindist], head: &mut Option<usize>, m: usize) {
    mds[m].prev = None;
    mds[m].next = *head;
    if let Some(h) = *head {
        mds[h].prev = Some(m);
    }
    *head = Some(m);
}

/// Takes mindist `m` out of the manager's list (`md_unlink`).
fn md_unlink(mds: &mut [Mindist], head: &mut Option<usize>, m: usize) {
    let (prev, next) = (mds[m].prev, mds[m].next);
    match prev {
        Some(p) => mds[p].next = next,
        None => {
            if *head == Some(m) {
                *head = next;
            }
        }
    }
    if let Some(n) = next {
        mds[n].prev = prev;
    }
    mds[m].next = None;
    mds[m].prev = None;
}

/// Whether the mindist's solve is current (`recalc_ok`).
fn recalc_ok(md: &Mindist) -> bool {
    (md.flags & MDF_RECALC) == 0
}

/// The recalc loop of one mindist (the body of `recalc`, 4.5): the solver counts `count` steps, retries the backside
/// fix up to `tries` times, and clears the failure bit on success.
fn solve_loop(md: &mut Mindist, sides: [Side; 2], count: i32, tries: u32) -> i32 {
    let mut solver = Solver::new(md, sides, count);
    let mut rc = 2;
    for _ in 0..tries {
        rc = solver.step();
        if rc == 1 {
            solver.mindist().flags &= !MDF_RECALC;
            return 1;
        }
        let f = solver.mindist().flags;
        solver.mindist().flags = (f & !0x8000) | 0x4000;
        if rc == 2 {
            break;
        }
        if rc == 3 {
            solver.fix_backside();
        }
    }
    rc
}

impl World {
    /// The object's collision geometry and extra radius, set by the glue before the object is enabled.
    pub fn set_object_geometry(
        &mut self,
        obj: usize,
        kind: ObjKind,
        surface: Option<Surface>,
        extra_radius: f32,
    ) {
        self.ensure_object(obj);
        let o = &mut self.coll.objs[obj];
        o.kind = Some(kind);
        o.surface = surface;
        o.extra_radius = extra_radius;
        o.cache_valid = false;
    }

    /// The object's material, set by the glue: its friction and elasticity (the triangles carry no index of their own).
    pub fn set_object_material(&mut self, obj: usize, friction: f32, elasticity: f32) {
        self.ensure_object(obj);
        self.coll.objs[obj].friction = friction;
        self.coll.objs[obj].elasticity = elasticity;
    }

    /// Makes room for object `obj` in the collision tables.
    pub(crate) fn ensure_object(&mut self, obj: usize) {
        if self.coll.objs.len() <= obj {
            self.coll.objs.resize_with(obj + 1, Default::default);
        }
        if self.coll.hulls.len() <= obj {
            self.coll.hulls.resize_with(obj + 1, Default::default);
        }
    }

    /// The object's cache at the current time (`ivp_cache_get`, 0x1001a190): refreshed when stale, and while the
    /// object is simulated when the time code moved on.
    pub(crate) fn obj_cache(&mut self, obj: usize) -> ObjCache {
        self.ensure_object(obj);
        let now = self.env.current_time;
        let time_code = self.time_code;
        let simulated = self
            .cores
            .get(obj)
            .is_some_and(|c| i32::from(c.movement_state) < 8);
        let o = &self.coll.objs[obj];
        let stale = !o.cache_valid || (simulated && o.cache_time_code < time_code);
        if stale {
            let (r, vv, pos) = match self.cores.get(obj) {
                Some(c) => {
                    let (m, pos) = c.cache_pose_at(now);
                    (m.r, m.vv, pos)
                }
                None => (
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    [0.0; 3],
                    [0.0; 3],
                ),
            };
            let o = &mut self.coll.objs[obj];
            o.cache = ObjCache {
                r,
                vv,
                core_pos: pos,
            };
            o.cache_time_code = time_code;
            o.cache_valid = true;
        }
        self.coll.objs[obj].cache
    }

    /// The solve state of a mindist's objects: their caches, the object indices and the surfaces (polygons only).
    fn md_sides(&mut self, m: usize) -> ([usize; 2], [ObjCache; 2]) {
        let objs = [self.coll.mds[m].syn[0].obj, self.coll.mds[m].syn[1].obj];
        let caches = [self.obj_cache(objs[0]), self.obj_cache(objs[1])];
        (objs, caches)
    }

    /// Takes a mindist into the arena, from the free slots first.
    fn md_alloc(&mut self, md: Mindist) -> usize {
        self.coll.md_created += 1;
        match self.coll.free_mds.pop() {
            Some(i) => {
                self.coll.mds[i] = md;
                i
            }
            None => {
                self.coll.mds.push(md);
                self.coll.mds.len() - 1
            }
        }
    }

    /// Makes the EXACT state's links (`link_exact`, 0x10016e30; no wheel: Ballance has no car wheels).
    fn link_exact(&mut self, m: usize) {
        self.coll.mds[m].set_status(MD_EXACT);
        md_push(&mut self.coll.mds, &mut self.coll.exact, m);
        for k in 0..2 {
            let obj = self.coll.mds[m].syn[k].obj;
            syn_push(
                &mut self.coll.mds,
                &mut self.coll.objs[obj].exact_syn,
                syn_ref(m, k),
            );
        }
    }

    /// Takes a mindist out of the EXACT state, cancelling its event (`remove_exact`, 0x10018230).
    fn remove_exact(&mut self, m: usize) {
        if let Some(slot) = self.coll.mds[m].ev_slot.take() {
            self.time_remove(slot);
        }
        md_unlink(&mut self.coll.mds, &mut self.coll.exact, m);
        for k in 0..2 {
            let obj = self.coll.mds[m].syn[k].obj;
            syn_unlink(
                &mut self.coll.mds,
                &mut self.coll.objs[obj].exact_syn,
                syn_ref(m, k),
            );
        }
    }

    /// Makes the INVALID state's links (`insert_invalid`, 0x10016ef0).
    fn insert_invalid(&mut self, m: usize) {
        self.coll.mds[m].set_status(MD_INVALID);
        md_push(&mut self.coll.mds, &mut self.coll.invalid, m);
        for k in 0..2 {
            let obj = self.coll.mds[m].syn[k].obj;
            syn_push(
                &mut self.coll.mds,
                &mut self.coll.objs[obj].invalid_syn,
                syn_ref(m, k),
            );
        }
    }

    /// Takes a mindist out of the INVALID state (`remove_invalid`, 0x10018310).
    fn remove_invalid(&mut self, m: usize) {
        md_unlink(&mut self.coll.mds, &mut self.coll.invalid, m);
        for k in 0..2 {
            let obj = self.coll.mds[m].syn[k].obj;
            syn_unlink(
                &mut self.coll.mds,
                &mut self.coll.objs[obj].invalid_syn,
                syn_ref(m, k),
            );
        }
    }

    /// Takes both synapses off their objects' hull managers (`remove_hull`, 0x10018390).
    fn remove_hull(&mut self, m: usize) {
        for k in 0..2 {
            let obj = self.coll.mds[m].syn[k].obj;
            if let Some(slot) = self.coll.mds[m].syn[k].hull_slot.take() {
                self.coll.hulls[obj].remove(slot);
            }
        }
    }

    /// The HULL state: both synapses listen on their objects' hulls, with thresholds `sa` and `sb` above the current
    /// hull values (`hull_insert`, 0x100183c0).
    fn hull_insert(&mut self, m: usize, sa: f32, sb: f32) {
        self.coll.mds[m].set_status(MD_HULL);
        let now = self.env.current_time;
        let s = [sa, sb];
        let mut dt = [0.0f32; 2];
        for k in 0..2 {
            let obj = self.coll.mds[m].syn[k].obj;
            let h = &self.coll.hulls[obj];
            let dte = now - h.last_time;
            dt[k] = dte as f32;
            let key =
                (dte * f64::from(h.gradient) + f64::from(h.hull_value) + f64::from(s[k])) as f32;
            let slot = self.coll.hulls[obj].insert(
                encode_listener(ListenerKind::Synapse, syn_ref(m, k) as u32),
                key,
            );
            self.coll.mds[m].syn[k].hull_slot = Some(slot);
        }
        let oa = self.coll.mds[m].syn[0].obj;
        let ob = self.coll.mds[m].syn[1].obj;
        let ha = &self.coll.hulls[oa];
        let hb = &self.coll.hulls[ob];
        let t1 =
            (hb.gradient - hb.center_gradient) * dt[1] + (hb.hull_value - hb.center_hull_value);
        let t2 =
            (ha.gradient - ha.center_gradient) * dt[0] + (ha.hull_value - ha.center_hull_value);
        self.coll.mds[m].sum_angular_hull_time = f64::from(t1 + t2);
    }

    /// Splits the budget `dist` between the two objects by their speeds (`insert_hull`, 0x10018480): all of it to the
    /// moving side when the other is static.
    fn insert_hull(&mut self, m: usize, dist: f32) {
        let oa = self.coll.mds[m].syn[0].obj;
        let ob = self.coll.mds[m].syn[1].obj;
        let moving =
            |w: &World, o: usize| w.cores.get(o).is_some_and(|c| (c.movement_state & 7) != 0);
        if !moving(self, oa) {
            self.hull_insert(m, 0.0, dist);
        } else if !moving(self, ob) {
            self.hull_insert(m, dist, 0.0);
        } else {
            let (ca, cb) = (&self.cores[oa], &self.cores[ob]);
            let ra = f64::from(ca.max_surface_rot_speed)
                + f64::from(ca.current_speed)
                + f64::from(1e-10f32);
            let rb = f64::from(cb.max_surface_rot_speed)
                + f64::from(cb.current_speed)
                + f64::from(1e-10f32);
            let wa = (rb * f64::from(0.1f32) + ra) as f32;
            let wb = ra * f64::from(0.1f32) + rb;
            let f = (f64::from(dist) / (wb + f64::from(wa))) as f32;
            self.hull_insert(m, f * wa, (f64::from(f) * wb) as f32);
        }
    }

    /// The EXACT mindist went INVALID: out of EXACT, into INVALID (the vtable's went_invalid, 0x10016f70).
    fn went_invalid(&mut self, m: usize) {
        self.remove_exact(m);
        self.insert_invalid(m);
    }

    /// Links a new mindist as EXACT, solves it, and schedules its event, or makes it INVALID (`insert_and_recalc_exact`,
    /// 0x10016f90).
    fn insert_and_recalc_exact(&mut self, m: usize) {
        self.link_exact(m);
        self.recalc(m, 20, 2);
        if recalc_ok(&self.coll.mds[m]) {
            let a = self.coll.mds[m].syn[0].obj;
            let b = self.coll.mds[m].syn[1].obj;
            let st =
                (self.cores[a].movement_state as u8 | self.cores[b].movement_state as u8) as i8;
            self.update_events(m, i32::from(st) < 0x21, 0);
        } else {
            self.went_invalid(m);
        }
    }

    /// The feature a polygon's first pair is solved from: its single convex piece's first edge. None for a polygon
    /// with more than one piece (its pairs need the OO watcher, which is not ported) or without geometry.
    fn single_feat(&self, o: usize) -> Option<Feat> {
        let s = self.coll.objs.get(o)?.surface.as_ref()?;
        match s.tree.first()?.kind {
            NodeKind::Leaf { ledge } => Some(Feat {
                ledge,
                edge: Edge { tri: 0, slot: 0 },
            }),
            NodeKind::Inner { .. } => None,
        }
    }

    /// Creates the mindist of two objects and its collision record (`ivp_mindist_create`, 0x10016290). Returns the
    /// collision index, or None when the pair has no mindist: an object without geometry, or a polygon with more than
    /// one piece (its pairs go to a watcher).
    pub(crate) fn mindist_create(&mut self, a: usize, b: usize) -> Option<usize> {
        let ka = self.coll.objs.get(a)?.kind?;
        let kb = self.coll.objs.get(b)?.kind?;
        let feat_a = if ka == ObjKind::Polygon {
            Some(self.single_feat(a)?)
        } else {
            None
        };
        let feat_b = if kb == ObjKind::Polygon {
            Some(self.single_feat(b)?)
        } else {
            None
        };
        let m = self.mindist_create_with(None, a, b, feat_a, feat_b)?;
        let c = self.collision_new(CollKind::Mindist, [a, b], m);
        self.coll.mds[m].collision = Some(c);
        Some(c)
    }

    /// Creates a mindist between two pieces, owned by a watcher (`owner`) or by the pair's collision record (None).
    /// The features are the pieces' first edges, or None for a ball. Solves it and schedules it (the body of
    /// `ivp_mindist_create`).
    pub(crate) fn mindist_create_with(
        &mut self,
        owner: Option<usize>,
        a: usize,
        b: usize,
        feat_a: Option<Feat>,
        feat_b: Option<Feat>,
    ) -> Option<usize> {
        let ka = self.coll.objs.get(a)?.kind?;
        let kb = self.coll.objs.get(b)?.kind?;
        let mut md = Mindist {
            owner,
            child_index: -1,
            alive: true,
            flags: 0x0fc0_0000,
            // a new mindist has never been solved: no time code matches its stamp, so its first recalc runs
            recalc_time_stamp: u32::MAX,
            sum_extra_radius: self.coll.objs[a].extra_radius + self.coll.objs[b].extra_radius,
            ..Mindist::default()
        };
        let first = if kb == ObjKind::Polygon {
            md.syn[1] = Synapse {
                obj: b,
                feat: feat_b,
                status: ST_POINT,
                ..Synapse::default()
            };
            0
        } else {
            // the ball of a ball pair goes to the synapse of the object with the smaller index (the entity order)
            let ball_is_0 = ka != ObjKind::Ball || a < b;
            let (ball, first) = if ball_is_0 { (0, 1) } else { (1, 0) };
            md.syn[ball] = Synapse {
                obj: b,
                feat: None,
                status: ST_BALL,
                ..Synapse::default()
            };
            first
        };
        md.syn[first] = Synapse {
            obj: a,
            feat: feat_a,
            status: if ka == ObjKind::Polygon {
                ST_POINT
            } else {
                ST_BALL
            },
            ..Synapse::default()
        };
        let m = self.md_alloc(md);
        self.insert_and_recalc_exact(m);
        Some(m)
    }

    /// Deletes a mindist: out of its state's lists, its event cancelled, and its collision released
    /// (`ivp_mindist_delete`, 0x100162f0).
    pub(crate) fn mindist_delete(&mut self, m: usize) {
        match self.coll.mds[m].status() {
            MD_INVALID => self.remove_invalid(m),
            MD_EXACT => self.remove_exact(m),
            MD_HULL_RECURSIVE | MD_HULL => self.remove_hull(m),
            _ => {}
        }
        if let Some(slot) = self.coll.mds[m].ev_slot.take() {
            self.time_remove(slot);
        }
        if let Some(w) = self.coll.mds[m].owner {
            self.watcher_child_deleted(w, m);
        } else if let Some(c) = self.coll.mds[m].collision.take() {
            self.collision_release(c);
        }
        self.coll.mds[m] = Mindist::default();
        self.coll.free_mds.push(m);
        self.coll.md_deleted += 1;
    }

    /// The recalc (4.5, 0x10019950 with 20 countdown steps and two tries; 0x100197f0 with one try and none): solves
    /// the mindist's features, once per time code. Returns the solver's code (1 converged, 2 failed, 3 backside, 4 done).
    pub(crate) fn recalc(&mut self, m: usize, count: i32, tries: u32) -> i32 {
        if self.coll.mds[m].recalc_time_stamp == self.time_code {
            return 4;
        }
        self.coll.mds[m].recalc_time_stamp = self.time_code;
        let (objs, caches) = self.md_sides(m);
        let mut md = std::mem::take(&mut self.coll.mds[m]);
        let rc = {
            let sides = [
                Side {
                    cache: caches[0],
                    surface: self.coll.objs[objs[0]].surface.as_ref(),
                },
                Side {
                    cache: caches[1],
                    surface: self.coll.objs[objs[1]].surface.as_ref(),
                },
            ];
            solve_loop(&mut md, sides, count, tries)
        };
        self.coll.mds[m] = md;
        if rc == 1 {
            return 1;
        }
        let flags = self.coll.mds[m].flags;
        if (flags & MDF_FUNCTION) != 0x1000 && (flags & MDF_STATUS) != 0x100000 {
            self.contact_inter_penetration(m);
        }
        rc
    }

    /// Schedules the mindist's next event (`update_mindist_events`, 4.6 / 5.2, 0x10017870): its gap closes below the
    /// hull budget, or the event solver finds a time; `allow_hull` lets a mindist go to the HULL state, and `mode`
    /// picks the rule for an event at once (0: now, 1 and 2: a little later).
    pub(crate) fn update_events(&mut self, m: usize, allow_hull: bool, mode: i32) {
        let (objs, caches) = self.md_sides(m);
        let md = self.coll.mds[m].clone();
        let a = self.cores[md.syn[md.sorted(0)].obj].clone();
        let b = self.cores[md.syn[md.sorted(1)].obj].clone();
        let rot_sum = f64::from(a.max_surface_rot_speed + b.max_surface_rot_speed);
        let worst_total = f64::from(b.current_speed + a.current_speed) + rot_sum;
        if let Some(slot) = self.coll.mds[m].ev_slot.take() {
            self.time_remove(slot);
        }
        let cd = f64::from(COLL_DIST);
        let len = f64::from(md.len_numerator);
        let delta = self.env.delta_psi;
        if delta * worst_total * 2.099_999_904_632_568_4 + cd < len {
            if !allow_hull {
                return;
            }
            self.remove_exact(m);
            self.insert_hull(m, (len - cd) as f32);
            return;
        }
        let n = md.normal;
        let proj_v = (f64::from(n[2]) * f64::from(b.speed[2])
            + f64::from(n[1]) * f64::from(b.speed[1])
            + f64::from(n[0]) * f64::from(b.speed[0]))
            - (f64::from(n[2]) * f64::from(a.speed[2])
                + f64::from(n[1]) * f64::from(a.speed[1])
                + f64::from(n[0]) * f64::from(a.speed[0]));
        let da = f64::from(a.rotation_axis_world[0]) * f64::from(n[0])
            + f64::from(a.rotation_axis_world[1]) * f64::from(n[1])
            + f64::from(a.rotation_axis_world[2]) * f64::from(n[2]);
        let db = f64::from(b.rotation_axis_world[0]) * f64::from(n[0])
            + f64::from(b.rotation_axis_world[1]) * f64::from(n[1])
            + f64::from(b.rotation_axis_world[2]) * f64::from(n[2]);
        let worst_approach = (1.001_000_046_730_041_1 - da * da).sqrt()
            * f64::from(a.max_surface_rot_speed)
            + (1.001_000_046_730_041_1 - db * db).sqrt() * f64::from(b.max_surface_rot_speed)
            + proj_v;
        let t_now = self.env.current_time;
        let t_max = self.env.time_of_next_psi;
        if worst_approach < f64::from(SPEED_AFTER_KEEPER) && worst_approach < 1e-19 {
            return;
        }
        if (t_max - t_now) * worst_approach + cd <= len {
            return;
        }
        if (md.flags & MDF_FUNCTION) == 0x1000 {
            return;
        }
        // the collision-distance index decays by one every few updates (no numeric effect: every distance is 0.01)
        let mut idx = (md.flags & MDF_COLL_DIST) >> MDF_COLL_DIST_SHIFT;
        if idx != 0 {
            let counter = self.coll.coll_dist_counter;
            self.coll.coll_dist_counter = counter.wrapping_add(1);
            if counter > 2 {
                idx -= 1;
                self.coll.mds[m].flags =
                    (self.coll.mds[m].flags & !MDF_COLL_DIST) | (idx << MDF_COLL_DIST_SHIFT);
                self.coll.coll_dist_counter = 0;
            }
        }
        let sides = [
            EventSide {
                obj: objs[0],
                cache: caches[0],
                core: core_info(&self.cores[objs[0]]),
                simulated: i32::from(self.cores[objs[0]].movement_state) < 8,
                surface: self.coll.objs[objs[0]].surface.as_ref(),
            },
            EventSide {
                obj: objs[1],
                cache: caches[1],
                core: core_info(&self.cores[objs[1]]),
                simulated: i32::from(self.cores[objs[1]].movement_state) < 8,
                surface: self.coll.objs[objs[1]].surface.as_ref(),
            },
        ];
        let poses = CorePoses { cores: &self.cores };
        let ev = event::solve(
            &md,
            sides,
            &poses,
            t_now,
            t_max,
            worst_approach,
            worst_total,
        );
        if ev.code == 0 {
            return;
        }
        let mut t = ev.time;
        if t - t_now < 9.999_999_974_752_427e-7 {
            if mode == 0 {
                t = t_now;
            } else {
                let slack = len - f64::from(REAL_COLL_DIST);
                if mode == 2 || (ev.code & 0xf) != 0 {
                    t = if slack < 1e-12 {
                        t_now + delta * f64::from(0.001f32)
                    } else {
                        slack / worst_total + t_now + delta * f64::from(1e-4f32)
                    };
                } else {
                    t = if slack < 1e-12 {
                        t_now + delta * f64::from(1e-5f32)
                    } else {
                        slack * 0.1 / worst_total + t_now + delta * f64::from(1e-7f32)
                    };
                }
                if t - self.env.time_of_next_psi >= 0.0 {
                    return;
                }
            }
        }
        let slot = self.time_insert(m as u32, t);
        self.coll.mds[m].ev_slot = Some(slot);
        self.coll.mds[m].flags = (self.coll.mds[m].flags & !MDF_COLL_TYPE) | (ev.code & 0xff);
    }

    /// A mindist's event fired (`ivp_mindist_simulate_event`, 0x100181b0): the solve at the event time, then the
    /// impact or the next prediction.
    pub(crate) fn mindist_simulate_event(&mut self, payload: u32) {
        let m = payload as usize;
        if m >= self.coll.mds.len() || !self.coll.mds[m].alive {
            return;
        }
        self.coll.mds[m].ev_slot = None;
        self.recalc(m, 20, 2);
        if !recalc_ok(&self.coll.mds[m]) {
            return;
        }
        if self.coll.mds[m].flags & 0xf != 0 {
            self.update_events(m, false, 1);
        } else if self.coll.mds[m].len_numerator < COLL_DIST + MIN_FRICTION_DIST {
            self.contact_do_impact(m);
        } else {
            self.update_events(m, false, 2);
        }
    }

    /// The angular hull value of an object now: (h - ch) + (g - cg)(t - t_h) (phys_mindist.c, `angular_hull`).
    fn angular_hull(&self, obj: usize, t: f64) -> f64 {
        let h = &self.coll.hulls[obj];
        f64::from(h.hull_value - h.center_hull_value)
            + (t - h.last_time) * f64::from(h.gradient - h.center_gradient)
    }

    /// A synapse of a HULL mindist fired (`ivp_mindist_hull_exceeded`, 0x10017d70): the halfspace re-estimate re-arms
    /// the synapses while the gap still exceeds six PSIs of worst-case approach; otherwise the mindist goes back to
    /// EXACT.
    pub(crate) fn mindist_hull_exceeded(&mut self, m: usize, intrusion: f32) {
        let sorted = [self.coll.mds[m].sorted(0), self.coll.mds[m].sorted(1)];
        let oa = self.coll.mds[m].syn[sorted[0]].obj;
        let ob = self.coll.mds[m].syn[sorted[1]].obj;
        let now = self.env.current_time;
        let pa = core_pos_at(&self.cores[oa], now);
        let pb = core_pos_at(&self.cores[ob], now);
        let d = [pa[0] - pb[0], pa[1] - pb[1], pa[2] - pb[2]];
        let (ca, cb) = (&self.cores[oa], &self.cores[ob]);
        let ra = f64::from(ca.max_surface_rot_speed) + f64::from(ca.current_speed) + 1e-19;
        let rb = f64::from(cb.max_surface_rot_speed) + f64::from(cb.current_speed) + 1e-19;
        let sum = ra + rb;
        let md = &self.coll.mds[m];
        if (md.flags & 0x30000) == 0 {
            let n = md.normal;
            let proj = f64::from(n[1]) * d[1] + f64::from(n[2]) * d[2] + f64::from(n[0]) * d[0];
            let ang = self.angular_hull(oa, now) + self.angular_hull(ob, now);
            let nw = f64::from(md.len_numerator)
                - (ang - md.sum_angular_hull_time)
                - (f64::from(md.contact_dot_diff_center) - proj)
                + f64::from(intrusion);
            if nw > self.env.delta_psi * sum * 6.0 {
                let ratio = nw / sum;
                let (slot_a, slot_b) = (
                    self.coll.mds[m].syn[sorted[0]].hull_slot,
                    self.coll.mds[m].syn[sorted[1]].hull_slot,
                );
                let md = &mut self.coll.mds[m];
                md.sum_angular_hull_time = ang;
                md.contact_dot_diff_center = proj as f32;
                md.len_numerator = (nw - f64::from(intrusion)) as f32;
                if let Some(s) = slot_a {
                    self.coll.hulls[oa].remove(s);
                }
                let va = self.coll.hulls[oa].value_at(now);
                let slot = self.coll.hulls[oa].insert(
                    encode_listener(ListenerKind::Synapse, syn_ref(m, sorted[0]) as u32),
                    (f64::from(va) + ratio * ra) as f32,
                );
                self.coll.mds[m].syn[sorted[0]].hull_slot = Some(slot);
                if let Some(s) = slot_b {
                    self.coll.hulls[ob].remove(s);
                }
                let vb = self.coll.hulls[ob].value_at(now);
                let slot = self.coll.hulls[ob].insert(
                    encode_listener(ListenerKind::Synapse, syn_ref(m, sorted[1]) as u32),
                    (f64::from(vb) + ratio * rb) as f32,
                );
                self.coll.mds[m].syn[sorted[1]].hull_slot = Some(slot);
                return;
            }
        }
        self.remove_hull(m);
        self.insert_and_recalc_exact(m);
    }

    /// The core of an object integrated: its hull grows with the core's speeds, and the object is queued for the
    /// hull phase when its first listener lies below the next PSI (`phys_collision_core_integrated`, 0x1001e750).
    pub(crate) fn hull_update_core(&mut self, obj: usize, dt: f64) {
        self.ensure_object(obj);
        let c = &self.cores[obj];
        let speed = c.max_surface_rot_speed + c.current_speed;
        let center = c.current_speed;
        let now = self.env.current_time;
        let h = &mut self.coll.hulls[obj];
        h.update(now, dt as f32, speed, center);
        if h.list.min_value() - h.hull_value_next_psi < 0.0 {
            self.coll.check.push(obj);
        }
    }

    /// The hull phase (`ivp_hull_phase`, 0x1001eb10): the queued hulls, last queued first, fire their listeners whose
    /// threshold the next PSI reaches (at most 101 per hull), and rebase their values every ten seconds.
    pub(crate) fn hull_phase(&mut self) {
        while let Some(obj) = self.coll.check.pop() {
            let mut k = 100i32;
            loop {
                let h = &self.coll.hulls[obj];
                let d = h.list.min_value() - h.hull_value_next_psi;
                if !(d < 0.0) {
                    break;
                }
                let Some((_, payload)) = h.list.first() else {
                    break;
                };
                self.listener_exceeded(obj, payload, d);
                let stop = k < 0;
                k -= 1;
                if stop {
                    break;
                }
            }
            let h = &self.coll.hulls[obj];
            if h.reset_time < h.last_time {
                self.hull_reset(obj);
                let h = &mut self.coll.hulls[obj];
                h.reset_time = (h.last_time + 10.0).trunc();
            }
        }
    }

    /// A listener's threshold was reached (`listener_exceeded`): a mindist's synapse, an OV element, or a watcher.
    fn listener_exceeded(&mut self, obj: usize, payload: u32, intrusion: f32) {
        let l = crate::hull::decode_listener(payload);
        match l.kind {
            ListenerKind::Synapse => {
                let m = (l.index / 2) as usize;
                let _ = obj;
                self.mindist_hull_exceeded(m, intrusion);
            }
            ListenerKind::Ov => self.recheck_ov_element(l.index as usize),
            ListenerKind::Watcher => self.watcher_check(l.index as usize),
        }
    }

    /// Rebases an object's hull to zero (`ivp_hull_reset`, 0x1001a820): the synapses' angular sums move with it
    /// (`ivp_mindist_hull_reset`, 0x10017d50).
    pub(crate) fn hull_reset(&mut self, obj: usize) {
        let h = &self.coll.hulls[obj];
        let dh = -h.hull_value;
        let dc = -h.center_hull_value;
        let touched = self.coll.hulls[obj].reset();
        for payload in touched {
            let l = crate::hull::decode_listener(payload);
            if l.kind == ListenerKind::Synapse {
                let m = (l.index / 2) as usize;
                let sum = self.coll.mds[m].sum_angular_hull_time as f32;
                self.coll.mds[m].sum_angular_hull_time = f64::from((dh - dc) + sum);
            }
        }
    }

    /// Short pass (`0x10017850`): every EXACT mindist re-solves; the ones that fail go INVALID.
    pub fn short_pass(&mut self) {
        let mut cur = self.coll.exact;
        while let Some(m) = cur {
            cur = self.coll.mds[m].next;
            self.recalc(m, 20, 2);
            if !recalc_ok(&self.coll.mds[m]) {
                self.went_invalid(m);
            }
        }
    }

    /// Critic pass (`0x10017770`): every EXACT mindist updates its event, allowing the hull.
    pub fn critic_pass(&mut self) {
        let mut cur = self.coll.exact;
        while let Some(m) = cur {
            cur = self.coll.mds[m].next;
            self.update_events(m, true, 1);
        }
    }

    /// After the controllers: an INVALID mindist whose solve succeeds goes back to EXACT, unscheduled
    /// (`0x100099a0`).
    pub(crate) fn object_after_controllers(&mut self, obj: usize) {
        let Some(mut cur) = self.coll.objs.get(obj).and_then(|o| o.invalid_syn) else {
            return;
        };
        loop {
            let next = self.coll.mds[cur / 2].syn[cur % 2].next;
            let m = cur / 2;
            if self.coll.mds[m].status() == MD_INVALID {
                self.recalc(m, 0, 1);
                if recalc_ok(&self.coll.mds[m]) {
                    // both synapses of the mindist leave the list at once: skip the other one
                    let next = match next {
                        Some(n) if n / 2 == m => self.coll.mds[m].syn[n % 2].next,
                        other => other,
                    };
                    self.remove_invalid(m);
                    self.link_exact(m);
                    match next {
                        Some(n) => cur = n,
                        None => return,
                    }
                    continue;
                }
            }
            match next {
                Some(n) => cur = n,
                None => return,
            }
        }
    }

    /// The recovery of a penetrating pair after a failed solve (the contact system's, `crate::impact`).
    fn contact_inter_penetration(&mut self, m: usize) {
        self.inter_penetration(m);
    }

    /// The impact of a mindist whose gap closed (the contact system's, `crate::impact`).
    fn contact_do_impact(&mut self, m: usize) {
        self.do_impact_mindist(m);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::convex;
    use crate::core::World;
    use crate::ledge::Surface;
    use crate::math::Quat;
    use crate::mindist::MD_HULL;

    fn unit_cube() -> Surface {
        let mut p = Vec::new();
        for &x in &[0.0, 1.0] {
            for &y in &[0.0, 1.0] {
                for &z in &[0.0, 1.0] {
                    p.push([x, y, z]);
                }
            }
        }
        Surface::compile(vec![convex::hull(&p).expect("a cube has a hull")]).expect("a surface")
    }

    fn two_cubes() -> World {
        let mut w = World::new();
        let q = Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        };
        for (i, x) in [0.0, 2.0].into_iter().enumerate() {
            let c = w.core_init(i, q, [x, 0.0, 0.0], false);
            w.cores[c].upper_limit_radius = 0.866;
            // a new core is not simulated until it is revived; the pair rule needs one moving object
            w.cores[c].movement_state = crate::core::MT_MOVING;
            w.set_object_geometry(c, ObjKind::Polygon, Some(unit_cube()), 0.0);
        }
        w
    }

    fn live_mindists(w: &World) -> Vec<usize> {
        (0..w.coll.mds.len())
            .filter(|&m| w.coll.mds[m].alive)
            .collect()
    }

    #[test]
    fn two_cubes_get_a_mindist_at_their_gap() {
        let mut w = two_cubes();
        w.enable_collision_detection(0);
        assert!(live_mindists(&w).is_empty(), "one object alone has no pair");
        w.enable_collision_detection(1);
        let live = live_mindists(&w);
        assert_eq!(live.len(), 1, "one mindist for the pair");
        let md = &w.coll.mds[live[0]];
        assert!(
            (md.len_numerator - 1.0).abs() < 1e-3,
            "gap {}",
            md.len_numerator
        );
        // a gap of 1 is beyond what one PSI can close at the objects' speeds, so the budget goes to the hulls
        assert_eq!(md.status(), MD_HULL, "the pair waits on the hull listeners");
        // each hull holds its object's OV element and the pair's synapse on that object
        assert_eq!(w.coll.hulls[0].list.len(), 2);
        assert_eq!(w.coll.hulls[1].list.len(), 2);
    }

    #[test]
    fn disabling_the_pair_removes_its_mindist() {
        let mut w = two_cubes();
        w.enable_collision_detection(0);
        w.enable_collision_detection(1);
        w.disable_collision_detection(0);
        w.disable_collision_detection(1);
        assert!(live_mindists(&w).is_empty());
        assert!(w.coll.exact.is_none());
    }
}
