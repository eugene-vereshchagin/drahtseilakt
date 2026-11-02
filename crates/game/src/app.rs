// Running the game: in a window (keyboard or a replay drives the simulation), or without one (replay checks and
// screenshots). The simulation always advances in fixed 60 Hz ticks; the window draws at the display's rate and
// interpolates between the last two ticks.

use crate::Options;
use crate::assets::{Data, LoadedLevel};
pub use gamedata::profile::setting;
use gamedata::profile::{settings_path, settings_text};
use crate::render::Renderer;
use crate::replay::Replay;
use crate::audio::Audio;
use crate::ballsound::{BallSound, FanSound, SoundCmd};
use crate::freecam::FreeCam;
use crate::music::Music;
use crate::menu::{Action, Menu};
use crate::scene::{self, BallMeshes, Snapshot};
use crate::collide::closest_point;
use crate::sim::{BALL_RADIUS, Event, Sim, TICK};
use crate::watch::{Finding, Watch};
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
    level: LoadedLevel,
    start_ball: crate::sim::BallKind,
    balls: LoadedLevel,
    ball_meshes: BallMeshes,
    /// The balls' pieces, for each new run's bursts.
    pieces: crate::pieces::Pieces,
    placements: Vec<scene::Placement>,
    modules: scene::Modules,
    /// The transformer animation's file (`AnimTrafo.nmo`), drawn after the modules, if the copy has it.
    trafo: Option<(LoadedLevel, crate::trafo::TrafoModel)>,
    sim: Sim,
    replay_in: Option<Replay>,
    replay_out: Replay,
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
    /// The run was helped (slowed down, or taken back to a saved moment): it is kept, but never the best.
    assisted: bool,
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
        .or_else(|| at.and_then(|p| crate::sim::sector_near(level, glam::Vec3::from(p))).map(|n| n + 1))
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
        let mut level = data.load(&o.source)?;
        scene::cloud_material(&mut level.level);
        for w in &level.warnings {
            eprintln!("warning: {w}");
        }
        // the shared parts (balls, modules, transformer): the level folder's own when it has them, else the copy's
        let balls = data
            .part("3D Entities/Balls.nmo")
            .ok_or("Balls.nmo not found or unreadable in the game data")?;
        let replay_in = match &o.replay {
            Some(p) => {
                let text =
                    String::from_utf8(crate::hints::read_plain(p, crate::hints::MAX_REPLAY_FILE)?)
                        .map_err(|_| format!("{}: not UTF-8", p.display()))?;
                let r = Replay::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
                // a replay plays on its level file, or on a level converted from that file (an open-format export
                // records its source's hash): the same level, so the same run
                if r.level_hash != level.hash && Some(r.level_hash) != level.level.settings.source_hash {
                    return Err(format!(
                        "{}: recorded on another level file than {}",
                        p.display(),
                        level.name
                    ));
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
        let mut sim = Sim::with_modules(&level.level, start_ball, &modules, tutorial);
        sim.set_ball_boxes(ball_meshes.boxes);
        let pieces = crate::pieces::Pieces::find(&balls.level);
        sim.set_pieces(pieces.clone());
        // a replay recorded before the bursts plays without them, as it was recorded
        sim.bursts = replay_in.as_ref().is_none_or(|r| r.version >= 2);
        sim.pieces_expire = replay_in.as_ref().is_none_or(|r| r.version >= 3);
        sim.checkpoints_by_name = replay_in.as_ref().is_none_or(|r| r.version >= 4);
        let level_bonus = level.level.settings.level_bonus.unwrap_or(0);
        sim.level_bonus = level_bonus;
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
        // a run re-recorded from an old replay plays by its rules, so it keeps its format
        if let Some(r) = &replay_in {
            replay_out.version = r.version;
        }
        let module_file = |name: &str| data.part(&format!("3D Entities/PH/{name}.nmo")).map(|m| m.level);
        let effects = crate::particles::Effects::new(&level.level, &module_file);
        let mut placements = scene::placements(&level.level, &modules, sim.loose(), sim.module_bodies());
        if effects.has_flames() {
            // the flames are drawn, so their stand-ins need no markers
            placements.retain(|p| {
                p.marker.is_none()
                    || !level.level.groups_of(p.object).any(|g| {
                        g.eq_ignore_ascii_case("PC_Checkpoints") || g.eq_ignore_ascii_case("PS_Levelstart")
                    })
            });
        }
        let trafo_model = scene::FIRST_MODULE_MODEL + modules.models.len();
        let trafo = data
            .part("3D Entities/AnimTrafo.nmo")
            .and_then(|f| crate::trafo::TrafoModel::new(&f.level, trafo_model).map(|m| (f, m)));
        Ok(Game {
            trafo,
            placements,
            modules,
            ball_meshes,
            pieces,
            sim,
            start_ball,
            replay_out,
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
            assisted: false,
            last_level: level.level.settings.last_level,
            runs_root: {
                let (root, old) = runs_root(&o.source);
                let stem = level.name.trim_end_matches(".NMO").trim_end_matches(".nmo");
                match old.map(|old| move_runs(&old, &root, stem)) {
                    Some(Ok(true)) => println!("runs of {} moved to {}", shown(stem), shown_path(&root)),
                    Some(Err(e)) => eprintln!("could not move the runs of {}: {}", shown(stem), shown(&e)),
                    _ => {}
                }
                root
            },
            shown_name: match &o.source {
                crate::assets::Source::Original(_) => level.name.clone(),
                crate::assets::Source::Nmo(_) | crate::assets::Source::Open(_) => loading_name(&o.source),
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
        self.sim = Sim::with_modules(
            &self.level.level,
            self.start_ball,
            &self.modules,
            self.level.name.eq_ignore_ascii_case("Level_01.NMO"),
        );
        self.sim.set_ball_boxes(self.ball_meshes.boxes);
        self.sim.set_pieces(self.pieces.clone());
        self.sim.bursts = self.replay_in.as_ref().is_none_or(|r| r.version >= 2);
        self.sim.pieces_expire = self.replay_in.as_ref().is_none_or(|r| r.version >= 3);
        self.sim.checkpoints_by_name = self.replay_in.as_ref().is_none_or(|r| r.version >= 4);
        self.sim.level_bonus = self.level_bonus;
        self.ball_sound.silence(&mut self.sound_out);
        self.fan_sound.silence(&mut self.sound_out);
        if let Some(st) = &self.start {
            begin_at(&mut self.sim, st);
        }
        self.ball_lost_at = None;
        self.watch = Watch::default();
        self.replay_out = Replay::new(&self.level.name, self.level.hash);
        self.replay_out.start = self.start.clone();
        if let Some(r) = &self.replay_in {
            self.replay_out.version = r.version;
        }
    }

    /// The folder of this level's runs, attempts, watch log and findings.
    fn run_dir(&self) -> PathBuf {
        let stem = self.level.name.trim_end_matches(".NMO").trim_end_matches(".nmo");
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
        if let Ok(mut log) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("watch.log")) {
            for l in &lines {
                let _ = writeln!(log, "{l}");
            }
        }
        let snapshot = dir.join(format!("{}-{}.replay", f.kind.name(), f.tick));
        if let Err(e) = std::fs::write(&snapshot, self.replay_out.to_text()) {
            eprintln!("could not save the replay of the {} to {}: {e}", f.kind.name(), snapshot.display());
        }
    }

    /// Opens the session log of a windowed run in the level's folder: one line for the ball at every tick, and one
    /// for each of its contacts. A replay played back is logged too, so the same case can be compared with the run.
    fn start_session(&mut self) {
        let dir = self.run_dir();
        let _ = std::fs::create_dir_all(&dir);
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let path = dir.join(format!("session-{stamp}.log"));
        match std::fs::File::create(&path) {
            Ok(f) => {
                let mut w = std::io::BufWriter::new(f);
                use std::io::Write as _;
                let _ = writeln!(w, "# {} from tick {} ({})", self.level.name, self.sim.tick, clock(self.sim.tick));
                self.session = Some(w);
                println!("session log: {}", path.display());
            }
            Err(e) => eprintln!("could not open the session log {}: {e}", path.display()),
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
                c.entities, c.types, c.gap, c.pressure, c.normal[0], c.normal[1], c.normal[2], c.point[0], c.point[1], c.point[2]
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
        let input = match &self.replay_in {
            Some(r) if next <= r.end => r.input_at(next),
            Some(_) => 0,
            None => keys,
        };
        if self.sim.finished {
            // After the finish the world goes on for the eye (the balloon leaves); the run is over and recorded.
            self.sim.step(0);
            self.effects.tick(&self.sim);
            return;
        }
        let events_before = self.sim.events.len();
        self.sim.step(input);
        self.effects.tick(&self.sim);
        if self.sim.drawn_ball() == Some(self.sim.ball.kind) && !self.sim.game_over {
            let world = self.sim.world();
            let speed = self.sim.ball.velocity.length();
            self.ball_sound.tick(&world.ball_sounds, self.sim.ball.kind, speed, world.time(), &mut self.sound_out);
        } else {
            self.ball_sound.silence(&mut self.sound_out);
        }
        if self.sim.finished || self.sim.game_over {
            self.fan_sound.silence(&mut self.sound_out);
        } else {
            self.fan_sound.tick(&self.sim.fans(), self.sim.ball.position, &mut self.sound_out);
        }
        if self.sound_out.len() > 4096 {
            // nobody plays them (a run without a window)
            self.sound_out.clear();
        }
        self.replay_out.record(self.sim.tick, input);
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
        let sink = if sink.is_finite() { BALL_RADIUS - sink } else { 0.0 };
        let found = self.watch.observe(self.sim.tick, ball.position, ball.velocity, input, ball.on_ground, sink);
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
        let mut targets = vec![dir.join(format!("{}-{}.replay", stem, self.sim.tick))];
        if let Some(r) = &self.record {
            targets.push(r.clone());
        }
        let best = dir.join("best.replay");
        let best_ticks = std::fs::read_to_string(&best)
            .ok()
            .and_then(|t| Replay::parse(&t).ok())
            .map(|r| r.end);
        // A run begun elsewhere than the level start is a test, never the level's best.
        if self.start.is_none() && !self.assisted && best_ticks.is_none_or(|b| self.sim.tick < b) {
            targets.push(best.clone());
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

    fn title(&self) -> String {
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

/// F3 goes this far back (five seconds); the run is kept once a second for the last half minute.
const REWIND_TICKS: u64 = 300;
const HISTORY_EVERY: u64 = 60;
const HISTORY_KEPT: usize = 31;

/// The mode saved in the settings: extended (our additions on) only when chosen, else classic (the original).
fn load_extended() -> bool {
    setting("profile").as_deref() == Some("extended")
}

/// The name last entered for a highscore, in the game's Latin-1.
fn load_player() -> Vec<u8> {
    setting("player")
        .map(|n| n.chars().filter_map(|c| u8::try_from(u32::from(c)).ok()).take(crate::highscore::NAME_MAX).collect())
        .unwrap_or_default()
}

/// The extended mode (from the settings, or `--debug` for one run) and the additions the settings turn off.
fn load_features(debug: bool) -> crate::features::Features {
    crate::features::Features::load(debug || load_extended(), setting)
}

/// The Graphics page's choices kept in the settings.
fn load_graphics() -> crate::options::Graphics {
    crate::options::Graphics::load(setting)
}

/// The menus' words in the chosen language (lang.rs): the copy's for the original's interface, else ours.
fn load_strings(data: &Data) -> crate::lang::Strings {
    let copy = gamedata::resources::in_use() != gamedata::resources::Provider::Own;
    crate::lang::Strings::load(data, &crate::lang::chosen(setting("language").as_deref()), copy)
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
        // the sound theme is the extended mode's choice, written only once the player made one (until then the
        // default plays, whichever it is); the classic mode (the default theme) keeps the line as it was
        match gamedata::sound_themes::wanted() {
            Some(w) if features.mode => format!("sound-theme {w}\n"),
            _ => String::new(),
        },
        features.lines()
    );
    if !player.is_empty() {
        text.push_str(&format!("player {}\n", crate::lang::latin1(player)));
    }
    // the lines only the file sets (language, the level folders) are kept as they are
    let ours: &[&str] = if features.mode {
        &["sound", "music", "profile", "player", "interface", "sound-theme"]
    } else {
        &["sound", "music", "profile", "player", "interface"]
    };
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
            gamedata::package::title_of(p, &custom_roots()).replace('…', "...")
        }
    }
}

/// The custom levels' folders (all but the original's).
fn custom_roots() -> Vec<PathBuf> {
    crate::levels::KINDS
        .into_iter()
        .filter(|&k| k != crate::levels::Kind::Original)
        .map(crate::levels::dir)
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
            let kind = folder.and_then(Path::parent).filter(|k| custom_roots().iter().any(|r| r == k));
            match (kind.and_then(name), folder.and_then(name)) {
                (Some(kind), Some(folder)) => {
                    // the old place was named by the folder alone, and a loose map's by its kind's folder's name (that
                    // folder may be moved, ~/MyMaps): a level folder named like one of those shared it then
                    let kinds = crate::levels::KINDS.into_iter().map(crate::levels::dir);
                    let shared = folder == "other" || kinds.filter_map(|d| name(&d)).any(|k| k == folder);
                    let old = (!shared).then(|| maps.join(&folder));
                    (replay_dir().join("levels").join(kind).join(folder), old)
                }
                _ => (maps.join(folder.and_then(name).unwrap_or_else(|| "other".into())), None),
            }
        }
        crate::assets::Source::Open(p) => {
            (maps.join(p.parent().and_then(name).unwrap_or_else(|| "other".into())), None)
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
    let real_dir = |p: &Path| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink());
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

pub fn replay_dir() -> PathBuf {
    crate::levels::data_dir().join("replays")
}

pub fn run(o: Options, data: Data) -> Result<(), String> {
    // where the interface's files come from: the copy's own when it has its menu, else ours (resources.rs); the
    // extended mode may choose another
    let explicit = gamedata::resources::from_settings();
    let default = gamedata::resources::default_for(data.copy_has("3D Entities/Menu.nmo"));
    let chosen = explicit.unwrap_or(default);
    gamedata::resources::set_chosen(chosen);
    let extended = load_features(o.debug).mode;
    gamedata::resources::set_in_use(if extended { chosen } else { default });
    // the sound theme: the extended mode's choice (`sound-theme`), the default (generated) theme in the classic mode
    // (the interface no longer implies the original's sounds: the copy's interface is now the default)
    let theme = if extended {
        setting("sound-theme")
    } else {
        None
    };
    gamedata::sound_themes::start(theme.as_deref());
    // a level folder's own game files (modules, sky, sounds) come before the copy's
    let data = data.for_level(data.level_dir(&o.source).as_deref());
    if let Some(dir) = &o.export {
        let loaded = data.load(&o.source)?;
        return crate::assets::export(&data, &loaded, dir);
    }
    // the video tracking draws the levels as it was tuned on: without the cloud layer
    if o.locate.is_some() || o.follow.is_some() || o.frames.is_some() {
        scene::set_clouds(false);
    }
    if let Some(dir) = &o.locate {
        let mut game = Game::new(&o, &data)?;
        return locate_run(&mut game, &data, &o, dir);
    }
    if let Some(follow) = &o.follow {
        return follow_run(&o, &data, follow);
    }
    let mut game = Game::new(&o, &data)?;
    if let Some(dir) = &o.frames {
        return frames_run(&mut game, &data, dir);
    }
    if let Some(path) = &o.screenshot {
        // the player's clouds and field of view, as the window would show them
        show_graphics(&load_graphics(), load_features(o.debug));
        return screenshot(&mut game, &data, &o, path);
    }
    if o.autopilot {
        return autopilot(&mut game, &data, &o);
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
        while game.sim.tick < end && !game.sim.finished && !game.sim.game_over {
            game.tick(0);
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
                // the loose things near the ball (boxes, balls, modules' parts), with DRAHTSEILAKT_TRACE_LOOSE set
                if std::env::var_os("DRAHTSEILAKT_TRACE_LOOSE").is_some() {
                    for &(o, body) in game.sim.loose() {
                        let Some(st) = game.sim.world().state(body) else { continue };
                        if st.pos.distance(b.position) < 40.0 {
                            let name = game.level.level.objects.get(o).map_or("?", |ob| ob.name.as_str());
                            println!("    {name} ({:8.2} {:8.2} {:8.2})", st.pos.x, st.pos.y, st.pos.z);
                        }
                    }
                    // and the modules' moving parts
                    for m in game.sim.module_bodies() {
                        let Some(st) = game.sim.world().state(m.body) else { continue };
                        if st.pos.distance(b.position) < 40.0 {
                            let (o, p) = (&m.object, st.pos);
                            println!("    {o} #{} ({:8.2} {:8.2} {:8.2})", m.body, p.x, p.y, p.z);
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
        match end_music {
            Some(t) => println!("end music: Music_EndCheckpoint loops from {} ({} before the end)", clock(t),
                clock(s.tick.saturating_sub(t))),
            None => println!("end music: Music_EndCheckpoint never began"),
        }
        return Ok(());
    }
    if let Some(tick) = o.from {
        game.fast_forward(tick);
    }
    game.start_session();
    let video = o.video.clone();
    // Without a level named, the window opens with the main menu over the first level.
    crate::levels::prepare(&crate::lang::docs_language(setting("language").as_deref()));
    let strings = load_strings(&data);
    let features = load_features(o.debug);
    let graphics = load_graphics();
    let bindings = load_bindings(&strings);
    show_graphics(&graphics, features);
    let menu = (!o.chosen && o.replay.is_none()).then(|| {
        let mut m = Menu::new(&data, &strings, crate::menu::Screen::Main, Some(1), features);
        m.volumes = load_volumes();
        m.graphics = graphics;
        m.bindings = bindings;
        m
    });
    let mut app = App {
        number: match o.source {
            crate::assets::Source::Original(n) => Some(n),
            _ => None,
        },
        opts: o.clone(),
        menu,
        loading: None,
        features,
        strings,
        graphics,
        bindings,
        pads: crate::options::Pads::open(),
        pad_keys: 0,
        pad_held: false,
        cursor: None,
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
        music: None,
        blitz: crate::lights::Blitz::new(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64),
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
        menu_ms: 0.0,
        held: std::collections::HashSet::new(),
    };
    if let Some(path) = video {
        return app.video(&path);
    }
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    app.game.save_attempt();
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Plays the replay and draws `locate::RATE` frames of it a second (640 × 480 with the in-game interface, as a
/// video of the original would show it) into `dir` as s0001.jpg, ..., with `truth.txt`: each frame's number, the
/// ball's place and the view's quarter. A video with a known answer, to check `--locate` with.
fn frames_run(game: &mut Game, data: &Data, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let end = game.replay_in.as_ref().map(|r| r.end).ok_or("--frames needs --replay")?;
    let (_, device, queue) = pollster::block_on(device(None))?;
    let mut r = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
    upload(&mut r, game, data);
    let off = crate::locate::Offscreen::sized(&r, 640, 480);
    let mut truth = String::from("# frame x y z quarter\n");
    let mut k = 0;
    while game.sim.tick < end && !game.sim.game_over {
        game.tick(0);
        if !game.sim.tick.is_multiple_of(u64::from(60 / crate::locate::RATE)) {
            continue;
        }
        k += 1;
        let snap = Snapshot::of(&game.sim);
        let mut inst = scene::instances(
            &game.placements,
            &game.sim.taken,
            &game.sim.world().bodies,
            &game.ball_meshes,
            game.trafo.as_ref().map(|t| &t.1),
            (&game.sim.extra_views(), game.extra_model.as_ref()),
            &snap,
            &game.sim.pieces_view(),
        );
        if snap.kind.is_some() {
            scene::shadows(&game.shadow_receivers, snap.ball, &mut inst);
        }
        let screen = crate::ui::Screen { w: 640.0, h: 480.0 };
        let quads = crate::hud::interface(&game.sim, game.font.as_ref(), screen);
        let px = off.pixels(&mut r, &scene::view(&snap), &inst, &quads).ok_or("the frame could not be read back")?;
        let img = image::RgbaImage::from_raw(640, 480, px).ok_or("frame size")?;
        let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
        rgb.save(dir.join(format!("s{k:04}.jpg"))).map_err(|e| e.to_string())?;
        let b = game.sim.ball.position;
        truth.push_str(&format!("{k} {:.2} {:.2} {:.2} {}\n", b.x, b.y, b.z, game.sim.camera.quarter));
    }
    std::fs::write(dir.join("truth.txt"), truth).map_err(|e| e.to_string())?;
    println!("{k} frames in {}", dir.display());
    Ok(())
}

/// Finds the ball's way in a video's frames (`locate::RATE` a second) and writes it as a track: per frame its
/// number, the ball's place, the view's quarter and how alike the best picture was.
fn locate_run(game: &mut Game, data: &Data, o: &Options, dir: &Path) -> Result<(), String> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('s') && n.ends_with(".jpg")))
        .collect();
    // in the frames' order (s0001.jpg, ..., s10000.jpg: by number, not by name)
    let number = |p: &PathBuf| -> Option<u64> {
        p.file_stem()?.to_str()?.strip_prefix('s')?.parse().ok()
    };
    names.retain(|p| number(p).is_some());
    names.sort_by_key(|p| number(p));
    if let Some(n) = o.ticks {
        names.truncate(n as usize);
    }
    let frames: Vec<Vec<f32>> = names
        .iter()
        .map(|p| crate::locate::frame_edges(p).ok_or_else(|| format!("{}: not a picture", p.display())))
        .collect::<Result<_, _>>()?;
    println!("{} frames", frames.len());
    let start = game.sim.ball.position;
    let route = crate::autopilot::Route::find(&game.level.level, &game.modules, &game.sim.updrafts(), start, &[]);
    let floor: Vec<glam::Vec3> = route.floor_points().collect();
    let (_, device, queue) = pollster::block_on(device(None))?;
    let mut r = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm);
    upload(&mut r, game, data);
    let off = crate::locate::Offscreen::new(&r);
    let base = Snapshot::of(&game.sim);
    let mut draw = |view: &crate::render::View, ball: glam::Vec3| {
        let snap = Snapshot { ball, eye: view.eye, target: view.target, ..base };
        let inst = scene::instances(
            &game.placements,
            &game.sim.taken,
            &game.sim.world().bodies,
            &game.ball_meshes,
            None,
            (&[], None),
            &snap,
            &game.sim.pieces_view(),
        );
        off.edges(&mut r, view, &inst)
    };
    let quarter = game.sim.camera.quarter;
    let fixes = crate::locate::track(&frames, &floor, start, quarter, &mut draw)?;
    let mut text = format!("{}\n", crate::locate::TRACK_HEAD);
    for (k, f) in fixes.iter().enumerate() {
        let (b, q, s) = (f.ball, f.quarter, f.score);
        text.push_str(&format!("{} {:.2} {:.2} {:.2} {q} {s:.3}\n", k + 1, b.x, b.y, b.z));
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
    let mut game = Game::new(&own, data)?;
    let trafos: Vec<_> = game.sim.trafos().collect();
    // where the loose objects (boxes, balls) and the modules' moving parts start, and how near the ball comes to
    // one to move it
    const PUSH: f32 = 8.0;
    // (the level's objects where they start: the bodies of later sectors are not in the physics yet)
    // (a module's part is placed by the module, not a level object: where its body is made, or else the module's)
    let objects = &game.level.level.objects;
    let at = |o: usize| objects.get(o).map(|o| glam::Vec4::from(o.matrix[3]).truncate());
    let parts = game
        .sim
        .module_bodies()
        .iter()
        .filter_map(|m| game.sim.world().state(m.body).map(|s| s.pos).or_else(|| at(m.placeholder)));
    let loose: Vec<glam::Vec3> = game.sim.loose().iter().filter_map(|l| at(l.0)).chain(parts).collect();
    let mut kind = game.sim.ball.kind;
    let mut path: Vec<glam::Vec3> = Vec::new();
    let mut gates: Vec<crate::autopilot::Gate> = Vec::new();
    let mut last: Option<glam::Vec3> = None;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let v: Option<Vec<f32>> = line.split_whitespace().map(|x| x.parse().ok()).collect();
        let Some([_, x, y, z, _, likeness]) = v.as_deref().and_then(|v| <[f32; 6]>::try_from(v).ok()) else {
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
        if let Some(i) = path.iter().skip(kept).rposition(|p| p.distance(floor) < 2.5) {
            // a loop to a loose object (a box to push, say) did something there: it stays
            let pushed = |p: &glam::Vec3| loose.iter().any(|o| o.distance(*p) < PUSH);
            if !path.get(kept + i..).unwrap_or_default().iter().any(pushed) {
                // (the last gate's own point stays)
                let keep = if gates.is_empty() { 0 } else { kept + 1 };
                path.truncate((kept + i).max(keep));
            }
        }
        path.push(floor);
        let trafo = trafos.iter().find(|t| t.1 != kind && t.0.distance(ball) < crate::sim::TRAFO_REACH);
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
        let apart = stages.last().is_none_or(|l| l.distance(p) >= if near_loose { 1.5 } else { STAGE });
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
    let route = crate::autopilot::Route::through(&game.level.level, &game.modules, &fans, start, &stages);
    let gates: Vec<crate::autopilot::Gate> =
        gate_stages.iter().filter_map(|&(k, q, kind)| route.marks.get(k).map(|&m| (m, q, kind))).collect();
    let (places, ways) = (path.len(), route.points.len());
    println!("track: {places} places, {} stages, way {ways} points, transformers {gates:?}", stages.len());
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
    pilot_run(&mut game, data, &own, crate::autopilot::Pilot::new(route, gates))
}

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
            return Err(format!("{}: --then needs a run begun at a sector", more.display()));
        };
        // the followed way up to that sector, then the other run's
        let cut = w.sectors.iter().position(|&s| s >= from).unwrap_or(w.path.len());
        w.path.truncate(cut);
        w.sectors.truncate(cut);
        w.gates.retain(|g| g.0 < cut);
        w.gates.extend(m.gates.iter().map(|g| (g.0 + cut, g.1, g.2)));
        w.path.extend(m.path);
        w.sectors.extend(m.sectors);
        let ticks = w.demo.ticks.iter().position(|t| t.sector >= from).unwrap_or(w.demo.ticks.len());
        w.demo.ticks.truncate(ticks);
        w.demo.ticks.extend(m.demo.ticks);
        w.deaths = m.deaths.iter().map(|d| d + cut).collect();
        w.finished = m.finished;
    }
    println!("followed: {} points, transformers {:?}", w.path.len(), w.gates);
    // the test player's own run begins where the followed one began
    let mut own = o.clone();
    own.replay = None;
    own.follow = None;
    if let Some(st) = &w.run.start {
        own.sector = Some(st.sector);
        own.ball = st.ball.clone();
        own.at = st.at.map(|[x, y, z]| format!("{x},{y},{z}"));
    }
    let mut game = Game::new(&own, data)?;
    if !w.finished {
        way_on(&game, &own, &mut w)?;
    }
    let route = crate::autopilot::Route::along(&game.level.level, &game.modules, &game.sim.updrafts(), w.path);
    let mut pilot = crate::autopilot::Pilot::new(route, w.gates);
    pilot.demo = Some(w.demo);
    pilot_run(&mut game, data, &own, pilot)
}

/// A followed run that did not finish: its way up to its last sector and its try there that got furthest (lost its
/// ball furthest from where it came into that sector), then the way the test player finds by itself from there to
/// the end.
fn way_on(game: &Game, o: &Options, w: &mut Watched) -> Result<(), String> {
    let Some(&last) = w.sectors.last() else { return Ok(()) };
    let first = w.sectors.iter().position(|&s| s == last).unwrap_or(0);
    let entry = w.path.get(first).copied().unwrap_or_default();
    // its tries in that sector, each from where it came in (or a new ball) to where it lost the ball
    let mut ends: Vec<usize> = w.deaths.iter().copied().filter(|&d| d > first && d <= w.path.len()).collect();
    ends.push(w.path.len());
    let tries: Vec<(usize, usize)> = std::iter::once(first).chain(ends.iter().copied()).zip(ends.iter().copied())
        .filter(|(a, b)| b > a)
        .collect();
    let far = |t: &(usize, usize)| w.path.get(t.1 - 1).map_or(0.0, |p| p.distance(entry));
    if let Some(&(a, b)) = tries.iter().max_by(|x, y| far(x).total_cmp(&far(y))) {
        let mut path: Vec<glam::Vec3> = w.path.get(..first).unwrap_or_default().to_vec();
        path.extend(w.path.get(a..b).unwrap_or_default());
        let mut gates: Vec<crate::autopilot::Gate> = w.gates.iter().copied().filter(|g| g.0 < first).collect();
        gates.extend(w.gates.iter().filter(|g| g.0 >= a && g.0 < b).map(|g| (g.0 - a + first, g.1, g.2)));
        w.sectors.truncate(first);
        w.sectors.extend(std::iter::repeat_n(last, b - a));
        w.path = path;
        w.gates = gates;
    }
    let Some(&from) = w.path.last() else { return Ok(()) };
    let kind = w.gates.last().map_or(game.sim.ball.kind, |g| g.2);
    let up = from + glam::Vec3::Y * crate::sim::BALL_RADIUS;
    let (route, gates) = found_way(game, o, up, last, &w.path, kind)?;
    let n = w.path.len();
    println!("the followed run ends unfinished at {from:?} (sector {}): the way on is found, {} points", last + 1,
        route.points.len());
    w.gates.extend(gates.iter().map(|g| (g.0 + n, g.1, g.2)));
    w.sectors.extend(std::iter::repeat_n(last, route.points.len()));
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
        });
        let before = seen.sim.events.len();
        seen.tick(0);
        for (_, e) in seen.sim.events.iter().skip(before) {
            match e {
                crate::sim::Event::Died => {
                    lost = true;
                    deaths.push(path.len());
                    // the player's last seconds before losing a ball are no line to follow
                    let n = demo.ticks.len();
                    for t in demo.ticks.iter_mut().skip(n.saturating_sub(DOOMED_TICKS)) {
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
        if std::env::var_os("AUTOPILOT_TRACE_FOLLOW").is_some() && seen.sim.tick.is_multiple_of(30) {
            let p = b.position;
            eprintln!("follow t {} s {} ({:.1} {:.1} {:.1}) q {}", seen.sim.tick, seen.sim.sector + 1, p.x, p.y, p.z,
                seen.sim.camera.quarter);
        }
        let floor = b.position - glam::Vec3::Y * crate::sim::BALL_RADIUS;
        if !lost && b.on_ground && path.last().is_none_or(|p| p.distance(floor) >= 1.5) {
            path.push(floor);
            sectors.push(seen.sim.sector);
        }
    }
    let finished = seen.sim.finished;
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
) -> (Vec<glam::Vec3>, Vec<usize>, Vec<crate::autopilot::Gate>, Vec<usize>) {
    let last = sectors.last().copied().unwrap_or(0);
    let mut keep = vec![true; path.len()];
    let mut try_start = 0;
    for &d in &deaths {
        let d = d.min(path.len());
        let s = d.checked_sub(1).and_then(|i| sectors.get(i)).copied().unwrap_or(0);
        if finished || s != last {
            let from = (try_start..d).find(|&i| sectors.get(i) == Some(&s)).unwrap_or(d);
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
    let gates = gates.into_iter().filter(|g| keep.get(g.0).copied().unwrap_or(true)).map(|g| (new(g.0), g.1, g.2));
    let in_last = |d: usize| sectors.get(d.saturating_sub(1)) == Some(&last);
    let deaths = deaths.iter().filter(|&&d| d >= keep.len() || !finished && in_last(d));
    let deaths: Vec<usize> = deaths.map(|&d| new(d)).collect();
    let path = path.into_iter().zip(&keep).filter(|p| *p.1).map(|p| p.0).collect();
    let sectors = sectors.into_iter().zip(&keep).filter(|p| *p.1).map(|p| p.0).collect();
    (path, sectors, gates.collect(), deaths)
}

/// The test player plays the level from where the run begins: it saves the run at --record (or as an attempt) and
/// prints how far it got.
fn autopilot(game: &mut Game, data: &Data, o: &Options) -> Result<(), String> {
    let start = game.sim.ball.position;
    let (route, gates) = found_way(game, o, start, game.sim.sector, &[], game.sim.ball.kind)?;
    println!("route: {} points, stages at {:?}", route.points.len(), route.marks);
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
/// but for the objects of it a player's way `passed` already came by (within 20: those and all before them).
fn found_way(
    game: &Game,
    o: &Options,
    start: glam::Vec3,
    sector: usize,
    passed: &[glam::Vec3],
    kind: crate::sim::BallKind,
) -> Result<(crate::autopilot::Route, Vec<crate::autopilot::Gate>), String> {
    let level = &game.level.level;
    let stages: Vec<glam::Vec3> = game.sim.stages().into_iter().skip(sector).collect();
    // The transformers the way passes (within 30) are part of the way, in its order: a player rolls into them
    // rather than around them.
    let base = crate::autopilot::Route::find(level, &game.modules, &game.sim.updrafts(), start, &stages);
    let mut waypoints: Vec<(usize, glam::Vec3, bool)> = Vec::new();
    for t in game.sim.transformers() {
        let near = base
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| (i, glam::Vec3::new(p.x - t.x, 0.0, p.z - t.z).length() + (p.y - t.y).abs()))
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
        let end = stages.first().and_then(|c| planned.iter().position(|p| p.0.distance(*c) < 1.0));
        let came = planned
            .iter()
            .take(end.unwrap_or(planned.len()))
            .rposition(|p| passed.iter().any(|q| q.distance(p.0) < 20.0));
        if let Some(k) = came {
            planned.drain(..=k);
        }
        waypoints = planned.into_iter().map(|(p, trafo)| (0, p, trafo)).collect();
    }
    let fans = game.sim.updrafts();
    // the transformers' places on the way: their gates
    // (each with the ball its transformer makes: the level's transformer at that place)
    let kind_at = |p: glam::Vec3| {
        game.sim.trafos().min_by(|a, b| a.0.distance(p).total_cmp(&b.0.distance(p))).map(|t| t.1)
    };
    let mut tries = 0;
    loop {
        let stages: Vec<glam::Vec3> = waypoints.iter().map(|w| w.1).collect();
        let route = crate::autopilot::Route::find(level, &game.modules, &fans, start, &stages);
        let gates: Vec<crate::autopilot::Gate> = route
            .marks
            .iter()
            .zip(&waypoints)
            .filter(|(_, w)| w.2)
            .filter_map(|(&m, w)| Some((m, w.1, kind_at(w.1)?)))
            .collect();
        // Only a paper ball rides an updraft: a lift on the way taken as another ball is preceded by the paper
        // transformer nearest the fan, and the way is found again (a few times at most).
        let ball_at = |i: usize| gates.iter().take_while(|g| g.0 <= i).last().map_or(kind, |g| g.2);
        let wrong = route.lifts.iter().find(|&&(i, _)| ball_at(i) != crate::sim::BallKind::Paper);
        let paper = wrong.and_then(|&(i, f)| {
            let base = fans.get(f)?.0;
            let t = game.sim.trafos().filter(|t| t.1 == crate::sim::BallKind::Paper).min_by(|a, b| {
                a.0.distance(base).total_cmp(&b.0.distance(base))
            })?;
            (t.0.distance(base) < FAN_PAPER_NEAR).then_some((i, t.0))
        });
        tries += 1;
        let Some((i, at)) = paper.filter(|_| tries <= FAN_PAPER_TRIES) else { return Ok((route, gates)) };
        let before = route.marks.iter().position(|&m| m >= i).unwrap_or(waypoints.len());
        if waypoints.iter().any(|w| w.1.distance(at) < 1.0) {
            return Ok((route, gates));
        }
        println!("the way rides a fan at {i} not as paper: into the paper transformer at {at:?} first");
        waypoints.insert(before, (0, at, true));
    }
}

/// How often the way is found again to take a paper transformer before a fan, and how far from the fan one may be.
const FAN_PAPER_TRIES: usize = 4;
const FAN_PAPER_NEAR: f32 = 150.0;

/// A plan: one level object a line (`NAME  # note`), in the order the way goes through them; blank lines and
/// `#` comments are skipped. Returns where each is and whether it is a transformer, from the checkpoint sector
/// `sector` (from 0) begins at on (PC_TwoFlames_0N; all of it for the first).
fn read_plan(path: &Path, level: &level::Level, sector: usize) -> Result<Vec<(glam::Vec3, bool)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    let begins = format!("PC_TwoFlames_{sector:02}");
    for line in text.lines() {
        let name = line.split('#').next().unwrap_or_default().trim().trim_end_matches('?').trim();
        if name.is_empty() {
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
fn level_stem(game: &Game) -> String {
    Path::new(&game.level.name).file_stem().map_or_else(String::new, |s| s.to_string_lossy().into_owned())
}

/// The level's hint file: --hints, else hints/<level>.txt in the data folder.
fn hints_path(game: &Game, o: &Options) -> PathBuf {
    o.hints.clone().unwrap_or_else(|| {
        crate::levels::data_dir().join("hints").join(format!("{}.txt", level_stem(game)))
    })
}

/// The hints of the level, a replay stretch read into the view's quarter at its start and its inputs (the replay is
/// played to its first tick to know the quarter).
fn load_hints(game: &Game, data: &Data, o: &Options) -> Result<Vec<crate::hints::Hint>, String> {
    let path = hints_path(game, o);
    let mut hints = crate::hints::load(&path)?;
    let dir = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    for h in &mut hints {
        for step in &mut h.steps {
            let crate::hints::Step::Replay(file, from, to) = step else { continue };
            let file = dir.join(&*file);
            let bytes = crate::hints::read_plain(&file, crate::hints::MAX_REPLAY_FILE)?;
            let text = String::from_utf8(bytes).map_err(|_| format!("{}: not UTF-8", file.display()))?;
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
        let f = dir.join(if k == 0 { stem.clone() } else { format!("{stem}-{k}") });
        match std::fs::create_dir(&f) {
            Ok(()) => {
                folder = Some(f);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", f.display())),
        }
    }
    let folder = folder.ok_or_else(|| format!("{}: no new folder for the report", dir.display()))?;
    let mut t = String::new();
    let _ = writeln!(t, "level {}  tick {} ({})  sector {}  lives {}", game.level.name, s.tick, clock(s.tick),
        s.sector + 1, s.lives);
    let v = s.ball.velocity;
    let _ = writeln!(t, "ball {:?} at ({:.2} {:.2} {:.2}) velocity ({:.2} {:.2} {:.2}) on ground {}", s.ball.kind,
        b.x, b.y, b.z, v.x, v.y, v.z, s.ball.on_ground);
    let _ = writeln!(t, "view quarter {} (up pushes along {:?})", s.camera.quarter, s.camera.push_forward());
    let _ = writeln!(t, "route {} / {}; ahead:", pilot.reached(), pilot.route.points.len());
    for p in pilot.route.points.iter().skip(pilot.reached()).step_by(5).take(12) {
        let _ = writeln!(t, "  ({:.1} {:.1} {:.1})", p.x, p.y, p.z);
    }
    let _ = writeln!(t, "transformers:");
    for (p, k) in s.trafos() {
        if p.distance(b) < 80.0 {
            let _ = writeln!(t, "  {k:?} at ({:.1} {:.1} {:.1}) {:.1} away", p.x, p.y, p.z, p.distance(b));
        }
    }
    let level = &game.level.level;
    let _ = writeln!(t, "loose bodies:");
    for &(obj, body) in s.loose() {
        let (Some(o), Some(st)) = (level.objects.get(obj), s.world().state(body)) else { continue };
        if st.pos.distance(b) < 60.0 {
            let p = st.pos;
            let _ = writeln!(t, "  {} at ({:.1} {:.1} {:.1}) {:.1} away", o.name, p.x, p.y, p.z, p.distance(b));
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
                let groups: Vec<&str> = level.groups_of(i).filter(|g| !g.starts_with("Sector")).take(4).collect();
                (d, format!("  {} at ({:.1} {:.1} {:.1}) {:.1} away {:?}", ob.name, p.x, p.y, p.z, d, groups))
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

/// Runs the test player to --ticks (or the finish), prints how far it got and saves its run at --record. Where it
/// is stuck it writes a report (--report), and it follows the level's hints.
fn pilot_run(game: &mut Game, data: &Data, o: &Options, mut pilot: crate::autopilot::Pilot) -> Result<(), String> {
    pilot.hints = crate::hints::Runner::new(load_hints(game, data, o)?);
    let level = &game.level.level;
    pilot.bridges = (0..level.objects.len())
        .filter(|&i| crate::scene::module_of(level, i) == Some("P_Modul_25"))
        .filter_map(|i| level.objects.get(i).map(|ob| glam::Vec3::from(ob.position())))
        .collect();
    if game.start.is_some() {
        pilot.take_up(&game.sim);
    }
    // the player's own data folder, not a shared temporary one
    let report = o.report.clone().unwrap_or_else(|| crate::levels::data_dir().join("stuck"));
    let limit = o.ticks.unwrap_or(60 * 60 * 20);
    let (mut best, mut since) = (0usize, game.sim.tick);
    // a dead run: the ball neither falls nor gets on along the way for --give-up seconds (it stands or goes to and
    // fro, not knowing where to go); its report is written and the run stops, for whoever writes the hints. A ball
    // that falls is an ordinary mistake: each fall starts the wait again, and the pilot goes on by itself.
    let give_up = o.give_up.saturating_mul(60);
    let mut heard = game.sim.events.len();
    let mut hint_at = pilot.hints.at();
    let mut dead = false;
    while game.sim.tick < limit && !game.sim.finished && !game.sim.game_over {
        let i = pilot.next(&game.sim);
        game.tick(i);
        if pilot.reached() > best {
            (best, since) = (pilot.reached(), game.sim.tick);
        }
        let fell = game.sim.events.iter().skip(heard).any(|(_, e)| matches!(e, crate::sim::Event::Died));
        if game.sim.events.iter().skip(heard).any(|(_, e)| matches!(e, crate::sim::Event::NewBall)) {
            pilot.new_ball(&game.sim);
            // back at the reset point: getting on is counted from there again
            best = pilot.reached();
        }
        heard = game.sim.events.len();
        // a hint going on to its next step gets on too (its steps lead off the way: round, back for a run-up), and so
        // does one playing a recorded run
        let now_at = pilot.hints.at();
        if fell || (now_at != hint_at && now_at.is_some()) || pilot.hints.replaying() {
            since = game.sim.tick;
        }
        hint_at = now_at;
        if game.sim.tick - since > give_up {
            let b = game.sim.ball.position;
            println!("dead run: ball at ({:.1} {:.1} {:.1}), not falling and the way not on for {} s", b.x, b.y, b.z,
                (game.sim.tick - since) / 60);
            if let Err(e) = stuck_report(game, data, o, &pilot, &report) {
                eprintln!("stuck report: {e}");
            }
            dead = true;
            break;
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
    let s = &game.sim;
    println!(
        "result: {} after {} ({}), sector {}, lives {}, route {} / {}",
        if s.finished {
            "finished"
        } else if s.game_over {
            "game over"
        } else if dead {
            "dead run"
        } else {
            "not finished"
        },
        s.tick,
        clock(s.tick),
        s.sector + 1,
        s.lives,
        pilot.reached(),
        pilot.route.points.len()
    );
    if let Some(path) = &o.record {
        std::fs::write(path, game.replay_out.to_text()).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("saved {}", path.display());
    }
    Ok(())
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
    let (adapter, device, queue) =
        pollster::block_on(adapter_device(&instance, Some(&surface)))?;
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
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
    Ok(Gpu { surface, config, renderer, modes: caps.present_modes })
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
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("drahtseilakt"),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("no graphics device: {e}"))?;
    Ok((adapter, device, queue))
}

/// Uploads the menu scene's model (after the game's, whose model numbers it must not take).
fn upload_menu_scene(renderer: &mut Renderer, scene: Option<&mut crate::menuscene::MenuScene>, data: &Data) {
    if let Some(ms) = scene {
        let loader = |t: &level::Texture| data.texture(t);
        ms.model = Some(renderer.add_model(&ms.level, &loader));
    }
}

fn upload(renderer: &mut Renderer, game: &Game, data: &Data) {
    // the level's light colour (the level table's Light; white for a level without a row)
    renderer.light.color = game.level.level.settings.light.unwrap_or([1.0; 3]);
    let loader = |t: &level::Texture| data.texture(t);
    renderer.add_model(&game.level.level, &loader);
    renderer.add_model(&game.balls.level, &loader);
    let letter = game.level.level.sky.clone().unwrap_or_else(|| "A".into());
    let sky = crate::sky::model(&letter);
    renderer.add_model(&sky, &loader);
    for m in &game.modules.models {
        renderer.add_model(m, &loader);
    }
    if let Some((file, _)) = &game.trafo {
        renderer.add_model(&file.level, &loader);
    }
    let shadow = level::Texture {
        name: "HardShadow".into(),
        source: level::TextureSource::File("HardShadow.bmp".into()),
        color_key: None,
    };
    if let Some((w, h, rgba)) = data.texture(&shadow) {
        renderer.set_shadow_texture(w, h, rgba);
    }
    // the interface's textures; the own interface's picture behind its menus only with it (the copy has none)
    let own = gamedata::resources::in_use() == gamedata::resources::Provider::Own;
    let interface = ["Font_1", "Button01_special", "Button01_select", "Button01_deselect"];
    let backdrop = own.then_some(crate::ui::BACKDROP);
    // the own interface draws its texts in the shipped fonts: the copy's font is not read
    let fonts = interface.into_iter().filter(|n| !(own && *n == "Font_1"));
    for name in crate::particles::textures().into_iter().chain(fonts).chain(backdrop) {
        let t = level::Texture {
            name: name.to_owned(),
            source: level::TextureSource::File(if name.starts_with("Font_") || name.starts_with("Button") {
                format!("{name}.tga")
            } else if name == crate::ui::BACKDROP {
                format!("{name}.png")
            } else {
                format!("{name}.bmp")
            }),
            color_key: None,
        };
        if let Some((w, h, rgba)) = data.texture(&t) {
            renderer.add_named_texture(name, w, h, rgba);
        }
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

fn screenshot(game: &mut Game, data: &Data, o: &Options, path: &Path) -> Result<(), String> {
    let ticks = o
        .ticks
        .or(game.replay_in.as_ref().map(|r| r.end))
        .unwrap_or(1);
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
    upload(&mut r, game, data);
    // the passing lights as the window would show them (the lightning on a fixed schedule)
    if game.last_level {
        r.flash = crate::lights::Blitz::new(1).step(game.sim.tick).0;
    }
    r.point = crate::lights::point(&game.sim.events, game.sim.tick, snap.ball);
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
    let view = target.create_view(&Default::default());
    let inst = scene::instances(
        &game.placements,
        &game.sim.taken,
        &game.sim.world().bodies,
        &game.ball_meshes,
        game.trafo.as_ref().map(|t| &t.1),
        (&game.sim.extra_views(), game.extra_model.as_ref()),
        &snap,
        &game.sim.pieces_view(),
    );
    let mut inst = inst;
    if snap.kind.is_some() {
        scene::shadows(&game.shadow_receivers, snap.ball, &mut inst);
    }
    let particles = game.effects.batches(snap.eye, snap.target);
    r.draw(&view, w, h, &scene::view(&snap), &inst, &particles);
    r.draw_overlay(
        &view,
        w,
        h,
        &crate::hud::overlay(&game.sim, w, h, true, game.font.is_some()),
    );
    let screen = crate::ui::Screen { w: w as f32, h: h as f32 };
    let mut quads = crate::hud::interface(&game.sim, game.font.as_ref(), screen);
    let strings = load_strings(data);
    let lost = strings.ours("ball lost");
    quads.extend(crate::hud::ball_lost(&game.sim, game.ball_lost_at, &lost, game.font.as_ref(), screen));
    if let (Some(t), Some(f)) = (crate::endscreen::of(&game.sim), game.font.as_ref()) {
        quads.extend(crate::endscreen::interface(&t, f, &strings, screen));
    }
    // DRAHTSEILAKT_SHOT_MENU=base:keys draws a menu over the frame, to look at the menus without a window: base is
    // main, pause, dead or end, keys are u d l r (arrows), e (Enter) and b (Esc), e.g. "main:ddde" for Options.
    let shot = std::env::var("DRAHTSEILAKT_SHOT_MENU").ok();
    if let Some(m) = shot.and_then(|spec| shot_menu(&spec, data, game, o, (w, h))) {
        if !m.over_level() {
            quads.clear();
        }
        quads.extend(m.interface(game.font.as_ref(), screen));
    }
    if let Some((aw, ah, rgba)) = crate::fallback::take_changed() {
        r.add_named_texture(crate::fallback::TEXTURE, aw, ah, rgba);
    }
    r.draw_ui(&view, w, h, &quads);

    let pixels = read_back(&r, &target, w, h)?;
    image::save_buffer(path, &pixels, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    println!(
        "{} written ({}x{}, tick {})",
        path.display(),
        w,
        h,
        game.sim.tick
    );
    Ok(())
}

pub(crate) struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub renderer: Renderer,
    /// The presenting modes the surface offers.
    pub modes: Vec<wgpu::PresentMode>,
}

struct App {
    game: Game,
    /// The options the game was started with, for loading another level.
    opts: Options,
    /// The original level being played, if it is one.
    number: Option<u32>,
    /// The menu, while it is open; the run is paused under it.
    menu: Option<Menu>,
    /// A level to load: (the level, whether the loading screen has been drawn).
    loading: Option<(crate::assets::Source, bool)>,
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
    /// The wheel's turn not yet a whole notch (a touchpad's pixels come in small steps).
    wheel: f64,
    /// The cursor is shown (menus) or hidden (the run); None until first set.
    cursor_shown: Option<bool>,
}

/// The sizes the Graphics page offers: the monitor's modes and the window sizes that fit on it, smallest first.
fn sizes(window: &Window) -> Vec<(u32, u32)> {
    let monitor = window.current_monitor();
    let fits = monitor.as_ref().map(|m| m.size()).map_or((u32::MAX, u32::MAX), |s| (s.width, s.height));
    let mut v: Vec<(u32, u32)> = monitor
        .iter()
        .flat_map(|m| m.video_modes())
        .map(|m| (m.size().width, m.size().height))
        .chain(crate::options::WINDOWED.into_iter().filter(|s| s.0 <= fits.0 && s.1 <= fits.1))
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
        let area = |m: &winit::monitor::VideoModeHandle| u64::from(m.size().width) * u64::from(m.size().height);
        let wanted = |m: &&winit::monitor::VideoModeHandle| Some((m.size().width, m.size().height)) == size;
        let best = modes
            .iter()
            .filter(wanted)
            .max_by_key(|m| m.refresh_rate_millihertz())
            .or_else(|| modes.iter().max_by_key(|m| (area(m), m.refresh_rate_millihertz())))?;
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
    scene::set_fov((features.mode && g.fov != crate::options::FOV_ORIGINAL).then_some(g.fov));
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let attrs = Window::default_attributes()
            .with_title(self.game.title())
            .with_inner_size(winit::dpi::PhysicalSize::new(self.size.0, self.size.1));
        let window = Arc::new(event_loop.create_window(attrs).map_err(|e| e.to_string())?);
        let Gpu { surface, config, mut renderer, modes } = window_gpu(&window)?;
        upload(&mut renderer, &self.game, &self.data);
        if self.menu_scene.is_none() {
            // the original's 3D scene behind its menus; the own interface has its picture (menu.rs)
            if gamedata::resources::in_use() != gamedata::resources::Provider::Own {
                self.menu_scene = crate::menuscene::MenuScene::load(&self.data);
            }
        }
        upload_menu_scene(&mut renderer, self.menu_scene.as_mut(), &self.data);
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

    /// Loads a level in place of the current one: a new run, its models uploaded anew.
    fn switch(&mut self, source: crate::assets::Source) -> Result<(), String> {
        self.game.save_attempt();
        let mut o = self.opts.clone();
        let number = match source {
            crate::assets::Source::Original(n) => Some(n),
            _ => None,
        };
        o.source = source;
        (o.replay, o.from, o.sector, o.at, o.ball) = (None, None, None, None, None);
        self.data = self.data.for_level(self.data.level_dir(&o.source).as_deref());
        let mut game = Game::new(&o, &self.data)?;
        game.start_session();
        self.game = game;
        if let Some(a) = &mut self.audio {
            a.stop_all();
            a.forget_sounds();
        }
        self.rolling = [None; 3 + crate::ballsound::FAN_SLOTS];
        self.heard = 0;
        self.started = false;
        self.music = None;
        self.number = number;
        self.prev = None;
        self.keys = 0;
        if let Some(gpu) = &mut self.gpu {
            gpu.renderer.models.clear();
            upload(&mut gpu.renderer, &self.game, &self.data);
            upload_menu_scene(&mut gpu.renderer, self.menu_scene.as_mut(), &self.data);
        }
        if let Some(w) = &self.window {
            w.set_title(&self.game.title());
        }
        Ok(())
    }

    /// The sounds of what happened in the run since the last frame, as the original's Sound_Manager plays them on
    /// the game's messages (Sound.nmo, Simple Sound Messages).
    fn sound_events(&mut self) {
        let Some(audio) = &mut self.audio else {
            self.game.sound_out.clear();
            return;
        };
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
            let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos());
            let replay = self.game.replay_in.as_ref();
            let seed = replay.map_or_else(|| now() as u64, |r| r.level_hash ^ r.end.rotate_left(32));
            self.music = Some(Music::new(self.game.music_theme, self.game.last_level, seed, sim.tick));
            // the sounds' picks from the same seed, their pools' other files decoded ahead
            audio.seed_sounds(seed.rotate_left(29) ^ 0x5851_f42d_4c95_7f2d);
            audio.preload_sound_pools(&self.data);
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
        let tools = self.tools() && self.game.replay_in.is_none();
        self.game.sim.endless_lives = tools && self.features.has(crate::features::Feature::EndlessLives);
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
        if self.history.back().is_none_or(|(s, _)| tick >= s.tick + HISTORY_EVERY) {
            self.history.push_back((self.game.sim.clone(), self.game.replay_out.clone()));
            if self.history.len() > HISTORY_KEPT {
                self.history.pop_front();
            }
        }
    }

    /// The run starts again: its music and its start sound too.
    fn restart(&mut self) {
        self.game.restart();
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
            self.load(crate::assets::Source::Original(n));
        }
    }

    /// Loads a level after the loading screen has been drawn.
    fn load(&mut self, source: crate::assets::Source) {
        self.menu = None;
        self.loading = Some((source, false));
        if let Some(a) = &mut self.audio {
            a.play(&self.data, "Menu_load", 1.0, 0.0);
        }
    }

    /// Opens the menu with `base` as its first screen: the main menu, or the pause, dead or end menu over the level.
    fn open_menu(&mut self, base: crate::menu::Screen) {
        let mut menu = Menu::new(&self.data, &self.strings, base, self.number, self.features);
        menu.volumes = self.volumes;
        menu.graphics = self.graphics;
        menu.bindings = self.bindings;
        menu.resolutions = self.resolutions.clone();
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
        let lines: Vec<(String, String)> = g.lines().into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
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
        upload(&mut r, &self.game, &self.data);
        let target = r.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("video"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
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
        let end = self.game.replay_in.as_ref().map(|r| r.end).ok_or("--video needs --replay")?;
        let lost = self.strings.ours("ball lost");
        let screen = crate::ui::Screen { w: w as f32, h: h as f32 };
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
                    a.play(&self.data, "Music_thunder", 1.0, crate::lights::THUNDER_DELAY);
                }
            }
            let sim = &self.game.sim;
            r.point = crate::lights::point(&sim.events, sim.tick, snap.ball);
            let g = &self.game;
            let mut inst = scene::instances(
                &g.placements,
                &sim.taken,
                &sim.world().bodies,
                &g.ball_meshes,
                g.trafo.as_ref().map(|t| &t.1),
                (&sim.extra_views(), g.extra_model.as_ref()),
                &snap,
                &sim.pieces_view(),
            );
            if snap.kind.is_some() {
                scene::shadows(&g.shadow_receivers, snap.ball, &mut inst);
            }
            let particles = g.effects.batches(snap.eye, snap.target);
            r.draw(&view, w, h, &scene::view(&snap), &inst, &particles);
            r.draw_overlay(&view, w, h, &crate::hud::overlay(sim, w, h, false, g.font.is_some()));
            let font = g.font.as_ref();
            let mut quads = crate::hud::interface(sim, font, screen);
            quads.extend(crate::hud::ball_lost(sim, g.ball_lost_at, &lost, font, screen));
            if let (Some(t), Some(f)) = (crate::endscreen::of(sim), font) {
                quads.extend(crate::endscreen::interface(&t, f, &self.strings, screen));
            }
            if let Some((aw, ah, rgba)) = crate::fallback::take_changed() {
                r.add_named_texture(crate::fallback::TEXTURE, aw, ah, rgba);
            }
            r.draw_ui(&view, w, h, &quads);
            let px = read_back(&r, &target, w, h)?;
            let rgb: Vec<u8> = px.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
            sink.write_all(&rgb).map_err(|e| format!("{}: {e}", out.display()))?;
            frames += 1;
        }
        sink.flush().map_err(|e| format!("{}: {e}", out.display()))?;
        // the sound file is written as the mix is let go
        self.audio = None;
        println!("video: {frames} frames of {w}x{h} at 60 a second into {}", out.display());
        Ok(())
    }

    fn tools(&self) -> bool {
        self.features.has(crate::features::Feature::Tools)
    }

    fn run_over(&mut self) {
        if self.menu.is_some() || self.loading.is_some() || self.fly.is_some() {
            return;
        }
        // with a saved moment (extended mode) a lost run waits for F9 or R; Esc opens the dead menu
        if self.game.sim.game_over && !(self.tools() && self.saved.is_some()) {
            self.open_menu(crate::menu::Screen::Dead);
        } else if crate::endscreen::over(&self.game.sim) {
            self.open_menu(crate::menu::Screen::End);
            let counts = self.game.start.is_none() && !self.game.assisted && self.game.replay_in.is_none();
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
        if matches!(code, KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::Escape)
            && let Some(a) = &mut self.audio
        {
            a.play(&self.data, "Menu_click", 1.0, 0.0);
        }
        let Some(menu) = &mut self.menu else { return };
        // a key awaited on the Controls page takes the key pressed
        if menu.waiting() {
            let action = menu.capture(code);
            self.menu_action(action, event_loop);
            return;
        }
        // the name entry takes what is typed
        if menu.screen() == crate::menu::Screen::Entry && !matches!(code, KeyCode::Enter | KeyCode::NumpadEnter) {
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
            KeyCode::Escape => menu.back(),
            _ => Action::None,
        };
        self.menu_action(action, event_loop);
    }

    /// Does what a menu's key asked.
    fn menu_action(&mut self, action: Action, event_loop: &ActiveEventLoop) {
        match action {
            Action::None => {}
            Action::Continue => self.menu = None,
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
                save_settings(self.volumes, self.features, &self.player);
            }
            Action::ToggleDebug | Action::Switch(_) => {
                match action {
                    Action::Switch(f) => self.features.toggle(f),
                    _ => self.features.mode = !self.features.mode,
                }
                // the sound theme follows the mode: the saved choice in the extended mode, the default theme else
                // (taken before the settings are written, which keep the extended mode's choice)
                let theme = if self.features.mode { setting("sound-theme") } else { None };
                gamedata::sound_themes::restore(theme.as_deref());
                if let Some(a) = &mut self.audio {
                    a.forget_sounds();
                }
                save_settings(self.volumes, self.features, &self.player);
                // the field of view is the extended mode's: the classic one plays at the original's
                show_graphics(&load_graphics(), self.features);
                if !self.features.has(crate::features::Feature::Tools) {
                    self.time_scale = 1.0;
                    self.saved = None;
                    self.fly = None;
                }
                if let Some(m) = &mut self.menu {
                    m.set_features(self.features, &self.data);
                }
            }
            Action::Graphics(g) => self.set_graphics(g),
            Action::Bindings(b) => {
                self.bindings = b;
                put_settings(&b.lines());
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

    fn key(&mut self, code: KeyCode, down: bool, typed: Option<&str>, event_loop: &ActiveEventLoop) {
        if self.loading.is_some() {
            return;
        }
        if self.menu.is_some() {
            if down {
                self.menu_key(code, typed, event_loop);
            }
            return;
        }
        if code == KeyCode::F2 && down && self.tools() {
            self.fly = match self.fly {
                Some(_) => None,
                None => Some(FreeCam::at(self.last_view.0, self.last_view.1)),
            };
            self.keys = 0;
            self.fly_keys = 0;
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
        // the keys bound to the game's actions (the Controls page) come before the practice keys
        let bound = self.bindings.bits(code);
        let bit = match code {
            _ if bound != 0 => bound,
            // the run waits under the pause menu; a finished or lost run has its own menu
            KeyCode::Escape if down && !self.game.sim.finished => {
                let lost = self.game.sim.game_over;
                self.open_menu(if lost { crate::menu::Screen::Dead } else { crate::menu::Screen::Pause });
                return;
            }
            KeyCode::KeyR if down && self.tools() => {
                self.restart();
                self.saved = None;
                0
            }
            // the extended profile's practice keys: slower or faster, save the moment, back to it
            KeyCode::BracketLeft | KeyCode::BracketRight if down && self.tools() => {
                let slower = code == KeyCode::BracketLeft;
                self.time_scale = if slower { (self.time_scale / 2.0).max(0.125) } else { (self.time_scale * 2.0).min(1.0) };
                if self.time_scale < 1.0 {
                    self.game.assisted = true;
                }
                0
            }
            KeyCode::F3 if down && self.tools() => {
                // five seconds back, from the moments kept each second; pressed again it goes on further back
                let to = self.game.sim.tick.saturating_sub(REWIND_TICKS);
                while self.history.len() > 1 && self.history.back().is_some_and(|(s, _)| s.tick > to) {
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
                0
            }
            KeyCode::F5 if down && self.tools() => {
                self.saved = Some((self.game.sim.clone(), self.game.replay_out.clone()));
                0
            }
            KeyCode::F9 if down && self.tools() => {
                if let Some((sim, replay)) = &self.saved {
                    // the run goes on from the saved moment: its record is cut back to it
                    self.game.sim = sim.clone();
                    self.game.replay_out = replay.clone();
                    self.game.assisted = true;
                    // what happened after the saved moment did not happen
                    self.game.ball_lost_at = self.game.ball_lost_at.filter(|&t| t <= sim.tick);
                    self.prev = None;
                }
                0
            }
            // the full screen on or off, as the Graphics page sets it
            KeyCode::F11 if down => {
                use crate::options::Fullscreen as Full;
                let on = self.window.as_ref().is_some_and(|w| w.fullscreen().is_some());
                let how = if on { Full::Off } else { Full::Borderless };
                self.set_graphics(crate::options::Graphics { fullscreen: how, ..self.graphics });
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

    fn frame(&mut self) {
        // Immediate presenting does not wait for the monitor, so the frames are capped here at about 60 a second.
        let since = self.last.elapsed();
        if since < FRAME {
            std::thread::sleep(FRAME - since);
        }
        let now = Instant::now();
        // this frame's real step (after the cap's sleep), held to a quarter second after a stall
        let step = (now - self.last).as_secs_f32().min(0.25);
        self.acc = (self.acc + (now - self.last).as_secs_f32() * self.time_scale).min(0.25);
        self.last = now;
        // A level to load is loaded after a frame that says so; the run waits while the menu is open.
        match self.loading.take() {
            Some((source, true)) => {
                if let Err(e) = self.switch(source) {
                    eprintln!("drahtseilakt: {e}");
                    self.open_menu(crate::menu::Screen::Main);
                }
                self.acc = 0.0;
            }
            Some((source, false)) => self.loading = Some((source, true)),
            None => {}
        }
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
            self.prev = Some(Snapshot::of(&self.game.sim));
            // the gamepads press the same keys; the camera's turn inverted as chosen
            self.game.tick(self.bindings.turn(self.keys | self.pad_keys));
            self.keep_history();
            self.acc -= TICK;
            ticks += 1;
        }
        let simulated = started.elapsed();
        self.run_over();
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
                a.play(&self.data, "Music_thunder", 1.0, crate::lights::THUNDER_DELAY);
            }
        }
        let point = crate::lights::point(&sim.events, sim.tick, snap.ball);
        if let Some(c) = &self.fly {
            (snap.eye, snap.target) = c.view();
        }
        self.last_view = (snap.eye, snap.target);
        let tools = self.tools();
        let Some(gpu) = &mut self.gpu else { return };
        let acquire_started = Instant::now();
        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => {
                gpu.surface.configure(&gpu.renderer.device, &gpu.config);
                return;
            }
        };
        let acquired = acquire_started.elapsed();
        let view = frame.texture.create_view(&Default::default());
        let build_started = Instant::now();
        // With a menu up, the original's menu scene is behind it, its camera going round
        let menu_up = self.menu.as_ref().is_some_and(|m| !m.over_level()) || self.loading.is_some();
        let menu_scene = self.menu_scene.as_mut().filter(|m| m.model.is_some() && menu_up);
        let (inst, particles, frame_view) = if let Some(ms) = menu_scene {
            self.menu_ms += f64::from(step) * 1000.0;
            ms.effects.tick_menu(step * 1000.0);
            let (eye, target) = ms.camera((self.menu_ms % f64::from(crate::menuscene::ROUND_MS)) as f32);
            let mut inst = ms.instances(self.menu_ms);
            for mesh in 0..crate::sky::FACE_COUNT {
                inst.push(crate::render::Instance::plain(scene::SKY_MODEL, mesh, glam::Mat4::from_translation(eye)));
            }
            let particles = ms.effects.batches(eye, target);
            (inst, particles, crate::render::View { eye, target, fov_y: ms.fov_y })
        } else {
            let mut inst = scene::instances(
                &self.game.placements,
                &self.game.sim.taken,
                &self.game.sim.world().bodies,
                &self.game.ball_meshes,
                self.game.trafo.as_ref().map(|t| &t.1),
                (&self.game.sim.extra_views(), self.game.extra_model.as_ref()),
                &snap,
                &self.game.sim.pieces_view(),
            );
            if snap.kind.is_some() {
                scene::shadows(&self.game.shadow_receivers, snap.ball, &mut inst);
            }
            let particles = self.game.effects.batches(snap.eye, snap.target);
            (inst, particles, scene::view(&snap))
        };
        let built = build_started.elapsed();
        let draw_started = Instant::now();
        (gpu.renderer.flash, gpu.renderer.point) = if menu_up { (0.0, None) } else { (flash, point) };
        gpu.renderer.draw(&view, gpu.config.width, gpu.config.height, &frame_view, &inst, &particles);
        let drawn = draw_started.elapsed();
        let overlay_started = Instant::now();
        gpu.renderer.draw_overlay(
            &view,
            gpu.config.width,
            gpu.config.height,
            &match (&self.menu, &self.loading) {
                // with the game's font the menus are the original's (drawn below), else the prototype's
                (_, Some(_)) | (Some(_), None) if self.game.font.is_some() => Vec::new(),
                (_, Some((source, _))) => {
                    Menu::loading_overlay(gpu.config.width, gpu.config.height, &loading_name(source))
                }
                (Some(m), None) => m.overlay(gpu.config.width, gpu.config.height),
                (None, None) => {
                    let mut o = crate::hud::overlay(
                        &self.game.sim,
                        gpu.config.width,
                        gpu.config.height,
                        tools,
                        self.game.font.is_some(),
                    );
                    if tools && (self.time_scale < 1.0 || self.saved.is_some()) {
                        let scale = (gpu.config.height as f32 / 240.0).round().max(2.0);
                        let slow = if self.time_scale < 1.0 { format!("SLOW 1/{}", (1.0 / self.time_scale).round()) } else { String::new() };
                        let saved = if self.saved.is_some() { "   SAVED: F9 BACK" } else { "" };
                        crate::hud::text(&mut o, &format!("{slow}{saved}"), scale, scale * 21.0, scale, [0.6, 0.9, 1.0, 1.0]);
                    }
                    if let Some(c) = &self.fly {
                        let scale = (gpu.config.height as f32 / 240.0).round().max(2.0);
                        let line = format!("FREE CAMERA   {:.1} {:.1} {:.1}   F2 BACK", c.pos.x, c.pos.y, c.pos.z);
                        crate::hud::text(&mut o, &line, scale, scale * 11.0, scale, [1.0, 0.85, 0.3, 1.0]);
                    }
                    o
                }
            },
        );
        let screen = crate::ui::Screen { w: gpu.config.width as f32, h: gpu.config.height as f32 };
        let font = self.game.font.as_ref();
        let quads = match (&self.menu, &self.loading, font) {
            (_, Some((source, _)), Some(f)) => {
                Menu::loading_interface(f, screen, &self.strings, &loading_name(source))
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
                q.extend(crate::hud::ball_lost(&self.game.sim, self.game.ball_lost_at, &lost, font, screen));
                if let (Some(t), Some(f)) = (crate::endscreen::of(&self.game.sim), font) {
                    q.extend(crate::endscreen::interface(&t, f, &self.strings, screen));
                }
                q
            }
            _ => Vec::new(),
        };
        if let Some((w, h, rgba)) = crate::fallback::take_changed() {
            gpu.renderer.add_named_texture(crate::fallback::TEXTURE, w, h, rgba);
        }
        gpu.renderer.draw_ui(&view, gpu.config.width, gpu.config.height, &quads);
        let overlaid = overlay_started.elapsed();
        let present_started = Instant::now();
        gpu.renderer.queue.present(frame);
        let presented = present_started.elapsed();
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
        if let Some(w) = &self.window {
            if self.game.sim.tick.is_multiple_of(15) {
                w.set_title(&self.game.title());
            }
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
            screen: crate::ui::Screen { w: gpu.config.width as f32, h: gpu.config.height as f32 },
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
        if self.menu.is_none() || self.loading.is_some() {
            return;
        }
        match button {
            MouseButton::Left => {
                let Some(p) = self.pointer() else { return };
                if let Some(action) = self.menu.as_mut().and_then(|m| m.click(p)) {
                    self.menu_click();
                    self.menu_action(action, event_loop);
                }
            }
            MouseButton::Right | MouseButton::Back => self.menu_key(KeyCode::Escape, None, event_loop),
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
        let shown = self.menu.is_some() && self.loading.is_none();
        if self.cursor_shown != Some(shown)
            && let Some(w) = &self.window
        {
            w.set_cursor_visible(shown);
            self.cursor_shown = Some(shown);
        }
    }
}

impl ApplicationHandler for App {
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
            WindowEvent::RedrawRequested => self.frame(),
            // the mouse in the menus: the cursor chooses (only when it moves), a click presses, the wheel scrolls
            WindowEvent::CursorMoved { position, .. } => {
                let at = Some((position.x, position.y));
                if at != self.cursor {
                    self.cursor = at;
                    self.mouse_moved();
                }
            }
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                self.mouse_button(button, event_loop)
            }
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
        let (old, new) = (root.join("maps/Folder"), root.join("levels/community/Folder"));
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
        assert!(old.join("Level_07/run-2.replay").is_file() && !new.join("Level_07/run-2.replay").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn runs_behind_a_link_are_not_moved() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-runs-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("elsewhere/Level_07")).unwrap();
        std::fs::create_dir_all(root.join("maps")).unwrap();
        std::os::unix::fs::symlink(root.join("elsewhere"), root.join("maps/Folder")).unwrap();
        assert_eq!(move_runs(&root.join("maps/Folder"), &root.join("levels/c/Folder"), "Level_07"), Ok(false));
        assert!(root.join("elsewhere/Level_07").is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_loose_map_and_a_level_folder_of_one_name_keep_their_runs_apart() {
        let community = crate::levels::dir(crate::levels::Kind::Community);
        let (loose, _) = runs_root(&crate::assets::Source::Nmo(community.join("Foo.nmo")));
        let (folder, old) = runs_root(&crate::assets::Source::Nmo(community.join("Foo").join("Foo.nmo")));
        // the loose map's runs are in <loose>/Foo, the folder's in <folder>/Foo: neither inside the other
        assert!(!folder.join("Foo").starts_with(loose.join("Foo")) && !loose.join("Foo").starts_with(&folder));
        assert_eq!(old, Some(replay_dir().join("maps").join("Foo")));
        // a level folder named exactly like a kind's folder (wherever it is moved) shared its old place: not moved
        let kind_name = community.file_name().unwrap().to_owned();
        let (_, none) = runs_root(&crate::assets::Source::Nmo(community.join(&kind_name).join("x.nmo")));
        assert_eq!(none, None);
        let (_, some) = runs_root(&crate::assets::Source::Nmo(community.join("Community-like").join("x.nmo")));
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
        let gates = vec![(4, Vec3::ZERO, crate::sim::BallKind::Paper), (7, Vec3::X, crate::sim::BallKind::Stone)];
        let (p, s, g, d) = failed_tries_out(path, sectors, gates, vec![6], true);
        assert_eq!(p.iter().map(|v| v.x as i32).collect::<Vec<_>>(), vec![0, 1, 2, 6, 7, 8, 9]);
        assert_eq!(s.len(), 7);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].0, 4, "the transformer at old 7 is now at 4");
        assert!(d.is_empty());
        // unfinished, the tries in its last sector are kept for way_on to choose from
        let path: Vec<Vec3> = (0..6).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
        let (p, _, _, d) = failed_tries_out(path, vec![0, 0, 1, 1, 1, 1], Vec::new(), vec![4], false);
        assert_eq!(p.len(), 6);
        assert_eq!(d, vec![4]);
    }
}
