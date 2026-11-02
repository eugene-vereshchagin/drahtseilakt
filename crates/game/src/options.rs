// The player's graphics and controls settings (the Options' Graphics and Controls pages): what settings.txt keeps
// of them, the keys bound to the game's six actions, and the gamepads, read into the same digital keys as the
// keyboard so that a run (and its replay) is the same whatever played it.

use crate::sim::input;
use winit::keyboard::KeyCode;

/// The window sizes offered beside the monitor's own modes.
pub const WINDOWED: [(u32, u32); 8] = [
    (800, 600),
    (1024, 768),
    (1280, 720),
    (1280, 960),
    (1366, 768),
    (1600, 900),
    (1920, 1080),
    (2560, 1440),
];

/// The sizes the settings take: from the original's smallest mode to 8K, the largest surface the renderer asks
/// the graphics device for (its default limit, 8192 a side; a larger one would fail on every start).
pub const SURFACE_MAX: u32 = 8192;
const WIDTH: std::ops::RangeInclusive<u32> = 320..=SURFACE_MAX;
const HEIGHT: std::ops::RangeInclusive<u32> = 200..=SURFACE_MAX;

/// A size written `WxH` (or `W*H`, as the menu shows it), refused outside the sizes a window can have.
pub fn parse_size(v: &str) -> Option<(u32, u32)> {
    let (w, h) = v.trim().split_once(['x', 'X', '*'])?;
    let (w, h) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    (WIDTH.contains(&w) && HEIGHT.contains(&h)).then_some((w, h))
}

/// The window over the whole screen or not: borderless takes the desktop's mode, exclusive changes it (where the
/// platform can; Wayland cannot, and gets borderless).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fullscreen {
    #[default]
    Off,
    Borderless,
    Exclusive,
}

impl Fullscreen {
    const ALL: [Fullscreen; 3] = [
        Fullscreen::Off,
        Fullscreen::Borderless,
        Fullscreen::Exclusive,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Fullscreen::Off => "off",
            Fullscreen::Borderless => "borderless",
            Fullscreen::Exclusive => "exclusive",
        }
    }

    fn from_key(v: &str) -> Option<Fullscreen> {
        Fullscreen::ALL.into_iter().find(|f| f.key() == v.trim())
    }

    /// The next choice to the left (-1) or right (+1), round.
    pub fn next(self, by: i32) -> Fullscreen {
        let i = Fullscreen::ALL.iter().position(|&f| f == self).unwrap_or(0) as i32;
        let n = Fullscreen::ALL.len() as i32;
        Fullscreen::ALL[(i + by).rem_euclid(n) as usize]
    }
}

/// The original's field of view: InGameCam's 1.0123 radians (58°) across its 4:3 view.
pub const FOV_ORIGINAL: u32 = 58;
/// The extended mode's range of it, and its step on the menu.
pub const FOV_RANGE: std::ops::RangeInclusive<u32> = 40..=100;
pub const FOV_STEP: u32 = 2;

/// The Graphics page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Graphics {
    /// The window's size (the screen's mode when exclusive); the start size when none is kept.
    pub resolution: Option<(u32, u32)>,
    pub fullscreen: Fullscreen,
    /// Synch to Screen: the frames wait for the monitor (no tearing).
    pub vsync: bool,
    /// The levels' cloud layer (their `SkyLayer` objects).
    pub clouds: bool,
    /// The horizontal field of view across 4:3, degrees (the extended mode's; the classic mode keeps the original's).
    pub fov: u32,
}

impl Default for Graphics {
    fn default() -> Graphics {
        Graphics {
            resolution: None,
            fullscreen: Fullscreen::Off,
            vsync: false,
            clouds: true,
            fov: FOV_ORIGINAL,
        }
    }
}

/// `on` / `off` in the settings.
fn on_off(v: &str) -> Option<bool> {
    match v.trim() {
        "on" | "yes" => Some(true),
        "off" | "no" => Some(false),
        _ => None,
    }
}

