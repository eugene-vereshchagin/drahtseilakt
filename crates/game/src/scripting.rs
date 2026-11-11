// A community map's own scripts in a run: the script runtime (crates/script) driven once a tick before the
// physics, and the engine's side of it (the `Host`): objects hidden, moved or deleted by scripts are kept here
// as overlays on the level (the renderer and the tests read them), the ball is teleported through the physics,
// the game's own tables (`CurrentLevel`) are answered from the run, and what a script did or could not do goes
// to its log. Phase 1 (docs/drahtseilakt/scripts.md): no physics for scripted objects beyond the ball.

use glam::{Mat4, Vec3};
use level::Level;
use script::{CameraChange, CameraState, Ent, Host, Runtime, SoundCmd, Value};
use std::collections::{BTreeMap, HashMap};

/// How many log lines a run keeps.
const LOG_CAP: usize = 2_000;
/// The most objects a run holds with the scripts' copies (the runtime caps the copies well below).
const MAX_OBJECTS: usize = 1 << 20;

#[derive(Clone)]
pub struct Scripts {
    pub rt: Runtime,
    /// Objects a script hid (or deleted), by level object index.
    pub hidden: Vec<bool>,
    /// Objects a script moved: their world matrix now.
    pub moved: Vec<Option<Mat4>>,
    /// Each object's bounding box in its own frame, for the intersection blocks.
    local_bounds: Vec<Option<(Vec3, Vec3)>>,
    /// The level's objects as loaded: names, matrices (the initial conditions), meshes; and its groups' members.
    names: Vec<String>,
    initial: Vec<[[f32; 4]; 4]>,
    meshes: Vec<Option<usize>>,
    groups: Vec<Vec<usize>>,
    /// Each level mesh's materials in its order, and each material's diffuse and emissive colour (read only).
    materials: std::sync::Arc<Materials>,
    /// The log: (tick, what happened), capped.
    pub log: Vec<(u64, String)>,
    /// The ball's pseudo-object in the graph (`Ball_Pos_Frame`), if a script can name it.
    ball_id: Option<script::Id>,
    pub teleports: u32,
    /// The map's sounds: what plays in the run's own clock, and what the audio is to do about it.
    pub sound: SoundState,
    /// The camera the scripts changed (Set As Active Camera, Set Clipping Planes, ...).
    pub camera: CameraOverride,
}

/// The most sound events kept for the audio between two drains (a run with no audio never drains them).
const SOUND_EVENT_CAP: usize = 1_024;
/// The most voices a map's scripts hold at once in the audio (instances and wave players together).
pub const MAX_SCRIPT_VOICES: usize = 16;

/// What the scripts changed of the camera for this run: a field of view in degrees, clipping planes, and the camera
/// object the view is attached to. None of it reaches the simulation; a new run starts without it.
#[derive(Debug, Default, Clone, Copy)]
pub struct CameraOverride {
    pub fov: Option<f32>,
    pub clip: Option<(f32, f32)>,
    pub active: Option<usize>,
}

/// What the audio is to do for a script: the wave sound object, its file and the command.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundEvent {
    pub key: usize,
    pub file: String,
    pub cmd: SoundCmd,
}

/// A wave sound that plays in the run's clock: until a tick (None: looping), or paused with this many ticks left.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Playing {
    until: Option<u64>,
    paused: Option<u64>,
}

/// The sounds of a run. `playing` is what a script can see (the Wave Player's End Playing); it follows the sound's
/// length in ticks, never the audio device, so a replay is the same with sound on or off, with a file or without.
#[derive(Debug, Clone, Default)]
pub struct SoundState {
    /// Each sound file's length in ticks (by lower-case file stem), set by the app from the sound set; a file not in
    /// it is silent and ends at once.
    pub ticks: std::sync::Arc<HashMap<String, u64>>,
    playing: BTreeMap<usize, Playing>,
    /// Files already noted as silent.
    noted: Vec<String>,
    /// For the audio, drained by the app each frame.
    pub events: Vec<SoundEvent>,
}

/// A sound file's name as the sound set knows it: the last part of the path, no extension (the length table is
/// keyed by it in lower case).
pub fn sound_stem(file: &str) -> String {
    let base = file.rsplit(['/', '\\']).next().unwrap_or(file);
    base.rsplit_once('.').map_or(base, |(s, _)| s).to_string()
}

