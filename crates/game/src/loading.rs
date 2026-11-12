// The loading screen and the start-up's title screen. A level's load reports where it is through the functions here
// (`stage`, `textures_planned`, ...: global counters, so any part of the load can say what it is doing without
// plumbing); the window's frames read them (`fraction`, `text`). What is seen is a picture traced by a shader
// (titlescene.rs) with the screens' words over it (`Art`), drawn at the window's own size.

use crate::lang::Strings;
use crate::ui::{Quad, Screen};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering::Relaxed};

/// Where a level's load is: the stages in the order they come.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// The level's files are read and parsed.
    Reading = 0,
    /// The simulation and the scene are built.
    Building = 1,
    /// The shapes are rounded (smoothing the level's meshes).
    Rounding = 2,
    /// The pictures are decoded and put on the card.
    Textures = 3,
    /// Materials, meshes and pipelines on the card.
    Finishing = 4,
}

static STAGE: AtomicU8 = AtomicU8::new(0);
static TEX_TOTAL: AtomicUsize = AtomicUsize::new(0);
static TEX_DONE: AtomicUsize = AtomicUsize::new(0);
/// How far the finishing stage is, in thousandths.
static FINISH: AtomicUsize = AtomicUsize::new(0);

/// A level's load begins.
pub fn begin() {
    STAGE.store(Stage::Reading as u8, Relaxed);
    TEX_TOTAL.store(0, Relaxed);
    TEX_DONE.store(0, Relaxed);
    FINISH.store(0, Relaxed);
}

/// The load has come to `s` (a stage that is already past is not gone back to).
pub fn stage(s: Stage) {
    STAGE.fetch_max(s as u8, Relaxed);
}

/// The load found `n` textures to put on the card.
pub fn textures_planned(n: usize) {
    TEX_TOTAL.store(n, Relaxed);
}

/// A texture is on the card (or given up).
pub fn texture_done() {
    TEX_DONE.fetch_add(1, Relaxed);
}

/// The finishing stage is `permille` thousandths done.
pub fn finishing(permille: usize) {
    FINISH.store(permille.min(1000), Relaxed);
}

fn current() -> Stage {
    match STAGE.load(Relaxed) {
        0 => Stage::Reading,
        1 => Stage::Building,
        2 => Stage::Rounding,
        3 => Stage::Textures,
        _ => Stage::Finishing,
    }
}

/// How far the load is, 0 to 1: the stages weigh what they cost on a typical map.
pub fn fraction() -> f32 {
    let (done, total) = (TEX_DONE.load(Relaxed), TEX_TOTAL.load(Relaxed));
    let tex = if total == 0 {
        0.0
    } else {
        (done as f32 / total as f32).min(1.0)
    };
    let f = match current() {
        Stage::Reading => 0.02,
        Stage::Building => 0.08,
        Stage::Rounding => 0.14,
        Stage::Textures => 0.2 + 0.55 * tex,
        Stage::Finishing => 0.75 + 0.25 * (FINISH.load(Relaxed) as f32 / 1000.0),
    };
    f.clamp(0.0, 1.0)
}

/// What the load is doing, in the menus' language ("Textures 57/174...").
pub fn text(strings: &Strings) -> String {
    match current() {
        Stage::Reading => strings.ours("loading reading"),
        Stage::Building => strings.ours("loading building"),
        Stage::Rounding => strings.ours("loading rounding"),
        Stage::Textures => strings
            .ours("loading textures")
            .replace("{n}", &TEX_DONE.load(Relaxed).to_string())
            .replace("{total}", &TEX_TOTAL.load(Relaxed).to_string()),
        Stage::Finishing => strings.ours("loading finishing"),
    }
}

