// The first start without the game (docs/drahtseilakt/levels.md): a window that brings the player's own Ballance
// into levels/original, from an installed game, the CD in its drive or an image of it. What it finds on the machine
// is listed to choose with the arrows; a folder or image dropped on the window, or a path typed, is taken too. When
// the copy is made the game starts again with it (winit runs one event loop a process). The machine is searched on
// a thread of its own, so a slow drive never keeps the window from opening; the window is drawn only when something
// changed, and the copying starts after a frame that says so.

use crate::app::{Gpu, window_gpu};
use crate::hud::{Rect, text, width};
use crate::levels::{self, Found};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Opens the import window; true when the original was imported (the caller starts the game again).
pub fn run(size: (u32, u32)) -> Result<bool, String> {
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || drop(tx.send(levels::candidates())));
    let mut s = Setup {
        size,
        window: None,
        gpu: None,
        searching: Some(rx),
        pending: None,
        found: Vec::new(),
        chosen: 0,
        typed: String::new(),
        message: None,
        imported: false,
        error: None,
    };
    event_loop.run_app(&mut s).map_err(|e| e.to_string())?;
    match s.error {
        Some(e) => Err(e),
        None => Ok(s.imported),
    }
}

/// The longest path typed.
const TYPED_MAX: usize = 1024;
/// Found places shown at once.
const SHOWN: usize = 8;

struct Setup {
    size: (u32, u32),
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    /// The search of the machine while it runs, and an import to start after the next frame (which says so).
    searching: Option<Receiver<Vec<(PathBuf, Found)>>>,
    pending: Option<PathBuf>,
    found: Vec<(PathBuf, Found)>,
    chosen: usize,
    typed: String,
    /// The last import's outcome, shown under the list.
    message: Option<(String, bool)>,
    imported: bool,
    error: Option<String>,
}

impl Setup {
    /// Asks for an import from `from`: the next frame says it is copying, then it is done.
    fn ask(&mut self, from: PathBuf) {
        if self.imported || self.pending.is_some() {
            return;
        }
        if levels::what(&from).is_none() {
            let m = format!("NO BALLANCE THERE: {}", from.display());
            self.message = Some((m, false));
            return;
        }
        self.message = Some(("COPYING...".into(), true));
        self.pending = Some(from);
    }

    /// Imports from `from`; on success the window closes and the game starts again.
    fn import(&mut self, from: PathBuf, event_loop: &ActiveEventLoop) {
        match levels::import(&from) {
            Ok(n) => {
                levels::prepare(&crate::lang::docs_language(
                    crate::app::setting("language").as_deref(),
                ));
                println!(
                    "{n} files copied to {}",
                    levels::dir(levels::Kind::Original).display()
                );
                self.imported = true;
                event_loop.exit();
            }
            Err(e) => self.message = Some((e, false)),
        }
    }

