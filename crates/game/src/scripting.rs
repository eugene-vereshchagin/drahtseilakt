// A community map's own scripts in a run: the script runtime (crates/script) driven once a tick before the
// physics, and the engine's side of it (the `Host`): objects hidden, moved or deleted by scripts are kept here
// as overlays on the level (the renderer and the tests read them), the ball is teleported through the physics,
// the game's own tables (`CurrentLevel`) are answered from the run, and what a script did or could not do goes
// to its log. Phase 1 (docs/drahtseilakt/scripts.md): no physics for scripted objects beyond the ball.

use crate::entities::Entities;
use glam::{Mat4, Vec3};
use level::Level;
use script::{CameraChange, CameraState, Ent, Host, Runtime, SoundCmd, Value};
use std::collections::{BTreeMap, HashMap};

/// How many log lines a run keeps.
const LOG_CAP: usize = 2_000;
/// The most objects a run holds with the scripts' copies (the runtime caps the copies well below).
pub(crate) const MAX_OBJECTS: usize = 1 << 20;

#[derive(Clone)]
pub struct Scripts {
    pub rt: Runtime,
    /// The run's entity table (entities.rs): the level's objects, then the scripts' copies and created objects,
    /// with what scripts did to them (hidden, moved, parents). The renderer reads it.
    pub ents: Entities,
    /// The level's groups' members.
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
    /// The text boxes the map's scripts show now, by the block that shows each (drawn by hud.rs under the interface;
    /// they rewind and restart with the run).
    pub text: Vec<(script::Id, script::TextSpec)>,
    /// The game's facade: the objects of the ball by kind, the run's numbers last seen (for the game's messages),
    /// the sector left last, and what scripts wrote into the game's arrays (kept in the run, never in its rules).
    ball_objs: [Option<script::Id>; 3],
    seen: Option<crate::facade::Seen>,
    left_sector: usize,
    written: BTreeMap<(String, usize, usize), Value>,
    /// The planar/spherical particle emitters the map's blocks run now, by block (particles.rs draws them; they
    /// rewind and restart with the run, and never reach it).
    pub emitters: BTreeMap<usize, EmitterView>,
}

/// One script emitter: where it is and what its block says.
#[derive(Clone, Debug)]
pub struct EmitterView {
    pub frame: Mat4,
    pub spec: script::EmitterSpec,
    pub state: script::EmitState,
}

