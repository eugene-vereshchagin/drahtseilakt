// The mods' runtime (docs/drahtseilakt/mods.md): it does what the enabled mods' rules say (their mod.txt, parsed by
// gamedata::mods). Two sides, as the format splits them:
//
// - the run's side (`SimMods`, held inside the Sim, so it is cloned, rewound and replayed with it): the rules on the
//   run's events (start, tick, sector, lost, ...), reading only the run, setting only the ball's stickiness, gravity
//   and the physics' time — deterministic, so a replay naming the mods plays the same run;
// - the view's side (`ViewMods`, held by the window): the rules on frame and key, which move the camera, dim the
//   light, write HUD lines, show the ghost, take a screenshot or restart.
//
// Which mods are on, and their option values, are the settings' `mod.<key> on|off` and `mod.<key>.<option> <value>`;
// the classic mode runs none.

use gamedata::mods::{Action, Corner, Event, Expr, Format, Mod, ModTxt, Scope, Value};
use std::sync::Arc;

/// One mod running: its rules, its option values and its variables (each side keeps its own).
#[derive(Clone, Debug)]
pub struct ModRun {
    pub key: String,
    pub txt: Arc<ModTxt>,
    pub opts: Vec<f32>,
    pub vars: Vec<f32>,
    /// Which variables are the run side's (a view rule reads those from the run side), worked out once.
    sim_var: Arc<[bool]>,
}

impl ModRun {
    pub fn new(key: &str, txt: Arc<ModTxt>, opts: Vec<f32>) -> ModRun {
        let vars = txt.vars.iter().map(|v| v.1).collect();
        let sim_var = txt.vars.iter().map(|v| v.2).collect();
        ModRun {
            key: key.to_owned(),
            txt,
            opts,
            vars,
            sim_var,
        }
    }

    /// Its variables back to their first values (a run starts again).
    pub fn reset(&mut self) {
        self.vars = self.txt.vars.iter().map(|v| v.1).collect();
    }

    /// The `mod` line a replay keeps of it: its key, its mod.txt's hash, its options.
    pub fn replay_line(&self) -> String {
        let mut s = format!("mod {} {:016x}", self.key, self.txt.hash);
        for (o, v) in self.txt.options.iter().zip(&self.opts) {
            s.push_str(&format!(" {}={}", o.key, o.show(*v)));
        }
        s
    }
}

/// What a rule's expressions read.
struct Look<'a> {
    values: &'a dyn Fn(Value) -> f32,
    vars: &'a [f32],
    /// The run side's variables of the same mod (a view rule may read them).
    run_vars: &'a [f32],
    sim_var: &'a [bool],
    opts: &'a [f32],
}

impl Scope for Look<'_> {
    fn value(&self, v: Value) -> f32 {
        (self.values)(v)
    }

    fn var(&self, i: usize) -> f32 {
        let own = self.vars.get(i).copied().unwrap_or(0.0);
        if self.sim_var.get(i).copied().unwrap_or(false) {
            self.run_vars.get(i).copied().unwrap_or(own)
        } else {
            own
        }
    }

    fn opt(&self, i: usize) -> f32 {
        self.opts.get(i).copied().unwrap_or(0.0)
    }
}

/// Runs `run`'s rules for `event`: each whose conditions hold does its actions in order; a variable is set at once
/// (the next action sees it), any other action goes to `act` with its expressions' values.
fn fire(
    run: &mut ModRun,
    event: Event,
    values: &dyn Fn(Value) -> f32,
    run_vars: &[f32],
    act: &mut dyn FnMut(&Action, &[f32]),
) {
    let txt = Arc::clone(&run.txt);
    let sim_var = Arc::clone(&run.sim_var);
    for rule in txt.rules.iter().filter(|r| r.event == event) {
        let holds = {
            let look = Look {
                values,
                vars: &run.vars,
                run_vars,
                sim_var: &sim_var,
                opts: &run.opts,
            };
            rule.conditions.iter().all(|c| c.eval(&look) != 0.0)
        };
        if !holds {
            continue;
        }
        for a in &rule.actions {
            let look = Look {
                values,
                vars: &run.vars,
                run_vars,
                sim_var: &sim_var,
                opts: &run.opts,
            };
            match a {
                Action::Set(i, e) => {
                    let v = e.eval(&look);
                    if let Some(x) = run.vars.get_mut(*i) {
                        *x = v;
                    }
                }
                _ => {
                    let vals: Vec<f32> = action_exprs(a).iter().map(|e| e.eval(&look)).collect();
                    act(a, &vals);
                }
            }
        }
    }
}

