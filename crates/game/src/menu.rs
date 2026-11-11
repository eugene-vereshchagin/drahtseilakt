// The menus as the original's Menu.nmo has them (docs/drahtseilakt/menu.md): the main menu, the level list, the
// highscores, the options and their pages, the credits; over a level the pause, dead and end menus, the yes/no
// questions and the new highscore's name entry. The words are the copy's (lang.rs), the layout the original's 2D
// frames as the C game shows them. The extended mode adds the custom levels and a page of its own; Options adds the
// Language page. Chosen with the arrows and Enter, or with the mouse (the cursor, a click, the wheel, on the very
// rectangles drawn); drawn with the interface's textures (our own, or the copy's) and its face (ui.rs), or with the
// prototype's font when there is neither.

use crate::assets::Data;
use crate::features::{FEATURES, Feature, Features};
use crate::highscore::{NAME_MAX, Table};
use crate::hud::{Rect, text, width};
use crate::lang::{Strings, languages, latin1};
use crate::levels::Kind;
use crate::options::{Bindings, Fullscreen, Graphics};
use gamedata::package::Package;
use std::path::{Path, PathBuf};

/// The original game's levels: 1 to 12, and 13 of the add-on when the copy has it.
pub const LEVELS: u32 = 13;
/// Lines of a long list (custom levels) shown at once.
const LIST_LINES: usize = 13;
/// Rows of the Mods screen and a mod's page shown at once (the help under them).
const MODS_LINES: usize = 9;
/// Rows of the community maps' list (the author and the thanks are under them).
const COMMUNITY_LINES: usize = 9;
/// The most lines of a map's check the note shows after its verdict.
const CHECK_LINES: usize = 3;

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
            let after =
                i64::try_from(sizes.iter().filter(|s| area(s) < area(&now)).count()).ok()?;
            if by > 0 { after - 1 } else { after }
        }
    };
    sizes
        .get(usize::try_from((at + i64::from(by)).rem_euclid(n)).ok()?)
        .copied()
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
    /// Start: which levels (by meaning, the same in both modes): Classic Ballance, Drahtseilakt, Ballance Challenges,
    /// Community maps.
    Kinds,
    /// Start, Community maps: the best (shipped) and the downloaded ones.
    Community,
    Start,
    Custom,
    Highscore,
    Options,
    Graphics,
    Controls,
    Sound,
    Extended,
    /// The mods (docs/drahtseilakt/mods.md): the practice tools and endless lives, then each mod found, on or off.
    Mods,
    /// One mod's page: on or off, its options, its description.
    ModPage,
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
    /// Bring in the player's copy of the original (shown while there is none).
    Import,
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
    /// Classic Ballance: the 13 original levels of the player's copy (the list of Start).
    Classic,
    /// Drahtseilakt: our own levels, shipped with the game.
    Ours,
    /// Ballance Challenges: the community remakes of the original levels (levels/modified).
    Challenges,
    /// Community maps: the submenu of the best and the downloaded ones.
    Community,
    /// The best community maps, shipped with the game and credited to their authors (assets/levels/community).
    Best,
    /// The player's downloaded levels: community maps and open-format levels (levels/downloaded and the folders it
    /// replaces).
    Downloaded,
    /// Graphics: the Look, one choice of how levels are drawn (look.rs); only the looks available here go round.
    Look,
    Map(usize),
    Music,
    Effects,
    Mode,
    /// The Mods screen; a mod on it (by its place among those found); its page's switch and options.
    ModsMenu,
    Mod(usize),
    ModOn,
    ModOpt(usize),
    /// Where the interface's files come from: own, the original, its remaster (resources.rs).
    Interface,
    /// The texture pack the levels are drawn with (packs.rs): the arrows go round the packs found.
    /// The model pack the parts and balls are drawn with (packs.rs).
    Models,
    /// The sound theme (sound_themes.rs): the arrows go round the themes found.
    SoundTheme,
    /// The Language page, and a language on it (by its place in lang::languages()).
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
    /// Open the window that brings in the player's copy of the original.
    Import,
    Restart,
    /// Leave the level for the main menu.
    Home,
    Quit,
    /// The extended mode on or off.
    ToggleDebug,
    /// One of the extended mode's additions on or off.
    Switch(Feature),
    /// The look chosen: the next level loads with it.
    Look(crate::look::Look),
    /// The volumes changed: sound and music, 0 to 1.
    Volumes([f32; 2]),
    /// The interface's provider chosen (it takes effect on the next start).
    Interface(gamedata::resources::Provider),
    /// The levels' textures chosen (the next level loads with them).
    /// A texture or model pack chosen (by its key).
    Pack(gamedata::packs::Kind, String),
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
    /// A mod switched on or off (by its place among those found).
    ModSwitch(usize),
    /// A mod's option stepped: the mod, the option, the step.
    ModOption(usize, usize, i32),
}

/// What a button under the cursor is: a line of the screen (by its place in the screen's lines), or one of the
/// highscores' arrows (left -1, right +1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Line(usize),
    Arrow(i32),
    /// A scrolling list's bar: a click scrolls the list there.
    Bar,
}

/// A button where it is drawn: its rectangle in window pixels (left, top, right, bottom).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub rect: [f32; 4],
    pub target: Target,
}

/// The mouse's cursor over the menu: where it is in the window (pixels), the window's size, and whether the menu is
/// drawn as the interface (a face to draw with) or as the prototype's lines.
#[derive(Clone, Copy, Debug)]
pub struct Pointer {
    pub interface: bool,
    pub screen: crate::ui::Screen,
    pub x: f32,
    pub y: f32,
}

/// A line whose value the arrows change (a volume, a size, a switch): a click changes it as they do.
fn valued(line: Line) -> bool {
    matches!(
        line,
        Line::Music
            | Line::Effects
            | Line::Resolution
            | Line::Synch
            | Line::Clouds
            | Line::Fullscreen
            | Line::Fov
            | Line::Invert
            | Line::Mode
            | Line::Look
            | Line::Interface
            | Line::Models
            | Line::SoundTheme
            | Line::Switch(_)
            | Line::Mod(_)
            | Line::ModOn
            | Line::ModOpt(_)
    )
}

pub struct Menu {
    /// Test hook: show the help of this value (an index into the line's values) instead of the current one.
    help_force: std::cell::Cell<Option<usize>>,
    /// The rows the open list with a note under it shows: as many as leave the note whole (set by `draw`).
    note_rows: std::cell::Cell<usize>,
    /// The screens open, the first the menu's base, each with its chosen line.
    stack: Vec<(Screen, usize)>,
    pub entries: Vec<Entry>,
    /// The custom levels shown: the levels of the kind's folder, by title (levels.md, "A level is a folder").
    pub maps: Vec<Package>,
    /// The kind's folder the custom levels come from.
    folder: Option<PathBuf>,
    /// Which list the custom levels shown are.
    list: List,
    /// Our own levels and the best community maps are shipped (the Start items are left out when not).
    has_ours: bool,
    has_best: bool,
    /// The player's copy of the original is there: without it the levels are our own and the menu offers the import.
    copy: bool,
    /// There are modified originals (levels/modified) to offer; they are listed only with a copy too.
    modified: bool,
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
    /// The mods found, which are on and their options (the game's, shown and changed here); the mod whose page is up.
    pub mods: crate::modrt::Mods,
    mod_page: usize,
    /// The first line a long list shows (the custom levels): it moves only as far as keeps the chosen line in view,
    /// so the line under the cursor stays where it is.
    top: usize,
    /// The Credits' first line shown, and how far they scroll (set when they are drawn).
    credits_top: usize,
    credits_max: std::cell::Cell<usize>,
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
            help_force: std::cell::Cell::new(None),
            note_rows: std::cell::Cell::new(LIST_LINES),
            stack: Vec::new(),
            entries,
            maps: Vec::new(),
            folder: None,
            list: List::None,
            has_ours: !crate::levels::shipped().is_empty(),
            has_best: !crate::levels::community().is_empty(),
            copy: data.has_copy(),
            modified: crate::modified::offered(data.has_copy()),
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
            mods: crate::modrt::Mods::default(),
            mod_page: 0,
            top: 0,
            credits_top: 0,
            credits_max: std::cell::Cell::new(0),
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

    /// The rows a list shows at once: the community maps leave room under them for the author and the notice.
    fn page_len(&self) -> usize {
        match self.screen() {
            Screen::Custom => self.note_rows.get().clamp(1, self.max_rows()),
            sc => page(sc),
        }
    }

    /// The most rows the open list shows: the community maps leave room for a longer note.
    fn max_rows(&self) -> usize {
        if self.note_below() {
            COMMUNITY_LINES
        } else {
            LIST_LINES
        }
    }

    /// Whether the open list has a note on a panel under its rows: the shipped community picks (author and thanks)
    /// and the player's own maps folder (what it is, where it is).
    fn note_below(&self) -> bool {
        self.list != List::None
    }

    /// A note's text with the Esc hint as its last line (the hint alone when there is no note).
    fn with_hint(&self, note: String) -> String {
        let hint = self.strings.ours("esc back");
        if note.trim().is_empty() {
            hint
        } else {
            format!("{note}\n{hint}")
        }
    }

    /// The note under an open list with one: the picks' (what they are, the chosen map's author, the authors' notice)
    /// or the player's folder's.
    fn list_note(&self, chosen: Option<Line>) -> String {
        if !self.note_below() {
            return String::new();
        }
        // Back: what it does
        if chosen == Some(Line::Back) {
            return self.help_of("help back");
        }
        let map = || self.help_of("help map");
        match self.list {
            List::Best => {
                let by = match chosen {
                    Some(Line::Map(m)) => self.credit(m),
                    _ => String::new(),
                };
                let notice = self.strings.ours("community notice");
                let about = self.strings.ours("note picks");
                if by.is_empty() {
                    format!("{about}\n{notice}")
                } else {
                    format!("{about}\n{by}\n{notice}")
                }
            }
            List::Downloaded => {
                let about = self.describe(Some(Line::Downloaded));
                if let Some(Line::Map(m)) = chosen {
                    // the map check from the import, when there is one: what the game does not support yet in it
                    let check = self.check_note(m);
                    // the folder's description gives way to the check: the rows matter more than the path
                    let head = if check.is_empty() {
                        format!("{about}\n{}", map())
                    } else {
                        map()
                    };
                    format!("{head}\n{}{check}", self.strings.ours("note beautiful"))
                } else {
                    about
                }
            }
            List::Ours => {
                let about = self.describe(Some(Line::Ours));
                if matches!(chosen, Some(Line::Map(_))) {
                    format!("{about}\n{}", map())
                } else {
                    about
                }
            }
            List::Challenges => {
                let about = self.describe(Some(Line::Challenges));
                if matches!(chosen, Some(Line::Map(_))) {
                    format!("{about}\n{}", map())
                } else {
                    about
                }
            }
            List::None => String::new(),
        }
    }

    /// The map check of map `i` as note lines (a newline first): its verdict and the lines that matter most; nothing
    /// when the map was never checked.
    fn check_note(&self, i: usize) -> String {
        let Some(r) = self.maps.get(i).and_then(|p| p.report.as_ref()) else {
            return String::new();
        };
        let s = crate::report::summarize(r);
        let text = |k: &str| self.strings.ours(k);
        format!(
            "\n{}",
            crate::report::words(&s, self.strings.language(), &text, CHECK_LINES)
        )
    }

    /// How many rows the list can show so that its note stays whole for every chosen map (the text shrinks to
    /// MIN_TEXT first; rows are given up only after that), so that the rows do not change as the choice moves.
    fn fit_rows(&self, f: &crate::ui::Face) -> usize {
        let texts: Vec<String> = if self.list == List::Best {
            (0..self.maps.len())
                .map(|m| self.list_note(Some(Line::Map(m))))
                .chain([self.list_note(None), self.list_note(Some(Line::Back))])
                .collect()
        } else if self.list == List::Downloaded {
            // the notes differ by the maps' checks: every map with one counts (a few hundred at most), so that
            // the rows do not change as the choice moves
            (0..self.maps.len().min(512))
                .filter(|&m| m == 0 || self.maps.get(m).is_some_and(|p| p.report.is_some()))
                .map(|m| self.list_note(Some(Line::Map(m))))
                .chain([self.list_note(None), self.list_note(Some(Line::Back))])
                .collect()
        } else {
            vec![
                self.list_note(Some(Line::Map(0))),
                self.list_note(None),
                self.list_note(Some(Line::Back)),
            ]
        };
        let texts: Vec<String> = texts.into_iter().map(|t| self.with_hint(t)).collect();
        let max_w = (NOTE_X[1] - NOTE_X[0]) * crate::ui::CONTEXT_W - 2.0 * NOTE_PAD[0];
        for rows in (1..=self.max_rows()).rev() {
            let top = LIST_TOP + rows as f32 * ROW_STEP + HINT_H + 0.008;
            let h = (NOTE_BOTTOM - top) * crate::ui::CONTEXT_H - 2.0 * NOTE_PAD[1];
            let room = |t: &String| {
                let (lines, k, ok) = fit_whole(f, t, max_w, h, &FONT_HELP);
                ok && lines.len() as f32 * row_pitch(&scaled_style(&FONT_HELP, k)) <= h + 1e-3
            };
            if texts.iter().all(room) {
                return rows;
            }
        }
        1
    }

    /// What a list or kind line is, in a sentence (the player's folder with its place), empty for the others.
    fn describe(&self, line: Option<Line>) -> String {
        let s = &self.strings;
        match line {
            Some(Line::Classic) => s.ours("note classic"),
            Some(Line::Ours) => s.ours("note ours"),
            Some(Line::Challenges) => s.ours("note challenges"),
            Some(Line::Community) => s.ours("note community"),
            Some(Line::Best) => s.ours("note picks"),
            // where the folder is, as the player finds it in the game's data folder
            Some(Line::Downloaded) => {
                let dir = shown_path(&crate::levels::dir(Kind::Downloaded).display().to_string());
                format!("{}\n{dir}", s.ours("note downloaded"))
            }
            _ => String::new(),
        }
    }

