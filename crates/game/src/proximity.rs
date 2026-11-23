// The original's "TT Scaleable Proximity" building block (TT_Toolbox_RT), which the module scripts use to start
// things when the ball comes near: the distance between two points, tested every N frames, N going linearly from a
// minimum near to a maximum far, and Enter / In / Exit / Out range events. Ported from the C reference
// (src/bb/bb_interface.c); the distances are compared squared, as the block does by default, so N is linear in the
// squared distance, and only over the axes its setting names (the modules' wake-ups measure across, x and z).
// Reason for the allow: distance-based gain maths over fixed constants and the simulation's ticks.
#![allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

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

/// The axes a proximity measures over (its setting: bit 1 x, 2 y, 4 z).
pub const ACROSS: u8 = 5;
pub const ALL_AXES: u8 = 7;

#[derive(Clone, Copy, Debug)]
pub struct Proximity {
    pub distance: f32,
    axes: u8,
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
    /// A block switched off, with its distance, the axes it measures over and its exactness.
    pub const fn new(
        distance: f32,
        axes: u8,
        near: f32,
        far: f32,
        frames: (u32, u32),
    ) -> Proximity {
        Proximity {
            distance,
            axes,
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

    /// Frames until the next test (the block's "Last Check"), which it keeps while switched off.
    pub fn countdown(&self) -> u32 {
        self.countdown
    }

    pub fn set_countdown(&mut self, frames: u32) {
        self.countdown = frames;
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
        let d = b - a;
        let d = Vec3::new(
            if self.axes & 1 != 0 { d.x } else { 0.0 },
            if self.axes & 2 != 0 { d.y } else { 0.0 },
            if self.axes & 4 != 0 { d.z } else { 0.0 },
        );
        let d2 = d.length_squared();
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
        let mut p = Proximity::new(10.0, ALL_AXES, 20.0, 40.0, (2, 10));
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 50.0), None, "off");
        p.switch_on();
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 50.0), Some(Range::Exit));
        // beyond 40 it waits 10 frames
        for _ in 0..9 {
            assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 50.0), None);
        }
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 5.0), Some(Range::Enter));
        assert_eq!(
            p.frame(Vec3::ZERO, Vec3::X * 5.0),
            None,
            "every 2 frames within 20"
        );
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 5.0), Some(Range::In));
        // half way in the squared distance between 20 and 40: 2 + 8 * (900 - 400) / 1200, truncated
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 30.0), None);
        assert_eq!(p.frame(Vec3::ZERO, Vec3::X * 30.0), Some(Range::Exit));
        assert_eq!(p.countdown, 5);
    }

    #[test]
    fn across_ignores_the_height() {
        let mut p = Proximity::new(10.0, ACROSS, 20.0, 40.0, (1, 1));
        p.switch_on();
        assert_eq!(
            p.frame(Vec3::ZERO, Vec3::new(3.0, 500.0, 4.0)),
            Some(Range::Enter)
        );
    }
}
