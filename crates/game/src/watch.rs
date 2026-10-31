// Watches a run for three faults: the ball stuck while it is pushed, the ball thrown into the air from the ground, and
// the ball moving far faster than it ever does in play. A finding is a fact about one tick, not a judgement about the
// level. The game writes each finding to the level's watch log and saves the replay up to that tick, so the case
// replays exactly with --headless.

use crate::sim::{input, TICK};
use glam::Vec3;

/// Seconds the ball must stay (almost) still while pushed before it counts as stuck.
const STUCK_SECONDS: f32 = 2.0;
/// Below this speed, in units per second, the ball counts as still.
const STILL_SPEED: f32 = 0.3;
/// An upward speed, in units per second, with which the ball leaves the ground. Rolling over seams stays well below.
const LAUNCH_UP: f32 = 3.0;
/// A speed, in units per second, no rolling ball reaches in play; the report is an error in the physics.
const FAST_SPEED: f32 = 40.0;

fn stuck_ticks() -> u32 {
    (STUCK_SECONDS / TICK).round() as u32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Pushed for `STUCK_SECONDS` without moving.
    Stuck,
    /// Left the ground with an upward speed.
    Launch,
    /// Faster than `FAST_SPEED`.
    Fast,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Stuck => "stuck",
            Kind::Launch => "launch",
            Kind::Fast => "fast",
        }
    }
}

/// One finding: what happened, at which tick, and the ball's state there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Finding {
    pub kind: Kind,
    pub tick: u64,
    pub pos: Vec3,
    pub vel: Vec3,
    pub input: u8,
    pub on_ground: bool,
}

/// Whether the input pushes the ball: the arrows, except Left and Right while Shift turns the camera.
pub fn pushing(input: u8) -> bool {
    input & (input::UP | input::DOWN) != 0 || (input & input::SHIFT == 0 && input & (input::LEFT | input::RIGHT) != 0)
}

/// The state of one run's watch: each fault is reported once per episode.
#[derive(Debug, Default)]
pub struct Watch {
    still_ticks: u32,
    stuck_reported: bool,
    prev_ground: bool,
    launch_reported: bool,
    fast_reported: bool,
}

impl Watch {
    /// Looks at the state after tick `tick` and returns the findings that begin there. An episode ends when the
    /// fault is gone: the ball moves on, the push stops, the ball lands, or it slows down again.
    pub fn observe(&mut self, tick: u64, pos: Vec3, vel: Vec3, input: u8, on_ground: bool) -> Vec<Finding> {
        let found = |kind| Finding { kind, tick, pos, vel, input, on_ground };
        let mut out = Vec::new();

        // stuck: pushed and still, for a while
        if pushing(input) && vel.length() < STILL_SPEED {
            self.still_ticks += 1;
            if !self.stuck_reported && self.still_ticks >= stuck_ticks() {
                self.stuck_reported = true;
                out.push(found(Kind::Stuck));
            }
        } else {
            self.still_ticks = 0;
            self.stuck_reported = false;
        }

        // launch: the ball leaves the ground with an upward speed
        if self.prev_ground && !on_ground && vel.y > LAUNCH_UP {
            if !self.launch_reported {
                self.launch_reported = true;
                out.push(found(Kind::Launch));
            }
        } else if on_ground {
            self.launch_reported = false;
        }
        self.prev_ground = on_ground;

        // fast: far beyond what the ball does in play
        if vel.length() > FAST_SPEED {
            if !self.fast_reported {
                self.fast_reported = true;
                out.push(found(Kind::Fast));
            }
        } else {
            self.fast_reported = false;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUSH: u8 = input::RIGHT;

    #[test]
    fn a_pushed_ball_that_does_not_move_is_stuck_once() {
        let mut w = Watch::default();
        let found: Vec<Finding> = (1..=400)
            .flat_map(|t| w.observe(t, Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO, PUSH, true))
            .collect();
        assert_eq!(found.len(), 1, "one report for the episode");
        assert_eq!(found[0].kind, Kind::Stuck);
        assert_eq!(found[0].tick, u64::from(stuck_ticks()));
        assert_eq!(found[0].pos, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn a_ball_that_is_not_pushed_is_not_stuck() {
        let mut w = Watch::default();
        for t in 1..=400 {
            assert!(w.observe(t, Vec3::ZERO, Vec3::ZERO, 0, true).is_empty());
        }
    }

    #[test]
    fn turning_the_camera_with_shift_is_not_a_push() {
        assert!(!pushing(input::SHIFT | input::LEFT));
        assert!(pushing(input::SHIFT | input::UP));
        assert!(pushing(input::LEFT));
    }

    #[test]
    fn leaving_the_ground_upward_is_a_launch_reported_once() {
        let mut w = Watch::default();
        assert!(w.observe(1, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0), 0, true).is_empty());
        let f = w.observe(2, Vec3::ZERO, Vec3::new(5.0, 6.0, 0.0), 0, false);
        assert_eq!(f.iter().map(|f| f.kind).collect::<Vec<_>>(), vec![Kind::Launch]);
        assert!(w.observe(3, Vec3::ZERO, Vec3::new(5.0, 6.5, 0.0), 0, false).is_empty(), "reported once");
    }

    #[test]
    fn running_off_an_edge_downward_is_not_a_launch() {
        let mut w = Watch::default();
        w.observe(1, Vec3::ZERO, Vec3::new(9.0, 0.0, 0.0), 0, true);
        assert!(w.observe(2, Vec3::ZERO, Vec3::new(9.0, -4.0, 0.0), 0, false).is_empty());
    }

    #[test]
    fn a_ball_rolling_up_a_slope_on_the_ground_is_not_a_launch() {
        let mut w = Watch::default();
        for t in 1..=60 {
            let vy = 0.5 * t as f32;
            assert!(w.observe(t, Vec3::ZERO, Vec3::new(9.0, vy, 0.0), 0, true).is_empty());
        }
    }

    #[test]
    fn a_speed_far_beyond_play_is_fast_once_until_it_drops() {
        let mut w = Watch::default();
        let fast: Vec<Kind> = (1..=10)
            .flat_map(|t| w.observe(t, Vec3::ZERO, Vec3::new(98.0, 0.0, 0.0), 0, true))
            .map(|f| f.kind)
            .collect();
        assert_eq!(fast, vec![Kind::Fast]);
        w.observe(11, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0), 0, true);
        let again = w.observe(12, Vec3::ZERO, Vec3::new(98.0, 0.0, 0.0), 0, true);
        assert_eq!(again.iter().map(|f| f.kind).collect::<Vec<_>>(), vec![Kind::Fast]);
    }
}
