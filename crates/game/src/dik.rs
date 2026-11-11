// The window's physical keys as DirectInput scan codes, the key codes a map's scripts read (the original's input
// manager), and the mouse in the original's 640x480 frame. Only the live window uses this; a run keeps the result
// in its replay (replay format 11), so playing a replay never looks at the machine.

use script::InputFrame;
use script::input::{LEFT, MIDDLE, MOUSE_FRAME, RIGHT};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// The scan code of a key, none for a key the original's keyboard had no code for here.
pub fn scan_code(code: KeyCode) -> Option<u8> {
    use KeyCode::*;
    Some(match code {
        Escape => 0x01,
        Digit1 => 0x02,
        Digit2 => 0x03,
        Digit3 => 0x04,
        Digit4 => 0x05,
        Digit5 => 0x06,
        Digit6 => 0x07,
        Digit7 => 0x08,
        Digit8 => 0x09,
        Digit9 => 0x0a,
        Digit0 => 0x0b,
        Minus => 0x0c,
        Equal => 0x0d,
        Backspace => 0x0e,
        Tab => 0x0f,
        KeyQ => 0x10,
        KeyW => 0x11,
        KeyE => 0x12,
        KeyR => 0x13,
        KeyT => 0x14,
        KeyY => 0x15,
        KeyU => 0x16,
        KeyI => 0x17,
        KeyO => 0x18,
        KeyP => 0x19,
        BracketLeft => 0x1a,
        BracketRight => 0x1b,
        Enter => 0x1c,
        ControlLeft => 0x1d,
        KeyA => 0x1e,
        KeyS => 0x1f,
        KeyD => 0x20,
        KeyF => 0x21,
        KeyG => 0x22,
        KeyH => 0x23,
        KeyJ => 0x24,
        KeyK => 0x25,
        KeyL => 0x26,
        Semicolon => 0x27,
        Quote => 0x28,
        Backquote => 0x29,
        ShiftLeft => 0x2a,
        Backslash => 0x2b,
        KeyZ => 0x2c,
        KeyX => 0x2d,
        KeyC => 0x2e,
        KeyV => 0x2f,
        KeyB => 0x30,
        KeyN => 0x31,
        KeyM => 0x32,
        Comma => 0x33,
        Period => 0x34,
        Slash => 0x35,
        ShiftRight => 0x36,
        NumpadMultiply => 0x37,
        AltLeft => 0x38,
        Space => 0x39,
        CapsLock => 0x3a,
        F1 => 0x3b,
        F2 => 0x3c,
        F3 => 0x3d,
        F4 => 0x3e,
        F5 => 0x3f,
        F6 => 0x40,
        F7 => 0x41,
        F8 => 0x42,
        F9 => 0x43,
        F10 => 0x44,
        NumLock => 0x45,
        ScrollLock => 0x46,
        Numpad7 => 0x47,
        Numpad8 => 0x48,
        Numpad9 => 0x49,
        NumpadSubtract => 0x4a,
        Numpad4 => 0x4b,
        Numpad5 => 0x4c,
        Numpad6 => 0x4d,
        NumpadAdd => 0x4e,
        Numpad1 => 0x4f,
        Numpad2 => 0x50,
        Numpad3 => 0x51,
        Numpad0 => 0x52,
        NumpadDecimal => 0x53,
        F11 => 0x57,
        F12 => 0x58,
        NumpadEnter => 0x9c,
        ControlRight => 0x9d,
        NumpadDivide => 0xb5,
        AltRight => 0xb8,
        Home => 0xc7,
        ArrowUp => 0xc8,
        PageUp => 0xc9,
        ArrowLeft => 0xcb,
        ArrowRight => 0xcd,
        End => 0xcf,
        ArrowDown => 0xd0,
        PageDown => 0xd1,
        Insert => 0xd2,
        Delete => 0xd3,
        _ => return None,
    })
}

/// The mouse button's bit in `InputFrame::buttons`.
pub fn button_bit(b: MouseButton) -> u8 {
    match b {
        MouseButton::Left => LEFT,
        MouseButton::Right => RIGHT,
        MouseButton::Middle => MIDDLE,
        _ => 0,
    }
}

/// The window's mouse position (pixels, in a window of the given size) in the original's 640x480 frame, whole
/// window mapped to the frame; (0, 0) while the cursor is outside.
pub fn mouse_in_frame(cursor: Option<(f64, f64)>, size: (u32, u32)) -> [i16; 2] {
    let Some((x, y)) = cursor else {
        return [0, 0];
    };
    let scale = |v: f64, n: u32, frame: i16| {
        if n == 0 || !v.is_finite() {
            return 0;
        }
        (v / f64::from(n) * f64::from(frame))
            .round()
            .clamp(0.0, f64::from(frame)) as i16
    };
    [
        scale(x, size.0, MOUSE_FRAME.0),
        scale(y, size.1, MOUSE_FRAME.1),
    ]
}

/// The live keyboard and mouse as a frame.
pub fn frame_of(
    held: &std::collections::HashSet<KeyCode>,
    cursor: Option<(f64, f64)>,
    size: (u32, u32),
    buttons: u8,
) -> InputFrame {
    let mut f = InputFrame::default();
    for &k in held {
        if let Some(c) = scan_code(k) {
            f.set(c, true);
        }
    }
    f.mouse = mouse_in_frame(cursor, size);
    f.buttons = buttons;
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_directinput_codes_and_the_mouse_to_the_original_frame() {
        assert_eq!(scan_code(KeyCode::Enter), Some(0x1c));
        assert_eq!(scan_code(KeyCode::ArrowUp), Some(0xc8));
        assert_eq!(scan_code(KeyCode::KeyA), Some(0x1e));
        assert_eq!(scan_code(KeyCode::MediaPlayPause), None);
        assert_eq!(
            mouse_in_frame(Some((960.0, 540.0)), (1920, 1080)),
            [320, 240]
        );
        assert_eq!(mouse_in_frame(Some((5000.0, -4.0)), (1920, 1080)), [640, 0]);
        assert_eq!(mouse_in_frame(None, (1920, 1080)), [0, 0]);
        let mut held = std::collections::HashSet::new();
        held.insert(KeyCode::Space);
        let f = frame_of(&held, Some((0.0, 0.0)), (100, 100), 3);
        assert!(f.down(0x39) && !f.down(0x1c));
        assert_eq!(f.buttons, 3);
    }
}