fn word(b: bool) -> String {
    (if b { "on" } else { "off" }).to_owned()
}

impl Graphics {
    /// The settings' choices (`get` reads a line's value); a value it cannot read keeps the default.
    pub fn load(get: impl Fn(&str) -> Option<String>) -> Graphics {
        let d = Graphics::default();
        Graphics {
            resolution: get("resolution").as_deref().and_then(parse_size),
            fullscreen: get("fullscreen")
                .as_deref()
                .and_then(Fullscreen::from_key)
                .unwrap_or(d.fullscreen),
            vsync: get("vsync").as_deref().and_then(on_off).unwrap_or(d.vsync),
            clouds: get("clouds")
                .as_deref()
                .and_then(on_off)
                .unwrap_or(d.clouds),
            fov: get("fov")
                .and_then(|v| v.trim().parse::<u32>().ok())
                .filter(|f| FOV_RANGE.contains(f))
                .unwrap_or(d.fov),
        }
    }

    /// The settings' lines for it.
    pub fn lines(&self) -> Vec<(&'static str, String)> {
        let mut l = Vec::new();
        if let Some((w, h)) = self.resolution {
            l.push(("resolution", format!("{w}x{h}")));
        }
        l.push(("fullscreen", self.fullscreen.key().to_owned()));
        l.push(("vsync", word(self.vsync)));
        l.push(("clouds", word(self.clouds)));
        l.push(("fov", self.fov.to_string()));
        l
    }

    /// The field of view moved by `by` steps, inside its range.
    pub fn fov_by(&self, by: i32) -> u32 {
        let f = i64::from(self.fov) + i64::from(by) * i64::from(FOV_STEP);
        let (lo, hi) = (i64::from(*FOV_RANGE.start()), i64::from(*FOV_RANGE.end()));
        u32::try_from(f.clamp(lo, hi)).unwrap_or(FOV_ORIGINAL)
    }
}

/// The game's six actions, in the Controls page's order: their settings' names and the keys they press.
pub const ACTIONS: [(&str, u8); 6] = [
    ("forward", input::UP),
    ("backward", input::DOWN),
    ("left", input::LEFT),
    ("right", input::RIGHT),
    ("rotation", input::SHIFT),
    ("overview", input::LIFT),
];

/// The original's keys for them (base.cmo, DB_Options).
pub const DEFAULT_KEYS: [KeyCode; 6] = [
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ShiftLeft,
    KeyCode::Space,
];

