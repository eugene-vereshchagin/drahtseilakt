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
    /// The movable bodies scripts made (docs/physics.md 2.1): entity row -> the world's body (none while the body's
    /// command is queued). A BTreeMap, so every walk over it is in row order, never in hash order.
    pub bodies: BTreeMap<usize, Option<usize>>,
    /// The forces scripts made: (entity row, block) -> the world's force.
    forces: BTreeMap<(usize, usize), (usize, Vec3, Vec3)>,
    /// The matrix each script body's object had when the physics last wrote it (or the body was made): a different
    /// matrix later means a script moved the object, and the body is put there.
    body_last: BTreeMap<usize, Mat4>,
    /// The matrix each moved level object's baked body was last put by (absent: as loaded).
    baked_last: BTreeMap<usize, Mat4>,
    /// What the scripts asked of the physics this tick, in graph order; the run applies it (`apply_physics`).
    queue: Vec<PhysCmd>,
    /// Bodies made by scripts so far (slots in the world are never reused, so this bounds them).
    made: usize,
    /// Rows whose fixed or concave Physicalize was already reported.
    noted_static: Vec<usize>,
    /// The fixed bodies scripts made (docs/physics.md 2.1): entity row -> the world's handle (none while the body's
    /// command is queued) and the object's matrix the body was last placed by. In row order.
    pub statics: BTreeMap<usize, StaticSlot>,
    /// Fixed bodies made by scripts so far (bounds the handles in the world).
    made_static: usize,
}

/// A script's fixed body and the object matrix it stands for.
#[derive(Clone, Debug)]
pub struct StaticSlot {
    pub handle: Option<usize>,
    pub last: Mat4,
}

/// The most fixed bodies the scripts of one run make.
pub const MAX_SCRIPT_STATICS: usize = 1_024;
/// The most triangles one fixed body of a script has (a larger mesh makes no body).
pub const MAX_STATIC_TRIANGLES: usize = 65_536;

/// How a fixed body is made.
#[derive(Clone, Debug)]
pub enum StaticShape {
    /// The mesh's triangles in the object's frame (concave).
    Triangles(Vec<[Vec3; 3]>),
    /// The hull of points in the object's frame (a convex list).
    Hull(Vec<Vec3>),
    /// A sphere of this radius (a ball shape with Fixed = TRUE).
    Sphere(f32),
}

/// The most movable bodies the scripts of one run make.
pub const MAX_SCRIPT_BODIES: usize = 256;
/// The most distinct vertices a script body's hull is made from (a larger mesh is sampled evenly).
pub const MAX_HULL_POINTS: usize = 1_024;

/// A request of the scripts to the physics world.
#[derive(Clone, Debug)]
pub enum PhysCmd {
    Ball {
        ent: usize,
        radius: f32,
        m: crate::physics::Material,
        pos: Vec3,
        rot: glam::Quat,
        frozen: bool,
        group: String,
    },
    Convex {
        ent: usize,
        points: Vec<Vec3>,
        m: crate::physics::Material,
        pos: Vec3,
        rot: glam::Quat,
        frozen: bool,
        collide: bool,
        shift: Option<[f32; 3]>,
        group: String,
    },
    Static {
        ent: usize,
        shape: StaticShape,
        friction: f32,
        elasticity: f32,
        group: String,
        pos: Vec3,
        rot: glam::Quat,
        /// The object's matrix at the request, which the body stands for.
        at: Mat4,
    },
    Remove {
        ent: usize,
        /// The world's fixed body of the object, taken from the table by the host.
        fixed: Option<usize>,
    },
    Force {
        ent: usize,
        key: usize,
        create: bool,
        at: Vec3,
        force: Vec3,
    },
    Impulse {
        ent: usize,
        at: Vec3,
        impulse: Vec3,
    },
    Wake {
        ent: usize,
    },
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
                points: level.meshes.iter().map(|m| m.positions.clone()).collect(),
                triangles: level
                    .meshes
                    .iter()
                    .map(|m| {
                        m.parts
                            .iter()
                            .flat_map(|p| p.indices.as_chunks::<3>().0.iter().copied())
                            .collect()
                    })
                    .collect(),
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
            bodies: BTreeMap::new(),
            forces: BTreeMap::new(),
            body_last: BTreeMap::new(),
            baked_last: BTreeMap::new(),
            queue: Vec::new(),
            made: 0,
            noted_static: Vec::new(),
            statics: BTreeMap::new(),
            made_static: 0,
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
    /// The ball is dying, and the last checkpoint has been passed (for the game's messages).
    pub dying: bool,
    pub last_stage: bool,
    /// What the scripts asked of the system's mouse cursor this tick: shown or hidden.
    pub cursor: Option<bool>,
    /// The sim camera's place and aim this tick (where the view looks without a camera object).
    pub eye: Vec3,
    pub target: Vec3,
}

