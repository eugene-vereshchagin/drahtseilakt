// The keyboard and the mouse as a map's scripts see them in one tick. It is part of a run's input stream (the game
// records it in the replay, format 11), so scripts never read the live machine: a replay gives them the same keys
// and mouse, tick by tick. Keys are DirectInput scan codes (the original's key codes), 0 to 255; the mouse is in
// the original's 640x480 frame.

/// The mouse buttons, as bits of `InputFrame::buttons`.
pub const LEFT: u8 = 1;
pub const RIGHT: u8 = 2;
pub const MIDDLE: u8 = 4;

/// The frame the mouse position is given in (the original's resolution).
pub const MOUSE_FRAME: (i16, i16) = (640, 480);

/// The keyboard and the mouse at one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputFrame {
    /// The keys down, one bit per scan code.
    pub keys: [u64; 4],
    /// The mouse position in the 640x480 frame, clamped to it.
    pub mouse: [i16; 2],
    /// The mouse buttons down (`LEFT`, `RIGHT`, `MIDDLE`).
    pub buttons: u8,
}

impl InputFrame {
    pub fn down(&self, key: u8) -> bool {
        self.keys
            .get(usize::from(key >> 6))
            .is_some_and(|w| w >> (key & 63) & 1 == 1)
    }

    pub fn set(&mut self, key: u8, on: bool) {
        if let Some(w) = self.keys.get_mut(usize::from(key >> 6)) {
            if on {
                *w |= 1 << (key & 63);
            } else {
                *w &= !(1 << (key & 63));
            }
        }
    }

    /// The keys down, in scan code order.
    pub fn pressed(&self) -> Vec<u8> {
        (0..=255u8).filter(|&k| self.down(k)).collect()
    }

    /// The same frame with the mouse put inside the 640x480 frame.
    pub fn clamped(mut self) -> InputFrame {
        self.mouse[0] = self.mouse[0].clamp(0, MOUSE_FRAME.0);
        self.mouse[1] = self.mouse[1].clamp(0, MOUSE_FRAME.1);
        self.buttons &= LEFT | RIGHT | MIDDLE;
        self
    }
}
