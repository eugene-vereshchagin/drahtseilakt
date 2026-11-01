// The menus: the main menu (continue the run, the levels, custom levels, settings, credits, debug, quit), the level
// menu (the original levels of the player's copy, each with its best time) and the pages behind the other lines,
// chosen with the arrows and Enter. Custom levels and debug are placeholders until they are built. The main menu is
// shown when the game starts without a level named on the command line, and on Esc during a run. Drawn as the
// original's menu screens (`interface`), or with the prototype's own font (hud.rs) when the copy lacks the game font.

use crate::assets::Data;
use crate::hud::{Rect, text, width};

/// The original game's levels: 1 to 12, and 13 of the add-on when the copy has it.
pub const LEVELS: u32 = 13;

pub struct Entry {
    pub number: u32,
    /// The level file is in the copy.
    pub available: bool,
    /// The best finished run, in ticks.
    pub best: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Main,
    Levels,
    /// A page of text behind a main menu line (custom levels, settings, credits, debug).
    Page(Item),
}

/// The main menu's lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Continue,
    Levels,
    Custom,
    Settings,
    Credits,
    Debug,
    Quit,
}

impl Item {
    /// The button's label in the original's menu style.
    fn title(self) -> &'static str {
        match self {
            Item::Continue => "Continue",
            Item::Levels => "Start",
            Item::Custom => "Custom levels",
            Item::Settings => "Options",
            Item::Credits => "Credits",
            Item::Debug => "Debug",
            Item::Quit => "Exit",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Item::Continue => "CONTINUE",
            Item::Levels => "LEVELS",
            Item::Custom => "CUSTOM LEVELS",
            Item::Settings => "SETTINGS",
            Item::Credits => "CREDITS",
            Item::Debug => "DEBUG",
            Item::Quit => "QUIT",
        }
    }

    /// The lines of the page behind a line of the main menu.
    fn page(self) -> Vec<String> {
        let lines: &[&str] = match self {
            Item::Custom => &[
                "Community maps are not in the menu yet.",
                "",
                "Play one from the command line:",
                "drahtseilakt --map file.nmo",
            ],
            Item::Settings => &["", "Screen and keys come here."],
            Item::Credits => &[
                "Drahtseilakt by Veresk",
                "",
                "Ballance by Cyparade / Atari 2004",
                "Started as a fork of OpenBallance by emdzej",
                "Physics: IVP, ported from OpenBallance",
                "",
                "Plays your own copy of the game",
            ],
            Item::Debug => &[
                "",
                "Begin anywhere from the command line:",
                "--sector n  --at x,y,z or object  --ball kind",
                "",
                "Replays and session logs:",
            ],
            _ => &[],
        };
        let mut lines: Vec<String> = lines.iter().map(|l| (*l).to_owned()).collect();
        if self == Item::Debug {
            lines.push(crate::app::replay_dir().display().to_string());
        }
        lines
    }
}

/// What a key in the menu asks of the game.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    None,
    Continue,
    Play(u32),
    Quit,
    /// The Debug page's switch: debug mode (the free camera on F2) on or off.
    ToggleDebug,
    /// The Settings page's volumes changed: sound and music, 0 to 1.
    Volumes([f32; 2]),
}

pub struct Menu {
    pub screen: Screen,
    /// The main menu's lines (Continue only while a run can go on).
    pub items: Vec<Item>,
    pub item: usize,
    pub entries: Vec<Entry>,
    pub selected: usize,
    /// Debug mode, as the Debug page shows it.
    pub debug: bool,
    /// The volumes the Settings page sets (sound, music), and its line chosen.
    pub volumes: [f32; 2],
    pub setting: usize,
}

