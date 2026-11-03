// Port of OpenBallance src/phys/phys_impact.c (GPL-3.0; see the LICENSE of the repository): the impact path (docs/
// ivp_contact.md 6, "IM"): the mindist's do_impact at the exact event time, the impact solver (friction-cone compression
// in steps, restitution at sqrt(e), the rescue separation), the impact system that chains impacts through the friction
// system's pairs, and the post-collision event, which is kept for the glue to read.
//
// The cores are referred to by their object index. A pair of a friction system is found by its two cores, since the
// system's pair list changes as points come and go.

use crate::core::World;
use crate::math::{self, Quat};
use crate::mindist::{Mindist, ST_BALL};
use crate::contact::ContactInfo;
use crate::friction::C_01F;

type P3 = [f32; 3];
type V3 = [f64; 3];

/// Impact constants (ivp_const.h, IM): the approach limit, the restitution epsilons and the fixed mass.
const C_APPROACH: f64 = -9.999_999_747_378_752e-5;
const C_DIFF_EPS: f64 = 9.999_999_747_378_752e-5;
const C_PUSHES_SPEED: f64 = 0.0099999998;
const C_RESCUE_FACTOR: f32 = 1.2;
const C_DELAY_FACTOR: f32 = -0.8333333;
const C_FIXED_MASS: f64 = 100000.0;
const C_ROT_RESCUE: f64 = 2.5e-5;
const C_FAR_GAP: f32 = 1e20;
const C_TAYLOR4: f32 = 0.041666668;
const C_MAX_IMPACT_ROUNDS: u32 = 5000;
const SET_MAX_DIST_FOR_IMPACT_SYSTEM: f32 = f32::from_bits(0x3e61_47ae);
const SET_MINDIST_CHANGE_FORCE_DIST: f32 = f32::from_bits(0x3ca3_d70a);
const C_1E19: f64 = 1e-19;

/// The post-collision event the glue reads (`PhysImpactEvent`): the objects, the relative velocity and the normal
/// of the impact, its point, and the time since the pair's last impact.
#[derive(Clone, Debug, Default)]
pub struct ImpactEvent {
    pub obj: [usize; 2],
    /// The objects in the mindist's order: the listeners of obj order[0] run first, then order[1]'s.
    pub order: [usize; 2],
    pub rel: P3,
    pub normal: P3,
    pub pos: V3,
    pub dt_last: f32,
}

/// The impact solver's working copies (`ImpactSolver`, IM5): the two cores (object indices), their points in core
/// space, the working speeds and rotations, the last pushes, and the impulse direction.
struct Solver {
    rescue: f32,
    m0: f64,
    m1: f64,
    allow_delaying: bool,
    rot: [P3; 2],
    speed: [P3; 2],
    dw: [P3; 2],
    dv: [P3; 2],
    rel: P3,
    dir: P3,
    dir2: P3,
    core: [usize; 2],
    p_cs: [P3; 2],
    e: f32,
    cos_: f32,
    sin_: f32,
    n: P3,
}

fn ndot(n: &P3, v: &P3) -> f64 {
    (f64::from(v[2]) * f64::from(n[2]) + f64::from(v[1]) * f64::from(n[1])) + f64::from(v[0]) * f64::from(n[0])
}

fn dotd(a: &P3, b: &P3) -> f64 {
    (f64::from(a[2]) * f64::from(b[2]) + f64::from(a[1]) * f64::from(b[1])) + f64::from(a[0]) * f64::from(b[0])
}

/// The f32 vector normalized in place (`normize`, 0x1000e030).
fn normize(v: &mut P3) {
    let l = dotd(v, v);
    if l < C_1E19 {
        return;
    }
    let y = 1.0 / l.sqrt();
    for c in v.iter_mut() {
        *c = (f64::from(*c) * y) as f32;
    }
}

fn v3sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn v3add(a: P3, b: P3) -> P3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn v3scale(a: P3, s: f32) -> P3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

impl World {
    fn movable(&self, c: usize) -> bool {
        !self.cores[c].unmovable
    }

    /// Brings a core to the current time for the impact: its rotation and pose move to now, its rotation speed is
    /// recomputed from the PSI's rotation, and the old values are kept to restore (`synchronize_with_rot_z`, 0x1000cfa0).
    pub(crate) fn core_sync(&mut self, c: usize) {
        let now = self.env.current_time;
        let core = &mut self.cores[c];
        core.synced = true;
        core.sync_pushed = false;
        core.sync_rot_speed = core.rot_speed;
        core.sync_q = core.q_next;
        let inv = Quat {
            x: -core.q_last.x,
            y: -core.q_last.y,
            z: -core.q_last.z,
            w: core.q_last.w,
        };
        let d = math::quat_mul(&inv, &core.q_next);
        let t = now - core.time_of_last_psi;
        let r = math::quat_slerp(&core.q_last, &core.q_next, t * f64::from(core.i_delta_time));
        core.q_next = r;
        core.m_world_f_core.r = math::quat_to_matrix(&core.q_next);
        for k in 0..3 {
            core.m_world_f_core.vv[k] = f64::from(core.delta_pos[k]) * t + core.pos_last[k];
        }
        let q = f64::from(core.i_delta_time) + f64::from(core.i_delta_time);
        core.rot_speed[0] = (d.x.asin() * q) as f32;
        core.rot_speed[1] = (d.y.asin() * q) as f32;
        core.rot_speed[2] = (d.z.asin() * q) as f32;
    }

