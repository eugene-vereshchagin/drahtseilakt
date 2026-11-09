// Port of OpenBallance src/phys/phys_core.c (GPL-3.0; see the LICENSE of the repository): the IVP core and the per-PSI
// dynamics (docs/ivp_core.md): core creation, pose queries, pushes, damping, gravity, simulation units and
// controllers, the PSI step, integration, anomaly clamps, sleep, freeze and revive.
//
// Objects live in arenas (`World::cores`, `World::units`, `World::controllers`) and refer to each other by index, not
// by pointer. Where the original calls the collision code (`phys_collision_*`), this port calls the hooks of
// `World::collision` (see `crate::collision`), which carry the same contract.

use crate::math::{self, Matrix, Quat};

/// Movement states (0.4): a body is simulated (1, 2, 3 = moving, slow, calm), not simulated (8), or unmovable (0x10).
pub const MT_MOVING: i8 = 1;
pub const MT_SLOW: i8 = 2;
pub const MT_CALM: i8 = 3;
pub const MT_NOT_SIM: i8 = 8;
pub const MT_STATIC: i8 = 0x10;

/// Anomaly limits of the default manager (9): velocity clamp and angular clamp per PSI.
pub const MAX_VELOCITY_DEFAULT: f32 = 2000.0;

/// The DLL-global random generator of 0x1002fcd0: a multiplicative congruence shared by all worlds, never reseeded.
#[derive(Debug, Clone)]
pub struct Rng {
    seed: u32,
}

impl Default for Rng {
    fn default() -> Self {
        Rng { seed: 1 }
    }
}

impl Rng {
    pub fn next_unit(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(75);
        (self.seed & 0xffff) as f32 * 1.52587890625e-05
    }
}

/// An IVP core (1.2): the physics state of one object's mass centre.
#[derive(Clone, Debug)]
pub struct Core {
    pub unmovable: bool,
    pub rot_inertias_equal: bool,
    pub in_revive_list: bool,
    pub upper_limit_radius: f32,
    pub max_surface_deviation: f32,
    pub rot_inertia: [f32; 3],
    pub mass: f32,
    pub rot_speed_damp: [f32; 3],
    pub inv_rot_inertia: [f32; 3],
    pub inv_mass: f32,
    pub speed_damp: f32,
    pub inv_object_diameter: f32,
    pub movement_state: i8,
    pub time_of_last_psi: f64,
    pub i_delta_time: f32,
    pub rot_speed_change: [f32; 3],
    pub speed_change: [f32; 3],
    pub rot_speed: [f32; 3],
    pub speed: [f32; 3],
    pub pos_last: [f64; 3],
    pub delta_pos: [f32; 3],
    pub q_last: Quat,
    pub q_next: Quat,
    pub m_world_f_core: Matrix,
    pub rotation_axis_world: [f32; 3],
    pub current_speed: f32,
    pub abs_omega: f32,
    pub max_surface_rot_speed: f32,
    /// Sleep test references (10.0): the time of each window, its orientation and its position.
    pub time_calm_ref: [f64; 2],
    pub q_calm_ref: [[f32; 4]; 2],
    pub pos_calm_ref: [[f32; 3]; 2],
    /// The simulation unit this core belongs to.
    pub unit: usize,
    /// The controllers this core is attached to, in attachment order.
    pub controllers: Vec<usize>,
    /// The body (object) this core belongs to, as an index into the world's bodies.
    pub body: usize,
    pub temporarily_unmovable: u8,
    pub impacts_psi: i16,
    /// The synchronized state of an impact: the pose and rotation to restore, and whether it pushed.
    pub synced: bool,
    pub sync_pushed: bool,
    pub sync_rot_speed: [f32; 3],
    pub sync_q: Quat,
    /// The impact counter of the last impact this core took part in.
    pub impact_stamp: u32,
    /// The object's origin in core coordinates (`shift_core_f_object`); zero for most bodies.
    pub shift: [f32; 3],
    /// The game's id of the body this core belongs to (`PhysBodyDesc`'s entity), for the readouts.
    pub entity: u32,
    /// A body destroyed by `World::body_destroy`: its index stays reserved and nothing refers to it any more.
    pub removed: bool,
}

impl Core {
    /// The pose of the object at time `t` (`ivp_body_m_world_f_object_at`, phys_core.c 333): the core's rotation
    /// slerped from the last PSI, its position, and the object's shift rotated into the world.
    pub fn object_pose_at(&self, t: f64) -> Matrix {
        let dt = t - self.time_of_last_psi;
        let q = math::quat_slerp(
            &self.q_last,
            &self.q_next,
            dt * f64::from(self.i_delta_time),
        );
        let r = math::quat_to_matrix(&q);
        let mut vv = [0.0; 3];
        for k in 0..3 {
            vv[k] = self.pos_last[k] + f64::from(self.delta_pos[k]) * dt;
        }
        if self.shift != [0.0; 3] {
            let s = math::rmul(
                &r,
                &[
                    f64::from(self.shift[0]),
                    f64::from(self.shift[1]),
                    f64::from(self.shift[2]),
                ],
            );
            for k in 0..3 {
                vv[k] += s[k];
            }
        }
        Matrix { r, vv }
    }
}

/// A simulation unit (1.4): a set of cores that are simulated together, with its controllers' entries.
#[derive(Clone, Debug, Default)]
pub struct SimUnit {
    pub state: i8,
    pub changed: bool,
    pub fast: bool,
    pub fast_prev: bool,
    pub cores: Vec<usize>,
    /// Entries in ascending priority: a controller and the cores of this unit it acts on.
    pub entries: Vec<(usize, Vec<usize>)>,
}