/// The keys that can be bound: their names in the settings (winit's), and their row in the copy's key names
/// (Language.nmo `all_keys`, which DB_Options counts in). The rows are DirectInput's scan codes in order, so a row
/// is the same physical key as winit's code, whatever the keyboard's layout.
const KEYS: &[(KeyCode, &str, Option<usize>)] = &[
    (KeyCode::Digit1, "Digit1", Some(0)),
    (KeyCode::Digit2, "Digit2", Some(1)),
    (KeyCode::Digit3, "Digit3", Some(2)),
    (KeyCode::Digit4, "Digit4", Some(3)),
    (KeyCode::Digit5, "Digit5", Some(4)),
    (KeyCode::Digit6, "Digit6", Some(5)),
    (KeyCode::Digit7, "Digit7", Some(6)),
    (KeyCode::Digit8, "Digit8", Some(7)),
    (KeyCode::Digit9, "Digit9", Some(8)),
    (KeyCode::Digit0, "Digit0", Some(9)),
    (KeyCode::Minus, "Minus", Some(10)),
    (KeyCode::Equal, "Equal", Some(11)),
    (KeyCode::Backspace, "Backspace", Some(12)),
    (KeyCode::Tab, "Tab", Some(13)),
    (KeyCode::KeyQ, "KeyQ", Some(14)),
    (KeyCode::KeyW, "KeyW", Some(15)),
    (KeyCode::KeyE, "KeyE", Some(16)),
    (KeyCode::KeyR, "KeyR", Some(17)),
    (KeyCode::KeyT, "KeyT", Some(18)),
    (KeyCode::KeyY, "KeyY", Some(19)),
    (KeyCode::KeyU, "KeyU", Some(20)),
    (KeyCode::KeyI, "KeyI", Some(21)),
    (KeyCode::KeyO, "KeyO", Some(22)),
    (KeyCode::KeyP, "KeyP", Some(23)),
    (KeyCode::BracketLeft, "BracketLeft", Some(24)),
    (KeyCode::BracketRight, "BracketRight", Some(25)),
    (KeyCode::ControlLeft, "ControlLeft", Some(26)),
    (KeyCode::KeyA, "KeyA", Some(27)),
    (KeyCode::KeyS, "KeyS", Some(28)),
    (KeyCode::KeyD, "KeyD", Some(29)),
    (KeyCode::KeyF, "KeyF", Some(30)),
    (KeyCode::KeyG, "KeyG", Some(31)),
    (KeyCode::KeyH, "KeyH", Some(32)),
    (KeyCode::KeyJ, "KeyJ", Some(33)),
    (KeyCode::KeyK, "KeyK", Some(34)),
    (KeyCode::KeyL, "KeyL", Some(35)),
    (KeyCode::Semicolon, "Semicolon", Some(36)),
    (KeyCode::Quote, "Quote", Some(37)),
    (KeyCode::Backquote, "Backquote", Some(38)),
    (KeyCode::ShiftLeft, "ShiftLeft", Some(39)),
    (KeyCode::Backslash, "Backslash", Some(40)),
    (KeyCode::KeyZ, "KeyZ", Some(41)),
    (KeyCode::KeyX, "KeyX", Some(42)),
    (KeyCode::KeyC, "KeyC", Some(43)),
    (KeyCode::KeyV, "KeyV", Some(44)),
    (KeyCode::KeyB, "KeyB", Some(45)),
    (KeyCode::KeyN, "KeyN", Some(46)),
    (KeyCode::KeyM, "KeyM", Some(47)),
    (KeyCode::Comma, "Comma", Some(48)),
    (KeyCode::Period, "Period", Some(49)),
    (KeyCode::Slash, "Slash", Some(50)),
    (KeyCode::ShiftRight, "ShiftRight", Some(51)),
    (KeyCode::AltLeft, "AltLeft", Some(52)),
    (KeyCode::Space, "Space", Some(53)),
    (KeyCode::Numpad7, "Numpad7", Some(54)),
    (KeyCode::Numpad8, "Numpad8", Some(55)),
    (KeyCode::Numpad9, "Numpad9", Some(56)),
    (KeyCode::NumpadSubtract, "NumpadSubtract", Some(57)),
    (KeyCode::Numpad4, "Numpad4", Some(58)),
    (KeyCode::Numpad5, "Numpad5", Some(59)),
    (KeyCode::Numpad6, "Numpad6", Some(60)),
    (KeyCode::NumpadAdd, "NumpadAdd", Some(61)),
    (KeyCode::Numpad1, "Numpad1", Some(62)),
    (KeyCode::Numpad2, "Numpad2", Some(63)),
    (KeyCode::Numpad3, "Numpad3", Some(64)),
    (KeyCode::Numpad0, "Numpad0", Some(65)),
    (KeyCode::NumpadDecimal, "NumpadDecimal", Some(66)),
    (KeyCode::IntlBackslash, "IntlBackslash", Some(67)),
    (KeyCode::ArrowUp, "ArrowUp", Some(68)),
    (KeyCode::ArrowDown, "ArrowDown", Some(69)),
    (KeyCode::ArrowLeft, "ArrowLeft", Some(70)),
    (KeyCode::ArrowRight, "ArrowRight", Some(71)),
    // ours: keys the original's list lacks
    (KeyCode::Enter, "Enter", None),
    (KeyCode::NumpadEnter, "NumpadEnter", None),
    (KeyCode::ControlRight, "ControlRight", None),
    (KeyCode::AltRight, "AltRight", None),
    (KeyCode::Insert, "Insert", None),
    (KeyCode::Delete, "Delete", None),
    (KeyCode::Home, "Home", None),
    (KeyCode::End, "End", None),
    (KeyCode::PageUp, "PageUp", None),
    (KeyCode::PageDown, "PageDown", None),
    (KeyCode::NumpadMultiply, "NumpadMultiply", None),
    (KeyCode::NumpadDivide, "NumpadDivide", None),
];

