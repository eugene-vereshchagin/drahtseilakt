#!/usr/bin/env python3
"""Pieces for levelkit's LevelBuilder that the level description (leveldsl.py) puts beside the track: plazas and
platforms with a brick rim and railings (gaps and all), round pedestals, a pit in a floor with a stop wall, the free
standing scenery (lamp towers, striped masts, garlands, the lighthouse, wall blocks) and a scatter of distant scenery
along the way. Everything here is made of the kit's own materials and primitives; only the plazas, pedestals, pits,
wall blocks and the lighthouse's shaft are solid to the ball (Phys_Floors), the rest is only drawn.

Needs Python 3 with numpy; levelkit.py and ownkit.py beside it.
"""

import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import levelkit as L  # noqa: E402
import ownkit as K  # noqa: E402

UP = np.array([0.0, 1.0, 0.0])
# A plaza's, a pedestal's and a pit's tops lie exactly at the height of the floors that run on from them (the kit once
# put them 0.03 under to dodge the seam test; levelcheck.py now refuses such steps and cracks).
FLUSH = 0.0
SIDES = ("back", "front", "left", "right")
# a brick rim (1 wide, 2.2 high: the ball is turned back by its face) and a low kerb (0.8 wide, 0.7 high, as a gutter's)
RIMS = {"brick": (L.BORDER_W, L.BORDER_H, "border"), "kerb": (0.8, 0.7, "kerb")}


def materials(b):
    """The scenery's materials, by the game's own textures."""
    g = K.GAME
    b.mat("stripes", g + "Stick_Stripes", diffuse=1.0, emissive=0.35, specular=0.2, power=40)
    b.mat("roof", g + "Tower_Roof", diffuse=1.0, emissive=0.3)
    b.mat("beige", g + "Column_beige", diffuse=1.0, emissive=0.32)
    b.mat("panel", g + "Wood_Panel", diffuse=1.0, emissive=0.28, specular=0.2, power=40)
    b.mat("brick", g + "Brick", diffuse=1.0, emissive=0.3)


def solid(m, top_mat, side_mat, centre, axes, half, scale=L.SLAB):
    """An oriented box from its centre, its axes (forward, up, left) and half sizes; its top face (+up) of top_mat, the
    others of side_mat; texture coordinates along the box's own axes."""
    c = np.asarray(centre, np.float64)
    ax = [np.asarray(a, np.float64) for a in axes]
    for i in range(3):
        j, k = [(1, 2), (0, 2), (0, 1)][i]
        for sgn in (-1, 1):
            pts, uvs = [], []
            for sa, sb in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                co = [0, 0, 0]
                co[i], co[j], co[k] = sgn, sa, sb
                p = c + sum(ax[q] * co[q] * half[q] for q in range(3))
                pts.append(p)
                uvs.append((float(np.dot(p, ax[j])) / scale, float(np.dot(p, ax[k])) / scale))
            n = ax[i] * sgn
            m.face(top_mat if (i == 1 and sgn == 1) else side_mat, pts, uvs, normal=n)


def frame_of(fwd):
    f = np.asarray(fwd, np.float64).copy()
    f[1] = 0.0
    f /= np.linalg.norm(f) + 1e-12
    return f, np.array([-f[2], 0.0, f[0]])


def _intervals(length, cuts):
    """The parts of 0..length left when the cuts (centre, width) are taken out."""
    out, at = [], 0.0
    for c, w in sorted(cuts):
        lo, hi = c - w / 2, c + w / 2
        if lo > at + 1e-6:
            out.append((at, min(lo, length)))
        at = max(at, hi)
    if at < length - 1e-6:
        out.append((at, length))
    return [(a, b) for a, b in out if b - a > 1e-6]


