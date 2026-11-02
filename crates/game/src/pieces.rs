// The ball's pieces (Balls.nmo): when a transformer changes the ball, the old one bursts into its pieces as the
// original's Ball_Explosion_<kind> scripts do. Each kind's pieces (the group Ball_<Kind>_Pieces, placed about the
// frame Ball_<Kind>Pieces_Frame) are set where the ball was, made bodies of their own hulls in the ball's collision
// group (they do not meet the ball) with the script's mass, friction, elasticity and dampings, and pushed along their
// own up by a random impulse. They lie where they fall for 20 seconds (Gameplay.nmo's Fadeout Manager), or until the
// ball is lost; then Ball_ResetPieces_<kind> fades them out over two seconds and takes them away.

use crate::sim::BallKind;
use glam::{Mat4, Quat, Vec3};
use level::Level;

/// The fade (Fade <Kind> Pieces: a Bezier progression of 2000 ms), in ticks.
pub const FADE_TICKS: u64 = 120;
/// How long the pieces lie before they fade (Gameplay.nmo's Fadeout Manager: a timer of 20000 ms from the burst).
pub const LIE_TICKS: u64 = 1200;

/// One piece: its mesh in the balls' model, where it sits about its frame, and its hull in its own frame.
#[derive(Clone, Debug)]
pub struct Piece {
    pub mesh: usize,
    rel: Mat4,
    pub hull: Vec<Vec3>,
}

/// What a kind's explosion script gives its pieces: the point and direction of the push in the piece's own frame,
/// the push's range, and the body's values (paper's mass and friction are random in a range too).
#[derive(Clone, Copy, Debug)]
pub struct Burst {
    pub point: Vec3,
    pub impulse: (f32, f32),
    pub mass: (f32, f32),
    pub friction: (f32, f32),
    pub elasticity: f32,
    pub lin_damp: f32,
    pub rot_damp: f32,
}

/// The values of Ball_Explosion_Wood, _Stone and _Paper (Balls.nmo), measured from the scripts.
pub fn burst(kind: BallKind) -> Burst {
    match kind {
        BallKind::Wood => Burst {
            point: Vec3::new(0.0, 1.0, 0.0),
            impulse: (1.5, 3.0),
            mass: (0.2, 0.2),
            friction: (2.0, 2.0),
            elasticity: 1.0,
            lin_damp: 0.3,
            rot_damp: 0.2,
        },
        BallKind::Stone => Burst {
            point: Vec3::new(-0.05, 1.0, 0.05),
            impulse: (4.0, 9.0),
            mass: (0.8, 0.8),
            friction: (2.0, 2.0),
            elasticity: 1.0,
            lin_damp: 0.3,
            rot_damp: 0.2,
        },
        BallKind::Paper => Burst {
            point: Vec3::new(-0.03, 1.0, 0.02),
            impulse: (0.5, 1.3),
            mass: (0.02, 0.09),
            friction: (1.0, 5.0),
            elasticity: 1.0,
            lin_damp: 6.0,
            rot_damp: 0.5,
        },
    }
}

/// Every kind's pieces and the frame they are placed about.
#[derive(Clone, Debug, Default)]
pub struct Pieces {
    /// Wood, stone, paper.
    sets: [Vec<Piece>; 3],
    /// Each frame's turn (its pieces are placed turned as in the file, at the ball).
    turns: [Quat; 3],
}

pub fn index(kind: BallKind) -> usize {
    match kind {
        BallKind::Wood => 0,
        BallKind::Stone => 1,
        BallKind::Paper => 2,
    }
}

impl Pieces {
    /// The pieces of Balls.nmo; a kind the file lacks has none.
    pub fn find(balls: &Level) -> Pieces {
        let mut p = Pieces {
            turns: [Quat::IDENTITY; 3],
            ..Pieces::default()
        };
        for (k, name) in ["Wood", "Stone", "Paper"].into_iter().enumerate() {
            let Some((_, frame)) = balls.object(&format!("Ball_{name}Pieces_Frame")) else {
                continue;
            };
            let frame = Mat4::from_cols_array_2d(&frame.matrix);
            let (_, turn, _) = frame.to_scale_rotation_translation();
            let inverse = frame.inverse();
            let set: Vec<Piece> = balls
                .members(&format!("Ball_{name}_Pieces"))
                .filter_map(|(_, o)| {
                    let mesh = o.mesh?;
                    let positions = &balls.meshes.get(mesh)?.positions;
                    let hull: Vec<Vec3> = positions.iter().map(|&v| Vec3::from(v)).collect();
                    let rel = inverse * Mat4::from_cols_array_2d(&o.matrix);
                    (hull.len() >= 4 && rel.is_finite()).then_some(Piece { mesh, rel, hull })
                })
                .collect();
            if let (Some(s), Some(t)) = (p.sets.get_mut(k), p.turns.get_mut(k)) {
                *s = set;
                *t = turn;
            }
        }
        p
    }

    pub fn of(&self, kind: BallKind) -> &[Piece] {
        self.sets.get(index(kind)).map_or(&[], Vec::as_slice)
    }

    /// Where a piece of `kind` starts when the ball bursts at `at`: its position and turn.
    pub fn pose(&self, kind: BallKind, piece: &Piece, at: Vec3) -> (Vec3, Quat) {
        let turn = self
            .turns
            .get(index(kind))
            .copied()
            .unwrap_or(Quat::IDENTITY);
        let world = Mat4::from_rotation_translation(turn, at) * piece.rel;
        let (_, rot, pos) = world.to_scale_rotation_translation();
        (pos, rot)
    }
}

/// A small deterministic generator for the pushes and the paper's values (the run must replay the same).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }

    /// A value between `lo` and `hi`.
    pub fn range(&mut self, (lo, hi): (f32, f32)) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let u = (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40) as f32 / (1u64 << 24) as f32;
        lo + (hi - lo) * u
    }
}

/// How opaque the pieces are `age` ticks into their fade.
pub fn fade(age: u64) -> f32 {
    1.0 - (age as f32 / FADE_TICKS as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generator_stays_in_its_range_and_repeats() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        for _ in 0..1000 {
            let v = a.range((1.5, 3.0));
            assert!((1.5..=3.0).contains(&v));
            assert_eq!(v, b.range((1.5, 3.0)));
        }
        assert_eq!(fade(0), 1.0);
        assert_eq!(fade(FADE_TICKS), 0.0);
    }
}
