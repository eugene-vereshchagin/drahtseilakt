// Controllers.dll: keyboard and mouse. Every block reads the keys and the mouse of the current tick through the
// host, which takes them from the run's input stream (never the live machine), so a replay gives the scripts the
// same input. Semantics: docs/gameplay_bbs.md, "Input blocks".

use super::*;
use crate::input::{LEFT, MIDDLE, RIGHT};

/// A key code as a block's input parameter holds it.
fn key_code(rt: &mut Runtime, b: &Block, i: usize, host: &mut dyn Host) -> u8 {
    match b.pins.get(i) {
        Some(&p) => (rt.read(p, host).as_int() & 0xff) as u8,
        None => 0,
    }
}

pub(crate) fn key_event(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 1) {
        return Flow::Done;
    }
    let mut state = false;
    if input(rt, b, 0) {
        if let Some(&p) = b.pouts.first() {
            rt.write(p, Value::Bool(false));
        }
    } else if let Some(Value::Bool(s)) = pout_value(rt, b, 0) {
        state = s;
    }
    let now = host.input();
    let all = (0..b.pins.len()).all(|i| now.down(key_code(rt, b, i, host)));
    if all && !state {
        state = true;
        out(rt, b, 0);
    } else if !all && state {
        state = false;
        out(rt, b, 1);
    }
    if let Some(&p) = b.pouts.first() {
        rt.write(p, Value::Bool(state));
    }
    Flow::Continue
}

/// Whether `k` went down in this tick.
fn just_pressed(now: &crate::InputFrame, before: &crate::InputFrame, k: u8) -> bool {
    now.down(k) && !before.down(k)
}

pub(crate) fn key_waiter(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !active(rt, b, 0) {
        return Flow::Done;
    }
    let (now, before) = (host.input(), host.input_before());
    if local(rt, b, 0).as_bool() {
        if let Some(k) = (1..=255u8).find(|&k| just_pressed(&now, &before, k)) {
            input(rt, b, 0);
            out(rt, b, 0);
            if let Some(&p) = b.pouts.first() {
                rt.write(p, Value::Int(i32::from(k)));
            }
            return Flow::Done;
        }
        return Flow::Continue;
    }
    if now.down(key_code(rt, b, 0, host)) {
        input(rt, b, 0);
        out(rt, b, 0);
        return Flow::Done;
    }
    Flow::Continue
}

/// Switch On Key: output i fires in the tick pin i's key is just pressed. Stays active until Off.
pub(crate) fn switch_on_key(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 1) {
        return Flow::Done;
    }
    input(rt, b, 0);
    let (now, before) = (host.input(), host.input_before());
    for i in 0..b.pins.len().min(b.outputs.len()) {
        if just_pressed(&now, &before, key_code(rt, b, i, host)) {
            out(rt, b, i);
        }
    }
    Flow::Continue
}

pub(crate) fn mouse_waiter(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let has_off = match local(rt, b, 0) {
        Value::None => true,
        v => v.as_bool(),
    };
    let mask = match local(rt, b, 1) {
        Value::None => 0xffff,
        v => v.as_int(),
    };
    input(rt, b, 0);
    if has_off && input(rt, b, 1) {
        return Flow::Done;
    }
    let prev = match pout_value(rt, b, 0) {
        Some(Value::Vec2(p)) => p,
        _ => [0.0, 0.0],
    };
    let m = host.input();
    let pos = [f32::from(m.mouse[0]), f32::from(m.mouse[1])];
    set_pout(rt, b, "Mouse Position", 0, Value::Vec2(pos));
    set_pout(rt, b, "Wheel Direction", 1, Value::Float(0.0));
    // per button, in the order of the outputs (left, right, middle): the output parameter holding its value, its
    // bit, and its value before this tick
    let mut edges = Vec::new();
    for (po, bit) in [(2usize, LEFT), (4, RIGHT), (3, MIDDLE)] {
        let was = pout_value(rt, b, po).is_some_and(|v| v.as_bool());
        let is = m.buttons & bit != 0;
        if let Some(&p) = b.pouts.get(po) {
            rt.write(p, Value::Bool(is));
        }
        edges.push((was, is));
    }
    // the outputs that exist, by the mask's bits: Move, then Pressed and Released of each button
    let mut want: Vec<bool> = vec![prev != pos];
    for &(was, is) in &edges {
        want.push(!was && is);
        want.push(was && !is);
    }
    let mut next = 0usize;
    for (bit, fire) in want.into_iter().enumerate() {
        if mask >> bit & 1 != 0 {
            if fire {
                out(rt, b, next);
            }
            next = next.saturating_add(1);
        }
    }
    Flow::Continue
}

