// Running the game: in a window (keyboard or a replay drives the simulation), or without one (replay checks and
// screenshots). The simulation always advances in fixed 60 Hz ticks; the window draws at the display's rate and
// interpolates between the last two ticks.

use crate::Options;
use crate::assets::{Data, LoadedLevel};
use crate::audio::Audio;
use crate::ballsound::{BallSound, FanSound, SoundCmd};
use crate::collide::closest_point;
use crate::freecam::FreeCam;
use crate::menu::{Action, Menu};
use crate::music::Music;
use crate::render::Renderer;
use crate::replay::Replay;
use crate::scene::{self, BallMeshes, Snapshot};
use crate::sim::{BALL_RADIUS, Event, Sim, TICK};
use crate::watch::{Finding, Watch};
/// The language `--screenshot` draws the menus in (DRAHTSEILAKT_SHOT_LANG), else the settings'.
static SHOT_LANG: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// A value of the settings file; the language of a menu screenshot may be set apart from it.
pub fn setting(key: &str) -> Option<String> {
    match (key, SHOT_LANG.get()) {
        ("language", Some(l)) => Some(l.clone()),
        _ => gamedata::profile::setting(key),
    }
}
use gamedata::profile::{settings_path, settings_text};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

/// The frame timings of one measuring period (DRAHTSEILAKT_TIMING), printed every two seconds.
#[derive(Debug)]
struct Timing {
    since: Instant,
    frames: u64,
    ticks: u64,
    simulate: std::time::Duration,
    frame: std::time::Duration,
    /// Where the drawing time goes: acquire, scene instances, draw, overlay, present.
    stages: [std::time::Duration; 5],
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            since: Instant::now(),
            frames: 0,
            ticks: 0,
            simulate: std::time::Duration::ZERO,
            frame: std::time::Duration::ZERO,
            stages: [std::time::Duration::ZERO; 5],
        }
    }
}

struct Game {
    /// No level is loaded: the menu at the start (see `Game::placeholder`).
    placeholder: bool,
    /// The game's files for this level: its parts ours or the copy's first, as the level says (Data::with_parts).
    data: Data,
    level: LoadedLevel,
    start_ball: crate::sim::BallKind,
    balls: LoadedLevel,
    ball_meshes: BallMeshes,
    /// The balls' pieces, for each new run's bursts.
    pieces: crate::pieces::Pieces,
    placements: Vec<scene::Placement>,
    /// Whether the sacks' supports were looked for (`show_supports`): only once a mod asks for them.
    supports_planned: bool,
    modules: scene::Modules,
    /// The transformer animation's file (`AnimTrafo.nmo`), drawn after the modules, if the copy has it.
    trafo: Option<(LoadedLevel, crate::trafo::TrafoModel)>,
    sim: Sim,
    replay_in: Option<Replay>,
    replay_out: Replay,
    /// The view's turn the window hands to the next tick (a `view.turn` mod), whole degrees; a replay brings its own.
    pub view_turn: u16,
    /// Input bits a mod's key asked for, pressed at the next tick only (the spawn point's, `input::SPAWN_*`).
    pulse: u8,
    /// The live keyboard and mouse for the scripts, set by the window each frame; a replay brings its own.
    pub live_input: script::InputFrame,
    /// Where the run begins when not at the level start (--sector, --at, --ball, or the replay's).
    start: Option<crate::replay::Start>,
    record: Option<PathBuf>,
    /// The tick the ball was last lost, for the "ball lost" message.
    ball_lost_at: Option<u64>,
    /// Looks for stuck balls and launches; see `watch.rs`.
    watch: Watch,
    /// The ball's spin magnitude at the last tick (for the spin-loss dump in `log_tick`).
    prev_spin: f32,
    /// The session log of a windowed run: the ball and its contacts at every tick (see `start_session`).
    session: Option<std::io::BufWriter<std::fs::File>>,
    /// The extra points' module meshes, when the copy has the module.
    extra_model: Option<scene::ExtraModel>,
    /// The particle systems (flames, the extra points' sparkles); only for the eye.
    effects: crate::particles::Effects,
    /// The interface's face: the own one in the shipped fonts, or the copy's font (Font_1 with its glyph table) for
    /// the original's interface.
    font: Option<crate::ui::Face>,
    /// The floors that take the ball's shadow.
    shadow_receivers: Vec<scene::ShadowReceiver>,
    /// The level's music theme (the level table's Music), and whether it is the last original level.
    music_theme: Option<u32>,
    /// The level table's LevelBonus.
    level_bonus: i64,
    /// The lengths of the sounds the map's scripts play, in ticks (scripting.rs `SoundState`).
    sound_ticks: std::sync::Arc<std::collections::HashMap<String, u64>>,
    /// The run was helped (slowed down, or taken back to a saved moment): it is kept, but never the best.
    assisted: bool,
    /// The mods that run on the run's side (modrt.rs), as each new run starts with them: a replay's own (its `mod`
    /// lines), else what the window sets (the mods switched on).
    mods: crate::modrt::SimMods,
    last_level: bool,
    /// Where the level's runs are kept: the original levels' in replays/, a custom level's under replays/maps/ in its
    /// folder's name (community, open, remastered), so no two kinds share runs.
    runs_root: PathBuf,
    /// The level's name as shown (the window's title): a custom level's title, capped (package.rs), else its file's.
    shown_name: String,
    /// The ball's sounds, and what they ask of the sound output since the window last played them.
    ball_sound: BallSound,
    fan_sound: FanSound,
    pub sound_out: Vec<SoundCmd>,
}

/// The start the options ask for, if any: --at is a point or a level object (the ball 4 above it), the sector the
/// one given or the nearest object's.
fn start_of(o: &Options, level: &level::Level) -> Result<Option<crate::replay::Start>, String> {
    if o.sector.is_none() && o.at.is_none() && o.ball.is_none() {
        return Ok(None);
    }
    let at = match &o.at {
        None => None,
        Some(v) => Some(match crate::replay::parse_point(v) {
            Some(p) => p,
            None => {
                let obj = level
                    .objects
                    .iter()
                    .find(|ob| ob.name.eq_ignore_ascii_case(v))
                    .ok_or_else(|| format!("--at: no point or object {v}"))?;
                let [x, y, z] = obj.position();
                [x, y + 4.0, z]
            }
        }),
    };
    let sector = o
        .sector
        .or_else(|| {
            at.and_then(|p| crate::sim::sector_near(level, glam::Vec3::from(p)))
                .map(|n| n + 1)
        })
        .unwrap_or(1);
    Ok(Some(crate::replay::Start {
        sector,
        ball: o.ball.clone(),
        at,
    }))
}

fn begin_at(sim: &mut Sim, st: &crate::replay::Start) {
    let kind = st.ball.as_deref().and_then(crate::sim::BallKind::from_name);
    sim.start_from(st.sector - 1, kind, st.at.map(glam::Vec3::from));
}

impl Game {
    fn new(o: &Options, data: &Data) -> Result<Game, String> {
        Self::build(o, data, false)
    }

    /// A game with no level in it, for the menu at the start: nothing is read or built, the level comes when the
    /// player picks one (`App::switch`). It costs next to nothing, and is never run.
    fn placeholder(o: &Options, data: &Data) -> Result<Game, String> {
        Self::build(o, data, true)
    }

    fn build(o: &Options, data: &Data, empty: bool) -> Result<Game, String> {
        let mut level = if empty {
            LoadedLevel {
                level: level::Level::default(),
                name: "Drahtseilakt".into(),
                hash: 0,
                warnings: Vec::new(),
            }
        } else {
            data.load(&o.source)?
        };
        crate::loading::stage(crate::loading::Stage::Building);
        let data = &data.with_parts(&level.level);
        scene::cloud_material(&mut level.level);
        for w in &level.warnings {
            eprintln!("warning: {w}");
        }
        // a map's own scripts: say what they are made of, and which blocks this engine does not know yet
        if let Some(graph) = &level.level.scripts {
            let protos = graph.prototypes();
            let unknown: Vec<String> = protos
                .iter()
                .filter(|(g, _, _)| !script::blocks::known(*g))
                .map(|(g, name, n)| format!("{name} ({g}) x{n}"))
                .collect();
            eprintln!(
                "scripts: {} of them, {} blocks of {} kinds{}",
                graph.scripts.len(),
                protos.iter().map(|p| p.2).sum::<usize>(),
                protos.len(),
                if unknown.is_empty() {
                    String::new()
                } else {
                    format!("; not known: {}", unknown.join(", "))
                }
            );
        }
        // the shared parts (balls, modules, transformer): the level folder's own when it has them, else ours or the
        // copy's (assets.rs); without any the ball is drawn plain and bursts into nothing (said once, never fatal)
        let balls = (!empty)
            .then(|| data.part("3D Entities/Balls.nmo"))
            .flatten()
            .unwrap_or_else(|| crate::assets::LoadedLevel {
                level: level::Level::default(),
                name: "Balls".into(),
                hash: 0,
                warnings: Vec::new(),
            });
        let replay_in = match &o.replay {
            Some(p) => {
                let text =
                    String::from_utf8(crate::hints::read_plain(p, crate::hints::MAX_REPLAY_FILE)?)
                        .map_err(|_| format!("{}: not UTF-8", p.display()))?;
                let r = Replay::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
                // a replay plays on its level file, or on a level converted from that file (an open-format export
                // records its source's hash): the same level, so the same run
                if r.level_hash != level.hash
                    && Some(r.level_hash) != level.level.settings.source_hash
                {
                    return Err(format!(
                        "{}: recorded on another level file than {}",
                        p.display(),
                        level.name
                    ));
                }
                if let Some(why) =
                    crate::replay::stale_for_scripts(r.version, level.level.scripts.is_some())
                {
                    return Err(format!("{}: {why}", p.display()));
                }
                Some(r)
            }
            None => None,
        };
        let start_ball = level
            .level
            .start_ball
            .as_deref()
            .and_then(crate::sim::BallKind::from_name)
            .unwrap_or(crate::sim::BallKind::Wood);
        let modules = scene::load_modules(&level.level, data);
        let extra_model = scene::ExtraModel::new(&modules);
        // The original's level 1 begins with its tutorial, which stops the level before the ball comes (the level's
        // settings say so: an .nmo's from the game's table, an open-format level's its own).
        let tutorial = level.level.settings.tutorial;
        let ball_meshes = BallMeshes::find(&balls.level);
        let format = replay_in
            .as_ref()
            .map_or(crate::replay::VERSION, |r| r.version);
        let mut sim = Sim::with_rules(&level.level, start_ball, &modules, tutorial, format);
        sim.set_ball_boxes(ball_meshes.boxes);
        let pieces = crate::pieces::Pieces::find(&balls.level);
        sim.set_pieces(pieces.clone());
        // a replay recorded before the bursts plays without them, as it was recorded
        sim.bursts = replay_in.as_ref().is_none_or(|r| r.version >= 2);
        sim.pieces_expire = replay_in.as_ref().is_none_or(|r| r.version >= 3);
        sim.checkpoints_by_name = replay_in.as_ref().is_none_or(|r| r.version >= 4);
        sim.respawn_faces_on = replay_in.as_ref().is_none_or(|r| r.version >= 6);
        sim.set_playback_pauses(replay_in.as_ref().map(|r| r.pauses.clone()));
        let level_bonus = level.level.settings.level_bonus.unwrap_or(0);
        sim.level_bonus = level_bonus;
        let sound_ticks = std::sync::Arc::new(script_sound_ticks(&sim, data));
        sim.set_sound_ticks(sound_ticks.clone());
        // a replay played with mods that changed the run plays with the very same mods, or not at all
        let mods = match &replay_in {
            Some(r) if !r.mods.is_empty() => {
                crate::modrt::Mods::load(gamedata::mods::scan(o.debug), setting)
                    .for_replay(&r.mod_refs())
                    .map_err(|e| {
                        format!(
                            "{}: {e}",
                            o.replay
                                .as_ref()
                                .map_or(String::new(), |p| p.display().to_string())
                        )
                    })?
            }
            _ => crate::modrt::SimMods::default(),
        };
        sim.mods = mods.clone();
        let start = match &replay_in {
            Some(r) => r.start.clone(),
            None => start_of(o, &level.level)?,
        };
        if let Some(st) = &start {
            if st.sector > sim.sector_count() {
                return Err(format!("{} has {} sectors", level.name, sim.sector_count()));
            }
            begin_at(&mut sim, st);
        }
        let mut replay_out = Replay::new(&level.name, level.hash);
        replay_out.start = start.clone();
        replay_out.mods = mods.replay_lines();
        // a run re-recorded from an old replay plays by its rules, so it keeps its format
        if let Some(r) = &replay_in {
            replay_out.version = r.version;
        }
        let module_file = |name: &str| {
            data.part(&format!("3D Entities/PH/{name}.nmo"))
                .map(|m| m.level)
        };
        let effects = crate::particles::Effects::new(&level.level, &module_file);
        let mut placements =
            scene::placements(&level.level, &modules, sim.loose(), sim.module_bodies());
        if effects.has_flames() {
            // the flames are drawn, so their stand-ins need no markers
            placements.retain(|p| {
                p.marker.is_none()
                    || !level.level.groups_of(p.object).any(|g| {
                        g.eq_ignore_ascii_case("PC_Checkpoints")
                            || g.eq_ignore_ascii_case("PS_Levelstart")
                    })
            });
        }
        let trafo_model = scene::FIRST_MODULE_MODEL + modules.models.len();
        let trafo = (!empty)
            .then(|| data.part("3D Entities/AnimTrafo.nmo"))
            .flatten()
            .and_then(|f| crate::trafo::TrafoModel::new(&f.level, trafo_model).map(|m| (f, m)));
        Ok(Game {
            data: data.clone(),
            trafo,
            placements,
            supports_planned: false,
            modules,
            ball_meshes,
            pieces,
            sim,
            start_ball,
            replay_out,
            view_turn: 0,
            pulse: 0,
            live_input: script::InputFrame::default(),
            start,
            record: o.record.clone(),
            ball_lost_at: None,
            watch: Watch::default(),
            session: None,
            extra_model,
            effects,
            font: crate::ui::Face::new(data, gamedata::resources::in_use()),
            shadow_receivers: scene::shadow_receivers(&level.level),
            music_theme: level.level.settings.music,
            level_bonus,
            sound_ticks,
            assisted: false,
            mods,
            last_level: level.level.settings.last_level,
            placeholder: empty,
            runs_root: {
                let (root, old) = if empty {
                    (replay_dir(), None)
                } else {
                    runs_root(&o.source)
                };
                let stem = level.name.trim_end_matches(".NMO").trim_end_matches(".nmo");
                match old.map(|old| move_runs(&old, &root, stem)) {
                    Some(Ok(true)) => {
                        println!("runs of {} moved to {}", shown(stem), shown_path(&root))
                    }
                    Some(Err(e)) => {
                        eprintln!("could not move the runs of {}: {}", shown(stem), shown(&e))
                    }
                    _ => {}
                }
                root
            },
            shown_name: match &o.source {
                _ if empty => "Drahtseilakt".into(),
                crate::assets::Source::Original(_) => level.name.clone(),
                crate::assets::Source::Nmo(_) | crate::assets::Source::Open(_) => {
                    loading_name(&o.source)
                }
            },
            ball_sound: BallSound::default(),
            fan_sound: FanSound::default(),
            sound_out: Vec::new(),
            prev_spin: 0.0,
            replay_in,
            level,
            balls,
        })
    }

    fn restart(&mut self) {
        self.save_attempt();
        self.assisted = false;
        self.sim = self.fresh_sim(
            self.replay_in
                .as_ref()
                .map_or(crate::replay::VERSION, |r| r.version),
        );
        self.sim.mods = self.mods.clone();
        self.ball_sound.silence(&mut self.sound_out);
        self.fan_sound.silence(&mut self.sound_out);
        if let Some(st) = &self.start {
            begin_at(&mut self.sim, st);
        }
        self.ball_lost_at = None;
        self.watch = Watch::default();
        self.replay_out = Replay::new(&self.level.name, self.level.hash);
        self.replay_out.start = self.start.clone();
        self.replay_out.mods = self.mods.replay_lines();
        if let Some(r) = &self.replay_in {
            self.replay_out.version = r.version;
        }
    }

    /// The folder of this level's runs, attempts, watch log and findings.
    fn run_dir(&self) -> PathBuf {
        let stem = self
            .level
            .name
            .trim_end_matches(".NMO")
            .trim_end_matches(".nmo");
        replay_dir().join(stem)
    }

