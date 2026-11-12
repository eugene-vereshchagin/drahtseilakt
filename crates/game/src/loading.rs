// The loading bar and the start-up splash. A level's load reports where it is through the functions here (`stage`,
// `textures_planned`, ...: global counters, so any part of the load can say what it is doing without plumbing); the
// window's frames read them (`fraction`, `text`) and draw the bar (`bar`). The splash (`splash`) is the title screen
// drawn while the menu's scene, fonts and first pipelines get ready.

use crate::lang::Strings;
use crate::ui::{Anchor, Face, Quad, Screen, Style, homogeneous};
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

const fn style(scale: [f32; 2], color: [f32; 4]) -> Style {
    Style {
        scale,
        spacing: [0.0, 0.0],
        align: 0,
        flags: crate::ui::TEXT_SCREEN,
        margins: [2.0; 4],
        color,
        v2: true,
        shadow: None,
    }
}

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const PLAIN: &str = "plain";

fn plain(rect: [f32; 4], top: [f32; 4], bottom: [f32; 4]) -> Quad {
    Quad {
        angle: 0.0,
        rect,
        uv: [0.0, 0.0, 1.0, 1.0],
        top,
        bottom,
        texture: PLAIN,
    }
}

fn faded(q: Quad, a: f32) -> Quad {
    Quad {
        top: [q.top[0], q.top[1], q.top[2], q.top[3] * a],
        bottom: [q.bottom[0], q.bottom[1], q.bottom[2], q.bottom[3] * a],
        ..q
    }
}

/// A golden ball (the interface's) as horizontal strips: centre and radius in window pixels.
fn ball(out: &mut Vec<Quad>, cx: f32, cy: f32, r: f32) {
    const STRIPS: usize = 28;
    let shade = |t: f32| {
        // t: -1 at the top, 1 at the bottom; lit from the upper left
        let l = (0.95 - 0.38 * (t + 1.0) / 2.0).clamp(0.0, 1.0);
        [0.98 * l + 0.05, 0.76 * l + 0.03, 0.24 * l, 1.0]
    };
    for i in 0..STRIPS {
        let (t0, t1) = (
            i as f32 / STRIPS as f32 * 2.0 - 1.0,
            (i + 1) as f32 / STRIPS as f32 * 2.0 - 1.0,
        );
        let mid = (t0 + t1) / 2.0;
        let half = (1.0 - mid * mid).max(0.0).sqrt() * r;
        out.push(plain(
            [cx - half, cy + t0 * r, cx + half, cy + t1 * r + 0.5],
            shade(t0),
            shade(t1),
        ));
    }
    // the highlight
    out.push(plain(
        [cx - 0.55 * r, cy - 0.6 * r, cx - 0.15 * r, cy - 0.3 * r],
        [1.0, 0.96, 0.8, 0.85],
        [1.0, 0.9, 0.6, 0.55],
    ));
}

/// The loading screen's interface: a dark band over the menu's scene, "Loading {name}", the bar with a ball rolling
/// at the head of what is done, and the stage below it. `t`: seconds since the screen came up.
pub fn bar(
    face: &Face,
    screen: Screen,
    strings: &Strings,
    name: &str,
    fraction: f32,
    stage: &str,
    t: f32,
) -> Vec<Quad> {
    let mut out = Vec::new();
    let appear = (t / 0.25).clamp(0.0, 1.0);
    // the splash's dusk, so the two screens are one thing
    out.push(faded(
        plain(
            [0.0, 0.0, screen.w, screen.h],
            [0.09, 0.09, 0.2, 1.0],
            [0.44, 0.33, 0.45, 1.0],
        ),
        appear,
    ));
    if face.own() {
        out.push(faded(
            Quad {
                angle: 0.0,
                rect: screen.rect(homogeneous([0.0, 0.40, 1.0, 0.64]), Anchor::Center),
                uv: [0.0, 0.99, 1.0, 1.0],
                top: [1.0, 1.0, 1.0, 0.5],
                bottom: [1.0, 1.0, 1.0, 0.5],
                texture: "Button01_special",
            },
            appear,
        ));
    }
    let title = strings.ours("loading").replace("{name}", name);
    out.extend(
        crate::ui::place(
            face.layout(
                &title,
                homogeneous([0.15, 0.415, 0.85, 0.49]),
                &style([0.4, 0.5], WHITE),
            ),
            screen,
            Anchor::Center,
        )
        .into_iter()
        .map(|q| faded(q, appear)),
    );
    // the bar: a dark groove with a light edge, filled with gold up to the ball
    let r = screen.rect(homogeneous([0.25, 0.515, 0.75, 0.545]), Anchor::Center);
    let (h, w) = (r[3] - r[1], r[2] - r[0]);
    let f = fraction.clamp(0.0, 1.0);
    let edge = (h * 0.12).max(1.0);
    out.push(faded(
        plain(
            [r[0] - edge, r[1] - edge, r[2] + edge, r[3] + edge],
            [0.78, 0.74, 0.8, 0.9],
            [0.5, 0.46, 0.55, 0.9],
        ),
        appear,
    ));
    out.push(faded(
        plain(r, [0.07, 0.06, 0.09, 1.0], [0.16, 0.14, 0.17, 1.0]),
        appear,
    ));
    let head = r[0] + w * f;
    if head > r[0] + 1.0 {
        out.push(faded(
            plain(
                [r[0], r[1] + edge * 0.5, head, r[3] - edge * 0.5],
                [1.0, 0.86, 0.45, 1.0],
                [0.78, 0.5, 0.16, 1.0],
            ),
            appear,
        ));
    }
    let radius = h * 0.95;
    let mut balls = Vec::new();
    ball(
        &mut balls,
        (r[0] + radius * 0.2 + (w - radius * 0.4) * f).clamp(r[0], r[2]),
        (r[1] + r[3]) / 2.0,
        radius,
    );
    out.extend(balls.into_iter().map(|q| faded(q, appear)));
    out.extend(
        crate::ui::place(
            face.layout(
                stage,
                homogeneous([0.2, 0.565, 0.8, 0.615]),
                &style([0.3, 0.34], [0.86, 0.86, 0.9, 1.0]),
            ),
            screen,
            Anchor::Center,
        )
        .into_iter()
        .map(|q| faded(q, appear)),
    );
    out
}

