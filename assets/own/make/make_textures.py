#!/usr/bin/env python3
"""Makes the game's own textures for the levels, their parts and the balls (`assets/own/Textures/<name>.png`, each
standing in for the original's texture of that name: resources.rs, `textures own`). Each is made from freely licensed
photo textures (free_textures.py: CC0, pinned and checksummed) and shapes drawn here, to numbers measured on the
original's texture of the same name (analysed only, never copied, traced or resampled): its palette (mean colour),
its grain (the lightness's standard deviation against its mean), and its layout (where its brick strips, metal bands,
rings and shading lie, as fractions of its side). Each keeps the original's proportions and tiling, so it fits the
levels' texture coordinates: a texture that tiles tiles, a ball's wraps its sphere the same way. They are made at
twice the original's resolution.

  python3 -I assets/own/make/make_textures.py [STEP...]   # all, or only floors, stone, wood, marks, env, balls, extras

Needs Python 3 with numpy, scipy and Pillow, and the sources (python3 -I free_textures.py fetch). The output is
deterministic. What the original has that is not here (skies, particles, the lightning) still comes from the player's copy.
"""

import math
import os
import sys

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)  # free_textures.py beside this script (python3 -I leaves the script's folder out)
import free_textures as ft  # noqa: E402

OUT = os.path.join(os.path.dirname(HERE), "Textures")
SCALE = 2  # our side against the original's


def save(img, name, alpha=None):
    a = np.clip(img * 255 + 0.5, 0, 255).astype(np.uint8)
    mode = "RGB"
    if alpha is not None:
        a = np.concatenate([a, np.clip(alpha * 255 + 0.5, 0, 255).astype(np.uint8)[..., None]], -1)
        mode = "RGBA"
    os.makedirs(OUT, exist_ok=True)
    Image.fromarray(a, mode).save(os.path.join(OUT, name + ".png"), optimize=True)


def rgb(*v):
    return np.array(v, np.float64) / 255.0


def noise(n, cells, seed, octaves=4):
    """Tiling value noise 0..1 (n x n)."""
    rng = np.random.default_rng(seed)
    out = np.zeros((n, n))
    amp = total = 0.0
    amp = 1.0
    for o in range(octaves):
        c = max(2, int(cells * 2**o))
        g = rng.random((c, c))
        t = np.arange(n) / n * c
        i0 = np.floor(t).astype(int)
        f = t - i0
        f = f * f * (3 - 2 * f)
        i1 = (i0 + 1) % c
        i0 %= c
        a = g[i0][:, i0] * (1 - f) + g[i0][:, i1] * f
        b = g[i1][:, i0] * (1 - f) + g[i1][:, i1] * f
        out += (a * (1 - f)[:, None] + b * f[:, None]) * amp
        total += amp
        amp *= 0.5
    return out / total


def photo(sid, size, mean, rel, hue=0.35, flat=0.3, turns=0, crop=None, fix=True):
    """A source graded to a measured palette and grain at `size` (crop: (x0, y0, w, h) of the 1024 source first)."""
    img = np.rot90(ft.load(sid), turns)
    if crop:
        x0, y0, w, h = crop
        img = img[y0:y0 + h, x0:x0 + w]
    img = ft.grade(img, mean, rel, hue, flat)
    if fix:
        img = ft.tile_fix(img) if not crop else force_tile(img)
    return ft.resize(img, size)


