#!/usr/bin/env python3
"""Builds the own interface's 3D scene behind the menus (the role of the original's MenuLevel.nmo,
docs/drahtseilakt/resources.md) in the game's open level format: `assets/own/3D Entities/MenuLevel/` with level.gltf,
level.bin and its textures. Everything is made here from nothing: the geometry from a few measured proportions, the
textures from noise and shading, the sky from a function of the direction.

What it shows is the same kind of place as the original's menu: an island of Ballance's track pieces hanging in a dusk
sky, seen by a camera going round it. A central block with a dark bowl, four arms of track with raised borders, a
round start with four cups, flames and a stone ball, three transformer boxes at the end of one arm, two lantern towers
on tall poles, steel rails, a wooden ramp and columns reaching down into the clouds. Measured on the original (its
numbers only): the camera's circle (radius 95, height 40, round the origin), the lens, the pieces' sizes against
each other, where they stand round the camera's target, and the colours of the sky and of the stone.

The camera's way is given as nodes the game reads (menuscene.rs): `I_MenuLevel_Curve_0..3` (the points of its closed
curve), `Cam_MenuLevel_Target` (what it looks at); the flames burn at `FourFlames_Flame_A..D`; the sky is the
`Sky_*` objects, drawn round the camera.

Run from anywhere: python3 -I assets/own/make/make_menu_scene.py
Needs Python 3 with numpy and Pillow. The output is deterministic (fixed noise seeds).
"""

import json
import math
import os
import struct
import sys

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(os.path.dirname(HERE), "3D Entities", "MenuLevel")
TEX = 512


# --- noise and colour --------------------------------------------------------------------------------------------


def value_noise(h, w, cells, seed, octaves=5, tile=True):
    """Fractal value noise in 0..1; tiles seamlessly when `tile` (the lattice wraps)."""
    rng = np.random.default_rng(seed)
    out = np.zeros((h, w), np.float64)
    amp, total = 1.0, 0.0
    for o in range(octaves):
        ch, cw = max(2, int(cells[0] * 2**o)), max(2, int(cells[1] * 2**o))
        grid = rng.random((ch, cw))
        ys = np.arange(h) / h * ch
        xs = np.arange(w) / w * cw
        y0, x0 = np.floor(ys).astype(int), np.floor(xs).astype(int)
        fy, fx = ys - y0, xs - x0
        fy, fx = fy * fy * (3 - 2 * fy), fx * fx * (3 - 2 * fx)
        y1, x1 = (y0 + 1) % ch, (x0 + 1) % cw
        y0, x0 = y0 % ch, x0 % cw
        a = grid[y0][:, x0] * (1 - fx) + grid[y0][:, x1] * fx
        b = grid[y1][:, x0] * (1 - fx) + grid[y1][:, x1] * fx
        out += (a * (1 - fy)[:, None] + b * fy[:, None]) * amp
        total += amp
        amp *= 0.5
    return out / total


def rgb(*v):
    return np.array(v, np.float64) / 255.0


def save(img, name, alpha=None):
    a = np.clip(img * 255 + 0.5, 0, 255).astype(np.uint8)
    mode = "RGB"
    if alpha is not None:
        a = np.concatenate([a, np.clip(alpha * 255 + 0.5, 0, 255).astype(np.uint8)[..., None]], -1)
        mode = "RGBA"
    os.makedirs(os.path.join(OUT, "textures"), exist_ok=True)
    Image.fromarray(a, mode).save(os.path.join(OUT, "textures", name), optimize=True)


# --- textures ----------------------------------------------------------------------------------------------------
# Colours measured on the original's menu as rendered (its stone a warm sandy beige, its sides a little darker and
# browner, its border strips reddish brown, its poles tan, its sky violet blue at the top and rose at the horizon).


def stone(base, seed, slabs=2, mottle=0.16, grain=0.07, seams=0.35):
    n = value_noise(TEX, TEX, (4, 4), seed)
    f = value_noise(TEX, TEX, (64, 64), seed + 1, octaves=2)
    img = base[None, None, :] * (1 - mottle / 2 + mottle * n)[..., None] * (1 - grain / 2 + grain * f)[..., None]
    if slabs:
        k = TEX // slabs
        y, x = np.mgrid[0:TEX, 0:TEX]
        seam = np.minimum(np.minimum(y % k, k - 1 - y % k), np.minimum(x % k, k - 1 - x % k))
        img = img * (1 - seams * np.exp(-seam / 1.5))[..., None]
    return img


def strata(base, seed):
    n = value_noise(TEX, TEX, (2, 16), seed)
    rows = value_noise(TEX, 1, (24, 1), seed + 3, octaves=3)[:, :1]
    img = base[None, None, :] * (0.84 + 0.22 * n)[..., None] * (0.9 + 0.2 * rows)[..., None]
    y = np.arange(TEX)[:, None]
    img = img * (1 - 0.25 * np.exp(-np.minimum(y % 128, 127 - y % 128) / 1.2))[..., None]
    return img


