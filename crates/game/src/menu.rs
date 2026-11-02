// The menus as the original's Menu.nmo has them (docs/drahtseilakt/menu.md): the main menu, the level list, the
// highscores, the options and their pages, the credits; over a level the pause, dead and end menus, the yes/no
// questions and the new highscore's name entry. The words are the copy's (lang.rs), the layout the original's 2D
// frames as the C game shows them. The extended mode adds the custom levels and a page of its own; Options adds the
// Language page. Chosen with the arrows and Enter; drawn with the interface's textures (our own, or the copy's) and
// its face (ui.rs), or with the prototype's font when there is neither.

use crate::assets::Data;
use crate::features::{FEATURES, Feature, Features};
use crate::highscore::{NAME_MAX, Table};
use crate::hud::{Rect, text, width};
use crate::lang::{LANGUAGES, Strings, latin1};
use crate::levels::Kind;
use crate::options::{Bindings, Fullscreen, Graphics};
use gamedata::package::Package;
use std::path::{Path, PathBuf};

/// The original game's levels: 1 to 12, and 13 of the add-on when the copy has it.
pub const LEVELS: u32 = 13;
/// Lines of a long list (custom levels) shown at once.
const LIST_LINES: usize = 12;

/// The Controls page's rows: the copy's names for them (Language.nmo), in options::ACTIONS' order.
const KEY_NAMES: [(&str, &str); 6] = [
    ("Menu_Opt_Keys Forward", "Forward"),
    ("Menu_Opt_Keys Backward", "Backward"),
    ("Menu_Opt_Keys Left", "Left"),
    ("Menu_Opt_Keys Right", "Right"),
    ("Menu_Opt_Keys Rotation", "Rotation"),
    ("Menu_Opt_Keys Overview", "Overview"),
];

/// The size `by` places from `now` among `sizes` (sorted), round; from where `now` would be when it is not one.
fn step_size(sizes: &[(u32, u32)], now: (u32, u32), by: i32) -> Option<(u32, u32)> {
    let n = i64::try_from(sizes.len()).ok().filter(|&n| n > 0)?;
    let area = |s: &(u32, u32)| (u64::from(s.0) * u64::from(s.1), s.0);
    let at = match sizes.iter().position(|&s| s == now) {
        Some(i) => i64::try_from(i).ok()?,
        // between two sizes: one step lands on the size next to it in that direction
        None => {
            let after = i64::try_from(sizes.iter().filter(|s| area(s) < area(&now)).count()).ok()?;
            if by > 0 { after - 1 } else { after }
        }
    };
    sizes.get(usize::try_from((at + i64::from(by)).rem_euclid(n)).ok()?).copied()
}

pub struct Entry {
    pub number: u32,
    /// The level file is in the copy.
    pub available: bool,
    /// Playable: the level before it is finished (as the original unlocks them), or the extended mode is on.
    pub open: bool,
    /// The best finished run, in ticks.
    pub best: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Main,
    /// The extended mode's choice of levels: the original, remastered, community and open-format ones.
    Kinds,
    Start,
    Custom,
    Highscore,
    Options,
    Graphics,
    Controls,
    Sound,
    Extended,
    /// The extended mode's debug additions (practice tools, endless lives).
    Debug,
    /// The menus' language.
    Language,
    Credits,
    /// Over a level: Esc in a run, no lives left, the level finished.
    Pause,
    Dead,
    End,
    Ask(Ask),
    /// A new highscore: the player types a name.
    Entry,
}

/// The yes/no questions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    Quit,
    Restart,
    Exit,
}

/// What a line of a screen is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    Start,
    Highscore,
    Options,
    Credits,
    Exit,
    Back,
    Restart,
    ExitLevel,
    Next,
    Home,
    Graphics,
    Controls,
    Sound,
    Extended,
    Level(usize),
    /// A kind of levels, by its place in LEVEL_KINDS.
    Kind(usize),
    Map(usize),
    Music,
    Effects,
    Mode,
    /// The Debug mode screen.
    DebugMenu,
    /// Where the interface's files come from: own, the original, its remaster (resources.rs).
    Interface,
    /// The sound theme (sound_themes.rs): the arrows go round the themes found.
    SoundTheme,
    /// The Language page, and a language on it (by its place in lang::LANGUAGES).
    Language,
    Lang(usize),
    /// One of the extended mode's additions, by its place in FEATURES.
    Switch(usize),
    Yes,
    No,
    Ok,
    /// The Graphics page: the window's size, synch to screen, the clouds, the full screen, the field of view.
    Resolution,
    Synch,
    Clouds,
    Fullscreen,
    Fov,
    /// The Controls page: an action's key (by its place in options::ACTIONS), the turn inverted, the original's keys.
    Key(usize),
    Invert,
    Defaults,
}

/// What a key in the menu asks of the game.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    None,
    /// Close the menu: back to the run.
    Continue,
    Play(u32),
    Map(PathBuf),
    Restart,
    /// Leave the level for the main menu.
    Home,
    Quit,
    /// The extended mode on or off.
    ToggleDebug,
    /// One of the extended mode's additions on or off.
    Switch(Feature),
    /// The volumes changed: sound and music, 0 to 1.
    Volumes([f32; 2]),
    /// The interface's provider chosen (it takes effect on the next start).
    Interface(gamedata::resources::Provider),
    /// The sound theme chosen (its key).
    SoundTheme(String),
    /// The menus' language chosen (its code): the game saves it and speaks it at once.
    Language(String),
    /// The new highscore's name was entered: the game saves it and shows the list.
    Score(Vec<u8>),
    /// The Graphics page changed: the game applies and saves it.
    Graphics(Graphics),
    /// The keys changed: the game takes and saves them.
    Bindings(Bindings),
}

pub struct Menu {
    /// The screens open, the first the menu's base, each with its chosen line.
    stack: Vec<(Screen, usize)>,
    pub entries: Vec<Entry>,
    /// The custom levels shown: the levels of the kind's folder, by title (levels.md, "A level is a folder").
    pub maps: Vec<Package>,
    /// The kind's folder the custom levels come from.
    folder: Option<PathBuf>,
    /// The extended mode and its additions.
    pub features: Features,
    pub volumes: [f32; 2],
    pub strings: Strings,
    pub scores: Table,
    /// The level being played (the first level when none is).
    pub level: u32,
    /// The level whose highscores are shown.
    shown: u32,
    /// The window's size, shown on the Graphics page.
    pub resolution: (u32, u32),
    /// The name being typed for a new highscore, and its points.
    pub name: Vec<u8>,
    pub points: i64,
    /// The highscore list was reached from the name entry: its button reads Next.
    after_entry: bool,
    /// The level played is a custom level: no highscores and no next level.
    pub custom: bool,
    /// The Graphics page's choices, and the sizes its resolution goes through (the monitor's and window sizes).
    pub graphics: Graphics,
    pub resolutions: Vec<(u32, u32)>,
    /// The Controls page's keys, and the action whose new key is awaited.
    pub bindings: Bindings,
    waiting: Option<usize>,
}

impl Menu {
    /// The menu with `base` as its first screen (Main, or Pause, Dead or End over original level `current`, or over
    /// a custom level when it is None).
    pub fn new(
        data: &Data,
        strings: &Strings,
        base: Screen,
        current: Option<u32>,
        features: Features,
    ) -> Menu {
        let entries: Vec<Entry> = (1..=LEVELS)
            .map(|number| Entry {
                number,
                available: data.original(number).is_some(),
                open: features.has(Feature::AllLevels) || number == 1 || finished(number - 1),
                best: features
                    .has(Feature::BestTimes)
                    .then(|| best_ticks(number))
                    .flatten(),
            })
            .collect();
        let mut m = Menu {
            stack: Vec::new(),
            entries,
            maps: Vec::new(),
            folder: None,
            features,
            volumes: [1.0; 2],
            strings: strings.clone(),
            scores: Table::load(&strings.highscores),
            level: current.unwrap_or(1).max(1),
            shown: current.unwrap_or(1).max(1),
            resolution: (0, 0),
            name: Vec::new(),
            points: 0,
            after_entry: false,
            custom: current.is_none(),
            graphics: Graphics::default(),
            resolutions: Vec::new(),
            bindings: Bindings::load(|_| None, strings.controls, strings.invert),
            waiting: None,
        };
        m.push(base);
        m
    }

    pub fn screen(&self) -> Screen {
        self.stack.last().map_or(Screen::Main, |s| s.0)
    }

    /// The menu is over a level (pause, dead, end and what they open): the level stays in view behind it.
    pub fn over_level(&self) -> bool {
        matches!(
            self.stack.first(),
            Some((Screen::Pause | Screen::Dead | Screen::End, _))
        )
    }

    fn chosen(&self) -> usize {
        self.stack.last().map_or(0, |s| s.1)
    }

    fn set_chosen(&mut self, i: usize) {
        if let Some(s) = self.stack.last_mut() {
            s.1 = i;
        }
    }

    /// Opens `screen` over the current one, its line chosen as the original chooses it.
    pub fn push(&mut self, screen: Screen) {
        let lines = self.lines(screen);
        let at = |l: Line| lines.iter().position(|&x| x == l);
        let first = match screen {
            Screen::Pause
            | Screen::Options
            | Screen::Highscore
            | Screen::Credits
            | Screen::Graphics
            | Screen::Controls => at(Line::Back),
            Screen::Ask(_) => at(Line::No),
            Screen::End => at(Line::Next),
            Screen::Start => self
                .entries
                .iter()
                .position(|e| e.number == self.level && e.available && e.open)
                .and_then(|i| at(Line::Level(i))),
            Screen::Language => {
                LANGUAGES.iter().position(|l| l.0 == self.strings.language()).and_then(|i| at(Line::Lang(i)))
            }
            _ => None,
        };
        if screen == Screen::Highscore {
            self.shown = self.level;
        }
        self.stack.push((screen, first.unwrap_or(0)));
        if !self.selectable(screen, self.chosen()) {
            self.step(1);
        }
    }

    fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    /// The lines of `screen`, in the order the arrows go through them.
    fn lines(&self, screen: Screen) -> Vec<Line> {
        match screen {
            Screen::Main => vec![
                Line::Start,
                Line::Highscore,
                Line::Options,
                Line::Credits,
                Line::Exit,
            ],
            Screen::Kinds => (0..LEVEL_KINDS.len()).map(Line::Kind).chain([Line::Back]).collect(),
            Screen::Start => (0..self.entries.len())
                .map(Line::Level)
                .chain([Line::Back])
                .collect(),
            Screen::Custom => (0..self.maps.len())
                .map(Line::Map)
                .chain([Line::Back])
                .collect(),
            Screen::Options => vec![
                Line::Graphics,
                Line::Controls,
                Line::Sound,
                Line::Language,
                Line::Extended,
                Line::Back,
            ],
            Screen::Language => (0..LANGUAGES.len()).map(Line::Lang).chain([Line::Back]).collect(),
            // the original's two volumes: the music's and the effects'
            Screen::Sound => vec![Line::Music, Line::Effects, Line::Back],
            Screen::Extended if self.features.mode => [Line::Mode]
                .into_iter()
                .chain(switches(false))
                .chain([Line::Interface, Line::SoundTheme, Line::DebugMenu, Line::Back])
                .collect(),
            Screen::Debug => switches(true).chain([Line::Back]).collect(),
            Screen::Extended => vec![Line::Mode, Line::Back],
            Screen::Highscore if self.after_entry => vec![Line::Next],
            // the original's lines, the full screen (a window can be one), and the extended mode's field of view
            Screen::Graphics => {
                let mut l = vec![Line::Resolution, Line::Synch, Line::Clouds, Line::Fullscreen];
                if self.features.mode {
                    l.push(Line::Fov);
                }
                l.push(Line::Back);
                l
            }
            // the original's lines, and the extended mode's way back to the original's keys
            Screen::Controls => {
                let mut l: Vec<Line> = (0..crate::options::ACTIONS.len()).map(Line::Key).collect();
                if self.features.mode {
                    l.push(Line::Defaults);
                }
                l.push(Line::Invert);
                l.push(Line::Back);
                l
            }
            Screen::Highscore | Screen::Credits => vec![Line::Back],
            Screen::Pause if self.custom => {
                vec![Line::Restart, Line::Options, Line::ExitLevel, Line::Back]
            }
            Screen::Pause => vec![
                Line::Restart,
                Line::Highscore,
                Line::Options,
                Line::ExitLevel,
                Line::Back,
            ],
            Screen::Dead => vec![Line::Restart, Line::Home],
            Screen::End if self.custom => vec![Line::Restart, Line::Options, Line::Home],
            Screen::End => {
                let mut l = vec![Line::Restart, Line::Highscore, Line::Options, Line::Home];
                if self.next_level().is_some() {
                    l.push(Line::Next);
                }
                l
            }
            Screen::Ask(_) => vec![Line::Yes, Line::No],
            Screen::Entry => vec![Line::Ok],
        }
    }

    fn selectable(&self, screen: Screen, i: usize) -> bool {
        match self.lines(screen).get(i) {
            Some(Line::Level(k)) => self.entries.get(*k).is_some_and(|e| e.available && e.open),
            Some(_) => true,
            None => false,
        }
    }

    /// The level after the one played, if the copy has it.
    fn next_level(&self) -> Option<u32> {
        let n = self.level.checked_add(1)?;
        self.entries
            .iter()
            .find(|e| e.number == n && e.available)
            .map(|e| e.number)
    }

    /// Up (-1) or down (+1): the next line that can be chosen.
    pub fn step(&mut self, by: isize) {
        let screen = self.screen();
        let n = self.lines(screen).len() as isize;
        let mut i = self.chosen() as isize;
        for _ in 0..n {
            i = (i + by).rem_euclid(n.max(1));
            if self.selectable(screen, i as usize) {
                self.set_chosen(i as usize);
                return;
            }
        }
    }

    /// Left (-1) or right (+1): a volume down or up by a tenth, the highscores' level, the yes/no choice.
    pub fn adjust(&mut self, by: i32) -> Action {
        let line = self.lines(self.screen()).get(self.chosen()).copied();
        match (self.screen(), line) {
            (Screen::Sound, Some(l @ (Line::Music | Line::Effects))) => {
                let k = if l == Line::Effects { 0 } else { 1 };
                let Some(v) = self.volumes.get_mut(k) else {
                    return Action::None;
                };
                *v = ((*v * 10.0).round() + by as f32).clamp(0.0, 10.0) / 10.0;
                Action::Volumes(self.volumes)
            }
            (Screen::Highscore, _) => {
                let last = self
                    .entries
                    .iter()
                    .filter(|e| e.available)
                    .map(|e| e.number)
                    .max()
                    .unwrap_or(1);
                let next = i64::from(self.shown) + i64::from(by);
                self.shown = if next < 1 {
                    last
                } else if next > i64::from(last) {
                    1
                } else {
                    next as u32
                };
                Action::None
            }
            (Screen::Ask(_), _) => {
                self.step(by as isize);
                Action::None
            }
            (Screen::Graphics, Some(l)) => self.change_graphics(l, by),
            (Screen::Controls, Some(Line::Invert)) => self.toggle_invert(),
            (Screen::Extended, Some(Line::Mode)) => Action::ToggleDebug,
            (Screen::Extended, Some(Line::Interface)) => Action::Interface(next_provider(by)),
            (Screen::Extended, Some(Line::SoundTheme)) => Action::SoundTheme(next_theme(by)),
            (Screen::Extended | Screen::Debug, Some(Line::Switch(i))) => {
                FEATURES.get(i).map_or(Action::None, |&f| Action::Switch(f))
            }
            _ => Action::None,
        }
    }

    /// Enter on the current screen.
    pub fn enter(&mut self) -> Action {
        let screen = self.screen();
        let Some(line) = self.lines(screen).get(self.chosen()).copied() else {
            return Action::None;
        };
        let open = |m: &mut Menu, s: Screen| {
            m.push(s);
            Action::None
        };
        match line {
            // in the extended mode the levels come by kind; the original ones are the classic list
            Line::Start if self.features.has(Feature::CustomLevels) => open(self, Screen::Kinds),
            Line::Start => open(self, Screen::Start),
            Line::Kind(k) => match LEVEL_KINDS.get(k) {
                Some(Kind::Original) => open(self, Screen::Start),
                Some(&kind) => {
                    let root = crate::levels::dir(kind);
                    self.maps = maps_in(&root);
                    self.folder = Some(root);
                    open(self, Screen::Custom)
                }
                None => Action::None,
            },
            Line::Highscore => open(self, Screen::Highscore),
            Line::Options => open(self, Screen::Options),
            Line::Credits => open(self, Screen::Credits),
            Line::Graphics => open(self, Screen::Graphics),
            Line::Controls => open(self, Screen::Controls),
            Line::Sound => open(self, Screen::Sound),
            Line::Extended => open(self, Screen::Extended),
            Line::DebugMenu => open(self, Screen::Debug),
            Line::Language => open(self, Screen::Language),
            Line::Lang(i) => match LANGUAGES.get(i) {
                Some(&(code, _)) => {
                    self.pop();
                    Action::Language(code.to_owned())
                }
                None => Action::None,
            },
            Line::Exit => open(self, Screen::Ask(Ask::Quit)),
            Line::ExitLevel => open(self, Screen::Ask(Ask::Exit)),
            // the pause asks first; the dead and end menus restart at once
            Line::Restart if screen == Screen::Pause => open(self, Screen::Ask(Ask::Restart)),
            Line::Restart => Action::Restart,
            Line::Home => Action::Home,
            Line::Next if screen == Screen::Highscore => {
                self.after_entry = false;
                self.pop();
                Action::None
            }
            Line::Next => self.next_level().map_or(Action::None, Action::Play),
            Line::Level(i) => match self.entries.get(i) {
                Some(e) if e.available && e.open => Action::Play(e.number),
                _ => Action::None,
            },
            Line::Map(i) => self.maps.get(i).map_or(Action::None, |p| Action::Map(p.level.clone())),
            Line::Mode => Action::ToggleDebug,
            Line::Interface => Action::Interface(next_provider(1)),
            Line::SoundTheme => Action::SoundTheme(next_theme(1)),
            Line::Switch(i) => FEATURES.get(i).map_or(Action::None, |&f| Action::Switch(f)),
            Line::Music | Line::Effects => Action::None,
            Line::Resolution | Line::Synch | Line::Clouds | Line::Fullscreen | Line::Fov => {
                self.change_graphics(line, 1)
            }
            Line::Key(i) => {
                self.waiting = Some(i);
                Action::None
            }
            Line::Invert => self.toggle_invert(),
            Line::Defaults => {
                self.bindings = Bindings::default();
                Action::Bindings(self.bindings)
            }
            Line::Yes => match screen {
                Screen::Ask(Ask::Quit) => Action::Quit,
                Screen::Ask(Ask::Restart) => Action::Restart,
                Screen::Ask(Ask::Exit) => Action::Home,
                _ => Action::None,
            },
            Line::No | Line::Back => self.back(),
            // a highscore needs a name
            Line::Ok if self.name.iter().all(|&c| c == b' ') => Action::None,
            Line::Ok => Action::Score(self.name.clone()),
        }
    }