/// A line of the screens' text: the textures it is kept in, and how it is set.
struct Spec {
    name: &'static str,
    /// The texture of its glow and shadow, and their reach in capital heights.
    glow: Option<(&'static str, f32)>,
    weight: f32,
    widen: f32,
    tracking: f32,
}

const TITLE: Spec = Spec {
    name: "Art_Title",
    glow: Some(("Art_TitleGlow", 0.42)),
    weight: 800.0,
    widen: 1.28,
    tracking: 0.11,
};
const NAME: Spec = Spec {
    name: "Art_Name",
    glow: Some(("Art_NameGlow", 0.42)),
    weight: 700.0,
    widen: 1.22,
    tracking: 0.09,
};
const CREDIT: Spec = Spec {
    name: "Art_Credit",
    glow: None,
    weight: 500.0,
    widen: 1.2,
    tracking: 0.16,
};
const DOING: Spec = Spec {
    name: "Art_Stage",
    glow: None,
    weight: 500.0,
    widen: 1.2,
    tracking: 0.14,
};

/// The stage line's type case: the characters a stage's text may need, set well apart (`CASE_GAP` of an em) so that
/// each can be cut out of the one texture. The count in the line then runs without a texture being made: each
/// texture made costs a frame of the load a wait behind the loading thread's uploads.
const CASE_DIGITS: &str = "0123456789";
const CASE_GAP: f32 = 0.5;

/// A line as it was last drawn.
struct Made {
    glyphs: Vec<crate::fallback::Set>,
    key: String,
    w: u32,
    h: u32,
    base: f32,
    /// The glow's picture: its size, how many pixels each of its covers, its margin in its own pixels.
    glow: Option<(u32, u32, u32, u32)>,
}

/// How a placed line is coloured: its letters from `top` to `bottom`, its glow, its shadow.
struct Ink {
    top: [f32; 4],
    bottom: [f32; 4],
    glow: [f32; 4],
    shadow: [f32; 4],
}

/// The title and loading screens' words, and the loading screen's progress as it is shown. The words are drawn at
/// the size they have in the window (fallback::line), each line into a texture of its own that is made again only
/// when its text or the window's size changes: sharp at any resolution, where the interface's atlas would be scaled.
#[derive(Default)]
pub struct Art {
    made: std::collections::HashMap<&'static str, Made>,
    /// The size each line was fitted to: (what was asked, the capitals' height it got).
    fitted: std::collections::HashMap<&'static str, (String, f32)>,
    uploads: Vec<(&'static str, u32, u32, Vec<u8>)>,
    shown: f32,
    last: Option<std::time::Instant>,
    /// How many lines may still be drawn anew this frame: one, so that a screen coming up spreads its lines over
    /// its first frames (each texture made waits behind the loading thread's uploads; a line missing for a frame
    /// of the fade-in is not seen, a frame half a second long is).
    budget: u32,
}

fn white(cover: &[u8]) -> Vec<u8> {
    cover.iter().flat_map(|&c| [255, 255, 255, c]).collect()
}

fn tint(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

impl Art {
    /// The textures made since the last call (name, width, height, RGBA): the renderer takes them before it draws.
    pub fn uploads(&mut self) -> Vec<(&'static str, u32, u32, Vec<u8>)> {
        std::mem::take(&mut self.uploads)
    }

    /// The progress to show for the load's `target`: it runs after it by the clock, so the ball rolls evenly
    /// however the frames and the load's steps fall. `fresh`: the screen has just come up.
    pub fn progress(&mut self, target: f32, fresh: bool) -> f32 {
        let now = std::time::Instant::now();
        let dt = self
            .last
            .map_or(0.0, |l| now.duration_since(l).as_secs_f32())
            .min(0.25);
        self.last = Some(now);
        let target = target.clamp(0.0, 1.0);
        if fresh || target < self.shown - 0.2 {
            self.shown = 0.0;
        }
        // most of the way in a third of a second, and never slower than a crawl once the goal is ahead
        let step = (target - self.shown) * (1.0 - (-dt * 5.0).exp());
        self.shown =
            (self.shown + step.max((dt * 0.02).min(target - self.shown).max(0.0))).min(1.0);
        self.shown
    }

    fn ensure(&mut self, spec: &Spec, text: &str, cap: f32) -> Option<&Made> {
        let key = format!("{cap:.0}|{text}");
        if self.made.get(spec.name).is_none_or(|m| m.key != key) {
            if self.budget == 0 {
                // next frame; what was there for another text or size is not shown meanwhile
                self.made.remove(spec.name);
                return None;
            }
            self.budget -= 1;
            let Some(line) =
                crate::fallback::line(text, cap.round(), spec.weight, spec.widen, spec.tracking)
            else {
                self.made.remove(spec.name);
                return None;
            };
            let glow = spec.glow.map(|(name, reach)| {
                let radius = (cap * reach).max(2.0);
                let step = ((radius / 3.0).round() as u32).clamp(1, 16);
                let margin = (2.2 * radius / step as f32).ceil() as u32 + 1;
                let (w, h, cover) = crate::fallback::soften(&line, radius, step, margin);
                self.uploads.retain(|u| u.0 != name);
                self.uploads.push((name, w, h, white(&cover)));
                (w, h, step, margin)
            });
            self.uploads.retain(|u| u.0 != spec.name);
            self.uploads
                .push((spec.name, line.w, line.h, white(&line.cover)));
            self.made.insert(
                spec.name,
                Made {
                    glyphs: line.glyphs,
                    key,
                    w: line.w,
                    h: line.h,
                    base: line.base,
                    glow,
                },
            );
        }
        self.made.get(spec.name)
    }

    /// Puts `text` centred on `cx` with its baseline at `y` and capitals `cap` pixels high, no wider than `fit`.
    #[allow(clippy::too_many_arguments)]
    fn put(
        &mut self,
        out: &mut Vec<Quad>,
        spec: &Spec,
        text: &str,
        cx: f32,
        y: f32,
        cap: f32,
        fit: f32,
        ink: &Ink,
    ) {
        if text.trim().is_empty() || ink.top[3] <= 0.0 {
            return;
        }
        let mut cap = cap.round().max(6.0);
        // a line too wide for the window is set smaller (its width goes with its size): measured once
        let asked = format!("{cap:.0}|{fit:.0}|{text}");
        match self.fitted.get(spec.name) {
            Some((key, c)) if *key == asked => cap = *c,
            _ => {
                let wide = crate::fallback::line(text, cap, spec.weight, spec.widen, spec.tracking)
                    .map_or(0.0, |l| l.w as f32);
                if wide > fit && fit > 0.0 {
                    cap = (cap * fit / wide).floor().max(6.0);
                }
                self.fitted.insert(spec.name, (asked, cap));
            }
        }
        let Some(m) = self.ensure(spec, text, cap) else {
            return;
        };
        let (x0, y0) = ((cx - m.w as f32 / 2.0).round(), (y - m.base).round());
        let quad = |rect: [f32; 4],
                    color_top: [f32; 4],
                    color_bottom: [f32; 4],
                    texture: &'static str| Quad {
            angle: 0.0,
            rect,
            uv: [0.0, 0.0, 1.0, 1.0],
            top: color_top,
            bottom: color_bottom,
            texture,
        };
        if let (Some((name, _)), Some((gw, gh, step, margin))) = (spec.glow, m.glow) {
            let (gx, gy) = (x0 - (margin * step) as f32, y0 - (margin * step) as f32);
            let rect = [gx, gy, gx + (gw * step) as f32, gy + (gh * step) as f32];
            let drop = (cap * 0.07).max(1.0);
            let lower = [rect[0], rect[1] + drop, rect[2], rect[3] + drop];
            // the shadow twice: a soft blur covers little, and the words must stand on a bright sky too
            out.push(quad(lower, ink.shadow, ink.shadow, name));
            out.push(quad(lower, ink.shadow, ink.shadow, name));
            out.push(quad(rect, ink.glow, ink.glow, name));
        }
        let rect = [x0, y0, x0 + m.w as f32, y0 + m.h as f32];
        if spec.glow.is_none() {
            // a plain shadow: the line itself, dark, a little down and to the right
            let d = (cap * 0.06).max(1.0).round();
            let lower = [rect[0] + d, rect[1] + d, rect[2] + d, rect[3] + d];
            out.push(quad(lower, ink.shadow, ink.shadow, spec.name));
        }
        out.push(quad(rect, ink.top, ink.bottom, spec.name));
    }

    /// Sets `text` centred on `at.0` with its baseline at `at.1` out of the stage line's type case: one texture for
    /// all the characters of the text and the digits, made again only when the text brings a new character.
    fn typeset(
        &mut self,
        out: &mut Vec<Quad>,
        text: &str,
        at: (f32, f32),
        cap: f32,
        color: [f32; 4],
        shadow: [f32; 4],
    ) {
        if text.trim().is_empty() || color[3] <= 0.0 {
            return;
        }
        let cap = cap.round().max(6.0);
        let mut case: Vec<char> = text
            .chars()
            .chain(CASE_DIGITS.chars())
            .filter(|c| !c.is_control())
            .collect();
        case.sort_unstable();
        case.dedup();
        let case: String = case.into_iter().collect();
        let spec = Spec {
            tracking: CASE_GAP,
            ..DOING
        };
        let Some(m) = self.ensure(&spec, &case, cap) else {
            return;
        };
        // the pen's steps: each character's advance and the line's own tracking (of an em, which is near enough
        // 1.4 capitals high in the interface's face)
        let gap = DOING.tracking * cap * 1.4;
        let find = |c: char| m.glyphs.iter().find(|g| g.c == c);
        let width: f32 = text
            .chars()
            .filter_map(find)
            .map(|g| g.advance + gap)
            .sum::<f32>()
            - gap;
        let (mut x, y0) = (at.0 - width / 2.0, (at.1 - m.base).round());
        let d = (cap * 0.06).max(1.0).round();
        let mut letters = Vec::new();
        for g in text.chars().filter_map(find) {
            if let Some((from, to, off)) = g.ink {
                let left = (x + off).round();
                let rect = [left, y0, left + (to - from) as f32, y0 + m.h as f32];
                let uv = [from as f32 / m.w as f32, 0.0, to as f32 / m.w as f32, 1.0];
                let quad = |rect: [f32; 4], c: [f32; 4]| Quad {
                    angle: 0.0,
                    rect,
                    uv,
                    top: c,
                    bottom: c,
                    texture: DOING.name,
                };
                out.push(quad(
                    [rect[0] + d, rect[1] + d, rect[2] + d, rect[3] + d],
                    shadow,
                ));
                letters.push(quad(rect, color));
            }
            x += g.advance + gap;
        }
        out.extend(letters);
    }

    /// The title screen's words over its picture (titlescene.rs): the game's name, coming up a little after the
    /// picture. `t`: seconds since the screen came up; `fade`: 1 while it stands, down to 0 as it gives way.
    pub fn splash(&mut self, screen: Screen, t: f32, fade: f32) -> Vec<Quad> {
        let (w, h) = (screen.w, screen.h);
        let mut out = Vec::new();
        self.budget = 1;
        let a = ((t - 0.15) / 0.6).clamp(0.0, 1.0);
        let a = a * a * (3.0 - 2.0 * a);
        let rise = (1.0 - a) * h * 0.012;
        self.put(
            &mut out,
            &TITLE,
            "Drahtseilakt",
            w / 2.0,
            h * 0.31 + rise,
            h * 0.092,
            w * 0.86,
            &Ink {
                top: [1.0, 0.97, 0.88, a * fade],
                bottom: [1.0, 0.80, 0.52, a * fade],
                glow: [1.0, 0.58, 0.22, 0.42 * a * fade],
                shadow: [0.04, 0.015, 0.07, 0.55 * a * fade],
            },
        );
        out
    }

    /// The loading screen's words over its picture: the level's name large, who made it under it, and under the
    /// wire what the load is doing. `t`: seconds since the screen came up.
    pub fn loading(
        &mut self,
        screen: Screen,
        name: &str,
        credit: Option<&str>,
        stage: &str,
        t: f32,
    ) -> Vec<Quad> {
        let (w, h) = (screen.w, screen.h);
        let mut out = Vec::new();
        self.budget = 1;
        let a = (t / 0.25).clamp(0.0, 1.0);
        let shadow = [0.03, 0.02, 0.06, 0.6 * a];
        self.put(
            &mut out,
            &NAME,
            name,
            w / 2.0,
            h * 0.40,
            h * 0.062,
            w * 0.88,
            &Ink {
                top: [1.0, 0.98, 0.93, a],
                bottom: [1.0, 0.86, 0.64, a],
                glow: [1.0, 0.62, 0.26, 0.30 * a],
                shadow,
            },
        );
        if let Some(c) = credit {
            let soft = [0.95, 0.90, 0.84, 0.92 * a];
            self.put(
                &mut out,
                &CREDIT,
                c,
                w / 2.0,
                h * 0.475,
                (h * 0.022).max(9.0),
                w * 0.8,
                &Ink {
                    top: soft,
                    bottom: soft,
                    glow: [0.0; 4],
                    shadow,
                },
            );
        }
        let quiet = [0.93, 0.92, 0.96, 0.88 * a];
        self.typeset(
            &mut out,
            stage,
            (w / 2.0, h * 0.715),
            (h * 0.0185).max(8.0),
            quiet,
            tint(shadow, 0.9),
        );
        out
    }
}

/// What a loading thread hands over, in the order it makes it.
pub enum Item {
    /// Model `.0`'s texture `.1`, already on the card.
    View(usize, usize, Option<wgpu::TextureView>),
    /// Model `.0`'s meshes, packed.
    Packed(usize, crate::render::PackedMeshes),
    /// All the textures are sent.
    Finished(crate::render::TextureStats),
}

enum Feed {
    Thread(std::sync::mpsc::Receiver<Item>),
}

/// A set of models going up to the card a slice a frame: textures as they are read, then each model's materials,
/// buffers and pipelines. The caller adds what is its own (a level's extra textures) and puts the models in place.
pub struct Uploader {
    feed: Feed,
    /// The textures' views by model, and the meshes packed by the thread.
    slots: Vec<std::collections::HashMap<usize, Option<wgpu::TextureView>>>,
    packed: Vec<Option<crate::render::PackedMeshes>>,
    textures_sent: bool,
    began: bool,
    /// A level begins a count of its textures and a model set of its own; the menu's scene shares nothing before it.
    level: bool,
    builds: Vec<crate::render::ModelBuild>,
    total_builds: usize,
    pub staged: Vec<crate::render::Model>,
    pub started: std::time::Instant,
    pub lost: bool,
    /// What the thread's uploads cost.
    pub stats: Option<crate::render::TextureStats>,
}

impl Uploader {
    /// Textures arrive from a loading thread.
    pub fn threaded(rx: std::sync::mpsc::Receiver<Item>, level: bool) -> Uploader {
        Self::with(Feed::Thread(rx), level)
    }

