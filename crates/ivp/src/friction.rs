// Port of OpenBallance src/phys/phys_friction.c and the solve of phys_fsolver.c (ivp_fs_solve) (GPL-3.0; see the LICENSE
// of the repository): the friction systems (docs/ivp_contact.md 4, "FS"): the contact points of a connected group of
// touching cores, grouped by core pair, with their three controllers (2000: contact update and energy easing; 600: the
// tangential friction spring; 0: the normal force, the contact removal and the split), the creation of points from
// mindists, the merge and split of systems by union-find, and the revive cascade.
//
// Systems, pairs and friction infos live in arenas and refer to each other by index. A system's contact list is in the
// upstream's order, the head first; a movable core's friction info is one, a fixed core's are one per system.

use crate::contact::SET_FRICTION_DIST;
use crate::core::{Controller, FrictionKind, World};
use crate::event_sim::EventSim;
use crate::fsolver::{core_reaction_init, core_reaction_push2, gauss_solve, lcp_solve};
use crate::math;
use crate::mindist::Mindist;

type P3 = [f32; 3];

/// Thresholds of the friction code (ivp_const.h, FS2 and the mindist settings).
pub const SET_MAX_DIST_FOR_FRICTION: f32 = f32::from_bits(0x3d38_51eb);
const C_1E6F: f64 = 9.999_999_974_752_427e-7;
const C_1E38: f64 = 1e-38;
const C_001F: f64 = 0.001_000_000_047_497_451_3;
pub(crate) const C_01F: f64 = 0.100_000_001_490_116_12;
const C_1E19: f64 = 1e-19;
const C_1E5F: f64 = 9.999_999_747_378_752e-6;
const C_981F: f32 = 9.81;
/// Systems with more contacts than this are not solved this PSI (IVP_C_MAX_CONTACTS).
const MAX_CONTACTS: usize = 150;

/// A pair of cores' contact points within a system (`IvpFrictionPair`, 2.8).
#[derive(Clone, Debug, Default)]
pub struct Pair {
    /// The pair's points, head first.
    pub cps: Vec<usize>,
    pub next_ease: i32,
    pub last_impact: f64,
    pub anti_energy: f32,
    pub c: [usize; 2],
}

/// A friction system (`IvpFrictionSystem`, 2.8): its contacts, pairs and cores, and its three controllers.
#[derive(Clone, Debug, Default)]
pub struct FrictionSystem {
    pub alive: bool,
    /// The contacts, head first (`first`).
    pub cps: Vec<usize>,
    pub pairs: Vec<Pair>,
    /// All cores, fixed ones included, in the upstream's vector order.
    pub cores: Vec<usize>,
    pub movable: Vec<usize>,
    pub n_cores: i32,
    pub n_contacts: i32,
    pub uf_needed: bool,
    /// The controllers (normal, spring, update) in the world's controller arena.
    pub ctrl: [usize; 3],
}

/// The friction info of a core in a system (`IvpFrictionInfo`): its contacts and its system.
#[derive(Clone, Debug, Default)]
pub struct FrictionInfo {
    pub alive: bool,
    pub cps: Vec<usize>,
    pub fs: usize,
}

/// The relative motion of two cores in an energy easing (`RelMotion`, FS6.2).
struct Rel {
    dv: P3,
    ax0: P3,
    ax1: P3,
    vlen: f64,
    im0: f64,
    im1: f64,
    wlen: f64,
    iir0: f64,
    iir1: f64,
    er: f64,
    el: f64,
    etot: f64,
    c0: usize,
    c1: usize,
}

fn dotd(a: &P3, b: &P3) -> f64 {
    (f64::from(a[2]) * f64::from(b[2]) + f64::from(a[1]) * f64::from(b[1])) + f64::from(a[0]) * f64::from(b[0])
}

fn dotf(a: &P3, b: &P3) -> f32 {
    dotd(a, b) as f32
}

fn v3scale(a: &P3, s: f32) -> P3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// The rotation of a vector by a pose in f32 out (`ivp_rmul_f`), and its transpose (`ivp_rtmul_f`).
fn rmul_f(r: &[[f64; 3]; 3], v: &P3) -> P3 {
    let o = math::rmul(r, &[f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]);
    [o[0] as f32, o[1] as f32, o[2] as f32]
}

fn rtmul_f(r: &[[f64; 3]; 3], v: &P3) -> P3 {
    let o = math::rtmul(r, &[f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]);
    [o[0] as f32, o[1] as f32, o[2] as f32]
}

/// The f32 vector normalized in place, returning the length it had in f64 (`fast_normize`, 0x1000df30).
fn fast_normize(v: &mut P3) -> f64 {
    let n = dotd(v, v);
    if n < C_1E19 {
        return 0.0;
    }
    let y = 1.0 / n.sqrt();
    for c in v.iter_mut() {
        *c = (f64::from(*c) * y) as f32;
    }
    n * y
}

fn len3(v: &P3) -> f64 {
    dotd(v, v).sqrt()
}

/// The kinetic energy of a core with the given speeds (`ivp_core_energy`, 0x1000c480).
pub fn core_energy(mass: f32, rot_inertia: &P3, v: &P3, w: &P3) -> f64 {
    0.5 * (f64::from(mass) * dotd(v, v)
        + f64::from(rot_inertia[0]) * f64::from(w[0]) * f64::from(w[0])
        + f64::from(rot_inertia[1]) * f64::from(w[1]) * f64::from(w[1])
        + f64::from(rot_inertia[2]) * f64::from(w[2]) * f64::from(w[2]))
}

/// The relative energy of a pair of masses for the relative speed `v` (`rel_energy`, 0x1001cd60).
fn rel_energy(v: f64, m0: f64, m1: f64, im0: f64, im1: f64) -> f64 {
    let a = v / (im0 + im1);
    let u = v - a * im1;
    let big_u = u * u * m1;
    let w = a * im0;
    ((v * v * m1 - w * w * m0) - big_u) * 0.5
}

/// The impulse that removes energy `e` from relative speed `v` (`rel_impulse`, 0x1001cd20).
fn rel_impulse(v: f64, i0: f64, i1: f64, e: f64) -> f64 {
    (v - (v * v - 2.0 * e * (i0 + i1)).abs().sqrt()) / (i0 + i1)
}

impl World {
    // ---- friction infos (2.8) ----

    /// The friction info of core `obj` in system `fs` (`ivp_core_fi`, 0x1000d960): one for a fixed core, the movable
    /// core's own when it is in the system.
    pub(crate) fn core_fi(&self, obj: usize, fs: usize) -> Option<usize> {
        let o = self.coll.objs.get(obj)?;
        if self.cores[obj].unmovable {
            o.fixed_fis.iter().copied().find(|&fi| self.coll.fis[fi].fs == fs)
        } else {
            o.fi.filter(|&fi| self.coll.fis[fi].fs == fs)
        }
    }

    fn fi_new(&mut self, fs: usize) -> usize {
        let fi = FrictionInfo {
            alive: true,
            cps: Vec::new(),
            fs,
        };
        match self.coll.free_fis.pop() {
            Some(i) => {
                self.coll.fis[i] = fi;
                i
            }
            None => {
                self.coll.fis.push(fi);
                self.coll.fis.len() - 1
            }
        }
    }

    /// Attaches a friction info to a core: a fixed core keeps a list, a movable one a single slot (`0x1000d9b0`).
    fn fi_insert(&mut self, obj: usize, fi: usize) {
        self.ensure_object(obj);
        if self.cores[obj].unmovable {
            self.coll.objs[obj].fixed_fis.push(fi);
        } else {
            self.coll.objs[obj].fi = Some(fi);
        }
    }

