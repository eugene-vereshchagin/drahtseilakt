// Port of OpenBallance src/phys/phys_time.c, the time manager and its event loop (docs/ivp_collision.md 2.1.4, 2.2.5;
// GPL-3.0). Events are the PSI event and the mindist events; the list is ordered by time relative to a base, so the
// rebase at each PSI keeps the values small. The mindist event handler is a hook (see `collision.rs`) until the
// collision port lands.

use crate::core::World;
use crate::minlist::{MinList, NO_INDEX};

/// The payload of the PSI event in the time list. Mindist events carry their own index below this value.
pub const PSI_EVENT: u32 = u32::MAX - 1;

#[derive(Debug, Clone)]
pub struct TimeManager {
    pub list: MinList,
    /// The slot of the PSI event while it is in the list.
    pub psi_slot: u32,
    pub last_rel: f64,
    pub base: f64,
}

impl Default for TimeManager {
    fn default() -> Self {
        TimeManager {
            list: MinList::new(),
            psi_slot: NO_INDEX,
            last_rel: 0.0,
            base: 0.0,
        }
    }
}

impl World {
    /// Starts the time manager with the first PSI at t = 0 (`0x1002f0b0`).
    pub fn time_init(&mut self) {
        self.time.list = MinList::new();
        self.time.base = 0.0;
        self.time.last_rel = 0.0;
        self.time_insert(PSI_EVENT, 0.0);
    }

    /// Inserts an event at absolute time `t`; its key is `(f32)(t - base)` (`0x1002f1d0`). Returns the slot.
    pub fn time_insert(&mut self, payload: u32, t: f64) -> u32 {
        let key = (t - self.time.base) as f32;
        let slot = self.time.list.add(payload, key);
        if payload == PSI_EVENT {
            self.time.psi_slot = slot;
        }
        slot
    }

    /// Removes an event by its slot (`0x1002f200`).
    pub fn time_remove(&mut self, slot: u32) {
        self.time.list.remove(slot);
        if self.time.psi_slot == slot {
            self.time.psi_slot = NO_INDEX;
        }
    }

    /// Runs every event strictly before `target`, in list order, then sets the current time to `target`
    /// (`0x1002f250` -> `0x1002ef70`).
    pub fn time_simulate_until(&mut self, target: f64) {
        loop {
            let m = self.time.list.min_value();
            if !(target - self.time.base > f64::from(m)) {
                break;
            }
            let Some((slot, payload)) = self.time.list.first() else {
                break;
            };
            self.time.list.remove(slot);
            self.time.last_rel = f64::from(m);
            self.set_current_time(f64::from(m) + self.time.base);
            if payload == PSI_EVENT {
                self.psi_event();
            } else {
                self.mindist_simulate_event(payload);
            }
        }
        self.set_current_time(target);
    }

    /// Sets the current time and counts the change (`0x100138f0`).
    fn set_current_time(&mut self, t: f64) {
        self.env.current_time = t;
        self.time_code = self.time_code.wrapping_add(1);
    }

    /// The PSI event (`0x1002f2c0`): the next PSI is due one step later; the list is rebased to the current time;
    /// the simulation runs one PSI; the next PSI event is inserted.
    fn psi_event(&mut self) {
        self.env.time_of_last_psi = self.env.current_time;
        self.env.time_of_next_psi = self.env.time_of_last_psi + self.env.delta_psi;
        let t = self.env.time_of_last_psi;
        let d = t - self.time.base;
        self.time.list.shift_all(d as f32);
        self.time.base = t;
        self.time.last_rel = 0.0;
        self.simulate_psi();
        let next = self.env.time_of_next_psi;
        self.time_insert(PSI_EVENT, next);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::core::World;

    #[test]
    fn psi_events_run_at_the_psi_rate_without_bodies() {
        let mut w = World::new();
        w.time_init();
        w.env.time_of_next_psi = w.env.delta_psi;
        w.time_simulate_until(1.0);
        // 66 PSIs per second of game time, the first at t = 0, the last before t = 1.
        assert!(w.env.time_of_last_psi > 0.95 && w.env.time_of_last_psi <= 1.0);
        assert_eq!(w.env.current_time, 1.0);
        assert!(w.env.state != 0 || w.time.list.len() == 1);
    }

    #[test]
    fn time_is_monotonic_and_rebased() {
        let mut w = World::new();
        w.time_init();
        w.env.time_of_next_psi = w.env.delta_psi;
        w.time_simulate_until(0.5);
        let base_after_half = w.time.base;
        w.time_simulate_until(0.6);
        assert!(w.time.base >= base_after_half);
        assert!(w.time.list.min_value() >= 0.0);
    }
}
