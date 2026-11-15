#!/usr/bin/env python3
"""Shared toolkit of the scripts that make the game's own content in the open level format
(docs/drahtseilakt/open-format.md): noise and texture helpers, a mesh builder with primitives, and the writer of a
level folder (level.gltf, level.bin, textures/) exactly as crates/level/src/open.rs reads it.

Coordinates here are the game's (Virtools: left-handed, Y up), as the game's code and the level's matrices use them;
the writer turns them into glTF's right-handed ones (z negated, every triangle's winding reversed, matrices S·M·S with
S = diag(1, 1, -1, 1)), which the reader turns back, so what is written here is what the game sees.

Matrices are numpy 4×4 in the usual column-vector convention (M @ [x, y, z, 1]); `pose()` builds one from a position,
two axes and a scale.

Needs Python 3 with numpy and Pillow. Nothing here reads any file of the original game.
"""

import json
import math
import os
import struct

import numpy as np
from PIL import Image

# --- noise and colour --------------------------------------------------------------------------------------------


def value_noise(h, w, cells, seed, octaves=5):
    """Fractal value noise in 0..1 that tiles seamlessly (the lattice wraps)."""
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
    """A colour from 0..255 components as 0..1 floats."""
    return np.array(v, np.float64) / 255.0


def save_png(folder, name, img, alpha=None):
    """Saves an image (h × w × 3 floats in 0..1, optional h × w alpha) as `folder/name`."""
    a = np.clip(img * 255 + 0.5, 0, 255).astype(np.uint8)
    mode = "RGB"
    if alpha is not None:
        a = np.concatenate([a, np.clip(alpha * 255 + 0.5, 0, 255).astype(np.uint8)[..., None]], -1)
        mode = "RGBA"
    os.makedirs(folder, exist_ok=True)
    Image.fromarray(a, mode).save(os.path.join(folder, name), optimize=True)


def stone(size, base, seed, slabs=2, mottle=0.16, grain=0.07, seams=0.35):
    """A stone surface: mottled base colour, fine grain, optional slab seams."""
    n = value_noise(size, size, (4, 4), seed)
    f = value_noise(size, size, (64, 64), seed + 1, octaves=2)
    img = base[None, None, :] * (1 - mottle / 2 + mottle * n)[..., None] * (1 - grain / 2 + grain * f)[..., None]
    if slabs:
        k = size // slabs
        y, x = np.mgrid[0:size, 0:size]
        seam = np.minimum(np.minimum(y % k, k - 1 - y % k), np.minimum(x % k, k - 1 - x % k))
        img = img * (1 - seams * np.exp(-seam / 1.5))[..., None]
    return np.clip(img, 0, 1)


