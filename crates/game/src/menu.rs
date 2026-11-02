// The menus as the original's Menu.nmo has them (docs/drahtseilakt/menu.md): the main menu, the level list, the
// highscores, the options and their pages, the credits; over a level the pause, dead and end menus, the yes/no
// questions and the new highscore's name entry. The words are the copy's (lang.rs), the layout the original's 2D
// frames as the C game shows them. The extended mode adds the custom levels and a page of its own. Chosen with the
// arrows and Enter; drawn with the copy's textures and font, or with the prototype's font when the copy lacks it.

use crate::assets::Data;
use crate::features::{FEATURES, Feature, Features};
use crate::highscore::{NAME_MAX, Table};
use crate::hud::{Rect, text, width};
use crate::lang::{Strings, latin1};
use std::path::{Path, PathBuf};

/// The original game's levels: 1 to 12, and 13 of the add-on when the copy has it.
pub const LEVELS: u32 = 13;
/// Lines of a long list (custom levels) shown at once.
const LIST_LINES: usize = 12;

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
    Start,
    Custom,
    Highscore,
    Options,
    Graphics,
    Controls,
    Sound,
    Extended,
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
    Custom,
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
    Map(usize),
    Music,
    Effects,
    Mode,
    /// One of the extended mode's additions, by its place in FEATURES.
    Switch(usize),
    Yes,
    No,
    Ok,
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
    /// The new highscore's name was entered: the game saves it and shows the list.
    Score(Vec<u8>),
}

