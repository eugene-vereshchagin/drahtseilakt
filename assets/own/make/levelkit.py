#!/usr/bin/env python3
"""Builds the game's own levels in the open format (docs/drahtseilakt/open-format.md) from track pieces: floors with
their safety (railings, gutter, brick border or none), rails, the start's bowl with its four flame cups, checkpoints'
bowls with their two flame cups across the way, the finish's balloon, and the
stand-ins of the parts (boxes, domes, loose balls, transformers, extra points and lives, the modules), each in the
groups the game's rules read (docs/drahtseilakt/nmo-format.md, "What the game reads from levels"). A level made here
names only our own parts: the game takes them from assets/own/ (resources.md), so it plays with no copy of the
original at all.

The game's coordinates (Virtools: left-handed, Y up). The camera starts looking along +X, so a level runs along +X
from its start. Floors' tops are at the heights given; a stand-in's origin is where its part stands.

Sizes measured on the original (numbers only, nothing of its geometry): the ball's radius 2, the flames of the start at
(±7.3, 2.05, ±6.1) and of a checkpoint at (0, 2.05, ±7) about its stand-in (the parts PS_FourFlames and PC_TwoFlames
hold those frames), a box 5 on a side, a dome 13.3 across and 3.55 high, a transformer's pad 5 across; the floors'
cross-sections, the bowls and the textures' scale below.

Needs Python 3 with numpy and Pillow; ownkit.py beside it.
"""

import math
import os
import shutil

import numpy as np

import ownkit as K

TEX = 256
OWN = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# The game's own texture set (CC0), by the original's texture names.
TEXTURE_SET = os.path.join(OWN, "Textures")
# The track's own textures (make_textures.py, step track) and the menu scene's bricks, which the borders share.
TRACK_SET = os.path.join(OWN, "Track")
MENU_SET = os.path.join(OWN, "3D Entities", "MenuLevel", "textures")
# The track's cross-section (across, up): its top at 0, 2.5 deep.
DEPTH = 2.5
# How a stretch of track keeps the ball on (decisions in docs/drahtseilakt/levels.md, "Safety"), measured on the
# original where it has the kind (numbers only):
#   railings  a steel railing along both edges (the original's Geländer: a hand rail about 2.7 over the floor, a
#             second rail lower, posts between), the ball stopped by its rails;
#   gutter    the original's profiled floor: its edges 0.7 over a flat middle, falling over 2.5 to it in two facets
#             (0.46 high 0.84 in), a ball drifting to an edge rolls back to the middle;
#   border    a raised brick border along both edges, 1.0 wide and 2.2 high: higher than the ball's middle (its
#             radius is 2), so the ball meets its face square and is turned back; a lower one (the menu scene's 1.2,
#             or 1.4) meets it below its middle on the border's edge, and a fast ball rides up over it;
#   none      the bare floor, its edges marked only by the brick strip.
SAFETY = ("railings", "gutter", "border", "none")
KERB = 1.25  # the edge band: the brick strip, a groove and the shade, as one kerb stone
GUTTER = ((0.0, 0.7), (0.84, 0.46), (2.5, 0.0))  # (in from the edge, up) of a gutter's slope
BORDER_W, BORDER_H = 1.0, 2.2
RAIL_IN, RAIL_HEIGHTS, RAIL_R, POST_EVERY = 0.35, (1.3, 2.6), 0.17, 6.0
# The textures' scale: a slab of 5 units, four slabs along a texture; the border's bricks 10 units a repeat (as the
# menu scene has them).
SLAB, REPEAT, BRICK = 5.0, 20.0, 10.0
# The start's and the checkpoints' bowls, measured on the original's (radius, depth below the floor round them):
# the start's 7.5 across its radius, 1.25 deep, curving (3.05, 0.21) (4.87, 0.5) (6.35, 0.85); a checkpoint's 5,
# 1.02 deep, curving (2.03, 0.22) (3.24, 0.51) (4.23, 0.86). The original's checkpoint has a lip 0.24 high round its
# bowl besides; ours leaves it out, the floor round the bowl at the bowl's edge: a ball stopped in the bowl must roll out
# of it again, and a lip that steep (a step for the ball) holds the stone ball, slow as it is, in the bowl. Their frames (PS_FourFlames,
# PC_TwoFlames) and reset points stand in the bowl's bottom, the reset points 3.65 and 3.33 over it, and a
# checkpoint's flames at ±7 across the way (its frame's z axis across).
START_BOWL = [(0.0, 0.0), (3.05, 0.21), (4.87, 0.5), (6.35, 0.85), (7.5, 1.25)]
START_RADIUS, START_DEEP, START_RESET = 10.0, 5.0, 3.65
CHECK_BOWL = [(0.0, 0.0), (2.03, 0.22), (3.24, 0.51), (4.23, 0.86), (5.0, 1.02)]
CHECK_LEN, CHECK_RESET, CHECK_FLAMES = 10.0, 3.33, 7.0
CUP_R, CUP_UP = 2.0, 0.3
# Where the ball comes back after a fall, above the floor round the start or a checkpoint.
RESET_UP = START_RESET - START_BOWL[-1][1]