    /// Esc: back a screen; the pause menu back to the run. A key being awaited is left as it was.
    pub fn back(&mut self) -> Action {
        if self.waiting.take().is_some() {
            return Action::None;
        }
        match self.screen() {
            Screen::Pause => Action::Continue,
            Screen::Main | Screen::Dead | Screen::End | Screen::Entry => Action::None,
            Screen::Highscore if self.after_entry => self.enter(),
            _ => {
                if self.screen() == Screen::Custom {
                    self.folder = None;
                }
                self.pop();
                Action::None
            }
        }
    }

    /// A Graphics line changed by Enter or an arrow: the next size, a switch flipped, the next full screen, the
    /// field of view a step wider or narrower.
    fn change_graphics(&mut self, line: Line, by: i32) -> Action {
        let g = &mut self.graphics;
        match line {
            Line::Resolution => {
                let Some(next) = step_size(&self.resolutions, self.resolution, by) else {
                    return Action::None;
                };
                g.resolution = Some(next);
                self.resolution = next;
            }
            Line::Synch => g.vsync = !g.vsync,
            Line::Clouds => g.clouds = !g.clouds,
            Line::Fullscreen => g.fullscreen = g.fullscreen.next(by),
            Line::Fov => g.fov = g.fov_by(by),
            _ => return Action::None,
        }
        Action::Graphics(*g)
    }

    fn toggle_invert(&mut self) -> Action {
        self.bindings.invert = !self.bindings.invert;
        Action::Bindings(self.bindings)
    }

    /// A new key is awaited (Enter on a Controls row).
    pub fn waiting(&self) -> bool {
        self.waiting.is_some()
    }

    /// The key pressed while one is awaited: bound to the action (a key another action has is swapped). Esc gives
    /// the wait up; a key that cannot be bound leaves it on.
    pub fn capture(&mut self, code: winit::keyboard::KeyCode) -> Action {
        let Some(action) = self.waiting else {
            return Action::None;
        };
        if code == winit::keyboard::KeyCode::Escape {
            self.waiting = None;
            return Action::None;
        }
        match self.bindings.bind(action, code) {
            Ok(_) => {
                self.waiting = None;
                Action::Bindings(self.bindings)
            }
            Err(_) => Action::None,
        }
    }

    /// Text typed on the name entry (Latin-1 letters, digits, space and punctuation the font has).
    pub fn typed(&mut self, s: &str) {
        if self.screen() != Screen::Entry {
            return;
        }
        for c in s.chars() {
            if self.name.len() >= NAME_MAX {
                break;
            }
            if let Ok(b) = u8::try_from(u32::from(c))
                && (b' '..=b'~').contains(&b) | (b >= 0xa0)
            {
                self.name.push(b);
            }
        }
    }

    /// Backspace on the name entry.
    pub fn erase(&mut self) {
        if self.screen() == Screen::Entry {
            self.name.pop();
        }
    }

    /// After the name was entered: the list with the new place, then the end menu.
    pub fn show_scores(&mut self, scores: Table) {
        self.scores = scores;
        self.pop();
        self.after_entry = true;
        self.push(Screen::Highscore);
    }

    /// Asks for a name: `points` made level `level`'s highscore list.
    pub fn ask_name(&mut self, name: Vec<u8>, points: i64) {
        self.name = name;
        self.points = points;
        self.push(Screen::Entry);
    }

    /// The extended mode or one of its additions switched: the lines that depend on them follow.
    pub fn set_features(&mut self, features: Features, data: &Data) {
        let fresh = Menu::new(
            data,
            &self.strings,
            Screen::Main,
            Some(self.level),
            features,
        );
        self.apply_mode(features, fresh.entries, fresh.maps);
    }

    /// The mode and what depends on it set; each open screen keeps its chosen line, wherever the lines move to.
    fn apply_mode(&mut self, features: Features, entries: Vec<Entry>, maps: Vec<Package>) {
        let chosen: Vec<Option<Line>> = self
            .stack
            .iter()
            .map(|&(sc, i)| self.lines(sc).get(i).copied())
            .collect();
        self.entries = entries;
        self.maps = maps;
        self.folder = None;
        self.features = features;
        for k in 0..self.stack.len() {
            let Some(&(sc, i)) = self.stack.get(k) else {
                continue;
            };
            let lines = self.lines(sc);
            let at = chosen
                .get(k)
                .copied()
                .flatten()
                .and_then(|l| lines.iter().position(|&x| x == l));
            if let Some(e) = self.stack.get_mut(k) {
                e.1 = at.unwrap_or(i.min(lines.len().saturating_sub(1)));
            }
        }
    }

    /// The label of a line in the menus' words.
    fn label(&self, line: Line) -> String {
        let s = &self.strings;
        match line {
            Line::Start => s.get("Menu_Main Button Start", "Start"),
            Line::Kind(k) => match LEVEL_KINDS.get(k) {
                Some(Kind::Original) => s.ours("kind original"),
                Some(Kind::Remastered) => s.ours("kind remastered"),
                Some(Kind::Community) => s.ours("kind community"),
                Some(Kind::Open) => s.ours("kind open"),
                None => String::new(),
            },
            Line::Highscore => s.get("Menu_Main Button Highscore", "Highscore"),
            Line::Options => s.get("Menu_Main Button Options", "Options"),
            Line::Credits => s.get("Menu_Main Button Credits", "Credits"),
            Line::Exit => s.get("Menu_Main Button ExitApplication", "Exit"),
            Line::Back => s.get("Menu_Start Button Back", "Back"),
            Line::Restart => s.get("Menu_Pause Button Restart", "Restart Level"),
            Line::ExitLevel => s.get("Menu_Pause Button Exit", "Exit Level"),
            Line::Next if self.screen() == Screen::Highscore => s.get("Menu_Highscore \"Next\"", "Next"),
            Line::Next => s.get("Menu_End Next Level", "Next level"),
            Line::Home => s.get("Menu_Dead Home", "Home"),
            Line::Graphics => s.get("Menu_Options Button Graphics", "Graphics"),
            Line::Controls => s.get("Menu_Options Button Controls", "Controls"),
            Line::Sound => s.get("Menu_Options Button Sound", "Sound"),
            Line::Language => s.ours("language"),
            Line::Lang(i) => LANGUAGES.get(i).map_or("", |l| l.1).to_owned(),
            Line::Extended => s.ours("extended"),
            Line::Level(i) => {
                let n = self.entries.get(i).map_or(0, |e| e.number);
                // the copy's own label for the level, else ours
                let copy = s.get(&format!("Menu_Start Button Level{n}"), "");
                if copy.is_empty() { s.numbered("level", n) } else { copy }
            }
            // the level's title (cleaned and capped, package.rs), any script
            Line::Map(i) => self.map_text(i).unwrap_or_default(),
            Line::Music => s.get("Menu_Opt_Sound Volume", "Music Volume"),
            Line::Effects => s.ours("sound volume"),
            Line::Mode => s.ours("extended mode"),
            Line::DebugMenu => s.ours("debug mode"),
            Line::Interface => s.ours("interface"),
            Line::SoundTheme => s.ours("sounds"),
            Line::Switch(i) => FEATURES.get(i).map_or_else(String::new, |f| s.ours(&format!("feature {}", f.key()))),
            Line::Yes => s.get("Menu_Opt_Graphics Synch Yes", "Yes"),
            Line::No => s.get("Menu_Opt_Graphics Synch No", "No"),
            Line::Ok => s.get("Menu_YesNo YES", "OK"),
            Line::Resolution => s.get("Menu_Opt_Graphics Resolution", "Screen Resolution"),
            Line::Synch => s.get("Menu_Opt_Graphics Synch", "Synch to Screen?"),
            Line::Clouds => s.get("Menu_Opt_Graphics Clouds", "Clouds?"),
            Line::Fullscreen => s.ours("fullscreen"),
            Line::Fov => s.ours("field of view"),
            Line::Key(i) => {
                let (key, name) = KEY_NAMES.get(i).copied().unwrap_or(("", ""));
                s.get(key, name)
            }
            Line::Invert => s.get("Menu_Opt_Keys Invert Cam Rot?", "Invert Rotation?"),
            Line::Defaults => s.ours("defaults"),
        }
    }

    /// A custom level's title as it is (any script), a cut one's "…" as "..." (the game's font has no "…").
    fn map_text(&self, i: usize) -> Option<String> {
        self.maps.get(i).map(|p| p.title.replace('…', "..."))
    }

    /// A switch's row: its label and yes or no.
    fn switch_field(&self, line: Line) -> (String, String) {
        let on = match line {
            Line::Switch(i) => FEATURES.get(i).is_some_and(|&f| self.features.switched_on(f)),
            _ => false,
        };
        let s = &self.strings;
        let yes = if on {
            s.get("Menu_Opt_Graphics Synch Yes", "Yes")
        } else {
            s.get("Menu_Opt_Graphics Synch No", "No")
        };
        (self.label(line), yes)
    }

