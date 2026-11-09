// Particles, as the original's TT_ParticleSystems_RT building blocks make them (docs/particles.md; ported from the C
// reference's ck_particles.c): an emitter frame spawns particles every emission delay, each moving in a straight
// line in world space while its colour and size change linearly over its life; they are drawn as camera-facing
// quads, triangles, streaks or lines, blended, unlit and without writing depth. The game's emitters and their values
// are the scripts' (the table below, from the module files); this is only for the eye and never touches the run.

use glam::{Mat4, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Point,
    TimeDependent,
    Planar,
    Spherical,
}

/// How the particles are drawn (the block's render mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Render {
    Line,
    Sprite,
    Orientable,
    Fast,
}

/// Blending against the frame: additive with alpha (5/2), or one / source colour (2/3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blend {
    AlphaAdd,
    OneSrcColor,
}

const VAR_SPEED: u32 = 1;
const VAR_LIFE: u32 = 4;
const VAR_EMISSION: u32 = 8;
const VAR_SIZE0: u32 = 0x10;
const VAR_SIZE1: u32 = 0x20;
const EVOL_SIZE: u32 = 1;
const EVOL_COLOR: u32 = 2;

/// One of the game's particle systems: the emitter frame it belongs to and the block's values.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub frame: &'static str,
    pub kind: Kind,
    pub render: Render,
    pub blend: Blend,
    pub evolutions: u32,
    pub variances: u32,
    /// Milliseconds between emissions, and how many each time (± variance).
    pub delay: f32,
    pub emission: i32,
    pub emission_var: i32,
    pub max: usize,
    /// Units a millisecond.
    pub speed: f32,
    pub speed_var: f32,
    pub yaw_var: f32,
    pub pitch_var: f32,
    /// Milliseconds.
    pub life: f32,
    pub life_var: f32,
    pub size0: f32,
    pub size0_var: f32,
    pub size1: f32,
    pub size1_var: f32,
    pub color0: [f32; 4],
    pub color1: [f32; 4],
    /// The texture's name in the game's particle textures (`Textures/Particles`), none for untextured lines.
    pub texture: Option<&'static str>,
    /// A time-dependent system's trail: particles a second along the frame's path.
    pub trail_rate: f32,
}

const FLAME: [f32; 4] = [0.921_569, 0.164_706, 0.274_51, 1.0];
const DEG3: f32 = 0.052_359_9;
const FULL: f32 = std::f32::consts::TAU;

const fn spec(frame: &'static str) -> Spec {
    Spec {
        frame,
        kind: Kind::Point,
        render: Render::Fast,
        blend: Blend::AlphaAdd,
        evolutions: 3,
        variances: 29,
        delay: 20.0,
        emission: 1,
        emission_var: 1,
        max: 50,
        speed: 0.008,
        speed_var: 0.003,
        yaw_var: DEG3,
        pitch_var: DEG3,
        life: 1000.0,
        life_var: 250.0,
        size0: 3.0,
        size0_var: 0.3,
        size1: 0.1,
        size1_var: 0.0,
        color0: FLAME,
        color1: [0.0; 4],
        texture: Some("Particle_Flames"),
        trail_rate: 0.0,
    }
}