    /// Detaches a friction info from a core (`0x1000da50`).
    fn fi_remove(&mut self, obj: usize, fi: usize) {
        if self.cores[obj].unmovable {
            self.coll.objs[obj].fixed_fis.retain(|&x| x != fi);
        } else if self.coll.objs[obj].fi == Some(fi) {
            self.coll.objs[obj].fi = None;
        }
    }

    /// Detaches and frees a friction info (`0x1000da80`).
    fn fi_delete(&mut self, obj: usize, fi: usize) {
        self.fi_remove(obj, fi);
        self.coll.fis[fi].alive = false;
        self.coll.fis[fi].cps.clear();
        self.coll.free_fis.push(fi);
    }

    // ---- the system (2.8) ----

    /// A new friction system with its three controllers (`0x1000b000`), reusing a freed slot and its controllers.
    fn fs_new(&mut self) -> usize {
        match self.coll.free_fss.pop() {
            Some(i) => {
                let ctrl = self.coll.fss[i].ctrl;
                self.coll.fss[i] = FrictionSystem {
                    alive: true,
                    ctrl,
                    ..FrictionSystem::default()
                };
                self.set_fs_controllers(i);
                i
            }
            None => {
                let mut fs = FrictionSystem {
                    alive: true,
                    ..FrictionSystem::default()
                };
                fs.ctrl = [
                    self.add_controller(Controller::Friction { fs: 0, kind: FrictionKind::Normal }),
                    self.add_controller(Controller::Friction { fs: 0, kind: FrictionKind::Spring }),
                    self.add_controller(Controller::Friction { fs: 0, kind: FrictionKind::Update }),
                ];
                self.coll.fss.push(fs);
                let i = self.coll.fss.len() - 1;
                self.set_fs_controllers(i);
                i
            }
        }
    }

    /// Points the three controller records of a system at it.
    fn set_fs_controllers(&mut self, fs: usize) {
        let ctrl = self.coll.fss[fs].ctrl;
        let kinds = [FrictionKind::Normal, FrictionKind::Spring, FrictionKind::Update];
        for k in 0..3 {
            self.controllers[ctrl[k]] = Controller::Friction { fs, kind: kinds[k] };
        }
    }

    /// Deletes a system: its pairs go, its controllers stay with the slot (`0x1000b100`).
    fn fs_delete(&mut self, fs: usize) {
        self.coll.fss[fs].alive = false;
        self.coll.fss[fs].pairs.clear();
        self.coll.fss[fs].cps.clear();
        self.coll.fss[fs].cores.clear();
        self.coll.fss[fs].movable.clear();
        self.coll.free_fss.push(fs);
    }

    /// A movable core gets the three controllers in its unit; every core joins the system (`0x1000b390`).
    fn fs_add_core(&mut self, fs: usize, obj: usize) {
        self.coll.fss[fs].cores.push(obj);
        if !self.cores[obj].unmovable {
            self.coll.fss[fs].movable.push(obj);
            let ctrl = self.coll.fss[fs].ctrl;
            self.core_add_controller(obj, ctrl[0]);
            self.core_add_controller(obj, ctrl[1]);
            self.core_add_controller(obj, ctrl[2]);
        }
        self.coll.fss[fs].n_cores += 1;
    }

    /// Removes a core from the system and its controllers (`0x1000b410`).
    fn fs_remove_core(&mut self, fs: usize, obj: usize) {
        remove_value(&mut self.coll.fss[fs].cores, obj);
        if !self.cores[obj].unmovable {
            remove_value(&mut self.coll.fss[fs].movable, obj);
            let ctrl = self.coll.fss[fs].ctrl;
            self.core_remove_controller(obj, ctrl[2]);
            self.core_remove_controller(obj, ctrl[1]);
            self.core_remove_controller(obj, ctrl[0]);
        }
        self.coll.fss[fs].n_cores -= 1;
    }

    /// A point at the head of the system's list (`0x1000b360`).
    fn fs_push_front(&mut self, fs: usize, cp: usize) {
        self.coll.cps[cp].fs = Some(fs);
        self.coll.fss[fs].cps.insert(0, cp);
        self.coll.fss[fs].n_contacts += 1;
    }

    /// Takes a point out of the system's list; its `fs` is kept (`0x1000b290`).
    fn fs_unlink(&mut self, fs: usize, cp: usize) {
        remove_value(&mut self.coll.fss[fs].cps, cp);
        self.coll.fss[fs].n_contacts -= 1;
    }

    /// The pair of two cores in either order (`0x1001d380`), searched from the end.
    pub(crate) fn fs_find_pair(&self, fs: usize, a: usize, b: usize) -> Option<usize> {
        let pairs = &self.coll.fss[fs].pairs;
        (0..pairs.len()).rev().find(|&i| {
            let p = &pairs[i];
            (p.c[0] == a && p.c[1] == b) || (p.c[0] == b && p.c[1] == a)
        })
    }

    /// Adds a point to its core pair, creating the pair (`0x1000b2c0`).
    fn fs_add_pair(&mut self, fs: usize, cp: usize) {
        let (a, b) = (self.coll.cps[cp].syn[0].obj, self.coll.cps[cp].syn[1].obj);
        let idx = match self.fs_find_pair(fs, a, b) {
            Some(i) => i,
            None => {
                self.coll.fss[fs].pairs.push(Pair {
                    cps: Vec::new(),
                    next_ease: 1,
                    last_impact: -1000.0,
                    anti_energy: 0.0,
                    c: [a, b],
                });
                self.coll.fss[fs].pairs.len() - 1
            }
        };
        self.coll.fss[fs].pairs[idx].cps.push(cp);
    }

    /// Takes a point out of its pair; true when the pair died (`0x1000b210`).
    fn fs_pair_remove(&mut self, fs: usize, cp: usize) -> bool {
        let (a, b) = (self.coll.cps[cp].syn[0].obj, self.coll.cps[cp].syn[1].obj);
        let Some(i) = self.fs_find_pair(fs, a, b) else { return false };
        remove_value(&mut self.coll.fss[fs].pairs[i].cps, cp);
        if !self.coll.fss[fs].pairs[i].cps.is_empty() {
            return false;
        }
        self.coll.fss[fs].pairs.remove(i);
        true
    }

    /// Removes a point from its system and its cores' infos, and destroys it (`delete_friction_distance`, 0x1001c460).
    pub(crate) fn fs_remove_cp(&mut self, fs: usize, cp: usize) {
        let c = [self.coll.cps[cp].syn[0].obj, self.coll.cps[cp].syn[1].obj];
        self.reset_freeze_check_values(c[0]);
        self.reset_freeze_check_values(c[1]);
        self.fs_unlink(fs, cp);
        if self.fs_pair_remove(fs, cp) {
            self.coll.fss[fs].uf_needed = true;
        }
        for k in 0..2 {
            let Some(fi) = self.core_fi(c[k], fs) else { continue };
            remove_value(&mut self.coll.fis[fi].cps, cp);
            if !self.coll.fis[fi].cps.is_empty() {
                continue;
            }
            self.fi_delete(c[k], fi);
            self.fs_remove_core(fs, c[k]);
            if let Some(u) = self.cores.get(c[k]).map(|core| core.unit) {
                self.units[u].changed = true;
            }
        }
        self.cp_destroy(cp);
    }

