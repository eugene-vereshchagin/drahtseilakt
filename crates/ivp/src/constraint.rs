// Port of OpenBallance src/phys/phys_constraint.c (GPL-3.0; see the LICENSE of the repository): the spring actuator and
// the constraint solver (docs/ivp_core.md 12): Set Physics Spring, Hinge, Slider and Ball Joint. Both are controllers of
// the sim unit their bodies are merged into. A joint's bodies are objects, referred to by their core index; a missing
// body (None) is the world.

use crate::core::{Controller, World};
use crate::event_sim::EventSim;
use crate::math::{self, Matrix};

type P3 = [f32; 3];
type V3 = [f64; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointKind {
    Hinge,
    Slider,
    BallJoint,
    Spring,
}

/// A joint (`PhysJoint`, 12.2 and 12.5): a spring with its anchors, or a constraint with its axes' types, limits and
/// frames. The controllers that act on it are in the world's controller arena.
#[derive(Clone, Debug)]
pub struct Joint {
    pub alive: bool,
    pub kind: JointKind,
    /// The two bodies (objects); None is the world.
    pub a: Option<usize>,
    pub b: Option<usize>,
    /// The movable cores the joint acts on.
    pub cores: Vec<usize>,
    /// The controller record of the joint in the world's arena.
    pub ctrl: usize,
    // spring
    pub anchor_core: [P3; 2],
    pub length: f32,
    pub k: f32,
    pub c_lin: f32,
    pub c_rel: f32,
    // constraint: per axis (three translations, three rotations) a flag (bit 0 active, bit 1 limited) and limits
    pub axis_type: [u32; 6],
    pub lo: [f32; 6],
    pub hi: [f32; 6],
    pub force_factor: f32,
    pub damp_factor: f32,
    pub limit_factor: f32,
    pub m_rcs_f_rcore: Matrix,
    pub m_acs_f_acore: Matrix,
    pub rot_order: [u8; 3],
    pub nfixed_rot: u8,
    pub nlimited_rot: u8,
    pub nactive: u8,
}

fn identity() -> Matrix {
    Matrix::identity()
}

fn rtmul_fd(r: &[[f64; 3]; 3], v: &[f64; 3]) -> P3 {
    let o = math::rtmul(r, v);
    [o[0] as f32, o[1] as f32, o[2] as f32]
}

fn to_v3(v: &P3) -> V3 {
    [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]
}

/// `R v` of an f32 vector, the result f32 (`ivp_rmul_f`).
fn rmul_f(r: &[[f64; 3]; 3], v: &P3) -> P3 {
    let o = math::rmul(r, &to_v3(v));
    [o[0] as f32, o[1] as f32, o[2] as f32]
}

/// `R v` of an f32 vector, the result f64 (`rmul_ff`).
fn rmul_ff(r: &[[f64; 3]; 3], v: &P3) -> V3 {
    math::rmul(r, &to_v3(v))
}

/// `R^T v` of an f32 vector, the result f32 (`ivp_rtmul_f`).
fn rtmul_f(r: &[[f64; 3]; 3], v: &P3) -> P3 {
    let o = math::rtmul(r, &to_v3(v));
    [o[0] as f32, o[1] as f32, o[2] as f32]
}

fn cross(a: &P3, b: &P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Gauss elimination with partial pivoting and back substitution of an n x n system (`lin_solve`, 0x10033e50).
fn lin_solve(a: &mut [[f64; 6]; 6], b: &mut [f64; 6], n: usize, x: &mut [f64; 6]) -> bool {
    let eps = f64::from(1e-9f32);
    for i in 0..n {
        let mut p = i;
        let mut best = a[i][i].abs();
        for r in (i + 1..n).rev() {
            if a[r][i].abs() > best {
                best = a[r][i].abs();
                p = r;
            }
        }
        if p != i {
            for k in 0..n {
                let t = a[i][k];
                a[i][k] = a[p][k];
                a[p][k] = t;
            }
            b.swap(i, p);
        }
        let piv = a[i][i];
        if piv.abs() < eps {
            continue;
        }
        for r in i + 1..n {
            if !(a[r][i].abs() > eps) {
                continue;
            }
            let f = a[r][i] * (-1.0 / piv);
            for k in i..n {
                a[r][k] += f * a[i][k];
            }
            b[r] += f * b[i];
        }
    }
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in i + 1..n {
            s -= a[i][k] * x[k];
        }
        if a[i][i].abs() >= eps {
            x[i] = s / a[i][i];
        } else if s.abs() < eps * 1000.0 {
            x[i] = 0.0;
        } else {
            for v in x.iter_mut().take(n) {
                *v = 0.0;
            }
            return false;
        }
    }
    true
}

impl World {
    /// The pose of an object at the current time, or the identity for the world (`object_world`).
    fn joint_object_world(&self, obj: Option<usize>) -> Matrix {
        match obj {
            Some(o) => self.cores[o].object_pose_at(self.env.current_time),
            None => identity(),
        }
    }

    /// The frame of the core relative to its object: the shift as the origin (`core_f_object`, 0x10009be0).
    fn joint_core_f_object(&self, obj: Option<usize>) -> Matrix {
        let mut m = identity();
        if let Some(o) = obj {
            m.vv = [f64::from(self.cores[o].shift[0]), f64::from(self.cores[o].shift[1]), f64::from(self.cores[o].shift[2])];
        }
        m
    }

    /// An anchor given in the world, kept in core space (`anchor_init`, 0x10013ea0).
    fn anchor_init(&self, obj: Option<usize>, pos_ws: P3) -> P3 {
        let Some(o) = obj else { return pos_ws };
        let m = self.joint_object_world(Some(o));
        let mi = math::mat_inverse(&m);
        let p = math::rmul(&mi.r, &to_v3(&pos_ws));
        let p = [p[0] + mi.vv[0], p[1] + mi.vv[1], p[2] + mi.vv[2]];
        let shift = self.cores[o].shift;
        [p[0] as f32 + shift[0], p[1] as f32 + shift[1], p[2] as f32 + shift[2]]
    }

    /// The surface speed of a body at a core-space point, zero for the world or a fixed body (`vsurf`, 0x1000c0b0).
    fn joint_vsurf(&self, obj: Option<usize>, p_cs: &P3) -> P3 {
        match obj {
            Some(o) if !self.cores[o].unmovable => self.surface_speed(o, p_cs),
            _ => [0.0; 3],
        }
    }

    /// The velocity of a core's point in world space (`ivp_surface_speed`, 0x1000bf90 with the cross product).
    pub(crate) fn surface_speed(&self, c: usize, p_cs: &P3) -> P3 {
        let core = &self.cores[c];
        let t = cross(&core.rot_speed, p_cs);
        let tw = rmul_f(&core.m_world_f_core.r, &t);
        [core.speed[0] + tw[0], core.speed[1] + tw[1], core.speed[2] + tw[2]]
    }

    /// A world-space impulse at a world point, applied to the core's speed changes (`ivp_async_push_core_ws`, 0x1000a4c0):
    /// the rotation change from the lever arm, and the speed change from the mass.
    pub(crate) fn async_push_core_ws(&mut self, c: usize, p_ws: V3, imp_ws: P3) {
        let core = &mut self.cores[c];
        let mut r = [0.0f32; 3];
        for k in 0..3 {
            r[k] = (p_ws[k] - core.m_world_f_core.vv[k]) as f32;
        }
        let t = cross(&r, &imp_ws);
        let tc = rtmul_f(&core.m_world_f_core.r, &t);
        for i in 0..3 {
            core.rot_speed_change[i] += core.inv_rot_inertia[i] * tc[i];
        }
        for i in 0..3 {
            core.speed_change[i] = (f64::from(core.speed_change[i]) + f64::from(imp_ws[i]) * f64::from(core.inv_mass)) as f32;
        }
    }

    /// A rotation impulse given in core space (`ivp_rot_push_core_cs`, 0x1000b2a0).
    fn rot_push_core_cs(&mut self, c: usize, t_cs: &P3) {
        let core = &mut self.cores[c];
        for i in 0..3 {
            core.rot_speed[i] += core.inv_rot_inertia[i] * t_cs[i];
        }
    }

    /// Attaches a controller to its cores, merging their units first; a frozen unit is revived when some core is
    /// awake (`ivp_controller_attach_merge`, 0x10011b30).
    fn controller_attach_merge(&mut self, ctrl: usize, cores: &[usize]) {
        let Some(&first) = cores.first() else { return };
        let into = self.cores[first].unit;
        for &c in &cores[1..] {
            let from = self.cores[c].unit;
            if from != into {
                self.sim_unit_merge(into, from);
            }
        }
        let mut acc: i32 = 8;
        for &c in cores {
            self.core_add_controller(c, ctrl);
            acc &= i32::from(self.cores[c].movement_state as u8);
        }
        if acc < 8 {
            self.revive_sim_unit(into);
        }
    }

    /// Detaches a controller from its cores; their units are marked changed (`ivp_controller_detach`, 0x10011a60).
    fn controller_detach(&mut self, ctrl: usize, cores: &[usize]) {
        for &c in cores {
            self.core_remove_controller(c, ctrl);
            let u = self.cores[c].unit;
            self.units[u].changed = true;
        }
    }

    /// The movable cores of a joint's bodies: the second skipped when it is the first (`joint_cores`).
    fn joint_cores_of(&self, a: Option<usize>, b: Option<usize>) -> Vec<usize> {
        let mut out = Vec::new();
        if let Some(a) = a {
            if !self.cores[a].unmovable {
                out.push(a);
            }
        }
        if let Some(b) = b {
            if !self.cores[b].unmovable && (a.is_none() || Some(b) != a) {
                out.push(b);
            }
        }
        out
    }

    fn joint_alloc(&mut self, j: Joint) -> usize {
        self.joints.push(j);
        self.joints.len() - 1
    }

    /// Set Physics Spring: anchors on two bodies (or the world), a rest length, a constant and two dampings
    /// (`phys_spring_create`, 0x10013700 / 0x10014250).
    pub fn spring_create(&mut self, a: Option<usize>, pa: P3, b: Option<usize>, pb: P3, length: f32, constant: f32, lin_damp: f32, glob_damp: f32) -> usize {
        let anchor0 = self.anchor_init(a, pa);
        let anchor1 = self.anchor_init(b, pb);
        let cores = self.joint_cores_of(a, b);
        let ctrl = self.add_controller(Controller::Spring { joint: self.joints.len() });
        let j = self.joint_alloc(Joint {
            alive: true,
            kind: JointKind::Spring,
            a,
            b,
            cores: cores.clone(),
            ctrl,
            anchor_core: [anchor0, anchor1],
            length,
            k: constant,
            c_lin: lin_damp,
            c_rel: glob_damp,
            ..Joint::blank()
        });
        self.controllers[ctrl] = Controller::Spring { joint: j };
        self.controller_attach_merge(ctrl, &cores);
        j
    }

    /// The spring's simulation: the force along the anchors' line, the surface speeds' damping, pushed asynchronously
    /// (`spring_simulate`, 0x100146d0).
    pub(crate) fn spring_simulate(&mut self, j: usize, ev: &EventSim) {
        let jo = self.joints[j].clone();
        let p0 = match jo.a {
            Some(a) => math::rmul(&self.cores[a].m_world_f_core.r, &to_v3(&jo.anchor_core[0])),
            None => to_v3(&jo.anchor_core[0]),
        };
        let p0 = match jo.a {
            Some(a) => [p0[0] + self.cores[a].m_world_f_core.vv[0], p0[1] + self.cores[a].m_world_f_core.vv[1], p0[2] + self.cores[a].m_world_f_core.vv[2]],
            None => p0,
        };
        let p1 = match jo.b {
            Some(b) => math::rmul(&self.cores[b].m_world_f_core.r, &to_v3(&jo.anchor_core[1])),
            None => to_v3(&jo.anchor_core[1]),
        };
        let p1 = match jo.b {
            Some(b) => [p1[0] + self.cores[b].m_world_f_core.vv[0], p1[1] + self.cores[b].m_world_f_core.vv[1], p1[2] + self.cores[b].m_world_f_core.vv[2]],
            None => p1,
        };
        let mut d = [(p0[0] - p1[0]) as f32, (p0[1] - p1[1]) as f32, (p0[2] - p1[2]) as f32];
        let len = math::normalize_f(&mut d);
        if len < 1e-10 {
            return;
        }
        let f = (f64::from(len) - f64::from(jo.length)) * f64::from(jo.k);
        let u1 = self.joint_vsurf(jo.b, &jo.anchor_core[1]);
        let u0 = self.joint_vsurf(jo.a, &jo.anchor_core[0]);
        let u = [u1[0] - u0[0], u1[1] - u0[1], u1[2] - u0[2]];
        let s = (f - f64::from(jo.c_lin) * (f64::from(d[0]) * f64::from(u[0]) + f64::from(d[1]) * f64::from(u[1]) + f64::from(d[2]) * f64::from(u[2])))
            * ev.delta_time;
        let cr = f64::from(jo.c_rel) * ev.delta_time;
        let mut imp = [0.0f32; 3];
        for k in 0..3 {
            imp[k] = (f64::from(d[k]) * s - cr * f64::from(u[k])) as f32;
        }
        let m_imp = [-imp[0], -imp[1], -imp[2]];
        if let Some(b) = jo.b {
            if i32::from(self.cores[b].movement_state) < 8 {
                self.async_push_core_ws(b, p1, imp);
            }
        }
        if let Some(a) = jo.a {
            if i32::from(self.cores[a].movement_state) < 8 {
                self.async_push_core_ws(a, p0, m_imp);
            }
        }
    }

    /// The constraint between two bodies: the frames of the anchor on each side, and the axes it locks
    /// (`constraint_create`, 0x10012570 and 0x100129c0).
    fn constraint_create(&mut self, kind: JointKind, r: Option<usize>, a: Option<usize>, anchor: P3, axis: Option<V3>, ntrans: u32, nrot: u32) -> Option<usize> {
        if r.is_none() && a.is_none() {
            return None;
        }
        let m_r = self.joint_object_world(r);
        let m_ri = math::mat_inverse(&m_r);
        let anc = to_v3(&anchor);
        let mut rfs_f_rcs = identity();
        let t = math::rmul(&m_ri.r, &anc);
        rfs_f_rcs.vv = [t[0] + m_ri.vv[0], t[1] + m_ri.vv[1], t[2] + m_ri.vv[2]];
        if let Some(ax) = axis {
            let ax_w = math::rtmul(&m_r.r, &ax);
            if nrot == 2 || ntrans == 2 {
                rfs_f_rcs.r = math::basis(&ax_w, 2);
            } else if nrot == 1 || ntrans == 1 {
                rfs_f_rcs.r = math::basis(&ax_w, 0);
            }
        }
        let mut axis_type = [0u32; 6];
        for i in 0..3 {
            axis_type[i] = u32::from(ntrans > i as u32);
            axis_type[3 + i] = u32::from(nrot > i as u32);
        }
        // the constraint object: the A side's frame in R's, and the core frames of both sides
        let wl_a = self.joint_object_world(a);
        let wl_ri = math::mat_inverse(&m_r);
        let rfs_f_afs = math::mat_mul(&wl_ri, &wl_a);
        let rcore_f_rfs = self.joint_core_f_object(r);
        let acore_f_afs = self.joint_core_f_object(a);
        let afs_f_acore = math::mat_inverse(&acore_f_afs);
        let t = math::mat_mul(&rcore_f_rfs, &rfs_f_afs);
        let rcore_f_acore = math::mat_mul(&t, &afs_f_acore);
        let t = math::mat_mul(&rcore_f_rfs, &rfs_f_rcs);
        let m_rcs_f_rcore = math::mat_inverse_general(&t, 1e-19).unwrap_or_else(identity);
        let m_acs_f_acore = math::mat_mul(&m_rcs_f_rcore, &rcore_f_acore);
        let cores = self.joint_cores_of(r, a);
        let ctrl = self.add_controller(Controller::Constraint { joint: self.joints.len() });
        let j = self.joint_alloc(Joint {
            alive: true,
            kind,
            a: r,
            b: a,
            cores,
            ctrl,
            axis_type,
            force_factor: 1.0,
            damp_factor: 1.0,
            limit_factor: 0.3,
            m_rcs_f_rcore,
            m_acs_f_acore,
            ..Joint::blank()
        });
        self.controllers[ctrl] = Controller::Constraint { joint: j };
        Some(j)
    }

    /// Orders the rotation axes (fixed, then limited, then free) and attaches the constraint (`constraint_register`,
    /// 0x100283f0).
    fn constraint_register(&mut self, j: usize) {
        let mut order = [0u8; 3];
        let mut o = 0usize;
        let mut nfixed = 0u8;
        let mut nlimited = 0u8;
        for i in 0..3 {
            if self.joints[j].axis_type[3 + i] == 1 {
                order[o] = i as u8;
                o += 1;
                nfixed += 1;
            }
        }
        for i in 0..3 {
            if self.joints[j].axis_type[3 + i] & 2 != 0 {
                order[o] = i as u8;
                o += 1;
                nlimited += 1;
            }
        }
        for i in 0..3 {
            if self.joints[j].axis_type[3 + i] == 0 {
                order[o] = i as u8;
                o += 1;
            }
        }
        self.joints[j].rot_order = order;
        self.joints[j].nfixed_rot = nfixed;
        self.joints[j].nlimited_rot = nlimited;
        let (cores, ctrl) = (self.joints[j].cores.clone(), self.joints[j].ctrl);
        self.controller_attach_merge(ctrl, &cores);
    }

    /// Set Physics Hinge: three translations and two rotations locked about the axis (`0x100059d0`).
    pub fn hinge_create(&mut self, a: Option<usize>, b: Option<usize>, anchor: P3, axis: P3) -> Option<usize> {
        let ax = [f64::from(axis[0]), f64::from(axis[1]), f64::from(axis[2])];
        let j = self.constraint_create(JointKind::Hinge, a, b, anchor, Some(ax), 3, 2)?;
        self.constraint_register(j);
        Some(j)
    }

    /// Set Physics Slider: the axis from p0 to p1, two translations and three rotations locked, with limits on the
    /// translation along the axis when asked (`0x10005f10`, 0x10012960).
    pub fn slider_create(&mut self, a: Option<usize>, b: Option<usize>, p0: P3, p1: P3, limits: bool, lo: f32, hi: f32) -> Option<usize> {
        let ax = [f64::from(p1[0]) - f64::from(p0[0]), f64::from(p1[1]) - f64::from(p0[1]), f64::from(p1[2]) - f64::from(p0[2])];
        let j = self.constraint_create(JointKind::Slider, a, b, p0, Some(ax), 2, 3)?;
        if limits {
            let jj = &mut self.joints[j];
            jj.axis_type[2] = 2;
            jj.lo[2] = lo;
            jj.hi[2] = hi;
        }
        self.constraint_register(j);
        Some(j)
    }

    /// Set Physics Ball Joint: three translations locked (`0x100052f0`).
    pub fn balljoint_create(&mut self, a: Option<usize>, b: Option<usize>, anchor: P3) -> Option<usize> {
        let j = self.constraint_create(JointKind::BallJoint, a, b, anchor, None, 3, 0)?;
        self.constraint_register(j);
        Some(j)
    }

    /// Detaches a joint from its cores and forgets it (`ivp_joint_free`, 0x10011a60 and the destroy).
    pub fn joint_destroy(&mut self, j: usize) {
        if !self.joints.get(j).is_some_and(|x| x.alive) {
            return;
        }
        let (cores, ctrl) = (self.joints[j].cores.clone(), self.joints[j].ctrl);
        self.controller_detach(ctrl, &cores);
        self.joints[j].alive = false;
    }

    /// The constraint's simulation: one exact velocity solve of the active axes per PSI, applied immediately
    /// (`constraint_simulate`, 0x10028960).
    pub(crate) fn constraint_simulate(&mut self, j: usize, ev: &EventSim) {
        let jo = self.joints[j].clone();
        let (r_obj, a_obj) = (jo.a, jo.b);
        let m_r_ok = r_obj.is_some_and(|o| !self.cores[o].unmovable);
        let m_a_ok = a_obj.is_some_and(|o| !self.cores[o].unmovable);
        let wr = match r_obj {
            Some(o) => self.cores[o].m_world_f_core,
            None => identity(),
        };
        let wa = match a_obj {
            Some(o) => self.cores[o].m_world_f_core,
            None => identity(),
        };
        let dt = ev.delta_time as f32;
        let idt = ev.i_delta_time;
        let rr = jo.m_rcs_f_rcore.r;
        let ra = jo.m_acs_f_acore.r;
        let cs_w = math::mat_rrt(&rr, &wr.r);
        let cs_a = math::mat_rr(&cs_w, &wa.r);

        // the angular error
        let mut erot = [0.0f64; 3];
        let nrot = jo.nfixed_rot as u32 + jo.nlimited_rot as u32;
        if nrot == 3 || nrot == 2 {
            let q_mat = math::mat_rrt(&cs_a, &ra);
            if nrot == 3 {
                let q = math::matrix_to_quat(&q_mat);
                let qv = [f64::from(q.x as f32), q.y, q.z];
                for i in 0..3 {
                    let x = qv[i];
                    let x3 = x * x * x;
                    erot[i] = 2.0 * x + f64::from(0.24f32) * x3 + f64::from(0.58f32) * x3 * x * x;
                }
                if q.w < 0.0 {
                    for v in erot.iter_mut() {
                        *v = -*v;
                    }
                }
            } else {
                let (o0, o1, o2) = (jo.rot_order[0] as usize, jo.rot_order[1] as usize, jo.rot_order[2] as usize);
                let b = [q_mat[0][o2], q_mat[1][o2], q_mat[2][o2]];
                erot[o0] = -b[o1].atan2(b[o2]);
                erot[o1] = b[o0].atan2(b[o2]);
                erot[o2] = 0.0;
            }
        }

        // the linear part: the anchors' points in world space and their surface speeds
        let neg_rcs = [-jo.m_rcs_f_rcore.vv[0], -jo.m_rcs_f_rcore.vv[1], -jo.m_rcs_f_rcore.vv[2]];
        let neg_acs = [-jo.m_acs_f_acore.vv[0], -jo.m_acs_f_acore.vv[1], -jo.m_acs_f_acore.vv[2]];
        let a_r = rtmul_fd(&rr, &neg_rcs);
        let a_a = rtmul_fd(&ra, &neg_acs);
        let p_r = math::rmul(&wr.r, &to_v3(&a_r));
        let p_r = [p_r[0] + wr.vv[0], p_r[1] + wr.vv[1], p_r[2] + wr.vv[2]];
        let p_a = math::rmul(&wa.r, &to_v3(&a_a));
        let p_a = [p_a[0] + wa.vv[0], p_a[1] + wa.vv[1], p_a[2] + wa.vv[2]];
        let v_r = if r_obj.is_some() { self.surface_speed(r_obj.unwrap_or(0), &a_r) } else { [0.0; 3] };
        let v_a = if a_obj.is_some() { self.surface_speed(a_obj.unwrap_or(0), &a_a) } else { [0.0; 3] };
        let d = [p_a[0] - p_r[0], p_a[1] - p_r[1], p_a[2] - p_r[2]];
        let err = math::rmul(&cs_w, &d);
        let cv_a = rmul_ff(&cs_w, &v_a);
        let cv_r = rmul_ff(&cs_w, &v_r);
        let relv = [cv_a[0] - cv_r[0], cv_a[1] - cv_r[1], cv_a[2] - cv_r[2]];
        let kd = f64::from(dt) * f64::from(jo.damp_factor);
        let g = -f64::from(jo.force_factor) * f64::from(idt as f32);
        let mut axis_type = jo.axis_type;
        let mut rhs = [0.0f64; 6];
        for i in 0..3 {
            let mut b = err[i] + kd * relv[i];
            if axis_type[i] & 2 != 0 {
                if b < f64::from(jo.lo[i]) {
                    axis_type[i] |= 1;
                    b = f64::from(dt) * relv[i] + (err[i] - f64::from(jo.lo[i])) * f64::from(jo.limit_factor);
                } else if b > f64::from(jo.hi[i]) {
                    axis_type[i] |= 1;
                    b = f64::from(dt) * relv[i] + (err[i] - f64::from(jo.hi[i])) * f64::from(jo.limit_factor);
                } else {
                    axis_type[i] &= !1;
                }
            }
            rhs[i] = f64::from(b as f32) * g;
        }

        // the angular part
        let w_a = if a_obj.is_some() { rmul_ff(&cs_a, &self.cores[a_obj.unwrap_or(0)].rot_speed) } else { [0.0; 3] };
        let w_r = if r_obj.is_some() { rmul_ff(&rr, &self.cores[r_obj.unwrap_or(0)].rot_speed) } else { [0.0; 3] };
        let relw = [w_a[0] - w_r[0], w_a[1] - w_r[1], w_a[2] - w_r[2]];
        for i in 0..3 {
            let mut c = erot[i] + kd * relw[i];
            if axis_type[3 + i] & 2 != 0 {
                if c < f64::from(jo.lo[3 + i]) {
                    axis_type[3 + i] |= 1;
                    c = f64::from(dt) * relw[i] + (erot[i] - f64::from(jo.lo[3 + i])) * f64::from(jo.limit_factor);
                } else if c > f64::from(jo.hi[3 + i]) {
                    axis_type[3 + i] |= 1;
                    c = f64::from(dt) * relw[i] + (erot[i] - f64::from(jo.hi[3 + i])) * f64::from(jo.limit_factor);
                } else {
                    axis_type[3 + i] &= !1;
                }
            }
            rhs[3 + i] = f64::from(c as f32) * g;
        }
        self.joints[j].axis_type = axis_type;
        let act: Vec<usize> = (0..6).filter(|&i| axis_type[i] & 1 != 0).collect();
        let na = act.len();
        self.joints[j].nactive = na as u8;
        if na == 0 {
            return;
        }

        // the response matrix: column ci is the measured axes' response to a unit impulse on active axis act[ci]
        let mut m = [[0.0f64; 6]; 6];
        for (ci, &a) in act.iter().enumerate() {
            let mut e = [0.0f64; 3];
            e[a % 3] = 1.0;
            let ne = [-e[0], -e[1], -e[2]];
            let mut t_sum = [0.0f64; 3];
            let mut rm = [0.0f64; 3];
            if a < 3 {
                if m_r_ok {
                    let ro = r_obj.unwrap_or(0);
                    let dc = rtmul_fd(&rr, &e);
                    let dw_ = rtmul_fd(&cs_w, &e);
                    let (dv, dw) = self.calc_push_core(ro, a_r, dc, dw_);
                    let c = cross(&dw, &a_r);
                    let mut cw = rmul_f(&wr.r, &c);
                    cw = [cw[0] + dv[0], cw[1] + dv[1], cw[2] + dv[2]];
                    let tv = rmul_ff(&cs_w, &cw);
                    for k in 0..3 {
                        t_sum[k] -= tv[k];
                    }
                    let tv = rmul_ff(&rr, &dw);
                    for k in 0..3 {
                        rm[k] -= tv[k];
                    }
                }
                if m_a_ok {
                    let ao = a_obj.unwrap_or(0);
                    let dc = rtmul_fd(&cs_a, &ne);
                    let dw_ = rtmul_fd(&cs_w, &ne);
                    let (dv, dw) = self.calc_push_core(ao, a_a, dc, dw_);
                    let c = cross(&dw, &a_a);
                    let mut cw = rmul_f(&wa.r, &c);
                    cw = [cw[0] + dv[0], cw[1] + dv[1], cw[2] + dv[2]];
                    let tv = rmul_ff(&cs_w, &cw);
                    for k in 0..3 {
                        t_sum[k] += tv[k];
                    }
                    let tv = rmul_ff(&cs_a, &dw);
                    for k in 0..3 {
                        rm[k] += tv[k];
                    }
                }
            } else {
                let mut dw = [0.0f32; 3];
                if m_r_ok {
                    let ro = r_obj.unwrap_or(0);
                    let tc = rtmul_fd(&rr, &e);
                    for k in 0..3 {
                        dw[k] = self.cores[ro].inv_rot_inertia[k] * tc[k];
                    }
                    let dv = cross(&dw, &a_r);
                    let tv = rmul_ff(&rr, &dv);
                    for k in 0..3 {
                        t_sum[k] -= tv[k];
                    }
                    let tv = rmul_ff(&rr, &dw);
                    for k in 0..3 {
                        rm[k] -= tv[k];
                    }
                }
                if m_a_ok {
                    let ao = a_obj.unwrap_or(0);
                    let tc = rtmul_fd(&cs_a, &ne);
                    for k in 0..3 {
                        dw[k] = self.cores[ao].inv_rot_inertia[k] * tc[k];
                    }
                    let dv = cross(&dw, &a_a);
                    let tv = rmul_ff(&cs_a, &dv);
                    for k in 0..3 {
                        t_sum[k] += tv[k];
                    }
                    let tv = rmul_ff(&cs_a, &dw);
                    for k in 0..3 {
                        rm[k] += tv[k];
                    }
                }
            }
            for (ri, &r) in act.iter().enumerate() {
                m[ri][ci] = if r < 3 { t_sum[r] } else { rm[r - 3] };
            }
        }
        let mut bb = [0.0f64; 6];
        for ri in 0..na {
            bb[ri] = rhs[act[ri]];
        }
        let mut x = [0.0f64; 6];
        if !lin_solve(&mut m, &mut bb, na, &mut x) {
            return;
        }
        let mut lt = [0.0f64; 3];
        let mut lr = [0.0f64; 3];
        for (ci, &a) in act.iter().enumerate() {
            if a < 3 {
                lt[a] = x[ci];
            } else {
                lr[a - 3] = x[ci];
            }
        }
        let nlt = [-lt[0], -lt[1], -lt[2]];
        let nlr = [-lr[0], -lr[1], -lr[2]];
        // applied at once
        if m_r_ok {
            let ro = r_obj.unwrap_or(0);
            let ic = rtmul_fd(&rr, &lt);
            let iw = rmul_f(&wr.r, &ic);
            self.push_core(ro, a_r, ic, iw);
            let tc = rtmul_fd(&rr, &lr);
            self.rot_push_core_cs(ro, &tc);
        }
        if m_a_ok {
            let ao = a_obj.unwrap_or(0);
            let ic = rtmul_fd(&cs_a, &nlt);
            let iw = rmul_f(&wa.r, &ic);
            self.push_core(ao, a_a, ic, iw);
            let tc = rtmul_fd(&cs_a, &nlr);
            self.rot_push_core_cs(ao, &tc);
        }
    }
}

impl Joint {
    fn blank() -> Joint {
        Joint {
            alive: false,
            kind: JointKind::Spring,
            a: None,
            b: None,
            cores: Vec::new(),
            ctrl: 0,
            anchor_core: [[0.0; 3]; 2],
            length: 0.0,
            k: 0.0,
            c_lin: 0.0,
            c_rel: 0.0,
            axis_type: [0; 6],
            lo: [0.0; 6],
            hi: [0.0; 6],
            force_factor: 1.0,
            damp_factor: 1.0,
            limit_factor: 0.3,
            m_rcs_f_rcore: identity(),
            m_acs_f_acore: identity(),
            rot_order: [0; 3],
            nfixed_rot: 0,
            nlimited_rot: 0,
            nactive: 0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::math::Quat;

    fn at(w: &mut World, x: f64) -> usize {
        let q = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
        let c = w.core_init(w.cores.len(), q, [x, 0.0, 0.0], false);
        w.cores[c].upper_limit_radius = 0.5;
        w.cores[c].movement_state = crate::core::MT_MOVING;
        // a unit mass with a unit inertia (the core is created with none)
        w.cores[c].mass = 1.0;
        w.cores[c].inv_mass = 1.0;
        w.cores[c].rot_inertia = [1.0; 3];
        w.cores[c].inv_rot_inertia = [1.0; 3];
        c
    }

    #[test]
    fn a_stretched_spring_pulls_its_bodies_together() {
        let mut w = World::new();
        let a = at(&mut w, 0.0);
        let b = at(&mut w, 2.0);
        // rest length 1, stretched to 2: the bodies are pulled toward each other along x
        // the anchors are world points: the centre of each body
        let j = w.spring_create(Some(a), [0.0; 3], Some(b), [2.0, 0.0, 0.0], 1.0, 10.0, 0.0, 0.0);
        let ev = EventSim {
            delta_time: 1.0 / 66.0,
            i_delta_time: 66.0,
            sim_unit: 0,
        };
        w.spring_simulate(j, &ev);
        // the pending speed changes pull a towards b (+x) and b towards a (-x)
        assert!(w.cores[a].speed_change[0] > 0.0, "a {:?}", w.cores[a].speed_change);
        assert!(w.cores[b].speed_change[0] < 0.0, "b {:?}", w.cores[b].speed_change);
    }

    #[test]
    fn a_ball_joint_keeps_two_anchors_together() {
        let mut w = World::new();
        let a = at(&mut w, 0.0);
        let b = at(&mut w, 1.0);
        let j = w.balljoint_create(Some(a), Some(b), [0.5, 0.0, 0.0]).expect("a ball joint");
        assert_eq!(w.joints[j].nactive, 0, "nothing active before the first solve");
        let ev = EventSim {
            delta_time: 1.0 / 66.0,
            i_delta_time: 66.0,
            sim_unit: 0,
        };
        w.constraint_simulate(j, &ev);
        assert_eq!(w.joints[j].nactive, 3, "three translations locked");
    }
}
