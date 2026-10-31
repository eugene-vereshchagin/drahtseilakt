// Port of OpenBallance src/phys/phys_time.c, the min list part (GPL-3.0; see the LICENSE of the repository).
// IVP_U_Min_List (0x100300f0 / 0x10030180 / 0x10030470): a sorted doubly linked list over a slot array. A new element
// goes before the first one with a value >= its own, so ties keep the newest first; the upstream notes that a binary
// heap would not do this (docs/ivp_collision.md 2.1.5). Payloads are indices into the owner's arena, not pointers.

/// The "no index" sentinel of the upstream (`0xffffffff`).
pub const NO_INDEX: u32 = u32::MAX;

#[derive(Clone, Copy, Debug)]
struct Elem {
    value: f32,
    payload: u32,
    prev: u32,
    next: u32,
}

#[derive(Clone, Debug)]
pub struct MinList {
    elems: Vec<Elem>,
    free_head: u32,
    first: u32,
    last: u32,
    n: u32,
    /// The first value; 1e10 when empty.
    min_value: f32,
}

impl Default for MinList {
    fn default() -> Self {
        Self::new()
    }
}

impl MinList {
    pub fn new() -> MinList {
        MinList { elems: Vec::new(), free_head: NO_INDEX, first: NO_INDEX, last: NO_INDEX, n: 0, min_value: 1e10 }
    }

    pub fn len(&self) -> usize {
        self.n as usize
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    pub fn min_value(&self) -> f32 {
        self.min_value
    }

    /// Adds a payload with a key; returns the slot index the caller keeps to remove it later.
    pub fn add(&mut self, payload: u32, value: f32) -> u32 {
        if self.free_head == NO_INDEX {
            // Grow to 2n + 1, threading the new slots on the free list (upstream order: highest first).
            let old = self.elems.len() as u32;
            let cap = old * 2 + 1;
            self.elems.resize(cap as usize, Elem { value: 0.0, payload: 0, prev: NO_INDEX, next: NO_INDEX });
            for i in (old..cap).rev() {
                self.elems[i as usize].next = self.free_head;
                self.free_head = i;
            }
        }
        let i = self.free_head;
        self.free_head = self.elems[i as usize].next;
        self.elems[i as usize].value = value;
        self.elems[i as usize].payload = payload;
        if self.n == 0 || value <= self.min_value {
            self.elems[i as usize].prev = NO_INDEX;
            self.elems[i as usize].next = self.first;
            if self.first != NO_INDEX {
                self.elems[self.first as usize].prev = i;
            } else {
                self.last = i;
            }
            self.first = i;
            self.min_value = value;
        } else {
            // After the last element with a smaller value, that is before the first one >= value.
            let mut p = self.last;
            while self.elems[p as usize].value >= value {
                p = self.elems[p as usize].prev;
            }
            let next = self.elems[p as usize].next;
            self.elems[i as usize].prev = p;
            self.elems[i as usize].next = next;
            if next != NO_INDEX {
                self.elems[next as usize].prev = i;
            } else {
                self.last = i;
            }
            self.elems[p as usize].next = i;
        }
        self.n += 1;
        i
    }

    /// Removes the element at slot `i` (`0x10030470`).
    pub fn remove(&mut self, i: u32) {
        let x = self.elems[i as usize];
        if x.prev != NO_INDEX {
            self.elems[x.prev as usize].next = x.next;
        } else {
            self.first = x.next;
        }
        if x.next != NO_INDEX {
            self.elems[x.next as usize].prev = x.prev;
        } else {
            self.last = x.prev;
        }
        self.n -= 1;
        self.min_value = if self.first != NO_INDEX { self.elems[self.first as usize].value } else { 1e10 };
        self.elems[i as usize].next = self.free_head;
        self.free_head = i;
    }

    /// The first element's payload and slot, if any.
    pub fn first(&self) -> Option<(u32, u32)> {
        (self.n > 0).then(|| (self.first, self.elems[self.first as usize].payload))
    }

    /// Payloads in list order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, f32, u32)> + '_ {
        let mut i = self.first;
        std::iter::from_fn(move || {
            if i == NO_INDEX {
                return None;
            }
            let e = self.elems[i as usize];
            let slot = i;
            i = e.next;
            Some((slot, e.value, e.payload))
        })
    }

    /// Subtracts `d` from every value and from the minimum, as the PSI and hull resets do.
    pub fn shift_all(&mut self, d: f32) {
        let mut i = self.first;
        while i != NO_INDEX {
            let next = self.elems[i as usize].next;
            self.elems[i as usize].value -= d;
            i = next;
        }
        self.min_value = if self.n > 0 { self.min_value - d } else { 1e10 };
    }

    /// Changes the value of a slot in place (the caller keeps the order valid).
    pub fn set_value(&mut self, i: u32, value: f32) {
        self.elems[i as usize].value = value;
        if self.first == i {
            self.min_value = value;
        }
    }

    pub fn value(&self, i: u32) -> f32 {
        self.elems[i as usize].value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(l: &MinList) -> Vec<u32> {
        l.iter().map(|(_, _, p)| p).collect()
    }

    #[test]
    fn keeps_values_sorted_and_ties_newest_first() {
        let mut l = MinList::new();
        l.add(1, 3.0);
        l.add(2, 1.0);
        l.add(3, 2.0);
        l.add(4, 1.0);
        assert_eq!(order(&l), vec![4, 2, 3, 1]);
        assert_eq!(l.min_value(), 1.0);
    }

    #[test]
    fn remove_updates_the_first_value_and_reuses_slots() {
        let mut l = MinList::new();
        let a = l.add(1, 1.0);
        l.add(2, 5.0);
        l.remove(a);
        assert_eq!(order(&l), vec![2]);
        assert_eq!(l.min_value(), 5.0);
        let b = l.add(3, 0.5);
        assert_eq!(b, a, "the freed slot is reused first");
        l.remove(b);
        l.remove(2);
        assert!(l.is_empty());
        assert_eq!(l.min_value(), 1e10);
    }

    #[test]
    fn shift_moves_everything_and_keeps_order() {
        let mut l = MinList::new();
        l.add(1, 4.0);
        l.add(2, 9.0);
        l.shift_all(3.0);
        assert_eq!(order(&l), vec![1, 2]);
        assert_eq!(l.min_value(), 1.0);
        assert_eq!(l.value(l.first().unwrap().0), 1.0);
    }
}