    /// A screen's title, if it has one.
    fn title(&self, screen: Screen) -> Option<String> {
        let s = &self.strings;
        Some(match screen {
            Screen::Options => s.get("Menu_Options Title", "Options"),
            Screen::Graphics => s.get("Menu_Options Button Graphics", "Graphics"),
            Screen::Controls => s.get("Menu_Options Button Controls", "Controls"),
            Screen::Sound => s.get("Menu_Options Button Sound", "Sound"),
            Screen::Extended => s.ours("extended"),
            Screen::Debug => s.ours("debug mode"),
            Screen::Language => s.ours("language"),
            Screen::Kinds => s.ours("levels"),
            Screen::Custom => match self.folder.as_ref().and_then(|root| {
                LEVEL_KINDS.iter().position(|&k| crate::levels::dir(k) == *root)
            }) {
                Some(k) => self.label(Line::Kind(k)),
                None => s.ours("custom levels"),
            },
            Screen::Highscore => s.numbered("Highscore Title (+ SPACE!)", self.shown),
            Screen::Ask(Ask::Quit) => {
                s.get("Menu_YesNo Exit Game?", "Do you want to quit the game?")
            }
            Screen::Ask(Ask::Restart) => s.get(
                "Menu_YesNo Restart Level?",
                "Do you want to restart the level?",
            ),
            Screen::Ask(Ask::Exit) => {
                s.get("Menu_YesNo Exit Level?", "Do you want to exit the level?")
            }
            Screen::Entry => s.get(
                "Menu_HighscoreEntry \"Enter Highscore\"",
                "New highscore entry!",
            ),
            _ => return None,
        })
    }

    /// Lines of text a page shows besides its buttons: (label, value).
    fn fields(&self, screen: Screen) -> Vec<(String, String)> {
        let s = &self.strings;
        let yes_no = |b: bool| {
            if b {
                s.get("Menu_Opt_Graphics Synch Yes", "Yes")
            } else {
                s.get("Menu_Opt_Graphics Synch No", "No")
            }
        };
        match screen {
            Screen::Graphics => {
                let g = &self.graphics;
                let full = match g.fullscreen {
                    Fullscreen::Off => yes_no(false),
                    Fullscreen::Borderless => yes_no(true),
                    Fullscreen::Exclusive => s.ours("exclusive"),
                };
                let mut f = vec![
                    (
                        self.label(Line::Resolution),
                        format!("{} * {}", self.resolution.0, self.resolution.1),
                    ),
                    (self.label(Line::Synch), yes_no(g.vsync)),
                    (self.label(Line::Clouds), yes_no(g.clouds)),
                    (self.label(Line::Fullscreen), full),
                ];
                if self.features.mode {
                    f.push((self.label(Line::Fov), format!("{}", g.fov)));
                }
                f
            }
            Screen::Controls => {
                let mut f: Vec<_> = self
                    .bindings
                    .keys
                    .iter()
                    .enumerate()
                    .map(|(i, &k)| {
                        let key = if self.waiting == Some(i) {
                            s.ours("press a key")
                        } else {
                            crate::options::key_label(k, |row, name| s.key(row, name))
                        };
                        (self.label(Line::Key(i)), key)
                    })
                    .collect();
                if self.features.mode {
                    f.push((self.label(Line::Defaults), String::new()));
                }
                f.push((self.label(Line::Invert), yes_no(self.bindings.invert)));
                f
            }
            Screen::Sound => {
                let pct = |v: f32| format!("{}", (v * 10.0).round() as u32);
                vec![
                    (self.label(Line::Music), pct(self.volumes[1])),
                    (self.label(Line::Effects), pct(self.volumes[0])),
                ]
            }
            Screen::Extended => {
                // the mode, then each addition's switch, then where things are
                let mut f = vec![(self.label(Line::Mode), yes_no(self.features.mode))];
                if self.features.mode {
                    f.extend(switches(false).map(|l| self.switch_field(l)));
                    f.push((self.label(Line::Interface), self.interface_value()));
                    f.push((self.label(Line::SoundTheme), theme_value()));
                    f.push((self.label(Line::DebugMenu), ">".to_owned()));
                }
                f
            }
            Screen::Debug => {
                let mut f: Vec<(String, String)> = switches(true).map(|l| self.switch_field(l)).collect();
                for l in ["help 1", "help 2", "help 3"] {
                    f.push((s.ours(l), String::new()));
                }
                f
            }
            Screen::Credits => [
                "Drahtseilakt by Veresk",
                "",
                "Ballance by Cyparade / Atari 2004",
                "Started as a fork of OpenBallance by emdzej",
                "Physics: IVP, ported from OpenBallance",
                "Music: Kevin MacLeod (incompetech.com, CC-BY 4.0),",
                "elerya, Hitctrl, Spring Spring, WakianTech (K-ManZ),",
                "Tsorthan Grove, robcro6010, CarlosCarty, Amingliang,",
                "juskiddink, botpl (CC-BY), neolein, Tozan (CC0)",
                "Sounds: Kenney, freesound and OpenGameArt authors",
                "(CC0, CC-BY): assets/sounds/system/CREDITS.md",
            ]
            .iter()
            .map(|l| ((*l).to_owned(), String::new()))
            .collect(),
            Screen::Highscore => self
                .scores
                .list(self.shown)
                .iter()
                .map(|(n, p)| (latin1(n), p.to_string()))
                .collect(),
            Screen::Entry => vec![
                (
                    s.get("Menu_HighscoreEntry \"Points\"", "Points"),
                    self.points.to_string(),
                ),
                (latin1(&self.name), String::new()),
            ],
            // where to put them, as the folder inside the game's data folder (not a whole path)
            Screen::Custom if self.maps.is_empty() => vec![
                (s.ours("no levels"), String::new()),
                (
                    self.folder
                        .as_ref()
                        .and_then(|f| f.file_name())
                        .map(|n| format!("levels/{}/", n.to_string_lossy()))
                        .unwrap_or_default(),
                    String::new(),
                ),
            ],
            _ => Vec::new(),
        }
    }

    /// The prototype's menu over the frame (when the copy lacks the game's font): a shaded screen, the title, the
    /// lines and the page's fields.
    pub fn overlay(&self, width_px: u32, height_px: u32) -> Vec<Rect> {
        let (w, h) = (width_px as f32, height_px as f32);
        let mut out = vec![Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            color: [0.0, 0.0, 0.0, 0.6],
        }];
        let s = (h * 0.9 / 200.0).floor().max(1.0);
        let line = 10.0 * s;
        let centre = |out: &mut Vec<Rect>, t: &str, y: f32, scale: f32, color: [f32; 4]| {
            text(out, t, (w - width(t, scale)) / 2.0, y, scale, color);
        };
        let screen = self.screen();
        let lines = self.lines(screen);
        let fields = self.fields(screen);
        let shown = window(lines.len(), self.chosen());
        let total = 6.0 + fields.len() as f32 + 1.2 * shown.len() as f32;
        let mut y = ((h - total * line) / 2.0).max(0.0);
        let title = self.title(screen).map_or_else(|| "DRAHTSEILAKT".to_owned(), |t| t.trim().to_uppercase());
        centre(&mut out, &title, y, s * 1.5, GOLD);
        y += 2.5 * line;
        for (l, v) in &fields {
            let t = format!("{}  {}", l.trim(), v)
                .trim()
                .to_uppercase();
            centre(&mut out, &t, y, s, WHITE);
            y += line;
        }
        y += line;
        for i in shown {
            let Some(&l) = lines.get(i) else { continue };
            let mut t = self.label(l).trim().to_uppercase();
            if let (Line::Level(k), true) = (l, self.features.has(Feature::BestTimes))
                && let Some(b) = self.entries.get(k).and_then(|e| e.best)
            {
                t = format!("{t}   {}", clock(b));
            }
            let color = match (i == self.chosen(), self.selectable(screen, i)) {
                (true, _) => GOLD,
                (false, true) => WHITE,
                (false, false) => GREY,
            };
            if i == self.chosen() {
                let pad = s * 3.0;
                let lw = width(&t, s);
                out.push(Rect {
                    x: (w - lw) / 2.0 - pad,
                    y: y - s * 1.5,
                    w: lw + 2.0 * pad,
                    h: line,
                    color: SELECT,
                });
            }
            centre(&mut out, &t, y, s, color);
            y += 1.2 * line;
        }
        out
    }

    /// The loading screen in the prototype's font.
    pub fn loading_overlay(width_px: u32, height_px: u32, what: &str) -> Vec<Rect> {
        let (w, h) = (width_px as f32, height_px as f32);
        let s = (h * 0.9 / 200.0).floor().max(1.0);
        let mut out = vec![Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            color: [0.0, 0.0, 0.0, 0.6],
        }];
        let t = format!("LOADING {}", what.to_uppercase());
        text(
            &mut out,
            &t,
            (w - width(&t, s * 1.5)) / 2.0,
            h / 2.0,
            s * 1.5,
            WHITE,
        );
        out
    }
}

/// The lines of a list shown at once: all of a short one, else a window around the chosen line.
fn window(n: usize, chosen: usize) -> std::ops::Range<usize> {
    if n <= LIST_LINES + 1 {
        return 0..n;
    }
    let start = chosen.saturating_sub(LIST_LINES / 2).min(n - LIST_LINES);
    start..start + LIST_LINES
}