/// The controllers (6, 7): what acts on the cores each PSI.
#[derive(Clone, Debug)]
pub enum Controller {
    /// The gravity controller (1.7, priority 1000): the game's gravity vector.
    Gravity {
        g: [f32; 3],
    },
    /// One of a friction system's three controllers (`crate::friction`).
    Friction {
        fs: usize,
        kind: FrictionKind,
    },
    /// A spring (`crate::constraint`, priority 1500) and a constraint (priority 405) acting on a joint.
    Spring {
        joint: usize,
    },
    Constraint {
        joint: usize,
    },
    /// SetPhysicsForce's controller (`crate::glue`, priority 1500): a force on one body every PSI.
    Force(crate::glue::Force),
}

/// The controllers of a friction system: the normal force (0), the tangential spring (600), the contact update (2000).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum FrictionKind {
    Normal,
    Spring,
    Update,
}

impl Controller {
    pub fn priority(&self) -> i32 {
        match self {
            Controller::Gravity { .. } => 1000,
            Controller::Spring { .. } => 1500,
            Controller::Constraint { .. } => 405,
            Controller::Force(_) => 1500,
            Controller::Friction { kind, .. } => match kind {
                FrictionKind::Normal => 0,
                FrictionKind::Spring => 600,
                FrictionKind::Update => 2000,
            },
        }
    }

    /// Whether the controller keeps its cores in one unit (springs and constraints do; gravity does not).
    pub fn links_cores(&self) -> bool {
        match self {
            Controller::Gravity { .. } | Controller::Force(_) => false,
            Controller::Friction { kind, .. } => *kind == FrictionKind::Spring,
            Controller::Spring { .. } | Controller::Constraint { .. } => true,
        }
    }
}

/// The environment's time and anomaly state (1.6, 9).
#[derive(Clone, Debug)]
pub struct Env {
    pub time_factor: f32,
    pub delta_psi: f64,
    pub inv_delta_psi: f64,
    pub current_time: f64,
    pub time_of_next_psi: f64,
    pub time_of_last_psi: f64,
    pub freeze_check_time: f32,
    pub max_velocity: f32,
    pub max_angular_velocity_per_psi: f32,
    pub sleep_countdown: u16,
    pub state: i32,
}

/// The environment as the world's constructor leaves it (`phys_world_create`, the IVP environment ctor `0x10012a60`):
/// the first sleep test after 10 PSIs, a freeze check time of 0.3 s, the state after a PSI.
impl Default for Env {
    fn default() -> Self {
        Env {
            time_factor: 0.001,
            delta_psi: 1.0 / 66.0,
            inv_delta_psi: 66.0,
            current_time: 0.0,
            time_of_next_psi: 1.0 / 66.0,
            time_of_last_psi: 0.0,
            freeze_check_time: 0.3,
            max_velocity: MAX_VELOCITY_DEFAULT,
            max_angular_velocity_per_psi: 1.570_796_37,
            sleep_countdown: 10,
            state: 5,
        }
    }
}

/// The cores, units and controllers of one physics world.
#[derive(Debug, Default, Clone)]
pub struct World {
    pub env: Env,
    pub cores: Vec<Core>,
    pub units: Vec<SimUnit>,
    pub controllers: Vec<Controller>,
    /// The active units in list order (front first) and the frozen ones.
    pub active: Vec<usize>,
    pub frozen: Vec<usize>,
    /// Cores waiting to be revived at the start of the next PSI.
    pub revive: Vec<usize>,
    pub rng: Rng,
    pub time: crate::time::TimeManager,
    /// Bumped on every change of the current time (the upstream's time code).
    pub time_code: u32,
    /// The joints (springs and constraints) of the world (`crate::constraint`).
    pub joints: Vec<crate::constraint::Joint>,
    /// The game's listeners of impacts and contacts (`phys_set_listeners`).
    pub listeners: crate::glue::Listeners,
    /// The frame time smoothing of `phys_frame`, in milliseconds.
    pub smoothed_ms: f32,
    /// The collision state: the broadphase, the hull managers and the collision records (`crate::collision`).
    pub coll: crate::collision::CollWorld,
}

fn v3(a: [f32; 3]) -> [f64; 3] {
    [f64::from(a[0]), f64::from(a[1]), f64::from(a[2])]
}

fn dot3(a: &[f32; 3]) -> f64 {
    f64::from(a[0]) * f64::from(a[0])
        + f64::from(a[1]) * f64::from(a[1])
        + f64::from(a[2]) * f64::from(a[2])
}

impl World {
    /// A world as `phys_world_create` makes it: the environment's defaults, and the time code at 1, so that a new
    /// collision cache or mindist (stamped 0) is computed at the first time step.
    pub fn new() -> World {
        World {
            time_code: 1,
            ..World::default()
        }
    }

    /// Adds a gravity controller (the world's own, attached to every new core).
    pub fn add_controller(&mut self, c: Controller) -> usize {
        self.controllers.push(c);
        self.controllers.len() - 1
    }

    fn unit_list_add(&mut self, su: usize) {
        let state = self.units[su].state;
        if state < MT_NOT_SIM {
            self.active.insert(0, su);
        } else {
            self.frozen.insert(0, su);
        }
    }

    pub(crate) fn unit_list_remove(&mut self, su: usize) {
        self.active.retain(|&u| u != su);
        self.frozen.retain(|&u| u != su);
    }

    fn unit_set_state(&mut self, su: usize, state: i8) {
        self.unit_list_remove(su);
        self.units[su].state = state;
        self.unit_list_add(su);
    }

    fn new_unit(&mut self) -> usize {
        self.units.push(SimUnit {
            state: MT_NOT_SIM,
            ..SimUnit::default()
        });
        self.units.len() - 1
    }

    fn unit_add_core(&mut self, su: usize, c: usize) {
        self.units[su].cores.push(c);
        self.cores[c].unit = su;
    }