    /// Restores a synced core that did not push (`0x1000d140`).
    fn core_restore(&mut self, c: usize) {
        let core = &mut self.cores[c];
        core.rot_speed = core.sync_rot_speed;
        core.q_next = core.sync_q;
        core.synced = false;
    }

    /// The rescue speed of a point's rotating cores (`rot_rescue`, 0x100248a0).
    fn rot_rescue(&self, cp: usize) -> f32 {
        let p = &self.coll.cps[cp];
        let mut s = 0.0f32;
        for i in 0..2 {
            let Some(c) = p.info.core[i] else { continue };
            if p.syn[i].typ == ST_BALL {
                continue;
            }
            let core = &self.cores[c];
            let mut w2 = dotd(&core.rot_speed, &core.rot_speed) * C_ROT_RESCUE;
            if w2 > 0.25 {
                w2 = 0.25;
            }
            s = (f64::from(s) + (1.0 - w2.sqrt().cos()) * f64::from(core.upper_limit_radius) * self.env.inv_delta_psi) as f32;
        }
        s
    }

    /// The rescue speed of a point (`ivp_cp_rescue_speed`, 0x10024930).
    pub(crate) fn cp_rescue_speed(&mut self, cp: usize) -> f32 {
        let gap = self.coll.cps[cp].gap;
        let v = if gap < crate::event::COLL_DIST {
            (f64::from(crate::event::COLL_DIST) - f64::from(gap)) * self.env.inv_delta_psi
        } else {
            self.coll.cps[cp].info.rescue = 0.0;
            0.0
        };
        let s = v + f64::from(self.rot_rescue(cp));
        (s + s) as f32
    }

    /// The velocity of a core's point from the given speeds (`ivp_core_point_velocity` with explicit speeds).
    fn point_velocity(&self, c: usize, p_cs: P3, speed: P3, rot: P3) -> P3 {
        crate::contact::core_point_velocity(&self.cores[c].m_world_f_core.r, &p_cs, &speed, &rot)
    }

    /// The relative velocity v1(p1) - v0(p0) of the solver's working copies (`calc_rel`, 0x10022500).
    fn calc_rel(&self, s: &mut Solver) {
        let v0 = self.point_velocity(s.core[0], s.p_cs[0], s.speed[0], s.rot[0]);
        let v1 = self.point_velocity(s.core[1], s.p_cs[1], s.speed[1], s.rot[1]);
        s.rel = v3sub(v1, v0);
    }

    /// The push direction clamped to the friction cone around -n (`calc_push_direction`, 0x10022580).
    fn calc_push_direction(&self, s: &mut Solver) {
        s.dir = s.rel;
        normize(&mut s.dir);
        let c = ndot(&s.n, &s.dir);
        if c > 0.0 {
            s.dir = s.dir2;
            normize(&mut s.dir);
        } else if -f64::from(s.cos_) < c {
            // outside the cone: sliding
            let mut t = [0.0f32; 3];
            for k in 0..3 {
                t[k] = (f64::from(s.dir[k]) - c * f64::from(s.n[k])) as f32;
            }
            normize(&mut t);
            for k in 0..3 {
                s.dir[k] = (f64::from(-s.cos_ * s.n[k]) + f64::from(s.sin_ * t[k])) as f32;
            }
        }
    }

    /// The change of velocity of a core for an impulse in world space at its point (`test_push`, 0x1000ca80).
    fn test_push(&self, c: usize, p_cs: P3, imp_ws: P3) -> (P3, P3) {
        let r = self.cores[c].m_world_f_core.r;
        let imp_cs_d = math::rtmul(&r, &[f64::from(imp_ws[0]), f64::from(imp_ws[1]), f64::from(imp_ws[2])]);
        let imp_cs = [imp_cs_d[0] as f32, imp_cs_d[1] as f32, imp_cs_d[2] as f32];
        self.calc_push_core(c, p_cs, imp_cs, imp_ws)
    }

    /// Pushes both movable cores by `f` along the direction (`push`, 0x10023940).
    fn push(&self, s: &mut Solver, f: f64) {
        let imp = v3scale(s.dir, f as f32);
        for i in 0..2 {
            if !self.movable(s.core[i]) {
                continue;
            }
            let im = if i == 1 { [-imp[0], -imp[1], -imp[2]] } else { imp };
            let (dv, dw) = self.test_push(s.core[i], s.p_cs[i], im);
            s.dv[i] = dv;
            s.dw[i] = dw;
            s.rot[i] = v3add(s.rot[i], dw);
            s.speed[i] = v3add(s.speed[i], dv);
        }
    }

    /// Takes the last pushes back (`undo_push`, 0x10022950).
    fn undo_push(&self, s: &mut Solver) {
        for i in 0..2 {
            if !self.movable(s.core[i]) {
                continue;
            }
            s.rot[i] = v3sub(s.rot[i], s.dw[i]);
            s.speed[i] = v3sub(s.speed[i], s.dv[i]);
        }
    }