fn action_exprs(a: &Action) -> Vec<&Expr> {
    match a {
        Action::Stick(e)
        | Action::SteerYaw(e)
        | Action::ViewTurn(e)
        | Action::CameraSmooth(e)
        | Action::RenderFpsCap(e)
        | Action::RenderColourBits(e)
        | Action::RenderSupports(e)
        | Action::RenderPlain(e)
        | Action::Decoys(e)
        | Action::BallDeform(e)
        | Action::SetSector(e)
        | Action::Gravity(e)
        | Action::TimeFactor(e)
        | Action::CameraFov(e)
        | Action::CameraDistance(e)
        | Action::CameraYaw(e)
        | Action::Light(e)
        | Action::Lantern(e)
        | Action::Ghost(e)
        | Action::EndlessLives(e)
        | Action::Rewind(e)
        | Action::Set(_, e) => vec![e],
        Action::Show(_, _, _, v) => v.iter().collect(),
        Action::RenderScale(a, b) | Action::RenderAspect(a, b) | Action::GravitySide(a, b) => {
            vec![a, b]
        }
        Action::RenderFilter(_) => Vec::new(),
        Action::Splits(_)
        | Action::Keys(_)
        | Action::Screenshot
        | Action::Restart
        | Action::SaveMoment
        | Action::LoadMoment
        | Action::Slower
        | Action::Faster
        | Action::PlayTime
        | Action::FreeCam
        | Action::SpawnSet
        | Action::SpawnGo => Vec::new(),
    }
}

// --- the run's side ----------------------------------------------------------------------------------------------

/// What the run's mods set: the ball's stickiness and the world's gravity and time factor (the original's unless a
/// mod says otherwise).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimOut {
    pub stick: f32,
    pub gravity: f32,
    /// Gravity's sideways components along the world's x and z (`world.gravity_xz`; the original's 0, 0).
    pub gravity_xz: [f32; 2],
    pub time_factor: f32,
    /// The last ball is never lost (god-mode's `lives.endless`).
    pub endless: bool,
    /// Degrees the steering is turned about the vertical (`steer.yaw`).
    pub steer_yaw: f32,
}

impl Default for SimOut {
    fn default() -> SimOut {
        SimOut {
            stick: 0.0,
            gravity: crate::physics::GRAVITY,
            gravity_xz: [0.0, 0.0],
            time_factor: crate::physics::TIME_FACTOR as f32,
            endless: false,
            steer_yaw: 0.0,
        }
    }
}

/// The run's mods, inside the Sim.
#[derive(Clone, Debug, Default)]
pub struct SimMods {
    pub runs: Vec<ModRun>,
    pub out: SimOut,
}

impl SimMods {
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// Runs the rules of `event` with the run's values.
    pub fn fire(&mut self, event: Event, values: &dyn Fn(Value) -> f32) {
        let mut out = self.out;
        for run in &mut self.runs {
            fire(run, event, values, &[], &mut |a, v| {
                let x = v.first().copied().unwrap_or(0.0);
                match a {
                    Action::Stick(_) => out.stick = x.clamp(0.0, 1.0),
                    Action::Gravity(_) => out.gravity = x.clamp(0.0, 60.0),
                    Action::GravitySide(..) => {
                        let side = |i: usize| {
                            v.get(i)
                                .copied()
                                .filter(|x| x.is_finite())
                                .unwrap_or(0.0)
                                .clamp(-60.0, 60.0)
                        };
                        out.gravity_xz = [side(0), side(1)];
                    }
                    Action::TimeFactor(_) => out.time_factor = x.clamp(0.5, 4.0),
                    Action::EndlessLives(_) => out.endless = x != 0.0,
                    Action::SteerYaw(_) => out.steer_yaw = x.rem_euclid(360.0),
                    _ => {}
                }
            });
        }
        self.out = out;
    }

    /// The `mod` lines of the mods that change the run (a replay's header).
    pub fn replay_lines(&self) -> Vec<String> {
        self.runs
            .iter()
            .filter(|r| r.txt.modded())
            .map(ModRun::replay_line)
            .collect()
    }

    /// One of them has the spawn point's keys: the run heeds their input bits.
    pub fn spawn(&self) -> bool {
        self.runs.iter().any(|r| {
            r.txt
                .rules
                .iter()
                .any(|x| x.actions.iter().any(Action::run_deed))
        })
    }

    /// Any of them changes the run: it is modded.
    pub fn modded(&self) -> bool {
        self.runs.iter().any(|r| r.txt.modded())
    }

    /// The run side's variables of a mod (for its view rules).
    pub fn vars_of(&self, key: &str) -> &[f32] {
        self.runs
            .iter()
            .find(|r| r.key == key)
            .map_or(&[], |r| &r.vars)
    }
}

// --- the view's side ---------------------------------------------------------------------------------------------

/// What the view's mods set: the camera, the light, the ghost, the HUD lines (this frame's), and what is to be done.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewOut {
    /// Degrees added to the field of view, the camera's distance as a factor, degrees it is turned about the ball.
    pub fov: f32,
    pub distance: f32,
    pub yaw: f32,
    /// The level's light (1 as it is) and a light over the ball (0 none).
    pub light: f32,
    pub lantern: f32,
    /// The ghost's opacity (0 none).
    pub ghost: f32,
    /// The camera's easing (0 none), the decoy balls, the wooden ball's deformation, the view's turn for the run (whole
    /// degrees; None: not handed on), and whether this frame counts as time played.
    pub smooth: f32,
    /// The picture's look (`render.*`): low size, filtering, shape, frame cap, colour depth.
    pub look: crate::render::RenderLook,
    pub decoys: u8,
    pub deform: f32,
    pub turn: Option<u16>,
    pub playtime: bool,
    /// This frame's HUD lines by corner, and the corners showing the splits.
    pub lines: Vec<(Corner, String)>,
    pub splits: Vec<Corner>,
    pub screenshot: bool,
    pub restart: bool,
    /// The practice's deeds asked for by a key, in order (the window does them).
    pub practice: Vec<Practice>,
}

