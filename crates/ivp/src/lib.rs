// Port of the physics of OpenBallance (https://github.com/emdzej/openballance, src/phys, GPL-3.0). Every file of
// this crate that is a port says so in its header. Behaviour is kept as the upstream code computes it: the
// original game's physics runs in x87 extended precision, which this port reproduces as f64 where the upstream did.

// The solver works on indices it creates itself (slots of its own arenas, dimensions of 3-vectors); its inputs come
// from the level model, not from files, so the input-hardening lints of the workspace do not apply here.
#![allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
// The port keeps the upstream code's shape so a reader can follow it line by line and the results stay bit-exact
// (replays depend on them): its index loops, argument lists, branches, constants written to more digits than f32
// holds, and `!(a < b)` tests, which differ from `a >= b` when a value is NaN. Clippy's style advice on these would
// change the code away from the reference, so it is off for this crate only.
#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::collapsible_if,
    clippy::neg_multiply,
    clippy::excessive_precision,
    clippy::assign_op_pattern,
    clippy::needless_late_init,
    clippy::if_same_then_else,
    clippy::manual_clamp
)]

pub mod body;
pub mod collision;
pub mod constraint;
pub mod contact;
pub mod convex;
pub mod core;
pub mod event;
pub mod event_sim;
pub mod friction;
pub mod fsolver;
pub mod glue;
pub mod hull;
pub mod impact;
pub mod ledge;
pub mod manager;
pub mod math;
pub mod mindist;
pub mod minimize;
pub mod minlist;
pub mod ov;
pub mod time;
pub mod watcher;