    /// The help of a line: the key of its text, and the words of its values (one text each; none for a line without
    /// values). Menu.txt holds `help <key>` or `help <key> <value>`.
    fn help_spec(&self, screen: Screen, line: Line) -> Option<(String, &'static [&'static str])> {
        const NONE: &[&str] = &[];
        const YN: &[&str] = &["yes", "no"];
        let k = |s: &str| format!("help {s}");
        Some(match line {
            Line::Start => (k("start"), NONE),
            Line::Import => (k("import"), NONE),
            Line::Highscore => (k("highscore"), NONE),
            Line::Options => (k("options"), NONE),
            Line::Credits => (k("credits"), NONE),
            Line::Exit => (k("exit"), NONE),
            Line::Back if screen == Screen::Pause => (k("back pause"), NONE),
            Line::Back => (k("back"), NONE),
            Line::Restart => (k("restart"), NONE),
            Line::ExitLevel => (k("exit level"), NONE),
            Line::Home => (k("home"), NONE),
            Line::Next => (k("next"), NONE),
            Line::Level(_) => (k("level"), NONE),
            Line::Map(_) => (k("map"), NONE),
            Line::Graphics => (k("graphics"), NONE),
            Line::Controls => (k("controls"), NONE),
            Line::Sound => (k("sound"), NONE),
            Line::Language => (k("language"), NONE),
            Line::Extended => (k("extended"), NONE),
            Line::Lang(_) => (k("lang"), NONE),
            Line::Look => (
                k("look"),
                &["original", "remaster", "drahtseilakt", "player"],
            ),
            Line::Resolution => (k("resolution"), NONE),
            Line::Synch => (k("synch"), YN),
            Line::Clouds => (k("clouds"), YN),
            Line::Fullscreen => (k("fullscreen"), &["off", "borderless", "exclusive"]),
            Line::Models => (k("models"), &["level", "own", "original", "player"]),
            Line::Fov => (k("fov"), &["original", "wider", "narrower"]),
            Line::Key(i) => (
                k(&format!("key {}", crate::options::ACTIONS.get(i)?.0)),
                NONE,
            ),
            Line::Defaults => (k("defaults"), NONE),
            Line::Invert => (k("invert"), YN),
            Line::Music => (k("music"), NONE),
            Line::Effects => (k("effects"), NONE),
            Line::SoundTheme => (k("theme"), &["neuro", "system", "original", "player"]),
            Line::Mode => (k("mode"), YN),
            Line::Switch(i) => (k(FEATURES.get(i)?.key()), YN),
            Line::Interface => (k("interface"), &["own", "original", "remaster"]),
            Line::ModsMenu => (k("mods"), NONE),
            Line::Yes | Line::No => {
                let Screen::Ask(a) = screen else { return None };
                let what = match a {
                    Ask::Quit => "quit",
                    Ask::Restart => "restart",
                    Ask::Exit => "exit",
                };
                let v: &'static [&'static str] = if line == Line::Yes { &["yes"] } else { &["no"] };
                (k(&format!("ask {what}")), v)
            }
            _ => return None,
        })
    }

    /// Which of the line's values is the current one (an index into its `help_spec` values).
    fn help_value(&self, line: Line) -> usize {
        use gamedata::packs::{Kind as PackKind, LEVEL, ORIGINAL, OWN};
        let yn = |on: bool| usize::from(!on);
        match line {
            Line::Look => ALL_LOOKS
                .iter()
                .position(|&l| l == crate::look::chosen())
                .unwrap_or(2),
            Line::Synch => yn(self.graphics.vsync),
            Line::Clouds => yn(self.graphics.clouds),
            Line::Fullscreen => match self.graphics.fullscreen {
                Fullscreen::Off => 0,
                Fullscreen::Borderless => 1,
                Fullscreen::Exclusive => 2,
            },
            Line::Models => match gamedata::packs::chosen(PackKind::Models).as_str() {
                LEVEL => 0,
                OWN => 1,
                ORIGINAL => 2,
                _ => 3,
            },
            Line::Fov => match self.graphics.fov.cmp(&crate::options::FOV_ORIGINAL) {
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => 2,
            },
            Line::Invert => yn(self.bindings.invert),
            Line::SoundTheme => match gamedata::sound_themes::chosen().as_str() {
                "neuro" => 0,
                "system" => 1,
                "original" => 2,
                _ => 3,
            },
            Line::Mode => yn(self.features.mode),
            Line::Switch(i) => yn(FEATURES
                .get(i)
                .is_some_and(|&f| self.features.switched_on(f))),
            Line::Interface => match gamedata::resources::chosen() {
                gamedata::resources::Provider::Own => 0,
                gamedata::resources::Provider::Original => 1,
                gamedata::resources::Provider::Remastered => 2,
            },
            _ => 0,
        }
    }

    /// A player's pack or theme: the description its pack.txt or theme.txt gives in the menus' language, if any
    /// (`description.<lang>`, else `description.en`, else `description`), a few lines long at most.
    fn description_of(&self, dir: &Path, file: &str) -> Option<String> {
        use std::io::Read;
        let mut text = String::new();
        std::fs::File::open(dir.join(file))
            .ok()?
            .take(4096)
            .read_to_string(&mut text)
            .ok()?;
        let find = |key: &str| {
            text.lines()
                .filter(|l| !l.starts_with('#'))
                .find_map(|l| l.strip_prefix(key).and_then(|v| v.strip_prefix(' ')))
                .map(|v| {
                    v.chars()
                        .filter(|c| !c.is_control())
                        .take(220)
                        .collect::<String>()
                })
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };
        find(&format!("description.{}", self.strings.language()))
            .or_else(|| find("description.en"))
            .or_else(|| find("description"))
    }

    /// The descriptions of the player's packs or themes a Textures, Models or Sound theme line can show (the
    /// chosen one's alone when `only_chosen`).
    fn player_descriptions(&self, line: Line, only_chosen: bool) -> Vec<String> {
        use gamedata::packs::{Kind as PackKind, chosen, packs};
        let pack = |kind: PackKind| {
            let c = chosen(kind);
            packs(kind)
                .into_iter()
                .filter(|p| !only_chosen || p.key == c)
                .filter_map(|p| self.description_of(p.dir.as_deref()?, "pack.txt"))
                .collect::<Vec<_>>()
        };
        match line {
            Line::Look => pack(PackKind::Textures),
            Line::Models => pack(PackKind::Models),
            Line::SoundTheme => {
                let c = gamedata::sound_themes::chosen();
                gamedata::sound_themes::themes()
                    .into_iter()
                    .filter(|t| !t.shipped && (!only_chosen || t.key == c))
                    .filter_map(|t| self.description_of(&t.dir, "theme.txt"))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// The text of a help key.
    fn help_of(&self, key: &str) -> String {
        self.strings.ours(key)
    }

    /// The help of `line` on `screen` for its current value: what it is, how it works, what it gives. The Start
    /// kinds and the lists' folders have their own sentences (`describe`).
    fn help_now(&self, screen: Screen, line: Line) -> String {
        let own = self.describe(Some(line));
        if !own.is_empty() {
            return own;
        }
        let Some((key, values)) = self.help_spec(screen, line) else {
            return String::new();
        };
        if values.is_empty() {
            return self.help_of(&key);
        }
        let now = self
            .help_force
            .get()
            .unwrap_or_else(|| self.help_value(line))
            .min(values.len() - 1);
        let mut text = self.help_of(&format!("{key} {}", values[now]));
        // beauty comes through the toolbox: said where the look is chosen
        if line == Line::Look {
            text = format!("{text}\n{}", self.help_of("help look beautiful"));
        }
        // a player's own pack or theme: its description when it gives one
        if values[now] == "player" && self.help_force.get().is_none() {
            if let Some(d) = self.player_descriptions(line, true).into_iter().next() {
                return format!("{text}\n{d}");
            }
        }
        text
    }

    /// How many values the line's help has (1 for a line without).
    #[cfg(test)]
    fn help_values(&self, screen: Screen, line: Line) -> usize {
        self.help_spec(screen, line)
            .map_or(1, |(_, v)| v.len().max(1))
    }

    /// Every help the screen's lines can show, one for each value of each line, and the descriptions of the
    /// player's packs: the room the panel must have (and what the tests walk through).
    fn helps_all(&self, screen: Screen) -> Vec<String> {
        let mut out = Vec::new();
        for line in self.lines(screen) {
            let own = self.describe(Some(line));
            if !own.is_empty() {
                out.push(own);
                continue;
            }
            let Some((key, values)) = self.help_spec(screen, line) else {
                continue;
            };
            if values.is_empty() {
                out.push(self.help_of(&key));
            }
            for v in values {
                let mut text = self.help_of(&format!("{key} {v}"));
                if line == Line::Look {
                    text = format!("{text}\n{}", self.help_of("help look beautiful"));
                }
                if *v == "player" {
                    for d in self.player_descriptions(line, false) {
                        out.push(format!("{text}\n{d}"));
                    }
                }
                out.push(text);
            }
        }
        out
    }

    /// The text of the notes panel of a screen with helps: the line under the cursor, Esc's hint last where the
    /// screen has a Back.
    fn help_panel(&self, screen: Screen) -> String {
        let lines = self.lines(screen);
        let text = lines
            .get(self.chosen())
            .map(|&l| self.help_now(screen, l))
            .unwrap_or_default();
        if lines.contains(&Line::Back) {
            self.with_hint(text)
        } else {
            text
        }
    }

    /// Where the panel of a screen with helps begins (a fraction of the frame's height): its bottom is NOTE_BOTTOM,
    /// its height the tallest of the screen's helps, so the rows above it do not move as the cursor does.
    fn help_top(&self, f: &crate::ui::Face, screen: Screen) -> f32 {
        let max_w = (NOTE_X[1] - NOTE_X[0]) * crate::ui::CONTEXT_W - 2.0 * NOTE_PAD[0];
        let has_back = self.lines(screen).contains(&Line::Back);
        let mut worst = 0.0f32;
        for t in self.helps_all(screen) {
            let t = if has_back { self.with_hint(t) } else { t };
            let (lines, k, _) = fit_whole(f, &t, max_w, 1.0e4, &FONT_HELP);
            let h =
                lines.len() as f32 * row_pitch(&scaled_style(&FONT_HELP, k)) + 2.0 * NOTE_PAD[1];
            worst = worst.max(h);
        }
        NOTE_BOTTOM - worst / crate::ui::CONTEXT_H - 0.012
    }

    fn chosen(&self) -> usize {
        self.stack.last().map_or(0, |s| s.1)
    }

    fn set_chosen(&mut self, i: usize) {
        if let Some(s) = self.stack.last_mut() {
            s.1 = i;
        }
        self.top = self.shown(self.list_len(self.screen())).start;
    }

    /// How many lines of `screen` scroll as a list: a list's without its Back, else all of its lines.
    fn list_len(&self, screen: Screen) -> usize {
        let n = self.lines(screen).len();
        if scrolls(screen) {
            n.saturating_sub(1)
        } else {
            n
        }
    }

    /// The lines of a list of `n` shown at once: all of a short one, else a page of it from the list's top, moved as
    /// little as keeps the chosen line in view.
    fn shown(&self, n: usize) -> std::ops::Range<usize> {
        let lines = self.page_len();
        if n <= lines {
            return 0..n;
        }
        let mut start = self.top.min(n - lines);
        let c = self.chosen();
        if c < start {
            start = c;
        } else if c < n && c >= start + lines {
            start = c + 1 - lines;
        }
        start..start + lines
    }

    /// A newly shown screen's list starts with its chosen line in the middle.
    fn centre_list(&mut self) {
        self.top = self.chosen().saturating_sub(self.page_len() / 2);
    }

    /// The list scrolled to `top` (clamped), its chosen line kept within the rows shown.
    fn scroll_to(&mut self, top: usize) {
        let n = self.list_len(self.screen());
        let lines = self.page_len();
        if n <= lines {
            return;
        }
        self.top = top.min(n - lines);
        let c = self.chosen();
        if c < n
            && let Some(s) = self.stack.last_mut()
        {
            s.1 = c.clamp(self.top, self.top + lines - 1);
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
            Screen::Language => languages()
                .iter()
                .position(|l| l.0 == self.strings.language())
                .and_then(|i| at(Line::Lang(i))),
            _ => None,
        };
        if screen == Screen::Highscore {
            self.shown = self.level;
        }
        self.credits_top = 0;
        self.stack.push((screen, first.unwrap_or(0)));
        self.centre_list();
        if !self.selectable(screen, self.chosen()) {
            self.step(1);
        }
    }

    fn pop(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.centre_list();
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
            // by meaning, the same in both modes (owner's decision 2026-11-09); what is empty or unavailable is
            // left out: the original needs the copy, the challenges the copy and their folder
            Screen::Kinds => [
                self.has_ours.then_some(Line::Ours),
                self.copy.then_some(Line::Classic),
                (self.copy && self.modified).then_some(Line::Challenges),
                Some(Line::Community),
                Some(Line::Back),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Screen::Community => [
                self.has_best.then_some(Line::Best),
                Some(Line::Downloaded),
                Some(Line::Back),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Screen::Start => (0..self.entries.len())
                .map(Line::Level)
                .chain([Line::Back])
                .collect(),
            Screen::Custom => (0..self.maps.len())
                .map(Line::Map)
                .chain([Line::Back])
                .collect(),
            Screen::Options => [
                Some(Line::Graphics),
                Some(Line::Controls),
                Some(Line::Sound),
                Some(Line::Language),
                // bringing in the player's own Ballance: a setting, offered while there is no copy
                (!self.copy).then_some(Line::Import),
                Some(Line::Extended),
                Some(Line::Back),
            ]
            .into_iter()
            .flatten()
            .collect(),
            Screen::Language => (0..languages().len())
                .map(Line::Lang)
                .chain([Line::Back])
                .collect(),
            // the original's two volumes: the music's and the effects'
            // the original's two volumes, and ours: the sound theme
            Screen::Sound => vec![Line::Music, Line::Effects, Line::SoundTheme, Line::Back],
            Screen::Extended if self.features.mode => [Line::Mode]
                .into_iter()
                .chain(switches(self.features))
                .chain([
                    Line::Interface,
                    Line::Models,
                    Line::SoundTheme,
                    Line::ModsMenu,
                    Line::Back,
                ])
                .collect(),
            Screen::Mods => (0..self.mods.found.len())
                .map(Line::Mod)
                .chain([Line::Back])
                .collect(),
            Screen::ModPage => [Line::ModOn]
                .into_iter()
                .chain((0..self.page_options().len()).map(Line::ModOpt))
                .chain([Line::Back])
                .collect(),
            Screen::Extended => vec![Line::Mode, Line::Back],
            Screen::Highscore if self.after_entry => vec![Line::Next],
            // the original's lines, ours (the full screen, the texture and model packs: either mode), and the extended
            // mode's field of view
            Screen::Graphics => {
                let mut l = vec![
                    Line::Resolution,
                    Line::Synch,
                    Line::Clouds,
                    Line::Fullscreen,
                    Line::Look,
                ];
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
        if screen == Screen::Credits {
            // the credits scroll a line a press
            self.credits_top = self
                .credits_top
                .saturating_add_signed(by)
                .min(self.credits_max.get());
            return;
        }
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

    /// PageUp (-1) or PageDown (+1): a list moves by a page (the chosen row with it); the credits by a page of lines.
    pub fn page_by(&mut self, by: isize) {
        let sc = self.screen();
        if sc == Screen::Credits {
            for _ in 0..10 {
                self.step(by);
            }
            return;
        }
        let n = self.list_len(sc);
        let lines = self.page_len();
        if !scrolls(sc) || n <= lines {
            // a short screen: the first or the last line
            return self.edge(by);
        }
        let c = self.chosen().min(n - 1);
        let start = self.shown(n).start;
        let moved = lines as isize * by;
        let top = start.saturating_add_signed(moved).min(n - lines);
        let at = c.saturating_add_signed(moved).min(n - 1);
        self.top = top;
        if let Some(s) = self.stack.last_mut() {
            s.1 = at.clamp(top, top + lines - 1);
        }
    }

    /// Home (-1) or End (+1): the first or the last line of the list (the last row, not Back).
    pub fn edge(&mut self, by: isize) {
        let sc = self.screen();
        if sc == Screen::Credits {
            self.credits_top = if by < 0 { 0 } else { self.credits_max.get() };
            return;
        }
        let n = if scrolls(sc) {
            self.list_len(sc)
        } else {
            self.lines(sc).len()
        };
        let order: Vec<usize> = if by < 0 {
            (0..n).collect()
        } else {
            (0..n).rev().collect()
        };
        if let Some(i) = order.into_iter().find(|&i| self.selectable(sc, i)) {
            self.set_chosen(i);
            if scrolls(sc) {
                self.top = if by < 0 {
                    0
                } else {
                    n.saturating_sub(self.page_len())
                };
            }
        }
    }

    /// A letter typed on a long list: the next row (after the chosen one, round the list) whose title starts with it.
    pub fn letter(&mut self, typed: &str) {
        let sc = self.screen();
        let Some(c) = typed.chars().next().filter(|c| c.is_alphanumeric()) else {
            return;
        };
        if !scrolls(sc) || self.waiting() {
            return;
        }
        let lines = self.lines(sc);
        let n = self.list_len(sc);
        let start = self.chosen();
        let first = |l: Line| {
            self.label(l)
                .chars()
                .find(|c| c.is_alphanumeric())
                .is_some_and(|f| f.to_lowercase().eq(c.to_lowercase()))
        };
        for k in 1..=n {
            let i = (start + k) % n.max(1);
            if lines.get(i).is_some_and(|&l| first(l)) && self.selectable(sc, i) {
                self.set_chosen(i);
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
            (_, Some(Line::Look)) => Action::Look(crate::look::next(by)),
            (_, Some(Line::Models)) => next_pack(gamedata::packs::Kind::Models, by),
            (_, Some(Line::SoundTheme)) => Action::SoundTheme(next_theme(by)),
            (Screen::Graphics, Some(l)) => self.change_graphics(l, by),
            (Screen::Controls, Some(Line::Invert)) => self.toggle_invert(),
            (Screen::Extended, Some(Line::Mode)) => Action::ToggleDebug,
            (Screen::Extended, Some(Line::Interface)) => Action::Interface(next_provider(by)),
            (_, Some(Line::Mod(i))) => Action::ModSwitch(i),
            (Screen::ModPage, Some(Line::ModOn)) => Action::ModSwitch(self.mod_page),
            (Screen::ModPage, Some(Line::ModOpt(o))) => Action::ModOption(self.mod_page, o, by),
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
            // the levels come by meaning (decisions.md, "Start by meaning"): the original ones (the classic list),
            // ours, the challenges, and the community maps in a submenu
            Line::Start => open(self, Screen::Kinds),
            Line::Classic => open(self, Screen::Start),
            Line::Community => open(self, Screen::Community),
            Line::Ours => {
                self.maps = crate::levels::shipped();
                self.show(List::Ours, None);
                open(self, Screen::Custom)
            }
            Line::Best => {
                self.maps = crate::levels::community();
                self.show(List::Best, None);
                open(self, Screen::Custom)
            }
            Line::Downloaded => {
                self.maps = downloaded_levels();
                self.show(List::Downloaded, Some(crate::levels::dir(Kind::Downloaded)));
                open(self, Screen::Custom)
            }
            // grouped by set, titled from the names (modified.rs)
            Line::Challenges => {
                let root = crate::levels::dir(Kind::Modified);
                self.maps = crate::modified::levels(&root);
                self.show(List::Challenges, Some(root));
                open(self, Screen::Custom)
            }
            Line::Import => Action::Import,
            Line::Highscore => open(self, Screen::Highscore),
            Line::Options => open(self, Screen::Options),
            Line::Credits => open(self, Screen::Credits),
            Line::Graphics => open(self, Screen::Graphics),
            Line::Controls => open(self, Screen::Controls),
            Line::Sound => open(self, Screen::Sound),
            Line::Extended => open(self, Screen::Extended),
            Line::ModsMenu => open(self, Screen::Mods),
            Line::Mod(i) => {
                self.mod_page = i;
                open(self, Screen::ModPage)
            }
            Line::ModOn => Action::ModSwitch(self.mod_page),
            Line::ModOpt(o) => Action::ModOption(self.mod_page, o, 1),
            Line::Language => open(self, Screen::Language),
            Line::Lang(i) => match languages().get(i) {
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
            Line::Map(i) => self
                .maps
                .get(i)
                .map_or(Action::None, |p| Action::Map(p.level.clone())),
            Line::Mode => Action::ToggleDebug,
            Line::Look => Action::Look(crate::look::next(1)),
            Line::Interface => Action::Interface(next_provider(1)),
            Line::Models => next_pack(gamedata::packs::Kind::Models, 1),
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

    /// The list the Custom screen shows, and the folder it comes from (for its empty message).
    fn show(&mut self, list: List, folder: Option<PathBuf>) {
        self.list = list;
        self.folder = folder;
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
                    self.list = List::None;
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

    /// The buttons of the screen shown and where they are drawn, in a window of `screen`'s size: the interface's
    /// (the very rectangles it draws, any size and aspect) or the prototype's lines.
    pub fn hits(&self, interface: bool, screen: crate::ui::Screen) -> Vec<Hit> {
        if interface {
            let hits = std::cell::RefCell::new(Vec::new());
            self.draw(None, screen, &hits, None);
            hits.into_inner()
        } else {
            let mut hits = Vec::new();
            self.sketch(screen.w, screen.h, &mut hits);
            hits
        }
    }

    /// The button under the cursor.
    fn hit(&self, p: Pointer) -> Option<Hit> {
        let inside =
            |h: &Hit| p.x >= h.rect[0] && p.x < h.rect[2] && p.y >= h.rect[1] && p.y < h.rect[3];
        self.hits(p.interface, p.screen).into_iter().find(inside)
    }

    /// The cursor moved: the line under it is chosen, as the arrows would choose it (a line that cannot be chosen,
    /// or a key being awaited, leaves the choice alone). True when the chosen line changed.
    pub fn hover(&mut self, p: Pointer) -> bool {
        if self.waiting() {
            return false;
        }
        let Some(Hit {
            target: Target::Line(i),
            ..
        }) = self.hit(p)
        else {
            return false;
        };
        if i == self.chosen() || !self.selectable(self.screen(), i) {
            return false;
        }
        self.set_chosen(i);
        true
    }

    /// A left click: the button under the cursor chosen and pressed as Enter presses it; a line with a value goes
    /// down on its left half and up on its right half, as Left and Right change it; the highscores' arrows change
    /// the level. None when no button is there (or a key is awaited: a mouse button is not a key to bind).
    pub fn click(&mut self, p: Pointer) -> Option<Action> {
        if self.waiting() {
            return None;
        }
        let hit = self.hit(p)?;
        match hit.target {
            Target::Arrow(by) => Some(self.adjust(by)),
            Target::Bar => {
                // the list's page centred where the bar was clicked
                let n = self.list_len(self.screen());
                let f =
                    ((p.y - hit.rect[1]) / (hit.rect[3] - hit.rect[1]).max(1.0)).clamp(0.0, 1.0);
                let at = (f * n as f32).round() as usize;
                self.scroll_to(at.saturating_sub(self.page_len() / 2));
                Some(Action::None)
            }
            Target::Line(i) => {
                let sc = self.screen();
                if !self.selectable(sc, i) {
                    return None;
                }
                self.set_chosen(i);
                let by = if p.x < (hit.rect[0] + hit.rect[2]) / 2.0 {
                    -1
                } else {
                    1
                };
                Some(match self.lines(sc).get(i) {
                    Some(&l) if valued(l) => self.adjust(by),
                    _ => self.enter(),
                })
            }
        }
    }

    /// The wheel turned a notch, down (+1) or up (-1): a long list (the custom levels) scrolls, its line under the
    /// cursor (`at`) chosen, else the chosen line kept in view; the highscores go through the levels; elsewhere the
    /// lines are chosen as the arrows choose them.
    pub fn wheel(&mut self, by: i32, at: Option<Pointer>) -> Action {
        let by = by.signum();
        if self.waiting() || by == 0 {
            return Action::None;
        }
        let sc = self.screen();
        let n = self.list_len(sc);
        match sc {
            Screen::Highscore => return self.adjust(by),
            Screen::Entry => {}
            _ if scrolls(sc) && n > self.page_len() => {
                let start = self.shown(n).start;
                self.scroll_to(start.saturating_add_signed(by as isize));
                if let Some(p) = at {
                    self.hover(p);
                }
            }
            _ => self.step(by as isize),
        }
        Action::None
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

    /// The mod whose page is up, as read.
    fn page_txt(&self) -> Option<&gamedata::mods::ModTxt> {
        self.mods
            .found
            .get(self.mod_page)
            .and_then(|m| m.parsed.as_ref().ok())
    }

    fn page_options(&self) -> Vec<gamedata::mods::OptSpec> {
        self.page_txt()
            .map(|t| t.options.clone())
            .unwrap_or_default()
    }

    /// The help under the Mods screen's rows: the chosen line's description (the practice tools' keys; a mod's
    /// description, or why it was refused).
    fn mods_help(&self) -> Vec<String> {
        let s = &self.strings;
        match self.lines(Screen::Mods).get(self.chosen()) {
            Some(Line::Mod(i)) if self.screen() == Screen::Mods => {
                match self.mods.found.get(*i).map(|m| &m.parsed) {
                    Some(Ok(t)) => {
                        let mut v = vec![t.description(s.language())];
                        if let Some((_, why)) = self.mods.note.as_ref().filter(|n| n.0 == *i) {
                            v.push(format!("{}: {why}", s.ours("mod budget")));
                        }
                        if t.modded() {
                            v.push(s.ours("mod modded"));
                        }
                        v
                    }
                    Some(Err(e)) => vec![format!(
                        "{}: {}",
                        s.ours("mod refused"),
                        e.chars().take(80).collect::<String>()
                    )],
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// The help under a mod's page: its description, who made it, and whether it changes the rules.
    fn page_help(&self) -> Vec<String> {
        let s = &self.strings;
        let Some(t) = self.page_txt() else {
            return Vec::new();
        };
        let mut v = vec![t.description(s.language())];
        let by: Vec<&str> = [t.author.as_str(), t.version.as_str(), t.licence.as_str()]
            .into_iter()
            .filter(|x| !x.is_empty())
            .collect();
        if !by.is_empty() {
            v.push(by.join(" · "));
        }
        if t.modded() {
            v.push(s.ours("mod modded"));
        }
        v
    }

    /// The label of a line in the menus' words.
    fn label(&self, line: Line) -> String {
        let s = &self.strings;
        match line {
            Line::Start => s.get("Menu_Main Button Start", "Start"),
            Line::Import => s.ours("import"),
            Line::Classic => s.ours("classic ballance"),
            Line::Ours => s.ours("drahtseilakt"),
            Line::Challenges => s.ours("ballance challenges"),
            Line::Community => s.ours("community maps"),
            Line::Best => s.ours("best maps"),
            Line::Downloaded => s.ours("downloaded maps"),
            Line::Look => s.ours("look"),
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
            Line::Language => s.ours("language"),
            Line::Lang(i) => languages().get(i).map_or("", |l| l.1).to_owned(),
            Line::Extended => s.ours("extended"),
            Line::Level(i) => {
                let n = self.entries.get(i).map_or(0, |e| e.number);
                // the copy's own label for the level, else ours
                let copy = s.get(&format!("Menu_Start Button Level{n}"), "");
                if copy.is_empty() {
                    s.numbered("level", n)
                } else {
                    copy
                }
            }
            // the level's title (cleaned and capped, package.rs), any script
            Line::Map(i) => self.map_text(i).unwrap_or_default(),
            Line::Music => s.get("Menu_Opt_Sound Volume", "Music Volume"),
            Line::Effects => s.ours("sound volume"),
            Line::Mode => s.ours("extended mode"),
            Line::ModsMenu => s.ours("mods"),
            Line::Mod(i) => self
                .mods
                .found
                .get(i)
                .map_or_else(String::new, |m| m.name(s.language())),
            Line::ModOn => s.ours("mod on"),
            Line::ModOpt(o) => {
                let key = self
                    .page_options()
                    .get(o)
                    .map(|o| o.key.clone())
                    .unwrap_or_default();
                self.page_txt()
                    .map_or(key.clone(), |t| t.label(&key, s.language()))
            }
            Line::Interface => s.ours("interface"),
            Line::Models => s.ours("models"),
            Line::SoundTheme => s.ours("sounds"),
            Line::Switch(i) => FEATURES
                .get(i)
                .map_or_else(String::new, |f| s.ours(&format!("feature {}", f.key()))),
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

    /// A custom level's title as it is (any script), in the menus' language when it has one in it, a cut one's "…"
    /// as "..." (the game's font has no "…").
    fn map_text(&self, i: usize) -> Option<String> {
        self.maps
            .get(i)
            .map(|p| p.title_in(self.strings.language()).replace('…', "..."))
    }

    /// The community map `i`'s credit line: its author, version and year, as its level.txt gives them.
    fn credit(&self, i: usize) -> String {
        let Some(p) = self.maps.get(i) else {
            return String::new();
        };
        let mut parts: Vec<String> = Vec::new();
        if let Some(a) = &p.author {
            parts.push(format!("{} {a}", self.strings.ours("by")));
        }
        if let Some(v) = &p.version {
            parts.push(format!("v{v}"));
        }
        if let Some(y) = &p.posted {
            parts.push(y.chars().take(4).collect());
        }
        parts.join(", ").replace('…', "...")
    }

    /// A switch's row: its label and yes or no.
    fn switch_field(&self, line: Line) -> (String, String) {
        let on = match line {
            Line::Switch(i) => FEATURES
                .get(i)
                .is_some_and(|&f| self.features.switched_on(f)),
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
            Screen::Mods => s.ours("mods"),
            Screen::ModPage => self.label(Line::Mod(self.mod_page)),
            Screen::Language => s.ours("language"),
            Screen::Kinds => s.ours("levels"),
            Screen::Community => s.ours("community maps"),
            Screen::Custom => match self.list {
                List::Ours => self.label(Line::Ours),
                List::Best => self.label(Line::Best),
                List::Downloaded => self.label(Line::Downloaded),
                List::Challenges => self.label(Line::Challenges),
                List::None => s.ours("custom levels"),
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
                    (self.label(Line::Look), self.look_value()),
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
                    (self.label(Line::SoundTheme), theme_value(&self.strings)),
                ]
            }
            Screen::Extended => {
                // the mode, then each addition's switch, then where things are
                let mut f = vec![(self.label(Line::Mode), yes_no(self.features.mode))];
                if self.features.mode {
                    f.extend(switches(self.features).map(|l| self.switch_field(l)));
                    f.push((self.label(Line::Interface), self.interface_value()));
                    f.push((
                        self.label(Line::Models),
                        self.pack_value(gamedata::packs::Kind::Models),
                    ));
                    f.push((self.label(Line::SoundTheme), theme_value(&self.strings)));
                    f.push((self.label(Line::ModsMenu), ">".to_owned()));
                }
                f
            }
            Screen::Mods => {
                let mut f: Vec<(String, String)> = Vec::new();
                for (i, m) in self.mods.found.iter().enumerate() {
                    let v = match &m.parsed {
                        Err(_) => s.ours("mod refused"),
                        Ok(_) => yes_no(self.mods.on.get(i).copied().unwrap_or(false)),
                    };
                    f.push((self.label(Line::Mod(i)), v));
                }
                // under the rows: what the chosen line is
                for l in self.mods_help() {
                    f.push((l, String::new()));
                }
                f
            }
            Screen::ModPage => {
                let i = self.mod_page;
                let mut f = vec![(
                    self.label(Line::ModOn),
                    yes_no(self.mods.on.get(i).copied().unwrap_or(false)),
                )];
                for (k, o) in self.page_options().iter().enumerate() {
                    let v = self
                        .mods
                        .opts
                        .get(i)
                        .and_then(|v| v.get(k))
                        .copied()
                        .unwrap_or(o.default);
                    let shown = match o.kind {
                        gamedata::mods::OptKind::Switch => yes_no(v != 0.0),
                        // a key by its name in the menus' words, else winit's, short
                        gamedata::mods::OptKind::Key(_) => {
                            let name = o.show(v);
                            s.key(None, &name)
                                .unwrap_or_else(|| crate::modrt::key_text(&name))
                        }
                        gamedata::mods::OptKind::Choice(_) => self.page_txt().map_or_else(
                            || o.show(v),
                            |t| t.choice_label(&o.key, &o.show(v), s.language()),
                        ),
                        _ => o.show(v),
                    };
                    f.push((self.label(Line::ModOpt(k)), shown));
                }
                for l in self.page_help() {
                    f.push((l, String::new()));
                }
                f
            }
            Screen::Credits => [
                "Drahtseilakt by Veresk".to_owned(),
                "Ballance by Cyparade / Atari 2004".to_owned(),
                "Started as a fork of OpenBallance by emdzej".to_owned(),
                "Physics: IVP, ported from OpenBallance".to_owned(),
                "Music: Kevin MacLeod (incompetech.com, CC-BY 4.0), elerya, Hitctrl, Spring Spring, WakianTech \
                 (K-ManZ), Tsorthan Grove, robcro6010, CarlosCarty, Amingliang, juskiddink, botpl (CC-BY), \
                 neolein, Tozan (CC0)"
                    .to_owned(),
                "Sounds: Kenney, freesound and OpenGameArt authors (CC0, CC-BY): assets/sounds/system/CREDITS.md"
                    .to_owned(),
                "Textures: ambientCG.com (CC0), thank you".to_owned(),
                (1..=3)
                    .map(|k| s.ours(&format!("community credit {k}")))
                    .collect::<Vec<_>>()
                    .join(" "),
            ]
            .into_iter()
            .map(|l| (l, String::new()))
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
        self.sketch(width_px as f32, height_px as f32, &mut Vec::new())
    }

    /// The prototype's menu, and in `hits` where its lines are (each line's band, as the chosen one is shaded).
    fn sketch(&self, w: f32, h: f32, hits: &mut Vec<Hit>) -> Vec<Rect> {
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
        // a long list shows the lines around the chosen one (the custom levels' Back as well, after them)
        let mut shown: Vec<usize> = self.shown(self.list_len(screen)).collect();
        if screen == Screen::Custom {
            shown.push(lines.len().saturating_sub(1));
        }
        let total = 6.0 + fields.len() as f32 + 1.2 * shown.len() as f32;
        let mut y = ((h - total * line) / 2.0).max(0.0);
        let title = self
            .title(screen)
            .map_or_else(|| "DRAHTSEILAKT".to_owned(), |t| t.trim().to_uppercase());
        centre(&mut out, &title, y, s * 1.5, GOLD);
        y += 2.5 * line;
        for (l, v) in &fields {
            let t = format!("{}  {}", l.trim(), v).trim().to_uppercase();
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
            let pad = s * 3.0;
            let lw = width(&t, s);
            let band = Rect {
                x: (w - lw) / 2.0 - pad,
                y: y - s * 1.5,
                w: lw + 2.0 * pad,
                h: line,
                color: SELECT,
            };
            hits.push(Hit {
                rect: [band.x, band.y, band.x + band.w, band.y + band.h],
                target: Target::Line(i),
            });
            if i == self.chosen() {
                out.push(band);
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

/// The original's menu screens (Menu.nmo's 2D frames, measured in the C game): a dark band down the middle, the
/// buttons over Button01_select (chosen), Button01_deselect or Button01_special (not playable), their labels in the
/// game's fonts (GameFont_01 for the main buttons, GameFont_03 and the grey GameFont_03a for the small ones).
impl Menu {
    pub fn interface(
        &self,
        font: Option<&crate::ui::Face>,
        screen: crate::ui::Screen,
    ) -> Vec<crate::ui::Quad> {
        self.draw(font, screen, &std::cell::RefCell::new(Vec::new()), None)
    }

    /// The interface's quads, and in `hits` the buttons a click or the cursor can choose, where they are drawn.
    fn draw(
        &self,
        font: Option<&crate::ui::Face>,
        screen: crate::ui::Screen,
        hits: &std::cell::RefCell<Vec<Hit>>,
        probe: Option<&std::cell::RefCell<Vec<Probe>>>,
    ) -> Vec<crate::ui::Quad> {
        use crate::ui::{Anchor, Quad, homogeneous};
        // behind the menus the game draws the interface's 3D scene (the original's or our own, menuscene.rs)
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
        // a label is fitted to its frame (fit_label): smaller, on two lines where the frame is tall enough, or
        // shortened; what was drawn is told to `probe` (the layout tests)
        // a label on a button with its ball at the left end (`ball`: the ball's width): centred on the whole
        // button while it clears the ball, else centred on what lies right of the ball (the owner's report,
        // 2026-11-08: long Russian labels ran over the ball)
        let areas: std::cell::RefCell<Vec<([f32; 4], f32)>> = std::cell::RefCell::new(Vec::new());
        // while a column of labels is sized (no probe record, nothing kept), and the column's number (0: none)
        let dry = std::cell::Cell::new(false);
        let group = std::cell::Cell::new(0u32);
        let groups = std::cell::Cell::new(0u32);
        let label_ball =
            |out: &mut Vec<Quad>, t: &str, r: [f32; 4], style: &crate::ui::Style, ball: f32| {
                let Some(f) = font else { return };
                // on a button: no further right than its art lets text go
                let mut r = r;
                let centre = [(r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0];
                if let Some(&(b, right)) = areas.borrow().iter().rev().find(|(b, _)| {
                    centre[0] >= b[0] && centre[0] <= b[2] && centre[1] >= b[1] && centre[1] <= b[3]
                }) {
                    r[2] = r[2].min(b[0] + (b[2] - b[0]) * right);
                }
                let fit = |r: [f32; 4]| {
                    let frame = homogeneous(r);
                    let (quads, k) = fit_label(f, t, frame, style);
                    (frame, crate::ui::place(quads, screen, Anchor::Center), k)
                };
                // the text area: right of the ball (the art's text limit at the right); every line of every label
                // is centred on this area's middle, the block in its height
                let clear = (ball > 0.0).then(|| {
                    screen.rect(homogeneous([r[0] + ball, r[1], r[2], r[3]]), Anchor::Center)[0]
                });
                let (frame, placed, k) = fit([r[0] + ball, r[1], r[2], r[3]]);
                if let (Some(p), false) = (probe, dry.get()) {
                    p.borrow_mut().push(Probe {
                        text: t.to_owned(),
                        frame: screen.rect(frame, Anchor::Center),
                        quads: placed.iter().map(|q| q.rect).collect(),
                        scale: k,
                        wrap: style.flags & crate::ui::TEXT_WORDWRAP != 0,
                        line: screen.y(line_height(style) * k) - screen.y(0.0),
                        clear_left: clear,
                        size: style.scale[1] * k,
                        panel: None,
                        cut: LAST_CUT.with(|c| c.get()),
                        art_right: None,
                        fill: None,
                        group: group.get(),
                    });
                }
                out.extend(placed);
            };
        let label = |out: &mut Vec<Quad>, t: &str, r: [f32; 4], style: &crate::ui::Style| {
            label_ball(out, t, r, style, 0.0)
        };
        // a column of buttons of one kind: the labels share one size, on one line each if that stays readable, else
        // on two lines each (where a text has a place to break); (text, frame, style, ball)
        let label_group =
            |out: &mut Vec<Quad>, items: &[(String, [f32; 4], &crate::ui::Style, f32)]| {
                if items.len() < 2 {
                    for (t, r, st, b) in items {
                        label_ball(out, t, *r, st, *b);
                    }
                    return;
                }
                dry.set(true);
                let mut scratch = Vec::new();
                let mut cands = Vec::new();
                for (t, r, st, b) in items {
                    label_ball(&mut scratch, t, *r, st, *b);
                    cands.push((
                        CAND.with(|c| c.get()),
                        floor_k(st).max(FIT_SHRINK),
                        floor_k(st),
                    ));
                }
                dry.set(false);
                let one = cands.iter().map(|c| c.0.0).fold(1.0f32, f32::min);
                let fit_min = cands.iter().map(|c| c.1).fold(0.0f32, f32::max);
                let kmin = cands.iter().map(|c| c.2).fold(0.0f32, f32::max);
                // a hair under, so that the text that sets the size does not run a rounding error past its frame
                let force = if one >= fit_min {
                    (one * 0.999, false)
                } else {
                    let two = cands
                        .iter()
                        .map(|c| c.0.1.unwrap_or(c.0.0))
                        .fold(1.0f32, f32::min);
                    ((two * 0.999).max(kmin), true)
                };
                groups.set(groups.get() + 1);
                group.set(groups.get());
                FORCE.with(|c| c.set(Some(force)));
                for (t, r, st, b) in items {
                    label_ball(out, t, *r, st, *b);
                }
                FORCE.with(|c| c.set(None));
                group.set(0);
            };
        let button =
            |out: &mut Vec<Quad>, r: [f32; 4], uv: [f32; 4], state: Button, at: Option<Target>| {
                if let Some(target) = at {
                    hits.borrow_mut().push(Hit {
                        rect: screen.rect(homogeneous(r), Anchor::Center),
                        target,
                    });
                }
                let tex = match state {
                    Button::Over => SELECT_TEX,
                    Button::Up => DESELECT_TEX,
                    Button::Inactive => SPECIAL,
                };
                areas.borrow_mut().push((r, text_right(uv)));
                if let Some(p) = probe {
                    let px = screen.rect(homogeneous(r), Anchor::Center);
                    p.borrow_mut().push(Probe {
                        text: String::new(),
                        frame: px,
                        quads: Vec::new(),
                        scale: 1.0,
                        wrap: false,
                        line: 0.0,
                        clear_left: None,
                        size: 1.0,
                        panel: None,
                        cut: false,
                        art_right: Some(px[0] + (px[2] - px[0]) * text_right(uv)),
                        fill: None,
                        group: 0,
                    });
                }
                out.push(quad(r, uv, tex));
            };
        // a row of our own screens: the key row's bar without the mark in its middle (that mark parts a key's name
        // from the key on the Controls page; under a label it reads as stripes), stitched from the bar's two ends
        let row = |out: &mut Vec<Quad>, r: [f32; 4], state: Button, at: Option<Target>| {
            if let Some(target) = at {
                hits.borrow_mut().push(Hit {
                    rect: screen.rect(homogeneous(r), Anchor::Center),
                    target,
                });
            }
            let tex = match state {
                Button::Over => SELECT_TEX,
                Button::Up => DESELECT_TEX,
                Button::Inactive => SPECIAL,
            };
            areas.borrow_mut().push((r, text_right(KEY_UV)));
            if let Some(p) = probe {
                let px = screen.rect(homogeneous(r), Anchor::Center);
                p.borrow_mut().push(Probe {
                    text: String::new(),
                    frame: px,
                    quads: Vec::new(),
                    scale: 1.0,
                    wrap: false,
                    line: 0.0,
                    clear_left: None,
                    size: 1.0,
                    panel: None,
                    cut: false,
                    art_right: Some(px[0] + (px[2] - px[0]) * text_right(KEY_UV)),
                    fill: None,
                    group: 0,
                });
            }
            let mid = (r[0] + r[2]) / 2.0;
            let [u0, v0, u1, v1] = KEY_UV;
            out.push(quad([r[0], r[1], mid, r[3]], [u0, v0, ROW_CUT[0], v1], tex));
            out.push(quad([mid, r[1], r[2], r[3]], [ROW_CUT[1], v0, u1, v1], tex));
        };
        // a scrolling list's indicator: a slim bar beside the rows, its thumb as long as the part shown, and how many
        // rows are above and below ("▲ 3 more", "▼ 12 more"); the bar is a button (a click scrolls there)
        let scroll = |out: &mut Vec<Quad>,
                      x: f32,
                      top: f32,
                      bottom: f32,
                      shown: std::ops::Range<usize>,
                      total: usize| {
            if total <= shown.len() || total == 0 {
                return;
            }
            let plain = |out: &mut Vec<Quad>, r: [f32; 4], a: f32| {
                out.push(Quad {
                    top: [1.0, 1.0, 1.0, a],
                    bottom: [1.0, 1.0, 1.0, a],
                    ..quad(r, [0.0, 0.0, 1.0, 1.0], PLAIN)
                });
            };
            let track = [x, top, x + SCROLL_W, bottom];
            hits.borrow_mut().push(Hit {
                rect: screen.rect(homogeneous(track), Anchor::Center),
                target: Target::Bar,
            });
            plain(out, track, 0.18);
            let h = bottom - top;
            let t0 = top + h * shown.start as f32 / total as f32;
            let t1 = top + h * shown.end as f32 / total as f32;
            plain(out, [x, t0, x + SCROLL_W, t1.max(t0 + 0.01)], 0.7);
            let more = |n: usize| self.strings.ours("more").replace("{n}", &n.to_string());
            let mut st = FONT_HINT;
            st.align = crate::ui::ALIGN_RIGHT;
            if shown.start > 0 {
                label(
                    out,
                    &format!("▲ {}", more(shown.start)),
                    [x - 0.3, top - HINT_H, x + SCROLL_W, top],
                    &st,
                );
            }
            if shown.end < total {
                label(
                    out,
                    &format!("▼ {}", more(total - shown.end)),
                    [x - 0.3, bottom, x + SCROLL_W, bottom + HINT_H],
                    &st,
                );
            }
        };
        // a note: help or thanks as lines centred on a panel sized to them (a dark, rounded box with the buttons' rim),
        // its bottom (`up`) or its top at `y`, at most `max_h` high, in the centre strip; the box is drawn over
        // nothing else (the layouts keep it clear of the rows and of Back)
        let note = |out: &mut Vec<Quad>, text: &str, y: f32, up: bool, max_h: f32| {
            let Some(f) = font else { return };
            if text.trim().is_empty() {
                return;
            }
            let (pad_x, pad_y, corner) = (NOTE_PAD[0], NOTE_PAD[1], 7.0f32);
            let st = FONT_HELP;
            let max_w = (NOTE_X[1] - NOTE_X[0]) * crate::ui::CONTEXT_W - 2.0 * pad_x;
            // a note is never cut: smaller down to MIN_TEXT, then taller than `max_h` (the layouts leave room)
            let (lines, k, _) = fit_whole(
                f,
                text,
                max_w,
                max_h * crate::ui::CONTEXT_H - 2.0 * pad_y,
                &st,
            );
            let st = scaled_style(&st, k);
            let pitch = row_pitch(&st);
            let wide = lines
                .iter()
                .map(|l| text_wide(f, l, &st))
                .fold(0.0f32, f32::max);
            let pw = (wide + 2.0 * pad_x).max(60.0);
            let ph = lines.len() as f32 * pitch + 2.0 * pad_y;
            let cx = crate::ui::CONTEXT_W / 2.0;
            let y0 = if up {
                y * crate::ui::CONTEXT_H - ph
            } else {
                y * crate::ui::CONTEXT_H
            };
            let (x0, x1, y1) = (cx - pw / 2.0, cx + pw / 2.0, y0 + ph);
            let fr = |a: f32, b: f32, c: f32, d: f32| {
                [
                    a / crate::ui::CONTEXT_W,
                    b / crate::ui::CONTEXT_H,
                    c / crate::ui::CONTEXT_W,
                    d / crate::ui::CONTEXT_H,
                ]
            };
            // the dark fill: exactly the rim's inner opening (the nine's middle), its corners rounded in steps so it
            // meets the rim and never pokes out of it; pieces that do not overlap (the fill is translucent)
            let (fx0, fy0, fx1, fy1) = (x0 + corner, y0 + corner, x1 - corner, y1 - corner);
            let r = 3.0f32.min((fx1 - fx0) / 2.0).min((fy1 - fy0) / 2.0);
            let dark = |out: &mut Vec<Quad>, a: f32, b: f32, c: f32, d: f32| {
                if c > a && d > b {
                    out.push(Quad {
                        top: [0.0, 0.0, 0.0, 0.78],
                        bottom: [0.0, 0.0, 0.0, 0.78],
                        ..quad(fr(a, b, c, d), [0.0, 0.0, 1.0, 1.0], PLAIN)
                    });
                }
            };
            dark(out, fx0, fy0 + r, fx1, fy1 - r);
            dark(out, fx0 + r * 0.5, fy0 + r * 0.25, fx1 - r * 0.5, fy0 + r);
            dark(out, fx0 + r * 0.5, fy1 - r, fx1 - r * 0.5, fy1 - r * 0.25);
            dark(out, fx0 + r, fy0, fx1 - r, fy0 + r * 0.25);
            dark(out, fx0 + r, fy1 - r * 0.25, fx1 - r, fy1);
            let fill_box = [
                screen.x(fx0, Anchor::Center),
                screen.y(fy0),
                screen.x(fx1, Anchor::Center),
                screen.y(fy1),
            ];
            // the rim: the Yes/No button's rounded box (metal, no ball) cut in nine, its corners kept
            let [u0, v0, u1, v1] = YES_NO_UV;
            let d = 9.0 / 256.0;
            let xs = [x0, x0 + corner, x1 - corner, x1];
            let ys = [y0, y0 + corner, y1 - corner, y1];
            let us = [u0, u0 + d, u1 - d, u1];
            let vs = [v0, v0 + d, v1 - d, v1];
            for r in 0..3 {
                for c in 0..3 {
                    if (r, c) == (1, 1) {
                        continue;
                    }
                    out.push(quad(
                        fr(xs[c], ys[r], xs[c + 1], ys[r + 1]),
                        [us[c], vs[r], us[c + 1], vs[r + 1]],
                        DESELECT_TEX,
                    ));
                }
            }
            let panel = [
                screen.x(x0, Anchor::Center),
                screen.y(y0),
                screen.x(x1, Anchor::Center),
                screen.y(y1),
            ];
            let m = st.margins;
            for (i, l) in lines.iter().enumerate() {
                let ly = y0 + pad_y + i as f32 * pitch;
                let _ = m;
                let frame = [x0, ly, x1, ly + pitch];
                let q = crate::ui::place(f.layout(l, frame, &st), screen, Anchor::Center);
                if let Some(p) = probe {
                    p.borrow_mut().push(Probe {
                        text: l.clone(),
                        frame: screen.rect(frame, Anchor::Center),
                        quads: q.iter().map(|q| q.rect).collect(),
                        scale: k,
                        wrap: false,
                        line: screen.y(line_height(&st)) - screen.y(0.0),
                        clear_left: None,
                        size: st.scale[1],
                        panel: Some(panel),
                        cut: false,
                        art_right: None,
                        fill: Some(fill_box),
                        group: 0,
                    });
                }
                out.extend(q);
            }
        };
        let sc = self.screen();
        let lines = self.lines(sc);
        // the screens with a help panel: where it begins (the rows above it keep clear of it)
        let help_top = match (font, sc) {
            (
                Some(f),
                Screen::Main
                | Screen::Kinds
                | Screen::Community
                | Screen::Start
                | Screen::Options
                | Screen::Graphics
                | Screen::Controls
                | Screen::Sound
                | Screen::Extended
                | Screen::Language
                | Screen::Pause
                | Screen::Dead
                | Screen::End
                | Screen::Ask(_),
            ) => self.help_top(f, sc),
            _ => 0.6,
        };
        let help_panel = |out: &mut Vec<Quad>, y: f32, up: bool, max_h: f32| {
            note(out, &self.help_panel(sc), y, up, max_h);
        };
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
        let big = |out: &mut Vec<Quad>, top: f32, last: f32| {
            let n = lines.iter().filter(|&&l| l != Line::Back).count().max(1);
            let step = if top + 0.15 * (n as f32 - 1.0) <= last + 0.02 {
                0.15
            } else {
                (last - top) / (n as f32 - 1.0).max(1.0)
            };
            let mut k = 0;
            let mut column = Vec::new();
            for (i, &l) in lines.iter().enumerate() {
                if l == Line::Back {
                    button(out, BACK_RECT, BACK_UV, state(i), Some(Target::Line(i)));
                    label_ball(out, &self.label(l), BACK_RECT, &FONT_01, BALL_BACK);
                    continue;
                }
                let y = top + k as f32 * step;
                let r = [0.35, y, 0.65, y + (step - 0.004).min(0.0938)];
                button(out, r, MAIN_UV, state(i), Some(Target::Line(i)));
                column.push((self.label(l), r, &FONT_01, BALL_MAIN));
                k += 1;
            }
            label_group(out, &column);
        };
        let title = |out: &mut Vec<Quad>| {
            if let Some(t) = self.title(sc) {
                label(out, &t, [0.35, 0.15, 0.65, 0.25], &FONT_01);
            }
        };
        // the title of a page with a help panel: higher, to leave the rows room
        let title_up = |out: &mut Vec<Quad>| {
            if let Some(t) = self.title(sc) {
                label(out, &t, [0.35, 0.06, 0.65, 0.15], &FONT_01);
            }
        };
        let back_only = |out: &mut Vec<Quad>| {
            if let Some(i) = lines
                .iter()
                .position(|&l| l == Line::Back || l == Line::Next)
            {
                button(out, BACK_RECT, BACK_UV, state(i), Some(Target::Line(i)));
                label_ball(out, &self.label(lines[i]), BACK_RECT, &FONT_01, BALL_BACK);
            }
        };
        match sc {
            Screen::Main | Screen::Pause | Screen::Dead | Screen::End => {
                big(&mut out, 0.15, (help_top - 0.105).min(0.74));
                help_panel(&mut out, NOTE_BOTTOM, true, NOTE_BOTTOM - help_top);
            }
            Screen::Options | Screen::Kinds | Screen::Community => {
                title_up(&mut out);
                big(&mut out, 0.17, (help_top - 0.105).min(0.74));
                help_panel(&mut out, NOTE_BOTTOM, true, NOTE_BOTTOM - help_top);
            }
            Screen::Language => {
                title_up(&mut out);
                help_panel(&mut out, NOTE_BOTTOM, true, NOTE_BOTTOM - help_top);
                let step = 0.065f32.min((help_top - 0.105 - 0.17) / 6.0);
                // the languages in their own names, the one spoken in gold
                for (i, &l) in lines.iter().enumerate() {
                    let Line::Lang(k) = l else {
                        button(
                            &mut out,
                            BACK_RECT,
                            BACK_UV,
                            state(i),
                            Some(Target::Line(i)),
                        );
                        label_ball(&mut out, &self.label(l), BACK_RECT, &FONT_01, BALL_BACK);
                        continue;
                    };
                    let y = 0.17 + k as f32 * step;
                    let r = [0.4031, y, 0.5969, y + step - 0.013];
                    button(&mut out, r, LEVEL_UV, state(i), Some(Target::Line(i)));
                    let spoken = languages()
                        .get(k)
                        .is_some_and(|c| c.0 == self.strings.language());
                    let style = if spoken { FONT_03_GOLD } else { FONT_03 };
                    label(
                        &mut out,
                        &self.label(l),
                        [0.42, y, 0.5969, y + step - 0.013],
                        &style,
                    );
                }
            }
            Screen::Start => {
                // 0.06 apart from 0.09 (twelve levels); a thirteenth closes them up
                let n = self.entries.len().max(1);
                let step = ((help_top - 0.075) / n as f32).min(0.06);
                help_panel(&mut out, NOTE_BOTTOM, true, NOTE_BOTTOM - help_top);
                let mut column = Vec::new();
                for (i, &l) in lines.iter().enumerate() {
                    let Line::Level(k) = l else {
                        button(
                            &mut out,
                            BACK_RECT,
                            BACK_UV,
                            state(i),
                            Some(Target::Line(i)),
                        );
                        label_ball(&mut out, &self.label(l), BACK_RECT, &FONT_01, BALL_BACK);
                        continue;
                    };
                    let y = 0.07 + k as f32 * step;
                    let r = [0.4031, y, 0.5969, y + step - 0.01];
                    button(&mut out, r, LEVEL_UV, state(i), Some(Target::Line(i)));
                    let style = if self.selectable(sc, i) {
                        &FONT_03
                    } else {
                        &FONT_03A
                    };
                    column.push((self.label(l), r, style, 0.0));
                    if let Some(t) = self.entries.get(k).and_then(|e| e.best) {
                        let mut st = FONT_03A;
                        st.align = crate::ui::ALIGN_LEFT;
                        label(&mut out, &clock(t), [0.6, y, 0.7, y + step - 0.01], &st);
                    }
                }
                label_group(&mut out, &column);
            }
            Screen::Custom => {
                title(&mut out);
                // each level: its difficulty (a star and 1 to 10, "?" unknown), its title in the menus' language
                if let Some(f) = font {
                    self.note_rows.set(self.fit_rows(f));
                }
                let n = self.list_len(sc);
                let shown = self.shown(n);
                for (k, i) in shown.clone().enumerate() {
                    let y = LIST_TOP + k as f32 * ROW_STEP;
                    let r = [LIST_X[0], y, LIST_X[1], y + ROW_H];
                    row(&mut out, r, state(i), Some(Target::Line(i)));
                    let x = LIST_X[0] + ROW_INDENT;
                    if let Line::Map(m) = lines[i] {
                        let d = self
                            .maps
                            .get(m)
                            .and_then(|p| p.difficulty)
                            .map_or_else(|| "?".to_owned(), |d| d.to_string());
                        let mut st = FONT_HINT;
                        st.align = crate::ui::ALIGN_LEFT;
                        label(
                            &mut out,
                            &format!("★{d}"),
                            [x, y, x + 0.045, y + ROW_H],
                            &st,
                        );
                    }
                    let mut st = FONT_ROW;
                    st.align = crate::ui::ALIGN_LEFT;
                    label(
                        &mut out,
                        &self.label(lines[i]),
                        [x + 0.045, y, LIST_X[1] - 0.012, y + ROW_H],
                        &st,
                    );
                }
                let bottom = LIST_TOP + shown.len().saturating_sub(1) as f32 * ROW_STEP + ROW_H;
                scroll(&mut out, LIST_X[1] + 0.006, LIST_TOP, bottom, shown, n);
                let said: Vec<String> = self.fields(sc).into_iter().map(|f| f.0).collect();
                note(&mut out, &said.join("\n"), 0.30, false, 0.3);
                // the list's note (what it is, the chosen map's author, where the folder is), Esc's hint its last
                // line; without a note the hint alone takes the note's place
                let text = if self.note_below() {
                    self.list_note(lines.get(self.chosen()).copied())
                } else {
                    String::new()
                };
                let top = LIST_TOP + self.page_len() as f32 * ROW_STEP + HINT_H + 0.008;
                note(
                    &mut out,
                    &self.with_hint(text),
                    NOTE_BOTTOM,
                    true,
                    NOTE_BOTTOM - top,
                );
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
                    Some(Target::Arrow(-1)),
                );
                button(
                    &mut out,
                    [0.6038, 0.174, 0.64, 0.2257],
                    ARROW_RIGHT_UV,
                    Button::Up,
                    Some(Target::Arrow(1)),
                );
                for (k, (name, points)) in self.fields(sc).iter().enumerate() {
                    let y = 0.30 + k as f32 * 0.05;
                    button(
                        &mut out,
                        [0.36, y, 0.64, y + 0.041],
                        PLACE_UV,
                        Button::Inactive,
                        None,
                    );
                    let u = 0.0627 * k as f32;
                    let ny = y + 0.007;
                    button(
                        &mut out,
                        [0.364, ny, 0.384, ny + 0.0267],
                        [u, 0.0, u + 0.0588, 0.0588],
                        Button::Inactive,
                        None,
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
            Screen::Mods | Screen::ModPage => {
                title(&mut out);
                let fields = self.fields(sc);
                // the rows (a long list scrolls), then the help under them as a paragraph
                let rows = lines.len().saturating_sub(1);
                let shown = self.shown(rows);
                for (k, i) in shown.clone().enumerate() {
                    let Some((name, value)) = fields.get(i) else {
                        continue;
                    };
                    let y = MODS_TOP + k as f32 * ROW_STEP;
                    let st = if self.chosen() == i {
                        Button::Over
                    } else {
                        Button::Up
                    };
                    row(
                        &mut out,
                        [ROWS_X[0], y, ROWS_X[1], y + ROW_H],
                        st,
                        Some(Target::Line(i)),
                    );
                    // the value gets what its longest needs (a switch's Yes or No, a number, a key), the name the rest
                    let split = font.map_or(ROWS_X[0] + (ROWS_X[1] - ROWS_X[0]) * 0.74, |f| {
                        let need = fields
                            .iter()
                            .map(|(_, v)| text_wide(f, v, &FONT_ROW))
                            .fold(0.0f32, f32::max);
                        let value = (need + 14.0) / crate::ui::CONTEXT_W;
                        (ROWS_X[1] - 0.03 - value.clamp(0.06, 0.12)).max(ROWS_X[0] + 0.15)
                    });
                    let mut left = FONT_ROW;
                    left.align = crate::ui::ALIGN_LEFT;
                    label(
                        &mut out,
                        name,
                        [ROWS_X[0] + ROW_INDENT, y, split, y + ROW_H],
                        &left,
                    );
                    label(
                        &mut out,
                        value,
                        [split, y, ROWS_X[1] - 0.01, y + ROW_H],
                        &FONT_ROW,
                    );
                }
                let bottom = MODS_TOP + shown.len().saturating_sub(1) as f32 * ROW_STEP + ROW_H;
                let shown_n = shown.len();
                scroll(&mut out, ROWS_X[1] + 0.006, MODS_TOP, bottom, shown, rows);
                let help: Vec<&str> = fields
                    .iter()
                    .skip(rows)
                    .map(|(l, _)| l.as_str())
                    .filter(|l| !l.is_empty())
                    .collect();
                let top = MODS_TOP + shown_n as f32 * ROW_STEP + HINT_H + 0.008;
                note(
                    &mut out,
                    &self.with_hint(help.join("\n")),
                    NOTE_BOTTOM,
                    true,
                    NOTE_BOTTOM - top,
                );
                back_only(&mut out);
            }
            Screen::Graphics | Screen::Controls | Screen::Sound | Screen::Extended => {
                title_up(&mut out);
                let fields = self.fields(sc);
                // one row for each line but Back: its name left, its value right; the keys as the key bar
                let n = fields.len().max(2);
                let cap: f32 = if sc == Screen::Controls { 0.06 } else { 0.05 };
                let step = cap.min((help_top - 0.17 - 0.0396 - 0.012) / (n as f32 - 1.0));
                for (k, (name, value)) in fields.iter().enumerate() {
                    let line = lines[k];
                    let y = 0.17 + k as f32 * step;
                    let st = if self.chosen() == k {
                        Button::Over
                    } else {
                        Button::Up
                    };
                    let at = Some(Target::Line(k));
                    if matches!(line, Line::Key(_)) {
                        // the rows a little wider than the original's and their words a size smaller, so the
                        // longest names (Russian, French) fit either side of the bar's middle mark
                        button(&mut out, [0.32, y, 0.68, y + 0.0396], KEY_UV, st, at);
                        label(
                            &mut out,
                            name,
                            [0.32 + ROW_INDENT, y, 0.494, y + 0.0396],
                            &FONT_ROW,
                        );
                        label(&mut out, value, [0.508, y, 0.67, y + 0.0396], &FONT_ROW);
                    } else if line == Line::Defaults {
                        row(&mut out, [0.32, y, 0.68, y + 0.0396], st, at);
                        label(
                            &mut out,
                            name,
                            [0.32 + ROW_INDENT, y, 0.67, y + 0.0396],
                            &FONT_ROW,
                        );
                    } else {
                        row(&mut out, [0.32, y, 0.68, y + 0.0396], st, at);
                        let mut left = FONT_ROW;
                        left.align = crate::ui::ALIGN_LEFT;
                        // the name and the value share the row by what each needs: the value keeps its usual
                        // place (right of 0.56) unless it is wider, then the name gives way, down to the share
                        // the two words have of the room
                        let mut mid = 0.56;
                        let mut x0 = 0.32 + ROW_INDENT;
                        let x1 = 0.32 + 0.36 * text_right(KEY_UV);
                        if let Some(f) = font {
                            let nat = |t: &str| {
                                quads_wide(&f.layout(
                                    t,
                                    crate::ui::homogeneous([0.0, y, 1.0, y + 0.0396]),
                                    &left,
                                ))
                            };
                            let (nw, vw) = (nat(name), nat(value));
                            let room = (x1 - x0) * crate::ui::CONTEXT_W;
                            if vw > (x1 - 0.56) * crate::ui::CONTEXT_W - 4.0 && nw + vw > 0.0 {
                                let want = if nw + vw + 6.0 <= room {
                                    x1 - (vw + 6.0) / crate::ui::CONTEXT_W
                                } else {
                                    // too much for the row at full size: both are drawn as small as allowed
                                    // (the name gets what it needs at that size, the value the rest)
                                    x0 = 0.326;
                                    let k = floor_k(&left);
                                    let m = left.margins[0] + left.margins[2];
                                    x0 + (nw * k + m) / crate::ui::CONTEXT_W
                                };
                                mid = want.clamp(x0 + 0.04, 0.56);
                            }
                        }
                        label(&mut out, name, [x0, y, mid, y + 0.0396], &left);
                        label(&mut out, value, [mid, y, 0.668, y + 0.0396], &FONT_ROW);
                    }
                }
                help_panel(&mut out, NOTE_BOTTOM, true, NOTE_BOTTOM - help_top);
                back_only(&mut out);
            }
            Screen::Credits => {
                // the lines of each paragraph wrapped to the centre strip, scrolled a line at a time
                if let Some(f) = font {
                    let mut st = FONT_ROW;
                    st.align = crate::ui::ALIGN_LEFT;
                    let w = (CREDITS_X[1] - CREDITS_X[0]) * crate::ui::CONTEXT_W - 4.0;
                    let mut all: Vec<String> = Vec::new();
                    let mut before = 1;
                    for (l, _) in self.fields(sc) {
                        // a blank line parts the paragraphs of several lines; one-line ones follow each other
                        let part = wrap_lines(f, &l, w, &st);
                        if !all.is_empty() && (before > 1 || part.len() > 1) {
                            all.push(String::new());
                        }
                        before = part.len();
                        all.extend(part);
                    }
                    let pitch = row_pitch(&st) / crate::ui::CONTEXT_H;
                    let cap = (((CREDITS_BOTTOM - CREDITS_TOP) / pitch).floor() as usize).max(1);
                    self.credits_max.set(all.len().saturating_sub(cap));
                    let from = self.credits_top.min(self.credits_max.get());
                    let shown = from..(from + cap).min(all.len());
                    for (k, i) in shown.clone().enumerate() {
                        let y = CREDITS_TOP + k as f32 * pitch;
                        if let Some(t) = all.get(i) {
                            let m = 2.0 / crate::ui::CONTEXT_H;
                            label(
                                &mut out,
                                t,
                                [CREDITS_X[0], y - m, CREDITS_X[1], y + pitch + m],
                                &st,
                            );
                        }
                    }
                    scroll(
                        &mut out,
                        CREDITS_X[1] + 0.004,
                        CREDITS_TOP,
                        CREDITS_BOTTOM,
                        shown,
                        all.len(),
                    );
                }
                back_only(&mut out);
            }
            Screen::Ask(_) => {
                if let Some(t) = self.title(sc) {
                    let mut st = FONT_01;
                    st.flags |= crate::ui::TEXT_WORDWRAP;
                    label(&mut out, &t, [0.35, 0.30, 0.65, 0.50], &st);
                }
                let mut column = Vec::new();
                for (i, r) in [YES_RECT, NO_RECT].into_iter().enumerate() {
                    button(&mut out, r, YES_NO_UV, state(i), Some(Target::Line(i)));
                    column.push((self.label(lines[i]), r, &FONT_03, 0.0));
                }
                label_group(&mut out, &column);
                help_panel(&mut out, 0.60, false, NOTE_BOTTOM - 0.60);
            }
            Screen::Entry => {
                if let Some(t) = self.title(sc) {
                    label(&mut out, &t, [0.35, 0.30, 0.65, 0.40], &FONT_01);
                }
                let fields = self.fields(sc);
                if let Some((l, p)) = fields.first() {
                    label(
                        &mut out,
                        &format!("{l}: {p}"),
                        [0.38, 0.43133, 0.62, 0.50133],
                        &FONT_01,
                    );
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
                    Some(Target::Line(0)),
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

    /// The loading screen in the interface's face: with the own interface a band behind it (the menu scene is
    /// drawn under it).
    pub fn loading_interface(
        face: &crate::ui::Face,
        screen: crate::ui::Screen,
        strings: &Strings,
        what: &str,
    ) -> Vec<crate::ui::Quad> {
        use crate::ui::{Anchor, Quad, homogeneous};
        let mut out = Vec::new();
        if face.own() {
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

/// A label as drawn, for the layout tests: its text, its frame and its glyphs' rectangles (window pixels), the size
/// it was drawn at against its style's, and whether it is a paragraph (word-wrapped).
#[derive(Clone, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct Probe {
    pub text: String,
    pub frame: [f32; 4],
    pub quads: Vec<[f32; 4]>,
    pub scale: f32,
    pub wrap: bool,
    /// A line's height at the size drawn (window pixels).
    pub line: f32,
    /// On a button with a ball at its left end: where the ball ends (window pixels); the glyphs stay right of it.
    pub clear_left: Option<f32>,
    /// The size the text was drawn at, a line's height in the font's units (compared with MIN_TEXT).
    pub size: f32,
    /// On a note: the panel it is drawn on (window pixels), the glyphs inside it and the panel clear of every button.
    pub panel: Option<[f32; 4]>,
    /// On a record of a button (no text): where the button's art lets text end, window x (its `frame` is the button).
    pub art_right: Option<f32>,
    /// The text lost its middle to fit (".."): the layout test refuses that for the game's own texts.
    pub cut: bool,
    /// On a note: the bounding box of its dark fill (window pixels), which must lie inside the panel's rim.
    pub fill: Option<[f32; 4]>,
    /// The labels of one column of buttons (one kind, one screen) share a group (nonzero) and one size.
    pub group: u32,
}

thread_local! {
    /// Whether the last `fit_label` shortened its text (read by the layout probe).
    static LAST_CUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// A size and form every label of a column is laid out at (`fit_label` obeys it): the size against the
    /// style's, and whether on two lines (where a text has a place to break).
    static FORCE: std::cell::Cell<Option<(f32, bool)>> = const { std::cell::Cell::new(None) };
    /// What the last `fit_label` could do: the size on one line, on two lines (None for one word), both at most 1.
    static CAND: std::cell::Cell<(f32, Option<f32>)> = const { std::cell::Cell::new((1.0, None)) };
}

/// The smallest text the menus draw (a style's y scale, as FONT_ROW's 0.34 and FONT_01's 0.55 are): a label is
/// shortened rather than drawn smaller, a paragraph is cut rather than drawn smaller.
pub const MIN_TEXT: f32 = 0.30;
/// The smallest a one-line label is drawn to fit, against its style's size; below it the label loses its middle.
const SHRINK: f32 = 0.6;
/// A label is drawn smaller down to this before two lines are tried, and two lines are drawn only this large or more
/// (so a low button never holds two tiny lines).
const FIT_SHRINK: f32 = 0.75;

/// The height of a line of `style` in context pixels: the font's cell, a sixteenth of the size (ui.rs, fallback.rs).
fn line_height(style: &crate::ui::Style) -> f32 {
    let base = if style.flags & crate::ui::TEXT_SCREEN != 0 {
        crate::ui::CONTEXT_H
    } else {
        512.0
    };
    style.scale[1] * base / 16.0
}

/// The smallest a style is drawn at, against its own size: MIN_TEXT, and a big label no further than SHRINK.
fn floor_k(style: &crate::ui::Style) -> f32 {
    (MIN_TEXT / style.scale[1].max(1e-3)).clamp(SHRINK, 1.0)
}

/// `style` drawn `k` times its size, one line at a time.
fn scaled_style(style: &crate::ui::Style, k: f32) -> crate::ui::Style {
    let mut st = *style;
    st.scale = [st.scale[0] * k, st.scale[1] * k];
    st.flags &= !crate::ui::TEXT_WORDWRAP;
    st
}

/// The width of glyph quads.
fn quads_wide(q: &[crate::ui::Quad]) -> f32 {
    if q.is_empty() {
        return 0.0;
    }
    q.iter().map(|q| q.rect[2]).fold(f32::MIN, f32::max)
        - q.iter().map(|q| q.rect[0]).fold(f32::MAX, f32::min)
}

/// How wide `t` is drawn in `style` on one line (context pixels).
fn text_wide(f: &crate::ui::Face, t: &str, style: &crate::ui::Style) -> f32 {
    quads_wide(&f.layout(t, [0.0, 0.0, 4000.0, 100.0], &scaled_style(style, 1.0)))
}

/// A folder as the player knows it: the home folder written `~` (Windows: `%USERPROFILE%`, and the roaming data
/// folder `%APPDATA%`), the rest as it is. A test may put a long path in its place.
fn shown_path(path: &str) -> String {
    #[cfg(test)]
    if let Some(p) = TEST_PATH.with(|p| p.borrow().clone()) {
        return p;
    }
    let env = |k: &str| std::env::var(k).ok().filter(|v| v.len() > 1);
    let under = |base: Option<String>, name: &str| {
        let base = base?;
        let base = base.trim_end_matches(['/', '\\']);
        let rest = path.strip_prefix(base)?;
        (rest.is_empty() || rest.starts_with(['/', '\\'])).then(|| format!("{name}{rest}"))
    };
    under(env("APPDATA"), "%APPDATA%")
        .or_else(|| under(env("HOME"), "~"))
        .or_else(|| under(env("USERPROFILE"), "%USERPROFILE%"))
        .unwrap_or_else(|| path.to_owned())
}

#[cfg(test)]
thread_local! {
    static TEST_PATH: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    /// A player's pack or theme name the Extended screen shows in place of the chosen one's.
    static TEST_NAME: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Whether a word is a folder path: it has a separator.
fn is_path(word: &str) -> bool {
    !word.contains(' ') && word.contains(['/', '\\'])
}

/// The path of `text` (a line of it) with its middle shortened by one more segment (".../"), `None` when only its
/// first and last segments are left.
fn shorten_path(text: &str) -> Option<String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let line = lines.iter_mut().rev().find(|l| is_path(l))?;
    let sep = line.chars().find(|c| matches!(c, '/' | '\\'))?;
    let mark = format!("...{sep}");
    let mut segs: Vec<String> = line.split_inclusive(sep).map(str::to_owned).collect();
    let n = segs.len();
    match segs.iter().position(|s| *s == mark) {
        None if n > 3 => segs[n / 2] = mark,
        Some(m) if m + 2 < n => {
            segs.remove(m + 1);
        }
        Some(m) if m >= 2 => {
            segs.remove(m - 1);
        }
        _ => return None,
    }
    *line = segs.concat();
    Some(lines.join("\n"))
}

/// A line of text: where a long one may be broken (at spaces, between the characters of a script without any).
fn is_wide_char(c: char) -> bool {
    u32::from(c) >= 0x2E80
}

/// `text` broken into lines at most `w` wide in `style` (explicit newlines kept); a word longer than a line is cut.
fn wrap_lines(f: &crate::ui::Face, text: &str, w: f32, style: &crate::ui::Style) -> Vec<String> {
    let mut lines = Vec::new();
    for part in text.split('\n') {
        // the pieces: a word, or one character of a script without spaces
        let mut pieces: Vec<(String, bool)> = Vec::new();
        let mut word = String::new();
        for c in part.chars() {
            if c == ' ' {
                if !word.is_empty() {
                    pieces.push((std::mem::take(&mut word), true));
                }
            } else if is_wide_char(c) {
                if !word.is_empty() {
                    pieces.push((std::mem::take(&mut word), true));
                }
                pieces.push((c.to_string(), false));
            } else {
                word.push(c);
            }
        }
        if !word.is_empty() {
            pieces.push((word, true));
        }
        // a folder path breaks only after a separator, never inside a name
        let pieces: Vec<(String, bool)> = pieces
            .into_iter()
            .flat_map(|(p, spaced)| {
                if spaced && is_path(&p) {
                    let sep = p.chars().find(|c| matches!(c, '/' | '\\')).unwrap_or('/');
                    p.split_inclusive(sep)
                        .enumerate()
                        .map(|(i, seg)| (seg.to_owned(), i == 0))
                        .collect()
                } else {
                    vec![(p, spaced)]
                }
            })
            .collect();
        let mut cur = String::new();
        for (p, spaced) in pieces {
            let joined = if cur.is_empty() {
                p.clone()
            } else if spaced {
                format!("{cur} {p}")
            } else {
                format!("{cur}{p}")
            };
            if cur.is_empty() || text_wide(f, &joined, style) <= w {
                cur = joined;
                // a word wider than the line is cut where it fills it
                while text_wide(f, &cur, style) > w && cur.chars().count() > 1 {
                    let mut head = cur.clone();
                    let mut tail = Vec::new();
                    while text_wide(f, &head, style) > w && head.chars().count() > 1 {
                        tail.push(head.pop().unwrap_or(' '));
                    }
                    lines.push(head);
                    cur = tail.into_iter().rev().collect();
                }
            } else {
                lines.push(std::mem::take(&mut cur));
                cur = p;
            }
        }
        lines.push(cur);
    }
    lines
}

/// The height of a line's row in a paragraph (context pixels) at `style`.
fn row_pitch(style: &crate::ui::Style) -> f32 {
    line_height(style) * 1.15
}

/// `text` as lines at most `w` wide that stay within `h` (context pixels): at the style's size, else smaller down
/// to its floor; what still does not fit loses its last lines, the last one ending in "..". Returns the lines and
/// the size drawn at against the style's.
fn fit_lines(
    f: &crate::ui::Face,
    text: &str,
    w: f32,
    h: f32,
    style: &crate::ui::Style,
) -> (Vec<String>, f32) {
    let kmin = floor_k(style);
    let mut k = 1.0f32;
    let mut text = text.to_owned();
    loop {
        let st = scaled_style(style, k);
        let mut lines = wrap_lines(f, &text, w, &st);
        let room = (h / row_pitch(&st)).floor().max(1.0) as usize;
        if lines.len() <= room {
            return (lines, k);
        }
        if k <= kmin + 1e-4 {
            // at the least size a long folder path loses its middle, at a separator
            if let Some(shorter) = shorten_path(&text) {
                text = shorter;
                continue;
            }
            lines.truncate(room);
            if let Some(last) = lines.last_mut() {
                let mut t = last.trim_end().to_owned();
                while text_wide(f, &format!("{t}.."), &st) > w && t.chars().count() > 1 {
                    t.pop();
                }
                *last = format!("{t}..");
            }
            return (lines, k);
        }
        k = (k * 0.96).max(kmin);
    }
}

/// `text` as lines at most `w` wide, whole: at the style's size, else smaller down to its floor. Returns the lines,
/// the size drawn at and whether they stay within `h`; when they do not, all of them still come back (never cut).
fn fit_whole(
    f: &crate::ui::Face,
    text: &str,
    w: f32,
    h: f32,
    style: &crate::ui::Style,
) -> (Vec<String>, f32, bool) {
    let kmin = floor_k(style);
    let mut k = 1.0f32;
    loop {
        let st = scaled_style(style, k);
        let lines = wrap_lines(f, text, w, &st);
        let room = (h / row_pitch(&st)).floor().max(1.0) as usize;
        if lines.len() <= room {
            return (lines, k, true);
        }
        if k <= kmin + 1e-4 {
            return (lines, k, false);
        }
        k = (k * 0.96).max(kmin);
    }
}

/// A word-wrapped paragraph in `frame` (context pixels, the style's margins kept): the lines centred, as a block, in
/// the frame; smaller down to the style's floor, then cut. Returns the glyphs and the size drawn at.
fn paragraph(
    f: &crate::ui::Face,
    t: &str,
    frame: [f32; 4],
    style: &crate::ui::Style,
) -> (Vec<crate::ui::Quad>, f32) {
    let m = style.margins;
    let (w, h) = (
        frame[2] - frame[0] - m[0] - m[2],
        frame[3] - frame[1] - m[1] - m[3],
    );
    if w <= 0.0 || h <= 0.0 || t.trim().is_empty() {
        return (Vec::new(), 1.0);
    }
    let (lines, k) = fit_lines(f, t, w, h, style);
    let st = scaled_style(style, k);
    let pitch = row_pitch(&st);
    let top = (frame[1] + frame[3]) / 2.0 - pitch * lines.len() as f32 / 2.0;
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let y = top + i as f32 * pitch;
        out.extend(f.layout(l, [frame[0], y, frame[2], y + pitch], &st));
    }
    (out, k)
}

/// `t` laid out to stay inside `frame` (context pixels, the style's margins kept): as it is when it fits; else a
/// little smaller (down to the style's floor, MIN_TEXT); else on two lines when the frame is tall enough for two at
/// that size or more; else shortened in its middle (".."). A word-wrapped paragraph is `paragraph`. Returns the
/// glyphs and the size drawn at against the style's.
fn fit_label(
    f: &crate::ui::Face,
    t: &str,
    frame: [f32; 4],
    style: &crate::ui::Style,
) -> (Vec<crate::ui::Quad>, f32) {
    use crate::ui::Quad;
    LAST_CUT.with(|c| c.set(false));
    let m = style.margins;
    let avail_w = frame[2] - frame[0] - m[0] - m[2];
    let avail_h = frame[3] - frame[1] - m[1] - m[3];
    let line = line_height(style);
    if avail_w <= 0.0 || avail_h <= 0.0 || line <= 0.0 || t.trim().is_empty() {
        return (Vec::new(), 1.0);
    }
    if style.flags & crate::ui::TEXT_WORDWRAP != 0 {
        return paragraph(f, t, frame, style);
    }
    let wide = |q: &[Quad]| quads_wide(q);
    let scaled = |k: f32| scaled_style(style, k);
    let kmin = floor_k(style);
    let fit_min = kmin.max(FIT_SHRINK);
    let one = f.layout(t, frame, style);
    let w = wide(&one);
    let fits_h = (avail_h / line).min(1.0);
    let k1 = if w > 0.0 {
        (avail_w / w).min(fits_h)
    } else {
        fits_h
    };
    let split = two_lines(t).map(|(a, b)| {
        let wa = wide(&f.layout(&a, frame, style));
        let wb = wide(&f.layout(&b, frame, style));
        let k2 = (avail_w / wa.max(wb).max(1e-3))
            .min(avail_h / (2.0 * line))
            .min(1.0);
        (a, b, k2)
    });
    CAND.with(|c| c.set((k1.min(1.0), split.as_ref().map(|s| s.2))));
    if let Some((k, two)) = FORCE.with(|c| c.get()) {
        let st = scaled(k);
        let fitted = |s: &str| {
            let mut s = s.to_owned();
            while wide(&f.layout(&s, frame, &st)) > avail_w && s.chars().count() > 4 {
                s = shorten(&s);
                LAST_CUT.with(|c| c.set(true));
            }
            s
        };
        return match split {
            Some((a, b, _)) if two => (
                f.layout(&format!("{}\n{}", fitted(&a), fitted(&b)), frame, &st),
                k,
            ),
            _ => (f.layout(&fitted(t), frame, &st), k),
        };
    }
    if k1 >= 1.0 {
        return (one, 1.0);
    }
    if k1 >= fit_min {
        return (f.layout(t, frame, &scaled(k1)), k1);
    }
    // two lines, each at most the frame's width, both within its height
    if let Some((a, b, k2)) = split {
        if k2 >= fit_min && k2 > k1 {
            return (f.layout(&format!("{a}\n{b}"), frame, &scaled(k2)), k2);
        }
    }
    // one line, as small as allowed, and what still does not fit loses its middle
    let k = k1.max(kmin).min(fits_h);
    let st = scaled(k);
    let mut text = t.to_owned();
    let mut q = f.layout(&text, frame, &st);
    while wide(&q) > avail_w && text.chars().count() > 4 {
        text = shorten(&text);
        LAST_CUT.with(|c| c.set(true));
        q = f.layout(&text, frame, &st);
    }
    (q, k)
}

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
        Some(at) => (
            chars.iter().take(at).collect(),
            chars.iter().skip(at + 1).collect(),
        ),
        None if chars.iter().any(|&c| u32::from(c) >= 0x2E80) && chars.len() > 1 => (
            chars.iter().take(mid).collect(),
            chars.iter().skip(mid).collect(),
        ),
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
/// Where the art of each button ends for text, as a fraction of the button's width from its left: the rounded or
/// bracketed right end (measured on Button01_*: the art stops short of its rect, and the end caps hold a bracket).
fn text_right(uv: [f32; 4]) -> f32 {
    if uv == MAIN_UV {
        0.92
    } else if uv == BACK_UV {
        0.86
    } else if uv == FIELD_UV {
        0.93
    } else if uv == LEVEL_UV {
        0.94
    } else if uv == YES_NO_UV {
        0.94
    } else {
        0.96
    }
}
const BACK_RECT: [f32; 4] = [0.4031, 0.85, 0.5906, 0.9438];
/// The width of the ball at the left end of a big button or a field (MAIN_UV, FIELD_UV), and of Back's (BACK_UV),
/// measured on the drawn buttons: a label stays right of it.
const BALL_MAIN: f32 = 0.058;
const BALL_BACK: f32 = 0.042;
/// Where the key row's bar is cut to leave its middle mark out (texture u of the left end's end, the right's start).
const ROW_CUT: [f32; 2] = [0.46, 0.58];
/// A texture no interface has: drawn as plain white, tinted (the scroll bar).
const PLAIN: &str = "plain";
/// Our list screens' rows: their height and spacing, the custom levels' and the mods' first row, their left and
/// right edges, the scroll bar's width and the hint's height (fractions of the 4:3 frame).
const ROW_H: f32 = 0.036;
const ROW_STEP: f32 = 0.042;
const LIST_TOP: f32 = 0.27;
const LIST_X: [f32; 2] = [0.315, 0.67];
const MODS_TOP: f32 = 0.26;
const ROWS_X: [f32; 2] = [0.32, 0.67];
const SCROLL_W: f32 = 0.008;
/// A row's label begins after the ball at the row's left end.
const ROW_INDENT: f32 = 0.042;
const HINT_H: f32 = 0.032;
/// The help under the Mods screen and a mod's page: a paragraph above Back.
/// The notes' panels: at most this wide (inside the dark centre strip), their bottom above Back.
const NOTE_X: [f32; 2] = [0.31, 0.69];
const NOTE_BOTTOM: f32 = 0.838;
/// A note panel's padding (context pixels): sideways, up and down.
const NOTE_PAD: [f32; 2] = [12.0, 7.0];
/// The credits' text: its left and right edges (the bar right of them), its top and bottom.
const CREDITS_X: [f32; 2] = [0.318, 0.672];
const CREDITS_TOP: f32 = 0.10;
const CREDITS_BOTTOM: f32 = 0.815;
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
/// Our list screens' rows (the Extended page, the mods, a mod's page, the custom levels): a size smaller than the
/// original's small font, so the longest labels (German, Russian, French) fit and more rows show; the help under
/// them and the scroll hints smaller still.
const FONT_ROW: crate::ui::Style = font_style([0.3, 0.34], WHITE);
const FONT_HELP: crate::ui::Style = font_style([0.3, 0.34], [0.85, 0.85, 0.85, 1.0]);
const FONT_HINT: crate::ui::Style = font_style([0.3, 0.34], [0.8, 0.8, 0.8, 1.0]);
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

/// Level `number` was finished once: a finished run of it is kept (`Level_NN-<ticks>.replay`, or the best). A run
/// played with mods that change the game (`...-modded.replay`) opens no level; the debug mode's "All levels" does.
fn finished(number: u32) -> bool {
    let prefix = format!("Level_{number:02}-");
    std::fs::read_dir(runs_dir(number)).is_ok_and(|d| {
        d.flatten().any(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n == "best.replay"
                || (n.starts_with(&prefix)
                    && n.ends_with(".replay")
                    && !n.ends_with("-modded.replay"))
        })
    })
}

/// How long a held arrow or page key waits before it repeats, and how fast it repeats then (rows a second, more
/// after a second of repeating).
pub const REPEAT_DELAY: f32 = 0.35;
const REPEAT_SLOW: f32 = 12.0;
const REPEAT_FAST: f32 = 30.0;
const REPEAT_FASTER_AT: f32 = 1.35;

/// The menu's key repeat: the platform marks no repeats for us, so a held Up, Down, PageUp or PageDown is counted
/// here from the time it has been down.
#[derive(Default)]
pub struct KeyRepeat {
    held: Option<(winit::keyboard::KeyCode, f32, u32)>,
}

impl KeyRepeat {
    /// A key went down: the repeatable ones start counting.
    pub fn press(&mut self, code: winit::keyboard::KeyCode) {
        use winit::keyboard::KeyCode as K;
        self.held = matches!(code, K::ArrowUp | K::ArrowDown | K::PageUp | K::PageDown)
            .then_some((code, 0.0, 0));
    }

    /// A key went up.
    pub fn release(&mut self, code: winit::keyboard::KeyCode) {
        if self.held.is_some_and(|h| h.0 == code) {
            self.held = None;
        }
    }

    /// Nothing is held (the menu closed).
    pub fn clear(&mut self) {
        self.held = None;
    }

    /// `dt` seconds passed: the held key and how many times it repeats in them.
    pub fn advance(&mut self, dt: f32) -> Option<(winit::keyboard::KeyCode, u32)> {
        let (code, t, done) = self.held.as_mut()?;
        *t += dt;
        let slow = (t.min(REPEAT_FASTER_AT) - REPEAT_DELAY).max(0.0) * REPEAT_SLOW;
        let fast = (*t - REPEAT_FASTER_AT).max(0.0) * REPEAT_FAST;
        let total = if *t < REPEAT_DELAY {
            0
        } else {
            1 + (slow + fast) as u32
        };
        let due = total.saturating_sub(*done);
        *done = total;
        (due > 0).then_some((*code, due))
    }
}

/// The screens whose lines scroll as a list above their Back.
fn scrolls(screen: Screen) -> bool {
    matches!(screen, Screen::Custom | Screen::Mods | Screen::ModPage)
}

/// The rows a screen's list shows at once.
fn page(screen: Screen) -> usize {
    match screen {
        Screen::Mods | Screen::ModPage => MODS_LINES,
        _ => LIST_LINES,
    }
}

/// The switch lines of the extended mode's additions offered (a debug-only one in a debug run alone).
fn switches(features: Features) -> impl Iterator<Item = Line> {
    FEATURES
        .iter()
        .enumerate()
        .filter(move |(_, f)| features.offered(**f))
        .map(|(i, _)| Line::Switch(i))
}

/// The interface's provider after the chosen one, `by` steps on (the arrows go round the three).
fn next_provider(by: i32) -> gamedata::resources::Provider {
    use gamedata::resources::{Provider, chosen};
    let all = Provider::ALL;
    // only the ones usable here: without the copy (or its remaster) they are not offered
    let ok: Vec<Provider> = all
        .into_iter()
        .filter(|&p| gamedata::resources::usable(p))
        .collect();
    let at = ok.iter().position(|&p| p == chosen()).unwrap_or(0) as i32;
    let n = ok.len().max(1) as i32;
    ok.get((at + by).rem_euclid(n) as usize)
        .copied()
        .unwrap_or(Provider::Own)
}

/// The pack of a kind `by` steps after the chosen one (the arrows go round the packs found).
fn next_pack(kind: gamedata::packs::Kind, by: i32) -> Action {
    Action::Pack(kind, gamedata::packs::next(kind, by))
}

/// The sound theme after the chosen one, `by` steps on (the arrows go round the themes found and "original").
fn next_theme(by: i32) -> String {
    use gamedata::sound_themes::{SYSTEM, choices, chosen};
    let all = choices();
    let at = all.iter().position(|k| *k == chosen()).unwrap_or(0) as i32;
    let n = all.len().max(1) as i32;
    all.get((at + by).rem_euclid(n) as usize)
        .cloned()
        .unwrap_or_else(|| SYSTEM.to_owned())
}

/// The Sounds line's value: the chosen theme's key (its folder's name).
fn theme_value(s: &Strings) -> String {
    let key = gamedata::sound_themes::chosen();
    #[cfg(test)]
    if let Some(n) = TEST_NAME.with(|n| n.borrow().clone()) {
        return n;
    }
    if key == gamedata::sound_themes::ORIGINAL {
        s.ours("interface original")
    } else {
        key
    }
}

const ALL_LOOKS: [crate::look::Look; 4] = crate::look::ALL;

impl Menu {
    /// The Look line's value: the look chosen, in the menus' language.
    fn look_value(&self) -> String {
        self.strings
            .ours(&format!("look {}", crate::look::chosen().key()))
    }

    /// The Textures or Models line's value: the chosen pack's name in the menus' language (a built-in pack's in ours).
    fn pack_value(&self, kind: gamedata::packs::Kind) -> String {
        use gamedata::packs::{LEVEL, ORIGINAL, OWN, REMASTERED, chosen, packs};
        let key = chosen(kind);
        #[cfg(test)]
        if let Some(n) = TEST_NAME.with(|n| n.borrow().clone()) {
            return n;
        }
        match key.as_str() {
            OWN => self.strings.ours("interface own"),
            ORIGINAL => self.strings.ours("interface original"),
            REMASTERED => self.strings.ours("interface remaster"),
            LEVEL => self.strings.ours("models level"),
            _ => packs(kind)
                .iter()
                .find(|p| p.key == key)
                .map_or(key.clone(), |p| p.name(self.strings.language())),
        }
    }

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

/// Which list the Custom screen shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum List {
    None,
    /// Drahtseilakt: our own levels.
    Ours,
    /// The best community maps (shipped).
    Best,
    /// The player's downloaded levels.
    Downloaded,
    /// Ballance Challenges.
    Challenges,
}

/// The levels of a custom levels folder by title: the level folders in it and the `.nmo` files dropped loose into
/// it; folders that hold no level or several are not shown, and said so on the console.
/// The player's downloaded levels: the downloaded folder's and those of the folders it replaced (community, open),
/// each read where it is, in one list.
fn downloaded_levels() -> Vec<Package> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut v = Vec::new();
    for kind in gamedata::profile::DOWNLOADED {
        let root = crate::levels::dir(kind);
        if !seen.contains(&root) {
            v.extend(maps_in(&root));
            seen.push(root);
        }
    }
    gamedata::package::sort(&mut v);
    v
}

/// The levels of a custom levels folder by title: the level folders in it and the `.nmo` files dropped loose into
/// it; folders that hold no level or several are not shown, and said so on the console.
fn maps_in(dir: &Path) -> Vec<Package> {
    let (levels, skipped) = gamedata::package::scan(dir);
    if !skipped.is_empty() {
        let shown = gamedata::package::printable(&dir.to_string_lossy());
        eprintln!(
            "{shown}: {} folders are not levels (a level folder holds one level):",
            skipped.len()
        );
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
        // by title, a level named by level.txt before one named by its file; no folders to open, nested levels and
        // junk not shown
        let titles: Vec<String> = (0..m.maps.len()).map(|i| m.label(Line::Map(i))).collect();
        assert_eq!(titles, ["Alpha Road", "Alone"]);
        assert_eq!(m.enter(), Action::Map(root.join("Zeta/Level_07.NMO")));
        m.set_chosen(1);
        assert_eq!(m.enter(), Action::Map(root.join("Alone.NMO")));
        // Esc leaves the screen at once
        m.back();
        assert_ne!(m.screen(), Screen::Custom);
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn menu(open: &[bool], extended: bool) -> Menu {
        let mut m = Menu {
            help_force: std::cell::Cell::new(None),
            note_rows: std::cell::Cell::new(LIST_LINES),
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
            list: List::None,
            has_ours: true,
            has_best: true,
            copy: true,
            modified: false,
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
            mods: crate::modrt::Mods::default(),
            mod_page: 0,
            top: 0,
            credits_top: 0,
            credits_max: std::cell::Cell::new(0),
        };
        m.push(Screen::Main);
        m
    }

    #[test]
    fn the_mods_screen_switches_mods_and_steps_their_options() {
        let mut m = menu(&[true], true);
        let text = "name.en Fov\noption strength number 10 100 10 50\nlabel strength en Strength\non frame\ndo camera.fov opt.strength\n";
        let found = vec![
            gamedata::mods::Mod {
                key: "fov".into(),
                factory: true,
                parsed: gamedata::mods::parse_mod_txt(text),
            },
            gamedata::mods::Mod {
                key: "bad".into(),
                factory: false,
                parsed: Err("no mod.txt".into()),
            },
        ];
        m.mods = crate::modrt::Mods::load(found, |_| None);
        m.push(Screen::Extended);
        let at = m
            .lines(Screen::Extended)
            .iter()
            .position(|&l| l == Line::ModsMenu)
            .unwrap();
        m.set_chosen(at);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Mods);
        let lines = m.lines(Screen::Mods);
        assert!(
            !lines.iter().any(|l| matches!(l, Line::Switch(_))),
            "the practice is god-mode's now, a mod"
        );
        let fov = lines.iter().position(|&l| l == Line::Mod(0)).unwrap();
        m.set_chosen(fov);
        assert_eq!(m.adjust(1), Action::ModSwitch(0));
        let fields = m.fields(Screen::Mods);
        assert_eq!(fields[fov].0, "Fov");
        assert_eq!(fields[fov + 1].1, "refused", "a refused mod says so");
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::ModPage);
        m.set_chosen(1);
        assert_eq!(m.label(Line::ModOpt(0)), "Strength");
        assert_eq!(m.adjust(-1), Action::ModOption(0, 0, -1));
        let _ = m.interface(None, crate::ui::Screen { w: 640.0, h: 480.0 });
        let _ = m.overlay(640, 480);
    }

    /// Window sizes of every shape: the original's, wide, ultra-wide, portrait, tiny.
    const WINDOWS: [(f32, f32); 5] = [
        (640.0, 480.0),
        (1920.0, 1080.0),
        (3440.0, 1080.0),
        (1080.0, 1920.0),
        (320.0, 200.0),
    ];

    fn at(m: &Menu, interface: bool, w: f32, h: f32, target: Target, fx: f32) -> Pointer {
        let screen = crate::ui::Screen { w, h };
        let hit = m
            .hits(interface, screen)
            .into_iter()
            .find(|h| h.target == target)
            .unwrap();
        let r = hit.rect;
        Pointer {
            interface,
            screen,
            x: r[0] + (r[2] - r[0]) * fx,
            y: (r[1] + r[3]) / 2.0,
        }
    }

    fn line_of(m: &Menu, l: Line) -> Target {
        Target::Line(m.lines(m.screen()).iter().position(|&x| x == l).unwrap())
    }

    #[test]
    fn the_buttons_are_hit_where_they_are_drawn_in_any_window() {
        let m = menu(&[true, true], true);
        for (w, h) in WINDOWS {
            let screen = crate::ui::Screen { w, h };
            let hits = m.hits(true, screen);
            // one per line, each exactly a button the interface draws
            assert_eq!(hits.len(), m.lines(Screen::Main).len());
            let quads = m.interface(None, screen);
            for hit in &hits {
                assert!(quads.iter().any(|q| q.rect == hit.rect), "{w}x{h}: {hit:?}");
                assert!(
                    hit.rect[0] >= 0.0
                        && hit.rect[2] <= w
                        && hit.rect[1] >= 0.0
                        && hit.rect[3] <= h
                );
            }
            // a click on Options opens it, wherever the window puts it
            let mut m = menu(&[true, true], true);
            let p = at(&m, true, w, h, line_of(&m, Line::Options), 0.5);
            assert_eq!(m.click(p), Some(Action::None));
            assert_eq!(m.screen(), Screen::Options);
            // the same with the prototype's lines
            let mut m = menu(&[true, true], true);
            let p = at(&m, false, w, h, line_of(&m, Line::Credits), 0.5);
            assert_eq!(m.click(p), Some(Action::None));
            assert_eq!(m.screen(), Screen::Credits);
        }
    }

    #[test]
    fn hovering_chooses_a_line_and_nothing_else() {
        let mut m = menu(&[true, false, true], false);
        m.push(Screen::Start);
        let level2 = at(&m, true, 1920.0, 1080.0, Target::Line(1), 0.5);
        let level3 = at(&m, true, 1920.0, 1080.0, Target::Line(2), 0.5);
        // a locked level is not chosen, an open one is, and once chosen it says nothing changed
        assert!(!m.hover(level2));
        assert!(m.hover(level3));
        assert_eq!(m.chosen(), 2);
        assert!(!m.hover(level3));
        assert_eq!(m.click(level2), None);
        assert_eq!(m.click(level3), Some(Action::Play(3)));
        // between the buttons nothing is hit
        let gap = Pointer {
            x: 5.0,
            y: 5.0,
            ..level3
        };
        assert!(!m.hover(gap));
        assert_eq!(m.click(gap), None);
    }

    #[test]
    fn a_click_on_a_value_goes_down_on_its_left_half_and_up_on_its_right() {
        let mut m = menu(&[true], true);
        m.push(Screen::Options);
        m.push(Screen::Sound);
        m.volumes = [0.5, 0.5];
        let music = line_of(&m, Line::Music);
        assert_eq!(
            m.click(at(&m, true, 1600.0, 900.0, music, 0.2)),
            Some(Action::Volumes([0.5, 0.4]))
        );
        assert_eq!(
            m.click(at(&m, true, 1600.0, 900.0, music, 0.8)),
            Some(Action::Volumes([0.5, 0.5]))
        );
        m.back();
        m.push(Screen::Graphics);
        let fov = m.graphics.fov;
        let p = at(&m, true, 1600.0, 900.0, line_of(&m, Line::Fov), 0.9);
        assert!(graphics(m.click(p).unwrap()).unwrap().fov > fov);
        let p = at(&m, true, 1600.0, 900.0, line_of(&m, Line::Fov), 0.1);
        assert_eq!(graphics(m.click(p).unwrap()).unwrap().fov, fov);
        // a switch flips on either half
        let synch = m.graphics.vsync;
        let p = at(&m, true, 1600.0, 900.0, line_of(&m, Line::Synch), 0.1);
        assert_eq!(graphics(m.click(p).unwrap()).unwrap().vsync, !synch);
        // the resolution goes through the sizes
        let p = at(&m, true, 1600.0, 900.0, line_of(&m, Line::Resolution), 0.9);
        assert_eq!(
            graphics(m.click(p).unwrap()).unwrap().resolution,
            Some((800, 600))
        );
        // Back is a button like any other
        let p = at(&m, true, 1600.0, 900.0, line_of(&m, Line::Back), 0.5);
        m.click(p);
        assert_eq!(m.screen(), Screen::Options);
    }

    #[test]
    fn the_highscores_arrows_change_the_level() {
        let mut m = menu(&[true, true, true], false);
        m.push(Screen::Highscore);
        let right = at(&m, true, 800.0, 600.0, Target::Arrow(1), 0.5);
        m.click(right);
        assert_eq!(m.shown, 2);
        let left = at(&m, true, 800.0, 600.0, Target::Arrow(-1), 0.5);
        m.click(left);
        m.click(left);
        assert_eq!(m.shown, 3);
        // the wheel too
        m.wheel(1, None);
        assert_eq!(m.shown, 1);
    }

    #[test]
    fn a_click_on_a_key_waits_for_a_key_and_no_mouse_button_is_bound() {
        let mut m = menu(&[true], false);
        m.push(Screen::Options);
        m.push(Screen::Controls);
        let forward = at(&m, true, 1280.0, 1024.0, Target::Line(0), 0.5);
        let left = at(&m, true, 1280.0, 1024.0, Target::Line(2), 0.5);
        assert_eq!(m.click(left), Some(Action::None));
        assert!(m.waiting());
        // the cursor and more clicks leave the wait alone; Esc (the right button) gives it up
        assert!(!m.hover(forward));
        assert_eq!(m.click(forward), None);
        assert_eq!(m.chosen(), 2);
        m.back();
        assert!(!m.waiting());
        assert_eq!(m.screen(), Screen::Controls);
    }

    #[test]
    fn yes_and_no_are_clicked() {
        let mut m = menu(&[true], false);
        m.push(Screen::Ask(Ask::Quit));
        let no = at(&m, true, 1366.0, 768.0, line_of(&m, Line::No), 0.5);
        let yes = at(&m, true, 1366.0, 768.0, line_of(&m, Line::Yes), 0.5);
        assert_eq!(m.click(yes), Some(Action::Quit));
        m.click(no);
        assert_eq!(m.screen(), Screen::Main);
    }

    #[test]
    fn a_held_key_repeats_after_a_delay_then_faster() {
        use winit::keyboard::KeyCode as K;
        let mut r = KeyRepeat::default();
        r.press(K::ArrowDown);
        assert_eq!(r.advance(0.30), None, "nothing before the delay");
        assert_eq!(
            r.advance(0.06),
            Some((K::ArrowDown, 1)),
            "the first repeat at 0.35 s"
        );
        let mut n = 1;
        for _ in 0..60 {
            n += r.advance(1.0 / 60.0).map_or(0, |h| h.1);
        }
        // a second on: 12 rows a second until 1.35 s
        assert!((12..=14).contains(&n), "{n}");
        let before = n;
        for _ in 0..60 {
            n += r.advance(1.0 / 60.0).map_or(0, |h| h.1);
        }
        assert!(n - before >= 25, "faster after a second: {}", n - before);
        r.release(K::ArrowDown);
        assert_eq!(r.advance(1.0), None);
        r.press(K::KeyA);
        assert_eq!(r.advance(1.0), None, "a letter does not repeat");
    }

    #[test]
    fn pages_ends_letters_and_back_work_on_a_long_list() {
        let mut m = menu(&[true], true);
        m.maps = (0..100)
            .map(|i| Package {
                level: PathBuf::from(format!("{i}.nmo")),
                folder: PathBuf::from(format!("{i}.nmo")),
                title: if i == 60 {
                    "Zebra".to_owned()
                } else {
                    format!("Level {i}")
                },
                author: None,
                version: None,
                posted: None,
                report: None,
                titles: Vec::new(),
                order: None,
                hidden: false,
                difficulty: None,
                titled: true,
            })
            .collect();
        m.push(Screen::Custom);
        m.page_by(1);
        assert_eq!(m.chosen(), LIST_LINES);
        m.page_by(1);
        assert_eq!(m.chosen(), 2 * LIST_LINES);
        m.page_by(-1);
        assert_eq!(m.chosen(), LIST_LINES);
        m.edge(1);
        assert_eq!(m.chosen(), 99, "End: the last row, not Back");
        assert_eq!(m.shown(100).end, 100);
        m.edge(-1);
        assert_eq!(m.chosen(), 0);
        assert_eq!(m.shown(100).start, 0);
        m.letter("z");
        assert_eq!(m.chosen(), 60);
        assert!(m.shown(100).contains(&60));
        // Esc from a row deep in the list goes back at once
        assert_eq!(m.back(), Action::None);
        assert_ne!(m.screen(), Screen::Custom);
    }

    #[test]
    fn back_stays_pinned_under_a_scrolling_list() {
        let mut m = menu(&[true], true);
        m.maps = (0..100)
            .map(|i| Package {
                level: PathBuf::from(format!("{i}.nmo")),
                folder: PathBuf::from(format!("{i}.nmo")),
                title: format!("Level {i}"),
                author: None,
                version: None,
                posted: None,
                report: None,
                titles: Vec::new(),
                order: None,
                hidden: false,
                difficulty: None,
                titled: true,
            })
            .collect();
        m.push(Screen::Custom);
        let screen = crate::ui::Screen {
            w: 1920.0,
            h: 1080.0,
        };
        for _ in 0..3 {
            m.page_by(1);
            let rows: Vec<usize> = m
                .hits(true, screen)
                .iter()
                .filter_map(|h| {
                    if let Target::Line(i) = h.target {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(rows.last(), Some(&100), "Back is drawn after every scroll");
        }
    }

    #[test]
    fn a_long_list_keeps_still_under_the_cursor_and_the_wheel_scrolls_it() {
        let mut m = menu(&[true], true);
        m.maps = (0..30)
            .map(|i| Package {
                level: PathBuf::from(format!("{i}.nmo")),
                folder: PathBuf::from(format!("{i}.nmo")),
                title: format!("Level number {i} with a rather long title"),
                author: None,
                version: None,
                posted: None,
                report: None,
                titles: Vec::new(),
                order: None,
                hidden: false,
                difficulty: None,
                titled: true,
            })
            .collect();
        m.push(Screen::Custom);
        let (w, h) = (2560.0, 1440.0);
        let rows = |m: &Menu| -> Vec<usize> {
            let screen = crate::ui::Screen { w, h };
            let line = |h: &Hit| {
                if let Target::Line(i) = h.target {
                    Some(i)
                } else {
                    None
                }
            };
            m.hits(true, screen).iter().filter_map(line).collect()
        };
        // a page of maps and Back
        assert_eq!(rows(&m), (0..LIST_LINES).chain([30]).collect::<Vec<_>>());
        // the cursor down the rows: the list does not move under it
        for i in [3, 11, 7] {
            assert!(m.hover(at(&m, true, w, h, Target::Line(i), 0.5)));
            assert_eq!(m.chosen(), i);
            assert_eq!(rows(&m)[0], 0);
        }
        // the wheel scrolls; the row under the cursor (the eighth shown) is chosen
        let p = at(&m, true, w, h, Target::Line(7), 0.5);
        m.wheel(1, Some(p));
        m.wheel(1, Some(p));
        assert_eq!(rows(&m)[0], 2);
        assert_eq!(m.chosen(), 9);
        // without the cursor on it the chosen line stays in view
        for _ in 0..40 {
            m.wheel(1, None);
        }
        let end = 30 - LIST_LINES;
        assert_eq!(rows(&m)[0], end);
        assert_eq!(m.chosen(), end);
        m.wheel(-1, None);
        assert_eq!(rows(&m)[0], end - 1);
        // the arrows still move the view only as far as the chosen line needs
        m.step(1);
        assert_eq!((m.chosen(), rows(&m)[0]), (end + 1, end - 1));
        assert_eq!(
            m.click(at(&m, true, w, h, Target::Line(20), 0.5)),
            Some(Action::Map(PathBuf::from("20.nmo")))
        );
    }

    /// Every screen and list the menu can show, in every language, at windows of several shapes, as the interface
    /// really draws it (the glyph quads of `draw` with the own font, not estimates). Each text: inside its frame and
    /// its button or row, right of the ball on a button, inside the dark centre strip and the window, at least
    /// MIN_TEXT (two lines only at FIT_SHRINK or more); a note: on a panel inside the strip that touches no button
    /// (Back included); no two buttons overlap. Lists are filled with the long real data: the thirteen community
    /// maps, the modified originals in every language, long titles, every mod, a long highscore name.
    #[test]
    fn every_text_of_every_screen_fits_in_every_language_and_window() {
        if !crate::fallback::available() {
            eprintln!("skipped: the shipped fonts (assets/fonts) are not there");
            return;
        }
        let face = crate::ui::Face::Own;
        let long = |i: usize| Package {
            level: PathBuf::from(format!("{i}.nmo")),
            folder: PathBuf::from(format!("{i}.nmo")),
            title: [
                "W".repeat(40),
                "超长的社区地图名称超长的社区地图名称超长的社区地图名称超长的社区地".to_owned(),
                "Der längste Titel einer Karte überhaupt!".to_owned(),
            ][i % 3]
                .clone(),
            author: Some("An Author With A Very Long Name Indeed".to_owned()),
            version: Some("12.3.4-beta".to_owned()),
            posted: Some("2012-03-01".to_owned()),
            report: Some(
                [
                    "script-block-unknown",
                    "animation-played",
                    "texture-missing",
                    "sound-unplayed",
                ]
                .map(|k| (k.to_owned(), 12345))
                .to_vec(),
            ),
            titles: Vec::new(),
            order: None,
            hidden: false,
            difficulty: if i % 2 == 0 { Some(10) } else { None },
            titled: true,
        };
        let community = crate::levels::community();
        assert_eq!(community.len(), 13, "the thirteen shipped community maps");
        let lists: Vec<(&str, List, Vec<Package>)> = vec![
            (
                "own",
                List::Ours,
                crate::levels::shipped()
                    .into_iter()
                    .chain((0..20).map(long))
                    .collect(),
            ),
            ("best", List::Best, community),
            ("downloaded", List::Downloaded, (0..20).map(long).collect()),
            (
                "challenges",
                List::Challenges,
                crate::modified::sample_packages(),
            ),
            ("none", List::Downloaded, Vec::new()),
        ];
        let mods = gamedata::mods::scan(true);
        assert!(
            mods.iter().any(|m| m.key == "god-mode" && m.parsed.is_ok()),
            "god-mode is read in a debug run"
        );
        let sizes = [
            (640.0, 480.0),
            (1024.0, 768.0),
            (1280.0, 720.0),
            (1920.0, 1080.0),
            (3440.0, 1440.0),
            (1080.0, 1920.0),
        ];
        let mut checked = 0;
        let mut seen_notes = 0;
        let (mut seen_hints, mut seen_paths) = (0, 0);
        // the folder note's path: long, so that it must wrap (at its separators only)
        TEST_PATH.with(|p| {
            *p.borrow_mut() = Some(
                "~/.local/share/drahtseilakt-with-a-long-profile/community-maps/levels/community"
                    .to_owned(),
            );
        });
        // mod texts shortened in their middle (the mod page's labels and values, the mods' names)
        let mut cut = std::collections::BTreeSet::new();
        for &(lang, _) in languages() {
            // a player's pack or theme with a name of the longest allowed (16 letters), of wide ones
            let wide = match lang {
                "ru" => "Щ".repeat(16),
                "zh" => "汉".repeat(16),
                _ => "W".repeat(16),
            };
            TEST_NAME.with(|n| *n.borrow_mut() = Some(wide));
            for mode in [true, false] {
                for (list, kind, maps) in &lists {
                    let mut m = menu(&[true; 13], mode);
                    m.features.debug = true;
                    m.strings = Strings::ours_in(lang);
                    m.mods = crate::modrt::Mods::load(mods.clone(), |_| None);
                    m.maps = maps.clone();
                    m.list = *kind;
                    m.modified = true;
                    m.name = b"WWWWWWWWWWWWWWWW".to_vec();
                    m.points = 99_999_999;
                    for level in 1..=13 {
                        for k in 0..10 {
                            m.scores
                                .insert(level, b"WWWWWWWWWWWWWWWW", 9_000_000_000 + k);
                        }
                    }
                    let only_lists = *list != "own";
                    let mut states: Vec<(Screen, usize, usize)> = Vec::new();
                    if only_lists {
                        states.push((Screen::Custom, 0, 0));
                        states.push((Screen::Custom, 3, 0));
                        states.push((Screen::Custom, 12, 0));
                        states.push((Screen::Custom, 100, 0));
                    } else {
                        for sc in [
                            Screen::Main,
                            Screen::Kinds,
                            Screen::Community,
                            Screen::Start,
                            Screen::Highscore,
                            Screen::Options,
                            Screen::Graphics,
                            Screen::Controls,
                            Screen::Sound,
                            Screen::Extended,
                            Screen::Language,
                            Screen::Credits,
                            Screen::Pause,
                            Screen::Dead,
                            Screen::End,
                            Screen::Ask(Ask::Quit),
                            Screen::Ask(Ask::Restart),
                            Screen::Ask(Ask::Exit),
                            Screen::Entry,
                            Screen::Custom,
                        ] {
                            for i in 0..m.lines(sc).len().max(1) {
                                states.push((sc, i, 0));
                            }
                        }
                        // the credits at their end too
                        states.push((Screen::Credits, 0, 10_000));
                        for i in 0..m.lines(Screen::Mods).len() {
                            states.push((Screen::Mods, i, 0));
                        }
                        for page in 0..m.mods.found.len() {
                            for i in 0..=m
                                .mods
                                .found
                                .get(page)
                                .map_or(0, |f| f.parsed.as_ref().map_or(0, |t| t.options.len()))
                            {
                                states.push((Screen::ModPage, i, page));
                            }
                        }
                    }
                    for (sc, i, extra) in states {
                        m.mod_page = if sc == Screen::ModPage { extra } else { 0 };
                        m.credits_top = if sc == Screen::Credits { extra } else { 0 };
                        let n = m.lines(sc).len();
                        m.stack = vec![(Screen::Main, 0), (sc, i.min(n.saturating_sub(1)))];
                        m.top = 0;
                        m.top = m.shown(m.list_len(sc)).start;
                        if sc == Screen::Custom && i > 0 {
                            m.top = i.saturating_sub(4);
                        }
                        for (w, h) in sizes {
                            let screen = crate::ui::Screen { w, h };
                            let hits = std::cell::RefCell::new(Vec::new());
                            let probe = std::cell::RefCell::new(Vec::new());
                            m.draw(Some(&face), screen, &hits, Some(&probe));
                            let hits = hits.into_inner();
                            let strip = screen.rect(
                                crate::ui::homogeneous([0.3, 0.0, 0.7, 1.0]),
                                crate::ui::Anchor::Center,
                            );
                            let tol = 1.5;
                            let inside = |b: [f32; 4], r: [f32; 4]| {
                                b[0] >= r[0] - tol
                                    && b[2] <= r[2] + tol
                                    && b[1] >= r[1] - tol
                                    && b[3] <= r[3] + tol
                            };
                            let id =
                                format!("{lang} mode={mode} {list} {sc:?}/{i}/{extra} {w}x{h}");
                            // the buttons: on the window, none on another (the scroll bar lies beside its rows)
                            for (a, ha) in hits.iter().enumerate() {
                                assert!(
                                    inside(ha.rect, [0.0, 0.0, w, h]),
                                    "a button off the window: {id} {:?}",
                                    ha.rect
                                );
                                for hb in hits.iter().skip(a + 1) {
                                    if ha.target == Target::Bar || hb.target == Target::Bar {
                                        continue;
                                    }
                                    let (r, q) = (ha.rect, hb.rect);
                                    assert!(
                                        r[2] <= q[0] + tol
                                            || q[2] <= r[0] + tol
                                            || r[3] <= q[1] + tol
                                            || q[3] <= r[1] + tol,
                                        "two buttons overlap: {id} {r:?} {q:?}"
                                    );
                                }
                            }
                            let probes = probe.into_inner();
                            let buttons: Vec<([f32; 4], f32)> = probes
                                .iter()
                                .filter_map(|p| p.art_right.map(|a| (p.frame, a)))
                                .collect();
                            // a folder path inside a note: every line of it but the last ends at a separator
                            // the labels of one column of buttons share one size
                            let mut sizes_of: std::collections::BTreeMap<u32, (f32, String)> =
                                std::collections::BTreeMap::new();
                            for p in probes
                                .iter()
                                .filter(|p| p.group != 0 && !p.quads.is_empty())
                            {
                                let (s0, t0) = sizes_of
                                    .entry(p.group)
                                    .or_insert_with(|| (p.size, p.text.clone()));
                                assert!(
                                    (p.size - *s0).abs() < 1e-3,
                                    "buttons of one column in different sizes: {id} {:?} at {} but {t0:?} at {s0}",
                                    p.text,
                                    p.size
                                );
                            }
                            // where Back is (its top): nothing of a note lies under it
                            let back = hits
                                .iter()
                                .find(|h| {
                                    matches!(h.target, Target::Line(k)
                                        if matches!(m.lines(sc).get(k), Some(Line::Back) | Some(Line::Next) if sc == Screen::Highscore || matches!(m.lines(sc).get(k), Some(Line::Back))))
                                })
                                .map(|h| h.rect);
                            let mut path_end: Option<bool> = None;
                            let esc = Strings::ours_in(lang).ours("esc back");
                            for p in probes {
                                if p.quads.is_empty() {
                                    continue;
                                }
                                if p.panel.is_some() {
                                    let t = p.text.as_str();
                                    let like = !t.contains(' ')
                                        && (t.starts_with(['~', '/', '%'])
                                            || t.contains(['/', '\\']));
                                    if !t.contains(' ') && t != esc && (like || path_end.is_some())
                                    {
                                        if let Some(ends) = path_end {
                                            assert!(
                                                ends,
                                                "a path breaks inside a name: {id} {t:?} follows a line not ending at a separator"
                                            );
                                        }
                                        seen_paths += 1;
                                        path_end = Some(t.ends_with(['/', '\\']));
                                    } else {
                                        path_end = None;
                                    }
                                } else {
                                    path_end = None;
                                }
                                // a hint is on a note panel, not over the plain background
                                if p.text == esc {
                                    assert!(
                                        p.panel.is_some(),
                                        "the Esc hint is not on a panel: {id} {:?}",
                                        p.text
                                    );
                                    seen_hints += 1;
                                }
                                let b = p.quads.iter().fold(
                                    [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
                                    |a, q| {
                                        [
                                            a[0].min(q[0]),
                                            a[1].min(q[1]),
                                            a[2].max(q[2]),
                                            a[3].max(q[3]),
                                        ]
                                    },
                                );
                                let at = format!("{id} {:?} drawn {b:?} in {:?}", p.text, p.frame);
                                assert!(inside(b, p.frame), "a text out of its frame: {at}");
                                assert!(inside(b, [0.0, 0.0, w, h]), "a text off the window: {at}");
                                assert!(
                                    b[0] >= strip[0] - tol && b[2] <= strip[2] + tol,
                                    "a text out of the dark centre strip {strip:?}: {at}"
                                );
                                assert!(
                                    p.size >= MIN_TEXT - 1e-3,
                                    "a text smaller than the least readable ({} < {MIN_TEXT}): {at}",
                                    p.size
                                );
                                if let Some(panel) = p.panel {
                                    assert!(
                                        inside(b, panel),
                                        "a note out of its panel {panel:?}: {at}"
                                    );
                                    assert!(
                                        inside(panel, strip),
                                        "a panel out of the centre strip {strip:?}: {at} {panel:?}"
                                    );
                                    for hit in &hits {
                                        let (r, q) = (hit.rect, panel);
                                        assert!(
                                            r[2] <= q[0] + tol
                                                || q[2] <= r[0] + tol
                                                || r[3] <= q[1] + tol
                                                || q[3] <= r[1] + tol,
                                            "a panel over a button {r:?} (Back too): {at} {panel:?}"
                                        );
                                    }
                                    // the fill lies inside the rim: within the panel, clear of its edges by the rim
                                    let fill = p.fill.expect("a note has its fill");
                                    assert!(
                                        inside(fill, panel),
                                        "a note's fill out of its rim: {at} {fill:?} {panel:?}"
                                    );
                                    assert!(
                                        fill[0] > panel[0]
                                            && fill[1] > panel[1]
                                            && fill[2] < panel[2]
                                            && fill[3] < panel[3],
                                        "a note's fill touches the rim's outer edge: {at} {fill:?} {panel:?}"
                                    );
                                    // a note is never cut or ellipsized
                                    assert!(
                                        !p.cut && !p.text.trim_end().ends_with(".."),
                                        "a note cut: {at}"
                                    );
                                    seen_notes += 1;
                                }
                                let centre = [
                                    (p.frame[0] + p.frame[2]) / 2.0,
                                    (p.frame[1] + p.frame[3]) / 2.0,
                                ];
                                if let Some(hit) = hits.iter().find(|h| {
                                    centre[0] >= h.rect[0]
                                        && centre[0] <= h.rect[2]
                                        && centre[1] >= h.rect[1]
                                        && centre[1] <= h.rect[3]
                                }) {
                                    assert!(
                                        inside(b, hit.rect),
                                        "a text off its button {:?}: {at}",
                                        hit.rect
                                    );
                                }
                                // the art of a button ends before its rect does (its rounded or bracketed end)
                                if let Some(&(r, right)) = buttons.iter().rev().find(|(r, _)| {
                                    centre[0] >= r[0]
                                        && centre[0] <= r[2]
                                        && centre[1] >= r[1]
                                        && centre[1] <= r[3]
                                }) {
                                    assert!(
                                        b[2] <= right + tol,
                                        "a text past the end of its button's art ({right}, rect {r:?}): {at}"
                                    );
                                }
                                if let Some(c) = p.clear_left {
                                    assert!(
                                        b[0] >= c - tol,
                                        "a text over its button's ball (ends at {c}): {at}"
                                    );
                                }
                                // no note and no hint lies below Back
                                if let (Some(panel), Some(bk)) = (p.panel, back) {
                                    assert!(
                                        panel[3] <= bk[1] + tol,
                                        "a note below Back {bk:?}: {at} {panel:?}"
                                    );
                                }
                                // a button's label (ball at its left end): every line centred on the one axis of
                                // the text area (right of the ball, left of the art's limit), the block centred
                                // in the button's height
                                if let (Some(c), None, false, Some(&(r, right))) = (
                                    p.clear_left,
                                    p.panel,
                                    p.wrap,
                                    buttons.iter().rev().find(|(r, _)| {
                                        centre[0] >= r[0]
                                            && centre[0] <= r[2]
                                            && centre[1] >= r[1]
                                            && centre[1] <= r[3]
                                    }),
                                ) {
                                    let axis = (c + right.min(r[2])) / 2.0;
                                    let mut qs = p.quads.clone();
                                    qs.sort_by(|a, b| a[1].total_cmp(&b[1]));
                                    let mut lines: Vec<[f32; 4]> = Vec::new();
                                    let mut top = f32::MIN;
                                    for q in qs {
                                        match lines.last_mut() {
                                            Some(l) if q[1] - top < 0.5 * p.line => {
                                                l[0] = l[0].min(q[0]);
                                                l[2] = l[2].max(q[2]);
                                                l[1] = l[1].min(q[1]);
                                                l[3] = l[3].max(q[3]);
                                            }
                                            _ => {
                                                top = q[1];
                                                lines.push(q);
                                            }
                                        }
                                    }
                                    for l in &lines {
                                        let mid = (l[0] + l[2]) / 2.0;
                                        // (the last glyph's trailing spacing: a tenth of a line at most)
                                        assert!(
                                            (mid - axis).abs() <= tol + 0.1 * p.line,
                                            "a label line off the button's text axis {axis} (at {mid}): {at}"
                                        );
                                    }
                                    if (p.frame[1] - r[1]).abs() <= tol
                                        && (p.frame[3] - r[3]).abs() <= tol
                                    {
                                        let mid = (b[1] + b[3]) / 2.0;
                                        let want = (r[1] + r[3]) / 2.0;
                                        // the glyph cells carry the font's own asymmetry (about a fifth of a line)
                                        assert!(
                                            (mid - want).abs() <= tol + 0.25 * p.line,
                                            "a label block not centred in its button's height ({mid} vs {want}): {at}"
                                        );
                                    }
                                }
                                if !p.wrap && p.panel.is_none() && b[3] - b[1] > 1.4 * p.line {
                                    assert!(p.scale >= FIT_SHRINK - 1e-3, "two small lines: {at}");
                                }
                                // no label or value of the game's own texts is ever shortened (only list rows of
                                // levels and the highscores' names, which come from outside, may be)
                                if !matches!(sc, Screen::Custom | Screen::Highscore | Screen::Entry)
                                    && !p.wrap
                                    && p.panel.is_none()
                                {
                                    let letters =
                                        p.text.chars().filter(|c| !c.is_whitespace()).count();
                                    if p.cut || p.quads.len() < letters {
                                        cut.insert(format!("{lang} {id} {:?} ({w}x{h})", p.text));
                                    }
                                }
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(cut.is_empty(), "texts are ellipsized: {cut:#?}");
        eprintln!("{checked} texts checked, {seen_notes} of them on notes");
        assert!(checked > 50_000, "only {checked} texts checked");
        assert!(seen_notes > 500, "only {seen_notes} note lines checked");
        assert!(seen_hints > 100, "only {seen_hints} Esc hints checked");
        assert!(seen_paths > 50, "only {seen_paths} path lines checked");
        TEST_PATH.with(|p| *p.borrow_mut() = None);
        TEST_NAME.with(|n| *n.borrow_mut() = None);
    }

    /// The screens whose lines have a help on the notes panel.
    const HELP_SCREENS: [Screen; 14] = [
        Screen::Main,
        Screen::Kinds,
        Screen::Community,
        Screen::Start,
        Screen::Options,
        Screen::Graphics,
        Screen::Controls,
        Screen::Sound,
        Screen::Extended,
        Screen::Language,
        Screen::Pause,
        Screen::Dead,
        Screen::End,
        Screen::Ask(Ask::Quit),
    ];

    #[test]
    fn every_line_and_every_value_has_a_help_in_every_language() {
        // the language files: every help text has every language
        let mut helps = std::collections::BTreeMap::new();
        for (code, _) in languages() {
            for key in crate::lang::table_keys(code)
                .into_iter()
                .filter(|k| k.starts_with("help "))
            {
                let t = Strings::ours_for(&key, code);
                assert!(
                    !t.is_empty() && t != key,
                    "an empty help text: {key} in {code}"
                );
                helps.insert(key, code.to_string());
            }
        }
        // every line of every screen, and every value of it, names a help that is there
        let mut used = std::collections::BTreeSet::new();
        let mut checked = 0;
        for copy in [true, false] {
            for mode in [true, false] {
                let mut m = menu(&[true; 13], mode);
                m.copy = copy;
                m.features.debug = true;
                m.modified = true;
                let mut screens = HELP_SCREENS.to_vec();
                screens.extend([Screen::Ask(Ask::Restart), Screen::Ask(Ask::Exit)]);
                for sc in screens {
                    for line in m.lines(sc) {
                        if !m.describe(Some(line)).is_empty() {
                            continue;
                        }
                        let (key, values) = m
                            .help_spec(sc, line)
                            .unwrap_or_else(|| panic!("no help for {sc:?} {line:?}"));
                        let keys: Vec<String> = if values.is_empty() {
                            vec![key]
                        } else {
                            values.iter().map(|v| format!("{key} {v}")).collect()
                        };
                        assert_eq!(keys.len(), m.help_values(sc, line).max(1).max(keys.len()));
                        for k in keys {
                            assert!(
                                helps.contains_key(&k),
                                "missing help text {k:?} ({sc:?} {line:?})"
                            );
                            used.insert(k);
                            checked += 1;
                        }
                    }
                }
            }
        }
        // the list screens: Map and Back
        for k in ["help map", "help back", "help look beautiful"] {
            assert!(helps.contains_key(k));
            used.insert(k.to_owned());
        }
        for k in helps.keys() {
            assert!(used.contains(k), "a help text nothing uses: {k}");
        }
        assert!(checked > 100, "only {checked} helps checked");
    }

    #[test]
    fn every_help_fits_its_panel_for_every_value_in_every_language() {
        if !crate::fallback::available() {
            eprintln!("skipped: the shipped fonts (assets/fonts) are not there");
            return;
        }
        let face = crate::ui::Face::Own;
        let (mut shown, mut values) = (0, 0);
        for &(lang, _) in languages() {
            for mode in [true, false] {
                let mut m = menu(&[true; 13], mode);
                m.features.debug = true;
                m.modified = true;
                m.strings = Strings::ours_in(lang);
                let mut screens = HELP_SCREENS.to_vec();
                screens.extend([Screen::Ask(Ask::Restart), Screen::Ask(Ask::Exit)]);
                for sc in screens {
                    for i in 0..m.lines(sc).len() {
                        let line = m.lines(sc)[i];
                        for j in 0..m.help_values(sc, line) {
                            m.help_force.set(Some(j));
                            m.stack = vec![(Screen::Main, 0), (sc, i)];
                            for (w, h) in [(640.0, 480.0), (1280.0, 720.0), (1024.0, 768.0)] {
                                let screen = crate::ui::Screen { w, h };
                                let hits = std::cell::RefCell::new(Vec::new());
                                let probe = std::cell::RefCell::new(Vec::new());
                                m.draw(Some(&face), screen, &hits, Some(&probe));
                                let hits = hits.into_inner();
                                let id = format!("{lang} mode={mode} {sc:?}/{i} value {j} {w}x{h}");
                                let strip = screen.rect(
                                    crate::ui::homogeneous([0.3, 0.0, 0.7, 1.0]),
                                    crate::ui::Anchor::Center,
                                );
                                let tol = 1.5;
                                let mut on_panel = 0;
                                let text = m.help_now(sc, line);
                                for p in probe.into_inner().iter().filter(|p| p.panel.is_some()) {
                                    let panel = p.panel.unwrap();
                                    on_panel += 1;
                                    assert!(
                                        !p.cut && !p.text.ends_with(".."),
                                        "a help cut: {id} {:?}",
                                        p.text
                                    );
                                    assert!(p.size >= MIN_TEXT - 1e-3, "a help too small: {id}");
                                    assert!(
                                        panel[0] >= strip[0] - tol && panel[2] <= strip[2] + tol,
                                        "a help panel out of the strip: {id}"
                                    );
                                    for hit in &hits {
                                        let (r, q) = (hit.rect, panel);
                                        assert!(
                                            r[2] <= q[0] + tol
                                                || q[2] <= r[0] + tol
                                                || r[3] <= q[1] + tol
                                                || q[3] <= r[1] + tol,
                                            "a help panel over a button {r:?}: {id} {panel:?}"
                                        );
                                    }
                                }
                                assert!(on_panel > 0, "no help panel: {id}");
                                // the whole text is there: its words all appear on the panel's lines
                                assert!(!text.trim().is_empty(), "an empty help: {id}");
                                shown += 1;
                            }
                            values += 1;
                        }
                    }
                }
            }
        }
        eprintln!("{values} line values, {shown} drawings of helps");
        assert!(values > 500, "only {values} line values walked");
    }

    #[test]
    fn the_name_entry_ok_is_clicked() {
        let mut m = menu(&[true], false);
        m.ask_name(b"Veresk".to_vec(), 1234);
        let ok = at(&m, true, 1024.0, 768.0, Target::Line(0), 0.5);
        assert_eq!(m.click(ok), Some(Action::Score(b"Veresk".to_vec())));
    }

    #[test]
    fn the_classic_graphics_page_has_the_originals_lines_and_the_full_screen() {
        let m = menu(&[true], false);
        assert_eq!(
            m.lines(Screen::Graphics),
            [
                Line::Resolution,
                Line::Synch,
                Line::Clouds,
                Line::Fullscreen,
                Line::Look,
                Line::Back
            ]
        );
        assert_eq!(
            m.lines(Screen::Sound),
            [Line::Music, Line::Effects, Line::SoundTheme, Line::Back]
        );
        // a line's value is shown for each, and the arrows ask for the pack or theme (the same keys as extended)
        assert_eq!(m.fields(Screen::Graphics).len(), 5);
        assert_eq!(m.fields(Screen::Sound).len(), 3);
        let e = menu(&[true], true);
        assert!(e.lines(Screen::Graphics).contains(&Line::Fov));
        assert_eq!(e.fields(Screen::Graphics).len(), 6);
        assert!(!m.lines(Screen::Controls).contains(&Line::Defaults));
        assert!(e.lines(Screen::Controls).contains(&Line::Defaults));
    }

    fn graphics(a: Action) -> Option<Graphics> {
        if let Action::Graphics(g) = a {
            Some(g)
        } else {
            None
        }
    }

    fn bindings(a: Action) -> Option<Bindings> {
        if let Action::Bindings(b) = a {
            Some(b)
        } else {
            None
        }
    }

    #[test]
    fn graphics_lines_change_and_say_so() {
        let mut m = menu(&[true], true);
        m.push(Screen::Graphics);
        // Back is chosen first, as the original's; up to Fov, then the others
        m.step(-1);
        assert_eq!(
            m.adjust(1),
            Action::Graphics(Graphics {
                fov: 60,
                ..Graphics::default()
            })
        );
        // ours: the look (not a graphics setting), then the full screen
        m.step(-1);
        assert!(matches!(m.adjust(1), Action::Look(_)));
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
        assert_eq!(
            m.fields(Screen::Controls)[0].1,
            m.strings.ours("press a key")
        );
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
        assert_eq!(labels, ["Start", "Highscore", "Options", "Credits", "Exit"]);
        let mut m = menu(&[true], true);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(
            m.screen(),
            Screen::Kinds,
            "Start asks which levels, in either mode"
        );
        let kinds: Vec<String> = m
            .lines(Screen::Kinds)
            .into_iter()
            .map(|l| m.label(l))
            .collect();
        assert_eq!(
            kinds,
            [
                "Featured maps",
                "Classic Ballance",
                "Community maps",
                "Back"
            ]
        );
        m.step(1);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(
            m.screen(),
            Screen::Start,
            "the original levels are the classic list"
        );
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
        assert_eq!(
            m.screen(),
            Screen::Kinds,
            "the classic mode: the original levels and the community maps"
        );
        m.step(1);
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
    fn without_a_copy_the_menu_offers_our_levels_and_the_import() {
        let mut m = menu(&[false; 13], false);
        m.copy = false;
        assert_eq!(
            m.lines(Screen::Main),
            [
                Line::Start,
                Line::Highscore,
                Line::Options,
                Line::Credits,
                Line::Exit
            ]
        );
        assert!(m.lines(Screen::Options).contains(&Line::Import));
        assert_eq!(m.enter(), Action::None);
        assert_eq!(
            m.screen(),
            Screen::Kinds,
            "Start offers our levels and the community maps"
        );
        assert_eq!(
            m.lines(Screen::Kinds),
            [Line::Ours, Line::Community, Line::Back]
        );
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Custom, "our own levels");
        assert_eq!(m.title(Screen::Custom).as_deref(), Some("Featured maps"));
        m.back();
        m.back();
        m.step(1);
        m.step(1);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(
            m.screen(),
            Screen::Options,
            "the import is among the options"
        );
        let at = m
            .lines(Screen::Options)
            .iter()
            .position(|&l| l == Line::Import);
        // a page opens on its last line (Back): the arrows wrap to the first
        for _ in 0..=at.unwrap_or(0) {
            m.step(1);
        }
        assert_eq!(m.enter(), Action::Import);
    }

    #[test]
    fn the_challenges_are_offered_only_with_a_copy_and_levels() {
        let kinds = |m: &Menu| m.lines(Screen::Kinds);
        // nothing in the folder: not listed, in either mode
        assert!(!kinds(&menu(&[true], true)).contains(&Line::Challenges));
        assert_eq!(
            kinds(&menu(&[true], false)),
            [Line::Ours, Line::Classic, Line::Community, Line::Back],
            "the same in both modes"
        );
        assert_eq!(kinds(&menu(&[true], false)), kinds(&menu(&[true], true)));
        // levels and a copy
        let mut m = menu(&[true], false);
        m.modified = true;
        assert_eq!(
            kinds(&m),
            [
                Line::Ours,
                Line::Classic,
                Line::Challenges,
                Line::Community,
                Line::Back
            ]
        );
        assert_eq!(m.label(Line::Challenges), "Ballance Challenges");
        assert_eq!(m.label(Line::Classic), "Classic Ballance");
        // the flag is only ever set with a copy (modified::offered); without one the classic list is ours
        let mut m = menu(&[true], false);
        m.copy = false;
        m.modified = true;
        assert_eq!(kinds(&m), [Line::Ours, Line::Community, Line::Back]);
        assert!(!crate::modified::offered(false));
        // the original's two entries are listed only with the copy there (owner's reminder), in every mode
        for mode in [false, true] {
            for modified in [false, true] {
                let mut m = menu(&[true], mode);
                m.copy = false;
                m.modified = modified;
                let l = kinds(&m);
                assert!(
                    !l.contains(&Line::Classic) && !l.contains(&Line::Challenges),
                    "{l:?}"
                );
                m.copy = true;
                assert!(kinds(&m).contains(&Line::Classic));
                assert_eq!(kinds(&m).contains(&Line::Challenges), modified);
            }
        }
        // an empty shipped set leaves its item out
        let mut m = menu(&[true], false);
        m.has_ours = false;
        m.has_best = false;
        assert_eq!(kinds(&m), [Line::Classic, Line::Community, Line::Back]);
        assert_eq!(m.lines(Screen::Community), [Line::Downloaded, Line::Back]);
        for lang in ["en", "ru", "de", "zh", "fr", "es", "it"] {
            let s = crate::lang::Strings::ours_in(lang);
            for key in [
                "classic ballance",
                "drahtseilakt",
                "ballance challenges",
                "community maps",
                "best maps",
                "downloaded maps",
                "look",
                "look original",
                "look remaster",
                "look drahtseilakt",
                "look player",
                "note beautiful",
                "note classic",
                "note ours",
                "note challenges",
                "note community",
                "note downloaded",
                "note picks",
            ] {
                assert_ne!(s.ours(key), key, "{lang} {key}");
            }
        }
        let ru = crate::lang::Strings::ours_in("ru");
        assert_eq!(ru.ours("classic ballance"), "Классический Ballance");
        assert_eq!(ru.ours("ballance challenges"), "Испытания Ballance");
        assert_eq!(ru.ours("community maps"), "Карты сообщества");
    }

    #[test]
    fn look_values_are_proper_names() {
        let want = [
            ("interface own", "Drahtseilakt"),
            ("interface original", "Ballance"),
            ("interface remaster", "Remaster"),
            ("look drahtseilakt", "Drahtseilakt"),
        ];
        for lang in ["en", "ru", "de", "zh", "fr", "es", "it"] {
            let s = crate::lang::Strings::ours_in(lang);
            for (key, name) in want {
                assert_eq!(s.ours(key), name, "{lang} {key}");
            }
        }
    }

    #[test]
    fn every_start_item_says_what_it_is() {
        let m = menu(&[true], false);
        for l in [
            Line::Classic,
            Line::Ours,
            Line::Challenges,
            Line::Community,
            Line::Best,
            Line::Downloaded,
        ] {
            assert!(m.describe(Some(l)).len() > 20, "{l:?}");
        }
        // the downloaded folder's place is shown
        assert!(m.describe(Some(Line::Downloaded)).ends_with("downloaded"));
    }

    #[test]
    fn the_community_submenu_has_the_best_and_the_downloaded() {
        let mut m = menu(&[true], false);
        m.stack.clear();
        m.push(Screen::Kinds);
        m.set_chosen(2);
        assert_eq!(m.lines(Screen::Kinds)[2], Line::Community);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Community);
        assert_eq!(
            m.lines(Screen::Community),
            [Line::Best, Line::Downloaded, Line::Back]
        );
        m.step(1);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Custom);
        assert_eq!(m.list, List::Downloaded);
        assert_eq!(m.title(Screen::Custom).as_deref(), Some("Downloaded"));
    }

    #[test]
    fn the_look_line_is_on_the_graphics_page_and_goes_round_the_available_looks() {
        let mut m = menu(&[true], false);
        assert!(m.lines(Screen::Graphics).contains(&Line::Look));
        assert!(!m.lines(Screen::Options).contains(&Line::Look));
        assert_eq!(m.label(Line::Look), "Look");
        assert_eq!(m.look_value(), "Drahtseilakt");
        // every look offered is available, and the arrows only reach those
        let next = crate::look::next(1);
        assert!(next.available());
        m.strings = crate::lang::Strings::ours_in("en");
    }

    #[test]
    fn the_community_maps_are_listed_in_order_with_their_authors() {
        let mut m = menu(&[true], false);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(
            m.lines(Screen::Kinds),
            [Line::Ours, Line::Classic, Line::Community, Line::Back],
            "the classic mode too"
        );
        m.step(1);
        m.step(1);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Community);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen(), Screen::Custom);
        assert_eq!(m.title(Screen::Custom).as_deref(), Some("Best"));
        assert_eq!(m.maps.len(), 13, "the thirteen shipped maps");
        let orders: Vec<_> = m.maps.iter().map(|p| p.order).collect();
        assert_eq!(
            orders,
            (1..=13).map(Some).collect::<Vec<_>>(),
            "in the order of the levels' own numbers"
        );
        assert!(
            m.maps
                .iter()
                .all(|p| p.author.is_some() && p.posted.is_some()),
            "each credits its author"
        );
        assert!(m.credit(0).starts_with("by "), "{}", m.credit(0));
        for lang in ["en", "ru", "de", "zh", "fr", "es", "it"] {
            let s = crate::lang::Strings::ours_in(lang);
            assert!(s.ours("community notice").len() > 30, "{lang}");
            assert_ne!(s.ours("community notice"), "community notice", "{lang}");
        }
        assert!(
            crate::lang::Strings::ours_in("ru")
                .ours("community notice")
                .contains("спасибо им")
        );
        m.back();
        assert_eq!(m.list, List::None);
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
        assert_eq!(
            two_lines("Interface: remaster"),
            pair("Interface:", "remaster")
        );
        assert_eq!(two_lines("Word"), None);
        assert_eq!(two_lines(" Word "), None);
        assert_eq!(two_lines(""), None);
        assert_eq!(
            two_lines("确定退出游戏"),
            pair("确定退", "出游戏"),
            "Chinese at its middle character"
        );
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
