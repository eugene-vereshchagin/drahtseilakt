// The collision part of the world (docs/ivp_collision.md; the records of phys_coll.h). The broadphase (`crate::ov`),
// the hull listeners, the mindists and their manager (`crate::manager`) are ported. The OO watchers and the contact
// system are not yet, so a pair whose object has more than one convex piece gets no collision, and the contact hooks
// of the manager are empty. Until those land, the game must not rely on this world for contacts.

use crate::core::World;
use crate::hull::HullManager;
use crate::ledge::Surface;
use crate::mindist::{Mindist, ObjCache};
use crate::ov::OvTree;

/// What a collision record is (`IvpCollision`): a mindist of the root delegator, or an OO watcher's child (3.1.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollKind {
    Mindist,
    Watcher,
}

/// The header every collision record shares (`IvpCollision`, phys_coll.h): the two objects it joins, the slot of
/// each object's element vector that holds it (`fvec`, -1 for none), and the record it stands for.
#[derive(Clone, Copy, Debug)]
pub struct Collision {
    pub kind: CollKind,
    pub objs: [usize; 2],
    pub fvec: [i32; 2],
    /// The mindist arena index for a mindist record.
    pub index: usize,
    /// Whether the record is in use; freed records keep their slot for reuse.
    pub alive: bool,
}

/// The kind of an object's collision geometry: a ball, or a polygon with its compact surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjKind {
    Polygon,
    Ball,
}

/// What the collision code keeps per object beyond its core: its geometry, its extra radius, its cache, and the heads
/// of its EXACT and INVALID synapse lists (`IvpCollObject`, 4.2.7).
#[derive(Debug, Default)]
pub struct ObjInfo {
    pub kind: Option<ObjKind>,
    pub surface: Option<Surface>,
    pub extra_radius: f32,
    pub cache: ObjCache,
    pub cache_valid: bool,
    pub cache_time_code: u32,
    pub exact_syn: Option<usize>,
    pub invalid_syn: Option<usize>,
}

/// The collision state of one world: the OV tree, the per-object records and hull managers, the collision records and
/// the mindist arena with its lists.
#[derive(Debug, Default)]
pub struct CollWorld {
    pub ov: OvTree,
    pub hulls: Vec<HullManager>,
    pub groups: Vec<[u8; 8]>,
    pub objs: Vec<ObjInfo>,
    pub collisions: Vec<Collision>,
    pub free_collisions: Vec<usize>,
    pub mds: Vec<Mindist>,
    pub watchers: Vec<crate::watcher::Watcher>,
    pub free_watchers: Vec<usize>,
    /// The hull managers (object indices) whose hull grew past a listener this PSI, checked after integration.
    pub check: Vec<usize>,
    pub free_mds: Vec<usize>,
    /// The heads of the manager's EXACT and INVALID mindist lists.
    pub exact: Option<usize>,
    pub invalid: Option<usize>,
    /// The upstream's global counter of the collision-distance index decay (0x10075edc).
    pub coll_dist_counter: u32,
    /// Statistics of the upstream's counters (env +0x78 onward), kept for the tests and the debug overlay.
    pub ov_rechecks: u32,
    pub watcher_checks: u32,
}

impl World {
    /// Revival of a core: the contact code's answer (1 asks the unit to restart its loop). 0x1001d4d0.
    pub(crate) fn core_revived(&mut self, _c: usize) -> i32 {
        0
    }

    /// A core froze: its collision objects leave the hull managers. 0x10017140.
    pub(crate) fn core_frozen(&mut self, _c: usize) {}

    /// A core integrated into the next PSI: the hull update of its object (0x1001e750, phys_contact.c 431).
    pub(crate) fn core_integrated(&mut self, c: usize, dt: f64) {
        self.hull_update_core(c, dt);
    }

    /// After the controllers of a unit ran: the per-object hook of each of its cores (0x100099a0).
    pub(crate) fn after_controllers(&mut self, unit: usize) {
        let cores = match self.units.get(unit) {
            Some(u) => u.cores.clone(),
            None => return,
        };
        for c in cores {
            self.object_after_controllers(c);
        }
    }

    /// Before the PSI: the begin hook and the mindist pre-PSI pass. 0x100124c0 preamble.
    pub(crate) fn psi_start(&mut self) {}

    /// After the PSI's integration: the hull phase, then the short and critic passes (0x10013cb0).
    pub(crate) fn psi_end(&mut self) {
        self.hull_phase();
        self.short_pass();
        self.critic_pass();
    }
}
