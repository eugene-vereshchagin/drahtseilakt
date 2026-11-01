// The original's "TT Scaleable Proximity" building block (TT_Toolbox_RT), which the module scripts use to start
// things when the ball comes near: the distance between two points, tested every N frames, N going linearly from a
// minimum near to a maximum far, and Enter / In / Exit / Out range events. Ported from the C reference
// (src/bb/bb_interface.c); the distances are compared squared, as the block does by default, so N is linear in the
// squared distance.

use glam::Vec3;

/// What one test found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Range {
    /// Within the distance, and it was not (or the block was just switched on).
    Enter,
    /// Within, as before.
    In,
    /// Beyond the distance, and it was within (or the block was just switched on).
    Exit,
    /// Beyond, as before.
    Out,
}

#[derive(Clone, Copy, Debug)]
pub struct Proximity {
    pub distance: f32,
    /// The exactness: tested every `frames.0` frames within `near`, every `frames.1` beyond `far`.
    near: f32,
    far: f32,
    frames: (u32, u32),
    /// Frames until the next test.
    countdown: u32,
    /// The last result: within or not; `None` after it is switched on.
    within: Option<bool>,
    pub on: bool,
}

impl Proximity {
    /// A block switched off, with its distance and exactness.
    pub const fn new(distance: f32, near: f32, far: f32, frames: (u32, u32)) -> Proximity {
        Proximity {
            distance,
            near,
            far,
            frames,
            countdown: 0,
            within: None,
            on: false,
        }
    }

    /// The In input: it tests from now on, and the first test gives Enter or Exit.
    pub fn switch_on(&mut self) {
        self.on = true;
        self.within = None;
    }

    /// The Off input.
    pub fn switch_off(&mut self) {
        self.on = false;
    }

    /// One frame with `a` and `b` where they are: the result if it tested this frame.
    pub fn frame(&mut self, a: Vec3, b: Vec3) -> Option<Range> {
        if !self.on {
            return None;
        }
        if self.countdown > 1 {
            self.countdown -= 1;
            return None;
        }
        let d2 = a.distance_squared(b);
        let (near, far) = (self.near * self.near, self.far * self.far);
        let (f0, f1) = self.frames;
        let n = if d2 >= far {
            f1
        } else if d2 <= near {
            f0
        } else {
            // truncated, as the block's integer conversion
            ((d2 - near) / (far - near) * (f1 as f32 - f0 as f32)) as i64 as u32 + f0
        };
        self.countdown = n.max(1);
        let inside = d2 < self.distance * self.distance;
        let r = match (inside, self.within) {
            (true, Some(true)) => Range::In,
            (true, _) => Range::Enter,
            (false, Some(false)) => Range::Out,
            (false, _) => Range::Exit,
        };
        self.within = Some(inside);
        Some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_tests_less_often_far_away_and_reports_entering_once() {
        let mut p = Proximity::new(10.0, 20.0, 40.0, (2, 10));
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 50.0), None, "off");
        p.switch_on();
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 50.0), Some(Range::Exit));
        // beyond 40 it waits 10 frames
        for _ in 0..9 {
            assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 50.0), None);
        }
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 5.0), Some(Range::Enter));
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 5.0), None, "every 2 frames within 20");
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 5.0), Some(Range::In));
        // half way in the squared distance between 20 and 40: 2 + 8 * (900 - 400) / 1200, truncated
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 30.0), None);
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 30.0), Some(Range::Exit));
        assert_eq!(p.countdown, 5);
    }
}