/// A point on the splash's rope, `x` 0 to 1 across the window: its place in window pixels. The rope sags between two
/// posts off the window's sides, a little more while the ball is on it.
fn rope(w: f32, h: f32, x: f32, load: f32) -> (f32, f32) {
    let sag = h * (0.05 + 0.025 * load);
    (x * w, h * 0.66 + sag * 4.0 * x * (1.0 - x))
}

/// The splash: the game's name over a rope with a ball walking it, on a dusk of the menu scene's colours. `t`:
/// seconds since it came up; `fade`: 1 while it stands, down to 0 as it gives way to the menu.
pub fn splash(face: &Face, screen: Screen, t: f32, fade: f32) -> Vec<Quad> {
    let (w, h) = (screen.w, screen.h);
    let mut out = Vec::new();
    let appear = (t / 0.5).clamp(0.0, 1.0);
    // the dusk: a gradient to the horizon, then a haze of the sky's colour low down
    out.push(faded(
        plain(
            [0.0, 0.0, w, h],
            [0.09, 0.09, 0.2, 1.0],
            [0.44, 0.33, 0.45, 1.0],
        ),
        fade,
    ));
    out.push(faded(
        plain(
            [0.0, h * 0.7, w, h],
            [0.0, 0.0, 0.0, 0.0],
            [0.05, 0.03, 0.06, 0.55],
        ),
        fade,
    ));
    // stars, a few, twinkling slowly
    for i in 0..40u32 {
        let (a, b) = (
            (i.wrapping_mul(2654435761) >> 8) as f32 / 16777216.0,
            (i.wrapping_mul(40503).wrapping_add(977) % 1000) as f32 / 1000.0,
        );
        let tw = 0.5 + 0.5 * (t * (0.8 + a * 1.7) + a * 40.0).sin();
        let s = (h * 0.003).max(1.0) * (1.0 + (i % 3) as f32 * 0.5);
        let (x, y) = (a * w, b * h * 0.45);
        out.push(faded(
            plain(
                [x, y, x + s, y + s],
                [1.0, 1.0, 1.0, 0.7 * tw],
                [1.0, 1.0, 1.0, 0.7 * tw],
            ),
            fade * appear,
        ));
    }
    // the ball walks the rope back and forth
    let walk = 0.5 - 0.5 * (t * 0.75).cos();
    let bx = 0.12 + 0.76 * walk;
    let load = 1.0 - (2.0 * bx - 1.0).abs();
    let thick = (h * 0.006).max(2.0);
    const SEGMENTS: usize = 64;
    for i in 0..SEGMENTS {
        let (a, b) = (
            rope(w, h, i as f32 / SEGMENTS as f32, load),
            rope(w, h, (i + 1) as f32 / SEGMENTS as f32, load),
        );
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt() + 1.5;
        let (mx, my) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        for (grow, color) in [(1.6, [0.0, 0.0, 0.0, 0.35]), (1.0, [0.62, 0.45, 0.28, 1.0])] {
            out.push(faded(
                Quad {
                    angle: dy.atan2(dx).to_degrees(),
                    ..plain(
                        [
                            mx - len / 2.0,
                            my - thick * grow / 2.0,
                            mx + len / 2.0,
                            my + thick * grow / 2.0,
                        ],
                        color,
                        color,
                    )
                },
                fade * appear,
            ));
        }
    }
    // the ball: turning shows only in its highlight, which slides across as it rolls
    let radius = h * 0.04;
    let (px, py) = rope(w, h, bx, load);
    let mut balls = Vec::new();
    ball(&mut balls, px, py - radius - thick * 0.4, radius);
    out.extend(balls.into_iter().map(|q| faded(q, fade * appear)));
    // the name, its letters coming one after the other
    let title = crate::ui::place(
        face.layout(
            "Drahtseilakt",
            homogeneous([0.0, 0.2, 1.0, 0.42]),
            &style([1.15, 1.35], [1.0, 0.93, 0.78, 1.0]),
        ),
        screen,
        Anchor::Center,
    );
    // centred by what is drawn (the layout's own centring counts the advance of the last letter's gap)
    let (left, right) = title.iter().fold((f32::MAX, f32::MIN), |(l, r), q| {
        (l.min(q.rect[0]), r.max(q.rect[2]))
    });
    let shift = if left < right {
        w / 2.0 - (left + right) / 2.0
    } else {
        0.0
    };
    let title: Vec<Quad> = title
        .into_iter()
        .map(|q| Quad {
            rect: [q.rect[0] + shift, q.rect[1], q.rect[2] + shift, q.rect[3]],
            ..q
        })
        .collect();
    let n = title.len().max(1) as f32;
    for (i, q) in title.into_iter().enumerate() {
        let a = ((t - 0.15 - i as f32 / n * 0.7) / 0.35).clamp(0.0, 1.0);
        let rise = (1.0 - a) * h * 0.02;
        let q = Quad {
            rect: [q.rect[0], q.rect[1] + rise, q.rect[2], q.rect[3] + rise],
            ..q
        };
        out.push(faded(q, fade * a));
    }
    out
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