    fn key(&mut self, code: KeyCode, typed: Option<&str>, event_loop: &ActiveEventLoop) {
        if self.imported || self.pending.is_some() {
            return;
        }
        let n = self.found.len();
        match code {
            KeyCode::Escape => event_loop.exit(),
            KeyCode::ArrowUp if n > 0 => self.chosen = (self.chosen + n - 1) % n,
            KeyCode::ArrowDown if n > 0 => self.chosen = (self.chosen + 1) % n,
            KeyCode::Backspace => drop(self.typed.pop()),
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let from = if self.typed.trim().is_empty() {
                    self.found.get(self.chosen).map(|(p, _)| p.clone())
                } else {
                    Some(typed_path(&self.typed))
                };
                if let Some(p) = from {
                    self.ask(p);
                }
            }
            _ => {
                for c in typed
                    .unwrap_or_default()
                    .chars()
                    .filter(|c| !c.is_control())
                {
                    if self.typed.len() < TYPED_MAX {
                        self.typed.push(c);
                    }
                }
            }
        }
    }

    /// The window's lines: what is needed, what was found, the typed path, the last outcome and the keys.
    fn overlay(&self, w: f32, h: f32) -> Vec<Rect> {
        let mut out = vec![Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            color: [0.05, 0.05, 0.07, 1.0],
        }];
        let s = (h / 300.0).floor().max(1.0);
        let line = 11.0 * s;
        let x = 6.0 * s;
        let mut y = 4.0 * line / 2.0;
        // a line that does not fit loses the start of its last part (a path's end tells the most); the part before
        // a tab stays whole
        let say = |out: &mut Vec<Rect>, t: &str, scale: f32, color: [f32; 4], y: &mut f32| {
            let room = ((w - 2.0 * x) / (6.0 * scale)).max(16.0) as usize;
            let (head, tail) = t.split_once('\t').unwrap_or(("", t));
            let room = room.saturating_sub(head.chars().count()).max(8);
            let chars: Vec<char> = tail.chars().collect();
            let tail: String = if chars.len() > room {
                std::iter::once('.')
                    .chain(chars.iter().skip(chars.len() - room + 1).copied())
                    .collect()
            } else {
                tail.to_owned()
            };
            text(out, &format!("{head}{tail}"), x, *y, scale, color);
            *y += line * scale / s;
        };
        say(&mut out, "DRAHTSEILAKT", s * 2.0, GOLD, &mut y);
        y += line;
        for l in [
            "THE GAME PLAYS YOUR OWN COPY OF BALLANCE. BRING IT IN ONCE:",
            "DROP ITS FOLDER OR CD IMAGE (.CUE, .ISO) ON THIS WINDOW,",
            "CHOOSE ONE FOUND ON THIS COMPUTER, OR TYPE ITS PATH.",
        ] {
            say(&mut out, l, s, WHITE, &mut y);
        }
        y += line;
        if self.searching.is_some() {
            say(&mut out, "LOOKING ON THIS COMPUTER...", s, GREY, &mut y);
        } else if self.found.is_empty() {
            say(&mut out, "NOTHING FOUND ON THIS COMPUTER.", s, GREY, &mut y);
        }
        let first = self
            .chosen
            .saturating_sub(SHOWN / 2)
            .min(self.found.len().saturating_sub(SHOWN));
        for (i, (p, kind)) in self.found.iter().enumerate().skip(first).take(SHOWN) {
            let what = match kind {
                Found::Installed => "GAME ",
                Found::Disc => "CD   ",
                Found::Image => "IMAGE",
            };
            let mark = if i == self.chosen && self.typed.is_empty() {
                ">"
            } else {
                " "
            };
            let color = if i == self.chosen && self.typed.is_empty() {
                GOLD
            } else {
                WHITE
            };
            say(
                &mut out,
                &format!("{mark} {what} \t{}", p.display()),
                s,
                color,
                &mut y,
            );
        }
        y += line;
        say(
            &mut out,
            &format!("PATH: \t{}_", self.typed),
            s,
            if self.typed.is_empty() { GREY } else { GOLD },
            &mut y,
        );
        y += line;
        if let Some((m, ok)) = &self.message {
            for part in m.split(": ") {
                say(
                    &mut out,
                    &part.to_uppercase(),
                    s,
                    if *ok { WHITE } else { RED },
                    &mut y,
                );
            }
        }
        let keys = "UP/DOWN CHOOSE   ENTER COPY   ESC QUIT";
        text(
            &mut out,
            keys,
            (w - width(keys, s)) / 2.0,
            h - 2.0 * line,
            s,
            GREY,
        );
        out
    }

    fn draw(&mut self) {
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
        let (w, h) = (gpu.config.width, gpu.config.height);
        let rects = self.overlay(w as f32, h as f32);
        if let Some(gpu) = &self.gpu {
            gpu.renderer.draw_overlay(&view, w, h, &rects);
            gpu.renderer.queue.present(frame);
        }
    }
}

/// A typed path as the player means it: quotes around it (Windows' "copy as path") dropped, `~` their home folder.
fn typed_path(t: &str) -> PathBuf {
    let t = t.trim().trim_matches('"').trim_matches('\'');
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    match (t.strip_prefix("~/").or_else(|| t.strip_prefix("~\\")), home) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(t),
    }
}

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GREY: [f32; 4] = [0.6, 0.6, 0.6, 1.0];
const GOLD: [f32; 4] = [1.0, 0.85, 0.3, 1.0];
const RED: [f32; 4] = [1.0, 0.5, 0.45, 1.0];

impl ApplicationHandler for Setup {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Drahtseilakt: bring in your Ballance")
            .with_inner_size(winit::dpi::PhysicalSize::new(self.size.0, self.size.1));
        let made = event_loop
            .create_window(attrs)
            .map_err(|e| e.to_string())
            .map(Arc::new);
        match made.and_then(|w| window_gpu(&w).map(|g| (w, g))) {
            Ok((w, g)) => {
                self.window = Some(w);
                self.gpu = Some(g);
            }
            Err(e) => {
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
            WindowEvent::DroppedFile(path) => self.ask(path),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        text,
                        ..
                    },
                ..
            } => self.key(code, text.as_deref(), event_loop),
            WindowEvent::RedrawRequested => {
                self.draw();
                // the frame saying it copies is up: now the copying
                if let Some(from) = self.pending.take() {
                    self.import(from, event_loop);
                    self.redraw();
                }
                return;
            }
            _ => {}
        }
        self.redraw();
    }

    /// While the machine is searched, a look every tenth of a second for its result.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(rx) = &self.searching else { return };
        match rx.try_recv() {
            Ok(found) => {
                self.found = found;
                self.searching = None;
                self.redraw();
            }
            Err(TryRecvError::Disconnected) => {
                self.searching = None;
                self.redraw();
            }
            Err(TryRecvError::Empty) => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    Instant::now() + Duration::from_millis(100),
                ));
                return;
            }
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

impl Setup {
    fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