    /// Recomputes a pair's points at this time, dropping those that left their feature; returns how many are left
    /// (`ivp_fs_recalc_pair`, 0x1001cc20).
    #[allow(dead_code)] // the impact pre-pass, ported next, calls it
    pub(crate) fn fs_recalc_pair(&mut self, fs: usize, pair: usize) -> usize {
        let mut n = self.coll.fss[fs].pairs[pair].cps.len();
        let mut i = n;
        while i > 0 {
            i -= 1;
            let cp = self.coll.fss[fs].pairs[pair].cps[i];
            self.cp_update(cp);
            self.cp_read_materials(cp);
            if ci_left(&self.coll.cps[cp].info) {
                n -= 1;
                self.fs_remove_cp(fs, cp);
            }
        }
        n
    }

    /// Moves `src`'s points and cores into `dst`, and deletes `src` (`0x1001c570`).
    fn fs_merge(&mut self, dst: usize, src: usize) {
        while let Some(&cp) = self.coll.fss[src].cps.first() {
            self.fs_unlink(src, cp);
            self.fs_pair_remove(src, cp);
            self.fs_push_front(dst, cp);
            self.fs_add_pair(dst, cp);
        }
        let n = self.coll.fss[src].cores.len();
        for i in (0..n).rev() {
            let c = self.coll.fss[src].cores[i];
            let Some(fi_s) = self.core_fi(c, src) else { continue };
            match self.core_fi(c, dst) {
                None => {
                    self.coll.fis[fi_s].fs = dst;
                    self.fs_remove_core(src, c);
                    self.fs_add_core(dst, c);
                }
                Some(fd) => {
                    let moved = std::mem::take(&mut self.coll.fis[fi_s].cps);
                    self.coll.fis[fd].cps.extend(moved);
                    self.fi_delete(c, fi_s);
                    self.fs_remove_core(src, c);
                }
            }
        }
        self.fs_delete(src);
    }

    // ---- creation (CP3.3, IM11) ----

    /// The contact point of a mindist, creating it and its friction system when it is new (`0x10022180`, the
    /// try-to-generate of the manager). Returns (point, created, system). `keep` is the unit a merge must keep.
    pub(crate) fn try_generate_friction(&mut self, md: &Mindist, keep: Option<usize>, update: bool) -> (usize, bool, usize) {
        let (oa, ob) = (md.syn[md.sorted(0)].obj, md.syn[md.sorted(1)].obj);
        let (mut cm, mut co) = (oa, ob);
        if self.cores[cm].unmovable {
            std::mem::swap(&mut cm, &mut co);
        }
        let fi_m = self.coll.objs[cm].fi;
        let (cp, is_new) = self.cp_find_or_create(md);
        if !is_new {
            let fs = match fi_m {
                Some(fi) => self.coll.fis[fi].fs,
                None => self.coll.cps[cp].fs.unwrap_or(0),
            };
            if update {
                self.cp_update(cp);
                self.cp_read_materials(cp);
            }
            return (cp, false, fs);
        }
        if update {
            self.cp_update(cp);
            self.cp_read_materials(cp);
        }
        // "friction created": the listeners hear of the new contact before its system is made
        let (a, b) = (self.coll.cps[cp].syn[0].obj, self.coll.cps[cp].syn[1].obj);
        self.notify_contact(a, b, true);
        let fs;
        let fi_o;
        let fi_m_now;
        if let Some(fim) = fi_m {
            fs = self.coll.fis[fim].fs;
            fi_m_now = fim;
            fi_o = match self.core_fi(co, fs) {
                Some(f) => f,
                None => {
                    let f2 = if self.cores[co].unmovable { None } else { self.coll.objs[co].fi };
                    match f2 {
                        Some(f2) => {
                            self.fs_merge(fs, self.coll.fis[f2].fs);
                            f2
                        }
                        None => {
                            let f = self.fi_new(fs);
                            self.fi_insert(co, f);
                            self.fs_add_core(fs, co);
                            f
                        }
                    }
                }
            };
            self.fs_push_front(fs, cp);
            self.fs_add_pair(fs, cp);
        } else {
            let f_m = self.fi_new(0);
            fi_m_now = f_m;
            let f2 = if self.cores[co].unmovable { None } else { self.coll.objs[co].fi };
            if let Some(f2) = f2 {
                fs = self.coll.fis[f2].fs;
                fi_o = f2;
                self.coll.fis[f_m].fs = fs;
                self.fi_insert(cm, f_m);
                self.fs_add_core(fs, cm);
                self.fs_push_front(fs, cp);
                self.fs_add_pair(fs, cp);
            } else {
                fs = self.fs_new();
                fi_o = self.fi_new(fs);
                self.coll.fis[f_m].fs = fs;
                self.fi_insert(co, fi_o);
                self.fi_insert(cm, f_m);
                self.fs_push_front(fs, cp);
                self.fs_add_pair(fs, cp);
                self.fs_add_core(fs, cm);
                self.fs_add_core(fs, co);
            }
        }
        self.coll.fis[fi_m_now].cps.push(cp);
        self.coll.fis[fi_o].cps.push(cp);
        self.cp_init_constants(cp);
        // two movable cores in different units: one unit absorbs the other
        if !self.cores[cm].unmovable && !self.cores[co].unmovable {
            let (um, uo) = (self.cores[cm].unit, self.cores[co].unit);
            if um != uo {
                if Some(uo) == keep {
                    self.sim_unit_merge(uo, um);
                } else {
                    self.sim_unit_merge(um, uo);
                }
            }
        }
        (cp, true, fs)
    }

    /// Removes every contact of an object; unless `keep_asleep`, its cores are revived first, and an emptied system is
    /// deleted (`0x10009a40`).
    #[allow(dead_code)] // the glue's object deletion calls it once the glue is ported
    pub(crate) fn object_remove_contacts(&mut self, obj: usize, keep_asleep: bool) {
        while let Some(&(cp, _)) = self.coll.objs.get(obj).and_then(|o| o.fric.last()) {
            let Some(fs) = self.coll.cps[cp].fs else {
                self.cp_destroy(cp);
                continue;
            };
            if !keep_asleep {
                let (a, b) = (self.coll.cps[cp].syn[0].obj, self.coll.cps[cp].syn[1].obj);
                self.ensure_in_simulation(a);
                self.ensure_in_simulation(b);
            }
            self.fs_remove_cp(fs, cp);
            if self.coll.fss[fs].n_contacts == 0 {
                self.fs_delete(fs);
            }
        }
    }

    /// When a core revives, the resting movable neighbours that have no friction get one; 1 when one was made
    /// (`ivp_fs_revive_core`, 0x1001d4d0).
    pub(crate) fn fs_revive_core(&mut self, obj: usize) -> i32 {
        let mut cur = self.coll.objs.get(obj).and_then(|o| o.exact_syn);
        while let Some(s) = cur {
            let (m, k) = (s / 2, s % 2);
            let next = self.coll.mds[m].syn[k].next;
            cur = next;
            if self.coll.mds[m].flags & crate::mindist::MDF_FUNCTION != 0 {
                continue;
            }
            let other = self.coll.mds[m].syn[k ^ 1].obj;
            if self.cores[other].unmovable || self.coll.objs[other].fi.is_some() {
                continue;
            }
            self.recalc(m, 20, 2);
            if self.coll.mds[m].flags & crate::mindist::MDF_RECALC != 0 {
                continue;
            }
            if !(self.coll.mds[m].len_numerator < SET_MAX_DIST_FOR_FRICTION) {
                continue;
            }
            let md = self.coll.mds[m].clone();
            let (_, created, _) = self.try_generate_friction(&md, Some(self.cores[obj].unit), true);
            if created {
                self.reset_freeze_check_values(other);
                return 1;
            }
        }
        0
    }

