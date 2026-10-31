// Port of OpenBallance src/phys/phys_mindist.c and the mindist records of phys_coll.h (GPL-3.0; see the LICENSE of the
// repository): a mindist is the closest-feature pair of two objects (docs/ivp_collision.md 4). This file holds the
// records; the solver that fills them is `crate::minimize`, and the manager (creation, events, passes) follows.

use crate::ledge::Edge;

/// A feature of an object: an edge of one of its ledges. The upstream keeps a pointer; here the ledge is its index in
/// the object's surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Feat {
    pub ledge: usize,
    pub edge: Edge,
}

/// The object state a mindist solve reads (`IvpCache`, 4.2.7): the object's pose in world space and its core position
/// at the time of the cache.
#[derive(Clone, Copy, Debug, Default)]
pub struct ObjCache {
    pub r: [[f64; 3]; 3],
    pub vv: [f64; 3],
    pub core_pos: [f64; 3],
}

/// Synapse status (2.1.7): which feature of its object the mindist's closest point lies on.
pub const ST_POINT: i16 = 0;
pub const ST_EDGE: i16 = 1;
pub const ST_FACE: i16 = 2;
pub const ST_BALL: i16 = 3;
pub const ST_BACKSIDE: i16 = 5;

/// A synapse of a mindist (`IvpSynapse`): the object, the feature and the status.
#[derive(Clone, Copy, Debug, Default)]
pub struct Synapse {
    pub obj: usize,
    pub feat: Option<Feat>,
    pub status: i16,
}

/// Mindist flags (4.2.3): the sort bit, the recalc failure bits, the status, and the collision-distance index.
pub const MDF_COLL_TYPE: u32 = 0xff;
pub const MDF_SORT: u32 = 0x100;
pub const MDF_FUNCTION: u32 = 0x3000;
pub const MDF_RECALC: u32 = 0xc000;
pub const MDF_STATUS_SHIFT: u32 = 18;
pub const MDF_STATUS: u32 = 0x3c0000;
pub const MDF_COLL_DIST_SHIFT: u32 = 22;
pub const MDF_COLL_DIST: u32 = 0x3fc00000;

/// The mindist states (4.2.3): invalid, exact, hull-recursive and hull.
pub const MD_INVALID: u32 = 2;
pub const MD_EXACT: u32 = 3;
pub const MD_HULL_RECURSIVE: u32 = 4;
pub const MD_HULL: u32 = 5;

/// A mindist (`IvpMindist`, 4.2.1). Its output for the contact system: the two objects and their features, ordered by
/// the sort flag, the gap `len_numerator` (distance minus the extra radii) and the unit world normal from the second
/// sorted synapse's object toward the first's.
#[derive(Clone, Debug, Default)]
pub struct Mindist {
    pub flags: u32,
    pub syn: [Synapse; 2],
    pub sum_extra_radius: f32,
    pub len_numerator: f32,
    pub contact_dot_diff_center: f32,
    pub sum_angular_hull_time: f64,
    pub normal: [f32; 3],
    pub recalc_time_stamp: u32,
    /// The triangle the wheel pass last handled (`wheel_edge`, 0x84).
    pub wheel_edge: Option<Feat>,
}

impl Mindist {
    pub fn status(&self) -> u32 {
        (self.flags & MDF_STATUS) >> MDF_STATUS_SHIFT
    }

    pub fn set_status(&mut self, s: u32) {
        self.flags = (self.flags & !MDF_STATUS) | (s << MDF_STATUS_SHIFT);
    }

    /// The index (0 or 1) of the synapse that is sorted first (`ivp_md_sel`).
    pub fn sel(&self) -> usize {
        ((self.flags >> 8) & 1) as usize
    }

    /// The synapse index of the k-th sorted synapse (`ivp_md_sorted`): syn[sel ^ k].
    pub fn sorted(&self, k: usize) -> usize {
        self.sel() ^ k
    }
}
