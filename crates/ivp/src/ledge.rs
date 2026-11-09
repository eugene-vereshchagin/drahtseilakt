// Port of OpenBallance src/phys/phys_ledge.h, phys_surface.c and the ledge generator of phys_convex.c (GPL-3.0; see the
// LICENSE of the repository): IVP's compact collision geometry (docs/ivp_collision.md 1.2 and 1.6).
//
// The upstream stores a ledge and a surface in one byte block that the solvers walk with pointer arithmetic. Here a
// ledge is its points and its triangles, an edge is the pair (triangle, slot) where slot 0..3 is the edge from point
// `slot` to point `slot + 1`, and a surface's ledge tree is a vector of nodes in the upstream's preorder (the left child
// follows its parent). The values the solvers read are the upstream's; the layout is not.

use crate::convex;

pub type Point = [f32; 3];
type V3 = [f64; 3];

/// An edge: slot `slot` of triangle `tri` of its ledge (`IvpEdge`, without the byte offsets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub tri: usize,
    pub slot: usize,
}

/// A triangle of a ledge: the points of its three edges' starts, its pierce partner (the triangle the mindist code pairs
/// it with), and the twin edge of each of its edges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Triangle {
    pub start: [usize; 3],
    pub pierce: usize,
    pub opposite: [Edge; 3],
}

/// A convex piece of collision geometry (`IvpCompactLedge`): its points and its triangles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ledge {
    pub points: Vec<Point>,
    pub tris: Vec<Triangle>,
}

impl Ledge {
    /// The start point of an edge.
    pub fn p(&self, e: Edge) -> Point {
        self.points[self.tris[e.tri].start[e.slot]]
    }

    /// The next edge around the same triangle (`ivp_next`).
    pub fn next(e: Edge) -> Edge {
        Edge {
            tri: e.tri,
            slot: (e.slot + 1) % 3,
        }
    }

    /// The previous edge around the same triangle (`ivp_prev`).
    pub fn prev(e: Edge) -> Edge {
        Edge {
            tri: e.tri,
            slot: (e.slot + 2) % 3,
        }
    }

    /// The twin edge in the neighbouring triangle (`ivp_opp`).
    pub fn opp(&self, e: Edge) -> Edge {
        self.tris[e.tri].opposite[e.slot]
    }

    /// The edge's first point and its triangle's index, the two things a solver reads from an edge.
    pub fn first_edge(&self) -> Option<Edge> {
        (!self.tris.is_empty()).then_some(Edge { tri: 0, slot: 0 })
    }
}

/// A two-sided triangle ledge (`ivp_ledge_triangle`, 0x1003ac00): the triangle and its reverse, with the corners
/// stored as f32. None when the triangle is degenerate (|(p1 - p0) x (p2 - p0)|^2 < 1e-12).
pub fn triangle(p0: V3, p1: V3, p2: V3) -> Option<Ledge> {
    let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let v = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let c = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    if c[0] * c[0] + c[1] * c[1] + c[2] * c[2] < 1e-12 {
        return None;
    }
    let pts: Vec<Point> = [p0, p1, p2]
        .iter()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .collect();
    generate(&pts, &[[0, 1, 2], [1, 0, 2]], &[1, 0])
}

/// A ledge of a point set (`ivp_ledge_from_points`, 0x1003ae20): the points are deduplicated by their bits first.
pub fn from_points(p: &[V3]) -> Option<Ledge> {
    if p.len() < 3 {
        return None;
    }
    if p.len() == 3 {
        return triangle(p[0], p[1], p[2]);
    }
    let mut q: Vec<V3> = Vec::with_capacity(p.len());
    for pt in p {
        let bits = pt.map(f64::to_bits);
        if !q.iter().any(|k| k.map(f64::to_bits) == bits) {
            q.push(*pt);
        }
    }
    match q.len() {
        3 => triangle(q[0], q[1], q[2]),
        m if m < 3 => None,
        _ => convex::hull(&q),
    }
}

