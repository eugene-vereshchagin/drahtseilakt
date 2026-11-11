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
    ALIGN_LEFT, ALIGN_TOP, Anchor, CONTEXT_H, CONTEXT_W, Quad, Screen, Style, TEXT_SCREEN,
    TEXT_WORDWRAP, place,
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
        lang: crate::lang::chosen(crate::app::setting("language").as_deref()),
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
/// One text of the window: its rectangle in the frame, the height of its line, colour, and whether it is centred.
struct Item {
    text: String,
    rect: [f32; 4],
    line: f32,
    color: [f32; 4],
    centre: bool,
}

impl Item {
    fn style(&self) -> Style {
        Style {
            scale: [
                self.line / CONTEXT_H * 16.0 * 0.9,
                self.line / CONTEXT_H * 16.0,
            ],
            spacing: [1.0, 1.0],
            align: if self.centre { 0 } else { ALIGN_LEFT } | ALIGN_TOP,
            flags: TEXT_SCREEN | TEXT_WORDWRAP,
            margins: [0.0; 4],
            color: self.color,
            v2: true,
            shadow: Some(([1.0, 1.0], [0.0, 0.0, 0.0, 0.8])),
        }
    }
}

/// Where the parts sit in the 640x480 frame (rectangles: left, top, right, bottom).
struct Geo {
    panel: [f32; 4],
    title: [f32; 4],
    intro: [f32; 4],
    list_top: f32,
    field: [f32; 4],
    message: [f32; 4],
    import: [f32; 4],
    cancel: [f32; 4],
    hint: [f32; 4],
}