/// The key a settings name stands for.
pub fn key_named(name: &str) -> Option<KeyCode> {
    KEYS.iter().find(|k| k.1 == name.trim()).map(|k| k.0)
}

/// The key in the copy's key names' row `row`.
fn key_of_row(row: usize) -> Option<KeyCode> {
    KEYS.iter().find(|k| k.2 == Some(row)).map(|k| k.0)
}

/// A key's name in the settings.
pub fn key_name(code: KeyCode) -> Option<&'static str> {
    KEYS.iter().find(|k| k.0 == code).map(|k| k.1)
}

/// A key as the Controls page shows it: the copy's name where it has one (as the original shows it), else ours.
pub fn key_label(code: KeyCode, copy: impl Fn(usize) -> Option<Vec<u8>>) -> Vec<u8> {
    let Some(&(_, name, row)) = KEYS.iter().find(|k| k.0 == code) else {
        return b"?".to_vec();
    };
    if let Some(t) = row.and_then(copy) {
        return t;
    }
    let short = ["Key", "Digit"]
        .iter()
        .find_map(|p| name.strip_prefix(p))
        .unwrap_or(name);
    short.as_bytes().to_vec()
}

/// The other one of a pair of keys (the shifts, controls, alts): it does what its twin does when it is bound to
/// nothing itself, as the game always took either shift.
fn twin(code: KeyCode) -> Option<KeyCode> {
    Some(match code {
        KeyCode::ShiftLeft => KeyCode::ShiftRight,
        KeyCode::ShiftRight => KeyCode::ShiftLeft,
        KeyCode::ControlLeft => KeyCode::ControlRight,
        KeyCode::ControlRight => KeyCode::ControlLeft,
        KeyCode::AltLeft => KeyCode::AltRight,
        KeyCode::AltRight => KeyCode::AltLeft,
        _ => return None,
    })
}

/// Why a key was not bound.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Not a key the game takes (Esc, the function keys, and keys outside KEYS).
    NotBindable,
}

/// The Controls page: the key of each action, and the camera's turn inverted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bindings {
    pub keys: [KeyCode; 6],
    pub invert: bool,
}

impl Default for Bindings {
    fn default() -> Bindings {
        Bindings {
            keys: DEFAULT_KEYS,
            invert: false,
        }
    }
}

impl Bindings {
    /// The settings' keys (`key.<action>` lines), else the copy's (DB_Options rows and its invert), else the
    /// original's. Keys that would do two things at once (a hand-edited file) fall back to the copy's or defaults.
    pub fn load(
        get: impl Fn(&str) -> Option<String>,
        copy_rows: [usize; 6],
        copy_invert: bool,
    ) -> Bindings {
        let mut copy = Bindings::default();
        for (slot, &row) in copy.keys.iter_mut().zip(&copy_rows) {
            if let Some(k) = key_of_row(row) {
                *slot = k;
            }
        }
        copy.invert = copy_invert;
        if !copy.distinct() {
            copy = Bindings {
                invert: copy_invert,
                ..Bindings::default()
            };
        }
        let mut b = copy;
        for (slot, (name, _)) in b.keys.iter_mut().zip(ACTIONS) {
            if let Some(k) = get(&format!("key.{name}")).as_deref().and_then(key_named) {
                *slot = k;
            }
        }
        if let Some(i) = get("invert-rotation").as_deref().and_then(on_off) {
            b.invert = i;
        }
        if b.distinct() {
            b
        } else {
            Bindings {
                invert: b.invert,
                ..copy
            }
        }
    }

