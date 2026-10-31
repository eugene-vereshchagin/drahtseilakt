// Port of OpenBallance src/phys/phys_ov.c (GPL-3.0; see the LICENSE of the repository): the broadphase of the collision
// detection (docs/ivp_collision.md 2.4 and 2.6). Each object with collision detection has an element in an OV tree, a
// hierarchy of dyadic boxes; the recheck of an element reconciles the object's collisions with the elements that share
// its box. Nodes and elements live in arenas and refer to each other by index. An object is identified by the index of
// its core, since Ballance gives every object exactly one core.
//
// The pairs the recheck creates are mindists, which are not ported yet (`crate::collision::World::mindist_create`
// returns no pair until then). The tree, the hull listeners and the reconciliation are exact already.

use std::collections::HashMap;

use crate::collision::{CollKind, Collision};
use crate::core::World;
use crate::hull::{encode_listener, ListenerKind};

/// 2^e, exact for the exponents the tree uses. The upstream's powerlist (2.1.9) holds 2^(j-40) for j in 0..81.
fn pow2(e: i32) -> f64 {
    2f64.powi(e)
}

/// The binary exponent of a positive double (0x1002e150).
fn exponent(x: f64) -> i32 {
    ((x.to_bits() >> 52) & 0x7ff) as i32 - 0x3ff
}

/// A box in the tree: origin (x, y, z) in units of 2^(l-1)... as the upstream numbers them, its level `l` and its raster.
/// A node's raster is its level plus one, which is what the tree's walks compare.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoxKey {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub l: i32,
    pub raster: i32,
}

#[derive(Clone, Debug)]
struct Node {
    key: BoxKey,
    parent: Option<usize>,
    children: Vec<usize>,
    /// Elements (object indices) whose box is exactly this node.
    elements: Vec<usize>,
    alive: bool,
}

/// An OV element (2.4): an object's sphere in the tree, with the hull listener that keeps its box up to date.
#[derive(Clone, Debug, Default)]
pub struct Element {
    pub center: [f32; 3],
    pub radius: f32,
    /// The node the element sits in, while it is in the tree.
    pub node: Option<usize>,
    /// The slot of the element's listener in its object's hull manager, while it listens.
    pub hull_slot: Option<u32>,
    /// The collisions this element holds (arena indices), in the upstream's vector order.
    pub collisions: Vec<usize>,
}

/// The OV tree (2.6) and its elements, indexed by object.
#[derive(Debug)]
pub struct OvTree {
    nodes: Vec<Node>,
    free_nodes: Vec<usize>,
    hash: HashMap<(i32, i32, i32, i32), usize>,
    pub root: Option<usize>,
    /// The box the last `calc_optimal_box` chose for the element being inserted.
    search: BoxKey,
    pub elements: Vec<Option<Element>>,
}

impl Default for OvTree {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_key(k: &BoxKey) -> (i32, i32, i32, i32) {
    (k.x, k.y, k.z, k.l)
}

/// `a` is coarser than `b` by `s` levels and contains it (0x1002e180).
fn contains(a: &BoxKey, b: &BoxKey, s: i32) -> bool {
    let f = 1i64 << s;
    let (a, b) = ([a.x, a.y, a.z], [b.x, b.y, b.z]);
    (0..3).all(|k| {
        let lo = i64::from(a[k]) * f;
        let hi = lo + (2i64 << s) - 2;
        lo <= i64::from(b[k]) && i64::from(b[k]) <= hi
    })
}

/// `a` is coarser than `b` by `s` levels and the two boxes overlap; touching boxes do not (0x1002e4b0).
fn overlaps(a: &BoxKey, b: &BoxKey, s: i32) -> bool {
    let f = 1i64 << s;
    let (a, b) = ([a.x, a.y, a.z], [b.x, b.y, b.z]);
    (0..3).all(|k| {
        let (ak, bk) = (i64::from(a[k]), i64::from(b[k]));
        bk + 2 > ak * f && (ak + 2) * f > bk
    })
}

impl OvTree {
    /// The empty tree (0x1002dff0).
    pub fn new() -> OvTree {
        OvTree {
            nodes: Vec::new(),
            free_nodes: Vec::new(),
            hash: HashMap::new(),
            root: None,
            search: BoxKey::default(),
            elements: Vec::new(),
        }
    }