impl Menu {
    /// The main menu, with level `current` selected in the level menu; `can_continue`: a run is going on.
    pub fn new(data: &Data, current: u32, can_continue: bool) -> Menu {
        let entries: Vec<Entry> = (1..=LEVELS)
            .map(|number| Entry {
                number,
                available: data
                    .file(&format!("3D Entities/Level/Level_{number:02}.NMO"))
                    .is_some(),
                best: best_ticks(number),
            })
            .collect();
        let selected = entries
            .iter()
            .position(|e| e.number == current && e.available)
            .unwrap_or(0);
        let mut items = vec![
            Item::Levels,
            Item::Custom,
            Item::Settings,
            Item::Credits,
            Item::Debug,
            Item::Quit,
        ];
        if can_continue {
            items.insert(0, Item::Continue);
        }
        Menu {
            screen: Screen::Main,
            items,
            item: 0,
            entries,
            selected,
            debug: false,
            volumes: [1.0; 2],
            setting: 0,
        }
    }

    /// Left (-1) or right (+1): on the Settings page, the chosen volume down or up by a tenth.
    pub fn adjust(&mut self, by: i32) -> Action {
        if self.screen != Screen::Page(Item::Settings) {
            return Action::None;
        }
        let Some(v) = self.volumes.get_mut(self.setting) else { return Action::None };
        *v = ((*v * 10.0).round() + by as f32).clamp(0.0, 10.0) / 10.0;
        Action::Volumes(self.volumes)
    }

    /// Up (-1) or down (+1) on the current screen.
    pub fn step(&mut self, by: isize) {
        match self.screen {
            Screen::Main => {
                let n = self.items.len().max(1) as isize;
                self.item = (self.item as isize + by).rem_euclid(n) as usize;
            }
            Screen::Levels => self.step_level(by),
            Screen::Page(Item::Settings) => self.setting = (self.setting as isize + by).rem_euclid(2) as usize,
            Screen::Page(_) => {}
        }
    }

    /// Enter on the current screen.
    pub fn enter(&mut self) -> Action {
        match self.screen {
            Screen::Main => match self.items.get(self.item) {
                Some(Item::Continue) => Action::Continue,
                Some(Item::Levels) => {
                    self.screen = Screen::Levels;
                    Action::None
                }
                Some(Item::Quit) => Action::Quit,
                Some(&item) => {
                    self.screen = Screen::Page(item);
                    Action::None
                }
                None => Action::None,
            },
            Screen::Levels => self.chosen().map_or(Action::None, Action::Play),
            Screen::Page(Item::Debug) => Action::ToggleDebug,
            Screen::Page(_) => {
                self.screen = Screen::Main;
                Action::None
            }
        }
    }

    /// Esc: the level menu goes back to the main menu, the main menu back to the run if there is one.
    pub fn back(&mut self) -> Action {
        match self.screen {
            Screen::Levels | Screen::Page(_) => {
                self.screen = Screen::Main;
                Action::None
            }
            Screen::Main if self.items.contains(&Item::Continue) => Action::Continue,
            Screen::Main => Action::None,
        }
    }

    /// Moves the level selection by `by` lines, over the levels the copy has.
    fn step_level(&mut self, by: isize) {
        let n = self.entries.len() as isize;
        let mut i = self.selected as isize;
        for _ in 0..n {
            i = (i + by).rem_euclid(n);
            if self.entries.get(i as usize).is_some_and(|e| e.available) {
                self.selected = i as usize;
                return;
            }
        }
    }

    /// The selected level's number, if the copy has it.
    pub fn chosen(&self) -> Option<u32> {
        self.entries
            .get(self.selected)
            .filter(|e| e.available)
            .map(|e| e.number)
    }