def force_tile(img, band=0.12):
    """A crop made to tile: its edges blended with itself shifted by half."""
    h, w = img.shape[:2]
    shifted = np.roll(img, (h // 2, w // 2), (0, 1))
    y = np.minimum(np.arange(h), h - 1 - np.arange(h)) / (band * h)
    x = np.minimum(np.arange(w), w - 1 - np.arange(w)) / (band * w)
    a = np.clip(np.minimum(y[:, None], x[None, :]), 0, 1)
    a = a * a * (3 - 2 * a)
    return img * a[..., None] + shifted * (1 - a[..., None])


def board_gaps(sid):
    """The columns where a plank texture's boards meet: the deepest dips of its columns' mean lightness (against their
    surroundings), at least half a typical board apart."""
    from scipy import ndimage

    src = ft.load(sid)
    col = (src @ ft.LUMA).mean(0)
    dip = ndimage.gaussian_filter1d(col, 24, mode="wrap") - ndimage.gaussian_filter1d(col, 1.5, mode="wrap")
    w = len(col)
    order = np.argsort(-dip)
    gaps = []
    for x in order:
        if dip[x] <= 0 or len(gaps) >= 40:
            break
        if all(min(abs(x - g), w - abs(x - g)) > 40 for g in gaps):
            gaps.append(int(x))
    return sorted(gaps)


def wood(size, mean, rel, n_boards, sid="WoodFloor040", hue=0.45, seed=0, turns=0):
    """`n_boards` whole upright boards of a plank texture side by side, unstretched (a square crop from one board gap
    to another, made to tile down), graded and sized."""
    gaps = board_gaps(sid)
    if len(gaps) > n_boards:
        start = seed % (len(gaps) - n_boards)
        x0, w = gaps[start], gaps[start + n_boards] - gaps[start]
    else:
        x0, w = 0, 1024
    img = ft.load(sid)[:, x0:x0 + w]
    y0 = (seed * 131) % max(1, 1024 - w)
    img = img[y0:y0 + w].copy()
    # each board its own tone, as the original's boards differ (some darker, redder or greener)
    rng = np.random.default_rng(seed + 100)
    inner = [g - x0 for g in gaps if x0 < g < x0 + w]
    for a, b in zip([0] + inner, inner + [w]):
        tone = rng.uniform(0.75, 1.12) * np.array([1.0, rng.uniform(0.86, 1.0), rng.uniform(0.72, 0.97)])
        img[:, a:b] = np.clip(img[:, a:b] * tone, 0, 1)
    img = ft.grade(img, mean, rel, hue, 0.2)
    img = ft.resize(tile_down(img), size)
    return np.rot90(img, turns) if turns else img


def tile_down(img, band=0.15):
    """A crop made to tile down (it already tiles across, cut at board gaps): its top and bottom blended with itself
    shifted by half its height."""
    h = img.shape[0]
    shifted = np.roll(img, h // 2, 0)
    y = np.minimum(np.arange(h), h - 1 - np.arange(h)) / (band * h)
    a = np.clip(y, 0, 1)
    a = (a * a * (3 - 2 * a))[:, None, None]
    return img * a + shifted * (1 - a)


def concrete(size, mean, rel, sid="Concrete047A", flat=0.7, hue=0.25):
    return photo(sid, size, mean, rel, hue, flat)


def iron(h, w, mean=(150, 150, 156), rel=0.10, seed=0):
    """A strip of worn iron (h x w), lit from above like a rounded bar."""
    src = photo("Metal063", 256, mean, rel, 0.5, 0.2)
    y = (np.arange(h) + 0.5) / h
    shade = 0.75 + 0.45 * np.sin(np.pi * y) - 0.15 * y
    tile = np.tile(src, (math.ceil(h / 256) + 1, math.ceil(w / 256) + 1, 1))
    off = seed * 37 % 200
    return np.clip(tile[off:off + h, off:off + w] * shade[:, None, None], 0, 1)


def rivet(img, cx, cy, r, light=1.25):
    """A domed rivet: lit top left, a dark rim."""
    h, w = img.shape[:2]
    y, x = np.mgrid[0:h, 0:w]
    d = np.hypot(x - cx, y - cy) / r
    inside = d < 1
    lit = 1 + (light - 1) * np.clip(1 - np.hypot(x - cx + r * 0.35, y - cy + r * 0.35) / r, 0, 1) * 2
    rim = np.clip((d - 0.75) / 0.25, 0, 1)
    shade = np.where(inside, lit * (1 - 0.45 * rim), 1.0)
    base = rgb(165, 165, 170)
    img[inside] = np.clip(base * shade[inside][:, None], 0, 1)
    # its shadow down right
    sh = (np.hypot(x - cx - r * 0.4, y - cy - r * 0.4) < r) & ~inside
    img[sh] *= 0.7


def band(img, y0, y1, rivets=None, seed=0):
    """A riveted iron band across the texture between rows y0 and y1."""
    h, w = img.shape[:2]
    img[y0:y1] = iron(y1 - y0, w, seed=seed)
    img[y0:y0 + 1] *= 1.15
    img[y1 - 1:y1] *= 0.55
    if y1 < h:
        img[y1:min(h, y1 + 2)] *= 0.7
    if rivets:
        r = max(1.5, (y1 - y0) * 0.25)
        for k in range(int(w / rivets)):
            rivet(img, (k + 0.5) * rivets, (y0 + y1) / 2, r)


def plate(img, x0, y0, x1, y1, bolts=0, seed=0, mean=(150, 150, 156)):
    """A small iron plate (a strap or a clip) with bolts along it."""
    img[y0:y1, x0:x1] = iron(y1 - y0, x1 - x0, mean=mean, seed=seed)
    img[y0:y1, x0:x0 + 1] *= 1.15
    img[y1 - 1:y1, x0:x1] *= 0.55
    img[y1:y1 + 2, x0 + 2:x1 + 2] *= 0.7
    vertical = (y1 - y0) > (x1 - x0)
    for k in range(bolts):
        t = (k + 0.5) / bolts
        cx, cy = ((x0 + x1) / 2, y0 + t * (y1 - y0)) if vertical else (x0 + t * (x1 - x0), (y0 + y1) / 2)
        rivet(img, cx, cy, min(x1 - x0, y1 - y0) * 0.22)


# --- the floors ----------------------------------------------------------------------------------------------------
# Measured on the original's floor textures (128 texels a side): the brick strip at an edge is 10 texels wide, a grey
# groove of 2 follows, then the concrete, a shadow fading over the next 4. The concrete's mean colour (195, 176, 146),
# its grain 0.03; the strip's bricks (152, 114, 83), 0.18, about 7 texels high. The profiled floor is darkened down its
# middle (a valley 0.28 deep, its width 20 texels to 1/e); the flat profiled one's light X from corner to corner.

S = 128 * SCALE
STRIP, GROOVE, FADE = 10 * SCALE, 2 * SCALE, 4 * SCALE
FLOOR = (195, 176, 146)


def floor_concrete(mean=FLOOR, rel=0.035):
    return concrete(S, mean, rel)


def brick_strip(h, w, seed):
    """A column of small bricks (as the floors' edges have), from the brick photo, with mortar between them."""
    src = photo("Bricks005", 512, (160, 104, 72), 0.16, 0.5, 0.2)
    rng = np.random.default_rng(seed)
    out = np.zeros((h, w, 3))
    mortar = rgb(120, 112, 100)
    y = 0
    while y < h:
        bh = min(int(round(7 * SCALE * rng.uniform(0.85, 1.15))), h - y)
        sy, sx = rng.integers(0, 512 - bh), rng.integers(0, 512 - w)
        out[y:y + bh] = src[sy:sy + bh, sx:sx + w] * rng.uniform(0.85, 1.1)
        out[y + bh - SCALE:y + bh] = mortar
        y += bh
    # its last brick ends at the texture's end, so the strip tiles down
    out[h - SCALE:] = mortar
    out[:, :1] *= 0.7
    return out


def floor_edge(img, side, seed):
    """The brick strip, the groove and the shadow at the left (side 0) or right (side 1) edge."""
    h, w = img.shape[:2]
    strip = brick_strip(h, STRIP, seed)
    groove = rgb(105, 100, 96) * (0.9 + 0.2 * noise(h, 8, seed + 5)[:, :GROOVE, None])
    fade = 1 - 0.18 * (1 - np.arange(FADE) / FADE)
    if side == 0:
        img[:, :STRIP] = strip
        img[:, STRIP:STRIP + GROOVE] = groove
        img[:, STRIP + GROOVE:STRIP + GROOVE + FADE] *= fade[None, :, None]
    else:
        img[:, w - STRIP:] = strip[:, ::-1]
        img[:, w - STRIP - GROOVE:w - STRIP] = groove
        img[:, w - STRIP - GROOVE - FADE:w - STRIP - GROOVE] *= fade[::-1][None, :, None]
    return img


def make_floors():
    out = {}
    out["Floor_Top_Borderless"] = floor_concrete()
    out["Floor_Top_Border"] = floor_edge(floor_concrete(), 0, 1)
    out["Floor_Top_Flat"] = floor_edge(floor_edge(floor_concrete(), 0, 2), 1, 3)
    x = (np.arange(S) + 0.5) / S
    valley = 1 - 0.28 * np.exp(-(((x - 0.5) * 128 / 20) ** 2))
    prof = floor_concrete((188, 170, 140)) * valley[None, :, None]
    out["Floor_Top_Profil"] = floor_edge(floor_edge(prof, 0, 4), 1, 5)
    # its shading: two dark lines from the top's middle out to the strips (measured: from x 63 at the top, 0.45 texels
    # out for each one down, 0.17 deep, about 5 wide), lighter outside them than between them
    yy, xx = np.mgrid[0:S, 0:S] / SCALE
    off = np.abs(xx - 63.5) - 0.45 * yy
    lines = 1 - 0.17 * np.exp(-((off / 5) ** 2))
    side = np.where(off > 0, 1.06, 0.97)
    pf = floor_concrete((176, 158, 130)) * (lines * side)[..., None]
    out["Floor_Top_ProfilFlat"] = floor_edge(floor_edge(pf, 0, 6), 1, 7)
    out["Floor_Top_Checkpoint"] = checkpoint()
    # the floor's side: darker concrete, its own brick strip at the left, lighter towards the right
    side = concrete(S, (141, 121, 95), 0.10, "Concrete012", 0.4, 0.3)
    side *= np.linspace(0.82, 1.12, S)[None, :, None]
    out["Floor_Side"] = floor_edge(side, 0, 8)
    for name, img in out.items():
        save(np.clip(img, 0, 1), name)


def checkpoint():
    """The checkpoint's floor: a ring of bricks round a lighter slab, a cross of brick lines, an inner ring of bricks
    round a grate (measured radii in texels of 128: grate 12, inner bricks 26 to 30, slab to 54, outer bricks 54 to
    62; cross lines 3 wide between the rings)."""
    img = floor_concrete((186, 166, 136), 0.04)
    y, x = np.mgrid[0:S, 0:S]
    c = (S - 1) / 2
    r = np.hypot(x - c, y - c) / SCALE
    ang = np.arctan2(y - c, x - c)
    slab = (r > 30) & (r < 54)
    img[slab] = img[slab] * 1.04
    bricks = photo("Bricks005", S, (160, 104, 72), 0.16, 0.5, 0.2)
    mortar = rgb(120, 112, 100)

    def ring(r0, r1, n):
        m = (r >= r0) & (r < r1)
        joint = (np.abs(((ang / (2 * np.pi) * n) % 1) - 0.5) > 0.44) | (r < r0 + 0.6) | (r > r1 - 0.6)
        img[m] = np.where(joint[m][:, None], mortar, bricks[m])

    ring(54, 62, 40)
    ring(26, 30, 22)
    cross = ((np.abs(x - c) < 1.5 * SCALE) | (np.abs(y - c) < 1.5 * SCALE)) & (r > 30) & (r < 54)
    img[cross] = bricks[cross] * 0.95
    # the grate: a steel dish with a lattice of holes
    g = r < 12
    steel = photo("Metal009", S, (150, 150, 158), 0.08, 0.2, 0.2)
    holes = ((x // (3 * SCALE) + y // (3 * SCALE)) % 2 == 0) & (r < 9)
    dish = 0.65 + 0.45 * np.clip(1 - np.hypot(x - c + 3 * SCALE, y - c + 3 * SCALE) / (12 * SCALE), 0, 1)
    img[g] = steel[g] * dish[g][:, None]
    img[holes] = rgb(25, 24, 22)
    rim = (r >= 12) & (r < 14)
    img[rim] = rgb(200, 196, 188)
    return img


# --- columns, bricks, metal ---------------------------------------------------------------------------------------


def make_stone():
    col = concrete(S, (183, 164, 133), 0.085, "Concrete012", 0.5, 0.3)
    save(col, "Column_beige")
    # the fading column: the same, opaque at the top and fading to nothing at the bottom (measured: linear)
    save(col, "Column_beige_fade", alpha=np.tile(np.linspace(1, 0, S)[:, None], (1, S)))
    save(photo("Marble012", S, (141, 159, 191), 0.09, 0.4, 0.3), "Column_blue")
    # the light bricks: about eleven courses down the texture (a course 11 texels of 128)
    save(courses("Bricks053", S, (187, 177, 162), 0.13, 128 / 11), "Brick")
    save(photo("Metal063", 64 * SCALE, (115, 111, 122), 0.10, 0.6, 0.2), "Metal_stained")
    save(photo("RoofingTiles015A", 64 * SCALE, (97, 89, 75), 0.29, 0.3, 0.2), "Tower_Roof")
    save(photo("Fabric066", S, (155, 141, 117), 0.20, 0.4, 0.3), "sandsack")
    # the light concrete modules: Modul15 with a thin brick strip each side, Modul16 a column of light stones right
    m15 = floor_concrete((190, 167, 135), 0.06)
    m15[:, :5 * SCALE] = brick_strip(S, 5 * SCALE, 21)
    m15[:, -5 * SCALE:] = brick_strip(S, 5 * SCALE, 22)
    save(m15, "Modul15")
    m16 = floor_concrete((176, 158, 129), 0.08)
    stones = courses("Bricks053", S, (215, 210, 200), 0.10, 128 / 11)
    rng = np.random.default_rng(16)
    for k in range(12):
        y0 = int(k * S / 12)
        x0 = int((100 + rng.integers(-6, 6)) * SCALE)
        m16[y0:y0 + S // 12, x0:] = stones[y0:y0 + S // 12, x0:]
    save(m16, "Modul16")


def courses(sid, size, mean, rel, period_texels):
    """A brick texture with its courses `period_texels` apart on a side of 128: the source cropped to as many of its
    own courses (found from its rows' autocorrelation), made to tile."""
    src = ft.load(sid)
    rows = (src @ ft.LUMA).mean(1)
    rows = rows - rows.mean()
    ac = [float(np.dot(rows, np.roll(rows, k))) for k in range(20, 200)]
    period = 20 + int(np.argmax(ac))
    want = 128 / period_texels
    side = int(min(1024, round(period * want)))
    return photo(sid, size, mean, rel, 0.4, 0.2, crop=(0, 0, side, side))


# --- wood ------------------------------------------------------------------------------------------------------------
# Measured: the boards stand upright, about nine across (Wood_Plain, the wooden ball), seven (Wood_Metal, Modul11), the
# old and raft wood two wide weathered boards; the bands, straps and frames where the comments say (texels of 128).


def make_wood():
    t = SCALE
    save(wood(S, (159, 126, 80), 0.30, 9, seed=1), "Wood_Plain")
    save(wood(S, (203, 163, 111), 0.16, 9, seed=2), "Wood_Plain2")
    save(wood(S, (165, 139, 98), 0.16, 3, "WoodFloor041", hue=0.3, seed=1), "Wood_Old")
    save(wood(S, (139, 123, 94), 0.16, 3, "WoodFloor041", hue=0.2, seed=3), "Wood_Raft")
    # Wood_Metal: seven boards, a riveted iron band along the top (rows 3 to 10) and the bottom (117 to 124)
    wm = wood(S, (128, 100, 75), 0.20, 7, seed=4)
    band(wm, 3 * t, 10 * t, rivets=11.6 * t, seed=1)
    band(wm, 117 * t, 124 * t, rivets=11.6 * t, seed=2)
    save(wm, "Wood_Metal")
    # Wood_MetalStripes: nine boards, a short iron strap at the top and bottom of each, a few between
    ws = wood(S, (160, 131, 88), 0.30, 9, seed=5)
    rng = np.random.default_rng(6)
    for k in range(9):
        cx = int((k + 0.5) * S / 9)
        for y0 in (2, 116):
            plate(ws, cx - 2 * t, y0 * t, cx + 2 * t, (y0 + 10) * t, bolts=2, seed=k)
        if rng.random() < 0.4:
            y0 = int(rng.integers(40, 70))
            plate(ws, cx - 2 * t, y0 * t, cx + 2 * t, (y0 + 12) * t, bolts=2, seed=k + 9)
    save(ws, "Wood_MetalStripes")
    # Wood_Nailed: boards, two cross planks (top 10 to 24, bottom 100 to 114, x 46 to 108) with nails, clips at the edges
    wn = wood(S, (148, 116, 74), 0.32, 9, seed=7)
    for y0 in (10, 100):
        wn[y0 * t:(y0 + 14) * t, 46 * t:108 * t] = wood(S, (120, 80, 50), 0.25, 3, seed=8, turns=1)[:14 * t, :62 * t]
        wn[(y0 + 14) * t:(y0 + 15) * t, 46 * t:108 * t] *= 0.5
        for k in range(3):
            rivet(wn, (56 + 20 * k) * t, (y0 + 7) * t, 2 * t)
    for y0 in (4, 20, 36, 52):
        plate(wn, 0, y0 * t, 6 * t, (y0 + 4) * t, seed=y0)
        plate(wn, S - 6 * t, y0 * t, S, (y0 + 4) * t, seed=y0 + 1)
    save(wn, "Wood_Nailed")
    # Wood_Misc: a plank with three bolts across the top (rows 6 to 36), three hinges below, a log's end and a hole
    wmisc = wood(S, (165, 131, 89), 0.30, 4, seed=9)
    wmisc[6 * t:36 * t, 4 * t:124 * t] = wood(S, (125, 85, 55), 0.25, 3, seed=10, turns=1)[:30 * t, :120 * t]
    wmisc[36 * t:38 * t, 4 * t:124 * t] *= 0.5
    for k in range(3):
        rivet(wmisc, (32 + 30 * k) * t, 21 * t, 4 * t)
    for k, x0 in enumerate((14, 38, 70)):
        plate(wmisc, x0 * t, 80 * t, (x0 + 10) * t, 120 * t, bolts=3, seed=20 + k)
    y, x = np.mgrid[0:S, 0:S]
    d = np.hypot(x - 104 * t, y - 50 * t) / t
    log = d < 16
    rings = 0.8 + 0.2 * np.cos(d * 2.2 + 3 * noise(S, 6, 11))
    wmisc[log] = (rgb(200, 150, 90) * rings[log][:, None])
    wmisc[(d >= 16) & (d < 17)] *= 0.5
    d = np.hypot(x - 104 * t, y - 104 * t) / t
    wmisc[d < 12] = (rgb(120, 90, 60) * np.clip(d[d < 12] / 12, 0.05, 1)[:, None] ** 2)
    save(wmisc, "Wood_Misc")
    # Wood_Panel: two wide dark boards, a dark frame round them with iron corners (frame 6 wide)
    wp = wood(S, (110, 80, 50), 0.30, 4, seed=12)
    f = 6 * t
    for sl in ((slice(0, f), slice(None)), (slice(S - f, S), slice(None))):
        wp[sl] *= 0.55
    wp[:, S // 2 - t:S // 2 + t] *= 0.3
    for cx in (0, S // 2 - 6 * t, S // 2 + t, S - 6 * t):
        for y0 in (0, S - f):
            plate(wp, cx, y0, cx + 6 * t, y0 + f, seed=cx + y0)
    save(wp, "Wood_Panel")
    # Modul11: seven light boards with iron straps (Modul11_Wood two short ones, Modul11_13_Wood one along the top)
    m11 = wood(S, (196, 157, 107), 0.18, 7, seed=13)
    plate(m11, 46 * t, 26 * t, 84 * t, 32 * t, bolts=4, seed=1)
    plate(m11, 30 * t, 58 * t, 68 * t, 64 * t, bolts=4, seed=2)
    save(m11, "Modul11_Wood")
    m113 = wood(S, (198, 160, 110), 0.18, 7, seed=14)
    plate(m113, 0, 16 * t, 80 * t, 22 * t, bolts=8, seed=3)
    plate(m113, 108 * t, 16 * t, S, 22 * t, bolts=2, seed=4)
    save(m113, "Modul11_13_Wood")
    # E_Holzbeschlag: a crate's side, wood with the grain across inside a riveted iron frame 18 wide
    eh = wood(S, (150, 100, 60), 0.25, 5, seed=15, turns=1)
    fr = 18 * t
    frame = iron(S, S, mean=(130, 130, 136))
    m = np.zeros((S, S), bool)
    m[:fr] = m[-fr:] = True
    m[:, :fr] = m[:, -fr:] = True
    eh[m] = frame[m]
    eh[fr:fr + 2 * t, fr:S - fr] *= 0.5
    eh[fr:S - fr, fr:fr + 2 * t] *= 0.5
    for k in range(10):
        p = (k + 0.5) * S / 10
        for cx, cy in ((p, fr / 2), (p, S - fr / 2), (fr / 2, p), (S - fr / 2, p)):
            rivet(eh, cx, cy, 3 * t)
    save(eh, "E_Holzbeschlag")
    # Modul30's sides: boards in a grey frame (top bar 14, right post 16, bottom 4)
    m30 = wood(S, (126, 103, 83), 0.18, 6, seed=16)
    grey = rgb(128, 128, 128)
    m30[:14 * t] = iron(14 * t, S, mean=(128, 120, 112))
    m30[:, 112 * t:] = grey
    m30[S - 4 * t:] = iron(4 * t, S, mean=(128, 128, 132))
    save(m30, "Modul30_d_Seiten")


# --- markings ----------------------------------------------------------------------------------------------------------


def make_marks():
    # the sticks' stripes: red and cream, a period of 8 texels of 32, the red 4.5 of them centred half a texel down
    n = 32 * SCALE
    y = (np.arange(n) + 0.5) / n * 4
    dist = np.abs((y - 0.0625 + 0.5) % 1 - 0.5)
    red = np.clip((0.28 - dist) * 20 + 0.5, 0, 1)
    paint = rgb(214, 205, 176)[None] * (1 - red)[:, None] + rgb(139, 6, 5)[None] * red[:, None]
    grain = 0.94 + 0.12 * noise(n, 4, 31)[..., None]
    save(np.tile(paint[:, None, :], (1, n, 1)) * grain, "Stick_Stripes")
    # the target: cream, a black middle (radius 8 of 64), a red ring (15 to 23), faint rings outside
    n = 64 * SCALE
    yy, xx = np.mgrid[0:n, 0:n]
    r = np.hypot(xx - (n - 1) / 2, yy - (n - 1) / 2) / SCALE
    img = np.zeros((n, n, 3)) + rgb(240, 220, 170)
    img *= (0.95 + 0.05 * np.cos(r * 1.4))[..., None]
    edge = lambda a, b: np.clip((r - a) * 1.2, 0, 1) * np.clip((b - r) * 1.2, 0, 1)
    img = img * (1 - edge(15, 23))[..., None] + rgb(142, 25, 12) * edge(15, 23)[..., None]
    black = np.clip((8 - r) * 1.2, 0, 1)
    img = img * (1 - black)[..., None] + rgb(35, 34, 32) * black[..., None]
    save(img * (0.95 + 0.1 * noise(n, 4, 32))[..., None], "Target")
    # Modul18: a steel plate over the top half, wood with yellow and black warning stripes below
    n = 64 * SCALE
    m = wood(n, (150, 120, 80), 0.2, 4, seed=17)
    m[:n // 2] = photo("Metal009", n, (170, 170, 175), 0.12, 0.2, 0.2)[:n // 2]
    m[:3 * SCALE] = m[n // 2 - 3 * SCALE:n // 2] = rgb(110, 110, 115)
    m[:, n // 2 - SCALE:n // 2 + SCALE][: n // 2] *= 0.6
    yy, xx = np.mgrid[0:n, 0:n]
    s = ((xx * 0.55 - yy * 0.35) / (8 * SCALE)) % 1
    stripe = (yy > n // 2 + 2 * SCALE) & (xx > n * 0.2) & (xx < n * 0.85)
    yellow = stripe & (s < 0.32)
    blackm = stripe & (s >= 0.32) & (s < 0.45)
    m[yellow] = rgb(240, 225, 40)
    m[blackm] = rgb(20, 18, 15)
    save(m, "Modul18")
    # its grate: a white lattice with a dark round hole, the gaps transparent (bars 3 of every 9 texels)
    n = 64 * SCALE
    yy, xx = np.mgrid[0:n, 0:n]
    bars = ((xx % (9 * SCALE)) < 3 * SCALE) | ((yy % (9 * SCALE)) < 3 * SCALE)
    alpha = bars.astype(float)
    hole = np.hypot(xx - n / 2, yy - n / 2) < 10 * SCALE
    alpha[hole] = 1
    col = np.zeros((n, n, 3)) + rgb(210, 210, 210)
    col[hole] = rgb(60, 60, 60)
    save(col * (0.9 + 0.1 * noise(n, 4, 33))[..., None], "Modul18_Gitter", alpha=alpha)
    # the lantern's glass: warm yellow glowing inside a dark frame (measured: the frame 3 texels of 32, soft inside)
    n = 32 * SCALE
    yy, xx = np.mgrid[0:n, 0:n]
    d = np.minimum(np.minimum(xx, n - 1 - xx), np.minimum(yy, n - 1 - yy)) / SCALE
    glow = np.clip((d - 2.5) / 2.5, 0, 1)
    save(rgb(250, 225, 60) * (0.08 + 0.92 * glow)[..., None] * (0.92 + 0.08 * noise(n, 3, 34))[..., None],
         "Laterne_Glas")


# --- what reflects (environment maps) -----------------------------------------------------------------------------


def env(n, mean, rel, seed, streak=1.0):
    """A reflection map: soft clouds of light, streaked sideways as polished metal shows them."""
    a = noise(n, 3, seed, 5)
    b = noise(n, 6, seed + 1, 3)
    v = a * 0.7 + np.roll(b, n // 3, 1) * 0.3
    v = (v - v.mean()) / v.std()
    if streak:
        from scipy import ndimage
        v = ndimage.gaussian_filter(v, (0.5, n / 40 * streak), mode="wrap")
        v = (v - v.mean()) / v.std()
    m = rgb(*mean)
    return np.clip(m[None, None, :] * (1 + rel * v)[..., None], 0, 1)


def make_env():
    save(env(32 * SCALE, (121, 121, 142), 0.12, 41), "Rail_Environment")
    save(env(64 * SCALE, (104, 104, 114), 0.37, 42), "Trafo_Environment")
    save(env(64 * SCALE, (104, 104, 114), 0.37, 43), "Trafo_Anim_Environment")
    save(env(128 * SCALE, (141, 141, 141), 0.35, 44, 0.5), "Dome")
    save(env(128 * SCALE, (170, 179, 189), 0.10, 45, 0.2), "Misc_Ufo")


# --- the balls -----------------------------------------------------------------------------------------------------
# Wrapped round the sphere as the original's are (the texture's width once round, its height pole to pole): the stone
# grey green and pitted (125, 131, 115; 0.21), the wood upright boards (159, 126, 80; 0.34), the paper a printed sheet
# (189 grey; 0.22).


def make_balls():
    n = 128 * SCALE
    save(pits(stone_ball(n, 61), 61, count=150, sizes=(0.6, 1.6), depth=0.16), "Ball_Stone")
    woodball = wood(n, (159, 126, 80), 0.34, 9, seed=18)
    save(woodball, "Ball_Wood")
    save(woodball, "P_Ball_Wood")
    save(collage(n), "Ball_Paper")


def wood_ball(n, seed):
    """The wooden ball's skin, after the owner's concept (a reference, not copied): turned from one log, so the year
    rings stand round each pole (on the sphere's mapping, rows of the same latitude), light wood with darker late
    wood, fine radial cracks running out from the poles, a fine grain. Its mean the original's (159, 126, 80)."""
    rng = np.random.default_rng(seed)
    v = (np.arange(n) + 0.5) / n
    d = np.minimum(v, 1 - v)[:, None]  # 0 at a pole, 0.5 at the equator (by row)
    jitter = noise(n, 6, seed, 4)
    # rings: denser outwards, each a little uneven round its circle
    phase = (d * 2.0) ** 0.8 * 26.0 + 1.2 * (jitter - 0.5)
    ring = 0.5 + 0.5 * np.cos(phase * 2 * np.pi)
    late = np.clip(ring - 0.55, 0, 1) / 0.45
    early, dark = np.array([214, 172, 118]) / 255.0, np.array([150, 104, 58]) / 255.0
    img = early[None, None] + (dark - early)[None, None] * (0.95 * late ** 1.2)[..., None]
    grain = noise(n, n // 6, seed + 1, 2) - 0.5
    img = img * (1 + 0.10 * grain)[..., None] * (1 + 0.08 * (noise(n, 4, seed + 2, 4) - 0.5))[..., None]
    # cracks: thin dark lines along a meridian from a pole out to some latitude, wandering a little
    yy = np.arange(n)
    for _ in range(26):
        u0 = rng.uniform(0, n)
        top = rng.random() < 0.5
        length = rng.uniform(0.12, 0.38) * n
        wobble = rng.uniform(1.5, 4.0)
        for y in range(int(length)):
            row = y if top else n - 1 - y
            x = int(u0 + wobble * math.sin(y / (n * 0.04))) % n
            fade = 1 - y / length
            width = 1 + int(2 * fade)
            for dx in range(-width, width + 1):
                img[row, (x + dx) % n] *= 1 - 0.45 * fade * (1 - abs(dx) / (width + 1))
    mean = img.reshape(-1, 3).mean(0)
    return np.clip(img * (np.array([159, 126, 80]) / 255.0 / mean)[None, None], 0, 1)


def stone_ball(n, seed):
    """The stone ball's skin: one solid, worn stone (measured on the original's: mean colour (126, 132, 115), a fine
    speckled grain, lightness std 27 of 255, most of it from texel to texel), soft clouding and a few faint veins, no
    cracks (a photographed rock's cracks read as a ball fallen to pieces). Tiles both ways, so the sphere's seam does
    not show."""
    base = np.array([126, 132, 115]) / 255.0
    fine = noise(n, n // 2, seed, 2) - 0.5
    grain = noise(n, n // 8, seed + 1, 3) - 0.5
    cloud = noise(n, 4, seed + 2, 5) - 0.5
    # veins: thin soft lines where a warped field crosses a level
    warp = noise(n, 3, seed + 3, 5)
    vein = np.exp(-((np.sin(warp * 18.0) * 3.0) ** 2)) * (noise(n, 6, seed + 4, 3) > 0.55)
    light = 1 + 0.42 * fine + 0.22 * grain + 0.16 * cloud - 0.10 * vein
    tint = 1 + 0.04 * (noise(n, 5, seed + 5, 4)[..., None] - 0.5) * np.array([1.0, -0.6, 0.4])
    return np.clip(base[None, None] * light[..., None] * tint, 0, 1)


def pits(img, seed, count=260, sizes=(1.0, 3.0), depth=0.28):
    """Small hollows in a stone: dark on the side the light comes from, a light rim on the other (as the original's
    pitted stone ball, measured: about 260 pits, 1 to 3 texels of 128 across)."""
    n = img.shape[0]
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:n, 0:n]
    out = img.copy()
    for _ in range(count):
        cx, cy = rng.uniform(0, n, 2)
        r = rng.uniform(*sizes) * SCALE
        dx = (xx - cx + n / 2) % n - n / 2
        dy = (yy - cy + n / 2) % n - n / 2
        d = np.hypot(dx / 1.3, dy) / r
        m = d < 1.4
        if not m.any():
            continue
        shade = np.where(d < 1, 1 - depth + 0.25 * (dx + dy) / (2 * r), 1 + 0.12 * np.clip(1.4 - d, 0, 0.4) / 0.4)
        out[m] = out[m] * shade[m][:, None]
    return np.clip(out, 0, 1)


def newspaper(n):
    """A crumpled printed sheet: columns of grey lines of "text", a few headlines and pictures, on paper (our own
    drawing; the paper's crumples from the photo)."""
    paper = photo("Paper003", n, (222, 222, 218), 0.06, 0.1, 0.0)
    rng = np.random.default_rng(51)
    img = paper.copy()
    ink = np.zeros((n, n))
    cols = 4
    cw = n // cols
    for c in range(cols):
        x0 = c * cw + 3 * SCALE
        y = int(rng.integers(0, 6)) * SCALE
        while y < n:
            kind = rng.random()
            if kind < 0.12:  # a headline: a thick dark bar of words
                h = 4 * SCALE
                x = x0
                while x < x0 + cw - 8 * SCALE:
                    w = int(rng.integers(5, 14)) * SCALE
                    ink[y:y + h, x:min(x + w, x0 + cw - 6 * SCALE)] = 0.85
                    x += w + 2 * SCALE
                y += h + 3 * SCALE
            elif kind < 0.2:  # a picture
                h = int(rng.integers(12, 24)) * SCALE
                pic = noise(n, 3, int(rng.integers(0, 1000)), 3)[:h, :cw - 8 * SCALE]
                ink[y:y + h, x0:x0 + cw - 8 * SCALE] = 0.25 + 0.5 * pic[:min(h, n - y)]
                y += h + 3 * SCALE
            else:  # a paragraph of lines of words
                for _ in range(int(rng.integers(3, 9))):
                    x = x0
                    end = x0 + cw - 6 * SCALE - int(rng.integers(0, 3)) * 4 * SCALE
                    while x < end:
                        w = int(rng.integers(2, 7)) * SCALE
                        ink[y:y + SCALE, x:min(x + w, end)] = 0.55
                        x += w + SCALE
                    y += 2 * SCALE
                y += 2 * SCALE
    from scipy import ndimage
    ink = ndimage.gaussian_filter(np.clip(ink, 0, 1), 0.6, mode="wrap")
    img = img * (1 - 0.85 * ink)[..., None]
    return np.clip(img, 0, 1)


def collage(n):
    """Sheets of newspaper lying over each other at angles, each with a soft shadow, wrapping round (the original's
    paper ball is such a heap of sheets: about a dozen visible, a quarter of the side each)."""
    from scipy import ndimage

    sheet = newspaper(n)
    rng = np.random.default_rng(52)
    out = photo("Paper003", n, (200, 200, 196), 0.06, 0.1, 0.0)
    for _ in range(16):
        w, h = int(rng.uniform(0.3, 0.45) * n), int(rng.uniform(0.35, 0.55) * n)
        sx, sy = rng.integers(0, n - w), rng.integers(0, n - h)
        piece = sheet[sy:sy + h, sx:sx + w]
        ang = rng.uniform(-40, 40)
        im = Image.fromarray((piece * 255).astype(np.uint8)).rotate(ang, Image.BILINEAR, expand=True)
        mk = Image.new("L", (w, h), 255).rotate(ang, Image.BILINEAR, expand=True)
        p = np.asarray(im, np.float64) / 255
        a = np.asarray(mk, np.float64) / 255
        ph, pw = a.shape
        cx, cy = rng.integers(0, n), rng.integers(0, n)
        ys = (np.arange(ph) + cy) % n
        xs = (np.arange(pw) + cx) % n
        shadow = ndimage.gaussian_filter(a, 2.5 * SCALE)
        region = out[np.ix_(ys, xs)]
        region = region * (1 - 0.35 * shadow[..., None])
        region = region * (1 - a[..., None]) + p * a[..., None]
        out[np.ix_(ys, xs)] = region
    return np.clip(out, 0, 1)


# --- our levels' track ----------------------------------------------------------------------------------------------
# The track pieces of our own levels (levelkit.py) have textures of their own, in assets/own/Track/ (not stand-ins for
# an original's texture, so not in Textures/). They follow what makes the original's floors read, measured on its
# floor textures and levels (numbers only): a texture spans about 5 units of floor (Floor_Top_Borderless 4.96 units a
# repeat, Floor_Top_Profil 5.1, Floor_Side 5.0); the floor's edge is a strip of small bricks a twelfth of that wide
# (10 texels of 128: 0.4 units), a dark groove after it (2 texels), a shadow fading into the concrete (4 texels); the
# concrete is nearly plain (lightness std 0.03 of its mean, mean colour (195, 176, 146)); a profiled (hollow) floor's
# slope darkens towards its crease (0.28 at the deepest); the side is darker concrete (141, 121, 95), lighter
# downwards, its own brick strip along the top. On top of those, so that a long track does not read as one flat
# texture: slabs of 5 by 5 units with joints between them, each slab its own tone, dirt and wear (four slabs along a
# texture, 20 units before it repeats), three sets of slabs (a level takes one by its seed), darker grooves where the
# kerb meets the slabs, and the ambient shading a raised border casts on the floor beside it. Every joint falls on a
# texture's edge, so the textures tile without a seam, and levelkit.py maps them by the distance along the track, so
# one piece runs on into the next.

TRACK_OUT = os.path.join(os.path.dirname(HERE), "Track")
PX = 256 / 5.0  # texels a unit (a slab of 5 units is 256 texels)
SLABS = 4  # slabs along a texture: 20 units a repeat
# The owner's wish (2026-11-03, after playing level 1): darker and greyer than the original's sandy (195, 176, 146),
# more like concrete, and the slabs' grid less marked.
TRACK_FLOOR = (152, 147, 139)
TRACK_SIDE = (118, 112, 104)
JOINT_DEPTH = 0.26


def save_track(img, name):
    a = np.clip(img * 255 + 0.5, 0, 255).astype(np.uint8)
    os.makedirs(TRACK_OUT, exist_ok=True)
    Image.fromarray(a, "RGB").save(os.path.join(TRACK_OUT, name + ".png"), optimize=True)


def joint(img, axis, at_start=True, at_end=True, width=2, depth=0.5):
    """Darkens the texture's edges across `axis` (0: rows, the slabs' ends; 1: columns, their sides) into a joint
    half of it at each edge, so two slabs side by side show one groove `width` texels wide."""
    n = img.shape[axis]
    k = np.arange(n, dtype=np.float64)
    d = np.minimum(k + 0.5 if at_start else np.inf, n - k - 0.5 if at_end else np.inf)
    f = 1 - depth * np.exp(-((d / (width * 0.5)) ** 2)) - 0.12 * np.exp(-d / (width * 2.5))
    shape = [1, 1, 1]
    shape[axis] = n
    return img * f.reshape(shape)


def slab_variety(h, w, seed, tone=0.035, dirt=0.10, wear=0.06):
    """A slab's own look as a factor: its tone, blotches of dirt and lighter worn patches (tiling noise)."""
    rng = np.random.default_rng(seed)
    t = 1 + rng.uniform(-tone, tone)
    warm = np.array([1.0, rng.uniform(0.99, 1.01), rng.uniform(0.98, 1.01)])
    d = noise(max(h, w), 3, seed + 1, 5)[:h, :w]
    e = noise(max(h, w), 6, seed + 2, 3)[:h, :w]
    f = t * (1 - dirt * np.clip((d - 0.45) * 2.2, 0, 1)) * (1 + wear * np.clip((e - 0.55) * 3, 0, 1))
    return f[..., None] * warm


def track_concrete(seed, h, w, mean=TRACK_FLOOR, rel=0.045):
    """A column of SLABS slabs of concrete (h texels long, w across), each cut from another part of the photo, its own
    tone, dirt and wear, with joints between them and at its sides."""
    src = photo("Concrete047A", 1024, mean, rel, 0.25, 0.7, fix=False)
    rng = np.random.default_rng(seed)
    n = h // SLABS
    out = np.zeros((h, w, 3))
    for k in range(SLABS):
        y0, x0 = rng.integers(0, 1024 - n), rng.integers(0, 1024 - w)
        sl = src[y0:y0 + n, x0:x0 + w].copy()
        sl = sl * slab_variety(n, w, seed * 31 + k)
        out[k * n:(k + 1) * n] = joint(sl, 0, depth=JOINT_DEPTH)
    return out


def kerb(seed, w, bricks=True, shade=0.30):
    """The edge band (w texels across, u = 0 at the floor's edge): a strip of bricks, a groove, then concrete shaded
    towards the edge (what a border or the brick strip casts), its slabs' joints where the top's are, a groove at its
    inner side where it meets the slabs."""
    h = int(PX * 5) * SLABS
    img = track_concrete(seed, h, w)
    x = np.arange(w) / PX
    strip, groove = (0.4, 0.08) if bricks else (0.0, 0.0)
    img *= (1 - shade * np.exp(-np.maximum(x - strip - groove, 0) / 0.35))[None, :, None]
    if bricks:
        s, g = int(round(strip * PX)), max(2, int(round(groove * PX)))
        img[:, :s] = brick_strip(h, s, seed + 7)
        img[:, s:s + g] = rgb(96, 90, 84) * (0.9 + 0.2 * noise(h, 8, seed + 5)[:, :g, None])
    return joint(img, 1, at_start=False, width=2, depth=0.35)


def track_slope(seed, w):
    """A profiled floor's slope (u = 0 at its high outer edge, 1 at its crease): the brick strip and groove at the edge,
    the concrete darkening down towards the crease (the original's profiled top: 0.28 at the deepest)."""
    img = kerb(seed, w, bricks=True, shade=0.18)
    x = np.arange(w) / w
    img *= (1 - 0.26 * x ** 1.6)[None, :, None]
    return joint(img, 1, at_start=False, width=3, depth=0.45)


def track_side(seed):
    """The floor's sides (u = 0 at the top edge, 5 units down to 1): a brick strip along the top, a groove, darker
    concrete lighter downwards."""
    h = int(PX * 5) * SLABS
    w = int(PX * 5)
    src = photo("Concrete012", 1024, TRACK_SIDE, 0.10, 0.3, 0.4, fix=False)
    rng = np.random.default_rng(seed)
    n = h // SLABS
    img = np.zeros((h, w, 3))
    for k in range(SLABS):
        y0, x0 = rng.integers(0, 1024 - n), rng.integers(0, 1024 - w)
        img[k * n:(k + 1) * n] = joint(src[y0:y0 + n, x0:x0 + w] * slab_variety(n, w, seed * 17 + k, dirt=0.15), 0,
                                       width=2, depth=0.3)
    img *= np.linspace(0.84, 1.12, w)[None, :, None]
    img = weather(img, seed + 3, along_rows=True)
    s, g = int(round(0.4 * PX)), 3
    img[:, :s] = brick_strip(h, s, seed + 9)
    img[:, s:s + g] = rgb(70, 66, 62)
    # the edge under the strip worn lighter in places
    wear = np.clip((noise(h, 12, seed + 11)[:, :1] - 0.55) * 3, 0, 1)
    img[:, s + g:s + g + 6] *= (1 + 0.12 * wear)[:, :, None]
    return img


def weather(img, seed, along_rows):
    """Rain streaks run down a concrete face from its top edge: darker stains, narrow and uneven, fading as they run
    down, and a little grime all over. `along_rows`: the face's top edge is the texture's first column and down runs
    along its rows' x (the track's sides); else its top is the first row and down runs along y (a column)."""
    h, w = img.shape[:2]
    n = max(h, w)
    if along_rows:
        streak = np.clip((noise(n, 48, seed, 3)[:h, :w].mean(1, keepdims=True) - 0.5) * 4, 0, 1)
        fade = np.exp(-np.arange(w) / (w * 0.45))[None, :]
        dark = 0.22 * streak * fade
    else:
        streak = np.clip((noise(n, 48, seed, 3)[:h, :w].mean(0, keepdims=True) - 0.5) * 4, 0, 1)
        fade = np.exp(-np.arange(h) / (h * 0.6))[:, None]
        dark = 0.22 * streak * fade
    grime = 0.08 * noise(n, 6, seed + 1, 4)[:h, :w]
    return img * (1 - dark - grime + 0.04)[..., None]


def track_rail(seed):
    """The rails' steel (u round a tube, v along it, 4 units a repeat): worn galvanised steel, brushed along the tube,
    a light rust tint in patches and along streaks, and its relief (a normal map: the brushing) for the highlight to
    run along; after the owner's concept of the rails (a reference)."""
    h, w = 512, 128
    brush = np.repeat(noise(w, 96, seed, 2)[:1], h, 0)  # lines along v
    brush = brush + 0.5 * np.repeat(noise(w, 32, seed + 1, 3)[:1], h, 0)
    along = noise(h, 6, seed + 2, 4)[:, :1]
    base = np.array([0.60, 0.62, 0.64])
    img = base[None, None] * (0.82 + 0.25 * (brush - 0.6) + 0.10 * (along - 0.5))[..., None]
    rust = np.clip((noise(h, 5, seed + 3, 5)[:, :w] - 0.58) * 3.0, 0, 1)
    rust = np.maximum(rust, 0.6 * np.clip((noise(h, 40, seed + 4, 3)[:, :w] - 0.7) * 4.0, 0, 1))
    tint = np.array([0.55, 0.36, 0.22])
    img = img * (1 - 0.45 * rust[..., None]) + tint[None, None] * (0.45 * rust[..., None])
    height = brush * 0.6 + rust * 0.4
    gx = np.roll(height, -1, 1) - np.roll(height, 1, 1)
    gy = np.roll(height, -1, 0) - np.roll(height, 1, 0)
    nrm = np.stack([-gx * 2.0, -gy * 2.0, np.ones_like(gx)], -1)
    nrm /= np.linalg.norm(nrm, axis=-1, keepdims=True)
    return np.clip(img, 0, 1), nrm * 0.5 + 0.5


def track_column(seed):
    """The columns under the floors (u round, v down from the top, 5 units a repeat): the side's concrete, streaked
    from the top, a joint where one cast ring meets the next."""
    n = int(PX * 5)
    src = photo("Concrete012", 1024, TRACK_SIDE, 0.12, 0.3, 0.4, fix=False)
    rng = np.random.default_rng(seed)
    y0, x0 = rng.integers(0, 1024 - n), rng.integers(0, 1024 - n)
    img = src[y0:y0 + n, x0:x0 + n] * slab_variety(n, n, seed, dirt=0.18)
    img = weather(img, seed + 1, along_rows=False)
    return joint(img, 0, width=3, depth=0.35)


def make_track():
    h = int(PX * 5) * SLABS
    for k, name in enumerate("ABC"):
        save_track(joint(track_concrete(200 + 10 * k, h, int(PX * 5)), 1, width=2, depth=JOINT_DEPTH),
                   f"Track_Top_{name}")
        save_track(kerb(300 + 10 * k, int(PX * 1.25)), f"Track_Kerb_{name}")
        save_track(kerb(400 + 10 * k, int(PX * 1.25), bricks=False, shade=0.38), f"Track_Shade_{name}")
        save_track(track_slope(500 + 10 * k, int(PX * 2.5)), f"Track_Slope_{name}")
    save_track(track_side(600), "Track_Side")
    save_track(track_column(700), "Track_Column")
    rail, rail_n = track_rail(800)
    save_track(rail, "Track_Rail")
    save_track(rail_n, "Track_Rail_n")
    # the borders' bricks are the menu scene's (make_menu_scene.py), kept here by the track's names too, so that every
    # texture our levels name as the game's (`game:<name>`, levelkit.py) is one of the game's own
    menu = os.path.join(os.path.dirname(HERE), "3D Entities", "MenuLevel", "textures")
    for name in ("floor_border", "floor_border_n"):
        with open(os.path.join(menu, name + ".png"), "rb") as src, \
                open(os.path.join(TRACK_OUT, f"Track_{name}.png"), "wb") as dst:
            dst.write(src.read())


def radial(n, power=2.0):
    """1 at the centre, 0 at the edge of the square (a soft round blob)."""
    y, x = (np.arange(n) + 0.5) / n * 2 - 1, (np.arange(n) + 0.5) / n * 2 - 1
    r = np.sqrt(y[:, None] ** 2 + x[None, :] ** 2)
    return np.clip(1 - r, 0, 1) ** power


def make_extras():
    """What the copy supplies that needs no photograph: glows, shadows, light cones, the cloud layer, a few plain
    surfaces. Drawn from the measured mean colour and the shape of the original's texture of the name (analysed, never
    copied), at twice its size, so a level made for the copy's textures looks right without the copy. The additive and
    shadow ones are dark or transparent where they should vanish, as the original's are."""
    S2 = SCALE
    # dark layers drawn additively: nearly black, a little cloud in them
    n = 128 * S2
    save(np.clip(rgb(19, 19, 19)[None, None, :] * (0.55 + 0.9 * noise(n, 4, 71, 5))[..., None], 0, 1), "SkyLayer")
    save(np.clip(rgb(27, 27, 27)[None, None, :] * (0.4 + 1.2 * noise(n, 3, 72, 5))[..., None], 0, 1), "Sky_Vortex")
    # the lanterns' cone: warm light fading along its length
    n = 16 * S2
    fade = np.linspace(1.05, 0.1, n)[:, None] * np.ones((1, n))
    save(fade[..., None] * rgb(125, 106, 2)[None, None, :], "Laterne_Verlauf")
    # shadows: black with a soft alpha
    n = 16 * S2
    save(np.zeros((n, n, 3)), "Laterne_Schatten", alpha=0.05 + 0.2 * radial(n, 0.6))
    n = 64 * S2
    save(np.zeros((n, n, 3)), "Trafo_Shadow_Big", alpha=0.3 + 0.2 * radial(n, 0.5))
    save(np.zeros((n, n, 3)), "DomeShadow", alpha=0.25 + 0.4 * radial(n, 0.6))
    # the stick's foot: bluish metal reflection fading out downwards
    save(np.ones((n, n, 3)) * rgb(121, 122, 143)[None, None, :] * (0.95 + 0.1 * noise(n, 3, 73))[..., None],
         "Stick_Bottom", alpha=np.linspace(1.0, 0.1, n)[:, None] * np.ones((1, n)))
    # glows and flashes (additive): a bright centre fading to black
    save(radial(n, 1.6)[..., None] * rgb(170, 62, 62)[None, None, :], "FloorGlow")
    save(radial(n, 1.2)[..., None] * rgb(60, 60, 60)[None, None, :], "P_Extra_Life_Shadow")
    n = 128 * S2
    save(radial(n, 1.4)[..., None] * rgb(95, 75, 75)[None, None, :], "Misc_UFO_Flash")
    # the extra life's oil drop: dark violet, swirled
    save(np.clip(rgb(27, 15, 28)[None, None, :] * (0.5 + 1.0 * noise(n, 3, 74, 4))[..., None], 0, 1), "P_Extra_Life_Oil")
    # reflections of the balloon and the dome
    save(env(n, (201, 201, 201), 0.08, 75, 0.3), "PE_Ufo_env")
    save(env(n, (165, 163, 202), 0.07, 76, 0.3), "DomeEnvironment")
    # plain surfaces: the balloon's cloth and platform, a module's wall and floor, brown stone
    def surface(mean, rel, seed, cells, size, planks=0):
        v = noise(size, cells, seed, 4)
        v = (v - v.mean()) / v.std()
        img = np.clip(rgb(*mean)[None, None, :] * (1 + rel * v)[..., None], 0, 1)
        if planks:
            gap = (np.arange(size) % (size // planks)) < max(1, size // 128)
            img[gap, :, :] *= 0.55
        return img
    save(surface((183, 174, 149), 0.10, 77, 6, 128 * S2, planks=8), "PE_Bal_balloons")
    save(surface((104, 98, 88), 0.14, 78, 5, 256 * S2, planks=6), "PE_Bal_platform")
    save(surface((151, 141, 125), 0.09, 79, 8, 256 * S2), "Modul03_Wall")
    save(surface((179, 152, 118), 0.07, 80, 4, 128 * S2, planks=8), "Modul03_Floor")
    save(surface((135, 112, 92), 0.12, 81, 6, 128 * S2), "Stone_Brown")


STEPS = {"floors": make_floors, "track": make_track, "stone": make_stone, "wood": make_wood, "marks": make_marks, "env": make_env,
         "balls": make_balls, "extras": make_extras}


def main(argv):
    names = argv[1:] or list(STEPS)
    for n in names:
        if n not in STEPS:
            print(__doc__)
            return 2
        STEPS[n]()
    print(OUT, "made:", ", ".join(names))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