    // ---- the tangential friction (FS4) ----

    /// The tangential friction of a point: drives the relative tangential speed to s / dt within mu N dt; returns the
    /// change of the spring energy (`friction_force_local_constraint_2d`, 0x1001b080).
    fn friction_2d(&mut self, cp: usize, es: &EventSim) -> f32 {
        let (max_i, c0, c1, t, s) = {
            let p = &self.coll.cps[cp];
            let max_i = f64::from(p.mu) * f64::from(p.pressure) * es.delta_time;
            (max_i, p.info.core[0], p.info.core[1], p.info.clone(), p.s)
        };
        if max_i < C_1E6F {
            return 0.0;
        }
        let ra = c0.map(|c| self.cores[c].clone());
        let rb = c1.map(|c| self.cores[c].clone());
        let tcb = core_reaction_init(ra.as_ref(), rb.as_ref(), t.cp_ws, t.span[0], t.span[1]);
        let i_dt = es.i_delta_time;
        let a = f64::from(s[0]) * i_dt - tcb.dv[0] as f64;
        let b = f64::from(s[1]) * i_dt - tcb.dv[1] as f64;
        let det = tcb.m00 * tcb.m11 - tcb.m01 * tcb.m01;
        if det * det < C_1E38 {
            return 0.0;
        }
        let inv = 1.0 / det;
        let c00 = tcb.m11 * inv;
        let c01 = -tcb.m01 * inv;
        let c11 = inv * tcb.m00;
        let mut imp = [(c00 * a + c01 * b) as f32, (c01 * a + c11 * b) as f32];
        let q = f64::from(imp[0]) * f64::from(imp[0]) + f64::from(imp[1]) * f64::from(imp[1]);
        if q > max_i * max_i {
            let k = (1.0 / f64::from(q as f32).sqrt()) * max_i;
            imp[0] = (f64::from(imp[0]) * k) as f32;
            imp[1] = (f64::from(imp[1]) * k) as f32;
        }
        // the speeds are written straight into the cores (0x10033ad0)
        let mut speed = [ra.as_ref().map_or([0.0; 3], |c| c.speed), rb.as_ref().map_or([0.0; 3], |c| c.speed)];
        let mut rot = [ra.as_ref().map_or([0.0; 3], |c| c.rot_speed), rb.as_ref().map_or([0.0; 3], |c| c.rot_speed)];
        let inv_mass = [ra.as_ref().map_or(0.0, |c| f64::from(c.inv_mass)), rb.as_ref().map_or(0.0, |c| f64::from(c.inv_mass))];
        core_reaction_push2(&tcb, [1.0, -1.0], inv_mass, imp, &mut speed, &mut rot);
        if let Some(c) = c0 {
            self.cores[c].speed = speed[0];
            self.cores[c].rot_speed = rot[0];
        }
        if let Some(c) = c1 {
            self.cores[c].speed = speed[1];
            self.cores[c].rot_speed = rot[1];
        }
        let s2 = f64::from(s[0]) * f64::from(s[0]) + f64::from(s[1]) * f64::from(s[1]);
        // q before the clamp (the upstream's quirk)
        let e = (es.delta_time * es.delta_time * q * s2).sqrt() * 0.5;
        let d = (e - f64::from(self.coll.cps[cp].old_energy)) as f32;
        self.coll.cps[cp].old_energy = e as f32;
        d
    }

    /// The slip clamp: a spring beyond `t` slips, destroying energy (`0x1001d750`, 0x1001bc20).
    fn clamp_spring(&mut self, cp: usize, t: f32, th: f64) {
        let p = &mut self.coll.cps[cp];
        let s2 = f64::from(p.s[0]) * f64::from(p.s[0]) + f64::from(p.s[1]) * f64::from(p.s[1]);
        if !(s2 > th) {
            return;
        }
        let r = 1.0 / f64::from(s2 as f32).sqrt();
        p.destroyed = ((r * s2 - f64::from(t)) * f64::from(p.pressure) * f64::from(p.mu) + f64::from(p.destroyed)) as f32;
        p.recheck = 1;
        let k = r * f64::from(t);
        p.s[0] = (f64::from(p.s[0]) * k) as f32;
        p.s[1] = (f64::from(p.s[1]) * k) as f32;
    }

    /// The springs of two coplanar points of a pair equalized along the line joining them (`0x1001bdf0`). Returns the
    /// increments of the two points' spring vectors.
    fn ease_two(&self, a: usize, b: usize, f: f64) -> (P3, P3) {
        let pa = &self.coll.cps[a];
        let pb = &self.coll.cps[b];
        let sg: f32 = if pa.syn[0].obj == pb.syn[0].obj { 1.0 } else { -1.0 };
        let ff = f as f32;
        let (ta, tb) = (&pa.info, &pb.info);
        let mut sa = [0.0f32; 3];
        let mut sbv = [0.0f32; 3];
        let mut u = [0.0f32; 3];
        for k in 0..3 {
            sa[k] = pa.s[0] * ta.span[0][k] + pa.s[1] * ta.span[1][k];
            sbv[k] = pb.s[0] * tb.span[0][k] + pb.s[1] * tb.span[1][k];
            u[k] = (ta.cp_ws[k] - tb.cp_ws[k]) as f32;
        }
        let l2 = dotd(&u, &u);
        if l2 >= C_1E19 {
            let y = 1.0 / l2.sqrt();
            for k in 0..3 {
                u[k] = (f64::from(u[k]) * y) as f32;
            }
        }
        let ua = dotf(&u, &sa);
        let ub = dotf(&u, &sbv) * sg;
        let mut da = [0.0f32; 3];
        let mut db = [0.0f32; 3];
        for k in 0..3 {
            let pa_k = u[k] * ua;
            let pb_k = u[k] * ub;
            let avg = (pa_k + pb_k) * 0.5;
            da[k] = (avg - pa_k) * ff;
            db[k] = (avg - pb_k) * (sg * ff);
        }
        (da, db)
    }

    /// Eases every coplanar pair of a pair's points (`0x1001c070`, the spring easing).
    fn ease_pair(&mut self, fs: usize, pair: usize) {
        let cps = self.coll.fss[fs].pairs[pair].cps.clone();
        let n = cps.len();
        let mut d = vec![[0.0f32; 3]; n];
        let f = 1.0 / (n as f64 + 1e-19);
        for i in 0..n {
            for j in i + 1..n {
                let c = dotd(&self.coll.cps[cps[i]].info.n, &self.coll.cps[cps[j]].info.n);
                if ((c.abs() - 1.0).abs()) < C_001F {
                    let (dj, di) = self.ease_two(cps[j], cps[i], f);
                    for k in 0..3 {
                        d[j][k] += dj[k];
                        d[i][k] += di[k];
                    }
                }
            }
        }
        for k in 0..n {
            let p = &mut self.coll.cps[cps[k]];
            p.s[0] += dotf(&p.info.span[0], &d[k]);
            p.s[1] += dotf(&p.info.span[1], &d[k]);
        }
    }