    fn with(feed: Feed, level: bool) -> Uploader {
        Uploader {
            feed,
            slots: Vec::new(),
            packed: Vec::new(),
            textures_sent: false,
            began: false,
            level,
            builds: Vec::new(),
            total_builds: 0,
            staged: Vec::new(),
            started: std::time::Instant::now(),
            lost: false,
            stats: None,
        }
    }

    fn slot(
        &mut self,
        mi: usize,
    ) -> &mut std::collections::HashMap<usize, Option<wgpu::TextureView>> {
        while self.slots.len() <= mi {
            self.slots.push(Default::default());
        }
        &mut self.slots[mi]
    }

    /// Whether every texture is on the card.
    pub fn textures_done(&self) -> bool {
        self.textures_sent
    }

    /// Takes textures up to `deadline`.
    pub fn pump_textures(&mut self, r: &mut crate::render::Renderer, deadline: std::time::Instant) {
        if !self.began {
            self.began = true;
            if self.level {
                r.begin_level();
            } else {
                r.end_sharing();
            }
        }
        while !self.textures_done() {
            let item = match &mut self.feed {
                Feed::Thread(rx) => match rx.try_recv() {
                    Ok(item) => item,
                    Err(std::sync::mpsc::TryRecvError::Empty) => return,
                    Err(_) => {
                        self.lost = true;
                        return;
                    }
                },
            };
            match item {
                Item::Finished(stats) => {
                    self.textures_sent = true;
                    self.stats = Some(stats);
                }
                Item::View(mi, i, v) => {
                    self.slot(mi).insert(i, v);
                }
                Item::Packed(mi, p) => {
                    while self.packed.len() <= mi {
                        self.packed.push(None);
                    }
                    self.packed[mi] = Some(p);
                }
            }
            if std::time::Instant::now() >= deadline {
                return;
            }
        }
    }

