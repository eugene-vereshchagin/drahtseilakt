// Running the game: in a window (keyboard or a replay drives the simulation), or without one (replay checks and
// screenshots). The simulation always advances in fixed 60 Hz ticks; the window draws at the display's rate and
// interpolates between the last two ticks.

use crate::Options;
use crate::assets::{Data, LoadedLevel};
use crate::render::Renderer;
use crate::replay::Replay;
use crate::audio::Audio;
use crate::ballsound::{BallSound, FanSound, SoundCmd};
use crate::freecam::FreeCam;
use crate::music::Music;
use crate::menu::{Action, Menu};
use crate::scene::{self, BallMeshes, Snapshot};
use crate::collide::closest_point;
use crate::sim::{BALL_RADIUS, Event, Sim, TICK, input};
use crate::watch::{Finding, Watch};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
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
    /// The game's font (Font_1 with its glyph table), when the copy has it.
    font: Option<crate::ui::Font>,
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
        let level = data.load(&o.source)?;
        for w in &level.warnings {
            eprintln!("warning: {w}");
        }
        let balls_path = data
            .file("3D Entities/Balls.nmo")
            .ok_or("Balls.nmo not found in the game data")?;
        let balls = crate::assets::load_nmo(&balls_path)?;
        let replay_in = match &o.replay {
            Some(p) => {
                let text =
                    std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
                let r = Replay::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
                if r.level_hash != level.hash {
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
        // The original's level 1 begins with its tutorial, which stops the level before the ball comes.
        let tutorial = level.name.eq_ignore_ascii_case("Level_01.NMO");
        let ball_meshes = BallMeshes::find(&balls.level);
        let mut sim = Sim::with_modules(&level.level, start_ball, &modules, tutorial);
        sim.set_ball_boxes(ball_meshes.boxes);
        let level_bonus = data.settings(&level.name).level_bonus.unwrap_or(0);
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
        let module_file = |name: &str| {
            let path = data.file(&format!("3D Entities/PH/{name}.nmo"))?;
            crate::assets::load_nmo(&path).ok().map(|m| m.level)
        };
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
            .file("3D Entities/AnimTrafo.nmo")
            .and_then(|p| crate::assets::load_nmo(&p).ok())
            .and_then(|f| crate::trafo::TrafoModel::new(&f.level, trafo_model).map(|m| (f, m)));
        Ok(Game {
            trafo,
            placements,
            modules,
            ball_meshes,
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
            font: crate::ui::Font::game(data),
            shadow_receivers: scene::shadow_receivers(&level.level),
            music_theme: data.settings(&level.name).music,
            level_bonus,
            assisted: false,
            last_level: matches!(o.source, crate::assets::Source::Original(12)),
            runs_root: match &o.source {
                crate::assets::Source::Original(_) => replay_dir(),
                crate::assets::Source::Nmo(p) | crate::assets::Source::Open(p) => {
                    let kind = p.parent().and_then(|d| d.file_name()).map_or_else(|| "other".into(), |n| n.to_owned());
                    replay_dir().join("maps").join(kind)
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
        self.sim = Sim::with_modules(
            &self.level.level,
            self.start_ball,
            &self.modules,
            self.level.name.eq_ignore_ascii_case("Level_01.NMO"),
        );
        self.sim.set_ball_boxes(self.ball_meshes.boxes);
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
    /// inputs stay in the run's replay, so the attempt saved later still replays from the start.
    fn fast_forward(&mut self, tick: u64) {
        let end = self.replay_in.as_ref().map_or(0, |r| r.end).min(tick);
        while self.sim.tick < end && !self.sim.finished && !self.sim.game_over {
            self.tick(0);
        }
        self.replay_in = None;
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
            Ok(()) => println!("attempt saved to {}", path.display()),
            Err(e) => eprintln!("could not save the attempt to {}: {e}", path.display()),
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
                Ok(()) => println!("run saved to {}", path.display()),
                Err(e) => eprintln!("could not save the run to {}: {e}", path.display()),
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
            self.level.name,
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

/// The player's settings file: one `name value` a line (`sound 0.8`, `music 0.5`).
fn settings_path() -> PathBuf {
    crate::levels::data_dir().join("settings.txt")
}

/// The settings file's text: a file larger than any we write is not ours and is ignored; bytes that are not UTF-8
/// are read as replacement characters, so the other lines still count.
fn settings_text() -> String {
    const MAX: u64 = 64 * 1024;
    let small = std::fs::metadata(settings_path()).is_ok_and(|m| m.len() <= MAX);
    let bytes = if small { std::fs::read(settings_path()).unwrap_or_default() } else { Vec::new() };
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    String::from_utf8_lossy(bytes).into_owned()
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

/// A value of the settings file ("key value" lines).
pub fn setting(key: &str) -> Option<String> {
    settings_text()
        .lines()
        .filter_map(|l| l.split_once(' '))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.trim().to_owned())
}

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

/// Saves the settings; the language, which only the file sets, is kept.
/// The extended mode (from the settings, or `--debug` for one run) and the additions the settings turn off.
fn load_features(debug: bool) -> crate::features::Features {
    crate::features::Features::load(debug || load_extended(), setting)
}

fn save_settings(v: [f32; 2], features: crate::features::Features, player: &[u8]) {
    let path = settings_path();
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let profile = if features.mode { "extended" } else { "classic" };
    let mut text = format!("sound {}\nmusic {}\nprofile {profile}\n{}", v[0], v[1], features.lines());
    if !player.is_empty() {
        text.push_str(&format!("player {}\n", crate::lang::latin1(player)));
    }
    // the lines only the file sets (language, the level folders) are kept as they are
    let ours = ["sound", "music", "profile", "player"];
    for line in settings_text().lines() {
        let key = line.split_once(' ').map_or(line, |(k, _)| k);
        if !ours.contains(&key) && !key.starts_with("extended.") && !line.trim().is_empty() {
            text.push_str(line);
            text.push('\n');
        }
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
    let strings = crate::lang::Strings::load(data, &crate::lang::chosen(setting("language").as_deref()));
    let number = match o.source {
        crate::assets::Source::Original(n) => Some(n),
        _ => None,
    };
    let mut m = Menu::new(data, &strings, base, number, load_features(o.debug));
    m.resolution = size;
    m.volumes = load_volumes();
    for k in keys.chars() {
        match k {
            'u' => m.step(-1),
            'd' => m.step(1),
            'l' => drop(m.adjust(-1)),
            'r' => drop(m.adjust(1)),
            'e' => drop(m.enter()),
            'b' => drop(m.back()),
            'n' => m.ask_name(load_player(), crate::endscreen::score(&game.sim)),
            _ => {}
        }
    }
    Some(m)
}

/// What the loading screen says is loading.
fn loading_name(source: &crate::assets::Source) -> String {
    match source {
        crate::assets::Source::Original(n) => format!("level {n}"),
        crate::assets::Source::Nmo(p) | crate::assets::Source::Open(p) => {
            p.file_stem().map_or_else(String::new, |s| s.to_string_lossy().into_owned())
        }
    }
}

pub fn replay_dir() -> PathBuf {
    crate::levels::data_dir().join("replays")
}

pub fn run(o: Options, data: Data) -> Result<(), String> {
    if let Some(dir) = &o.export {
        let loaded = data.load(&o.source)?;
        return crate::assets::export(&data, &loaded, dir);
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
        return screenshot(&mut game, &data, &o, path);
    }
    if o.autopilot {
        return autopilot(&mut game, &o);
    }
    if o.headless {
        game.start_session();
        let end = game
            .replay_in
            .as_ref()
            .map(|r| r.end)
            .ok_or("--headless needs --replay")?;
        while game.sim.tick < end && !game.sim.finished && !game.sim.game_over {
            game.tick(0);
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
        return Ok(());
    }
    if let Some(tick) = o.from {
        game.fast_forward(tick);
    }
    game.start_session();
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Poll);
    // Without a level named, the window opens with the main menu over the first level.
    crate::levels::prepare(&crate::lang::docs_language(setting("language").as_deref()));
    let strings = crate::lang::Strings::load(&data, &crate::lang::chosen(setting("language").as_deref()));
    let features = load_features(o.debug);
    let menu = (!o.chosen && o.replay.is_none()).then(|| {
        let mut m = Menu::new(&data, &strings, crate::menu::Screen::Main, Some(1), features);
        m.volumes = load_volumes();
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
        player: load_player(),
        time_scale: 1.0,
        saved: None,
        fly: None,
        fly_keys: 0,
        last_view: (glam::Vec3::ZERO, glam::Vec3::X),
        audio: Audio::open().map(|mut a| {
            let v = load_volumes();
            a.set_volumes(v[0], v[1]);
            a
        }),
        heard: 0,
        rolling: [None; 3 + crate::ballsound::FAN_SLOTS],
        music: None,
        volumes: load_volumes(),
        game,
        data,
        size: o.size,
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
    let route = crate::autopilot::Route::find(&game.level.level, &game.modules, start, &[]);
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
    let route = crate::autopilot::Route::through(&game.level.level, &game.modules, start, &stages);
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
    pilot_run(&mut game, &own, crate::autopilot::Pilot::new(route, gates))
}

/// The test player follows a player's run: the ball's way in that replay (where it rolled, in order, every 1.5;
/// not while a ball is being lost) is its way, the transformers it went into its gates, and it begins where the run
/// began.
fn follow_run(o: &Options, data: &Data, follow: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(follow).map_err(|e| format!("{}: {e}", follow.display()))?;
    if text.starts_with(crate::locate::TRACK_HEAD) {
        return follow_track(o, data, &text);
    }
    let run = Replay::parse(&text)?;
    let mut watch = o.clone();
    watch.replay = Some(follow.to_path_buf());
    watch.follow = None;
    let mut seen = Game::new(&watch, data)?;
    let mut path: Vec<glam::Vec3> = Vec::new();
    let mut gates: Vec<crate::autopilot::Gate> = Vec::new();
    let mut lost = false;
    while seen.sim.tick < run.end && !seen.sim.finished && !seen.sim.game_over {
        let before = seen.sim.events.len();
        seen.tick(0);
        for (_, e) in seen.sim.events.iter().skip(before) {
            match e {
                crate::sim::Event::Died => lost = true,
                crate::sim::Event::NewBall => lost = false,
                crate::sim::Event::Transform(k) => {
                    let at = seen.sim.ball.position - glam::Vec3::Y * crate::sim::BALL_RADIUS;
                    gates.push((path.len(), at, *k))
                }
                _ => {}
            }
        }
        let b = &seen.sim.ball;
        let floor = b.position - glam::Vec3::Y * crate::sim::BALL_RADIUS;
        if !lost && b.on_ground && path.last().is_none_or(|p| p.distance(floor) >= 1.5) {
            path.push(floor);
        }
    }
    println!("followed: {} points, transformers {:?}", path.len(), gates);
    // the test player's own run begins where the followed one began
    let mut own = o.clone();
    own.replay = None;
    own.follow = None;
    if let Some(st) = &run.start {
        own.sector = Some(st.sector);
        own.ball = st.ball.clone();
        own.at = st.at.map(|[x, y, z]| format!("{x},{y},{z}"));
    }
    let mut game = Game::new(&own, data)?;
    let route = crate::autopilot::Route::along(&game.level.level, &game.modules, path);
    pilot_run(&mut game, &own, crate::autopilot::Pilot::new(route, gates))
}

/// The test player plays the level from where the run begins: it saves the run at --record (or as an attempt) and
/// prints how far it got.
fn autopilot(game: &mut Game, o: &Options) -> Result<(), String> {
    let level = &game.level.level;
    let stages: Vec<glam::Vec3> = game.sim.stages().into_iter().skip(game.sim.sector).collect();
    // The transformers the way passes (within 30) are part of the way, in its order: a player rolls into them
    // rather than around them.
    let start = game.sim.ball.position;
    let base = crate::autopilot::Route::find(level, &game.modules, start, &stages);
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
    // replaces the found order.
    if let Some(plan) = &o.plan {
        waypoints = read_plan(plan, level)?.into_iter().map(|(p, trafo)| (0, p, trafo)).collect();
    }
    let stages: Vec<glam::Vec3> = waypoints.iter().map(|w| w.1).collect();
    let route = crate::autopilot::Route::find(level, &game.modules, start, &stages);
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
    // the transformers' places on the new way: their gates
    // (each with the ball its transformer makes: the level's transformer at that place)
    let kind_at = |p: glam::Vec3| {
        game.sim.trafos().min_by(|a, b| a.0.distance(p).total_cmp(&b.0.distance(p))).map(|t| t.1)
    };
    let gates: Vec<crate::autopilot::Gate> = route
        .marks
        .iter()
        .zip(&waypoints)
        .filter(|(_, w)| w.2)
        .filter_map(|(&m, w)| Some((m, w.1, kind_at(w.1)?)))
        .collect();
    println!("transformers on the way: {:?}", gates);
    pilot_run(game, o, crate::autopilot::Pilot::new(route, gates))
}

/// A plan: one level object a line (`NAME  # note`), in the order the way goes through them; blank lines and
/// `#` comments are skipped. Returns where each is and whether it is a transformer.
fn read_plan(path: &Path, level: &level::Level) -> Result<Vec<(glam::Vec3, bool)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
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
        let trafo = level.groups_of(i).any(|g| g.starts_with("P_Trafo"));
        out.push((glam::Vec3::from(level.objects[i].position()), trafo));
    }
    Ok(out)
}

/// Runs the test player to --ticks (or the finish), prints how far it got and saves its run at --record.
fn pilot_run(game: &mut Game, o: &Options, mut pilot: crate::autopilot::Pilot) -> Result<(), String> {
    let limit = o.ticks.unwrap_or(60 * 60 * 20);
    while game.sim.tick < limit && !game.sim.finished && !game.sim.game_over {
        let i = pilot.next(&game.sim);
        game.tick(i);
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
        if s.finished { "finished" } else if s.game_over { "game over" } else { "not finished" },
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

/// The time of one frame at 60 frames a second.
const FRAME: std::time::Duration = std::time::Duration::from_micros(16_667);

/// How frames are shown. `Immediate` by default: on the test machine the vertical-sync modes (`Fifo`,
/// `FifoRelaxed`) blocked the presenting for about a second per frame, while `Immediate` presents at once and may
/// tear. DRAHTSEILAKT_PRESENT=fifo gives the synchronised mode back, which never tears.
fn present_mode(offered: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    let wanted = match std::env::var("DRAHTSEILAKT_PRESENT").as_deref() {
        Ok("fifo") => wgpu::PresentMode::Fifo,
        Ok("relaxed") => wgpu::PresentMode::FifoRelaxed,
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
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: present_mode(&caps.present_modes),
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
    Ok(Gpu { surface, config, renderer })
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
    for name in crate::particles::textures().into_iter().chain(["Font_1", "Button01_special", "Button01_select", "Button01_deselect"]) {
        let t = level::Texture {
            name: name.to_owned(),
            source: level::TextureSource::File(if name.starts_with("Font_") || name.starts_with("Button") {
                format!("{name}.tga")
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
        &crate::hud::overlay(&game.sim, w, h, game.ball_lost_at, true, game.font.is_some()),
    );
    let screen = crate::ui::Screen { w: w as f32, h: h as f32 };
    let mut quads = crate::hud::interface(&game.sim, game.font.as_ref(), screen);
    if let (Some(t), Some(f)) = (crate::endscreen::of(&game.sim), game.font.as_ref()) {
        quads.extend(crate::endscreen::interface(&t, f, screen));
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
    r.draw_ui(&view, w, h, &quads);

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
            texture: &target,
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
    /// The loops playing, by slot: rolling (stone, wood, metal), then the fans'.
    rolling: [Option<u64>; 3 + crate::ballsound::FAN_SLOTS],
    /// The level's music.
    music: Option<Music>,
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
    /// The keys down now.
    held: std::collections::HashSet<KeyCode>,
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let attrs = Window::default_attributes()
            .with_title(self.game.title())
            .with_inner_size(winit::dpi::PhysicalSize::new(self.size.0, self.size.1));
        let window = Arc::new(event_loop.create_window(attrs).map_err(|e| e.to_string())?);
        let Gpu { surface, config, mut renderer } = window_gpu(&window)?;
        upload(&mut renderer, &self.game, &self.data);
        if self.menu_scene.is_none() {
            self.menu_scene = crate::menuscene::MenuScene::load(&self.data);
        }
        upload_menu_scene(&mut renderer, self.menu_scene.as_mut(), &self.data);
        self.gpu = Some(Gpu {
            surface,
            config,
            renderer,
        });
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
        let mut game = Game::new(&o, &self.data)?;
        game.start_session();
        self.game = game;
        if let Some(a) = &mut self.audio {
            a.stop_all();
        }
        self.rolling = [None; 3 + crate::ballsound::FAN_SLOTS];
        self.heard = 0;
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
            // a new run
            self.heard = 0;
        }
        if self.music.is_none() && sim.tick >= 1 {
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
            self.music = Some(Music::new(self.game.music_theme, self.game.last_level, seed, sim.tick));
        }
        if let Some(m) = &mut self.music {
            m.frame(sim, audio, &self.data);
        }
        if sim.tick == 1 && self.heard == 0 {
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

    /// The run starts again: its music too.
    fn restart(&mut self) {
        self.game.restart();
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
        if let Some(gpu) = &self.gpu {
            menu.resolution = (gpu.config.width, gpu.config.height);
        }
        self.menu = Some(menu);
        self.keys = 0;
    }

    /// A lost run opens the dead menu; a finished one, once its points are counted, the new highscore's name entry
    /// (an original level, begun at its start and not helped) or the end menu.
    /// The practice tools are on: the extended mode with its tools switched on.
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
            Action::ToggleDebug | Action::Switch(_) => {
                match action {
                    Action::Switch(f) => self.features.toggle(f),
                    _ => self.features.mode = !self.features.mode,
                }
                save_settings(self.volumes, self.features, &self.player);
                if !self.features.has(crate::features::Feature::Tools) {
                    self.time_scale = 1.0;
                    self.saved = None;
                    self.fly = None;
                }
                if let Some(m) = &mut self.menu {
                    m.set_features(self.features, &self.data);
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
        let bit = match code {
            KeyCode::ArrowUp => input::UP,
            KeyCode::ArrowDown => input::DOWN,
            KeyCode::ArrowLeft => input::LEFT,
            KeyCode::ArrowRight => input::RIGHT,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => input::SHIFT,
            KeyCode::Space => input::LIFT,
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
            KeyCode::F11 if down => {
                if let Some(w) = &self.window {
                    let full = w.fullscreen().is_some();
                    w.set_fullscreen(if full {
                        None
                    } else {
                        Some(Fullscreen::Borderless(None))
                    });
                }
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
            self.game.tick(self.keys);
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
            );
            if snap.kind.is_some() {
                scene::shadows(&self.game.shadow_receivers, snap.ball, &mut inst);
            }
            let particles = self.game.effects.batches(snap.eye, snap.target);
            (inst, particles, scene::view(&snap))
        };
        let built = build_started.elapsed();
        let draw_started = Instant::now();
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
                        self.game.ball_lost_at,
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
            (_, Some((source, _)), Some(f)) => Menu::loading_interface(f, screen, &loading_name(source)),
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
                if let (Some(t), Some(f)) = (crate::endscreen::of(&self.game.sim), font) {
                    q.extend(crate::endscreen::interface(&t, f, screen));
                }
                q
            }
            _ => Vec::new(),
        };
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
                    gpu.config.width = size.width.max(1);
                    gpu.config.height = size.height.max(1);
                    gpu.surface.configure(&gpu.renderer.device, &gpu.config);
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
            // a key let go while the window was not ours may never be reported released
            WindowEvent::Focused(false) => self.held.clear(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