    fn distinct(&self) -> bool {
        self.keys
            .iter()
            .enumerate()
            .all(|(i, k)| !self.keys.iter().skip(i + 1).any(|o| o == k))
    }

    /// The settings' lines for them.
    pub fn lines(&self) -> Vec<(String, String)> {
        let mut l: Vec<(String, String)> = ACTIONS
            .iter()
            .zip(self.keys)
            .filter_map(|((name, _), k)| Some((format!("key.{name}"), key_name(k)?.to_owned())))
            .collect();
        l.push(("invert-rotation".to_owned(), word(self.invert)));
        l
    }

    /// Binds `key` to `action`. A key another action has is swapped: that action takes this one's old key, so no
    /// key ever does two things and no action is left without one. The other action, if any, is returned.
    pub fn bind(&mut self, action: usize, key: KeyCode) -> Result<Option<usize>, Refused> {
        if key_name(key).is_none() {
            return Err(Refused::NotBindable);
        }
        let Some(&old) = self.keys.get(action) else {
            return Err(Refused::NotBindable);
        };
        let other = self
            .keys
            .iter()
            .position(|&k| k == key)
            .filter(|&o| o != action);
        if let Some(slot) = other.and_then(|o| self.keys.get_mut(o)) {
            *slot = old;
        }
        if let Some(slot) = self.keys.get_mut(action) {
            *slot = key;
        }
        Ok(other)
    }

    /// The game's keys `code` presses (0 for a key bound to nothing).
    pub fn bits(&self, code: KeyCode) -> u8 {
        let find = |c: KeyCode| self.keys.iter().position(|&k| k == c);
        find(code)
            .or_else(|| {
                twin(code)
                    .filter(|&t| find(code).is_none() && key_name(t).is_some())
                    .and_then(find)
            })
            .and_then(|i| ACTIONS.get(i))
            .map_or(0, |a| a.1)
    }

    /// The keys held, with the camera's turn inverted when it is chosen: with Rotation held, left turns right.
    pub fn turn(&self, keys: u8) -> u8 {
        if !self.invert || keys & input::SHIFT == 0 {
            return keys;
        }
        let side = keys & (input::LEFT | input::RIGHT);
        let swapped = (if side & input::LEFT != 0 {
            input::RIGHT
        } else {
            0
        }) | (if side & input::RIGHT != 0 {
            input::LEFT
        } else {
            0
        });
        (keys & !(input::LEFT | input::RIGHT)) | swapped
    }
}

/// What a gamepad's buttons ask of the menus (and Start of the run: the pause).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadPress {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Back,
    Start,
}

/// How far a stick goes before it counts as a key held.
const STICK: f32 = 0.5;

/// The gamepads, when the platform has them (gilrs: evdev on Linux, Windows Gaming Input, IOKit on macOS).
pub struct Pads {
    gilrs: Option<gilrs::Gilrs>,
}