    /// The point velocity change of a unit impulse (`unit_response`).
    fn unit_response(&self, c: usize, p_cs: P3, imp: P3) -> P3 {
        let (dv, dw) = self.test_push(c, p_cs, imp);
        self.point_velocity(c, p_cs, dv, dw)
    }

    /// Pushes along -dir up to the minimum separating speed (`ensure_min_separation`, 0x10022a20).
    fn ensure_min_separation(&self, s: &mut Solver, dir_in: P3, clamp: bool) {
        let d = [-dir_in[0], -dir_in[1], -dir_in[2]];
        let v0 = self.point_velocity(s.core[0], s.p_cs[0], s.speed[0], s.rot[0]);
        let v1 = self.point_velocity(s.core[1], s.p_cs[1], s.speed[1], s.rot[1]);
        let r = v3sub(v1, v0);
        let mut sep = ndot(&d, &r);
        if clamp && sep > 0.0 {
            sep = 0.0;
        }
        let diff = f64::from(s.rescue) - sep;
        if !(diff >= 0.0) {
            return;
        }
        let mut inv = 0.0;
        let md = [-d[0], -d[1], -d[2]];
        if self.movable(s.core[1]) {
            let u = self.unit_response(s.core[1], s.p_cs[1], d);
            inv += ndot(&d, &u);
        }
        if self.movable(s.core[0]) {
            let u = self.unit_response(s.core[0], s.p_cs[0], md);
            inv += ndot(&md, &u);
        }
        let k = diff / inv;
        if !(k >= 0.0) {
            return;
        }
        for i in 0..2 {
            if !self.movable(s.core[i]) {
                continue;
            }
            let imp = v3scale(d, (if i == 1 { k } else { -k }) as f32);
            let (dv, dw) = self.test_push(s.core[i], s.p_cs[i], imp);
            s.dv[i] = dv;
            s.dw[i] = dw;
            s.rot[i] = v3add(s.rot[i], dw);
            s.speed[i] = v3add(s.speed[i], dv);
        }
    }

    /// The elastic impulse of the pair from the virtual masses along the normal (`full_elastic_impulse`, 0x10023b90).
    fn full_elastic_impulse(&self, s: &mut Solver) -> f64 {
        let mn = [-s.n[0], -s.n[1], -s.n[2]];
        let u = self.unit_response(s.core[1], s.p_cs[1], s.n);
        s.m1 = 1.0 / dotd(&u, &u).sqrt();
        let u = self.unit_response(s.core[0], s.p_cs[0], mn);
        s.m0 = 1.0 / dotd(&u, &u).sqrt();
        if !self.movable(s.core[0]) {
            s.m0 = s.m1 * C_FIXED_MASS;
        }
        if !self.movable(s.core[1]) {
            s.m1 = s.m0 * C_FIXED_MASS;
        }
        (1.0 / (s.m0 + s.m1)) * 2.0 * -ndot(&s.n, &s.rel) * s.m1 * s.m0
    }

    /// The writes straight into the cores' speeds (`write_direct`, 0x100228c0).
    fn write_direct(&mut self, s: &Solver, i: usize) {
        let c = &mut self.cores[s.core[i]];
        c.speed = s.speed[i];
        c.rot_speed = s.rot[i];
    }

    /// The final velocities: the heavier body's response is delayed when the lighter one leaves it (`apply_result`,
    /// 0x10023680). `pushed` tells which cores pushed.
    fn apply_result(&mut self, s: &Solver, pushed: &mut [Option<usize>; 2]) {
        for i in 0..2 {
            if self.movable(s.core[i]) {
                // a count the upstream keeps in 16 bits and lets wrap; a map's pile of bodies can pass 32767 impacts
                // in one PSI (found by fuzzing a level played), which overflowed here
                self.cores[s.core[i]].impacts_psi = self.cores[s.core[i]].impacts_psi.wrapping_add(1);
            }
        }
        if s.allow_delaying {
            let h = if s.m0 <= s.m1 { 1 } else { 0 };
            let l = 1 - h;
            if let Some(ph) = pushed[h] {
                if self.movable(ph) {
                    let ch = s.core[h];
                    let vh = self.point_velocity(ch, s.p_cs[h], self.cores[ch].speed, self.cores[ch].rot_speed);
                    let vl = self.point_velocity(s.core[l], s.p_cs[l], s.speed[l], s.rot[l]);
                    let r = v3sub(vh, vl);
                    let mut q = ndot(&s.n, &r);
                    if h == 1 {
                        q *= -1.0;
                    }
                    if q < f64::from(s.rescue * C_DELAY_FACTOR) {
                        pushed[h] = None;
                        for i in 0..2 {
                            self.cores[s.core[i]].speed_change = [0.0; 3];
                            self.cores[s.core[i]].rot_speed_change = [0.0; 3];
                        }
                        self.write_direct(s, l);
                        let ch_speed = self.cores[ch].speed;
                        let ch_rot = self.cores[ch].rot_speed;
                        self.cores[ch].speed_change = v3sub(s.speed[h], ch_speed);
                        self.cores[ch].rot_speed_change = v3sub(s.rot[h], ch_rot);
                        self.cores[ch].impacts_psi = self.cores[ch].impacts_psi.wrapping_sub(1);
                        return;
                    }
                }
            }
        }
        for i in 0..2 {
            self.cores[s.core[i]].speed_change = [0.0; 3];
            self.cores[s.core[i]].rot_speed_change = [0.0; 3];
        }
        self.write_direct(s, 0);
        self.write_direct(s, 1);
    }