def planks(size, base, seed, boards=4, gaps=0.6):
    """Wooden boards along x with grain and dark gaps between them."""
    y, x = np.mgrid[0:size, 0:size]
    k = size // boards
    board = np.minimum(y // k, boards - 1)
    rng = np.random.default_rng(seed)
    shade = rng.uniform(0.82, 1.12, boards)[board]
    grain = value_noise(size, size, (24, 2), seed + 1, octaves=3)
    rings = 0.5 + 0.5 * np.sin(grain * 40.0 + y * 0.3)
    img = base[None, None, :] * (shade * (0.85 + 0.2 * rings))[..., None]
    gap = np.minimum(y % k, k - 1 - y % k)
    img = img * (1 - gaps * np.exp(-gap / 1.0))[..., None]
    return np.clip(img, 0, 1)


def metal(size, base, seed, brushed=0.08):
    """Brushed metal: streaks along x and a soft mottle."""
    s = value_noise(size, size, (64, 1), seed, octaves=3)
    n = value_noise(size, size, (3, 3), seed + 1)
    img = base[None, None, :] * (1 - brushed + 2 * brushed * s)[..., None] * (0.92 + 0.12 * n)[..., None]
    return np.clip(img, 0, 1)


def env_map(size, top, horizon, bottom, seed):
    """A reflection map (for texgen reflect/chrome): a sky above, a bright band, a dark ground, a little noise."""
    y = np.linspace(0, 1, size)[:, None]
    up = np.clip(1 - 2 * y, 0, 1)
    down = np.clip(2 * y - 1, 0, 1)
    mid = 1 - up - down
    img = top * up[..., None] + horizon * mid[..., None] + bottom * down[..., None]
    img = np.broadcast_to(img, (size, size, 3)).copy()
    img *= (0.94 + 0.12 * value_noise(size, size, (4, 4), seed))[..., None]
    return np.clip(img, 0, 1)


# --- matrices ----------------------------------------------------------------------------------------------------


def pose(pos=(0, 0, 0), xaxis=(1, 0, 0), yaxis=(0, 1, 0), scale=(1, 1, 1)):
    """A world matrix from a position, the object's x and y axes (made orthonormal) and its scale."""
    x = np.asarray(xaxis, np.float64)
    x /= np.linalg.norm(x)
    y = np.asarray(yaxis, np.float64)
    y = y - x * np.dot(x, y)
    y /= np.linalg.norm(y)
    z = np.cross(x, y)
    s = np.broadcast_to(np.asarray(scale, np.float64), (3,))
    m = np.eye(4)
    m[:3, 0], m[:3, 1], m[:3, 2] = x * s[0], y * s[1], z * s[2]
    m[:3, 3] = pos
    return m


def rot_y(deg):
    """A turn about the vertical axis as axes for pose(): (xaxis, yaxis)."""
    a = math.radians(deg)
    return (math.cos(a), 0.0, -math.sin(a)), (0.0, 1.0, 0.0)


# --- meshes ------------------------------------------------------------------------------------------------------


class Mesh:
    """A mesh: vertices with normal and texture coordinates, triangles by material index (None: no material, for
    collision meshes the game never draws)."""

    def __init__(self, name):
        self.name = name
        self.p, self.n, self.uv = [], [], []
        self.parts = {}
        # what is drawn instead of this mesh (another Mesh), when its shape is the physics' (open-format.md, a mesh's
        # finer look): the game draws the look and gives the physics and the rules this one
        self.look = None
        # material channels (open-format.md): further passes over the whole mesh, each (material index, (source,
        # destination) blend factor names, a texture coordinate per vertex), drawn over the parts
        self.channels = []

    def channel(self, mat, blend, uvs):
        """One more pass over the whole mesh with material `mat`, blended by the pair of factor names `blend`, with
        its own texture coordinates `uvs` (one per vertex, or a function of a vertex's (position, normal, uv))."""
        if callable(uvs):
            uvs = [uvs(np.asarray(p), np.asarray(n), uv) for p, n, uv in zip(self.p, self.n, self.uv)]
        self.channels.append((mat, tuple(blend), [tuple(float(c) for c in uv) for uv in uvs]))

    def vertex(self, p, n, uv):
        self.p.append(tuple(float(c) for c in p))
        self.n.append(tuple(float(c) for c in n))
        self.uv.append(tuple(float(c) for c in uv))
        return len(self.p) - 1

    def tri(self, mat, a, b, c):
        self.parts.setdefault(mat, []).extend([a, b, c])

    def face(self, mat, pts, uvs=None, normal=None):
        """A flat convex polygon (as a fan), its normal from its points unless given; winding as given."""
        pts = [np.asarray(q, np.float64) for q in pts]
        if normal is None:
            normal = np.zeros(3)
            for i in range(len(pts)):
                normal += np.cross(pts[i] - pts[0], pts[(i + 1) % len(pts)] - pts[0])
        normal = np.asarray(normal, np.float64)
        normal = normal / (np.linalg.norm(normal) + 1e-12)
        if uvs is None:
            uvs = planar_uv(pts, normal)
        ids = [self.vertex(q, normal, u) for q, u in zip(pts, uvs)]
        for k in range(1, len(ids) - 1):
            self.tri(mat, ids[0], ids[k], ids[k + 1])

    def quad(self, mat, pts, uvs=None, normal=None):
        self.face(mat, pts, uvs, normal)

    def transformed(self, m):
        """Applies a matrix to everything added so far (normals by its inverse transpose)."""
        m = np.asarray(m, np.float64)
        nm = np.linalg.inv(m[:3, :3]).T
        self.p = [tuple(m[:3, :3] @ np.array(p) + m[:3, 3]) for p in self.p]
        out = []
        for n in self.n:
            v = nm @ np.array(n)
            out.append(tuple(v / (np.linalg.norm(v) + 1e-12)))
        self.n = out
        return self

    def bounds(self):
        a = np.array(self.p)
        return a.min(0), a.max(0)


def planar_uv(pts, normal, scale=4.0):
    """Texture coordinates projected along the normal's main axis, `scale` units a repeat."""
    k = int(np.argmax(np.abs(normal)))
    a, b = [(1, 2), (0, 2), (0, 1)][k]
    return [(q[a] / scale, q[b] / scale) for q in pts]


def add(dst, src, m=None):
    """Adds the triangles of mesh `src` (optionally moved by matrix m) to mesh `dst`."""
    m = np.eye(4) if m is None else np.asarray(m, np.float64)
    nm = np.linalg.inv(m[:3, :3]).T
    base = len(dst.p)
    for p, n, uv in zip(src.p, src.n, src.uv):
        v = nm @ np.array(n)
        dst.vertex(m[:3, :3] @ np.array(p) + m[:3, 3], v / (np.linalg.norm(v) + 1e-12), uv)
    for mat, idx in src.parts.items():
        dst.parts.setdefault(mat, []).extend(i + base for i in idx)
    return dst


def hexahedron(m, mat, c, scale=4.0):
    """A solid of eight corners: c[0..3] the bottom (x-, z- going round), c[4..7] the top above them. Faces are flat
    when the corners make them so (boxes, frustums, wedges with a zero edge are fine)."""
    c = [np.asarray(q, np.float64) for q in c]
    centre = sum(c) / 8
    faces = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
    for f in faces:
        pts = [c[i] for i in f]
        # drop repeated corners (a wedge's edge of zero length)
        uniq = []
        for p in pts:
            if not any(np.allclose(p, q, atol=1e-9) for q in uniq):
                uniq.append(p)
        if len(uniq) < 3:
            continue
        n = np.cross(uniq[1] - uniq[0], uniq[2] - uniq[0])
        if np.dot(n, sum(uniq) / len(uniq) - centre) < 0:
            uniq = uniq[::-1]
        m.face(mat, uniq, normal=None, uvs=planar_uv(uniq, np.cross(uniq[1] - uniq[0], uniq[2] - uniq[0]), scale))


def box(m, mat, lo, hi, scale=4.0):
    """An axis-aligned box from its low and high corners."""
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    hexahedron(m, mat, [(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1),
                        (x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)], scale)


def obox(m, mat, centre, axes, half, scale=4.0):
    """An oriented box: centre, three axes (unit vectors), half sizes along them."""
    c = np.asarray(centre, np.float64)
    ax = [np.asarray(a, np.float64) / np.linalg.norm(a) for a in axes]
    pts = []
    for sy in (-1, 1):
        for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            pts.append(c + ax[0] * sx * half[0] + ax[1] * sy * half[1] + ax[2] * sz * half[2])
    hexahedron(m, mat, pts, scale)


def frustum(m, mat, centre, bottom, top, height, top_shift=(0.0, 0.0), scale=4.0):
    """A box narrowing upwards: bottom (w, d) and top (w, d) sizes, `height` along y, centred at `centre`; the top
    may be shifted (x, z). A zero top size makes a wedge or a pyramid."""
    cx, cy, cz = centre
    (bw, bd), (tw, td) = bottom, top
    y0, y1 = cy - height / 2, cy + height / 2
    sx, sz = top_shift
    hexahedron(m, mat, [(cx - bw / 2, y0, cz - bd / 2), (cx + bw / 2, y0, cz - bd / 2),
                        (cx + bw / 2, y0, cz + bd / 2), (cx - bw / 2, y0, cz + bd / 2),
                        (cx + sx - tw / 2, y1, cz + sz - td / 2), (cx + sx + tw / 2, y1, cz + sz - td / 2),
                        (cx + sx + tw / 2, y1, cz + sz + td / 2), (cx + sx - tw / 2, y1, cz + sz + td / 2)], scale)


def _basis(axis):
    a = np.asarray(axis, np.float64)
    a = a / np.linalg.norm(a)
    t = np.array([1.0, 0, 0]) if abs(a[0]) < 0.9 else np.array([0, 0, 1.0])
    u = np.cross(a, t)
    u /= np.linalg.norm(u)
    v = np.cross(a, u)
    return a, u, v


def cylinder(m, mat, base, axis, r, length, seg=16, caps=True, cap_mat=None, r_top=None, phase=0.0,
             smooth=True, vscale=4.0):
    """A cylinder (or cone/frustum with r_top) from `base` along `axis` for `length`, `seg` sides; `smooth` shades it
    round, else each side flat (an n-gon prism)."""
    a, u, v = _basis(axis)
    b = np.asarray(base, np.float64)
    rt = r if r_top is None else r_top
    slope = (r - rt) / length
    ang = [phase + 2 * math.pi * k / seg for k in range(seg + 1)]
    ring = lambda t: u * math.cos(t) + v * math.sin(t)
    if smooth:
        verts = []
        for k, t in enumerate(ang):
            d = ring(t)
            n = d + a * slope
            n /= np.linalg.norm(n)
            lo = m.vertex(b + d * r, n, (k / seg * 2, 0))
            hi = m.vertex(b + a * length + d * rt, n, (k / seg * 2, length / vscale))
            verts.append((lo, hi))
        for k in range(seg):
            (a0, a1), (b0, b1) = verts[k], verts[k + 1]
            m.tri(mat, a0, b1, b0)
            m.tri(mat, a0, a1, b1)
    else:
        for k in range(seg):
            d0, d1 = ring(ang[k]), ring(ang[k + 1])
            pts = [b + d0 * r, b + d1 * r, b + a * length + d1 * rt, b + a * length + d0 * rt]
            n = (d0 + d1) / 2 + a * slope
            uv = [(0, 0), (1, 0), (1, length / vscale), (0, length / vscale)]
            m.face(mat, pts, uv, normal=n)
    if caps:
        cm = mat if cap_mat is None else cap_mat
        for c, rr, sgn in ((b, r, -1), (b + a * length, rt, 1)):
            if rr <= 0:
                continue
            pts = [c + ring(t) * rr for t in ang[:-1]]
            if sgn < 0:
                pts = pts[::-1]
            uv = [(0.5 + 0.5 * math.cos(t), 0.5 + 0.5 * math.sin(t)) for t in ang[:-1]]
            if sgn < 0:
                uv = uv[::-1]
            m.face(cm, pts, uv, normal=a * sgn)


def prism(m, mat, base, axis, r, length, sides, phase=0.0, **kw):
    """A regular n-sided prism (flat sides), `r` the corner radius."""
    cylinder(m, mat, base, axis, r, length, seg=sides, phase=phase, smooth=False, **kw)


def ellipsoid(m, mat, centre, radii, seg=16, rings=None, uv_scale=1.0):
    """An ellipsoid (a sphere when the radii are equal), shaded round."""
    rings = rings or seg // 2
    cx, cy, cz = centre
    rx, ry, rz = radii
    rows = []
    for i in range(rings + 1):
        th = math.pi * i / rings
        row = []
        for k in range(seg + 1):
            ph = 2 * math.pi * k / seg
            d = np.array([math.sin(th) * math.cos(ph), math.cos(th), math.sin(th) * math.sin(ph)])
            n = d / np.array([rx, ry, rz])
            n /= np.linalg.norm(n)
            row.append(m.vertex((cx + rx * d[0], cy + ry * d[1], cz + rz * d[2]), n,
                                (k / seg * uv_scale, i / rings * uv_scale)))
        rows.append(row)
    for i in range(rings):
        for k in range(seg):
            a, b, c, d = rows[i][k], rows[i][k + 1], rows[i + 1][k + 1], rows[i + 1][k]
            m.tri(mat, a, c, b)
            m.tri(mat, a, d, c)


def sphere(m, mat, centre, r, seg=16, **kw):
    ellipsoid(m, mat, centre, (r, r, r), seg, **kw)


def torus(m, mat, centre, axis, R, r, seg=24, tube_seg=10):
    """A ring of radius R about `axis` through `centre`, its tube r thick."""
    a, u, v = _basis(axis)
    c = np.asarray(centre, np.float64)
    grid = []
    for i in range(seg + 1):
        t = 2 * math.pi * i / seg
        d = u * math.cos(t) + v * math.sin(t)
        row = []
        for k in range(tube_seg + 1):
            s = 2 * math.pi * k / tube_seg
            n = d * math.cos(s) + a * math.sin(s)
            row.append(m.vertex(c + d * R + n * r, n, (i / seg * 4, k / tube_seg)))
        grid.append(row)
    for i in range(seg):
        for k in range(tube_seg):
            p, q, s, t = grid[i][k], grid[i + 1][k], grid[i + 1][k + 1], grid[i][k + 1]
            m.tri(mat, p, s, q)
            m.tri(mat, p, t, s)


def frames(path):
    """Each point of a path with its forward, sideways and up directions (up kept near the world's)."""
    out = []
    pts = [np.asarray(p, np.float64) for p in path]
    for i, p in enumerate(pts):
        f = pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]
        f /= np.linalg.norm(f) + 1e-12
        ref = np.array([0.0, 1.0, 0.0]) if abs(f[1]) < 0.95 else np.array([1.0, 0.0, 0.0])
        side = np.cross(ref, f)
        side /= np.linalg.norm(side) + 1e-12
        up = np.cross(f, side)
        out.append((p, f, side, up))
    return out


