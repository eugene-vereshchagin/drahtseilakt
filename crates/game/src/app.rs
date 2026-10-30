// Running the game: in a window (keyboard or a replay drives the simulation), or without one (replay checks and
// screenshots). The simulation always advances in fixed 60 Hz ticks; the window draws at the display's rate and
// interpolates between the last two ticks.

use crate::Options;
use crate::assets::{Data, LoadedLevel};
use crate::render::Renderer;
use crate::replay::Replay;
use crate::scene::{self, BallMeshes, Snapshot};
use crate::sim::{Event, Sim, TICK, input};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

struct Game {
    level: LoadedLevel,
    settings: crate::assets::Settings,
    start_ball: crate::sim::BallKind,
    balls: LoadedLevel,
    ball_meshes: BallMeshes,
    sim: Sim,
    replay_in: Option<Replay>,
    replay_out: Replay,
    record: Option<PathBuf>,
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
        let settings = data.settings(&level.name);
        let start_ball = settings
            .start_ball
            .as_deref()
            .and_then(crate::sim::BallKind::from_name)
            .unwrap_or(crate::sim::BallKind::Wood);
        Ok(Game {
            ball_meshes: BallMeshes::find(&balls.level),
            sim: Sim::new(&level.level, start_ball),
            settings,
            start_ball,
            replay_out: Replay::new(&level.name, level.hash),
            record: o.record.clone(),
            replay_in,
            level,
            balls,
        })
    }

    fn restart(&mut self) {
        self.sim = Sim::new(&self.level.level, self.start_ball);
        self.replay_out = Replay::new(&self.level.name, self.level.hash);
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
        for (t, e) in self.sim.events.iter().skip(events_before) {
            println!("{} {e:?}", clock(*t));
            if *e == Event::Finished {
                self.save_run();
            }
        }
    }

    fn save_run(&self) {
        let text = self.replay_out.to_text();
        let path = self.record.clone().unwrap_or_else(|| {
            replay_dir().join(format!(
                "{}-{}.replay",
                self.level
                    .name
                    .trim_end_matches(".NMO")
                    .trim_end_matches(".nmo"),
                self.sim.tick
            ))
        });
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(&path, text) {
            Ok(()) => println!("run saved to {}", path.display()),
            Err(e) => eprintln!("could not save the run to {}: {e}", path.display()),
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
            "Drahtseilakt — {}   points {}   lives {}   sector {}   {}{state}",
            self.level.name,
            s.points,
            s.lives,
            s.sector + 1,
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
    let mut game = Game::new(&o, &data)?;
    if let Some(path) = &o.screenshot {
        return screenshot(&mut game, &data, &o, path);
    }
    if o.headless {
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
        prev: None,
        error: None,
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
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
    let letter = game.settings.sky.clone().unwrap_or_else(|| "A".into());
    let sky = crate::sky::model(&letter);
    renderer.add_model(&sky, &loader);
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
        let lin = |v: f64| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        renderer.clear = wgpu::Color {
            r: lin(avg(0)),
            g: lin(avg(1)),
            b: lin(avg(2)),
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
    let snap = Snapshot::lerp(&prev, &Snapshot::of(&game.sim), 1.0);
    let (_, device, queue) = pollster::block_on(device(None))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
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
    let inst = scene::instances(&game.level.level, &game.sim.taken, &game.ball_meshes, &snap);
    r.draw(&view, w, h, &scene::view(&snap), &inst);

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
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or(caps.formats.first().copied())
            .ok_or("no surface format")?;
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
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
        let now = Instant::now();
        self.acc = (self.acc + (now - self.last).as_secs_f32()).min(0.25);
        self.last = now;
        while self.acc >= TICK {
            self.prev = Some(Snapshot::of(&self.game.sim));
            self.game.tick(self.keys);
            self.acc -= TICK;
        }
        let cur = Snapshot::of(&self.game.sim);
        let snap = Snapshot::lerp(&self.prev.unwrap_or(cur), &cur, self.acc / TICK);
        let Some(gpu) = &mut self.gpu else { return };
        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => {
                gpu.surface.configure(&gpu.renderer.device, &gpu.config);
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let inst = scene::instances(
            &self.game.level.level,
            &self.game.sim.taken,
            &self.game.ball_meshes,
            &snap,
        );
        gpu.renderer.draw(
            &view,
            gpu.config.width,
            gpu.config.height,
            &scene::view(&snap),
            &inst,
        );
        gpu.renderer.queue.present(frame);
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
        if self.window.is_none() {
            if let Err(e) = self.init(event_loop) {
                self.error = Some(e);
                event_loop.exit();
            }
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