/// The game's particle systems by emitter frame (docs/particles.md, 2.1, with the module files' pins).
pub const SPECS: &[Spec] = &[
    spec("PS_FourFlames_Flame_A"),
    spec("PS_FourFlames_Flame_B"),
    Spec {
        max: 90,
        ..spec("PS_FourFlames_Flame_C")
    },
    spec("PS_FourFlames_Flame_D"),
    spec("PC_TwoFlames_Flame_SmallA"),
    spec("PC_TwoFlames_Flame_SmallB"),
    Spec {
        speed: 0.012,
        speed_var: 0.004,
        life_var: 500.0,
        size0: 5.0,
        color0: [FLAME[0], FLAME[1], FLAME[2], 0.98],
        ..spec("PC_TwoFlames_Flame_Big")
    },
    // the fan's wind: streaks, and faint smoke
    Spec {
        kind: Kind::Planar,
        render: Render::Line,
        evolutions: 2,
        variances: 4,
        delay: 0.0,
        emission: 3,
        emission_var: 0,
        max: 100,
        speed: 0.04,
        speed_var: 0.0,
        yaw_var: 0.0,
        pitch_var: 0.0,
        life: 400.0,
        life_var: 10.0,
        size0: 4.0,
        size0_var: 0.0,
        color0: [1.0, 1.0, 1.0, 0.235],
        texture: None,
        ..spec("P_Modul_18_Particle")
    },
    Spec {
        kind: Kind::Planar,
        render: Render::Sprite,
        variances: 12,
        max: 40,
        speed: 0.036,
        speed_var: 0.0,
        yaw_var: 0.0,
        pitch_var: 0.0,
        life: 800.0,
        life_var: 10.0,
        size0: 2.3,
        size0_var: 0.0,
        size1: 3.0,
        color0: [1.0, 1.0, 1.0, 0.118],
        texture: Some("Particle_Smoke"),
        ..spec("P_Modul_18_Particle")
    },
    // the extra life's blob and fizz
    Spec {
        kind: Kind::Spherical,
        render: Render::Orientable,
        variances: 53,
        delay: 200.0,
        emission: 10,
        emission_var: 0,
        max: 20,
        speed: 0.01,
        speed_var: 0.004,
        life: 150.0,
        life_var: 100.0,
        size0: 1.0,
        size0_var: 0.6,
        size1: 0.5,
        size1_var: 0.1,
        color0: [1.0; 4],
        color1: [1.0, 1.0, 1.0, 0.392],
        texture: Some("P_Extra_Life_Particle"),
        ..spec("P_Extra_Life_Particle_Blob")
    },
    Spec {
        kind: Kind::Spherical,
        render: Render::Line,
        evolutions: 2,
        variances: 5,
        delay: 200.0,
        emission: 50,
        emission_var: 0,
        max: 100,
        speed: 0.01,
        speed_var: 0.005,
        life: 200.0,
        life_var: 200.0,
        color0: [0.816, 0.69, 0.835, 1.0],
        color1: [0.816, 0.69, 0.835, 1.0],
        texture: None,
        ..spec("P_Extra_Life_Particle_Fizz")
    },
];

/// The extra points' particles: the burst at the centre when one fires, the trails of the small balls, the bursts
/// where they hit.
/// The ball's smoke (Balls.nmo's BallParticle_Frame script): one puff around the ball when it appears and when a
/// transformer bursts the old one; its frame is the ball's, scaled 1.5. On for six frames, the pool of 60 is full
/// after three.
pub const BALL_SMOKE: Spec = Spec {
    kind: Kind::Spherical,
    render: Render::Sprite,
    blend: Blend::AlphaAdd,
    evolutions: 3,
    variances: 5,
    delay: 0.0,
    emission: 20,
    emission_var: 0,
    max: 60,
    speed: 0.001,
    speed_var: 0.0005,
    yaw_var: 0.349,
    pitch_var: 0.349,
    life: 1600.0,
    life_var: 1000.0,
    size0: 2.0,
    size0_var: 0.0,
    size1: 3.0,
    size1_var: 0.0,
    color0: [0.784; 4],
    color1: [0.0; 4],
    texture: Some("Particle_Smoke"),
    ..spec("Ball_Particle_Frame")
};

pub const EXTRA_BURST: Spec = Spec {
    blend: Blend::OneSrcColor,
    variances: 4,
    delay: 0.0,
    emission: 40,
    emission_var: 0,
    max: 100,
    speed: 0.005,
    speed_var: 0.0,
    yaw_var: FULL,
    pitch_var: FULL,
    life: 400.0,
    life_var: 0.0,
    size0: 1.0,
    size0_var: 0.0,
    size1: 0.0,
    color0: [1.0; 4],
    color1: [0.392; 4],
    texture: Some("ExtraBall"),
    ..spec("P_Extra_Point_Frame0")
};
pub const EXTRA_HIT: Spec = Spec {
    render: Render::Sprite,
    speed: 0.003,
    size0: 0.5,
    color1: [1.0; 4],
    ..EXTRA_BURST
};
pub const EXTRA_TRAIL: Spec = Spec {
    kind: Kind::TimeDependent,
    variances: 0,
    delay: 90.0,
    emission: 1,
    emission_var: 0,
    max: 100,
    speed: 0.0,
    speed_var: 0.0,
    yaw_var: 0.0,
    pitch_var: 0.0,
    life: 1000.0,
    life_var: 0.0,
    size0: 0.5,
    size0_var: 0.0,
    size1: 0.2,
    color0: [1.0; 4],
    color1: [0.157; 4],
    texture: Some("ExtraParticle"),
    trail_rate: 90.0,
    ..spec("P_Extra_Point_Frame1")
};