    /// Insertion sort of the entries to ascending priority; swaps only when strictly greater, so equal priorities keep
    /// their order (`0x10011db0`).
    fn unit_sort(&mut self, su: usize) {
        let controllers = &self.controllers;
        let entries = &mut self.units[su].entries;
        for i in 1..entries.len() {
            let mut j = i;
            while j > 0
                && controllers[entries[j - 1].0].priority() > controllers[entries[j].0].priority()
            {
                entries.swap(j, j - 1);
                j -= 1;
            }
        }
    }

    /// Adds a core to a controller's entry in a unit, creating the entry when needed (`0x10011cc0`).
    fn unit_add_controller_core(&mut self, su: usize, ctrl: usize, c: usize) {
        let pos = self.units[su].entries.iter().position(|e| e.0 == ctrl);
        match pos {
            Some(i) => self.units[su].entries[i].1.push(c),
            None => self.units[su].entries.push((ctrl, vec![c])),
        }
        self.unit_sort(su);
    }

    /// The core leaves the controller's entry; an empty entry is deleted (`0x10011d00`).
    fn unit_remove_controller_core(&mut self, su: usize, ctrl: usize, c: usize) {
        if let Some(i) = self.units[su].entries.iter().position(|e| e.0 == ctrl) {
            self.units[su].entries[i].1.retain(|&x| x != c);
            if self.units[su].entries[i].1.is_empty() {
                self.units[su].entries.remove(i);
            }
        }
    }

    /// Attaches a controller to a core (`0x10011c70`).
    pub fn core_add_controller(&mut self, c: usize, ctrl: usize) {
        self.cores[c].controllers.push(ctrl);
        let su = self.cores[c].unit;
        self.unit_add_controller_core(su, ctrl, c);
    }

    /// Detaches a controller from a core (`0x10011c00`).
    pub fn core_remove_controller(&mut self, c: usize, ctrl: usize) {
        if let Some(i) = self.cores[c].controllers.iter().position(|&x| x == ctrl) {
            self.cores[c].controllers.remove(i);
            let su = self.cores[c].unit;
            self.unit_remove_controller_core(su, ctrl, c);
        }
    }

    /// The cores of `from` join `into`, with their controllers; `from` is deleted (`0x10011720`).
    pub fn sim_unit_merge(&mut self, into: usize, from: usize) {
        if into == from {
            return;
        }
        let cores = self.units[from].cores.clone();
        for c in cores {
            self.unit_add_core(into, c);
            let ctrls = self.cores[c].controllers.clone();
            for k in ctrls {
                self.unit_add_controller_core(into, k, c);
            }
        }
        self.unit_list_remove(from);
        self.units[from] = SimUnit::default();
    }

    /// Creates a core for an object at `pos` with orientation `q`, in a new frozen unit, attached to gravity
    /// (`0x1000d400`, 2.2).
    pub fn core_init(&mut self, body: usize, q: Quat, pos: [f64; 3], unmovable: bool) -> usize {
        let mut q_next = q;
        math::quat_normalize(&mut q_next);
        let m = Matrix {
            r: math::quat_to_matrix(&q_next),
            vv: pos,
        };
        let idx = self.cores.len();
        self.cores.push(Core {
            unmovable,
            rot_inertias_equal: false,
            in_revive_list: false,
            upper_limit_radius: 1000.0,
            max_surface_deviation: 1000.0,
            rot_inertia: [1.0; 3],
            mass: 1.0,
            rot_speed_damp: [0.0; 3],
            inv_rot_inertia: [1.0; 3],
            inv_mass: 1.0,
            speed_damp: 0.0,
            inv_object_diameter: 0.5 / 1000.0,
            movement_state: MT_NOT_SIM,
            time_of_last_psi: 0.0,
            i_delta_time: 0.0,
            rot_speed_change: [0.0; 3],
            speed_change: [0.0; 3],
            rot_speed: [0.0; 3],
            speed: [0.0; 3],
            pos_last: pos,
            delta_pos: [0.0; 3],
            q_last: q_next,
            q_next,
            m_world_f_core: m,
            rotation_axis_world: [0.0; 3],
            current_speed: 0.0,
            abs_omega: 0.0,
            max_surface_rot_speed: 0.0,
            time_calm_ref: [0.0; 2],
            q_calm_ref: [[0.0; 4]; 2],
            pos_calm_ref: [[0.0; 3]; 2],
            unit: 0,
            controllers: Vec::new(),
            body,
            temporarily_unmovable: 0,
            impacts_psi: 0,
            synced: false,
            sync_pushed: false,
            sync_rot_speed: [0.0; 3],
            sync_q: q,
            impact_stamp: 0,
            shift: [0.0; 3],
            entity: 0,
            removed: false,
        });
        let su = self.new_unit();
        self.unit_add_core(su, idx);
        self.units[su].state = MT_NOT_SIM;
        let gravity = self
            .controllers
            .iter()
            .position(|c| matches!(c, Controller::Gravity { .. }));
        if let Some(g) = gravity {
            self.core_add_controller(idx, g);
        }
        self.cores[idx].movement_state = MT_NOT_SIM;
        self.unit_list_add(su);
        idx
    }

    /// Mass, inertia and damping of a core (`0x1000d1a0`, 2.6): derived values and the equal-inertia flag.
    pub fn calc_calc(&mut self, c: usize) {
        let core = &mut self.cores[c];
        for i in 0..3 {
            core.inv_rot_inertia[i] = 1.0 / core.rot_inertia[i];
        }
        core.inv_mass = 1.0 / core.mass;
        let a = core.inv_rot_inertia;
        let i = core.rot_inertia;
        let d = (f64::from(a[1]) - f64::from(a[2])).powi(2)
            + (f64::from(a[0]) - f64::from(a[1])).powi(2)
            + (f64::from(a[2]) - f64::from(a[0])).powi(2);
        // The upstream's quirk: the spread of the inverse inertias against the squared inertias, scaled by 0.01.
        core.rot_inertias_equal = d
            < (f64::from(i[2]) * f64::from(i[2])
                + f64::from(i[1]) * f64::from(i[1])
                + f64::from(i[0]) * f64::from(i[0]))
                * 0.01;
        core.inv_object_diameter = 0.5 / core.upper_limit_radius;
    }