    /// Reports a finding: a line on the console and in the level's watch log, and the replay up to that tick, so that
    /// the case replays with --headless. A replay played back is only reported, not logged again.
    fn report(&self, f: &Finding, contacts: &[ivp::glue::ContactReadout]) {
        let line = format!(
            "{} {} tick {} at ({:.2} {:.2} {:.2}) speed {:.2} input {:#04x}{}",
            clock(f.tick),
            f.kind.name(),
            f.tick,
            f.pos.x,
            f.pos.y,
            f.pos.z,
            f.vel.length(),
            f.input,
            if f.on_ground { " ground" } else { " air" }
        );
        // the ball's contact points at that tick: which surfaces it touches, and along which normal
        let mut lines = vec![line];
        for c in contacts {
            lines.push(format!(
                "  contact {:?} types {:?} gap {:.4} normal ({:.3} {:.3} {:.3}) point ({:.2} {:.2} {:.2})",
                c.entities, c.types, c.gap, c.normal[0], c.normal[1], c.normal[2], c.point[0], c.point[1], c.point[2]
            ));
        }
        for l in &lines {
            println!("{l}");
        }
        if self.replay_in.is_some() {
            return;
        }
        let dir = self.run_dir();
        let _ = std::fs::create_dir_all(&dir);
        use std::io::Write as _;
        if let Ok(mut log) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("watch.log"))
        {
            for l in &lines {
                let _ = writeln!(log, "{l}");
            }
        }
        let snapshot = dir.join(format!("{}-{}.replay", f.kind.name(), f.tick));
        if let Err(e) = std::fs::write(&snapshot, self.replay_out.to_text()) {
            eprintln!(
                "could not save the replay of the {} to {}: {e}",
                f.kind.name(),
                snapshot.display()
            );
        }
    }

    /// Opens the session log of a windowed run in the level's folder: one line for the ball at every tick, and one
    /// for each of its contacts. A replay played back is logged too, so the same case can be compared with the run.
    fn start_session(&mut self) {
        if self.placeholder {
            return;
        }
        let dir = self.run_dir();
        let _ = std::fs::create_dir_all(&dir);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("session-{stamp}.log"));
        match std::fs::File::create(&path) {
            Ok(f) => {
                let mut w = std::io::BufWriter::new(f);
                use std::io::Write as _;
                let _ = writeln!(
                    w,
                    "# {} from tick {} ({})",
                    self.level.name,
                    self.sim.tick,
                    clock(self.sim.tick)
                );
                let _ = writeln!(
                    w,
                    "# assets {} | data {}",
                    gamedata::resources::assets_root()
                        .map_or("(none)".into(), |p| p.display().to_string()),
                    gamedata::profile::data_dir().display()
                );
                self.session = Some(w);
                println!("session log: {}", path.display());
            }
            Err(e) => eprintln!("could not open the session log {}: {e}", path.display()),
        }
    }

    /// One line in the session log (a stall), written out at once.
    fn log_line(&mut self, line: &str) {
        use std::io::Write as _;
        if let Some(w) = &mut self.session {
            let _ = writeln!(w, "# {line}");
            let _ = w.flush();
        }
    }

    /// One session line for the ball and one per contact, after the tick.
    fn log_tick(&mut self, input: u8) {
        let Some(w) = &mut self.session else { return };
        use std::io::Write as _;
        let b = &self.sim.ball;
        let _ = writeln!(
            w,
            "{} {} ball {:.4} {:.4} {:.4} vel {:.3} {:.3} {:.3} spin {:.3} {:.3} {:.3} {} input {:#04x} {:?} ms {}",
            self.sim.tick,
            clock(self.sim.tick),
            b.position.x,
            b.position.y,
            b.position.z,
            b.velocity.x,
            b.velocity.y,
            b.velocity.z,
            b.spin.x,
            b.spin.y,
            b.spin.z,
            if b.on_ground { "ground" } else { "air" },
            input,
            b.kind,
            self.sim.world().movement_state(0).map_or(-1, |m| m),
        );
        for c in self.sim.world().ball_contacts() {
            let _ = writeln!(
                w,
                "  contact {:?} types {:?} gap {:.4} pressure {:.3} normal {:.3} {:.3} {:.3} point {:.3} {:.3} {:.3}",
                c.entities,
                c.types,
                c.gap,
                c.pressure,
                c.normal[0],
                c.normal[1],
                c.normal[2],
                c.point[0],
                c.point[1],
                c.point[2]
            );
        }
        if self.sim.tick.is_multiple_of(60) {
            let _ = w.flush();
        }
    }

    /// A landing that takes more than 3 rad/s of spin in one tick: the floor around the ball is written to the level's
    /// folder, so that the case can be rebuilt exactly (the replay reproduces the tick).
    fn dump_spin_loss(&mut self) {
        let (position, velocity, spin_v, on_ground) = {
            let b = &self.sim.ball;
            (b.position, b.velocity, b.spin, b.on_ground)
        };
        let spin = spin_v.length();
        if on_ground && self.prev_spin - spin > 3.0 {
            let floor = self.sim.world().floor_near(position, 6.0);
            let mut text = format!(
                "# tick {} {} ball position {:?} velocity {:?} spin {:?} (spin {:.3} before, {:.3} now)\n",
                self.sim.tick,
                clock(self.sim.tick),
                position,
                velocity,
                spin_v,
                self.prev_spin,
                spin
            );
            for t in floor {
                text.push_str(&format!(
                    "tri {} {} {} {} {} {} {} {} {}\n",
                    t[0].x, t[0].y, t[0].z, t[1].x, t[1].y, t[1].z, t[2].x, t[2].y, t[2].z
                ));
            }
            let dir = self.run_dir();
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join(format!("spin-loss-{}.txt", self.sim.tick));
            if let Err(e) = std::fs::write(&path, text) {
                eprintln!("could not write {}: {e}", path.display());
            }
        }
        self.prev_spin = spin;
    }

    /// Plays the replay up to a tick without a window, then gives the keys to the player (`--from`). The replay's
    /// inputs stay in the run's replay, so the attempt saved later still replays from the start; it keeps the
    /// replay's format too, as the run goes on by that format's rules.
    fn fast_forward(&mut self, tick: u64) {
        let end = self.replay_in.as_ref().map_or(0, |r| r.end).min(tick);
        while self.sim.tick < end && !self.sim.finished && !self.sim.game_over {
            self.tick(0);
        }
        if let Some(r) = self.replay_in.take() {
            self.replay_out.version = r.version;
            self.sim.set_playback_pauses(None);
        }
    }

    /// The folder of a level's runs: the original levels' by name, a custom map's under maps/, so that a map named
    /// like an original level never counts as one.
    fn runs_dir(&self, stem: &str) -> PathBuf {
        self.runs_root.join(stem)
    }

    /// Saves a run that did not finish (restarted or closed before the end), so that the falls and the lifts of
    /// the level can be studied later. Named by the time it was saved; the folder is the level's.
    fn save_attempt(&self) {
        if self.sim.finished || self.sim.tick == 0 || self.replay_in.is_some() {
            return;
        }
        let stem = self
            .level
            .name
            .trim_end_matches(".NMO")
            .trim_end_matches(".nmo")
            .to_owned();
        let dir = self.runs_dir(&stem);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("attempt-{stamp}-{}.replay", self.sim.tick));
        let _ = std::fs::create_dir_all(&dir);
        match std::fs::write(&path, self.replay_out.to_text()) {
            Ok(()) => println!("attempt saved to {}", shown_path(&path)),
            Err(e) => eprintln!("could not save the attempt to {}: {e}", shown_path(&path)),
        }
    }

    /// One tick: the replay's input if there is one, else the keyboard's.
    fn tick(&mut self, keys: u8) {
        let next = self.sim.tick + 1;
        let pulse = std::mem::take(&mut self.pulse);
        let input = match &self.replay_in {
            Some(r) if next <= r.end => r.input_at(next),
            Some(_) => 0,
            None => keys | pulse,
        };
        if self.sim.finished {
            // After the finish the world goes on for the eye (the balloon leaves); the run is over and recorded.
            self.sim.step(0);
            self.effects.tick(&self.sim);
            return;
        }
        let events_before = self.sim.events.len();
        let turn = match &self.replay_in {
            Some(r) if next <= r.end => r.turn_at(next),
            Some(_) => 0,
            None => self.view_turn,
        };
        self.sim.turn = turn;
        // the scripts' keyboard and mouse: the replay's, else the window's; recorded with the run
        let frame = match &self.replay_in {
            Some(r) if next <= r.end => r.frame_at(next),
            Some(_) => script::InputFrame::default(),
            None => self.live_input.clamped(),
        };
        self.sim.set_input(frame);
        self.sim.step(input);
        self.effects.tick(&self.sim);
        if self.sim.drawn_ball() == Some(self.sim.ball.kind) && !self.sim.game_over {
            let world = self.sim.world();
            let speed = self.sim.ball.velocity.length();
            self.ball_sound.tick(
                &world.ball_sounds,
                self.sim.ball.kind,
                speed,
                world.time(),
                &mut self.sound_out,
            );
        } else {
            self.ball_sound.silence(&mut self.sound_out);
        }
        if self.sim.finished || self.sim.game_over {
            self.fan_sound.silence(&mut self.sound_out);
        } else {
            self.fan_sound.tick(
                &self.sim.fans(),
                self.sim.ball.position,
                &mut self.sound_out,
            );
        }
        if self.sound_out.len() > 4096 {
            // nobody plays them (a run without a window)
            self.sound_out.clear();
        }
        self.replay_out.record(self.sim.tick, input);
        self.replay_out.record_turn(self.sim.tick, turn);
        self.replay_out.record_frame(self.sim.tick, frame);
        if self.replay_out.pauses.len() != self.sim.pauses.len() {
            self.replay_out.record_pauses(&self.sim.pauses);
        }
        self.log_tick(input);
        self.dump_spin_loss();
        let ball = &self.sim.ball;
        // How deep the ball is in the floor: its radius less its distance to the nearest floor triangle.
        let sink = self
            .sim
            .world()
            .floor_near(ball.position, BALL_RADIUS + 1.0)
            .iter()
            .map(|t| ball.position.distance(closest_point(t, ball.position)))
            .fold(f32::INFINITY, f32::min);
        let sink = if sink.is_finite() {
            BALL_RADIUS - sink
        } else {
            0.0
        };
        let found = self.watch.observe(
            self.sim.tick,
            ball.position,
            ball.velocity,
            input,
            ball.on_ground,
            sink,
        );
        if !found.is_empty() {
            let contacts = self.sim.world().ball_contacts();
            for f in &found {
                self.report(f, &contacts);
            }
        }
        for (t, e) in self.sim.events.iter().skip(events_before) {
            if *e == Event::Died {
                self.ball_lost_at = Some(*t);
            }
            println!("{} {e:?}", clock(*t));
            if *e == Event::Finished {
                self.save_run();
            }
        }
    }

    /// Saves a finished run: to --record if given, always among the runs of this level, and as the level's best
    /// run when it is faster than the best so far (the replay of a perfect run for later rendering).
    fn save_run(&self) {
        // Replays recorded from a replay are not new runs.
        if self.replay_in.is_some() && self.record.is_none() {
            return;
        }
        let text = self.replay_out.to_text();
        let stem = self
            .level
            .name
            .trim_end_matches(".NMO")
            .trim_end_matches(".nmo")
            .to_owned();
        let dir = self.runs_dir(&stem);
        // a modded run is kept as such and opens no level (menu.rs `finished`)
        let tag = if self.sim.mods.modded() {
            "-modded"
        } else {
            ""
        };
        let mut targets = vec![dir.join(format!("{}-{}{tag}.replay", stem, self.sim.tick))];
        if let Some(r) = &self.record {
            targets.push(r.clone());
        }
        let best = dir.join("best.replay");
        let best_ticks = std::fs::read_to_string(&best)
            .ok()
            .and_then(|t| Replay::parse(&t).ok())
            // a stale run (recorded under the old script rules) is no longer this level's best
            .filter(|r| {
                crate::replay::stale_for_scripts(r.version, self.level.level.scripts.is_some())
                    .is_none()
            })
            .map(|r| r.end);
        // A run begun elsewhere than the level start is a test, never the level's best; nor is a modded run.
        if self.start.is_none()
            && !self.assisted
            && !self.sim.mods.modded()
            && best_ticks.is_none_or(|b| self.sim.tick < b)
        {
            targets.push(best.clone());
            let splits: Vec<String> = crate::modrt::split_ticks(&self.sim.events)
                .iter()
                .map(u64::to_string)
                .collect();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("splits.txt"), splits.join(" ") + "\n");
            println!(
                "best run of {} so far: {}",
                self.level.name,
                clock(self.sim.tick)
            );
        }
        for path in targets {
            if let Some(d) = path.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            match std::fs::write(&path, &text) {
                Ok(()) => println!("run saved to {}", shown_path(&path)),
                Err(e) => eprintln!("could not save the run to {}: {e}", shown_path(&path)),
            }
        }
    }

    /// The mods the window has switched on, for the runs from the next start (a replay keeps its own). A run not yet
    /// begun takes them at once.
    fn set_mods(&mut self, mods: crate::modrt::SimMods) {
        if self.replay_in.is_some() {
            return;
        }
        self.mods = mods;
        if self.sim.tick == 0 {
            self.sim.mods = self.mods.clone();
            self.replay_out.mods = self.mods.replay_lines();
        }
    }

    /// The run so far could become the best: begun at the level start, not modded, not a replay (a ghost and splits
    /// compare only such runs).
    fn counts_so_far(&self) -> bool {
        self.start.is_none() && !self.sim.mods.modded() && self.replay_in.is_none()
    }

    /// The best run's sector times (each sector reached, then the finish), as the best run left them.
    fn best_splits(&self) -> Vec<u64> {
        let path = self.runs_dir(&level_stem(self)).join("splits.txt");
        crate::hints::read_plain(&path, 4096)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .map(|t| {
                t.split_whitespace()
                    .take(256)
                    .filter_map(|w| w.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A fresh run of this level by `version`'s rules, as `restart` makes it (no mods: for the ghost).
    fn fresh_sim(&self, version: u32) -> Sim {
        let mut sim = Sim::with_rules(
            &self.level.level,
            self.start_ball,
            &self.modules,
            self.level.name.eq_ignore_ascii_case("Level_01.NMO"),
            version,
        );
        sim.set_ball_boxes(self.ball_meshes.boxes);
        sim.set_pieces(self.pieces.clone());
        sim.bursts = version >= 2;
        sim.pieces_expire = version >= 3;
        sim.checkpoints_by_name = version >= 4;
        sim.respawn_faces_on = version >= 6;
        sim.level_bonus = self.level_bonus;
        sim.set_sound_ticks(self.sound_ticks.clone());
        sim
    }

    /// The level's best run as a ghost: its replay (on this very level file, from the start, not modded) and a run
    /// to play it in.
    fn ghost(&self) -> Option<Ghost> {
        let path = self.runs_dir(&level_stem(self)).join("best.replay");
        let text =
            String::from_utf8(crate::hints::read_plain(&path, crate::hints::MAX_REPLAY_FILE).ok()?)
                .ok()?;
        let replay = Replay::parse(&text).ok()?;
        if replay.level_hash != self.level.hash || replay.start.is_some() || !replay.mods.is_empty()
        {
            return None;
        }
        if crate::replay::stale_for_scripts(replay.version, self.level.level.scripts.is_some())
            .is_some()
        {
            return None;
        }
        let sim = self.fresh_sim(replay.version);
        Some(Ghost {
            sim,
            replay,
            path: Vec::new(),
        })
    }

    /// The run counts (a best run, a highscore): begun at the level start, not helped, not modded, not a replay.
    fn counts(&self) -> bool {
        self.start.is_none()
            && !self.assisted
            && !self.sim.mods.modded()
            && self.replay_in.is_none()
    }

    fn title(&self) -> String {
        if self.placeholder {
            return "Drahtseilakt".into();
        }
        let s = &self.sim;
        let state = if s.finished {
            "  FINISHED"
        } else if s.game_over {
            "  GAME OVER (R restarts)"
        } else {
            ""
        };
        format!(
            "Drahtseilakt — {}   points {}   lives {}   sector {}   tick {}   {}{state}",
            self.shown_name,
            s.points,
            s.lives,
            s.sector + 1,
            s.tick,
            clock(s.tick)
        )
    }
}

fn clock(tick: u64) -> String {
    let secs = tick as f64 * f64::from(TICK);
    format!("{:02}:{:05.2}", (secs / 60.0) as u64, secs % 60.0)
}

/// The sound and music volumes from the settings file, full where it has none.
fn load_volumes() -> [f32; 2] {
    let text = settings_text();
    let get = |key: &str| {
        text.lines()
            .filter_map(|l| l.split_once(' '))
            .find(|(k, _)| *k == key)
            .and_then(|(_, v)| v.trim().parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .map_or(1.0, |v| v.clamp(0.0, 1.0))
    };
    [get("sound"), get("music")]
}

/// The level's start sound is played when the run is seen within its first second.
const START_SOUND_WITHIN: u64 = 60;

/// The run is kept once a second for the last half minute while a practice mod is on (god-mode rewinds from these).
const HISTORY_EVERY: u64 = 60;
const HISTORY_KEPT: usize = 31;

/// The mode saved in the settings: extended (our additions on) only when chosen, else classic (the original).
fn load_extended() -> bool {
    setting("profile").as_deref() == Some("extended")
}

/// The name last entered for a highscore, in the game's Latin-1.
fn load_player() -> Vec<u8> {
    setting("player")
        .map(|n| {
            n.chars()
                .filter_map(|c| u8::try_from(u32::from(c)).ok())
                .take(crate::highscore::NAME_MAX)
                .collect()
        })
        .unwrap_or_default()
}

/// The extended mode (from the settings, or `--debug` for one run) and the additions the settings turn off.
fn load_features(debug: bool) -> crate::features::Features {
    crate::features::Features::load(debug || load_extended(), debug, setting)
}

/// The Graphics page's choices kept in the settings.
fn load_graphics() -> crate::options::Graphics {
    crate::options::Graphics::load(setting)
}

/// The menus' words in the chosen language (lang.rs): the copy's for the original's interface, else ours.
fn load_strings(data: &Data) -> crate::lang::Strings {
    let copy = gamedata::resources::in_use() != gamedata::resources::Provider::Own;
    crate::lang::Strings::load(
        data,
        &crate::lang::chosen(setting("language").as_deref()),
        copy,
    )
}

/// The keys kept in the settings, else the copy's (its DB_Options), else the original's.
fn load_bindings(strings: &crate::lang::Strings) -> crate::options::Bindings {
    crate::options::Bindings::load(setting, strings.controls, strings.invert)
}

/// Sets `lines` (key and value) in the settings; every other line is kept as it is.
fn put_settings(lines: &[(String, String)]) {
    let mut text: String = lines.iter().map(|(k, v)| format!("{k} {v}\n")).collect();
    for line in settings_text().lines() {
        let key = line.split_once(' ').map_or(line, |(k, _)| k);
        if !lines.iter().any(|(k, _)| k == key) && !line.trim().is_empty() {
            text.push_str(line);
            text.push('\n');
        }
    }
    write_settings(&text);
}

/// Saves the settings; the language, which only the file sets, is kept.
fn save_settings(v: [f32; 2], features: crate::features::Features, player: &[u8]) {
    let profile = if features.mode { "extended" } else { "classic" };
    let mut text = format!(
        "sound {}\nmusic {}\nprofile {profile}\ninterface {}\n{}{}",
        v[0],
        v[1],
        gamedata::resources::chosen().key(),
        // the sound theme, in either mode, written only once the player made a choice (until then the default
        // plays, whichever it is)
        match gamedata::sound_themes::wanted() {
            Some(w) => format!("sound-theme {w}\n"),
            None => String::new(),
        },
        features.lines()
    );
    if !player.is_empty() {
        text.push_str(&format!("player {}\n", crate::lang::latin1(player)));
    }
    // the lines only the file sets (language, the level folders) are kept as they are
    let ours: &[&str] = &[
        "sound",
        "music",
        "profile",
        "player",
        "interface",
        "sound-theme",
    ];
    for line in settings_text().lines() {
        let key = line.split_once(' ').map_or(line, |(k, _)| k);
        if !ours.contains(&key) && !key.starts_with("extended.") && !line.trim().is_empty() {
            text.push_str(line);
            text.push('\n');
        }
    }
    write_settings(&text);
}

/// Writes the settings file, unless it is not ours to rewrite.
fn write_settings(text: &str) {
    let path = settings_path();
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    // a file that is not ours to rewrite (too big, or not UTF-8) is left as it is
    let foreign = std::fs::metadata(&path).is_ok_and(|m| m.len() > 64 * 1024)
        || std::fs::read(&path).is_ok_and(|b| std::str::from_utf8(&b).is_err());
    if foreign {
        eprintln!("{}: not saved over (too big or not UTF-8)", path.display());
        return;
    }
    // written beside it, then moved over it: a crash never leaves half a file
    let temp = path.with_extension("txt.new");
    if let Err(e) = std::fs::write(&temp, text).and_then(|()| std::fs::rename(&temp, &path)) {
        eprintln!("could not save the settings to {}: {e}", path.display());
    }
}

/// The menu `spec` asks for ("base:keys", see `screenshot`).
fn shot_menu(spec: &str, data: &Data, game: &Game, o: &Options, size: (u32, u32)) -> Option<Menu> {
    use crate::menu::Screen;
    let (base, keys) = spec.split_once(':').unwrap_or((spec, ""));
    let base = match base {
        "main" => Screen::Main,
        "pause" => Screen::Pause,
        "dead" => Screen::Dead,
        "end" => Screen::End,
        _ => return None,
    };
    let strings = load_strings(data);
    let number = match o.source {
        crate::assets::Source::Original(n) => Some(n),
        _ => None,
    };
    let mut m = Menu::new(data, &strings, base, number, load_features(o.debug));
    m.resolution = size;
    m.volumes = load_volumes();
    m.graphics = load_graphics();
    m.bindings = load_bindings(&strings);
    m.mods = crate::modrt::Mods::load(gamedata::mods::scan(o.debug), setting);
    m.resolutions = crate::options::WINDOWED.to_vec();
    for k in keys.chars() {
        match k {
            'u' => m.step(-1),
            'd' => m.step(1),
            'l' => drop(m.adjust(-1)),
            'r' => drop(m.adjust(1)),
            'e' => drop(m.enter()),
            'b' => drop(m.back()),
            // a key pressed while one is awaited on the Controls page (W)
            'w' => drop(m.capture(KeyCode::KeyW)),
            'n' => m.ask_name(load_player(), crate::endscreen::score(&game.sim)),
            _ => {}
        }
    }
    Some(m)
}

/// What the loading screen says is loading: a custom level's title, capped like the menu's (package.rs).
fn loading_name(source: &crate::assets::Source) -> String {
    match source {
        crate::assets::Source::Original(n) => format!("level {n}"),
        crate::assets::Source::Nmo(p) | crate::assets::Source::Open(p) => {
            let lang = crate::lang::chosen(setting("language").as_deref());
            gamedata::package::title_of(p, &custom_roots(), &lang).replace('…', "...")
        }
    }
}

/// What the loading screen shows of a level: its name, who made it, and the mood of its sky (level.txt's `sky`; an
/// original level's goes by its number).
fn loading_card(
    source: &crate::assets::Source,
) -> (String, Option<String>, crate::titlescene::Mood) {
    use crate::titlescene::Mood;
    let name = loading_name(source);
    match source {
        crate::assets::Source::Original(n) => (name, None, Mood::of(None, Some(*n))),
        crate::assets::Source::Nmo(p) | crate::assets::Source::Open(p) => {
            let info = gamedata::package::info_of(p);
            // an original level's rounded export (levels/remastered/Level_NN) names no sky: its number says it
            let number = p
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("Level_"))
                .and_then(|n| n.parse().ok());
            let mood = Mood::of(info.sky.as_deref(), number);
            (name, info.author.filter(|a| !a.trim().is_empty()), mood)
        }
    }
}

/// Original level `n` from its remastered export (the rounding cache), when the look is a rounded one (Remaster,
/// Drahtseilakt, Player's pack) and the export has the level; else the level as it is. The look is a setting, not a
/// kind of level.
fn look_source(n: u32) -> Option<crate::assets::Source> {
    if !crate::look::chosen().rounded() {
        return None;
    }
    let dir = crate::levels::dir(crate::levels::Kind::Remastered).join(format!("Level_{n:02}"));
    dir.join("level.gltf")
        .is_file()
        .then_some(crate::assets::Source::Open(dir))
}

/// The custom levels' folders (all but the original's), and the levels shipped with the game.
fn custom_roots() -> Vec<PathBuf> {
    crate::levels::KINDS
        .into_iter()
        .filter(|&k| k != crate::levels::Kind::Original)
        .map(crate::levels::dir)
        // the modified originals lie one folder deeper, in a folder per set (modified.rs)
        .chain(crate::modified::set_folders(&crate::levels::dir(
            crate::levels::Kind::Modified,
        )))
        .chain(gamedata::resources::shipped_drahtseilakt_root())
        .chain(gamedata::resources::shipped_community_root())
        .collect()
}

/// Where a level's runs are kept: the original levels' in replays/, a loose custom level's (a `.nmo` straight in a
/// kind's folder) and an open-format one's under replays/maps/<kind>/, a level folder's (levels.md, "A level is a
/// folder") under replays/levels/<kind>/<folder>/, apart from the loose ones (a loose Foo.nmo and a folder Foo/ never
/// share runs) and from each other (Level_07.NMO of two collections). With it, where a level folder's runs were kept
/// before (replays/maps/<folder>/), to move them from.
fn runs_root(source: &crate::assets::Source) -> (PathBuf, Option<PathBuf>) {
    let maps = replay_dir().join("maps");
    let name = |p: &Path| p.file_name().map(std::ffi::OsStr::to_owned);
    match source {
        crate::assets::Source::Original(_) => (replay_dir(), None),
        crate::assets::Source::Nmo(p) => {
            let folder = p.parent();
            let kind = folder
                .and_then(Path::parent)
                .filter(|k| custom_roots().iter().any(|r| r == k));
            match (kind.and_then(name), folder.and_then(name)) {
                (Some(kind), Some(folder)) => {
                    // the old place was named by the folder alone, and a loose map's by its kind's folder's name (that
                    // folder may be moved, ~/MyMaps): a level folder named like one of those shared it then
                    let kinds = crate::levels::KINDS.into_iter().map(crate::levels::dir);
                    let shared =
                        folder == "other" || kinds.filter_map(|d| name(&d)).any(|k| k == folder);
                    let old = (!shared).then(|| maps.join(&folder));
                    // runs kept under the kind's earlier folder name (levels/downloaded/..., before user-levels/)
                    let earlier: &[&str] = match kind.to_str() {
                        Some("user-levels") => &["downloaded", "community", "open"],
                        Some("ballance-original") => &["original"],
                        Some("ballance-remaster") => &["remastered"],
                        _ => &[],
                    };
                    let old = earlier
                        .iter()
                        .map(|e| replay_dir().join("levels").join(e).join(&folder))
                        .find(|p| p.is_dir())
                        .or(old);
                    (replay_dir().join("levels").join(kind).join(folder), old)
                }
                _ => (
                    maps.join(folder.and_then(name).unwrap_or_else(|| "other".into())),
                    None,
                ),
            }
        }
        crate::assets::Source::Open(p) => {
            let kind = p.parent().and_then(name).unwrap_or_else(|| "other".into());
            // runs kept under the kind's earlier folder name (maps/downloaded/..., maps/levels/... for the shipped set)
            let old = match kind.to_str() {
                Some("user-levels") => ["downloaded", "community", "open"].as_slice(),
                Some("drahtseilakt") => ["levels"].as_slice(),
                Some("ballance-remaster") => ["remastered"].as_slice(),
                _ => [].as_slice(),
            }
            .iter()
            .map(|e| maps.join(e))
            .find(|d| d.is_dir());
            (maps.join(kind), old)
        }
    }
}

/// Text and paths from level names, printed with control and format characters escaped.
fn shown(s: &str) -> String {
    gamedata::package::printable(s)
}

fn shown_path(p: &Path) -> String {
    gamedata::package::printable(&p.to_string_lossy())
}

/// Moves a level's runs from where an earlier version kept them (`old`/`stem`) to `new`/`stem`, once: only real
/// folders (no link anywhere on the way is followed), never over runs already in the new place.
fn move_runs(old: &Path, new: &Path, stem: &str) -> Result<bool, String> {
    let real_dir = |p: &Path| {
        std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
    };
    let (from, to) = (old.join(stem), new.join(stem));
    let maps = old.parent().map(Path::to_path_buf).unwrap_or_default();
    if stem.is_empty() || !real_dir(&maps) || !real_dir(old) || !real_dir(&from) {
        return Ok(false);
    }
    if std::fs::symlink_metadata(&to).is_ok() {
        return Ok(false);
    }
    let fail = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    std::fs::create_dir_all(new).map_err(|e| fail(new, e))?;
    if !real_dir(new) {
        return Err(format!("{}: not a folder of ours", new.display()));
    }
    std::fs::rename(&from, &to).map_err(|e| fail(&to, e))?;
    // the old folder goes when it is left empty
    let _ = std::fs::remove_dir(old);
    Ok(true)
}

/// The kept statistics of the time played (a mod's `playtime`).
fn playtime_file() -> PathBuf {
    gamedata::profile::data_dir().join("playtime.txt")
}

pub fn replay_dir() -> PathBuf {
    crate::levels::data_dir().join("replays")
}

/// The texture and model packs: found, and `textures` and `models` chosen when they are there, else the defaults (our
/// textures, the level's own parts).
fn start_packs(textures: Option<&str>, models: Option<&str>) {
    use gamedata::packs::{Kind, LEVEL, OWN, start};
    start(Kind::Textures, textures, OWN);
    start(Kind::Models, models, LEVEL);
}

/// When the game began running (for the start-up numbers).
static LAUNCHED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn since_launch() -> f64 {
    LAUNCHED
        .get()
        .map_or(0.0, |t| t.elapsed().as_secs_f64() * 1000.0)
}

/// A frame whose own work (the run's ticks, the picture, presenting; not the pause that holds the frame rate) takes
/// longer than this is logged with what happened in it.
const STALL: std::time::Duration = std::time::Duration::from_millis(100);

/// What the window loop notes for the frame in progress, for the stall log (`App::frame`).
#[derive(Default)]
struct Stall {
    /// What happened this frame that can take long: a level loaded, music begun, the surface set up again...
    notes: Vec<String>,
    /// The frame's stages: ticks, acquire, instances, draw, overlay, present.
    stages: [std::time::Duration; 6],
    /// The slowest tick of the frame, and its number.
    slowest: (std::time::Duration, u64),
    /// Time spent on the map's material sync, and sleeping to hold the frame rate (not work).
    sync: std::time::Duration,
    slept: std::time::Duration,
}

impl Stall {
    fn note(&mut self, what: &str, took: std::time::Duration) {
        if took >= std::time::Duration::from_millis(5) {
            self.notes
                .push(format!("{what} {:.0} ms", took.as_secs_f64() * 1000.0));
        }
    }
}

pub fn run(mut o: Options, data: Data) -> Result<(), String> {
    LAUNCHED.get_or_init(Instant::now);
    // where the interface's files come from: the copy's own when it has its menu, else ours (resources.rs); the
    // extended mode may choose another
    let explicit = gamedata::resources::from_settings();
    let default = gamedata::resources::default_for(data.copy_has("3D Entities/Menu.nmo"));
    gamedata::resources::set_copy(data.has_copy());
    let chosen = match explicit {
        Some(p) if !gamedata::resources::usable(p) => {
            eprintln!(
                "interface {}: not available here (it needs the player's copy or its remaster), using {}",
                p.key(),
                default.key()
            );
            default
        }
        other => other.unwrap_or(default),
    };
    gamedata::resources::set_chosen(chosen);
    gamedata::packs::set_copy(data.has_copy());
    let extended = load_features(o.debug).mode;
    gamedata::resources::set_in_use(if extended { chosen } else { default });
    // the levels' texture and model packs (packs.rs): the extended mode's choice, the command line's for one run; ours
    // and the level's own parts otherwise (the classic mode always). An export keeps the level's own (the copy's
    // textures) unless `--textures` asks for a pack in it.
    let (textures, models) = if o.export.is_some() {
        (Some(gamedata::packs::ORIGINAL.to_owned()), None)
    } else {
        (setting("textures"), setting("models"))
    };
    start_packs(
        o.textures.clone().or(textures).as_deref(),
        o.models.clone().or(models).as_deref(),
    );
    // the one Look picks the textures (a `--textures` pack of this run wins); the shapes follow it per level
    if o.textures.is_none() && o.export.is_none() {
        crate::look::start(setting("look").as_deref(), setting("textures").as_deref());
    }
    // an original level named on the command line is drawn in the look too (the window, screenshots, videos); a run
    // only checked without a picture stays on the copy's level, as replays were made on it
    if let crate::assets::Source::Original(n) = o.source
        && o.export.is_none()
        && (!o.headless || o.screenshot.is_some())
        && let Some(src) = look_source(n)
    {
        o.source = src;
    }
    for (asked, kind) in [
        (&o.textures, gamedata::packs::Kind::Textures),
        (&o.models, gamedata::packs::Kind::Models),
    ] {
        if let Some(k) = asked
            && gamedata::packs::chosen(kind) != *k
        {
            return Err(format!(
                "--{} {k}: no such pack (docs/drahtseilakt/packs.md)",
                kind.key()
            ));
        }
    }
    // the sound theme: the extended mode's choice (`sound-theme`), the default (generated) theme in the classic mode
    // (the interface no longer implies the original's sounds: the copy's interface is now the default)
    let theme = setting("sound-theme");
    gamedata::sound_themes::start(theme.as_deref());
    scene::set_show_colliders(o.show_colliders);
    if !o.audit.is_empty() {
        let prepare = |copy: bool| {
            gamedata::resources::set_copy(copy);
            gamedata::packs::set_copy(copy);
            // the packs a new player has (ours, the level's own parts), not this player's settings: the audit says
            // what the game gives, the same on every machine; --textures and --models ask for another
            start_packs(o.textures.as_deref(), o.models.as_deref());
        };
        return crate::audit::run(&o.audit, &data, o.audit_csv.as_deref(), &prepare);
    }
    if !o.audit_report.is_empty() {
        // as a new player has it, like --audit: the report says what the game gives, the same on every machine
        gamedata::resources::set_copy(data.has_copy());
        gamedata::packs::set_copy(data.has_copy());
        start_packs(o.textures.as_deref(), o.models.as_deref());
        let language = crate::lang::chosen(setting("language").as_deref());
        return crate::report::run(&o.audit_report, &data, &language);
    }
    if let Some(runs) = o.supports_check {
        // the supports come with our own models (the original's have none), whatever the player's settings choose:
        // the check measures them, unless --models asks for another pack
        start_packs(
            o.textures.as_deref(),
            Some(o.models.as_deref().unwrap_or(gamedata::packs::OWN)),
        );
        return crate::supports::run(&o, &data, runs.then_some(Path::new("hints")));
    }
    if let Some(dir) = &o.level_stats {
        return crate::levelstats::run(&o, &data, dir, Path::new("hints"));
    }
    // a level folder's own game files (modules, sky, sounds) come before the copy's
    let data = data.for_source(&o.source);
    if let Some(dir) = &o.export {
        let loaded = data.load(&o.source)?;
        // the own look ships no data of the copy: the level relies on the game's own parts and sounds
        let own = o.textures.as_deref() == Some(gamedata::packs::OWN);
        let with_game_files = o.export_parts.unwrap_or(!own);
        return crate::assets::export(&data, &loaded, dir, with_game_files);
    }
    // the video tracking draws the levels as it was tuned on: without the cloud layer
    if o.locate.is_some() || o.follow.is_some() || o.frames.is_some() {
        scene::set_clouds(false);
    }
    if let Some(dir) = &o.locate {
        let mut game = Game::new(&o, &data)?;
        return locate_run(&mut game, &o, dir);
    }
    if let Some(follow) = &o.follow {
        return follow_run(&o, &data, follow);
    }
    // the menu at the start has no level behind it: nothing of any level is read or uploaded before it shows
    let menu_start = !o.chosen
        && o.replay.is_none()
        && o.frames.is_none()
        && o.screenshot.is_none()
        && !o.autopilot
        && !o.headless
        && o.video.is_none()
        && o.from.is_none()
        && o.sector.is_none()
        && o.at.is_none()
        && o.ball.is_none();
    let mut game = if menu_start {
        Game::placeholder(&o, &data)?
    } else {
        Game::new(&o, &data)?
    };
    if let Some(dir) = &o.frames {
        return frames_run(&mut game, dir);
    }
    if let Some(path) = &o.screenshot {
        // the player's clouds and field of view, as the window would show them
        show_graphics(&load_graphics(), load_features(o.debug));
        return screenshot(&mut game, &data, &o, path);
    }
    if o.autopilot {
        // (--replay with --from: the run's keys up to that tick, then the test player from where they left it)
        let mut game = game;
        if let Some(tick) = o.from {
            game.fast_forward(tick);
        }
        return autopilot(game, &data, &o);
    }
    if o.headless {
        game.start_session();
        let end = game
            .replay_in
            .as_ref()
            .map(|r| r.end)
            .ok_or("--headless needs --replay")?;
        // when the end music (Music_EndCheckpoint) would begin to loop, and the finish music after it
        let mut end_music = None;
        // how near the floors' edges the ball rolls (edges.rs), for comparing runs
        let edges = crate::autopilot::level_edges(&game.level.level, &game.modules);
        let mut near_edges = crate::edges::EdgeStats::default();
        let mut script_lines = 0usize;
        while game.sim.tick < end && !game.sim.finished && !game.sim.game_over {
            game.tick(0);
            // what the map's scripts did this tick
            if let Some(s) = &game.sim.scripts {
                while let Some((t, line)) = s.log.get(script_lines) {
                    println!("{} script: {line}", clock(*t));
                    script_lines = script_lines.saturating_add(1);
                }
            }
            near_edges.tick(&edges, &game.sim.ball);
            if end_music.is_none() && crate::music::near_end(&game.sim) {
                end_music = Some(game.sim.tick);
            }
            if let Some(n) = o.trace.filter(|&n| n > 0 && game.sim.tick % n == 0) {
                let b = &game.sim.ball;
                let _ = n;
                println!(
                    "{} ball ({:8.2} {:8.2} {:8.2}) speed {:6.2} {}",
                    clock(game.sim.tick),
                    b.position.x,
                    b.position.y,
                    b.position.z,
                    b.velocity.length(),
                    if b.on_ground { "ground" } else { "air" }
                );
                // one body's contact points (its game index, as the loose trace's #), with DRAHTSEILAKT_TRACE_CONTACTS set
                if let Some(i) = std::env::var("DRAHTSEILAKT_TRACE_CONTACTS")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                {
                    for c in game.sim.world().body_contacts(i) {
                        println!(
                            "    contact {:?} types {:?} gap {:.4} normal ({:.3} {:.3} {:.3}) point ({:.2} {:.2} {:.2})",
                            c.entities,
                            c.types,
                            c.gap,
                            c.normal[0],
                            c.normal[1],
                            c.normal[2],
                            c.point[0],
                            c.point[1],
                            c.point[2]
                        );
                    }
                }
                // the loose things near the ball (boxes, balls, modules' parts), with DRAHTSEILAKT_TRACE_LOOSE set
                if std::env::var_os("DRAHTSEILAKT_TRACE_LOOSE").is_some() {
                    let trace_r: f32 = std::env::var("DRAHTSEILAKT_TRACE_R")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(40.0);
                    for &(o, body) in game.sim.loose() {
                        let Some(st) = game.sim.world().state(body) else {
                            continue;
                        };
                        if st.pos.distance(b.position) < trace_r {
                            let name = game
                                .level
                                .level
                                .objects
                                .get(o)
                                .map_or("?", |ob| ob.name.as_str());
                            println!(
                                "    {name} ({:8.2} {:8.2} {:8.2})",
                                st.pos.x, st.pos.y, st.pos.z
                            );
                        }
                    }
                    // and the modules' moving parts
                    for m in game.sim.module_bodies() {
                        let Some(st) = game.sim.world().state(m.body) else {
                            continue;
                        };
                        if st.pos.distance(b.position) < trace_r {
                            let (o, p) = (&m.object, st.pos);
                            let (r, w) = (st.rot, st.spin);
                            println!(
                                "    {o} #{} ({:8.2} {:8.2} {:8.2}) rot ({:.3} {:.3} {:.3} {:.3}) spin ({:.2} {:.2} {:.2})",
                                m.body, p.x, p.y, p.z, r.x, r.y, r.z, r.w, w.x, w.y, w.z
                            );
                        }
                    }
                }
            }
        }
        let s = &game.sim;
        println!(
            "result: {} after {} ticks ({}), points {}, lives {}, sector {}",
            if s.finished {
                "finished"
            } else if s.game_over {
                "game over"
            } else {
                "not finished"
            },
            s.tick,
            clock(s.tick),
            s.points,
            s.lives,
            s.sector + 1
        );
        println!("{}", near_edges.line());
        match end_music {
            Some(t) => println!(
                "end music: Music_EndCheckpoint loops from {} ({} before the end)",
                clock(t),
                clock(s.tick.saturating_sub(t))
            ),
            None => println!("end music: Music_EndCheckpoint never began"),
        }
        return Ok(());
    }
    if let Some(tick) = o.from {
        game.fast_forward(tick);
    }
    window(game, data, &o, None)
}

/// The game in its window (`pilot`: the test player plays it, with --watch).
fn window(
    mut game: Game,
    data: Data,
    o: &Options,
    pilot: Option<Box<PilotRun>>,
) -> Result<(), String> {
    game.start_session();
    let video = o.video.clone();
    // Without a level named, the window opens with the main menu over the first level.
    crate::levels::prepare(&crate::lang::docs_language(setting("language").as_deref()));
    let strings = load_strings(&data);
    let features = load_features(o.debug);
    let graphics = load_graphics();
    let bindings = load_bindings(&strings);
    show_graphics(&graphics, features);
    let mods = crate::modrt::Mods::load(gamedata::mods::scan(o.debug), setting);
    for m in &mods.found {
        if let Err(e) = &m.parsed {
            eprintln!("mod {}: refused: {e}", m.key);
        }
    }
    // the test player (--watch) plays the original's rules: no mod changes its run
    let (sim_mods, view_mods) = mods.running(features.mode && pilot.is_none());
    game.set_mods(sim_mods);
    let menu = (!o.chosen && o.replay.is_none()).then(|| {
        let mut m = Menu::new(
            &data,
            &strings,
            crate::menu::Screen::Main,
            Some(1),
            features,
        );
        m.volumes = load_volumes();
        m.graphics = graphics;
        m.bindings = bindings;
        m.mods = mods.clone();
        m
    });
    let mut app = App {
        number: match o.source {
            // no level is loaded at the menu's start: every pick loads one
            _ if game.placeholder => None,
            crate::assets::Source::Original(n) => Some(n),
            _ => None,
        },
        opts: o.clone(),
        menu,
        pause_told: false,
        loading: None,
        autoloaded: false,
        splash: None,
        art: Default::default(),
        startup: None,
        load_frames: Default::default(),
        load_busy: Default::default(),
        load_slow: 0,
        load_last_step: Default::default(),
        load_stages: Default::default(),
        features,
        strings,
        graphics,
        bindings,
        pads: crate::options::Pads::open(),
        pad_keys: 0,
        pad_held: false,
        cursor: None,
        mouse_down: 0,
        wheel: 0.0,
        cursor_shown: None,
        resolutions: crate::options::WINDOWED.to_vec(),
        player: load_player(),
        time_scale: 1.0,
        saved: None,
        history: std::collections::VecDeque::new(),
        fly: None,
        fly_keys: 0,
        last_view: (glam::Vec3::ZERO, glam::Vec3::X),
        audio: Audio::open().map(|mut a| {
            let v = load_volumes();
            a.set_volumes(v[0], v[1]);
            a
        }),
        heard: 0,
        started: false,
        rolling: [None; 3 + crate::ballsound::FAN_SLOTS],
        script_voices: crate::script_sound::Voices::default(),
        music: None,
        blitz: crate::lights::Blitz::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64),
        ),
        volumes: load_volumes(),
        game,
        data,
        // the settings' size, unless --size asks for one
        size: match graphics.resolution {
            Some(r) if !o.size_given => r,
            _ => o.size,
        },
        window: None,
        gpu: None,
        keys: 0,
        last: Instant::now(),
        acc: 0.0,
        timing: std::env::var_os("DRAHTSEILAKT_TIMING").map(|_| Timing::default()),
        prev: None,
        error: None,
        menu_scene: None,
        mods,
        view_mods,
        mod_began: Instant::now(),
        playtimes: crate::playtime::PlayTimes::load(&playtime_file()),
        playtime_saved: Instant::now(),
        eased: None,
        mouse: (0.0, 0.0),
        fps: 60.0,
        ghost: None,
        ghost_tried: false,
        stall: Stall::default(),
        frames: 0,
        shown_title: String::new(),
        best_splits: None,
        light_base: None,
        shot_due: false,
        last_shot: None,
        menu_ms: 0.0,
        held: std::collections::HashSet::new(),
        key_repeat: Default::default(),
        repeat_at: Instant::now(),
        pilot: pilot.map(|run| PilotWatch {
            run,
            paused: false,
            speed: 1.0,
            hand: None,
            pending: None,
            notice: None,
            done: false,
            keys_shown: true,
            trail: std::collections::VecDeque::new(),
            marking: None,
            look_back: LOOK_BACK,
            zones_shown: false,
            flash: None,
            marked: Vec::new(),
        }),
    };
    if let Some(path) = video {
        return app.video(&path);
    }
    eprintln!(
        "startup: event loop begins {:.0} ms after launch",
        since_launch()
    );
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    // a watched run closed before its end: summed up and saved as it stands
    if let Some(w) = app.pilot.take().filter(|w| !w.done) {
        w.run.summary(&app.game, o)?;
    }
    app.game.save_attempt();
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Plays the replay and draws `locate::RATE` frames of it a second (640 × 480 with the in-game interface, as a
/// video of the original would show it) into `dir` as s0001.jpg, ..., with `truth.txt`: each frame's number, the
/// ball's place and the view's quarter. A video with a known answer, to check `--locate` with.
fn frames_run(game: &mut Game, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let end = game
        .replay_in
        .as_ref()
        .map(|r| r.end)
        .ok_or("--frames needs --replay")?;
    let (_, device, queue) = pollster::block_on(device(None))?;
    let mut r = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
    upload(&mut r, game);
    let off = crate::locate::Offscreen::sized(&r, 640, 480);
    let mut truth = String::from("# frame x y z quarter\n");
    let mut k = 0;
    while game.sim.tick < end && !game.sim.game_over {
        game.tick(0);
        if !game
            .sim
            .tick
            .is_multiple_of(u64::from(60 / crate::locate::RATE))
        {
            continue;
        }
        k += 1;
        let snap = Snapshot::of(&game.sim);
        let mut inst = scene::instances(
            &game.placements,
            &game.sim.taken,
            &game.sim.unseen,
            &game.sim.world().bodies,
            &game.ball_meshes,
            game.trafo.as_ref().map(|t| &t.1),
            (&game.sim.extra_views(), game.extra_model.as_ref()),
            &snap,
            &game.sim.pieces_view(),
            game.sim.script_entities(),
        );
        if snap.kind.is_some() {
            scene::shadows(&game.shadow_receivers, snap.ball, &mut inst);
        }
        let screen = crate::ui::Screen { w: 640.0, h: 480.0 };
        let quads = crate::hud::interface(&game.sim, game.font.as_ref(), screen);
        if let Some(sc) = &game.sim.scripts {
            r.sync_materials(scene::LEVEL_MODEL, &sc.mats);
        }
        r.apply_script_sky(
            scene::LEVEL_MODEL,
            game.sim.script_env().sky.as_ref(),
            snap.eye,
            &mut inst,
        );
        let px = off
            .pixels(&mut r, &scene::view(&snap), &inst, &quads)
            .ok_or("the frame could not be read back")?;
        let img = image::RgbaImage::from_raw(640, 480, px).ok_or("frame size")?;
        let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
        rgb.save(dir.join(format!("s{k:04}.jpg")))
            .map_err(|e| e.to_string())?;
        let b = game.sim.ball.position;
        truth.push_str(&format!(
            "{k} {:.2} {:.2} {:.2} {}\n",
            b.x, b.y, b.z, game.sim.camera.quarter
        ));
    }
    std::fs::write(dir.join("truth.txt"), truth).map_err(|e| e.to_string())?;
    println!("{k} frames in {}", dir.display());
    Ok(())
}

/// Finds the ball's way in a video's frames (`locate::RATE` a second) and writes it as a track: per frame its
/// number, the ball's place, the view's quarter and how alike the best picture was.
fn locate_run(game: &mut Game, o: &Options, dir: &Path) -> Result<(), String> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('s') && n.ends_with(".jpg"))
        })
        .collect();
    // in the frames' order (s0001.jpg, ..., s10000.jpg: by number, not by name)
    let number =
        |p: &PathBuf| -> Option<u64> { p.file_stem()?.to_str()?.strip_prefix('s')?.parse().ok() };
    names.retain(|p| number(p).is_some());
    names.sort_by_key(|p| number(p));
    if let Some(n) = o.ticks {
        names.truncate(n as usize);
    }
    let frames: Vec<Vec<f32>> = names
        .iter()
        .map(|p| {
            crate::locate::frame_edges(p).ok_or_else(|| format!("{}: not a picture", p.display()))
        })
        .collect::<Result<_, _>>()?;
    println!("{} frames", frames.len());
    let start = game.sim.ball.position;
    let route = crate::autopilot::Route::find(
        &game.level.level,
        &game.modules,
        &game.sim.updrafts(),
        start,
        &[],
    );
    let floor: Vec<glam::Vec3> = route.floor_points().collect();
    let (_, device, queue) = pollster::block_on(device(None))?;
    let mut r = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
    upload(&mut r, game);
    let off = crate::locate::Offscreen::new(&r);
    let base = Snapshot::of(&game.sim);
    let mut draw = |view: &crate::render::View, ball: glam::Vec3| {
        let snap = Snapshot {
            ball,
            eye: view.eye,
            target: view.target,
            ..base
        };
        let inst = scene::instances(
            &game.placements,
            &game.sim.taken,
            &game.sim.unseen,
            &game.sim.world().bodies,
            &game.ball_meshes,
            None,
            (&[], None),
            &snap,
            &game.sim.pieces_view(),
            game.sim.script_entities(),
        );
        off.edges(&mut r, view, &inst)
    };
    let quarter = game.sim.camera.quarter;
    let fixes = crate::locate::track(&frames, &floor, start, quarter, &mut draw)?;
    let mut text = format!("{}\n", crate::locate::TRACK_HEAD);
    for (k, f) in fixes.iter().enumerate() {
        let (b, q, s) = (f.ball, f.quarter, f.score);
        text.push_str(&format!(
            "{} {:.2} {:.2} {:.2} {q} {s:.3}\n",
            k + 1,
            b.x,
            b.y,
            b.z
        ));
    }
    match &o.record {
        Some(p) => std::fs::write(p, &text).map_err(|e| format!("{}: {e}", p.display()))?,
        None => print!("{text}"),
    }
    Ok(())
}

