#!/usr/bin/env python3
"""Builds the game's own levels in the open format (docs/drahtseilakt/open-format.md) from track pieces: floors with
borders, rails, the start with its four flame posts, checkpoints with their two posts, the finish's balloon, and the
stand-ins of the parts (boxes, domes, loose balls, transformers, extra points and lives, the modules), each in the
groups the game's rules read (docs/drahtseilakt/nmo-format.md, "What the game reads from levels"). A level made here
names only our own parts: the game takes them from assets/own/ (resources.md), so it plays with no copy of the
original at all.

The game's coordinates (Virtools: left-handed, Y up). The camera starts looking along +X, so a level runs along +X
from its start. Floors' tops are at the heights given; a stand-in's origin is where its part stands.

Sizes measured on the original (numbers only, nothing of its geometry): the ball's radius 2, the reset point 3.5 above
the start's and checkpoints' floor, the flames of the start at (±7.3, 2.05, ±6.1) and of a checkpoint at
(0, 2.05, ±7) about its stand-in (the parts PS_FourFlames and PC_TwoFlames hold those frames), a box 5 on a side, a
dome 13.3 across and 3.55 high, a transformer's pad 5 across.

Needs Python 3 with numpy and Pillow; ownkit.py beside it.
"""

import math
import os
import shutil

import numpy as np

import ownkit as K

TEX = 256
# The game's own texture set (CC0), by the original's texture names.
TEXTURE_SET = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "Textures")
# The track's cross-section (across, up): its top at 0, borders 0.8 high and 1 wide, 2.5 deep.
BORDER_H, BORDER_W, DEPTH = 0.8, 1.0, 2.5
# Where the ball comes back after a fall: this far above the start's or a checkpoint's floor.
RESET_UP = 3.5


