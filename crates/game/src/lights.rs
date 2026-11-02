// The level's passing lights, as the original's scripts drive them. They only light the picture; the run does not
// depend on them, so they are kept by the window and timed by the run's ticks (they stop while it is paused).
//
// - The last level's lightning (Gameplay_Blitz in Gameplay.nmo): four seconds after the start, and then after a
//   random 10 to 90 seconds each time, the directional light Light_Blitz is shown, its colour going from black to
//   white over 200 ms, and hidden; 150 ms after it shows, the thunder ("Donner", Music_thunder) plays.
// - The new ball's light (Ball_Lightning_PointLight in Balls.nmo): a point light 9 units above the ball, reaching
//   20 units and falling off as 1/d, that goes from black to blue over 2.5 s and then from black to white over
//   1.5 s, and is hidden.

use glam::Vec3;

const TICKS_PER_SECOND: u64 = 60;
/// The lightning's first flash after the start (Delayer 4000 ms).
const FIRST_FLASH: u64 = 4 * TICKS_PER_SECOND;
/// The random wait between flashes (Random 10000..90000 ms).
const WAIT: (u64, u64) = (10 * TICKS_PER_SECOND, 90 * TICKS_PER_SECOND);
/// The flash (Bezier Progression 200 ms).
const FLASH_TICKS: u64 = 12;
/// The thunder after the flash shows (Delayer 150 ms), in seconds.
pub const THUNDER_DELAY: f64 = 0.15;
/// Where Light_Blitz shines.
pub const FLASH_DIR: Vec3 = Vec3::new(-0.5165, -0.7209, -0.4621);

/// The new ball's light: its two ramps, its height over the ball and its reach.
const BLUE_TICKS: u64 = 150;
const WHITE_TICKS: u64 = 90;
pub const POINT_LIFT: f32 = 9.0;
pub const POINT_RANGE: f32 = 20.0;

/// The lightning's schedule.
pub struct Blitz {
    seed: u64,
    /// The tick of the flash now showing or last shown, and of the next.
    shown: Option<u64>,
    next: u64,
    /// The last tick seen, to notice a run starting again.
    seen: u64,
}

impl Blitz {
    pub fn new(seed: u64) -> Blitz {
        Blitz {
            seed: seed | 1,
            shown: None,
            next: FIRST_FLASH,
            seen: 0,
        }
    }

    fn wait(&mut self) -> u64 {
        // xorshift: the flashes need no more than to look random
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        WAIT.0 + self.seed % (WAIT.1 - WAIT.0 + 1)
    }

    /// Moves the schedule to `tick`: the flash's brightness now (0: none) and whether a flash began, so the thunder
    /// is due.
    pub fn step(&mut self, tick: u64) -> (f32, bool) {
        if tick < self.seen {
            // the run started again
            self.shown = None;
            self.next = FIRST_FLASH;
        }
        self.seen = tick;
        let mut began = false;
        while tick >= self.next {
            self.shown = Some(self.next);
            let wait = self.wait();
            self.next = self.next.saturating_add(wait);
            began = true;
        }
        let bright = match self.shown {
            Some(at) if tick - at < FLASH_TICKS => (tick - at) as f32 / FLASH_TICKS as f32,
            _ => 0.0,
        };
        (bright, began)
    }
}

/// The new ball's light colour `age` ticks after the ball came; None once it is hidden.
pub fn new_ball(age: u64) -> Option<[f32; 3]> {
    if age < BLUE_TICKS {
        Some([0.0, 0.0, age as f32 / BLUE_TICKS as f32])
    } else if age < BLUE_TICKS + WHITE_TICKS {
        let t = (age - BLUE_TICKS) as f32 / WHITE_TICKS as f32;
        Some([t; 3])
    } else {
        None
    }
}

/// The new ball's light at `tick` of a run with these events, the ball at `ball`: where it is and its colour.
pub fn point(
    events: &[(u64, crate::sim::Event)],
    tick: u64,
    ball: Vec3,
) -> Option<(Vec3, [f32; 3])> {
    let (at, _) = events
        .iter()
        .rev()
        .find(|(_, e)| matches!(e, crate::sim::Event::NewBall))?;
    new_ball(tick.saturating_sub(*at)).map(|c| (ball + Vec3::Y * POINT_LIFT, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lightning_flashes_on_its_schedule() {
        let mut b = Blitz::new(42);
        assert_eq!(b.step(FIRST_FLASH - 1), (0.0, false));
        assert_eq!(b.step(FIRST_FLASH), (0.0, true));
        let (bright, began) = b.step(FIRST_FLASH + 6);
        assert!(!began && (bright - 0.5).abs() < 1e-6);
        assert_eq!(b.step(FIRST_FLASH + FLASH_TICKS), (0.0, false));
        let mut flashes = 0;
        let mut last = FIRST_FLASH;
        for t in FIRST_FLASH + FLASH_TICKS..FIRST_FLASH + 3600 * 60 {
            if b.step(t).1 {
                assert!((WAIT.0..=WAIT.1).contains(&(t - last)));
                last = t;
                flashes += 1;
            }
        }
        assert!(flashes > 40);
        // a run starting again starts the schedule again
        assert_eq!(b.step(1), (0.0, false));
        assert_eq!(b.step(FIRST_FLASH), (0.0, true));
    }

    #[test]
    fn the_new_balls_light_goes_blue_then_white() {
        assert_eq!(new_ball(0), Some([0.0; 3]));
        assert_eq!(new_ball(75), Some([0.0, 0.0, 0.5]));
        assert_eq!(new_ball(BLUE_TICKS + 45), Some([0.5; 3]));
        assert_eq!(new_ball(BLUE_TICKS + WHITE_TICKS), None);
    }
}
