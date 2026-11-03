// Mods (docs/drahtseilakt/mods.md): additions of behaviour and features the player switches on or off, written in our
// own data-only format. A mod is a folder holding one `mod.txt`: a header (names, description, author, version,
// licence, the game version it needs), options the player can set, labels for its HUD lines, variables, and *rules*:
// `on <event>`, then `if <condition>` and `do <action>` lines. Events, conditions, values and actions are a fixed
// library of the engine's building blocks; expressions are small and total (no loops, no recursion beyond a capped
// depth, every operation defined for every input). Nothing in a mod is ever run as code: the game's runtime
// (crates/game/src/modrt.rs) interprets the parsed rules, as the original's behaviour graphs are data driven by its
// runtime. Sound themes, texture and model packs and levels are not mods: they have their own menus and folders.
//
// The game ships "factory" mods (assets/mods/<key>/) written in this very format — they are the features and the
// examples a modder starts from; the player's own go into `<data>/mods/<key>/`. BML mods (Windows DLLs) are never
// loaded nor imported (decisions.md, "Our own data-only mod format").
//
// Determinism: a rule on a *sim* event (tick, start, sector, ...) may only read the run's own state (the ball, the
// run, the arrows pressed, its options and its sim variables) and is the only kind that may change the run (the
// ball's stickiness, gravity, the physics' time); a rule on a *view* event (frame, key) reads anything, changes only
// what is seen or done outside the run (camera, light, HUD, ghost, screenshot, restart). The parser enforces it, so a
// replay that names the run's sim mods (with their mod.txt hash and option values) plays back the same run. A mod
// with any sim action is *modded*: its runs are marked and are never a best run or a highscore.
//
// A player's mod comes from a stranger, so it is hostile input: a folder of a plain name holding nothing but a small
// mod.txt and at most a readme and a licence (a link, a sub-folder, any other file refuses the whole mod with the
// reason shown); mod.txt is UTF-8, capped in bytes and lines; every line must be one the format knows; every count
// is capped; an error names its line. This file is pure parsing, unit-tested here; its fuzz target comes later
// (the owner's call, 2026-11-03: before any release).

use std::path::{Path, PathBuf};

/// The largest mod.txt, its most lines, the most mods of a folder, the longest key or name, the longest text.
pub const MAX_MOD_TXT: u64 = 32 * 1024;
pub const MAX_LINES: usize = 512;
pub const MAX_MODS: usize = 32;
pub const MAX_KEY: usize = 48;
pub const MAX_TEXT: usize = 160;
/// The most options, variables, labels, rules of a mod; conditions and actions of a rule; actions of a mod.
pub const MAX_OPTIONS: usize = 16;
pub const MAX_VARS: usize = 32;
pub const MAX_LABELS: usize = 128;
pub const MAX_RULES: usize = 64;
pub const MAX_CONDITIONS: usize = 8;
pub const MAX_ACTIONS: usize = 16;
pub const MAX_ALL_ACTIONS: usize = 256;
/// An expression's deepest nesting and most nodes; a key option's most keys.
pub const MAX_DEPTH: usize = 16;
pub const MAX_NODES: usize = 64;
pub const MAX_KEYS: usize = 8;
/// The largest readme or licence beside mod.txt (never read by the game, only allowed to be there).
pub const MAX_SIDE_FILE: u64 = 64 * 1024;
/// The most entries of a mods folder looked at (a guard against a folder of millions; real ones hold a few).
const MAX_ENTRIES: usize = 65_536;
/// The most expression nodes the enabled mods may evaluate in all (each mod's `cost`): a static budget per tick.
pub const MAX_TOTAL_COST: usize = 20_000;

/// The languages a mod's texts may be in: the menus' (lang.rs LANGUAGES).
pub const LANGUAGES: [&str; 7] = ["en", "ru", "de", "zh", "fr", "es", "it"];

/// The game's version, which a mod's `requires` is compared with.
pub const GAME_VERSION: (u32, u32, u32) = (0, 1, 0);

// --- the library: events, values, actions ----------------------------------------------------------------------

/// What starts a rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    // the run's (sim) events, in tick order: deterministic
    /// The run (re)starts: before its first tick.
    Start,
    /// Every tick, before the physics.
    Tick,
    /// A new sector is reached (a checkpoint).
    Sector,
    /// The ball is lost.
    Lost,
    /// A new ball comes after a lost one.
    NewBall,
    /// The ball changes in a transformer.
    Transform,
    /// An extra point or life is taken.
    Extra,
    /// The level is finished.
    Finish,
    // the view's events: what is seen, outside the run
    /// Every frame drawn.
    Frame,
    /// The key of one of the mod's key options is pressed (its index among the options).
    Key(usize),
}

impl Event {
    pub fn sim(self) -> bool {
        !matches!(self, Event::Frame | Event::Key(_))
    }
}

/// A value an expression reads. The first group is the run's own (sim), the rest the view's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    BallX,
    BallY,
    BallZ,
    BallSpeed,
    /// The ball's vertical speed.
    BallVy,
    /// 0 wood, 1 stone, 2 paper.
    BallKind,
    /// 1 while the ball touches something.
    BallTouching,
    /// 1 while what it touches is steep (a wall).
    BallWall,
    RunTick,
    /// The run's time, seconds.
    RunTime,
    /// 1 for the first sector.
    RunSector,
    RunLives,
    RunPoints,
    InputForward,
    InputBackward,
    InputLeft,
    InputRight,
    InputRotation,
    InputOverview,
    // view-only values
    /// Real seconds since the run began (pauses and menus included).
    RealTime,
    /// The last frame's real length, seconds, and the frames a second.
    FrameDt,
    Fps,
    ClockHour,
    ClockMinute,
    ClockSecond,
    /// The mouse's motion this frame (pixels).
    MouseDx,
    MouseDy,
    /// The extra points and lives left in the current sector, and how many it had.
    ExtrasPoints,
    ExtrasPointsAll,
    ExtrasLives,
    ExtrasLivesAll,
    /// The level's best time in seconds (-1: none yet).
    BestTime,
}

const VALUES: [(&str, Value); 35] = [
    ("ball.x", Value::BallX),
    ("ball.y", Value::BallY),
    ("ball.z", Value::BallZ),
    ("ball.speed", Value::BallSpeed),
    ("ball.vy", Value::BallVy),
    ("ball.kind", Value::BallKind),
    ("ball.touching", Value::BallTouching),
    ("ball.wall", Value::BallWall),
    ("run.tick", Value::RunTick),
    ("run.time", Value::RunTime),
    ("run.sector", Value::RunSector),
    ("run.lives", Value::RunLives),
    ("run.points", Value::RunPoints),
    ("input.forward", Value::InputForward),
    ("input.backward", Value::InputBackward),
    ("input.left", Value::InputLeft),
    ("input.right", Value::InputRight),
    ("input.rotation", Value::InputRotation),
    ("input.overview", Value::InputOverview),
    ("real.time", Value::RealTime),
    ("frame.dt", Value::FrameDt),
    ("frame.fps", Value::Fps),
    ("clock.hour", Value::ClockHour),
    ("clock.minute", Value::ClockMinute),
    ("clock.second", Value::ClockSecond),
    ("mouse.dx", Value::MouseDx),
    ("mouse.dy", Value::MouseDy),
    ("extras.points", Value::ExtrasPoints),
    ("extras.points_all", Value::ExtrasPointsAll),
    ("extras.lives", Value::ExtrasLives),
    ("extras.lives_all", Value::ExtrasLivesAll),
    ("best.time", Value::BestTime),
    // aliases kept short for HUD lines
    ("run.ticks", Value::RunTick),
    ("ball.v", Value::BallSpeed),
    ("time.real", Value::RealTime),
];