    /// The impact solve: compression in steps within the friction cone, restitution, the minimum separation, the
    /// anomaly limits, and the result (`impact_solve`, 0x10022ed0).
    fn impact_solve(&mut self, s: &mut Solver, pushed: &mut [Option<usize>; 2], allow_delaying: bool, pushes: i16, rescue_addon: f32, rel_out: &mut P3) {
        let inv_pushes = 1.0f32 / (f32::from(pushes) * 0.5f32 + 1.0f32);
        let e_eff = f64::from(1.0f32 - (1.0f32 - s.e) * inv_pushes);
        s.allow_delaying = allow_delaying;
        s.rescue = (SET_MINDIST_CHANGE_FORCE_DIST + rescue_addon) * C_RESCUE_FACTOR;
        pushed[0] = Some(s.core[0]);
        pushed[1] = Some(s.core[1]);
        for i in 0..2 {
            let c = &self.cores[s.core[i]];
            s.rot[i] = v3add(c.rot_speed, c.rot_speed_change);
            s.speed[i] = v3add(c.speed, c.speed_change);
        }
        self.calc_rel(s);
        *rel_out = s.rel;
        let mut iter = 0u32;
        let f = self.full_elastic_impulse(s) * C_01F;
        let mn = [-s.n[0], -s.n[1], -s.n[2]];
        if ndot(&s.n, &s.rel) <= C_APPROACH {
            self.calc_push_direction(s);
            s.dir2 = [0.0; 3];
            let vn0 = -ndot(&s.n, &s.rel);
            let mut target = f64::from(pushes).sqrt() * C_PUSHES_SPEED + e_eff.sqrt() * vn0;
            let mut vn = vn0;
            while vn > 0.0 && iter < 100 {
                iter += 1;
                self.push(s, f);
                self.calc_rel(s);
                vn = -ndot(&s.n, &s.rel);
                self.calc_push_direction(s);
            }
            target += vn;
            s.dir = mn;
            if target > 0.0 && iter != 100 {
                // restitution
                self.push(s, 1.0);
                self.calc_rel(s);
                let d = ndot(&s.n, &s.rel) + vn;
                let k = if d.abs() <= C_DIFF_EPS { 0.0 } else { target / d };
                self.undo_push(s);
                self.push(s, k);
            }
            let dir = s.dir;
            self.ensure_min_separation(s, dir, false);
        } else {
            self.ensure_min_separation(s, mn, true);
            if pushes > 10 {
                s.allow_delaying = false;
            }
        }
        // the anomaly limits (the spin clip is never set in Ballance)
        for i in 0..2 {
            let a = self.env.inv_delta_psi * self.env.max_angular_velocity_per_psi as f64;
            if dotd(&s.rot[i], &s.rot[i]) > a * a {
                let mut v = s.rot[i];
                self.max_angular_velocity_exceeded(&mut v);
                s.rot[i] = v;
            }
            if dotd(&s.speed[i], &s.speed[i]) > f64::from(self.env.max_velocity) * f64::from(self.env.max_velocity) {
                let mut v = s.speed[i];
                self.max_velocity_exceeded(&mut v);
                s.speed[i] = v;
            }
        }
        self.apply_result(s, pushed);
    }

    /// The long-term impact of a point: the solver's setup from the point's cores and normal, with the friction cone
    /// (`impact_long_term`, 0x10024140).
    fn impact_long_term(&mut self, cp: usize, pushed: &mut [Option<usize>; 2], rescue: f32) {
        let t: ContactInfo = self.coll.cps[cp].info.clone();
        let mu = self.coll.cps[cp].mu;
        let (o0, o1) = (self.coll.cps[cp].syn[0].obj, self.coll.cps[cp].syn[1].obj);
        let mut s = Solver {
            rescue: 0.0,
            m0: 0.0,
            m1: 0.0,
            allow_delaying: true,
            rot: [[0.0; 3]; 2],
            speed: [[0.0; 3]; 2],
            dw: [[0.0; 3]; 2],
            dv: [[0.0; 3]; 2],
            rel: [0.0; 3],
            dir: [0.0; 3],
            dir2: [0.0; 3],
            core: [0, 0],
            p_cs: [[0.0; 3]; 2],
            e: 0.0,
            cos_: 0.0,
            sin_: 0.0,
            n: [0.0; 3],
        };
        if t.core[1].is_none() {
            // object 1 is fixed: the movable core becomes core 1
            s.core = [o1, t.core[0].unwrap_or(o0)];
            s.p_cs = [t.cp_cs[1], t.cp_cs[0]];
            s.n = [-t.n[0], -t.n[1], -t.n[2]];
        } else {
            s.core = [t.core[0].unwrap_or(o0), t.core[1].unwrap_or(o1)];
            s.p_cs = [t.cp_cs[0], t.cp_cs[1]];
            s.n = t.n;
        }
        s.e = t.elasticity;
        let (c, sn) = cos_sin_for_impact(mu, s.e);
        s.cos_ = c;
        s.sin_ = sn;
        let mut rel = t.rel;
        self.impact_solve(&mut s, pushed, true, t.pushes, rescue, &mut rel);
        self.coll.cps[cp].info.rel = rel;
    }

