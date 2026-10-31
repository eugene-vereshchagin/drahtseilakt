// Running the game: in a window (keyboard or a replay drives the simulation), or without one (replay checks and
// screenshots). The simulation always advances in fixed 60 Hz ticks; the window draws at the display's rate and
// interpolates between the last two ticks.

use crate::Options;
use crate::assets::{Data, LoadedLevel};
use crate::render::Renderer;
use crate::replay::Replay;
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
    sim: Sim,
    replay_in: Option<Replay>,
    replay_out: Replay,
    record: Option<PathBuf>,
    /// The tick the ball was last lost, for the "ball lost" message.
    ball_lost_at: Option<u64>,
    /// Looks for stuck balls and launches; see `watch.rs`.
    watch: Watch,
    /// The ball's spin magnitude at the last tick (for the spin-loss dump in `log_tick`).
    prev_spin: f32,
    /// The session log of a windowed run: the ball and its contacts at every tick (see `start_session`).
    session: Option<std::io::BufWriter<std::fs::File>>,
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
        let sim = Sim::new(&level.level, start_ball);
        let (placements, modules) = scene::placements(&level.level, data, sim.loose());
        Ok(Game {
            placements,
            modules,
            ball_meshes: BallMeshes::find(&balls.level),
            sim,
            start_ball,
            replay_out: Replay::new(&level.name, level.hash),
            record: o.record.clone(),
            ball_lost_at: None,
            watch: Watch::default(),
            session: None,
            prev_spin: 0.0,
            replay_in,
            level,
            balls,
        })
    }

    fn restart(&mut self) {
        self.save_attempt();
        self.sim = Sim::new(&self.level.level, self.start_ball);
        self.ball_lost_at = None;
        self.watch = Watch::default();
        self.replay_out = Replay::new(&self.level.name, self.level.hash);
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
            "{} {} ball {:.4} {:.4} {:.4} vel {:.3} {:.3} {:.3} spin {:.3} {:.3} {:.3} {} input {:#04x} {:?}",
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
        let dir = replay_dir().join(&stem);
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
        let events_before = self.sim.events.len();
        self.sim.step(input);
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
        let dir = replay_dir().join(&stem);
        let mut targets = vec![dir.join(format!("{}-{}.replay", stem, self.sim.tick))];
        if let Some(r) = &self.record {
            targets.push(r.clone());
        }
        let best = dir.join("best.replay");
        let best_ticks = std::fs::read_to_string(&best)
            .ok()
            .and_then(|t| Replay::parse(&t).ok())
            .map(|r| r.end);
        if best_ticks.is_none_or(|b| self.sim.tick < b) {
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

fn replay_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("drahtseilakt/replays")
}

pub fn run(o: Options, data: Data) -> Result<(), String> {
    if let Some(dir) = &o.export {
        let loaded = data.load(&o.source)?;
        return crate::assets::export(&data, &loaded, dir);
    }
    let mut game = Game::new(&o, &data)?;
    if let Some(path) = &o.screenshot {
        return screenshot(&mut game, &data, &o, path);
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
    let mut app = App {
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
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    app.game.save_attempt();
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
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
    while game.sim.tick < ticks && !game.sim.finished && !game.sim.game_over {
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
        &snap,
    );
    r.draw(&view, w, h, &scene::view(&snap), &inst);
    r.draw_overlay(
        &view,
        w,
        h,
        &crate::hud::overlay(&game.sim, w, h, game.ball_lost_at),
    );

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

struct Gpu {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

struct App {
    game: Game,
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
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let attrs = Window::default_attributes()
            .with_title(self.game.title())
            .with_inner_size(winit::dpi::PhysicalSize::new(self.size.0, self.size.1));
        let window = Arc::new(event_loop.create_window(attrs).map_err(|e| e.to_string())?);
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
        let mut renderer = Renderer::new(device, queue, format);
        upload(&mut renderer, &self.game, &self.data);
        self.gpu = Some(Gpu {
            surface,
            config,
            renderer,
        });
        self.window = Some(window);
        self.last = Instant::now();
        Ok(())
    }

    fn key(&mut self, code: KeyCode, down: bool, event_loop: &ActiveEventLoop) {
        let bit = match code {
            KeyCode::ArrowUp => input::UP,
            KeyCode::ArrowDown => input::DOWN,
            KeyCode::ArrowLeft => input::LEFT,
            KeyCode::ArrowRight => input::RIGHT,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => input::SHIFT,
            KeyCode::Space => input::LIFT,
            KeyCode::Escape if down => {
                event_loop.exit();
                0
            }
            KeyCode::KeyR if down => {
                self.game.restart();
                self.prev = None;
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
        self.acc = (self.acc + (now - self.last).as_secs_f32()).min(0.25);
        self.last = now;
        let started = Instant::now();
        let mut ticks = 0u32;
        while self.acc >= TICK {
            self.prev = Some(Snapshot::of(&self.game.sim));
            self.game.tick(self.keys);
            self.acc -= TICK;
            ticks += 1;
        }
        let simulated = started.elapsed();
        let cur = Snapshot::of(&self.game.sim);
        let snap = Snapshot::lerp(&self.prev.unwrap_or(cur), &cur, self.acc / TICK);
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
        let inst = scene::instances(
            &self.game.placements,
            &self.game.sim.taken,
            &self.game.sim.world().bodies,
            &self.game.ball_meshes,
            &snap,
        );
        let built = build_started.elapsed();
        let draw_started = Instant::now();
        gpu.renderer.draw(
            &view,
            gpu.config.width,
            gpu.config.height,
            &scene::view(&snap),
            &inst,
        );
        let drawn = draw_started.elapsed();
        let overlay_started = Instant::now();
        gpu.renderer.draw_overlay(
            &view,
            gpu.config.width,
            gpu.config.height,
            &crate::hud::overlay(
                &self.game.sim,
                gpu.config.width,
                gpu.config.height,
                self.game.ball_lost_at,
            ),
        );
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
                        ..
                    },
                ..
            } => self.key(code, state == ElementState::Pressed, event_loop),
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
