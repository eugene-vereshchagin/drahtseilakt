// The collision part of the world (docs/ivp_collision.md; the records of phys_coll.h). The broadphase and the hull
// listeners are ported (`crate::ov`); the mindists, the watchers and their solvers are not yet, so the pairs of the
// broadphase are not created, and the hooks the core calls into the collision code are still stubs.
//
// Until the collision port lands, a world with these hooks simulates free bodies only, and the game must not rely on
// it for contacts. Each hook keeps the contract the core expects.

use crate::core::World;
use crate::hull::HullManager;
use crate::ov::OvTree;

/// What a collision record is (`IvpCollision`): a mindist of the root delegator, or an OO watcher's child (3.1.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollKind {
    Mindist,
    Watcher,
}

/// The header every collision record shares (`IvpCollision`, phys_coll.h): the two objects it joins and the slot of
/// each object's element vector that holds it (`fvec`, -1 for none).
#[derive(Clone, Copy, Debug)]
pub struct Collision {
    pub kind: CollKind,
    pub objs: [usize; 2],
    pub fvec: [i32; 2],
    /// Whether the record is in use; freed records keep their slot for reuse.
    pub alive: bool,
}

/// The collision state of one world: the OV tree, one hull manager per object, the collision records and the
/// collision filter's group names (the glue's group of each object, all zero for an unnamed one).
#[derive(Debug, Default)]
pub struct CollWorld {
    pub ov: OvTree,
    pub hulls: Vec<HullManager>,
    pub groups: Vec<[u8; 8]>,
    pub collisions: Vec<Collision>,
    pub free_collisions: Vec<usize>,
    /// Statistics of the upstream's counters (env +0x78 onward), kept for the tests and the debug overlay.
    pub ov_rechecks: u32,
}

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

    /// The mindist between two objects that the broadphase found touching (0x10016290). Not ported yet: no pair is
    /// created, so the broadphase keeps its tree but no contact is computed.
    pub(crate) fn mindist_create(&mut self, _a: usize, _b: usize) -> Option<usize> {
        None
    }
}

impl World {
    /// A mindist event of the time list fired (`0x100181b0`). Not ported yet: mindist events are never scheduled
    /// by this port, so this is unreachable until the collision port lands.
    pub(crate) fn mindist_simulate_event(&mut self, _payload: u32) {}
}