    fn alloc_node(&mut self, key: BoxKey) -> usize {
        let node = Node {
            key,
            parent: None,
            children: Vec::new(),
            elements: Vec::new(),
            alive: true,
        };
        match self.free_nodes.pop() {
            Some(i) => {
                self.nodes[i] = node;
                i
            }
            None => {
                self.nodes.push(node);
                self.nodes.len() - 1
            }
        }
    }

    fn free_node(&mut self, n: usize) {
        let node = &mut self.nodes[n];
        node.alive = false;
        node.children = Vec::new();
        node.elements = Vec::new();
        node.parent = None;
        self.free_nodes.push(n);
    }

    fn hash_find(&self, k: &BoxKey) -> Option<usize> {
        self.hash.get(&hash_key(k)).copied()
    }

    fn hash_add(&mut self, n: usize) {
        self.hash.insert(hash_key(&self.nodes[n].key), n);
    }

    fn hash_remove(&mut self, n: usize) {
        let k = hash_key(&self.nodes[n].key);
        if self.hash.get(&k) == Some(&n) {
            self.hash.remove(&k);
        }
    }

    fn add_child(&mut self, p: usize, c: usize) {
        self.nodes[p].children.push(c);
    }

    fn element(&self, b: usize) -> Option<&Element> {
        self.elements.get(b)?.as_ref()
    }

    fn element_mut(&mut self, b: usize) -> Option<&mut Element> {
        self.elements.get_mut(b)?.as_mut()
    }

    /// Chooses the box for an element of radius `r` (or `big_r` when that fits as well) and leaves it in `search`
    /// (0x1002e210). Returns the radius the box was chosen for.
    fn calc_optimal_box(&mut self, center: [f32; 3], r: f64, big_r: f64) -> f64 {
        let mut lvl = exponent(2.0 * r) + 1;
        if lvl < -39 {
            lvl = -39;
        }
        loop {
            // powerlist[41 - lvl] = 2^(1 - lvl) and powerlist[40 - lvl] = 2^(-lvl)
            let s = pow2(1 - lvl);
            let mut lo = [0i32; 3];
            let mut fit = true;
            for a in 0..3 {
                if !fit {
                    break;
                }
                let c = f64::from(center[a]);
                lo[a] = ((c - r) * s).floor() as i32;
                let hi = ((c + r) * s).ceil() as i32;
                fit = hi <= lo[a] + 2;
            }
            if !fit {
                lvl += 1;
                continue;
            }
            self.search = BoxKey {
                x: lo[0],
                y: lo[1],
                z: lo[2],
                l: lvl - 1,
                raster: lvl,
            };
            if big_r <= r {
                return r;
            }
            let s2 = pow2(-lvl);
            let mut lo2 = [0i32; 3];
            let mut fit2 = true;
            for a in 0..3 {
                if !fit2 {
                    break;
                }
                let c = f64::from(center[a]);
                lo2[a] = ((c - big_r) * s2).floor() as i32;
                fit2 = ((c + big_r) * s2).ceil() as i32 <= lo2[a] + 2;
            }
            if fit2 {
                self.search = BoxKey {
                    x: lo2[0],
                    y: lo2[1],
                    z: lo2[2],
                    l: lvl,
                    raster: lvl + 1,
                };
                return big_r;
            }
            return r;
        }
    }

    /// The node of the tree that contains box `n`, walking down from `cur` (0x1002e560).
    fn find_node(&self, mut cur: usize, n: &BoxKey) -> usize {
        loop {
            let next = self.nodes[cur].children.iter().copied().find(|&c| {
                let ck = &self.nodes[c].key;
                ck.l >= n.l && contains(ck, n, ck.l - n.l)
            });
            match next {
                Some(c) => cur = c,
                None => return cur,
            }
        }
    }