/// The test player follows a track `--locate` found in a video: the places it was sure of (a good likeness, not
/// far from the frame before), in order, are its way, without the loops of it (the player went round, or waited by a
/// tutorial's text) unless a loop went to a loose object or a module's moving part (to push a box: the box moved is
/// part of the way). Where the way passes a transformer of another kind than the ball's, the ball is taken to have
/// gone into it (a gate of the way, which no loop is cut across).
fn follow_track(o: &Options, data: &Data, text: &str) -> Result<(), String> {
    let mut own = o.clone();
    own.follow = None;
    let game = Game::new(&own, data)?;
    let trafos: Vec<_> = game.sim.trafos().collect();
    // where the loose objects (boxes, balls) and the modules' moving parts start, and how near the ball comes to
    // one to move it
    const PUSH: f32 = 8.0;
    // (the level's objects where they start: the bodies of later sectors are not in the physics yet)
    // (a module's part is placed by the module, not a level object: where its body is made, or else the module's)
    let objects = &game.level.level.objects;
    let at = |o: usize| {
        objects
            .get(o)
            .map(|o| glam::Vec4::from(o.matrix[3]).truncate())
    };
    let parts = game.sim.module_bodies().iter().filter_map(|m| {
        game.sim
            .world()
            .state(m.body)
            .map(|s| s.pos)
            .or_else(|| at(m.placeholder))
    });
    let loose: Vec<glam::Vec3> = game
        .sim
        .loose()
        .iter()
        .filter_map(|l| at(l.0))
        .chain(parts)
        .collect();
    let mut kind = game.sim.ball.kind;
    let mut path: Vec<glam::Vec3> = Vec::new();
    let mut gates: Vec<crate::autopilot::Gate> = Vec::new();
    let mut last: Option<glam::Vec3> = None;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let v: Option<Vec<f32>> = line.split_whitespace().map(|x| x.parse().ok()).collect();
        let Some([_, x, y, z, _, likeness]) =
            v.as_deref().and_then(|v| <[f32; 6]>::try_from(v).ok())
        else {
            return Err(format!("not a track line: {line}"));
        };
        let ball = glam::Vec3::new(x, y, z);
        let floor = ball - glam::Vec3::Y * crate::sim::BALL_RADIUS;
        // (a likeness that is not a number is not sure either)
        if likeness.is_nan() || likeness < crate::locate::SURE || !floor.is_finite() {
            continue;
        }
        // a slip: far from the frame before (the next frames, if they go on from here, are taken again)
        let slip = last.is_some_and(|l: glam::Vec3| l.distance(floor) > crate::locate::JUMP);
        last = Some(floor);
        if slip || path.last().is_some_and(|p| p.distance(floor) < 1.5) {
            continue;
        }
        let kept = gates.last().map_or(0, |g| g.0);
        if let Some(i) = path
            .iter()
            .skip(kept)
            .rposition(|p| p.distance(floor) < 2.5)
        {
            // a loop to a loose object (a box to push, say) did something there: it stays
            let pushed = |p: &glam::Vec3| loose.iter().any(|o| o.distance(*p) < PUSH);
            if !path.get(kept + i..).unwrap_or_default().iter().any(pushed) {
                // (the last gate's own point stays)
                let keep = if gates.is_empty() { 0 } else { kept + 1 };
                path.truncate((kept + i).max(keep));
            }
        }
        path.push(floor);
        let trafo = trafos
            .iter()
            .find(|t| t.1 != kind && t.0.distance(ball) < crate::sim::TRAFO_REACH);
        if let Some(&(q, k)) = trafo {
            gates.push((path.len() - 1, q, k));
            kind = k;
        }
    }
    // the way between the track's places is found over the floors (a frame or a lost stretch may skip a corner,
    // and a straight line from one place to the next would go through the wall there): the places, one every
    // `STAGE`, and the gates' are its stages
    const STAGE: f32 = 6.0;
    let mut stages: Vec<glam::Vec3> = Vec::new();
    let mut gate_stages: Vec<(usize, glam::Vec3, crate::sim::BallKind)> = Vec::new();
    for (i, &p) in path.iter().enumerate() {
        let gate = gates.iter().find(|g| g.0 == i);
        // (by a loose object every place is a stage: how far the player went to push it matters)
        let near_loose = loose.iter().any(|o| o.distance(p) < PUSH);
        let apart = stages
            .last()
            .is_none_or(|l| l.distance(p) >= if near_loose { 1.5 } else { STAGE });
        if gate.is_some() || i + 1 == path.len() || apart {
            stages.push(p);
        }
        if let Some(&(_, q, kind)) = gate {
            gate_stages.push((stages.len() - 1, q, kind));
        }
    }
    // a stage the next one comes back from (the track jittered along a straight, or the player overshot and came
    // back) is no stage; a gate's is kept, and one by a loose object (the player went to push it)
    let mut k = 1;
    while k + 1 < stages.len() {
        let (a, b, c) = (stages[k - 1], stages[k], stages[k + 1]);
        let spike = a.distance(c) < 0.5 * a.distance(b).max(b.distance(c));
        let pushing = loose.iter().any(|o| o.distance(b) < PUSH);
        if spike && !pushing && gate_stages.iter().all(|g| g.0 != k) {
            stages.remove(k);
            for g in gate_stages.iter_mut().filter(|g| g.0 > k) {
                g.0 -= 1;
            }
            k = k.saturating_sub(1).max(1);
        } else {
            k += 1;
        }
    }
    let start = game.sim.ball.position;
    let fans = game.sim.updrafts();
    let route =
        crate::autopilot::Route::through(&game.level.level, &game.modules, &fans, start, &stages);
    let gates: Vec<crate::autopilot::Gate> = gate_stages
        .iter()
        .filter_map(|&(k, q, kind)| route.marks.get(k).map(|&m| (m, q, kind)))
        .collect();
    let (places, ways) = (path.len(), route.points.len());
    println!(
        "track: {places} places, {} stages, way {ways} points, transformers {gates:?}",
        stages.len()
    );
    if std::env::var_os("AUTOPILOT_FLOOR").is_some() {
        for p in route.floor_points() {
            eprintln!("floor {:.1} {:.1} {:.1}", p.x, p.y, p.z);
        }
    }
    if std::env::var_os("AUTOPILOT_DEBUG").is_some() {
        for (i, p) in route.points.iter().enumerate() {
            eprintln!("way {i} {:.1} {:.1} {:.1}", p.x, p.y, p.z);
        }
    }
    pilot_run(game, data, &own, crate::autopilot::Pilot::new(route, gates))
}

/// A run that did not finish is followed this many way points (and ticks of the player's) past the player's last
/// transformer in its last sector.
const LAST_GATE_ON: usize = 3;
const LAST_GATE_TICKS: usize = 60;

/// The followed player's keys this long before a lost ball are not followed (four seconds).
const DOOMED_TICKS: usize = 240;

/// A watched player's run: the replay, the ball's way (and the sector of each of its points), the transformers it
/// went into (gates) and the player's ticks.
struct Watched {
    run: Replay,
    path: Vec<glam::Vec3>,
    sectors: Vec<usize>,
    gates: Vec<crate::autopilot::Gate>,
    demo: crate::autopilot::Demo,
    /// Whether the run finished the level, and where on its way it lost each ball.
    finished: bool,
    deaths: Vec<usize>,
}

/// The test player follows a player's run: the ball's way in that replay (where it rolled, in order, every 1.5;
/// not while a ball is being lost) is its way, the transformers it went into its gates, and it begins where the run
/// began. Each --then run (one begun at a sector) is followed from its sector on instead.
fn follow_run(o: &Options, data: &Data, follow: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(follow).map_err(|e| format!("{}: {e}", follow.display()))?;
    if text.starts_with(crate::locate::TRACK_HEAD) {
        return follow_track(o, data, &text);
    }
    let mut w = watch_run(o, data, follow, &text)?;
    for more in &o.then {
        let text = std::fs::read_to_string(more).map_err(|e| format!("{}: {e}", more.display()))?;
        let m = watch_run(o, data, more, &text)?;
        let Some(from) = m.run.start.as_ref().map(|st| st.sector.saturating_sub(1)) else {
            return Err(format!(
                "{}: --then needs a run begun at a sector",
                more.display()
            ));
        };
        // the followed way up to that sector, then the other run's
        let cut = w
            .sectors
            .iter()
            .position(|&s| s >= from)
            .unwrap_or(w.path.len());
        w.path.truncate(cut);
        w.sectors.truncate(cut);
        w.gates.retain(|g| g.0 < cut);
        w.gates
            .extend(m.gates.iter().map(|g| (g.0 + cut, g.1, g.2)));
        w.path.extend(m.path);
        w.sectors.extend(m.sectors);
        let ticks = w
            .demo
            .ticks
            .iter()
            .position(|t| t.sector >= from)
            .unwrap_or(w.demo.ticks.len());
        w.demo.ticks.truncate(ticks);
        w.demo.ticks.extend(m.demo.ticks);
        w.deaths = m.deaths.iter().map(|d| d + cut).collect();
        w.finished = m.finished;
    }
    println!(
        "followed: {} points, transformers {:?}",
        w.path.len(),
        w.gates
    );
    // the test player's own run begins where the followed one began
    let mut own = o.clone();
    own.replay = None;
    own.follow = None;
    if let Some(st) = &w.run.start {
        own.sector = Some(st.sector);
        own.ball = st.ball.clone();
        own.at = st.at.map(|[x, y, z]| format!("{x},{y},{z}"));
    }
    let game = Game::new(&own, data)?;
    if !w.finished {
        way_on(&game, &own, &mut w)?;
    }
    // (a way partly found by the test player takes the steeper floors that way was found over)
    let route = crate::autopilot::Route::along(
        &game.level.level,
        &game.modules,
        &game.sim.updrafts(),
        w.path,
        !w.finished,
    );
    let mut pilot = crate::autopilot::Pilot::new(route, w.gates);
    pilot.demo = Some(std::sync::Arc::new(w.demo));
    pilot_run(game, data, &own, pilot)
}

/// A followed run that did not finish: its way up to its last sector and its try there that got furthest (lost its
/// ball furthest from where it came into that sector), then the way the test player finds by itself from there to
/// the end.
fn way_on(game: &Game, o: &Options, w: &mut Watched) -> Result<(), String> {
    let Some(&last) = w.sectors.last() else {
        return Ok(());
    };
    let first = w.sectors.iter().position(|&s| s == last).unwrap_or(0);
    let entry = w.path.get(first).copied().unwrap_or_default();
    // its tries in that sector, each from where it came in (or a new ball) to where it lost the ball
    let mut ends: Vec<usize> = w
        .deaths
        .iter()
        .copied()
        .filter(|&d| d > first && d <= w.path.len())
        .collect();
    ends.push(w.path.len());
    let tries: Vec<(usize, usize)> = std::iter::once(first)
        .chain(ends.iter().copied())
        .zip(ends.iter().copied())
        .filter(|(a, b)| b > a)
        .collect();
    let far = |t: &(usize, usize)| w.path.get(t.1 - 1).map_or(0.0, |p| p.distance(entry));
    if let Some(&(a, b)) = tries.iter().max_by(|x, y| far(x).total_cmp(&far(y))) {
        let mut path: Vec<glam::Vec3> = w.path.get(..first).unwrap_or_default().to_vec();
        path.extend(w.path.get(a..b).unwrap_or_default());
        let mut gates: Vec<crate::autopilot::Gate> =
            w.gates.iter().copied().filter(|g| g.0 < first).collect();
        gates.extend(
            w.gates
                .iter()
                .filter(|g| g.0 >= a && g.0 < b)
                .map(|g| (g.0 - a + first, g.1, g.2)),
        );
        w.sectors.truncate(first);
        w.sectors.extend(std::iter::repeat_n(last, b - a));
        w.path = path;
        w.gates = gates;
    }
    // After the player's last transformer in that sector the player was looking for the way (and may have rolled
    // where the ball is stuck: a stone ball into a fan's pit): the way on is found from just past it, and the
    // player's line ends there.
    if let Some(&(g, at, kind)) = w.gates.iter().rev().find(|g| g.0 >= first) {
        let cut = (g + LAST_GATE_ON).min(w.path.len());
        w.path.truncate(cut);
        w.sectors.truncate(cut);
        let made = w
            .demo
            .ticks
            .iter()
            .rposition(|t| t.sector == last && t.kind == kind && t.pos.distance(at) < 6.0);
        if let Some(k) = made {
            for t in w.demo.ticks.iter_mut().skip(k + LAST_GATE_TICKS) {
                t.alive = false;
                t.doomed_in = None;
            }
        }
    }
    let Some(&from) = w.path.last() else {
        return Ok(());
    };
    let kind = w.gates.last().map_or(game.sim.ball.kind, |g| g.2);
    let up = from + glam::Vec3::Y * crate::sim::BALL_RADIUS;
    let took: Vec<glam::Vec3> = w.gates.iter().map(|g| g.1).collect();
    let (route, gates) = found_way(game, o, up, last, &w.path, &took, kind)?;
    let n = w.path.len();
    if std::env::var_os("AUTOPILOT_DEBUG").is_some() {
        for (i, p) in route.points.iter().enumerate() {
            eprintln!("way on {} {:.1} {:.1} {:.1}", i + n, p.x, p.y, p.z);
        }
    }
    println!(
        "the followed run ends unfinished at {from:?} (sector {}): the way on is found, {} points, transformers \
        {gates:?}",
        last + 1,
        route.points.len()
    );
    w.gates.extend(gates.iter().map(|g| (g.0 + n, g.1, g.2)));
    w.sectors
        .extend(std::iter::repeat_n(last, route.points.len()));
    w.path.extend(route.points);
    Ok(())
}

/// Watches a player's run (`text`, read from `file`) to its end.
fn watch_run(o: &Options, data: &Data, file: &Path, text: &str) -> Result<Watched, String> {
    let run = Replay::parse(text)?;
    let mut watch = o.clone();
    watch.replay = Some(file.to_path_buf());
    watch.follow = None;
    let mut seen = Game::new(&watch, data)?;
    let mut path: Vec<glam::Vec3> = Vec::new();
    let mut sectors: Vec<usize> = Vec::new();
    // the tick of each place of the way
    let mut at_tick: Vec<u64> = Vec::new();
    let mut gates: Vec<crate::autopilot::Gate> = Vec::new();
    let mut demo = crate::autopilot::Demo::default();
    let mut lost = false;
    let mut deaths = Vec::new();
    while seen.sim.tick < run.end && !seen.sim.finished && !seen.sim.game_over {
        // the player's tick: the ball as it is, and the push the player gives now, in the world
        let s = &seen.sim;
        demo.ticks.push(crate::autopilot::DemoTick {
            pos: s.ball.position,
            vel: s.ball.velocity,
            kind: s.ball.kind,
            sector: s.sector,
            alive: !s.is_dying() && !lost,
            push: crate::hints::world_push(run.input_at(s.tick + 1), s.camera.quarter),
            bits: run.input_at(s.tick + 1),
            quarter: s.camera.quarter,
            doomed_in: None,
        });
        let before = seen.sim.events.len();
        seen.tick(0);
        for (_, e) in seen.sim.events.iter().skip(before) {
            match e {
                crate::sim::Event::Died => {
                    lost = true;
                    // nor its way in them: the ball was rolling off (a try that did not finish is followed to
                    // where it lost the ball, and the way on found from there)
                    let doomed = seen.sim.tick.saturating_sub(DOOMED_TICKS as u64);
                    while at_tick.last().is_some_and(|&t| t >= doomed) {
                        at_tick.pop();
                        path.pop();
                        sectors.pop();
                    }
                    gates.retain(|g| g.0 <= path.len());
                    deaths.push(path.len());
                    // the player's last seconds before losing a ball are no line to follow
                    let n = demo.ticks.len();
                    let from = n.saturating_sub(DOOMED_TICKS);
                    for (i, t) in demo.ticks.iter_mut().enumerate().skip(from) {
                        if t.alive {
                            t.doomed_in = u32::try_from(n - i).ok();
                        }
                        t.alive = false;
                    }
                }
                crate::sim::Event::NewBall => lost = false,
                crate::sim::Event::Transform(k) => {
                    let at = seen.sim.ball.position - glam::Vec3::Y * crate::sim::BALL_RADIUS;
                    gates.push((path.len(), at, *k))
                }
                _ => {}
            }
        }
        let b = &seen.sim.ball;
        // AUTOPILOT_TRACE_FOLLOW: the followed ball's place every half second, to find a stretch for a hint
        if std::env::var_os("AUTOPILOT_TRACE_FOLLOW").is_some() && seen.sim.tick.is_multiple_of(30)
        {
            let p = b.position;
            eprintln!(
                "follow t {} s {} ({:.1} {:.1} {:.1}) q {}",
                seen.sim.tick,
                seen.sim.sector + 1,
                p.x,
                p.y,
                p.z,
                seen.sim.camera.quarter
            );
        }
        let floor = b.position - glam::Vec3::Y * crate::sim::BALL_RADIUS;
        if !lost && b.on_ground && path.last().is_none_or(|p| p.distance(floor) >= 1.5) {
            path.push(floor);
            sectors.push(seen.sim.sector);
            at_tick.push(seen.sim.tick);
        }
    }
    let finished = seen.sim.finished;
    // the player's last seconds before a loss are kept to only where the run got on past that sector afterwards (a
    // loss that served the level), not in the sector a run that did not finish ended in
    let last = seen.sim.sector;
    for t in demo
        .ticks
        .iter_mut()
        .filter(|t| !finished && t.sector >= last)
    {
        t.doomed_in = None;
    }
    let (path, sectors, gates, deaths) = failed_tries_out(path, sectors, gates, deaths, finished);
    Ok(Watched {
        run,
        path,
        sectors,
        gates,
        demo,
        finished,
        deaths,
    })
}

/// A player's way without the tries that lost the ball: each one's stretch from where it came into the sector it was
/// lost in (the ball starts that sector again at its reset point) to where it was lost. The tries in the last sector
/// of a run that did not finish are kept: which got furthest is chosen later (way_on).
fn failed_tries_out(
    path: Vec<glam::Vec3>,
    sectors: Vec<usize>,
    gates: Vec<crate::autopilot::Gate>,
    deaths: Vec<usize>,
    finished: bool,
) -> (
    Vec<glam::Vec3>,
    Vec<usize>,
    Vec<crate::autopilot::Gate>,
    Vec<usize>,
) {
    let last = sectors.last().copied().unwrap_or(0);
    let mut keep = vec![true; path.len()];
    let mut try_start = 0;
    for &d in &deaths {
        let d = d.min(path.len());
        let s = d
            .checked_sub(1)
            .and_then(|i| sectors.get(i))
            .copied()
            .unwrap_or(0);
        if finished || s != last {
            let from = (try_start..d)
                .find(|&i| sectors.get(i) == Some(&s))
                .unwrap_or(d);
            keep.iter_mut().take(d).skip(from).for_each(|k| *k = false);
        }
        try_start = d;
    }
    // the new places of the points kept
    let mut at = Vec::with_capacity(path.len() + 1);
    let mut n = 0;
    for &k in &keep {
        at.push(n);
        n += usize::from(k);
    }
    at.push(n);
    let new = |i: usize| at.get(i).copied().unwrap_or(n);
    let gates = gates
        .into_iter()
        .filter(|g| keep.get(g.0).copied().unwrap_or(true))
        .map(|g| (new(g.0), g.1, g.2));
    let in_last = |d: usize| sectors.get(d.saturating_sub(1)) == Some(&last);
    let deaths = deaths
        .iter()
        .filter(|&&d| d >= keep.len() || !finished && in_last(d));
    let deaths: Vec<usize> = deaths.map(|&d| new(d)).collect();
    let path = path
        .into_iter()
        .zip(&keep)
        .filter(|p| *p.1)
        .map(|p| p.0)
        .collect();
    let sectors = sectors
        .into_iter()
        .zip(&keep)
        .filter(|p| *p.1)
        .map(|p| p.0)
        .collect();
    (path, sectors, gates.collect(), deaths)
}

/// The test player plays the level from where the run begins: it saves the run at --record (or as an attempt) and
/// prints how far it got.
fn autopilot(game: Game, data: &Data, o: &Options) -> Result<(), String> {
    let start = game.sim.ball.position;
    let (route, gates) = found_way(
        &game,
        o,
        start,
        game.sim.sector,
        &[],
        &[],
        game.sim.ball.kind,
    )?;
    println!(
        "route: {} points, stages at {:?}",
        route.points.len(),
        route.marks
    );
    if std::env::var_os("AUTOPILOT_FLOOR").is_some() {
        for p in route.floor_points() {
            eprintln!("floor {:.1} {:.1} {:.1}", p.x, p.y, p.z);
        }
    }
    if std::env::var_os("AUTOPILOT_DEBUG").is_some() {
        for (i, p) in route.points.iter().enumerate() {
            println!("route {i} {:.2} {:.2} {:.2}", p.x, p.y, p.z);
        }
    }
    println!("transformers on the way: {:?}", gates);
    pilot_run(game, data, o, crate::autopilot::Pilot::new(route, gates))
}

/// The way the test player finds by itself from `start` in `sector` (from 0) through the rest of the level's
/// checkpoints to its end, and the transformers on it (its gates). A plan is followed from the start of the sector,
/// but for the objects of it a player's way `passed` already came by (within 20: those and all before them; a
/// transformer only when the player's ball went into it, at one of `took`).
fn found_way(
    game: &Game,
    o: &Options,
    start: glam::Vec3,
    sector: usize,
    passed: &[glam::Vec3],
    took: &[glam::Vec3],
    kind: crate::sim::BallKind,
) -> Result<(crate::autopilot::Route, Vec<crate::autopilot::Gate>), String> {
    let level = &game.level.level;
    let stages: Vec<glam::Vec3> = game.sim.stages().into_iter().skip(sector).collect();
    // The transformers the way passes (within 30) are part of the way, in its order: a player rolls into them
    // rather than around them.
    let base =
        crate::autopilot::Route::find(level, &game.modules, &game.sim.updrafts(), start, &stages);
    let mut waypoints: Vec<(usize, glam::Vec3, bool)> = Vec::new();
    for t in game.sim.transformers() {
        let near = base
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    i,
                    glam::Vec3::new(p.x - t.x, 0.0, p.z - t.z).length() + (p.y - t.y).abs(),
                )
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = near.filter(|n| n.1 < 30.0) {
            waypoints.push((i, t, true));
        }
    }
    for (k, &m) in base.marks.iter().enumerate() {
        if let Some(&s) = stages.get(k) {
            waypoints.push((m, s, false));
        }
    }
    waypoints.sort_by_key(|w| w.0);
    // A plan (--plan) names the objects to pass through, in order, as a player who knows the level does: it
    // replaces the found order (from the checkpoint the sector begins at on).
    if let Some(plan) = &o.plan {
        let mut planned = read_plan(plan, level, sector)?;
        // (up to the sector's checkpoint: what comes after it is not passed yet)
        let end = stages
            .first()
            .and_then(|c| planned.iter().position(|p| p.0.distance(*c) < 1.0));
        let taken = |p: &(glam::Vec3, bool)| took.iter().any(|q| q.distance(p.0) < 8.0);
        let came = planned
            .iter()
            .take(end.unwrap_or(planned.len()))
            // (a transformer is passed when the player's ball went into it, not by)
            .rposition(|p| {
                if p.1 {
                    taken(p)
                } else {
                    passed.iter().any(|q| q.distance(p.0) < 20.0)
                }
            });
        if let Some(k) = came {
            // but a transformer after the last one the player took, not taken, is still to go into (the player's
            // ball went by it, or did not get to it before it was lost)
            let last_taken = planned.iter().take(k + 1).rposition(|p| p.1 && taken(p));
            let mut i = 0;
            planned.retain(|p| {
                i += 1;
                i > k + 1 || (p.1 && !taken(p) && last_taken.is_none_or(|t| i > t + 1))
            });
        }
        waypoints = planned
            .into_iter()
            .map(|(p, trafo)| (0, p, trafo))
            .collect();
    }
    let fans = game.sim.updrafts();
    // the transformers' places on the way: their gates
    // (each with the ball its transformer makes: the level's transformer at that place)
    let kind_at = |p: glam::Vec3| {
        game.sim
            .trafos()
            .min_by(|a, b| a.0.distance(p).total_cmp(&b.0.distance(p)))
            .map(|t| t.1)
    };
    let mut tries = 0;
    // the stretches of the way (by goal) found without riding a fan
    let mut grounded: Vec<usize> = Vec::new();
    loop {
        let stages: Vec<glam::Vec3> = waypoints.iter().map(|w| w.1).collect();
        // the transformers not on the way are kept off (rolling over one makes the ball over)
        let shun: Vec<glam::Vec3> = game
            .sim
            .transformers()
            .into_iter()
            .filter(|t| stages.iter().all(|s| s.distance(*t) > 1.0))
            .collect();
        let route = crate::autopilot::Route::find_shunning(
            level,
            &game.modules,
            &fans,
            start,
            &stages,
            &shun,
            &grounded,
        );
        let gates: Vec<crate::autopilot::Gate> = route
            .marks
            .iter()
            .zip(&waypoints)
            .filter(|(_, w)| w.2)
            .filter_map(|(&m, w)| Some((m, w.1, kind_at(w.1)?)))
            .collect();
        // Only a paper ball rides an updraft: a lift on the way taken as another ball is preceded by the paper
        // transformer nearest the fan, and the way is found again (a few times at most).
        let ball_at = |i: usize| {
            gates
                .iter()
                .take_while(|g| g.0 <= i)
                .last()
                .map_or(kind, |g| g.2)
        };
        let wrong = route
            .lifts
            .iter()
            .find(|&&(i, _)| ball_at(i) != crate::sim::BallKind::Paper);
        let paper = wrong.and_then(|&(i, f)| {
            let base = fans.get(f)?.0;
            let t = game
                .sim
                .trafos()
                .filter(|t| t.1 == crate::sim::BallKind::Paper)
                .min_by(|a, b| a.0.distance(base).total_cmp(&b.0.distance(base)))?;
            (t.0.distance(base) < FAN_PAPER_NEAR).then_some((i, t.0))
        });
        tries += 1;
        let Some((i, at)) = paper.filter(|_| tries <= FAN_PAPER_TRIES) else {
            return Ok((route, gates));
        };
        let before = route
            .marks
            .iter()
            .position(|&m| m >= i)
            .unwrap_or(waypoints.len());
        if waypoints.iter().any(|w| w.1.distance(at) < 1.0) {
            // the paper transformer is on the way already, after this lift: the stretch the lift is on is found
            // again over the floors only (its goal may be that very transformer, up where the lift would lead)
            if grounded.contains(&before) {
                return Ok((route, gates));
            }
            println!(
                "the way rides a fan at {i} not as paper: that stretch is found again without lifts"
            );
            grounded.push(before);
            continue;
        }
        println!(
            "the way rides a fan at {i} not as paper: into the paper transformer at {at:?} first"
        );
        waypoints.insert(before, (0, at, true));
    }
}

/// How often the way is found again to take a paper transformer before a fan, and how far from the fan one may be.
const FAN_PAPER_TRIES: usize = 4;
const FAN_PAPER_NEAR: f32 = 150.0;

/// A plan: one level object a line (`NAME  # note`, or a place `X Y Z`), in the order the way goes through them; blank lines and
/// `#` comments are skipped. Returns where each is and whether it is a transformer, from the checkpoint sector
/// `sector` (from 0) begins at on (PC_TwoFlames_0N; all of it for the first).
fn read_plan(
    path: &Path,
    level: &level::Level,
    sector: usize,
) -> Result<Vec<(glam::Vec3, bool)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    let begins = format!("PC_TwoFlames_{sector:02}");
    for line in text.lines() {
        let name = line
            .split('#')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_end_matches('?')
            .trim();
        if name.is_empty() {
            continue;
        }
        // a place on the way written as `X Y Z` (a rail to take rather than another, a point the way must come by)
        let xyz: Vec<f32> = name
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .filter(|v: &f32| v.is_finite())
            .collect();
        if xyz.len() == 3 && name.split_whitespace().count() == 3 {
            out.push((glam::Vec3::new(xyz[0], xyz[1], xyz[2]), false));
            continue;
        }
        let i = level
            .objects
            .iter()
            .position(|ob| ob.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("{}: no object {name}", path.display()))?;
        if sector > 0 && name.eq_ignore_ascii_case(&begins) {
            out.clear();
            continue;
        }
        let trafo = level.groups_of(i).any(|g| g.starts_with("P_Trafo"));
        out.push((glam::Vec3::from(level.objects[i].position()), trafo));
    }
    Ok(out)
}

/// The level file's name without its extension (Level_01 for Level_01.NMO).
/// The lengths in ticks of the sounds a map's scripts name, from the game's sound set (no audio device needed: scripts
/// wait on them). A name the set does not have is left out, and so is silent.
fn script_sound_ticks(sim: &Sim, data: &Data) -> std::collections::HashMap<String, u64> {
    let mut out = std::collections::HashMap::new();
    let Some(scripts) = &sim.scripts else {
        return out;
    };
    for stem in scripts.sound_files() {
        if let Some(sec) = crate::audio::sound_seconds(data, &stem) {
            let ticks = (sec / f64::from(crate::sim::TICK)).ceil();
            out.insert(stem.to_ascii_lowercase(), ticks as u64);
        }
    }
    out
}

fn level_stem(game: &Game) -> String {
    Path::new(&game.level.name)
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
}

/// The level's hint file: --hints, else hints/<level>.txt in the data folder.
fn hints_path(game: &Game, o: &Options) -> PathBuf {
    o.hints.clone().unwrap_or_else(|| {
        crate::levels::data_dir()
            .join("hints")
            .join(format!("{}.txt", level_stem(game)))
    })
}

/// The level's zones file (zones.rs): `<level>.zones` beside its hint file.
fn zones_path(game: &Game, o: &Options) -> PathBuf {
    let hints = hints_path(game, o);
    let dir = hints.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    dir.join(format!("{}.zones", level_stem(game)))
}

/// The learned table's file name, beside the hints: one for every level.
const LEARNED: &str = "learned.txt";

/// The learned table, the lines left out said.
fn load_learned(path: &Path) -> crate::learned::Learned {
    let (l, notes) = crate::learned::load(path);
    for n in notes {
        eprintln!("learned: {n}");
    }
    if !l.is_empty() {
        println!("{} learned rules from {}", l.rules.len(), path.display());
    }
    l
}

/// The zones of the level, the lines left out said.
fn load_zones(path: &Path) -> crate::zones::Zones {
    let (zones, notes) = crate::zones::load(path);
    for n in notes {
        eprintln!("zones: {n}");
    }
    if !zones.is_empty() {
        println!("{} zones from {}", zones.zones.len(), path.display());
    }
    zones
}

/// The hints of the level, a replay stretch read into the view's quarter at its start and its inputs (the replay is
/// played to its first tick to know the quarter).
fn load_hints(game: &Game, data: &Data, o: &Options) -> Result<Vec<crate::hints::Hint>, String> {
    let path = hints_path(game, o);
    let mut hints = crate::hints::load(&path)?;
    let dir = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    for h in &mut hints {
        for step in &mut h.steps {
            if let crate::hints::Step::Trace(file, from, to, gain) = step {
                let track = load_track(&dir.join(&*file), *from, *to, *gain, data, o)?;
                *step = crate::hints::Step::Track(Box::new(track));
                continue;
            }
            let crate::hints::Step::Replay(file, from, to) = step else {
                continue;
            };
            let file = dir.join(&*file);
            let bytes = crate::hints::read_plain(&file, crate::hints::MAX_REPLAY_FILE)?;
            let text =
                String::from_utf8(bytes).map_err(|_| format!("{}: not UTF-8", file.display()))?;
            let run = Replay::parse(&text).map_err(|e| format!("{}: {e}", file.display()))?;
            let mut watch = o.clone();
            watch.replay = Some(file.clone());
            watch.follow = None;
            watch.autopilot = false;
            let mut seen = Game::new(&watch, data)?;
            while seen.sim.tick < *from && !seen.sim.game_over {
                seen.tick(0);
            }
            // the input of each tick of the stretch, in one pass over the replay's changes
            let mut inputs = Vec::new();
            let mut changes = run.changes.iter().peekable();
            let mut input = 0u8;
            for t in 0..=*to {
                while let Some(&&(_, i)) = changes.peek().filter(|c| c.0 <= t) {
                    input = i;
                    changes.next();
                }
                if t > *from {
                    inputs.push(input);
                }
            }
            *step = crate::hints::Step::Inputs(seen.sim.camera.quarter, inputs);
        }
    }
    if !hints.is_empty() {
        println!("{} hints from {}", hints.len(), path.display());
    }
    Ok(hints)
}