impl Value {
    /// The run's own state (a sim rule may read it).
    pub fn sim(self) -> bool {
        matches!(
            self,
            Value::BallX
                | Value::BallY
                | Value::BallZ
                | Value::BallSpeed
                | Value::BallVy
                | Value::BallKind
                | Value::BallTouching
                | Value::BallWall
                | Value::RunTick
                | Value::RunTime
                | Value::RunSector
                | Value::RunLives
                | Value::RunPoints
                | Value::InputForward
                | Value::InputBackward
                | Value::InputLeft
                | Value::InputRight
                | Value::InputRotation
                | Value::InputOverview
        )
    }
}

/// The functions an expression may call (each total).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Func {
    Min,
    Max,
    Abs,
    Clamp,
    If,
    Floor,
    Sqrt,
}

const FUNCS: [(&str, Func, usize); 7] = [
    ("min", Func::Min, 2),
    ("max", Func::Max, 2),
    ("abs", Func::Abs, 1),
    ("clamp", Func::Clamp, 3),
    ("if", Func::If, 3),
    ("floor", Func::Floor, 1),
    ("sqrt", Func::Sqrt, 1),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

/// An expression, parsed: total to evaluate (see `eval`).
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f32),
    Value(Value),
    /// A variable of the mod, by its place.
    Var(usize),
    /// An option of the mod, by its place (its value as the player set it).
    Opt(usize),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Call(Func, Vec<Expr>),
}

/// What an expression reads its values, variables and options from.
pub trait Scope {
    fn value(&self, v: Value) -> f32;
    fn var(&self, i: usize) -> f32;
    fn opt(&self, i: usize) -> f32;
}

/// A number kept finite: what is not becomes 0.
fn finite(x: f32) -> f32 {
    if x.is_finite() { x } else { 0.0 }
}

fn truth(x: f32) -> bool {
    x != 0.0
}

fn flag(b: bool) -> f32 {
    if b { 1.0 } else { 0.0 }
}

impl Expr {
    /// The value, always finite (a division by zero is 0, as is anything not finite); its depth is capped by the
    /// parser, so this never recurses far.
    pub fn eval(&self, s: &dyn Scope) -> f32 {
        let x = match self {
            Expr::Num(n) => *n,
            Expr::Value(v) => s.value(*v),
            Expr::Var(i) => s.var(*i),
            Expr::Opt(i) => s.opt(*i),
            Expr::Neg(e) => -e.eval(s),
            Expr::Not(e) => flag(!truth(e.eval(s))),
            Expr::Bin(op, a, b) => {
                let (a, b) = (a.eval(s), b.eval(s));
                match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div => {
                        if b == 0.0 {
                            0.0
                        } else {
                            a / b
                        }
                    }
                    Op::Lt => flag(a < b),
                    Op::Le => flag(a <= b),
                    Op::Gt => flag(a > b),
                    Op::Ge => flag(a >= b),
                    Op::Eq => flag(a == b),
                    Op::Ne => flag(a != b),
                    Op::And => flag(truth(a) && truth(b)),
                    Op::Or => flag(truth(a) || truth(b)),
                }
            }
            Expr::Call(f, args) => {
                let a = |i: usize| args.get(i).map_or(0.0, |e| e.eval(s));
                match f {
                    Func::Min => a(0).min(a(1)),
                    Func::Max => a(0).max(a(1)),
                    Func::Abs => a(0).abs(),
                    Func::Clamp => {
                        let (lo, hi) = (a(1), a(2));
                        if lo <= hi { a(0).clamp(lo, hi) } else { lo }
                    }
                    Func::If => {
                        if truth(a(0)) {
                            a(1)
                        } else {
                            a(2)
                        }
                    }
                    Func::Floor => a(0).floor(),
                    Func::Sqrt => a(0).max(0.0).sqrt(),
                }
            }
        };
        finite(x)
    }

    /// Its nodes (each value, number, operation and call).
    pub fn size(&self) -> usize {
        1usize.saturating_add(match self {
            Expr::Neg(e) | Expr::Not(e) => e.size(),
            Expr::Bin(_, a, b) => a.size().saturating_add(b.size()),
            Expr::Call(_, args) => args.iter().map(Expr::size).fold(0usize, usize::saturating_add),
            _ => 0,
        })
    }

    /// Visits every leaf: what the expression reads.
    fn leaves(&self, f: &mut dyn FnMut(&Expr)) {
        match self {
            Expr::Neg(e) | Expr::Not(e) => e.leaves(f),
            Expr::Bin(_, a, b) => {
                a.leaves(f);
                b.leaves(f);
            }
            Expr::Call(_, args) => args.iter().for_each(|e| e.leaves(f)),
            leaf => f(leaf),
        }
    }
}

/// How a HUD line shows its numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Minutes, seconds and hundredths (from seconds).
    Time,
    Int,
    Dec1,
    Dec2,
    /// Hours and minutes (from hours, minutes).
    Clock,
    /// Whole numbers as a / b.
    Ratio,
}

const FORMATS: [(&str, Format); 6] = [
    ("ratio", Format::Ratio),
    ("time", Format::Time),
    ("int", Format::Int),
    ("dec1", Format::Dec1),
    ("dec2", Format::Dec2),
    ("clock", Format::Clock),
];

/// Where a HUD line goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

const CORNERS: [(&str, Corner); 4] = [
    ("top-left", Corner::TopLeft),
    ("top-right", Corner::TopRight),
    ("bottom-left", Corner::BottomLeft),
    ("bottom-right", Corner::BottomRight),
];

/// A rule's action: the library's building blocks.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// A variable set (a sim variable in a sim rule, a view variable in a view rule).
    Set(usize, Expr),
    // the run's (sim) actions: only in sim rules; they make the mod modded
    /// How strongly the ball holds to what it touches, 0 to 1 (a wall can be climbed).
    Stick(Expr),
    /// Gravity's pull, 0 to 60 (the original's 20).
    Gravity(Expr),
    /// The physics' time factor, 0.5 to 4 (the original's 2).
    TimeFactor(Expr),
    // the view's actions
    /// The field of view widened by so many degrees (-30 to 60).
    CameraFov(Expr),
    /// The camera's distance from the ball, as a factor (0.5 to 2).
    CameraDistance(Expr),
    /// The view turned about the ball by so many degrees (the steering keeps the camera's own heading).
    CameraYaw(Expr),
    /// The level's light, 0 (dark) to 1 (as it is).
    Light(Expr),
    /// A light over the ball, 0 to 1.
    Lantern(Expr),
    /// A HUD line: corner, label (by its place), format, up to three numbers.
    Show(Corner, Option<usize>, Format, Vec<Expr>),
    /// The sectors' splits against the best run, in a corner.
    Splits(Corner),
    /// The best run as a translucent ball, so opaque (0 to 1; 0 hides it).
    Ghost(Expr),
    Screenshot,
    Restart,
}