def rail_line(b, name, pts, post_every=L.POST_EVERY):
    """A railing along a polyline of base points (on a floor's top): two rails and posts, solid to the ball."""
    m = K.Mesh(name)
    P = [np.asarray(p, np.float64) for p in pts]
    for h in L.RAIL_HEIGHTS:
        K.tube(m, b.mat_ids["rail"], [p + UP * h for p in P], L.RAIL_R, caps=True)
    seg = [float(np.linalg.norm(q - p)) for p, q in zip(P, P[1:])]
    total = sum(seg)
    count = max(1, int(round(total / post_every)))
    for j in range(count + 1):
        t = total * j / count
        q = 0
        while q < len(seg) - 1 and t > seg[q]:
            t -= seg[q]
            q += 1
        c = t / seg[q] if seg[q] > 0 else 0.0
        p = P[q] * (1 - c) + P[q + 1] * c
        K.tube(m, b.mat_ids["rail"], [p, p + UP * (L.RAIL_HEIGHTS[-1] + L.RAIL_R)], L.RAIL_R * 0.8, caps=True)
    return b.add(name, m, groups=b._floor_groups("metal"))


def plaza(b, name, at, fwd, width, length, rim, gaps, fence, sector, surface="stone", thick=L.DEPTH):
    """A flat floor `width` across and `length` along `fwd` from the middle of its back edge `at` (its top at at's
    height), with a rim (brick or kerb, or None) round it except at `gaps` {side: [(centre, width)]} (back and front:
    the centre across, to the left; left and right: along from the back edge), and a railing along the sides in
    `fence` (also stopping at the gaps)."""
    materials(b)
    f, lft = frame_of(fwd)
    at = np.asarray(at, np.float64) - UP * FLUSH
    mid = at + f * length / 2
    axes = (f, UP, lft)
    m = K.Mesh(name)
    top = "wood" if surface == "wood" else "top"
    solid(m, b.mat_ids[top], b.mat_ids["side"], mid - UP * thick / 2, axes, (length / 2, thick / 2, width / 2))
    edge = {"back": (at, lft, width, -f), "front": (at + f * length, lft, width, f),
            "left": (at + lft * width / 2, f, length, lft), "right": (at - lft * width / 2, f, length, -lft)}
    # a gap on the back or front edge is given across from the edge's middle (to the left); the edge is measured from
    # its right end
    gaps = {k: ([(c + width / 2, w) for c, w in v] if k in ("back", "front") else v) for k, v in gaps.items()}
    if rim:
        rw, rh, rmat = RIMS[rim]
        for side in SIDES:
            o, along, span, out = edge[side]
            origin = o - along * span / 2 if side in ("back", "front") else o
            lo_trim = rw if side in ("left", "right") else 0.0
            for a, c in _intervals(span, gaps.get(side, [])):
                if side in ("left", "right"):
                    a, c = max(a, rw), min(c, span - rw)
                    if c - a < 1e-6:
                        continue
                p0, p1 = origin + along * a, origin + along * c
                # the side rims a hair lower than the back and front ones, where they meet at the corners
                h = rh - FLUSH if side in ("left", "right") else rh
                centre = (p0 + p1) / 2 - out * rw / 2 + UP * h / 2
                solid(m, b.mat_ids[rmat], b.mat_ids[rmat], centre, (along, UP, out), ((c - a) / 2, h / 2, rw / 2),
                      scale=L.BRICK)
    groups = b._floor_groups("wood" if surface == "wood" else "stone")
    node = b.add(name, m, groups=groups)
    for side in fence:
        o, along, span, out = edge[side]
        origin = o - along * span / 2 if side in ("back", "front") else o
        for k, (a, c) in enumerate(_intervals(span, gaps.get(side, []))):
            base = [origin + along * t - out * L.RAIL_IN + UP * 0.0 for t in np.linspace(a, c, max(2, int((c - a) / 4) + 1))]
            rail_line(b, f"{name}_Fence_{side}{k + 1}", base)
    return node


def disc(b, name, centre, r, sector, thick=L.DEPTH, surface="stone", rim=False):
    """A round floor of radius r, its top at the centre's height."""
    materials(b)
    x, y, z = centre
    y -= FLUSH
    m = K.Mesh(name)
    top = b.mat_ids["wood" if surface == "wood" else "top"]
    K.cylinder(m, b.mat_ids["side"], (x, y - thick, z), UP, r, thick, seg=40, cap_mat=top, vscale=L.SLAB)
    if rim:
        K.cylinder(m, b.mat_ids["border"], (x, y - 0.6, z), UP, r + 0.25, 1.0, seg=40, vscale=L.BRICK)
    return b.add(name, m, groups=b._floor_groups(surface))