def section(width, safety, surface="stone"):
    """A track's cross-section for its width and safety: its outline (across, up), from the bottom of its left side up,
    over the top and down the right side (the bottom closes it), and for each edge its material and how its texture
    runs across it: ("down", top) the side's, by the depth under `top`; ("band", d0, d1, w) a band's, by the distance
    in from its outer edge (d0 at the edge's first point, d1 at its second) over the band's width; ("slab",) the
    top's, by the distance across; ("brick",) the border's, by the run round its outline."""
    w = width / 2
    top = "wood" if surface == "wood" else "top"
    if safety == "gutter":
        g = min(1.0, (w - 1.5) / GUTTER[-1][0]) if w - 1.5 < GUTTER[-1][0] else 1.0
        slope = [(d * g, h) for d, h in GUTTER]
        h0 = slope[0][1]
        left = [(-w + d, h) for d, h in slope]
        pts = [(-w, -DEPTH)] + left + [(w - d, h) for d, h in reversed(slope)] + [(w, -DEPTH)]
        sw = slope[-1][0]
        edges = [("side", ("down", h0))]
        edges += [("slope", ("band", slope[k][0], slope[k + 1][0], sw)) for k in range(len(slope) - 1)]
        edges += [(top, ("slab",))]
        edges += [("slope", ("band", slope[k + 1][0], slope[k][0], sw)) for k in reversed(range(len(slope) - 1))]
        edges += [("side", ("down", h0)), ("side", ("slab",))]
        return pts, edges
    if safety == "border":
        b, h = BORDER_W, BORDER_H
        pts = [(-w, -DEPTH), (-w, h), (-w + b, h), (-w + b, 0.0), (-w + b + KERB, 0.0), (w - b - KERB, 0.0),
               (w - b, 0.0), (w - b, h), (w, h), (w, -DEPTH)]
        edges = [("side", ("down", h)), ("border", ("brick",)), ("border", ("brick",)),
                 ("shade", ("band", 0.0, KERB, KERB)), (top, ("slab",)), ("shade", ("band", KERB, 0.0, KERB)),
                 ("border", ("brick",)), ("border", ("brick",)), ("side", ("down", h)), ("side", ("slab",))]
        return pts, edges
    # railings and none: the bare floor with its kerbs (the railings stand on it, an object of their own)
    k = min(KERB, w / 3)
    pts = [(-w, -DEPTH), (-w, 0.0), (-w + k, 0.0), (w - k, 0.0), (w, 0.0), (w, -DEPTH)]
    edges = [("side", ("down", 0.0)), ("kerb", ("band", 0.0, k, KERB)), (top, ("slab",)),
             ("kerb", ("band", k, 0.0, KERB)), ("side", ("down", 0.0)), ("side", ("slab",))]
    return pts, edges


def _edge_uv(kind, a, b, run_a, run_b):
    """The texture's u at an edge's two points (see section)."""
    if kind[0] == "down":
        return (kind[1] - a[1]) / SLAB, (kind[1] - b[1]) / SLAB
    if kind[0] == "band":
        return kind[1] / kind[3], kind[2] / kind[3]
    if kind[0] == "brick":
        return run_a / BRICK, run_b / BRICK
    return a[0] / SLAB + 0.5, b[0] / SLAB + 0.5


def ear_clip(poly):
    """Triangles (index triples, counter-clockwise) of a simple polygon given either way round."""
    P = [np.asarray(q, np.float64) for q in poly]
    area = sum(P[k - 1][0] * P[k][1] - P[k][0] * P[k - 1][1] for k in range(len(P)))
    idx = list(range(len(poly))) if area > 0 else list(reversed(range(len(poly))))
    out = []

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    guard = 0
    while len(idx) > 3 and guard < 10000:
        guard += 1
        for k in range(len(idx)):
            i0, i1, i2 = idx[k - 1], idx[k], idx[(k + 1) % len(idx)]
            if cross(P[i0], P[i1], P[i2]) <= 1e-12:
                continue
            if any(j not in (i0, i1, i2) and cross(P[i0], P[i1], P[j]) >= 0 and cross(P[i1], P[i2], P[j]) >= 0
                   and cross(P[i2], P[i0], P[j]) >= 0 for j in idx):
                continue
            out.append((i0, i1, i2))
            idx.pop(k)
            break
        else:
            break
    if len(idx) == 3:
        out.append(tuple(idx))
    return out


