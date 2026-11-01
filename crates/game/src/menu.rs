// The menus: the main menu (continue the run, the levels, custom levels, settings, credits, debug, quit), the level
// menu (the original levels of the player's copy, each with its best time) and the pages behind the other lines,
// chosen with the arrows and Enter. Custom levels, settings and debug are placeholders until they are built. The main menu is shown when the game
// starts without a level named on the command line, and on Esc during a run. Drawn with the prototype's own font
// (hud.rs) until the original's menu screens are ported.

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
                "COMMUNITY MAPS: NOT IN THE MENU YET",
                "",
                "PLAY ONE FROM THE COMMAND LINE:",
                "DRAHTSEILAKT --MAP FILE.NMO",
            ],
            Item::Settings => &["NOTHING TO SET YET", "", "SCREEN, KEYS AND SOUND COME HERE"],
            Item::Credits => &[
                "DRAHTSEILAKT BY VERESK",
                "",
                "BALLANCE BY CYPARADE / ATARI 2004",
                "STARTED AS A FORK OF OPENBALLANCE BY EMDZEJ",
                "PHYSICS: IVP PORTED FROM OPENBALLANCE",
                "",
                "PLAYS YOUR OWN COPY OF THE GAME",
            ],
            Item::Debug => &[
                "NOTHING TO SWITCH HERE YET",
                "",
                "BEGIN ANYWHERE FROM THE COMMAND LINE:",
                "--SECTOR N   --AT X,Y,Z OR OBJECT   --BALL KIND",
                "",
                "REPLAYS AND SESSION LOGS:",
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Continue,
    Play(u32),
    Quit,
}

pub struct Menu {
    pub screen: Screen,
    /// The main menu's lines (Continue only while a run can go on).
    pub items: Vec<Item>,
    pub item: usize,
    pub entries: Vec<Entry>,
    pub selected: usize,
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
        }
    }

    /// Up (-1) or down (+1) on the current screen.
    pub fn step(&mut self, by: isize) {
        match self.screen {
            Screen::Main => {
                let n = self.items.len().max(1) as isize;
                self.item = (self.item as isize + by).rem_euclid(n) as usize;
            }
            Screen::Levels => self.step_level(by),
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
            for l in item.page() {
                centre(&mut out, &l, y, s, WHITE);
                y += line;
            }
            y += line;
            centre(&mut out, "ENTER OR ESC BACK", y, s, GREY);
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