    /// The impact pair of the impact system: the impacting cores in a system, and the pair's points that pushed.
    fn add_pushed_core(&mut self, sys: &mut ImpactSys, c: usize, exclude: (usize, usize)) {
        sys.pushed.push(c);
        self.cores[c].sync_pushed = true;
        let fs = sys.fs;
        let n = self.coll.fss[fs].pairs.len();
        for i in (0..n).rev() {
            let Some(q) = self.coll.fss[fs].pairs.get(i).cloned() else { continue };
            let key = (q.c[0].min(q.c[1]), q.c[0].max(q.c[1]));
            if (q.c[0] == c || q.c[1] == c) && key != exclude && !sys.pairs.contains(&key) && self.fs_recalc_pair_key(fs, key) > 0 {
                sys.pairs.push(key);
            }
        }
    }

    /// Recomputes a pair's points by its cores, dropping the ones that left their feature (`0x1001cc20`).
    fn fs_recalc_pair_key(&mut self, fs: usize, key: (usize, usize)) -> usize {
        let Some(i) = self.fs_find_pair(fs, key.0, key.1) else { return 0 };
        self.fs_recalc_pair(fs, i)
    }

    /// The prediction of a point's gap at the next PSI (`calc_prediction`, 0x100249b0).
    fn calc_prediction(&mut self, cp: usize) {
        let gap = self.coll.cps[cp].gap;
        {
            let t = &mut self.coll.cps[cp].info;
            t.flags = (t.flags & !0xff) | 1;
        }
        if gap > SET_MAX_DIST_FOR_IMPACT_SYSTEM {
            self.coll.cps[cp].info.pred_gap = C_FAR_GAP;
            return;
        }
        let rescue = self.cp_rescue_speed(cp);
        self.coll.cps[cp].info.rescue = rescue;
        let t = self.coll.cps[cp].info.clone();
        let mut vc = 0.0f64;
        if let Some(c) = t.core[0] {
            vc = dotd(&self.cores[c].rot_speed, &t.cross_cs[0]) + dotd(&self.cores[c].speed, &t.n);
        }
        if let Some(c) = t.core[1] {
            vc = vc - (dotd(&self.cores[c].rot_speed, &t.cross_cs[1]) + dotd(&self.cores[c].speed, &t.n));
        }
        self.coll.cps[cp].info.pred_gap = (f64::from(gap) - (vc + f64::from(rescue * 0.5)) * self.env.delta_psi) as f32;
    }

    fn fixed_or_tu(&self, c: usize) -> bool {
        self.cores[c].unmovable || self.cores[c].temporarily_unmovable != 0
    }

    /// The point that would be deepest below the collision distance at the next PSI is impacted (`next_impact`,
    /// 0x10024560). Returns 1 when an impact was made.
    fn next_impact(&mut self, sys: &mut ImpactSys) -> i32 {
        let mut best = f64::from(crate::event::COLL_DIST);
        let mut best_cp: Option<usize> = None;
        let mut best_pair: Option<(usize, usize)> = None;
        let fs = sys.fs;
        for i in (0..sys.pairs.len()).rev() {
            let key = sys.pairs[i];
            if self.fixed_or_tu(key.0) && self.fixed_or_tu(key.1) {
                continue;
            }
            let Some(pi) = self.fs_find_pair(fs, key.0, key.1) else { continue };
            let cps = self.coll.fss[fs].pairs[pi].cps.clone();
            for &cp in cps.iter().rev() {
                if (self.coll.cps[cp].info.flags & 0xff) != 1 {
                    self.calc_prediction(cp);
                }
                let pg = self.coll.cps[cp].info.pred_gap;
                if f64::from(pg) < best {
                    best = f64::from(pg);
                    best_cp = Some(cp);
                    best_pair = Some(key);
                }
            }
        }
        let (Some(cp), Some(pair)) = (best_cp, best_pair) else { return 0 };
        let t = self.coll.cps[cp].info.clone();
        for c in [t.core[1], t.core[0]].into_iter().flatten() {
            if self.cores[c].synced {
                continue;
            }
            sys.synced.push(c);
            if self.cores[c].movement_state < crate::core::MT_NOT_SIM {
                self.core_sync(c);
            }
        }
        // 16 bits as the upstream's, which wrap (a counter, not a size)
        self.coll.cps[cp].info.pushes = self.coll.cps[cp].info.pushes.wrapping_add(1);
        let rescue = self.coll.cps[cp].info.rescue;
        let mut pc: [Option<usize>; 2] = [None, None];
        self.impact_long_term(cp, &mut pc, rescue);
        let ps = [pc[1], pc[0]];
        for c in ps.into_iter().flatten() {
            if !self.movable(c) {
                continue;
            }
            if !self.cores[c].sync_pushed {
                self.add_pushed_core(sys, c, pair);
            } else {
                self.reset_processed(sys, c);
            }
        }
        1
    }

    /// The core's points are predicted again (`reset_processed`, 0x10024700).
    fn reset_processed(&mut self, sys: &ImpactSys, c: usize) {
        let Some(fi) = self.core_fi(c, sys.fs) else { return };
        for cp in self.coll.fis[fi].cps.clone() {
            self.coll.cps[cp].info.flags &= !0xff;
        }
    }