/// A practice deed (god-mode's keys): the window does it, outside the run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Practice {
    /// So many seconds back (1 to 30).
    Rewind(f32),
    Save,
    Load,
    Slower,
    Faster,
    FreeCam,
    /// The run begun again at this sector (1 first).
    Sector(u32),
    /// A spawn point set where the ball is; the ball put back there at rest (the next tick's input bits).
    SpawnSet,
    SpawnGo,
}

impl Default for ViewOut {
    fn default() -> ViewOut {
        ViewOut {
            fov: 0.0,
            distance: 1.0,
            yaw: 0.0,
            light: 1.0,
            lantern: 0.0,
            ghost: 0.0,
            smooth: 0.0,
            look: crate::render::RenderLook::default(),
            decoys: 0,
            deform: 0.0,
            turn: None,
            playtime: false,
            lines: Vec::new(),
            splits: Vec::new(),
            screenshot: false,
            restart: false,
            practice: Vec::new(),
        }
    }
}

/// The view's mods, held by the window.
#[derive(Clone, Debug, Default)]
pub struct ViewMods {
    pub runs: Vec<ModRun>,
    pub out: ViewOut,
}

/// A key's name on screen: winit's, short ("KeyR" R, "Digit1" 1, "BracketLeft" [).
pub fn key_text(name: &str) -> String {
    match name {
        "BracketLeft" => "[".to_owned(),
        "BracketRight" => "]".to_owned(),
        "Minus" => "-".to_owned(),
        "Equal" => "=".to_owned(),
        _ => name
            .strip_prefix("Key")
            .or_else(|| name.strip_prefix("Digit"))
            .unwrap_or(name)
            .to_owned(),
    }
}

/// Formats a HUD line's numbers.
pub fn format(f: Format, v: &[f32]) -> String {
    let one = |x: f32| match f {
        Format::Time => {
            let t = x.max(0.0);
            format!("{:02}:{:05.2}", (t / 60.0).floor() as u64, t % 60.0)
        }
        Format::Int => format!("{}", x.round() as i64),
        Format::Dec1 => format!("{x:.1}"),
        Format::Dec2 => format!("{x:.2}"),
        Format::Clock => format!("{:02}", x.max(0.0).floor() as u64),
        Format::Ratio => format!("{}", x.round() as i64),
    };
    let sep = match f {
        Format::Clock => ":",
        Format::Ratio => "/",
        _ => "  ",
    };
    v.iter().map(|x| one(*x)).collect::<Vec<_>>().join(sep)
}

impl ViewMods {
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// One of them has practice deeds (god-mode): the window keeps the run's moments for going back.
    pub fn practice(&self) -> bool {
        self.runs.iter().any(|r| r.txt.practice())
    }

    /// A run (re)starts: the view's variables and settings as at first.
    pub fn reset(&mut self) {
        for r in &mut self.runs {
            r.reset();
        }
        self.out = ViewOut::default();
    }