def tube(m, mat, path, r, seg=8, caps=False):
    """A round tube along a path of points."""
    fr = frames(path)
    rings = []
    along = 0.0
    for i, (p, f, s, u) in enumerate(fr):
        if i:
            along += float(np.linalg.norm(p - fr[i - 1][0]))
        ring = []
        for k in range(seg + 1):
            a = 2 * math.pi * k / seg
            n = s * math.cos(a) + u * math.sin(a)
            ring.append(m.vertex(p + n * r, n, (k / seg, along / 4.0)))
        rings.append(ring)
    for i in range(len(rings) - 1):
        for k in range(seg):
            a, b, c, d = rings[i][k], rings[i][k + 1], rings[i + 1][k + 1], rings[i + 1][k]
            m.tri(mat, a, c, b)
            m.tri(mat, a, d, c)
    if caps:
        for i, sgn in ((0, -1), (len(fr) - 1, 1)):
            p, f, s, u = fr[i]
            pts = [p + (s * math.cos(2 * math.pi * k / seg) + u * math.sin(2 * math.pi * k / seg)) * r
                   for k in range(seg)]
            m.face(mat, pts if sgn > 0 else pts[::-1], normal=f * sgn)


def sweep(m, path, profile, mats, closed=True, uscale=4.0, vscale=8.0, end_mat=None):
    """A cross-section carried along a path: `profile` points (across, up), `mats` a material per edge; the ends are
    closed with `end_mat` when given."""
    fr = frames(path)
    along = [0.0]
    for i in range(1, len(fr)):
        along.append(along[-1] + float(np.linalg.norm(fr[i][0] - fr[i - 1][0])))
    n = len(profile)
    run = [0.0]
    for e in range(n if closed else n - 1):
        a, b = np.array(profile[e]), np.array(profile[(e + 1) % n])
        run.append(run[-1] + float(np.linalg.norm(b - a)))
    for e in range(n if closed else n - 1):
        a, b = np.array(profile[e], np.float64), np.array(profile[(e + 1) % n], np.float64)
        d = b - a
        out2 = np.array([d[1], -d[0]]) / (np.linalg.norm(d) + 1e-12)
        for i in range(len(fr) - 1):
            p0, _, s0, u0 = fr[i]
            p1, _, s1, u1 = fr[i + 1]
            q = [p0 + s0 * a[0] + u0 * a[1], p0 + s0 * b[0] + u0 * b[1], p1 + s1 * b[0] + u1 * b[1],
                 p1 + s1 * a[0] + u1 * a[1]]
            nm = (s0 + s1) / 2 * out2[0] + (u0 + u1) / 2 * out2[1]
            uvs = [(run[e] / uscale, along[i] / vscale), (run[e + 1] / uscale, along[i] / vscale),
                   (run[e + 1] / uscale, along[i + 1] / vscale), (run[e] / uscale, along[i + 1] / vscale)]
            m.face(mats[e], q, uvs, normal=nm)
    if end_mat is not None:
        for i, sgn in ((0, -1), (len(fr) - 1, 1)):
            p, f, s, u = fr[i]
            pts = [p + s * q[0] + u * q[1] for q in profile]
            m.face(end_mat, pts, normal=f * sgn)