impl Action {
    pub fn sim(&self) -> bool {
        matches!(
            self,
            Action::Stick(_) | Action::Gravity(_) | Action::TimeFactor(_)
        )
    }

    fn exprs(&self) -> Vec<&Expr> {
        match self {
            Action::Set(_, e)
            | Action::Stick(e)
            | Action::Gravity(e)
            | Action::TimeFactor(e)
            | Action::CameraFov(e)
            | Action::CameraDistance(e)
            | Action::CameraYaw(e)
            | Action::Light(e)
            | Action::Lantern(e)
            | Action::Ghost(e) => vec![e],
            Action::Show(_, _, _, v) => v.iter().collect(),
            Action::Splits(_) | Action::Screenshot | Action::Restart => Vec::new(),
        }
    }
}

/// A rule: when its event comes and all its conditions hold, its actions are done in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub event: Event,
    pub conditions: Vec<Expr>,
    pub actions: Vec<Action>,
}

/// How an option is set.
#[derive(Clone, Debug, PartialEq)]
pub enum OptKind {
    Switch,
    Number {
        min: f32,
        max: f32,
        step: f32,
    },
    /// One of a few keys (winit's names); the value is its place.
    Key(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptSpec {
    pub key: String,
    pub kind: OptKind,
    pub default: f32,
}

impl OptSpec {
    /// `v` brought into the option's range (a number onto its steps from `min`).
    pub fn clamp(&self, v: f32) -> f32 {
        let v = finite(v);
        match &self.kind {
            OptKind::Switch => flag(truth(v)),
            OptKind::Number { min, max, step } => {
                let v = v.clamp(*min, *max);
                if *step > 0.0 {
                    let n = ((v - min) / step).round();
                    (min + n * step).clamp(*min, *max)
                } else {
                    v
                }
            }
            OptKind::Key(list) => v.round().clamp(0.0, list.len().saturating_sub(1) as f32),
        }
    }

    /// The value as the settings write it.
    pub fn show(&self, v: f32) -> String {
        match &self.kind {
            OptKind::Switch => if truth(v) { "on" } else { "off" }.to_owned(),
            OptKind::Number { .. } => format!("{}", self.clamp(v)),
            OptKind::Key(list) => list
                .get(self.clamp(v) as usize)
                .cloned()
                .unwrap_or_default(),
        }
    }

    /// The value a text stands for, if it is one this option takes.
    pub fn parse(&self, s: &str) -> Option<f32> {
        match &self.kind {
            OptKind::Switch => match s {
                "on" | "yes" => Some(1.0),
                "off" | "no" => Some(0.0),
                _ => None,
            },
            OptKind::Number { min, max, .. } => {
                let v: f32 = s.parse().ok().filter(|v: &f32| v.is_finite())?;
                (*min..=*max).contains(&v).then(|| self.clamp(v))
            }
            OptKind::Key(list) => list.iter().position(|k| k == s).map(|i| i as f32),
        }
    }

    /// One step of the arrows from `v` (round a key list, along a number's steps, a switch flipped).
    pub fn stepped(&self, v: f32, by: i32) -> f32 {
        match &self.kind {
            OptKind::Switch => flag(!truth(v)),
            OptKind::Number { step, .. } => self.clamp(v + step * by as f32),
            OptKind::Key(list) => {
                let n = list.len().max(1) as i64;
                ((v as i64).saturating_add(i64::from(by)).rem_euclid(n)) as f32
            }
        }
    }
}

/// A mod.txt read: its header, options, labels, variables and rules, all checked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModTxt {
    pub names: Vec<(String, String)>,
    pub descriptions: Vec<(String, String)>,
    pub author: String,
    pub version: String,
    pub licence: String,
    pub requires: (u32, u32, u32),
    pub options: Vec<OptSpec>,
    /// Labels: (name, language, text); an option's label is named like the option.
    pub labels: Vec<(String, String, String)>,
    /// Variables: name, initial value, and whether sim rules own it.
    pub vars: Vec<(String, f32, bool)>,
    pub rules: Vec<Rule>,
    /// The hash of the text (fnv-1a 64), which a replay names.
    pub hash: u64,
    /// The expression nodes its rules can evaluate in all (conditions and actions, one more an action): the most work
    /// one round of its events can do, held against MAX_TOTAL_COST when it is switched on.
    pub cost: usize,
}

impl ModTxt {
    pub fn name(&self, lang: &str, key: &str) -> String {
        pick(&self.names, lang).unwrap_or_else(|| key.to_owned())
    }

    pub fn description(&self, lang: &str) -> String {
        pick(&self.descriptions, lang).unwrap_or_default()
    }

    /// A label's text in `lang`, else English, else its name.
    pub fn label(&self, name: &str, lang: &str) -> String {
        let find = |l: &str| {
            self.labels
                .iter()
                .find(|(n, k, _)| n == name && k == l)
                .map(|t| t.2.clone())
        };
        find(lang)
            .or_else(|| find("en"))
            .unwrap_or_else(|| name.to_owned())
    }

    /// It changes the run (any sim action): its runs are modded.
    pub fn modded(&self) -> bool {
        self.rules.iter().any(|r| r.actions.iter().any(Action::sim))
    }

    /// The label names in their order (`Show` refers to them by place).
    fn label_index(&self, name: &str) -> usize {
        // a label's languages are kept together, so its first place names it
        match self.labels.iter().position(|(n, _, _)| n == name) {
            Some(i) => i,
            None => self.labels.len(),
        }
    }
}

fn pick(list: &[(String, String)], lang: &str) -> Option<String> {
    let find = |l: &str| list.iter().find(|(k, _)| k == l).map(|(_, v)| v.clone());
    find(lang).or_else(|| find("en"))
}

/// A name of an option, variable or label: lower-case letters, digits and `_`, beginning with a letter.
fn ident(s: &str) -> bool {
    s.len() <= MAX_KEY
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A key's name as winit names it (letters and digits).
fn key_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 24 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// A text value: no control characters, no bidirectional overrides, at most MAX_TEXT characters.
fn text_value(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("an empty text".into());
    }
    if s.chars().count() > MAX_TEXT {
        return Err(format!("a text longer than {MAX_TEXT} characters"));
    }
    if s.chars().any(|c| {
        c.is_control()
            || ('\u{202a}'..='\u{202e}').contains(&c)
            || ('\u{2066}'..='\u{2069}').contains(&c)
    }) {
        return Err("a control character in a text".into());
    }
    Ok(s.to_owned())
}

fn version(s: &str) -> Option<(u32, u32, u32)> {
    let mut p = s.split('.');
    let a = p.next()?.parse().ok()?;
    let b = p.next().map_or(Some(0), |x| x.parse().ok())?;
    let c = p.next().map_or(Some(0), |x| x.parse().ok())?;
    p.next().is_none().then_some((a, b, c))
}

/// A short, printable piece of a hostile word for an error message.
fn clip(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(24).collect()
}

fn number(s: &str) -> Result<f32, String> {
    s.parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("not a number: \"{}\"", clip(s)))
}

// --- expressions -------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f32),
    Word(String),
    Sym(&'static str),
}