/// The most characters Input String holds.
pub const MAX_TEXT: usize = 256;

const LOWER: &[u8; 0x3a] =
    b"\0\x1b1234567890-=\x08\tqwertyuiop[]\r\0asdfghjkl;'`\0\\zxcvbnm,./\0*\0 ";
const UPPER: &[u8; 0x3a] =
    b"\0\x1b!@#$%^&*()_+\x08\tQWERTYUIOP{}\r\0ASDFGHJKL:\"~\0|ZXCVBNM<>?\0*\0 ";

/// The character of a key on a US layout, none for keys without one.
fn ascii_of(k: u8, shift: bool) -> Option<char> {
    let t = if shift { UPPER } else { LOWER };
    t.get(usize::from(k))
        .copied()
        .filter(|&c| c != 0 && c != 0x1b && c != 0x08 && c != b'\t' && c != b'\r')
        .map(char::from)
}

/// The keypad's characters.
fn keypad_char(k: u8) -> Option<char> {
    Some(match k {
        0x37 => '*',
        0x47 => '7',
        0x48 => '8',
        0x49 => '9',
        0x4a => '-',
        0x4b => '4',
        0x4c => '5',
        0x4d => '6',
        0x4e => '+',
        0x4f => '1',
        0x50 => '2',
        0x51 => '3',
        0x52 => '0',
        0x53 => '.',
        0x8d => '=',
        0xb3 => ',',
        0xb5 => '/',
        _ => return None,
    })
}

/// The text with the caret's mark (a backspace character) where the caret is.
fn marked(text: &str, caret: usize) -> String {
    let mut s: String = text.chars().take(caret).collect();
    s.push('\u{8}');
    s.extend(text.chars().skip(caret));
    s
}

fn max_size(rt: &Runtime, b: &Block) -> usize {
    match local(rt, b, 0) {
        Value::None => 32,
        v => usize::try_from(v.as_int()).unwrap_or(1).clamp(1, MAX_TEXT),
    }
}

/// Input String's text and caret after one key, and whether it changed.
fn edit(
    text: &mut Vec<char>,
    caret: &mut usize,
    k: u8,
    shift: bool,
    multiline: bool,
    section: i32,
    max: usize,
) -> bool {
    match k {
        0x0e => {
            if *caret > 0 {
                *caret = caret.saturating_sub(1);
                text.remove(*caret);
                return true;
            }
        }
        0xd3 => {
            if *caret < text.len() {
                text.remove(*caret);
                return true;
            }
        }
        0xcb => {
            if section & 2 == 0 && *caret > 0 {
                *caret = caret.saturating_sub(1);
                return true;
            }
        }
        0xcd => {
            if section & 2 == 0 && *caret < text.len() {
                *caret = caret.saturating_add(1);
                return true;
            }
        }
        0xc7 | 0xc9 | 0xcf | 0xd1 => {}
        _ => {
            let ch = if k == 0x1c || k == 0x9c {
                multiline.then_some('\n')
            } else if let Some(c) = keypad_char(k) {
                (section & 1 == 0).then_some(c)
            } else {
                ascii_of(k, shift)
            };
            if let Some(c) = ch {
                // a full text takes no character at its end; elsewhere the tail is pushed out
                if text.len() >= max && *caret >= text.len() {
                    return false;
                }
                text.insert(*caret, c);
                text.truncate(max);
                *caret = caret.saturating_add(1).min(max);
                return true;
            }
        }
    }
    false
}