    /// The model builds are made once the textures are up and the levels are known.
    pub fn make_builds(&mut self, levels: &[&level::Level]) {
        for (mi, l) in levels.iter().enumerate() {
            let mut views = vec![None; l.textures.len()];
            if let Some(map) = self.slots.get_mut(mi) {
                for (i, v) in map.drain() {
                    if let Some(slot) = views.get_mut(i) {
                        *slot = v;
                    }
                }
            }
            let packed = self.packed.get_mut(mi).and_then(Option::take);
            self.builds
                .push(crate::render::ModelBuild::new(l, views, packed));
        }
        self.builds.reverse();
        self.total_builds = self.builds.len();
    }

    pub fn builds_made(&self) -> bool {
        self.total_builds > 0
    }

    /// Builds models up to `deadline`; true when they all are (in `staged`).
    pub fn pump_models(
        &mut self,
        r: &mut crate::render::Renderer,
        deadline: std::time::Instant,
    ) -> bool {
        while let Some(b) = self.builds.last_mut() {
            match b.step(r, Some(deadline)) {
                Some(m) => {
                    self.builds.pop();
                    self.staged.push(m);
                    finishing(self.staged.len() * 700 / self.total_builds.max(1));
                }
                None => return false,
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shown_progress_runs_after_the_load_and_never_back() {
        let mut art = Art::default();
        assert_eq!(
            art.progress(0.5, true),
            0.0,
            "the first frame has no time behind it"
        );
        let mut last = 0.0;
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            let p = art.progress(0.5, false);
            assert!(p >= last && p <= 0.5 + 1e-6, "{p} after {last}");
            last = p;
        }
        assert!(last > 0.2, "{last}: most of the way in a third of a second");
        // a small step back of the goal (it never happens within a load) does not move the ball back
        assert!(art.progress(0.45, false) >= last);
        // a new load starts from the wire's beginning
        assert_eq!(art.progress(0.0, true), 0.0);
        assert!(art.progress(f32::NAN, false).is_finite());
    }

    #[test]
    fn words_without_fonts_or_text_draw_nothing() {
        let mut art = Art::default();
        let screen = Screen {
            w: 1280.0,
            h: 720.0,
        };
        assert!(art.loading(screen, "", None, " ", 1.0).is_empty());
        // before the screen has come up its words are not there
        assert!(
            art.loading(screen, "Name", Some("Author"), "Stage", 0.0)
                .is_empty()
        );
    }
}