# --- the open format ---------------------------------------------------------------------------------------------

S = np.diag([1.0, 1.0, -1.0, 1.0])


# An image naming the game's texture by name instead of a file of the level's (open-format.md, `game:`).
GAME = "game:"


def material(name, texture=None, diffuse=1.0, emissive=0.3, ambient=0.1, specular=0.0, power=0.0,
             double_sided=False, alpha="OPAQUE", cutoff=0.5, blend=None, texgen=None, unlit=False, z_write=None,
             normal=None, parallax=None, alpha_cutoff=None):
    """A material's description for write_open: `texture` a file in the folder's textures/ (made with save_png), or
    `game:<name>` for the game's texture of that name (no file: the game takes it from the chosen texture pack, ours by
    default; docs/drahtseilakt/open-format.md),
    `normal` its relief (a normal map there, glTF's normalTexture, on the same texture coordinates), the
    colours scalars (grey) or rgb(a) lists, `alpha` OPAQUE, MASK (with `cutoff`) or BLEND (with `blend`, a pair of
    the format's factor names; `alpha_cutoff`: an alpha test on top of the blend), `texgen` reflect or chrome,
    `parallax` the relief's depth in texture coordinates (over 0, at most 0.25; the normal map's alpha is then the
    height, 1 the top)."""
    return dict(name=name, texture=texture, diffuse=diffuse, emissive=emissive, ambient=ambient, specular=specular,
                power=power, double_sided=double_sided, alpha=alpha, cutoff=cutoff, blend=blend, texgen=texgen,
                unlit=unlit, z_write=z_write, normal=normal, parallax=parallax, alpha_cutoff=alpha_cutoff)


