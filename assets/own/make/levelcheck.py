#!/usr/bin/env python3
"""The level builder's geometry checks (levelkit.LevelBuilder.write runs them; a level that violates one is not
written, and each finding is named with its coordinates):

  check_seams       no gaps, cracks or small steps at the floors' joints (the test of crates/game/src/ownlevels.rs,
                    `seams`, made stricter: a step under STEP_MAX between two pieces that should meet flush counts);
  check_scenery     no scenery (lamp towers, masts, plinths, garlands, wall blocks, columns, lanterns, signs) inside the
                    clearance volume of a floor the ball can roll on (the floor's top, its width, BALL_H + CLEAR_MARGIN
                    up), unless the description marks it `obstacle`;
  check_shortcuts   no floor of a later part of the route can be reached from an earlier one by falling, by a flight
                    off an edge at speed, or by the updraft of a fan (a conservative estimate: FLIGHT_V and gravity),
                    unless the description marks the landing as intended (`drop`);
  check_coplanar    no two drawn faces (of two objects, or of two materials of one) lie in one plane facing one way
                    and overlap: the depth buffer cannot tell them apart and the picture flickers between their
                    textures as the view moves (z-fighting). Faces back to back (a floor's end against the next
                    one's) are hidden inside the solids and not counted.

The checks read the builder's meshes, nodes and groups, and its route (`route`: floor node -> (from, to) distance along
the way, set by leveldsl.py) for the shortcuts. Needs numpy and scipy.
"""

import math
import os
import re

import numpy as np

# --- the numbers -------------------------------------------------------------------------------------------------
# Seams: a step between two pieces that meet, under STEP_MAX high (0.05: the 0.03 offsets the kit once used, not a gutter profile meeting a flat piece), is a seam (the Rust test: 0.02).
STEP_MAX = 0.05
FLUSH_TOL = 0.005
# Scenery: the volume over a floor's top the ball can be in: its height (two radii) and a margin, and a margin beside.
BALL_H = 4.0
CLEAR_MARGIN = 1.0
SIDE_MARGIN = 0.3
SCENERY = re.compile(r"^(Lamptower|Mast|Plinth|Wallblock|Lighthouse|Garland|Column|Lantern|Sign)_")
# Shortcuts. The balls' top speeds on a flat floor, measured in the engine (the arrow held 6000 ticks on a floor 40000
# wide): wood 15.9, stone 17.9, paper 13.9 units a second; off a downhill or blown, faster, so the estimate takes
# FLIGHT_V. Gravity 20 (physics.rs). A launching piece tilted up gives the flight an upward speed, RAMP_UP of the
# speed at most.
FLIGHT_V = 20.0
GRAVITY = 20.0
RAMP_UP = 0.35
# A fan carries the paper ball to about 17 over its floor (docs/drahtseilakt/levels.md).
FAN_LIFT = 17.0
# What lies this much (or more) of the way ahead of a floor is a skip if it can be reached; nearer is the route's own.
SKIP_MIN = 30.0
# A launch edge with a railing or a border (obstacle samples in both of two bands over the floor, this close) turns
# the ball back.
GUARD_R = 1.5
GUARD_BANDS = ((0.5, 1.5), (1.8, 3.0))
SAMPLE = 1.5
TARGET_SAMPLE = 2.0
SHOW = 12


def _tris_of(b, i):
    """Node i's triangles in world space (n, 3, 3) and the mean y of their vertex normals (up-facing > 0)."""
    node = b.nodes[i]
    if node.get("mesh") is None:
        return np.zeros((0, 3, 3)), np.zeros(0)
    m = b.meshes[node["mesh"]]
    if not m.p:
        return np.zeros((0, 3, 3)), np.zeros(0)
    M = node["matrix"]
    P = np.asarray(m.p, np.float64) @ M[:3, :3].T + M[:3, 3]
    N = np.asarray(m.n, np.float64) @ M[:3, :3].T
    tri, ups = [], []
    for idx in m.parts.values():
        t = np.reshape(idx, (-1, 3))
        n = N[t]
        n = n / np.maximum(np.linalg.norm(n, axis=2, keepdims=True), 1e-12)
        tri.append(P[t])
        ups.append(n[:, :, 1].mean(1))
    if not tri:
        return np.zeros((0, 3, 3)), np.zeros(0)
    return np.concatenate(tri), np.concatenate(ups)