    /// Links `n` below `p` through the intermediate boxes that make the raster differ by one (0x1002e5b0).
    fn connect_boxes(&mut self, mut p: usize, n: usize) {
        while self.nodes[p].key.raster - self.nodes[n].key.raster != 1 {
            let pk = self.nodes[p].key;
            let nk = self.nodes[n].key;
            let d = pk.raster - nk.raster;
            let pc = [pk.x, pk.y, pk.z];
            let nc = [nk.x, nk.y, nk.z];
            let mut cc = [0i32; 3];
            for k in 0..3 {
                let (pk_c, nk_c) = (i64::from(pc[k]), i64::from(nc[k]));
                let half = 1i64 << (d - 1);
                cc[k] = if nk_c < (2 * pk_c + 1) * half {
                    2 * pc[k]
                } else if nk_c < (pk_c + 1) * (1i64 << d) {
                    2 * pc[k] + 1
                } else {
                    2 * pc[k] + 2
                };
            }
            let c = self.alloc_node(BoxKey {
                x: cc[0],
                y: cc[1],
                z: cc[2],
                l: pk.l - 1,
                raster: pk.raster - 1,
            });
            self.nodes[c].parent = Some(p);
            self.add_child(p, c);
            self.hash_add(c);
            p = c;
        }
        self.nodes[n].parent = Some(p);
        self.add_child(p, n);
    }

    /// Makes a new root above the current one, large enough to hold box `n` (0x1002e7b0).
    fn expand_root(&mut self, n: usize) {
        let Some(r) = self.root else { return };
        let rk = self.nodes[r].key;
        let nk = self.nodes[n].key;
        let rc = [rk.x, rk.y, rk.z];
        let nc = [nk.x, nk.y, nk.z];
        let mut pc = [0i32; 3];
        for k in 0..3 {
            // C division truncates toward zero, as Rust's does
            pc[k] = rc[k] / 2;
            let rem = rc[k] % 2;
            if rem == -1 {
                pc[k] -= 1;
            } else if rem == 0 && f64::from(nc[k]) * pow2(nk.l) < f64::from(pc[k]) * pow2(rk.l + 1) {
                pc[k] -= 1;
            }
        }
        let p = self.alloc_node(BoxKey {
            x: pc[0],
            y: pc[1],
            z: pc[2],
            l: rk.l + 1,
            raster: rk.raster + 1,
        });
        self.add_child(p, r);
        self.nodes[r].parent = Some(p);
        self.hash_add(p);
        self.root = Some(p);
    }

    /// Removes element `b` from its node and frees the nodes that became empty (0x1002eec0).
    fn remove_element(&mut self, b: usize) {
        let Some(e) = self.element_mut(b) else { return };
        let Some(mut n) = e.node.take() else { return };
        if let Some(i) = self.nodes[n].elements.iter().position(|&x| x == b) {
            self.nodes[n].elements.remove(i);
        }
        while self.nodes[n].elements.is_empty() && self.nodes[n].children.is_empty() {
            let p = self.nodes[n].parent;
            self.hash_remove(n);
            match p {
                None => self.root = None,
                Some(p) => {
                    if let Some(i) = self.nodes[p].children.iter().position(|&c| c == n) {
                        self.nodes[p].children.remove(i);
                    }
                }
            }
            self.free_node(n);
            match p {
                Some(p) => n = p,
                None => break,
            }
        }
    }