/// The original's menu screens (Menu.nmo's 2D frames, measured in the C game): a dark band down the middle, the
/// buttons over Button01_select (chosen), Button01_deselect or Button01_special (not playable), their labels in the
/// game's fonts (GameFont_01 for the main buttons, GameFont_03 and the grey GameFont_03a for the small ones).
impl Menu {
    pub fn interface(&self, font: Option<&crate::ui::Face>, screen: crate::ui::Screen) -> Vec<crate::ui::Quad> {
        use crate::ui::{Anchor, Quad, homogeneous};
        let mut out = Vec::new();
        // the own interface's picture behind the menus (the original's has its 3D scene, drawn by the game)
        if font.is_some_and(crate::ui::Face::own) && !self.over_level() {
            out.push(crate::ui::backdrop(screen));
        }
        let quad = |r: [f32; 4], uv: [f32; 4], texture: &'static str| Quad {
            rect: screen.rect(homogeneous(r), Anchor::Center),
            uv,
            top: WHITE,
            bottom: WHITE,
            texture,
        };
        // M_BlackScreen: a strip of Button01_special's black at 0.61
        out.push(Quad {
            top: [1.0, 1.0, 1.0, 0.61],
            bottom: [1.0, 1.0, 1.0, 0.61],
            ..quad([0.3, 0.0, 0.7, 1.0], [0.0, 0.99, 1.0, 1.0], SPECIAL)
        });
        // a label wider than its frame is drawn smaller, down to SHRINK of its size, and what still does not fit
        // loses its middle to ".."
        let label = |out: &mut Vec<Quad>, t: &str, r: [f32; 4], style: &crate::ui::Style| {
            let Some(f) = font else { return };
            let frame = homogeneous(r);
            let avail = frame[2] - frame[0] - style.margins[0] - style.margins[2];
            let wide = |q: &[Quad]| {
                q.iter().map(|q| q.rect[2]).fold(f32::MIN, f32::max)
                    - q.iter().map(|q| q.rect[0]).fold(f32::MAX, f32::min)
            };
            let tall = |q: &[Quad]| {
                q.iter().map(|q| q.rect[3]).fold(f32::MIN, f32::max)
                    - q.iter().map(|q| q.rect[1]).fold(f32::MAX, f32::min)
            };
            let scaled = |k: f32| {
                let mut st = *style;
                st.scale = [st.scale[0] * k, st.scale[1] * k];
                st
            };
            let mut quads = f.layout(t, frame, style);
            let w = wide(&quads);
            if style.flags & crate::ui::TEXT_WORDWRAP == 0 && w > avail && avail > 0.0 {
                let k = avail / w;
                if k >= FIT_SHRINK {
                    // a little smaller is enough
                    quads = f.layout(t, frame, &scaled(k));
                } else if let Some((a, b)) = two_lines(t) {
                    // two lines, each in its half of the frame, as large as both fit
                    let (wa, wb) = (wide(&f.layout(&a, frame, style)), wide(&f.layout(&b, frame, style)));
                    let half = (frame[3] - frame[1]) / 2.0;
                    let k2 = (avail / wa.max(wb)).min(half / tall(&quads).max(1e-6)).min(1.0);
                    let mid = (frame[1] + frame[3]) / 2.0;
                    let st = scaled(k2.max(WRAP_SHRINK));
                    // each line still too wide at the smallest size loses its middle to ".."
                    let fit = |mut text: String, r: [f32; 4]| {
                        let mut q = f.layout(&text, r, &st);
                        while wide(&q) > avail && text.chars().count() > 4 {
                            text = shorten(&text);
                            q = f.layout(&text, r, &st);
                        }
                        q
                    };
                    quads = fit(a, [frame[0], frame[1], frame[2], mid]);
                    quads.extend(fit(b, [frame[0], mid, frame[2], frame[3]]));
                } else {
                    let st = scaled(k.max(SHRINK));
                    let mut text = t.to_owned();
                    quads = f.layout(&text, frame, &st);
                    while wide(&quads) > avail && text.chars().count() > 4 {
                        text = shorten(&text);
                        quads = f.layout(&text, frame, &st);
                    }
                }
            }
            out.extend(crate::ui::place(quads, screen, Anchor::Center));
        };
        let button = |out: &mut Vec<Quad>, r: [f32; 4], uv: [f32; 4], state: Button| {
            let tex = match state {
                Button::Over => SELECT_TEX,
                Button::Up => DESELECT_TEX,
                Button::Inactive => SPECIAL,
            };
            out.push(quad(r, uv, tex));
        };
        let sc = self.screen();
        let lines = self.lines(sc);
        let state = |i: usize| {
            if i == self.chosen() {
                Button::Over
            } else if self.selectable(sc, i) {
                Button::Up
            } else {
                Button::Inactive
            }
        };
        // the big buttons from 0.15 (or under a title from 0.30), 0.15 apart; Back at the bottom
        let big = |out: &mut Vec<Quad>, top: f32| {
            let n = lines.iter().filter(|&&l| l != Line::Back).count().max(1);
            let step = if top + 0.15 * (n as f32 - 1.0) <= 0.76 {
                0.15
            } else {
                (0.74 - top) / (n as f32 - 1.0).max(1.0)
            };
            let mut k = 0;
            for (i, &l) in lines.iter().enumerate() {
                if l == Line::Back {
                    button(out, BACK_RECT, BACK_UV, state(i));
                    label(out, &self.label(l), BACK_RECT, &FONT_01);
                    continue;
                }
                let y = top + k as f32 * step;
                let r = [0.35, y, 0.65, y + 0.0938];
                button(out, r, MAIN_UV, state(i));
                label(out, &self.label(l), r, &FONT_01);
                k += 1;
            }
        };
        let title = |out: &mut Vec<Quad>| {
            if let Some(t) = self.title(sc) {
                label(out, &t, [0.35, 0.15, 0.65, 0.25], &FONT_01);
            }
        };
        let back_only = |out: &mut Vec<Quad>| {
            if let Some(i) = lines
                .iter()
                .position(|&l| l == Line::Back || l == Line::Next)
            {
                button(out, BACK_RECT, BACK_UV, state(i));
                label(out, &self.label(lines[i]), BACK_RECT, &FONT_01);
            }
        };
        // a field: its label at the top, its value under it (a yes/no pair, or text between arrows)
        let field = |out: &mut Vec<Quad>, y: f32, name: &str, value: &str, chosen: bool| {
            let r = [0.35, y, 0.65, y + 0.1];
            button(
                out,
                r,
                FIELD_UV,
                if chosen { Button::Over } else { Button::Up },
            );
            label(out, name, [0.35, y + 0.005, 0.65, y + 0.045], &FONT_03);
            label(out, value, [0.40, y + 0.045, 0.60, y + 0.085], &FONT_03);
        };
        match sc {
            Screen::Main | Screen::Pause | Screen::Dead | Screen::End => big(&mut out, 0.15),
            Screen::Options | Screen::Kinds => {
                title(&mut out);
                big(&mut out, 0.30);
            }
            Screen::Language => {
                title(&mut out);
                // the languages in their own names, the one spoken in gold
                for (i, &l) in lines.iter().enumerate() {
                    let Line::Lang(k) = l else {
                        button(&mut out, BACK_RECT, BACK_UV, state(i));
                        label(&mut out, &self.label(l), BACK_RECT, &FONT_01);
                        continue;
                    };
                    let y = 0.28 + k as f32 * 0.065;
                    let r = [0.4031, y, 0.5969, y + 0.052];
                    button(&mut out, r, LEVEL_UV, state(i));
                    let spoken = LANGUAGES.get(k).is_some_and(|c| c.0 == self.strings.language());
                    let style = if spoken { FONT_03_GOLD } else { FONT_03 };
                    label(&mut out, &self.label(l), [0.42, y, 0.5969, y + 0.052], &style);
                }
            }
            Screen::Start => {
                // 0.06 apart from 0.09 (twelve levels); a thirteenth closes them up
                let n = self.entries.len().max(1);
                let step = if n <= 12 { 0.06 } else { 0.72 / n as f32 };
                for (i, &l) in lines.iter().enumerate() {
                    let Line::Level(k) = l else {
                        button(&mut out, BACK_RECT, BACK_UV, state(i));
                        label(&mut out, &self.label(l), BACK_RECT, &FONT_01);
                        continue;
                    };
                    let y = 0.09 + k as f32 * step;
                    let r = [0.4031, y, 0.5969, y + step - 0.01];
                    button(&mut out, r, LEVEL_UV, state(i));
                    let style = if self.selectable(sc, i) {
                        &FONT_03
                    } else {
                        &FONT_03A
                    };
                    label(&mut out, &self.label(l), r, style);
                    if let Some(t) = self.entries.get(k).and_then(|e| e.best) {
                        let mut st = FONT_03A;
                        st.align = crate::ui::ALIGN_LEFT;
                        label(
                            &mut out,
                            &clock(t),
                            [0.6, y, 0.7, y + step - 0.01],
                            &st,
                        );
                    }
                }
            }
            Screen::Custom => {
                title(&mut out);
                let shown = window(lines.len().saturating_sub(1), self.chosen());
                for (row, i) in shown.enumerate() {
                    let y = 0.27 + row as f32 * 0.045;
                    let r = [0.36, y, 0.64, y + 0.04];
                    button(&mut out, r, LEVEL_UV, state(i));
                    // a long name smaller or on two lines, to stay on its button (the label does it)
                    let mut name = self.label(lines[i]);
                    if name.chars().count() > 60 {
                        name = name.chars().take(58).collect::<String>() + "..";
                    }
                    label(&mut out, &name, [0.42, y, 0.615, y + 0.04], &FONT_03);
                }
                for (row, (l, _)) in self.fields(sc).iter().enumerate() {
                    let y = 0.30 + row as f32 * 0.05;
                    label(&mut out, l, [0.31, y, 0.69, y + 0.05], &FONT_03);
                }
                back_only(&mut out);
            }
            Screen::Highscore => {
                if let Some(t) = self.title(sc) {
                    label(&mut out, &t, [0.40, 0.15, 0.60, 0.25], &FONT_03);
                }
                button(
                    &mut out,
                    [0.36, 0.174, 0.39625, 0.2257],
                    ARROW_LEFT_UV,
                    Button::Up,
                );
                button(
                    &mut out,
                    [0.6038, 0.174, 0.64, 0.2257],
                    ARROW_RIGHT_UV,
                    Button::Up,
                );
                for (k, (name, points)) in self.fields(sc).iter().enumerate() {
                    let y = 0.30 + k as f32 * 0.05;
                    button(
                        &mut out,
                        [0.36, y, 0.64, y + 0.041],
                        PLACE_UV,
                        Button::Inactive,
                    );
                    let u = 0.0627 * k as f32;
                    let ny = y + 0.007;
                    button(
                        &mut out,
                        [0.364, ny, 0.384, ny + 0.0267],
                        [u, 0.0, u + 0.0588, 0.0588],
                        Button::Inactive,
                    );
                    let mut left = FONT_03;
                    left.align = crate::ui::ALIGN_LEFT;
                    let mut right = FONT_03;
                    right.align = crate::ui::ALIGN_RIGHT;
                    label(&mut out, name, [0.39, y, 0.55, y + 0.041], &left);
                    label(&mut out, points, [0.52, y, 0.635, y + 0.041], &right);
                }
                back_only(&mut out);
            }
            Screen::Graphics | Screen::Controls | Screen::Sound | Screen::Extended | Screen::Debug => {
                title(&mut out);
                let fields = self.fields(sc);
                if sc == Screen::Controls {
                    // the six keys' rows, the extended mode's defaults row under them, the invert field
                    for (k, ((name, key), line)) in fields.iter().zip(&lines).enumerate() {
                        let chosen = self.chosen() == k;
                        let st = if chosen { Button::Over } else { Button::Up };
                        if *line == Line::Invert {
                            field(&mut out, if self.features.mode { 0.715 } else { 0.70 }, name, key, chosen);
                            continue;
                        }
                        let y = 0.30 + k as f32 * 0.06;
                        if *line == Line::Defaults {
                            button(&mut out, [0.35, y, 0.65, y + 0.0396], KEY_UV, st);
                            label(&mut out, name, [0.375, y, 0.505, y + 0.0396], &FONT_03);
                            continue;
                        }
                        button(&mut out, [0.35, y, 0.65, y + 0.0396], KEY_UV, st);
                        label(&mut out, name, [0.375, y, 0.505, y + 0.0396], &FONT_03);
                        label(&mut out, key, [0.505, y, 0.65, y + 0.0396], &FONT_03);
                    }
                } else if (sc == Screen::Extended && self.features.mode) || sc == Screen::Debug {
                    // Extended: the mode's field first; then the switches (and the Debug mode line) as the Controls
                    // page's rows, then lines of help or the folder
                    let first = usize::from(sc == Screen::Extended);
                    if let (1, Some((name, value))) = (first, fields.first()) {
                        field(&mut out, 0.27, name, value, self.chosen() == 0);
                    }
                    let rows = |l: &&Line| {
                        matches!(l, Line::Switch(_) | Line::Interface | Line::SoundTheme | Line::DebugMenu)
                    };
                    let n = lines.iter().filter(rows).count();
                    let top = if first == 1 { 0.40 } else { 0.30 };
                    for (k, (name, value)) in fields.iter().skip(first).take(n).enumerate() {
                        let y = top + k as f32 * 0.05;
                        let st = if self.chosen() == k + first {
                            Button::Over
                        } else {
                            Button::Up
                        };
                        button(&mut out, [0.35, y, 0.65, y + 0.0396], KEY_UV, st);
                        label(&mut out, name, [0.375, y, 0.55, y + 0.0396], &FONT_03);
                        label(&mut out, value, [0.55, y, 0.65, y + 0.0396], &FONT_03);
                    }
                    for (k, (l, _)) in fields.iter().skip(first + n).enumerate() {
                        let y = top + n as f32 * 0.05 + 0.03 + k as f32 * 0.045;
                        label(&mut out, l, [0.31, y, 0.69, y + 0.045], &FONT_03);
                    }
                } else {
                    // the original's fields 0.15 apart; the extended mode's fifth closes them up
                    let (top, step) = if fields.len() > 4 { (0.25, 0.12) } else { (0.295, 0.15) };
                    for (k, (name, value)) in fields.iter().enumerate() {
                        let y = top + k as f32 * step;
                        field(&mut out, y, name, value, self.chosen() == k);
                    }
                }
                back_only(&mut out);
            }
            Screen::Credits => {
                for (k, (l, _)) in self.fields(sc).iter().enumerate() {
                    let y = 0.30 + k as f32 * 0.05;
                    label(&mut out, l, [0.31, y, 0.69, y + 0.05], &FONT_03);
                }
                back_only(&mut out);
            }
            Screen::Ask(_) => {
                if let Some(t) = self.title(sc) {
                    let mut st = FONT_01;
                    st.flags |= crate::ui::TEXT_WORDWRAP;
                    label(&mut out, &t, [0.35, 0.30, 0.65, 0.50], &st);
                }
                for (i, r) in [YES_RECT, NO_RECT].into_iter().enumerate() {
                    button(&mut out, r, YES_NO_UV, state(i));
                    label(&mut out, &self.label(lines[i]), r, &FONT_03);
                }
            }
            Screen::Entry => {
                if let Some(t) = self.title(sc) {
                    label(&mut out, &t, [0.35, 0.30, 0.65, 0.40], &FONT_01);
                }
                let fields = self.fields(sc);
                if let Some((l, p)) = fields.first() {
                    label(&mut out, &format!("{l}: {p}"), [0.38, 0.43133, 0.62, 0.50133], &FONT_01);
                }
                out.push(Quad {
                    top: [0.0, 0.0, 0.0, 0.43],
                    bottom: [0.0, 0.0, 0.0, 0.43],
                    ..quad(
                        [0.38, 0.52733, 0.62, 0.59733],
                        [0.0, 0.99, 1.0, 1.0],
                        SPECIAL,
                    )
                });
                let name = latin1(&self.name) + "_";
                label(&mut out, &name, [0.38, 0.52733, 0.62, 0.59733], &FONT_01);
                button(
                    &mut out,
                    [0.4031, 0.80, 0.5906, 0.8938],
                    BACK_UV,
                    Button::Over,
                );
                label(
                    &mut out,
                    &self.label(Line::Ok),
                    [0.4031, 0.80, 0.5906, 0.8938],
                    &FONT_01,
                );
            }
        }
        out
    }

    /// The loading screen in the interface's face: the own interface's picture and band behind it.
    pub fn loading_interface(
        face: &crate::ui::Face,
        screen: crate::ui::Screen,
        strings: &Strings,
        what: &str,
    ) -> Vec<crate::ui::Quad> {
        use crate::ui::{Anchor, Quad, homogeneous};
        let mut out = Vec::new();
        if face.own() {
            out.push(crate::ui::backdrop(screen));
            out.push(Quad {
                rect: screen.rect(homogeneous([0.0, 0.43, 1.0, 0.57]), Anchor::Center),
                uv: [0.0, 0.99, 1.0, 1.0],
                top: [1.0, 1.0, 1.0, 0.61],
                bottom: [1.0, 1.0, 1.0, 0.61],
                texture: SPECIAL,
            });
        }
        let text = strings.ours("loading").replace("{name}", what);
        out.extend(crate::ui::place(
            face.layout(&text, homogeneous([0.2, 0.45, 0.8, 0.55]), &FONT_01),
            screen,
            Anchor::Center,
        ));
        out
    }
}