impl Scripts {
    /// Each sound file the map's wave objects name (by stem), for the app to measure.
    pub fn sound_files(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .rt
            .graph
            .waves
            .iter()
            .map(|w| sound_stem(&w.file))
            .filter(|s| !s.is_empty())
            .collect();
        v.sort();
        v.dedup();
        v
    }
}

impl Scripts {
    pub fn new(level: &Level) -> Option<Scripts> {
        let graph = level.scripts.clone()?;
        let ball_id = graph.find("Ball_Pos_Frame");
        let local_bounds = level
            .objects
            .iter()
            .map(|o| {
                let mesh = level.meshes.get(o.mesh?)?;
                let mut lo = Vec3::splat(f32::INFINITY);
                let mut hi = Vec3::splat(f32::NEG_INFINITY);
                for p in &mesh.positions {
                    lo = lo.min(Vec3::from(*p));
                    hi = hi.max(Vec3::from(*p));
                }
                (lo.is_finite() && hi.is_finite()).then_some((lo, hi))
            })
            .collect();
        Some(Scripts {
            rt: Runtime::new(graph),
            hidden: vec![false; level.objects.len()],
            moved: vec![None; level.objects.len()],
            local_bounds,
            names: level.objects.iter().map(|o| o.name.clone()).collect(),
            initial: level.objects.iter().map(|o| o.matrix).collect(),
            meshes: level.objects.iter().map(|o| o.mesh).collect(),
            groups: level.groups.iter().map(|g| g.members.clone()).collect(),
            materials: std::sync::Arc::new(Materials {
                of_mesh: level
                    .meshes
                    .iter()
                    .map(|m| m.parts.iter().filter_map(|p| p.material).collect())
                    .collect(),
                colours: level
                    .materials
                    .iter()
                    .map(|m| {
                        let [r, g, b] = m.emissive;
                        (m.diffuse, [r, g, b, 1.0])
                    })
                    .collect(),
            }),
            log: Vec::new(),
            ball_id,
            teleports: 0,
            sound: SoundState::default(),
            camera: CameraOverride::default(),
        })
    }
}

/// What the scripts see of the run at a tick, and what they ask of it.
pub struct RunView {
    pub tick: u64,
    pub ball: Vec3,
    pub ball_radius: f32,
    pub points: i64,
    /// The ball is in play (the original's `Activation Phase?` of CurrentLevel).
    pub in_play: bool,
    /// Where a script put the ball this tick, to be applied by the run.
    pub teleport: Option<Vec3>,
    /// The sim camera's place and aim this tick (where the view looks without a camera object).
    pub eye: Vec3,
    pub target: Vec3,
}

/// The level's materials as the operations read them.
#[derive(Debug, Default)]
struct Materials {
    of_mesh: Vec<Vec<usize>>,
    colours: Vec<([f32; 4], [f32; 4])>,
}

struct SimHost<'a> {
    s: &'a mut ScriptsParts,
    run: &'a mut RunView,
}

/// The parts of `Scripts` apart from the runtime, which is borrowed separately while it ticks.
struct ScriptsParts {
    hidden: Vec<bool>,
    moved: Vec<Option<Mat4>>,
    local_bounds: Vec<Option<(Vec3, Vec3)>>,
    names: Vec<String>,
    initial: Vec<[[f32; 4]; 4]>,
    meshes: Vec<Option<usize>>,
    groups: Vec<Vec<usize>>,
    materials: std::sync::Arc<Materials>,
    log: Vec<(u64, String)>,
    ball_id: Option<script::Id>,
    tick: u64,
    sound: SoundState,
    camera: CameraOverride,
}

impl ScriptsParts {
    fn note(&mut self, text: String) {
        if self.log.len() < LOG_CAP {
            self.log.push((self.tick, text));
        }
    }

    fn name(&self, i: usize) -> &str {
        self.names.get(i).map_or("?", String::as_str)
    }
}