pub struct Menu {
    /// The screens open, the first the menu's base, each with its chosen line.
    stack: Vec<(Screen, usize)>,
    pub entries: Vec<Entry>,
    /// The custom levels shown: the levels and the folders holding more of them, in the folder open (the levels
    /// folders' tops when none is).
    pub maps: Vec<PathBuf>,
    /// The folders opened in the custom levels, innermost last.
    folder: Vec<PathBuf>,
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
            maps: if features.has(Feature::CustomLevels) {
                maps()
            } else {
                Vec::new()
            },
            folder: Vec::new(),
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
            Screen::Main if self.features.has(Feature::CustomLevels) => {
                vec![
                    Line::Start,
                    Line::Custom,
                    Line::Highscore,
                    Line::Options,
                    Line::Credits,
                    Line::Exit,
                ]
            }
            Screen::Main => vec![
                Line::Start,
                Line::Highscore,
                Line::Options,
                Line::Credits,
                Line::Exit,
            ],
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
                Line::Extended,
                Line::Back,
            ],
            Screen::Sound if self.features.has(Feature::SoundVolume) => {
                vec![Line::Music, Line::Effects, Line::Back]
            }
            Screen::Sound => vec![Line::Music, Line::Back],
            Screen::Extended if self.features.mode => [Line::Mode]
                .into_iter()
                .chain((0..FEATURES.len()).map(Line::Switch))
                .chain([Line::Back])
                .collect(),
            Screen::Extended => vec![Line::Mode, Line::Back],
            Screen::Highscore if self.after_entry => vec![Line::Next],
            Screen::Highscore | Screen::Graphics | Screen::Controls | Screen::Credits => {
                vec![Line::Back]
            }
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
            (Screen::Extended, Some(Line::Mode)) => Action::ToggleDebug,
            (Screen::Extended, Some(Line::Switch(i))) => {
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
            Line::Start => open(self, Screen::Start),
            Line::Custom => open(self, Screen::Custom),
            Line::Highscore => open(self, Screen::Highscore),
            Line::Options => open(self, Screen::Options),
            Line::Credits => open(self, Screen::Credits),
            Line::Graphics => open(self, Screen::Graphics),
            Line::Controls => open(self, Screen::Controls),
            Line::Sound => open(self, Screen::Sound),
            Line::Extended => open(self, Screen::Extended),
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
            Line::Map(i) => match self.maps.get(i).cloned() {
                Some(p) if is_folder(&p) => {
                    self.maps = maps_in(&p);
                    self.folder.push(p);
                    self.set_chosen(0);
                    Action::None
                }
                p => p.map_or(Action::None, Action::Map),
            },
            Line::Mode => Action::ToggleDebug,
            Line::Switch(i) => FEATURES.get(i).map_or(Action::None, |&f| Action::Switch(f)),
            Line::Music | Line::Effects => Action::None,
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

    /// Esc: back a screen; the pause menu back to the run.
    pub fn back(&mut self) -> Action {
        match self.screen() {
            Screen::Pause => Action::Continue,
            Screen::Main | Screen::Dead | Screen::End | Screen::Entry => Action::None,
            Screen::Highscore if self.after_entry => self.enter(),
            // out of a folder of custom levels, onto the folder left
            Screen::Custom if !self.folder.is_empty() => {
                let left = self.folder.pop();
                self.maps = self.folder.last().map_or_else(maps, |f| maps_in(f));
                let at = self.maps.iter().position(|p| Some(p) == left.as_ref()).unwrap_or(0);
                self.set_chosen(at);
                Action::None
            }
            _ => {
                self.pop();
                Action::None
            }
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
    fn apply_mode(&mut self, features: Features, entries: Vec<Entry>, maps: Vec<PathBuf>) {
        let chosen: Vec<Option<Line>> = self
            .stack
            .iter()
            .map(|&(sc, i)| self.lines(sc).get(i).copied())
            .collect();
        self.entries = entries;
        self.maps = maps;
        self.folder.clear();
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

    /// The label of a line in the copy's words.
    fn label(&self, line: Line) -> Vec<u8> {
        let s = &self.strings;
        match line {
            Line::Start => s.get("Menu_Main Button Start", "Start"),
            Line::Custom => b"Custom levels".to_vec(),
            Line::Highscore => s.get("Menu_Main Button Highscore", "Highscore"),
            Line::Options => s.get("Menu_Main Button Options", "Options"),
            Line::Credits => s.get("Menu_Main Button Credits", "Credits"),
            Line::Exit => s.get("Menu_Main Button ExitApplication", "Exit"),
            Line::Back => s.get("Menu_Start Button Back", "Back"),
            Line::Restart => s.get("Menu_Pause Button Restart", "Restart Level"),
            Line::ExitLevel => s.get("Menu_Pause Button Exit", "Exit Level"),
            Line::Next if self.screen() == Screen::Highscore => {
                s.get("Menu_Highscore \"Next\"", "Next")
            }
            Line::Next => s.get("Menu_End Next Level", "Next level"),
            Line::Home => s.get("Menu_Dead Home", "Home"),
            Line::Graphics => s.get("Menu_Options Button Graphics", "Graphics"),
            Line::Controls => s.get("Menu_Options Button Controls", "Controls"),
            Line::Sound => s.get("Menu_Options Button Sound", "Sound"),
            Line::Extended => b"Extended".to_vec(),
            Line::Level(i) => {
                let n = self.entries.get(i).map_or(0, |e| e.number);
                s.get(
                    &format!("Menu_Start Button Level{n}"),
                    &format!("Level {n}"),
                )
            }
            // the file name in the font's Latin-1, what it lacks as '?'; an open-format level (a folder) says which
            // folder it is in, as an original level converted there is named like the original
            Line::Map(i) => self
                .maps
                .get(i)
                .map(|p| {
                    let name = p
                        .file_stem()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let folder = p
                        .parent()
                        .and_then(|d| d.file_name())
                        .map(|d| d.to_string_lossy().into_owned());
                    let name = match folder {
                        _ if is_folder(p) => format!("{name}/"),
                        Some(f) if p.is_dir() => format!("{name} ({f})"),
                        _ => name,
                    };
                    name.chars()
                        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
                        .collect()
                })
                .unwrap_or_default(),
            Line::Music => s.get("Menu_Opt_Sound Volume", "Music Volume"),
            Line::Effects => b"  Sound Volume".to_vec(),
            Line::Mode => b"  Extended mode?".to_vec(),
            Line::Switch(i) => FEATURES
                .get(i)
                .map_or("", |f| f.label())
                .as_bytes()
                .to_vec(),
            Line::Yes => s.get("Menu_Opt_Graphics Synch Yes", "Yes"),
            Line::No => s.get("Menu_Opt_Graphics Synch No", "No"),
            Line::Ok => s.get("Menu_YesNo YES", "OK"),
        }
    }

    /// A screen's title, if it has one.
    fn title(&self, screen: Screen) -> Option<Vec<u8>> {
        let s = &self.strings;
        Some(match screen {
            Screen::Options => s.get("Menu_Options Title", "Options"),
            Screen::Graphics => s.get("Menu_Options Button Graphics", "Graphics"),
            Screen::Controls => s.get("Menu_Options Button Controls", "Controls"),
            Screen::Sound => s.get("Menu_Options Button Sound", "Sound"),
            Screen::Extended => b"Extended".to_vec(),
            Screen::Custom => b"Custom levels".to_vec(),
            Screen::Highscore => {
                let mut t = s.get("Highscore Title (+ SPACE!)", "Highscore Level ");
                t.extend_from_slice(self.shown.to_string().as_bytes());
                t
            }
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
    fn fields(&self, screen: Screen) -> Vec<(Vec<u8>, Vec<u8>)> {
        let s = &self.strings;
        let yes_no = |b: bool| {
            if b {
                s.get("Menu_Opt_Graphics Synch Yes", "Yes")
            } else {
                s.get("Menu_Opt_Graphics Synch No", "No")
            }
        };
        match screen {
            Screen::Graphics => vec![
                (
                    s.get("Menu_Opt_Graphics Resolution", "Screen Resolution"),
                    format!("{} * {}", self.resolution.0, self.resolution.1).into_bytes(),
                ),
                (
                    s.get("Menu_Opt_Graphics Synch", "Synch to Screen?"),
                    yes_no(true),
                ),
                (s.get("Menu_Opt_Graphics Clouds", "Clouds?"), yes_no(true)),
            ],
            Screen::Controls => {
                let names = [
                    ("Menu_Opt_Keys Forward", "Forward"),
                    ("Menu_Opt_Keys Backward", "Backward"),
                    ("Menu_Opt_Keys Left", "Left"),
                    ("Menu_Opt_Keys Right", "Right"),
                    ("Menu_Opt_Keys Rotation", "Rotation"),
                    ("Menu_Opt_Keys Overview", "Overview"),
                ];
                let mut f: Vec<_> = names
                    .iter()
                    .zip(s.controls)
                    .map(|(&(k, d), row)| (s.get(k, d), s.key(row)))
                    .collect();
                f.push((
                    s.get("Menu_Opt_Keys Invert Cam Rot?", "Invert Rotation?"),
                    yes_no(s.invert),
                ));
                f
            }
            Screen::Sound => {
                let pct = |v: f32| format!("{}", (v * 10.0).round() as u32).into_bytes();
                let mut f = vec![(self.label(Line::Music), pct(self.volumes[1]))];
                if self.features.has(Feature::SoundVolume) {
                    f.push((self.label(Line::Effects), pct(self.volumes[0])));
                }
                f
            }
            Screen::Extended => {
                // the mode, then each addition's switch, then where things are
                let mut f = vec![(self.label(Line::Mode), yes_no(self.features.mode))];
                if self.features.mode {
                    for (i, &x) in FEATURES.iter().enumerate() {
                        f.push((
                            self.label(Line::Switch(i)),
                            yes_no(self.features.switched_on(x)),
                        ));
                    }
                    for l in [
                        "Tools: R restart, F2 free camera, [ ] speed,",
                        "F5 / F9 save and back. Helped runs never count.",
                        &short_path(&crate::levels::data_dir().join("levels")),
                    ] {
                        f.push((l.as_bytes().to_vec(), Vec::new()));
                    }
                }
                f
            }
            Screen::Credits => [
                "Drahtseilakt by Veresk",
                "",
                "Ballance by Cyparade / Atari 2004",
                "Started as a fork of OpenBallance by emdzej",
                "Physics: IVP, ported from OpenBallance",
                "",
                "Plays your own copy of the game",
            ]
            .iter()
            .map(|l| (l.as_bytes().to_vec(), Vec::new()))
            .collect(),
            Screen::Highscore => self
                .scores
                .list(self.shown)
                .iter()
                .map(|(n, p)| (n.clone(), p.to_string().into_bytes()))
                .collect(),
            Screen::Entry => vec![
                (
                    s.get("Menu_HighscoreEntry \"Points\"", "Points"),
                    self.points.to_string().into_bytes(),
                ),
                (self.name.clone(), Vec::new()),
            ],
            Screen::Custom if self.maps.is_empty() => vec![
                (b"No levels yet. Put .nmo maps into".to_vec(), Vec::new()),
                (short_path(&maps_dir()).into_bytes(), Vec::new()),
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
        let title = self.title(screen).map_or_else(
            || "DRAHTSEILAKT".to_owned(),
            |t| latin1(&t).trim().to_uppercase(),
        );
        centre(&mut out, &title, y, s * 1.5, GOLD);
        y += 2.5 * line;
        for (l, v) in &fields {
            let t = format!("{}  {}", latin1(l).trim(), latin1(v))
                .trim()
                .to_uppercase();
            centre(&mut out, &t, y, s, WHITE);
            y += line;
        }
        y += line;
        for i in shown {
            let Some(&l) = lines.get(i) else { continue };
            let mut t = latin1(&self.label(l)).trim().to_uppercase();
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
    pub fn interface(
        &self,
        font: Option<&crate::ui::Font>,
        screen: crate::ui::Screen,
    ) -> Vec<crate::ui::Quad> {
        use crate::ui::{Anchor, Quad, homogeneous};
        let mut out = Vec::new();
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
        let label = |out: &mut Vec<Quad>, t: &[u8], r: [f32; 4], style: &crate::ui::Style| {
            let Some(f) = font else { return };
            let frame = homogeneous(r);
            let avail = frame[2] - frame[0] - style.margins[0] - style.margins[2];
            let wide = |q: &[Quad]| {
                q.iter().map(|q| q.rect[2]).fold(f32::MIN, f32::max)
                    - q.iter().map(|q| q.rect[0]).fold(f32::MAX, f32::min)
            };
            let mut quads = f.layout(t, frame, style);
            let w = wide(&quads);
            if style.flags & crate::ui::TEXT_WORDWRAP == 0 && w > avail && avail > 0.0 {
                let k = (avail / w).max(SHRINK);
                let mut st = *style;
                st.scale = [st.scale[0] * k, st.scale[1] * k];
                let mut text = t.to_vec();
                quads = f.layout(&text, frame, &st);
                while wide(&quads) > avail && text.len() > 4 {
                    text = shorten(&text);
                    quads = f.layout(&text, frame, &st);
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
                (0.78 - top) / (n as f32 - 1.0).max(1.0)
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
        let field = |out: &mut Vec<Quad>, y: f32, name: &[u8], value: &[u8], chosen: bool| {
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
            Screen::Options => {
                title(&mut out);
                big(&mut out, 0.30);
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
                            clock(t).as_bytes(),
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
                    // a long name smaller, to stay on its button
                    let mut name = self.label(lines[i]);
                    if name.len() > 30 {
                        name.truncate(28);
                        name.extend_from_slice(b"..");
                    }
                    label(&mut out, &name, [0.39, y, 0.635, y + 0.04], &FONT_03);
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
            Screen::Graphics | Screen::Controls | Screen::Sound | Screen::Extended => {
                title(&mut out);
                let fields = self.fields(sc);
                if sc == Screen::Controls {
                    for (k, (name, key)) in fields.iter().enumerate() {
                        if k == 6 {
                            field(&mut out, 0.70, name, key, false);
                            continue;
                        }
                        let y = 0.30 + k as f32 * 0.06;
                        button(&mut out, [0.35, y, 0.65, y + 0.0396], KEY_UV, Button::Up);
                        label(&mut out, name, [0.375, y, 0.505, y + 0.0396], &FONT_03);
                        label(&mut out, key, [0.505, y, 0.65, y + 0.0396], &FONT_03);
                    }
                } else if sc == Screen::Extended && self.features.mode {
                    if let Some((name, value)) = fields.first() {
                        field(&mut out, 0.27, name, value, self.chosen() == 0);
                    }
                    // the additions' switches as the Controls page's rows, then two lines of help and the folder
                    let n = FEATURES.len();
                    for (k, (name, value)) in fields.iter().skip(1).take(n).enumerate() {
                        let y = 0.40 + k as f32 * 0.05;
                        let st = if self.chosen() == k + 1 {
                            Button::Over
                        } else {
                            Button::Up
                        };
                        button(&mut out, [0.35, y, 0.65, y + 0.0396], KEY_UV, st);
                        label(&mut out, name, [0.375, y, 0.58, y + 0.0396], &FONT_03);
                        label(&mut out, value, [0.58, y, 0.65, y + 0.0396], &FONT_03);
                    }
                    for (k, (l, _)) in fields.iter().skip(1 + n).enumerate() {
                        let y = 0.66 + k as f32 * 0.045;
                        label(&mut out, l, [0.31, y, 0.69, y + 0.045], &FONT_03);
                    }
                } else {
                    for (k, (name, value)) in fields.iter().enumerate() {
                        let y = 0.295 + k as f32 * 0.15;
                        let chosen = sc != Screen::Graphics && self.chosen() == k;
                        field(&mut out, y, name, value, chosen);
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
                    let mut t = l.clone();
                    t.extend_from_slice(b": ");
                    t.extend_from_slice(p);
                    label(&mut out, &t, [0.38, 0.43133, 0.62, 0.50133], &FONT_01);
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
                let mut name = self.name.clone();
                name.push(b'_');
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

    /// The loading screen in the game's font.
    pub fn loading_interface(
        font: &crate::ui::Font,
        screen: crate::ui::Screen,
        what: &str,
    ) -> Vec<crate::ui::Quad> {
        use crate::ui::{Anchor, homogeneous};
        let r = [0.3, 0.45, 0.7, 0.55];
        crate::ui::place(
            font.layout(
                format!("Loading {what}").as_bytes(),
                homogeneous(r),
                &FONT_01,
            ),
            screen,
            Anchor::Center,
        )
    }
}

/// The smallest a label is drawn to fit, against its style's size.
const SHRINK: f32 = 0.6;

/// `t` a letter shorter, its middle shown as "..".
fn shorten(t: &[u8]) -> Vec<u8> {
    let Some(at) = t.windows(2).position(|w| w == b"..") else {
        // the first time three letters of the middle make room for ".."
        let mid = t.len() / 2;
        let head = t.get(..mid.saturating_sub(1)).unwrap_or_default();
        let tail = t.get(mid + 2..).unwrap_or_default();
        return [head, b"..", tail].concat();
    };
    let head = t.get(..at).unwrap_or_default();
    let tail = t.get(at + 2..).unwrap_or_default();
    if head.len() > tail.len() {
        [head.get(..head.len() - 1).unwrap_or_default(), b"..", tail].concat()
    } else {
        [head, b"..", tail.get(1..).unwrap_or_default()].concat()
    }
}

/// A path as the menu shows it: the home folder as ~.
fn short_path(p: &Path) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match home.as_deref().and_then(|h| p.strip_prefix(h).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => p.display().to_string(),
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

/// The folder players put community maps into.
pub fn maps_dir() -> PathBuf {
    crate::levels::dir(crate::levels::Kind::Community)
}

/// The custom levels: the community maps, then the open-format and remastered levels (levels.md).
fn maps() -> Vec<PathBuf> {
    use crate::levels::{Kind, dir};
    [Kind::Community, Kind::Open, Kind::Remastered]
        .into_iter()
        .flat_map(|k| maps_in(&dir(k)))
        .take(4096)
        .collect()
}

/// How deep folders of custom levels may go (a map collection sorted by kind, season and map).
const FOLDER_DEPTH: u32 = 4;

/// A folder of levels to open, not a level: a folder that is not an open-format level.
fn is_folder(p: &Path) -> bool {
    p.is_dir() && !p.join("level.gltf").is_file()
}

/// A hidden file or folder (its name begins with a dot): never shown (the toolbox keeps its parts in one).
fn hidden(p: &Path) -> bool {
    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'))
}

fn is_level(p: &Path) -> bool {
    let nmo = p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("nmo"));
    nmo || (p.is_dir() && p.join("level.gltf").is_file())
}

/// The folder holds a level, itself or in its folders (`depth` deep at most).
fn holds_levels(dir: &Path, depth: u32) -> bool {
    let Ok(d) = std::fs::read_dir(dir) else {
        return false;
    };
    d.flatten().take(4096).map(|e| e.path()).any(|p| {
        is_level(&p) || (depth > 0 && p.is_dir() && !p.is_symlink() && holds_levels(&p, depth - 1))
    })
}

/// A folder's levels by name: `.nmo` files and folders holding an open-format level (level.gltf), then by name the
/// folders holding more levels.
fn maps_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(d) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let all: Vec<PathBuf> = d.flatten().map(|e| e.path()).filter(|p| !hidden(p)).take(4096).collect();
    let mut levels: Vec<PathBuf> = all.iter().filter(|p| is_level(p)).cloned().collect();
    let mut folders: Vec<PathBuf> = all
        .into_iter()
        .filter(|p| is_folder(p) && !p.is_symlink() && holds_levels(p, FOLDER_DEPTH))
        .collect();
    levels.sort();
    folders.sort();
    levels.extend(folders);
    levels
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn custom_levels_open_and_leave_their_folders() {
        let root = std::env::temp_dir().join(format!("drahtseilakt-menu-{}", std::process::id()));
        let season = root.join("Pro Racing").join("Season 1");
        std::fs::create_dir_all(&season).unwrap();
        std::fs::create_dir_all(root.join("Empty")).unwrap();
        std::fs::write(season.join("Track.nmo"), b"").unwrap();
        std::fs::write(root.join("Alone.NMO"), b"").unwrap();
        let mut m = menu(&[true], true);
        // inside a folder already, as the tops are the player's own level folders
        m.folder.push(root.clone());
        m.maps = maps_in(&root);
        m.stack.push((Screen::Custom, 0));
        // the levels first, then the folders holding levels; an empty folder is not shown
        assert_eq!(m.maps, vec![root.join("Alone.NMO"), root.join("Pro Racing")]);
        assert_eq!(m.label(Line::Map(1)), b"Pro Racing/".to_vec());
        m.set_chosen(1);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.maps, vec![root.join("Pro Racing").join("Season 1")]);
        m.enter();
        assert_eq!(m.maps, vec![season.join("Track.nmo")]);
        assert_eq!(m.enter(), Action::Map(season.join("Track.nmo")));
        // Esc goes back up a folder at a time, onto the folder left, and then leaves the screen
        m.back();
        m.back();
        assert_eq!((m.chosen(), m.folder.len()), (1, 1));
        assert_eq!(m.maps[1], root.join("Pro Racing"));
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
            folder: Vec::new(),
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
        };
        m.push(Screen::Main);
        m
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
            [&b"Start"[..], b"Highscore", b"Options", b"Credits", b"Exit"]
        );
        let m = menu(&[true], true);
        assert!(
            m.lines(Screen::Main).contains(&Line::Custom),
            "custom levels in the extended mode only"
        );
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
        assert_eq!(m.label(m.lines(Screen::Highscore)[0]), b"Next");
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
        assert_eq!(m.title(Screen::Highscore).unwrap(), b"Highscore Level 3");
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
        assert_eq!(m.lines(Screen::Main).len(), 6);
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
    fn shortening_keeps_both_ends() {
        let mut t = b"abcdefgh".to_vec();
        t = shorten(&t);
        assert_eq!(t, b"abc..gh");
        t = shorten(&t);
        assert_eq!(t, b"ab..gh");
        t = shorten(&t);
        assert_eq!(t, b"ab..h");
    }

    #[test]
    fn maps_are_the_nmo_files_of_the_folder() {
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
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(names, ["a.nmo", "b.NMO", "open level"]);
    }
}