/// The level's materials as the operations read them.
#[derive(Debug, Default)]
struct Materials {
    /// Each mesh's vertices, for the hulls of script bodies.
    points: Vec<Vec<[f32; 3]>>,
    /// Each mesh's triangles as vertex indices, for the fixed bodies of scripts.
    triangles: Vec<Vec<[u32; 3]>>,
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
    bodies: BTreeMap<usize, Option<usize>>,
    queue: Vec<PhysCmd>,
    made: usize,
    noted_static: Vec<usize>,
    statics: BTreeMap<usize, StaticSlot>,
    made_static: usize,
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

impl SimHost<'_> {
    /// A fixed Physicalize (Fixed = TRUE, or a concave list only): the object's mesh becomes a fixed body of the
    /// world, in the object's frame with its scale, as the level's own floors are (docs/physics.md 2.1).
    fn physicalize_static(&mut self, i: usize, name: &str, world: Mat4, spec: &script::PhysSpec) {
        if self.s.made_static >= MAX_SCRIPT_STATICS {
            self.s
                .note(format!("physicalizes {name}: too many fixed bodies"));
            return;
        }
        let Some((pos, rot, scale)) = pose_of(&world) else {
            return;
        };
        let Some(mesh) = self.s.ents.mesh(i) else {
            self.s
                .note(format!("physicalizes {name}: no mesh for a fixed body"));
            return;
        };
        let points = self.s.materials.points.get(mesh);
        let shape = if spec.ball_count > 0 {
            // the radius pin; a block that has none gets the mesh's half extent (as for a movable ball)
            let radius = spec.radius.unwrap_or_else(|| {
                let (lo, hi) = points.map_or((Vec3::ZERO, Vec3::ZERO), |v| {
                    v.iter().fold(
                        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                        |(lo, hi), &p| {
                            let p = Vec3::from(p) * scale;
                            (lo.min(p), hi.max(p))
                        },
                    )
                });
                (hi - lo).max_element() * 0.5
            });
            if !(radius.is_finite() && radius > 0.0) {
                self.s
                    .note(format!("physicalizes {name}: no usable radius"));
                return;
            }
            StaticShape::Sphere(radius)
        } else if spec.convex_count > 0 {
            let mut seen = std::collections::BTreeSet::new();
            let mut pts: Vec<Vec3> = points
                .map(|v| v.iter().map(|&p| Vec3::from(p) * scale).collect::<Vec<_>>())
                .unwrap_or_default()
                .into_iter()
                .filter(|p| p.is_finite() && seen.insert(p.to_array().map(f32::to_bits)))
                .collect();
            if pts.len() > MAX_HULL_POINTS {
                let step = pts.len().div_ceil(MAX_HULL_POINTS);
                pts = pts.into_iter().step_by(step).collect();
            }
            if pts.len() < 4 {
                self.s
                    .note(format!("physicalizes {name}: no mesh for a fixed body"));
                return;
            }
            StaticShape::Hull(pts)
        } else {
            let (Some(points), Some(tris)) = (points, self.s.materials.triangles.get(mesh)) else {
                return;
            };
            if tris.len() > MAX_STATIC_TRIANGLES {
                self.s.note(format!(
                    "physicalizes {name}: too many triangles for a fixed body"
                ));
                return;
            }
            let at = |k: u32| {
                points
                    .get(k as usize)
                    .map_or(Vec3::ZERO, |&p| Vec3::from(p) * scale)
            };
            StaticShape::Triangles(
                tris.iter()
                    .map(|t| [at(t[0]), at(t[1]), at(t[2])])
                    .collect(),
            )
        };
        self.s.made_static += 1;
        self.s.statics.insert(
            i,
            StaticSlot {
                handle: None,
                last: world,
            },
        );
        self.s.queue.push(PhysCmd::Static {
            ent: i,
            shape,
            friction: spec.friction,
            elasticity: spec.elasticity,
            group: spec.group.clone(),
            pos,
            rot,
            at: world,
        });
    }
}

