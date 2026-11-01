// The transformer's animation (`3D Entities/AnimTrafo.nmo`): while the ball is held, a ring opens around the
// transformer, bars with a flash field rise around the ball and come down, and the ring closes. The original runs it
// from the file's `AnimTrafo_MF Script` (Ring_Open, Up'n Down, Ring_Close); the timeline here is that script's,
// measured frame by frame in the C reference game at 60 frames a second (docs/gameplay_bbs.md, the transformer).

use crate::render::Instance;
use glam::{Mat4, Vec3};
use level::Level;

/// Ticks after the transformer takes the ball at which the animation begins (its objects are placed and shown).
pub const BEGIN: u64 = 1;
/// The ring's four parts move out by half a unit along their frames' (-1, 0, -1) over these ticks, speeding up.
const RING_OPEN: f32 = 9.0;
/// The bars and the flash field rise from this age (ticks since the animation began).
const BARS_FROM: f32 = 22.0;
/// The bars' lift every 3 ticks from `BARS_FROM`: the script's Bezier progression of 2 s up to (0, 5.2, 0) and down.
const BARS_LIFT: [f32; 41] = [
    0.01, 0.09, 0.26, 0.52, 0.84, 1.21, 1.62, 2.06, 2.51, 2.97, 3.42, 3.85, 4.25, 4.59, 4.88, 5.10,
    5.20, 5.20, 5.20, 5.20, 5.20, 5.20, 5.20, 5.20, 5.15, 4.96, 4.70, 4.38, 4.00, 3.59, 3.16, 2.71,
    2.26, 1.81, 1.40, 1.01, 0.67, 0.39, 0.17, 0.04, 0.00,
];
/// The flash field is shown while the bars move.
const FLASH_UNTIL: f32 = 142.0;
/// The ring closes from this age, speeding up, over `RING_CLOSE` ticks.
const RING_CLOSE_FROM: f32 = 142.0;
const RING_CLOSE: f32 = 11.0;
/// The age at which the script ends and hides it all.
pub const END: f32 = 153.0;
/// The age at which the old ball is hidden; the new one comes a tick later.
pub const OLD_BALL_HIDDEN: f32 = 147.0;

#[derive(Clone, Copy)]
enum Role {
    /// A ring part, moving along its frame's (-1, 0, -1) (in the transformer's space).
    Ring(Vec3),
    Bars,
    Flash,
}

/// The animation's meshes, each with its matrix relative to `AnimTrafo_MF` and its role.
pub struct TrafoModel {
    pub model: usize,
    parts: Vec<(usize, Mat4, Role)>,
}

impl TrafoModel {
    /// The parts of a loaded `AnimTrafo.nmo` drawn as model `model`, or `None` if the file lacks them.
    pub fn new(file: &Level, model: usize) -> Option<TrafoModel> {
        let object = |name: &str| {
            file.objects
                .iter()
                .find(|o| o.name.eq_ignore_ascii_case(name))
        };
        let matrix = |name: &str| object(name).map(|o| Mat4::from_cols_array_2d(&o.matrix));
        let origin = matrix("AnimTrafo_MF")?.inverse();
        let mut parts = Vec::new();
        let mut add = |name: &str, role: Role| {
            if let Some(o) = object(name)
                && let Some(mesh) = o.mesh
            {
                parts.push((mesh, origin * Mat4::from_cols_array_2d(&o.matrix), role));
            }
        };
        add("AnimTrafo_Bars", Role::Bars);
        add("AnimTrafo_Flashfield", Role::Flash);
        for k in 1..=4 {
            // Each part moves in its frame's space: the frame's axes turn the move into the transformer's.
            let Some(frame) = matrix(&format!("AnimTrafo_Frame0{k}")) else {
                continue;
            };
            let dir = (origin * frame).transform_vector3(Vec3::new(-1.0, 0.0, -1.0));
            add(&format!("AnimTrafo_Ringpart{k}"), Role::Ring(dir));
        }
        (!parts.is_empty()).then_some(TrafoModel { model, parts })
    }

    /// The animation's meshes at `age` ticks after it began, for a transformer at `at`.
    pub fn instances(&self, at: &Mat4, age: f32, out: &mut Vec<Instance>) {
        if !(0.0..END).contains(&age) {
            return;
        }
        let lift = Vec3::Y * bars_lift(age);
        for &(mesh, local, role) in &self.parts {
            let offset = match role {
                Role::Bars => lift,
                Role::Flash if (BARS_FROM..FLASH_UNTIL).contains(&age) => lift,
                Role::Flash => continue,
                // The ring parts hang from the bars.
                Role::Ring(dir) => lift + dir * 0.5 * ring_open(age),
            };
            let mut inst = Instance::plain(self.model, mesh, *at * Mat4::from_translation(offset) * local);
            if matches!(role, Role::Flash) {
                inst.uv_offset = [flash_scroll(age), 0.0];
            }
            out.push(inst);
        }
    }
}

/// The flash field's texture shift along u at an age: from the ring's opening on, the script's FlashAnim scrolls it
/// by +0.5 and back by -0.5 in turn, a step every 5 ticks (a Delayer of 50 ms, 4 ticks, and a link's delay).
fn flash_scroll(age: f32) -> f32 {
    let steps = ((age - RING_OPEN).max(0.0) / 5.0) as u32;
    if steps.is_multiple_of(2) { 0.5 } else { 0.0 }
}

/// The alpha of the shadows of the transformers making the new ball while the animation plays: Set Diffuse with
/// (1,1,1,1) to (1,1,1,0.196) by the bars' progression; None outside the bars' rise and fall.
pub fn shadow_alpha(age: f32) -> Option<f32> {
    if !(BARS_FROM..FLASH_UNTIL).contains(&age) {
        return None;
    }
    let v = bars_lift(age) / 5.2;
    Some(1.0 + (0.196_078 - 1.0) * v)
}

/// How far the bars are up at an age.
fn bars_lift(age: f32) -> f32 {
    if age < BARS_FROM {
        return 0.0;
    }
    let t = (age - BARS_FROM) / 3.0;
    let i = t.floor() as usize;
    match (BARS_LIFT.get(i), BARS_LIFT.get(i + 1)) {
        (Some(a), Some(b)) => a + (b - a) * t.fract(),
        _ => 0.0,
    }
}

/// How far the ring is open (0 closed, 1 open) at an age.
fn ring_open(age: f32) -> f32 {
    if age < RING_CLOSE_FROM {
        (age / RING_OPEN).clamp(0.0, 1.0).powi(2)
    } else {
        1.0 - ((age - RING_CLOSE_FROM) / RING_CLOSE)
            .clamp(0.0, 1.0)
            .powi(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bars_rise_to_their_top_and_come_back() {
        assert_eq!(bars_lift(0.0), 0.0);
        assert!((bars_lift(BARS_FROM + 60.0) - 5.2).abs() < 1e-6);
        assert_eq!(bars_lift(BARS_FROM + 120.0), 0.0);
        assert_eq!(bars_lift(END), 0.0);
    }

    #[test]
    fn the_ring_opens_then_closes() {
        assert_eq!(ring_open(0.0), 0.0);
        assert_eq!(ring_open(RING_OPEN), 1.0);
        assert_eq!(ring_open(100.0), 1.0);
        assert_eq!(ring_open(RING_CLOSE_FROM + RING_CLOSE), 0.0);
    }
}