    /// The menu over the frame: a shaded screen, the title, a line per level and the keys.
    pub fn overlay(&self, width_px: u32, height_px: u32, loading: Option<u32>) -> Vec<Rect> {
        let (w, h) = (width_px as f32, height_px as f32);
        let mut out = vec![Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            color: [0.0, 0.0, 0.0, 0.6],
        }];
        // Room for the title, the levels and the keys: about 20 lines of 10 font pixels.
        let s = (h * 0.9 / 200.0).floor().max(1.0);
        let line = 10.0 * s;
        let centre = |out: &mut Vec<Rect>, t: &str, y: f32, scale: f32, color: [f32; 4]| {
            text(out, t, (w - width(t, scale)) / 2.0, y, scale, color);
        };
        // Each screen centred: the title and its gap, then the main menu's lines or the levels, then the keys.
        let lines = match self.screen {
            Screen::Main => 9.0 + 1.6 * self.items.len() as f32,
            Screen::Levels => 6.0 + self.entries.len() as f32,
            Screen::Page(item) => 8.0 + item.page().len() as f32,
        };
        let mut y = (h - lines * line) / 2.0;
        centre(&mut out, "DRAHTSEILAKT", y, s * 2.0, GOLD);
        y += 3.0 * line;
        if let Some(n) = loading {
            centre(
                &mut out,
                &format!("LOADING LEVEL {n}"),
                y + 6.0 * line,
                s * 1.5,
                WHITE,
            );
            return out;
        }
        if let Screen::Page(item) = self.screen {
            centre(&mut out, item.label(), y, s * 1.5, WHITE);
            y += 2.5 * line;
            if item == Item::Settings {
                for (i, (name, v)) in [("SOUND", self.volumes[0]), ("MUSIC", self.volumes[1])].into_iter().enumerate() {
                    let text = format!("{name}  {:>3}%", (v * 100.0).round() as u32);
                    centre(&mut out, &text, y, s * 1.5, if i == self.setting { GOLD } else { WHITE });
                    y += 1.6 * line;
                }
            }
            if item == Item::Debug {
                let switch = format!("DEBUG MODE: {}   FREE CAMERA ON F2", if self.debug { "ON" } else { "OFF" });
                centre(&mut out, &switch, y, s, if self.debug { GOLD } else { WHITE });
                y += line;
            }
            for l in item.page() {
                centre(&mut out, &l, y, s, WHITE);
                y += line;
            }
            y += line;
            let keys = match item {
                Item::Debug => "ENTER SWITCHES   ESC BACK",
                Item::Settings => "UP/DOWN CHOOSE   LEFT/RIGHT CHANGE   ESC BACK",
                _ => "ENTER OR ESC BACK",
            };
            centre(&mut out, keys, y, s, GREY);
            return out;
        }
        if self.screen == Screen::Main {
            y += 3.0 * line;
            for (i, item) in self.items.iter().enumerate() {
                let label = item.label();
                let scale = s * 1.5;
                if i == self.item {
                    let pad = s * 3.0;
                    let lw = width(label, scale);
                    out.push(Rect {
                        x: (w - lw) / 2.0 - pad,
                        y: y - pad,
                        w: lw + 2.0 * pad,
                        h: 7.0 * scale + 2.0 * pad,
                        color: SELECT,
                    });
                }
                centre(
                    &mut out,
                    label,
                    y,
                    scale,
                    if i == self.item { GOLD } else { WHITE },
                );
                y += 1.6 * line;
            }
            y += 2.0 * line;
            let keys = if self.items.contains(&Item::Continue) {
                "UP/DOWN CHOOSE   ENTER   ESC BACK TO THE RUN"
            } else {
                "UP/DOWN CHOOSE   ENTER"
            };
            centre(&mut out, keys, y, s, GREY);
            return out;
        }
        for (i, e) in self.entries.iter().enumerate() {
            let best = match e.best {
                Some(t) => format!("BEST {}", clock(t)),
                None if e.available => "-".to_owned(),
                None => "NOT IN THIS COPY".to_owned(),
            };
            let label = format!("LEVEL {:2}   {best:>14}", e.number);
            let color = match (i == self.selected, e.available) {
                (true, _) => GOLD,
                (false, true) => WHITE,
                (false, false) => GREY,
            };
            if i == self.selected {
                let pad = s * 3.0;
                let lw = width(&label, s);
                out.push(Rect {
                    x: (w - lw) / 2.0 - pad,
                    y: y - s * 1.5,
                    w: lw + 2.0 * pad,
                    h: line,
                    color: SELECT,
                });
            }
            centre(&mut out, &label, y, s, color);
            y += line;
        }
        y += line;
        centre(
            &mut out,
            "UP/DOWN CHOOSE   ENTER PLAY   ESC BACK",
            y,
            s,
            GREY,
        );
        out
    }
}