/// The smallest a label is drawn to fit, against its style's size.
const SHRINK: f32 = 0.6;
/// A label is drawn a little smaller to fit down to this; below it, it goes on two lines (down to WRAP_SHRINK).
const FIT_SHRINK: f32 = 0.75;
const WRAP_SHRINK: f32 = 0.45;

/// A label in two lines, broken at the space nearest its middle, or for a text without spaces in a script that
/// needs none (Chinese) at its middle character; None for one word.
fn two_lines(t: &str) -> Option<(String, String)> {
    let chars: Vec<char> = t.chars().collect();
    let mid = chars.len() / 2;
    let at = chars
        .iter()
        .enumerate()
        .filter(|&(_, &c)| c == ' ')
        .map(|(i, _)| i)
        .min_by_key(|&i| i.abs_diff(mid));
    let (a, b): (String, String) = match at {
        Some(at) => (chars.iter().take(at).collect(), chars.iter().skip(at + 1).collect()),
        None if chars.iter().any(|&c| u32::from(c) >= 0x2E80) && chars.len() > 1 => {
            (chars.iter().take(mid).collect(), chars.iter().skip(mid).collect())
        }
        None => return None,
    };
    let (a, b) = (a.trim().to_owned(), b.trim().to_owned());
    (!a.is_empty() && !b.is_empty()).then_some((a, b))
}

/// `t` a letter shorter, its middle shown as "..".
fn shorten(t: &str) -> String {
    let c: Vec<char> = t.chars().collect();
    let s = |r: &[char]| r.iter().collect::<String>();
    let Some(at) = c.windows(2).position(|w| w == ['.', '.']) else {
        // the first time three letters of the middle make room for ".."
        let mid = c.len() / 2;
        let head = c.get(..mid.saturating_sub(1)).unwrap_or_default();
        let tail = c.get(mid + 2..).unwrap_or_default();
        return s(head) + ".." + &s(tail);
    };
    let head = c.get(..at).unwrap_or_default();
    let tail = c.get(at + 2..).unwrap_or_default();
    if head.len() > tail.len() {
        s(head.get(..head.len() - 1).unwrap_or_default()) + ".." + &s(tail)
    } else {
        s(head) + ".." + &s(tail.get(1..).unwrap_or_default())
    }
}