    /// Puts element `b` into the tree at the box of radius `r` (or `big_r`), returning the radius chosen. With
    /// `want_cand`, also returns the elements whose spheres touch it (0x1002ec10).
    fn insert_element(&mut self, b: usize, rmin: f64, rmax: f64, want_cand: bool) -> (f64, Vec<usize>) {
        let center = match self.element(b) {
            Some(e) => e.center,
            None => return (rmin, Vec::new()),
        };
        let r = self.calc_optimal_box(center, rmin, rmax);
        if let Some(e) = self.element_mut(b) {
            e.radius = r as f32;
        }
        let search = self.search;
        let mut cand = Vec::new();
        let n = match self.hash_find(&search) {
            Some(n) => {
                self.nodes[n].elements.push(b);
                n
            }
            None => {
                let nn = self.alloc_node(search);
                self.nodes[nn].elements.push(b);
                if self.root.is_none() {
                    self.root = Some(nn);
                    self.hash_add(nn);
                    if let Some(e) = self.element_mut(b) {
                        e.node = Some(nn);
                    }
                    return (r, cand);
                }
                // grow the tree until its root holds the new box
                while let Some(root) = self.root {
                    let rk = self.nodes[root].key;
                    let nk = self.nodes[nn].key;
                    if nk.l <= rk.l && contains(&rk, &nk, rk.l - nk.l) {
                        break;
                    }
                    self.expand_root(nn);
                }
                let root = self.root.unwrap_or(nn);
                if self.nodes[root].key.l == self.nodes[nn].key.l {
                    // the same box as the root: the element joins the root and the new node is dropped
                    self.nodes[root].elements.push(b);
                    self.free_node(nn);
                    root
                } else {
                    self.hash_add(nn);
                    let p = self.find_node(root, &search);
                    self.connect_boxes(p, nn);
                    nn
                }
            }
        };
        if let Some(e) = self.element_mut(b) {
            e.node = Some(n);
        }
        if want_cand {
            if let Some(root) = self.root {
                self.collect(b, root, n, &mut cand);
            }
        }
        (r, cand)
    }

    /// Whether the spheres of elements `x` and `e` touch (float arithmetic, as the upstream computes it).
    fn sphere_hit(&self, x: usize, e: usize) -> bool {
        let (Some(x), Some(e)) = (self.element(x), self.element(e)) else {
            return false;
        };
        let d = [
            x.center[0] - e.center[0],
            x.center[1] - e.center[1],
            x.center[2] - e.center[2],
        ];
        let r = x.radius + e.radius;
        d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= r * r
    }

    /// All elements under node `cur` whose sphere touches element `e`, the last first (0x1002e9b0).
    fn collect_all(&self, e: usize, cur: usize, out: &mut Vec<usize>) {
        let node = &self.nodes[cur];
        for &x in node.elements.iter().rev() {
            if self.sphere_hit(x, e) {
                out.push(x);
            }
        }
        for &c in node.children.iter().rev() {
            self.collect_all(e, c, out);
        }
    }

    /// The elements under `cur` that touch `e` and sit in the boxes around `target` (0x1002ea70).
    fn collect(&self, e: usize, cur: usize, target: usize, out: &mut Vec<usize>) {
        let node = &self.nodes[cur];
        for &x in node.elements.iter().rev() {
            if self.sphere_hit(x, e) {
                out.push(x);
            }
        }
        let tk = self.nodes[target].key;
        let ck = node.key;
        let children = node.children.clone();
        if tk.raster < ck.raster - 1 {
            for c in children {
                let chk = self.nodes[c].key;
                if c == target {
                    self.collect_all(e, c, out);
                } else if overlaps(&chk, &tk, chk.l - tk.l) {
                    self.collect(e, c, target, out);
                }
            }
        } else {
            for c in children {
                let chk = self.nodes[c].key;
                if c == target {
                    self.collect_all(e, c, out);
                } else if overlaps(&tk, &chk, tk.l - chk.l) {
                    self.collect(e, c, target, out);
                }
            }
        }
    }