def planks(base, seed, boards=6):
    y, x = np.mgrid[0:TEX, 0:TEX]
    k = TEX // boards
    board = np.minimum(y // k, boards - 1)
    rng = np.random.default_rng(seed)
    shade = rng.uniform(0.82, 1.12, boards)[board]
    grain = value_noise(TEX, TEX, (48, 3), seed + 1, octaves=3)
    rings = 0.5 + 0.5 * np.sin(grain * 40.0 + y * 0.15)
    img = base[None, None, :] * (shade * (0.85 + 0.2 * rings))[..., None]
    gap = np.minimum(y % k, k - 1 - y % k)
    img = img * (1 - 0.6 * np.exp(-gap / 1.0))[..., None]
    return img


def make_textures():
    save(stone(rgb(158, 140, 110), 11, slabs=2, seams=0.25), "floor_top.png")
    save(stone(rgb(132, 86, 62), 12, slabs=0, mottle=0.22), "floor_border.png")
    save(strata(rgb(176, 150, 122), 13), "floor_side.png")
    col = value_noise(TEX, TEX, (1, 24), 14, octaves=4)
    save(rgb(176, 152, 112)[None, None, :] * (0.78 + 0.32 * col)[..., None], "column.png")
    save(planks(rgb(124, 82, 50), 15), "wood.png")
    save(stone(rgb(122, 134, 148), 16, slabs=4, mottle=0.25, seams=0.5), "bluestone.png")
    # the transformers: granite, oak and paper-white plaster
    g = value_noise(TEX, TEX, (96, 96), 17, octaves=2)
    save(rgb(150, 150, 140)[None, None, :] * (0.65 + 0.6 * g)[..., None], "granite.png")
    save(planks(rgb(170, 120, 70), 18, boards=4), "oak.png")
    save(stone(rgb(226, 220, 204), 19, slabs=0, mottle=0.1), "plaster.png")
    # the roof's slates
    y, x = np.mgrid[0:TEX, 0:TEX]
    row = y // 32
    off = (row % 2) * 32
    tile = np.minimum((x + off) % 64, 63 - (x + off) % 64)
    slate = rgb(118, 108, 100)[None, None, :] * (0.8 + 0.3 * value_noise(TEX, TEX, (8, 8), 20))[..., None]
    slate = slate * (1 - 0.5 * np.exp(-np.minimum(tile, y % 32) / 1.5))[..., None]
    save(slate, "slate.png")
    # the lantern's glass glowing, and the grate of the start
    glow = 1 - 0.35 * np.hypot((x - TEX / 2) / TEX, (y - TEX / 2) / TEX)
    save(rgb(255, 214, 90)[None, None, :] * glow[..., None], "lantern.png")
    bars = ((x % 48 < 10) | (y % 48 < 10)).astype(np.float64)
    save(rgb(70, 66, 60)[None, None, :] * (0.4 + 0.6 * bars)[..., None], "grate.png")
    # the start's top: rings of stone round its middle
    r = np.hypot(x - TEX / 2, y - TEX / 2) / (TEX / 2)
    ring = 0.88 + 0.12 * np.cos(r * 28)
    pad = stone(rgb(172, 150, 116), 21, slabs=0) * ring[..., None]
    pad = np.where((r > 0.8)[..., None], pad * np.array([0.9, 0.72, 0.6]), pad)
    save(pad, "pad_top.png")
    # the stone ball: grey green, mottled
    m = value_noise(TEX, TEX, (6, 6), 22)
    save(rgb(150, 160, 140)[None, None, :] * (0.7 + 0.45 * m)[..., None], "ball_stone.png")
    # what polished steel reflects (the material's chrome mapping): a pale sky, a dark horizon, a bright floor
    v = y / TEX
    env = np.where(v < 0.45, 0.82 - 0.2 * v, np.where(v < 0.6, 0.32, 0.95))
    env = env + 0.05 * value_noise(TEX, TEX, (4, 8), 23)
    save(np.stack([env * 0.95, env * 0.97, env * 1.03], -1).clip(0, 1), "steel_env.png")
    # the dark bowl in the middle of the block
    save(np.stack([0.05 + 0.25 * env, 0.06 + 0.25 * env, 0.1 + 0.3 * env], -1), "bowl_env.png")


# --- the sky -----------------------------------------------------------------------------------------------------

# The five faces as the game's sky box draws them (sky.rs): corners on the unit cube, from the bottom left as seen
# from inside, the texture's rows from the top.
FACES = {
    "Front": [(-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)],
    "Right": [(1, -1, 1), (1, -1, -1), (1, 1, -1), (1, 1, 1)],
    "Back": [(1, -1, -1), (-1, -1, -1), (-1, 1, -1), (1, 1, -1)],
    "Left": [(-1, -1, -1), (-1, -1, 1), (-1, 1, 1), (-1, 1, -1)],
    "Down": [(-1, -1, -1), (1, -1, -1), (1, -1, 1), (-1, -1, 1)],
}
SKY = 512


# The sky's colour by the elevation (degrees), measured on the original's menu as rendered (the camera looks 23 degrees
# down, so the menu sees the sky from the horizon down): violet blue at the horizon, warming through lilac to a rose
# beige sea of cloud below.
SKY_PROFILE = [(-90, (212, 176, 160)), (-40, (209, 173, 158)), (-32, (195, 166, 156)), (-25, (165, 150, 154)),
               (-18, (155, 144, 162)), (-12, (142, 136, 168)), (-8, (134, 130, 171)), (-3, (122, 122, 172)),
               (90, (110, 112, 168))]


def sky_colour(d):
    """The sky in direction d (unit vectors, ... x 3): the measured gradient by elevation, thin streaks of cloud
    near the horizon and a sea of soft lit clouds below (our own noise)."""
    e = np.degrees(np.arcsin(np.clip(d[..., 1], -1, 1)))
    xs = [p[0] for p in SKY_PROFILE]
    c = np.stack([np.interp(e, xs, [p[1][k] / 255.0 for p in SKY_PROFILE]) for k in range(3)], -1)
    az = np.arctan2(d[..., 0], d[..., 2])
    u = (az / (2 * np.pi) + 0.5) % 1.0
    k = 1024
    sx = np.clip(u * (k - 1), 0, k - 1).astype(int)
    # thin streaks a little below the horizon, stretched along it
    streak_map = value_noise(128, k, (3, 5), 31, octaves=5)
    sy = np.clip((-e + 2) / 30 * 127, 0, 127).astype(int)
    band = np.exp(-(((e + 12) / 9) ** 2))
    streak = np.clip((streak_map[sy, sx] - 0.5) * 2.5, 0, 1) * band
    c = c + (np.array([0.86, 0.78, 0.8]) - c) * (0.45 * streak)[..., None]
    # the cloud sea further down: soft puffs, lighter and warmer where they are lit
    sea_map = value_noise(256, k, (6, 5), 37, octaves=6)
    py = np.clip((-e - 10) / 80 * 255, 0, 255).astype(int)
    puff = np.clip((sea_map[py, sx] - 0.45) * 2.2, -1, 1)
    deep = np.clip((-e - 14) / 16, 0, 1)
    lit = np.array([0.9, 0.78, 0.72])
    shade = np.array([0.6, 0.55, 0.62])
    c = c + np.where((puff > 0)[..., None], (lit - c) * (0.6 * puff)[..., None], (shade - c) * (-0.4 * puff)[..., None]) \
        * deep[..., None]
    return np.clip(c, 0, 1)


def make_sky():
    j, i = np.mgrid[0:SKY, 0:SKY]
    s = (i + 0.5) / SKY
    t = 1 - (j + 0.5) / SKY  # rows from the top: t = 1 at the top
    for face, corners in FACES.items():
        p0, p1, p2, p3 = (np.array(c, np.float64) for c in corners)
        p = (p0[None, None] * ((1 - s) * (1 - t))[..., None] + p1[None, None] * (s * (1 - t))[..., None]
             + p2[None, None] * (s * t)[..., None] + p3[None, None] * ((1 - s) * t)[..., None])
        d = p / np.linalg.norm(p, axis=-1, keepdims=True)
        save(np.clip(sky_colour(d), 0, 1), f"sky_{face.lower()}.png")


# --- geometry ----------------------------------------------------------------------------------------------------


class Mesh:
    def __init__(self, name):
        self.name = name
        self.p, self.n, self.uv = [], [], []
        self.parts = {}

    def vertex(self, p, n, uv):
        self.p.append(tuple(map(float, p)))
        self.n.append(tuple(map(float, n)))
        self.uv.append(tuple(map(float, uv)))
        return len(self.p) - 1

    def tri(self, mat, a, b, c):
        self.parts.setdefault(mat, []).extend([a, b, c])

    def quad(self, mat, pts, uvs, normal=None):
        pts = [np.asarray(q, np.float64) for q in pts]
        n = normal if normal is not None else np.cross(pts[1] - pts[0], pts[3] - pts[0])
        n = n / (np.linalg.norm(n) + 1e-12)
        ids = [self.vertex(q, n, u) for q, u in zip(pts, uvs)]
        self.tri(mat, ids[0], ids[1], ids[2])
        self.tri(mat, ids[0], ids[2], ids[3])


def box(m, mat, lo, hi, scale=8.0, mats=None):
    """An axis-aligned box; `mats` may give the top, the sides and the bottom their own materials."""
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    top, side, bottom = mats if mats else (mat, mat, mat)
    u = lambda a, b: (a / scale, b / scale)
    m.quad(top, [(x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)],
           [u(x0, z0), u(x1, z0), u(x1, z1), u(x0, z1)], np.array([0, 1, 0]))
    m.quad(bottom, [(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)],
           [u(x0, z0), u(x1, z0), u(x1, z1), u(x0, z1)], np.array([0, -1, 0]))
    m.quad(side, [(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0)],
           [u(x0, y0), u(x1, y0), u(x1, y1), u(x0, y1)], np.array([0, 0, -1]))
    m.quad(side, [(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)],
           [u(x0, y0), u(x1, y0), u(x1, y1), u(x0, y1)], np.array([0, 0, 1]))
    m.quad(side, [(x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0)],
           [u(z0, y0), u(z1, y0), u(z1, y1), u(z0, y1)], np.array([-1, 0, 0]))
    m.quad(side, [(x1, y0, z0), (x1, y0, z1), (x1, y1, z1), (x1, y1, z0)],
           [u(z0, y0), u(z1, y0), u(z1, y1), u(z0, y1)], np.array([1, 0, 0]))


def cylinder(m, mat, base, r, h, seg=16, caps=True, cap_mat=None, vscale=8.0, r_top=None):
    """An upright cylinder (or a cone, with r_top) standing on `base`."""
    bx, by, bz = base
    rt = r if r_top is None else r_top
    ring = []
    for k in range(seg + 1):
        a = 2 * math.pi * k / seg
        ca, sa = math.cos(a), math.sin(a)
        slope = (r - rt) / h
        n = np.array([ca, slope, sa])
        n /= np.linalg.norm(n)
        lo = m.vertex((bx + r * ca, by, bz + r * sa), n, (k / seg * 2, by / vscale))
        hi = m.vertex((bx + rt * ca, by + h, bz + rt * sa), n, (k / seg * 2, (by + h) / vscale))
        ring.append((lo, hi))
    for k in range(seg):
        (a0, a1), (b0, b1) = ring[k], ring[k + 1]
        m.tri(mat, a0, b0, b1)
        m.tri(mat, a0, b1, a1)
    if caps:
        cm = cap_mat if cap_mat is not None else mat
        for y, rr, ny in ((by, r, -1), (by + h, rt, 1)):
            if rr <= 0:
                continue
            c = m.vertex((bx, y, bz), (0, ny, 0), (0.5, 0.5))
            pts = [m.vertex((bx + rr * math.cos(2 * math.pi * k / seg), y, bz + rr * math.sin(2 * math.pi * k / seg)),
                            (0, ny, 0), (0.5 + 0.5 * math.cos(2 * math.pi * k / seg),
                                         0.5 + 0.5 * math.sin(2 * math.pi * k / seg))) for k in range(seg + 1)]
            for k in range(seg):
                m.tri(cm, c, pts[k], pts[k + 1])


def sphere(m, mat, centre, r, seg=20):
    cx, cy, cz = centre
    rows = []
    for i in range(seg // 2 + 1):
        th = math.pi * i / (seg // 2)
        row = []
        for k in range(seg + 1):
            ph = 2 * math.pi * k / seg
            n = (math.sin(th) * math.cos(ph), math.cos(th), math.sin(th) * math.sin(ph))
            row.append(m.vertex((cx + r * n[0], cy + r * n[1], cz + r * n[2]), n, (k / seg, i / (seg // 2))))
        rows.append(row)
    for i in range(seg // 2):
        for k in range(seg):
            a, b, c, d = rows[i][k], rows[i][k + 1], rows[i + 1][k + 1], rows[i + 1][k]
            m.tri(mat, a, b, c)
            m.tri(mat, a, c, d)


def frames(path):
    """Along a path of points: each point with its forward, sideways and up directions (up stays near the world's)."""
    out = []
    pts = [np.asarray(p, np.float64) for p in path]
    for i, p in enumerate(pts):
        f = pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]
        f /= np.linalg.norm(f) + 1e-12
        side = np.cross(np.array([0.0, 1.0, 0.0]), f)
        side /= np.linalg.norm(side) + 1e-12
        up = np.cross(f, side)
        out.append((p, f, side, up))
    return out


def sweep(m, path, profile, mats, closed_profile=True, uscale=6.0, vscale=10.0):
    """A cross-section (points (across, up) with a material per edge) carried along a path."""
    fr = frames(path)
    along = [0.0]
    for i in range(1, len(fr)):
        along.append(along[-1] + float(np.linalg.norm(fr[i][0] - fr[i - 1][0])))
    n = len(profile)
    edges = range(n if closed_profile else n - 1)
    run = [0.0]
    for e in range(n - 1 if not closed_profile else n):
        a, b = np.array(profile[e]), np.array(profile[(e + 1) % n])
        run.append(run[-1] + float(np.linalg.norm(b - a)))
    for e in edges:
        a, b = np.array(profile[e]), np.array(profile[(e + 1) % n])
        d = b - a
        nrm2 = np.array([d[1], -d[0]]) / (np.linalg.norm(d) + 1e-12)
        mat = mats[e]
        for i in range(len(fr) - 1):
            p0, _, s0, u0 = fr[i]
            p1, _, s1, u1 = fr[i + 1]
            q = [p0 + s0 * a[0] + u0 * a[1], p1 + s1 * a[0] + u1 * a[1], p1 + s1 * b[0] + u1 * b[1],
                 p0 + s0 * b[0] + u0 * b[1]]
            nm = (s0 + s1) / 2 * nrm2[0] + (u0 + u1) / 2 * nrm2[1]
            uvs = [(run[e] / uscale, along[i] / vscale), (run[e] / uscale, along[i + 1] / vscale),
                   (run[e + 1] / uscale, along[i + 1] / vscale), (run[e + 1] / uscale, along[i] / vscale)]
            if mat in ("top", "border_top"):
                # the floor's top is mapped flat from above, so its slabs keep their size
                uvs = [(qq[0] / 16.0, qq[2] / 16.0) for qq in q]
            m.quad(MAT[mat], q, uvs, nm)
    # the ends closed with the section's outline as a fan
    for i, sgn in ((0, -1), (len(fr) - 1, 1)):
        p, f, s, u = fr[i]
        c = m.vertex(p + u * float(np.mean([q[1] for q in profile])), f * sgn, (0.5, 0.5))
        ids = [m.vertex(p + s * q[0] + u * q[1], f * sgn, (q[0] / uscale, q[1] / uscale)) for q in profile]
        for k in range(n):
            m.tri(MAT["side"], c, ids[k], ids[(k + 1) % n])


def arc_path(cx, cz, r, a0, a1, y0, y1, steps=24):
    return [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * k / steps)), y0 + (y1 - y0) * k / steps,
             cz + r * math.sin(math.radians(a0 + (a1 - a0) * k / steps))) for k in range(steps + 1)]


def line_path(a, b, steps=4):
    a, b = np.asarray(a, np.float64), np.asarray(b, np.float64)
    return [tuple(a + (b - a) * k / steps) for k in range(steps + 1)]


# A track's cross-section (across, up), measured against the ball: 10 wide, borders 1.2 high and 1.2 wide, 3 deep.
TRACK = [(-5.0, -3.0), (-5.0, 1.2), (-3.8, 1.2), (-3.8, 0.0), (3.8, 0.0), (3.8, 1.2), (5.0, 1.2), (5.0, -3.0)]
TRACK_MATS = ["side", "border_top", "border", "top", "border", "border_top", "side", "side"]
# The narrow track (the original's high west arm is 4 wide).
NARROW = [(-3.0, -2.5), (-3.0, 0.9), (-2.2, 0.9), (-2.2, 0.0), (2.2, 0.0), (2.2, 0.9), (3.0, 0.9), (3.0, -2.5)]
NARROW_MATS = TRACK_MATS


def tube(m, mat, path, r, seg=8):
    """A round tube along a path (rails, posts)."""
    fr = frames(path)
    rings = []
    for p, f, s, u in fr:
        ring = []
        for k in range(seg + 1):
            a = 2 * math.pi * k / seg
            n = s * math.cos(a) + u * math.sin(a)
            ring.append(m.vertex(p + n * r, n, (k / seg, 0)))
        rings.append(ring)
    for i in range(len(rings) - 1):
        for k in range(seg):
            a, b, c, d = rings[i][k], rings[i][k + 1], rings[i + 1][k + 1], rings[i + 1][k]
            m.tri(mat, a, b, c)
            m.tri(mat, a, c, d)


def rails(m, path, gauge=1.3, r=0.28, posts=6.0):
    """Two steel tubes side by side, a short post under them every few units."""
    fr = frames(path)
    for side in (-gauge / 2, gauge / 2):
        tube(m, MAT["steel"], [p + s * side for p, f, s, u in fr], r)
    run = 0.0
    for i in range(1, len(fr)):
        run += float(np.linalg.norm(fr[i][0] - fr[i - 1][0]))
        if run >= posts:
            run = 0.0
            p, f, s, u = fr[i]
            tube(m, MAT["steel"], [p - s * gauge / 2, p + s * gauge / 2], r * 0.8)
            tube(m, MAT["steel"], [p, p - u * 1.6], r * 0.8)


# --- the scene ---------------------------------------------------------------------------------------------------

MATERIALS = [
    # name, texture, diffuse, emissive, specular, power, extra
    ("top", "floor_top.png", 1.0, 0.12, 0.31, 100, {}),
    ("border", "floor_border.png", 1.0, 0.24, 0.0, 0, {}),
    ("border_top", "floor_border.png", 1.0, 0.28, 0.0, 0, {}),
    ("side", "floor_side.png", 0.6, 0.42, 0.0, 0, {}),
    ("column", "column.png", 0.85, 0.32, 0.0, 0, {}),
    ("wood", "wood.png", 1.0, 0.25, 0.39, 50, {}),
    ("bluestone", "bluestone.png", 0.8, 0.32, 0.6, 31, {}),
    ("granite", "granite.png", 0.8, 0.35, 0.3, 30, {}),
    ("oak", "oak.png", 0.8, 0.35, 0.3, 30, {}),
    ("plaster", "plaster.png", 0.8, 0.35, 0.3, 30, {}),
    ("slate", "slate.png", 0.85, 0.3, 0.25, 100, {}),
    ("lantern", "lantern.png", 0.0, 1.0, 0.0, 0, {}),
    ("grate", "grate.png", 1.0, 0.1, 0.0, 0, {}),
    ("pad", "pad_top.png", 1.0, 0.25, 0.31, 100, {}),
    ("ball", "ball_stone.png", 1.0, 0.31, 0.29, 11, {}),
    ("steel", "steel_env.png", 0.4, 0.4, 0.82, 10, {"texgen": "chrome"}),
    ("bowl", "bowl_env.png", 0.3, 0.6, 0.9, 13, {"texgen": "reflect"}),
]
MAT = {name: i for i, (name, *_rest) in enumerate(MATERIALS)}
SKY_MATS = {face: len(MATERIALS) + i for i, face in enumerate(FACES)}


def build():
    """The meshes and the objects placing them."""
    meshes, objects = [], []

    def add(name, mesh):
        meshes.append(mesh)
        objects.append((name, len(meshes) - 1, True))

    # the central block: its top at 8, a dark bowl in it, legs reaching down into the clouds
    hub = Mesh("Hub")
    box(hub, MAT["top"], (-11, -22, -11), (11, 8, 11), mats=(MAT["top"], MAT["side"], MAT["side"]))
    for x in (-11.6, 11.6):
        box(hub, MAT["border"], (x - 0.6, 7.2, -11.6), (x + 0.6, 9.2, 11.6))
    for z in (-11.6, 11.6):
        box(hub, MAT["border"], (-11.6, 7.2, z - 0.6), (11.6, 9.2, z + 0.6))
    cylinder(hub, MAT["bowl"], (0, 8.02, 0), 6.5, 0.05, seg=32)
    tube(hub, MAT["steel"], arc_path(0, 0, 6.8, 0, 360, 8.3, 8.3, 48), 0.45)
    for x in (-7.5, 7.5):
        for z in (-7.5, 7.5):
            cylinder(hub, MAT["column"], (x, -260, z), 1.5, 238, caps=False, vscale=24.0)
    add("Menu_Hub", hub)

    # The pieces stand where the original's do round the camera's target (measured: the heights of its floors'
    # tops and the ground they cover): the block's top at 8; a rising arm east to a platform at 13 with the three
    # transformers; a high narrow arm west at 15 ending in a spiral of rail; the start south-west at 6; two lantern
    # towers north; a lower floor at -7 crossing under the block north and south; far floors at -28 further north.
    arms = Mesh("Arms")
    sweep(arms, line_path((11, 8, 0), (36, 13, 0)), TRACK, TRACK_MATS)
    sweep(arms, line_path((-11, 8, 0), (-36, 15, 0)) + line_path((-36, 15, 0), (-98, 15, 0))[1:], NARROW,
          NARROW_MATS)
    sweep(arms, line_path((-6, 8, -11), (-15, 6.4, -29)), TRACK, TRACK_MATS)
    add("Menu_Arms", arms)

    low = Mesh("Lower")
    sweep(low, line_path((22, -7, -22.5), (-30, -7, -22.5)) + arc_path(-30, -2, 20.5, 270, 90, -7, -7, 20)[1:]
          + line_path((-30, -7, 18.5), (31, -7, 18.5))[1:], NARROW, NARROW_MATS)
    box(low, MAT["top"], (-7, -10, 16), (7, -3.9, 29), mats=(MAT["top"], MAT["side"], MAT["side"]))
    sweep(low, line_path((-50.5, -7, -2), (-80, -6, -40)) + line_path((-80, -6, -40), (-110, -5, -80))[1:], NARROW,
          NARROW_MATS)
    box(low, MAT["top"], (-136, -9, -96), (-100, -4.9, -78), mats=(MAT["top"], MAT["side"], MAT["side"]))
    for x, z in ((20, -22.5), (-30, -22.5), (30, 18.5), (-30, 18.5), (-118, -87), (-80, -40)):
        cylinder(low, MAT["column"], (x, -260, z), 1.2, 253, caps=False, vscale=24.0)
    add("Menu_Lower", low)

    # the platform at the east arm's end
    ends = Mesh("Ends")
    sweep(ends, line_path((36, 13, 0), (59, 13, 0)), TRACK, TRACK_MATS)
    sweep(ends, line_path((45, 13.02, -13), (45, 13.02, 13)), TRACK, TRACK_MATS)
    for x in (42, 54):
        cylinder(ends, MAT["column"], (x, -240, 0), 1.2, 246, caps=False, vscale=24.0)
    cylinder(ends, MAT["column"], (-70, -240, 0), 1.0, 252, caps=False, vscale=24.0)
    add("Menu_Ends", ends)

    # three transformers: a cube of each ball's stuff on a steel-edged plinth, where the original's stand
    trafos = Mesh("Trafos")
    for (x, z), mat in (((45, -10), "granite"), ((55, 0), "oak"), ((45, 10), "plaster")):
        box(trafos, MAT["steel"], (x - 2.8, 13.1, z - 2.8), (x + 2.8, 13.8, z + 2.8))
        box(trafos, MAT[mat], (x - 2.4, 13.8, z - 2.4), (x + 2.4, 18.2, z + 2.4), scale=4.8)
        tube(trafos, MAT["steel"], arc_path(x, z, 1.5, 0, 360, 18.3, 18.3, 24), 0.3)
    add("Menu_Trafos", trafos)

    # the start: a round slab with four cups for its flames, a grate in its middle (measured: its top at 6 to 7,
    # its flames on a square of 12.2 round (-19.8, -39.1))
    pad = Mesh("Start")
    sx, sz = -19.8, -39.1
    cylinder(pad, MAT["side"], (sx, 2.5, sz), 8.5, 4.0, seg=40, cap_mat=MAT["pad"])
    cylinder(pad, MAT["grate"], (sx, 6.52, sz), 2.4, 0.05, seg=24)
    flames = []
    for k, a in enumerate((45, 135, 225, 315)):
        cx, cz = sx + 8.6 * math.cos(math.radians(a)), sz + 8.6 * math.sin(math.radians(a))
        cylinder(pad, MAT["side"], (cx, 2.5, cz), 2.6, 4.2, seg=20, cap_mat=MAT["pad"])
        cylinder(pad, MAT["grate"], (cx, 6.72, cz), 1.5, 0.05, seg=16)
        tube(pad, MAT["border"], arc_path(cx, cz, 1.7, 0, 360, 6.8, 6.8, 20), 0.25)
        flames.append((cx, 7.0, cz))
    cylinder(pad, MAT["column"], (sx, -250, sz), 1.4, 252.5, caps=False, vscale=24.0)
    add("Menu_Start", pad)
    ball = Mesh("Ball")
    sphere(ball, MAT["ball"], (sx, 8.5, sz), 2.0)
    add("Menu_Ball_Stone", ball)

    # two lantern towers on tall poles north of the block (measured: x +-14.4, z 53, from 9 to 27)
    towers = Mesh("Towers")
    for x in (-14.4, 14.4):
        z = 53.2
        cylinder(towers, MAT["column"], (x, -220, z), 0.9, 230, caps=False, vscale=24.0)
        box(towers, MAT["bluestone"], (x - 1.9, 10, z - 1.9), (x + 1.9, 13.6, z + 1.9), scale=3.8)
        box(towers, MAT["border"], (x - 2.2, 13.6, z - 2.2), (x + 2.2, 14.1, z + 2.2))
        for dx in (-1.5, 1.5):
            for dz in (-1.5, 1.5):
                tube(towers, MAT["steel"], [(x + dx, 14.1, z + dz), (x + dx, 19.6, z + dz)], 0.16, seg=6)
        box(towers, MAT["lantern"], (x - 0.8, 15.0, z - 0.8), (x + 0.8, 18.2, z + 0.8), scale=1.6)
        cylinder(towers, MAT["slate"], (x, 19.6, z), 4.0, 3.2, seg=4, r_top=0.0, vscale=3.2)
        tube(towers, MAT["steel"], [(x, 23.6, z), (x, 26.4, z)], 0.12, seg=6)
        sphere(towers, MAT["steel"], (x, 26.6, z), 0.3, seg=8)
    add("Menu_Towers", towers)

    # steel rails: loops from the high arms down to the lower floor round the block, one descending south-west
    # and a spiral at the end of the west arm
    steel = Mesh("Rails")
    rails(steel, [(24, 13.6, 4)] + arc_path(24, 14, 10, 270, 90, 10, -6.4, 24)[1:] + [(30, -6.4, 18.5)])
    rails(steel, [(-24, 12.6, -2.5)] + arc_path(-24, -12, 10, 90, 270, 10, -6.4, 24)[1:] + [(-24, -6.4, -22.5)])
    rails(steel, line_path((-50, 6.6, -40), (-90, -2, -70), 8) + line_path((-90, -2, -70), (-130, -10, -90), 8)[1:])
    spiral = [(-109 + 8 * math.cos(t), 15.6 - 1.8 * t, -4 + 8 * math.sin(t)) for t in np.linspace(0, 4 * math.pi, 64)]
    rails(steel, [(-98, 15.6, 0)] + spiral)
    add("Menu_Rails", steel)

    # a wooden ramp from the lower floor north, waving down (measured: from -3 at z 30 to -27 at z 87)
    ramp = Mesh("Ramp")
    path = [(2.0 * math.sin(k / 3.0), -4 - 0.92 * k, 29 + 2.3 * k) for k in range(26)]
    plank = [(-2.2, -0.4), (-2.2, 0.0), (2.2, 0.0), (2.2, -0.4)]
    sweep(ramp, path, plank, ["wood"] * 4, uscale=4.4, vscale=4.4)
    sweep(ramp, [(-75 + 2.5 * k, 15.4 + 1.2 * math.sin(k / 2.0), 0) for k in range(10)], plank, ["wood"] * 4,
          uscale=4.4, vscale=4.4)
    add("Menu_Ramp", ramp)

    # the far floors north, at -28, and a stack of them further on
    far = Mesh("Far")
    box(far, MAT["top"], (-106, -34, 103), (6, -27.9, 125), mats=(MAT["top"], MAT["side"], MAT["side"]))
    sweep(far, line_path((-45, -28, 125), (-45, -28, 270)), TRACK, TRACK_MATS)
    sweep(far, line_path((0, -28, 125), (0, -28, 220)) + arc_path(-30, 220, 30, 0, 90, -28, -28, 16)[1:], TRACK,
          TRACK_MATS)
    for k, y in enumerate((-23, -18, -13, -8)):
        box(far, MAT["top"], (-53 - k, y - 3, 275 - k), (-16 + k, y + 0.1, 313 + k),
            mats=(MAT["top"], MAT["side"], MAT["side"]))
    for x, z in ((-45, 180), (-45, 260), (0, 180), (-100, 110), (0, 110), (-35, 294)):
        cylinder(far, MAT["column"], (x, -340, z), 1.6, 310, caps=False, vscale=24.0)
    add("Menu_Far", far)

    # the sky: five faces round the camera
    for face, corners in FACES.items():
        s = Mesh(f"Sky_{face}")
        q = [tuple(c * 1200 for c in corner) for corner in corners]
        ids = [s.vertex(p, (0, 1, 0), uv) for p, uv in zip(q, [(0.002, 0.998), (0.998, 0.998), (0.998, 0.002),
                                                               (0.002, 0.002)])]
        s.tri(SKY_MATS[face], ids[0], ids[1], ids[2])
        s.tri(SKY_MATS[face], ids[0], ids[2], ids[3])
        add(f"Sky_{face}", s)

    # the camera's way and target, the flames' frames: objects without meshes
    marks = [("Cam_MenuLevel_Target", (0.0, 0.0, 0.0))]
    for k, a in enumerate((270, 0, 90, 180)):
        marks.append((f"I_MenuLevel_Curve_{k}", (95 * math.cos(math.radians(a)), 40.0, 95 * math.sin(math.radians(a)))))
    for letter, p in zip("ABCD", flames):
        marks.append((f"FourFlames_Flame_{letter}", p))
    return meshes, objects, marks


# --- the open format (level.gltf + level.bin), as crates/level/src/open.rs reads it ------------------------------


def write(meshes, objects, marks):
    bin_ = bytearray()
    views, accessors = [], []

    def push(data, target, acc):
        while len(bin_) % 4:
            bin_.append(0)
        views.append({"buffer": 0, "byteOffset": len(bin_), "byteLength": len(data), "target": target})
        bin_.extend(data)
        acc["bufferView"] = len(views) - 1
        accessors.append(acc)
        return len(accessors) - 1

    gl_meshes = []
    for m in meshes:
        # Virtools is left-handed and glTF right-handed: z is negated and each face's winding turned (open.rs)
        pos = [(x, y, -z) for x, y, z in m.p]
        nor = [(x, y, -z) for x, y, z in m.n]
        lo = [min(p[k] for p in pos) for k in range(3)]
        hi = [max(p[k] for p in pos) for k in range(3)]
        flat = lambda vs: struct.pack(f"<{len(vs) * len(vs[0])}f", *[c for v in vs for c in v])
        pa = push(flat(pos), 34962, {"componentType": 5126, "count": len(pos), "type": "VEC3", "min": lo, "max": hi})
        na = push(flat(nor), 34962, {"componentType": 5126, "count": len(nor), "type": "VEC3"})
        ta = push(flat(m.uv), 34962, {"componentType": 5126, "count": len(m.uv), "type": "VEC2"})
        attrs = {"POSITION": pa, "NORMAL": na, "TEXCOORD_0": ta}
        prims = []
        for mat, idx in sorted(m.parts.items()):
            tri = [i for k in range(0, len(idx), 3) for i in (idx[k], idx[k + 2], idx[k + 1])]
            ia = push(struct.pack(f"<{len(tri)}I", *tri), 34963,
                      {"componentType": 5125, "count": len(tri), "type": "SCALAR"})
            prims.append({"attributes": attrs, "indices": ia, "material": mat})
        gl_meshes.append({"name": m.name, "primitives": prims})

    images = [n for (_, n, *_r) in MATERIALS] + [f"sky_{f.lower()}.png" for f in FACES]
    image_ids = {n: i for i, n in enumerate(dict.fromkeys(images))}
    materials = []
    for name, tex, diffuse, emissive, spec, power, extra in MATERIALS:
        x = {"ambient": [0.1, 0.1, 0.1], "z_write": True, "specular": [spec] * 3, "power": power}
        x.update(extra)
        materials.append({
            "name": name,
            "pbrMetallicRoughness": {"baseColorFactor": [diffuse, diffuse, diffuse, 1.0], "metallicFactor": 0.0,
                                     "roughnessFactor": 1.0, "baseColorTexture": {"index": image_ids[tex]}},
            "emissiveFactor": [emissive] * 3,
            "doubleSided": True,
            "alphaMode": "OPAQUE",
            "extras": {"drahtseilakt": x},
        })
    for face in FACES:
        materials.append({
            "name": f"Sky_{face}",
            "pbrMetallicRoughness": {"baseColorFactor": [1.0, 1.0, 1.0, 1.0], "metallicFactor": 0.0,
                                     "roughnessFactor": 1.0,
                                     "baseColorTexture": {"index": image_ids[f"sky_{face.lower()}.png"]}},
            "emissiveFactor": [0.0, 0.0, 0.0],
            "doubleSided": True,
            "alphaMode": "OPAQUE",
            "extensions": {"KHR_materials_unlit": {}},
            "extras": {"drahtseilakt": {"ambient": [0, 0, 0], "z_write": False, "specular": [0, 0, 0], "power": 0}},
        })
    ident = [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0]
    nodes = [{"name": name, "matrix": ident, "mesh": mesh, "extras": {"drahtseilakt": {"visible": vis}}}
             for name, mesh, vis in objects]
    for name, (x, y, z) in marks:
        nodes.append({"name": name, "matrix": [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0, 0, x, y, -z, 1.0],
                      "extras": {"drahtseilakt": {"visible": False}}})
    doc = {
        "asset": {"version": "2.0", "generator": "drahtseilakt make_menu_scene.py"},
        "extensionsUsed": ["KHR_materials_unlit"],
        "scene": 0,
        "scenes": [{"nodes": list(range(len(nodes))), "extras": {"drahtseilakt": {
            "format": 1, "groups": [], "sky": None, "start_ball": None,
            "settings": {"tutorial": False, "last_level": False}}}}],
        "nodes": nodes,
        "meshes": gl_meshes,
        "materials": materials,
        "textures": [{"source": i, "sampler": 0} for i in range(len(image_ids))],
        "images": [{"uri": f"textures/{n}", "name": n[:-4]} for n in image_ids],
        "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}],
        "buffers": [{"uri": "level.bin", "byteLength": len(bin_)}],
        "bufferViews": views,
        "accessors": accessors,
    }
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, "level.gltf"), "w") as f:
        json.dump(doc, f, indent=1)
    with open(os.path.join(OUT, "level.bin"), "wb") as f:
        f.write(bin_)
    print(f"{OUT}: {len(nodes)} objects, {sum(len(m.p) for m in meshes)} vertices, {len(bin_)} bytes")


def main():
    if "--geometry" not in sys.argv:
        make_textures()
        make_sky()
    write(*build())


if __name__ == "__main__":
    sys.exit(main())