    /// Initialises a core for simulation at the current PSI (`0x1000cc70`, 2.8).
    pub fn init_core_for_simulation(&mut self, c: usize, i_delta_time: f64) {
        let core = &mut self.cores[c];
        core.i_delta_time = i_delta_time as f32;
        core.q_next = core.q_last;
        core.abs_omega = 0.0;
        core.speed = [0.0; 3];
        core.delta_pos = [0.0; 3];
        core.current_speed = 0.0;
        core.max_surface_rot_speed = 0.0;
        core.rotation_axis_world = [1.0, 0.0, 0.0];
    }

    /// The reciprocal of a time step, with the upstream's value for (near) zero (`inv_dt`).
    pub fn inv_dt(dt: f64) -> f64 {
        if dt > 1e-10 {
            1.0 / dt
        } else {
            9_999_999_866.485_682
        }
    }

    /// The pose of a core at time `t` (`0x1000c510`).
    pub fn core_m_world_f_core_at(&self, c: usize, t: f64) -> Matrix {
        let core = &self.cores[c];
        let dt = t - core.time_of_last_psi;
        if core.movement_state < MT_NOT_SIM && dt != 0.0 {
            let q = math::quat_slerp(
                &core.q_last,
                &core.q_next,
                f64::from(core.i_delta_time) * dt,
            );
            let r = math::quat_to_matrix(&q);
            let mut vv = [0.0; 3];
            for k in 0..3 {
                vv[k] = core.pos_last[k] + f64::from(core.delta_pos[k]) * dt;
            }
            Matrix { r, vv }
        } else {
            core.m_world_f_core
        }
    }

    /// Pushes a core at the point `p_cs` (core coordinates) with impulse `imp_cs` (core) and `imp_ws` (world), as
    /// the change of speed and rotation (`ivp_calc_push_core`, f32).
    pub fn calc_push_core(
        &self,
        c: usize,
        p_cs: [f32; 3],
        imp_cs: [f32; 3],
        imp_ws: [f32; 3],
    ) -> ([f32; 3], [f32; 3]) {
        let core = &self.cores[c];
        let t = cross3(p_cs, imp_cs);
        let mut dw = [0.0f32; 3];
        for i in 0..3 {
            dw[i] = core.inv_rot_inertia[i] * t[i];
        }
        let dv = [
            imp_ws[0] * core.inv_mass,
            imp_ws[1] * core.inv_mass,
            imp_ws[2] * core.inv_mass,
        ];
        (dv, dw)
    }

    /// An asynchronous push of a core, into the changes that land at the next PSI (`ivp_async_push_core`).
    pub fn async_push_core(
        &mut self,
        c: usize,
        p_cs: [f32; 3],
        imp_cs: [f32; 3],
        imp_ws: [f32; 3],
    ) {
        let (dv, dw) = self.calc_push_core(c, p_cs, imp_cs, imp_ws);
        let core = &mut self.cores[c];
        for i in 0..3 {
            core.rot_speed_change[i] += dw[i];
            core.speed_change[i] += dv[i];
        }
    }

    /// A push applied to the velocities now (`ivp_push_core`).
    pub fn push_core(&mut self, c: usize, p_cs: [f32; 3], imp_cs: [f32; 3], imp_ws: [f32; 3]) {
        let (dv, dw) = self.calc_push_core(c, p_cs, imp_cs, imp_ws);
        let core = &mut self.cores[c];
        for i in 0..3 {
            core.rot_speed[i] += dw[i];
            core.speed[i] += dv[i];
        }
    }

    /// The pending changes of pushes become the velocities (`ivp_commit_pushes`).
    pub fn commit_pushes(&mut self, c: usize) {
        let core = &mut self.cores[c];
        for i in 0..3 {
            core.rot_speed[i] += core.rot_speed_change[i];
            core.speed[i] += core.speed_change[i];
            core.rot_speed_change[i] = 0.0;
            core.speed_change[i] = 0.0;
        }
    }

    /// Damping of one PSI (`0x1000c6a0`): per-axis exponential for rotation and the linear factor for speed; the
    /// small-argument branches use the upstream's linear approximations.
    fn apply_damping(&mut self, c: usize, dt: f64, rf: [f32; 3], lf: f32) {
        let a = [
            f64::from(rf[0]) * dt,
            f64::from(rf[1]) as f32 as f64 * dt,
            f64::from(rf[2]) * dt,
        ];
        let mut k = [0.0f32; 3];
        if a[0] * a[0] + a[1] * a[1] + a[2] * a[2] < 0.5 {
            for i in 0..3 {
                k[i] = (1.0 - a[i]) as f32;
            }
        } else {
            for i in 0..3 {
                k[i] = (-a[i]).exp() as f32;
            }
        }
        let b = dt * f64::from(lf);
        let kl = if b < 0.25 { 1.0 - b } else { (-b).exp() };
        let core = &mut self.cores[c];
        for i in 0..3 {
            core.rot_speed[i] *= k[i];
            core.speed[i] = (f64::from(core.speed[i]) * kl) as f32;
        }
    }

    /// The object's damping (`0x1000c610`): the factors grow by 0.1 when the core is slow or calm.
    fn damp_object(&mut self, c: usize, dt: f64) {
        if self.cores[c].movement_state >= MT_SLOW {
            let core = &self.cores[c];
            let rf = [
                core.rot_speed_damp[0] + 0.1,
                core.rot_speed_damp[1] + 0.1,
                core.rot_speed_damp[2] + 0.1,
            ];
            let lf = core.speed_damp + 0.1;
            self.apply_damping(c, dt, rf, lf);
        } else {
            let core = &self.cores[c];
            let (rf, lf) = (core.rot_speed_damp, core.speed_damp);
            self.apply_damping(c, dt, rf, lf);
        }
    }

