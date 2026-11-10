// The first start without the game (docs/drahtseilakt/levels.md): a window that brings the player's own Ballance
// into levels/original, from an installed game, the CD in its drive or an image of it. What it finds on the machine
// is listed to choose with the arrows; a folder or image dropped on the window, or a path typed, is taken too. When
// the copy is made the game starts again with it (winit runs one event loop a process). The machine is searched on
// a thread of its own, so a slow drive never keeps the window from opening; the window is drawn only when something
// changed, and the copying starts after a frame that says so. It looks like the game's menus (the interface's own
// font, a panel, buttons) and can be left three ways: Esc or the Cancel button go back to the game's menu, the
// window's close button ends the program, an import starts the game with the copy.

use crate::app::{Gpu, window_gpu};
use crate::hud::{Rect, text, width};
use crate::levels::{self, Found};
use crate::ui::{
    ALIGN_LEFT, ALIGN_TOP, Anchor, CONTEXT_H, CONTEXT_W, Quad, Screen, Style, TEXT_SCREEN, place,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// How the window was left.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// The original was imported: the game starts again with it.
    Imported,
    /// Esc or Cancel: back to the game's menu (the game starts again as it was).
    Back,
    /// The window's close button: the program ends, no game window follows.
    Quit,
}

/// Opens the import window and says how it was left.
pub fn run(size: (u32, u32)) -> Result<Outcome, String> {
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
        outcome: Outcome::Back,
        cursor: (0.0, 0.0),
        hover: None,
        error: None,
    };
    event_loop.run_app(&mut s).map_err(|e| e.to_string())?;
    match s.error {
        Some(e) => Err(e),
        None if s.imported => Ok(Outcome::Imported),
        None => Ok(s.outcome),
    }
}

/// The longest path typed.
const TYPED_MAX: usize = 1024;
/// Found places shown at once.
const SHOWN: usize = 8;
/// A row of the found places, in the 640x480 frame.
const ROW: f32 = 22.0;
/// The path field and the two buttons, in the 640x480 frame (left, top, right, bottom).
const FIELD: [f32; 4] = [40.0, 340.0, 600.0, 366.0];
const IMPORT_BUTTON: [f32; 4] = [150.0, 396.0, 300.0, 428.0];
const CANCEL_BUTTON: [f32; 4] = [340.0, 396.0, 490.0, 428.0];

/// The end of `t` when it is longer than `n` characters (a path's end tells the most), marked with a dot.
fn tail(t: &str, n: usize) -> String {
    let chars: Vec<char> = t.chars().collect();
    if chars.len() <= n {
        return t.to_owned();
    }
    std::iter::once('.')
        .chain(chars[chars.len() - (n - 1)..].iter().copied())
        .collect()
}

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
    outcome: Outcome,
    /// The pointer in window pixels, and the button it is over.
    cursor: (f32, f32),
    hover: Option<[f32; 4]>,
    error: Option<String>,
}