/// Builds a ledge from triangles over a point list (`ivp_ledge_generate`, 0x10048b60 / 0x10048ed0). `tri[i]` are point
/// indices in ring order, `pierce[i]` the partner of triangle i. Points get indices in first-appearance order. None
/// when an edge has no twin.
pub fn generate(pts: &[Point], tri: &[[usize; 3]], pierce: &[usize]) -> Option<Ledge> {
    let maxv = tri.iter().flatten().copied().max().unwrap_or(0);
    let mut idx: Vec<Option<usize>> = vec![None; maxv + 1];
    let mut order: Vec<usize> = Vec::new();
    for t in tri {
        for &v in t {
            if idx[v].is_none() {
                idx[v] = Some(order.len());
                order.push(v);
            }
        }
    }
    let points: Vec<Point> = order.iter().map(|&v| pts[v]).collect();
    let mut tris = Vec::with_capacity(tri.len());
    for (i, t) in tri.iter().enumerate() {
        let mut opposite = [Edge { tri: 0, slot: 0 }; 3];
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            // the twin: the first edge b -> a of the list
            let twin = tri.iter().enumerate().find_map(|(j, u)| {
                (0..3)
                    .find(|&m| u[m] == b && u[(m + 1) % 3] == a)
                    .map(|m| Edge { tri: j, slot: m })
            })?;
            opposite[k] = twin;
        }
        tris.push(Triangle {
            start: [idx[t[0]]?, idx[t[1]]?, idx[t[2]]?],
            pierce: pierce.get(i).copied().unwrap_or(i),
            opposite,
        });
    }
    Some(Ledge { points, tris })
}

/// The axis-aligned box of a ledge's points, min first (`0x100202e0`).
fn ledge_aabb(l: &Ledge) -> (Point, Point) {
    let Some(first) = l.tris.first() else {
        return ([0.0; 3], [0.0; 3]);
    };
    let p0 = l.points[first.start[0]];
    let mut mn = p0;
    let mut mx = p0;
    for t in &l.tris {
        for &s in &t.start {
            let p = l.points[s];
            for k in 0..3 {
                if p[k] < mn[k] {
                    mn[k] = p[k];
                }
                if p[k] > mx[k] {
                    mx[k] = p[k];
                }
            }
        }
    }
    (mn, mx)
}

/// A node of a surface's ledge tree (`IvpLedgeNode`): a bounding sphere and an AABB half-extent in units of
/// radius * 0.004, and either a leaf's ledge or two children.
#[derive(Clone, Debug)]
pub struct LedgeNode {
    pub center: [f32; 3],
    pub radius: f32,
    pub box_sizes: [u8; 3],
    pub kind: NodeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A leaf: the index of its ledge in `Surface::ledges`.
    Leaf { ledge: usize },
    /// An inner node: the indices of its children in `Surface::tree`.
    Inner { left: usize, right: usize },
}

/// A compact surface (`IvpCompactSurface`, phys_ledge.h): the mass properties, the ledge tree and the ledges.
#[derive(Clone, Debug)]
pub struct Surface {
    pub mass_center: [f32; 3],
    pub rotation_inertia: [f32; 3],
    pub upper_limit_radius: f32,
    pub max_factor_surface_deviation: u8,
    /// The tree in preorder; `tree[0]` is the root.
    pub tree: Vec<LedgeNode>,
    pub ledges: Vec<Ledge>,
    /// How often the tree builder had to split an empty side in list order (9.6).
    pub split_guards: u32,
}

/// The sphere record of the tree builder (0x10038750): a leaf's ledge, or the two spheres under an inner node.
#[derive(Clone, Debug)]
struct Sphere {
    center: V3,
    radius: f64,
    box_sizes: [u8; 3],
    ledge: Option<usize>,
    children: Option<(usize, usize)>,
}

fn box_size(extent: f64, radius: f64) -> u8 {
    ((extent / (0.004 * radius)) as i32 + 1) as u8
}

/// The box of a list of spheres, each member's box at its centre (`union_boxes`, FUN_10038b40).
fn union_boxes(arena: &[Sphere], list: &[usize]) -> ([f32; 3], [f32; 3]) {
    let mut bmin = [1e6f32; 3];
    let mut bmax = [-1e6f32; 3];
    for &i in list {
        let s = &arena[i];
        for k in 0..3 {
            let h = f64::from(s.box_sizes[k]) * (0.004 * s.radius);
            let lo = (s.center[k] - h) as f32;
            let hi = (s.center[k] + h) as f32;
            if lo < bmin[k] {
                bmin[k] = lo;
            }
            if hi > bmax[k] {
                bmax[k] = hi;
            }
        }
    }
    (bmin, bmax)
}