#[derive(Clone, Copy, Debug)]
struct Particle {
    pos: Vec3,
    vel: Vec3,
    color: [f32; 4],
    dcolor: [f32; 4],
    size: f32,
    dsize: f32,
    life: f32,
    last_dt: f32,
}

/// One emitter: its values, the frame it is at, and its particles.
#[derive(Clone, Debug)]
pub struct System {
    pub spec: Spec,
    pub frame: Mat4,
    pub emitting: bool,
    particles: Vec<Particle>,
    /// Emission events still to come, one each tick (a block switched on and off again a frame or two later).
    pub pulses: u32,
    elapsed: f32,
    trail: Option<(Vec3, f32)>,
    rand: u32,
}

impl System {
    pub fn new(spec: Spec, frame: Mat4, seed: u32) -> System {
        System {
            spec,
            frame,
            emitting: false,
            particles: Vec::new(),
            pulses: 0,
            elapsed: 0.0,
            trail: None,
            rand: seed,
        }
    }

    /// The MSVC rand() the original's blocks use: 0..32767.
    fn rand(&mut self) -> i32 {
        self.rand = self.rand.wrapping_mul(214_013).wrapping_add(2_531_011);
        ((self.rand >> 16) & 0x7fff) as i32
    }

    /// R(x): a value in [-x, x], as the block draws its variances.
    fn r(&mut self, x: f32) -> f32 {
        (self.rand() - 0x3fff) as f32 * 6.103_890_3e-5 * x
    }

    fn var(&mut self, bit: u32, x: f32) -> f32 {
        if self.spec.variances & bit != 0 {
            self.r(x)
        } else {
            0.0
        }
    }

    /// One emission event: `emission` particles (± its variance) where the frame is.
    pub fn burst(&mut self) {
        let n = self.spec.emission + self.var(VAR_EMISSION, self.spec.emission_var as f32) as i32;
        for _ in 0..n.max(0) {
            if self.particles.len() >= self.spec.max {
                break;
            }
            let (pos, dir) = self.place();
            self.spawn(pos, dir);
        }
    }

    /// Where a new particle starts and the way it goes, as the emitter's shape says.
    fn place(&mut self) -> (Vec3, Vec3) {
        let f = self.frame;
        match self.spec.kind {
            Kind::Planar => {
                let (a, b) = (self.r(1.0), self.r(1.0));
                (f.transform_point3(Vec3::new(b, a, 0.0)), self.direction())
            }
            Kind::Spherical => {
                let (a, b, c) = (self.r(1.0), self.r(1.0), self.r(1.0));
                let v = Vec3::new(c, b, a).normalize_or_zero();
                (
                    f.transform_point3(v),
                    f.transform_vector3(v).normalize_or_zero(),
                )
            }
            _ => (f.w_axis.truncate(), self.direction()),
        }
    }

    /// The jittered direction (not normalised: the frame's scale scales the speed).
    fn direction(&mut self) -> Vec3 {
        let cp = self.r(self.spec.pitch_var).cos();
        let x = self.r(self.spec.yaw_var).sin() * cp;
        let z = self.r(self.spec.yaw_var).cos() * cp;
        let y = self.r(self.spec.pitch_var).sin();
        self.frame.transform_vector3(Vec3::new(x, y, z))
    }

    fn spawn(&mut self, pos: Vec3, dir: Vec3) {
        let s = self.spec;
        let speed = s.speed + self.var(VAR_SPEED, s.speed_var);
        let life = (s.life + self.var(VAR_LIFE, s.life_var)).max(1.0);
        let size = s.size0 + self.var(VAR_SIZE0, s.size0_var);
        let end = s.size1 + self.var(VAR_SIZE1, s.size1_var);
        let mut dcolor = [0.0; 4];
        if s.evolutions & EVOL_COLOR != 0 {
            for c in 0..4 {
                dcolor[c] = (s.color1[c] - s.color0[c]) / life;
            }
        }
        let dsize = if s.evolutions & EVOL_SIZE != 0 {
            (end - size) / life
        } else {
            0.0
        };
        self.particles.push(Particle {
            pos,
            vel: dir * speed,
            color: s.color0,
            dcolor,
            size,
            dsize,
            life,
            last_dt: 0.01,
        });
    }