    /// The tangential spring controller (priority 600, `0x1001d750`).
    pub(crate) fn fs_spring_simulate(&mut self, fs: usize, es: &EventSim) {
        let dt = es.delta_time;
        let n_contacts = self.coll.fss[fs].n_contacts;
        if n_contacts > 1 {
            let npairs = self.coll.fss[fs].pairs.len();
            for i in (0..npairs).rev() {
                let cps = self.coll.fss[fs].pairs[i].cps.clone();
                if cps.is_empty() {
                    continue;
                }
                let mut sum = 0.0f64;
                for &cp in cps.iter().rev() {
                    let p = &self.coll.cps[cp];
                    sum += f64::from(p.pressure) * f64::from(p.mu) * f64::from(p.inv_vm_no_dir);
                }
                let t = (sum * dt * dt) as f32;
                let th = (f64::from(t) * f64::from(t) + f64::from(1e-6f32)) as f32;
                let mut e = 0.0f32;
                for &cp in cps.iter().rev() {
                    self.clamp_spring(cp, t, f64::from(th));
                    e = (f64::from(e) + self.friction_2d(cp, es) as f64) as f32;
                }
                if e > 0.0 {
                    let p = &mut self.coll.fss[fs].pairs[i];
                    p.anti_energy = (f64::from(e) + f64::from(p.anti_energy)) as f32;
                }
            }
            let npairs = self.coll.fss[fs].pairs.len();
            for i in (0..npairs).rev() {
                let p = &mut self.coll.fss[fs].pairs[i];
                p.next_ease -= 1;
                if p.next_ease == 0 {
                    self.ease_pair(fs, i);
                    self.coll.fss[fs].pairs[i].next_ease = 5;
                }
            }
            return;
        }
        let Some(&cp) = self.coll.fss[fs].cps.first() else { return };
        let p = &self.coll.cps[cp];
        let t = (f64::from(p.inv_vm_no_dir) * f64::from(p.pressure) * f64::from(p.mu) * dt * dt) as f32;
        let th = f64::from(t) * f64::from(t) + f64::from(1e-6f32);
        self.clamp_spring(cp, t, th);
        self.friction_2d(cp, es);
    }

    // ---- the energy easing (FS6.2) ----

    /// The relative motion of two cores (`rel_setup`, 0x1001cdb0 and 0x1001d060).
    fn rel_setup(&self, a: usize, b: usize) -> Rel {
        let (c0, c1) = if self.cores[b].unmovable { (b, a) } else { (a, b) };
        let (k0, k1) = (&self.cores[c0], &self.cores[c1]);
        let mut dv = [0.0f32; 3];
        for k in 0..3 {
            dv[k] = k1.speed[k] - k0.speed[k];
        }
        let vlen = fast_normize(&mut dv);
        let w1 = rmul_f(&k1.m_world_f_core.r, &k1.rot_speed);
        let w0 = rmul_f(&k0.m_world_f_core.r, &k0.rot_speed);
        let mut w = [w1[0] - w0[0], w1[1] - w0[1], w1[2] - w0[2]];
        let wlen = fast_normize(&mut w);
        let ax0 = rtmul_f(&k0.m_world_f_core.r, &w);
        let mw = [w[0] * -1.0, w[1] * -1.0, w[2] * -1.0];
        let ax1 = rtmul_f(&k1.m_world_f_core.r, &mw);
        let t0 = [k0.rot_inertia[0] * ax0[0], k0.rot_inertia[1] * ax0[1], k0.rot_inertia[2] * ax0[2]];
        let mut ir0 = len3(&t0);
        let mut iir0 = if ir0 < C_1E19 {
            ir0 = 1.0;
            1.0
        } else {
            1.0 / ir0
        };
        let t1 = [ax1[0] * k1.rot_inertia[0], ax1[1] * k1.rot_inertia[1], ax1[2] * k1.rot_inertia[2]];
        let mut ir1 = len3(&t1);
        let iir1 = if ir1 < C_1E19 {
            ir1 = 1.0;
            1.0
        } else {
            1.0 / ir1
        };
        let m1 = f64::from(k1.mass);
        let im1 = f64::from(k1.inv_mass);
        let (m0, im0);
        if self.cores[c0].unmovable {
            m0 = m1 * 10000.0;
            im0 = im1 * 1e-4;
            ir0 = ir1 * 10000.0;
            iir0 = iir1 * 1e-4;
        } else {
            m0 = f64::from(k0.mass);
            im0 = f64::from(k0.inv_mass);
        }
        let er = rel_energy(wlen, ir0, ir1, iir0, iir1);
        let el = rel_energy(vlen, m0, m1, im0, im1);
        Rel {
            dv,
            ax0,
            ax1,
            vlen,
            im0,
            im1,
            wlen,
            iir0,
            iir1,
            er,
            el,
            etot: el + er,
            c0,
            c1,
        }
    }

    /// Removes the fraction `f` of the relative energy through pending velocity changes (`rel_remove`, 0x1001d0e0).
    fn rel_remove(&mut self, r: &mut Rel, f: f64) {
        let ir = rel_impulse(r.wlen, r.iir0, r.iir1, f * r.er);
        let il = rel_impulse(r.vlen, r.im0, r.im1, f * r.el);
        if !self.cores[r.c0].unmovable {
            let c0 = &mut self.cores[r.c0];
            for k in 0..3 {
                c0.speed_change[k] = (f64::from(c0.speed_change[k]) + f64::from(r.dv[k]) * (il * r.im0)) as f32;
            }
            for k in 0..3 {
                c0.rot_speed_change[k] = (f64::from(c0.rot_speed_change[k]) + f64::from(r.ax0[k]) * (ir * r.iir0)) as f32;
            }
        }
        for k in 0..3 {
            r.dv[k] *= -1.0;
        }
        let c1 = &mut self.cores[r.c1];
        for k in 0..3 {
            c1.speed_change[k] = (f64::from(c1.speed_change[k]) + f64::from(r.dv[k]) * (il * r.im1)) as f32;
        }
        for k in 0..3 {
            c1.rot_speed_change[k] = (f64::from(c1.rot_speed_change[k]) + f64::from(r.ax1[k]) * (ir * r.iir1)) as f32;
        }
    }

    /// At most a tenth of the pair's relative energy per PSI (`ease_energy`, 0x1001ccd0 with 0x1001d2a0).
    fn ease_energy(&mut self, fs: usize, pair: usize, delta_psi: f64) {
        let mut e = 0.9f64.powf(delta_psi) * f64::from(self.coll.fss[fs].pairs[pair].anti_energy);
        self.coll.fss[fs].pairs[pair].anti_energy = e as f32;
        if !(e > 0.0) {
            return;
        }
        let c = [self.coll.fss[fs].pairs[pair].c[0], self.coll.fss[fs].pairs[pair].c[1]];
        let mut r = self.rel_setup(c[0], c[1]);
        if e > r.etot * C_01F {
            e = r.etot * C_01F;
        }
        let mut d = 0.0;
        if !(r.etot < C_1E19) {
            let etot = r.etot;
            self.rel_remove(&mut r, e / etot);
            d = e;
        }
        let p = &mut self.coll.fss[fs].pairs[pair];
        p.anti_energy = (f64::from(p.anti_energy) - d) as f32;
    }

    // ---- the contact update (FS5.1) ----