def pit(b, name, at, fwd, width, length, depth, wall_len, wall_h, wall_side, sector):
    """A pit in a way: the way's floors stop at `at` and begin `length` on (the description lays them); between, a
    flat bottom `depth` under the way and the pit's walls under the floors' ends, and, on the `wall_side` (+1 left,
    -1 right) kerb, beyond the pit's edge, a stop wall `wall_len` long and `wall_h` high along the way, centred on the
    pit: a ball coming across fast hits it and falls in all the same."""
    materials(b)
    f, lft = frame_of(fwd)
    at = np.asarray(at, np.float64) - UP * FLUSH
    m = K.Mesh(name)
    axes = (f, UP, lft)
    mid = at + f * length / 2
    solid(m, b.mat_ids["top"], b.mat_ids["side"], mid - UP * (depth + 0.5), axes, (length / 2, 0.5, width / 2))
    # the floors' ends fall to the bottom: a wall under each end, as wide as the way, below the floors' own 2.5
    wall_top = -L.DEPTH
    h = (depth - L.DEPTH) / 2 + 0.5
    for end in (0.0, length):
        c = at + f * (end + (-0.3 if end == 0.0 else 0.3)) + UP * (wall_top - h)
        solid(m, b.mat_ids["side"], b.mat_ids["side"], c, axes, (0.3, h + 0.0, width / 2))
    # the far side closed by a wall as high as the way, so a ball in the pit stays in
    solid(m, b.mat_ids["border"], b.mat_ids["border"], mid + lft * wall_side * (width / 2 + 0.5) - UP * (depth - 0.0) / 2 + UP * 0.0,
          axes, (length / 2 + 0.6, depth / 2 + 0.0, 0.5), scale=L.BRICK)
    if wall_len > 0:
        solid(m, b.mat_ids["border"], b.mat_ids["border"], mid + lft * wall_side * (width / 2 + 0.5) + UP * wall_h / 2,
              axes, (wall_len / 2, wall_h / 2, 0.5), scale=L.BRICK)
    return b.add(name, m, groups=b._floor_groups("stone"))


# --- scenery ---------------------------------------------------------------------------------------------------------


def _shaft(m, b, x, y_top, z, r, depth, mat="post", seg=14):
    K.cylinder(m, b.mat_ids[mat], (x, y_top - depth, z), UP, r, depth, seg=seg, vscale=L.SLAB)


def lamp_tower(b, name, base, r, height, depth=L.COLUMN_DEPTH):
    """A tower of the lamp kind: a stone shaft (radius r) from deep in the haze up to `height` over `base` (a point at
    the floors' level), a brick cornice and a gallery, a glowing lantern of glass with a roof and a finial."""
    materials(b)
    x, y, z = base
    top = y + height
    m = K.Mesh(name)
    _shaft(m, b, x, top, z, r, depth + height)
    for dy in (0.0, -height * 0.45):
        K.cylinder(m, b.mat_ids["border"], (x, top - 1.6 + dy, z), UP, r * 1.28, 1.6, seg=14, vscale=L.BRICK)
    K.cylinder(m, b.mat_ids["border"], (x, top, z), UP, r * 1.9, 0.7, seg=18, vscale=L.BRICK)
    # a railing ring on the gallery
    for k in range(10):
        a = 2 * math.pi * k / 10
        p = (x + math.cos(a) * r * 1.8, top + 0.7, z + math.sin(a) * r * 1.8)
        K.cylinder(m, b.mat_ids["rail"], p, UP, 0.12, 1.4, seg=6)
    K.torus(m, b.mat_ids["rail"], (x, top + 2.1, z), UP, r * 1.8, 0.12, seg=20, tube_seg=6)
    lr = r * 1.15
    K.cylinder(m, b.mat_ids["lamp"], (x, top + 0.7, z), UP, lr, r * 1.7, seg=12, cap_mat=b.mat_ids["rail"])
    K.cylinder(m, b.mat_ids["roof"], (x, top + 0.7 + r * 1.7, z), UP, lr * 1.45, r * 1.1, seg=12, r_top=0.1)
    K.cylinder(m, b.mat_ids["rail"], (x, top + 0.7 + r * 2.8, z), UP, 0.1, r * 0.9, seg=6)
    return b.add(name, m)