def node(name, mesh=None, matrix=None, visible=False, sprite=None):
    """A node's description: `mesh` an index into the meshes (None: a frame), `matrix` its world matrix (numpy 4×4,
    the game's coordinates), `sprite` (mode, (w, h), (x, y)) for a 3D sprite."""
    return dict(name=name, mesh=mesh, matrix=np.eye(4) if matrix is None else np.asarray(matrix, np.float64),
                visible=visible, sprite=sprite)


def _colour(v, n):
    if isinstance(v, (int, float)):
        return [float(v)] * 3 + ([1.0] if n == 4 else [])
    v = [float(c) for c in v]
    if n == 4 and len(v) == 3:
        v.append(1.0)
    return v[:n]


def write_open(out_dir, meshes, nodes, materials, groups=(), sky=None, start_ball=None, settings=None,
               generator="drahtseilakt ownkit.py", scripts=None):
    """Writes the level folder: level.gltf and level.bin (the textures are already in out_dir/textures/). `meshes`
    are Mesh objects (their parts keyed by material index, or None), `nodes` from node(), `materials` from
    material(), `groups` (name, [node indices]) pairs, `settings` the scene's settings (tutorial and last_level are
    required by the reader; music, light, level_bonus optional). `scripts`: the level's behaviour graphs (scriptkit.py's
    Graph.json(); open-format.md, "Scripts"), whose objects may name what they are instead of giving its index: an
    `entity` by its node's name, a `group`, `mesh` or `material` by name, a `texture` by its file's name as the
    materials give it; None: the level has no scripts."""
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

    flat = lambda vs: struct.pack(f"<{len(vs) * len(vs[0])}f", *[c for v in vs for c in v])
    gl_meshes = []

    def mesh_json(m):
        if not m.p or not any(m.parts.values()):
            raise ValueError(f"mesh {m.name} is empty")
        pos = [(x, y, -z) for x, y, z in m.p]
        nor = [(x, y, -z) for x, y, z in m.n]
        lo = [min(p[k] for p in pos) for k in range(3)]
        hi = [max(p[k] for p in pos) for k in range(3)]
        pa = push(flat(pos), 34962, {"componentType": 5126, "count": len(pos), "type": "VEC3", "min": lo, "max": hi})
        na = push(flat(nor), 34962, {"componentType": 5126, "count": len(nor), "type": "VEC3"})
        ta = push(flat(m.uv), 34962, {"componentType": 5126, "count": len(m.uv), "type": "VEC2"})
        attrs = {"POSITION": pa, "NORMAL": na, "TEXCOORD_0": ta}
        prims = []
        for mat, idx in sorted(m.parts.items(), key=lambda kv: -1 if kv[0] is None else kv[0]):
            if not idx:
                continue
            # z negated mirrors the mesh, so each triangle's winding is reversed to keep its front
            tri = [i for k in range(0, len(idx), 3) for i in (idx[k], idx[k + 2], idx[k + 1])]
            ia = push(struct.pack(f"<{len(tri)}I", *tri), 34963,
                      {"componentType": 5125, "count": len(tri), "type": "SCALAR"})
            prim = {"attributes": attrs, "indices": ia}
            if mat is not None:
                prim["material"] = mat
            prims.append(prim)
        out = {"name": m.name, "primitives": prims}
        if m.channels:
            # each channel's coordinates are one more TEXCOORD set (attrs is shared by the primitives)
            chans = []
            for k, (mat, blend, uvs) in enumerate(m.channels, 1):
                if len(uvs) != len(m.p):
                    raise ValueError(f"mesh {m.name}: channel {k} has not a coordinate per vertex")
                attrs[f"TEXCOORD_{k}"] = push(flat(uvs), 34962,
                                              {"componentType": 5126, "count": len(uvs), "type": "VEC2"})
                chans.append({"material": mat, "blend": list(blend), "texcoord": k})
            out["extras"] = {"drahtseilakt": {"channels": chans}}
        return out

    for m in meshes:
        gl_meshes.append(mesh_json(m))
    # each finer look is one more mesh after all the level's own, named in its mesh's extras
    for i, m in enumerate(meshes):
        if m.look is not None:
            gl_meshes.append(mesh_json(m.look))
            gl_meshes[i].setdefault("extras", {}).setdefault("drahtseilakt", {})["look"] = len(gl_meshes) - 1

    images = list(dict.fromkeys([mt["texture"] for mt in materials if mt["texture"]]
                                + [mt["normal"] for mt in materials if mt.get("normal")]))
    image_ids = {n: i for i, n in enumerate(images)}
    gl_mats = []
    unlit_used = False
    for mt in materials:
        pbr = {"baseColorFactor": _colour(mt["diffuse"], 4), "metallicFactor": 0.0, "roughnessFactor": 1.0}
        if mt["texture"]:
            pbr["baseColorTexture"] = {"index": image_ids[mt["texture"]]}
        x = {"ambient": _colour(mt["ambient"], 3), "specular": _colour(mt["specular"], 3), "power": mt["power"],
             "z_write": mt["z_write"] if mt["z_write"] is not None else mt["alpha"] != "BLEND"}
        v = {"name": mt["name"], "pbrMetallicRoughness": pbr, "emissiveFactor": _colour(mt["emissive"], 3),
             "doubleSided": mt["double_sided"], "alphaMode": mt["alpha"]}
        if mt.get("normal"):
            v["normalTexture"] = {"index": image_ids[mt["normal"]]}
            if mt.get("parallax"):
                x["parallax"] = float(mt["parallax"])
        if mt["alpha"] == "BLEND":
            x["blend"] = list(mt["blend"] or ("src_alpha", "inv_src_alpha"))
            if mt.get("alpha_cutoff") is not None:
                x["alpha_cutoff"] = float(mt["alpha_cutoff"])
        if mt["alpha"] == "MASK":
            v["alphaCutoff"] = mt["cutoff"]
        if mt["texgen"]:
            x["texgen"] = mt["texgen"]
        if mt["unlit"]:
            v["extensions"] = {"KHR_materials_unlit": {}}
            unlit_used = True
        v["extras"] = {"drahtseilakt": x}
        gl_mats.append(v)

    gl_nodes = []
    for nd in nodes:
        mg = S @ nd["matrix"] @ S
        v = {"name": nd["name"], "matrix": [float(c) for c in mg.T.flatten()],
             "extras": {"drahtseilakt": {"visible": bool(nd["visible"])}}}
        if nd["sprite"] is not None:
            mode, size, offset = nd["sprite"]
            v["extras"]["drahtseilakt"]["sprite"] = {"mode": mode, "size": list(size), "offset": list(offset)}
        if nd["mesh"] is not None:
            v["mesh"] = nd["mesh"]
        gl_nodes.append(v)

    st = {"tutorial": False, "last_level": False}
    st.update(settings or {})
    doc = {
        "asset": {"version": "2.0", "generator": generator},
        "extensionsUsed": ["KHR_materials_unlit"] if unlit_used else [],
        "scene": 0,
        "scenes": [{"nodes": list(range(len(gl_nodes))), "extras": {"drahtseilakt": {
            "format": 1, "groups": [{"name": g, "members": list(mem)} for g, mem in groups],
            "sky": sky, "start_ball": start_ball, "settings": st}}}],
        "nodes": gl_nodes,
        "meshes": gl_meshes,
        "materials": gl_mats,
        "textures": [{"source": i, "sampler": 0} for i in range(len(images))],
        "images": [{"uri": n, "name": n[len(GAME):]} if n.startswith(GAME)
                   else {"uri": f"textures/{n}", "name": os.path.splitext(n)[0]} for n in images],
        "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}],
        "buffers": [{"uri": "level.bin", "byteLength": len(bin_)}],
        "bufferViews": views,
        "accessors": accessors,
    }
    if scripts is not None:
        # names to the indices the reader counts: nodes, groups, meshes, materials, images (crates/level/src/open.rs)
        tables = {"entity": {nd["name"]: i for i, nd in enumerate(nodes)},
                  "group": {g: i for i, (g, _) in enumerate(groups)},
                  "mesh": {m.name: i for i, m in enumerate(meshes)},
                  "material": {mt["name"]: i for i, mt in enumerate(materials)},
                  "texture": image_ids}
        objects = []
        for name, cls, (kind, ref) in scripts["objects"]:
            if isinstance(ref, str):
                if ref not in tables.get(kind, {}):
                    raise ValueError(f"scripts: object {name} names the {kind} {ref}, which the level has not")
                ref = tables[kind][ref]
            objects.append([name, cls, [kind, ref]])
        doc["scenes"][0]["extras"]["drahtseilakt"]["scripts"] = dict(scripts, objects=objects)
    os.makedirs(out_dir, exist_ok=True)
    for n in images:
        if not n.startswith(GAME) and not os.path.isfile(os.path.join(out_dir, "textures", n)):
            raise FileNotFoundError(f"{out_dir}/textures/{n} not made")
    with open(os.path.join(out_dir, "level.gltf"), "w") as f:
        json.dump(doc, f, indent=1)
    with open(os.path.join(out_dir, "level.bin"), "wb") as f:
        f.write(bin_)
    return len(bin_)


