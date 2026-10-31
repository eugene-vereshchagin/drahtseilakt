// Port of OpenBallance src/phys/phys_watcher.c (GPL-3.0; see the LICENSE of the repository): the OO watcher (docs/
// ivp_collision.md 3.4, 3.5). For a pair where either surface has several convex pieces (every concave floor, every
// multi-convex module), a watcher keeps one plain mindist per pair of pieces that are in range. The watcher is
// re-checked whenever its hull listeners fire: it recomputes the range of the pair, exchanges the pieces, and re-arms
// its two hull listeners.
//
// A watcher's children are mindists whose owner is the watcher; a child is removed from the watcher's list when it is
// deleted, the last one taking its place, as the upstream does.

use crate::collision::CollKind;
use crate::core::World;
use crate::hull::{encode_listener, ListenerKind};
use crate::ledge::Edge;
use crate::math;
use crate::mindist::{Feat, Mindist, ObjCache};

/// An OO watcher (3.1.8): its two objects, its children, its hull listener slots and its collision record.
#[derive(Clone, Debug, Default)]
pub struct Watcher {
    pub alive: bool,
    pub objs: [usize; 2],
    pub children: Vec<usize>,
    pub hl_slots: [Option<u32>; 2],
    pub collision: Option<usize>,
}

/// The key a child is matched by: the object and the ledge of one side (`None` is a ball's dummy ledge).
type SideKey = (usize, Option<usize>);

impl World {
    /// Creates the watcher of a pair that has no single mindist, and its collision record (`ivp_watcher_create`,
    /// 0x10038000). None when an object has no geometry.
    pub(crate) fn watcher_pair(&mut self, a: usize, b: usize) -> Option<usize> {
        self.coll.objs.get(a)?.kind?;
        self.coll.objs.get(b)?.kind?;
        let w = self.watcher_alloc(Watcher {
            alive: true,
            objs: [a, b],
            ..Watcher::default()
        });
        let c = self.collision_new(CollKind::Watcher, [a, b], w);
        self.coll.watchers[w].collision = Some(c);
        // the hull listeners start "never" (1e20 above the hull), and the first check arms them
        let now = self.env.current_time;
        for k in 0..2 {
            let obj = self.coll.watchers[w].objs[k];
            let key = (f64::from(self.coll.hulls[obj].value_at(now)) + 1.000_000_020_040_877_3e20) as f32;
            let slot = self.coll.hulls[obj].insert(encode_listener(ListenerKind::Watcher, w as u32), key);
            self.coll.watchers[w].hl_slots[k] = Some(slot);
        }
        self.watcher_check(w);
        Some(c)
    }

    fn watcher_alloc(&mut self, wt: Watcher) -> usize {
        match self.coll.free_watchers.pop() {
            Some(i) => {
                self.coll.watchers[i] = wt;
                i
            }
            None => {
                self.coll.watchers.push(wt);
                self.coll.watchers.len() - 1
            }
        }
    }

    /// Re-checks a watcher (`ivp_watcher_check`, 0x10037e30): the pair's range, the exchange of its pieces, and the
    /// two listeners re-armed at their ranges.
    pub(crate) fn watcher_check(&mut self, w: usize) {
        self.coll.watcher_checks += 1;
        let [a, b] = self.coll.watchers[w].objs;
        let (r0, r1) = self.range_intra(a, b);
        self.watcher_exchange(w, r0 + r1);
        self.watcher_rearm(w, 0, r0);
        self.watcher_rearm(w, 1, r1);
    }

    /// Moves a watcher's listener of side `k` to the hull value now plus `r` (`rearm`, 0x10037e30's helper).
    fn watcher_rearm(&mut self, w: usize, k: usize, r: f64) {
        let obj = self.coll.watchers[w].objs[k];
        if let Some(slot) = self.coll.watchers[w].hl_slots[k].take() {
            self.coll.hulls[obj].remove(slot);
        }
        let now = self.env.current_time;
        let key = (f64::from(self.coll.hulls[obj].value_at(now)) + r) as f32;
        let slot = self.coll.hulls[obj].insert(encode_listener(ListenerKind::Watcher, w as u32), key);
        self.coll.watchers[w].hl_slots[k] = Some(slot);
    }

