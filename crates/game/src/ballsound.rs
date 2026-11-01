// The ball's sounds, as the original's Sound_Manager makes them (3D Entities/Sound.nmo): a hit sound when the ball
// strikes something of a known material (Hit Sounds: a PhysicsCollDetection per material), a rolling loop while it
// keeps touching one (Roll Wood/Stone, Roll Paper: PhysicsContinuousContact), its volume and pitch following the
// ball's speed (MultiRollSoundControl). The sounds come from the BallSound table, by ball kind. This works out what
// to play each tick; the window plays it (audio.rs), so runs without a window stay silent.

use crate::physics::BallSounds;
use crate::sim::BallKind;

/// What to do with the sound output.
#[derive(Clone, Debug, PartialEq)]
pub enum SoundCmd {
    /// Play a sound once at a volume.
    Play(&'static str, f32),
    /// Start rolling loop `slot` (0 stone, 1 wood, 2 metal) with a sound.
    Loop(usize, &'static str),
    /// Stop rolling loop `slot`.
    Stop(usize),
    /// Volume and pitch of every rolling loop.
    Roll(f32, f32),
    /// The volume of loop `slot`.
    Volume(usize, f32),
}

/// The loop slots of the fans' sounds (after the three rolling ones): the original has four Misc_Ventilator sounds.
pub const FAN_SLOTS: usize = 4;
pub const FAN_SLOT0: usize = 3;
/// A fan is heard within 25 of it, at full volume within 2 (P_Modul_18's PlaySound, TT ProximityVolumeControl).
const FAN_FAR: f32 = 25.0;
const FAN_NEAR: f32 = 2.0;

/// The fan sound's volume at a distance: 1 within 2, else 0.02 × 50^g with g going from 1 at 2 to 0 at 25 in the
/// squared distance (so it never quite falls silent before it stops).
pub fn fan_volume(d: f32) -> f32 {
    let (d2, near, far) = (d * d, FAN_NEAR * FAN_NEAR, FAN_FAR * FAN_FAR);
    let g = if d2 >= far {
        0.0
    } else if d2 <= near {
        1.0
    } else {
        1.0 - (d2 - near) / (far - near)
    };
    0.02 * 50f32.powf(g)
}

/// The fans' sounds: which fan each of the four loops plays for.
#[derive(Default)]
pub struct FanSound {
    playing: [Option<usize>; FAN_SLOTS],
}

impl FanSound {
    /// One tick: the fans in the physics (their points) and where the ball is.
    pub fn tick(&mut self, fans: &[glam::Vec3], ball: glam::Vec3, out: &mut Vec<SoundCmd>) {
        for (k, slot) in self.playing.iter_mut().enumerate() {
            if let Some(f) = *slot
                && fans.get(f).is_none_or(|p| p.distance(ball) >= FAN_FAR)
            {
                *slot = None;
                out.push(SoundCmd::Stop(FAN_SLOT0 + k));
            }
        }
        for (f, p) in fans.iter().enumerate() {
            let d = p.distance(ball);
            if d >= FAN_FAR {
                continue;
            }
            let k = match self.playing.iter().position(|s| *s == Some(f)) {
                Some(k) => k,
                None => {
                    // a free one of the four sounds; with none free the fan stays silent, as in the original
                    let Some(k) = self.playing.iter().position(Option::is_none) else {
                        continue;
                    };
                    self.playing[k] = Some(f);
                    out.push(SoundCmd::Loop(FAN_SLOT0 + k, "Misc_Ventilator"));
                    k
                }
            };
            out.push(SoundCmd::Volume(FAN_SLOT0 + k, fan_volume(d)));
        }
    }

    pub fn silence(&mut self, out: &mut Vec<SoundCmd>) {
        for (k, slot) in self.playing.iter_mut().enumerate() {
            if slot.take().is_some() {
                out.push(SoundCmd::Stop(FAN_SLOT0 + k));
            }
        }
    }
}

/// Materials, as the attributes number them: 1 stone, 2 wood, 3 metal, 4 dome (hits only).
const MATERIALS: usize = 4;

/// The hit listeners (Hit Sounds): per material, the minimum and maximum speed and the sleep after a hit, in seconds
/// of physics time.
const HITS: [(f32, f32, f64); MATERIALS] = [
    (2.0, 30.0, 1.0),
    (2.0, 14.0, 1.0),
    (2.0, 14.0, 2.0),
    (1.0, 15.0, 1.0),
];

/// The rolling contacts' delays (PhysicsContinuousContact): a loop starts after this long touching a material and
/// stops after this long without.
const ROLL_DELAY: f64 = 0.3;

/// MultiRollSoundControl: volume = speed × 0.05 (SoundVolumeFactor), pitch = 0.5 + speed × 0.01.
const VOLUME_PER_SPEED: f32 = 0.05;

/// The BallSound table: (roll on stone, wood, metal), (hit on stone, wood, metal, dome).
fn table(kind: BallKind) -> ([&'static str; 3], [&'static str; MATERIALS]) {
    match kind {
        BallKind::Paper => (["Roll_Paper"; 3], ["Hit_Paper"; MATERIALS]),
        BallKind::Wood => (
            ["Roll_Wood_Stone", "Roll_Wood_Wood", "Roll_Wood_Metal"],
            [
                "Hit_Wood_Stone",
                "Hit_Wood_Wood",
                "Hit_Wood_Metal",
                "Hit_Wood_Dome",
            ],
        ),
        BallKind::Stone => (
            ["Roll_Stone_Stone", "Roll_Stone_Wood", "Roll_Stone_Metal"],
            [
                "Hit_Stone_Stone",
                "Hit_Stone_Wood",
                "Hit_Stone_Metal",
                "Hit_Stone_Kuppel",
            ],
        ),
    }
}

#[derive(Default)]
pub struct BallSound {
    /// The physics time of each material's last hit sound.
    last_hit: [Option<f64>; MATERIALS],
    /// Per rolling material: since when it has been touched or untouched, and whether its loop plays.
    touch: [(Option<f64>, Option<f64>, bool); 3],
    kind: Option<BallKind>,
}

impl BallSound {
    /// One tick: what the ball met, its kind and speed, the physics time; the commands for the sound output.
    pub fn tick(
        &mut self,
        met: &BallSounds,
        kind: BallKind,
        speed: f32,
        now: f64,
        out: &mut Vec<SoundCmd>,
    ) {
        let (rolls, hits) = table(kind);
        if self.kind != Some(kind) {
            // a new ball kind: its loops are other sounds
            for (slot, t) in self.touch.iter_mut().enumerate() {
                if t.2 {
                    out.push(SoundCmd::Stop(slot));
                }
                *t = (None, None, false);
            }
            self.kind = Some(kind);
        }
        for &(hit, rel) in &met.impacts {
            let m = usize::from(hit);
            let Some(&(min, max, sleep)) = m.checked_sub(1).and_then(|k| HITS.get(k)) else {
                continue;
            };
            let last = &mut self.last_hit[m - 1];
            if last.is_some_and(|t| now - t < sleep) || rel <= min {
                continue;
            }
            *last = Some(now);
            out.push(SoundCmd::Play(hits[m - 1], (rel / max).min(1.0)));
        }
        for (slot, t) in self.touch.iter_mut().enumerate() {
            let touching = met.rolls.iter().any(|&r| usize::from(r) == slot + 1);
            let (on_since, off_since, playing) = t;
            if touching {
                *off_since = None;
                let since = *on_since.get_or_insert(now);
                if !*playing && now - since >= ROLL_DELAY {
                    *playing = true;
                    out.push(SoundCmd::Loop(slot, rolls[slot]));
                }
            } else {
                *on_since = None;
                let since = *off_since.get_or_insert(now);
                if *playing && now - since >= ROLL_DELAY {
                    *playing = false;
                    out.push(SoundCmd::Stop(slot));
                }
            }
        }
        if self.touch.iter().any(|t| t.2) {
            out.push(SoundCmd::Roll(speed * VOLUME_PER_SPEED, 0.5 + speed * 0.01));
        }
    }

    /// The ball is gone (lost, or in a transformer): its loops stop.
    pub fn silence(&mut self, out: &mut Vec<SoundCmd>) {
        for (slot, t) in self.touch.iter_mut().enumerate() {
            if t.2 {
                out.push(SoundCmd::Stop(slot));
            }
            *t = (None, None, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hit_sounds_once_per_sleep_and_scales_with_speed() {
        let mut b = BallSound::default();
        let mut out = Vec::new();
        let met = BallSounds {
            impacts: vec![(2, 7.0)],
            rolls: vec![],
        };
        b.tick(&met, BallKind::Wood, 0.0, 10.0, &mut out);
        assert_eq!(out, vec![SoundCmd::Play("Hit_Wood_Wood", 0.5)]);
        out.clear();
        b.tick(&met, BallKind::Wood, 0.0, 10.5, &mut out);
        assert!(out.is_empty(), "asleep for a second");
        b.tick(
            &BallSounds {
                impacts: vec![(2, 1.5), (0, 9.0)],
                rolls: vec![],
            },
            BallKind::Wood,
            0.0,
            12.0,
            &mut out,
        );
        assert!(out.is_empty(), "too slow, or no material");
    }

    #[test]
    fn a_fan_is_heard_near_it_and_stops_beyond_25() {
        assert_eq!(fan_volume(1.0), 1.0);
        assert!((fan_volume(24.99) - 0.02).abs() < 0.001);
        let mut f = FanSound::default();
        let mut out = Vec::new();
        let fans = [glam::Vec3::ZERO];
        f.tick(&fans, glam::Vec3::X * 10.0, &mut out);
        assert_eq!(out[0], SoundCmd::Loop(FAN_SLOT0, "Misc_Ventilator"));
        out.clear();
        f.tick(&fans, glam::Vec3::X * 30.0, &mut out);
        assert_eq!(out, vec![SoundCmd::Stop(FAN_SLOT0)]);
    }

    #[test]
    fn rolling_starts_and_stops_after_its_delays() {
        let mut b = BallSound::default();
        let mut out = Vec::new();
        let on = BallSounds {
            impacts: vec![],
            rolls: vec![1, 1],
        };
        let off = BallSounds::default();
        b.tick(&on, BallKind::Stone, 10.0, 0.0, &mut out);
        b.tick(&on, BallKind::Stone, 10.0, 0.2, &mut out);
        assert!(out.is_empty());
        b.tick(&on, BallKind::Stone, 10.0, 0.3, &mut out);
        assert_eq!(
            out,
            vec![
                SoundCmd::Loop(0, "Roll_Stone_Stone"),
                SoundCmd::Roll(0.5, 0.6)
            ]
        );
        out.clear();
        b.tick(&off, BallKind::Stone, 10.0, 0.4, &mut out);
        b.tick(&off, BallKind::Stone, 10.0, 0.75, &mut out);
        assert_eq!(out.last(), Some(&SoundCmd::Stop(0)));
    }
}