    /// Runs the rules of `event` (frame, or a key) with every value and the run side's variables.
    pub fn fire(&mut self, event: Event, values: &dyn Fn(Value) -> f32, sim: &SimMods, lang: &str) {
        let mut out = std::mem::take(&mut self.out);
        for run in &mut self.runs {
            let run_vars = sim.vars_of(&run.key);
            let txt = Arc::clone(&run.txt);
            let opts = run.opts.clone();
            fire(run, event, values, run_vars, &mut |a, v| {
                let x = v.first().copied().unwrap_or(0.0);
                match a {
                    Action::CameraFov(_) => out.fov = x.clamp(-30.0, 60.0),
                    Action::CameraDistance(_) => out.distance = x.clamp(0.5, 2.0),
                    Action::CameraYaw(_) => out.yaw = x.rem_euclid(360.0),
                    Action::Light(_) => out.light = x.clamp(0.0, 1.0),
                    Action::Lantern(_) => out.lantern = x.clamp(0.0, 1.0),
                    Action::Ghost(_) => out.ghost = x.clamp(0.0, 1.0),
                    Action::CameraSmooth(_) => out.smooth = x.clamp(0.0, 1.0),
                    Action::RenderScale(..) => {
                        let (w, h) = (
                            v.first().copied().unwrap_or(0.0),
                            v.get(1).copied().unwrap_or(0.0),
                        );
                        // from 160x120 up to 4096x4096; anything else is no scaling
                        out.look.scale = (w.is_finite()
                            && h.is_finite()
                            && (160.0..=4096.0).contains(&w)
                            && (120.0..=4096.0).contains(&h))
                        .then(|| (w.round() as u32, h.round() as u32));
                    }
                    Action::RenderFilter(f) => out.look.filter = *f,
                    Action::RenderAspect(..) => {
                        let (w, h) = (
                            v.first().copied().unwrap_or(0.0),
                            v.get(1).copied().unwrap_or(0.0),
                        );
                        out.look.aspect = (w.is_finite()
                            && h.is_finite()
                            && w >= 1.0
                            && h >= 1.0
                            && w / h < 8.0
                            && h / w < 8.0)
                            .then_some((w, h));
                    }
                    Action::RenderFpsCap(_) => {
                        out.look.fps_cap = x.clamp(0.0, 1000.0).round() as u32
                    }
                    Action::RenderColourBits(_) => {
                        out.look.colour_bits = if x.round() == 16.0 { 16 } else { 0 }
                    }
                    Action::RenderPlain(_) => out.look.plain = x >= 0.5,
                    Action::RenderSupports(_) => {
                        out.look.supports = match x.round() as i32 {
                            1 => crate::supports::PLAIN,
                            2 => crate::supports::VARIED,
                            _ => 0,
                        }
                    }
                    Action::Decoys(_) => out.decoys = x.clamp(0.0, 8.0).round() as u8,
                    Action::BallDeform(_) => out.deform = x.clamp(0.0, 1.0),
                    Action::ViewTurn(_) => out.turn = Some(x.rem_euclid(360.0).floor() as u16),
                    Action::PlayTime => out.playtime = true,
                    Action::SetSector(_) if out.practice.len() < 16 => {
                        out.practice
                            .push(Practice::Sector(x.clamp(1.0, 64.0).round() as u32));
                    }
                    Action::Show(c, label, f, _) => {
                        // a corner holds at most a few lines
                        if out.lines.iter().filter(|l| l.0 == *c).count() < 12 {
                            let name = label
                                .and_then(|i| txt.labels.get(i))
                                .map(|l| txt.label(&l.0, lang))
                                .unwrap_or_default();
                            let t = format(*f, v);
                            out.lines.push((
                                *c,
                                if name.is_empty() {
                                    t
                                } else {
                                    format!("{name} {t}")
                                },
                            ));
                        }
                    }
                    Action::Splits(c) => {
                        if !out.splits.contains(c) {
                            out.splits.push(*c);
                        }
                    }
                    Action::Keys(c) => {
                        // the mod's key options: "label: key", as many as a corner holds
                        for (o, v) in txt.options.iter().zip(&opts) {
                            if !matches!(o.kind, gamedata::mods::OptKind::Key(_)) {
                                continue;
                            }
                            if out.lines.iter().filter(|l| l.0 == *c).count() >= 12 {
                                break;
                            }
                            let name = o.show(*v);
                            let key = key_text(&name);
                            out.lines
                                .push((*c, format!("{}: {key}", txt.label(&o.key, lang))));
                        }
                    }
                    Action::Screenshot => out.screenshot = true,
                    Action::Restart => out.restart = true,
                    Action::Rewind(_) if out.practice.len() < 16 => {
                        out.practice.push(Practice::Rewind(x.clamp(1.0, 30.0)));
                    }
                    Action::SaveMoment
                    | Action::LoadMoment
                    | Action::Slower
                    | Action::Faster
                    | Action::FreeCam
                    | Action::SpawnSet
                    | Action::SpawnGo
                        if out.practice.len() < 16 =>
                    {
                        out.practice.push(match a {
                            Action::SaveMoment => Practice::Save,
                            Action::LoadMoment => Practice::Load,
                            Action::Slower => Practice::Slower,
                            Action::Faster => Practice::Faster,
                            Action::SpawnSet => Practice::SpawnSet,
                            Action::SpawnGo => Practice::SpawnGo,
                            _ => Practice::FreeCam,
                        });
                    }
                    _ => {}
                }
            });
        }
        self.out = out;
    }

    /// Fires a key's events for the mods whose key option it is (each mod only its own option).
    pub fn press(&mut self, name: &str, values: &dyn Fn(Value) -> f32, sim: &SimMods, lang: &str) {
        let keyed: Vec<(usize, usize)> = self
            .runs
            .iter()
            .enumerate()
            .flat_map(|(m, r)| {
                r.txt.options.iter().enumerate().filter_map(move |(i, o)| {
                    (matches!(o.kind, gamedata::mods::OptKind::Key(_))
                        && o.show(r.opts.get(i).copied().unwrap_or(o.default)) == name)
                        .then_some((m, i))
                })
            })
            .collect();
        for (m, i) in keyed {
            // fire this mod's rules only: take it out, fire, put it back
            let mut one = ViewMods {
                runs: self.runs.get(m).cloned().into_iter().collect(),
                out: std::mem::take(&mut self.out),
            };
            one.fire(Event::Key(i), values, sim, lang);
            self.out = one.out;
            if let (Some(r), Some(back)) = (self.runs.get_mut(m), one.runs.pop()) {
                *r = back;
            }
        }
    }
}

/// The ticks each sector was reached at (its checkpoint), then the finish's: a run's splits.
pub fn split_ticks(events: &[(u64, crate::sim::Event)]) -> Vec<u64> {
    events
        .iter()
        .filter(|(_, e)| {
            matches!(
                e,
                crate::sim::Event::Checkpoint(_) | crate::sim::Event::Finished
            )
        })
        .map(|(t, _)| *t)
        .take(256)
        .collect()
}

// --- which mods are on -------------------------------------------------------------------------------------------