def mast(b, name, base, height, r=0.55, depth=L.COLUMN_DEPTH, flag=True):
    """A striped mast from deep in the haze to `height` over `base`, with a pennant and a small lamp on its top."""
    materials(b)
    x, y, z = base
    top = y + height
    m = K.Mesh(name)
    K.cylinder(m, b.mat_ids["stripes"], (x, top - depth - height, z), UP, r, depth + height, seg=10, vscale=5.0)
    K.cylinder(m, b.mat_ids["rail"], (x, top - 0.5, z), UP, r * 1.4, 0.6, seg=10)
    K.cylinder(m, b.mat_ids["lamp"], (x, top + 0.1, z), UP, r * 1.2, r * 2.2, seg=8, cap_mat=b.mat_ids["rail"])
    K.sphere(m, b.mat_ids["lamp"], (x, top + r * 2.6, z), r * 0.9, seg=8)
    if flag:
        a = (sum(map(ord, name)) % 8) * math.pi / 4
        d = np.array([math.cos(a), 0.0, math.sin(a)])
        p0 = np.array([x, top - 1.2, z])
        pts = [p0 + d * 0.1, p0 + d * 4.2 - UP * 0.3, p0 + d * 0.1 - UP * 2.2]
        m.face(b.mat_ids["lamp"], pts, normal=np.cross(d, UP))
        m.face(b.mat_ids["lamp"], pts[::-1], normal=-np.cross(d, UP))
    return b.add(name, m)


def garland(b, name, a, c, sag, lamps=True, wire=0.09, every=5.0, lamp_r=0.4):
    """A wire from a to c hanging `sag` between them (a parabola), with small glowing lamps along it."""
    materials(b)
    a, c = np.asarray(a, np.float64), np.asarray(c, np.float64)
    n = max(4, int(np.linalg.norm(c - a) / 3.0))
    pts = [a + (c - a) * t - UP * sag * 4 * t * (1 - t) for t in np.linspace(0, 1, n + 1)]
    m = K.Mesh(name)
    K.tube(m, b.mat_ids["rail"], pts, wire, seg=5, caps=True)
    if lamps:
        k = max(1, int(np.linalg.norm(c - a) / every))
        for j in range(1, k):
            t = j / k
            p = a + (c - a) * t - UP * (sag * 4 * t * (1 - t) + lamp_r * 1.2)
            K.sphere(m, b.mat_ids["lamp"], tuple(p), lamp_r, seg=8)
    return b.add(name, m)


def wall_block(b, name, at, fwd, length, height, thick, sector, depth=L.DEPTH + 2.0):
    """A blank wall of stone beside a ledge: its foot on the floor's level `at` (the middle of its near face's foot),
    `length` along `fwd`, `height` high, `thick` deep away to the left; solid to the ball. A cornice runs along its
    top."""
    materials(b)
    f, lft = frame_of(fwd)
    at = np.asarray(at, np.float64)
    m = K.Mesh(name)
    c = at + lft * thick / 2 + f * length / 2 + UP * (height - depth) / 2
    solid(m, b.mat_ids["beige"], b.mat_ids["beige"], c, (f, UP, lft), (length / 2, (height + depth) / 2, thick / 2),
          scale=L.SLAB)
    solid(m, b.mat_ids["border"], b.mat_ids["border"], c + UP * ((height + depth) / 2 - 0.4), (f, UP, lft),
          (length / 2 + 0.3, 0.5, thick / 2 + 0.3), scale=L.BRICK)
    return b.add(name, m, groups=b._floor_groups("stone"))