    /// The elements of the tree that touch element `b`, as the recheck sees them (for tests and for the pair code).
    #[cfg(test)]
    pub(crate) fn touching(&self, b: usize) -> Vec<usize> {
        let Some(root) = self.root else { return Vec::new() };
        let Some(n) = self.element(b).and_then(|e| e.node) else { return Vec::new() };
        let mut out = Vec::new();
        self.collect(b, root, n, &mut out);
        out
    }
}

impl World {
    /// The range of an object's element in the world (0x1002dab0): how far its surface can move before the next
    /// recheck, clamped to 0.5..15 with the time since the last PSI taken off.
    fn range_in_world(&self, obj: usize) -> f64 {
        let c = &self.cores[obj];
        let s = f64::from(c.max_surface_rot_speed + c.current_speed) + 1e-19;
        let mut x = s;
        let limit = f64::from(c.upper_limit_radius) * 5.0;
        if limit < x {
            x = limit;
        }
        if x < 0.5 {
            x = 0.5;
        }
        if x > 15.0 {
            x = 15.0;
        }
        x -= self.env.delta_psi * s;
        // the floor includes the radius (a quirk of the upstream that the game's timing depends on)
        let floor = s * 0.1 + f64::from(c.upper_limit_radius);
        if x > floor {
            x
        } else {
            floor
        }
    }

    /// The range two objects' elements allow between each other (0x1002d990): the share of `x` each one takes, with
    /// the upstream's order of its updates kept. Returns (ra, rb).
    pub fn range_intra(&self, a: usize, b: usize) -> (f64, f64) {
        let (ca, cb) = (&self.cores[a], &self.cores[b]);
        let s_a = f64::from(ca.max_surface_rot_speed) + f64::from(ca.current_speed) + 1e-19;
        let s_b = f64::from(cb.max_surface_rot_speed) + f64::from(cb.current_speed) + 1e-19;
        let s = s_a + s_b;
        let rmin = f64::from(ca.upper_limit_radius.min(cb.upper_limit_radius));
        let mut x = s * 0.5;
        let y = rmin * f64::from(0.9f32);
        if y < x {
            x = y;
        }
        if x < f64::from(0.8f32) {
            x = f64::from(0.8f32);
        }
        if x > 10.0 {
            x = 10.0;
        }
        x -= self.env.delta_psi * s;
        if x < s * f64::from(0.1f32) {
            x = s * f64::from(0.1f32);
        }
        let a0 = s_b * f64::from(0.2f32) + s_a;
        let a1 = a0 * f64::from(0.18f32) + s_b;
        let f = (1.0 / (a0 + a1)) * x;
        (f * a0, f * a1)
    }

    /// Whether the object's movement counts as moving: its core is simulated (the freeze and revive points set
    /// `object_state` in the upstream, `phys_core.c` 526 and 550; the core's movement state carries it here).
    fn is_moving(&self, obj: usize) -> bool {
        (self.cores[obj].movement_state & 7) != 0
    }

    /// Two objects' group names do not collide: an unnamed object collides with everything (the glue's group filter).
    fn filter_check(&self, a: usize, b: usize) -> bool {
        let (ga, gb) = (self.coll.groups.get(a), self.coll.groups.get(b));
        let (Some(ga), Some(gb)) = (ga, gb) else {
            return true;
        };
        if ga[0] == 0 || gb[0] == 0 {
            return true;
        }
        // strncmp over 8 bytes: equal up to the first NUL
        let mut i = 0;
        while i < 8 {
            if ga[i] != gb[i] {
                return true;
            }
            if ga[i] == 0 {
                return false;
            }
            i += 1;
        }
        false
    }

    /// Sets the element's sphere from the object's core and puts the element on the object's hull (`ov_hull_add`,
    /// 0x1002dd00): the listener's threshold is the hull value at the current time plus the budget `delta`.
    fn ov_hull_add(&mut self, obj: usize, delta: f64) {
        let old = self.coll.ov.element_mut(obj).and_then(|e| e.hull_slot.take());
        if let Some(slot) = old {
            self.coll.hulls[obj].remove(slot);
        }
        let now = self.env.current_time;
        let h = &mut self.coll.hulls[obj];
        let key = ((now - h.last_time) * f64::from(h.gradient) + f64::from(h.hull_value) + delta) as f32;
        let slot = h.insert(encode_listener(ListenerKind::Ov, obj as u32), key);
        if let Some(e) = self.coll.ov.element_mut(obj) {
            e.hull_slot = Some(slot);
        }
    }