    /// The contact update controller (priority 2000, `0x1001d610`).
    pub(crate) fn fs_update_simulate(&mut self, fs: usize, su: usize) {
        if self.coll.fss[fs].n_contacts <= 1 {
            if let Some(&cp) = self.coll.fss[fs].cps.first() {
                self.cp_update(cp);
            }
            return;
        }
        let npairs = self.coll.fss[fs].pairs.len();
        for i in (0..npairs).rev() {
            let cps = self.coll.fss[fs].pairs[i].cps.clone();
            let mut e = 0.0f32;
            for &cp in cps.iter().rev() {
                let g_old = self.coll.cps[cp].gap;
                self.cp_update(cp);
                e = ((f64::from(g_old) - f64::from(self.coll.cps[cp].gap)) * f64::from(self.coll.cps[cp].pressure) + f64::from(e)) as f32;
            }
            if e > 0.0 {
                let p = &mut self.coll.fss[fs].pairs[i];
                p.anti_energy = (f64::from(e) + f64::from(p.anti_energy)) as f32;
            }
        }
        let (fast, fast_prev) = (self.units[su].fast, self.units[su].fast_prev);
        if fast_prev {
            for p in self.coll.fss[fs].pairs.iter_mut() {
                p.anti_energy = 0.0;
            }
        }
        if !fast {
            let delta_psi = self.env.delta_psi;
            let npairs = self.coll.fss[fs].pairs.len();
            for i in (0..npairs).rev() {
                self.ease_energy(fs, i, delta_psi);
            }
        }
    }

    // ---- the normal force (FS5.3-5.4) ----

    /// One impulse driving the approach speed to -k (desired - gap), 20 k when too far (`0x1001ae40`).
    fn normal_single(&mut self, cp: usize, es: &EventSim, desired: f32, k: f32) {
        let t = self.coll.cps[cp].info.clone();
        let (c0, c1) = (t.core[0], t.core[1]);
        let mut vr = 0.0f64;
        if let Some(c) = c0 {
            vr = dotd(&t.cross_cs[0], &self.cores[c].rot_speed) + dotd(&self.cores[c].speed, &t.n);
        }
        if let Some(c) = c1 {
            vr -= dotd(&self.cores[c].rot_speed, &t.cross_cs[1]) + dotd(&self.cores[c].speed, &t.n);
        }
        let gap = self.coll.cps[cp].gap;
        let x = f64::from(desired) - f64::from(gap);
        let mut kk = f64::from(k);
        if x < 0.0 {
            kk *= 20.0;
        }
        let imp = (kk * x + vr) * f64::from(t.virt_mass);
        if imp <= 0.0 {
            self.coll.cps[cp].pressure = 0.0;
            return;
        }
        self.coll.cps[cp].pressure = (imp * es.i_delta_time) as f32;
        if let Some(c) = c0 {
            let core = &mut self.cores[c];
            for i in 0..3 {
                core.rot_speed[i] = (f64::from(core.rot_speed[i]) + f64::from(t.cross_cs[0][i] * core.inv_rot_inertia[i]) * -imp) as f32;
            }
            let s = -(f64::from(core.inv_mass) * imp);
            for i in 0..3 {
                core.speed[i] = (f64::from(core.speed[i]) + f64::from(t.n[i]) * s) as f32;
            }
        }
        if let Some(c) = c1 {
            let core = &mut self.cores[c];
            for i in 0..3 {
                core.rot_speed[i] = (f64::from(core.rot_speed[i]) + f64::from(t.cross_cs[1][i] * core.inv_rot_inertia[i]) * imp) as f32;
            }
            let s = f64::from(core.inv_mass) * imp;
            for i in 0..3 {
                core.speed[i] = (f64::from(core.speed[i]) + f64::from(t.n[i]) * s) as f32;
            }
        }
    }

    // ---- union-find split (FS7.2-7.3) ----

    fn uf_root(&self, mut c: usize) -> usize {
        while let Some(p) = self.coll.objs[c].uf_parent {
            c = p;
        }
        c
    }

    /// A root of a component other than the first movable core's, or None (`0x1001c7f0`).
    fn fs_union_find(&mut self, fs: usize) -> Option<usize> {
        let cores = self.coll.fss[fs].cores.clone();
        for &c in &cores {
            self.coll.objs[c].uf_parent = None;
        }
        let pairs = self.coll.fss[fs].pairs.clone();
        for p in pairs.iter().rev() {
            if self.cores[p.c[0]].unmovable || self.cores[p.c[1]].unmovable {
                continue;
            }
            let r0 = self.uf_root(p.c[0]);
            let r1 = self.uf_root(p.c[1]);
            if r0 != r1 {
                self.coll.objs[r1].uf_parent = Some(r0);
            }
        }
        let mut reference = None;
        for &c in &cores {
            if !self.cores[c].unmovable {
                reference = Some(self.uf_root(c));
                break;
            }
        }
        let movable = self.coll.fss[fs].movable.clone();
        let mut ret = None;
        for &c in movable.iter().rev() {
            let r = self.uf_root(c);
            if Some(r) != reference {
                ret = Some(r);
            }
        }
        ret
    }

    fn fs_drop_all(&mut self, fs: usize) {
        while let Some(&c) = self.coll.fss[fs].cores.first() {
            if let Some(fi) = self.core_fi(c, fs) {
                self.fi_delete(c, fi);
            }
            self.fs_remove_core(fs, c);
        }
        self.fs_delete(fs);
    }

    /// The components other than the root's move to new systems (`0x1001c8a0`).
    fn fs_split(&mut self, fs: usize, mut root: usize) {
        loop {
            let nw = self.fs_new();
            let ncores = self.coll.fss[fs].cores.len();
            for i in (0..ncores).rev() {
                let Some(&c) = self.coll.fss[fs].cores.get(i) else { continue };
                if !self.cores[c].unmovable {
                    if self.uf_root(c) != root {
                        continue;
                    }
                    self.fs_remove_core(fs, c);
                    self.fs_add_core(nw, c);
                    if let Some(fi) = self.coll.objs[c].fi {
                        self.coll.fis[fi].fs = nw;
                    }
                } else {
                    let f = self.fi_new(nw);
                    self.fi_insert(c, f);
                    self.fs_add_core(nw, c);
                }
            }
            let npairs = self.coll.fss[fs].pairs.len();
            for i in (0..npairs).rev() {
                let Some(p) = self.coll.fss[fs].pairs.get(i).cloned() else { continue };
                let mc = if self.cores[p.c[0]].unmovable { p.c[1] } else { p.c[0] };
                if self.uf_root(mc) != root {
                    continue;
                }
                let fx = if self.cores[p.c[0]].unmovable {
                    Some(p.c[0])
                } else if self.cores[p.c[1]].unmovable {
                    Some(p.c[1])
                } else {
                    None
                };
                let fi_old = fx.and_then(|x| self.core_fi(x, fs));
                let fi_nw = fx.and_then(|x| self.core_fi(x, nw));
                self.coll.fss[fs].pairs.remove(i);
                self.coll.fss[nw].pairs.push(p.clone());
                for &cp in &p.cps {
                    self.fs_unlink(fs, cp);
                    self.fs_push_front(nw, cp);
                    if let (Some(fo), Some(fnw)) = (fi_old, fi_nw) {
                        remove_value(&mut self.coll.fis[fo].cps, cp);
                        self.coll.fis[fnw].cps.push(cp);
                    }
                }
            }
            let ncores = self.coll.fss[fs].cores.len();
            for i in (0..ncores).rev() {
                let Some(&c) = self.coll.fss[fs].cores.get(i) else { continue };
                if !self.cores[c].unmovable {
                    continue;
                }
                let fnw = self.core_fi(c, nw);
                let fo = self.core_fi(c, fs);
                if let Some(f) = fnw {
                    if self.coll.fis[f].cps.is_empty() {
                        self.fi_delete(c, f);
                        self.fs_remove_core(nw, c);
                    }
                }
                if let Some(f) = fo {
                    if self.coll.fis[f].cps.is_empty() {
                        self.fi_delete(c, f);
                        self.fs_remove_core(fs, c);
                    }
                }
            }
            if self.coll.fss[nw].n_cores < 2 {
                self.fs_drop_all(nw);
                return;
            }
            if self.coll.fss[fs].n_cores < 2 {
                self.fs_drop_all(fs);
                return;
            }
            match self.fs_union_find(fs) {
                Some(r) => root = r,
                None => return,
            }
        }
    }