def lighthouse(b, name, base, half, height, sector, depth=L.COLUMN_DEPTH):
    """The Lighthouse: a square tower (`half` the half side) from the haze to `height` over `base` (the quay's floor
    level), banded with brick, a gallery with a railing, a glass lantern with a roof and a finial. Its shaft is solid
    to the ball (the way winds round it)."""
    materials(b)
    x, y, z = base
    top = y + height
    f, lft = np.array([1.0, 0.0, 0.0]), np.array([0.0, 0.0, 1.0])
    axes = (f, UP, lft)
    body = K.Mesh(name)
    # the shaft is solid from just under the quay up; below it the haze hides a plain drawn one (the clouds lie under
    # the lowest floor, not under this)
    bottom = y - 12.0
    solid(body, b.mat_ids["beige"], b.mat_ids["beige"], (x, (top + bottom) / 2, z), axes,
          (half, (top - bottom) / 2, half), scale=L.SLAB)
    node = b.add(name, body, groups=b._floor_groups("stone"))
    deep = K.Mesh(name + "_Deep")
    lo = y - depth
    solid(deep, b.mat_ids["beige"], b.mat_ids["beige"], (x, (lo + bottom) / 2, z), axes,
          (half, (bottom - lo) / 2, half), scale=L.SLAB)
    b.add(name + "_Deep", deep)
    m = K.Mesh(name + "_Trim")
    band = 0.0
    while y + band < top - 4.0:
        solid(m, b.mat_ids["border"], b.mat_ids["border"], (x, y + band - 0.6, z), axes, (half + 0.5, 0.6, half + 0.5),
              scale=L.BRICK)
        band += 11.0
    solid(m, b.mat_ids["border"], b.mat_ids["border"], (x, top - 0.6, z), axes, (half + 2.4, 0.6, half + 2.4),
          scale=L.BRICK)
    solid(m, b.mat_ids["side"], b.mat_ids["side"], (x, top - 1.8, z), axes, (half + 1.4, 0.6, half + 1.4))
    g = half + 2.0
    for k in range(-4, 5):
        for sx, sz in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            p = (x + sx * g + sz * k * g / 4.0, top, z + sz * g + sx * k * g / 4.0)
            K.cylinder(m, b.mat_ids["rail"], p, UP, 0.14, 2.6, seg=6)
    for sgn in (-1, 1):
        for h in (1.3, 2.7):
            K.tube(m, b.mat_ids["rail"], [(x - g, top + h, z + sgn * g), (x + g, top + h, z + sgn * g)], 0.16)
            K.tube(m, b.mat_ids["rail"], [(x + sgn * g, top + h, z - g), (x + sgn * g, top + h, z + g)], 0.16)
    lr = half * 0.55
    K.cylinder(m, b.mat_ids["lamp"], (x, top, z), UP, lr, lr * 1.5, seg=16, cap_mat=b.mat_ids["rail"])
    for k in range(8):
        a = 2 * math.pi * k / 8
        K.cylinder(m, b.mat_ids["rail"], (x + math.cos(a) * lr, top, z + math.sin(a) * lr), UP, 0.2, lr * 1.5, seg=6)
    K.cylinder(m, b.mat_ids["roof"], (x, top + lr * 1.5, z), UP, lr * 1.5, lr * 1.0, seg=16, r_top=0.15)
    K.cylinder(m, b.mat_ids["rail"], (x, top + lr * 2.5, z), UP, 0.15, lr * 0.8, seg=6)
    K.sphere(m, b.mat_ids["lamp"], (x, top + lr * 3.3, z), lr * 0.22, seg=8)
    b.add(name + "_Trim", m)
    return node


