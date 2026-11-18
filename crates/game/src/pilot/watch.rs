// The test player in the window (--watch): it plays the run through the game's controller hook
// (control.rs), and the watcher pauses it, changes its speed, takes over by hand (F5) and gives back, skips a hint's
// step (F6), writes a stuck report (F7) and marks zones (F8, F4) that it and the learned table obey from then on.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

use super::run::{PilotRun, hints_path, load_learned, load_zones, stuck_report};
use crate::Options;
use crate::app::{Controller, Ctx, Game};
use crate::app::{clock, level_stem, shown_path};
use crate::scene::{self, Snapshot};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use winit::keyboard::KeyCode;

/// The test player in the window (--watch): its run, and what the watcher does with it.
pub struct PilotWatch {
    run: Box<PilotRun>,
    paused: bool,
    /// How fast the run goes against real time (the run is the same tick by tick).
    speed: f32,
    /// Driven by hand (F5) since this tick, from this place, in this sector, as this ball.
    hand: Option<(u64, glam::Vec3, usize, crate::sim::BallKind)>,
    /// The test player deciding the next tick's input on its own thread (its searches take a while), since when.
    pending: Option<(
        std::sync::mpsc::Receiver<(crate::pilot::autopilot::Pilot, u8)>,
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
    marking: Option<(crate::pilot::zones::Kind, KeyCode, u64, usize, Instant)>,
    /// How far back a mark reaches before its key went down, in seconds ([ and ]).
    look_back: u32,
    /// The zones are shown in the world (F12).
    zones_shown: bool,
    /// The zone marked last, shown a moment so the watcher sees what got marked.
    flash: Option<(crate::pilot::zones::Zone, Instant)>,
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
                    .is_some_and(|m| m.0 == crate::pilot::zones::Kind::Flow),
            ),
            (
                strings.ours("watch careful"),
                self.marking
                    .is_some_and(|m| m.0 == crate::pilot::zones::Kind::Careful),
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
        use crate::pilot::zones::Kind;
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
            Kind::Calm => [0.3, 0.55, 1.0, a],
            Kind::Plain => [0.75, 0.75, 0.75, a],
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
            for p in crate::pilot::zones::thin(&path, MARK_STEP) {
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
    crate::files::write_plain(&file, &game.replay_out.to_text())?;
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
    let mut old = crate::files::read_text(&hints, crate::pilot::hints::MAX_FILE)?;
    if !old.is_empty() && !old.ends_with('\n') {
        old.push('\n');
    }
    old += &line;
    crate::files::write_plain(&hints, &old)?;
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

impl PilotWatch {
    /// The watch of a test player's run, as it begins: playing, at real speed, the keys' box shown.
    pub fn new(run: Box<PilotRun>) -> PilotWatch {
        PilotWatch {
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
        }
    }

    /// One tick of the watched test player: its decision (on its own thread, waited for within this frame's
    /// budget), then the tick. False when the decision is not ready yet (the view holds) or the run is over.
    fn watch_tick(&mut self, ctx: &mut Ctx, started: Instant) -> bool {
        let w = self;
        if !w.run.going(&*ctx.game) {
            w.done = true;
            w.pending = None;
            if let Err(e) = w.run.summary(&*ctx.game, ctx.opts) {
                eprintln!("drahtseilakt: {e}");
            }
            return false;
        }
        if w.pending.is_none() {
            w.run.before(&*ctx.game);
            let (tx, rx) = std::sync::mpsc::channel();
            let (mut p, sim) = (w.run.pilot.clone(), ctx.game.sim.clone());
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
        *ctx.prev = Some(Snapshot::of(&ctx.game.sim));
        let tick = ctx.game.sim.tick;
        w.run.after(&mut *ctx.game, ctx.data, ctx.opts, i);
        // gone back: the view jumps there, and says so
        if ctx.game.sim.tick <= tick {
            *ctx.prev = None;
            if let Some(line) = &w.run.went_back {
                w.notice = Some((format!("BACK: {line}"), Instant::now()));
            }
        }
        true
    }

    /// The watcher's keys (--watch): Space pause, + and - faster and slower, F5 the hands take over and give back,
    /// F6 the hint's step skipped, F7 a stuck report. True when the key was one of them.
    fn watch_key(&mut self, ctx: &mut Ctx, code: KeyCode) -> bool {
        let w = self;
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
                    let s = &ctx.game.sim;
                    w.hand = Some((s.tick, s.ball.position, s.sector, s.ball.kind));
                    w.paused = false;
                    *ctx.keys = 0;
                    w.notice = Some(("YOUR HANDS: the arrows drive, F5 hands back".into(), now));
                }
                Some(hand) => {
                    let text = match save_hand(&*ctx.game, ctx.opts, hand) {
                        Ok(t) => t,
                        Err(e) => format!("not saved: {e}"),
                    };
                    println!("{text}");
                    w.run.resume(&*ctx.game, hand.0);
                    *ctx.keys = 0;
                    w.notice = Some((text, now));
                }
            },
            KeyCode::F6 => {
                w.pending = None;
                w.run.pilot.hints.skip_step(&ctx.game.sim);
                w.notice = Some(("hint step skipped".into(), now));
            }
            // a zone: from the look-back before the key went down to where the ball is when it is let go (a tap: the
            // last seconds; held: on while held)
            KeyCode::F8 | KeyCode::F4 => {
                if w.marking.is_none() {
                    let kind = if code == KeyCode::F8 {
                        crate::pilot::zones::Kind::Flow
                    } else {
                        crate::pilot::zones::Kind::Careful
                    };
                    let s = &ctx.game.sim;
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
                            let mut t = match crate::pilot::zones::remove_mark(&path, &id) {
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
                                match crate::pilot::zones::remove_mark(&lp, &id) {
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
                let text = match stuck_report(&mut *ctx.game, ctx.data, ctx.opts, &pilot, &dir) {
                    Ok(()) => format!("stuck report written in {}", shown_path(&dir)),
                    Err(e) => format!("stuck report: {e}"),
                };
                w.notice = Some((text, now));
            }
            _ => return false,
        }
        true
    }

    /// A watcher's key let go: the end of the zone being marked with it. True when it was that key.
    fn watch_key_up(&mut self, game: &Game, code: KeyCode) -> bool {
        if self.marking.is_some_and(|m| m.1 == code) {
            self.end_mark(game);
            return true;
        }
        false
    }

    /// The zone being marked is done: the ball's path from its look-back to now (in the sector it was marked in),
    /// written to the zones file at once, obeyed from now on, and shown a moment.
    fn end_mark(&mut self, game: &Game) {
        let w = self;
        let Some((kind, _, from, sector, _)) = w.marking.take() else {
            return;
        };
        let s = &game.sim;
        let mut path: Vec<glam::Vec3> = w
            .trail
            .iter()
            .filter(|t| t.0 >= from && t.2 == sector)
            .map(|t| t.1)
            .collect();
        if s.sector == sector {
            path.push(s.ball.position);
        }
        let points = crate::pilot::zones::thin(&path, MARK_STEP);
        if points.is_empty() {
            w.notice = Some(("no path to mark (another sector)".into(), Instant::now()));
            return;
        }
        let zone = crate::pilot::zones::Zone {
            kind,
            sector: sector + 1,
            radius: crate::pilot::zones::MARK_RADIUS,
            speed: None,
            points,
        };
        let file = w.run.zones_path.clone();
        let secs = s.tick.saturating_sub(from) as f32 / 60.0;
        let id = crate::pilot::zones::new_id();
        let mut text = match crate::pilot::zones::append(&file, &zone, &id) {
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
        let samples: Vec<crate::pilot::learned::Situation> = stretch
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
            level_stem(game),
            sector + 1,
            at.x,
            at.y,
            at.z,
            crate::pilot::learned::today(),
            100.0 * stopping
        );
        let rule = crate::pilot::learned::learn(&samples, kind, None, evidence);
        let lp = w.run.learned_path.clone();
        let taught = match rule {
            Some(r) => match crate::pilot::learned::append(&lp, &r) {
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
    fn watch_trail(&mut self, game: &Game) {
        let w = &mut *self;
        let s = &game.sim;
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
            self.end_mark(game);
        }
    }
}

impl Controller for PilotWatch {
    fn done(&self) -> bool {
        self.done
    }

    fn drives(&self) -> bool {
        self.hand.is_none()
    }

    fn speed(&self) -> f32 {
        if self.paused { 0.0 } else { self.speed }
    }

    fn tick(&mut self, ctx: &mut Ctx, started: Instant) -> bool {
        if !self.watch_tick(ctx, started) {
            return false;
        }
        self.watch_trail(ctx.game);
        true
    }

    fn after_tick(&mut self, game: &Game) {
        self.run.near_edges.tick(&self.run.edges, &game.sim.ball);
        self.watch_trail(game);
    }

    fn key(&mut self, ctx: &mut Ctx, code: KeyCode, down: bool) -> bool {
        if down {
            self.watch_key(ctx, code)
        } else {
            self.watch_key_up(ctx.game, code)
        }
    }

    fn instances(&self, game: &Game, out: &mut Vec<crate::render::Instance>) {
        self.zone_instances(game, out);
    }

    fn status(&self, game: &Game) -> String {
        PilotWatch::status(self, game)
    }

    fn overlay(
        &self,
        game: &Game,
        strings: &crate::lang::Strings,
        font: &crate::ui::Face,
        screen: crate::ui::Screen,
    ) -> Vec<crate::ui::Quad> {
        let mut q = watch_overlay(&PilotWatch::status(self, game), font, screen);
        if self.keys_shown {
            q.extend(self.keys_box(strings, font, screen));
        }
        q
    }

    fn close(&mut self, game: &Game, o: &Options) -> Result<(), String> {
        self.run.summary(game, o)
    }
}