#[derive(Clone, Copy)]
enum Button {
    Over,
    Up,
    Inactive,
}

const SPECIAL: &str = "Button01_special";
const SELECT_TEX: &str = "Button01_select";
const DESELECT_TEX: &str = "Button01_deselect";
/// The texture parts of the original's buttons and fields (Button01_*), as its 2D frames use them.
const MAIN_UV: [f32; 4] = [0.0, 0.5137, 1.0, 0.7451];
const LEVEL_UV: [f32; 4] = [0.0, 0.2470, 0.6470, 0.3686];
const BACK_UV: [f32; 4] = [0.2392, 0.7529, 0.8666, 0.9843];
const FIELD_UV: [f32; 4] = [0.0, 0.0, 1.0, 0.2471];
const KEY_UV: [f32; 4] = [0.0, 0.4078, 1.0, 0.51];
const YES_NO_UV: [f32; 4] = [0.0, 0.8235, 0.226, 0.9098];
const PLACE_UV: [f32; 4] = [0.0, 0.0627, 1.0, 0.1686];
const ARROW_LEFT_UV: [f32; 4] = [0.6392, 0.2471, 0.7882, 0.4039];
const ARROW_RIGHT_UV: [f32; 4] = [0.7921, 0.2471, 0.9412, 0.4039];
const BACK_RECT: [f32; 4] = [0.4031, 0.85, 0.5906, 0.9438];
const YES_RECT: [f32; 4] = [0.395, 0.52667, 0.485, 0.57207];
const NO_RECT: [f32; 4] = [0.52, 0.52667, 0.61, 0.57207];

