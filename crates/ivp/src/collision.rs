// The hooks the core calls into the collision code (`phys_collision_*` of the upstream, see docs/ivp_collision.md).
// They are not ported yet: until the collision port lands, a world with these hooks simulates free bodies only, and
// the game must not rely on it for contacts. Each hook keeps the contract the core expects.

use crate::core::World;

impl World {
    /// Revival of a core: the contact code's answer (1 asks the unit to restart its loop). 0x1001d4d0.
    pub(crate) fn core_revived(&mut self, _c: usize) -> i32 {
        0
    }

    /// A core froze: its collision objects leave the hull managers. 0x10017140.
    pub(crate) fn core_frozen(&mut self, _c: usize) {}

    /// A core integrated into the next PSI: the hull update of its objects. 0x1001e750.
    pub(crate) fn core_integrated(&mut self, _c: usize, _dt: f64) {}

    /// After the controllers of a unit ran: the per-object hook. 0x100099a0.
    pub(crate) fn after_controllers(&mut self, _unit: usize) {}

    /// Before the PSI: the begin hook and the mindist pre-PSI pass. 0x100124c0 preamble.
    pub(crate) fn psi_start(&mut self) {}

    /// After the PSI's integration: the end hook. 0x1001ea50 tail.
    pub(crate) fn psi_end(&mut self) {}
}

impl World {
    /// A mindist event of the time list fired (`0x100181b0`). Not ported yet: mindist events are never scheduled
    /// by this port, so this is unreachable until the collision port lands.
    pub(crate) fn mindist_simulate_event(&mut self, _payload: u32) {}
}
