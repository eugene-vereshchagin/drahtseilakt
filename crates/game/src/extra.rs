// The extra points pickup, as the original's TT Extra building block runs it (docs/gameplay_bbs.md, TT Extra):
// six small balls orbit the pickup; when the ball comes within 3 they fly away for a second, then home in on the
// ball one by one, each that reaches it counting as a hit. The pickup gives 100 points when it fires and 20 for each
// hit (C game, level 1: an extra point moved onto the ball gave 100 at once and 220 in all).

use glam::{Quat, Vec3};

/// Within this of the pickup (squared, as the block compares) it fires.
const ACTIVATION: f32 = 3.0;
/// A small ball within this squared distance of the ball hits it (the pin is compared squared, so 2 units).
const COLL_DISTANCE_SQ: f32 = 4.0;
/// Orbit speed (rad/s), the radius the small balls keep, the fly-away push and its velocity retention, the homing
/// gains (Force + i × Forcewidth / N) and the homing velocity retention, and the fly-away time.
const ROTATION: f32 = 5.0;
const RADIUS: f32 = 2.0;
const AWAY_FORCE: f32 = 1.0;
const AWAY_DAMPING: f32 = 0.3;
const FORCE: f32 = 0.12;
const FORCE_WIDTH: f32 = 0.08;
const DAMPING: f32 = 0.95;
const FLY_AWAY_MS: f32 = 1000.0;
/// The block's time value: the frame time, sampled every second frame (1/60 s at 60 frames a second).
const TIME_VALUE: f32 = 1.0 / 60.0;
const FRAME_MS: f32 = 1000.0 / 60.0;
/// The points for firing and for each hit.
pub const POINTS_ACTIVATED: i64 = 100;
pub const POINTS_HIT: i64 = 20;

/// The orbit axis of each small ball (a table in the block; not normalised, as the original builds its matrix).
const AXES: [Vec3; 6] = [
    Vec3::new(0.0, 1.0, 0.0),
    Vec3::new(0.0, 0.0, 1.0),
    Vec3::new(1.0, 0.0, 0.0),
    Vec3::new(0.5, 0.5, 0.707),
    Vec3::new(0.707, -0.707, 0.0),
    Vec3::new(-0.5, -0.5, 0.707),
];

#[derive(Clone, Debug)]
struct SmallBall {
    /// World position, and where it was the frame before (its velocity memory).
    pos: Vec3,
    saved: Vec3,
    gain: f32,
    alive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Orbit,
    FlyAway,
    HomeIn,
    Done,
}

/// What a pickup did in a tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtraEvent {
    Activated,
    Hits(u32),
}

#[derive(Clone, Debug)]
pub struct Extra {
    /// The pickup's level object, its place and turn.
    pub object: usize,
    center: Vec3,
    rot: Quat,
    balls: Vec<SmallBall>,
    state: State,
    timer: f32,
    away: Vec3,
}