fn to_mat(m: &[[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(m)
}

impl Host for SimHost<'_> {
    fn delta_ms(&self) -> f32 {
        crate::sim::TICK * 1000.0
    }

    fn matrix(&self, e: Ent) -> Option<[[f32; 4]; 4]> {
        match e {
            Ent::Ball => Some(Mat4::from_translation(self.run.ball).to_cols_array_2d()),
            Ent::Object(i) => {
                if let Some(Some(m)) = self.s.moved.get(i) {
                    return Some(m.to_cols_array_2d());
                }
                self.s.initial.get(i).copied()
            }
        }
    }

    fn initial_matrix(&self, e: Ent) -> Option<[[f32; 4]; 4]> {
        match e {
            Ent::Ball => None,
            Ent::Object(i) => self.s.initial.get(i).copied(),
        }
    }

    fn set_matrix(&mut self, e: Ent, m: [[f32; 4]; 4]) {
        match e {
            Ent::Ball => {
                let p = Vec3::new(m[3][0], m[3][1], m[3][2]);
                if p.is_finite() {
                    self.run.teleport = Some(p);
                    self.s
                        .note(format!("ball set to {:.1} {:.1} {:.1}", p.x, p.y, p.z));
                }
            }
            Ent::Object(i) => {
                let mat = to_mat(&m);
                if !mat.is_finite() {
                    return;
                }
                if let Some(slot) = self.s.moved.get_mut(i) {
                    // a block that moves an object every tick (Rotate) is logged once, at its first move
                    let first = slot.is_none();
                    *slot = Some(mat);
                    if first {
                        let p = mat.w_axis.truncate();
                        self.s.note(format!(
                            "moves {} to {:.1} {:.1} {:.1}",
                            self.s.name(i),
                            p.x,
                            p.y,
                            p.z
                        ));
                    }
                }
            }
        }
    }

    fn set_visible(&mut self, e: Ent, on: bool) {
        if let Ent::Object(i) = e
            && let Some(h) = self.s.hidden.get_mut(i)
            && *h == on
        {
            *h = !on;
            let name = self.s.name(i).to_string();
            self.s
                .note(format!("{} {name}", if on { "shows" } else { "hides" }));
        }
    }

    fn bounds(&self, e: Ent) -> Option<([f32; 3], [f32; 3])> {
        match e {
            Ent::Ball => {
                let r = Vec3::splat(self.run.ball_radius);
                Some((
                    (self.run.ball - r).to_array(),
                    (self.run.ball + r).to_array(),
                ))
            }
            Ent::Object(i) => {
                // an object without a mesh has an empty box at its place, as the original's: a ball's box
                // meets it when the place is inside (the portals' template tests the ball against such a frame)
                let (lo, hi) = (*self.s.local_bounds.get(i)?).unwrap_or((Vec3::ZERO, Vec3::ZERO));
                let m = to_mat(&self.matrix(e)?);
                let mut wlo = Vec3::splat(f32::INFINITY);
                let mut whi = Vec3::splat(f32::NEG_INFINITY);
                for k in 0..8u8 {
                    let c = Vec3::new(
                        if k & 1 != 0 { hi.x } else { lo.x },
                        if k & 2 != 0 { hi.y } else { lo.y },
                        if k & 4 != 0 { hi.z } else { lo.z },
                    );
                    let w = m.transform_point3(c);
                    wlo = wlo.min(w);
                    whi = whi.max(w);
                }
                Some((wlo.to_array(), whi.to_array()))
            }
        }
    }

    fn delete(&mut self, e: Ent) {
        if let Ent::Object(i) = e
            && let Some(h) = self.s.hidden.get_mut(i)
        {
            *h = true;
            let name = self.s.name(i).to_string();
            self.s.note(format!("deletes {name}"));
        }
    }

    fn physicalize(&mut self, e: Ent, on: bool) {
        // The ball is always in the physics here; other objects' physics by script come with phase 2.
        if let Ent::Object(i) = e {
            let name = self.s.name(i).to_string();
            self.s.note(format!(
                "{} {name} (physics by script not yet)",
                if on { "physicalizes" } else { "unphysicalizes" }
            ));
        }
    }

    fn particles(&mut self, e: Ent, on: bool) {
        // The flame emitters are drawn natively by their frames (particles.rs); the script's switch is noted.
        if let Ent::Object(i) = e {
            let name = self.s.name(i).to_string();
            self.s.note(format!(
                "particles {} at {name}",
                if on { "on" } else { "off" }
            ));
        }
    }

    fn mesh_of(&self, e: Ent) -> Option<usize> {
        match e {
            Ent::Ball => None,
            Ent::Object(i) => self.s.meshes.get(i).copied().flatten(),
        }
    }

    fn group_members(&self, group: usize) -> Vec<usize> {
        self.s.groups.get(group).cloned().unwrap_or_default()
    }

    fn game_cell(&self, array: &str, row: usize, column: usize) -> Option<Value> {
        if !array.eq_ignore_ascii_case("CurrentLevel") || row != 0 {
            return None;
        }
        // Level ID, ActiveBall, Ball_Pos_Frame, CurrentResetpoint, Activation Phase?, Points (Gameplay.nmo).
        Some(match column {
            0 => Value::Int(0),
            1 | 2 => Value::Obj(self.s.ball_id),
            3 => Value::None,
            4 => Value::Bool(self.run.in_play),
            5 => Value::Int(i32::try_from(self.run.points).unwrap_or(i32::MAX)),
            _ => return None,
        })
    }

    fn message(&mut self, name: &str, to: Option<Ent>) {
        let to = match to {
            Some(Ent::Ball) => " to the ball".to_string(),
            Some(Ent::Object(i)) => format!(" to {}", self.s.name(i)),
            None => String::new(),
        };
        self.s.note(format!("message {name}{to}"));
    }

    fn camera_get(&self) -> CameraState {
        let (eye, forward) = match self.s.camera.active.and_then(|i| self.matrix(Ent::Object(i))) {
            Some(m) => {
                let m = to_mat(&m);
                (m.w_axis.truncate(), m.z_axis.truncate().normalize_or(Vec3::Z))
            }
            None => (self.run.eye, (self.run.target - self.run.eye).normalize_or(Vec3::Z)),
        };
        let cam = &self.s.camera;
        let (near, far) = cam.clip.unwrap_or((0.5, 3000.0));
        CameraState {
            position: eye.to_array(),
            forward: forward.to_array(),
            up: [0.0, 1.0, 0.0],
            fov: cam.fov.unwrap_or_else(|| crate::scene::fov_degrees(crate::scene::fov_y())),
            near,
            far,
            object: cam.active,
        }
    }

    fn camera_set(&mut self, change: CameraChange) {
        match change {
            CameraChange::Fov(d) if d.is_finite() => self.s.camera.fov = Some(d.clamp(1.0, 179.0)),
            CameraChange::Clip { near, far } if near.is_finite() && far.is_finite() && near > 0.0 && far > near => {
                self.s.camera.clip = Some((near, far));
            }
            CameraChange::Active(i) => self.s.camera.active = Some(i),
            _ => {}
        }
    }

    fn sound(&mut self, key: usize, file: &str, cmd: SoundCmd) {
        let tick = self.s.tick;
        let stem = sound_stem(file);
        let ticks = self.s.sound.ticks.get(&stem.to_ascii_lowercase()).copied();
        if ticks.is_none() && !self.s.sound.noted.contains(&stem) {
            self.s.sound.noted.push(stem.clone());
            self.s.note(format!("sound file {file:?} is not available: silent"));
        }
        let st = &mut self.s.sound;
        match cmd {
            SoundCmd::Play { looping, .. } => {
                let until = (!looping).then(|| tick.saturating_add(ticks.unwrap_or(0)));
                st.playing.insert(key, Playing { until, paused: None });
            }
            SoundCmd::Stop { .. } => {
                st.playing.remove(&key);
            }
            SoundCmd::Pause => {
                if let Some(p) = st.playing.get_mut(&key) {
                    p.paused = Some(p.until.map_or(0, |u| u.saturating_sub(tick)));
                }
            }
            SoundCmd::Resume => {
                if let Some(p) = st.playing.get_mut(&key)
                    && let Some(left) = p.paused.take()
                    && p.until.is_some()
                {
                    p.until = Some(tick.saturating_add(left));
                }
            }
            _ => {}
        }
        if st.events.len() < SOUND_EVENT_CAP {
            st.events.push(SoundEvent {
                key,
                file: stem,
                cmd,
            });
        }
    }

    fn sound_playing(&self, key: usize) -> bool {
        self.s.sound.playing.get(&key).is_some_and(|p| {
            p.paused.is_some() || p.until.is_none_or(|u| self.s.tick < u)
        })
    }

    fn text(&mut self, name: &str, text: &str, on: bool) {
        self.s.note(if on {
            format!("text {name}: {text:?}")
        } else {
            format!("text {name} off")
        });
    }

    fn note(&mut self, text: &str) {
        self.s.note(text.to_string());
    }

    fn copy(&mut self, e: Ent, name: &str) -> Option<usize> {
        // The copy is an object of the run alone: it has the original's place, mesh and box, and its own
        // overlays (hidden, moved), past the level's objects. It is not drawn yet and has no body.
        let Ent::Object(i) = e else { return None };
        if self.s.names.len() >= MAX_OBJECTS {
            return None;
        }
        let matrix = self.matrix(e)?;
        let new = self.s.names.len();
        self.s.names.push(name.to_string());
        self.s.initial.push(matrix);
        self.s.meshes.push(self.s.meshes.get(i).copied().flatten());
        self.s
            .local_bounds
            .push(self.s.local_bounds.get(i).copied().flatten());
        self.s
            .hidden
            .push(self.s.hidden.get(i).copied().unwrap_or(false));
        self.s.moved.push(None);
        self.s.note(format!("copies {} as {name}", self.s.name(i)));
        Some(new)
    }

    fn group_add(&mut self, group: usize, object: usize) {
        if let Some(g) = self.s.groups.get_mut(group)
            && !g.contains(&object)
            && g.len() < MAX_OBJECTS
        {
            g.push(object);
        }
    }

    fn group_remove(&mut self, group: usize, object: usize) {
        if let Some(g) = self.s.groups.get_mut(group) {
            g.retain(|&m| m != object);
        }
    }

    fn group_clear(&mut self, group: usize) {
        if let Some(g) = self.s.groups.get_mut(group) {
            g.clear();
        }
    }

    fn mesh_materials(&self, mesh: usize) -> Vec<usize> {
        self.s
            .materials
            .of_mesh
            .get(mesh)
            .cloned()
            .unwrap_or_default()
    }

    fn material_color(&self, material: usize, emissive: bool) -> Option<[f32; 4]> {
        let (d, e) = self.s.materials.colours.get(material)?;
        Some(if emissive { *e } else { *d })
    }
}

impl Scripts {
    /// One tick of the scripts over the run's view; the view carries back what the run must apply.
    /// The world matrix of a level object as the scripts have it: moved, else as loaded.
    pub fn object_matrix(&self, i: usize) -> Option<Mat4> {
        match self.moved.get(i) {
            Some(Some(m)) => Some(*m),
            _ => self.initial.get(i).map(to_mat),
        }
    }

    pub fn tick(&mut self, run: &mut RunView) {
        let mut parts = ScriptsParts {
            hidden: std::mem::take(&mut self.hidden),
            moved: std::mem::take(&mut self.moved),
            local_bounds: std::mem::take(&mut self.local_bounds),
            names: std::mem::take(&mut self.names),
            initial: std::mem::take(&mut self.initial),
            meshes: std::mem::take(&mut self.meshes),
            groups: std::mem::take(&mut self.groups),
            materials: self.materials.clone(),
            log: std::mem::take(&mut self.log),
            ball_id: self.ball_id,
            tick: run.tick,
            sound: std::mem::take(&mut self.sound),
            camera: self.camera,
        };
        {
            let mut host = SimHost { s: &mut parts, run };
            self.rt.tick(&mut host);
        }
        self.hidden = parts.hidden;
        self.moved = parts.moved;
        self.local_bounds = parts.local_bounds;
        self.names = parts.names;
        self.initial = parts.initial;
        self.meshes = parts.meshes;
        self.groups = parts.groups;
        self.log = parts.log;
        self.sound = parts.sound;
        self.camera = parts.camera;
        if run.teleport.is_some() {
            self.teleports = self.teleports.saturating_add(1);
        }
    }
}