fn tokens(s: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while let Some(&c) = b.get(i) {
        if c.is_whitespace() {
            i = i.saturating_add(1);
            continue;
        }
        if out.len() >= MAX_NODES.saturating_mul(2) {
            return Err("an expression too long".into());
        }
        if c.is_ascii_digit()
            || (c == '.' && b.get(i.saturating_add(1)).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while b.get(i).is_some_and(|c| c.is_ascii_digit() || *c == '.') {
                i = i.saturating_add(1);
            }
            let t: String = b.get(start..i).unwrap_or_default().iter().collect();
            out.push(Tok::Num(number(&t)?));
            continue;
        }
        if c.is_ascii_alphabetic() {
            let start = i;
            while b
                .get(i)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
            {
                i = i.saturating_add(1);
            }
            out.push(Tok::Word(
                b.get(start..i).unwrap_or_default().iter().collect(),
            ));
            continue;
        }
        let two: String = b
            .get(i..i.saturating_add(2))
            .map(|s| s.iter().collect())
            .unwrap_or_default();
        let sym = ["<=", ">=", "==", "!="].into_iter().find(|s| *s == two);
        if let Some(s) = sym {
            out.push(Tok::Sym(s));
            i = i.saturating_add(2);
            continue;
        }
        let one = ["+", "-", "*", "/", "<", ">", "(", ")", ","]
            .into_iter()
            .find(|s| s.starts_with(c));
        match one {
            Some(s) => out.push(Tok::Sym(s)),
            None => return Err(format!("unexpected character '{}'", c.escape_default())),
        }
        i = i.saturating_add(1);
    }
    Ok(out)
}

/// What names an expression can use: the mod's variables and options (and whether a variable is sim's).
struct Names<'a> {
    vars: &'a [(String, f32, bool)],
    options: &'a [OptSpec],
}

struct Parser<'a> {
    toks: Vec<Tok>,
    at: usize,
    nodes: usize,
    names: &'a Names<'a>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.at).cloned();
        self.at = self.at.saturating_add(1);
        t
    }

    fn node(&mut self, depth: usize) -> Result<(), String> {
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > MAX_NODES {
            return Err(format!("an expression of more than {MAX_NODES} parts"));
        }
        if depth > MAX_DEPTH {
            return Err(format!("an expression nested deeper than {MAX_DEPTH}"));
        }
        Ok(())
    }

    fn sym(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Tok::Sym(x)) if *x == s)
    }

    fn word(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Tok::Word(x)) if x == s)
    }

    // or > and > comparison > + - > * / > unary > atom
    fn or(&mut self, d: usize) -> Result<Expr, String> {
        self.node(d)?;
        let mut a = self.and(d)?;
        while self.word("or") {
            self.next();
            let b = self.and(d)?;
            self.node(d)?;
            a = Expr::Bin(Op::Or, Box::new(a), Box::new(b));
        }
        Ok(a)
    }

    fn and(&mut self, d: usize) -> Result<Expr, String> {
        self.node(d)?;
        let mut a = self.cmp(d)?;
        while self.word("and") {
            self.next();
            let b = self.cmp(d)?;
            self.node(d)?;
            a = Expr::Bin(Op::And, Box::new(a), Box::new(b));
        }
        Ok(a)
    }

    fn cmp(&mut self, d: usize) -> Result<Expr, String> {
        self.node(d)?;
        let a = self.sum(d)?;
        let op = match self.peek() {
            Some(Tok::Sym("<")) => Op::Lt,
            Some(Tok::Sym("<=")) => Op::Le,
            Some(Tok::Sym(">")) => Op::Gt,
            Some(Tok::Sym(">=")) => Op::Ge,
            Some(Tok::Sym("==")) => Op::Eq,
            Some(Tok::Sym("!=")) => Op::Ne,
            _ => return Ok(a),
        };
        self.next();
        let b = self.sum(d)?;
        Ok(Expr::Bin(op, Box::new(a), Box::new(b)))
    }

    fn sum(&mut self, d: usize) -> Result<Expr, String> {
        self.node(d)?;
        let mut a = self.product(d)?;
        loop {
            let op = if self.sym("+") {
                Op::Add
            } else if self.sym("-") {
                Op::Sub
            } else {
                return Ok(a);
            };
            self.next();
            let b = self.product(d)?;
            self.node(d)?;
            a = Expr::Bin(op, Box::new(a), Box::new(b));
        }
    }

    fn product(&mut self, d: usize) -> Result<Expr, String> {
        self.node(d)?;
        let mut a = self.unary(d)?;
        loop {
            let op = if self.sym("*") {
                Op::Mul
            } else if self.sym("/") {
                Op::Div
            } else {
                return Ok(a);
            };
            self.next();
            let b = self.unary(d)?;
            self.node(d)?;
            a = Expr::Bin(op, Box::new(a), Box::new(b));
        }
    }

    fn unary(&mut self, d: usize) -> Result<Expr, String> {
        self.node(d)?;
        if self.sym("-") {
            self.next();
            return Ok(Expr::Neg(Box::new(self.unary(d.saturating_add(1))?)));
        }
        if self.word("not") {
            self.next();
            return Ok(Expr::Not(Box::new(self.unary(d.saturating_add(1))?)));
        }
        self.atom(d)
    }

    fn atom(&mut self, d: usize) -> Result<Expr, String> {
        self.node(d)?;
        match self.next() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Sym("(")) => {
                let e = self.or(d.saturating_add(1))?;
                if !self.sym(")") {
                    return Err("a missing )".into());
                }
                self.next();
                Ok(e)
            }
            Some(Tok::Word(w)) => {
                if let Some(&(_, f, n)) = FUNCS.iter().find(|f| f.0 == w) {
                    if !self.sym("(") {
                        return Err(format!("{w} needs (...)"));
                    }
                    self.next();
                    let mut args = Vec::new();
                    loop {
                        args.push(self.or(d.saturating_add(1))?);
                        if self.sym(",") {
                            self.next();
                            continue;
                        }
                        break;
                    }
                    if !self.sym(")") {
                        return Err("a missing )".into());
                    }
                    self.next();
                    if args.len() != n {
                        return Err(format!("{w} takes {n} values"));
                    }
                    return Ok(Expr::Call(f, args));
                }
                if w == "true" {
                    return Ok(Expr::Num(1.0));
                }
                if w == "false" {
                    return Ok(Expr::Num(0.0));
                }
                if let Some(o) = w.strip_prefix("opt.") {
                    let i = self.names.options.iter().position(|x| x.key == o);
                    return i
                        .map(Expr::Opt)
                        .ok_or_else(|| format!("no option \"{}\"", clip(o)));
                }
                if let Some(&(_, v)) = VALUES.iter().find(|v| v.0 == w) {
                    return Ok(Expr::Value(v));
                }
                match self.names.vars.iter().position(|x| x.0 == w) {
                    Some(i) => Ok(Expr::Var(i)),
                    None => Err(format!("unknown name \"{}\"", clip(&w))),
                }
            }
            Some(t) => Err(format!("unexpected {t:?}")),
            None => Err("an expression ends too soon".into()),
        }
    }
}

fn expr(text: &str, names: &Names) -> Result<Expr, String> {
    let mut p = Parser {
        toks: tokens(text)?,
        at: 0,
        nodes: 0,
        names,
    };
    let e = p.or(0)?;
    if p.at < p.toks.len() {
        return Err("more after the expression".into());
    }
    Ok(e)
}