const fn font_style(scale: [f32; 2], color: [f32; 4]) -> crate::ui::Style {
    crate::ui::Style {
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
const FONT_01: crate::ui::Style = font_style([0.45, 0.55], WHITE);
const FONT_03: crate::ui::Style = font_style([0.35, 0.4], WHITE);
const FONT_03A: crate::ui::Style = font_style([0.35, 0.4], [0.514, 0.514, 0.514, 1.0]);
const FONT_03_GOLD: crate::ui::Style = font_style([0.35, 0.4], GOLD);

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GREY: [f32; 4] = [0.6, 0.6, 0.6, 1.0];
const GOLD: [f32; 4] = [1.0, 0.85, 0.3, 1.0];
const SELECT: [f32; 4] = [1.0, 1.0, 1.0, 0.15];

/// The time of a run of `ticks` (60 a second) as minutes, seconds and hundredths.
fn clock(ticks: u64) -> String {
    format!("{:02}:{:05.2}", ticks / 3600, (ticks % 3600) as f32 / 60.0)
}

/// The folder of a level's runs.
fn runs_dir(number: u32) -> PathBuf {
    crate::app::replay_dir().join(format!("Level_{number:02}"))
}

/// The best finished run of an original level, from its saved best replay.
fn best_ticks(number: u32) -> Option<u64> {
    let text = std::fs::read_to_string(runs_dir(number).join("best.replay")).ok()?;
    crate::replay::Replay::parse(&text).ok().map(|r| r.end)
}

/// Level `number` was finished once: a finished run of it is kept (`Level_NN-<ticks>.replay`, or the best).
fn finished(number: u32) -> bool {
    let prefix = format!("Level_{number:02}-");
    std::fs::read_dir(runs_dir(number)).is_ok_and(|d| {
        d.flatten().any(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n == "best.replay" || (n.starts_with(&prefix) && n.ends_with(".replay"))
        })
    })
}

/// The switch lines of the extended mode's additions: the debug ones, or the others.
fn switches(debug: bool) -> impl Iterator<Item = Line> {
    FEATURES.iter().enumerate().filter(move |(_, f)| f.debug() == debug).map(|(i, _)| Line::Switch(i))
}

/// The interface's provider after the chosen one, `by` steps on (the arrows go round the three).
fn next_provider(by: i32) -> gamedata::resources::Provider {
    use gamedata::resources::{Provider, chosen};
    let all = Provider::ALL;
    let at = all.iter().position(|&p| p == chosen()).unwrap_or(1) as i32;
    let n = all.len() as i32;
    all.get((at + by).rem_euclid(n) as usize).copied().unwrap_or(Provider::Original)
}

/// The sound theme after the chosen one, `by` steps on (the arrows go round the themes found and "original").
fn next_theme(by: i32) -> String {
    use gamedata::sound_themes::{SYSTEM, choices, chosen};
    let all = choices();
    let at = all.iter().position(|k| *k == chosen()).unwrap_or(0) as i32;
    let n = all.len().max(1) as i32;
    all.get((at + by).rem_euclid(n) as usize).cloned().unwrap_or_else(|| SYSTEM.to_owned())
}

/// The Sounds line's value: the chosen theme's key (its folder's name).
fn theme_value() -> String {
    gamedata::sound_themes::chosen()
}

impl Menu {
    /// The Interface line's value: the provider chosen, marked when it waits for the next start.
    fn interface_value(&self) -> String {
        use gamedata::resources::{Provider, chosen, in_use};
        let s = &self.strings;
        let mut v = match chosen() {
            Provider::Own => s.ours("interface own"),
            Provider::Original => s.ours("interface original"),
            Provider::Remastered => s.ours("interface remaster"),
        };
        if chosen() != in_use() {
            v = format!("{v} {}", s.ours("restart needed"));
        }
        v
    }
}

/// The kinds of levels the extended mode offers, in the menu's order (levels.md).
const LEVEL_KINDS: [Kind; 4] = [Kind::Original, Kind::Remastered, Kind::Community, Kind::Open];

/// The levels of a custom levels folder by title: the level folders in it and the `.nmo` files dropped loose into
/// it; folders that hold no level or several are not shown, and said so on the console.
fn maps_in(dir: &Path) -> Vec<Package> {
    let (levels, skipped) = gamedata::package::scan(dir);
    if !skipped.is_empty() {
        let shown = gamedata::package::printable(&dir.to_string_lossy());
        eprintln!("{shown}: {} folders are not levels (a level folder holds one level):", skipped.len());
        for s in skipped.iter().take(10) {
            eprintln!("  {s}");
        }
    }
    levels
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn custom_levels_are_a_flat_list_of_level_folders() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-menu-{}", std::process::id()));
        let season = root.join("Pro Racing").join("Season 1");
        std::fs::create_dir_all(&season).unwrap();
        std::fs::create_dir_all(root.join("Empty")).unwrap();
        std::fs::create_dir_all(root.join("Zeta")).unwrap();
        std::fs::write(season.join("Track.nmo"), b"").unwrap();
        std::fs::write(root.join("Zeta/Level_07.NMO"), b"").unwrap();
        std::fs::write(root.join("Zeta/level.txt"), b"title = Alpha Road\n").unwrap();
        std::fs::write(root.join("Alone.NMO"), b"").unwrap();
        std::fs::write(root.join("pack.tar.gz"), b"").unwrap();
        let mut m = menu(&[true], true);
        m.folder = Some(root.clone());
        m.maps = maps_in(&root);
        m.stack.push((Screen::Custom, 0));
        // by title; no folders to open, nested levels and junk not shown
        let titles: Vec<String> = (0..m.maps.len()).map(|i| m.label(Line::Map(i))).collect();
        assert_eq!(titles, ["Alone", "Alpha Road"]);
        assert_eq!(m.enter(), Action::Map(root.join("Alone.NMO")));
        m.set_chosen(1);
        assert_eq!(m.enter(), Action::Map(root.join("Zeta/Level_07.NMO")));
        // Esc leaves the screen at once
        m.back();
        assert_ne!(m.screen(), Screen::Custom);
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn menu(open: &[bool], extended: bool) -> Menu {
        let mut m = Menu {
            stack: Vec::new(),
            entries: open
                .iter()
                .enumerate()
                .map(|(i, &open)| Entry {
                    number: i as u32 + 1,
                    available: true,
                    open,
                    best: None,
                })
                .collect(),
            maps: Vec::new(),
            folder: None,
            features: Features::new(extended),
            volumes: [1.0; 2],
            strings: Strings::default(),
            scores: Table::default(),
            level: 1,
            shown: 1,
            resolution: (640, 480),
            name: Vec::new(),
            points: 0,
            after_entry: false,
            custom: false,
            graphics: Graphics::default(),
            resolutions: vec![(800, 600), (1280, 720), (1920, 1080)],
            bindings: Bindings::default(),
            waiting: None,
        };
        m.push(Screen::Main);
        m
    }

    #[test]
    fn the_classic_graphics_page_has_the_originals_lines_and_the_full_screen() {
        let m = menu(&[true], false);
        assert_eq!(
            m.lines(Screen::Graphics),
            [Line::Resolution, Line::Synch, Line::Clouds, Line::Fullscreen, Line::Back]
        );
        let e = menu(&[true], true);
        assert!(e.lines(Screen::Graphics).contains(&Line::Fov));
        assert!(!m.lines(Screen::Controls).contains(&Line::Defaults));
        assert!(e.lines(Screen::Controls).contains(&Line::Defaults));
    }

    fn graphics(a: Action) -> Option<Graphics> {
        if let Action::Graphics(g) = a { Some(g) } else { None }
    }

    fn bindings(a: Action) -> Option<Bindings> {
        if let Action::Bindings(b) = a { Some(b) } else { None }
    }

    #[test]
    fn graphics_lines_change_and_say_so() {
        let mut m = menu(&[true], true);
        m.push(Screen::Graphics);
        // Back is chosen first, as the original's; up to Fov, then the others
        m.step(-1);
        assert_eq!(m.adjust(1), Action::Graphics(Graphics { fov: 60, ..Graphics::default() }));
        m.step(-1);
        let g = graphics(m.adjust(1)).unwrap();
        assert_eq!(g.fullscreen, Fullscreen::Borderless);
        m.step(-1);
        let g = graphics(m.enter()).unwrap();
        assert!(!g.clouds);
        m.step(-1);
        let g = graphics(m.enter()).unwrap();
        assert!(g.vsync);
        m.step(-1);
        // 640 x 480 is not offered: right goes to the next size up, left to the one below it
        let g = graphics(m.adjust(1)).unwrap();
        assert_eq!(g.resolution, Some((800, 600)));
        assert_eq!(m.resolution, (800, 600));
        m.adjust(-1);
        assert_eq!(m.resolution, (1920, 1080), "round from the smallest");
    }

    #[test]
    fn sizes_step_round_and_from_between() {
        let s = [(800, 600), (1280, 720), (1920, 1080)];
        assert_eq!(step_size(&s, (1280, 720), 1), Some((1920, 1080)));
        assert_eq!(step_size(&s, (1920, 1080), 1), Some((800, 600)));
        assert_eq!(step_size(&s, (1600, 900), -1), Some((1280, 720)));
        assert_eq!(step_size(&s, (1600, 900), 1), Some((1920, 1080)));
        assert_eq!(step_size(&s, (4000, 3000), 1), Some((800, 600)));
        assert_eq!(step_size(&[], (800, 600), 1), None);
    }

    #[test]
    fn a_key_is_rebound_by_pressing_it_and_esc_gives_up() {
        use winit::keyboard::KeyCode;
        let mut m = menu(&[true], false);
        m.push(Screen::Controls);
        m.set_chosen(0);
        assert_eq!(m.enter(), Action::None);
        assert!(m.waiting());
        assert_eq!(m.fields(Screen::Controls)[0].1, m.strings.ours("press a key"));
        // Esc: nothing changes, the page stays
        assert_eq!(m.capture(KeyCode::Escape), Action::None);
        assert!(!m.waiting());
        assert_eq!(m.screen(), Screen::Controls);
        // F11 is the game's: the wait goes on; W is taken
        m.enter();
        assert_eq!(m.capture(KeyCode::F11), Action::None);
        assert!(m.waiting());
        let b = bindings(m.capture(KeyCode::KeyW)).unwrap();
        assert_eq!(b.keys[0], KeyCode::KeyW);
        // backward onto W: forward gets backward's old key
        m.set_chosen(1);
        m.enter();
        let b = bindings(m.capture(KeyCode::KeyW)).unwrap();
        assert_eq!((b.keys[0], b.keys[1]), (KeyCode::ArrowDown, KeyCode::KeyW));
        // Esc while waiting through back() too
        m.enter();
        assert_eq!(m.back(), Action::None);
        assert_eq!(m.screen(), Screen::Controls);
        // the invert line, and the extended mode's defaults
        m.set_chosen(6);
        let b = bindings(m.adjust(1)).unwrap();
        assert!(b.invert);
        let mut e = menu(&[true], true);
        e.bindings = b;
        e.push(Screen::Controls);
        e.set_chosen(6);
        assert_eq!(e.enter(), Action::Bindings(Bindings::default()));
    }

    #[test]
    fn the_classic_main_menu_is_the_originals() {
        let m = menu(&[true], false);
        let labels: Vec<_> = m
            .lines(Screen::Main)
            .into_iter()
            .map(|l| m.label(l))
            .collect();
        assert_eq!(
            labels,
            ["Start", "Highscore", "Options", "Credits", "Exit"]
        );
        let mut m = menu(&[true], true);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Kinds, "levels by kind in the extended mode only");
        let kinds: Vec<String> = m.lines(Screen::Kinds).into_iter().map(|l| m.label(l)).collect();
        assert_eq!(kinds[..4], ["Original", "Remastered", "Community maps", "Open format"]);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Start, "the original levels are the classic list");
    }

    #[test]
    fn options_open_the_language_page_and_a_language_is_chosen() {
        let mut m = menu(&[true], false);
        m.push(Screen::Options);
        assert!(m.lines(Screen::Options).contains(&Line::Language));
        m.set_chosen(3);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Language);
        // the language spoken is chosen first (English), its name in its own words
        assert_eq!(m.lines(Screen::Language)[m.chosen()], Line::Lang(0));
        assert_eq!(m.label(Line::Lang(1)), "Русский");
        m.step(1);
        assert_eq!(m.enter(), Action::Language("ru".to_owned()));
        assert_eq!(m.screen(), Screen::Options, "back on Options");
    }

    #[test]
    fn locked_levels_are_skipped() {
        let mut m = menu(&[true, false, true], false);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Start);
        m.step(1);
        assert_eq!(m.enter(), Action::Play(3));
        m.step(1);
        m.step(1);
        assert_eq!(
            m.enter(),
            Action::Play(1),
            "Back, then round to level 1, past locked level 2"
        );
    }

    #[test]
    fn exit_asks_first_and_no_is_chosen() {
        let mut m = menu(&[true], false);
        m.step(-1);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Ask(Ask::Quit));
        assert_eq!(m.enter(), Action::None, "No");
        assert_eq!(m.screen(), Screen::Main);
        m.enter();
        m.adjust(-1);
        assert_eq!(m.enter(), Action::Quit);
    }

    #[test]
    fn the_pause_menu_goes_back_to_the_run_and_asks_before_restarting() {
        let mut m = menu(&[true], false);
        m.stack.clear();
        m.push(Screen::Pause);
        assert_eq!(m.enter(), Action::Continue, "Back is chosen first");
        m.step(1);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Ask(Ask::Restart));
        m.step(-1);
        assert_eq!(m.enter(), Action::Restart);
        assert_eq!(m.back(), Action::None);
        assert_eq!(m.back(), Action::Continue);
    }

    #[test]
    fn a_new_highscore_takes_a_name() {
        let mut m = menu(&[true, true], false);
        m.stack.clear();
        m.push(Screen::End);
        m.ask_name(b"ve".to_vec(), 3000);
        m.typed("resk\u{1F600}");
        m.erase();
        m.typed("k");
        assert_eq!(m.enter(), Action::Score(b"veresk".to_vec()));
        let mut t = Table::default();
        t.insert(1, b"veresk", 3000);
        m.show_scores(t);
        assert_eq!(m.screen(), Screen::Highscore);
        assert_eq!(m.label(m.lines(Screen::Highscore)[0]), "Next");
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::End);
        assert_eq!(m.enter(), Action::Play(2), "Next level is chosen");
    }

    #[test]
    fn browsing_highscores_leaves_the_next_level_alone() {
        let mut m = menu(&[true, true, true, true], false);
        m.stack.clear();
        m.push(Screen::End);
        m.step(1);
        m.step(1);
        m.enter();
        assert_eq!(m.screen(), Screen::Highscore);
        m.adjust(1);
        m.adjust(1);
        assert_eq!(m.title(Screen::Highscore).unwrap(), "Highscore Level 3");
        m.back();
        m.step(-1);
        m.step(-1);
        assert_eq!(m.enter(), Action::Play(2));
    }

    #[test]
    fn the_chosen_line_stays_when_the_mode_switches() {
        let mut m = menu(&[true], false);
        m.step(1);
        m.step(1);
        assert_eq!(m.lines(Screen::Main)[m.chosen()], Line::Options);
        m.apply_mode(Features::new(true), Vec::new(), Vec::new());
        assert_eq!(m.lines(Screen::Main).len(), 5);
        assert_eq!(m.lines(Screen::Main)[m.chosen()], Line::Options);
    }

    #[test]
    fn a_highscore_needs_a_name() {
        let mut m = menu(&[true], false);
        m.ask_name(b"  ".to_vec(), 100);
        assert_eq!(m.enter(), Action::None);
        m.typed("a");
        assert_eq!(m.enter(), Action::Score(b"  a".to_vec()));
    }

    #[test]
    fn a_long_label_breaks_at_the_space_nearest_its_middle() {
        let pair = |a: &str, b: &str| Some((a.to_owned(), b.to_owned()));
        assert_eq!(two_lines("Levels by kind?"), pair("Levels", "by kind?"));
        assert_eq!(two_lines("Interface: remaster"), pair("Interface:", "remaster"));
        assert_eq!(two_lines("Word"), None);
        assert_eq!(two_lines(" Word "), None);
        assert_eq!(two_lines(""), None);
        assert_eq!(two_lines("确定退出游戏"), pair("确定退", "出游戏"), "Chinese at its middle character");
    }

    #[test]
    fn shortening_keeps_both_ends() {
        let mut t = "abcdefgh".to_owned();
        t = shorten(&t);
        assert_eq!(t, "abc..gh");
        t = shorten(&t);
        assert_eq!(t, "ab..gh");
        t = shorten(&t);
        assert_eq!(t, "ab..h");
        assert_eq!(shorten("уровень"), "ур..нь", "letters, not bytes");
    }

    #[test]
    fn maps_are_the_level_files_and_folders_of_the_folder() {
        let dir = std::env::temp_dir().join(format!("drahtseilakt-maps-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["b.NMO", "a.nmo", "c.txt"] {
            std::fs::write(dir.join(f), b"").unwrap();
        }
        std::fs::create_dir_all(dir.join("open level")).unwrap();
        std::fs::write(dir.join("open level/level.gltf"), b"").unwrap();
        std::fs::create_dir_all(dir.join("empty")).unwrap();
        let names: Vec<_> = maps_in(&dir)
            .iter()
            .map(|p| p.level.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(names, ["a.nmo", "b.NMO", "open level"]);
    }
}