    /// The pieces of each side within reach of the other side (`exchange`, 0x10016650): a ball is one piece, the
    /// dummy ledge (None), when the other core is within reach; a polygon's pieces are the ledges its surface walk
    /// returns. Children whose piece pair is still in range are kept, the missing pairs are created, and the rest are
    /// deleted.
    fn watcher_exchange(&mut self, w: usize, dist: f64) {
        let objs = self.coll.watchers[w].objs;
        let now = self.env.current_time;
        // the caches first: they refresh the objects' state
        let caches: [ObjCache; 2] = [self.obj_cache(objs[0]), self.obj_cache(objs[1])];
        let mut lists: [Vec<Option<usize>>; 2] = [Vec::new(), Vec::new()];
        for k in 0..2 {
            let me = objs[k];
            let other = objs[k ^ 1];
            let c_other = crate::manager::core_pos_at(&self.cores[other], now);
            let reach = f64::from(self.coll.objs[me].extra_radius + self.cores[other].upper_limit_radius) + dist;
            let ca = &caches[k];
            if self.coll.objs[me].kind == Some(crate::collision::ObjKind::Ball) {
                let d = [c_other[0] - ca.vv[0], c_other[1] - ca.vv[1], c_other[2] - ca.vv[2]];
                let x = math::rtmul(&ca.r, &d);
                let d2 = x[0] * x[0] + x[1] * x[1] + x[2] * x[2];
                if !(reach * reach < d2) {
                    lists[k] = vec![None];
                }
                continue;
            }
            let d = [c_other[0] - ca.vv[0], c_other[1] - ca.vv[1], c_other[2] - ca.vv[2]];
            let c_os = math::rtmul(&ca.r, &d);
            let found = match self.coll.objs[me].surface.as_ref() {
                Some(s) => s.ledges_within_radius(c_os, reach),
                None => Vec::new(),
            };
            lists[k] = found.into_iter().map(Some).collect();
        }

        let mut keep = 0usize;
        let mut fresh: Vec<usize> = Vec::new();
        for &la in lists[0].iter().rev() {
            for &lb in lists[1].iter().rev() {
                let key_a: SideKey = (objs[0], la);
                let key_b: SideKey = (objs[1], lb);
                let children = &self.coll.watchers[w].children;
                let found = (keep..children.len()).find(|&i| {
                    let m = &self.coll.mds[children[i]];
                    side_key(m, 0) == key_a && side_key(m, 1) == key_b
                });
                if let Some(found) = found {
                    if keep < found {
                        self.coll.watchers[w].children.swap(keep, found);
                        let (x, y) = (self.coll.watchers[w].children[keep], self.coll.watchers[w].children[found]);
                        self.coll.mds[x].child_index = keep as i32;
                        self.coll.mds[y].child_index = found as i32;
                    }
                    keep += 1;
                    continue;
                }
                let fa = la.map(|l| Feat {
                    ledge: l,
                    edge: Edge { tri: 0, slot: 0 },
                });
                let fb = lb.map(|l| Feat {
                    ledge: l,
                    edge: Edge { tri: 0, slot: 0 },
                });
                if let Some(m) = self.mindist_create_with(Some(w), objs[0], objs[1], fa, fb) {
                    fresh.push(m);
                }
            }
        }
        while self.coll.watchers[w].children.len() > keep {
            let last = self.coll.watchers[w].children[self.coll.watchers[w].children.len() - 1];
            self.mindist_delete(last);
        }
        for &m in fresh.iter().rev() {
            let children = &mut self.coll.watchers[w].children;
            children.push(m);
            let idx = children.len() - 1;
            self.coll.mds[m].child_index = idx as i32;
        }
    }