    /// The normal force controller (priority 0, `0x1001d6c0`).
    pub(crate) fn fs_normal_simulate(&mut self, fs: usize, es: &EventSim) {
        if self.coll.fss[fs].n_contacts <= 1 {
            if let Some(&cp) = self.coll.fss[fs].cps.first() {
                self.normal_single(cp, es, SET_FRICTION_DIST, 1.0);
                let p = &self.coll.cps[cp];
                if p.gap >= SET_MAX_DIST_FOR_FRICTION || ci_left(&p.info) {
                    self.fs_remove_cp(fs, cp);
                }
            }
        } else {
            self.fs_solve(fs, es);
        }
        if self.coll.fss[fs].n_contacts == 0 {
            self.fs_delete(fs);
            self.units[es.sim_unit].changed = true;
            return;
        }
        if self.coll.fss[fs].uf_needed {
            self.coll.fss[fs].uf_needed = false;
            if let Some(c) = self.fs_union_find(fs) {
                self.fs_split(fs, c);
                let u = self.cores[c].unit;
                self.units[u].changed = true;
            }
        }
    }

    // ---- the solve (ivp_fs_solve, phys_fsolver.c 0x10036d70) ----

    /// The total kinetic energy of a system's cores with their pending changes (`0x10036ec0`).
    fn fs_energy(&self, fs: usize) -> f64 {
        let mut e = 0.0;
        for &c in &self.coll.fss[fs].cores {
            let core = &self.cores[c];
            let mut v = [0.0f32; 3];
            let mut w = [0.0f32; 3];
            for k in 0..3 {
                v[k] = core.speed[k] + core.speed_change[k];
                w[k] = core.rot_speed[k] + core.rot_speed_change[k];
            }
            e += core_energy(core.mass, &core.rot_inertia, &v, &w);
        }
        e
    }

    /// Solves a system with more than one contact: the normal impulses, pushed asynchronously and committed or
    /// aborted by the energy test (`do_friction_system`, 0x10036d70).
    pub(crate) fn fs_solve(&mut self, fs: usize, es: &EventSim) {
        self.fs_sort_by_key(fs);
        self.fs_clear_far(fs);
        if self.coll.fss[fs].n_contacts as usize > MAX_CONTACTS {
            self.fs_too_many_contacts(fs);
            return;
        }
        let cps = self.coll.fss[fs].cps.clone();
        let n = cps.len();
        // setup_coords (0x100366a0): the infos of both sides, the gap, the key and the index of each point
        for (idx, &cp) in cps.iter().enumerate() {
            let (a, b) = (self.coll.cps[cp].syn[0].obj, self.coll.cps[cp].syn[1].obj);
            let fi0 = self.core_fi(a, fs);
            let fi1 = self.core_fi(b, fs);
            let p = &mut self.coll.cps[cp];
            p.info.index = idx as i32;
            p.info.gap = p.gap;
            p.info.fi = [fi0, fi1];
        }
        let mut a = vec![0.0f64; n * n];
        let mut bvec = vec![0.0f64; n];
        let mut wi: Vec<usize> = Vec::new();
        // build_matrix (0x10036760)
        for (i, &cp) in cps.iter().enumerate() {
            let t = self.coll.cps[cp].info.clone();
            let key = self.coll.cps[cp].key;
            let mut vrel = 0.0f64;
            if let Some(c) = t.core[0] {
                vrel = dotd(&self.cores[c].rot_speed, &t.cross_cs[0]) + dotd(&self.cores[c].speed, &t.n);
            }
            if let Some(c) = t.core[1] {
                vrel -= dotd(&self.cores[c].rot_speed, &t.cross_cs[1]) + dotd(&self.cores[c].speed, &t.n);
            }
            let d = f64::from(SET_FRICTION_DIST) - f64::from(self.coll.cps[cp].gap);
            let factor = if d < 0.0 { 20.0 } else { 1.0 };
            bvec[i] = factor * d + vrel;
            if key != 0 {
                wi.push(i);
            }
            for side in 0..2 {
                let Some(c) = t.core[side] else { continue };
                let core = &self.cores[c];
                let mut av = [0.0f32; 3];
                let bb: P3;
                if side == 0 {
                    for q in 0..3 {
                        av[q] = (t.cross_cs[0][q] * core.inv_rot_inertia[q]) * -1.0;
                    }
                    let s = core.inv_mass * -1.0;
                    bb = v3scale(&t.n, s);
                } else {
                    for q in 0..3 {
                        av[q] = t.cross_cs[1][q] * core.inv_rot_inertia[q];
                    }
                    bb = v3scale(&t.n, core.inv_mass);
                }
                let fi = t.fi[side];
                let Some(fi) = fi else { continue };
                let members = self.coll.fis[fi].cps.clone();
                for m in members {
                    let tj = self.coll.cps[m].info.clone();
                    let j = tj.index;
                    if j < 0 {
                        continue;
                    }
                    let (s, kk) = if tj.core[0] == Some(c) { (-1.0, 0) } else { (1.0, 1) };
                    if tj.core[kk].is_none() {
                        continue;
                    }
                    a[j as usize * n + i] += (dotd(&av, &tj.cross_cs[kk]) + dotd(&bb, &tj.n)) * s;
                }
            }
        }
        // scale (0x100362e0, 0x10036190)
        let mut maxd = 0.0f64;
        for i in (0..n).rev() {
            if a[i * n + i] > maxd {
                maxd = a[i * n + i];
            }
        }
        let s_a = if maxd > C_1E19 { 1.0 / maxd } else { 1.0 };
        let mut maxb = 0.0f64;
        for i in (0..n).rev() {
            if bvec[i].abs() > maxb {
                maxb = bvec[i].abs();
            }
        }
        let (s_b, scale) = if maxb > C_1E19 { (1.0 / maxb, s_a * maxb) } else { (1.0, s_a) };
        for v in a.iter_mut() {
            *v *= s_a;
        }
        for v in bvec.iter_mut() {
            *v *= s_b;
        }
        // the warm start from the points that pushed last PSI
        let k = wi.len();
        let mut m_mat = vec![0.0f64; k * k];
        let mut mb = vec![0.0f64; k];
        for r in 0..k {
            for c in 0..k {
                m_mat[r * k + c] = a[wi[r] * n + wi[c]];
            }
            mb[r] = bvec[wi[r]];
        }
        let mut gx = vec![0.0f64; k];
        let mut x = vec![0.0f64; n];
        let mut ok = gauss_solve(k, &mut m_mat, &mut mb, &mut gx, 1e-9) == 1;
        if ok {
            let mut flag = vec![false; n];
            let mut bad = false;
            let g = self.gravity_magnitude() as f32;
            for m in 0..k {
                let i = wi[m];
                let inv_virt = f64::from(self.coll.cps[cps[i]].info.inv_virt_mass);
                if inv_virt * scale * gx[m] < f64::from(g * 0.01) {
                    bad = true;
                }
                flag[i] = true;
                x[i] = gx[m];
            }
            for i in 0..n {
                if !ok {
                    break;
                }
                if flag[i] {
                    continue;
                }
                let mut s = 0.0;
                for c in 0..n {
                    s += a[i * n + c] * x[c];
                }
                if !(bvec[i].abs() * C_1E5F + s >= bvec[i]) {
                    ok = false;
                }
            }
            if bad {
                ok = false;
            }
        }
        let mut solved = true;
        if !ok {
            let mut nfirst = 0;
            for &cp in &cps {
                if self.coll.cps[cp].key < 0 {
                    nfirst += 1;
                } else {
                    break;
                }
            }
            x = vec![0.0; n];
            solved = lcp_solve(&a, &bvec, &mut x, n, nfirst);
        }
        if !solved {
            return;
        }
        for v in x.iter_mut() {
            *v *= scale;
        }
        let e0 = self.fs_energy(fs);
        // do_resulting_pushes (0x100364c0)
        for (i, &cp) in cps.iter().enumerate() {
            let xi = x[i];
            let key = self.coll.cps[cp].key;
            if xi > 0.0 {
                self.coll.cps[cp].key = if key >= 0 { -1 } else { key - 1 };
            } else if xi == 0.0 {
                self.coll.cps[cp].key = 0;
                self.coll.cps[cp].pressure = 0.0;
                continue;
            } else {
                let mut kk = key;
                if kk < 0 {
                    kk = 0;
                }
                kk += 1;
                self.coll.cps[cp].key = if kk > 9 { 0 } else { kk };
            }
            let t = self.coll.cps[cp].info.clone();
            if let Some(c) = t.core[0] {
                let core = &mut self.cores[c];
                for q in 0..3 {
                    core.rot_speed_change[q] = (f64::from(core.rot_speed_change[q]) + f64::from(t.cross_cs[0][q] * core.inv_rot_inertia[q]) * -xi) as f32;
                }
                let s = -(f64::from(core.inv_mass) * xi);
                for q in 0..3 {
                    core.speed_change[q] = (f64::from(core.speed_change[q]) + f64::from(t.n[q]) * s) as f32;
                }
            }
            if let Some(c) = t.core[1] {
                let core = &mut self.cores[c];
                for q in 0..3 {
                    core.rot_speed_change[q] = (f64::from(core.rot_speed_change[q]) + f64::from(t.cross_cs[1][q] * core.inv_rot_inertia[q]) * xi) as f32;
                }
                let s = f64::from(core.inv_mass) * xi;
                for q in 0..3 {
                    core.speed_change[q] = (f64::from(core.speed_change[q]) + f64::from(t.n[q]) * s) as f32;
                }
            }
            let p = if xi < 0.0 { 0.0 } else { xi };
            self.coll.cps[cp].pressure = (p * es.i_delta_time) as f32;
        }
        let e1 = self.fs_energy(fs);
        // the tolerance: 0.1 m g of each movable core, with |g| hard-coded as 9.81 (0x10036fc0)
        let mut tol = 0.0f64;
        for &c in &self.coll.fss[fs].cores {
            if !self.cores[c].unmovable {
                tol += f64::from(self.cores[c].mass * C_981F) * 0.1;
            }
        }
        let movable = self.coll.fss[fs].movable.clone();
        for c in movable {
            if tol + e0 < e1 {
                let core = &mut self.cores[c];
                core.speed_change = [0.0; 3];
                core.rot_speed_change = [0.0; 3];
            } else {
                self.commit_pushes(c);
            }
        }
    }

