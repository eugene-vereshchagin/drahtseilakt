// Watches a run for four faults: the ball stuck while it is pushed, the ball thrown into the air from the ground, the
// ball moving far faster than it ever does in play, and the ball sinking into the floor (the start of a fall through it). A finding is a fact about one tick, not a judgement about the
// level. The game writes each finding to the level's watch log and saves the replay up to that tick, so the case
// replays exactly with --headless.

use crate::sim::{input, TICK};
use glam::Vec3;

/// Seconds the ball must stay (almost) still while pushed before it counts as stuck.
const STUCK_SECONDS: f32 = 2.0;
/// Below this speed, in units per second, the ball counts as still.
const STILL_SPEED: f32 = 0.3;
/// An upward speed, in units per second, with which the ball leaves the ground. Rolling up a slope gives this too, so
/// it is only the first condition.
const LAUNCH_UP: f32 = 3.0;
/// Ticks in the air before a take-off counts: one or two ticks are the contact flag flickering at a seam.
const LAUNCH_AIR_TICKS: u32 = 6;
/// A speed, in units per second, no rolling ball reaches in play; the report is an error in the physics.
const FAST_SPEED: f32 = 40.0;
/// How deep, in units, the ball may be in the floor: contacts keep it within a few hundredths, so half a unit is a ball
/// going through.
const SINK_DEPTH: f32 = 0.5;

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
    /// Deeper than `SINK_DEPTH` in the floor.
    Sink,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Stuck => "stuck",
            Kind::Launch => "launch",
            Kind::Fast => "fast",
            Kind::Sink => "sink",
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
    /// The upward speed of the last take-off, and the ticks in the air since.
    takeoff_up: Option<f32>,
    air_ticks: u32,
    launch_reported: bool,
    fast_reported: bool,
    sink_reported: bool,
}

impl Watch {
    /// Looks at the state after tick `tick` and returns the findings that begin there. An episode ends when the
    /// fault is gone: the ball moves on, the push stops, the ball lands, it slows down again, or it is out of the
    /// floor. `sink` is how deep the ball is in the floor (its radius less its distance to the nearest floor triangle).
    pub fn observe(&mut self, tick: u64, pos: Vec3, vel: Vec3, input: u8, on_ground: bool, sink: f32) -> Vec<Finding> {
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

        // launch: the ball leaves the ground with an upward speed and stays in the air
        if on_ground {
            self.takeoff_up = None;
            self.air_ticks = 0;
            self.launch_reported = false;
        } else {
            if self.prev_ground && vel.y > LAUNCH_UP {
                self.takeoff_up = Some(vel.y);
            }
            if self.takeoff_up.is_some() {
                self.air_ticks += 1;
                if self.air_ticks >= LAUNCH_AIR_TICKS && !self.launch_reported {
                    self.launch_reported = true;
                    out.push(found(Kind::Launch));
                }
            }
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

        // sink: the ball is deep in the floor
        if sink > SINK_DEPTH {
            if !self.sink_reported {
                self.sink_reported = true;
                out.push(found(Kind::Sink));
            }
        } else {
            self.sink_reported = false;
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
            .flat_map(|t| w.observe(t, Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO, PUSH, true, 0.0))
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
            assert!(w.observe(t, Vec3::ZERO, Vec3::ZERO, 0, true, 0.0).is_empty());
        }
    }

    #[test]
    fn turning_the_camera_with_shift_is_not_a_push() {
        assert!(!pushing(input::SHIFT | input::LEFT));
        assert!(pushing(input::SHIFT | input::UP));
        assert!(pushing(input::LEFT));
    }

    #[test]
    fn a_flight_of_six_ticks_from_an_upward_takeoff_is_a_launch_reported_once() {
        let mut w = Watch::default();
        assert!(w.observe(1, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0), 0, true, 0.0).is_empty());
        let mut found = Vec::new();
        for t in 2..=12 {
            found.extend(w.observe(t, Vec3::ZERO, Vec3::new(5.0, 6.0 - t as f32, 0.0), 0, false, 0.0));
        }
        assert_eq!(found.iter().map(|f| f.kind).collect::<Vec<_>>(), vec![Kind::Launch]);
        assert_eq!(found[0].tick, 7, "the sixth tick in the air");
    }

    #[test]
    fn a_one_tick_flicker_of_the_contact_is_not_a_launch() {
        let mut w = Watch::default();
        w.observe(1, Vec3::ZERO, Vec3::new(13.0, 4.0, 0.0), 0, true, 0.0);
        assert!(w.observe(2, Vec3::ZERO, Vec3::new(13.0, 4.0, 0.0), 0, false, 0.0).is_empty());
        assert!(w.observe(3, Vec3::ZERO, Vec3::new(13.0, 4.0, 0.0), 0, true, 0.0).is_empty());
        assert!(w.observe(4, Vec3::ZERO, Vec3::new(13.0, 4.0, 0.0), 0, false, 0.0).is_empty());
    }

    #[test]
    fn running_off_an_edge_downward_is_not_a_launch() {
        let mut w = Watch::default();
        w.observe(1, Vec3::ZERO, Vec3::new(9.0, 0.0, 0.0), 0, true, 0.0);
        assert!(w.observe(2, Vec3::ZERO, Vec3::new(9.0, -4.0, 0.0), 0, false, 0.0).is_empty());
    }

    #[test]
    fn a_ball_rolling_up_a_slope_on_the_ground_is_not_a_launch() {
        let mut w = Watch::default();
        for t in 1..=60 {
            let vy = 0.5 * t as f32;
            assert!(w.observe(t, Vec3::ZERO, Vec3::new(9.0, vy, 0.0), 0, true, 0.0).is_empty());
        }
    }

    #[test]
    fn a_speed_far_beyond_play_is_fast_once_until_it_drops() {
        let mut w = Watch::default();
        let fast: Vec<Kind> = (1..=10)
            .flat_map(|t| w.observe(t, Vec3::ZERO, Vec3::new(98.0, 0.0, 0.0), 0, true, 0.0))
            .map(|f| f.kind)
            .collect();
        assert_eq!(fast, vec![Kind::Fast]);
        w.observe(11, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0), 0, true, 0.0);
        let again = w.observe(12, Vec3::ZERO, Vec3::new(98.0, 0.0, 0.0), 0, true, 0.0);
        assert_eq!(again.iter().map(|f| f.kind).collect::<Vec<_>>(), vec![Kind::Fast]);
    }

    #[test]
    fn a_ball_deep_in_the_floor_sinks_once_until_it_is_out() {
        let mut w = Watch::default();
        assert!(w.observe(1, Vec3::ZERO, Vec3::ZERO, 0, true, 0.02).is_empty(), "resting contact depth");
        let found: Vec<Kind> = (2..=5)
            .flat_map(|t| w.observe(t, Vec3::ZERO, Vec3::new(0.0, -5.0, 0.0), 0, false, 0.8))
            .map(|f| f.kind)
            .collect();
        assert_eq!(found, vec![Kind::Sink]);
        w.observe(6, Vec3::ZERO, Vec3::ZERO, 0, true, 0.0);
        assert_eq!(w.observe(7, Vec3::ZERO, Vec3::ZERO, 0, false, 1.0).len(), 1);
    }
}