/// A trace step's run: the replay `file` played (headless, from its own start) to tick `from`, then the ball's place
/// and speed and the push of the player's arrows at each tick to `to`.
fn load_track(
    file: &Path,
    from: u64,
    to: u64,
    gain: f32,
    data: &Data,
    o: &Options,
) -> Result<crate::hints::Track, String> {
    let bytes = crate::hints::read_plain(file, crate::hints::MAX_REPLAY_FILE)?;
    let text = String::from_utf8(bytes).map_err(|_| format!("{}: not UTF-8", file.display()))?;
    let run = Replay::parse(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut watch = o.clone();
    watch.replay = Some(file.to_path_buf());
    watch.follow = None;
    watch.autopilot = false;
    let mut seen = Game::new(&watch, data)?;
    while seen.sim.tick < from && !seen.sim.game_over {
        seen.tick(0);
    }
    let mut track = crate::hints::Track {
        gain,
        ..Default::default()
    };
    let mut changes = run.changes.iter().peekable();
    let mut input = 0u8;
    let mut bodies = Vec::new();
    for t in 0..to {
        // the input of tick t + 1 is the one the replay plays from the state at tick t (as for a replay step)
        while let Some(&&(_, i)) = changes.peek().filter(|c| c.0 <= t + 1) {
            input = i;
            changes.next();
        }
        if t < from {
            continue;
        }
        if seen.sim.game_over || seen.sim.finished {
            break;
        }
        let cam = &seen.sim.camera;
        track.pos.push(seen.sim.ball.position);
        track.vel.push(seen.sim.ball.velocity);
        track.push.push(crate::hints::push_of(
            input,
            cam.push_forward(),
            cam.push_right(),
        ));
        track.lift.push(input & crate::sim::input::LIFT != 0);
        track.on.push(crate::hints::carrier(&seen.sim));
        let world = seen.sim.world();
        bodies.push(
            seen.sim
                .loose()
                .iter()
                .filter_map(|&(o, b)| world.state(b).map(|st| (o, st.pos)))
                .collect::<Vec<_>>(),
        );
        seen.tick(0);
    }
    crate::hints::fill_meet(&mut track, &bodies);
    if std::env::var_os("TRACE_LOG").is_some() {
        let mut near = std::collections::BTreeMap::new();
        for (i, b) in bodies.iter().enumerate() {
            for &(o, p) in b {
                let d = track.pos.get(i).map_or(f32::INFINITY, |q| q.distance(p));
                let e = near.entry(o).or_insert((f32::INFINITY, 0usize));
                if d < e.0 {
                    *e = (d, i);
                }
            }
        }
        eprintln!("trace bodies nearest the ball (object: distance, tick): {near:?}");
    }
    println!(
        "trace {} {from}..{to}: {} ticks, {} on a body, {} before a body touches the ball ({} loose bodies)",
        file.display(),
        track.pos.len(),
        track.on.iter().filter(|o| o.is_some()).count(),
        track.meet.iter().filter(|o| o.is_some()).count(),
        bodies.first().map_or(0, Vec::len)
    );
    Ok(track)
}

/// A report of where the test player is stuck, for whoever writes its hints: what it knows of the place, and two
/// pictures, its own view and the place from above.
fn stuck_report(
    game: &mut Game,
    data: &Data,
    o: &Options,
    pilot: &crate::autopilot::Pilot,
    dir: &Path,
) -> Result<(), String> {
    use std::fmt::Write as _;
    let s = &game.sim;
    let b = s.ball.position;
    // a new folder of its own for each report (never one already there, so nothing in it is written through)
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let stem = format!("{}-t{}", level_stem(game), s.tick);
    let mut folder = None;
    for k in 0..100u32 {
        let f = dir.join(if k == 0 {
            stem.clone()
        } else {
            format!("{stem}-{k}")
        });
        match std::fs::create_dir(&f) {
            Ok(()) => {
                folder = Some(f);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", f.display())),
        }
    }
    let folder =
        folder.ok_or_else(|| format!("{}: no new folder for the report", dir.display()))?;
    let mut t = String::new();
    let _ = writeln!(
        t,
        "level {}  tick {} ({})  sector {}  lives {}",
        game.level.name,
        s.tick,
        clock(s.tick),
        s.sector + 1,
        s.lives
    );
    let v = s.ball.velocity;
    let _ = writeln!(
        t,
        "ball {:?} at ({:.2} {:.2} {:.2}) velocity ({:.2} {:.2} {:.2}) on ground {}",
        s.ball.kind, b.x, b.y, b.z, v.x, v.y, v.z, s.ball.on_ground
    );
    let _ = writeln!(
        t,
        "view quarter {} (up pushes along {:?})",
        s.camera.quarter,
        s.camera.push_forward()
    );
    let _ = writeln!(
        t,
        "route {} / {}; ahead:",
        pilot.reached(),
        pilot.route.points.len()
    );
    for p in pilot
        .route
        .points
        .iter()
        .skip(pilot.reached())
        .step_by(5)
        .take(12)
    {
        let _ = writeln!(t, "  ({:.1} {:.1} {:.1})", p.x, p.y, p.z);
    }
    let _ = writeln!(t, "transformers:");
    for (p, k) in s.trafos() {
        if p.distance(b) < 80.0 {
            let _ = writeln!(
                t,
                "  {k:?} at ({:.1} {:.1} {:.1}) {:.1} away",
                p.x,
                p.y,
                p.z,
                p.distance(b)
            );
        }
    }
    let level = &game.level.level;
    let _ = writeln!(t, "loose bodies:");
    for &(obj, body) in s.loose() {
        let (Some(o), Some(st)) = (level.objects.get(obj), s.world().state(body)) else {
            continue;
        };
        if st.pos.distance(b) < 60.0 {
            let p = st.pos;
            let _ = writeln!(
                t,
                "  {} at ({:.1} {:.1} {:.1}) {:.1} away",
                o.name,
                p.x,
                p.y,
                p.z,
                p.distance(b)
            );
        }
    }
    let _ = writeln!(t, "objects within 40:");
    let mut near: Vec<(f32, String)> = level
        .objects
        .iter()
        .enumerate()
        .filter(|(_, ob)| ob.mesh.is_some())
        .filter_map(|(i, ob)| {
            let p = glam::Vec3::from(ob.position());
            let d = p.distance(b);
            (d < 40.0).then(|| {
                let groups: Vec<&str> = level
                    .groups_of(i)
                    .filter(|g| !g.starts_with("Sector"))
                    .take(4)
                    .collect();
                (
                    d,
                    format!(
                        "  {} at ({:.1} {:.1} {:.1}) {:.1} away {:?}",
                        ob.name, p.x, p.y, p.z, d, groups
                    ),
                )
            })
        })
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, l) in near.iter().take(60) {
        let _ = writeln!(t, "{l}");
    }
    std::fs::write(folder.join("report.txt"), &t).map_err(|e| e.to_string())?;
    let mut shot = o.clone();
    shot.ticks = Some(game.sim.tick);
    shot.size = (960, 720);
    shot.look = None;
    screenshot(game, data, &shot, &folder.join("view.png"))?;
    for (name, h) in [("top.png", 45.0f32), ("wide.png", 120.0)] {
        shot.look = Some([b.x, b.y + h, b.z + 0.01, b.x, b.y, b.z]);
        screenshot(game, data, &shot, &folder.join(name))?;
    }
    println!("stuck: report in {}", folder.display());
    Ok(())
}

/// A state the test player goes back to: the run, its record, the player, how far it got and since when, the hint
/// at, how near the edges it rolled.
type Saved = (
    Sim,
    Replay,
    crate::autopilot::Pilot,
    usize,
    u64,
    Option<(usize, usize)>,
    crate::edges::EdgeStats,
);

/// The test player's run, tick by tick (headless in `pilot_run`, or in the window with --watch): its hints and
/// reports, going back when the ball is lost, giving up when the way stands still.
struct PilotRun {
    pilot: crate::autopilot::Pilot,
    report: PathBuf,
    limit: u64,
    give_up: u64,
    best: usize,
    since: u64,
    heard: usize,
    hint_at: Option<(usize, usize)>,
    dead: bool,
    rewinds: crate::autopilot::Rewinds<Saved>,
    edges: Arc<crate::edges::Edges>,
    near_edges: crate::edges::EdgeStats,
    /// The last time it went back, for whoever watches: what failed, where, and back to when.
    went_back: Option<String>,
    /// The watcher's zones file beside the hints (zones.rs), and the learned table shared by the levels (learned.rs).
    zones_path: PathBuf,
    learned_path: PathBuf,
}

impl PilotRun {
    fn new(
        game: &Game,
        data: &Data,
        o: &Options,
        mut pilot: crate::autopilot::Pilot,
    ) -> Result<PilotRun, String> {
        pilot.hints = crate::hints::Runner::new(load_hints(game, data, o)?);
        let zones_path = zones_path(game, o);
        pilot.zones = Arc::new(load_zones(&zones_path));
        let learned_path = zones_path.with_file_name(LEARNED);
        pilot.learned = Arc::new(load_learned(&learned_path));
        let level = &game.level.level;
        pilot.modules = Arc::new(
            (0..level.objects.len())
                .filter_map(|i| {
                    let name = crate::scene::module_of(level, i)?;
                    let ob = level.objects.get(i)?;
                    Some((glam::Vec3::from(ob.position()), name.to_owned()))
                })
                .collect(),
        );
        let level = &game.level.level;
        pilot.bridges = (0..level.objects.len())
            .filter(|&i| crate::scene::module_of(level, i) == Some("P_Modul_25"))
            .filter_map(|i| {
                level
                    .objects
                    .get(i)
                    .map(|ob| glam::Vec3::from(ob.position()))
            })
            .collect();
        if game.start.is_some() {
            pilot.take_up(&game.sim);
        }
        let hint_at = pilot.hints.at();
        // how near the floors' edges the ball rolls, in the run kept (edges.rs): the summary's last line
        let edges = pilot.route.edges.clone();
        Ok(PilotRun {
            pilot,
            // the player's own data folder, not a shared temporary one
            report: o
                .report
                .clone()
                .unwrap_or_else(|| crate::levels::data_dir().join("stuck")),
            limit: o.ticks.unwrap_or(60 * 60 * 20),
            // a dead run: the ball neither falls nor gets on along the way for --give-up seconds (it stands or goes
            // to and fro, not knowing where to go); its report is written and the run stops, for whoever writes the
            // hints. A ball that falls is an ordinary mistake: each fall starts the wait again.
            give_up: o.give_up.saturating_mul(60),
            best: 0,
            since: game.sim.tick,
            heard: game.sim.events.len(),
            hint_at,
            dead: false,
            // the states it goes back to when the ball is lost or the way stands still (autopilot.rs, Rewinds)
            rewinds: crate::autopilot::Rewinds::new(o.rewinds),
            edges,
            near_edges: crate::edges::EdgeStats::default(),
            went_back: None,
            zones_path,
            learned_path,
        })
    }

    /// Whether the run goes on.
    fn going(&self, game: &Game) -> bool {
        !self.dead && game.sim.tick < self.limit && !game.sim.finished && !game.sim.game_over
    }

    /// Before the player decides: a state saved to go back to, when one is due.
    fn before(&mut self, game: &Game) {
        let s = &game.sim;
        if self.rewinds.due(s) {
            let state = (
                s.clone(),
                game.replay_out.clone(),
                self.pilot.clone(),
                self.best,
                self.since,
                self.hint_at,
                self.near_edges,
            );
            self.rewinds.save(s.tick, state);
        }
    }

    /// One tick with input `i` (the player's), and what follows from it: back to a saved state when the ball was
    /// lost or the way stood still, a new ball's place on the way, a report when the run is dead.
    fn after(&mut self, game: &mut Game, data: &Data, o: &Options, i: u8) {
        game.tick(i);
        self.near_edges.tick(&self.edges, &game.sim.ball);
        let pilot = &mut self.pilot;
        if pilot.reached() > self.best {
            (self.best, self.since) = (pilot.reached(), game.sim.tick);
        }
        let heard = self.heard;
        // (a transformer rolled over by a slip, making the ball another kind than the way's, counts as a fall)
        let stray = game.sim.events.iter().skip(heard).any(|(_, e)| {
            matches!(e, crate::sim::Event::Transform(k) if pilot.stray_transform(*k, game.sim.ball.position))
        });
        let fell = stray
            || game
                .sim
                .events
                .iter()
                .skip(heard)
                .any(|(_, e)| matches!(e, crate::sim::Event::Died));
        // (a ball lost as the player lost one there is not gone back from: it may be what the level needs)
        let as_the_player = fell && !stray && pilot.lost_as_the_player(&game.sim);
        // lost, or standing still too long: back to a state saved before, varied (a try of its own, reproducible)
        let still = game.sim.tick - self.since > self.give_up.min(crate::autopilot::REWIND_STILL);
        // (standing still: from where it last got on, a state saved there too)
        let failed_at = if as_the_player {
            None
        } else if fell {
            Some(game.sim.tick)
        } else {
            still.then_some(self.since + 60)
        };
        if let Some(at) = failed_at
            && let Some((state, attempt)) = self.rewinds.back(
                at,
                (game.sim.sector, pilot.reached()),
                game.sim.ball.position,
                fell,
            )
        {
            let (sim, replay, p, b, sn, h, ne) = state;
            let line = format!(
                "{} {} at ({:.1} {:.1} {:.1}): back to {} (try {attempt}, {} rewinds)",
                clock(game.sim.tick),
                if fell { "lost" } else { "stuck" },
                game.sim.ball.position.x,
                game.sim.ball.position.y,
                game.sim.ball.position.z,
                clock(sim.tick),
                self.rewinds.total()
            );
            println!("{line}");
            self.went_back = Some(line);
            game.sim = sim;
            game.replay_out = replay;
            // (the zones as they are now: the watcher may have marked one since that state was saved)
            let (zones, learned) = (self.pilot.zones.clone(), self.pilot.learned.clone());
            self.pilot = p;
            self.pilot.zones = zones;
            self.pilot.learned = learned;
            self.pilot.perturb(&game.sim, attempt);
            (self.best, self.since, self.hint_at, self.near_edges) =
                (b, game.sim.tick.max(sn), h, ne);
            self.heard = game.sim.events.len();
            return;
        }
        // (a stray transformer taken as it came: back into one making the way's kind, as for a new ball)
        if stray
            || game
                .sim
                .events
                .iter()
                .skip(heard)
                .any(|(_, e)| matches!(e, crate::sim::Event::NewBall))
        {
            pilot.new_ball(&game.sim);
            // back at the reset point: getting on is counted from there again
            self.best = pilot.reached();
        }
        self.heard = game.sim.events.len();
        // a hint going on to its next step gets on too (its steps lead off the way: round, back for a run-up), and so
        // does one playing a recorded run; not one gone off course (its ball fell to a floor beneath a step's point)
        let now_at = pilot.hints.at();
        let hint_on = now_at != self.hint_at && now_at.is_some() && !pilot.hints.off_course();
        if fell || hint_on || pilot.hints.replaying() {
            self.since = game.sim.tick;
        }
        self.hint_at = now_at;
        if game.sim.tick - self.since > self.give_up {
            let b = game.sim.ball.position;
            println!(
                "dead run: ball at ({:.1} {:.1} {:.1}), not falling and the way not on for {} s",
                b.x,
                b.y,
                b.z,
                (game.sim.tick - self.since) / 60
            );
            if let Err(e) = stuck_report(game, data, o, &self.pilot, &self.report) {
                eprintln!("stuck report: {e}");
            }
            self.dead = true;
            return;
        }
        if game.sim.tick.is_multiple_of(600) {
            let b = game.sim.ball.position;
            println!(
                "{} sector {} lives {} route {} / {} ball ({:.1} {:.1} {:.1})",
                clock(game.sim.tick),
                game.sim.sector + 1,
                game.sim.lives,
                pilot.reached(),
                pilot.route.points.len(),
                b.x,
                b.y,
                b.z
            );
        }
    }

    /// The watcher drove since tick `began` and hands the ball back: the way is taken up from here, getting on is
    /// counted from here, and the states before are not gone back to (what the hands did is kept).
    fn resume(&mut self, game: &Game, began: u64) {
        self.pilot.resume(&game.sim, began);
        self.rewinds.forget();
        (self.best, self.since) = (self.pilot.reached(), game.sim.tick);
        self.heard = game.sim.events.len();
        self.hint_at = self.pilot.hints.at();
    }

    /// The summary of the run, and its record saved at --record.
    fn summary(&self, game: &Game, o: &Options) -> Result<(), String> {
        let s = &game.sim;
        let what = if s.finished {
            "finished"
        } else if s.game_over {
            "game over"
        } else if self.dead {
            "dead run"
        } else {
            "not finished"
        };
        println!(
            "result: {what} after {} ({}), sector {}, lives {}, route {} / {}",
            s.tick,
            clock(s.tick),
            s.sector + 1,
            s.lives,
            self.pilot.reached(),
            self.pilot.route.points.len()
        );
        println!("{}", self.near_edges.line());
        if let Some(path) = &o.record {
            std::fs::write(path, game.replay_out.to_text())
                .map_err(|e| format!("{}: {e}", path.display()))?;
            println!("saved {}", path.display());
        }
        Ok(())
    }
}

/// Runs the test player to --ticks (or the finish), prints how far it got and saves its run at --record. Where it
/// is stuck it writes a report (--report), and it follows the level's hints. With --watch it plays in the window.
fn pilot_run(
    mut game: Game,
    data: &Data,
    o: &Options,
    pilot: crate::autopilot::Pilot,
) -> Result<(), String> {
    let mut run = PilotRun::new(&game, data, o, pilot)?;
    if o.watch {
        return window(game, data.clone(), o, Some(Box::new(run)));
    }
    while run.going(&game) {
        run.before(&game);
        let i = run.pilot.next(&game.sim);
        run.after(&mut game, data, o, i);
    }
    run.summary(&game, o)
}

/// A video goes on this long past the finish (six seconds), for the end screen.
const VIDEO_AFTER: u64 = 360;

/// The time of one frame at 60 frames a second.
const FRAME: std::time::Duration = std::time::Duration::from_micros(16_667);

/// How frames are shown. `Immediate` by default: on the test machine the vertical-sync modes (`Fifo`,
/// `FifoRelaxed`) blocked the presenting for about a second per frame, while `Immediate` presents at once and may
/// tear. Synch to Screen (`vsync`, the Graphics page) or DRAHTSEILAKT_PRESENT=fifo gives the synchronised mode,
/// which never tears; the variable comes first.
pub(crate) fn present_mode(offered: &[wgpu::PresentMode], vsync: bool) -> wgpu::PresentMode {
    let wanted = match std::env::var("DRAHTSEILAKT_PRESENT").as_deref() {
        Ok("fifo") => wgpu::PresentMode::Fifo,
        Ok("relaxed") => wgpu::PresentMode::FifoRelaxed,
        Ok("immediate") => wgpu::PresentMode::Immediate,
        _ if vsync => wgpu::PresentMode::Fifo,
        _ => wgpu::PresentMode::Immediate,
    };
    if offered.contains(&wanted) {
        wanted
    } else {
        wgpu::PresentMode::Fifo
    }
}

/// The window's surface, its configuration and a renderer on it: gamma-space output, as the original (render.rs).
pub(crate) fn window_gpu(window: &Arc<Window>) -> Result<Gpu, String> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let surface = instance
        .create_surface(window.clone())
        .map_err(|e| e.to_string())?;
    let (adapter, device, queue) = pollster::block_on(adapter_device(&instance, Some(&surface)))?;
    let caps = surface.get_capabilities(&adapter);
    // Gamma-space output, as the original (see render.rs).
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .or(caps.formats.first().copied())
        .ok_or("no surface format")?;
    let size = window.inner_size();
    // read back for the screenshots where the surface allows it
    let usage =
        wgpu::TextureUsages::RENDER_ATTACHMENT | (caps.usages & wgpu::TextureUsages::COPY_SRC);
    let config = wgpu::SurfaceConfiguration {
        usage,
        format,
        width: size.width.clamp(1, crate::options::SURFACE_MAX),
        height: size.height.clamp(1, crate::options::SURFACE_MAX),
        present_mode: present_mode(&caps.present_modes, load_graphics().vsync),
        desired_maximum_frame_latency: 2,
        alpha_mode: caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto),
        view_formats: vec![],
        color_space: wgpu::SurfaceColorSpace::Auto,
    };
    eprintln!(
        "window: {:?} presentation {:?}, {}x{}, format {:?}, modes offered {:?}",
        adapter.get_info().name,
        config.present_mode,
        config.width,
        config.height,
        config.format,
        caps.present_modes
    );
    surface.configure(&device, &config);
    let renderer = Renderer::new(device, queue, format);
    Ok(Gpu {
        surface,
        config,
        renderer,
        modes: caps.present_modes,
    })
}

async fn device(
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), String> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    adapter_device(&instance, surface).await
}

async fn adapter_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        })
        .await
        .map_err(|e| format!("no graphics adapter: {e}"))?;
    // block-compressed textures (a remastered level's, gputex.rs) where the adapter has them: every desktop card
    // under Vulkan and Direct3D 12 and every Mac's under Metal; without them (some phones' chips, software
    // adapters) the game decodes them itself. DRAHTSEILAKT_NO_BC=1 plays as such an adapter does (for testing).
    let bc = wgpu::Features::TEXTURE_COMPRESSION_BC;
    let wanted = if std::env::var_os("DRAHTSEILAKT_NO_BC").is_some() {
        wgpu::Features::empty()
    } else {
        adapter.features() & bc
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("drahtseilakt"),
            required_features: wanted,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("no graphics device: {e}"))?;
    crate::render::note_adapter(&adapter);
    Ok((adapter, device, queue))
}

/// Uploads the menu scene's model (after the game's, whose model numbers it must not take).
fn upload_menu_scene(
    renderer: &mut Renderer,
    scene: Option<&mut crate::menuscene::MenuScene>,
    data: &Data,
) {
    if let Some(ms) = scene {
        let loader = |t: &level::Texture, bc: bool| data.texture_plan(t, bc);
        // the scene is another folder's: it shares no texture with the level uploaded before it
        renderer.end_sharing();
        ms.model = Some(renderer.add_model(&ms.level, &loader));
        renderer.end_sharing();
    }
}

/// The most pixels a map's 2D frame's texture may have (1024 x 1024: a signature or a logo is far smaller).
const FRAME_PIXELS_MAX: u64 = 1 << 20;

/// The level as its original has it (the Back to 2004 look): the meshes without the looks the toolbox rounded them
/// with, the textures from the folder's `textures.original/` where it has them, no normal maps. A level without any
/// of it comes out the same.
fn plain_level(level: &level::Level) -> level::Level {
    let mut l = level.clone();
    for m in &mut l.meshes {
        m.look = None;
    }
    for m in &mut l.materials {
        m.normal = None;
        m.parallax = 0.0;
    }
    for t in &mut l.textures {
        if let level::TextureSource::LevelFile(p) = &t.source
            && let (Some(dir), Some(name)) = (p.parent().and_then(Path::parent), p.file_name())
        {
            let original = dir.join("textures.original").join(name);
            if original.is_file() {
                t.source = level::TextureSource::LevelFile(original);
            }
        }
    }
    l
}

fn upload(renderer: &mut Renderer, game: &Game) {
    upload_prepare(renderer, game);
    let data = &game.data;
    let loader = |t: &level::Texture, bc: bool| data.texture_plan(t, bc);
    let started = std::time::Instant::now();
    renderer.begin_level();
    let plain = renderer.look.plain;
    renderer.plain_uploaded = plain;
    for l in level_models(game, plain) {
        renderer.add_model(&l, &loader);
    }
    models_uploaded(renderer, game, started);
    let mut step = 0;
    while upload_extras(renderer, game, step) {
        step += 1;
    }
}

/// What a level's upload sets before its models go up: the light, the fog, where a script reads textures from.
fn upload_prepare(renderer: &mut Renderer, game: &Game) {
    // the level's light colour (the level table's Light; white for a level without a row)
    renderer.light.color = game.level.level.settings.light.unwrap_or([1.0; 3]);
    renderer.fog = game.level.level.settings.fog;
    // where a script's first write into a level texture reads its picture from
    crate::matstate::set_texture_reader(game.data.clone());
}

/// The level's sky by its name (a name that could not name a file is taken as none); closed when it has a top.
fn sky_model(game: &Game) -> level::Level {
    let letter = crate::sky::override_name()
        .or_else(|| game.level.level.sky.clone())
        .filter(|n| crate::sky::valid_name(n))
        .unwrap_or_else(|| "A".into());
    let top = game
        .data
        .file(&format!(
            "Textures/Sky/{}",
            crate::sky::face_file(&letter, "Top")
        ))
        .is_some();
    crate::sky::model(&letter, top)
}

/// The models a game has on the card, in the order its instances name them: the level, the balls, the sky, the
/// modules, the transformer's animation.
fn level_models(game: &Game, plain: bool) -> Vec<std::borrow::Cow<'_, level::Level>> {
    use std::borrow::Cow;
    let mut v = vec![
        if plain {
            Cow::Owned(plain_level(&game.level.level))
        } else {
            Cow::Borrowed(&game.level.level)
        },
        Cow::Borrowed(&game.balls.level),
        Cow::Owned(sky_model(game)),
    ];
    v.extend(game.modules.models.iter().map(Cow::Borrowed));
    if let Some((file, _)) = &game.trafo {
        v.push(Cow::Borrowed(&file.level));
    }
    v
}

/// The level's models are up: nothing uploaded later shares with them; says what it cost.
fn models_uploaded(renderer: &mut Renderer, game: &Game, started: std::time::Instant) {
    crate::sky::set_faces(sky_model(game).meshes.len());
    renderer.end_sharing();
    let t = renderer.texture_stats;
    eprintln!(
        "textures: {} uploaded ({} compressed, {} shared with them), {} MB of video memory, in {:.2} s (reading {:.2} s, \
         upload {:.2} s){}",
        t.uploaded,
        t.compressed,
        t.shared,
        t.bytes / (1024 * 1024),
        started.elapsed().as_secs_f64(),
        t.read.as_secs_f64(),
        t.upload.as_secs_f64(),
        if renderer.bc {
            ""
        } else {
            "; no BC on this adapter: compressed textures are decoded, smaller"
        }
    );
}

/// The interface's and the particles' textures, by name.
fn interface_textures() -> Vec<&'static str> {
    // the own interface draws its texts in the shipped fonts: the copy's font is not read
    let own = gamedata::resources::in_use() == gamedata::resources::Provider::Own;
    let interface = [
        "Font_1",
        "Button01_special",
        "Button01_select",
        "Button01_deselect",
    ];
    crate::particles::textures()
        .into_iter()
        .chain(interface.into_iter().filter(|n| !(own && *n == "Font_1")))
        .collect()
}