    /// The other points of a pair are updated and dropped when they left their feature (`recalc_other_contacts`,
    /// 0x100242c0).
    fn recalc_other_contacts(&mut self, sys: &ImpactSys, pair: (usize, usize), skip: usize) {
        let Some(pi) = self.fs_find_pair(sys.fs, pair.0, pair.1) else { return };
        let cps = self.coll.fss[sys.fs].pairs[pi].cps.clone();
        for &cp in cps.iter().rev() {
            if cp == skip {
                continue;
            }
            self.cp_update(cp);
            self.cp_read_materials(cp);
            if crate::friction::ci_left(&self.coll.cps[cp].info) {
                self.fs_remove_cp(sys.fs, cp);
            }
        }
    }

    /// The synced cores that did not push are restored; the pushed ones get their next PSI from now (`finish`,
    /// 0x10024aa0).
    fn impact_finish(&mut self, sys: &mut ImpactSys) {
        for c in sys.synced.clone() {
            if self.cores[c].synced && !self.cores[c].sync_pushed {
                self.core_restore(c);
            }
            self.cores[c].synced = false;
        }
        let dt = self.env.time_of_next_psi - self.env.current_time;
        let idt = if dt <= 1e-10 { 9_999_999_866.485_682 } else { 1.0 / dt };
        for c in sys.pushed.clone() {
            if !self.movable(c) {
                continue;
            }
            self.calc_next_psi_matrix(c, dt, idt);
            self.cores[c].synced = false;
            self.cores[c].sync_pushed = false;
            self.reset_processed(sys, c);
        }
        self.hull_phase();
        for c in sys.pushed.clone() {
            if !self.movable(c) {
                continue;
            }
            self.cores[c].impact_stamp = self.coll.impact_counter;
            self.object_recalc_after_impact(c);
        }
    }

    /// The impact system of a pair: its cores push, their pairs' other points are predicted and impacted in turn
    /// (`impact_system`, 0x10024320). Returns false when the mindist must go.
    fn impact_system(&mut self, md: Option<usize>, fs: usize, pair: (usize, usize), cp: usize) -> bool {
        let mut sys = ImpactSys {
            fs,
            iter: 0,
            pushed: Vec::new(),
            synced: Vec::new(),
            pairs: Vec::new(),
        };
        let (ca, cb) = (self.coll.cps[cp].syn[0].obj, self.coll.cps[cp].syn[1].obj);
        if self.movable(ca) {
            self.add_pushed_core(&mut sys, ca, pair);
        }
        if self.movable(pair.0) {
            sys.synced.push(pair.0);
        }
        if self.movable(cb) {
            self.add_pushed_core(&mut sys, cb, pair);
        }
        if self.movable(pair.1) {
            sys.synced.push(pair.1);
        }
        sys.pairs.push(pair);
        self.recalc_other_contacts(&sys, pair, cp);
        let mut alive = true;
        while self.next_impact(&mut sys) == 1 {
            sys.iter += 1;
            if sys.iter > C_MAX_IMPACT_ROUNDS {
                alive = false;
                break;
            }
        }
        self.impact_finish(&mut sys);
        if !alive {
            if let Some(m) = md {
                self.mindist_delete(m);
            }
        }
        alive
    }

    /// The impact of a mindist's two objects (`impact_two_objects`, 0x10023cd0): the point is generated, the
    /// impact system runs, and the post-collision event is recorded for the glue.
    fn impact_two_objects(&mut self, m: usize) {
        let md: Mindist = self.coll.mds[m].clone();
        let (o0, o1) = (md.syn[0].obj, md.syn[1].obj);
        let keep = if i32::from(self.cores[o0].movement_state) >= 8 { self.cores[o0].unit } else { self.cores[o1].unit };
        let (cp, _created, fs) = self.try_generate_friction(&md, Some(keep), true);
        let rescue = self.cp_rescue_speed(cp);
        let mut pushed: [Option<usize>; 2] = [None, None];
        self.impact_long_term(cp, &mut pushed, rescue);
        let mut saved = self.coll.cps[cp].info.rel;
        if self.cores[o1].unmovable {
            saved = [-saved[0], -saved[1], -saved[2]];
        }
        let key = (o0.min(o1), o0.max(o1));
        let pi = self.fs_find_pair(fs, key.0, key.1);
        let t_prev = pi.map_or(-1000.0, |i| self.coll.fss[fs].pairs[i].last_impact);
        if let Some(i) = pi {
            self.coll.fss[fs].pairs[i].last_impact = self.env.current_time;
        }
        let pair = (o0, o1);
        let _alive = self.impact_system(Some(m), fs, pair, cp);
        self.coll.cps[cp].info.rel = saved;
        let t = self.coll.cps[cp].info.clone();
        let dt_last = (self.env.current_time - t_prev) as f32;
        let mut ev = ImpactEvent {
            obj: [t.obj[0], t.obj[1]],
            order: [o0, o1],
            rel: saved,
            normal: t.n,
            pos: t.cp_ws,
            dt_last,
        };
        self.notify_impact(&mut ev);
        // a listener may have negated the normal in place: the contact keeps that
        self.coll.cps[cp].info.n = ev.normal;
        self.coll.impact_events.push(ev);
    }