def track_sweep(m, mats, path, widths, along, safety, surface, caps=(True, True)):
    """A stretch of track: its cross-section (section) carried along `path`, each point with its width (a width
    changing between two points tapers) and its distance along the whole track (`along`), by which every texture runs
    on from one piece into the next with no seam. Ends closed where `caps` says (a stretch meeting another is not)."""
    fr = K.frames(path)
    secs = [section(w, safety, surface) for w in widths]
    edges = secs[0][1]
    n = len(secs[0][0])
    for e in range(n):
        mat, kind = edges[e]
        vscale = BRICK if kind[0] == "brick" else (SLAB if mat == "wood" else REPEAT)
        for i in range(len(fr) - 1):
            p0, _, s0, u0 = fr[i]
            p1, _, s1, u1 = fr[i + 1]
            quad, uvs = [], []
            for (p, s_, u_, sec, v) in ((p0, s0, u0, secs[i][0], along[i]), (p1, s1, u1, secs[i + 1][0], along[i + 1])):
                a, b = np.array(sec[e]), np.array(sec[(e + 1) % n])
                run = [0.0]
                for k in range(n):
                    run.append(run[-1] + float(np.linalg.norm(np.subtract(sec[(k + 1) % n], sec[k]))))
                ua, ub = _edge_uv(kind, a, b, run[e], run[e + 1])
                quad.append((p + s_ * a[0] + u_ * a[1], p + s_ * b[0] + u_ * b[1]))
                uvs.append(((ua, v / vscale), (ub, v / vscale)))
            q = [quad[0][0], quad[0][1], quad[1][1], quad[1][0]]
            uv = [uvs[0][0], uvs[0][1], uvs[1][1], uvs[1][0]]
            a, b = np.array(secs[i][0][e]), np.array(secs[i][0][(e + 1) % n])
            d = b - a
            out2 = np.array([d[1], -d[0]]) / (np.linalg.norm(d) + 1e-12)
            nm = (s0 + s1) / 2 * out2[0] + (u0 + u1) / 2 * out2[1]
            m.face(mats[mat], q, uv, normal=nm)
    for i, sgn, cap in ((0, -1, caps[0]), (len(fr) - 1, 1, caps[1])):
        if not cap:
            continue
        p, f, s_, u_ = fr[i]
        sec = secs[i][0]
        # the outline runs clockwise seen from ahead (+f): reversed it is counter-clockwise in (across, up)
        poly = sec[::-1]
        tris = ear_clip(poly)
        ids = [m.vertex(p + s_ * q[0] + u_ * q[1], f * sgn, (q[0] / SLAB, -q[1] / SLAB)) for q in poly]
        for t in tris:
            if sgn > 0:
                m.tri(mats["side"], ids[t[0]], ids[t[1]], ids[t[2]])
            else:
                m.tri(mats["side"], ids[t[0]], ids[t[2]], ids[t[1]])


def outline_mesh(m, mats, centre, outline, top_y, bottom_y, inner=None):
    """A platform's sides and bottom: vertical faces round `outline` (points (x, z) counter-clockwise from above, about
    `centre`) from `top_y` down to `bottom_y`, the bottom closing it."""
    cx, cz = centre[0], centre[2]
    run = 0.0
    n = len(outline)
    for k in range(n):
        (x0, z0), (x1, z1) = outline[k], outline[(k + 1) % n]
        L = math.hypot(x1 - x0, z1 - z0)
        if L < 1e-9:
            continue
        q = [(cx + x0, top_y, cz + z0), (cx + x1, top_y, cz + z1), (cx + x1, bottom_y, cz + z1),
             (cx + x0, bottom_y, cz + z0)]
        h = (top_y - bottom_y) / SLAB
        uv = [(0, run / REPEAT), (0, (run + L) / REPEAT), (h, (run + L) / REPEAT), (h, run / REPEAT)]
        nm = np.array([z1 - z0, 0.0, -(x1 - x0)]) / L
        m.face(mats["side"], q, uv, normal=nm)
        run += L
    pts = [(cx + x, bottom_y, cz + z) for x, z in outline]
    tris = ear_clip([(x, z) for x, z in outline])
    ids = [m.vertex(q, (0, -1, 0), (q[0] / SLAB, q[2] / SLAB)) for q in pts]
    for t in tris:
        m.tri(mats["side"], ids[t[0]], ids[t[2]], ids[t[1]])