/// One piece of what a level's upload does after its models (step `step`, from 0): the map's 2D frames and the
/// shadow, one of the interface's textures, the sea of cloud, the sky's colour. False when there is no such step.
fn upload_extras(renderer: &mut Renderer, game: &Game, step: usize) -> bool {
    let data = &game.data;
    let names = interface_textures();
    if step == 0 {
        // the map's own 2D frames: each one's texture under its slot's name, the slots of the level before emptied
        let frames = crate::hud::level_frames(&game.level.level);
        // one upload for each of the level's textures, whatever number of frames show it; a frame is a small picture,
        // and a larger one is not taken (a stranger's map does not get video memory outside the level's budget)
        let mut uploaded: Vec<(usize, &'static str)> = Vec::new();
        for (k, name) in crate::ui::FRAME_TEXTURES.into_iter().enumerate() {
            renderer.remove_named_texture(name);
            let Some(f) = frames.get(k) else { continue };
            if let Some((_, first)) = uploaded.iter().find(|(t, _)| *t == f.texture) {
                renderer.share_named_texture(name, first);
                continue;
            }
            let pixels = game
                .level
                .level
                .textures
                .get(f.texture)
                .and_then(|t| data.texture(t))
                .filter(|(w, h, _)| u64::from(*w) * u64::from(*h) <= FRAME_PIXELS_MAX);
            if let Some((w, h, rgba)) = pixels {
                renderer.add_named_texture(name, w, h, rgba);
                uploaded.push((f.texture, name));
            }
        }
        let shadow = level::Texture {
            name: "HardShadow".into(),
            source: level::TextureSource::File("HardShadow.bmp".into()),
            color_key: None,
        };
        if let Some((w, h, rgba)) = data.texture(&shadow) {
            renderer.set_shadow_texture(w, h, rgba);
        }
        return true;
    }
    if let Some(&name) = names.get(step - 1) {
        let t = level::Texture {
            name: name.to_owned(),
            source: level::TextureSource::File(
                if name.starts_with("Font_") || name.starts_with("Button") {
                    format!("{name}.tga")
                } else {
                    format!("{name}.bmp")
                },
            ),
            color_key: None,
        };
        if let Some((w, h, rgba)) = data.texture(&t) {
            renderer.add_named_texture(name, w, h, rgba);
        }
        return true;
    }
    let sky = sky_model(game);
    if step == names.len() + 1 {
        // the sea of cloud, fading far off into the sky's own colour at the horizon; it takes the place of the
        // level's flat cloud layer (SkyLayer)
        let sea = game.level.level.settings.clouds.or_else(|| {
            let sky = game.level.level.sky.as_deref()?;
            crate::clouds::default_sea(sky, game.sim.lowest_floor())
        });
        renderer.clouds = sea.and_then(|settings| {
            let faces: Vec<_> = sky
                .textures
                .iter()
                .take(4)
                .map(|t| data.texture(t))
                .collect();
            let (ring, haze) = crate::clouds::horizon_ring(&faces)?;
            Some(crate::clouds::Layer {
                settings,
                ring,
                haze,
            })
        });
        if let Some(ring) = renderer.clouds.as_ref().map(|l| l.ring.clone()) {
            renderer.set_cloud_ring(&ring);
        }
        scene::set_volume_clouds(renderer.clouds.is_some());
        return true;
    }
    // The zenith colour: the top row of the front texture, averaged.
    if let Some((w, _, rgba)) = sky.textures.first().and_then(|t| data.texture(t)) {
        let row = rgba.get(..(w as usize * 4)).unwrap_or_default();
        let n = (row.len() / 4).max(1) as f64;
        let avg = |c: usize| {
            row.iter()
                .skip(c)
                .step_by(4)
                .map(|&v| f64::from(v))
                .sum::<f64>()
                / n
                / 255.0
        };
        renderer.clear = wgpu::Color {
            r: avg(0),
            g: avg(1),
            b: avg(2),
            a: 1.0,
        };
    }
    false
}

/// The pixels of an offscreen frame (RGBA, rows top down).
fn read_back(r: &Renderer, target: &wgpu::Texture, w: u32, h: u32) -> Result<Vec<u8>, String> {
    let row = (4 * w).div_ceil(256) * 256;
    let buf = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(row) * u64::from(h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = r.device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    r.queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    r.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| e.to_string())?;
    let mapped = slice.get_mapped_range().map_err(|e| format!("{e:?}"))?;
    let mut pixels = Vec::with_capacity((4 * w * h) as usize);
    for y in 0..h {
        let start = (y * row) as usize;
        pixels.extend_from_slice(
            mapped
                .get(start..start + (4 * w) as usize)
                .unwrap_or_default(),
        );
    }
    Ok(pixels)
}

impl Game {
    /// Which of the sacks' supports are drawn (a view mod's `render.supports`: none, one design, varied). They are
    /// looked for the first time one is asked for, so a game without the mod (the classic mode, or the mod switched
    /// off) never spends a moment on them.
    fn show_supports(&mut self, set: u8) {
        if set != 0 && !self.supports_planned {
            self.supports_planned = true;
            let plan = crate::supports::plan(&self.level.level, &self.modules);
            self.placements.extend(plan.placements());
        }
        scene::set_supports(set);
    }
}

fn screenshot(game: &mut Game, data: &Data, o: &Options, path: &Path) -> Result<(), String> {
    if let Ok(l) = std::env::var("DRAHTSEILAKT_SHOT_LANG") {
        let _ = SHOT_LANG.set(l);
    }
    let ticks = o
        .ticks
        .or(game.replay_in.as_ref().map(|r| r.end))
        .unwrap_or(1);
    // the scripts' texture writes read their pictures from the game's files from the first tick on
    crate::matstate::set_texture_reader(data.clone());
    let mut prev = Snapshot::of(&game.sim);
    // after the finish the world goes on (the balloon, the end screen)
    while game.sim.tick < ticks && !game.sim.game_over {
        prev = Snapshot::of(&game.sim);
        game.tick(0);
    }
    let mut snap = Snapshot::lerp(&prev, &Snapshot::of(&game.sim), 1.0);
    if let Some([x, y, z, tx, ty, tz]) = o.look {
        snap.eye = glam::Vec3::new(x, y, z);
        snap.target = glam::Vec3::new(tx, ty, tz);
    }
    let (_, device, queue) = pollster::block_on(device(None))?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut r = Renderer::new(device, queue, format);
    // DRAHTSEILAKT_SHOT_MOD=key: the view mods' look of that mod switched on (e.g. back-to-2004), not for menu shots
    let look = match (
        std::env::var("DRAHTSEILAKT_SHOT_MOD"),
        std::env::var("DRAHTSEILAKT_SHOT_MENU"),
    ) {
        (Ok(key), Err(_)) => shot_look(&key, o.debug),
        _ => crate::render::RenderLook::default(),
    };
    r.look = look;
    upload(&mut r, game);
    // the passing lights as the window would show them (the lightning on a fixed schedule)
    if game.last_level {
        r.flash = crate::lights::Blitz::new(1).step(game.sim.tick).0;
    }
    r.point = crate::lights::point(&game.sim.events, game.sim.tick, snap.ball);
    // what the map's scripts changed of the light and the fog by then
    let env = game.sim.script_env();
    if let Some(c) = env.ingame_light() {
        r.light.color = c;
    }
    r.script_fog = env.fog;
    let (w, h) = o.size;
    let target = r.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("screenshot"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let full_view = target.create_view(&Default::default());
    r.look = look;
    game.show_supports(look.supports);
    let ((fw, fh), (w, h)) = ((w, h), look.size(w, h));
    let view = if look.post() {
        let format = r.format;
        let device = r.device.clone();
        r.retro
            .get_or_insert_with(|| crate::retro::Retro::new(&device, format))
            .target(&device, format, w, h)
    } else {
        full_view.clone()
    };
    // DRAHTSEILAKT_SHOT_MENU=base:keys draws a menu over the frame, to look at the menus without a window: base is
    // main, pause, dead or end, keys are u d l r (arrows), e (Enter) and b (Esc), e.g. "main:ddde" for Options. A
    // menu not over the level has the menu scene behind it, DRAHTSEILAKT_SHOT_MENU_MS (default 0) into the menu.
    let shot = std::env::var("DRAHTSEILAKT_SHOT_MENU").ok();
    let menu = shot.and_then(|spec| shot_menu(&spec, data, game, o, (w, h)));
    let mut menu_scene = menu
        .as_ref()
        .filter(|m| !m.over_level())
        .and_then(|_| load_menu_scene(data));
    upload_menu_scene(&mut r, menu_scene.as_mut(), data);
    if let Some(ms) = menu_scene.filter(|m| m.model.is_some()) {
        let at = std::env::var("DRAHTSEILAKT_SHOT_MENU_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        let (inst, particles, frame_view) = ms.frame(at);
        r.menu_frame = true;
        r.draw(&view, w, h, &frame_view, &inst, &particles);
        let screen = crate::ui::Screen {
            w: w as f32,
            h: h as f32,
        };
        let quads = menu
            .map(|m| m.interface(game.font.as_ref(), screen))
            .unwrap_or_default();
        if let Some((aw, ah, rgba)) = crate::fallback::take_changed() {
            r.add_named_texture(crate::fallback::TEXTURE, aw, ah, rgba);
        }
        r.draw_ui(&view, w, h, &quads);
        return save_target(&r, &target, (w, h), path, game.sim.tick);
    }
    let inst = scene::instances(
        &game.placements,
        &game.sim.taken,
        &game.sim.unseen,
        &game.sim.world().bodies,
        &game.ball_meshes,
        game.trafo.as_ref().map(|t| &t.1),
        (&game.sim.extra_views(), game.extra_model.as_ref()),
        &snap,
        &game.sim.pieces_view(),
        game.sim.script_entities(),
    );
    let mut inst = inst;
    if snap.kind.is_some() {
        scene::shadows(&game.shadow_receivers, snap.ball, &mut inst);
    }
    let mut particles = game.effects.batches(snap.eye, snap.target);
    if let Some(sc) = &game.sim.scripts {
        r.sync_materials(scene::LEVEL_MODEL, &sc.mats);
    }
    crate::script_text3d::add(game.sim.scripts.as_ref(), &mut particles, &mut r);
    r.apply_script_sky(scene::LEVEL_MODEL, env.sky.as_ref(), snap.eye, &mut inst);
    r.draw(&view, w, h, &scene::view(&snap), &inst, &particles);
    // DRAHTSEILAKT_BENCH=N: draws the frame N times more, with each quality of the clouds, and prints the time a frame
    // takes (the GPU waited for after each batch), to measure what the sea of cloud costs
    if let Some(n) = std::env::var("DRAHTSEILAKT_BENCH")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    {
        let chosen = crate::clouds::quality();
        for q in [
            crate::clouds::Quality::Off,
            crate::clouds::Quality::Low,
            crate::clouds::Quality::Medium,
            crate::clouds::Quality::High,
        ] {
            crate::clouds::set_quality(q);
            r.draw(&view, w, h, &scene::view(&snap), &inst, &particles);
            let _ = r.device.poll(wgpu::PollType::wait_indefinitely());
            let start = std::time::Instant::now();
            for _ in 0..n {
                r.draw(&view, w, h, &scene::view(&snap), &inst, &particles);
            }
            let _ = r.device.poll(wgpu::PollType::wait_indefinitely());
            println!(
                "clouds {}: {:.2} ms a frame at {w}x{h}",
                q.key(),
                start.elapsed().as_secs_f64() * 1000.0 / f64::from(n.max(1))
            );
        }
        crate::clouds::set_quality(chosen);
        r.draw(&view, w, h, &scene::view(&snap), &inst, &particles);
    }
    r.draw_overlay(
        &view,
        w,
        h,
        &crate::hud::overlay(&game.sim, w, h, true, game.font.is_some()),
    );
    let screen = crate::ui::Screen {
        w: w as f32,
        h: h as f32,
    };
    let mut quads = crate::hud::interface(&game.sim, game.font.as_ref(), screen);
    let strings = load_strings(data);
    let lost = strings.ours("ball lost");
    quads.extend(crate::hud::ball_lost(
        &game.sim,
        game.ball_lost_at,
        &lost,
        game.font.as_ref(),
        screen,
    ));
    if let (Some(t), Some(f)) = (crate::endscreen::of(&game.sim), game.font.as_ref()) {
        quads.extend(crate::endscreen::interface(&t, f, &strings, screen));
    }
    if let Some(m) = menu {
        if !m.over_level() {
            quads.clear();
        }
        quads.extend(m.interface(game.font.as_ref(), screen));
    }
    if let Some((aw, ah, rgba)) = crate::fallback::take_changed() {
        r.add_named_texture(crate::fallback::TEXTURE, aw, ah, rgba);
    }
    r.draw_ui(&view, w, h, &quads);
    if let (true, Some(retro)) = (look.post(), &r.retro) {
        let place = look.placed(fw, fh);
        retro.present(
            &r.device,
            &r.queue,
            &full_view,
            place,
            look.colour_bits == 16,
        );
    }
    save_target(&r, &target, (fw, fh), path, game.sim.tick)
}

/// The look a view mod asks of the picture, with its options as they come (a screenshot's `DRAHTSEILAKT_SHOT_MOD`).
fn shot_look(key: &str, debug: bool) -> crate::render::RenderLook {
    let want = format!("mod.{key}");
    let mods = crate::modrt::Mods::load(gamedata::mods::scan(debug), |k| {
        (k == want).then(|| "on".to_owned())
    });
    let (sim, mut view) = mods.running(true);
    view.fire(gamedata::mods::Event::Frame, &|_| 0.0, &sim, "en");
    view.out.look
}

/// Reads the rendered frame back and writes it to a PNG.
fn save_target(
    r: &Renderer,
    target: &wgpu::Texture,
    (w, h): (u32, u32),
    path: &Path,
    tick: u64,
) -> Result<(), String> {
    let pixels = read_back(r, target, w, h)?;
    image::save_buffer(path, &pixels, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    println!("{} written ({}x{}, tick {})", path.display(), w, h, tick);
    Ok(())
}

/// O_NOFOLLOW: the screenshot is never written through a link.
#[cfg(unix)]
fn libc_nofollow() -> i32 {
    libc::O_NOFOLLOW
}

/// The scene behind the menus: the original's (MenuLevel.nmo) with the copy's interface, else ours.
fn load_menu_scene(data: &Data) -> Option<crate::menuscene::MenuScene> {
    if gamedata::resources::in_use() == gamedata::resources::Provider::Own {
        let dir = gamedata::resources::own_root()?
            .join("3D Entities")
            .join("MenuLevel");
        return crate::menuscene::MenuScene::load_open(&dir);
    }
    crate::menuscene::MenuScene::load(data)
}

pub(crate) struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub renderer: Renderer,
    /// The presenting modes the surface offers.
    pub modes: Vec<wgpu::PresentMode>,
}

/// How long the card's share of a loading may take in one frame.
const LOAD_SLICE: std::time::Duration = std::time::Duration::from_millis(26);
/// A loading that takes longer than this shows its screen (a quicker one stays as it was).
const LOAD_SCREEN_AFTER: std::time::Duration = std::time::Duration::from_millis(500);

/// A level on its way in (`App::load`).
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

struct Load {
    source: crate::assets::Source,
    data: Data,
    number: Option<u32>,
    plain: bool,
    began: Instant,
    /// What the loading screen says and the mood of its picture: the level's name, who made it, its sky's.
    title: String,
    credit: Option<String>,
    mood: crate::titlescene::Mood,
    /// Set to stop the thread (Esc).
    cancel: Arc<std::sync::atomic::AtomicBool>,
    up: crate::loading::Uploader,
    game_rx: std::sync::mpsc::Receiver<Result<Box<Game>, String>>,
    game: Option<Box<Game>>,
    models_done: bool,
    extra: usize,
    /// The longest step of each kind so far (textures, making builds, models, extras).
    worst: Arc<[AtomicU64; 4]>,
}

impl Load {
    /// Whether the loading screen is up.
    fn shown(&self) -> bool {
        self.began.elapsed() >= LOAD_SCREEN_AFTER
    }
}

/// The loading thread: reads and builds the level, then reads the textures its models draw with and packs their
/// meshes, handing them over as they are ready (the channel holds a few, so the pictures never all wait in memory).
fn load_worker(
    o: Options,
    data: Data,
    bc: bool,
    plain: bool,
    uploader: crate::render::TextureUploader,
    tx: std::sync::mpsc::SyncSender<crate::loading::Item>,
    game_tx: std::sync::mpsc::Sender<Result<Box<Game>, String>>,
    cancel: &std::sync::atomic::AtomicBool,
) {
    use crate::loading::{Item, Stage};
    use std::sync::atomic::Ordering::Relaxed;
    let game = match Game::new(&o, &data) {
        Ok(g) => g,
        Err(e) => {
            let _ = game_tx.send(Err(e));
            return;
        }
    };
    let models = level_models(&game, plain);
    let loader = |t: &level::Texture, bc: bool| game.data.texture_plan(t, bc);
    let mut plans = std::collections::VecDeque::new();
    for (mi, l) in models.iter().enumerate() {
        for (i, p) in crate::render::texture_plans(l, &loader, bc) {
            plans.push_back((mi, i, p));
        }
    }
    crate::loading::textures_planned(plans.len());
    crate::loading::stage(Stage::Textures);
    let Some(uploader) = read_and_upload(plans, None, uploader, &tx, cancel) else {
        return;
    };
    for (mi, l) in models.iter().enumerate() {
        if cancel.load(Relaxed) {
            return;
        }
        if tx
            .send(Item::Packed(mi, crate::render::prepare_meshes(l)))
            .is_err()
        {
            return;
        }
    }
    drop(models);
    if tx.send(Item::Finished(uploader.stats)).is_err() {
        return;
    }
    let _ = game_tx.send(Ok(Box::new(game)));
}

/// How long the splash stands at least, and how long it takes to give way to the menu.
const SPLASH_MIN: std::time::Duration = std::time::Duration::from_millis(1400);
const SPLASH_FADE: std::time::Duration = std::time::Duration::from_millis(450);

/// The title screen at the start of the game.
struct Splash {
    began: Instant,
    /// A key or a click asked for the menu.
    skipped: bool,
    /// When the splash began to give way.
    fading: Option<Instant>,
}

impl Splash {
    /// 1 while it stands, down to 0 as it fades.
    fn fade(&self) -> f32 {
        self.fading.map_or(1.0, |t| {
            1.0 - (t.elapsed().as_secs_f32() / SPLASH_FADE.as_secs_f32()).clamp(0.0, 1.0)
        })
    }
}

/// What the start still has to put on the card for the menu: the first models, the extras, the menu's scene.
#[derive(Default)]
struct Startup {
    phase: u8,
    up: Option<crate::loading::Uploader>,
    extra: usize,
    /// How many models the placeholder game has: the menu's scene is the next one.
    models: usize,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

/// Reads textures (several at a time, each on its own thread) and puts them on the card, sending each view as it is
/// there. `boundary`: the model from which no texture is shared with those before. The uploader back, or None when
/// the receiver is gone or the work was cancelled.
fn read_and_upload(
    mut plans: std::collections::VecDeque<(usize, usize, gamedata::assets::TexturePlan)>,
    boundary: Option<usize>,
    mut uploader: crate::render::TextureUploader,
    tx: &std::sync::mpsc::SyncSender<crate::loading::Item>,
    cancel: &std::sync::atomic::AtomicBool,
) -> Option<crate::render::TextureUploader> {
    use crate::loading::Item;
    while !plans.is_empty() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let batch: Vec<_> = plans
            .drain(..plans.len().min(crate::render::LOADS_AT_ONCE))
            .collect();
        let loaded: Vec<_> = std::thread::scope(|s| {
            let threads: Vec<_> = batch
                .into_iter()
                .map(|(mi, i, plan)| (mi, i, s.spawn(move || plan.load())))
                .collect();
            threads
                .into_iter()
                .map(|(mi, i, t)| (mi, i, t.join().ok().flatten()))
                .collect()
        });
        for (mi, i, data) in loaded {
            if Some(mi) == boundary {
                uploader.end_sharing();
            }
            let view = data.and_then(|d| uploader.upload(d));
            crate::loading::texture_done();
            tx.send(Item::View(mi, i, view)).ok()?;
        }
    }
    Some(uploader)
}

struct App {
    /// The splash while it is up, and the start-up work behind it.
    /// DRAHTSEILAKT_AUTOLOAD has been done.
    autoloaded: bool,
    splash: Option<Splash>,
    /// The title and loading screens' words and shown progress.
    art: crate::loading::Art,
    startup: Option<Box<Startup>>,
    game: Game,
    /// The options the game was started with, for loading another level.
    opts: Options,
    /// The original level being played, if it is one.
    number: Option<u32>,
    /// The menu, while it is open; the run is paused under it.
    menu: Option<Menu>,
    /// The pause menu was opened over the run and the scripts were told (they hear the end of it too).
    pause_told: bool,
    /// A level being loaded: its CPU work on a thread, its upload to the card in slices (`App::loading_step`).
    loading: Option<Box<Load>>,
    /// The longest frame of the loading in progress, and how many there were.
    load_frames: (std::time::Duration, u32),
    /// The time the main thread spent in the loading's slices.
    load_busy: std::time::Duration,
    /// Frames of the loading that took over 50 ms.
    load_slow: u32,
    load_last_step: std::time::Duration,
    load_stages: [std::time::Duration; 6],
    /// The extended mode and which of its additions are on (features.rs).
    features: crate::features::Features,
    /// The copy's words for the menus.
    strings: crate::lang::Strings,
    /// The name last entered for a highscore.
    player: Vec<u8>,
    /// The free camera while it is up (the run waits), and the keys flying it.
    fly: Option<FreeCam>,
    fly_keys: u16,
    /// Where the camera looked at the last frame, for taking up the free camera there.
    last_view: (glam::Vec3, glam::Vec3),
    /// The sound output, if there is a device; and how many of the run's events have sounded.
    audio: Option<Audio>,
    heard: usize,
    /// The level's start sound has sounded for this run.
    started: bool,
    /// The loops playing, by slot: rolling (stone, wood, metal), then the fans'.
    rolling: [Option<u64>; 3 + crate::ballsound::FAN_SLOTS],
    /// The voices of the map's scripts in the audio (script_sound.rs).
    script_voices: crate::script_sound::Voices,
    /// The level's music.
    music: Option<Music>,
    /// The last level's lightning (lights.rs).
    blitz: crate::lights::Blitz,
    /// The player's sound and music volumes (the Settings page, kept in settings.txt).
    volumes: [f32; 2],
    data: Data,
    size: (u32, u32),
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    keys: u8,
    last: Instant,
    acc: f32,
    prev: Option<Snapshot>,
    /// Timings for DRAHTSEILAKT_TIMING: frames, and the time spent in the ticks and in the drawing of each.
    timing: Option<Timing>,
    error: Option<String>,
    /// The original's menu scene (MenuLevel.nmo), drawn behind the menus, and how long the menu has been up.
    menu_scene: Option<crate::menuscene::MenuScene>,
    menu_ms: f64,
    /// Extended profile: how fast the run goes against real time (the run itself is the same tick by tick), and a
    /// saved moment of the run (F5) to go back to (F9).
    time_scale: f32,
    saved: Option<(Sim, Replay)>,
    /// Extended profile: the run as it was each second of the last half minute, to go five seconds back (F3).
    history: std::collections::VecDeque<(Sim, Replay)>,
    /// The keys down now.
    held: std::collections::HashSet<KeyCode>,
    /// A held arrow or page key in the menus repeats (menu.rs).
    key_repeat: crate::menu::KeyRepeat,
    repeat_at: Instant,
    /// The Graphics page's choices, and the sizes its resolution offers.
    graphics: crate::options::Graphics,
    resolutions: Vec<(u32, u32)>,
    /// The keys of the game's actions, and the gamepads with the keys they hold.
    bindings: crate::options::Bindings,
    pads: crate::options::Pads,
    pad_keys: u8,
    /// A pad button that closed the menu is still held: the run takes none of the pad's keys until all are let go.
    pad_held: bool,
    /// The mouse's cursor in the window (pixels), while it is over it.
    cursor: Option<(f64, f64)>,
    /// The mouse buttons down (`script::input::LEFT` and the others), for the scripts.
    mouse_down: u8,
    /// The wheel's turn not yet a whole notch (a touchpad's pixels come in small steps).
    wheel: f64,
    /// The cursor is shown (menus) or hidden (the run); None until first set.
    cursor_shown: Option<bool>,
    /// The test player playing in the window (--watch), while it plays.
    pilot: Option<PilotWatch>,
    /// The mods (modrt.rs, docs/drahtseilakt/mods.md): those found, which are on and their options; their view side
    /// running; when this run began (real time); the mouse's motion this frame; the frames a second.
    mods: crate::modrt::Mods,
    view_mods: crate::modrt::ViewMods,
    mod_began: Instant,
    /// The time played per level (a mod's `playtime`), when it was last written, and the camera as eased (a mod's
    /// `camera.smooth`): yaw, distance, field of view, and the free camera's eye and target.
    playtimes: crate::playtime::PlayTimes,
    playtime_saved: Instant,
    eased: Option<([f32; 3], Option<(glam::Vec3, glam::Vec3)>)>,
    mouse: (f32, f32),
    fps: f32,
    /// The best run as a ghost (a mod's `ghost`), and whether it was looked for in this run.
    ghost: Option<Box<Ghost>>,
    ghost_tried: bool,
    /// The stall log's notes for the frame in progress, the frames drawn, the title as shown.
    stall: Stall,
    frames: u64,
    shown_title: String,
    /// The best run's sector times (a mod's `splits`), read once a run.
    best_splits: Option<Vec<u64>>,
    /// The level's light as uploaded (a mod's `light` dims it).
    light_base: Option<crate::render::Light>,
    /// A screenshot (a mod's `screenshot`) is due after this frame is drawn; when the last was taken.
    shot_due: bool,
    last_shot: Option<Instant>,
}

/// The level's best run played beside this one (a mod's `ghost`): its run, its input, and where its ball was at each
/// tick (so a run taken back still finds it).
struct Ghost {
    sim: Sim,
    replay: Replay,
    path: Vec<(glam::Vec3, glam::Quat, Option<crate::sim::BallKind>)>,
}

/// The test player in the window (--watch): its run, and what the watcher does with it.
struct PilotWatch {
    run: Box<PilotRun>,
    paused: bool,
    /// How fast the run goes against real time (the run is the same tick by tick).
    speed: f32,
    /// Driven by hand (F5) since this tick, from this place, in this sector, as this ball.
    hand: Option<(u64, glam::Vec3, usize, crate::sim::BallKind)>,
    /// The test player deciding the next tick's input on its own thread (its searches take a while), since when.
    pending: Option<(
        std::sync::mpsc::Receiver<(crate::autopilot::Pilot, u8)>,
        Instant,
    )>,
    /// A message for the watcher, and since when it shows.
    notice: Option<(String, Instant)>,
    /// The run is over and summed up: the window plays on as the game.
    done: bool,
    /// The box of the watcher's keys is shown (F1).
    keys_shown: bool,
    /// The ball's path of the last seconds (tick, place, sector), for the zones marked (F8, F4).
    /// (And the ball's kind, its speed and what steered then, for what a marked stretch was like: learned.rs.)
    trail: std::collections::VecDeque<Trail>,
    /// A zone being marked: its kind, the key held, from which tick of the trail, in which sector, since when.
    marking: Option<(crate::zones::Kind, KeyCode, u64, usize, Instant)>,
    /// How far back a mark reaches before its key went down, in seconds ([ and ]).
    look_back: u32,
    /// The zones are shown in the world (F12).
    zones_shown: bool,
    /// The zone marked last, shown a moment so the watcher sees what got marked.
    flash: Option<(crate::zones::Zone, Instant)>,
    /// The zones marked in this window, last last: each mark's id (in its lines) and whether it also taught a rule
    /// (undone with it).
    marked: Vec<(String, bool)>,
}

/// A tick of the watched ball's trail: the tick, its place, sector, kind, speed and what steered.
type Trail = (
    u64,
    glam::Vec3,
    usize,
    crate::sim::BallKind,
    f32,
    &'static str,
);

/// A mark reaches this far back before its key went down by default, and at most (seconds); the trail is kept a
/// little longer; a key held longer than `MARK_HELD_MAX` ends its zone (a release the window never heard).
const LOOK_BACK: u32 = 3;
const LOOK_BACK_MAX: u32 = 10;
const TRAIL_TICKS: u64 = 60 * 12;
const MARK_HELD_MAX: std::time::Duration = std::time::Duration::from_secs(120);
/// A zone just marked is shown this long; the trail is thinned to a point every so many units.
const FLASH: std::time::Duration = std::time::Duration::from_secs(4);
const MARK_STEP: f32 = 1.5;

/// The lines of the watched test player's status in the top left corner, in the interface's font.
fn watch_overlay(
    text: &str,
    f: &crate::ui::Face,
    screen: crate::ui::Screen,
) -> Vec<crate::ui::Quad> {
    let style = crate::ui::Style {
        scale: [0.45, 0.5],
        spacing: [1.0, 1.0],
        align: crate::ui::ALIGN_LEFT | crate::ui::ALIGN_TOP,
        flags: crate::ui::TEXT_SCREEN,
        margins: [0.0; 4],
        color: [0.75, 1.0, 0.75, 1.0],
        v2: true,
        shadow: Some(([1.0, 1.0], [0.0, 0.0, 0.0, 0.8])),
    };
    let mut q = Vec::new();
    for (k, line) in text.lines().enumerate() {
        let y = 0.01 + 0.032 * k as f32;
        let quads = f.layout(
            line,
            crate::ui::homogeneous([0.01, y, 0.99, y + 0.032]),
            &style,
        );
        q.extend(crate::ui::place(quads, screen, crate::ui::Anchor::Left));
    }
    q
}

/// The watcher's keys' box: a line a key, from this high in the window down to just over the lives.
const KEYS_LINE: f32 = 0.03;
const KEYS_BOTTOM: f32 = 0.85;
const KEYS_RIGHT: f32 = 0.62;

impl PilotWatch {
    /// The box of the watcher's keys, bottom left, in the menus' language: the state that is on (paused, driven by
    /// hand, another speed) stands out.
    fn keys_box(
        &self,
        strings: &crate::lang::Strings,
        f: &crate::ui::Face,
        screen: crate::ui::Screen,
    ) -> Vec<crate::ui::Quad> {
        let speed = format!("{}", self.speed);
        let lines = [
            (
                strings.ours(if self.paused {
                    "watch paused"
                } else {
                    "watch pause"
                }),
                self.paused,
            ),
            (
                strings.ours("watch speed").replace("{n}", &speed),
                self.speed != 1.0,
            ),
            (
                strings.ours(if self.hand.is_some() {
                    "watch driving"
                } else {
                    "watch hands"
                }),
                self.hand.is_some(),
            ),
            (strings.ours("watch skip"), false),
            (strings.ours("watch stuck"), false),
            (
                strings.ours("watch flow"),
                self.marking
                    .is_some_and(|m| m.0 == crate::zones::Kind::Flow),
            ),
            (
                strings.ours("watch careful"),
                self.marking
                    .is_some_and(|m| m.0 == crate::zones::Kind::Careful),
            ),
            (
                strings
                    .ours("watch mark how")
                    .replace("{n}", &self.look_back.to_string()),
                false,
            ),
            (
                strings
                    .ours("watch look back")
                    .replace("{n}", &self.look_back.to_string()),
                self.look_back != LOOK_BACK,
            ),
            (strings.ours("watch undo"), false),
            (
                strings.ours(if self.zones_shown {
                    "watch zones shown"
                } else {
                    "watch zones"
                }),
                self.zones_shown,
            ),
            (strings.ours("watch quit"), false),
            (strings.ours("watch box"), false),
        ];
        let top = KEYS_BOTTOM - KEYS_LINE * lines.len() as f32 - 0.01;
        let shade = [0.0, 0.0, 0.0, 0.55];
        let mut q = vec![crate::ui::Quad {
            angle: 0.0,
            rect: crate::ui::homogeneous([0.01, top, KEYS_RIGHT, KEYS_BOTTOM]),
            uv: [0.0, 0.0, 1.0, 1.0],
            top: shade,
            bottom: shade,
            // (no such texture: drawn plain)
            texture: "",
        }];
        let mut style = crate::ui::Style {
            scale: [0.42, 0.46],
            spacing: [1.0, 1.0],
            align: crate::ui::ALIGN_LEFT | crate::ui::ALIGN_TOP,
            flags: crate::ui::TEXT_SCREEN,
            margins: [0.0; 4],
            color: [0.85, 0.85, 0.85, 1.0],
            v2: true,
            shadow: None,
        };
        for (k, (line, on)) in lines.iter().enumerate() {
            let y = top + 0.005 + KEYS_LINE * k as f32;
            style.color = if *on {
                [1.0, 0.85, 0.3, 1.0]
            } else {
                [0.85, 0.85, 0.85, 1.0]
            };
            q.extend(f.layout(
                line,
                crate::ui::homogeneous([0.02, y, KEYS_RIGHT, y + KEYS_LINE]),
                &style,
            ));
        }
        crate::ui::place(q, screen, crate::ui::Anchor::Left)
    }

    /// The zones drawn in the world as translucent beads along their paths: all of this sector's with F12 (flow
    /// green, careful orange), the one being marked and the one just marked brighter and larger.
    fn zone_instances(&self, game: &Game, out: &mut Vec<crate::render::Instance>) {
        use crate::zones::Kind;
        let Some(mesh) = game
            .ball_meshes
            .wood
            .or(game.ball_meshes.stone)
            .or(game.ball_meshes.paper)
        else {
            return;
        };
        let color = |k: Kind, a: f32| match k {
            Kind::Flow => [0.2, 0.95, 0.45, a],
            Kind::Careful => [1.0, 0.55, 0.1, a],
        };
        let mut bead = |p: glam::Vec3, r: f32, c: [f32; 4]| {
            let m = glam::Mat4::from_scale_rotation_translation(
                glam::Vec3::splat(r / crate::sim::BALL_RADIUS),
                glam::Quat::IDENTITY,
                p,
            );
            out.push(crate::render::Instance {
                marker: Some(c),
                ..crate::render::Instance::plain(scene::BALLS_MODEL, mesh, m)
            });
        };
        let sector = game.sim.sector + 1;
        if self.zones_shown {
            for z in self
                .run
                .pilot
                .zones
                .zones
                .iter()
                .filter(|z| z.sector == sector)
            {
                if let [p] = z.points.as_slice() {
                    bead(*p, z.radius, color(z.kind, 0.15));
                }
                for &p in &z.points {
                    bead(p, 0.5, color(z.kind, 0.4));
                }
            }
        }
        if let Some((z, at)) = self
            .flash
            .as_ref()
            .filter(|f| f.1.elapsed() < FLASH && f.0.sector == sector)
        {
            // fading out
            // (in steps of a twentieth: each colour is a material of its own)
            let a = 0.7 * (1.0 - at.elapsed().as_secs_f32() / FLASH.as_secs_f32());
            let a = (a * 20.0).round() / 20.0;
            for &p in &z.points {
                bead(p, 0.9, color(z.kind, a.max(0.1)));
            }
        }
        if let Some((kind, _, from, sec, _)) = self.marking {
            let path: Vec<glam::Vec3> = self
                .trail
                .iter()
                .filter(|t| t.0 >= from && t.2 == sec)
                .map(|t| t.1)
                .collect();
            for p in crate::zones::thin(&path, MARK_STEP) {
                bead(p, 0.9, color(kind, 0.6));
            }
        }
    }

    /// What the watcher reads: the speed and state, the sector, lives and the edge, what the test player does and
    /// where to, the last notice.
    fn status(&self, game: &Game) -> String {
        let s = &game.sim;
        let mut state = format!("TEST PLAYER  x{}", self.speed);
        if self.paused {
            state += "  PAUSED (Space)";
        }
        if let Some((t, ..)) = self.hand {
            state += &format!("  YOUR HANDS since {} (F5 hands back)", clock(t));
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.1.elapsed() > THINKING)
        {
            state += "  thinking...";
        }
        if let Some((kind, ..)) = self.marking {
            state += &format!(
                "  MARKING {} ZONE (release the key to end)",
                kind.word().to_uppercase()
            );
        }
        let edge = self
            .run
            .edges
            .ball_clearance(s.ball.position)
            .map_or("-".into(), |d| format!("{d:.1}"));
        let mut t = format!(
            "{state}\n{}  sector {}  lives {}  edge {edge}  rewinds {}\n{}",
            clock(s.tick),
            s.sector + 1,
            s.lives,
            self.run.rewinds.total(),
            self.run.pilot.status()
        );
        // (with the zones shown, the place as the learned rules see it)
        if self.zones_shown {
            let p = &self.run.pilot;
            t += &format!(
                "\nhere: {}",
                p.situation(s.ball.position, p.reached(), s.ball.kind)
                    .describe()
            );
        }
        if let Some((n, at)) = &self.notice
            && at.elapsed() < NOTICE
        {
            t += &format!("\n{n}");
        }
        t
    }
}

/// The stretch the watcher drove by hand (`hand`: since which tick, from where, in which sector, as which ball),
/// saved beside the level's hints as `<level>-hand-<tick>.replay` (the run up to now, so it replays exactly), with
/// a hint line playing it from that place appended to `<level>-hand.txt`: what is said on the screen.
fn save_hand(
    game: &Game,
    o: &Options,
    hand: (u64, glam::Vec3, usize, crate::sim::BallKind),
) -> Result<String, String> {
    let (from, at, sector, kind) = hand;
    let path = hints_path(game, o);
    let dir = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    let stem = level_stem(game);
    let name = format!("{stem}-hand-{from}.replay");
    let file = dir.join(&name);
    crate::hints::write_plain(&file, &game.replay_out.to_text())?;
    let kind = format!("{kind:?}").to_ascii_lowercase();
    let line = format!(
        "at {:.2} {:.2} {:.2} r 4 sector {} kind {kind} : replay {name} {from} {}\n",
        at.x,
        at.y,
        at.z,
        sector + 1,
        game.sim.tick
    );
    let hints = dir.join(format!("{stem}-hand.txt"));
    // (read as a hint file is: a plain file, capped)
    let mut old = crate::hints::read_text(&hints, crate::hints::MAX_FILE)?;
    if !old.is_empty() && !old.ends_with('\n') {
        old.push('\n');
    }
    old += &line;
    crate::hints::write_plain(&hints, &old)?;
    Ok(format!(
        "saved {} (hint in {})",
        shown_path(&file),
        shown_path(&hints)
    ))
}

/// A decision taking longer than this shows "thinking"; a notice shows this long.
const THINKING: std::time::Duration = std::time::Duration::from_millis(300);
const NOTICE: std::time::Duration = std::time::Duration::from_secs(8);
/// The ticks of a frame of the watched run are simulated for at most this long, so the window keeps answering.
const WATCH_BUDGET: std::time::Duration = std::time::Duration::from_millis(12);

/// The sizes the Graphics page offers: the monitor's modes and the window sizes that fit on it, smallest first.
fn sizes(window: &Window) -> Vec<(u32, u32)> {
    let monitor = window.current_monitor();
    let fits = monitor
        .as_ref()
        .map(|m| m.size())
        .map_or((u32::MAX, u32::MAX), |s| (s.width, s.height));
    let mut v: Vec<(u32, u32)> = monitor
        .iter()
        .flat_map(|m| m.video_modes())
        .map(|m| (m.size().width, m.size().height))
        .chain(
            crate::options::WINDOWED
                .into_iter()
                .filter(|s| s.0 <= fits.0 && s.1 <= fits.1),
        )
        .filter(|&(w, h)| crate::options::parse_size(&format!("{w}x{h}")).is_some())
        .collect();
    v.sort_by_key(|&(w, h)| (u64::from(w) * u64::from(h), w));
    v.dedup();
    v
}

/// The window over the whole screen or not. Exclusive takes the monitor's mode of `size` (its fastest), else its
/// largest; a platform that cannot (Wayland) gets borderless.
fn set_fullscreen(window: &Window, how: crate::options::Fullscreen, size: Option<(u32, u32)>) {
    use crate::options::Fullscreen as Full;
    let exclusive = || {
        let modes: Vec<_> = window.current_monitor()?.video_modes().collect();
        let area = |m: &winit::monitor::VideoModeHandle| {
            u64::from(m.size().width) * u64::from(m.size().height)
        };
        let wanted =
            |m: &&winit::monitor::VideoModeHandle| Some((m.size().width, m.size().height)) == size;
        let best = modes
            .iter()
            .filter(wanted)
            .max_by_key(|m| m.refresh_rate_millihertz())
            .or_else(|| {
                modes
                    .iter()
                    .max_by_key(|m| (area(m), m.refresh_rate_millihertz()))
            })?;
        Some(Fullscreen::Exclusive(best.clone()))
    };
    match how {
        Full::Off => {
            window.set_fullscreen(None);
            if let Some((w, h)) = size {
                let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(w, h));
            }
        }
        Full::Borderless => window.set_fullscreen(Some(Fullscreen::Borderless(None))),
        Full::Exclusive => {
            window.set_fullscreen(exclusive().or(Some(Fullscreen::Borderless(None))));
            if window.fullscreen().is_none() {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }
    }
}

/// The Graphics page's switches that the drawing reads: the clouds, and the field of view (the extended mode's).
fn show_graphics(g: &crate::options::Graphics, features: crate::features::Features) {
    scene::set_clouds(g.clouds);
    crate::clouds::set_quality(if g.clouds {
        g.cloud_quality
    } else {
        crate::clouds::Quality::Off
    });
    scene::set_fov((features.mode && g.fov != crate::options::FOV_ORIGINAL).then_some(g.fov));
}

impl App {
    /// The window's title: the game's name while the main menu is up (or no level is loaded), else the level's.
    fn title(&self) -> String {
        match &self.menu {
            Some(m) if !m.over_level() => "Drahtseilakt".into(),
            _ => self.game.title(),
        }
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let attrs = Window::default_attributes()
            .with_title(self.title())
            .with_inner_size(winit::dpi::PhysicalSize::new(self.size.0, self.size.1));
        let window = Arc::new(event_loop.create_window(attrs).map_err(|e| e.to_string())?);
        let Gpu {
            surface,
            config,
            renderer,
            modes,
        } = window_gpu(&window)?;
        eprintln!(
            "startup: window and graphics card ready {:.0} ms after launch",
            since_launch()
        );
        // what the menu needs goes up in slices while the splash is drawn (`App::startup_step`)
        self.startup = Some(Box::new(Startup::default()));
        if self.game.font.is_some() && std::env::var_os("DRAHTSEILAKT_NO_SPLASH").is_none() {
            self.splash = Some(Splash {
                began: Instant::now(),
                skipped: false,
                fading: None,
            });
        }
        let mut renderer = renderer;
        if self.splash.is_some() {
            renderer.prepare_title();
        }
        self.gpu = Some(Gpu {
            surface,
            config,
            renderer,
            modes,
        });
        self.resolutions = sizes(&window);
        set_fullscreen(&window, self.graphics.fullscreen, self.graphics.resolution);
        self.window = Some(window);
        self.last = Instant::now();
        Ok(())
    }

    /// The mods switched on (in the extended mode) run: the run's side from the next start, the view's at once.
    fn apply_mods(&mut self) {
        let (sim, view) = self.running_mods();
        self.game.set_mods(sim);
        self.view_mods = view;
        self.reset_mod_run();
        // without a practice mod nothing of the practice stays: the speed, the saved moment, the free camera
        if !self.practice() {
            self.time_scale = 1.0;
            self.saved = None;
            self.fly = None;
            self.history.clear();
        }
    }

    /// The mods that run: those switched on, in the extended mode, never while the test player plays (--watch).
    fn running_mods(&self) -> (crate::modrt::SimMods, crate::modrt::ViewMods) {
        self.mods
            .running(self.features.mode && self.pilot.is_none())
    }

    /// A new run for the mods: the view's variables as at first, the real time from now, no ghost yet.
    fn reset_mod_run(&mut self) {
        self.view_mods.reset();
        self.mod_began = Instant::now();
        self.ghost = None;
        self.ghost_tried = false;
        self.best_splits = None;
    }

    /// A value a mod reads: the run's from the Sim, the view's from the window.
    fn mod_value(&self, v: gamedata::mods::Value) -> f32 {
        use gamedata::mods::Value as V;
        let sim = &self.game.sim;
        let now = || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
        };
        match v {
            V::RealTime => self.mod_began.elapsed().as_secs_f32(),
            V::FrameDt => 1.0 / self.fps.max(1.0),
            V::Fps => self.fps,
            // the clock is UTC: the game knows no time zone (no locale data is read)
            V::ClockHour => ((now() / 3600) % 24) as f32,
            V::ClockMinute => ((now() / 60) % 60) as f32,
            V::ClockSecond => (now() % 60) as f32,
            V::PlaytimeLevel => self.playtimes.level(&self.game.level.name),
            V::PlaytimeAll => self.playtimes.total(),
            V::PracticeSpeed => self.time_scale,
            V::PracticeSaved => f32::from(u8::from(self.saved.is_some())),
            V::MouseDx => self.mouse.0,
            V::MouseDy => self.mouse.1,
            V::ExtrasPoints => sim.extras_left().0 as f32,
            V::ExtrasPointsAll => sim.extras_left().1 as f32,
            V::ExtrasLives => sim.extras_left().2 as f32,
            V::ExtrasLivesAll => sim.extras_left().3 as f32,
            V::BestTime => self
                .best_splits
                .as_ref()
                .and_then(|b| b.last())
                .map_or(-1.0, |&t| t as f32 / 60.0),
            _ => sim.mod_value(v, self.bindings.turn(self.keys | self.pad_keys)),
        }
    }

    /// The view's mods for this frame: their frame rules, what they ask done, the ghost kept up with the run.
    fn mod_frame(&mut self, step: f32) {
        if step > 0.0 {
            self.fps = self.fps * 0.9 + (1.0 / step) * 0.1;
        }
        if self.view_mods.is_empty() || self.menu.is_some() || self.loading.is_some() {
            self.mouse = (0.0, 0.0);
            return;
        }
        if self.best_splits.is_none() {
            self.best_splits = Some(self.game.best_splits());
        }
        let mut view = std::mem::take(&mut self.view_mods);
        view.out.lines.clear();
        view.out.splits.clear();
        let lang = self.strings.language();
        view.fire(
            gamedata::mods::Event::Frame,
            &|v| self.mod_value(v),
            &self.game.sim.mods,
            lang,
        );
        self.view_mods = view;
        self.mouse = (0.0, 0.0);
        self.game.view_turn = self.view_mods.out.turn.unwrap_or(0);
        if std::mem::take(&mut self.view_mods.out.playtime)
            && self.fly.is_none()
            && !self.game.sim.game_over
            && !self.game.sim.finished
        {
            self.playtimes.add(&self.game.level.name, step);
        }
        if self.playtimes.dirty && self.playtime_saved.elapsed().as_secs() >= 10 {
            self.playtime_saved = Instant::now();
            self.playtimes.save(&playtime_file());
        }
        if std::mem::take(&mut self.view_mods.out.restart) {
            self.restart();
            self.saved = None;
        }
        if std::mem::take(&mut self.view_mods.out.screenshot) {
            // at most one a second, however the key is held
            if self
                .last_shot
                .is_none_or(|t| t.elapsed().as_secs_f32() >= 1.0)
            {
                self.last_shot = Some(Instant::now());
                self.shot_due = true;
            }
        }
        self.keep_ghost();
    }

    /// A key the mods take (one of their key options): its rules fire; true when some mod took it.
    fn mod_key(&mut self, code: KeyCode) -> bool {
        if self.view_mods.is_empty() {
            return false;
        }
        // a key the player bound to the game's actions stays the game's (mod.txt already refuses the reserved ones)
        if self.bindings.bits(code) != 0 {
            return false;
        }
        let name = format!("{code:?}");
        let mut view = std::mem::take(&mut self.view_mods);
        let taken = view.runs.iter().any(|r| {
            r.txt.options.iter().zip(&r.opts).any(|(o, v)| {
                matches!(o.kind, gamedata::mods::OptKind::Key(_)) && o.show(*v) == name
            })
        });
        if taken {
            let lang = self.strings.language();
            view.press(&name, &|v| self.mod_value(v), &self.game.sim.mods, lang);
        }
        self.view_mods = view;
        taken
    }

    /// The ghost of the best run, played up to this run's tick (a few ticks a frame, so a long run catches up).
    fn keep_ghost(&mut self) {
        if self.view_mods.out.ghost <= 0.0 || self.game.placeholder || !self.game.counts_so_far() {
            self.ghost = None;
            return;
        }
        if self.ghost.is_none() && !self.ghost_tried {
            self.ghost_tried = true;
            self.ghost = self.game.ghost().map(Box::new);
        }
        let Some(g) = &mut self.ghost else { return };
        let want = usize::try_from(self.game.sim.tick).unwrap_or(usize::MAX);
        let mut steps = 0;
        while g.path.len() <= want && steps < 240 {
            let next = g.sim.tick + 1;
            g.sim.step(if next <= g.replay.end {
                g.replay.input_at(next)
            } else {
                0
            });
            g.path
                .push((g.sim.ball.position, g.sim.ball.rotation, g.sim.drawn_ball()));
            steps += 1;
        }
    }

    /// The ghost's ball at this run's tick, as a translucent instance.
    fn ghost_instance(&self) -> Option<crate::render::Instance> {
        let g = self.ghost.as_ref()?;
        let at = usize::try_from(self.game.sim.tick).ok()?;
        let &(pos, rot, kind) = g.path.get(at).or(g.path.last())?;
        let (mesh, _) = self.game.ball_meshes.of(kind?)?;
        let a = self.view_mods.out.ghost;
        Some(crate::render::Instance {
            model: scene::BALLS_MODEL,
            mesh,
            matrix: glam::Mat4::from_rotation_translation(rot, pos),
            marker: None,
            uv_offset: [0.0; 2],
            diffuse: Some([0.7, 0.85, 1.0, a]),
            unlit: false,
            shadow: None,
            swaps: Vec::new(),
        })
    }

    /// The mods' HUD lines and the splits, in their corners, in the interface's font.
    fn mod_hud(&self, f: &crate::ui::Face, screen: crate::ui::Screen) -> Vec<crate::ui::Quad> {
        use gamedata::mods::Corner;
        let out = &self.view_mods.out;
        let mut by: Vec<(Corner, String, [f32; 4])> = out
            .lines
            .iter()
            .map(|(c, t)| (*c, t.clone(), [1.0, 1.0, 1.0, 1.0]))
            .collect();
        for c in &out.splits {
            for (t, color) in self.split_lines() {
                by.push((*c, t, color));
            }
        }
        let mut q = Vec::new();
        for corner in [
            Corner::TopLeft,
            Corner::TopRight,
            Corner::BottomLeft,
            Corner::BottomRight,
        ] {
            let lines: Vec<&(Corner, String, [f32; 4])> =
                by.iter().filter(|l| l.0 == corner).collect();
            let right = matches!(corner, Corner::TopRight | Corner::BottomRight);
            let bottom = matches!(corner, Corner::BottomLeft | Corner::BottomRight);
            for (k, (_, text, color)) in lines.iter().enumerate() {
                let step = 0.034;
                // the top left keeps clear of the points counter, the bottom right of the lives
                let y = if bottom {
                    0.86 - step * (lines.len() - k) as f32
                } else {
                    0.02 + if right { 0.0 } else { 0.10 } + step * k as f32
                };
                let style = crate::ui::Style {
                    scale: [0.5, 0.55],
                    spacing: [1.0, 1.0],
                    align: (if right {
                        crate::ui::ALIGN_RIGHT
                    } else {
                        crate::ui::ALIGN_LEFT
                    }) | crate::ui::ALIGN_TOP,
                    flags: crate::ui::TEXT_SCREEN,
                    margins: [0.0; 4],
                    color: *color,
                    v2: true,
                    shadow: Some(([1.0, 1.0], [0.0, 0.0, 0.0, 0.8])),
                };
                let quads = f.layout(
                    text,
                    crate::ui::homogeneous([0.015, y, 0.985, y + step]),
                    &style,
                );
                let anchor = if right {
                    crate::ui::Anchor::Right
                } else {
                    crate::ui::Anchor::Left
                };
                q.extend(crate::ui::place(quads, screen, anchor));
            }
        }
        q
    }

    /// The splits: each sector reached (and the finish) with its time, and the difference to the best run's
    /// (green ahead, red behind); the sector being played with its time so far.
    fn split_lines(&self) -> Vec<(String, [f32; 4])> {
        let sim = &self.game.sim;
        let reached = crate::modrt::split_ticks(&sim.events);
        let best = self.best_splits.clone().unwrap_or_default();
        let sector = self.strings.ours("sector");
        let mut v = Vec::new();
        for (k, &t) in reached.iter().enumerate() {
            let mut line = format!("{sector} {}  {}", k + 1, clock(t));
            let mut color = [1.0, 1.0, 1.0, 1.0];
            if let Some(&b) = best.get(k) {
                let d = (t as f64 - b as f64) / 60.0;
                line.push_str(&format!(
                    "  {}{:.2}",
                    if d < 0.0 { "-" } else { "+" },
                    d.abs()
                ));
                color = if d <= 0.0 {
                    [0.5, 1.0, 0.5, 1.0]
                } else {
                    [1.0, 0.5, 0.45, 1.0]
                };
            }
            v.push((line, color));
        }
        if !sim.finished {
            v.push((
                format!("{sector} {}  {}", reached.len() + 1, clock(sim.tick)),
                [1.0, 0.85, 0.3, 1.0],
            ));
        }
        v
    }

    /// Saves the frame just drawn as a PNG in the data folder's screenshots/ (a mod's `screenshot`).
    fn save_screenshot(texture: &wgpu::Texture, gpu: &Gpu) {
        use wgpu::TextureFormat as F;
        // only the 8-bit formats are read back as they are (any other would need converting: said, not guessed)
        let bgra = match gpu.config.format {
            F::Bgra8Unorm | F::Bgra8UnormSrgb => true,
            F::Rgba8Unorm | F::Rgba8UnormSrgb => false,
            other => {
                return eprintln!(
                    "screenshot: this display's format ({other:?}) cannot be saved yet"
                );
            }
        };
        let (w, h) = (gpu.config.width, gpu.config.height);
        let mut pixels = match read_back(&gpu.renderer, texture, w, h) {
            Ok(p) => p,
            Err(e) => return eprintln!("screenshot: {e}"),
        };
        for px in pixels.as_chunks_mut::<4>().0 {
            if bgra {
                px.swap(0, 2);
            }
            // the surface's alpha means nothing in a picture
            px[3] = 255;
        }
        let dir = gamedata::profile::data_dir().join("screenshots");
        let _ = std::fs::create_dir_all(&dir);
        // a real folder, not a link to elsewhere
        if !std::fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir()) {
            return eprintln!("screenshot: {} is not a folder", shown_path(&dir));
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let path = dir.join(format!("drahtseilakt-{stamp}.png"));
        let mut open = std::fs::OpenOptions::new();
        // a new file only, never through a link
        open.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            open.custom_flags(libc_nofollow());
        }
        let written = open.open(&path).map_err(|e| e.to_string()).and_then(|f| {
            use image::ImageEncoder;
            image::codecs::png::PngEncoder::new(std::io::BufWriter::new(f))
                .write_image(&pixels, w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())
        });
        match written {
            Ok(()) => println!("screenshot saved to {}", shown_path(&path)),
            Err(e) => eprintln!(
                "could not save the screenshot to {}: {e}",
                shown_path(&path)
            ),
        }
    }

    /// The sounds of what happened in the run since the last frame, as the original's Sound_Manager plays them on
    /// the game's messages (Sound.nmo, Simple Sound Messages).
    fn sound_events(&mut self) {
        let Some(audio) = &mut self.audio else {
            self.game.sound_out.clear();
            if let Some(s) = &mut self.game.sim.scripts {
                s.sound.events.clear();
            }
            return;
        };
        self.script_voices
            .frame(audio, &self.data, &mut self.game.sim);
        for cmd in self.game.sound_out.drain(..) {
            match cmd {
                SoundCmd::Play(name, volume) => drop(audio.play(&self.data, name, volume, 0.0)),
                SoundCmd::Loop(slot, name) => {
                    if let Some(id) = self.rolling.get_mut(slot).and_then(Option::take) {
                        audio.stop(id);
                    }
                    if let Some(r) = self.rolling.get_mut(slot) {
                        *r = audio.play_loop(&self.data, name, 0.0);
                    }
                }
                SoundCmd::Stop(slot) => {
                    if let Some(id) = self.rolling.get_mut(slot).and_then(Option::take) {
                        audio.stop(id);
                    }
                }
                SoundCmd::Roll(volume, pitch) => {
                    for id in self.rolling.iter().take(3).flatten() {
                        audio.set_volume(*id, volume);
                        audio.set_pitch(*id, pitch);
                    }
                }
                SoundCmd::Volume(slot, volume) => {
                    if let Some(Some(id)) = self.rolling.get(slot) {
                        audio.set_volume(*id, volume);
                    }
                }
            }
        }
        let sim = &self.game.sim;
        if self.heard > sim.events.len() || (sim.tick <= 1 && self.heard > 0) {
            // a new run (or one taken back)
            self.heard = 0;
            self.started = sim.tick > START_SOUND_WITHIN;
        }
        if self.music.is_none() && sim.tick >= 1 {
            // a replay plays its music in the same order each time; a run of the player's, in a new one
            let now = || {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |d| d.as_nanos())
            };
            let replay = self.game.replay_in.as_ref();
            let seed =
                replay.map_or_else(|| now() as u64, |r| r.level_hash ^ r.end.rotate_left(32));
            let began = Instant::now();
            self.music = Some(Music::new(
                self.game.music_theme,
                self.game.last_level,
                seed,
                sim.tick,
            ));
            // the sounds' picks from the same seed, their pools' other files decoded ahead
            audio.seed_sounds(seed.rotate_left(29) ^ 0x5851_f42d_4c95_7f2d);
            audio.preload_sound_pools(&self.data);
            self.stall.note("music and sound pools", began.elapsed());
        }
        if let Some(m) = &mut self.music {
            // what may play next decoded ahead (our pieces are long), then the music's frame; the rest let go
            audio.preload(&self.data, &m.upcoming());
            m.frame(sim, audio, &self.data);
            audio.keep_music(&m.keep());
        }
        // the start sound once a run: a frame may take more than one tick (the first after a level loaded does), so
        // it is not looked for on tick 1 alone
        if !self.started && (1..=START_SOUND_WITHIN).contains(&sim.tick) {
            self.started = true;
            audio.play(&self.data, "Misc_StartLevel", 1.0, 0.0);
        }
        for (_, e) in sim.events.iter().skip(self.heard) {
            match e {
                Event::Checkpoint(_) => drop(audio.play(&self.data, "Misc_Checkpoint", 1.0, 0.0)),
                Event::ExtraLife => {
                    // Life_Up: the blob, and the extra ball's sound 317 ms later
                    audio.play(&self.data, "Extra_Life_Blob", 1.0, 0.0);
                    audio.play(&self.data, "Misc_extraball", 1.0, 0.317);
                }
                Event::ExtraPoint => drop(audio.play(&self.data, "Extra_Start", 1.0, 0.0)),
                Event::ExtraHits(_) => drop(audio.play(&self.data, "Extra_Hit", 1.0, 0.0)),
                Event::Transform(_) => drop(audio.play(&self.data, "Misc_Trafo", 1.0, 0.0)),
                Event::Died => drop(audio.play(&self.data, "Misc_Fall", 1.0, 0.0)),
                Event::NewBall => drop(audio.play(&self.data, "Misc_Lightning", 1.0, 0.0)),
                Event::RopeTears => drop(audio.play(&self.data, "Misc_RopeTears", 1.0, 0.0)),
                Event::GameOver | Event::Finished => {}
            }
        }
        self.heard = sim.events.len();
    }

    /// Practice: keeps the run as it is once a second (the last half minute) for F3, and the last ball when endless
    /// lives are on. A run that went back (its tick behind the moments kept) starts the history again.
    fn keep_history(&mut self) {
        let tools = self.practice() && self.game.replay_in.is_none();
        if self.game.sim.lives_spared {
            self.game.assisted = true;
        }
        if !tools {
            self.history.clear();
            return;
        }
        let tick = self.game.sim.tick;
        if self.history.back().is_some_and(|(s, _)| s.tick > tick) {
            self.history.clear();
        }
        if self
            .history
            .back()
            .is_none_or(|(s, _)| tick >= s.tick + HISTORY_EVERY)
        {
            self.history
                .push_back((self.game.sim.clone(), self.game.replay_out.clone()));
            if self.history.len() > HISTORY_KEPT {
                self.history.pop_front();
            }
        }
    }

    /// The run starts again: its music and its start sound too.
    fn restart(&mut self) {
        self.game.restart();
        self.reset_mod_run();
        self.started = false;
        self.prev = None;
        if let (Some(a), Some(m)) = (&mut self.audio, &mut self.music) {
            m.stop(a);
        }
        self.music = None;
    }

    /// Plays original level `n`: the current run starts again if it is that level, otherwise it is loaded.
    fn play(&mut self, n: u32) {
        self.menu = None;
        if self.number == Some(n) {
            self.restart();
        } else {
            self.load(look_source(n).unwrap_or(crate::assets::Source::Original(n)));
        }
    }

    /// Loads a level: the work of reading and building it goes to a thread, the frames go on (a loading bar shows
    /// when it takes over half a second), the card's share is done in slices between them.
    fn load(&mut self, source: crate::assets::Source) {
        self.menu = None;
        if let Some(a) = &mut self.audio {
            a.play(&self.data, "Menu_load", 1.0, 0.0);
        }
        let Some(gpu) = &self.gpu else { return };
        let (bc, plain) = (gpu.renderer.bc, gpu.renderer.look.plain);
        let uploader = crate::render::TextureUploader::new(&gpu.renderer);
        let number = match source {
            crate::assets::Source::Original(n) => Some(n),
            _ => None,
        };
        let mut o = self.opts.clone();
        o.source = source.clone();
        (o.replay, o.from, o.sector, o.at, o.ball) = (None, None, None, None, None);
        let data = self.data.for_source(&o.source);
        crate::loading::begin();
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::sync_channel(64);
        let (game_tx, game_rx) = std::sync::mpsc::channel();
        let worker = (o, data.clone(), cancel.clone());
        let spawned = std::thread::Builder::new()
            .name("loading".into())
            .stack_size(64 << 20)
            .spawn(move || {
                load_worker(
                    worker.0, worker.1, bc, plain, uploader, tx, game_tx, &worker.2,
                )
            });
        if let Err(e) = spawned {
            eprintln!("drahtseilakt: could not start the loading thread: {e}");
            self.open_menu(crate::menu::Screen::Main);
            return;
        }
        self.load_frames = Default::default();
        self.load_busy = Default::default();
        self.load_slow = 0;
        let (title, credit, mood) = loading_card(&source);
        self.loading = Some(Box::new(Load {
            title,
            credit,
            mood,
            source,
            data,
            number,
            plain,
            began: Instant::now(),
            cancel,
            up: crate::loading::Uploader::threaded(rx, true),
            game_rx,
            game: None,
            models_done: false,
            extra: 0,
            worst: Default::default(),
        }));
    }

    /// Esc while a level loads: back to the menu, what the thread makes dropped.
    fn cancel_loading(&mut self) {
        if let Some(l) = self.loading.take() {
            l.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            eprintln!(
                "loading: cancelled after {:.2} s",
                l.began.elapsed().as_secs_f64()
            );
            drop(l);
        }
        if let Some(gpu) = &mut self.gpu {
            gpu.renderer.end_sharing();
        }
        self.open_menu(crate::menu::Screen::Main);
    }

    /// A slice of the start-up's work on the card, a frame at a time: what the menu needs, so the splash goes on
    /// moving meanwhile. The textures are read and put up by a thread; the clouds and the like are made here while
    /// it works.
    fn startup_step(&mut self) {
        let Some(mut st) = self.startup.take() else {
            return;
        };
        let deadline = Instant::now() + LOAD_SLICE;
        let Some(gpu) = &mut self.gpu else { return };
        let r = &mut gpu.renderer;
        let mut last_phase = 99;
        loop {
            if last_phase != st.phase {
                eprintln!("startup: phase {} at {:.0} ms", st.phase, since_launch());
                last_phase = st.phase;
            }
            match st.phase {
                // the menu's scene read, the textures of its models and the placeholder game's handed to a thread
                0 => {
                    upload_prepare(r, &self.game);
                    r.plain_uploaded = r.look.plain;
                    if self.menu_scene.is_none() {
                        self.menu_scene = load_menu_scene(&self.data);
                    }
                    let models = level_models(&self.game, r.look.plain);
                    st.models = models.len();
                    let mut plans = std::collections::VecDeque::new();
                    let own = &self.game.data;
                    let own_loader = |t: &level::Texture, bc: bool| own.texture_plan(t, bc);
                    for (mi, l) in models.iter().enumerate() {
                        for (i, p) in crate::render::texture_plans(l, &own_loader, r.bc) {
                            plans.push_back((mi, i, p));
                        }
                    }
                    if let Some(ms) = &self.menu_scene {
                        let data = &self.data;
                        let loader = |t: &level::Texture, bc: bool| data.texture_plan(t, bc);
                        for (i, p) in crate::render::texture_plans(&ms.level, &loader, r.bc) {
                            plans.push_back((models.len(), i, p));
                        }
                    }
                    let boundary = Some(models.len());
                    let uploader = crate::render::TextureUploader::new(r);
                    let (tx, rx) = std::sync::mpsc::sync_channel(64);
                    let cancel = st.cancel.clone();
                    crate::loading::begin();
                    crate::loading::textures_planned(plans.len());
                    let spawned =
                        std::thread::Builder::new()
                            .name("startup".into())
                            .spawn(move || {
                                if let Some(up) =
                                    read_and_upload(plans, boundary, uploader, &tx, &cancel)
                                {
                                    let _ = tx.send(crate::loading::Item::Finished(up.stats));
                                }
                            });
                    if spawned.is_err() {
                        eprintln!("drahtseilakt: could not start the start-up thread");
                        return;
                    }
                    st.up = Some(crate::loading::Uploader::threaded(rx, true));
                    st.phase = 1;
                }
                // the interface's and the clouds' pieces, while the textures come
                1 => {
                    if upload_extras(r, &self.game, st.extra) {
                        st.extra += 1;
                    } else {
                        self.light_base = Some(r.light);
                        st.phase = 2;
                    }
                    if let Some(up) = st.up.as_mut() {
                        up.pump_textures(r, deadline);
                    }
                }
                // the models on the card
                2 => {
                    let Some(up) = st.up.as_mut() else { break };
                    if !up.textures_done() {
                        up.pump_textures(r, deadline);
                        if up.lost {
                            eprintln!("drahtseilakt: the start-up thread stopped");
                            return;
                        }
                    } else if !up.builds_made() {
                        let models = level_models(&self.game, r.look.plain);
                        let mut refs: Vec<&level::Level> = models.iter().map(|m| &**m).collect();
                        if let Some(ms) = &self.menu_scene {
                            refs.push(&ms.level);
                        }
                        up.make_builds(&refs);
                    } else if up.pump_models(r, deadline) {
                        let mut staged = std::mem::take(&mut up.staged);
                        let scene = (staged.len() > st.models).then(|| staged.pop()).flatten();
                        r.append_models(staged);
                        if let (Some(m), Some(ms)) = (scene, &mut self.menu_scene) {
                            ms.model = Some(r.append_models(vec![m]));
                        }
                        if let Some(stats) = up.stats {
                            r.texture_stats = stats;
                        }
                        models_uploaded(r, &self.game, up.started);
                        st.up = None;
                        st.phase = 3;
                    }
                }
                _ => {
                    eprintln!(
                        "startup: the menu is ready {:.0} ms after launch",
                        since_launch()
                    );
                    self.last = Instant::now();
                    return;
                }
            }
            if Instant::now() >= deadline {
                break;
            }
        }
        self.startup = Some(st);
    }

    /// The splash's course: it gives way once the start-up is done, it has stood long enough, or a key asked.
    fn splash_step(&mut self) {
        let ready = self.startup.is_none();
        let Some(sp) = &mut self.splash else { return };
        // DRAHTSEILAKT_SPLASH_HOLD=seconds keeps the title screen up that long (for taking its picture)
        let hold = std::env::var("DRAHTSEILAKT_SPLASH_HOLD")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .map_or(SPLASH_MIN, |v| {
                std::time::Duration::from_secs_f32(v.clamp(0.0, 600.0))
            });
        let stood = sp.began.elapsed() >= hold;
        match sp.fading {
            None if ready && (stood || sp.skipped) => sp.fading = Some(Instant::now()),
            Some(t) if t.elapsed() >= SPLASH_FADE => self.splash = None,
            _ => {}
        }
    }

    /// A slice of the loading's work on the main thread (the card's share), a few milliseconds; the level takes
    /// its place when it is all there.
    fn loading_step(&mut self) {
        let began = Instant::now();
        self.loading_slice();
        self.load_busy += began.elapsed();
        self.load_last_step = began.elapsed();
    }

    fn loading_slice(&mut self) {
        let Some(mut l) = self.loading.take() else {
            return;
        };
        let deadline = Instant::now() + LOAD_SLICE;
        let fail = |app: &mut App, e: String| {
            eprintln!("drahtseilakt: {e}");
            if let Some(gpu) = &mut app.gpu {
                gpu.renderer.end_sharing();
            }
            app.open_menu(crate::menu::Screen::Main);
        };
        let Some(gpu) = &mut self.gpu else {
            return fail(self, "no window to load into".into());
        };
        let r = &mut gpu.renderer;
        let mut first = true;
        loop {
            if l.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let step_began = Instant::now();
            let what = if !l.up.textures_done() {
                "textures"
            } else if !l.up.builds_made() {
                "making builds"
            } else if !l.models_done {
                "models"
            } else {
                "extras"
            };
            let kind = ["textures", "making builds", "models", "extras"]
                .iter()
                .position(|k| *k == what)
                .unwrap_or(0);
            // a step that would run past the slice (judged by the longest of its kind so far) waits for the next
            // frame; the first of a frame always runs
            if !first
                && step_began + std::time::Duration::from_micros(l.worst[kind].load(Relaxed))
                    > deadline
            {
                break;
            }
            first = false;
            let _slow = scopeguard_note(step_began, what, l.extra, l.worst.clone(), kind);
            if !l.up.textures_done() {
                l.up.pump_textures(r, deadline);
                if l.up.lost {
                    // the thread is gone: its error (if it sent one) is the reason
                    let why = match l.game_rx.try_recv() {
                        Ok(Err(e)) => e,
                        _ => "the loading thread stopped".into(),
                    };
                    return fail(self, why);
                }
                if l.up.textures_done() {
                    crate::loading::stage(crate::loading::Stage::Finishing);
                }
                break;
            }
            if l.game.is_none() {
                match l.game_rx.try_recv() {
                    Ok(Ok(g)) => l.game = Some(g),
                    Ok(Err(e)) => return fail(self, e),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(_) => return fail(self, "the loading thread stopped".into()),
                }
                continue;
            }
            let Some(game) = l.game.as_deref() else { break };
            if !l.up.builds_made() {
                let models = level_models(game, l.plain);
                let refs: Vec<&level::Level> = models.iter().map(|m| &**m).collect();
                l.up.make_builds(&refs);
            } else if !l.models_done {
                l.models_done = l.up.pump_models(r, deadline);
                if l.models_done {
                    if let Some(stats) = l.up.stats {
                        r.texture_stats = stats;
                    }
                    models_uploaded(r, game, l.up.started);
                }
            } else if upload_extras(r, game, l.extra) {
                l.extra += 1;
                crate::loading::finishing(700 + 300 * l.extra / (interface_textures().len() + 3));
            } else {
                return self.finish_loading(l);
            }
            if Instant::now() >= deadline {
                break;
            }
        }
        self.loading = Some(l);
    }

    /// The level is loaded and on the card: it takes the old one's place.
    fn finish_loading(&mut self, mut l: Box<Load>) {
        let Some(game) = l.game.take() else { return };
        self.game.save_attempt();
        let mut game = *game;
        game.start_session();
        self.game = game;
        self.data = l.data.clone();
        if let Some(a) = &mut self.audio {
            a.stop_all_fading_music(crate::audio::MUSIC_FADE_OUT);
            a.forget_sounds();
        }
        self.rolling = [None; 3 + crate::ballsound::FAN_SLOTS];
        self.heard = 0;
        self.started = false;
        self.music = None;
        self.number = l.number;
        self.prev = None;
        self.keys = 0;
        if let Some(gpu) = &mut self.gpu {
            let r = &mut gpu.renderer;
            // the light as the level sets it (a mod's dimming never carried into the next level)
            r.light.ambient = crate::render::Light::default().ambient;
            upload_prepare(r, &self.game);
            r.plain_uploaded = l.plain;
            let kept = self.menu_scene.as_ref().and_then(|m| m.model);
            let at = r.replace_models(std::mem::take(&mut l.up.staged), kept);
            if let Some(ms) = &mut self.menu_scene {
                ms.model = at;
            }
            self.light_base = Some(r.light);
        }
        if let Some(w) = &self.window {
            w.set_title(&self.title());
        }
        self.apply_mods();
        self.acc = 0.0;
        let took = l.began.elapsed();
        self.stall.note("level load", took);
        eprintln!(
            "loading: {:.2} s in all ({:.2} s of it the card's work here), {} frames drawn meanwhile ({} over 50 ms), the longest {:.0} ms",
            took.as_secs_f64(),
            self.load_busy.as_secs_f64(),
            self.load_frames.1,
            self.load_slow,
            self.load_frames.0.as_secs_f64() * 1000.0
        );
        eprintln!(
            "loading: frame stages, ms in all: {:?}",
            self.load_stages.map(|d| d.as_millis())
        );
    }

    /// Opens the menu with `base` as its first screen: the main menu, or the pause, dead or end menu over the level.
    fn open_menu(&mut self, base: crate::menu::Screen) {
        self.playtimes.save(&playtime_file());
        let mut menu = Menu::new(&self.data, &self.strings, base, self.number, self.features);
        menu.volumes = self.volumes;
        menu.graphics = self.graphics;
        menu.bindings = self.bindings;
        menu.resolutions = self.resolutions.clone();
        menu.mods = self.mods.clone();
        if let Some(gpu) = &self.gpu {
            menu.resolution = (gpu.config.width, gpu.config.height);
        }
        self.menu = Some(menu);
        self.keys = 0;
    }

    /// The Graphics page's choices applied (live where the platform lets them be) and saved.
    fn set_graphics(&mut self, g: crate::options::Graphics) {
        let old = std::mem::replace(&mut self.graphics, g);
        show_graphics(&g, self.features);
        if let Some(gpu) = &mut self.gpu
            && g.vsync != old.vsync
        {
            gpu.config.present_mode = present_mode(&gpu.modes, g.vsync);
            gpu.surface.configure(&gpu.renderer.device, &gpu.config);
        }
        if let Some(w) = &self.window
            && (g.fullscreen != old.fullscreen || g.resolution != old.resolution)
        {
            set_fullscreen(w, g.fullscreen, g.resolution);
        }
        if let Some(m) = &mut self.menu {
            m.graphics = g;
        }
        let lines: Vec<(String, String)> = g
            .lines()
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        put_settings(&lines);
    }

    /// A lost run opens the dead menu; a finished one, once its points are counted, the new highscore's name entry
    /// (an original level, begun at its start and not helped) or the end menu.
    /// The practice tools are on: the extended mode with its tools switched on.
    /// Records the replay as a video, without a window: one frame a tick (60 a second) at the window's size, as raw
    /// RGB into `out` (a file, or a pipe a video encoder reads), the sound mixed as in the window (into
    /// DRAHTSEILAKT_SOUND_FILE when set). It goes on six seconds past the finish, for the end screen.
    fn video(&mut self, out: &Path) -> Result<(), String> {
        use std::io::Write as _;
        let (w, h) = self.size;
        let (_, device, queue) = pollster::block_on(device(None))?;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut r = Renderer::new(device, queue, format);
        upload(&mut r, &self.game);
        let target = r.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("video"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let file = std::fs::File::create(out).map_err(|e| format!("{}: {e}", out.display()))?;
        let mut sink = std::io::BufWriter::with_capacity(1 << 22, file);
        let end = self
            .game
            .replay_in
            .as_ref()
            .map(|r| r.end)
            .ok_or("--video needs --replay")?;
        let lost = self.strings.ours("ball lost");
        let screen = crate::ui::Screen {
            w: w as f32,
            h: h as f32,
        };
        let mut frames = 0u64;
        loop {
            let sim = &self.game.sim;
            let past = sim.finished && sim.tick >= sim.finished_at + VIDEO_AFTER;
            if sim.game_over || past || (!sim.finished && sim.tick >= end) {
                break;
            }
            self.game.tick(0);
            if let Some(a) = &mut self.audio {
                a.pass(f64::from(TICK));
            }
            self.sound_events();
            let sim = &self.game.sim;
            let snap = Snapshot::of(sim);
            if self.game.last_level {
                let (flash, began) = self.blitz.step(sim.tick);
                r.flash = flash;
                if let (true, Some(a)) = (began, &mut self.audio) {
                    a.play(
                        &self.data,
                        "Music_thunder",
                        1.0,
                        crate::lights::THUNDER_DELAY,
                    );
                }
            }
            let sim = &self.game.sim;
            r.point = crate::lights::point(&sim.events, sim.tick, snap.ball);
            let g = &self.game;
            let mut inst = scene::instances(
                &g.placements,
                &sim.taken,
                &sim.unseen,
                &sim.world().bodies,
                &g.ball_meshes,
                g.trafo.as_ref().map(|t| &t.1),
                (&sim.extra_views(), g.extra_model.as_ref()),
                &snap,
                &sim.pieces_view(),
                sim.script_entities(),
            );
            if snap.kind.is_some() {
                scene::shadows(&g.shadow_receivers, snap.ball, &mut inst);
            }
            let mut particles = g.effects.batches(snap.eye, snap.target);
            if let Some(sc) = &sim.scripts {
                r.sync_materials(scene::LEVEL_MODEL, &sc.mats);
            }
            crate::script_text3d::add(sim.scripts.as_ref(), &mut particles, &mut r);
            r.apply_script_sky(
                scene::LEVEL_MODEL,
                sim.script_env().sky.as_ref(),
                snap.eye,
                &mut inst,
            );
            r.draw(&view, w, h, &scene::view(&snap), &inst, &particles);
            r.draw_overlay(
                &view,
                w,
                h,
                &crate::hud::overlay(sim, w, h, false, g.font.is_some()),
            );
            let font = g.font.as_ref();
            let mut quads = crate::hud::interface(sim, font, screen);
            quads.extend(crate::hud::ball_lost(
                sim,
                g.ball_lost_at,
                &lost,
                font,
                screen,
            ));
            if let (Some(t), Some(f)) = (crate::endscreen::of(sim), font) {
                quads.extend(crate::endscreen::interface(&t, f, &self.strings, screen));
            }
            if let Some((aw, ah, rgba)) = crate::fallback::take_changed() {
                r.add_named_texture(crate::fallback::TEXTURE, aw, ah, rgba);
            }
            r.draw_ui(&view, w, h, &quads);
            let px = read_back(&r, &target, w, h)?;
            let rgb: Vec<u8> = px
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            sink.write_all(&rgb)
                .map_err(|e| format!("{}: {e}", out.display()))?;
            frames += 1;
        }
        sink.flush()
            .map_err(|e| format!("{}: {e}", out.display()))?;
        // the sound file is written as the mix is let go
        self.audio = None;
        println!(
            "video: {frames} frames of {w}x{h} at 60 a second into {}",
            out.display()
        );
        Ok(())
    }

    /// One tick of the watched test player: its decision (on its own thread, waited for within this frame's
    /// budget), then the tick. False when the decision is not ready yet (the view holds) or the run is over.
    fn watch_tick(&mut self, started: Instant) -> bool {
        let Some(w) = self.pilot.as_mut() else {
            return false;
        };
        if !w.run.going(&self.game) {
            w.done = true;
            w.pending = None;
            if let Err(e) = w.run.summary(&self.game, &self.opts) {
                eprintln!("drahtseilakt: {e}");
            }
            return false;
        }
        if w.pending.is_none() {
            w.run.before(&self.game);
            let (tx, rx) = std::sync::mpsc::channel();
            let (mut p, sim) = (w.run.pilot.clone(), self.game.sim.clone());
            std::thread::spawn(move || {
                let i = p.next(&sim);
                let _ = tx.send((p, i));
            });
            w.pending = Some((rx, Instant::now()));
        }
        let Some((rx, _)) = &w.pending else {
            return false;
        };
        let (p, i) = match rx.recv_timeout(WATCH_BUDGET.saturating_sub(started.elapsed())) {
            Ok(r) => r,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return false,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                w.done = true;
                w.pending = None;
                eprintln!("drahtseilakt: the test player stopped");
                return false;
            }
        };
        w.pending = None;
        w.run.pilot = p;
        self.prev = Some(Snapshot::of(&self.game.sim));
        let tick = self.game.sim.tick;
        w.run.after(&mut self.game, &self.data, &self.opts, i);
        // gone back: the view jumps there, and says so
        if self.game.sim.tick <= tick {
            self.prev = None;
            if let Some(line) = &w.run.went_back {
                w.notice = Some((format!("BACK: {line}"), Instant::now()));
            }
        }
        true
    }

    /// The watcher's keys (--watch): Space pause, + and - faster and slower, F5 the hands take over and give back,
    /// F6 the hint's step skipped, F7 a stuck report. True when the key was one of them.
    fn watch_key(&mut self, code: KeyCode) -> bool {
        let Some(w) = self.pilot.as_mut() else {
            return false;
        };
        let now = Instant::now();
        match code {
            KeyCode::Space if w.hand.is_none() => w.paused = !w.paused,
            KeyCode::F1 => w.keys_shown = !w.keys_shown,
            KeyCode::Equal | KeyCode::NumpadAdd => w.speed = (w.speed * 2.0).min(8.0),
            KeyCode::Minus | KeyCode::NumpadSubtract => w.speed = (w.speed / 2.0).max(0.25),
            KeyCode::F5 => match w.hand.take() {
                None => {
                    // (a decision on its way is dropped: the player decides again when it has the ball back)
                    w.pending = None;
                    let s = &self.game.sim;
                    w.hand = Some((s.tick, s.ball.position, s.sector, s.ball.kind));
                    w.paused = false;
                    self.keys = 0;
                    w.notice = Some(("YOUR HANDS: the arrows drive, F5 hands back".into(), now));
                }
                Some(hand) => {
                    let text = match save_hand(&self.game, &self.opts, hand) {
                        Ok(t) => t,
                        Err(e) => format!("not saved: {e}"),
                    };
                    println!("{text}");
                    w.run.resume(&self.game, hand.0);
                    self.keys = 0;
                    w.notice = Some((text, now));
                }
            },
            KeyCode::F6 => {
                w.pending = None;
                w.run.pilot.hints.skip_step(&self.game.sim);
                w.notice = Some(("hint step skipped".into(), now));
            }
            // a zone: from the look-back before the key went down to where the ball is when it is let go (a tap: the
            // last seconds; held: on while held)
            KeyCode::F8 | KeyCode::F4 => {
                if w.marking.is_none() {
                    let kind = if code == KeyCode::F8 {
                        crate::zones::Kind::Flow
                    } else {
                        crate::zones::Kind::Careful
                    };
                    let s = &self.game.sim;
                    let from = s.tick.saturating_sub(u64::from(w.look_back) * 60);
                    w.marking = Some((kind, code, from, s.sector, now));
                }
            }
            KeyCode::BracketLeft => w.look_back = w.look_back.saturating_sub(1).max(1),
            KeyCode::BracketRight => w.look_back = (w.look_back + 1).min(LOOK_BACK_MAX),
            KeyCode::F12 => w.zones_shown = !w.zones_shown,
            KeyCode::Backspace => {
                let text = if w.marking.take().is_some() {
                    "mark cancelled".to_owned()
                } else {
                    w.flash = None;
                    // the last mark of this window, by its id (the files may hold other sessions' marks after it)
                    match w.marked.pop() {
                        None => "no mark of this window to undo".to_owned(),
                        Some((id, rule)) => {
                            let path = w.run.zones_path.clone();
                            let mut t = match crate::zones::remove_mark(&path, &id) {
                                Ok(Some(line)) => {
                                    let kind = line
                                        .split_whitespace()
                                        .next()
                                        .unwrap_or_default()
                                        .to_owned();
                                    format!(
                                        "undone: the {kind} zone just marked, in {}",
                                        shown_path(&path)
                                    )
                                }
                                Ok(None) => {
                                    format!("its zone is no longer in {}", shown_path(&path))
                                }
                                Err(e) => format!("not undone: {e}"),
                            };
                            w.run.pilot.zones = Arc::new(load_zones(&path));
                            w.pending = None;
                            // and the rule it taught
                            if rule {
                                let lp = w.run.learned_path.clone();
                                match crate::zones::remove_mark(&lp, &id) {
                                    Ok(Some(_)) => t += " and the rule it taught",
                                    Ok(None) => t += " (its rule is no longer there)",
                                    Err(e) => t += &format!(" (its rule not undone: {e})"),
                                }
                                w.run.pilot.learned = Arc::new(load_learned(&lp));
                            }
                            t
                        }
                    }
                };
                println!("{text}");
                w.notice = Some((text, now));
            }
            KeyCode::F7 => {
                let (pilot, dir) = (w.run.pilot.clone(), w.run.report.clone());
                let text = match stuck_report(&mut self.game, &self.data, &self.opts, &pilot, &dir)
                {
                    Ok(()) => format!("stuck report written in {}", shown_path(&dir)),
                    Err(e) => format!("stuck report: {e}"),
                };
                if let Some(w) = self.pilot.as_mut() {
                    w.notice = Some((text, now));
                }
            }
            _ => return false,
        }
        true
    }

    /// A watcher's key let go: the end of the zone being marked with it. True when it was that key.
    fn watch_key_up(&mut self, code: KeyCode) -> bool {
        if self
            .pilot
            .as_ref()
            .and_then(|w| w.marking)
            .is_some_and(|m| m.1 == code)
        {
            self.end_mark();
            return true;
        }
        false
    }

    /// The zone being marked is done: the ball's path from its look-back to now (in the sector it was marked in),
    /// written to the zones file at once, obeyed from now on, and shown a moment.
    fn end_mark(&mut self) {
        let Some(w) = self.pilot.as_mut() else { return };
        let Some((kind, _, from, sector, _)) = w.marking.take() else {
            return;
        };
        let s = &self.game.sim;
        let mut path: Vec<glam::Vec3> = w
            .trail
            .iter()
            .filter(|t| t.0 >= from && t.2 == sector)
            .map(|t| t.1)
            .collect();
        if s.sector == sector {
            path.push(s.ball.position);
        }
        let points = crate::zones::thin(&path, MARK_STEP);
        if points.is_empty() {
            w.notice = Some(("no path to mark (another sector)".into(), Instant::now()));
            return;
        }
        let zone = crate::zones::Zone {
            kind,
            sector: sector + 1,
            radius: crate::zones::MARK_RADIUS,
            speed: None,
            points,
        };
        let file = w.run.zones_path.clone();
        let secs = s.tick.saturating_sub(from) as f32 / 60.0;
        let id = crate::zones::new_id();
        let mut text = match crate::zones::append(&file, &zone, &id) {
            Ok(()) => {
                w.run.pilot.zones = Arc::new(load_zones(&file));
                // (a decision on its way is made again, with the zone)
                w.pending = None;
                format!(
                    "marked a {} zone: {secs:.1} s, {} points, in {}",
                    kind.word(),
                    zone.points.len(),
                    shown_path(&file)
                )
            }
            Err(e) => {
                let t = format!("zone not saved: {e}");
                println!("{t}");
                w.notice = Some((t, Instant::now()));
                return;
            }
        };
        // what kind of place it was, for every level: a rule of the learned table (learned.rs)
        let stretch: Vec<&Trail> = w
            .trail
            .iter()
            .filter(|t| t.0 >= from && t.2 == sector)
            .collect();
        let pilot = &w.run.pilot;
        let samples: Vec<crate::learned::Situation> = stretch
            .iter()
            .step_by(6)
            .map(|t| pilot.situation(t.1, pilot.nearest_point(t.1), t.3))
            .collect();
        let n = stretch.len().max(1) as f32;
        let stopping = stretch.iter().filter(|t| t.4 < 1.0).count() as f32 / n;
        let off: Vec<f32> = stretch
            .iter()
            .filter_map(|t| pilot.route.off_middle(t.1).map(|o| o.0))
            .collect();
        let off = off.iter().sum::<f32>() / off.len().max(1) as f32;
        let mut modes: Vec<(&str, usize)> = Vec::new();
        for t in &stretch {
            match modes.iter_mut().find(|m| m.0 == t.5) {
                Some(m) => m.1 += 1,
                None => modes.push((t.5, 1)),
            }
        }
        let mode = modes.iter().max_by_key(|m| m.1).map_or("-", |m| m.0);
        let at = zone.points.last().copied().unwrap_or_default();
        let evidence = format!(
            "mark={id}: {} sector {} ({:.1} {:.1} {:.1}) {}: {secs:.1} s, stopping {:.0}%, mostly {mode}, off the middle {off:.1}",
            level_stem(&self.game),
            sector + 1,
            at.x,
            at.y,
            at.z,
            crate::learned::today(),
            100.0 * stopping
        );
        let rule = crate::learned::learn(&samples, kind, None, evidence);
        let lp = w.run.learned_path.clone();
        let taught = match rule {
            Some(r) => match crate::learned::append(&lp, &r) {
                Ok(true) => {
                    w.run.pilot.learned = Arc::new(load_learned(&lp));
                    text += &format!("; learned: {}", r.label());
                    true
                }
                Ok(false) => {
                    text += &format!("; known already: {}", r.label());
                    false
                }
                Err(e) => {
                    text += &format!("; rule not saved: {e}");
                    false
                }
            },
            None => {
                text += "; nothing special to learn there (the zone stays this level's)";
                false
            }
        };
        w.marked.push((id, taught));
        println!("{text}");
        w.notice = Some((text, Instant::now()));
        w.flash = Some((zone, Instant::now()));
    }

    /// After a tick of the watched run: the ball's place kept in the trail (cut back when the run went back), and a
    /// zone whose key has been held too long ended.
    fn watch_trail(&mut self) {
        let Some(w) = self.pilot.as_mut() else { return };
        let s = &self.game.sim;
        while w.trail.back().is_some_and(|t| t.0 >= s.tick) {
            w.trail.pop_back();
        }
        w.trail.push_back((
            s.tick,
            s.ball.position,
            s.sector,
            s.ball.kind,
            s.ball.velocity.length(),
            w.run.pilot.mode(),
        ));
        while w.trail.front().is_some_and(|t| t.0 + TRAIL_TICKS < s.tick) {
            w.trail.pop_front();
        }
        if w.marking.is_some_and(|m| m.4.elapsed() > MARK_HELD_MAX) {
            self.end_mark();
        }
    }

    /// A mod with practice deeds is on (god-mode, assets/mods-dev): the game keeps the run's moments for going back,
    /// shows the sector and tick, and a lost run waits for the saved moment.
    fn practice(&self) -> bool {
        self.view_mods.practice()
    }

    /// The practice deeds the mods' keys asked for: back so many seconds, save and load the moment, slower and faster,
    /// the free camera. Each helps the run (it is kept, never a best one; its mods already make it modded).
    fn do_practice(&mut self) {
        use crate::modrt::Practice;
        for deed in std::mem::take(&mut self.view_mods.out.practice) {
            match deed {
                Practice::Rewind(secs) => {
                    // back from the moments kept each second; pressed again it goes on further back
                    let ticks = (secs.clamp(1.0, 30.0) * 60.0).round() as u64;
                    let to = self.game.sim.tick.saturating_sub(ticks);
                    while self.history.len() > 1
                        && self.history.back().is_some_and(|(s, _)| s.tick > to)
                    {
                        self.history.pop_back();
                    }
                    if let Some((sim, replay)) = self.history.back() {
                        self.game.ball_lost_at = self.game.ball_lost_at.filter(|&t| t <= sim.tick);
                        self.game.sim = sim.clone();
                        self.game.replay_out = replay.clone();
                        self.game.assisted = true;
                        self.prev = None;
                        self.menu = None;
                    }
                }
                Practice::Save => {
                    self.saved = Some((self.game.sim.clone(), self.game.replay_out.clone()))
                }
                Practice::Load => {
                    if let Some((sim, replay)) = &self.saved {
                        // the run goes on from the saved moment: its record is cut back to it, and what happened
                        // after it did not happen
                        self.game.sim = sim.clone();
                        self.game.replay_out = replay.clone();
                        self.game.assisted = true;
                        self.game.ball_lost_at = self.game.ball_lost_at.filter(|&t| t <= sim.tick);
                        self.prev = None;
                        self.menu = None;
                    }
                }
                Practice::Slower | Practice::Faster => {
                    let slower = deed == Practice::Slower;
                    self.time_scale = if slower {
                        (self.time_scale / 2.0).max(0.125)
                    } else {
                        (self.time_scale * 2.0).min(1.0)
                    };
                    if self.time_scale < 1.0 {
                        self.game.assisted = true;
                    }
                }
                Practice::Sector(n) => {
                    // the run begun again at that sector, as --sector does; the next ordinary restart is the level's
                    let was = self.game.start.take();
                    let top = self.game.sim.sector_count();
                    self.game.start = Some(crate::replay::Start {
                        sector: (n as usize).clamp(1, top.max(1)),
                        ball: None,
                        at: None,
                    });
                    self.restart();
                    self.game.start = was;
                    self.saved = None;
                    self.history.clear();
                }
                Practice::SpawnSet => self.game.pulse |= crate::sim::input::SPAWN_SET,
                Practice::SpawnGo => self.game.pulse |= crate::sim::input::SPAWN_GO,
                Practice::FreeCam => {
                    self.fly = match self.fly {
                        Some(_) => None,
                        None => Some(FreeCam::at(self.last_view.0, self.last_view.1)),
                    };
                    self.keys = 0;
                    self.fly_keys = 0;
                }
            }
        }
    }

    fn run_over(&mut self) {
        if self.menu.is_some() || self.loading.is_some() || self.fly.is_some() {
            return;
        }
        // with a saved moment (extended mode) a lost run waits for F9 or R; Esc opens the dead menu
        if self.game.sim.game_over && !(self.practice() && self.saved.is_some()) {
            self.open_menu(crate::menu::Screen::Dead);
        } else if crate::endscreen::over(&self.game.sim) {
            self.open_menu(crate::menu::Screen::End);
            let counts = self.game.counts();
            let points = crate::endscreen::score(&self.game.sim);
            if let (Some(m), Some(n), true) = (&mut self.menu, self.number, counts)
                && m.scores.place(n, points).is_some()
            {
                m.ask_name(self.player.clone(), points);
            }
        }
    }

    fn menu_key(&mut self, code: KeyCode, typed: Option<&str>, event_loop: &ActiveEventLoop) {
        // every key the menu takes clicks, as the original's menu sends Menu_Click
        if matches!(
            code,
            KeyCode::ArrowUp
                | KeyCode::ArrowDown
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::Space
                | KeyCode::Escape
        ) && !(matches!(
            code,
            KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::Space
        ) && self.menu.as_ref().is_some_and(|m| m.on_sound_theme()))
            && let Some(a) = &mut self.audio
        {
            // (a change of the sound theme clicks with the new theme, in menu_action)
            a.play(&self.data, "Menu_click", 1.0, 0.0);
        }
        let Some(menu) = &mut self.menu else { return };
        menu.notice = None;
        // a key awaited on the Controls page takes the key pressed
        if menu.waiting() {
            let action = menu.capture(code);
            self.menu_action(action, event_loop);
            return;
        }
        // the name entry takes what is typed
        if menu.screen() == crate::menu::Screen::Entry
            && !matches!(code, KeyCode::Enter | KeyCode::NumpadEnter)
        {
            match code {
                KeyCode::Backspace => menu.erase(),
                _ => menu.typed(typed.unwrap_or_default()),
            }
            return;
        }
        let action = match code {
            KeyCode::ArrowUp => {
                menu.step(-1);
                Action::None
            }
            KeyCode::ArrowDown => {
                menu.step(1);
                Action::None
            }
            KeyCode::ArrowLeft => menu.adjust(-1),
            KeyCode::ArrowRight => menu.adjust(1),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => menu.enter(),
            // Esc and Backspace leave any screen from any row
            KeyCode::Escape | KeyCode::Backspace => menu.back(),
            KeyCode::F1 => Action::Manual(menu.manual_page()),
            KeyCode::PageUp => {
                menu.page_by(-1);
                Action::None
            }
            KeyCode::PageDown => {
                menu.page_by(1);
                Action::None
            }
            KeyCode::Home => {
                menu.edge(-1);
                Action::None
            }
            KeyCode::End => {
                menu.edge(1);
                Action::None
            }
            _ => {
                menu.letter(typed.unwrap_or_default());
                Action::None
            }
        };
        self.menu_action(action, event_loop);
    }

    /// Does what a menu's key asked.
    fn menu_action(&mut self, action: Action, event_loop: &ActiveEventLoop) {
        match action {
            Action::None => {}
            Action::Continue => {
                self.menu = None;
                if std::mem::take(&mut self.pause_told) {
                    self.game.sim.notify_pause(false);
                }
            }
            Action::Play(n) => self.play(n),
            Action::Map(path) if path.is_dir() => self.load(crate::assets::Source::Open(path)),
            Action::Map(path) => self.load(crate::assets::Source::Nmo(path)),
            Action::Restart => {
                self.menu = None;
                self.restart();
                self.saved = None;
            }
            Action::Home => self.open_menu(crate::menu::Screen::Main),
            Action::Quit => event_loop.exit(),
            Action::Manual(page) => {
                // the local manual, only because the player asked (manual.rs); a failure is said in the menu
                if let Err(e) = crate::manual::open(&self.strings, page)
                    && let Some(m) = &mut self.menu
                {
                    m.notice = Some(e);
                }
            }
            Action::Volumes(v) => {
                self.volumes = v;
                if let Some(a) = &mut self.audio {
                    a.set_volumes(v[0], v[1]);
                }
                save_settings(v, self.features, &self.player);
            }
            Action::Interface(p) => {
                gamedata::resources::set_chosen(p);
                save_settings(self.volumes, self.features, &self.player);
            }
            Action::Pack(kind, key) => {
                // the next level loads with it
                gamedata::packs::choose(kind, &key);
                put_settings(&[(kind.key().to_owned(), key)]);
            }
            Action::Look(look) => {
                // the next level loads with it
                let kept = setting("textures");
                crate::look::choose(look, kept.as_deref());
                put_settings(&crate::look::settings(look));
            }
            Action::Language(code) => {
                // spoken at once: the menus' words again, in the new language
                put_settings(&[("language".to_owned(), code)]);
                self.strings = load_strings(&self.data);
                if let Some(m) = &mut self.menu {
                    m.strings = self.strings.clone();
                }
            }
            Action::SoundTheme(key) => {
                // the next sounds and pieces come from the new theme
                gamedata::sound_themes::choose(&key);
                if let Some(a) = &mut self.audio {
                    a.forget_sounds();
                }
                // the preview: the click plays with the theme just chosen
                self.menu_click();
                save_settings(self.volumes, self.features, &self.player);
            }
            Action::ToggleDebug | Action::Switch(_) => {
                match action {
                    Action::Switch(f) => self.features.toggle(f),
                    _ => self.features.mode = !self.features.mode,
                }
                save_settings(self.volumes, self.features, &self.player);
                // the field of view is the extended mode's: the classic one plays at the original's
                show_graphics(&load_graphics(), self.features);
                if let Some(m) = &mut self.menu {
                    m.set_features(self.features, &self.data);
                }
                self.apply_mods();
            }
            Action::Graphics(g) => self.set_graphics(g),
            Action::Bindings(b) => {
                self.bindings = b;
                put_settings(&b.lines());
            }
            Action::ModSwitch(_) | Action::ModOption(..) => {
                match action {
                    Action::ModSwitch(i) => self.mods.toggle(i),
                    Action::ModOption(i, o, by) => self.mods.step_option(i, o, by),
                    _ => {}
                }
                put_settings(&self.mods.lines());
                // the view's side at once, the run's from the next start
                let (sim, view) = self.running_mods();
                self.game.set_mods(sim);
                let out = std::mem::take(&mut self.view_mods.out);
                self.view_mods = view;
                self.view_mods.out = crate::modrt::ViewOut {
                    lines: Vec::new(),
                    splits: Vec::new(),
                    look: Default::default(),
                    ..out
                };
                if let Some(m) = &mut self.menu {
                    m.mods = self.mods.clone();
                }
            }
            Action::Score(name) => {
                self.player = name.clone();
                save_settings(self.volumes, self.features, &self.player);
                if let Some(m) = &mut self.menu {
                    let mut scores = m.scores.clone();
                    scores.insert(m.level, &name, m.points);
                    scores.save();
                    m.show_scores(scores);
                }
            }
        }
    }

    fn key(
        &mut self,
        code: KeyCode,
        down: bool,
        typed: Option<&str>,
        event_loop: &ActiveEventLoop,
    ) {
        if let Some(sp) = &mut self.splash {
            // any key asks for the menu; the splash keeps the keys until it is gone
            if down {
                sp.skipped = true;
            }
            return;
        }
        if self.loading.is_some() {
            if down && code == KeyCode::Escape {
                self.cancel_loading();
            }
            return;
        }
        if self.menu.is_some() {
            if down {
                self.key_repeat.press(code);
                self.menu_key(code, typed, event_loop);
            } else {
                self.key_repeat.release(code);
            }
            return;
        }
        self.key_repeat.clear();
        // a key one of the mods' key options names (modrt.rs) is the mod's, also while the free camera flies (its
        // own key puts it down)
        if down && self.pilot.is_none() && self.mod_key(code) {
            self.do_practice();
            return;
        }
        if self.fly.is_some() && code != KeyCode::Escape && code != KeyCode::F11 {
            use crate::freecam::keys;
            let bit = match code {
                KeyCode::ArrowUp => keys::FORWARD,
                KeyCode::ArrowDown => keys::BACK,
                KeyCode::ArrowLeft => keys::TURN_LEFT,
                KeyCode::ArrowRight => keys::TURN_RIGHT,
                KeyCode::KeyA => keys::LEFT,
                KeyCode::KeyD => keys::RIGHT,
                KeyCode::PageUp => keys::UP,
                KeyCode::PageDown => keys::DOWN,
                KeyCode::Home => keys::LOOK_UP,
                KeyCode::End => keys::LOOK_DOWN,
                KeyCode::ShiftLeft | KeyCode::ShiftRight => keys::FAST,
                _ => 0,
            };
            if down {
                self.fly_keys |= bit;
            } else {
                self.fly_keys &= !bit;
            }
            return;
        }
        if self.pilot.as_ref().is_some_and(|w| !w.done) && down && self.watch_key(code) {
            return;
        }
        if !down && self.watch_key_up(code) {
            return;
        }
        // the keys bound to the game's actions (the Controls page) come before the practice keys
        let bound = self.bindings.bits(code);
        let bit = match code {
            _ if bound != 0 => bound,
            // the run waits under the pause menu; a finished or lost run has its own menu
            KeyCode::Escape if down && !self.game.sim.finished => {
                let lost = self.game.sim.game_over;
                if !lost && self.menu.is_none() {
                    self.game.sim.notify_pause(true);
                    self.pause_told = true;
                }
                self.open_menu(if lost {
                    crate::menu::Screen::Dead
                } else {
                    crate::menu::Screen::Pause
                });
                return;
            }
            // the full screen on or off, as the Graphics page sets it
            KeyCode::F11 if down => {
                use crate::options::Fullscreen as Full;
                let on = self
                    .window
                    .as_ref()
                    .is_some_and(|w| w.fullscreen().is_some());
                let how = if on { Full::Off } else { Full::Borderless };
                self.set_graphics(crate::options::Graphics {
                    fullscreen: how,
                    ..self.graphics
                });
                0
            }
            _ => 0,
        };
        if down {
            self.keys |= bit;
        } else {
            self.keys &= !bit;
        }
    }

    /// One frame, timed: one that takes over 100 ms of work is logged with what happened in it (the session log
    /// and stderr), so a hang in play can be found afterwards.
    fn frame(&mut self) {
        let began = Instant::now();
        self.stall.notes.clear();
        self.stall.slept = std::time::Duration::ZERO;
        self.stall.slowest = (std::time::Duration::ZERO, 0);
        self.stall.stages = Default::default();
        self.stall.sync = std::time::Duration::ZERO;
        let before = (
            self.game.sim.lives,
            self.game.sim.sector,
            self.game.sim.game_over,
            self.game.sim.finished,
            self.game.sim.tick,
            self.gpu.as_ref().map_or(0, |g| g.renderer.pipeline_count()),
            self.music.is_some(),
        );
        let loading = self.loading.is_some();
        self.frame_inner();
        self.frames += 1;
        if self.frames == 1 {
            eprintln!(
                "startup: first frame drawn {:.0} ms after launch",
                since_launch()
            );
        }
        if self.frames.is_multiple_of(15)
            && let Some(w) = &self.window
        {
            let t = self.title();
            if t != self.shown_title {
                w.set_title(&t);
                self.shown_title = t;
            }
        }
        let took = began.elapsed().saturating_sub(self.stall.slept);
        if loading {
            for (a, b) in self.load_stages.iter_mut().zip(self.stall.stages) {
                *a += b;
            }
            if took.as_millis() > 50 {
                self.load_slow += 1;
            }
            self.load_frames.0 = self.load_frames.0.max(took);
            self.load_frames.1 += 1;
        }
        if took < STALL {
            return;
        }
        let sim = &self.game.sim;
        let mut what = std::mem::take(&mut self.stall.notes);
        if sim.lives != before.0 || sim.game_over != before.2 {
            what.push(format!(
                "death or respawn (lives {} -> {})",
                before.0, sim.lives
            ));
        }
        if sim.sector != before.1 {
            what.push(format!("sector {} -> {}", before.1 + 1, sim.sector + 1));
        }
        if sim.finished && !before.3 {
            what.push("finish".into());
        }
        let pipelines = self.gpu.as_ref().map_or(0, |g| g.renderer.pipeline_count());
        if pipelines != before.5 {
            what.push(format!(
                "{} pipelines created",
                pipelines.saturating_sub(before.5)
            ));
        }
        if self.music.is_some() && !before.6 {
            what.push("music started".into());
        }
        let st = &self.stall.stages;
        let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
        let line = format!(
            "stall: frame {:.0} ms at tick {} (ticks {:.0} [slowest {:.0} at tick {}, scripts {}], acquire {:.0}, instances {:.0}, draw {:.0} [material sync {:.0}], overlay {:.0}, present {:.0}){}{}",
            ms(took),
            sim.tick,
            ms(st[0]),
            ms(self.stall.slowest.0),
            self.stall.slowest.1,
            if sim.scripts.is_some() { "on" } else { "off" },
            ms(st[1]),
            ms(st[2]),
            ms(st[3]),
            ms(self.stall.sync),
            ms(st[4]),
            ms(st[5]),
            if what.is_empty() { "" } else { "; " },
            what.join(", ")
        );
        eprintln!("{line}");
        self.game.log_line(&line);
    }

    fn frame_inner(&mut self) {
        // Immediate presenting does not wait for the monitor, so the frames are capped here at about 60 a second.
        let since = self.last.elapsed();
        if since < FRAME {
            std::thread::sleep(FRAME - since);
            self.stall.slept += FRAME - since;
        }
        let now = Instant::now();
        // this frame's real step (after the cap's sleep), held to a quarter second after a stall
        let step = (now - self.last).as_secs_f32().min(0.25);
        let scale = match &self.pilot {
            Some(w) if w.paused => 0.0,
            Some(w) => w.speed,
            None => self.time_scale,
        };
        self.acc = (self.acc + (now - self.last).as_secs_f32() * scale).min(0.25);
        self.last = now;
        // a debugging hook: DRAHTSEILAKT_AUTOLOAD=<level folder> loads that level once the menu is up (loading-bar tests)
        if !self.autoloaded
            && self.startup.is_none()
            && self.splash.is_none()
            && let Some(p) = std::env::var_os("DRAHTSEILAKT_AUTOLOAD").filter(|p| !p.is_empty())
        {
            self.autoloaded = true;
            self.menu = None;
            self.load(crate::assets::Source::Open(PathBuf::from(p)));
        }
        self.startup_step();
        self.splash_step();
        // a level being loaded gets a slice of the card's work each frame; the run waits
        self.loading_step();
        if let Some(c) = &mut self.fly {
            // the run waits while the free camera is up
            c.fly(self.fly_keys, self.acc);
            self.acc = 0.0;
        }
        if self.menu.is_some() || self.loading.is_some() {
            self.acc = 0.0;
        }
        let started = Instant::now();
        let mut ticks = 0u32;
        while self.acc >= TICK {
            // the test player plays (--watch), unless the watcher drives by hand
            if self
                .pilot
                .as_ref()
                .is_some_and(|w| !w.done && w.hand.is_none())
            {
                if !self.watch_tick(started) {
                    self.acc = self.acc.min(TICK);
                    break;
                }
                self.watch_trail();
                self.acc -= TICK;
                ticks += 1;
                continue;
            }
            self.prev = Some(Snapshot::of(&self.game.sim));
            // the scripts' keyboard and mouse, as they are at this tick
            let size = self
                .gpu
                .as_ref()
                .map_or((0, 0), |g| (g.config.width, g.config.height));
            self.game.live_input =
                crate::dik::frame_of(&self.held, self.cursor, size, self.mouse_down);
            // the gamepads press the same keys; the camera's turn inverted as chosen
            let tick_began = Instant::now();
            self.game
                .tick(self.bindings.turn(self.keys | self.pad_keys));
            let tick_took = tick_began.elapsed();
            if tick_took > self.stall.slowest.0 {
                self.stall.slowest = (tick_took, self.game.sim.tick);
            }
            if let Some(w) = self.pilot.as_mut().filter(|w| !w.done) {
                w.run.near_edges.tick(&w.run.edges, &self.game.sim.ball);
                self.watch_trail();
            }
            self.keep_history();
            self.acc -= TICK;
            ticks += 1;
        }
        let simulated = started.elapsed();
        self.run_over();
        self.mod_frame(step);
        if let Some(a) = &mut self.audio {
            a.pass(f64::from(ticks) * f64::from(TICK));
        }
        self.sound_events();
        let cur = Snapshot::of(&self.game.sim);
        let mut snap = Snapshot::lerp(&self.prev.unwrap_or(cur), &cur, self.acc / TICK);
        // the passing lights: the last level's lightning and its thunder, and the new ball's light
        let sim = &self.game.sim;
        let mut flash = 0.0;
        if self.game.last_level {
            let began;
            (flash, began) = self.blitz.step(sim.tick);
            if let (true, Some(a)) = (began, &mut self.audio) {
                a.play(
                    &self.data,
                    "Music_thunder",
                    1.0,
                    crate::lights::THUNDER_DELAY,
                );
            }
        }
        let point = crate::lights::point(&sim.events, sim.tick, snap.ball);
        // a mod's `camera.smooth`: how fast the camera follows what it is asked (a time constant of up to ~0.65 s)
        let ease = {
            let m = self.view_mods.out.smooth;
            if m > 0.0 && self.menu.is_none() {
                1.0 - (-step / (0.05 + m * 0.6)).exp()
            } else {
                1.0
            }
        };
        if let Some(c) = &self.fly {
            (snap.eye, snap.target) = c.view();
            if ease < 1.0 {
                if let Some((e, t)) = self.eased.as_mut().and_then(|x| x.1.as_mut()) {
                    *e += (snap.eye - *e) * ease;
                    *t += (snap.target - *t) * ease;
                    (snap.eye, snap.target) = (*e, *t);
                } else {
                    self.eased.get_or_insert(([0.0, 1.0, 0.0], None)).1 =
                        Some((snap.eye, snap.target));
                }
            }
        } else if let Some(x) = self.eased.as_mut() {
            x.1 = None;
        }
        self.last_view = (snap.eye, snap.target);
        let tools = self.practice();
        let ghost = self.ghost_instance();
        // a mod's `render.*`: the picture's look; drawn at its own (low) size, then scaled onto the window
        let look = self.view_mods.out.look;
        self.game.show_supports(look.supports);
        let (rw, rh) = self
            .gpu
            .as_ref()
            .map_or((1, 1), |g| look.size(g.config.width, g.config.height));
        let mod_quads = match (&self.gpu, self.game.font.as_ref()) {
            (Some(g), Some(f))
                if self.menu.is_none() && self.loading.is_none() && !self.view_mods.is_empty() =>
            {
                self.mod_hud(
                    f,
                    crate::ui::Screen {
                        w: rw as f32,
                        h: rh as f32,
                    },
                )
            }
            _ => Vec::new(),
        };
        let Some(gpu) = &mut self.gpu else { return };
        let acquire_started = Instant::now();
        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => {
                let began = Instant::now();
                gpu.surface.configure(&gpu.renderer.device, &gpu.config);
                self.stall.note("swapchain reconfigure", began.elapsed());
                return;
            }
        };
        let acquired = acquire_started.elapsed();
        gpu.renderer.look = look;
        if look.plain != gpu.renderer.plain_uploaded {
            // the Back to 2004 look went on or off: the level is drawn again from its other files
            gpu.renderer.models.clear();
            upload(&mut gpu.renderer, &self.game);
            upload_menu_scene(&mut gpu.renderer, self.menu_scene.as_mut(), &self.data);
        }
        let window_view = frame.texture.create_view(&Default::default());
        let view = if look.post() {
            let format = gpu.renderer.format;
            let device = gpu.renderer.device.clone();
            let retro = gpu
                .renderer
                .retro
                .get_or_insert_with(|| crate::retro::Retro::new(&device, format));
            retro.target(&device, format, rw, rh)
        } else {
            window_view.clone()
        };
        let build_started = Instant::now();
        // With a menu up, the original's menu scene is behind it, its camera going round
        let menu_up = self.menu.as_ref().is_some_and(|m| !m.over_level()) || self.loading.is_some();
        let menu_scene = self
            .menu_scene
            .as_mut()
            .filter(|m| m.model.is_some() && menu_up);
        gpu.renderer.menu_frame = menu_scene.is_some();
        let (inst, mut particles, frame_view) = if let Some(ms) = menu_scene {
            self.menu_ms += f64::from(step) * 1000.0;
            ms.effects.tick_menu(step * 1000.0);
            ms.frame(self.menu_ms)
        } else {
            let mut inst = scene::instances(
                &self.game.placements,
                &self.game.sim.taken,
                &self.game.sim.unseen,
                &self.game.sim.world().bodies,
                &self.game.ball_meshes,
                self.game.trafo.as_ref().map(|t| &t.1),
                (&self.game.sim.extra_views(), self.game.extra_model.as_ref()),
                &snap,
                &self.game.sim.pieces_view(),
                self.game.sim.script_entities(),
            );
            if snap.kind.is_some() {
                scene::shadows(&self.game.shadow_receivers, snap.ball, &mut inst);
            }
            if let Some(w) = self.pilot.as_ref().filter(|w| !w.done) {
                w.zone_instances(&self.game, &mut inst);
            }
            inst.extend(ghost);
            // a mod's view-only looks: the wooden ball deformed, decoy balls around it (never touch the run)
            let (deform, decoys) = (self.view_mods.out.deform, self.view_mods.out.decoys);
            if deform > 0.0 || decoys > 0 {
                let t = self.mod_began.elapsed().as_secs_f32();
                crate::scene::ball_looks(
                    &mut inst,
                    &snap,
                    &self.game.ball_meshes,
                    t,
                    deform,
                    decoys,
                );
            }
            let particles = self.game.effects.batches(snap.eye, snap.target);
            let mut view = scene::view(&snap);
            // the mods' camera (modrt.rs): turned about the ball, nearer or farther, a wider or narrower view
            let out = &self.view_mods.out;
            if self.fly.is_none() && !self.view_mods.is_empty() {
                let goal = [out.yaw, out.distance, out.fov];
                let e = self.eased.get_or_insert((goal, None));
                if ease < 1.0 {
                    let turn = (goal[0] - e.0[0] + 540.0).rem_euclid(360.0) - 180.0;
                    e.0[0] = (e.0[0] + turn * ease).rem_euclid(360.0);
                    e.0[1] += (goal[1] - e.0[1]) * ease;
                    e.0[2] += (goal[2] - e.0[2]) * ease;
                } else {
                    e.0 = goal;
                }
                let [yaw, distance, fov] = e.0;
                let back = glam::Quat::from_rotation_y(yaw.to_radians()) * (view.eye - view.target);
                view.eye = view.target + back * distance;
                let across = 2.0 * ((view.fov_y / 2.0).tan() / 0.75).atan() + fov.to_radians();
                view.fov_y = 2.0 * ((across.clamp(0.2, 3.0) / 2.0).tan() * 0.75).atan();
            }
            (inst, particles, view)
        };
        let built = build_started.elapsed();
        let draw_started = Instant::now();
        (gpu.renderer.flash, gpu.renderer.point) =
            if menu_up { (0.0, None) } else { (flash, point) };
        // the mods' light: the level's dimmed, a light over the ball
        // what the map's scripts changed of the look for this run: the game light's colour and the fog
        let env = if menu_up {
            crate::scripting::EnvOverride::default()
        } else {
            self.game.sim.script_env()
        };
        gpu.renderer.script_fog = env.fog;
        let mut inst = inst;
        gpu.renderer
            .apply_script_sky(scene::LEVEL_MODEL, env.sky.as_ref(), snap.eye, &mut inst);
        if let Some(base) = self.light_base {
            let k = if menu_up {
                1.0
            } else {
                self.view_mods.out.light
            };
            gpu.renderer.light.color = env.ingame_light().unwrap_or(base.color).map(|c| c * k);
            gpu.renderer.light.ambient = base.ambient.map(|c| c * k);
        }
        let lantern = self.view_mods.out.lantern;
        if !menu_up && lantern > 0.0 && gpu.renderer.point.is_none() {
            gpu.renderer.point = Some((snap.ball + glam::Vec3::Y * 4.0, [lantern; 3]));
        }
        if let Some(sc) = &self.game.sim.scripts {
            let began = Instant::now();
            gpu.renderer.sync_materials(scene::LEVEL_MODEL, &sc.mats);
            self.stall.sync = began.elapsed();
        }
        crate::script_text3d::add(
            self.game.sim.scripts.as_ref(),
            &mut particles,
            &mut gpu.renderer,
        );
        // the loading screen's own picture covers the window once it has come up: the scene behind it is not drawn
        // (it would cost a third of each frame while the card is busy taking the level's textures)
        let covered = self.loading.as_ref().is_some_and(|l| {
            l.began.elapsed() > LOAD_SCREEN_AFTER + std::time::Duration::from_millis(300)
        });
        if !covered {
            gpu.renderer
                .draw(&view, rw, rh, &frame_view, &inst, &particles);
        }
        let drawn = draw_started.elapsed();
        let overlay_started = Instant::now();
        gpu.renderer.draw_overlay(
            &view,
            rw,
            rh,
            &match (&self.menu, &self.loading) {
                // with the game's font the menus are the original's (drawn below), else the prototype's
                (_, Some(_)) | (Some(_), None) if self.game.font.is_some() => Vec::new(),
                (_, Some(l)) => {
                    if l.shown() {
                        Menu::loading_overlay(rw, rh, &loading_name(&l.source))
                    } else {
                        Vec::new()
                    }
                }
                (Some(m), None) => m.overlay(rw, rh),
                (None, None) => {
                    let mut o = crate::hud::overlay(
                        &self.game.sim,
                        rw,
                        rh,
                        tools,
                        self.game.font.is_some(),
                    );
                    if tools && (self.time_scale < 1.0 || self.saved.is_some()) {
                        let scale = (rh as f32 / 240.0).round().max(2.0);
                        let slow = if self.time_scale < 1.0 {
                            format!("SLOW 1/{}", (1.0 / self.time_scale).round())
                        } else {
                            String::new()
                        };
                        let saved = if self.saved.is_some() { "   SAVED" } else { "" };
                        crate::hud::text(
                            &mut o,
                            &format!("{slow}{saved}"),
                            scale,
                            scale * 21.0,
                            scale,
                            [0.6, 0.9, 1.0, 1.0],
                        );
                    }
                    if let Some(w) = self
                        .pilot
                        .as_ref()
                        .filter(|w| !w.done && self.game.font.is_none())
                    {
                        let scale = (rh as f32 / 360.0).round().max(1.0);
                        for (k, line) in w.status(&self.game).lines().enumerate() {
                            let y = scale * (21.0 + 9.0 * k as f32);
                            crate::hud::text(&mut o, line, scale, y, scale, [0.7, 1.0, 0.7, 1.0]);
                        }
                    }
                    if let Some(c) = &self.fly {
                        let scale = (rh as f32 / 240.0).round().max(2.0);
                        let line =
                            format!("FREE CAMERA   {:.1} {:.1} {:.1}", c.pos.x, c.pos.y, c.pos.z);
                        crate::hud::text(
                            &mut o,
                            &line,
                            scale,
                            scale * 11.0,
                            scale,
                            [1.0, 0.85, 0.3, 1.0],
                        );
                    }
                    o
                }
            },
        );
        let screen = crate::ui::Screen {
            w: rw as f32,
            h: rh as f32,
        };
        let font = self.game.font.as_ref();
        // the words of the title and loading screens go over their picture, the rest of the interface under it
        let mut art_quads = Vec::new();
        let quads = match (&self.menu, &self.loading, font) {
            (_, Some(l), Some(_)) => {
                if l.shown() {
                    art_quads = self.art.loading(
                        screen,
                        &l.title,
                        l.credit.as_deref(),
                        &crate::loading::text(&self.strings),
                        l.began.elapsed().as_secs_f32() - LOAD_SCREEN_AFTER.as_secs_f32(),
                    );
                }
                Vec::new()
            }
            (Some(m), None, Some(_)) => {
                // over a level its points and lives stay in view, as the original's pause shows them
                let mut q = Vec::new();
                if m.over_level() {
                    q = crate::hud::interface(&self.game.sim, font, screen);
                }
                q.extend(m.interface(font, screen));
                q
            }
            (None, None, _) => {
                let mut q = crate::hud::interface(&self.game.sim, font, screen);
                let lost = self.strings.ours("ball lost");
                q.extend(crate::hud::ball_lost(
                    &self.game.sim,
                    self.game.ball_lost_at,
                    &lost,
                    font,
                    screen,
                ));
                if let (Some(t), Some(f)) = (crate::endscreen::of(&self.game.sim), font) {
                    q.extend(crate::endscreen::interface(&t, f, &self.strings, screen));
                }
                q.extend(mod_quads);
                // the watched test player's status, small in the top left corner (--watch)
                if let (Some(w), Some(f)) = (self.pilot.as_ref().filter(|w| !w.done), font) {
                    q.extend(watch_overlay(&w.status(&self.game), f, screen));
                    if w.keys_shown {
                        q.extend(w.keys_box(&self.strings, f, screen));
                    }
                }
                q
            }
            _ => Vec::new(),
        };
        // the title and loading screens: their picture over the frame, then their words
        let shot = match (&self.splash, &self.loading, font) {
            (Some(sp), _, Some(_)) => {
                let t = sp.began.elapsed().as_secs_f32();
                art_quads = self.art.splash(screen, t, sp.fade());
                Some(crate::titlescene::Shot {
                    loading: false,
                    time: t,
                    progress: 0.0,
                    alpha: sp.fade(),
                    mood: crate::titlescene::DUSK,
                })
            }
            (None, Some(l), Some(_)) => {
                let t = l.began.elapsed().as_secs_f32() - LOAD_SCREEN_AFTER.as_secs_f32();
                let progress = self.art.progress(crate::loading::fraction(), t < 0.0);
                l.shown().then_some(crate::titlescene::Shot {
                    loading: true,
                    time: t,
                    progress,
                    alpha: (t / 0.25).clamp(0.0, 1.0),
                    mood: l.mood,
                })
            }
            _ => None,
        };
        if let Some((w, h, rgba)) = crate::fallback::take_changed() {
            gpu.renderer
                .add_named_texture(crate::fallback::TEXTURE, w, h, rgba);
        }
        for (name, w, h, rgba) in self.art.uploads() {
            gpu.renderer.add_named_texture(name, w, h, rgba);
        }
        // the words wait for their picture (its pipeline is made on a thread at the start)
        gpu.renderer
            .draw_screen(&view, (rw, rh), &quads, shot.as_ref(), &art_quads);
        if look.post() {
            if let Some(retro) = &gpu.renderer.retro {
                let place = look.placed(gpu.config.width, gpu.config.height);
                retro.present(
                    &gpu.renderer.device,
                    &gpu.renderer.queue,
                    &window_view,
                    place,
                    look.colour_bits == 16,
                );
            }
        }
        let overlaid = overlay_started.elapsed();
        let present_started = Instant::now();
        if std::mem::take(&mut self.shot_due) {
            if gpu.config.usage.contains(wgpu::TextureUsages::COPY_SRC) {
                Self::save_screenshot(&frame.texture, gpu);
            } else {
                eprintln!("screenshot: this display's surface cannot be read back");
            }
        }
        gpu.renderer.queue.present(frame);
        let presented = present_started.elapsed();
        self.stall.stages = [simulated, acquired, built, drawn, overlaid, presented];
        if let Some(t) = &mut self.timing {
            t.stages[0] += acquired;
            t.stages[1] += built;
            t.stages[2] += drawn;
            t.stages[3] += overlaid;
            t.stages[4] += presented;
            t.frames += 1;
            t.ticks += u64::from(ticks);
            t.simulate += simulated;
            t.frame += started.elapsed();
            if t.since.elapsed().as_secs() >= 2 {
                let per_tick = t.simulate.as_secs_f64() * 1000.0 / t.ticks.max(1) as f64;
                let per_frame = t.frame.as_secs_f64() * 1000.0 / t.frames.max(1) as f64;
                eprintln!(
                    "timing: {:.1} frames/s, {:.2} ticks per frame, tick {:.2} ms, frame {:.2} ms (of which the ticks {:.2} ms)",
                    t.frames as f64 / t.since.elapsed().as_secs_f64(),
                    t.ticks as f64 / t.frames.max(1) as f64,
                    per_tick,
                    per_frame,
                    t.simulate.as_secs_f64() * 1000.0 / t.frames.max(1) as f64,
                );
                let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0 / t.frames.max(1) as f64;
                eprintln!(
                    "timing stages per frame: acquire {:.2} ms, instances {:.2} ms, draw {:.2} ms, overlay {:.2} ms, present {:.2} ms",
                    ms(t.stages[0]),
                    ms(t.stages[1]),
                    ms(t.stages[2]),
                    ms(t.stages[3]),
                    ms(t.stages[4]),
                );
                *t = Timing::default();
            }
        }
        // a mod's `render.fps_cap`: the picture's frames at most so many a second (the run's ticks keep their clock)
        if look.fps_cap > 0 {
            let frame = std::time::Duration::from_secs_f64(1.0 / f64::from(look.fps_cap));
            if let Some(rest) = frame.checked_sub(started.elapsed()) {
                std::thread::sleep(rest);
                self.stall.slept += rest;
            }
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

impl App {
    /// The gamepads: the keys they hold, for the run; their buttons, as the menus' keys (and Start as Esc).
    fn gamepads(&mut self, event_loop: &ActiveEventLoop) {
        use crate::options::PadPress;
        let (keys, pressed) = self.pads.poll();
        // the run's keys only while it runs, as the keyboard's
        if keys == 0 {
            self.pad_held = false;
        }
        let running = self.menu.is_none() && self.loading.is_none() && self.fly.is_none();
        self.pad_keys = if running && !self.pad_held { keys } else { 0 };
        for p in pressed {
            let code = match p {
                PadPress::Up => KeyCode::ArrowUp,
                PadPress::Down => KeyCode::ArrowDown,
                PadPress::Left => KeyCode::ArrowLeft,
                PadPress::Right => KeyCode::ArrowRight,
                PadPress::Enter => KeyCode::Enter,
                PadPress::Back | PadPress::Start => KeyCode::Escape,
            };
            // in a run only Start (the pause) counts: the other buttons are its keys
            let menu = self.menu.as_ref().is_some_and(|m| !m.waiting());
            if menu || (self.menu.is_none() && p == PadPress::Start) {
                self.key(code, true, None, event_loop);
                self.key(code, false, None, event_loop);
                // the button that chose may close the menu: what it holds is not the run's
                self.pad_held = keys != 0;
            }
        }
    }
}

/// Pixels of a touchpad's scrolling that make one notch of the wheel.
const WHEEL_PIXELS: f64 = 40.0;

/// The mouse in the menus (menu.rs: hover, click, wheel). The run takes nothing from the mouse, so a click that
/// closes a menu has nothing to leak into it.
impl App {
    /// Where the cursor is over the menu, as the menu lays its buttons out in this window.
    fn pointer(&self) -> Option<crate::menu::Pointer> {
        let (x, y) = self.cursor?;
        let gpu = self.gpu.as_ref()?;
        Some(crate::menu::Pointer {
            interface: self.game.font.is_some(),
            screen: crate::ui::Screen {
                w: gpu.config.width as f32,
                h: gpu.config.height as f32,
            },
            x: x as f32,
            y: y as f32,
        })
    }

    /// The menu's click sound, as its keys make it.
    fn menu_click(&mut self) {
        if let Some(a) = &mut self.audio {
            a.play(&self.data, "Menu_click", 1.0, 0.0);
        }
    }

    fn mouse_moved(&mut self) {
        let Some(p) = self.pointer() else { return };
        if self.loading.is_none() && self.menu.as_mut().is_some_and(|m| m.hover(p)) {
            self.menu_click();
        }
    }

    /// Left presses the button under the cursor; right and the mouse's back button are Esc.
    fn mouse_button(&mut self, button: MouseButton, event_loop: &ActiveEventLoop) {
        if let Some(sp) = &mut self.splash {
            sp.skipped = true;
            return;
        }
        if self.menu.is_none() || self.loading.is_some() {
            return;
        }
        match button {
            MouseButton::Left => {
                let Some(p) = self.pointer() else { return };
                if let Some(action) = self.menu.as_mut().and_then(|m| m.click(p)) {
                    if !matches!(action, Action::SoundTheme(_)) {
                        self.menu_click();
                    }
                    self.menu_action(action, event_loop);
                }
            }
            MouseButton::Right | MouseButton::Back => {
                self.menu_key(KeyCode::Escape, None, event_loop)
            }
            _ => {}
        }
    }

    /// Each whole notch of the wheel (up scrolls up) is one step of the menu's wheel.
    fn mouse_wheel(&mut self, delta: MouseScrollDelta, event_loop: &ActiveEventLoop) {
        if self.menu.is_none() || self.loading.is_some() {
            self.wheel = 0.0;
            return;
        }
        self.wheel += match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y),
            MouseScrollDelta::PixelDelta(p) => p.y / WHEEL_PIXELS,
        };
        while self.wheel.abs() >= 1.0 {
            let by = if self.wheel > 0.0 { -1 } else { 1 };
            self.wheel += f64::from(by);
            let p = self.pointer();
            if let Some(action) = self.menu.as_mut().map(|m| m.wheel(by, p)) {
                self.menu_action(action, event_loop);
            }
        }
    }

    /// The cursor shown over the menus and hidden in the run (and while a level loads); never confined.
    fn show_cursor(&mut self) {
        // a map's script may ask for the cursor in the run (Show Mouse Cursor): the window's cursor only
        let shown = (self.menu.is_some() && self.loading.is_none())
            || (self.menu.is_none() && self.game.sim.script_cursor == Some(true));
        if self.cursor_shown != Some(shown)
            && let Some(w) = &self.window
        {
            w.set_cursor_visible(shown);
            self.cursor_shown = Some(shown);
        }
    }
}