/// The mods found (factory and the player's), which are on, and their option values.
#[derive(Clone, Debug, Default)]
pub struct Mods {
    pub found: Vec<Mod>,
    pub on: Vec<bool>,
    pub opts: Vec<Vec<f32>>,
    /// Why a mod was not switched on (by its place): the enabled mods' work would pass the budget.
    pub note: Option<(usize, String)>,
}

impl Mods {
    /// The mods found, set as the settings say (`setting` reads a key).
    pub fn load(found: Vec<Mod>, setting: impl Fn(&str) -> Option<String>) -> Mods {
        // switched on in order while the enabled mods' work stays within the budget (MAX_TOTAL_COST)
        let mut spent = 0usize;
        let on = found
            .iter()
            .map(|m| {
                // (a mod that is on by default stays on until the settings say off)
                let want = match setting(&format!("mod.{}", m.key)).as_deref() {
                    Some(v) => v == "on",
                    None => m.parsed.as_ref().is_ok_and(|t| t.default_on),
                };
                match (&m.parsed, want) {
                    (Ok(t), true)
                        if spent.saturating_add(t.cost) <= gamedata::mods::MAX_TOTAL_COST =>
                    {
                        spent = spent.saturating_add(t.cost);
                        true
                    }
                    (Ok(_), true) => {
                        eprintln!(
                            "mod {}: not switched on: the mods on would do too much work a tick",
                            m.key
                        );
                        false
                    }
                    _ => false,
                }
            })
            .collect();
        let opts = found
            .iter()
            .map(|m| match &m.parsed {
                Ok(t) => t
                    .options
                    .iter()
                    .map(|o| {
                        setting(&format!("mod.{}.{}", m.key, o.key))
                            .and_then(|v| o.parse(v.trim()))
                            .map_or(o.default, |v| o.clamp(v))
                    })
                    .collect(),
                Err(_) => Vec::new(),
            })
            .collect();
        Mods {
            found,
            on,
            opts,
            note: None,
        }
    }

    /// The settings lines of every mod (on or off, and the options set otherwise than their defaults).
    pub fn lines(&self) -> Vec<(String, String)> {
        let mut v = Vec::new();
        for (i, m) in self.found.iter().enumerate() {
            let on = self.on.get(i).copied().unwrap_or(false);
            v.push((
                format!("mod.{}", m.key),
                if on { "on" } else { "off" }.to_owned(),
            ));
            if let Ok(t) = &m.parsed {
                for (o, val) in t.options.iter().zip(self.opts.get(i).into_iter().flatten()) {
                    v.push((format!("mod.{}.{}", m.key, o.key), o.show(*val)));
                }
            }
        }
        v
    }

    /// The work the mods switched on can do a tick (their expression nodes in all).
    pub fn cost(&self) -> usize {
        self.found
            .iter()
            .zip(&self.on)
            .filter(|(_, on)| **on)
            .filter_map(|(m, _)| m.parsed.as_ref().ok().map(|t| t.cost))
            .fold(0usize, usize::saturating_add)
    }

    /// Mod `i` switched: off at once; on only when it can be read and the mods on stay within the budget (else
    /// `note` says why).
    pub fn toggle(&mut self, i: usize) {
        self.note = None;
        let cost = match self.found.get(i).map(|m| &m.parsed) {
            Some(Ok(t)) => t.cost,
            _ => return,
        };
        let spent = self.cost();
        if let Some(on) = self.on.get_mut(i) {
            if *on {
                *on = false;
            } else if spent.saturating_add(cost) <= gamedata::mods::MAX_TOTAL_COST {
                *on = true;
            } else {
                self.note = Some((
                    i,
                    format!("{spent} + {cost} > {}", gamedata::mods::MAX_TOTAL_COST),
                ));
            }
        }
    }

    /// An option of mod `i` stepped by `by`.
    pub fn step_option(&mut self, i: usize, o: usize, by: i32) {
        let Some(Ok(t)) = self.found.get(i).map(|m| &m.parsed) else {
            return;
        };
        let Some(spec) = t.options.get(o) else { return };
        if let Some(v) = self.opts.get_mut(i).and_then(|v| v.get_mut(o)) {
            *v = spec.stepped(*v, by);
        }
    }

    /// The running mods (on, and the mode on): the run's side (those with run rules) and the view's.
    pub fn running(&self, mode: bool) -> (SimMods, ViewMods) {
        let mut sim = SimMods::default();
        let mut view = ViewMods::default();
        if !mode {
            return (sim, view);
        }
        for (i, m) in self.found.iter().enumerate() {
            let (true, Ok(t)) = (self.on.get(i).copied().unwrap_or(false), &m.parsed) else {
                continue;
            };
            let txt = Arc::new(t.clone());
            let run = ModRun::new(&m.key, txt, self.opts.get(i).cloned().unwrap_or_default());
            if t.run_side() {
                sim.runs.push(run.clone());
            }
            if t.rules.iter().any(|r| !r.event.sim()) {
                view.runs.push(run);
            }
        }
        (sim, view)
    }