fn box_volume(arena: &[Sphere], list: &[usize]) -> f32 {
    let (a, b) = union_boxes(arena, list);
    (b[0] - a[0]).abs() * (b[1] - a[1]).abs() * (b[2] - a[2]).abs()
}

/// The top-down tree builder (0x10038c90): per axis the members split at the box midpoint; the axis with the smallest
/// summed box volume wins. `guards` counts the splits that had to fall back to list order.
fn build(arena: &mut Vec<Sphere>, list: &[usize], guards: &mut u32) -> usize {
    if list.len() == 1 {
        return list[0];
    }
    let (bmin, bmax) = union_boxes(arena, list);
    let mut center = [0.0f64; 3];
    let mut r2 = 0.0f64;
    for k in 0..3 {
        center[k] = 0.5 * f64::from(bmin[k]) + 0.5 * f64::from(bmax[k]);
        let d = bmax[k] - center[k] as f32;
        r2 += f64::from(d) * f64::from(d);
    }
    let radius = r2.sqrt();
    let mut box_sizes = [0u8; 3];
    for k in 0..3 {
        box_sizes[k] = box_size(f64::from(bmax[k]) - center[k], radius);
    }
    let node = arena.len();
    arena.push(Sphere {
        center,
        radius,
        box_sizes,
        ledge: None,
        children: None,
    });
    if list.len() == 2 {
        arena[node].children = Some((list[0], list[1]));
        return node;
    }

    let n = list.len();
    let eps = f64::from(1e-6f32);
    let mut low: [Vec<usize>; 3] = Default::default();
    let mut high: [Vec<usize>; 3] = Default::default();
    let mut cost = [0.0f32; 3];
    for k in 0..3 {
        let split = f64::from(bmin[k]) + (f64::from(bmax[k]) - f64::from(bmin[k])) * 0.5;
        let mut toggle = true;
        for j in 0..n {
            let c = arena[list[j]].center[k];
            if c < split - eps {
                low[k].push(list[j]);
            } else if c > split + eps {
                high[k].push(list[j]);
            } else {
                // a centre on the midplane follows its neighbour in the list, or alternates
                let nb = if j < n - 1 {
                    arena[list[j + 1]].center[k]
                } else {
                    arena[list[j - 1]].center[k]
                };
                if nb < split - eps {
                    high[k].push(list[j]);
                } else if nb > split + eps {
                    low[k].push(list[j]);
                } else if toggle {
                    low[k].push(list[j]);
                    toggle = false;
                } else {
                    high[k].push(list[j]);
                    toggle = true;
                }
            }
        }
        cost[k] = box_volume(arena, &low[k]) + box_volume(arena, &high[k]);
    }
    let ks = if cost[1] <= cost[0] {
        if cost[1] < cost[2] { 1 } else { 2 }
    } else if cost[0] < cost[2] {
        0
    } else {
        2
    };
    if low[ks].is_empty() || high[ks].is_empty() {
        // the upstream recurses forever here (9.6); split in list order instead
        *guards += 1;
        low[ks] = list[..n / 2].to_vec();
        high[ks] = list[n / 2..].to_vec();
    }
    let left = build(arena, &low[ks], guards);
    let right = build(arena, &high[ks], guards);
    arena[node].children = Some((left, right));
    node
}

/// The ledges of the tree's leaves, the leaves in the order a depth-first walk meets them.
fn leaf_ledges(tree: &[LedgeNode]) -> Vec<usize> {
    tree.iter()
        .filter_map(|n| match n.kind {
            NodeKind::Leaf { ledge } => Some(ledge),
            NodeKind::Inner { .. } => None,
        })
        .collect()
}

/// Second moments of the closed surface about `mc`: V and the integrals of x^2, y^2, z^2 over the tetrahedra to `mc`.
fn second_moments(ledges: &[&Ledge], mc: V3) -> V3 {
    let mut vol = 0.0;
    let mut inte = [0.0f64; 3];
    for l in ledges {
        for t in &l.tris {
            let mut a = [[0.0f64; 3]; 3];
            for (e, row) in a.iter_mut().enumerate() {
                let p = l.points[t.start[e]];
                for k in 0..3 {
                    row[k] = f64::from(p[k]) - mc[k];
                }
            }
            let [a, b, c] = a;
            let v6 = a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]);
            vol += v6 / 6.0;
            for k in 0..3 {
                inte[k] += v6 / 60.0
                    * (a[k] * a[k]
                        + b[k] * b[k]
                        + c[k] * c[k]
                        + a[k] * b[k]
                        + a[k] * c[k]
                        + b[k] * c[k]);
            }
        }
    }
    let mut m = [0.0; 3];
    for k in 0..3 {
        m[k] = if vol < 1e-19 { 1.0 } else { inte[k] / vol };
    }
    m
}