    /// `dt` milliseconds: emission while emitting, then every particle moves, ages and changes.
    pub fn tick(&mut self, dt: f32) {
        if self.pulses > 0 {
            self.pulses -= 1;
            self.burst();
        }
        if self.emitting {
            if self.spec.kind == Kind::TimeDependent {
                self.trail(dt);
            }
            let mut t = self.elapsed + dt;
            let d = self.spec.delay;
            if d <= 0.0 {
                self.burst();
            } else {
                while d < t {
                    t -= d;
                    self.burst();
                }
            }
            self.elapsed = t;
        }
        for p in &mut self.particles {
            let h = dt.min(p.life);
            p.pos += p.vel * h;
            for c in 0..4 {
                p.color[c] = (p.color[c] + p.dcolor[c] * h).clamp(0.0, 1.0);
            }
            p.size += p.dsize * h;
            p.life -= h;
            p.last_dt = h;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    /// A time-dependent system's trail: particles spread along the way the frame went since the last trail.
    fn trail(&mut self, dt: f32) {
        let cur = self.frame.w_axis.truncate();
        let Some((prev, acc)) = self.trail else {
            self.trail = Some((cur, 0.0));
            return;
        };
        let k = (acc + dt) * 0.001;
        let n = (self.spec.trail_rate * k) as i32;
        if self.spec.trail_rate * k > 0.9 {
            let n = n.max(1);
            let step = (prev - cur) / n as f32;
            for i in 0..n {
                if self.particles.len() >= self.spec.max {
                    break;
                }
                let dir = self.direction();
                self.spawn(cur + step * i as f32, dir);
            }
            self.trail = Some((cur, 0.0));
        } else {
            self.trail = Some((prev, acc + dt));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Switched on (a script activated with reset: it starts over) or off (deactivated: its particles go at once).
    pub fn set_active(&mut self, on: bool) {
        if on == self.emitting {
            return;
        }
        self.emitting = on;
        self.particles.clear();
        // On starts the clock at the delay, so the first emission comes at once
        self.elapsed = if on { self.spec.delay } else { 0.0 };
        self.trail = None;
    }

    /// The particles as world triangles (or lines, for line systems): position, texture coordinates, colour; with
    /// the view's right, up and forward axes.
    pub fn geometry(&self, right: Vec3, up: Vec3, forward: Vec3, out: &mut Vec<Vertex>) {
        for p in &self.particles {
            let c = p.color;
            match self.spec.render {
                Render::Sprite | Render::Fast => {
                    let (r, u) = (right * p.size * 0.5, up * p.size * 0.5);
                    let (a, b, d, e) = (p.pos - r + u, p.pos + r + u, p.pos + r - u, p.pos - r - u);
                    if self.spec.render == Render::Fast {
                        out.extend([
                            vtx(p.pos + u, [0.5, 0.0], c),
                            vtx(d, [1.0, 1.0], c),
                            vtx(e, [0.0, 1.0], c),
                        ]);
                    } else {
                        out.extend([
                            vtx(a, [0.0, 0.0], c),
                            vtx(b, [1.0, 0.0], c),
                            vtx(d, [1.0, 1.0], c),
                            vtx(a, [0.0, 0.0], c),
                            vtx(d, [1.0, 1.0], c),
                            vtx(e, [0.0, 1.0], c),
                        ]);
                    }
                }
                Render::Orientable => {
                    let t = p.pos - p.vel * p.last_dt;
                    let h = p.pos;
                    let d = (h - t).normalize_or_zero();
                    let s = forward.cross(d).normalize_or_zero();
                    let (sz, s0) = (p.size, p.size - p.dsize * p.last_dt);
                    let c0 = fade(c, p.dcolor, p.last_dt);
                    let q = [
                        h + d * sz + s * sz,
                        h + d * sz - s * sz,
                        t - d * s0 - s * s0,
                        t - d * s0 + s * s0,
                    ];
                    out.extend([
                        vtx(q[0], [0.0, 0.0], c),
                        vtx(q[1], [1.0, 0.0], c),
                        vtx(q[2], [1.0, 1.0], c0),
                        vtx(q[0], [0.0, 0.0], c),
                        vtx(q[2], [1.0, 1.0], c0),
                        vtx(q[3], [0.0, 1.0], c0),
                    ]);
                }
                Render::Line => {
                    let a = p.pos - p.vel * p.last_dt;
                    out.extend([
                        vtx(a, [0.0; 2], fade(c, p.dcolor, p.last_dt)),
                        vtx(p.pos, [0.0; 2], c),
                    ]);
                }
            }
        }
    }
}

fn fade(c: [f32; 4], d: [f32; 4], dt: f32) -> [f32; 4] {
    [
        c[0] - d[0] * dt,
        c[1] - d[1] * dt,
        c[2] - d[2] * dt,
        c[3] - d[3] * dt,
    ]
}

/// A particle vertex for the renderer.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

fn vtx(p: Vec3, uv: [f32; 2], color: [f32; 4]) -> Vertex {
    Vertex {
        pos: p.to_array(),
        uv,
        color: color.map(|c| c.clamp(0.0, 1.0)),
    }
}

/// The names of every particle texture the systems use (files in the game's `Textures`).
pub fn textures() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for t in SPECS
        .iter()
        .chain([&EXTRA_BURST, &EXTRA_HIT, &EXTRA_TRAIL, &BALL_SMOKE])
        .filter_map(|s| s.texture)
    {
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// Particles to draw with one texture and blend: triangles, or lines when `lines`.
pub struct Batch {
    pub texture: Option<&'static str>,
    pub blend: Blend,
    pub lines: bool,
    pub vertices: Vec<Vertex>,
}

/// When a level emitter burns, as its module's script switches it.
#[derive(Clone, Copy, Debug)]
enum When {
    /// The ball within `NEAR` of the module (the level start's four flames).
    Near(Vec3),
    /// Always (the menu scene's flames).
    Always,
    /// The checkpoint's big flame: near, until the checkpoint (the level object) is passed.
    Before(usize, Vec3),
    /// The checkpoint's two small flames: near, once it is passed.
    After(usize, Vec3),
    /// A fan's wind: while its updraft is switched on and the ball is within `FAN_NEAR`; the frame spins about the
    /// rotor's axis (the script's Rotate, `FAN_SPIN` a second) from `base` by `angle`.
    Fan { base: Mat4, axis: Vec3, angle: f32 },
    /// An extra life's blob and fizz: one emission when the ball takes it (the level object; its script switches
    /// them on and, five frames later, off again, within one emission delay).
    Taken { object: usize, done: bool },
    /// The ball's smoke: a puff where the game says (`Sim::smoke`).
    Smoke,
}

/// The modules' proximity (TT Scaleable Proximity, Distance 70).
const NEAR: f32 = 70.0;
/// Module 18's proximity (80) and its particle frame's spin (Per Second -15 into Rotate, radians).
const FAN_NEAR: f32 = 80.0;
const FAN_SPIN: f32 = -15.0;

/// The particle systems of an extra point: its burst when it fires, the six small balls' trails, their hits.
struct ExtraFx {
    object: usize,
    fired: bool,
    burst: System,
    trails: Vec<System>,
    hits: Vec<System>,
    last: Vec<(usize, Vec3)>,
}

/// Every particle system of a level, switched as the original's scripts switch them.
pub struct Effects {
    emitters: Vec<(System, When)>,
    extras: Vec<ExtraFx>,
    seed: u32,
}

impl Effects {
    /// The emitters of the level's flame modules (`PS_FourFlames` at the start, `PC_TwoFlames` at each
    /// checkpoint); `module` gives a module file's level by name, if the copy has it.
    pub fn new(level: &level::Level, module: &dyn Fn(&str) -> Option<level::Level>) -> Effects {
        let mut fx = Effects {
            emitters: Vec::new(),
            extras: Vec::new(),
            seed: 1,
        };
        let seed = fx.next_seed();
        fx.emitters
            .push((System::new(BALL_SMOKE, Mat4::IDENTITY, seed), When::Smoke));
        for (group, file) in [
            ("PS_Levelstart", "PS_FourFlames"),
            ("PC_Checkpoints", "PC_TwoFlames"),
            ("P_Modul_18", "P_Modul_18"),
            ("P_Extra_Life", "P_Extra_Life"),
        ] {
            if level.group(group).is_none() {
                continue;
            }
            let Some(m) = module(file) else { continue };
            let named = |n: &str| m.objects.iter().find(|o| o.name.eq_ignore_ascii_case(n));
            let root = named(&format!("{file}_MF"))
                .map_or(Mat4::IDENTITY, |o| Mat4::from_cols_array_2d(&o.matrix));
            let rotor =
                named(&format!("{file}_Rotor")).map(|o| Mat4::from_cols_array_2d(&o.matrix));
            for (i, o) in level.members(group) {
                let place = Mat4::from_cols_array_2d(&o.matrix);
                let at = place.w_axis.truncate();
                for spec in SPECS.iter().filter(|s| s.frame.starts_with(file)) {
                    let Some(f) = m
                        .objects
                        .iter()
                        .find(|f| f.name.eq_ignore_ascii_case(spec.frame))
                    else {
                        continue;
                    };
                    let frame = place * root.inverse() * Mat4::from_cols_array_2d(&f.matrix);
                    let when = match spec.frame {
                        f if f.starts_with("P_Modul_18") => When::Fan {
                            base: frame,
                            axis: rotor.map_or(Vec3::Y, |r| {
                                (place * root.inverse() * r).y_axis.truncate()
                            }),
                            angle: 0.0,
                        },
                        f if f.starts_with("P_Extra_Life") => When::Taken {
                            object: i,
                            done: false,
                        },
                        f if f.ends_with("_Big") => When::Before(i, at),
                        f if f.starts_with("PC_") => When::After(i, at),
                        _ => When::Near(at),
                    };
                    let seed = fx.next_seed();
                    fx.emitters.push((System::new(*spec, frame, seed), when));
                }
            }
        }
        fx
    }

    /// The menu scene's flames: the start's four flames at its frames `FourFlames_Flame_A..D` (or with the `PS_`
    /// prefix), burning all the time.
    pub fn menu(level: &level::Level) -> Effects {
        let mut fx = Effects {
            emitters: Vec::new(),
            extras: Vec::new(),
            seed: 7,
        };
        for spec in SPECS
            .iter()
            .filter(|s| s.frame.starts_with("PS_FourFlames_Flame_"))
        {
            let short = spec.frame.trim_start_matches("PS_");
            let Some(o) = level.objects.iter().find(|o| {
                o.name.trim().eq_ignore_ascii_case(short)
                    || o.name.trim().eq_ignore_ascii_case(spec.frame)
            }) else {
                continue;
            };
            let seed = fx.next_seed();
            let mut s = System::new(*spec, Mat4::from_cols_array_2d(&o.matrix), seed);
            s.set_active(true);
            fx.emitters.push((s, When::Always));
        }
        fx
    }

    /// The menu's flames go on by `dt` milliseconds.
    pub fn tick_menu(&mut self, dt: f32) {
        for (s, _) in &mut self.emitters {
            s.tick(dt);
        }
    }

    /// Whether the level has any flames (their stand-ins need no markers then).
    pub fn has_flames(&self) -> bool {
        self.emitters
            .iter()
            .any(|e| matches!(e.1, When::Near(_) | When::Before(..) | When::After(..)))
    }

    fn next_seed(&mut self) -> u32 {
        self.seed = self.seed.wrapping_add(0x9E37_79B9);
        self.seed
    }

    /// One tick of the game: switch the emitters as the scripts would, then move every particle.
    pub fn tick(&mut self, sim: &crate::sim::Sim) {
        let ball = sim.ball.position;
        let dt = crate::sim::TICK * 1000.0;
        let fans = sim.fans();
        for (s, when) in &mut self.emitters {
            let on = match when {
                When::Near(at) => at.distance(ball) < NEAR,
                When::Always => true,
                When::Before(o, at) => !sim.checkpoint_passed(*o) && at.distance(ball) < NEAR,
                When::After(o, at) => sim.checkpoint_passed(*o) && at.distance(ball) < NEAR,
                When::Fan { base, axis, angle } => {
                    let at = base.w_axis.truncate();
                    *angle = (*angle + FAN_SPIN * crate::sim::TICK) % std::f32::consts::TAU;
                    let turn = Mat4::from_axis_angle(axis.normalize_or(Vec3::Y), *angle);
                    s.frame =
                        Mat4::from_translation(at) * turn * Mat4::from_translation(-at) * *base;
                    fans.iter().any(|p| p.distance(at) < 1.0) && at.distance(ball) < FAN_NEAR
                }
                When::Smoke => {
                    if let Some(p) = sim.smoke() {
                        s.frame = Mat4::from_scale_rotation_translation(
                            Vec3::splat(1.5),
                            glam::Quat::IDENTITY,
                            p,
                        );
                        // on for six frames: three emissions fill the pool of sixty
                        s.pulses = 3;
                    }
                    false
                }
                When::Taken { object, done } => {
                    if !*done && sim.taken.contains(object) {
                        *done = true;
                        s.pulses = 1;
                    }
                    false
                }
            };
            s.set_active(on);
            s.tick(dt);
        }
        for v in sim.extra_views() {
            if !self.extras.iter().any(|e| e.object == v.object) {
                let at = Mat4::from_translation(v.center);
                let (a, b) = (self.next_seed(), self.next_seed());
                let trails = (0..6)
                    .map(|k| System::new(EXTRA_TRAIL, at, b.wrapping_add(k)))
                    .collect();
                self.extras.push(ExtraFx {
                    object: v.object,
                    fired: false,
                    burst: System::new(EXTRA_BURST, at, a),
                    trails,
                    hits: Vec::new(),
                    last: Vec::new(),
                });
            }
            let Some(e) = self.extras.iter_mut().find(|e| e.object == v.object) else {
                continue;
            };
            if v.fired && !e.fired {
                // TT Extra switches the burst on; a link two frames later switches it off
                e.burst.pulses = 2;
            }
            if v.fired {
                for &(k, p) in &e.last {
                    if !v.balls.iter().any(|b| b.0 == k) {
                        let mut h = System::new(
                            EXTRA_HIT,
                            Mat4::from_translation(p),
                            e.burst.rand.wrapping_add(k as u32),
                        );
                        h.pulses = 1;
                        e.hits.push(h);
                    }
                }
            }
            e.fired = v.fired;
            for (k, t) in e.trails.iter_mut().enumerate() {
                match v.balls.iter().find(|b| b.0 == k) {
                    Some(&(_, p)) => {
                        t.frame = Mat4::from_translation(p);
                        t.emitting = true;
                    }
                    None => t.emitting = false,
                }
            }
            e.last.clone_from(&v.balls);
            e.burst.tick(dt);
            for t in &mut e.trails {
                t.tick(dt);
            }
            for h in &mut e.hits {
                h.tick(dt);
            }
            e.hits.retain(|h| h.pulses > 0 || !h.is_empty());
        }
    }

    /// The particles seen from `eye` towards `target`, batched by texture, blend and kind.
    pub fn batches(&self, eye: Vec3, target: Vec3) -> Vec<Batch> {
        let forward = (target - eye).normalize_or(Vec3::Z);
        let right = Vec3::Y.cross(forward).normalize_or(Vec3::X);
        let up = forward.cross(right).normalize_or(Vec3::Y);
        let mut out: Vec<Batch> = Vec::new();
        let systems = self.emitters.iter().map(|e| &e.0).chain(
            self.extras
                .iter()
                .flat_map(|e| std::iter::once(&e.burst).chain(&e.trails).chain(&e.hits)),
        );
        for s in systems.filter(|s| !s.is_empty()) {
            let lines = s.spec.render == Render::Line;
            let at = match out.iter().position(|b| {
                b.texture == s.spec.texture && b.blend == s.spec.blend && b.lines == lines
            }) {
                Some(i) => i,
                None => {
                    out.push(Batch {
                        texture: s.spec.texture,
                        blend: s.spec.blend,
                        lines,
                        vertices: Vec::new(),
                    });
                    out.len() - 1
                }
            };
            if let Some(b) = out.get_mut(at) {
                s.geometry(right, up, forward, &mut b.vertices);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flame_keeps_burning_within_its_pool_and_dies_out_when_stopped() {
        let mut s = System::new(SPECS[0], Mat4::IDENTITY, 1);
        s.emitting = true;
        for _ in 0..600 {
            s.tick(1000.0 / 60.0);
        }
        assert!(!s.is_empty() && s.particles.len() <= s.spec.max);
        s.emitting = false;
        for _ in 0..120 {
            s.tick(1000.0 / 60.0);
        }
        assert!(s.is_empty(), "every particle lives at most 1250 ms");
    }
}