pub(crate) fn input_string(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let max = max_size(rt, b);
    let use_caret = local(rt, b, 4).as_bool();
    let (mut text, mut caret, mut on) = match rt.scratch(id) {
        Scratch::Typing { text, caret, on } => (text.chars().collect::<Vec<_>>(), *caret, *on),
        _ => (Vec::new(), 0, false),
    };
    let shown = |t: &[char], c: usize| {
        let s: String = t.iter().collect();
        if use_caret { marked(&s, c) } else { s }
    };
    let save = |rt: &mut Runtime, t: &[char], c: usize, on: bool| {
        rt.set_scratch(
            id,
            Scratch::Typing {
                text: t.iter().collect(),
                caret: c,
                on,
            },
        );
    };
    if input(rt, b, 1) {
        if on {
            set_pout(rt, b, "String", 0, Value::Str(text.iter().collect()));
            on = false;
        }
        save(rt, &text, caret, on);
        out(rt, b, 1);
        return Flow::Done;
    }
    if input(rt, b, 0) {
        let reset = pin(rt, b, "Reset String", 0, host).as_string();
        text = reset.chars().filter(|&c| c != '\u{8}').take(max).collect();
        caret = text.len();
        on = true;
        save(rt, &text, caret, on);
        set_pout(rt, b, "String", 0, Value::Str(shown(&text, caret)));
        out(rt, b, 0);
        return Flow::Continue;
    }
    if !on {
        // never switched on (the block ran by itself): it waits for its On
        return Flow::Continue;
    }
    let multiline = local(rt, b, 2).as_bool();
    let section = local(rt, b, 3).as_int();
    let end_key = if multiline {
        key_code(rt, b, 1, host)
    } else {
        0
    };
    let (now, before) = (host.input(), host.input_before());
    let shift = now.down(0x2a) || now.down(0x36);
    let mut changed = false;
    for k in (1..=255u8).filter(|&k| just_pressed(&now, &before, k)) {
        set_pout(rt, b, "Key", 1, Value::Int(i32::from(k)));
        let ends = if multiline {
            end_key != 0 && k == end_key
        } else {
            k == 0x1c || k == 0x9c
        };
        if ends {
            set_pout(rt, b, "String", 0, Value::Str(text.iter().collect()));
            save(rt, &text, caret, false);
            out(rt, b, 1);
            return Flow::Done;
        }
        changed |= edit(&mut text, &mut caret, k, shift, multiline, section, max);
    }
    if changed {
        set_pout(rt, b, "String", 0, Value::Str(shown(&text, caret)));
        out(rt, b, 2);
    }
    save(rt, &text, caret, on);
    Flow::Continue
}

pub(crate) fn show_mouse_cursor(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    let show = pin(rt, b, "Show", 0, host);
    out(rt, b, 0);
    host.show_cursor(show.as_bool());
    Flow::Done
}

/// Mouse Cursor and Mouse Cursor System: the engine draws its own interface, so they only pass the flow on, and
/// show or hide the system cursor when their first parameter is a Boolean.
pub(crate) fn mouse_cursor(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    if let Value::Bool(show) = pin(rt, b, "Show", 0, host) {
        host.show_cursor(show);
    }
    out(rt, b, 0);
    Flow::Done
}

const fn declare(
    guid: Guid,
    name: &'static str,
    pins: &'static [&'static str],
    pouts: &'static [&'static str],
    flags: u32,
    run: Run,
) -> Proto {
    Proto {
        guid,
        name,
        library: Library::Controllers,
        pins,
        pouts,
        flags,
        standing: Standing::Full,
        layouts: &[],
        run,
        event: None,
    }
}

pub(super) const PROTOS: &[Proto] = &[
    declare(
        proto::KEY_EVENT,
        "Key Event",
        &[],
        &["Key State"],
        flags::VAR_PINS,
        key_event,
    ),
    declare(
        proto::KEY_WAITER,
        "Key Waiter",
        &["Key"],
        &["Key"],
        0,
        key_waiter,
    ),
    declare(
        proto::SWITCH_ON_KEY,
        "Switch On Key",
        &[],
        &[],
        flags::VAR_PINS | flags::VAR_OUTPUTS,
        switch_on_key,
    ),
    declare(
        proto::MOUSE_WAITER,
        "Mouse Waiter",
        &[],
        &[
            "Mouse Position",
            "Wheel Direction",
            "Left Button",
            "Middle Button",
            "Right Button",
        ],
        flags::VAR_OUTPUTS,
        mouse_waiter,
    ),
    declare(
        proto::INPUT_STRING,
        "Input String",
        &["Reset String", "End Key"],
        &["String", "Key"],
        0,
        input_string,
    ),
    declare(
        proto::SHOW_MOUSE_CURSOR,
        "Show Mouse Cursor",
        &["Show"],
        &[],
        0,
        show_mouse_cursor,
    ),
    declare(
        proto::MOUSE_CURSOR_SYSTEM,
        "Mouse Cursor System",
        &["Show"],
        &[],
        0,
        mouse_cursor,
    ),
    declare(
        proto::MOUSE_CURSOR,
        "Mouse Cursor",
        &["Show"],
        &[],
        0,
        mouse_cursor,
    ),
];
