// Port of the physics of OpenBallance (https://github.com/emdzej/openballance, src/phys, GPL-3.0). Every file of
// this crate that is a port says so in its header. Behaviour is kept as the upstream code computes it: the
// original game's physics runs in x87 extended precision, which this port reproduces as f64 where the upstream did.

// The solver works on indices it creates itself (slots of its own arenas, dimensions of 3-vectors); its inputs come
// from the level model, not from files, so the input-hardening lints of the workspace do not apply here.
#![allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]

pub mod math;
pub mod hull;
pub mod minlist;