    /// The run's mods a replay names (its `mod` lines): each must be found with the same mod.txt.
    pub fn for_replay(&self, refs: &[crate::replay::ModRef]) -> Result<SimMods, String> {
        let mut sim = SimMods::default();
        for r in refs {
            if sim.runs.iter().any(|x| x.key == r.key) {
                return Err(format!("the replay names the mod \"{}\" twice", r.key));
            }
            let m = self.found.iter().find(|m| m.key == r.key).ok_or_else(|| {
                format!(
                    "the replay needs the mod \"{}\", which is not installed",
                    r.key
                )
            })?;
            let t = m
                .parsed
                .as_ref()
                .map_err(|e| format!("the replay's mod \"{}\" is refused: {e}", r.key))?;
            if t.hash != r.hash {
                return Err(format!(
                    "the replay was played with another version of the mod \"{}\"",
                    r.key
                ));
            }
            // every option given, each a value it takes, nothing else: a replay plays as recorded or not at all
            if let Some((k, _)) = r
                .opts
                .iter()
                .find(|(k, _)| !t.options.iter().any(|o| o.key == *k))
            {
                return Err(format!(
                    "the replay gives the mod \"{}\" an option it has not: {k}",
                    r.key
                ));
            }
            let opts = t
                .options
                .iter()
                .map(|o| {
                    r.opts
                        .iter()
                        .find(|(k, _)| *k == o.key)
                        .and_then(|(_, v)| o.parse(v))
                        .ok_or_else(|| {
                            format!(
                                "the replay's mod \"{}\": option {} missing or out of range",
                                r.key, o.key
                            )
                        })
                })
                .collect::<Result<Vec<f32>, String>>()?;
            sim.runs
                .push(ModRun::new(&m.key, Arc::new(t.clone()), opts));
        }
        Ok(sim)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    fn found(key: &str, text: &str) -> Mod {
        Mod {
            key: key.into(),
            factory: true,
            parsed: gamedata::mods::parse_mod_txt(text),
        }
    }

    const STICKY: &str = "name.en Sticky\noption strength number 10 100 10 60\n\
        on tick\nif ball.touching\ndo ball.stick opt.strength / 100\non tick\nif not ball.touching\ndo ball.stick 0\n";
    const FOV: &str = "name.en Fov\noption strength number 0 100 10 50\nvar seen 0\n\
        on frame\ndo camera.fov min(ball.speed, 30) * opt.strength / 100\ndo set seen = seen + 1\n\
        do show top-right - int seen\n";

    #[test]
    fn each_practice_factory_mod_works_alone() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/mods");
        let zero = |_: Value| 0.0;
        for (name, key, deed) in [
            ("rewind", "F3", Practice::Rewind(5.0)),
            ("save-moment", "F5", Practice::Save),
            ("save-moment", "F9", Practice::Load),
            ("slow-motion", "BracketLeft", Practice::Slower),
            ("slow-motion", "BracketRight", Practice::Faster),
            ("free-camera", "F2", Practice::FreeCam),
        ] {
            let text = std::fs::read_to_string(dir.join(name).join("mod.txt")).unwrap();
            let m = Mods::load(vec![found(name, &text)], |_| Some("on".into()));
            let (sim, mut view) = m.running(true);
            assert!(view.practice(), "{name} is a practice mod");
            view.press(key, &zero, &sim, "en");
            assert_eq!(
                std::mem::take(&mut view.out.practice),
                vec![deed],
                "{name} {key}"
            );
        }
        let text = std::fs::read_to_string(dir.join("endless-lives/mod.txt")).unwrap();
        let m = Mods::load(vec![found("endless-lives", &text)], |_| Some("on".into()));
        let (mut sim, view) = m.running(true);
        assert!(sim.modded() && !view.practice());
        sim.fire(Event::Tick, &zero);
        assert!(sim.out.endless, "endless lives");
    }