impl Extra {
    /// A pickup at `center` turned by `rot`, its small balls at `local` (the module's frames, in its own space).
    pub fn new(object: usize, center: Vec3, rot: Quat, local: &[Vec3]) -> Extra {
        let n = local.len().max(1) as f32;
        let balls = local
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                let pos = center + rot * p;
                SmallBall {
                    pos,
                    saved: pos,
                    gain: FORCE + i as f32 * FORCE_WIDTH / n,
                    alive: true,
                }
            })
            .collect();
        Extra {
            object,
            center,
            rot,
            balls,
            state: State::Orbit,
            timer: FLY_AWAY_MS,
            away: Vec3::ZERO,
        }
    }

    /// Where the pickup is.
    pub fn center(&self) -> Vec3 {
        self.center
    }

    /// Whether it has given all it gives.
    pub fn done(&self) -> bool {
        self.state == State::Done
    }

    /// Whether it has fired (the pickup itself is gone).
    pub fn fired(&self) -> bool {
        self.state != State::Orbit
    }

    /// The small balls still flying, for drawing.
    pub fn small_balls(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.balls.iter().filter(|b| b.alive).map(|b| b.pos)
    }

    /// One tick with the ball at `ball`; `near`: the ball is close enough for the pickup to be switched on.
    pub fn tick(&mut self, ball: Vec3, near: bool) -> Option<ExtraEvent> {
        match self.state {
            State::Orbit => {
                if !near {
                    return None;
                }
                if self.center.distance_squared(ball) <= ACTIVATION * ACTIVATION {
                    self.state = State::FlyAway;
                    for b in &mut self.balls {
                        b.saved = b.pos;
                    }
                    self.away = Vec3::new(ball.x, ball.y - 2.1, ball.z);
                    return Some(ExtraEvent::Activated);
                }
                let angle = ROTATION * TIME_VALUE;
                for (i, b) in self.balls.iter_mut().enumerate() {
                    let axis = AXES[i.min(AXES.len() - 1)];
                    let local = self.rot.inverse() * (b.pos - self.center);
                    let turned = rotate(axis, angle, local);
                    let l = turned.length();
                    let turned = if l > 0.0 && l != RADIUS {
                        turned * (RADIUS / l)
                    } else {
                        turned
                    };
                    b.pos = self.center + self.rot * turned;
                }
                None
            }
            State::FlyAway => {
                let a = self.away - Vec3::Y * 3.0;
                self.timer -= FRAME_MS;
                if self.timer <= 0.0 {
                    self.state = State::HomeIn;
                    return None;
                }
                let k = AWAY_FORCE * TIME_VALUE;
                for b in &mut self.balls {
                    let (o, p) = (b.saved, b.pos);
                    b.saved = p;
                    b.pos = p + (p - o) * AWAY_DAMPING + (p - a) * k;
                }
                None
            }
            State::HomeIn => {
                if self.balls.iter().all(|b| !b.alive) {
                    self.state = State::Done;
                    return None;
                }
                let mut hits = 0;
                for b in self.balls.iter_mut().filter(|b| b.alive) {
                    let p = b.pos;
                    if ball.distance_squared(p) <= COLL_DISTANCE_SQ {
                        hits += 1;
                        b.alive = false;
                    } else {
                        let o = b.saved;
                        b.saved = p;
                        b.pos = p + (ball - p) * b.gain * TIME_VALUE + (p - o) * DAMPING;
                    }
                }
                (hits > 0).then_some(ExtraEvent::Hits(hits))
            }
            State::Done => None,
        }
    }
}

/// `v` turned by `angle` about `axis` as Vx3DMatrixFromRotation builds it: the Rodrigues matrix of the axis as given
/// (it is not normalised).
fn rotate(axis: Vec3, angle: f32, v: Vec3) -> Vec3 {
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    let (x, y, z) = (axis.x, axis.y, axis.z);
    let m = [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
    ];
    Vec3::new(
        m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
        m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
        m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_fires_near_the_ball_and_its_six_balls_come_home() {
        let local = [
            Vec3::new(0.0, 0.0, 2.05),
            Vec3::new(0.0, 2.0, 0.05),
            Vec3::new(0.0, 0.0, -1.95),
            Vec3::new(-1.0, -0.97, 1.48),
            Vec3::new(1.0, 1.0, -1.36),
            Vec3::new(-1.35, 1.37, 0.0),
        ];
        let mut e = Extra::new(0, Vec3::ZERO, Quat::IDENTITY, &local);
        assert_eq!(e.tick(Vec3::X * 10.0, true), None, "orbiting");
        assert_eq!(e.tick(Vec3::X * 2.0, true), Some(ExtraEvent::Activated));
        let mut hits = 0;
        for _ in 0..600 {
            if let Some(ExtraEvent::Hits(n)) = e.tick(Vec3::X * 2.0, true) {
                hits += n;
            }
        }
        assert_eq!(hits, 6);
        assert!(e.done());
    }
}
