// Port of OpenBallance src/phys/phys_time.c, the hull manager part (docs/ivp_collision.md 2.1.6 and 2.3; GPL-3.0).
// A hull manager keeps the growth of an object's hull as a linear function of time and a sorted list of listeners
// whose thresholds it must watch. The events themselves are dispatched by the owner of the arena (the world); this
// module keeps the arithmetic and the list, and reports which listeners have been crossed.

use crate::minlist::MinList;

/// What a hull listener is (the upstream's three vtables, 2.1.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListenerKind {
    /// A synapse of a hull mindist (`IVP_HL_SYNAPSE`).
    Synapse,
    /// An OV element (`IVP_HL_OV`).
    Ov,
    /// A watcher's hull synapse (`IVP_HL_WATCHER`).
    Watcher,
}

/// A hull manager (2.1.6): `hull_value(t) = hull_value + (t - last_time) * gradient`.
#[derive(Clone, Debug)]
pub struct HullManager {
    pub last_time: f64,
    pub gradient: f32,
    pub center_gradient: f32,
    pub hull_value: f32,
    pub center_hull_value: f32,
    pub hull_value_next_psi: f32,
    pub reset_time: f64,
    pub list: MinList,
}

/// Where a listener's value is kept while it is in the list: the owner gives its arena index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listener {
    pub kind: ListenerKind,
    pub index: u32,
}

impl Default for HullManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HullManager {
    pub fn new() -> HullManager {
        HullManager {
            last_time: 0.0,
            gradient: 0.0,
            center_gradient: 0.0,
            hull_value: 0.0,
            center_hull_value: 0.0,
            hull_value_next_psi: 0.0,
            reset_time: 0.0,
            list: MinList::new(),
        }
    }

    /// The hull value at time `t` (`0x1001e6d0`-style linear growth since `last_time`).
    pub fn value_at(&self, t: f64) -> f32 {
        ((t - self.last_time) * f64::from(self.gradient) + f64::from(self.hull_value)) as f32
    }

    /// Adds a listener with its threshold (the slot is returned to the caller to keep).
    pub fn insert(&mut self, payload: u32, key: f32) -> u32 {
        self.list.add(payload, key)
    }

    pub fn remove(&mut self, slot: u32) {
        self.list.remove(slot);
    }

    /// The update after an integration step (`0x1001e750`): the values grow to `now`, the gradient becomes the
    /// new speed (scaled by 1.00001, the upstream's margin), and the next step's value is predicted.
    pub fn update(&mut self, now: f64, dt: f32, speed: f32, center_speed: f32) {
        let d = now - self.last_time;
        self.last_time = now;
        self.hull_value = (f64::from(self.hull_value) + d * f64::from(self.gradient)) as f32;
        self.center_hull_value = (f64::from(self.center_hull_value) + d * f64::from(self.center_gradient)) as f32;
        self.center_gradient = center_speed;
        self.gradient = speed * 1.00001;
        self.hull_value_next_psi = (f64::from(self.gradient) * f64::from(dt) + f64::from(self.hull_value)) as f32;
    }

    /// The rebasing to zero every ten seconds (`0x1001a820`). Returns the listeners whose synapses need the same
    /// shift, in list order, so that the owner can apply the mindist part.
    pub fn reset(&mut self) -> Vec<u32> {
        let dh = -self.hull_value;
        // Every value moves by dh and keeps its order; the minimum moves with them.
        let touched: Vec<u32> = self.list.iter().map(|(_, _, payload)| payload).collect();
        self.list.shift_all(-dh);
        self.hull_value = 0.0;
        self.center_hull_value = 0.0;
        self.hull_value_next_psi += dh;
        touched
    }

    /// The listeners whose threshold the hull has crossed at the predicted next step, most urgent first, at most
    /// `limit + 1` of them (the upstream's 102-event bound per manager per phase). Each entry is (payload, intrusion).
    pub fn crossed(&self, limit: usize) -> Vec<(u32, f32)> {
        let mut out = Vec::new();
        for (_, value, payload) in self.list.iter() {
            let d = value - self.hull_value_next_psi;
            if d >= 0.0 {
                break;
            }
            out.push((payload, d));
            if out.len() > limit {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_grows_linearly_with_time() {
        let mut h = HullManager::new();
        h.update(0.0, 1.0 / 66.0, 2.0, 0.0);
        h.update(1.0, 1.0 / 66.0, 2.0, 0.0);
        assert!((h.value_at(1.5) - (h.hull_value + 0.5 * h.gradient)).abs() < 1e-5);
    }

    #[test]
    fn crossed_lists_only_listeners_below_the_predicted_hull() {
        let mut h = HullManager::new();
        h.insert(10, 0.5);
        h.insert(11, 3.0);
        h.insert(12, 1.0);
        h.hull_value_next_psi = 2.0;
        let c: Vec<u32> = h.crossed(100).into_iter().map(|c| c.0).collect();
        assert_eq!(c, vec![10, 12]);
    }

    #[test]
    fn reset_rebases_the_list_by_the_hull_value() {
        let mut h = HullManager::new();
        h.insert(7, 5.0);
        h.hull_value = 2.0;
        h.hull_value_next_psi = 3.0;
        let touched = h.reset();
        assert_eq!(touched, vec![7]);
        assert_eq!(h.hull_value, 0.0);
        assert!((h.list.min_value() - 3.0).abs() < 1e-6);
        assert!((h.hull_value_next_psi - 1.0).abs() < 1e-6);
    }
}