    #[test]
    fn god_modes_keys_ask_for_its_deeds_and_its_hint_shows_them() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/mods-dev/god-mode/mod.txt"),
        )
        .unwrap();
        let m = Mods::load(vec![found("god-mode", &text)], |_| Some("on".into()));
        let (sim, mut view) = m.running(true);
        assert!(
            view.practice() && sim.modded(),
            "god-mode is a practice mod and its runs are modded"
        );
        assert!(sim.replay_lines()[0].starts_with("mod god-mode "));
        let zero = |_: Value| 0.0;
        for (key, deed) in [
            ("F3", Practice::Rewind(5.0)),
            ("F5", Practice::Save),
            ("F9", Practice::Load),
            ("BracketLeft", Practice::Slower),
            ("BracketRight", Practice::Faster),
            ("F2", Practice::FreeCam),
        ] {
            view.press(key, &zero, &sim, "en");
            assert_eq!(std::mem::take(&mut view.out.practice), vec![deed], "{key}");
        }
        view.press("KeyR", &zero, &sim, "en");
        assert!(view.out.restart);
        view.fire(
            Event::Frame,
            &|v| if v == Value::PracticeSpeed { 1.0 } else { 0.0 },
            &sim,
            "de",
        );
        let hint: Vec<&str> = view
            .out
            .lines
            .iter()
            .filter(|l| l.0 == Corner::BottomLeft)
            .map(|l| l.1.as_str())
            .collect();
        assert!(
            hint.contains(&"Zurück: F3") && hint.contains(&"Langsamer: ["),
            "{hint:?}"
        );
    }

    #[test]
    fn on_off_and_options_persist_through_the_settings() {
        let mods = || {
            vec![
                found("sticky", STICKY),
                found("fov", FOV),
                found("bad", "nonsense"),
            ]
        };
        let mut m = Mods::load(mods(), |_| None);
        assert_eq!(m.on, vec![false, false, false]);
        m.toggle(0);
        m.toggle(2);
        assert!(!m.on[2], "a refused mod cannot be switched on");
        m.step_option(0, 0, 2);
        let lines = m.lines();
        assert!(lines.contains(&("mod.sticky".into(), "on".into())));
        assert!(lines.contains(&("mod.sticky.strength".into(), "80".into())));
        let back = Mods::load(mods(), |k| {
            lines.iter().find(|l| l.0 == k).map(|l| l.1.clone())
        });
        assert_eq!(back.on, m.on);
        assert_eq!(back.opts, m.opts);
        let (sim, view) = back.running(false);
        assert!(
            sim.is_empty() && view.is_empty(),
            "the classic mode runs no mod"
        );
        let (sim, view) = back.running(true);
        assert_eq!(sim.runs.len(), 1);
        assert!(view.is_empty());
        assert!(sim.modded());
        assert_eq!(sim.replay_lines().len(), 1);
    }

    #[test]
    fn the_run_side_sets_the_ball_and_the_view_side_the_camera() {
        let mut m = Mods::load(vec![found("sticky", STICKY), found("fov", FOV)], |_| {
            Some("on".into())
        });
        m.opts[0][0] = 80.0;
        let (mut sim, mut view) = m.running(true);
        sim.fire(Event::Tick, &|v| {
            if v == Value::BallTouching { 1.0 } else { 0.0 }
        });
        assert!((sim.out.stick - 0.8).abs() < 1e-6);
        sim.fire(Event::Tick, &|_| 0.0);
        assert_eq!(sim.out.stick, 0.0);
        let speed = |v| if v == Value::BallSpeed { 50.0 } else { 0.0 };
        view.fire(Event::Frame, &speed, &sim, "en");
        assert_eq!(view.out.fov, 15.0);
        view.out.lines.clear();
        view.fire(Event::Frame, &speed, &sim, "en");
        assert_eq!(view.out.lines, vec![(Corner::TopRight, "2".to_owned())]);
    }

    #[test]
    fn every_factory_mod_reads_and_does_what_it_says() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/mods");
        let found = gamedata::mods::mods_in(&dir, true);
        assert!(found.len() >= 10, "{found:?}");
        for m in &found {
            let t = m
                .parsed
                .as_ref()
                .unwrap_or_else(|e| panic!("{}: {e}", m.key));
            for lang in ["en", "ru", "de", "zh", "fr", "es", "it"] {
                assert!(
                    t.names.iter().any(|n| n.0 == lang),
                    "{} has no name in {lang}",
                    m.key
                );
                assert!(
                    t.descriptions.iter().any(|n| n.0 == lang),
                    "{} has no description in {lang}",
                    m.key
                );
            }
            for o in &t.options {
                assert_ne!(
                    t.label(&o.key, "en"),
                    o.key,
                    "{}: option {} has no label",
                    m.key,
                    o.key
                );
            }
        }
        let modded: Vec<&str> = found
            .iter()
            .filter(|m| m.parsed.as_ref().is_ok_and(|t| t.modded()))
            .map(|m| m.key.as_str())
            .collect();
        assert_eq!(
            modded,
            vec![
                "endless-lives",
                "free-camera",
                "free-steering",
                "physics-lab",
                "rewind",
                "save-moment",
                "sector-jump",
                "slow-motion",
                "spawn-point",
                "sticky-ball"
            ],
            "only the rule variants change the run"
        );
        // each does its part: switched on, fired, its output set
        let mods = Mods::load(found, |k| {
            (!k.contains("mod.") || k.matches('.').count() == 1).then(|| "on".into())
        });
        let (mut sim, mut view) = mods.running(true);
        let values = |v: Value| match v {
            Value::BallSpeed => 30.0,
            Value::BallTouching => 1.0,
            Value::MouseDx => 100.0,
            Value::Fps => 60.0,
            _ => 0.0,
        };
        sim.fire(Event::Start, &values);
        sim.fire(Event::Tick, &values);
        assert_eq!(
            (sim.out.stick, sim.out.gravity, sim.out.time_factor),
            (0.6, 20.0, 2.0)
        );
        view.fire(Event::Frame, &values, &sim, "en");
        let out = &view.out;
        assert!(out.fov > 0.0, "dynamic fov");
        let look = out.look;
        assert_eq!(look.scale, Some((640, 480)), "back to 2004: the resolution");
        assert_eq!(
            (look.filter, look.fps_cap, look.colour_bits),
            (1, 60, 0),
            "its defaults"
        );
        assert_eq!(look.aspect, Some((4.0, 3.0)));
        assert!(
            look.post()
                && look.size(1920, 1080) == (640, 480)
                && look.placed(1920, 1080) == (240, 0, 1440, 1080)
        );
        assert!((out.yaw - 20.0).abs() < 1e-4, "free view: {}", out.yaw);
        assert!(
            (out.light - 0.1).abs() < 1e-4 && out.lantern > 0.0,
            "blackout"
        );
        assert!((out.ghost - 0.4).abs() < 1e-4, "spirit trail");
        assert_eq!(
            look.supports,
            crate::supports::VARIED,
            "sack supports: varied by default"
        );
        assert_eq!(out.splits, vec![Corner::TopRight], "segment timer");
        assert!(
            out.lines.iter().any(|l| l.1.starts_with("Speed 30.0")),
            "{:?}",
            out.lines
        );
        assert!(
            out.lines.iter().any(|l| l.1 == "Extra points 0/0"),
            "{:?}",
            out.lines
        );
        assert!(
            (out.smooth - 0.4).abs() < 1e-4 && out.decoys == 3 && (out.deform - 0.6).abs() < 1e-4,
            "looks"
        );
        assert!(
            out.playtime && out.turn == Some(20),
            "play time, the view's turn"
        );
        assert!(
            out.lines.iter().any(|l| l.1.starts_with("Played, level")),
            "{:?}",
            out.lines
        );
        view.press("F7", &values, &sim, "en");
        assert_eq!(view.out.practice, vec![Practice::Sector(1)], "sector jump");
        view.out.practice.clear();
        view.press("Backspace", &values, &sim, "en");
        assert!(view.out.restart, "quick restart");
        view.press("F12", &values, &sim, "en");
        assert!(view.out.screenshot, "screenshot");
        view.press("KeyV", &values, &sim, "en");
        view.out.lines.clear();
        view.fire(Event::Frame, &|_| 0.0, &sim, "en");
        assert_eq!(view.out.yaw, 0.0, "free view turned back");
    }

    #[test]
    fn a_mod_on_by_default_is_on_until_switched_off() {
        let txt = "name.en S\ndefault on\non frame\ndo render.supports 1\n";
        let found = || {
            vec![Mod {
                key: "s".into(),
                factory: true,
                parsed: gamedata::mods::parse_mod_txt(txt),
            }]
        };
        // nothing in the settings: on, and only the look (the run is not a modded one)
        let fresh = Mods::load(found(), |_| None);
        assert_eq!(fresh.on, vec![true]);
        let (sim, mut view) = fresh.running(true);
        assert!(!sim.modded() && sim.replay_lines().is_empty());
        view.fire(Event::Frame, &|_| 0.0, &sim, "en");
        assert_eq!(view.out.look.supports, crate::supports::PLAIN);
        // the classic mode runs no mod, this one neither
        assert!(fresh.running(false).1.is_empty());
        // switched off it stays off: the settings say so from then on
        let mut m = fresh;
        m.toggle(0);
        let lines = m.lines();
        assert!(lines.contains(&("mod.s".into(), "off".into())));
        let back = Mods::load(found(), |k| {
            lines.iter().find(|l| l.0 == k).map(|l| l.1.clone())
        });
        assert_eq!(back.on, vec![false]);
    }

    #[test]
    fn mods_past_the_work_budget_are_not_switched_on() {
        // a mod of about 12k nodes: one fits, two would pass the budget
        let sum = vec!["1"; 14].join("+");
        let mut text = "name.en Big\n".to_owned();
        for _ in 0..50 {
            text.push_str("on frame\n");
            for _ in 0..8 {
                text.push_str(&format!("if {sum} > 0\n"));
            }
        }
        let big = gamedata::mods::parse_mod_txt(&text).unwrap();
        assert!(
            big.cost > gamedata::mods::MAX_TOTAL_COST / 2
                && big.cost <= gamedata::mods::MAX_TOTAL_COST,
            "{}",
            big.cost
        );
        let found = || vec![found("a", &text), found("b", &text)];
        let mut m = Mods::load(found(), |_| None);
        m.toggle(0);
        m.toggle(1);
        assert_eq!(m.on, vec![true, false]);
        assert!(
            m.note.as_ref().is_some_and(|n| n.0 == 1),
            "the Mods screen says why"
        );
        m.toggle(0);
        m.toggle(1);
        assert_eq!(m.on, vec![false, true]);
        let back = Mods::load(found(), |_| Some("on".into()));
        assert_eq!(
            back.on,
            vec![true, false],
            "the settings cannot pass the budget either"
        );
    }

    #[test]
    fn a_replay_needs_the_same_mod() {
        let m = Mods::load(vec![found("sticky", STICKY)], |_| Some("on".into()));
        let (sim, _) = m.running(true);
        let line = sim.replay_lines()[0].clone();
        let r = crate::replay::ModRef::parse(&line).unwrap();
        assert_eq!(
            m.for_replay(std::slice::from_ref(&r)).unwrap().runs[0].opts,
            vec![60.0]
        );
        let mut bad = r.clone();
        bad.opts[0].1 = "999".into();
        assert!(m.for_replay(&[bad]).is_err(), "an option out of range");
        assert!(
            m.for_replay(&[r.clone(), r.clone()]).is_err(),
            "a mod twice"
        );
        let other = Mods::load(
            vec![found("sticky", &(STICKY.to_owned() + "# changed\n"))],
            |_| None,
        );
        assert!(other.for_replay(std::slice::from_ref(&r)).is_err());
        assert!(Mods::default().for_replay(&[r]).is_err());
    }
}