def convex_hull(points, eps=1e-9):
    """The faces of the convex hull of a few points (an incremental hull: fine for the tens of points a collision
    shape has), as lists of point indices wound outward."""
    p = np.asarray(points, np.float64)
    n = len(p)
    # a first tetrahedron of four points not in one plane
    i0 = 0
    i1 = int(np.argmax(np.linalg.norm(p - p[i0], axis=1)))
    d = np.cross(p - p[i0], p[i1] - p[i0])
    i2 = int(np.argmax(np.linalg.norm(d, axis=1)))
    nrm = np.cross(p[i1] - p[i0], p[i2] - p[i0])
    i3 = int(np.argmax(np.abs((p - p[i0]) @ nrm)))
    centre = p[[i0, i1, i2, i3]].mean(0)

    def oriented(a, b, c):
        nv = np.cross(p[b] - p[a], p[c] - p[a])
        return (a, b, c) if np.dot(nv, p[a] - centre) > 0 else (a, c, b)

    faces = [oriented(*f) for f in ((i0, i1, i2), (i0, i1, i3), (i0, i2, i3), (i1, i2, i3))]
    for q in range(n):
        if q in (i0, i1, i2, i3):
            continue
        seen = []
        for f in faces:
            a, b, c = f
            nv = np.cross(p[b] - p[a], p[c] - p[a])
            if np.dot(nv, p[q] - p[a]) > eps * max(1.0, np.linalg.norm(nv)):
                seen.append(f)
        if not seen:
            continue
        edges = {}
        for a, b, c in seen:
            for e in ((a, b), (b, c), (c, a)):
                edges[e] = edges.get(e, 0) + 1
        horizon = [e for e in edges if (e[1], e[0]) not in edges]
        faces = [f for f in faces if f not in seen] + [(a, b, q) for a, b in horizon]
    return faces


def hull(m, mat, points, smooth=False):
    """The convex hull of `points` as a solid: flat faces (or shaded round from the centre with `smooth`)."""
    p = np.asarray(points, np.float64)
    centre = p.mean(0)
    for a, b, c in convex_hull(p):
        if smooth:
            ids = []
            for i in (a, b, c):
                nv = p[i] - centre
                ids.append(m.vertex(p[i], nv / (np.linalg.norm(nv) + 1e-12), planar_uv([p[i]], nv)[0]))
            m.tri(mat, *ids)
        else:
            m.face(mat, [p[a], p[b], p[c]])