impl ApplicationHandler for App {
    /// The mouse's raw motion: the mods read it (`mouse.dx`, `mouse.dy`) while a run is up.
    fn device_event(
        &mut self,
        _: &ActiveEventLoop,
        _: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event
            && self.menu.is_none()
        {
            self.mouse.0 += delta.0 as f32;
            self.mouse.1 += delta.1 as f32;
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(e) = self.init(event_loop)
        {
            self.error = Some(e);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    // a window larger than the surface the device allows (a 10K monitor) draws at the largest one
                    gpu.config.width = size.width.clamp(1, crate::options::SURFACE_MAX);
                    gpu.config.height = size.height.clamp(1, crate::options::SURFACE_MAX);
                    gpu.surface.configure(&gpu.renderer.device, &gpu.config);
                }
                if let Some(m) = &mut self.menu {
                    m.resolution = (size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat: false,
                        text,
                        ..
                    },
                ..
            } => {
                // A key already down pressed again is a repeat the platform did not mark (X11 under load): ignored.
                let down = state == ElementState::Pressed;
                if down && !self.held.insert(code) {
                    return;
                }
                if !down {
                    self.held.remove(&code);
                }
                self.key(code, down, text.as_deref(), event_loop)
            }
            WindowEvent::RedrawRequested => {
                // a held arrow or page key in the menu repeats
                let dt = self.repeat_at.elapsed().as_secs_f32().min(0.25);
                self.repeat_at = Instant::now();
                if self.menu.is_some()
                    && let Some((code, n)) = self.key_repeat.advance(dt)
                {
                    for _ in 0..n {
                        self.menu_key(code, None, event_loop);
                    }
                }
                self.frame()
            }
            // the mouse in the menus: the cursor chooses (only when it moves), a click presses, the wheel scrolls
            WindowEvent::CursorMoved { position, .. } => {
                let at = Some((position.x, position.y));
                if at != self.cursor {
                    self.cursor = at;
                    self.mouse_moved();
                }
            }
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                self.mouse_down |= crate::dik::button_bit(button);
                self.mouse_button(button, event_loop)
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } => self.mouse_down &= !crate::dik::button_bit(button),
            WindowEvent::MouseWheel { delta, .. } => self.mouse_wheel(delta, event_loop),
            // a key let go while the window was not ours may never be reported released
            WindowEvent::Focused(false) => self.held.clear(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.gamepads(event_loop);
        self.show_cursor();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod runs_tests {
    use super::*;

    #[test]
    fn runs_move_once_from_the_old_place_and_never_over_new_ones() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-runs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (old, new) = (
            root.join("maps/Folder"),
            root.join("levels/community/Folder"),
        );
        std::fs::create_dir_all(old.join("Level_07")).unwrap();
        std::fs::write(old.join("Level_07/run-1.replay"), "r").unwrap();
        assert_eq!(move_runs(&old, &new, "Level_07"), Ok(true));
        assert!(new.join("Level_07/run-1.replay").is_file());
        assert!(!old.exists(), "the emptied old folder goes");
        // nothing to move any more
        assert_eq!(move_runs(&old, &new, "Level_07"), Ok(false));
        // runs in both places: the new ones stay, the old ones are left where they are
        std::fs::create_dir_all(old.join("Level_07")).unwrap();
        std::fs::write(old.join("Level_07/run-2.replay"), "o").unwrap();
        assert_eq!(move_runs(&old, &new, "Level_07"), Ok(false));
        assert!(
            old.join("Level_07/run-2.replay").is_file()
                && !new.join("Level_07/run-2.replay").exists()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn runs_behind_a_link_are_not_moved() {
        let root =
            std::env::temp_dir().join(format!("drahtseilakt-runs-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("elsewhere/Level_07")).unwrap();
        std::fs::create_dir_all(root.join("maps")).unwrap();
        std::os::unix::fs::symlink(root.join("elsewhere"), root.join("maps/Folder")).unwrap();
        assert_eq!(
            move_runs(
                &root.join("maps/Folder"),
                &root.join("levels/c/Folder"),
                "Level_07"
            ),
            Ok(false)
        );
        assert!(root.join("elsewhere/Level_07").is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_loose_map_and_a_level_folder_of_one_name_keep_their_runs_apart() {
        let community = crate::levels::dir(crate::levels::Kind::Community);
        let (loose, _) = runs_root(&crate::assets::Source::Nmo(community.join("Foo.nmo")));
        let (folder, old) = runs_root(&crate::assets::Source::Nmo(
            community.join("Foo").join("Foo.nmo"),
        ));
        // the loose map's runs are in <loose>/Foo, the folder's in <folder>/Foo: neither inside the other
        assert!(
            !folder.join("Foo").starts_with(loose.join("Foo"))
                && !loose.join("Foo").starts_with(&folder)
        );
        assert_eq!(old, Some(replay_dir().join("maps").join("Foo")));
        // a level folder named exactly like a kind's folder (wherever it is moved) shared its old place: not moved
        let kind_name = community.file_name().unwrap().to_owned();
        let (_, none) = runs_root(&crate::assets::Source::Nmo(
            community.join(&kind_name).join("x.nmo"),
        ));
        assert_eq!(none, None);
        let (_, some) = runs_root(&crate::assets::Source::Nmo(
            community.join("Community-like").join("x.nmo"),
        ));
        assert!(some.is_some());
        assert_eq!(shown("a\u{1b}[2J"), "a\\u{1b}[2J");
    }
}

#[cfg(test)]
mod follow_tests {
    use super::failed_tries_out;
    use glam::Vec3;

    /// A run that lost its ball in sector 2 and then got through it: the lost try's stretch in sector 2 goes, the
    /// way through sector 1 and the try that got through stay, and the transformers move with their points.
    #[test]
    fn a_lost_try_is_left_out_of_the_way_followed() {
        let path: Vec<Vec3> = (0..10).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        // sector 1 (0..3), the try lost in sector 2 (3..6), the new ball's try through sector 2 (6..10)
        let sectors = vec![0, 0, 0, 1, 1, 1, 1, 1, 1, 1];
        let gates = vec![
            (4, Vec3::ZERO, crate::sim::BallKind::Paper),
            (7, Vec3::X, crate::sim::BallKind::Stone),
        ];
        let (p, s, g, d) = failed_tries_out(path, sectors, gates, vec![6], true);
        assert_eq!(
            p.iter().map(|v| v.x as i32).collect::<Vec<_>>(),
            vec![0, 1, 2, 6, 7, 8, 9]
        );
        assert_eq!(s.len(), 7);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].0, 4, "the transformer at old 7 is now at 4");
        assert!(d.is_empty());
        // unfinished, the tries in its last sector are kept for way_on to choose from
        let path: Vec<Vec3> = (0..6).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let (p, _, _, d) =
            failed_tries_out(path, vec![0, 0, 1, 1, 1, 1], Vec::new(), vec![4], false);
        assert_eq!(p.len(), 6);
        assert_eq!(d, vec![4]);
    }
}

/// For the level statistics (levelstats.rs): the level and its modules as a run loads them, and the ball's way
/// through it — a replay (`o.replay`) played without a window, tick by tick, or else the way the test player would
/// find through the objects of a plan (`plan`), with no run behind it.
pub(crate) fn stats_run(
    o: &Options,
    data: &Data,
    plan: Option<&Path>,
) -> Result<crate::levelstats::Run, String> {
    use crate::levelstats::{Run, TickRec};
    let mut game = Game::new(o, data)?;
    let mut run = Run {
        level: game.level.level.clone(),
        modules: game.modules.clone(),
        ticks: Vec::new(),
        events: Vec::new(),
        moved: Vec::new(),
        finished: false,
        begun_elsewhere: game
            .start
            .as_ref()
            .is_some_and(|s| s.sector > 1 || s.at.is_some()),
        route: None,
        checkpoints: game.sim.stages(),
    };
    if let Some(plan) = plan {
        let start = game.sim.ball.position;
        let mut stages: Vec<glam::Vec3> = read_plan(plan, &game.level.level, 0)?
            .into_iter()
            .map(|p| p.0)
            .collect();
        if let Some(&end) = run.checkpoints.last() {
            stages.push(end);
        }
        let route = crate::autopilot::Route::find(
            &game.level.level,
            &game.modules,
            &game.sim.updrafts(),
            start,
            &stages,
        );
        run.route = Some(route.points.clone());
        return Ok(run);
    }
    let end = game
        .replay_in
        .as_ref()
        .map(|r| r.end)
        .ok_or("level statistics need a replay or a plan")?;
    // each loose thing's first place and how far it got from it (sampled twice a second)
    let mut loose: std::collections::HashMap<usize, (glam::Vec3, f32)> =
        std::collections::HashMap::new();
    let mut seen = 0;
    while game.sim.tick < end && !game.sim.finished && !game.sim.game_over {
        game.tick(0);
        let s = &game.sim;
        run.ticks.push(TickRec {
            pos: s.ball.position,
            ground: s.ball.on_ground,
            sector: s.sector + 1,
            kind: s.ball.kind,
            updraft: s.in_updraft(),
            held: s.is_held(),
            dying: s.is_dying(),
        });
        for e in s.events.iter().skip(seen) {
            run.events.push((e.0, e.1, s.sector + 1));
        }
        seen = s.events.len();
        if s.tick % 30 == 1 {
            for &(obj, body) in s.loose() {
                if let Some(st) = s.world().state(body) {
                    let e = loose.entry(obj).or_insert((st.pos, 0.0));
                    e.1 = e.1.max(st.pos.distance(e.0));
                }
            }
        }
    }
    run.finished = game.sim.finished;
    let mut moved: Vec<(usize, f32)> = loose.into_iter().map(|(o, (_, d))| (o, d)).collect();
    moved.sort_by_key(|m| m.0);
    run.moved = moved
        .into_iter()
        .filter_map(|(o, d)| Some((game.level.level.objects.get(o)?.name.clone(), d)))
        .collect();
    Ok(run)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod placeholder_tests {
    use super::*;

    #[test]
    fn the_menu_game_holds_no_level_and_is_named_for_the_game() {
        let o = crate::default_options();
        let started = Instant::now();
        let g = Game::placeholder(&o, &Data::new(None)).unwrap();
        assert!(g.placeholder);
        assert_eq!(g.title(), "Drahtseilakt");
        assert!(g.placements.is_empty(), "nothing of a level is built");
        assert!(g.level.level.objects.is_empty());
        assert!(started.elapsed().as_secs() < 5, "it costs next to nothing");
        // no session log is opened for it
        let mut g = g;
        g.start_session();
        assert!(g.session.is_none());
        // a stall line has nowhere to go without a session, and does not fail
        g.log_line("stall: test");
    }

    #[test]
    fn a_stall_note_keeps_only_what_took_time() {
        let mut s = Stall::default();
        s.note("quick", std::time::Duration::from_millis(1));
        s.note("slow", std::time::Duration::from_millis(250));
        assert_eq!(s.notes, vec!["slow 250 ms".to_owned()]);
    }
}

/// Notes (stderr) a step of the loading that held a frame for long: dropped when the step ends.
struct SlowStep(Instant, &'static str, usize, Arc<[AtomicU64; 4]>, usize);

impl Drop for SlowStep {
    fn drop(&mut self) {
        let took = self.0.elapsed();
        if self.1 != "textures" {
            self.3[self.4].fetch_max(took.as_micros() as u64, Relaxed);
        }
        if took.as_millis() >= 40 {
            eprintln!(
                "loading: slow step: {} {} took {:.0} ms",
                self.1,
                self.2,
                took.as_secs_f64() * 1000.0
            );
        }
    }
}

fn scopeguard_note(
    began: Instant,
    what: &'static str,
    n: usize,
    worst: Arc<[AtomicU64; 4]>,
    kind: usize,
) -> SlowStep {
    SlowStep(began, what, n, worst, kind)
}