def profile(width, borders=True):
    """A floor's cross-section: points (across, up) and the material of each edge (top, border, side)."""
    w = width / 2
    if not borders:
        return [(-w, -DEPTH), (-w, 0.0), (w, 0.0), (w, -DEPTH)], ["side", "top", "side", "side"]
    b = w - BORDER_W
    pts = [(-w, -DEPTH), (-w, BORDER_H), (-b, BORDER_H), (-b, 0.0), (b, 0.0), (b, BORDER_H), (w, BORDER_H),
           (w, -DEPTH)]
    return pts, ["side", "border_top", "border", "top", "border", "border_top", "side", "side"]


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
        """The track's materials: the game's own texture set (assets/own/Textures, CC0, made by make_textures.py) by
        the original's texture names, copied into the level so it carries its own free textures; a procedural
        stand-in where the set lacks one."""
        s = self.seed
        textures = {
            "Floor_Top_Flat.png": lambda: K.stone(TEX, K.rgb(158, 142, 116), s + 1, slabs=2, seams=0.25),
            "Floor_Top_Borderless.png": lambda: K.stone(TEX, K.rgb(128, 84, 62), s + 2, slabs=0, mottle=0.2),
            "Floor_Side.png": lambda: K.stone(TEX, K.rgb(150, 128, 104), s + 3, slabs=4, seams=0.15),
            "Wood_Plain.png": lambda: K.planks(TEX, K.rgb(150, 104, 62), s + 4),
            "Rail_Environment.png": lambda: K.metal(TEX, K.rgb(196, 200, 210), s + 5),
            "Column_beige.png": lambda: K.stone(TEX, K.rgb(120, 112, 104), s + 6, slabs=0),
            "Floor_Top_Checkpoint.png": lambda: K.stone(TEX, K.rgb(176, 150, 96), s + 7, slabs=1, seams=0.5),
        }
        for name, make in textures.items():
            src = os.path.join(TEXTURE_SET, name)
            if os.path.isfile(src):
                os.makedirs(self.tex, exist_ok=True)
                shutil.copyfile(src, os.path.join(self.tex, name))
            else:
                K.save_png(self.tex, name, make())
        self.mat("top", "Floor_Top_Borderless.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("border", "Floor_Top_Flat.png", diffuse=1.0, emissive=0.24)
        self.mat("border_top", "Floor_Top_Flat.png", diffuse=1.0, emissive=0.28)
        self.mat("side", "Floor_Side.png", diffuse=0.6, emissive=0.42)
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
        dome or a loose ball its mesh is the body's shape, so it is given as `mesh`."""
        if mesh is None:
            mesh = K.Mesh(name)
            K.box(mesh, None, lo, hi)
        return self.add(name, mesh, matrix, True, groups)

    @staticmethod
    def sector(n):
        return [f"Sector_{n:02d}"] if n else []

    # --- track -------------------------------------------------------------------------------------------------

    def floor(self, name, path, width, borders=True, sound="stone", sector=0):
        """A floor along a path of points (its top's centre line), with its borders."""
        pts, edge = profile(width, borders)
        m = K.Mesh(name)
        K.sweep(m, path, pts, [self.mat_ids[e] for e in edge], uscale=8.0, vscale=8.0, end_mat=self.mat_ids["side"])
        return self.add(name, m, groups=self._floor_groups(sound) + self.sector(sector))

    def slab(self, name, lo, hi, mat="top", sound="stone", sector=0):
        """A plain block of floor (pads, platforms) from its low to its high corner."""
        m = K.Mesh(name)
        K.box(m, self.mat_ids[mat], lo, hi, scale=8.0)
        return self.add(name, m, groups=self._floor_groups(sound) + self.sector(sector))

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

    def post(self, m, base, height=1.6, r=0.45):
        """A stone flame post: a column with a bowl on top, where the flames burn (their frames are the part's)."""
        bx, by, bz = base
        K.cylinder(m, self.mat_ids["post"], (bx, by, bz), (0, 1, 0), r * 1.3, 0.25, seg=12)
        K.cylinder(m, self.mat_ids["post"], (bx, by + 0.25, bz), (0, 1, 0), r, height - 0.25, seg=12)
        K.cylinder(m, self.mat_ids["border"], (bx, by + height, bz), (0, 1, 0), r * 0.7, 0.45, seg=12,
                   r_top=r * 1.4)

    # --- the rules' places -------------------------------------------------------------------------------------

    def start(self, at):
        """The start: a square platform with four flame posts, the stand-in of PS_FourFlames and the first reset
        point. `at` is the platform's top centre; the ball rolls off along +X."""
        x, y, z = at
        self.slab("Start_Platform", (x - 10, y - DEPTH, z - 9), (x + 10, y, z + 9), mat="plate")
        m = K.Mesh("Start_Posts")
        for sx in (-7.3, 7.3):
            for sz in (-6.1, 6.1):
                self.post(m, (x + sx, y, z + sz), height=1.65)
        self.add("Start_Posts", m, groups=["Shadow"])
        # its z axis points back at the camera (-X): x axis (0, 0, 1)
        self.placeholder("PS_FourFlames_01", ["PS_Levelstart"], K.pose(at, (0, 0, 1)), (-8, 0, -7), (8, 6, 7))
        self.reset(1, at)

    def reset(self, n, at):
        x, y, z = at
        self.placeholder(f"PR_Resetpoint_{n:02d}", ["PR_Resetpoints"], K.pose((x, y + RESET_UP, z), (0, 0, 1)),
                         (-2, -2, -2), (2, 2, 2))

    def checkpoint(self, n, at, width=14.0):
        """Checkpoint n (1, 2...): two flame posts across the track at `at` (the floor's top), a plate between them,
        the stand-in of PC_TwoFlames, and the next sector's reset point."""
        x, y, z = at
        m = K.Mesh(f"Checkpoint_{n:02d}")
        for sz in (-7.0, 7.0):
            self.post(m, (x, y, z + sz), height=1.65)
        K.cylinder(m, self.mat_ids["plate"], (x, y - 0.02, z), (0, 1, 0), 2.6, 0.06, seg=24)
        self.add(f"Checkpoint_{n:02d}", m, groups=["Shadow"])
        self.placeholder(f"PC_TwoFlames_{n:02d}", ["PC_Checkpoints"], K.pose(at, (0, 0, 1)), (-4, 0, -8), (4, 6, 8))
        self.reset(n + 1, at)

    def finish(self, at, sector):
        """The finish: the balloon's stand-in (PE_Balloon, group PE_Levelende) with its platform at `at`; its bridge
        of plates reaches back along -X from 9.5 to 21 (the part's measured plates), so the track ends 22 before."""
        self.placeholder("PE_Balloon_01", ["PE_Levelende"] + self.sector(sector), K.pose(at, (-1, 0, 0)), (-4, -1, -4),
                         (4, 1, 4))

    # --- parts -------------------------------------------------------------------------------------------------

    def box(self, n, at, sector):
        """A wooden box (P_Box): its body is its stand-in's mesh, a cube of 5 standing on `at`."""
        x, y, z = at
        m = K.Mesh("box")
        K.box(m, None, (-2.5, -2.5, -2.5), (2.5, 2.5, 2.5))
        return self.placeholder(f"P_Box_{n:02d}", ["P_Box"] + self.sector(sector), K.pose((x, y + 2.5, z)), mesh=m)

    def dome(self, n, at, sector):
        """A dome bumper (P_Dome): its body is its stand-in's mesh, a low spherical cap 13.3 across, 3.55 high."""
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

    def write(self, title, author, sky, start_ball, music, light=(1.0, 1.0, 1.0), bonus=100, licence=None):
        groups = sorted(self.groups.items())
        K.write_open(self.out, self.meshes, self.nodes, self.materials, groups, sky=sky, start_ball=start_ball,
                     settings={"music": music, "light": list(light), "level_bonus": bonus, "tutorial": False,
                               "last_level": False, "parts": "own"}, generator="drahtseilakt levelkit.py")
        with open(os.path.join(self.out, "level.txt"), "w") as f:
            f.write(f"title = {title}\nauthor = {author}\nversion = 1\n")
            if licence:
                f.write(f"licence = {licence}\n")