/// A row of the found places, in the 640x480 frame.
const ROW: f32 = 22.0;

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
    /// The menus' language (its code in lang::languages()).
    lang: String,
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
            let m = self.no_ballance(&from);
            self.message = Some((m, false));
            return;
        }
        self.message = Some((self.tr("copying"), true));
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
            Err(e) => self.message = Some((tail(&e, 44), false)),
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

    /// The message for a place with no Ballance in it (the end of a long path only: it tells the most).
    fn no_ballance(&self, from: &std::path::Path) -> String {
        self.tr("no ballance")
            .replace("{path}", &tail(&from.display().to_string(), 24))
    }

    /// A text of the window in the menus' language.
    fn tr(&self, key: &str) -> String {
        crate::lang::Strings::ours_for(&format!("setup {key}"), &self.lang)
    }

    /// Where everything sits in the 640x480 frame. The panel is as high as its content needs (the list has a row
    /// for each place found, one when there is none) and is centred.
    fn geo(&self) -> Geo {
        let rows = self.found.len().clamp(1, SHOWN) as f32;
        let list_top = 138.0;
        let field_top = list_top + rows * ROW + 12.0;
        let button_top = field_top + 58.0;
        let hint_top = button_top + 40.0;
        let bottom = hint_top + 34.0;
        let off = (CONTEXT_H - (bottom - 20.0)) / 2.0 - 20.0;
        let at = |r: [f32; 4]| [r[0], r[1] + off, r[2], r[3] + off];
        Geo {
            panel: at([24.0, 20.0, 616.0, bottom]),
            title: at([40.0, 32.0, 600.0, 62.0]),
            intro: at([44.0, 68.0, 596.0, 136.0]),
            list_top: list_top + off,
            field: at([40.0, field_top, 600.0, field_top + 26.0]),
            message: at([44.0, field_top + 30.0, 596.0, field_top + 50.0]),
            import: at([150.0, button_top, 300.0, button_top + 32.0]),
            cancel: at([340.0, button_top, 490.0, button_top + 32.0]),
            hint: at([40.0, hint_top, 600.0, hint_top + 28.0]),
        }
    }

    /// The window's content in the game's menu style, in the 640x480 frame: filled rectangles (rectangle, colour)
    /// and texts, in the menus' language.
    fn scene(&self) -> (Vec<([f32; 4], [f32; 4])>, Vec<Item>) {
        let g = self.geo();
        let mut fills = Vec::new();
        let mut items = Vec::new();
        let rim = [0.55, 0.57, 0.62, 1.0];
        let inset = |r: [f32; 4], d: f32| [r[0] + d, r[1] + d, r[2] - d, r[3] - d];
        let mut say = |text: String, rect: [f32; 4], line: f32, color: [f32; 4], centre: bool| {
            items.push(Item {
                text,
                rect,
                line,
                color,
                centre,
            });
        };
        fills.push((g.panel, rim));
        fills.push((inset(g.panel, 3.0), [0.07, 0.07, 0.1, 1.0]));
        say(self.tr("title"), g.title, 28.0, GOLD, true);
        say(self.tr("intro"), g.intro, 15.0, WHITE, false);
        let status = if self.searching.is_some() {
            Some("searching")
        } else if self.found.is_empty() {
            Some("nothing")
        } else {
            None
        };
        if let Some(k) = status {
            let y = g.list_top;
            say(self.tr(k), [44.0, y, 596.0, y + ROW], 15.0, GREY, false);
        }
        for (row, (i, (p, kind))) in self
            .found
            .iter()
            .enumerate()
            .skip(self.first())
            .take(SHOWN)
            .enumerate()
        {
            let y = g.list_top + row as f32 * ROW;
            let on = i == self.chosen && self.typed.is_empty();
            if on {
                fills.push(([40.0, y, 600.0, y + ROW - 2.0], [0.45, 0.3, 0.18, 0.75]));
            }
            let what = match kind {
                Found::Installed => "game",
                Found::Disc => "cd",
                Found::Image => "image",
            };
            say(
                self.tr(what),
                [48.0, y + 1.0, 130.0, y + ROW],
                14.0,
                if on { GOLD } else { GREY },
                false,
            );
            say(
                tail(&p.display().to_string(), 38),
                [134.0, y + 1.0, 596.0, y + ROW],
                14.0,
                if on { GOLD } else { WHITE },
                false,
            );
        }
        fills.push((g.field, rim));
        fills.push((inset(g.field, 2.0), [0.02, 0.02, 0.03, 1.0]));
        let label = self.tr("path");
        let shown = if self.typed.is_empty() {
            format!("{label} ")
        } else {
            format!("{label} {}_", tail(&self.typed, 36))
        };
        let f = g.field;
        say(
            shown,
            [f[0] + 8.0, f[1] + 3.0, f[2] - 4.0, f[3]],
            15.0,
            if self.typed.is_empty() { GREY } else { GOLD },
            false,
        );
        if let Some((m, ok)) = &self.message {
            say(
                m.clone(),
                g.message,
                14.0,
                if *ok { WHITE } else { RED },
                false,
            );
        }
        for (r, label, main) in [(g.import, "import", true), (g.cancel, "cancel", false)] {
            let hot = self.hover == Some(r);
            let body = match (main, hot) {
                (true, true) => [0.6, 0.38, 0.2, 1.0],
                (true, false) => [0.45, 0.3, 0.18, 1.0],
                (false, true) => [0.3, 0.3, 0.36, 1.0],
                (false, false) => [0.16, 0.16, 0.2, 1.0],
            };
            fills.push((r, [0.7, 0.72, 0.78, 1.0]));
            fills.push((inset(r, 2.0), body));
            let key = if main { "import" } else { "cancel" };
            let text = if main {
                crate::lang::Strings::ours_for(key, &self.lang)
            } else {
                self.tr(label)
            };
            say(text, [r[0], r[1] + 5.0, r[2], r[3]], 20.0, WHITE, true);
        }
        say(self.tr("keys"), g.hint, 13.0, GREY, true);
        (fills, items)
    }

    /// The window's picture: the scene in window pixels.
    fn frame(&self, w: f32, h: f32) -> (Vec<Rect>, Vec<Quad>) {
        let screen = Screen { w, h };
        let mut rects = vec![Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            color: [0.06, 0.06, 0.09, 1.0],
        }];
        let (fills, items) = self.scene();
        for (r, color) in fills {
            let [x0, y0, x1, y1] = screen.rect(r, Anchor::Center);
            rects.push(Rect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
                color,
            });
        }
        let mut quads = Vec::new();
        let own = crate::fallback::available();
        for it in items {
            if own {
                quads.extend(place(
                    crate::fallback::styled(&it.text, it.rect, &it.style()),
                    screen,
                    Anchor::Center,
                ));
            } else {
                // no shipped fonts: the built-in pixel font, upper case
                let k = screen.rect(it.rect, Anchor::Center);
                let sc = ((k[3] - k[1]).min(it.line * 2.0) / 9.0).floor().max(1.0);
                let t = it.text.to_uppercase();
                let x = if it.centre {
                    (k[0] + k[2] - width(&t, sc)) / 2.0
                } else {
                    k[0]
                };
                text(&mut rects, &t, x, k[1], sc, it.color);
            }
        }
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
        let g = self.geo();
        [g.import, g.cancel]
            .into_iter()
            .find(|r| x >= r[0] && x <= r[2] && y >= r[1] && y <= r[3])
    }

    /// A left click: a button, or a row of the found places.
    fn click(&mut self, event_loop: &ActiveEventLoop) {
        if self.imported || self.pending.is_some() {
            return;
        }
        match self.over() {
            Some(r) if r == self.geo().cancel => self.finish(Outcome::Back, event_loop),
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
                let row = ((y - self.geo().list_top) / ROW).floor();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(lang: &str) -> Setup {
        Setup {
            size: (640, 480),
            window: None,
            gpu: None,
            searching: None,
            pending: None,
            found: Vec::new(),
            chosen: 0,
            typed: String::new(),
            message: None,
            imported: false,
            lang: lang.to_owned(),
            outcome: Outcome::Back,
            cursor: (0.0, 0.0),
            hover: None,
            error: None,
        }
    }

    /// Every text of the window fits its rectangle in every language and in every state of the window (searching,
    /// nothing found, a full list, a long typed path, a long message), and the panel fits the frame.
    #[test]
    fn every_text_of_the_window_fits_in_every_language() {
        if !crate::fallback::available() {
            eprintln!("skipped: the shipped fonts (assets/fonts) are not there");
            return;
        }
        let long =
            "/home/a-player/Games/Ballance/with/a/very/long/path/to/the/original/copy".to_owned();
        let mut checked = 0;
        for &(lang, _) in crate::lang::languages() {
            for state in 0..5 {
                let mut s = blank(lang);
                match state {
                    0 => {}
                    1 => s.found = vec![(PathBuf::from(&long), Found::Image)],
                    2 => {
                        s.found = (0..12)
                            .map(|i| {
                                (
                                    PathBuf::from(format!("{long}{i}")),
                                    [Found::Installed, Found::Disc, Found::Image][i % 3],
                                )
                            })
                            .collect();
                        s.chosen = 5;
                    }
                    3 => {
                        s.typed = long.repeat(3);
                        s.message = Some((s.no_ballance(std::path::Path::new(&long)), false));
                    }
                    _ => s.message = Some((s.tr("copying"), true)),
                }
                let g = s.geo();
                assert!(
                    g.panel[1] >= 0.0 && g.panel[3] <= CONTEXT_H,
                    "{lang} {state}: the panel fits the frame"
                );
                let (_, items) = s.scene();
                for it in items {
                    let quads = crate::fallback::styled(&it.text, it.rect, &it.style());
                    for q in quads {
                        let [x0, y0, x1, y1] = q.rect;
                        let tol = 1.5;
                        assert!(
                            x0 >= it.rect[0] - tol
                                && x1 <= it.rect[2] + tol
                                && y0 >= it.rect[1] - tol
                                && y1 <= it.rect[3] + tol,
                            "{lang} state {state}: {:?} leaves its rectangle {:?}: glyph {:?}",
                            it.text,
                            it.rect,
                            q.rect
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 1000);
    }

    #[test]
    fn the_window_is_smaller_without_places() {
        let mut s = blank("en");
        let empty = s.geo().panel;
        s.found = (0..8)
            .map(|i| (PathBuf::from(format!("/p{i}")), Found::Disc))
            .collect();
        let full = s.geo().panel;
        assert!(full[3] - full[1] > empty[3] - empty[1]);
    }
}
