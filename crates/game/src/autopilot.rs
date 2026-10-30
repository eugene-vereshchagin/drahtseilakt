// A test driver for the prototype: it plays a level with the keys a player has, choosing each tick's input from
// a distance field over the physical floor. EXPERIMENTAL: it does not finish level 1 yet. The route has gaps that
// the ball crosses by falling, and a planner over the floor's triangles cannot represent a fall. Kept as a tool
// for the phase 3 work on routes; it is not part of the game.

use crate::collide::Collider;
use crate::sim::{Sim, input};
use glam::Vec3;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// The inputs worth trying: each direction on its own and the diagonals, plus coasting.
const CHOICES: [u8; 9] = [
    0,
    input::UP,
    input::UP | input::RIGHT,
    input::RIGHT,
    input::DOWN | input::RIGHT,
    input::DOWN,
    input::DOWN | input::LEFT,
    input::LEFT,
    input::UP | input::LEFT,
];

/// Ticks each candidate is simulated ahead.
const HORIZON: u32 = 30;

/// Distance field over the triangles of the physical floor, from a target. `cost` is in millimetres.
pub struct Route {
    centroids: Vec<Vec3>,
    cost: Vec<u64>,
}

impl Route {
    pub fn new(collider: &Collider, target: Vec3) -> Route {
        let tris = &collider.triangles;
        let centroids: Vec<Vec3> = tris.iter().map(|t| (t.a + t.b + t.c) / 3.0).collect();
        // Neighbours: triangles with a corner within a metre of one of this triangle's corners (floors and rails
        // meet with small gaps, so sharing exact vertices is too strict). The edge costs the distance between
        // the centres.
        const LINK: f32 = 4.0;
        let adjacent: Vec<Vec<(usize, u64)>> = tris
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let mut out: Vec<(usize, u64)> = Vec::new();
                for corner in [t.a, t.b, t.c] {
                    for j in collider.nearby(corner, LINK) {
                        let j = j as usize;
                        if j == i || out.iter().any(|&(k, _)| k == j) {
                            continue;
                        }
                        let d = centroids[i].distance(centroids[j]);
                        out.push((j, (d * 1000.0) as u64));
                    }
                }
                out
            })
            .collect();
        // Sources: the triangles nearest the target.
        let mut cost = vec![u64::MAX; tris.len()];
        let mut heap = BinaryHeap::new();
        let near: Vec<usize> = (0..tris.len())
            .filter(|&i| centroids[i].distance(target) < 6.0)
            .collect();
        let near = if near.is_empty() {
            (0..tris.len())
                .min_by(|&a, &b| {
                    centroids[a]
                        .distance(target)
                        .total_cmp(&centroids[b].distance(target))
                })
                .into_iter()
                .collect()
        } else {
            near
        };
        for i in near {
            cost[i] = 0;
            heap.push(Reverse((0u64, i)));
        }
        while let Some(Reverse((c, i))) = heap.pop() {
            if c > cost[i] {
                continue;
            }
            for &(j, step) in &adjacent[i] {
                let next = c + step;
                if next < cost[j] {
                    cost[j] = next;
                    heap.push(Reverse((next, j)));
                }
            }
        }
        Route { centroids, cost }
    }

    /// Distance to the target along the floor from the floor under `p`, in metres, plus how far `p` is from
    /// that floor. A large value when no floor is near.
    pub fn at(&self, collider: &Collider, p: Vec3) -> f32 {
        let mut best = f32::INFINITY;
        for i in collider.nearby(p, 10.0) {
            let i = i as usize;
            let (Some(c), Some(&k)) = (self.centroids.get(i), self.cost.get(i)) else {
                continue;
            };
            if k == u64::MAX {
                continue;
            }
            let off = c.distance(p);
            let total = k as f32 / 1000.0 + off;
            if total < best {
                best = total;
            }
        }
        if best.is_finite() { best } else { 1.0e6 }
    }
}

/// Plays one tick: the input chosen by looking ahead over the route to the next target.
pub fn choose(sim: &Sim, route: &Route) -> u8 {
    let collider = sim.collider();
    let lives = sim.lives;
    let mut best = (f32::INFINITY, 0u8);
    for &keys in &CHOICES {
        let mut look = sim.clone();
        for _ in 0..HORIZON {
            look.step(keys);
            if look.lives < lives || look.finished {
                break;
            }
        }
        let s = if look.lives < lives {
            f32::INFINITY
        } else if look.finished {
            -1.0e6
        } else {
            route.at(collider, look.ball.position) - 200.0 * look.sector as f32
        };
        if s < best.0 {
            best = (s, keys);
        }
    }
    best.1
}