def _sample_tris(tris, step):
    """Points on the triangles: corners, edges and inside, about `step` apart."""
    out = []
    for a, b, c in tris:
        n = max(1, int(math.ceil(max(np.linalg.norm(b - a), np.linalg.norm(c - a), np.linalg.norm(c - b)) / step)))
        uu, vv = np.meshgrid(np.arange(n + 1), np.arange(n + 1))
        keep = uu + vv <= n
        u, v = uu[keep] / n, vv[keep] / n
        out.append(a + np.outer(u, b - a) + np.outer(v, c - a))
    return np.concatenate(out) if out else np.zeros((0, 3))


def _floors(b, group="Phys_Floors", railings=True):
    ids = list(b.groups.get(group, []))
    if not railings:
        ids = [i for i in ids if "_Railing" not in b.nodes[i]["name"] and "_Fence" not in b.nodes[i]["name"]]
    return ids


def _fmt(p):
    return f"({p[0]:.1f}, {p[1]:.1f}, {p[2]:.1f})"


def _in_tri_xz(px, pz, t, grow):
    """Which of the points lie in the triangle t seen from above, grown by `grow` all round."""
    a, bb, c = t
    d = (bb[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (bb[2] - a[2])
    if abs(d) < 1e-12:
        return np.zeros(len(px), bool)
    u = ((px - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (pz - a[2])) / d
    v = ((bb[0] - a[0]) * (pz - a[2]) - (px - a[0]) * (bb[2] - a[2])) / d
    if grow <= 0:
        return (u >= -1e-9) & (v >= -1e-9) & (u + v <= 1 + 1e-9)
    la = math.hypot(bb[0] - a[0], bb[2] - a[2])
    lb = math.hypot(c[0] - a[0], c[2] - a[2])
    lc = math.hypot(c[0] - bb[0], c[2] - bb[2])
    area2 = abs(d)
    # distance to each edge's line, signed inwards: barycentric * 2 area / edge length (d may be negative: its sign
    # flips u and v and 1-u-v together, so compare with the sign)
    s = 1.0
    return (u * area2 / max(lb, 1e-9) >= -grow) & (v * area2 / max(la, 1e-9) >= -grow) \
        & ((1.0 - u - v) * area2 / max(lc, 1e-9) >= -grow) & (s > 0)


# --- 1. seams -------------------------------------------------------------------------------------------------------


def find_seams(b):
    """The floors' (Phys_Floors) free top edges with another top just beyond them that does not meet them flush: the
    other top lies 0.05 or 0.3 out at the edge's height (to STEP_MAX) while nothing at the edge's own height covers
    the edge itself. Returns [(point on the edge, height difference found)]."""
    tops = []
    for i in _floors(b):
        t, up = _tris_of(b, i)
        if len(t):
            ok = (up > 0.3) & (np.linalg.norm(np.cross(t[:, 1] - t[:, 0], t[:, 2] - t[:, 0]), axis=1) > 1e-6)
            tops.append(t[ok])
    if not tops:
        return []
    return seams_of_tops(np.concatenate(tops))


def seams_of_tops(T):
    """find_seams on the up-facing triangles T (n, 3, 3) of the floors."""
    key = lambda p: tuple(np.round(p * 1000.0).astype(np.int64))
    edges = {}
    for k, t in enumerate(T):
        for a, c in ((0, 1), (1, 2), (2, 0)):
            ka, kc = key(t[a]), key(t[c])
            e = (ka, kc) if ka < kc else (kc, ka)
            if e in edges:
                edges[e][0] += 1
            else:
                edges[e] = [1, k]
    CELL = 4.0
    grid = {}
    lo, hi = T.min(1), T.max(1)
    for k in range(len(T)):
        for x in range(int(math.floor(lo[k, 0] / CELL)), int(math.floor(hi[k, 0] / CELL)) + 1):
            for z in range(int(math.floor(lo[k, 2] / CELL)), int(math.floor(hi[k, 2] / CELL)) + 1):
                grid.setdefault((x, z), []).append(k)

    def heights(px, pz, skip):
        out = []
        for j in grid.get((int(math.floor(px / CELL)), int(math.floor(pz / CELL))), ()):
            if j == skip:
                continue
            a, bb, c = T[j]
            d = (bb[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (bb[2] - a[2])
            if abs(d) < 1e-9:
                continue
            u = ((px - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (pz - a[2])) / d
            v = ((bb[0] - a[0]) * (pz - a[2]) - (px - a[0]) * (bb[2] - a[2])) / d
            if u >= -1e-6 and v >= -1e-6 and u + v <= 1 + 1e-6:
                out.append(a[1] + u * (bb[1] - a[1]) + v * (c[1] - a[1]))
        return out

    found = []
    for (ka, kc), (count, k) in edges.items():
        if count != 1:
            continue
        a, c = np.array(ka) / 1000.0, np.array(kc) / 1000.0
        d = np.array([c[0] - a[0], 0.0, c[2] - a[2]])
        ln = np.linalg.norm(d)
        if ln < 1e-4:
            continue
        out = np.array([d[2], 0.0, -d[0]]) / ln
        if np.dot(T[k].mean(0) - (a + c) / 2, out) > 0:
            out = -out
        worst = None
        for s in (0.5, 0.2, 0.8):
            p = a + (c - a) * s
            at = p + out * 0.003
            near1 = heights(at[0], at[2], k)
            if near1:
                # a top meets the edge: flush, or a step (the height of that top at the edge itself, found from its
                # heights a little and a little more beyond it, so a slope is no step)
                far = p + out * 0.03
                far_h = heights(far[0], far[2], k)
                touching = []
                for h1 in near1:
                    h2 = min(far_h, key=lambda h: abs(h - h1), default=h1)
                    touching.append(abs(h1 - (h2 - h1) / 0.027 * 0.003 - p[1]))
                step = min(touching)
                if FLUSH_TOL < step < STEP_MAX:
                    worst = (p, step)
                    break
                continue
            for r in (0.05, 0.3):
                q = p + out * r
                near = [abs(h - p[1]) for h in heights(q[0], q[2], k) if abs(h - p[1]) < 0.02]
                if near:
                    worst = (p, min(near))
                    break
            if worst:
                break
        if worst:
            found.append(worst)
    return found


def check_seams(b):
    found = find_seams(b)
    if found:
        found.sort(key=lambda f: (round(f[0][0]), round(f[0][2])))
        shown = "; ".join(f"{_fmt(p)} step {s:.3f}" for p, s in found[:SHOW])
        raise ValueError(f"{len(found)} seams in the floors (an edge with another piece beside it that is not flush: a "
                         f"crack or a step of {FLUSH_TOL} to {STEP_MAX}), e.g. {shown}")


# --- 2. scenery in the ball's way ---------------------------------------------------------------------------------


def check_scenery(b):
    """No scenery sample point lies in a floor's clearance volume: over a top triangle (SIDE_MARGIN wider seen from
    above), from just under its top up BALL_H + CLEAR_MARGIN; for rails, 2 either side (the ball between the tubes)."""
    intended = getattr(b, "intended", set())
    nodes = [i for i, n in enumerate(b.nodes) if SCENERY.match(n["name"]) and n["name"] not in intended
             and n.get("mesh") is not None]
    hits = scenery_hits(b, nodes)
    if hits:
        rows = sorted(hits.items(), key=lambda kv: (b.nodes[kv[0][0]]["name"], b.nodes[kv[0][1]]["name"]))
        shown = "; ".join(f"{b.nodes[a]['name']} at {_fmt(p)} in the clearance over {b.nodes[f]['name']}"
                          for (a, f), p in rows[:SHOW])
        raise ValueError(f"{len(rows)} pieces of scenery in the ball's way (within {BALL_H + CLEAR_MARGIN} over a floor; "
                         f"an intended obstacle is marked `obstacle`): {shown}")


def scenery_hits(b, nodes, near=None):
    """check_scenery for the objects `nodes`: {(object, floor): a point of it in the floor's clearance}. `near`
    (centre, reach): only the floors' triangles within `reach` of `centre` seen from above (one object placed)."""
    pts, who = [], []
    for i in nodes:
        t, _ = _tris_of(b, i)
        if len(t):
            s = _sample_tris(t, 1.0)
            pts.append(s)
            who.extend([i] * len(s))
    if not pts:
        return {}
    S = np.concatenate(pts)
    who = np.array(who)
    CELL = 4.0
    cells = {}
    for k, p in enumerate(S):
        cells.setdefault((int(p[0] // CELL), int(p[2] // CELL)), []).append(k)
    hits = {}
    for group, grow, below in (("Phys_Floors", SIDE_MARGIN, 0.05), ("Phys_FloorRails", 2.0, 1.0)):
        for i in _floors(b, group, railings=False):
            t, up = _tris_of(b, i)
            tops = t[up > 0.3]
            if near is not None and len(tops):
                c, r = near
                lo, hi = tops.min(1), tops.max(1)
                tops = tops[(lo[:, 0] < c[0] + r) & (hi[:, 0] > c[0] - r) & (lo[:, 2] < c[2] + r) & (hi[:, 2] > c[2] - r)]
            for tri in tops:
                x0, z0 = tri[:, 0].min() - grow, tri[:, 2].min() - grow
                x1, z1 = tri[:, 0].max() + grow, tri[:, 2].max() + grow
                cand = []
                for cx in range(int(x0 // CELL), int(x1 // CELL) + 1):
                    for cz in range(int(z0 // CELL), int(z1 // CELL) + 1):
                        cand.extend(cells.get((cx, cz), ()))
                if not cand:
                    continue
                cand = np.array(cand)
                q = S[cand]
                m = (q[:, 1] > tri[:, 1].min() - below) & (q[:, 1] < tri[:, 1].max() + BALL_H + CLEAR_MARGIN)
                if not m.any():
                    continue
                cand, q = cand[m], q[m]
                inside = _in_tri_xz(q[:, 0], q[:, 2], tri, grow)
                cand, q = cand[inside], q[inside]
                if not len(cand):
                    continue
                # the top's height under each point (the plane of the triangle; its nearest edge when beside it)
                n = np.cross(tri[1] - tri[0], tri[2] - tri[0])
                if abs(n[1]) < 1e-9:
                    continue
                h = tri[0][1] - (n[0] * (q[:, 0] - tri[0][0]) + n[2] * (q[:, 2] - tri[0][2])) / n[1]
                ok = (q[:, 1] > h - below) & (q[:, 1] < h + BALL_H + CLEAR_MARGIN)
                for k in cand[ok]:
                    if who[k] != i:
                        hits.setdefault((int(who[k]), i), S[k])
    return hits


# --- 3. shortcuts ---------------------------------------------------------------------------------------------------


def reach(v, y0, y1, up_frac):
    """How far a ball leaving level at speed v (up_frac of it upward) has flown horizontally when it is at height y1,
    having left at height y0; None when it cannot be there."""
    vy = v * up_frac
    disc = vy * vy - 2 * GRAVITY * (y1 - y0)
    if disc < 0:
        return None
    return v * (vy + math.sqrt(disc)) / GRAVITY


def check_shortcuts(b):
    from scipy.spatial import cKDTree

    route = getattr(b, "route", {})
    drops = getattr(b, "drops", [])
    solid, tops = [], {}
    for g in ("Phys_Floors", "Phys_FloorRails"):
        for i in _floors(b, g):
            t, up = _tris_of(b, i)
            if not len(t):
                continue
            solid.append(_sample_tris(t, 1.0))
            if i in route and not b.nodes[i]["name"].startswith(("Lighthouse", "Wallblock")):
                tops[i] = t[up > 0.3]
    if not tops:
        return
    allsolid = np.vstack(solid)
    guard = cKDTree(allsolid[:, [0, 2]])
    launches, targets = [], []   # (point, node, upward fraction), (point, node)
    key = lambda p: tuple(np.round(p * 1000.0).astype(np.int64))
    for i, t in tops.items():
        if not len(t):
            continue
        seen = {}
        for k, tri in enumerate(t):
            for a, c in ((0, 1), (1, 2), (2, 0)):
                ka, kc = key(tri[a]), key(tri[c])
                seen.setdefault((ka, kc) if ka < kc else (kc, ka), []).append((k, a, c))
        for lst in seen.values():
            if len(lst) != 1:
                continue
            k, a, c = lst[0]
            p0, p1 = t[k][a], t[k][c]
            n = max(1, int(math.ceil(np.linalg.norm(p1 - p0) / SAMPLE)))
            nrm = np.cross(t[k][1] - t[k][0], t[k][2] - t[k][0])
            tilt = math.sqrt(max(0.0, 1 - (abs(nrm[1]) / max(np.linalg.norm(nrm), 1e-12)) ** 2))
            for j in range(n + 1):
                launches.append((p0 + (p1 - p0) * j / n, i, min(RAMP_UP, tilt), True))
        for p in _sample_tris(t, TARGET_SAMPLE):
            targets.append((p, i))
    # fans: a ball rises FAN_LIFT over the floor under the fan and flies on from there, nothing turning it back
    for n in b.nodes:
        if n["name"].startswith("P_Modul_18"):
            p = n["matrix"][:3, 3]
            under = [(abs(q[1] - p[1]), node) for q, node in targets
                     if abs(q[0] - p[0]) < 3 and abs(q[2] - p[2]) < 3 and q[1] < p[1] + 1.0]
            if under:
                launches.append((p + np.array([0.0, FAN_LIFT - 0.6, 0.0]), min(under)[1], 0.0, False))
    if not launches or not targets:
        return
    L = np.array([l[0] for l in launches])
    Lnode = np.array([l[1] for l in launches])
    Lfrac = np.array([l[2] for l in launches])
    Q = np.array([t[0] for t in targets])
    Qnode = np.array([t[1] for t in targets])
    s0 = np.array([route[n][0] for n in Qnode])
    s1 = np.array([route[n][1] for n in Lnode])
    gy = allsolid[:, 1]
    pairs = {}
    tree = cKDTree(Q[:, [0, 2]])
    rmax = FLIGHT_V * math.sqrt(2 * 400.0 / GRAVITY)
    for k, (p, node, frac, edge) in enumerate(launches):
        if edge:
            near = guard.query_ball_point(p[[0, 2]], GUARD_R)
            if near and all((((gy[near] - p[1]) > lo) & ((gy[near] - p[1]) < hi)).any() for lo, hi in GUARD_BANDS):
                continue
        for q in tree.query_ball_point(p[[0, 2]], rmax):
            n = Qnode[q]
            if n == node or s0[q] <= s1[k] + SKIP_MIN:
                continue
            d = float(np.hypot(Q[q][0] - p[0], Q[q][2] - p[2]))
            r = reach(FLIGHT_V, p[1], Q[q][1], frac)
            if r is None or d > r:
                continue
            if any(math.hypot(Q[q][0] - dp[0], Q[q][2] - dp[2]) < dr and abs(Q[q][1] - dp[1]) < 20.0
                   for dp, dr in drops):
                continue
            pr = (int(node), int(n))
            if pr not in pairs or r - d > pairs[pr][0]:
                pairs[pr] = (r - d, p.copy(), Q[q].copy(), d, r)
    if pairs:
        rows = sorted(pairs.items(), key=lambda kv: -kv[1][0])
        shown = "; ".join(
            f"{b.nodes[a]['name']} {_fmt(v[1])} -> {b.nodes[c]['name']} {_fmt(v[2])}: {v[3]:.0f} across, "
            f"{v[1][1] - v[2][1]:.0f} down, reach {v[4]:.0f} (route {route[a][1]:.0f} -> {route[c][0]:.0f})"
            for (a, c), v in rows[:SHOW])
        raise ValueError(f"{len(pairs)} pairs of floors the ball can fall or fly from one to a part of the route "
                         f"{SKIP_MIN:.0f} or more ahead (an intended drop is marked `drop`): {shown}")


# --- 4. pickups and transformers reachable -------------------------------------------------------------------------

# A ball's centre keeps this far (across) from a railing or a wall, over the floor between these heights.
REACH_CLEAR = 1.6
REACH_BAND = (0.3, 3.5)
REACH_STEP = 1.0
# A part counts as reached when a free point of floor lies this near it across (pickups are 2.5 across) and this close
# in height.
REACH_NEAR = 3.0
REACH_DY = 7.0
SEED = re.compile(r"^(Floor_\d+|Rails_\d+|Start_Platform)$")
TARGET = re.compile(r"^(P_Extra_Point|P_Extra_Life|P_Trafo_\w+?)_\d+$")


def check_reachable(b):
    """Every extra point, extra life and transformer can be reached by a ball over the floors from the route: a flood
    over points of floor (a step of up to 2 up or down, 1.6 between points) from the route's own floors, where a point
    of floor within REACH_CLEAR of a railing, a fence or a wall (over it between REACH_BAND) is not free."""
    from scipy.spatial import cKDTree
    from scipy.sparse import coo_matrix
    from scipy.sparse.csgraph import connected_components

    pts, owner, block = [], [], []
    for g in ("Phys_Floors", "Phys_FloorRails"):
        for i in _floors(b, g):
            name = b.nodes[i]["name"]
            t, up = _tris_of(b, i)
            if not len(t):
                continue
            if "_Railing" in name or "_Fence" in name or name.startswith("Wallblock"):
                block.append(_sample_tris(t, 0.7))
                continue
            if name.startswith("Lighthouse"):
                continue
            s = _sample_tris(t[up > 0.3], REACH_STEP) if (up > 0.3).any() else np.zeros((0, 3))
            pts.append(s)
            owner += [i] * len(s)
    if not pts:
        return
    P = np.vstack(pts)
    owner = np.array(owner)
    free = np.ones(len(P), bool)
    if block:
        B = np.vstack(block)
        tb = cKDTree(B[:, [0, 2]])
        for c0 in range(0, len(P), 4000):
            for k, near in enumerate(tb.query_ball_point(P[c0:c0 + 4000, [0, 2]], REACH_CLEAR), c0):
                if near:
                    dy = B[near, 1] - P[k, 1]
                    if ((dy > REACH_BAND[0]) & (dy < REACH_BAND[1])).any():
                        free[k] = False
    idx = np.nonzero(free)[0]
    F = P[idx]
    tree = cKDTree(F * np.array([1.0, 0.6, 1.0]))
    pairs = tree.query_pairs(1.7, output_type="ndarray")
    pairs = pairs[np.abs(F[pairs[:, 0], 1] - F[pairs[:, 1], 1]) <= 2.0]
    n = len(F)
    g = coo_matrix((np.ones(len(pairs)), (pairs[:, 0], pairs[:, 1])), shape=(n, n))
    _, lab = connected_components(g, directed=False)
    seeds = {lab[k] for k in range(n) if SEED.match(b.nodes[owner[idx[k]]]["name"])}
    ok = np.isin(lab, list(seeds))
    R = F[ok]
    if not len(R):
        return
    rt = cKDTree(R[:, [0, 2]])
    bad = []
    for node in b.nodes:
        m = TARGET.match(node["name"])
        if not m:
            continue
        c = node["matrix"][:3, 3]
        reached = any(abs(R[j, 1] - c[1]) < REACH_DY for j in rt.query_ball_point(c[[0, 2]], REACH_NEAR))
        if not reached:
            bad.append(f"{node['name']} at {_fmt(c)}")
    if bad:
        raise ValueError(f"{len(bad)} parts the ball cannot reach from the route (a railing, a wall or a gap in the way "
                         f"in), e.g. " + "; ".join(bad[:SHOW]))


# --- 5. faces in one plane -----------------------------------------------------------------------------------------
# Two faces count as one plane when their normals agree to COPLANAR_COS and their planes are COPLANAR_D apart at most;
# an overlap of COPLANAR_AREA or more (square units) is reported.
COPLANAR_COS, COPLANAR_D, COPLANAR_AREA = 0.9995, 0.004, 0.02


def _clip_left(poly, a, b):
    """The 2D polygon clipped to the left of the line a -> b."""
    out = []
    for i in range(len(poly)):
        p, q = poly[i], poly[(i + 1) % len(poly)]
        sp = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        sq = (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
        if sp >= 0:
            out.append(p)
        if (sp >= 0) != (sq >= 0):
            t = sp / (sp - sq)
            out.append((p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])))
    return out


def _area2d(poly):
    return sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1]
               for i in range(len(poly))) / 2


def _overlap2d(t1, t2):
    """The area two 2D triangles share."""
    p = [tuple(v) for v in t1]
    q = [tuple(v) for v in t2]
    if _area2d(p) < 0:
        p = p[::-1]
    if _area2d(q) < 0:
        q = q[::-1]
    for i in range(3):
        p = _clip_left(p, q[i], q[(i + 1) % 3])
        if len(p) < 3:
            return 0.0
    return abs(_area2d(p))


def coplanar_overlaps(items):
    """`items`: [(label, triangles (n, 3, 3), their facing normals (n, 3))]. Returns [(label, label, area, point)]
    for each pair of faces of two items in one plane, facing one way, overlapping."""
    if not items:
        return []
    T = np.concatenate([t for _, t, _ in items])
    F = np.concatenate([f for _, _, f in items])
    owner = np.concatenate([np.full(len(t), k) for k, (_, t, _) in enumerate(items)])
    g = np.cross(T[:, 1] - T[:, 0], T[:, 2] - T[:, 0])
    a2 = np.linalg.norm(g, axis=1)
    ok = a2 > 1e-6
    g[ok] /= a2[ok, None]
    # facing: the geometric normal turned to the side the face is drawn for
    g[np.einsum("ij,ij->i", g, F) < 0] *= -1.0
    d = np.einsum("ij,ij->i", g, T[:, 0])
    bins = {}
    for i in np.nonzero(ok)[0]:
        bins.setdefault((tuple(np.round(g[i] * 100).astype(int)), int(math.floor(d[i] / 0.05))), []).append(i)
    found, seen = [], set()
    for (kn, kd), ids in bins.items():
        ids = ids + bins.get((kn, kd + 1), [])
        if len({owner[i] for i in ids}) < 2:
            continue
        n = g[ids[0]]
        u = np.cross(n, (0.0, 1.0, 0.0) if abs(n[1]) < 0.9 else (1.0, 0.0, 0.0))
        u /= np.linalg.norm(u)
        v = np.cross(n, u)
        flat = {i: np.c_[T[i] @ u, T[i] @ v] for i in ids}
        cells = {}
        for i in ids:
            lo, hi = flat[i].min(0) // 4, flat[i].max(0) // 4
            for cx in range(int(lo[0]), int(hi[0]) + 1):
                for cy in range(int(lo[1]), int(hi[1]) + 1):
                    cells.setdefault((cx, cy), []).append(i)
        for cell in cells.values():
            for x, i in enumerate(cell):
                for j in cell[x + 1:]:
                    if owner[i] == owner[j] or (i, j) in seen or (j, i) in seen:
                        continue
                    if abs(d[i] - d[j]) > COPLANAR_D or g[i] @ g[j] < COPLANAR_COS:
                        continue
                    seen.add((i, j))
                    area = _overlap2d(flat[i], flat[j])
                    if area >= COPLANAR_AREA:
                        found.append((items[owner[i]][0], items[owner[j]][0], area, T[i].mean(0)))
                        if os.environ.get("LEVELCHECK_DEBUG"):
                            print("coplanar", items[owner[i]][0], T[i].round(2).tolist(), items[owner[j]][0], T[j].round(2).tolist())
    return found


def check_coplanar(b):
    """No two drawn faces of the level overlap in one plane facing one way (see the top)."""
    names = {v: k for k, v in getattr(b, "mat_ids", {}).items()}
    items = []
    for i, node in enumerate(b.nodes):
        if node.get("mesh") is None or not node.get("visible", True):
            continue
        m = b.meshes[node["mesh"]]
        if not m.p:
            continue
        M = node["matrix"]
        P = np.asarray(m.p, np.float64) @ M[:3, :3].T + M[:3, 3]
        N = np.asarray(m.n, np.float64) @ M[:3, :3].T if getattr(m, "n", None) else None
        for mat, idx in m.parts.items():
            if mat is None or not idx:
                continue
            t = np.reshape(idx, (-1, 3))
            tri = P[t]
            if N is not None:
                face = N[t].sum(1)
            else:
                face = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
            items.append((f"{node['name']}[{names.get(mat, mat)}]", tri, face))
    found = coplanar_overlaps(items)
    if found:
        per = {}
        for a, c, area, p in found:
            k = (a, c)
            per[k] = (per.get(k, (0.0, p))[0] + area, p)
        shown = "; ".join(f"{a} and {c} at {_fmt(p)}, {area:.2f}" for (a, c), (area, p) in
                          sorted(per.items(), key=lambda kv: -kv[1][0])[:SHOW])
        raise ValueError(f"{len(per)} pairs of faces overlap in one plane (they flicker: z-fighting), e.g. {shown}")


def check_all(b):
    """Runs the checks; returns the messages of those that fail (empty: all pass)."""
    msgs = []
    for fn in (check_seams, check_scenery, check_shortcuts, check_reachable, check_coplanar):
        try:
            fn(b)
        except ValueError as e:
            msgs.append(str(e))
    return msgs