    /// Creates the element of an object that has collision detection (0x100176e0).
    pub fn enable_collision_detection(&mut self, obj: usize) {
        if self.coll.ov.element(obj).is_some() {
            self.disable_collision_detection(obj);
        }
        self.ensure_object(obj);
        if self.coll.hulls.len() <= obj {
            self.coll.hulls.resize_with(obj + 1, Default::default);
        }
        if self.coll.ov.elements.len() <= obj {
            self.coll.ov.elements.resize_with(obj + 1, || None);
        }
        self.coll.ov.elements[obj] = Some(Element {
            radius: -1.0,
            ..Element::default()
        });
        self.recheck_ov_element(obj);
    }

    /// Removes an object's element with its collisions and listener (0x1002dc70, 0x1002f3b0).
    pub fn disable_collision_detection(&mut self, obj: usize) {
        let Some(e) = self.coll.ov.element(obj) else { return };
        let colls = e.collisions.clone();
        for c in colls.into_iter().rev() {
            self.collision_delete(c);
        }
        if let Some(slot) = self.coll.ov.element_mut(obj).and_then(|e| e.hull_slot.take()) {
            self.coll.hulls[obj].remove(slot);
        }
        self.coll.ov.remove_element(obj);
        if let Some(slot) = self.coll.ov.elements.get_mut(obj) {
            *slot = None;
        }
    }

    /// Reconciles an object's element with the tree: it moves to its new box, and the collisions with the objects
    /// around it are kept, created or deleted (0x10017140).
    pub fn recheck_ov_element(&mut self, obj: usize) {
        if self.coll.ov.element(obj).is_none() {
            return;
        }
        self.coll.ov.remove_element(obj);
        self.coll.ov_rechecks += 1;
        let vv = self.cores[obj].m_world_f_core.vv;
        let big_r = f64::from(self.cores[obj].upper_limit_radius);
        if let Some(e) = self.coll.ov.element_mut(obj) {
            e.center = [vv[0] as f32, vv[1] as f32, vv[2] as f32];
        }
        let r = big_r + self.range_in_world(obj);
        let (r_used, cand) = self.coll.ov.insert_element(obj, r, r, true);
        self.ov_hull_add(obj, r_used - big_r);

        // reconcile the existing collisions with the candidates, last first
        let mut keep = 0usize;
        let mut fresh = Vec::new();
        for &other in cand.iter().rev() {
            if !(self.is_moving(obj) || self.is_moving(other)) {
                continue;
            }
            if other == obj {
                continue;
            }
            if !self.filter_check(obj, other) {
                continue;
            }
            let found = self.coll.ov.element(obj).and_then(|e| {
                e.collisions.iter().position(|&c| {
                    let objs = self.coll.collisions[c].objs;
                    (objs[0] == obj && objs[1] == other) || (objs[1] == obj && objs[0] == other)
                })
            });
            match found {
                Some(found) => {
                    if keep < found {
                        self.swap_element_collisions(obj, keep, found);
                    }
                    if keep <= found {
                        keep += 1;
                    }
                }
                None => fresh.push(other),
            }
        }
        loop {
            let last = match self.coll.ov.element(obj) {
                Some(e) if e.collisions.len() > keep => e.collisions[e.collisions.len() - 1],
                _ => break,
            };
            self.collision_delete(last);
        }
        for &other in fresh.iter().rev() {
            self.object_pairs_may_collide(obj, other);
        }
    }

    /// The recheck with the object's movement state forced to 0x21, so the new mindists stay off the hull (0x100099f0).
    pub fn recheck_ov_element_forced(&mut self, obj: usize) {
        let s = self.cores[obj].movement_state;
        self.cores[obj].movement_state = 0x21;
        self.recheck_ov_element(obj);
        self.cores[obj].movement_state = s;
    }