    /// The mindist's do_impact (`phys_contact_do_impact`, 0x100240a0): the objects come to the current time and are
    /// woken, the impact is made, and the synced cores' flags end.
    pub(crate) fn do_impact_mindist(&mut self, m: usize) {
        let md = self.coll.mds[m].clone();
        for k in 0..2 {
            self.ensure_in_simulation(md.syn[k].obj);
        }
        for k in 0..2 {
            let c = md.syn[k].obj;
            if i32::from(self.cores[c].movement_state) < crate::core::MT_NOT_SIM as i32 {
                self.core_sync(c);
            }
        }
        self.coll.impact_counter += 1;
        self.coll.impacts += 1;
        self.impact_two_objects(m);
        for k in 0..2 {
            let c = md.syn[k].obj;
            self.cores[c].synced = false;
            self.cores[c].sync_pushed = false;
        }
    }

    /// After an impact, a core's exact mindists to the cores not stamped by the same impact recompute their events
    /// (`ivp_object_recalc_after_impact`, 0x10009610).
    pub(crate) fn object_recalc_after_impact(&mut self, obj: usize) {
        let mut cur = self.coll.objs.get(obj).and_then(|o| o.exact_syn);
        while let Some(s) = cur {
            let (m, k) = (s / 2, s % 2);
            cur = self.coll.mds[m].syn[k].next;
            let (a, b) = (self.coll.mds[m].syn[0].obj, self.coll.mds[m].syn[1].obj);
            if self.cores[a].impact_stamp == self.cores[b].impact_stamp {
                continue;
            }
            self.recalc(m, 20, 2);
            if (self.coll.mds[m].flags & crate::mindist::MDF_RECALC) == 0 {
                self.update_events(m, false, 2);
            }
        }
    }

    /// The inter-penetration recovery of a failed solve (`phys_contact_inter_penetration`, 0x1002f8f0): a penetrating
    /// pair is pushed apart at 2 |g| dPSI, against a fixed side along its face normal, else along the line of centres.
    pub(crate) fn inter_penetration(&mut self, m: usize) {
        let md = self.coll.mds[m].clone();
        let (a, b) = (md.syn[0].obj, md.syn[1].obj);
        let g = self.gravity_vector();
        let push = (2.0 * (f64::from(g[0]) * f64::from(g[0]) + f64::from(g[1]) * f64::from(g[1]) + f64::from(g[2]) * f64::from(g[2])).sqrt() * self.env.delta_psi) as f32;
        let fa = self.cores[a].unmovable;
        let fb = self.cores[b].unmovable;
        if fa != fb {
            let (fs_k, mov) = if fa { (0usize, b) } else { (1usize, a) };
            let fsyn = md.syn[fs_k];
            if fsyn.status < ST_BALL {
                // the plane of the fixed side's triangles farthest out from the moving core
                let Some(feat) = fsyn.feat else { return };
                let obj = fsyn.obj;
                let c = self.obj_cache(obj);
                let cp_c = self.cores[mov].m_world_f_core.vv;
                let d = [cp_c[0] - c.vv[0], cp_c[1] - c.vv[1], cp_c[2] - c.vv[2]];
                let x = math::rtmul(&c.r, &d);
                let Some(surface) = self.coll.objs[obj].surface.as_ref() else { return };
                let Some(l) = surface.ledges.get(feat.ledge) else { return };
                let mut best = -1e101;
                let mut nrm = [1.0, 0.0, 0.0];
                for t in 0..l.tris.len() {
                    let e = crate::ledge::Edge { tri: t, slot: 0 };
                    let mut mv = crate::contact::tri_normal(l, e);
                    if !math::normalize_d(&mut mv) {
                        continue;
                    }
                    let q0 = l.p(e);
                    let dd = (x[0] - f64::from(q0[0])) * mv[0] + (x[1] - f64::from(q0[1])) * mv[1] + (x[2] - f64::from(q0[2])) * mv[2];
                    if dd > best {
                        best = dd;
                        nrm = mv;
                    }
                }
                let nw = math::rmul(&c.r, &nrm);
                if !self.cores[mov].unmovable && self.cores[mov].movement_state < crate::core::MT_STATIC {
                    let core = &mut self.cores[mov];
                    for k in 0..3 {
                        core.speed_change[k] += (nw[k] * f64::from(push)) as f32;
                    }
                    self.ensure_in_simulation(mov);
                }
                return;
            }
        }
        // the line of centres: both movable objects are pushed apart
        let pa = self.cores[a].m_world_f_core.vv;
        let pb = self.cores[b].m_world_f_core.vv;
        let mut dir = [(pb[0] - pa[0]) as f32, (pb[1] - pa[1]) as f32, (pb[2] - pa[2]) as f32];
        if math::normalize_f(&mut dir) == 0.0 {
            return;
        }
        for (k, &o) in [a, b].iter().enumerate() {
            if self.cores[o].unmovable || i32::from(self.cores[o].movement_state) >= 8 {
                continue;
            }
            let sg = if k == 1 { 1.0f32 } else { -1.0f32 };
            let imp = v3scale(dir, sg * self.cores[o].mass * push);
            let pa_f = [pa[0], pa[1], pa[2]];
            self.async_push_ws(o, pa_f, imp);
        }
    }