/// Splits an argument list at its top-level commas.
fn split_args(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(s.get(start..i).unwrap_or_default());
                start = i.saturating_add(1);
            }
            _ => {}
        }
    }
    out.push(s.get(start..).unwrap_or_default());
    out
}

// --- mod.txt -----------------------------------------------------------------------------------------------------

fn parse_option(rest: &str) -> Result<OptSpec, String> {
    let f: Vec<&str> = rest.split_whitespace().collect();
    let (Some(&key), Some(&kind)) = (f.first(), f.get(1)) else {
        return Err("expected: option <name> switch|number|key ...".into());
    };
    if !ident(key) {
        return Err(format!("not a name: \"{}\"", clip(key)));
    }
    let o = match (kind, f.get(2..).unwrap_or_default()) {
        ("switch", [d]) => OptSpec {
            key: key.into(),
            kind: OptKind::Switch,
            default: match *d {
                "on" => 1.0,
                "off" => 0.0,
                _ => return Err("a switch is on or off".into()),
            },
        },
        ("number", [lo, hi, step, d]) => {
            let (min, max, step, default) = (number(lo)?, number(hi)?, number(step)?, number(d)?);
            // short values (a replay writes them as words) and a default on the steps (the run and its replay use the
            // very same value)
            if !(min < max && step >= 0.001 && (min..=max).contains(&default))
                || (max - min) / step > 10_000.0
                || min.abs() > 1e6
                || max.abs() > 1e6
            {
                return Err("a number option needs min < max (within a million), a step of 0.001 or more and the default between".into());
            }
            let o = OptSpec { key: key.into(), kind: OptKind::Number { min, max, step }, default };
            if o.clamp(default) != default {
                return Err("a number option's default must be on its steps from min".into());
            }
            o
        }
        ("key", [list]) => {
            let keys: Vec<String> = list.split(',').map(str::to_owned).collect();
            if keys.is_empty() || keys.len() > MAX_KEYS || !keys.iter().all(|k| key_name(k)) {
                return Err(format!("a key option lists 1 to {MAX_KEYS} key names, comma-separated"));
            }
            if let Some(k) = keys.iter().find(|k| RESERVED_KEYS.contains(&k.as_str())) {
                return Err(format!("{} is the game's own key", clip(k)));
            }
            OptSpec { key: key.into(), kind: OptKind::Key(keys), default: 0.0 }
        }
        _ => return Err("expected: option <name> switch on|off, number <min> <max> <step> <default>, key <K1,K2,...>".into()),
    };
    Ok(o)
}

/// Keys a mod may never take: the menu's and window's, the arrows, the game's default keys, the practice tools'.
pub const RESERVED_KEYS: [&str; 22] = [
    "Escape", "Enter", "NumpadEnter", "F11", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "ShiftLeft",
    "ShiftRight", "Space", "Tab", "KeyR", "F2", "F3", "F5", "F9", "BracketLeft", "BracketRight", "AltLeft", "AltRight",
    "Backquote",
];

fn parse_event(rest: &str, options: &[OptSpec]) -> Result<Event, String> {
    let mut f = rest.split_whitespace();
    let e = f.next().unwrap_or_default();
    let ev = match e {
        "start" => Event::Start,
        "tick" => Event::Tick,
        "sector" => Event::Sector,
        "lost" => Event::Lost,
        "newball" => Event::NewBall,
        "transform" => Event::Transform,
        "extra" => Event::Extra,
        "finish" => Event::Finish,
        "frame" => Event::Frame,
        "key" => {
            let o = f.next().unwrap_or_default();
            let i = options
                .iter()
                .position(|x| x.key == o && matches!(x.kind, OptKind::Key(_)))
                .ok_or_else(|| format!("\"on key\" names a key option: \"{}\" is none", clip(o)))?;
            Event::Key(i)
        }
        _ => return Err(format!("unknown event \"{}\"", clip(e))),
    };
    if f.next().is_some() {
        return Err("more after the event".into());
    }
    Ok(ev)
}