/// The original's menu screens (Menu.nmo's 2D frames and texts as the C game shows them): a dark band down the
/// middle, the buttons over Button01_select (chosen), Button01_deselect or Button01_special (not playable), their
/// labels in the game's fonts (GameFont_01 for the main buttons, GameFont_03 and the grey GameFont_03a for the levels).
impl Menu {
    pub fn interface(&self, font: Option<&crate::ui::Font>, screen: crate::ui::Screen, loading: Option<u32>) -> Vec<crate::ui::Quad> {
        use crate::ui::{Anchor, Quad, homogeneous};
        let mut out = Vec::new();
        let quad = |r: [f32; 4], uv: [f32; 4], color: [f32; 4], texture: &'static str| Quad {
            rect: screen.rect(homogeneous(r), Anchor::Center),
            uv,
            top: color,
            bottom: color,
            texture,
        };
        // M_BlackScreen: a strip of Button01_special's black at 0.61
        out.push(quad([0.3, 0.0, 0.7, 1.0], [0.0, 0.99, 1.0, 1.0], [1.0, 1.0, 1.0, 0.61], SPECIAL));
        let label = |out: &mut Vec<Quad>, t: &str, r: [f32; 4], style: &crate::ui::Style| {
            if let Some(f) = font {
                out.extend(crate::ui::place(f.layout(t.as_bytes(), homogeneous(r), style), screen, Anchor::Center));
            }
        };
        if let Some(n) = loading {
            label(&mut out, &format!("Loading level {n}"), [0.3, 0.45, 0.7, 0.55], &FONT_01);
            return out;
        }
        let button = |out: &mut Vec<Quad>, r: [f32; 4], uv: [f32; 4], state: Button| {
            let tex = match state {
                Button::Over => SELECT_TEX,
                Button::Up => DESELECT_TEX,
                Button::Inactive => SPECIAL,
            };
            out.push(quad(r, uv, WHITE, tex));
        };
        match self.screen {
            Screen::Main => {
                // five buttons 0.15 apart from 0.15 in the original; more lines close up
                let n = self.items.len().max(1);
                let step = if n <= 5 { 0.15 } else { 0.75 / n as f32 };
                let top = if n <= 5 { 0.15 } else { 0.1 };
                for (i, item) in self.items.iter().enumerate() {
                    let y = top + i as f32 * step;
                    let r = [0.35, y, 0.65, y + 0.0938];
                    let state = if i == self.item { Button::Over } else { Button::Up };
                    button(&mut out, r, MAIN_UV, state);
                    label(&mut out, item.title(), r, &FONT_01);
                }
            }
            Screen::Levels => {
                // 0.06 apart from 0.09 (twelve levels); a thirteenth closes them up
                let n = self.entries.len().max(1);
                let step = if n <= 12 { 0.06 } else { 0.72 / n as f32 };
                for (i, e) in self.entries.iter().enumerate() {
                    let y = 0.09 + i as f32 * step;
                    let r = [0.4031, y, 0.5969, y + step - 0.01];
                    let state = match (e.available, i == self.selected) {
                        (false, _) => Button::Inactive,
                        (true, true) => Button::Over,
                        (true, false) => Button::Up,
                    };
                    button(&mut out, r, LEVEL_UV, state);
                    label(&mut out, &format!("Level {}", e.number), r, if e.available { &FONT_03 } else { &FONT_03A });
                    if let Some(t) = e.best {
                        let mut st = FONT_03A;
                        st.align = crate::ui::ALIGN_LEFT;
                        label(&mut out, &clock(t), [0.6, y, 0.7, y + step - 0.01], &st);
                    }
                }
                let r = [0.4031, 0.85, 0.5906, 0.9438];
                button(&mut out, r, BACK_UV, Button::Up);
                label(&mut out, "Back", r, &FONT_01);
            }
            Screen::Page(item) => {
                let r = [0.35, 0.05, 0.65, 0.1438];
                button(&mut out, r, MAIN_UV, Button::Over);
                label(&mut out, item.title(), r, &FONT_01);
                let mut y = 0.2;
                if item == Item::Settings {
                    for (i, (name, v)) in [("Sound", self.volumes[0]), ("Music", self.volumes[1])].into_iter().enumerate() {
                        let r = [0.35, y, 0.65, y + 0.0938];
                        button(&mut out, r, MAIN_UV, if i == self.setting { Button::Over } else { Button::Up });
                        label(&mut out, &format!("{name}  {}%", (v * 100.0).round() as u32), r, &FONT_01);
                        y += 0.12;
                    }
                }
                if item == Item::Debug {
                    let on = if self.debug { "on" } else { "off" };
                    label(&mut out, &format!("Debug mode {on}, free camera on F2"), [0.3, y, 0.7, y + 0.05], &FONT_03);
                    y += 0.06;
                }
                for l in item.page() {
                    label(&mut out, &l, [0.3, y, 0.7, y + 0.05], &FONT_03);
                    y += 0.05;
                }
                let r = [0.4031, 0.85, 0.5906, 0.9438];
                button(&mut out, r, BACK_UV, Button::Up);
                label(&mut out, "Back", r, &FONT_01);
            }
        }
        out
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
/// The texture parts of the main buttons, the level buttons and the Back button.
const MAIN_UV: [f32; 4] = [0.0, 0.5137, 1.0, 0.7451];
const LEVEL_UV: [f32; 4] = [0.0, 0.2470, 0.6470, 0.3686];
const BACK_UV: [f32; 4] = [0.2392, 0.7529, 0.8666, 0.9843];

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

/// The best finished run of an original level, from its saved best replay.
fn best_ticks(number: u32) -> Option<u64> {
    let path = crate::app::replay_dir()
        .join(format!("Level_{number:02}"))
        .join("best.replay");
    let text = std::fs::read_to_string(path).ok()?;
    crate::replay::Replay::parse(&text).ok().map(|r| r.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(available: &[bool]) -> Menu {
        Menu {
            entries: available
                .iter()
                .enumerate()
                .map(|(i, &available)| Entry {
                    number: i as u32 + 1,
                    available,
                    best: None,
                })
                .collect(),
            selected: 0,
            screen: Screen::Levels,
            items: vec![Item::Levels, Item::Credits, Item::Quit],
            item: 0,
            debug: false,
            volumes: [1.0; 2],
            setting: 0,
        }
    }

    #[test]
    fn the_selection_skips_missing_levels_and_wraps() {
        let mut m = menu(&[true, false, true]);
        m.step(1);
        assert_eq!(m.chosen(), Some(3));
        m.step(1);
        assert_eq!(m.chosen(), Some(1));
        m.step(-1);
        assert_eq!(m.chosen(), Some(3));
    }

    #[test]
    fn the_main_menu_leads_to_the_levels_and_back() {
        let mut m = menu(&[true, true]);
        m.screen = Screen::Main;
        assert_eq!(m.back(), Action::None, "no run to go back to");
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen, Screen::Levels);
        m.step(1);
        assert_eq!(m.enter(), Action::Play(2));
        assert_eq!(m.back(), Action::None);
        m.step(1);
        assert_eq!(m.enter(), Action::None);
        assert_eq!(m.screen, Screen::Page(Item::Credits));
        assert_eq!(m.enter(), Action::None, "a page goes back");
        m.step(1);
        assert_eq!(m.enter(), Action::Quit);
        m.items = vec![Item::Continue, Item::Levels, Item::Quit];
        assert_eq!(m.back(), Action::Continue);
    }
}