    /// The gravity magnitude as the friction solve reads it (0x10036d70's sqrt of the world gravity).
    fn gravity_magnitude(&self) -> f64 {
        let g = self.gravity_vector();
        (f64::from(g[0]) * f64::from(g[0]) + f64::from(g[1]) * f64::from(g[1]) + f64::from(g[2]) * f64::from(g[2])).sqrt()
    }

    /// The points of a system in the order of their warm-start key, stable, ascending (`0x10036c60`).
    fn fs_sort_by_key(&mut self, fs: usize) {
        let cps = self.coll.fss[fs].cps.clone();
        if cps.len() < 2 {
            return;
        }
        let mut v = cps;
        for i in 1..v.len() {
            let mut j = i;
            while j > 0 && self.coll.cps[v[j - 1]].key > self.coll.cps[v[j]].key {
                v.swap(j, j - 1);
                j -= 1;
            }
        }
        self.coll.fss[fs].cps = v;
    }

    /// Removes the points that went far or left their feature (`0x10036b80`).
    fn fs_clear_far(&mut self, fs: usize) {
        let cps = self.coll.fss[fs].cps.clone();
        for cp in cps {
            let p = &self.coll.cps[cp];
            if p.gap >= SET_MAX_DIST_FOR_FRICTION || ci_left(&p.info) {
                self.fs_remove_cp(fs, cp);
            }
        }
    }

    /// Too many contacts: the movable cores with more than one pair among movable cores stop (`0x10036d10`).
    fn fs_too_many_contacts(&mut self, fs: usize) {
        let movable = self.coll.fss[fs].movable.clone();
        for &c in movable.iter().rev() {
            let mut k = 0;
            for p in &self.coll.fss[fs].pairs {
                if (p.c[0] == c || p.c[1] == c) && !self.cores[p.c[0]].unmovable && !self.cores[p.c[1]].unmovable {
                    k += 1;
                }
            }
            if k > 1 {
                let core = &mut self.cores[c];
                core.speed = [0.0; 3];
                core.rot_speed = [0.0; 3];
            }
        }
    }
}

/// IVP_CI_LEFT_FEATURE: the point's feature left its place (flags bits 8..9 == 1).
pub(crate) fn ci_left(t: &crate::contact::ContactInfo) -> bool {
    (t.flags & 0x300) == 0x100
}

/// Removes the first occurrence of a value, keeping the order (`ivp_vec_remove`).
fn remove_value(v: &mut Vec<usize>, x: usize) {
    if let Some(i) = v.iter().position(|&y| y == x) {
        v.remove(i);
    }
}


#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::collision::ObjKind;
    use crate::convex;
    use crate::ledge::Surface;
    use crate::math::Quat;

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

    #[test]
    fn a_touching_pair_gets_a_friction_system_with_its_point() {
        let mut w = World::new();
        let q = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
        for (i, x) in [0.0, 1.01].into_iter().enumerate() {
            let c = w.core_init(i, q, [x, 0.0, 0.0], false);
            w.cores[c].upper_limit_radius = 0.866;
            w.cores[c].movement_state = crate::core::MT_MOVING;
            w.set_object_geometry(c, ObjKind::Polygon, Some(unit_cube()), 0.0);
            w.set_object_material(c, 0.8, 0.2);
        }
        w.enable_collision_detection(0);
        w.enable_collision_detection(1);
        let m = (0..w.coll.mds.len()).find(|&m| w.coll.mds[m].alive).expect("a mindist");
        let md = w.coll.mds[m].clone();
        assert!(md.len_numerator < SET_MAX_DIST_FOR_FRICTION, "gap {}", md.len_numerator);
        let (cp, created, fs) = w.try_generate_friction(&md, None, true);
        assert!(created, "a new point");
        assert_eq!(w.coll.fss[fs].n_contacts, 1);
        assert!(w.coll.cps[cp].fs == Some(fs));
        assert!(w.coll.objs[0].fi.is_some() && w.coll.objs[1].fi.is_some(), "both movable cores have their info");
        // the same features again find the point rather than making another
        let (cp2, created2, _) = w.try_generate_friction(&md, None, true);
        assert!(!created2 && cp2 == cp);
    }
}