impl Surface {
    /// Compiles a list of ledges into a surface (`ivp_surface_compile`, 0x100384b0). None for an empty list.
    pub fn compile(ledges: Vec<Ledge>) -> Option<Surface> {
        if ledges.is_empty() {
            return None;
        }
        // a sphere per ledge, in insertion order
        let mut arena: Vec<Sphere> = Vec::with_capacity(2 * ledges.len());
        let mut list = Vec::with_capacity(ledges.len());
        for (i, l) in ledges.iter().enumerate() {
            let (mn, mx) = ledge_aabb(l);
            let mut center = [0.0f64; 3];
            let mut r2 = 0.0f64;
            for k in 0..3 {
                center[k] = 0.5 * f64::from(mn[k]) + 0.5 * f64::from(mx[k]);
                let d = f64::from(mx[k]) - center[k];
                r2 += d * d;
            }
            let radius = r2.sqrt();
            let mut box_sizes = [0u8; 3];
            for k in 0..3 {
                box_sizes[k] = box_size(f64::from(mx[k]) - center[k], radius);
            }
            list.push(arena.len());
            arena.push(Sphere {
                center,
                radius,
                box_sizes,
                ledge: Some(i),
                children: None,
            });
        }

        let mut guards = 0;
        let root = build(&mut arena, &list, &mut guards);

        // the preorder tree, left subtree first
        let mut tree = Vec::new();
        write_tree(&arena, root, &mut tree);
        let mut surface = Surface {
            mass_center: [0.0; 3],
            rotation_inertia: [0.0; 3],
            upper_limit_radius: 0.0,
            max_factor_surface_deviation: 1,
            tree,
            ledges,
            split_guards: guards,
        };
        surface.mass_props();
        Some(surface)
    }