/// An object's pose as the physics takes it: the position, the rotation without the scale, and the scale.
fn pose_of(world: &Mat4) -> Option<(Vec3, glam::Quat, Vec3)> {
    let (x, y, z) = (
        world.x_axis.truncate(),
        world.y_axis.truncate(),
        world.z_axis.truncate(),
    );
    let scale = Vec3::new(x.length(), y.length(), z.length());
    let basis = glam::Mat3::from_cols(
        x.normalize_or_zero(),
        y.normalize_or_zero(),
        z.normalize_or_zero(),
    );
    let rot = glam::Quat::from_mat3(&basis).normalize();
    let pos = world.w_axis.truncate();
    (pos.is_finite() && rot.is_finite() && scale.is_finite()).then_some((pos, rot, scale))
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
        // Unphysicalize: a body a script made goes out of the world (the ball is always in it).
        let Ent::Object(i) = e else {
            return;
        };
        let name = self.s.name(i).to_string();
        // the object's body goes: a movable or fixed one of a script, or the level's own (found when applied)
        let slot = self.s.statics.remove(&i);
        if !on {
            self.s.queue.push(PhysCmd::Remove {
                ent: i,
                fixed: slot.as_ref().and_then(|s| s.handle),
            });
        }
        let had = self.s.bodies.remove(&i).is_some() | slot.is_some();
        if !on && had {
            self.s.note(format!("unphysicalizes {name}"));
        } else {
            self.s.note(format!(
                "{} {name} (no body of a script)",
                if on { "physicalizes" } else { "unphysicalizes" }
            ));
        }
    }

    fn physicalize_body(&mut self, e: Ent, _key: usize, spec: &script::PhysSpec) {
        let Ent::Object(i) = e else {
            return;
        };
        // an object already physicalized keeps its body: the original does not update it (docs/physics.md 2.1)
        if self.s.bodies.contains_key(&i)
            || self.s.statics.contains_key(&i)
            || i >= self.s.ents.len()
        {
            return;
        }
        let name = self.s.name(i).to_string();
        let Some(world) = self.s.ents.world(i) else {
            return;
        };
        if spec.fixed
            || (spec.convex_count == 0 && spec.ball_count == 0 && !spec.surface.is_empty())
        {
            self.physicalize_static(i, &name, world, spec);
            return;
        }
        if self.s.made >= MAX_SCRIPT_BODIES {
            self.s
                .note(format!("physicalizes {name}: too many script bodies"));
            return;
        }
        let m = crate::physics::Material {
            friction: spec.friction,
            elasticity: spec.elasticity,
            mass: if spec.mass < 1e-8 { 1.0 } else { spec.mass },
            lin_damp: spec.lin_damp,
            rot_damp: spec.rot_damp,
        };
        let Some((pos, rot, scale)) = pose_of(&world) else {
            return;
        };
        let pts: Vec<Vec3> = self
            .s
            .ents
            .mesh(i)
            .and_then(|k| self.s.materials.points.get(k))
            .map(|v| v.iter().map(|&p| Vec3::from(p) * scale).collect())
            .unwrap_or_default();
        let cmd = if spec.ball_count > 0 || spec.surface.is_empty() {
            // the radius pin; a block that has none gets the mesh's half extent
            let radius = spec.radius.unwrap_or_else(|| {
                let (lo, hi) = pts.iter().fold(
                    (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                    |(lo, hi), &p| (lo.min(p), hi.max(p)),
                );
                (hi - lo).max_element() * 0.5
            });
            if !(radius.is_finite() && radius > 0.0) {
                self.s
                    .note(format!("physicalizes {name}: no usable radius"));
                return;
            }
            PhysCmd::Ball {
                ent: i,
                radius,
                m,
                pos,
                rot,
                frozen: spec.start_frozen,
                group: spec.group.clone(),
            }
        } else {
            // exact duplicates out, in a fixed order; a very large mesh is sampled evenly
            let mut seen = std::collections::BTreeSet::new();
            let mut points: Vec<Vec3> = pts
                .into_iter()
                .filter(|p| p.is_finite() && seen.insert(p.to_array().map(f32::to_bits)))
                .collect();
            if points.len() > MAX_HULL_POINTS {
                let step = points.len().div_ceil(MAX_HULL_POINTS);
                points = points.into_iter().step_by(step).collect();
            }
            if points.len() < 4 {
                self.s
                    .note(format!("physicalizes {name}: no mesh for a body"));
                return;
            }
            PhysCmd::Convex {
                ent: i,
                points,
                m,
                pos,
                rot,
                frozen: spec.start_frozen,
                collide: spec.collide,
                shift: (!spec.auto_centre).then_some(spec.shift),
                group: spec.group.clone(),
            }
        };
        self.s.made += 1;
        self.s.bodies.insert(i, None);
        self.s.queue.push(cmd);
    }

    fn physics_force(&mut self, e: Ent, key: usize, create: bool, at: [f32; 3], force: [f32; 3]) {
        if let Ent::Object(ent) = e
            && self.s.bodies.contains_key(&ent)
        {
            self.s.queue.push(PhysCmd::Force {
                ent,
                key,
                create,
                at: Vec3::from(at),
                force: Vec3::from(force),
            });
        }
    }

    fn physics_impulse(&mut self, e: Ent, at: [f32; 3], impulse: [f32; 3]) {
        if let Ent::Object(ent) = e
            && self.s.bodies.contains_key(&ent)
        {
            self.s.queue.push(PhysCmd::Impulse {
                ent,
                at: Vec3::from(at),
                impulse: Vec3::from(impulse),
            });
        }
    }

    fn physics_wake(&mut self, e: Ent) {
        if let Ent::Object(ent) = e
            && self.s.bodies.contains_key(&ent)
        {
            self.s.queue.push(PhysCmd::Wake { ent });
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
    /// The game's pause messages (the app's pause menu opened or closed over the run): broadcast like the others,
    /// heard when the scripts next tick.
    pub fn pause_event(&mut self, paused: bool, tick: u64) {
        for name in crate::facade::pause_events(paused) {
            self.rt.send(name, None);
            if self.log.len() < LOG_CAP {
                self.log.push((tick, format!("game message {name}")));
            }
        }
    }

    /// The game's messages for what the run did since the last tick (facade::events): broadcast, heard by the
    /// scripts' Wait Message at the next tick. Nothing here changes the run.
    fn emit_events(&mut self, run: &RunView) {
        let now = crate::facade::Seen {
            sector: run.sector,
            lives: run.lives,
            points: run.points,
            kind: run.kind,
            finished: run.finished,
            dying: run.dying,
            last_stage: run.last_stage,
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

    /// Applies what the scripts asked of the physics this tick, in the order they asked (the run calls it after
    /// `tick`, before the physics step).
    pub fn apply_physics(&mut self, world: &mut crate::physics::World, tick: u64) {
        let note = |log: &mut Vec<(u64, String)>, t: String| {
            if log.len() < LOG_CAP {
                log.push((tick, t));
            }
        };
        for cmd in std::mem::take(&mut self.queue) {
            match cmd {
                PhysCmd::Ball {
                    ent,
                    radius,
                    m,
                    pos,
                    rot,
                    frozen,
                    group,
                } => {
                    if world.has_baked(ent) {
                        self.bodies.remove(&ent);
                        note(
                            &mut self.log,
                            format!("{} already has the level's body", self.ents.name(ent)),
                        );
                        continue;
                    }
                    let i = world.add_sphere_in(radius, m, pos, rot, !frozen, &group);
                    self.bodies.insert(ent, Some(i));
                    self.remember_body(ent);
                    note(
                        &mut self.log,
                        format!(
                            "{} is a ball body of radius {radius:.2}",
                            self.ents.name(ent)
                        ),
                    );
                }
                PhysCmd::Convex {
                    ent,
                    points,
                    m,
                    pos,
                    rot,
                    frozen,
                    collide,
                    shift,
                    group,
                } => {
                    if world.has_baked(ent) {
                        self.bodies.remove(&ent);
                        note(
                            &mut self.log,
                            format!("{} already has the level's body", self.ents.name(ent)),
                        );
                        continue;
                    }
                    match world.add_scripted_convex(
                        &points, m, pos, rot, frozen, collide, shift, &group,
                    ) {
                        Some(i) => {
                            self.bodies.insert(ent, Some(i));
                            self.remember_body(ent);
                            note(
                                &mut self.log,
                                format!(
                                    "{} is a convex body of {} points",
                                    self.ents.name(ent),
                                    points.len()
                                ),
                            );
                        }
                        None => {
                            self.bodies.remove(&ent);
                            note(
                                &mut self.log,
                                format!("{} has no hull for a body", self.ents.name(ent)),
                            );
                        }
                    }
                }
                PhysCmd::Static {
                    ent,
                    shape,
                    friction,
                    elasticity,
                    group,
                    pos,
                    rot,
                    at,
                } => {
                    // the object is already a body of the level: the original does not physicalize it again
                    if world.has_baked(ent) {
                        self.statics.remove(&ent);
                        note(
                            &mut self.log,
                            format!("{} already has the level's body", self.ents.name(ent)),
                        );
                        continue;
                    }
                    let (h, what) = match &shape {
                        StaticShape::Triangles(t) => (
                            world.add_static_triangles(t, pos, rot, friction, elasticity, &group),
                            format!("{} triangles", t.len()),
                        ),
                        StaticShape::Sphere(r) => (
                            world.add_static_sphere(*r, pos, rot, friction, elasticity, &group),
                            format!("a sphere of radius {r:.2}"),
                        ),
                        StaticShape::Hull(p) => (
                            world.add_static_hull(p, pos, rot, friction, elasticity, &group),
                            format!("a hull of {} points", p.len()),
                        ),
                    };
                    match h {
                        Some(h) => {
                            self.statics.insert(
                                ent,
                                StaticSlot {
                                    handle: Some(h),
                                    last: at,
                                },
                            );
                            note(
                                &mut self.log,
                                format!("{} is a fixed body of {what}", self.ents.name(ent)),
                            );
                        }
                        None => {
                            self.statics.remove(&ent);
                            note(
                                &mut self.log,
                                format!(
                                    "{} has no usable mesh for a fixed body",
                                    self.ents.name(ent)
                                ),
                            );
                        }
                    }
                }
                PhysCmd::Remove { ent, fixed } => {
                    // a body made and removed in one tick was never in the world
                    if let Some(Some(i)) = self.bodies.get(&ent).copied() {
                        world.remove(i);
                    }
                    let gone: Vec<(usize, usize)> =
                        self.forces.keys().filter(|k| k.0 == ent).copied().collect();
                    for k in gone {
                        self.forces.remove(&k);
                    }
                    self.bodies.remove(&ent);
                    self.body_last.remove(&ent);
                    self.baked_last.remove(&ent);
                    if let Some(h) = fixed {
                        world.remove_static(h);
                    }
                    if let Some(slot) = self.statics.remove(&ent)
                        && let Some(h) = slot.handle
                    {
                        world.remove_static(h);
                    }
                    if world.remove_baked(ent) {
                        note(
                            &mut self.log,
                            format!("{} loses the level's body", self.ents.name(ent)),
                        );
                    }
                }
                PhysCmd::Force {
                    ent,
                    key,
                    create,
                    at,
                    force,
                } => {
                    let Some(Some(i)) = self.bodies.get(&ent).copied() else {
                        continue;
                    };
                    if create {
                        if !self.forces.contains_key(&(ent, key))
                            && let Some(c) = world.add_force(i, force, at)
                        {
                            self.forces.insert((ent, key), (c, at, force));
                        }
                    } else if let Some((c, _, _)) = self.forces.remove(&(ent, key)) {
                        world.remove_force(c);
                    }
                }
                PhysCmd::Impulse { ent, at, impulse } => {
                    if let Some(Some(i)) = self.bodies.get(&ent).copied() {
                        world.impulse_at(i, at, impulse);
                    }
                }
                PhysCmd::Wake { ent } => {
                    if let Some(Some(i)) = self.bodies.get(&ent).copied() {
                        world.wake(i);
                    }
                }
            }
        }
        self.follow_statics(world);
        self.follow_bodies(world);
    }

    fn remember_body(&mut self, ent: usize) {
        if let Some(m) = self.ents.world(ent) {
            self.body_last.insert(ent, m);
        }
    }

    /// Bodies of the level and of scripts follow their objects when a script moved them: a baked fixed body is put
    /// where the object went, a movable body is teleported (at rest, its forces carried over).
    fn follow_bodies(&mut self, world: &mut crate::physics::World) {
        let bits = |m: &Mat4| m.to_cols_array().map(f32::to_bits);
        for row in world.baked_row_list() {
            let (Some(cur), Some(init)) = (self.ents.world(row), self.ents.initial(row)) else {
                continue;
            };
            let last = self.baked_last.get(&row).copied().unwrap_or(init);
            if bits(&cur) == bits(&last) {
                continue;
            }
            let delta = cur * last.inverse();
            if delta.is_finite() {
                world.move_baked(row, delta);
            }
            self.baked_last.insert(row, cur);
        }
        let moved: Vec<(usize, usize, Mat4)> = self
            .bodies
            .iter()
            .filter_map(|(&ent, &b)| {
                let cur = self.ents.world(ent)?;
                let last = self.body_last.get(&ent)?;
                (bits(&cur) != bits(last)).then_some((ent, b?, cur))
            })
            .collect();
        for (ent, body, cur) in moved {
            self.body_last.insert(ent, cur);
            let Some((pos, rot, _)) = pose_of(&cur) else {
                continue;
            };
            let keys: Vec<(usize, usize)> =
                self.forces.keys().filter(|k| k.0 == ent).copied().collect();
            for k in &keys {
                if let Some((c, _, _)) = self.forces.get(k) {
                    world.remove_force(*c);
                }
            }
            world.teleport(body, pos, rot);
            for k in keys {
                if let Some((_, at, force)) = self.forces.get(&k).copied()
                    && let Some(c) = world.add_force(body, force, at)
                {
                    self.forces.insert(k, (c, at, force));
                }
            }
        }
    }

    /// Fixed bodies follow their objects: one whose object a script moved is put at the new place.
    fn follow_statics(&mut self, world: &mut crate::physics::World) {
        for (&ent, slot) in self.statics.iter_mut() {
            let (Some(h), Some(cur)) = (slot.handle, self.ents.world(ent)) else {
                continue;
            };
            let bits = |m: &Mat4| m.to_cols_array().map(f32::to_bits);
            if bits(&cur) != bits(&slot.last) {
                if let Some((pos, rot, _)) = pose_of(&cur) {
                    world.move_static(h, pos, rot);
                }
                slot.last = cur;
            }
        }
    }

    /// After the physics step: each script body's object takes the body's pose (the original writes the poses back
    /// every frame and never reads them; its matrix is orthonormal, the scale lives in the geometry only).
    pub fn sync_physics(&mut self, world: &crate::physics::World) {
        for (&ent, &body) in &self.bodies {
            if let Some(i) = body
                && let Some(s) = world.state(i)
            {
                let m = Mat4::from_rotation_translation(s.rot, s.pos);
                self.ents.set_world(ent, m);
                // read back what the table holds now (a child's matrix goes through its parent)
                if let Some(w) = self.ents.world(ent) {
                    self.body_last.insert(ent, w);
                }
            }
        }
    }

    pub fn tick(&mut self, run: &mut RunView) {
        self.emit_events(run);
        self.with_host(run, |rt, host| rt.tick(host));
    }

    /// Runs `f` with the runtime and the engine's side of it (the parts of `self` are lent to the host meanwhile).
    fn with_host(&mut self, run: &mut RunView, f: impl FnOnce(&mut Runtime, &mut SimHost<'_>)) {
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
            bodies: std::mem::take(&mut self.bodies),
            queue: std::mem::take(&mut self.queue),
            made: self.made,
            noted_static: std::mem::take(&mut self.noted_static),
            statics: std::mem::take(&mut self.statics),
            made_static: self.made_static,
        };
        {
            let mut host = SimHost { s: &mut parts, run };
            f(&mut self.rt, &mut host);
        }
        self.ents = parts.ents;
        self.groups = parts.groups;
        self.log = parts.log;
        self.sound = parts.sound;
        self.camera = parts.camera;
        self.text = parts.text;
        self.written = parts.written;
        self.emitters = parts.emitters;
        self.bodies = parts.bodies;
        self.queue = parts.queue;
        self.made = parts.made;
        self.noted_static = parts.noted_static;
        self.statics = parts.statics;
        self.made_static = parts.made_static;
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
            dying: false,
            last_stage: false,
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
                bodies: Default::default(),
                queue: Vec::new(),
                made: 0,
                noted_static: Vec::new(),
                statics: BTreeMap::new(),
                made_static: 0,
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

#[cfg(test)]
mod body_tests {
    // Physicalize by script (docs/physics.md 2.1, 2.2, 2.5, 2.8): movable bodies made through the host and the world.
    use super::*;
    use crate::collide::{StaticMesh, Triangle};
    use crate::physics::{GROUP_FLOOR, SoundIds, World};

    fn spec() -> script::PhysSpec {
        script::PhysSpec {
            fixed: false,
            friction: 0.7,
            elasticity: 0.3,
            mass: 1.0,
            group: String::new(),
            start_frozen: false,
            collide: true,
            auto_centre: true,
            shift: [0.0; 3],
            lin_damp: 0.1,
            rot_damp: 0.1,
            surface: "box".into(),
            convex_count: 1,
            ball_count: 0,
            concave_count: 0,
            radius: None,
        }
    }

    fn floor() -> Vec<StaticMesh> {
        let s = 100.0;
        let p = [
            Vec3::new(-s, 0.0, -s),
            Vec3::new(s, 0.0, -s),
            Vec3::new(s, 0.0, s),
            Vec3::new(-s, 0.0, s),
        ];
        let tri = |a, b, c| Triangle { a, b, c };
        vec![StaticMesh {
            triangles: vec![tri(p[0], p[2], p[1]), tri(p[0], p[3], p[2])],
            pos: Vec3::ZERO,
            rot: glam::Quat::IDENTITY.into(),
            group: GROUP_FLOOR,
            sound: SoundIds::default(),
        }]
    }

    /// A level with `n` objects, each a 2-unit cube at height 5 (object k at x = 10 k).
    fn scripts(n: usize) -> Scripts {
        let mut mesh = level::Mesh {
            name: "cube".into(),
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            parts: Vec::new(),
            channels: Vec::new(),
            look: None,
        };
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    mesh.positions.push([x, y, z]);
                }
            }
        }
        // twelve triangles, outward facing (a vertex's index is its bits x, y, z: 1 for +1)
        let mut idx = Vec::new();
        let v = |k: u32| {
            Vec3::new(
                if k & 4 != 0 { 1.0 } else { -1.0 },
                if k & 2 != 0 { 1.0 } else { -1.0 },
                if k & 1 != 0 { 1.0 } else { -1.0 },
            )
        };
        for axis in 0..3u32 {
            for side in [0u32, 1] {
                let bit = 4 >> axis;
                let face: Vec<u32> = (0..8u32)
                    .filter(|k| (k & bit != 0) == (side == 1))
                    .collect();
                // the face's corners in a ring: sorted by angle around its centre is overkill; the 4 corners'
                // two diagonals are (0,3) and (1,2) of the sorted list
                for tri in [[face[0], face[1], face[3]], [face[0], face[3], face[2]]] {
                    let (a, b, c) = (v(tri[0]), v(tri[1]), v(tri[2]));
                    let n = (b - a).cross(c - a);
                    let want = if side == 1 { 1.0 } else { -1.0 };
                    let dir = if axis == 0 {
                        n.x
                    } else if axis == 1 {
                        n.y
                    } else {
                        n.z
                    };
                    if dir * want > 0.0 {
                        idx.extend_from_slice(&tri);
                    } else {
                        idx.extend_from_slice(&[tri[0], tri[2], tri[1]]);
                    }
                }
            }
        }
        mesh.parts.push(level::Part {
            material: None,
            indices: idx,
        });
        let mut g = script::Graph::default();
        let mut level = Level::default();
        level.meshes.push(mesh);
        for k in 0..n {
            let name = format!("Box{k}");
            g.add_object(&name, script::ObjKind::Entity(k));
            level.objects.push(level::Object {
                name,
                matrix: Mat4::from_translation(Vec3::new(10.0 * k as f32, 5.0, 0.0))
                    .to_cols_array_2d(),
                mesh: Some(0),
                visible: true,
                sprite: None,
                parent: None,
            });
        }
        level.scripts = Some(std::sync::Arc::new(g));
        Scripts::new(&level).expect("a graph")
    }

    fn view() -> RunView {
        RunView {
            tick: 0,
            ball: Vec3::ZERO,
            ball_radius: 1.0,
            points: 0,
            in_play: true,
            teleport: None,
            input: script::InputFrame::default(),
            input_before: script::InputFrame::default(),
            lives: 3,
            sector: 1,
            kind: crate::sim::BallKind::Wood,
            finished: false,
            dying: false,
            last_stage: false,
            cursor: None,
            eye: Vec3::ZERO,
            target: Vec3::Z,
        }
    }

    fn physicalize(s: &mut Scripts, k: usize, sp: &script::PhysSpec) {
        s.with_host(&mut view(), |_, h| {
            h.physicalize_body(Ent::Object(k), 0, sp)
        });
    }

    fn run(s: &mut Scripts, w: &mut World, ticks: u32) {
        for _ in 0..ticks {
            s.apply_physics(w, 0);
            w.step(None);
            s.sync_physics(w);
        }
    }

    fn y_of(s: &Scripts, k: usize) -> f32 {
        s.ents.world(k).expect("row").w_axis.y
    }

    #[test]
    fn a_box_made_by_a_script_falls_and_rests_on_the_floor() {
        let mut s = scripts(1);
        let mut w = World::new(&floor());
        physicalize(&mut s, 0, &spec());
        run(&mut s, &mut w, 240);
        // the cube's half height is 1: it rests with its centre about 1 above the floor
        let y = y_of(&s, 0);
        assert!((y - 1.0).abs() < 0.15, "rests at y = {y}");
        assert!(s.ents.is_moved(0), "the row follows the body");
    }

    #[test]
    fn an_impulse_moves_it_and_a_second_physicalize_changes_nothing() {
        let mut s = scripts(1);
        let mut w = World::new(&floor());
        physicalize(&mut s, 0, &spec());
        run(&mut s, &mut w, 240);
        let x0 = s.ents.world(0).unwrap().w_axis.x;
        let at = s.ents.world(0).unwrap().w_axis.truncate();
        s.with_host(&mut view(), |_, h| {
            h.physics_impulse(Ent::Object(0), at.to_array(), [8.0, 0.0, 0.0]);
            h.physicalize_body(Ent::Object(0), 0, &spec());
        });
        run(&mut s, &mut w, 60);
        assert!(
            s.ents.world(0).unwrap().w_axis.x > x0 + 0.5,
            "pushed along +x"
        );
        assert_eq!(
            s.made, 1,
            "the second Physicalize did not make another body"
        );
    }

    #[test]
    fn a_force_pushes_until_destroyed_and_unphysicalize_removes_the_body() {
        let mut s = scripts(1);
        let mut w = World::new(&floor());
        physicalize(&mut s, 0, &spec());
        run(&mut s, &mut w, 120);
        // a force does nothing to a sleeping body (docs/physics.md 2.8): the script wakes it first
        s.with_host(&mut view(), |_, h| {
            h.physics_wake(Ent::Object(0));
            h.physics_force(Ent::Object(0), 7, true, [0.0; 3], [0.5, 0.0, 0.0])
        });
        run(&mut s, &mut w, 30);
        assert!(
            s.ents.world(0).unwrap().w_axis.x > 0.2,
            "the force moves it"
        );
        s.with_host(&mut view(), |_, h| h.physicalize(Ent::Object(0), false));
        run(&mut s, &mut w, 2);
        assert!(s.bodies.is_empty());
        let y = y_of(&s, 0);
        run(&mut s, &mut w, 60);
        assert_eq!(y, y_of(&s, 0), "the row is left where the body was");
    }

    #[test]
    fn a_rewound_run_gives_the_same_bodies() {
        let mut s = scripts(1);
        let mut w = World::new(&floor());
        physicalize(&mut s, 0, &spec());
        run(&mut s, &mut w, 30);
        let (s2, w2) = (s.clone(), w.clone());
        run(&mut s, &mut w, 90);
        let (mut s2, mut w2) = (s2, w2);
        run(&mut s2, &mut w2, 90);
        assert_eq!(s.ents.world(0), s2.ents.world(0));
    }

    #[test]
    fn caps_hold() {
        let n = MAX_SCRIPT_BODIES + 10;
        let mut s = scripts(n);
        for k in 0..n {
            physicalize(&mut s, k, &spec());
        }
        assert_eq!(s.made, MAX_SCRIPT_BODIES);
        assert_eq!(s.bodies.len(), MAX_SCRIPT_BODIES);
    }

    fn fixed_spec() -> script::PhysSpec {
        script::PhysSpec {
            fixed: true,
            convex_count: 0,
            concave_count: 1,
            ..spec()
        }
    }

    fn unphysicalize(s: &mut Scripts, k: usize) {
        s.with_host(&mut view(), |_, h| h.physicalize(Ent::Object(k), false));
    }

    /// A ball above the cube at x = 0 (the cube's top is at y = 6).
    fn ball_above(w: &mut World) -> usize {
        w.add_sphere(
            1.0,
            crate::physics::Material {
                friction: 0.7,
                elasticity: 0.3,
                mass: 1.0,
                lin_damp: 0.1,
                rot_damp: 0.1,
            },
            Vec3::new(0.0, 12.0, 0.0),
            glam::Quat::IDENTITY,
            true,
        )
    }

    fn ball_y(w: &World, b: usize) -> f32 {
        w.state(b).expect("ball").pos.y
    }

    #[test]
    fn a_fixed_box_by_script_stops_the_ball_and_unphysicalize_lets_it_fall() {
        let mut s = scripts(1);
        let mut w = World::new(&[]);
        let b = ball_above(&mut w);
        physicalize(&mut s, 0, &fixed_spec());
        run(&mut s, &mut w, 180);
        let y = ball_y(&w, b);
        assert!((y - 7.0).abs() < 0.3, "rests on the box at y = {y}");
        assert!(
            !w.floor_near(Vec3::new(0.0, 6.0, 0.0), 3.0).is_empty(),
            "the ball's floor lookup sees it"
        );
        unphysicalize(&mut s, 0);
        // a body at rest sleeps on (the physics forgets the contacts of the body that goes but wakes nobody; the
        // original is the same, and its scripts wake bodies themselves)
        w.wake(b);
        run(&mut s, &mut w, 120);
        assert!(
            ball_y(&w, b) < -10.0,
            "falls through, y = {}",
            ball_y(&w, b)
        );
        assert!(w.floor_near(Vec3::new(0.0, 6.0, 0.0), 3.0).is_empty());
        assert!(s.statics.is_empty());
    }

    #[test]
    fn a_fixed_hull_by_script_stops_the_ball_too() {
        let mut s = scripts(1);
        let mut w = World::new(&[]);
        let b = ball_above(&mut w);
        physicalize(
            &mut s,
            0,
            &script::PhysSpec {
                fixed: true,
                ..spec()
            },
        );
        run(&mut s, &mut w, 180);
        assert!((ball_y(&w, b) - 7.0).abs() < 0.3);
    }

    #[test]
    fn a_fixed_body_follows_its_moved_object() {
        let mut s = scripts(1);
        let mut w = World::new(&[]);
        let b = ball_above(&mut w);
        physicalize(&mut s, 0, &fixed_spec());
        run(&mut s, &mut w, 1);
        let h = s.statics[&0].handle.expect("made");
        assert_eq!(w.static_pose(h).expect("pose").0, Vec3::new(0.0, 5.0, 0.0));
        s.ents
            .set_world(0, Mat4::from_translation(Vec3::new(50.0, 5.0, 0.0)));
        run(&mut s, &mut w, 1);
        let h = s.statics[&0].handle.expect("still there");
        assert_eq!(w.static_pose(h).expect("pose").0, Vec3::new(50.0, 5.0, 0.0));
        // the ball no longer has it below: it falls through
        run(&mut s, &mut w, 120);
        assert!(ball_y(&w, b) < -10.0);
        assert!(!w.floor_near(Vec3::new(50.0, 6.0, 0.0), 3.0).is_empty());
        assert!(w.floor_near(Vec3::new(0.0, 6.0, 0.0), 3.0).is_empty());
    }

    #[test]
    fn a_cloned_run_gives_the_same_world_and_rewind_restores_it() {
        let mut s = scripts(2);
        let mut w = World::new(&[]);
        let b = ball_above(&mut w);
        physicalize(&mut s, 0, &fixed_spec());
        run(&mut s, &mut w, 30);
        let (s0, w0) = (s.clone(), w.clone());
        unphysicalize(&mut s, 0);
        physicalize(&mut s, 1, &fixed_spec());
        run(&mut s, &mut w, 100);
        // the copy taken earlier, run the same way, ends the same
        let (mut s1, mut w1) = (s0.clone(), w0.clone());
        unphysicalize(&mut s1, 0);
        physicalize(&mut s1, 1, &fixed_spec());
        run(&mut s1, &mut w1, 100);
        assert_eq!(ball_y(&w, b).to_bits(), ball_y(&w1, b).to_bits());
        assert_eq!(w.static_count(), w1.static_count());
        // and the saved copy still has the box
        let (mut s2, mut w2) = (s0, w0);
        run(&mut s2, &mut w2, 100);
        assert!(
            (ball_y(&w2, b) - 7.0).abs() < 0.3,
            "the rewound run still stands on it"
        );
    }

    #[test]
    fn an_object_with_the_levels_body_gets_no_second_one_and_unphysicalize_removes_it() {
        let mut s = scripts(2);
        let level_floor = floor();
        let mut w = World::new(&level_floor);
        w.tag_baked(&[0]);
        let n = w.static_count();
        physicalize(&mut s, 0, &fixed_spec());
        physicalize(&mut s, 1, &spec());
        s.apply_physics(&mut w, 0);
        // the movable one of object 1 is made; object 0 keeps the level's body, nothing is added for it
        assert_eq!(w.static_count(), n);
        assert!(s.statics.is_empty());
        physicalize(&mut s, 0, &spec());
        s.apply_physics(&mut w, 0);
        assert!(!s.bodies.contains_key(&0));
        unphysicalize(&mut s, 0);
        s.apply_physics(&mut w, 0);
        assert_eq!(w.static_count(), n - 1);
        assert!(!w.has_baked(0));
    }

    #[test]
    fn a_world_without_script_statics_is_the_baked_world() {
        let a = World::new(&floor());
        let mut b = World::new(&floor());
        b.tag_baked(&[0]);
        assert_eq!(a.static_count(), b.static_count());
        let na = a.floor_near(Vec3::new(0.0, 0.0, 0.0), 50.0);
        let nb = b.floor_near(Vec3::new(0.0, 0.0, 0.0), 50.0);
        assert_eq!(na, nb);
    }

    #[test]
    fn a_baked_body_follows_its_moved_object() {
        let mut s = scripts(1);
        let mut w = World::new(&floor());
        w.tag_baked(&[0]);
        let b = ball_above(&mut w);
        // row 0's own body is the 200 x 200 floor at y = 0 here: move the object 500 along x and the floor goes too
        run(&mut s, &mut w, 120);
        assert!(ball_y(&w, b) > 0.0 && ball_y(&w, b) < 3.0, "rests on the floor");
        s.ents.set_world(0, Mat4::from_translation(Vec3::new(500.0, 5.0, 0.0)));
        w.wake(b);
        run(&mut s, &mut w, 120);
        assert!(ball_y(&w, b) < -10.0, "the floor went with its object");
        assert!(w.has_baked(0));
        // and a ball dropped over the new place lands on it
        let c = ball_above(&mut w);
        w.teleport(c, Vec3::new(500.0, 12.0, 0.0), glam::Quat::IDENTITY);
        run(&mut s, &mut w, 180);
        assert!(ball_y(&w, c) > 0.0 && ball_y(&w, c) < 3.0, "y = {}", ball_y(&w, c));
        // a level without moved objects keeps its body where it was
        let mut s2 = scripts(1);
        let mut w2 = World::new(&floor());
        w2.tag_baked(&[0]);
        let b2 = ball_above(&mut w2);
        run(&mut s2, &mut w2, 240);
        assert!(ball_y(&w2, b2) > 0.0);
    }

    #[test]
    fn a_script_body_moved_by_a_script_is_teleported() {
        let mut s = scripts(1);
        let mut w = World::new(&floor());
        physicalize(&mut s, 0, &spec());
        run(&mut s, &mut w, 240);
        s.ents.set_world(0, Mat4::from_translation(Vec3::new(30.0, 20.0, 0.0)));
        s.apply_physics(&mut w, 0);
        let b = s.bodies[&0].expect("body");
        let st = w.state(b).expect("state");
        assert!((st.pos - Vec3::new(30.0, 20.0, 0.0)).length() < 1e-3, "{:?}", st.pos);
        run(&mut s, &mut w, 240);
        let m = s.ents.world(0).unwrap();
        assert!((m.w_axis.x - 30.0).abs() < 2.0, "x = {}", m.w_axis.x);
        assert!((m.w_axis.y - 1.0).abs() < 0.2, "it fell onto the floor again, y = {}", m.w_axis.y);
    }

    #[test]
    fn a_fixed_ball_shape_makes_a_fixed_sphere() {
        let mut s = scripts(1);
        let mut w = World::new(&[]);
        let b = ball_above(&mut w);
        physicalize(
            &mut s,
            0,
            &script::PhysSpec {
                fixed: true,
                convex_count: 0,
                ball_count: 1,
                radius: Some(3.0),
                ..spec()
            },
        );
        run(&mut s, &mut w, 240);
        assert_eq!(s.statics.len(), 1);
        assert_eq!(s.made, 0);
        assert_eq!(w.static_count(), 1);
        // the sphere (centre y = 5, radius 3) holds the ball (radius 1) on its top at y = 9, at the most
        assert!(ball_y(&w, b) > 8.0, "y = {}", ball_y(&w, b));
        unphysicalize(&mut s, 0);
        s.apply_physics(&mut w, 0);
        assert_eq!(w.static_count(), 0);
    }

    #[test]
    fn script_bodies_of_one_named_group_do_not_collide_with_each_other() {
        let stack = |group: &str| {
            let mut s = scripts(2);
            let mut w = World::new(&floor());
            s.ents.set_world(0, Mat4::from_translation(Vec3::new(0.0, 5.0, 0.0)));
            s.ents.set_world(1, Mat4::from_translation(Vec3::new(0.0, 9.0, 0.0)));
            let sp = script::PhysSpec {
                group: group.into(),
                ..spec()
            };
            physicalize(&mut s, 0, &sp);
            physicalize(&mut s, 1, &sp);
            run(&mut s, &mut w, 300);
            (y_of(&s, 0), y_of(&s, 1))
        };
        let (a, b) = stack("");
        assert!(b > a + 1.5, "no group: stacked, {a} {b}");
        let (a, b) = stack("Pile");
        assert!((a - 1.0).abs() < 0.2 && (b - 1.0).abs() < 0.2, "same group: both on the floor, {a} {b}");
    }

    #[test]
    fn the_pause_messages_are_sent_and_logged() {
        let mut s = scripts(1);
        s.pause_event(true, 5);
        s.pause_event(false, 6);
        let sent: Vec<&str> = s.log.iter().map(|l| l.1.as_str()).collect();
        assert_eq!(
            sent,
            [
                "game message Pause Level",
                "game message Unpause Level",
                "game message Refresh"
            ]
        );
    }

    #[test]
    fn a_live_pause_is_recorded_and_a_playback_ignores_live_ones() {
        use crate::sim::Sim;
        let level = level::Level::default();
        let mut sim = Sim::new(&level, crate::sim::BallKind::Wood);
        sim.notify_pause(true);
        sim.notify_pause(false);
        assert_eq!(sim.pauses, [(1, true), (1, false)]);
        let mut play = Sim::new(&level, crate::sim::BallKind::Wood);
        play.set_playback_pauses(Some(vec![(2, true)]));
        play.notify_pause(true);
        assert!(play.pauses.is_empty(), "live pauses are ignored in playback");
        play.step(0);
        play.step(0);
        assert_eq!(play.pauses, [(2, true)], "delivered at its tick");
    }

    #[test]
    fn fixed_bodies_are_capped() {
        let mut s = scripts(MAX_SCRIPT_STATICS + 5);
        for k in 0..MAX_SCRIPT_STATICS + 5 {
            physicalize(&mut s, k, &fixed_spec());
        }
        assert_eq!(s.statics.len(), MAX_SCRIPT_STATICS);
        assert_eq!(s.made, 0);
    }
}