    /// The gravity controller (`0x10012010`, priority 1000): damps the previous velocities, commits the pushes (not
    /// damped), then adds g dt.
    fn gravity_simulate(&mut self, g: [f32; 3], c: usize, dt: f64) {
        self.damp_object(c, dt);
        self.commit_pushes(c);
        let core = &mut self.cores[c];
        for k in 0..3 {
            core.speed[k] = (f64::from(core.speed[k]) + f64::from(g[k]) * dt) as f32;
        }
    }

    /// Applies one controller to its cores of a unit (`simulate` of each vtable).
    fn run_controller(&mut self, ctrl: usize, su: usize, cores: &[usize], dt: f64) {
        match self.controllers[ctrl].clone() {
            Controller::Gravity { g } => {
                for &c in cores.iter().rev() {
                    self.gravity_simulate(g, c, dt);
                }
            }
            Controller::Spring { joint } => {
                let es = crate::event_sim::EventSim {
                    delta_time: dt,
                    i_delta_time: Self::inv_dt(dt),
                    sim_unit: su,
                };
                self.spring_simulate(joint, &es);
            }
            Controller::Constraint { joint } => {
                let es = crate::event_sim::EventSim {
                    delta_time: dt,
                    i_delta_time: Self::inv_dt(dt),
                    sim_unit: su,
                };
                self.constraint_simulate(joint, &es);
            }
            Controller::Force(f) => {
                if let Some(c) = f.body {
                    self.force_simulate(c, &f);
                }
            }
            Controller::Friction { fs, kind } => {
                let es = crate::event_sim::EventSim {
                    delta_time: dt,
                    i_delta_time: Self::inv_dt(dt),
                    sim_unit: su,
                };
                match kind {
                    FrictionKind::Spring => self.fs_spring_simulate(fs, &es),
                    FrictionKind::Normal => self.fs_normal_simulate(fs, &es),
                    FrictionKind::Update => self.fs_update_simulate(fs, su),
                }
            }
        }
    }

    /// The world's gravity vector: the gravity controller's, zero without one.
    pub(crate) fn gravity_vector(&self) -> [f32; 3] {
        for c in &self.controllers {
            if let Controller::Gravity { g } = c {
                return *g;
            }
        }
        [0.0; 3]
    }

    /// Ensures a core is in simulation, reviving its unit when it is frozen (`0x1000dab0`).
    pub fn ensure_in_simulation(&mut self, c: usize) {
        if self.cores[c].unmovable {
            return;
        }
        if self.cores[c].movement_state != MT_NOT_SIM {
            let unit = self.cores[c].unit;
            let cores = self.units[unit].cores.clone();
            for k in cores {
                self.reset_freeze_check_values(k);
            }
        } else {
            let unit = self.cores[c].unit;
            self.revive_sim_unit(unit);
        }
    }

    /// Restarts the sleep timers of a core; the references are kept (`0x1000cec0`).
    pub(crate) fn reset_freeze_check_values(&mut self, c: usize) {
        let t = self.env.current_time;
        self.cores[c].time_calm_ref = [t, t];
    }

    /// Revives a core: awake, with fresh timers, and its collision state restored (`0x1000aea0`). Returns what the
    /// contact code reports (1 when the unit must restart its loop).
    fn revive_core(&mut self, c: usize) -> i32 {
        let t = self.env.current_time;
        let core = &mut self.cores[c];
        core.movement_state = MT_MOVING;
        self.reset_freeze_check_values(c);
        self.cores[c].time_of_last_psi = t;
        let next = self.env.time_of_next_psi;
        self.init_core_for_simulation(c, Self::inv_dt(next - t));
        // the broadphase of the revived core is rechecked as moving (0x1000cf20 -> 0x100099f0), then the contacts
        self.object_revived(c);
        self.core_revived(c)
    }