/// The most emitters a map's scripts run at once.
pub const MAX_EMITTERS: usize = 64;

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
        let ball_objs = [
            graph.find("Ball_Wood"),
            graph.find("Ball_Stone"),
            graph.find("Ball_Paper"),
        ];
        Some(Scripts {
            rt: Runtime::new(graph),
            ents: Entities::new(level),
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
            text: Vec::new(),
            ball_objs,
            seen: None,
            left_sector: 0,
            written: BTreeMap::new(),
            emitters: BTreeMap::new(),
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
    /// The keyboard and the mouse of this tick and of the tick before (the run's input stream).
    pub input: script::InputFrame,
    pub input_before: script::InputFrame,
    /// The run's lives, sector and ball kind, and whether the level is finished (the game's messages follow them).
    pub lives: u32,
    pub sector: usize,
    pub kind: crate::sim::BallKind,
    pub finished: bool,
    /// What the scripts asked of the system's mouse cursor this tick: shown or hidden.
    pub cursor: Option<bool>,
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
    ents: Entities,
    groups: Vec<Vec<usize>>,
    materials: std::sync::Arc<Materials>,
    log: Vec<(u64, String)>,
    ball_id: Option<script::Id>,
    tick: u64,
    sound: SoundState,
    camera: CameraOverride,
    text: Vec<(script::Id, script::TextSpec)>,
    ball_objs: [Option<script::Id>; 3],
    left_sector: usize,
    written: BTreeMap<(String, usize, usize), Value>,
    emitters: BTreeMap<usize, EmitterView>,
}

impl ScriptsParts {
    fn note(&mut self, text: String) {
        if self.log.len() < LOG_CAP {
            self.log.push((self.tick, text));
        }
    }

    fn name(&self, i: usize) -> &str {
        self.ents.name(i)
    }
}

fn to_mat(m: &[[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(m)
}

impl Host for SimHost<'_> {
    fn delta_ms(&self) -> f32 {
        crate::sim::TICK * 1000.0
    }

    fn input(&self) -> script::InputFrame {
        self.run.input
    }

    fn input_before(&self) -> script::InputFrame {
        self.run.input_before
    }

    fn show_cursor(&mut self, on: bool) {
        self.run.cursor = Some(on);
    }

    fn matrix(&self, e: Ent) -> Option<[[f32; 4]; 4]> {
        match e {
            Ent::Ball => Some(Mat4::from_translation(self.run.ball).to_cols_array_2d()),
            Ent::Object(i) => self.s.ents.world(i).map(|m| m.to_cols_array_2d()),
        }
    }

    fn initial_matrix(&self, e: Ent) -> Option<[[f32; 4]; 4]> {
        match e {
            Ent::Ball => None,
            Ent::Object(i) => self.s.ents.initial(i).map(|m| m.to_cols_array_2d()),
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
                if i < self.s.ents.len() {
                    // a block that moves an object every tick (Rotate) is logged once, at its first move
                    let first = !self.s.ents.is_moved(i);
                    self.s.ents.set_world(i, mat);
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
            && self.s.ents.set_hidden(i, !on)
        {
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
                let (lo, hi) = self
                    .s
                    .ents
                    .local_bounds(i)?
                    .unwrap_or((Vec3::ZERO, Vec3::ZERO));
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
            && i < self.s.ents.len()
        {
            self.s.ents.delete(i);
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

    fn emitter(
        &mut self,
        key: usize,
        e: Ent,
        spec: &script::EmitterSpec,
        state: script::EmitState,
    ) {
        if state == script::EmitState::Gone {
            self.s.emitters.remove(&key);
            return;
        }
        if !self.s.emitters.contains_key(&key) && self.s.emitters.len() >= MAX_EMITTERS {
            return;
        }
        let Some(m) = self.matrix(e) else {
            return;
        };
        self.s.emitters.insert(
            key,
            EmitterView {
                frame: to_mat(&m),
                spec: spec.clone(),
                state,
            },
        );
    }

    fn mesh_of(&self, e: Ent) -> Option<usize> {
        match e {
            Ent::Ball => None,
            Ent::Object(i) => self.s.ents.mesh(i),
        }
    }

    fn group_members(&self, group: usize) -> Vec<usize> {
        self.s.groups.get(group).cloned().unwrap_or_default()
    }

    fn game_cell(&self, array: &str, row: usize, column: usize) -> Option<Value> {
        let cols = crate::facade::columns(array)?;
        cols.get(column)?;
        if let Some(v) = self
            .s
            .written
            .get(&(array.to_ascii_lowercase(), row, column))
        {
            return Some(v.clone());
        }
        let kind_object = match self.run.kind {
            crate::sim::BallKind::Wood => self.s.ball_objs[0],
            crate::sim::BallKind::Stone => self.s.ball_objs[1],
            crate::sim::BallKind::Paper => self.s.ball_objs[2],
            crate::sim::BallKind::Sticky => None,
        };
        crate::facade::cell(
            array,
            row,
            column,
            &crate::facade::Fill {
                in_play: self.run.in_play,
                points: self.run.points,
                lives: self.run.lives,
                sector: self.run.sector,
                left_sector: self.s.left_sector,
                frame: self.s.ball_id,
                kind_object,
            },
        )
    }

    fn game_set_cell(&mut self, array: &str, row: usize, column: usize, v: Value) -> bool {
        // only a cell the array has, and the run's own copy of it: the game's rules never read it back
        if self.game_cell(array, row, column).is_none() || self.s.written.len() >= 256 {
            return false;
        }
        self.s
            .written
            .insert((array.to_ascii_lowercase(), row, column), v);
        true
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
        let (eye, forward) = match self
            .s
            .camera
            .active
            .and_then(|i| self.matrix(Ent::Object(i)))
        {
            Some(m) => {
                let m = to_mat(&m);
                (
                    m.w_axis.truncate(),
                    m.z_axis.truncate().normalize_or(Vec3::Z),
                )
            }
            None => (
                self.run.eye,
                (self.run.target - self.run.eye).normalize_or(Vec3::Z),
            ),
        };
        let cam = &self.s.camera;
        let (near, far) = cam.clip.unwrap_or((0.5, 3000.0));
        CameraState {
            position: eye.to_array(),
            forward: forward.to_array(),
            up: [0.0, 1.0, 0.0],
            fov: cam
                .fov
                .unwrap_or_else(|| crate::scene::fov_degrees(crate::scene::fov_y())),
            near,
            far,
            object: cam.active,
        }
    }

    fn camera_set(&mut self, change: CameraChange) {
        match change {
            CameraChange::Fov(d) if d.is_finite() => self.s.camera.fov = Some(d.clamp(1.0, 179.0)),
            CameraChange::Clip { near, far }
                if near.is_finite() && far.is_finite() && near > 0.0 && far > near =>
            {
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
            self.s
                .note(format!("sound file {file:?} is not available: silent"));
        }
        let st = &mut self.s.sound;
        match cmd {
            SoundCmd::Play { looping, .. } => {
                let until = (!looping).then(|| tick.saturating_add(ticks.unwrap_or(0)));
                st.playing.insert(
                    key,
                    Playing {
                        until,
                        paused: None,
                    },
                );
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
        self.s
            .sound
            .playing
            .get(&key)
            .is_some_and(|p| p.paused.is_some() || p.until.is_none_or(|u| self.s.tick < u))
    }

    fn text_box(&mut self, key: usize, spec: Option<script::TextSpec>) {
        let at = self.s.text.iter().position(|(k, _)| *k == key);
        match (at, spec) {
            (Some(i), None) => {
                self.s.text.remove(i);
            }
            (Some(i), Some(mut spec)) => {
                spec.text = spec.text.chars().take(script::MAX_TEXT_CHARS).collect();
                if let Some(slot) = self.s.text.get_mut(i) {
                    slot.1 = spec;
                }
            }
            (None, Some(mut spec)) if self.s.text.len() < script::MAX_TEXT_BOXES => {
                spec.text = spec.text.chars().take(script::MAX_TEXT_CHARS).collect();
                self.s.text.push((key, spec));
            }
            (None, _) => {}
        }
    }

    fn note(&mut self, text: &str) {
        self.s.note(text.to_string());
    }

    fn copy(&mut self, e: Ent, name: &str) -> Option<usize> {
        // The copy is a row of the run's entity table past the level's objects: the original's place, mesh,
        // materials and box, shown as the original is. The renderer draws it; it has no body yet (task 3.2).
        let Ent::Object(i) = e else { return None };
        let new = self.s.ents.copy(i, name)?;
        self.s.note(format!("copies {} as {name}", self.s.name(i)));
        Some(new)
    }

    fn set_parent(&mut self, e: Ent, parent: Option<Ent>) -> bool {
        let Ent::Object(i) = e else { return false };
        let p = match parent {
            Some(Ent::Object(p)) => Some(p),
            Some(Ent::Ball) => return false,
            None => None,
        };
        self.s.ents.link(i, p)
    }

    fn create(&mut self, name: &str, like: Option<Ent>, m: [[f32; 4]; 4]) -> Option<usize> {
        let (mesh, materials, bounds) = match like {
            Some(Ent::Object(i)) => {
                let r = self.s.ents.get(i)?;
                (r.mesh, r.materials.clone(), self.s.ents.local_bounds(i)?)
            }
            _ => (None, Vec::new(), None),
        };
        let new = self
            .s
            .ents
            .create(name, mesh, materials, bounds, to_mat(&m))?;
        self.s.note(format!("creates {name}"));
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
    /// The game's messages for what the run did since the last tick (facade::events): broadcast, heard by the
    /// scripts' Wait Message at the next tick. Nothing here changes the run.
    fn emit_events(&mut self, run: &RunView) {
        let now = crate::facade::Seen {
            sector: run.sector,
            lives: run.lives,
            points: run.points,
            kind: run.kind,
            finished: run.finished,
        };
        if let Some(p) = &self.seen
            && now.sector != p.sector
        {
            self.left_sector = p.sector;
        }
        for name in crate::facade::events(self.seen.as_ref(), &now) {
            self.rt.send(name, None);
            if self.log.len() < LOG_CAP {
                self.log.push((run.tick, format!("game message {name}")));
            }
        }
        self.seen = Some(now);
    }

    /// One tick of the scripts over the run's view; the view carries back what the run must apply.
    /// The world matrix of a level object as the scripts have it: moved, else as loaded.
    pub fn object_matrix(&self, i: usize) -> Option<Mat4> {
        self.ents.world(i)
    }

    pub fn tick(&mut self, run: &mut RunView) {
        self.emit_events(run);
        let mut parts = ScriptsParts {
            ents: std::mem::take(&mut self.ents),
            groups: std::mem::take(&mut self.groups),
            materials: self.materials.clone(),
            log: std::mem::take(&mut self.log),
            ball_id: self.ball_id,
            tick: run.tick,
            sound: std::mem::take(&mut self.sound),
            camera: self.camera,
            text: std::mem::take(&mut self.text),
            ball_objs: self.ball_objs,
            left_sector: self.left_sector,
            written: std::mem::take(&mut self.written),
            emitters: std::mem::take(&mut self.emitters),
        };
        {
            let mut host = SimHost { s: &mut parts, run };
            self.rt.tick(&mut host);
        }
        self.ents = parts.ents;
        self.groups = parts.groups;
        self.log = parts.log;
        self.sound = parts.sound;
        self.camera = parts.camera;
        self.text = parts.text;
        self.written = parts.written;
        self.emitters = parts.emitters;
        if run.teleport.is_some() {
            self.teleports = self.teleports.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod facade_tests {
    use super::*;
    use crate::sim::BallKind;

    fn view(tick: u64, sector: usize, lives: u32) -> RunView {
        RunView {
            tick,
            ball: Vec3::ZERO,
            ball_radius: 1.0,
            points: 1000,
            in_play: true,
            teleport: None,
            input: script::InputFrame::default(),
            input_before: script::InputFrame::default(),
            lives,
            sector,
            kind: BallKind::Wood,
            finished: false,
            cursor: None,
            eye: Vec3::ZERO,
            target: Vec3::Z,
        }
    }

    fn scripts() -> Scripts {
        let mut g = script::Graph::default();
        for n in ["Ball_Pos_Frame", "Ball_Wood"] {
            g.add_object(n, script::ObjKind::Ball);
        }
        let level = Level {
            scripts: Some(std::sync::Arc::new(g)),
            ..Level::default()
        };
        Scripts::new(&level).expect("a level with a graph has scripts")
    }

    #[test]
    fn the_engine_sends_the_games_messages_to_every_object() {
        // sent before the tick's scripts run, so a Wait Message hears it in that same tick
        let mut s = scripts();
        s.tick(&mut view(0, 1, 3));
        assert_eq!(s.rt.messages_to("Start Music", None).len(), 1);
        s.tick(&mut view(1, 1, 3));
        assert!(s.rt.messages_to("Start Music", None).is_empty(), "once");
        s.tick(&mut view(2, 2, 3));
        assert_eq!(s.rt.messages_to("Checkpoint reached", None).len(), 1);
        s.tick(&mut view(3, 2, 2));
        assert_eq!(s.rt.messages_to("Sub Life", None).len(), 1);
        assert_eq!(s.rt.messages_to("Ball Off", None).len(), 1);
        assert!(
            s.log
                .iter()
                .any(|(_, l)| l == "game message Checkpoint reached")
        );
    }

    #[test]
    fn a_script_write_stays_in_the_run() {
        let mut s = scripts();
        let level_parts = |s: &mut Scripts, f: &mut dyn FnMut(&mut SimHost)| {
            let mut run = view(0, 1, 3);
            let mut parts = ScriptsParts {
                ents: Entities::default(),
                groups: vec![],
                materials: s.materials.clone(),
                log: vec![],
                ball_id: s.ball_id,
                tick: 0,
                sound: SoundState::default(),
                camera: CameraOverride::default(),
                text: vec![],
                ball_objs: s.ball_objs,
                left_sector: 0,
                written: std::mem::take(&mut s.written),
                emitters: Default::default(),
            };
            f(&mut SimHost {
                s: &mut parts,
                run: &mut run,
            });
            s.written = parts.written;
        };
        level_parts(&mut s, &mut |h| {
            assert_eq!(h.game_cell("Energy", 0, 1), Some(Value::Int(3)));
            assert!(h.game_set_cell("Energy", 0, 1, Value::Int(99)));
            assert_eq!(h.game_cell("Energy", 0, 1), Some(Value::Int(99)));
            assert!(
                !h.game_set_cell("Energy", 0, 9, Value::Int(1)),
                "no such column"
            );
            assert!(!h.game_set_cell("Nope", 0, 0, Value::Int(1)));
        });
        // the run's own number is untouched
        assert_eq!(s.ball_objs[0], s.rt.graph.find("Ball_Wood"));
    }
}