def scatter(b, rng, path, reach, sectors_heights, every, avoid, keep_off=None):
    """Distant scenery along the way: about one object every `every` units of `path` (its points), left or right of
    it 24 to 75 away and up or down from its level, kept clear of every point of the floors (`avoid`: points and their
    reach), of each other, and of the `keep_off` boxes. Towers, masts and plinths, never two of a kind in a row."""
    materials(b)
    P = np.asarray(path, np.float64)
    A = np.asarray([p for p, _ in avoid], np.float64)
    R = np.asarray([r for _, r in avoid], np.float64)
    placed, kinds, run, made = [], [], 0.0, 0
    kinds_cycle = ("tower", "mast", "plinth", "tower", "mast", "plinth", "mast", "tower")
    for i in range(1, len(P)):
        run += float(np.linalg.norm(P[i] - P[i - 1]))
        if run < every:
            continue
        run = 0.0
        d = P[i] - P[i - 1]
        d[1] = 0.0
        d /= np.linalg.norm(d) + 1e-9
        lft = np.array([-d[2], 0.0, d[0]])
        for attempt in range(8):
            side = 1.0 if rng.random() < 0.5 else -1.0
            off = lft * side * rng.uniform(26.0, 75.0) + d * rng.uniform(-14.0, 14.0)
            c = P[i] + off
            kind = kinds_cycle[(made + attempt) % len(kinds_cycle)]
            r = {"tower": rng.uniform(2.2, 3.4), "mast": 0.6, "plinth": rng.uniform(4.0, 6.5)}[kind]
            flat = np.hypot(A[:, 0] - c[0], A[:, 2] - c[2])
            if np.any(flat < R + r + 9.0):
                continue
            if any(np.hypot(c[0] - q[0], c[2] - q[1]) < 22.0 for q in placed):
                continue
            y = P[i][1] + rng.uniform(-20.0, 8.0)
            placed.append((c[0], c[2]))
            made += 1
            n = f"Scenery_{made:03d}"
            if kind == "tower":
                lamp_tower(b, n, (c[0], y, c[2]), r, rng.uniform(9.0, 30.0))
            elif kind == "mast":
                mast(b, n, (c[0], y, c[2]), rng.uniform(16.0, 34.0))
            else:
                plinth(b, n, (c[0], y, c[2]), r)
            break
    return made


def plinth(b, name, top, r, depth=L.COLUMN_DEPTH):
    """A stone plinth: a thick shaft up to `top` with a wide brick cap and a lantern on a short post."""
    materials(b)
    x, y, z = top
    m = K.Mesh(name)
    _shaft(m, b, x, y, z, r, depth, mat="beige", seg=16)
    K.cylinder(m, b.mat_ids["border"], (x, y - 1.8, z), UP, r * 1.3, 1.8, seg=16, vscale=L.BRICK)
    K.cylinder(m, b.mat_ids["top"], (x, y, z), UP, r * 1.3, 0.5, seg=16)
    K.cylinder(m, b.mat_ids["wood"], (x, y + 0.5, z), UP, 0.25, 5.0, seg=6)
    K.cylinder(m, b.mat_ids["lamp"], (x, y + 5.0, z), UP, 0.7, 1.3, seg=8, cap_mat=b.mat_ids["rail"])
    K.cylinder(m, b.mat_ids["roof"], (x, y + 6.3, z), UP, 1.0, 0.9, seg=8, r_top=0.05)
    return b.add(name, m)


def rail_ring(b, name, centre, r, gaps, top=0.0):
    """A railing round a circle of radius r about `centre` (at the floor's top), open at `gaps`: (angle in degrees from
    +x, width in units of arc)."""
    x, y, z = centre
    cuts = [(math.radians(a), w / r) for a, w in gaps]
    n = 72
    arcs, run = [], []
    for k in range(n + 1):
        a = 2 * math.pi * k / n
        inside = any(abs((a - c + math.pi) % (2 * math.pi) - math.pi) < w / 2 for c, w in cuts)
        if inside:
            if len(run) > 1:
                arcs.append(run)
            run = []
        else:
            run.append(np.array([x + math.cos(a) * (r - L.RAIL_IN), y, z + math.sin(a) * (r - L.RAIL_IN)]))
    if len(run) > 1:
        arcs.append(run)
    for i, run in enumerate(arcs):
        rail_line(b, f"{name}_{i + 1}", run, post_every=3.0)