impl Pads {
    pub fn open() -> Pads {
        let gilrs = match gilrs::Gilrs::new() {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("no gamepads: {e}");
                None
            }
        };
        Pads { gilrs }
    }

    /// The buttons pressed since the last call, and the game's keys every pad holds now: the left stick and the
    /// pad's cross roll the ball, the shoulder buttons turn the camera (Rotation with left or right), the lower
    /// face button lifts it.
    pub fn poll(&mut self) -> (u8, Vec<PadPress>) {
        use gilrs::{Axis, Button, EventType};
        let Some(g) = &mut self.gilrs else {
            return (0, Vec::new());
        };
        let mut pressed = Vec::new();
        while let Some(ev) = g.next_event() {
            if let EventType::ButtonPressed(b, _) = ev.event {
                pressed.extend(match b {
                    Button::DPadUp => Some(PadPress::Up),
                    Button::DPadDown => Some(PadPress::Down),
                    Button::DPadLeft => Some(PadPress::Left),
                    Button::DPadRight => Some(PadPress::Right),
                    Button::South => Some(PadPress::Enter),
                    Button::East => Some(PadPress::Back),
                    Button::Start => Some(PadPress::Start),
                    _ => None,
                });
            }
        }
        let mut keys = 0;
        for (_, pad) in g.gamepads() {
            let (x, y) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            let held = |b: Button| pad.is_pressed(b);
            let mut k = 0;
            if held(Button::DPadUp) || y > STICK {
                k |= input::UP;
            }
            if held(Button::DPadDown) || y < -STICK {
                k |= input::DOWN;
            }
            if held(Button::DPadLeft) || x < -STICK {
                k |= input::LEFT;
            }
            if held(Button::DPadRight) || x > STICK {
                k |= input::RIGHT;
            }
            if held(Button::LeftTrigger) {
                k |= input::SHIFT | input::LEFT;
            }
            if held(Button::RightTrigger) {
                k |= input::SHIFT | input::RIGHT;
            }
            if held(Button::South) {
                k |= input::LIFT;
            }
            keys |= k;
        }
        (keys, pressed)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn from(lines: &str) -> impl Fn(&str) -> Option<String> + '_ {
        move |key| {
            lines
                .lines()
                .filter_map(|l| l.split_once(' '))
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_owned())
        }
    }

    #[test]
    fn sizes_are_read_and_refused_out_of_range() {
        assert_eq!(parse_size("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_size(" 1024 * 768 "), Some((1024, 768)));
        assert_eq!(parse_size("800X600"), Some((800, 600)));
        for bad in [
            "",
            "1920",
            "x1080",
            "0x0",
            "10x10",
            "99999x1080",
            "1920x-1",
            "1e3x768",
            "1920x1080x1",
        ] {
            assert_eq!(parse_size(bad), None, "{bad}");
        }
    }

    #[test]
    fn graphics_round_trip_and_bad_values_keep_defaults() {
        let g = Graphics {
            resolution: Some((1280, 720)),
            fullscreen: Fullscreen::Exclusive,
            vsync: true,
            clouds: false,
            fov: 70,
        };
        let text: String = g
            .lines()
            .iter()
            .map(|(k, v)| format!("{k} {v}\n"))
            .collect();
        assert_eq!(Graphics::load(from(&text)), g);
        let bad = "resolution huge\nfullscreen maybe\nvsync 2\nclouds\nfov 500\n";
        assert_eq!(Graphics::load(from(bad)), Graphics::default());
        assert_eq!(Fullscreen::Off.next(-1), Fullscreen::Exclusive);
        assert_eq!(Graphics { fov: 99, ..g }.fov_by(1), 100);
        assert_eq!(Graphics { fov: 40, ..g }.fov_by(-1), 40);
    }

    #[test]
    fn bindings_come_from_the_settings_then_the_copy_then_the_original() {
        let original = Bindings::load(from(""), [68, 69, 70, 71, 39, 53], false);
        assert_eq!(original, Bindings::default());
        // the copy's DB_Options rows: W A S D, Shift, Space as a German keyboard counts them
        let copy = Bindings::load(from(""), [15, 28, 27, 29, 39, 53], true);
        assert_eq!(
            copy.keys[..4],
            [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD]
        );
        assert!(copy.invert);
        let ours = Bindings::load(
            from("key.overview Enter\ninvert-rotation off\n"),
            [15, 28, 27, 29, 39, 53],
            true,
        );
        assert_eq!(ours.keys[5], KeyCode::Enter);
        assert!(!ours.invert);
        // round trip
        let text: String = ours
            .lines()
            .iter()
            .map(|(k, v)| format!("{k} {v}\n"))
            .collect();
        assert_eq!(
            Bindings::load(from(&text), [68, 69, 70, 71, 39, 53], false),
            ours
        );
    }

    #[test]
    fn bad_or_clashing_settings_fall_back() {
        let unknown = Bindings::load(
            from("key.forward Escape\nkey.left Nonsense\n"),
            [68, 69, 70, 71, 39, 53],
            false,
        );
        assert_eq!(unknown, Bindings::default());
        // two actions on one key: the whole set is refused
        let clash = Bindings::load(from("key.forward Space\n"), [68, 69, 70, 71, 39, 53], false);
        assert_eq!(clash, Bindings::default());
        // a copy with rows past the list or twice the same key gives the original's
        assert_eq!(
            Bindings::load(from(""), [999, 69, 70, 71, 39, 53], false),
            Bindings::default()
        );
        assert_eq!(
            Bindings::load(from(""), [68, 68, 70, 71, 39, 53], false),
            Bindings::default()
        );
    }

    #[test]
    fn a_key_in_use_is_swapped_and_reserved_keys_are_refused() {
        let mut b = Bindings::default();
        // Space (overview's) onto forward: overview takes the arrow up
        assert_eq!(b.bind(0, KeyCode::Space), Ok(Some(5)));
        assert_eq!(b.keys[0], KeyCode::Space);
        assert_eq!(b.keys[5], KeyCode::ArrowUp);
        assert!(b.distinct());
        // its own key again changes nothing
        assert_eq!(b.bind(0, KeyCode::Space), Ok(None));
        assert_eq!(b.bind(1, KeyCode::KeyS), Ok(None));
        for k in [
            KeyCode::Escape,
            KeyCode::F11,
            KeyCode::F2,
            KeyCode::F5,
            KeyCode::F9,
            KeyCode::F3,
        ] {
            assert_eq!(b.bind(2, k), Err(Refused::NotBindable), "{k:?}");
        }
        assert_eq!(b.bind(6, KeyCode::KeyQ), Err(Refused::NotBindable));
        assert!(b.distinct());
    }

    #[test]
    fn keys_press_their_bits_and_the_turn_inverts() {
        let mut b = Bindings::default();
        assert_eq!(b.bits(KeyCode::ArrowUp), input::UP);
        // either shift turns the camera while only one is bound
        assert_eq!(b.bits(KeyCode::ShiftRight), input::SHIFT);
        assert_eq!(b.bits(KeyCode::KeyQ), 0);
        b.bind(5, KeyCode::ShiftRight).unwrap();
        assert_eq!(b.bits(KeyCode::ShiftRight), input::LIFT);
        assert_eq!(b.bits(KeyCode::ShiftLeft), input::SHIFT);
        assert_eq!(
            b.turn(input::SHIFT | input::LEFT),
            input::SHIFT | input::LEFT
        );
        b.invert = true;
        assert_eq!(
            b.turn(input::SHIFT | input::LEFT | input::UP),
            input::SHIFT | input::RIGHT | input::UP
        );
        assert_eq!(
            b.turn(input::LEFT),
            input::LEFT,
            "rolling is never inverted"
        );
        assert_eq!(
            b.turn(input::SHIFT | input::LEFT | input::RIGHT),
            input::SHIFT | input::LEFT | input::RIGHT
        );
    }

    #[test]
    fn labels_use_the_copy_names_else_ours() {
        let copy = |row: usize| (row == 53).then(|| b"space".to_vec());
        assert_eq!(key_label(KeyCode::Space, copy), b"space");
        assert_eq!(key_label(KeyCode::KeyQ, copy), b"Q");
        assert_eq!(key_label(KeyCode::Enter, copy), b"Enter");
        assert_eq!(key_label(KeyCode::F7, copy), b"?");
    }
}