fn parse_action(rest: &str, m: &mut ModTxt, sim: bool, key: bool) -> Result<Action, String> {
    let (verb, args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let args = args.trim();
    let names_vars = m.vars.clone();
    let names = Names {
        vars: &names_vars,
        options: &m.options,
    };
    let e = |s: &str| expr(s, &names);
    let a = match verb {
        "set" => {
            let (v, x) = args
                .split_once('=')
                .ok_or("expected: set <variable> = <expression>")?;
            let v = v.trim();
            let i = names_vars
                .iter()
                .position(|x| x.0 == v)
                .ok_or_else(|| format!("no variable \"{}\"", clip(v)))?;
            Action::Set(i, e(x)?)
        }
        "ball.stick" => Action::Stick(e(args)?),
        "world.gravity" => Action::Gravity(e(args)?),
        "world.time" => Action::TimeFactor(e(args)?),
        "camera.fov" => Action::CameraFov(e(args)?),
        "camera.distance" => Action::CameraDistance(e(args)?),
        "camera.yaw" => Action::CameraYaw(e(args)?),
        "light" => Action::Light(e(args)?),
        "lantern" => Action::Lantern(e(args)?),
        "ghost" => Action::Ghost(e(args)?),
        "screenshot" if args.is_empty() => Action::Screenshot,
        "restart" if args.is_empty() => Action::Restart,
        "splits" => {
            let c = CORNERS
                .iter()
                .find(|c| c.0 == args)
                .ok_or("splits <corner>")?;
            Action::Splits(c.1)
        }
        "show" => {
            // show <corner> <label|-> <format> <expr>[, <expr>[, <expr>]]
            let mut f = args.splitn(4, char::is_whitespace);
            let (Some(c), Some(l), Some(fm), Some(x)) = (f.next(), f.next(), f.next(), f.next())
            else {
                return Err(
                    "expected: show <corner> <label|-> <format> <expression>[, ...]".into(),
                );
            };
            let corner = CORNERS
                .iter()
                .find(|k| k.0 == c)
                .ok_or_else(|| format!("unknown corner \"{}\"", clip(c)))?
                .1;
            let format = FORMATS
                .iter()
                .find(|k| k.0 == fm)
                .ok_or_else(|| format!("unknown format \"{}\"", clip(fm)))?
                .1;
            let label = if l == "-" {
                None
            } else {
                if !ident(l) {
                    return Err(format!("not a label name: \"{}\"", clip(l)));
                }
                let i = m.label_index(l);
                if i == m.labels.len() {
                    return Err(format!(
                        "no label \"{}\" (give it with a label line first)",
                        clip(l)
                    ));
                }
                Some(i)
            };
            let parts = split_args(x);
            if parts.len() > 3 {
                return Err("show takes at most three values".into());
            }
            let v = parts.into_iter().map(e).collect::<Result<Vec<_>, _>>()?;
            Action::Show(corner, label, format, v)
        }
        _ => return Err(format!("unknown action \"{}\"", clip(verb))),
    };
    // a sim rule changes the run and reads only the run; a view rule never changes it
    if a.sim() && !sim {
        return Err(format!(
            "{verb} changes the run: only on a run's event (start, tick, sector, ...)"
        ));
    }
    // a screenshot or a restart is a single deed: only on a key, never every frame
    if matches!(a, Action::Screenshot | Action::Restart) && !key {
        return Err(format!("{verb} only on a key (on key <option>)"));
    }
    if !a.sim() && sim && !matches!(a, Action::Set(..)) {
        return Err(format!("{verb} is for the view: only on frame or key"));
    }
    let mut bad = None;
    for x in a.exprs() {
        x.leaves(&mut |leaf| match leaf {
            Expr::Value(v) if sim && !v.sim() => {
                bad = Some(format!("a run's rule cannot read {v:?}"))
            }
            _ => {}
        });
    }
    if let Some(b) = bad {
        return Err(b);
    }
    Ok(a)
}

/// The variables' sides: a variable a view rule sets is the view's, and no run rule may set or read it (the run
/// would then depend on what is seen); every other variable is the run's.
fn check_vars(m: &mut ModTxt) -> Result<(), String> {
    let n = m.vars.len();
    let (mut sim_set, mut view_set, mut sim_read) =
        (vec![false; n], vec![false; n], vec![false; n]);
    let mark = |v: &mut Vec<bool>, i: usize| {
        if let Some(x) = v.get_mut(i) {
            *x = true;
        }
    };
    for r in &m.rules {
        let sim = r.event.sim();
        let mut exprs: Vec<&Expr> = r.conditions.iter().collect();
        for a in &r.actions {
            if let Action::Set(i, _) = a {
                mark(if sim { &mut sim_set } else { &mut view_set }, *i);
            }
            exprs.extend(a.exprs());
        }
        if sim {
            for e in exprs {
                e.leaves(&mut |leaf| {
                    if let Expr::Var(i) = leaf {
                        mark(&mut sim_read, *i);
                    }
                });
            }
        }
    }
    for (i, v) in m.vars.iter_mut().enumerate() {
        let view = view_set.get(i).copied().unwrap_or(false);
        if view
            && (sim_set.get(i).copied().unwrap_or(false)
                || sim_read.get(i).copied().unwrap_or(false))
        {
            return Err(format!(
                "variable \"{}\" is set by a view rule (frame, key) and used by a run rule",
                v.0
            ));
        }
        v.2 = !view;
    }
    Ok(())
}

/// Parses a mod.txt (the fuzz target's entry): every line known and checked, else an error naming the line.
pub fn parse_mod_txt(text: &str) -> Result<ModTxt, String> {
    if text.len() as u64 > MAX_MOD_TXT {
        return Err(format!("mod.txt is larger than {MAX_MOD_TXT} bytes"));
    }
    let mut m = ModTxt {
        hash: crate::fnv1a(text.as_bytes()),
        ..ModTxt::default()
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut seen: Vec<String> = Vec::new();
    let mut actions = 0usize;
    for (n, raw) in text.lines().enumerate() {
        if n >= MAX_LINES {
            return Err(format!("mod.txt has more than {MAX_LINES} lines"));
        }
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |e: String| format!("line {}: {e}", n.saturating_add(1));
        let (key, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        let mut once = |k: &str| -> Result<(), String> {
            if seen.iter().any(|s| s == k) {
                return Err(at(format!("\"{k}\" given twice")));
            }
            seen.push(k.to_owned());
            Ok(())
        };
        let lang_of = |prefix: &str| {
            key.strip_prefix(prefix)
                .filter(|l| LANGUAGES.contains(l))
                .map(str::to_owned)
        };
        if let Some(lang) = lang_of("name.") {
            once(key)?;
            m.names.push((lang, text_value(rest).map_err(at)?));
            continue;
        }
        if let Some(lang) = lang_of("description.") {
            once(key)?;
            m.descriptions.push((lang, text_value(rest).map_err(at)?));
            continue;
        }
        let in_rule = !m.rules.is_empty();
        match key {
            "author" | "licence" | "version" | "requires" if !in_rule => {
                once(key)?;
                let v = text_value(rest).map_err(at)?;
                match key {
                    "author" => m.author = v,
                    "licence" => m.licence = v,
                    "version" => m.version = v,
                    _ => {
                        m.requires = version(&v)
                            .ok_or_else(|| at(format!("not a version: \"{}\"", clip(&v))))?
                    }
                }
            }
            "option" if !in_rule => {
                if m.options.len() >= MAX_OPTIONS {
                    return Err(at(format!("more than {MAX_OPTIONS} options")));
                }
                let o = parse_option(rest).map_err(at)?;
                if m.options.iter().any(|x| x.key == o.key) {
                    return Err(at("an option given twice".into()));
                }
                m.options.push(o);
            }
            "var" if !in_rule => {
                if m.vars.len() >= MAX_VARS {
                    return Err(at(format!("more than {MAX_VARS} variables")));
                }
                let mut f = rest.split_whitespace();
                let (Some(name), Some(v), None) = (f.next(), f.next(), f.next()) else {
                    return Err(at("expected: var <name> <number>".into()));
                };
                if !ident(name)
                    || m.vars.iter().any(|x| x.0 == name)
                    || FUNCS.iter().any(|f| f.0 == name)
                {
                    return Err(at(format!("not a new variable name: \"{}\"", clip(name))));
                }
                if ["and", "or", "not", "true", "false"].contains(&name) {
                    return Err(at("a reserved word".into()));
                }
                m.vars
                    .push((name.to_owned(), number(v).map_err(at)?, false));
            }
            "label" => {
                if m.labels.len() >= MAX_LABELS {
                    return Err(at(format!("more than {MAX_LABELS} labels")));
                }
                let (name, r) = rest
                    .split_once(char::is_whitespace)
                    .ok_or_else(|| at("expected: label <name> <lang> <text>".into()))?;
                let (lang, t) = r
                    .trim()
                    .split_once(char::is_whitespace)
                    .ok_or_else(|| at("expected: label <name> <lang> <text>".into()))?;
                if !ident(name) || !LANGUAGES.contains(&lang) {
                    return Err(at("a bad label name or language".into()));
                }
                if m.labels.iter().any(|(n, l, _)| n == name && l == lang) {
                    return Err(at("a label given twice".into()));
                }
                let t = text_value(t).map_err(at)?;
                // a label's languages are kept together, so its first place names it
                match m.labels.iter().rposition(|(n, _, _)| n == name) {
                    Some(p) => m
                        .labels
                        .insert(p.saturating_add(1), (name.to_owned(), lang.to_owned(), t)),
                    None => m.labels.push((name.to_owned(), lang.to_owned(), t)),
                }
                // Show refers to labels by place: once rules exist, no new label may move them
                if in_rule {
                    return Err(at("labels come before the rules".into()));
                }
            }
            "on" => {
                if m.rules.len() >= MAX_RULES {
                    return Err(at(format!("more than {MAX_RULES} rules")));
                }
                let event = parse_event(rest, &m.options).map_err(at)?;
                m.rules.push(Rule {
                    event,
                    conditions: Vec::new(),
                    actions: Vec::new(),
                });
            }
            "if" if in_rule => {
                let names_vars = m.vars.clone();
                let names = Names {
                    vars: &names_vars,
                    options: &m.options,
                };
                let e = expr(rest, &names).map_err(at)?;
                let sim = m.rules.last().is_some_and(|r| r.event.sim());
                let mut bad = false;
                e.leaves(&mut |leaf| match leaf {
                    Expr::Value(v) if sim && !v.sim() => bad = true,
                    _ => {}
                });
                if bad {
                    return Err(at(
                        "a run's rule reads only the run (and its own run variables)".into(),
                    ));
                }
                let r = m
                    .rules
                    .last_mut()
                    .ok_or_else(|| at("if outside a rule".into()))?;
                if !r.actions.is_empty() {
                    return Err(at("conditions come before the rule's actions".into()));
                }
                if r.conditions.len() >= MAX_CONDITIONS {
                    return Err(at(format!("more than {MAX_CONDITIONS} conditions")));
                }
                r.conditions.push(e);
            }
            "do" if in_rule => {
                actions = actions.saturating_add(1);
                if actions > MAX_ALL_ACTIONS {
                    return Err(at(format!("more than {MAX_ALL_ACTIONS} actions")));
                }
                let sim = m.rules.last().is_some_and(|r| r.event.sim());
                let key = m.rules.last().is_some_and(|r| matches!(r.event, Event::Key(_)));
                let a = parse_action(rest, &mut m, sim, key).map_err(at)?;
                let r = m
                    .rules
                    .last_mut()
                    .ok_or_else(|| at("do outside a rule".into()))?;
                if r.actions.len() >= MAX_ACTIONS {
                    return Err(at(format!("more than {MAX_ACTIONS} actions in a rule")));
                }
                r.actions.push(a);
            }
            _ => return Err(at(format!("unknown or misplaced line \"{}\"", clip(key)))),
        }
    }
    check_vars(&mut m)?;
    m.cost = m
        .rules
        .iter()
        .map(|r| {
            let conds = r.conditions.iter().map(Expr::size).fold(0usize, usize::saturating_add);
            let acts = r
                .actions
                .iter()
                .map(|a| a.exprs().iter().map(|e| e.size()).fold(1usize, usize::saturating_add))
                .fold(0usize, usize::saturating_add);
            conds.saturating_add(acts)
        })
        .fold(0usize, usize::saturating_add);
    if m.names.is_empty() {
        return Err("mod.txt names the mod in no language (name.en ...)".into());
    }
    if m.requires > GAME_VERSION {
        let (a, b, c) = m.requires;
        return Err(format!("needs game version {a}.{b}.{c}"));
    }
    Ok(m)
}

// --- mods on disk ------------------------------------------------------------------------------------------------

/// A mod found: its folder's name (the settings' key), whether it ships with the game, and what it says or why it
/// was refused.
#[derive(Clone, Debug, PartialEq)]
pub struct Mod {
    pub key: String,
    pub factory: bool,
    pub parsed: Result<ModTxt, String>,
}

impl Mod {
    pub fn name(&self, lang: &str) -> String {
        match &self.parsed {
            Ok(m) => m.name(lang, &self.key),
            Err(_) => self.key.clone(),
        }
    }
}

/// The player's mods folder, and the factory mods' (the game's assets).
pub fn user_root() -> PathBuf {
    crate::profile::data_dir().join("mods")
}

pub fn factory_root() -> Option<PathBuf> {
    crate::resources::assets_root().map(|a| a.join("mods"))
}

/// The files a mod's folder may hold beside mod.txt (never read by the game).
const SIDE_FILES: [&str; 6] = [
    "readme.txt",
    "README.txt",
    "README.md",
    "licence.txt",
    "license.txt",
    "LICENSE.txt",
];

/// Reads a mod's folder: refused unless it is a real folder holding only a real, small mod.txt and side files.
pub fn read_mod(dir: &Path) -> Result<ModTxt, String> {
    use std::io::Read;
    let meta = std::fs::symlink_metadata(dir).map_err(|e| e.to_string())?;
    if !meta.is_dir() {
        return Err("not a folder (a link?)".into());
    }
    let mut has_txt = false;
    for (n, e) in std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .enumerate()
    {
        if n >= 16 {
            return Err("too many files".into());
        }
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name();
        let name = name.to_str().ok_or("a file name that is not UTF-8")?;
        let m = std::fs::symlink_metadata(e.path()).map_err(|e| e.to_string())?;
        if !m.file_type().is_file() {
            return Err(format!(
                "\"{}\" is not a plain file (a folder or a link)",
                clip(name)
            ));
        }
        if name == "mod.txt" {
            if m.len() > MAX_MOD_TXT {
                return Err(format!("mod.txt is larger than {MAX_MOD_TXT} bytes"));
            }
            has_txt = true;
        } else if SIDE_FILES.contains(&name) {
            if m.len() > MAX_SIDE_FILE {
                return Err(format!("{name} is too large"));
            }
        } else {
            return Err(format!(
                "\"{}\" is not something a mod may hold",
                clip(name)
            ));
        }
    }
    if !has_txt {
        return Err("no mod.txt".into());
    }
    let f = open_plain(&dir.join("mod.txt")).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    f.take(MAX_MOD_TXT.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_MOD_TXT {
        return Err(format!("mod.txt is larger than {MAX_MOD_TXT} bytes"));
    }
    let text = String::from_utf8(bytes).map_err(|_| "mod.txt is not UTF-8".to_owned())?;
    parse_mod_txt(&text)
}

/// Opens a file without following a link at its last step (a link swapped in after the check is refused).
fn open_plain(p: &Path) -> std::io::Result<std::fs::File> {
    let mut o = std::fs::OpenOptions::new();
    o.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let f = o.open(p)?;
    if !f.metadata()?.is_file() {
        return Err(std::io::Error::other("not a plain file"));
    }
    Ok(f)
}

/// The mods in `root`: its plain-named entries by name, at most MAX_MODS, each read or refused.
pub fn mods_in(root: &Path, factory: bool) -> Vec<Mod> {
    let Ok(d) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    // the plain names first, then sorted, then capped: which mods are taken never depends on the folder's order
    let mut keys: Vec<String> = d
        .flatten()
        .take(MAX_ENTRIES)
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|k| crate::sound_themes::plain(k) && k.len() <= MAX_KEY && !k.contains('.'))
        .collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .take(MAX_MODS)
        .map(|key| {
            let parsed = read_mod(&root.join(&key));
            Mod {
                key,
                factory,
                parsed,
            }
        })
        .collect()
}

/// Every mod: the factory ones, then the player's (a player's folder named like a factory mod is left out).
pub fn scan() -> Vec<Mod> {
    let mut all = factory_root()
        .map(|r| mods_in(&r, true))
        .unwrap_or_default();
    for m in mods_in(&user_root(), false) {
        if all.len() < MAX_MODS.saturating_mul(2) && !all.iter().any(|f| f.key == m.key) {
            all.push(m);
        }
    }
    all
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;

    const STICKY: &str = "\u{feff}name.en Sticky ball\nname.ru Липкий шар\ndescription.en Holds to walls.\n\
        author Someone\nversion 1.0\nlicence CC0-1.0\nrequires 0.1\n\
        option strength number 10 100 10 60\nlabel strength en Strength\nvar holding 0\n\
        on tick\nif ball.touching\ndo ball.stick opt.strength / 100\ndo set holding = 1\n\
        on tick\nif not ball.touching\ndo ball.stick 0\n";

    struct S;
    impl Scope for S {
        fn value(&self, v: Value) -> f32 {
            if v == Value::BallSpeed { 10.0 } else { 0.0 }
        }
        fn var(&self, _: usize) -> f32 {
            2.0
        }
        fn opt(&self, _: usize) -> f32 {
            60.0
        }
    }

    #[test]
    fn a_mod_txt_reads_into_rules() {
        let m = parse_mod_txt(STICKY).unwrap();
        assert_eq!(m.name("ru", "k"), "Липкий шар");
        assert_eq!(m.name("fr", "k"), "Sticky ball");
        assert_eq!(m.rules.len(), 2);
        assert!(m.modded());
        assert!(m.vars[0].2, "a run variable");
        let Action::Stick(e) = &m.rules[0].actions[0] else {
            panic!()
        };
        assert_eq!(e.eval(&S), 0.6);
        assert_eq!(m.label("strength", "de"), "Strength");
    }

    #[test]
    fn expressions_are_total() {
        let names = Names {
            vars: &[("v".into(), 0.0, false)],
            options: &[],
        };
        let ev = |s: &str| expr(s, &names).unwrap().eval(&S);
        assert_eq!(ev("1 + 2 * 3"), 7.0);
        assert_eq!(ev("(1 + 2) * 3"), 9.0);
        assert_eq!(ev("1 / 0"), 0.0);
        assert_eq!(ev("sqrt(-4)"), 0.0);
        assert_eq!(ev("clamp(ball.speed, 0, 5)"), 5.0);
        assert_eq!(ev("if(ball.speed > 5 and not false, v, -1)"), 2.0);
    }

    #[test]
    fn the_run_and_the_view_stay_apart() {
        for bad in [
            "name.en A\non frame\ndo ball.stick 1\n", // a view rule changing the run
            "name.en A\non tick\ndo camera.fov 10\n", // a run rule changing the view
            "name.en A\non tick\nif real.time > 3\ndo ball.stick 1\n", // a run rule reading real time
            "name.en A\nvar v 0\non frame\ndo set v = 1\non tick\ndo ball.stick v\n",
            "name.en A\nvar v 0\non tick\ndo set v = 1\non frame\ndo set v = 2\n",
        ] {
            assert!(parse_mod_txt(bad).is_err(), "{bad:?}");
        }
        let ok = parse_mod_txt(
            "name.en A\nvar v 0\non tick\ndo set v = v + 1\non frame\ndo camera.fov v\n",
        )
        .unwrap();
        assert!(!ok.modded(), "setting a variable alone changes nothing");
    }

    #[test]
    fn anything_unknown_or_out_of_bounds_refuses_the_mod() {
        let deep = format!(
            "name.en A\non frame\ndo light {}1{}\n",
            "(".repeat(40),
            ")".repeat(40)
        );
        let wide = format!(
            "name.en A\non frame\ndo light {}\n",
            vec!["1"; 80].join("+")
        );
        let many_rules = "name.en A\n".to_owned() + &"on frame\n".repeat(MAX_RULES + 1);
        let many_opts: String = (0..=MAX_OPTIONS)
            .map(|i| format!("option o{i} switch on\n"))
            .collect();
        for bad in [
            "",
            "name.en A\nname.en B\n",
            "name.xx A\n",
            "name.en A\nexec rm -rf /\n",
            "name.en A\non boot\n",
            "name.en A\non frame\ndo system(\"x\")\n",
            "name.en A\non frame\ndo light 1e30\n",
            "name.en A\non frame\ndo light foo\n",
            "name.en A\non frame\ndo light min(1)\n",
            "name.en A\non frame\ndo light (1\n",
            "name.en A\non frame\ndo show middle - int 1\n",
            "name.en A\non frame\ndo show top-left nolabel int 1\n",
            "name.en A\non frame\ndo show top-left - int 1, 2, 3, 4\n",
            "name.en A\non key nokey\n",
            "name.en A\noption k key ../x\n",
            "name.en A\noption n number 5 1 1 3\n",
            "name.en A\nif 1\n",
            "name.en A\non frame\ndo restart\nif 1\n",
            "name.en A\non frame\ndo restart\n",
            "name.en A\non frame\ndo screenshot\n",
            "name.en A\noption k key Escape\n",
            "name.en A\noption k key KeyQ,ArrowUp\n",
            "name.en A\noption n number 10 100 10 37\n",
            "name.en A\noption n number 0 1 0.00001 0\n",
            "name.en A\nrequires 9.0\n",
            "name.en A\u{202e}B\n",
            "name.en A\x07\n",
            &deep,
            &wide,
            &many_rules,
            &("name.en A\n".to_owned() + &many_opts),
        ] {
            assert!(parse_mod_txt(bad).is_err(), "{bad:?}");
        }
        assert!(parse_mod_txt(&format!("name.en {}\n", "x".repeat(MAX_TEXT + 1))).is_err());
        assert!(parse_mod_txt(&("name.en A\n".to_owned() + &"# c\n".repeat(MAX_LINES))).is_err());
    }

    fn scratch(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("drahtseilakt-mods-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn hostile_mod_folders_are_refused_whole() {
        let root = scratch("hostile");
        let mk = |k: &str| {
            let d = root.join(k);
            std::fs::create_dir_all(&d).unwrap();
            d
        };
        let ok = mk("good");
        std::fs::write(ok.join("mod.txt"), STICKY).unwrap();
        std::fs::write(ok.join("readme.txt"), "hello").unwrap();
        let dll = mk("dll");
        std::fs::write(dll.join("mod.txt"), "name.en A\n").unwrap();
        std::fs::write(dll.join("Mod.bmodp"), b"MZ").unwrap();
        let sub = mk("sub");
        std::fs::create_dir_all(sub.join("textures")).unwrap();
        std::fs::write(sub.join("mod.txt"), "name.en A\n").unwrap();
        let huge = mk("huge");
        std::fs::write(huge.join("mod.txt"), "# x\n".repeat(20_000)).unwrap();
        let bin = mk("bin");
        std::fs::write(bin.join("mod.txt"), b"name.en \xff\xfe\n").unwrap();
        mk("empty");
        mk(".hidden");
        mk("sp ace");
        #[cfg(unix)]
        {
            let link = mk("link");
            std::os::unix::fs::symlink(ok.join("mod.txt"), link.join("mod.txt")).unwrap();
            std::os::unix::fs::symlink(&ok, root.join("linkdir")).unwrap();
        }
        let found = mods_in(&root, false);
        let get = |k: &str| found.iter().find(|m| m.key == k);
        assert!(get("good").unwrap().parsed.is_ok());
        for k in ["dll", "sub", "huge", "bin", "empty"] {
            assert!(get(k).unwrap().parsed.is_err(), "{k}");
        }
        assert!(get(".hidden").is_none() && get("sp ace").is_none());
        #[cfg(unix)]
        for k in ["link", "linkdir"] {
            assert!(get(k).unwrap().parsed.is_err(), "{k}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn options_clamp_step_and_show() {
        let o = parse_option("strength number 10 100 10 60").unwrap();
        assert_eq!(o.clamp(37.0), 40.0);
        assert_eq!(o.clamp(1000.0), 100.0);
        assert_eq!(o.parse("5"), None);
        assert_eq!(o.stepped(60.0, 1), 70.0);
        let k = parse_option("key key F12,F10,KeyP").unwrap();
        assert_eq!(k.show(2.0), "KeyP");
        assert_eq!(k.stepped(2.0, 1), 0.0);
        assert_eq!(k.parse("F10"), Some(1.0));
    }
}