impl Setup {
    /// Asks for an import from `from`: the next frame says it is copying, then it is done.
    fn ask(&mut self, from: PathBuf) {
        if self.imported || self.pending.is_some() {
            return;
        }
        if levels::what(&from).is_none() {
            let m = format!("No Ballance there: {}", from.display());
            self.message = Some((m, false));
            return;
        }
        self.message = Some(("Copying...".into(), true));
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
            KeyCode::Escape => self.finish(Outcome::Back, event_loop),
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

    /// The window's picture in the game's menu style: a dark panel over a plain backdrop, the interface's own font,
    /// the found places as rows, a path field and two buttons (Import, Cancel). Rectangles are in pixels of the
    /// window, text quads too; both are laid out in the interface's 640x480 frame, centred in the window.
    fn frame(&self, w: f32, h: f32) -> (Vec<Rect>, Vec<Quad>) {
        let screen = Screen { w, h };
        let mut rects = vec![Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            color: [0.06, 0.06, 0.09, 1.0],
        }];
        let mut quads = Vec::new();
        let own = crate::fallback::available();
        // a rectangle of the frame (context pixels)
        let fill = |r: [f32; 4], color: [f32; 4], rects: &mut Vec<Rect>| {
            let [x0, y0, x1, y1] = screen.rect(r, Anchor::Center);
            rects.push(Rect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
                color,
            });
        };
        let mut say = |t: &str,
                       r: [f32; 4],
                       line: f32,
                       color: [f32; 4],
                       centre: bool,
                       rects: &mut Vec<Rect>| {
            if own {
                let st = Style {
                    scale: [line / CONTEXT_H * 16.0 * 0.9, line / CONTEXT_H * 16.0],
                    spacing: [1.0, 1.0],
                    align: if centre { 0 } else { ALIGN_LEFT } | ALIGN_TOP,
                    flags: TEXT_SCREEN,
                    margins: [0.0; 4],
                    color,
                    v2: true,
                    shadow: Some(([1.0, 1.0], [0.0, 0.0, 0.0, 0.8])),
                };
                quads.extend(place(
                    crate::fallback::styled(t, r, &st),
                    screen,
                    Anchor::Center,
                ));
            } else {
                // no shipped fonts: the built-in pixel font, upper case
                let k = screen.rect(r, Anchor::Center);
                let sc = ((k[3] - k[1]).min(line * 2.0) / 9.0).floor().max(1.0);
                let t = t.to_uppercase();
                let x = if centre {
                    (k[0] + k[2] - width(&t, sc)) / 2.0
                } else {
                    k[0]
                };
                text(rects, &t, x, k[1], sc, color);
            }
        };
        // the panel with the interface's rim
        fill(
            [24.0, 20.0, 616.0, 460.0],
            [0.55, 0.57, 0.62, 1.0],
            &mut rects,
        );
        fill(
            [27.0, 23.0, 613.0, 457.0],
            [0.07, 0.07, 0.1, 1.0],
            &mut rects,
        );
        say(
            "Bring in your Ballance",
            [40.0, 32.0, 600.0, 62.0],
            28.0,
            GOLD,
            true,
            &mut rects,
        );
        for (i, l) in [
            "The game plays your own copy of Ballance. Bring it in once:",
            "drop its folder or CD image (.cue, .iso) on this window,",
            "choose one found on this computer, or type its path.",
        ]
        .into_iter()
        .enumerate()
        {
            let y = 72.0 + i as f32 * 17.0;
            say(
                l,
                [44.0, y, 596.0, y + 17.0],
                15.0,
                WHITE,
                false,
                &mut rects,
            );
        }
        let status = if self.searching.is_some() {
            Some("Looking on this computer...")
        } else if self.found.is_empty() {
            Some("Nothing found on this computer.")
        } else {
            None
        };
        if let Some(t) = status {
            say(
                t,
                [44.0, 134.0, 596.0, 152.0],
                15.0,
                GREY,
                false,
                &mut rects,
            );
        }
        let first = self.first();
        for (row, (i, (p, kind))) in self
            .found
            .iter()
            .enumerate()
            .skip(first)
            .take(SHOWN)
            .enumerate()
            .map(|(r, x)| (r, x))
        {
            let y = 134.0 + row as f32 * ROW;
            let on = i == self.chosen && self.typed.is_empty();
            if on {
                fill(
                    [40.0, y, 600.0, y + ROW - 2.0],
                    [0.45, 0.3, 0.18, 0.75],
                    &mut rects,
                );
            }
            let what = match kind {
                Found::Installed => "Game",
                Found::Disc => "CD",
                Found::Image => "Image",
            };
            say(
                what,
                [48.0, y + 1.0, 110.0, y + ROW],
                14.0,
                if on { GOLD } else { GREY },
                false,
                &mut rects,
            );
            say(
                &tail(&p.display().to_string(), 62),
                [112.0, y + 1.0, 596.0, y + ROW],
                14.0,
                if on { GOLD } else { WHITE },
                false,
                &mut rects,
            );
        }
        // the path field
        fill(FIELD, [0.55, 0.57, 0.62, 1.0], &mut rects);
        fill(
            [
                FIELD[0] + 2.0,
                FIELD[1] + 2.0,
                FIELD[2] - 2.0,
                FIELD[3] - 2.0,
            ],
            [0.02, 0.02, 0.03, 1.0],
            &mut rects,
        );
        let shown = if self.typed.is_empty() {
            "Path: ".to_owned()
        } else {
            format!("Path: {}_", tail(&self.typed, 56))
        };
        say(
            &shown,
            [FIELD[0] + 8.0, FIELD[1] + 3.0, FIELD[2] - 4.0, FIELD[3]],
            15.0,
            if self.typed.is_empty() { GREY } else { GOLD },
            false,
            &mut rects,
        );
        if let Some((m, ok)) = &self.message {
            say(
                &tail(m, 70),
                [44.0, 372.0, 596.0, 392.0],
                14.0,
                if *ok { WHITE } else { RED },
                false,
                &mut rects,
            );
        }
        for (r, label, main) in [
            (IMPORT_BUTTON, "Import", true),
            (CANCEL_BUTTON, "Cancel", false),
        ] {
            let hot = self.hover == Some(r);
            let body = match (main, hot) {
                (true, true) => [0.6, 0.38, 0.2, 1.0],
                (true, false) => [0.45, 0.3, 0.18, 1.0],
                (false, true) => [0.3, 0.3, 0.36, 1.0],
                (false, false) => [0.16, 0.16, 0.2, 1.0],
            };
            fill(r, [0.7, 0.72, 0.78, 1.0], &mut rects);
            fill(
                [r[0] + 2.0, r[1] + 2.0, r[2] - 2.0, r[3] - 2.0],
                body,
                &mut rects,
            );
            say(
                label,
                [r[0], r[1] + 5.0, r[2], r[3]],
                20.0,
                WHITE,
                true,
                &mut rects,
            );
        }
        say(
            "Up/Down choose   Enter import   Esc back",
            [40.0, 434.0, 600.0, 454.0],
            13.0,
            GREY,
            true,
            &mut rects,
        );
        (rects, quads)
    }

    /// The first found place shown, so the chosen one stays in view.
    fn first(&self) -> usize {
        self.chosen
            .saturating_sub(SHOWN / 2)
            .min(self.found.len().saturating_sub(SHOWN))
    }

    /// The pointer's place in the 640x480 frame.
    fn context(&self, w: f32, h: f32) -> (f32, f32) {
        let k = (h / CONTEXT_H).min(w / CONTEXT_W).max(0.001);
        (
            (self.cursor.0 - (w - CONTEXT_W * k) / 2.0) / k,
            (self.cursor.1 - (h - CONTEXT_H * k) / 2.0) / k,
        )
    }

    /// What the pointer is over: a button's rectangle.
    fn over(&self) -> Option<[f32; 4]> {
        let size = self.window.as_ref()?.inner_size();
        let (w, h) = (size.width, size.height);
        let (x, y) = self.context(w as f32, h as f32);
        [IMPORT_BUTTON, CANCEL_BUTTON]
            .into_iter()
            .find(|r| x >= r[0] && x <= r[2] && y >= r[1] && y <= r[3])
    }

    /// A left click: a button, or a row of the found places.
    fn click(&mut self, event_loop: &ActiveEventLoop) {
        if self.imported || self.pending.is_some() {
            return;
        }
        match self.over() {
            Some(r) if r == CANCEL_BUTTON => self.finish(Outcome::Back, event_loop),
            Some(_) => self.key(KeyCode::Enter, None, event_loop),
            None => {
                let Some((w, h)) = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size())
                    .map(|s| (s.width, s.height))
                else {
                    return;
                };
                let (x, y) = self.context(w as f32, h as f32);
                let row = ((y - 134.0) / ROW).floor();
                if (40.0..=600.0).contains(&x) && row >= 0.0 && (row as usize) < SHOWN {
                    let i = self.first() + row as usize;
                    if i < self.found.len() {
                        self.chosen = i;
                        self.typed.clear();
                    }
                }
            }
        }
    }

    /// Ends the window with `how`.
    fn finish(&mut self, how: Outcome, event_loop: &ActiveEventLoop) {
        self.outcome = how;
        event_loop.exit();
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
        let (rects, quads) = self.frame(w as f32, h as f32);
        if let Some(gpu) = &mut self.gpu {
            if let Some((aw, ah, rgba)) = crate::fallback::take_changed() {
                gpu.renderer
                    .add_named_texture(crate::fallback::TEXTURE, aw, ah, rgba);
            }
            gpu.renderer.draw_overlay(&view, w, h, &rects);
            gpu.renderer.draw_ui(&view, w, h, &quads);
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
const GREY: [f32; 4] = [0.68, 0.68, 0.72, 1.0];
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
            WindowEvent::CloseRequested => self.finish(Outcome::Quit, event_loop),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                let over = self.over();
                if over == self.hover {
                    return;
                }
                self.hover = over;
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } => self.click(event_loop),
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