    /// Mass centre, unit inertia, radius and deviation (`mass_props`, 0x10039b30, 0x1003bbf0, 0x10020650).
    fn mass_props(&mut self) {
        let leaves = leaf_ledges(&self.tree);
        let ledges: Vec<&Ledge> = leaves.iter().map(|&i| &self.ledges[i]).collect();

        // the signed volume and the volume-weighted centroid, in the upstream's f32 steps
        let mut s2 = 0.0f64;
        let mut s6 = 0.0f64;
        let mut c = [0.0f32; 3];
        for l in &ledges {
            for t in &l.tris {
                let (p0, p1, p2) = (
                    l.points[t.start[0]],
                    l.points[t.start[1]],
                    l.points[t.start[2]],
                );
                let mut u = [0.0f32; 3];
                let mut v = [0.0f32; 3];
                for k in 0..3 {
                    u[k] = p1[k] - p0[k];
                    v[k] = p2[k] - p0[k];
                }
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                s2 += f64::from(n[0]) * f64::from(n[0])
                    + f64::from(n[1]) * f64::from(n[1])
                    + f64::from(n[2]) * f64::from(n[2]);
                let d = f64::from(n[0]) * f64::from(p0[0])
                    + f64::from(n[1]) * f64::from(p0[1])
                    + f64::from(n[2]) * f64::from(p0[2]);
                s6 += d;
                let f = (d * 0.25) as f32;
                for k in 0..3 {
                    c[k] += f * (p0[k] + p1[k] + p2[k]);
                }
            }
        }

        let mut mc = [0.0f64; 3];
        let mut inertia = [0.0f64; 3];
        let sq = s2.sqrt();
        if s6 <= sq * sq * sq * 1e-9 {
            // no volume (every concave soup: front and back cancel): the union box's centre
            let mut mn = [1e30f32; 3];
            let mut mx = [-1e30f32; 3];
            for l in &ledges {
                let (a, b) = ledge_aabb(l);
                for k in 0..3 {
                    if a[k] < mn[k] {
                        mn[k] = a[k];
                    }
                    if b[k] > mx[k] {
                        mx[k] = b[k];
                    }
                }
            }
            let mut d2 = 0.0f64;
            for k in 0..3 {
                mc[k] = 0.5 * f64::from(mn[k]) + 0.5 * f64::from(mx[k]);
                let e = f64::from(mx[k]) - f64::from(mn[k]);
                d2 += e * e;
            }
            let h = 0.5 * d2.sqrt();
            inertia = [h * h * 0.5; 3];
        } else {
            let inv = (1.0 / s6) as f32;
            for k in 0..3 {
                mc[k] = f64::from(c[k] * inv);
            }
            let m = second_moments(&ledges, mc);
            // the stored unit inertia is sqrt(m_y^2 + m_z^2), not m_y + m_z (the quirk the ported boxes need)
            inertia[0] = (m[1] * m[1] + m[2] * m[2]).sqrt();
            inertia[1] = (m[0] * m[0] + m[2] * m[2]).sqrt();
            inertia[2] = (m[0] * m[0] + m[1] * m[1]).sqrt();
        }
        for k in 0..3 {
            self.mass_center[k] = mc[k] as f32;
            self.rotation_inertia[k] = inertia[k] as f32;
        }

        // the radius about the mass centre, and the deviation of the faces from the plane through it
        let mut big_r = 0.0f64;
        let mut dev = 0.0f64;
        for l in &ledges {
            for t in &l.tris {
                let p = |e: usize| l.points[t.start[e]];
                let (p0, p1, p2) = (p(0), p(1), p(2));
                let u = [
                    f64::from(p1[0]) - f64::from(p0[0]),
                    f64::from(p1[1]) - f64::from(p0[1]),
                    f64::from(p1[2]) - f64::from(p0[2]),
                ];
                let v = [
                    f64::from(p2[0]) - f64::from(p0[0]),
                    f64::from(p2[1]) - f64::from(p0[1]),
                    f64::from(p2[2]) - f64::from(p0[2]),
                ];
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                let nn = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
                for e in 0..3 {
                    let q = p(e);
                    let d = [
                        f64::from(q[0]) - mc[0],
                        f64::from(q[1]) - mc[1],
                        f64::from(q[2]) - mc[2],
                    ];
                    let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    if r > big_r {
                        big_r = r;
                    }
                    let x = [
                        n[1] * d[2] - n[2] * d[1],
                        n[2] * d[0] - n[0] * d[2],
                        n[0] * d[1] - n[1] * d[0],
                    ];
                    if nn > 0.0 {
                        let a = ((x[0] * x[0] + x[1] * x[1] + x[2] * x[2]) / nn).sqrt();
                        if a > dev {
                            dev = a;
                        }
                    }
                }
            }
        }
        self.upper_limit_radius = big_r as f32;
        self.max_factor_surface_deviation = if big_r > 0.0 {
            ((dev / (big_r * 0.004) + 1.0) as i32) as u8
        } else {
            1
        };
    }

    /// The ledge of a surface with a single convex piece (`ivp_surface_single_convex`, 0x1000bcd0): the root is a leaf.
    pub fn single_convex(&self) -> Option<&Ledge> {
        match self.tree.first()?.kind {
            NodeKind::Leaf { ledge } => self.ledges.get(ledge),
            NodeKind::Inner { .. } => None,
        }
    }

    /// The bounding sphere of the surface about `c`: the radius, and the deviation (`0x1000bc40`).
    pub fn radius_dev(&self, c: [f32; 3]) -> (f32, f32) {
        let mut d2 = 0.0f64;
        for k in 0..3 {
            let e = f64::from(self.mass_center[k]) - f64::from(c[k]);
            d2 += e * e;
        }
        let d = d2.sqrt() as f32;
        let radius = self.upper_limit_radius + d;
        let dev =
            f32::from(self.max_factor_surface_deviation) * self.upper_limit_radius * 0.004f32 + d;
        (radius, dev)
    }

    /// The ledges whose boxes a sphere of radius `r` about `c` may touch, depth first (`0x1000b950` / `0x1000bb30`).
    pub fn ledges_within_radius(&self, c: V3, r: f64) -> Vec<usize> {
        let mut out = Vec::new();
        self.walk(0, c, r, &mut out);
        out
    }