class LevelBuilder:
    """A level being built: its meshes, objects, materials and groups, written by `write`."""

    def __init__(self, out_dir, seed=1):
        self.out = out_dir
        self.tex = os.path.join(out_dir, "textures")
        if os.path.isdir(self.tex):
            shutil.rmtree(self.tex)
        self.meshes, self.nodes, self.materials = [], [], []
        self.groups = {}
        self.mat_ids = {}
        self.seed = seed
        self._standard_materials()

    # --- materials ---------------------------------------------------------------------------------------------

    def _standard_materials(self):
        """The track's materials: its own textures (assets/own/Track, made by make_textures.py: slabs, kerbs with the
        brick strip, shade, gutter slopes, sides; one of three sets of slabs by the level's seed), the menu scene's
        bricks for the borders, and the game's own texture set (assets/own/Textures, CC0) by the original's texture
        names for the rest, each copied into the level so it carries its own free textures; a procedural stand-in
        where the set lacks one."""
        s = self.seed
        variant = "ABC"[s % 3]
        os.makedirs(self.tex, exist_ok=True)
        for name in (f"Track_Top_{variant}", f"Track_Kerb_{variant}", f"Track_Shade_{variant}",
                     f"Track_Slope_{variant}", "Track_Side"):
            shutil.copyfile(os.path.join(TRACK_SET, name + ".png"), os.path.join(self.tex, name + ".png"))
        for name in ("floor_border.png", "floor_border_n.png"):
            shutil.copyfile(os.path.join(MENU_SET, name), os.path.join(self.tex, "Track_" + name))
        textures = {
            "Wood_Plain.png": lambda: K.planks(TEX, K.rgb(150, 104, 62), s + 4),
            "Rail_Environment.png": lambda: K.metal(TEX, K.rgb(196, 200, 210), s + 5),
            "Column_beige.png": lambda: K.stone(TEX, K.rgb(120, 112, 104), s + 6, slabs=0),
            "Floor_Top_Checkpoint.png": lambda: K.stone(TEX, K.rgb(176, 150, 96), s + 7, slabs=1, seams=0.5),
        }
        for name, make in textures.items():
            src = os.path.join(TEXTURE_SET, name)
            if os.path.isfile(src):
                shutil.copyfile(src, os.path.join(self.tex, name))
            else:
                K.save_png(self.tex, name, make())
        self.mat("top", f"Track_Top_{variant}.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("kerb", f"Track_Kerb_{variant}.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("shade", f"Track_Shade_{variant}.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("slope", f"Track_Slope_{variant}.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("side", "Track_Side.png", diffuse=0.6, emissive=0.42)
        # the borders: the menu scene's brick, with its relief
        self.mat("border", "Track_floor_border.png", diffuse=1.0, emissive=0.24, normal="Track_floor_border_n.png")
        self.mat("wood", "Wood_Plain.png", diffuse=1.0, emissive=0.25, specular=0.39, power=50)
        self.mat("rail", "Rail_Environment.png", diffuse=0.5, emissive=0.35, specular=0.8, power=10, texgen="chrome")
        self.mat("post", "Column_beige.png", diffuse=0.8, emissive=0.3)
        self.mat("plate", "Floor_Top_Checkpoint.png", diffuse=1.0, emissive=0.2, specular=0.3, power=60)

    def mat(self, name, texture=None, **kw):
        if name not in self.mat_ids:
            self.mat_ids[name] = len(self.materials)
            self.materials.append(K.material(name, texture, **kw))
        return self.mat_ids[name]

    # --- objects -----------------------------------------------------------------------------------------------

    def add(self, name, mesh, matrix=None, visible=True, groups=()):
        """An object (with its mesh, or a frame when None) in `groups`; returns its index."""
        mi = None
        if mesh is not None:
            mesh.name = f"{name}_Mesh"
            self.meshes.append(mesh)
            mi = len(self.meshes) - 1
        self.nodes.append(K.node(name, mi, matrix, visible))
        i = len(self.nodes) - 1
        for g in groups:
            self.groups.setdefault(g, []).append(i)
        return i

    def placeholder(self, name, groups, matrix, lo=(-1, -1, -1), hi=(1, 1, 1), mesh=None):
        """A part's stand-in: a mesh without any material, never drawn (the game puts the part there). For a box, a
        dome or a loose ball its mesh is the body's shape for runs recorded in replay format 4 and before (from 5 the
        body is the part file's `_MF` object, as in the original), so it is given as `mesh`."""
        if mesh is None:
            mesh = K.Mesh(name)
            K.box(mesh, None, lo, hi)
        return self.add(name, mesh, matrix, True, groups)

    @staticmethod
    def sector(n):
        return [f"Sector_{n:02d}"] if n else []

    # --- track -------------------------------------------------------------------------------------------------

    def track(self, name, path, widths, along, safety="border", surface="stone", caps=(True, True)):
        """A stretch of track along a path of points (its top's centre line), each with its width and its distance
        along the whole track (track_sweep), with the railings when its safety is railings."""
        if safety not in SAFETY:
            raise ValueError(f"safety is one of {', '.join(SAFETY)}")
        m = K.Mesh(name)
        track_sweep(m, self.mat_ids, path, widths, along, safety, surface, caps)
        i = self.add(name, m, groups=self._floor_groups("wood" if surface == "wood" else "stone"))
        if safety == "railings":
            self.railings(f"{name}_Railing", path, widths)
        return i

    def railings(self, name, path, widths):
        """Steel railings along both edges of a stretch: two rails on posts (RAIL_HEIGHTS over the floor), solid to
        the ball (the original's railings are in the rails' physics group too)."""
        m = K.Mesh(name)
        fr = K.frames(path)
        rail = self.mat_ids["rail"]
        for sgn in (-1.0, 1.0):
            off = [sgn * (w / 2 - RAIL_IN) for w in widths]
            for h in RAIL_HEIGHTS:
                K.tube(m, rail, [p + s_ * o + u * h for (p, f, s_, u), o in zip(fr, off)], RAIL_R, caps=True)
            # posts at the ends and about every POST_EVERY between
            seg = [float(np.linalg.norm(fr[k + 1][0] - fr[k][0])) for k in range(len(fr) - 1)]
            total = sum(seg)
            count = max(1, int(round(total / POST_EVERY)))
            for j in range(count + 1):
                t = total * j / count
                k = 0
                while k < len(seg) - 1 and t > seg[k]:
                    t -= seg[k]
                    k += 1
                c = t / seg[k] if seg[k] > 0 else 0.0
                p = fr[k][0] * (1 - c) + fr[k + 1][0] * c
                s_ = fr[k][2] * (1 - c) + fr[k + 1][2] * c
                u = fr[k][3] * (1 - c) + fr[k + 1][3] * c
                o = off[k] * (1 - c) + off[k + 1] * c
                base = p + s_ * o
                K.tube(m, rail, [base, base + u * (RAIL_HEIGHTS[-1] + RAIL_R)], RAIL_R * 0.8, caps=True)
        return self.add(name, m, groups=self._floor_groups("metal"))

    def slab(self, name, lo, hi, mat="top", sound="stone", sector=0):
        """A plain block of floor (pads, platforms) from its low to its high corner."""
        m = K.Mesh(name)
        K.box(m, self.mat_ids[mat], lo, hi, scale=SLAB)
        return self.add(name, m, groups=self._floor_groups(sound) + self.sector(sector))

    def fan_housing(self, name, at):
        """A low steel frame round a fan on the floor at `at`, only for the look (the ball rolls through it)."""
        x, y, z = at
        m = K.Mesh(name)
        for lo, hi in (((-2.2, -2.2), (2.2, -1.8)), ((-2.2, 1.8), (2.2, 2.2)), ((-2.2, -1.8), (-1.8, 1.8)),
                       ((1.8, -1.8), (2.2, 1.8))):
            K.box(m, self.mat_ids["rail"], (x + lo[0], y, z + lo[1]), (x + hi[0], y + 0.3, z + hi[1]))
        return self.add(name, m)

    def stopper(self, name, lo, hi):
        """An unseen floor stopper (Phys_FloorStopper): it holds boxes and modules' parts, the ball passes it."""
        m = K.Mesh(name)
        K.box(m, None, lo, hi)
        return self.add(name, m, visible=False, groups=["Phys_FloorStopper"])

    def _floor_groups(self, sound):
        sid = {"stone": "01", "wood": "02", "metal": "03"}[sound]
        phys = "Phys_FloorRails" if sound == "metal" else "Phys_Floors"
        return [phys, f"Sound_HitID_{sid}", f"Sound_RollID_{sid}", "Shadow"]

    def rails(self, name, path, gauge=2.4, r=0.3, posts=6.0, sector=0):
        """Two steel tubes the ball rolls on, `gauge` apart, with short posts under them."""
        m = K.Mesh(name)
        fr = K.frames(path)
        for side in (-gauge / 2, gauge / 2):
            K.tube(m, self.mat_ids["rail"], [p + s * side for p, f, s, u in fr], r, caps=True)
        run = 0.0
        for i in range(1, len(fr)):
            run += float(np.linalg.norm(fr[i][0] - fr[i - 1][0]))
            if run >= posts:
                run = 0.0
                p, f, s, u = fr[i]
                K.tube(m, self.mat_ids["rail"], [p - s * gauge / 2 - u * r, p + s * gauge / 2 - u * r], r * 0.7)
        return self.add(name, m, groups=self._floor_groups("metal") + self.sector(sector))

    def bowl_piece(self, name, centre, fwd, bowl, outline, cups):
        """A platform with a bowl in its middle (the start, a checkpoint): its top at `centre` (the floor's height round
        the bowl), the bowl's profile `bowl` ((radius, height over its bottom) from its middle out), the platform's
        outline `outline(angle)` (its distance from the middle, the angle from `fwd` towards the left) with the
        angles its corners lie at (`outline.corners`), flame cups at `cups` ((ahead, left) about the middle). The
        bowl is drawn with the checkpoint's floor (its rings round a grate at its middle, the outer ring at its rim),
        the platform round it with the track's slabs."""
        c = np.asarray(centre, np.float64)
        f = np.asarray(fwd, np.float64)
        f = f / np.linalg.norm(f)
        left = np.array([-f[2], 0.0, f[0]])
        up = np.array([0.0, 1.0, 0.0])
        rb, deep = bowl[-1][0], bowl[-1][1]
        bottom = c - up * deep
        tex = rb * 64 / 62  # the texture's half side, its outer ring's edge (62 of 64) at the rim
        m = K.Mesh(name)
        plate, top = self.mat_ids["plate"], self.mat_ids["top"]
        angles = sorted(set([2 * math.pi * k / 64 for k in range(64)] + [a % (2 * math.pi) for a in outline.corners]))

        def at(r, a, h):
            return bottom + f * (r * math.cos(a)) + left * (r * math.sin(a)) + up * h

        def uv_bowl(r, a):
            return (0.5 + r * math.cos(a) / (2 * tex), 0.5 + r * math.sin(a) / (2 * tex))

        def uv_top(r, a):
            return (r * math.sin(a) / SLAB + 0.5, r * math.cos(a) / REPEAT)

        for k in range(len(angles)):
            a0, a1 = angles[k], angles[(k + 1) % len(angles)]
            for j in range(len(bowl) - 1):
                (r0, h0), (r1, h1) = bowl[j], bowl[j + 1]
                q = [at(r0, a0, h0), at(r1, a0, h1), at(r1, a1, h1), at(r0, a1, h0)]
                uv = [uv_bowl(r0, a0), uv_bowl(r1, a0), uv_bowl(r1, a1), uv_bowl(r0, a1)]
                if r0 < 1e-9:
                    q, uv = [q[0], q[1], q[2]], [uv[0], uv[1], uv[2]]
                nrm = np.cross(q[1] - q[0], q[2] - q[0])
                if np.linalg.norm(nrm) < 1e-12:
                    continue
                m.face(plate, q, uv, normal=nrm if nrm[1] >= 0 else -nrm)
            ra0, ra1 = outline(a0), outline(a1)
            if ra0 > rb + 1e-6 or ra1 > rb + 1e-6:
                q = [at(rb, a0, deep), at(ra0, a0, deep), at(ra1, a1, deep), at(rb, a1, deep)]
                uv = [uv_top(rb, a0), uv_top(ra0, a0), uv_top(ra1, a1), uv_top(rb, a1)]
                m.face(top, q, uv, normal=up)
        # the sides and the bottom round the outline, in (ahead, left) about the middle, turned into the world's
        pts = [(outline(a) * math.cos(a), outline(a) * math.sin(a)) for a in angles]
        world = [(f * x + left * z) for x, z in pts]
        outline_mesh(m, self.mat_ids, (c[0], 0.0, c[2]), [(w[0], w[2]) for w in world], c[1], c[1] - START_DEEP)
        # the flame cups: a column each, its top a little over the floor with the checkpoint's rings on it
        for ahead, side in cups:
            p = c + f * ahead + left * side
            K.cylinder(m, self.mat_ids["side"], p - up * (START_DEEP - 1.0), up, CUP_R, START_DEEP - 1.0 + CUP_UP,
                       seg=24, cap_mat=plate, vscale=SLAB)
        self.add(name, m, groups=self._floor_groups("stone"))
        return bottom

    def start(self, at, width=14.0):
        """The start: a round platform with a bowl in its middle the ball sits in, four flame cups round it and a neck
        as wide as the track leaving along +X (to 10 ahead), the stand-in of PS_FourFlames in the bowl's bottom and
        the first reset point over it. `at` is the platform's top centre (the floor's height round the bowl).
        Returns the reset point."""
        hw = min(width, 2 * START_RADIUS) / 2

        def outline(a):
            ca, sa = math.cos(a), abs(math.sin(a))
            if ca <= 1e-9:
                return START_RADIUS
            neck = min(START_RADIUS / ca, hw / sa if sa > 1e-9 else 1e9)
            return max(START_RADIUS, neck)

        corner = math.atan2(hw, START_RADIUS)
        meet = math.asin(min(1.0, hw / START_RADIUS))
        outline.corners = [corner, -corner, meet, -meet]
        bottom = self.bowl_piece("Start_Platform", at, (1, 0, 0), START_BOWL, outline,
                                 [(sa * 6.1, sl * 7.3) for sa in (-1, 1) for sl in (-1, 1)])
        # its z axis points back at the camera (-X): x axis (0, 0, 1)
        self.placeholder("PS_FourFlames_01", ["PS_Levelstart"], K.pose(tuple(bottom), (0, 0, 1)), (-8, 0, -7),
                         (8, 6, 7))
        reset = tuple(bottom + np.array([0.0, START_RESET, 0.0]))
        self.reset(1, reset)
        return reset

    def reset(self, n, point, xaxis=(0, 0, 1)):
        """Reset point n at `point` (where the ball comes back)."""
        self.placeholder(f"PR_Resetpoint_{n:02d}", ["PR_Resetpoints"], K.pose(point, xaxis), (-2, -2, -2), (2, 2, 2))

    def column(self, name, top, depth=40.0, r=2.2):
        """A stone column hanging under the floor from `top` down `depth`: only for the look (no physics)."""
        x, y, z = top
        m = K.Mesh(name)
        K.cylinder(m, self.mat_ids["post"], (x, y - DEPTH - depth, z), (0, 1, 0), r, depth, seg=12)
        K.cylinder(m, self.mat_ids["border"], (x, y - DEPTH - 1.2, z), (0, 1, 0), r * 1.35, 1.2, seg=12)
        return self.add(name, m)

    def checkpoint(self, n, at, width=14.0, fwd=(1, 0, 0)):
        """Checkpoint n (1, 2...) at `at` (the floor's top, the track along `fwd`, a multiple of 90 degrees): a piece
        of floor CHECK_LEN long and as wide as the track with the checkpoint's bowl in its middle, two flame cups
        across the way (at ±7 to the sides, as the original's flames), the stand-in of PC_TwoFlames in the bowl's
        bottom (its z axis across the way, where its flames stand) and the next sector's reset point over it. The
        floors before and after end at the piece. Returns the reset point."""
        f = np.asarray(fwd, np.float64)
        f /= np.linalg.norm(f)
        left = np.array([-f[2], 0.0, f[0]])
        hw, hl = width / 2, CHECK_LEN / 2
        rb = min(CHECK_BOWL[-1][0], hw - 0.5)
        bowl = [(r * rb / CHECK_BOWL[-1][0], h) for r, h in CHECK_BOWL]

        def outline(a):
            ca, sa = abs(math.cos(a)), abs(math.sin(a))
            return min(hl / ca if ca > 1e-9 else 1e9, hw / sa if sa > 1e-9 else 1e9)

        corner = math.atan2(hw, hl)
        outline.corners = [corner, math.pi - corner, math.pi + corner, -corner]
        bottom = self.bowl_piece(f"Checkpoint_{n:02d}", at, f, bowl, outline,
                                 [(0.0, -CHECK_FLAMES), (0.0, CHECK_FLAMES)])
        self.placeholder(f"PC_TwoFlames_{n:02d}", ["PC_Checkpoints"], K.pose(tuple(bottom), tuple(f)), (-4, 0, -8),
                         (4, 6, 8))
        reset = tuple(bottom + np.array([0.0, CHECK_RESET, 0.0]))
        self.reset(n + 1, reset, tuple(left))
        return reset

    def finish(self, at, sector, xaxis=(-1, 0, 0)):
        """The finish: the balloon's stand-in (PE_Balloon, group PE_Levelende) with its platform at `at`; its bridge
        of plates reaches back along -X from 9.5 to 21 (the part's measured plates), so the track ends 22 before."""
        self.placeholder("PE_Balloon_01", ["PE_Levelende"] + self.sector(sector), K.pose(at, xaxis), (-4, -1, -4),
                         (4, 1, 4))

    # --- parts -------------------------------------------------------------------------------------------------

    def box(self, n, at, sector):
        """A wooden box (P_Box): its body is its stand-in's mesh, a cube of 5 standing on `at`."""
        x, y, z = at
        m = K.Mesh("box")
        K.box(m, None, (-2.5, -2.5, -2.5), (2.5, 2.5, 2.5))
        return self.placeholder(f"P_Box_{n:02d}", ["P_Box"] + self.sector(sector), K.pose((x, y + 2.5, z)), mesh=m)

    def dome(self, n, at, sector):
        """A dome bumper (P_Dome): a low spherical cap 13.3 across, 3.55 high, the body of runs recorded in replay
        format 4 and before; from format 5 the body is the part's P_Dome_MF (13.3 across, 0.12 to 3.68 high)."""
        w, h = 13.29 / 2, 3.55
        rad = (w * w + h * h) / (2 * h)
        m = K.Mesh("dome")
        seg, rings = 16, 4
        pts = []
        for i in range(rings + 1):
            a = math.asin(w / rad) * (1 - i / rings)
            rr, yy = rad * math.sin(a), rad * math.cos(a) - (rad - h)
            ring = [(rr * math.cos(2 * math.pi * k / seg), yy, rr * math.sin(2 * math.pi * k / seg))
                    for k in range(seg if rr > 1e-6 else 1)]
            pts.append(ring)
        for i in range(rings):
            lo, hi = pts[i], pts[i + 1]
            for k in range(seg):
                a, b = lo[k], lo[(k + 1) % seg]
                if len(hi) == 1:
                    m.face(None, [a, hi[0], b])
                else:
                    m.face(None, [a, hi[k], hi[(k + 1) % seg], b])
        m.face(None, pts[0])
        return self.placeholder(f"P_Dome_{n:02d}", ["P_Dome"] + self.sector(sector), K.pose(at), mesh=m)

    def loose_ball(self, kind, n, at, sector):
        """A loose ball (P_Ball_Wood, _Stone, _Paper) lying on `at`; the wooden and stone ones are spheres of radius
        2 to the physics, the paper one the hull of its stand-in, a round ball of the same size."""
        x, y, z = at
        m = K.Mesh("ball")
        K.sphere(m, None, (0, 0, 0), 2.0, seg=12)
        name = f"P_Ball_{kind}"
        return self.placeholder(f"{name}_{n:02d}", [name] + self.sector(sector), K.pose((x, y + 2.0, z)), mesh=m)

    def trafo(self, kind, n, at, sector):
        """A transformer (P_Trafo_Wood, _Stone, _Paper): its pad lies on `at`."""
        x, y, z = at
        name = f"P_Trafo_{kind}"
        return self.placeholder(f"{name}_{n:02d}", [name] + self.sector(sector), K.pose((x, y + 0.1, z)),
                                (-2.5, -0.1, -2.5), (2.5, 2.4, 2.5))

    def extra_point(self, n, at, sector):
        x, y, z = at
        return self.placeholder(f"P_Extra_Point_{n:02d}", ["P_Extra_Point"] + self.sector(sector),
                                K.pose((x, y + 2.5, z)), (-2.5, -2.5, -2.5), (2.5, 2.5, 2.5))

    def extra_life(self, n, at, sector):
        x, y, z = at
        return self.placeholder(f"P_Extra_Life_{n:02d}", ["P_Extra_Life"] + self.sector(sector),
                                K.pose((x, y + 2.56, z)), (-2.5, -2.56, -2.5), (2.5, 1.2, 2.5))

    def module(self, name, n, matrix, sector, lo=(-2, -2, -2), hi=(2, 2, 2)):
        """A module (P_Modul_NN) at `matrix`, the frame its part is made in."""
        return self.placeholder(f"{name}_{n:02d}", [name] + self.sector(sector), matrix, lo, hi)

    # --- writing -----------------------------------------------------------------------------------------------

    def write(self, title, author, sky, start_ball, music, light=(1.0, 1.0, 1.0), bonus=100, licence=None,
              extra=None):
        """Writes the level and its level.txt (with `extra` keys besides title, author, version, licence)."""
        groups = sorted(self.groups.items())
        K.write_open(self.out, self.meshes, self.nodes, self.materials, groups, sky=sky, start_ball=start_ball,
                     settings={"music": music, "light": list(light), "level_bonus": bonus, "tutorial": False,
                               "last_level": False, "parts": "own"}, generator="drahtseilakt levelkit.py")
        with open(os.path.join(self.out, "level.txt"), "w") as f:
            f.write(f"title = {title}\nauthor = {author}\nversion = 1\n")
            if licence:
                f.write(f"licence = {licence}\n")
            for k, v in (extra or {}).items():
                f.write(f"{k} = {v}\n")