    /// An asynchronous push of a core in world space at a world point (`ivp_body_async_push_ws`, 0x1000a3e0).
    fn async_push_ws(&mut self, c: usize, p_ws: [f64; 3], imp_ws: P3) {
        let core = &self.cores[c];
        let p_cs_d = math::rtmul(&core.m_world_f_core.r, &[p_ws[0] - core.m_world_f_core.vv[0], p_ws[1] - core.m_world_f_core.vv[1], p_ws[2] - core.m_world_f_core.vv[2]]);
        let p_cs = [p_cs_d[0] as f32, p_cs_d[1] as f32, p_cs_d[2] as f32];
        let imp_cs_d = math::rtmul(&core.m_world_f_core.r, &[f64::from(imp_ws[0]), f64::from(imp_ws[1]), f64::from(imp_ws[2])]);
        let imp_cs = [imp_cs_d[0] as f32, imp_cs_d[1] as f32, imp_cs_d[2] as f32];
        self.async_push_core(c, p_cs, imp_cs, imp_ws);
    }
}

/// The impact system's state (`ImpactSystem`, IM8): the environment's friction system, the iteration count, the
/// pushed and synced cores, and the pairs in play (by their cores).
struct ImpactSys {
    fs: usize,
    iter: u32,
    pushed: Vec<usize>,
    synced: Vec<usize>,
    pairs: Vec<(usize, usize)>,
}

/// The cosine and sine of the cone for an impact (`cos_sin_for_impact`, 0x10023ff0): tan(theta) = mu (1 + sqrt(e)),
/// the cosine by its truncated Taylor series.
fn cos_sin_for_impact(mu: f32, e: f32) -> (f32, f32) {
    let x = (f64::from(e).sqrt() + 1.0) * f64::from(mu);
    let th = x.atan2(1.0);
    let t2 = th * th;
    let cc = t2 * t2 * f64::from(C_TAYLOR4) + (f64::from(1.0f32) - t2 * 0.5);
    (cc as f32, (cc * x) as f32)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::collision::ObjKind;
    use crate::convex;
    use crate::ledge::Surface;

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

    /// Two cubes touching at a 0.01 gap, the upper one falling onto the lower one at speed 2.
    fn falling_pair() -> World {
        let mut w = World::new();
        let q = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
        for (i, z) in [0.0, 1.01].into_iter().enumerate() {
            let c = w.core_init(i, q, [0.0, 0.0, z], false);
            w.cores[c].upper_limit_radius = 0.866;
            w.cores[c].movement_state = crate::core::MT_MOVING;
            w.set_object_geometry(c, ObjKind::Polygon, Some(unit_cube()), 0.0);
            w.set_object_material(c, 0.8, 0.2);
        }
        w.cores[1].speed = [0.0, 0.0, -2.0];
        w
    }

    /// Found by fuzzing a level played: the per-PSI impact count (16 bits, as the upstream's) overflowed when a
    /// map's bodies made more than 32767 impacts in one PSI; it wraps, as the upstream's does.
    #[test]
    fn the_impact_count_wraps() {
        let mut w = falling_pair();
        w.enable_collision_detection(0);
        w.enable_collision_detection(1);
        let m = (0..w.coll.mds.len()).find(|&m| w.coll.mds[m].alive).expect("a mindist");
        w.cores[0].impacts_psi = i16::MAX;
        w.cores[1].impacts_psi = i16::MAX;
        w.do_impact_mindist(m);
        assert!(w.cores.iter().take(2).any(|c| c.impacts_psi < 0));
    }

    #[test]
    fn an_impact_stops_the_approach_of_a_falling_cube() {
        let mut w = falling_pair();
        w.enable_collision_detection(0);
        w.enable_collision_detection(1);
        let m = (0..w.coll.mds.len()).find(|&m| w.coll.mds[m].alive).expect("a mindist");
        // the gap has closed to the impact: make the impact directly, as the event would
        w.do_impact_mindist(m);
        for c in 0..2 { eprintln!("IMP core {c} speed {:?} sc {:?} rot {:?} rc {:?} impacts_psi {}", w.cores[c].speed, w.cores[c].speed_change, w.cores[c].rot_speed, w.cores[c].rot_speed_change, w.cores[c].impacts_psi); }
        // The contact is a corner of the falling cube on the other's face (the closest feature), so the impact also
        // spins the cubes and their centres need not share the speed. What must hold: the momentum of the pair (-2)
        // is kept, and the translational energy is lost in the inelastic contact (the delayed response of the heavier
        // side is pending, so its change counts).
        let (v0, v1) = (w.cores[0].speed[2] + w.cores[0].speed_change[2], w.cores[1].speed[2] + w.cores[1].speed_change[2]);
        assert!((v0 + v1 + 2.0).abs() < 0.05, "momentum after impact {v0} {v1}");
        let energy_after = 0.5 * (v0 * v0 + v1 * v1);
        assert!(energy_after < 2.0 - 0.1, "kinetic energy after {energy_after} from 2.0");
        assert_eq!(w.coll.impact_events.len(), 1, "one post-collision event");
    }
}