    fn walk(&self, node: usize, c: V3, r: f64, out: &mut Vec<usize>) {
        let Some(n) = self.tree.get(node) else { return };
        let d = [
            f64::from(n.center[0]) - c[0],
            f64::from(n.center[1]) - c[1],
            f64::from(n.center[2]) - c[2],
        ];
        let big_r = f64::from(n.radius) + r;
        if big_r * big_r < d[0] * d[0] + d[1] * d[1] + d[2] * d[2] {
            return;
        }
        let s = (f64::from(n.radius) * 0.004) as f32;
        for k in 0..3 {
            if !(d[k].abs() < f64::from(n.box_sizes[k]) * f64::from(s) + r) {
                return;
            }
        }
        match n.kind {
            NodeKind::Leaf { ledge } => out.push(ledge),
            NodeKind::Inner { left, right } => {
                self.walk(left, c, r, out);
                self.walk(right, c, r, out);
            }
        }
    }
}

/// Writes the spheres under `s` into `tree` in preorder (`0x10039a70`), returning the index of the node written.
fn write_tree(arena: &[Sphere], s: usize, tree: &mut Vec<LedgeNode>) -> usize {
    let sp = &arena[s];
    let idx = tree.len();
    let center = [
        sp.center[0] as f32,
        sp.center[1] as f32,
        sp.center[2] as f32,
    ];
    let kind = match (sp.ledge, sp.children) {
        (Some(ledge), _) => NodeKind::Leaf { ledge },
        (None, Some((l, r))) => {
            tree.push(LedgeNode {
                center,
                radius: sp.radius as f32,
                box_sizes: sp.box_sizes,
                kind: NodeKind::Inner { left: 0, right: 0 },
            });
            let left = write_tree(arena, l, tree);
            let right = write_tree(arena, r, tree);
            tree[idx].kind = NodeKind::Inner { left, right };
            return idx;
        }
        (None, None) => NodeKind::Leaf { ledge: 0 },
    };
    tree.push(LedgeNode {
        center,
        radius: sp.radius as f32,
        box_sizes: sp.box_sizes,
        kind,
    });
    idx
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn unit_cube() -> Ledge {
        let mut p = Vec::new();
        for &x in &[0.0, 1.0] {
            for &y in &[0.0, 1.0] {
                for &z in &[0.0, 1.0] {
                    p.push([x, y, z]);
                }
            }
        }
        convex::hull(&p).expect("a cube has a hull")
    }

    #[test]
    fn triangle_ledge_has_two_closed_sides() {
        let l = triangle([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]).expect("a triangle");
        assert_eq!(l.tris.len(), 2);
        for (ti, t) in l.tris.iter().enumerate() {
            for k in 0..3 {
                let o = t.opposite[k];
                assert_ne!(o.tri, ti);
                assert_eq!(l.p(o), l.points[t.start[(k + 1) % 3]]);
                assert_eq!(l.opp(o), Edge { tri: ti, slot: k });
            }
        }
    }

    #[test]
    fn degenerate_triangle_has_no_ledge() {
        assert!(triangle([0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn cube_surface_has_its_centroid_and_unit_inertia() {
        let s = Surface::compile(vec![unit_cube()]).expect("a surface");
        for k in 0..3 {
            assert!(
                (s.mass_center[k] - 0.5).abs() < 1e-5,
                "centre {k}: {}",
                s.mass_center[k]
            );
        }
        // a unit cube's second moment about its centre is 1/12 on each axis; I_x = sqrt(1/12^2 + 1/12^2)
        let expected = (2.0f32).sqrt() / 12.0;
        for k in 0..3 {
            assert!(
                (s.rotation_inertia[k] - expected).abs() < 1e-4,
                "inertia {k}: {}",
                s.rotation_inertia[k]
            );
        }
        // the corners are sqrt(3)/2 from the centre
        assert!(
            (s.upper_limit_radius - 0.75f32.sqrt()).abs() < 1e-5,
            "radius {}",
            s.upper_limit_radius
        );
        assert!(s.single_convex().is_some());
    }

    #[test]
    fn walk_finds_the_leaf_around_a_point() {
        let a = unit_cube();
        let mut b = unit_cube();
        for p in &mut b.points {
            p[0] += 10.0;
        }
        let s = Surface::compile(vec![a, b]).expect("a surface");
        let near_first = s.ledges_within_radius([0.5, 0.5, 0.5], 0.1);
        assert!(near_first.contains(&0), "{near_first:?}");
        assert!(!near_first.contains(&1), "{near_first:?}");
        assert!(s.single_convex().is_none());
    }
}