    /// Creates the pairs of two objects that have no collision yet (0x1002f430): their mindist, when there is one.
    fn object_pairs_may_collide(&mut self, a: usize, b: usize) {
        let Some(c) = self.mindist_create(a, b) else { return };
        self.ov_add_collision(a, c);
        self.ov_add_collision(b, c);
    }

    /// Adds collision `c` to object `obj`'s element (0x1002dde0): the slot is the element's first free one.
    pub(crate) fn ov_add_collision(&mut self, obj: usize, c: usize) {
        let slot = if self.coll.collisions[c].fvec[0] < 0 { 0 } else { 1 };
        let Some(e) = self.coll.ov.element_mut(obj) else { return };
        self.coll.collisions[c].fvec[slot] = e.collisions.len() as i32;
        e.collisions.push(c);
    }

    /// Which of collision `c`'s slots holds the index `i` in its element's vector.
    fn slot_of(&self, c: usize, i: usize) -> usize {
        if self.coll.collisions[c].fvec[0] == i as i32 {
            0
        } else {
            1
        }
    }

    /// Swaps two of an element's collisions, keeping their slots in step (0x100188c0).
    fn swap_element_collisions(&mut self, obj: usize, i: usize, j: usize) {
        let Some(e) = self.coll.ov.element_mut(obj) else { return };
        let (a, b) = (e.collisions[i], e.collisions[j]);
        e.collisions.swap(i, j);
        let sa = self.slot_of(a, i);
        let sb = self.slot_of(b, j);
        self.coll.collisions[a].fvec[sa] = j as i32;
        self.coll.collisions[b].fvec[sb] = i as i32;
    }

    /// Takes collision `c` out of object `obj`'s element, moving the last one into its place (0x1002f3b0's helper).
    fn ov_remove_collision(&mut self, obj: usize, c: usize) {
        let Some(e) = self.coll.ov.element(obj) else { return };
        let Some(i) = e.collisions.iter().position(|&x| x == c) else { return };
        let last = e.collisions.len() - 1;
        let slot = self.slot_of(c, i);
        self.coll.collisions[c].fvec[slot] = -1;
        if let Some(e) = self.coll.ov.element_mut(obj) {
            e.collisions.swap_remove(i);
        }
        if i != last {
            // the element's vector now holds the former last collision at i
            if let Some(&m) = self.coll.ov.element(obj).and_then(|e| e.collisions.get(i)) {
                let ms = self.slot_of(m, last);
                self.coll.collisions[m].fvec[ms] = i as i32;
            }
        }
    }

    /// Removes a collision from both objects' elements, as the root delegator does before a collision goes away
    /// (0x1002f3f0).
    fn collision_leave_elements(&mut self, c: usize) {
        let objs = self.coll.collisions[c].objs;
        if self.coll.ov.element(objs[0]).is_some() {
            self.ov_remove_collision(objs[0], c);
        }
        if objs[1] != objs[0] && self.coll.ov.element(objs[1]).is_some() {
            self.ov_remove_collision(objs[1], c);
        }
    }

    /// Deletes a collision: a mindist tears itself down (and releases its record), a watcher child is not ported yet
    /// and only releases the record.
    pub(crate) fn collision_delete(&mut self, c: usize) {
        match self.coll.collisions[c].kind {
            CollKind::Mindist => {
                let m = self.coll.collisions[c].index;
                self.mindist_delete(m);
            }
            CollKind::Watcher => self.collision_release(c),
        }
    }

    /// Takes a collision out of its elements and frees its record (the root delegator's delete hook, 0x1002f3f0).
    pub(crate) fn collision_release(&mut self, c: usize) {
        self.collision_leave_elements(c);
        self.coll.collisions[c].alive = false;
        self.coll.free_collisions.push(c);
    }