    /// Revives every core of a unit that is not simulated, and the unit itself (`0x10011f80`).
    pub fn revive_sim_unit(&mut self, su: usize) {
        'restart: loop {
            let cores = self.units[su].cores.clone();
            for &c in cores.iter().rev() {
                if self.cores[c].movement_state < MT_NOT_SIM {
                    continue;
                }
                if self.revive_core(c) == 1 {
                    continue 'restart;
                }
            }
            break;
        }
        if self.units[su].state == MT_NOT_SIM {
            self.unit_set_state(su, MT_MOVING);
        }
    }

    /// The movement state of a core from its sleep windows (`0x1000d680`, 10.0): moving, slow, or calm.
    fn calc_movement_state(&mut self, c: usize, t: f64) -> i8 {
        let core = &self.cores[c];
        let r = f64::from(core.upper_limit_radius);
        let d0 = [
            core.pos_last[0] - f64::from(core.pos_calm_ref[0][0]),
            core.pos_last[1] - f64::from(core.pos_calm_ref[0][1]),
            core.pos_last[2] - f64::from(core.pos_calm_ref[0][2]),
        ];
        if d0[0] * d0[0] + d0[1] * d0[1] + d0[2] * d0[2] <= 9.999_999_552_965_169e-5 {
            let dot = qdot_f(&core.q_calm_ref[0], &core.q_next);
            if (1.0 - dot * dot) * 2.0 * r * r <= 2.499_999_888_241_292_3e-5 {
                return if t - core.time_calm_ref[0] > f64::from(self.env.freeze_check_time) {
                    MT_CALM
                } else {
                    MT_SLOW
                };
            }
        }
        // The short window moved: restart it.
        let pl = self.cores[c].pos_last;
        let qn = self.cores[c].q_next;
        {
            let core = &mut self.cores[c];
            core.q_calm_ref[0] = qstore_f(&qn);
            core.pos_calm_ref[0] = [pl[0] as f32, pl[1] as f32, pl[2] as f32];
            core.time_calm_ref[0] = t;
        }
        let core = &self.cores[c];
        let d1 = [
            core.pos_last[0] - f64::from(core.pos_calm_ref[1][0]),
            core.pos_last[1] - f64::from(core.pos_calm_ref[1][1]),
            core.pos_last[2] - f64::from(core.pos_calm_ref[1][2]),
        ];
        let dot = qdot_f(&core.q_calm_ref[1], &core.q_last);
        if d1[0] * d1[0] + d1[1] * d1[1] + d1[2] * d1[2] > 0.010_000_000_298_023_226
            || (1.0 - dot * dot) * 2.0 * r * r > 0.040_000_001_192_092_904
        {
            let ql = core.q_last;
            let pl = core.pos_last;
            let core = &mut self.cores[c];
            core.q_calm_ref[1] = qstore_f(&ql);
            core.pos_calm_ref[1] = [pl[0] as f32, pl[1] as f32, pl[2] as f32];
            core.time_calm_ref[1] = t;
            return MT_MOVING;
        }
        if t - self.cores[c].time_calm_ref[1] > 4.0 {
            MT_CALM
        } else {
            MT_MOVING
        }
    }

    /// Freezes a core (`0x1000afe0`, 0x1000ce20): it stops and leaves the simulation.
    fn freeze_core(&mut self, c: usize) {
        let inv = self.env.inv_delta_psi;
        let core = &mut self.cores[c];
        core.movement_state = MT_NOT_SIM;
        core.speed = [0.0; 3];
        core.rot_speed = [0.0; 3];
        core.speed_change = [0.0; 3];
        core.rot_speed_change = [0.0; 3];
        core.delta_pos = [0.0; 3];
        self.init_core_for_simulation(c, inv);
        self.core_frozen(c);
    }

    /// Tries to put a unit to sleep: every core must be calm (`0x100120f0`). Returns whether it froze.
    fn try_to_freeze(&mut self, su: usize) -> bool {
        let t = self.env.current_time;
        let cores = self.units[su].cores.clone();
        let mut acc: i8 = 3;
        for &c in cores.iter().rev() {
            let s = self.calc_movement_state(c, t);
            self.cores[c].movement_state = s;
            acc &= s;
        }
        if acc != 3 {
            return false;
        }
        for &c in cores.iter().rev() {
            self.freeze_core(c);
        }
        self.unit_set_state(su, MT_NOT_SIM);
        true
    }

    /// The unit split (`0x100120d0`): connected components of the cores, linked by controllers that act on several
    /// cores at once. Gravity links nothing, so a unit of free bodies stays one unit until a joint joins it.
    pub fn split_or_merge(&mut self, su: usize) {
        self.units[su].changed = false;
        let cores = self.units[su].cores.clone();
        let n = cores.len();
        if n < 2 {
            return;
        }
        let mut comp = vec![-1i32; n];
        let mut ncomp = 0;
        for s in 0..n {
            if comp[s] >= 0 {
                continue;
            }
            let mut stack = vec![s];
            comp[s] = ncomp;
            while let Some(i) = stack.pop() {
                for k in 0..n {
                    if comp[k] < 0 && self.cores_linked(su, cores[i], cores[k]) {
                        comp[k] = ncomp;
                        stack.push(k);
                    }
                }
            }
            ncomp += 1;
        }
        if ncomp > 1 {
            let fast = self.units[su].fast;
            let fast_prev = self.units[su].fast_prev;
            let state = self.units[su].state;
            for e in self.units[su].entries.iter_mut() {
                e.1.clear();
            }
            self.units[su].entries.clear();
            self.units[su].cores.clear();
            let mut units = vec![su];
            for _ in 1..ncomp {
                let u = self.new_unit();
                self.units[u].state = state;
                self.units[u].fast = fast;
                self.units[u].fast_prev = fast_prev;
                self.unit_list_add(u);
                units.push(u);
            }
            for (i, &c) in cores.iter().enumerate() {
                let u = units[comp[i] as usize];
                self.unit_add_core(u, c);
                let ctrls = self.cores[c].controllers.clone();
                for k in ctrls {
                    self.unit_add_controller_core(u, k, c);
                }
            }
        }
    }

    fn cores_linked(&self, su: usize, a: usize, b: usize) -> bool {
        for (ctrl, cores) in &self.units[su].entries {
            if !self.controllers[*ctrl].links_cores() {
                continue;
            }
            if cores.contains(&a) && cores.contains(&b) {
                return true;
            }
        }
        false
    }

    /// One PSI of one unit (`0x100121b0`): sync the pose, commit the pushes, the fast test, the sleep countdown,
    /// the controllers by descending priority, the freeze test, and the collision after-controllers hook.
    fn sim_unit_psi(&mut self, su: usize, out: &mut Vec<usize>) {
        let t = self.env.current_time;
        let cores = self.units[su].cores.clone();
        let mut fast = false;
        for &c in cores.iter().rev() {
            self.sync_psi_pose(c, t);
            self.commit_pushes(c);
            self.cores[c].temporarily_unmovable = 0;
            self.cores[c].impacts_psi = 0;
            let s = self.cores[c].speed;
            let v2 = dot3(&s);
            if ((1.0 - v2) as f32) < 0.0 {
                fast = true;
            }
        }
        let test_sleep;
        if fast {
            self.units[su].fast = true;
            test_sleep = false;
        } else {
            self.units[su].fast_prev = self.units[su].fast;
            if self.units[su].fast_prev {
                for &c in cores.iter().rev() {
                    self.reset_freeze_check_values(c);
                }
            }
            self.units[su].fast = false;
            test_sleep = self.sleep_countdown();
        }
        let entries = self.units[su].entries.clone();
        for (ctrl, ecores) in entries.iter().rev() {
            self.run_controller(*ctrl, su, ecores, self.env.delta_psi);
        }
        for &c in cores.iter().rev() {
            out.push(c);
        }
        if test_sleep {
            self.try_to_freeze(su);
        }
        self.after_controllers(su);
        if self.units[su].changed {
            self.split_or_merge(su);
        }
    }

    /// The countdown of the sleep test (`0x10013610`): every 10 to 15 PSIs, a random interval.
    fn sleep_countdown(&mut self) -> bool {
        self.env.sleep_countdown = self.env.sleep_countdown.wrapping_sub(1);
        if self.env.sleep_countdown != 0 {
            return false;
        }
        let r = self.rng.next_unit();
        self.env.sleep_countdown = (15 - (r * -5.0) as i32) as u16;
        true
    }

    /// The pose of a core at this PSI (`0x10012470`).
    fn sync_psi_pose(&mut self, c: usize, t: f64) {
        let dt = t - self.cores[c].time_of_last_psi;
        let core = &mut self.cores[c];
        core.m_world_f_core.r = math::quat_to_matrix(&core.q_next);
        for k in 0..3 {
            core.m_world_f_core.vv[k] = core.pos_last[k] + f64::from(core.delta_pos[k]) * dt;
        }
    }

    /// Clamps the velocity to the maximum (`0x1002f6d0`).
    pub fn max_velocity_exceeded(&self, v: &mut [f32; 3]) {
        let k = (f64::from(self.env.max_velocity) * 0.990_000_009_536_74) / dot3(v).sqrt();
        for c in v.iter_mut() {
            *c = (f64::from(*c) * k) as f32;
        }
    }

    /// Clamps the angular velocity to the per-PSI maximum (`0x1002f720`).
    pub fn max_angular_velocity_exceeded(&self, v: &mut [f32; 3]) {
        let k = (self.env.inv_delta_psi
            * f64::from(self.env.max_angular_velocity_per_psi)
            * 0.899_999_976_158_1)
            / dot3(v).sqrt();
        for c in v.iter_mut() {
            *c = (f64::from(*c) * k) as f32;
        }
    }

    /// The rotation information of this PSI (`0x1001e870`, 8.3).
    fn calc_rotation_info(&mut self, c: usize, q: &Quat) {
        let n = q.x * q.x + q.y * q.y + q.z * q.z;
        let core = &mut self.cores[c];
        if n <= 1e-19 {
            core.abs_omega = 0.0;
            core.rotation_axis_world = [1.0, 0.0, 0.0];
        } else {
            let y = math::rsqrt(n);
            let s = y * n;
            let as_ = s
                + s * s * s * f64::from(1.0f32 / 6.0f32)
                + s * s * s * s * s * 0.404_139_995_574_951_17;
            core.abs_omega = (2.0 * as_ * f64::from(core.i_delta_time)) as f32;
            let ax = math::rmul(&core.m_world_f_core.r, &[q.x * y, q.y * y, q.z * y]);
            core.rotation_axis_world = [ax[0] as f32, ax[1] as f32, ax[2] as f32];
        }
        core.max_surface_rot_speed = core.abs_omega * core.max_surface_deviation;
    }

    /// The step of a core into the next PSI (`0x1001e300`, 8.2): clamps, the rotation of this PSI, the move to
    /// the next PSI, and the collision's integration hook.
    pub fn calc_next_psi_matrix(&mut self, c: usize, delta_time: f64, i_delta_time: f64) {
        // Angular clamp (the max rotation speed is never set in Ballance).
        let a = self.env.inv_delta_psi * f64::from(self.env.max_angular_velocity_per_psi);
        let rs = self.cores[c].rot_speed;
        if dot3(&rs) > a * a {
            let mut v = rs;
            self.max_angular_velocity_exceeded(&mut v);
            self.cores[c].rot_speed = v;
        }
        let mut v2 = dot3(&self.cores[c].speed);
        if v2 > f64::from(self.env.max_velocity) * f64::from(self.env.max_velocity) {
            let mut v = self.cores[c].speed;
            self.max_velocity_exceeded(&mut v);
            self.cores[c].speed = v;
            v2 = dot3(&v);
        }
        self.cores[c].current_speed = v2.sqrt() as f32;
        self.cores[c].i_delta_time = i_delta_time as f32;
        let mut dt = delta_time;
        let rs = self.cores[c].rot_speed;
        let q = if !self.cores[c].rot_inertias_equal && self.env.state != 5 {
            // Unequal inertias: gyroscopic sub-steps of Euler's equations without torques.
            let mut n = 1;
            let x = dot3(&rs) * dt * dt;
            if x > 1.0 / 36.0 {
                n = (x * 144.0).sqrt() as i32 + 1;
                dt /= f64::from(n);
            }
            let mut q = math::quat_from_rot_poly(&rs, dt);
            let core = &mut self.cores[c];
            let (i, ii) = (core.rot_inertia, core.inv_rot_inertia);
            let e = [
                (f64::from(i[1]) - f64::from(i[2])) * f64::from(ii[0]),
                (f64::from(i[2]) - f64::from(i[0])) * f64::from(ii[1]),
                (f64::from(i[0]) - f64::from(i[1])) * f64::from(ii[2]),
            ];
            let mut k = 1;
            loop {
                let r = core.rot_speed;
                let g = [
                    (f64::from(r[2]) * f64::from(r[1]) * e[0]) as f32,
                    (f64::from(r[2]) * f64::from(r[0]) * e[1]) as f32,
                    (f64::from(r[0]) * f64::from(r[1]) * e[2]) as f32,
                ];
                let fdt = dt as f32;
                for j in 0..3 {
                    core.rot_speed[j] += g[j] * fdt;
                }
                if k >= n {
                    break;
                }
                let s = math::quat_from_rot_poly(&core.rot_speed, dt);
                q = math::quat_mul(&s, &q);
                k += 1;
            }
            q
        } else {
            math::quat_from_rot_sin(&rs, dt)
        };
        let t = self.env.current_time;
        let core = &mut self.cores[c];
        let dtp = t - core.time_of_last_psi;
        core.time_of_last_psi = t;
        for k in 0..3 {
            core.pos_last[k] += f64::from(core.delta_pos[k]) * dtp;
        }
        core.delta_pos = core.speed;
        core.q_last = core.q_next;
        core.q_next = math::quat_mul(&core.q_next, &q);
        math::quat_normalize_iter(&mut core.q_next);
        self.calc_rotation_info(c, &q);
        self.core_integrated(c, delta_time);
    }

    /// The pre-PSI hook of the collision code and the cores' revival, then the active units, then the integration
    /// of every core that took part (`0x10013cb0`).
    pub fn simulate_psi(&mut self) {
        self.env.state = 0;
        if !self.revive.is_empty() {
            let revive = std::mem::take(&mut self.revive);
            for &c in revive.iter().rev() {
                self.ensure_in_simulation(c);
                self.cores[c].in_revive_list = false;
            }
        }
        self.psi_start();
        // The active units in list order, except the last three, which go in reverse (the loop's look-ahead).
        let units = self.active.clone();
        let nu = units.len();
        let head = nu.saturating_sub(3);
        let mut order: Vec<usize> = units[..head].to_vec();
        order.extend(units[head..].iter().rev());
        let mut cores = Vec::new();
        for su in order {
            self.sim_unit_psi(su, &mut cores);
        }
        let delta = self.env.delta_psi;
        let inv = Self::inv_dt(self.env.delta_psi);
        for &c in cores.iter().rev() {
            self.calc_next_psi_matrix(c, delta, inv);
        }
        self.env.state = 2;
        self.psi_end();
        self.env.state = 5;
    }
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn qdot_f(a: &[f32; 4], b: &Quat) -> f64 {
    f64::from(a[0]) * b.x + f64::from(a[1]) * b.y + f64::from(a[2]) * b.z + f64::from(a[3]) * b.w
}

