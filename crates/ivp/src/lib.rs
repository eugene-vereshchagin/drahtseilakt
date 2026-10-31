// Port of the physics of OpenBallance (https://github.com/emdzej/openballance, src/phys, GPL-3.0). Every file of
// this crate that is a port says so in its header. Behaviour is kept as the upstream code computes it: the
// original game's physics runs in x87 extended precision, which this port reproduces as f64 where the upstream did.

pub mod math;