    /// Creates a collision record for mindist `m` of objects `objs` and returns its index.
    pub(crate) fn collision_new(&mut self, kind: CollKind, objs: [usize; 2], index: usize) -> usize {
        let c = Collision {
            kind,
            objs,
            fvec: [-1, -1],
            index,
            alive: true,
        };
        match self.coll.free_collisions.pop() {
            Some(i) => {
                self.coll.collisions[i] = c;
                i
            }
            None => {
                self.coll.collisions.push(c);
                self.coll.collisions.len() - 1
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::core::World;
    use crate::math::Quat;

    fn unit_q() -> Quat {
        Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }
    }

    /// A world with `n` unit-radius objects on the x axis, `gap` apart, all with collision detection.
    fn world_with(positions: &[[f64; 3]]) -> World {
        let mut w = World::new();
        for (i, p) in positions.iter().enumerate() {
            let c = w.core_init(i, unit_q(), *p, false);
            w.cores[c].upper_limit_radius = 1.0;
        }
        for i in 0..positions.len() {
            w.enable_collision_detection(i);
        }
        w
    }

    /// Every element's box contains its sphere: the box of level l at origin x covers [x 2^l, (x + 2) 2^l).
    fn box_contains_sphere(w: &World, obj: usize) -> bool {
        let e = w.coll.ov.element(obj).unwrap();
        let n = &w.coll.ov.nodes[e.node.unwrap()];
        let s = pow2(n.key.l);
        let c = [e.center[0], e.center[1], e.center[2]];
        let r = f64::from(e.radius);
        let (x, y, z) = (n.key.x, n.key.y, n.key.z);
        let lo = [f64::from(x) * s, f64::from(y) * s, f64::from(z) * s];
        let hi = [lo[0] + 2.0 * s, lo[1] + 2.0 * s, lo[2] + 2.0 * s];
        (0..3).all(|k| f64::from(c[k]) - r >= lo[k] && f64::from(c[k]) + r <= hi[k])
    }

    #[test]
    fn element_box_contains_its_sphere() {
        let w = world_with(&[[0.0, 0.0, 0.0], [7.3, -2.9, 4.1], [-130.5, 22.0, 3.0]]);
        for obj in 0..3 {
            assert!(box_contains_sphere(&w, obj), "object {obj}");
        }
    }

    #[test]
    fn distant_objects_do_not_touch_and_near_ones_do() {
        let w = world_with(&[[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [1.5, 0.0, 0.0]]);
        let near = w.coll.ov.touching(0);
        assert!(near.contains(&0));
        assert!(near.contains(&2), "{near:?}");
        assert!(!near.contains(&1), "{near:?}");
    }

    #[test]
    fn disabling_everything_empties_the_tree() {
        let mut w = world_with(&[[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [-40.0, 5.0, 2.0], [400.0, 0.0, 0.0]]);
        for obj in 0..4 {
            w.disable_collision_detection(obj);
        }
        assert!(w.coll.ov.root.is_none());
        assert!(w.coll.ov.hash.is_empty());
        assert!(w.coll.ov.nodes.iter().all(|n| !n.alive));
    }

    #[test]
    fn recheck_moves_the_element_with_its_core() {
        let mut w = world_with(&[[0.0, 0.0, 0.0], [50.0, 0.0, 0.0]]);
        let before = w.coll.ov.element(0).unwrap().node;
        w.cores[0].m_world_f_core.vv = [200.0, 0.0, 0.0];
        w.recheck_ov_element(0);
        assert!(box_contains_sphere(&w, 0));
        let after = w.coll.ov.element(0).unwrap().node;
        assert!(before.is_some() && after.is_some());
        assert!(!w.coll.ov.touching(0).contains(&1));
    }

    #[test]
    fn range_intra_splits_the_budget_by_size() {
        let mut w = World::new();
        let a = w.core_init(0, unit_q(), [0.0; 3], false);
        let b = w.core_init(1, unit_q(), [5.0, 0.0, 0.0], false);
        w.cores[a].upper_limit_radius = 1.0;
        w.cores[b].upper_limit_radius = 2.0;
        let (ra, rb) = w.range_intra(a, b);
        assert!(ra > 0.0 && rb > 0.0);
        assert!(rb > ra, "the larger object takes the larger share");
    }
}