    /// A child mindist was deleted: the last child takes its place (`child_deleted`, 0x100381d0).
    pub(crate) fn watcher_child_deleted(&mut self, w: usize, m: usize) {
        let i = self.coll.mds[m].child_index;
        let children = &self.coll.watchers[w].children;
        if i < 0 || (i as usize) >= children.len() || children[i as usize] != m {
            return;
        }
        let i = i as usize;
        self.coll.watchers[w].children.swap_remove(i);
        if i < self.coll.watchers[w].children.len() {
            let moved = self.coll.watchers[w].children[i];
            self.coll.mds[moved].child_index = i as i32;
        }
        self.coll.mds[m].child_index = -1;
    }

    /// Deletes a watcher: its children, its listeners, and its collision record (`ivp_watcher_delete`, 0x10038100).
    pub(crate) fn watcher_delete(&mut self, w: usize) {
        while let Some(&m) = self.coll.watchers[w].children.last() {
            let before = self.coll.watchers[w].children.len();
            self.mindist_delete(m);
            if self.coll.watchers[w].children.len() == before {
                // a child whose index does not match is not in this list: stop instead of looping
                self.coll.watchers[w].children.pop();
            }
        }
        let objs = self.coll.watchers[w].objs;
        for k in 0..2 {
            if let Some(slot) = self.coll.watchers[w].hl_slots[k].take() {
                self.coll.hulls[objs[k]].remove(slot);
            }
        }
        if let Some(c) = self.coll.watchers[w].collision.take() {
            self.collision_release(c);
        }
        self.coll.watchers[w] = Watcher::default();
        self.coll.free_watchers.push(w);
    }
}

/// The object and ledge of a mindist's synapse k, as the watcher matches its children.
fn side_key(m: &Mindist, k: usize) -> SideKey {
    (m.syn[k].obj, m.syn[k].feat.map(|f| f.ledge))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::collision::ObjKind;
    use crate::convex;
    use crate::ledge::Surface;
    use crate::math::Quat;

    /// A unit cube with its corner at `o`, as one convex ledge.
    fn cube_ledge(o: [f64; 3]) -> crate::ledge::Ledge {
        let mut p = Vec::new();
        for &x in &[0.0, 1.0] {
            for &y in &[0.0, 1.0] {
                for &z in &[0.0, 1.0] {
                    p.push([o[0] + x, o[1] + y, o[2] + z]);
                }
            }
        }
        convex::hull(&p).expect("a cube has a hull")
    }

    #[test]
    fn a_two_piece_object_pairs_through_a_watcher_with_children_in_reach() {
        let mut w = World::new();
        let q = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
        // object 0: two cubes, at x 0 and x 3 (two ledges of one surface)
        let a = w.core_init(0, q, [0.0; 3], false);
        w.cores[a].upper_limit_radius = 4.0;
        w.cores[a].movement_state = crate::core::MT_STATIC;
        let two = Surface::compile(vec![cube_ledge([0.0; 3]), cube_ledge([3.0, 0.0, 0.0])]).expect("a surface");
        w.set_object_geometry(a, ObjKind::Polygon, Some(two), 0.0);
        // object 1: one cube at x 4 (its piece spans 4..5), moving; the core radius of object 0 covers its pieces
        let b = w.core_init(1, q, [4.0, 0.0, 0.0], false);
        w.cores[b].upper_limit_radius = 0.866;
        w.cores[b].movement_state = crate::core::MT_MOVING;
        let one = Surface::compile(vec![cube_ledge([0.0; 3])]).expect("a surface");
        w.set_object_geometry(b, ObjKind::Polygon, Some(one), 0.0);

        w.enable_collision_detection(a);
        w.enable_collision_detection(b);

        let watchers: Vec<usize> = (0..w.coll.watchers.len()).filter(|&i| w.coll.watchers[i].alive).collect();
        assert_eq!(watchers.len(), 1, "one watcher for the two-piece pair");
        let children = w.coll.watchers[watchers[0]].children.clone();
        assert!(!children.is_empty(), "the piece at x 3 is within reach");
        for &m in &children {
            assert_eq!(w.coll.mds[m].owner, Some(watchers[0]));
        }
    }
}