fn qstore_f(q: &Quat) -> [f32; 4] {
    [q.x as f32, q.y as f32, q.z as f32, q.w as f32]
}

#[allow(dead_code)]
fn _unused(_: [f64; 3]) -> [f64; 3] {
    v3([0.0; 3])
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// A world with gravity (0, -20, 0) and one core of unit mass and inertia, awake.
    fn falling_core(y: f64, damp: f32) -> (World, usize) {
        let mut w = World::new();
        w.add_controller(Controller::Gravity {
            g: [0.0, -20.0, 0.0],
        });
        let c = w.core_init(
            0,
            Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            [0.0, y, 0.0],
            false,
        );
        w.cores[c].rot_inertia = [1.0; 3];
        w.cores[c].mass = 1.0;
        w.cores[c].speed_damp = damp;
        w.cores[c].rot_speed_damp = [damp; 3];
        w.cores[c].upper_limit_radius = 2.0;
        w.calc_calc(c);
        w.env.current_time = 0.0;
        w.env.time_of_next_psi = w.env.delta_psi;
        w.revive_sim_unit(w.cores[c].unit);
        (w, c)
    }

    fn run_seconds(w: &mut World, seconds: f64) {
        let steps = (seconds * 66.0).round() as usize;
        for k in 0..steps {
            w.env.current_time = (k + 1) as f64 * w.env.delta_psi;
            w.env.time_of_next_psi = w.env.current_time + w.env.delta_psi;
            w.simulate_psi();
        }
    }

    #[test]
    fn a_free_body_falls_with_the_game_gravity() {
        let (mut w, c) = falling_core(100.0, 0.0);
        run_seconds(&mut w, 1.0);
        let core = &w.cores[c];
        assert!(
            (core.speed[1] + 20.0).abs() < 0.5,
            "speed after 1 s is {}",
            core.speed[1]
        );
        // Explicit Euler over 66 PSIs: about 10 metres lower, a little less than the exact 10 (the first step
        // integrates the velocity at the start of the step).
        let fallen = 100.0 - core.pos_last[1];
        assert!((fallen - 10.0).abs() < 1.0, "fell {fallen} m in 1 s");
    }

    #[test]
    fn linear_damping_decays_the_speed_by_its_factor() {
        let (mut w, c) = falling_core(1000.0, 0.0);
        w.cores[c].speed = [0.0, 0.0, 0.0];
        w.cores[c].speed_damp = 1.0;
        w.cores[c].speed[0] = 10.0;
        w.cores[c].movement_state = MT_MOVING;
        run_seconds(&mut w, 0.5);
        let x = f64::from(w.cores[c].speed[0]);
        // The factor per second is exp(-1); the gravity is along y only, so x just decays.
        let want = 10.0 * (-0.5f64).exp();
        assert!(
            (x - want).abs() / want < 0.05,
            "speed x {x}, want about {want}"
        );
    }

    #[test]
    fn separate_bodies_start_in_separate_units() {
        let mut w = World::new();
        w.add_controller(Controller::Gravity {
            g: [0.0, -20.0, 0.0],
        });
        let a = w.core_init(
            0,
            Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            [0.0; 3],
            false,
        );
        let b = w.core_init(
            1,
            Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            [5.0, 0.0, 0.0],
            false,
        );
        // Each core starts in its own unit; a unit only grows when a controller joins cores (joints do).
        let ua = w.cores[a].unit;
        let ub = w.cores[b].unit;
        assert_ne!(ua, ub);
        assert!(w.units[ua].cores.len() == 1 && w.units[ub].cores.len() == 1);
    }
}
