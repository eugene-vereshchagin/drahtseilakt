#!/usr/bin/env python3
"""The three balls' skins, drawn by us (no photograph, nothing of the original's): the wooden ball turned from a blank
glued up of blocks, the stone ball of grey-white marble, the paper ball a crumpled newspaper. Each is drawn in space
and read off along the sphere's directions (the texture's width once round, its height pole to pole, as ownkit's sphere
has its coordinates), so nothing pinches at the poles or runs along the meridians, and comes with its relief (a normal
map, tangent space, green towards -v) for the parts to name (make_parts.py). make_textures.py saves the colours as the
pack's Ball_Wood, Ball_Stone and Ball_Paper; make_parts.py the same pictures and their reliefs into the parts' folders.

Needs Python 3 with numpy. The output is deterministic (fixed seeds).
"""

import math

import numpy as np

R = 2.0  # the balls' radius


def rgb(*v):
    return np.array(v, np.float64) / 255.0


def directions(w, h):
    """Each texel's direction on the sphere (h × w × 3), as ownkit.ellipsoid maps it: u round from +x towards +z, v
    from the top (+y) down."""
    ph = 2 * np.pi * (np.arange(w) + 0.5) / w
    th = np.pi * (np.arange(h) + 0.5) / h
    st, ct = np.sin(th)[:, None], np.cos(th)[:, None]
    return np.stack([st * np.cos(ph)[None], ct * np.ones((1, w)), st * np.sin(ph)[None]], -1)


def noise3(p, freq, seed, octaves=4):
    """Fractal value noise in 0..1 at points p (... × 3)."""
    rng = np.random.default_rng(seed)
    out = np.zeros(p.shape[:-1])
    amp, total = 1.0, 0.0
    for _ in range(octaves):
        g = rng.random((32, 32, 32))
        q = p * freq + rng.random(3) * 32
        i = np.floor(q).astype(np.int64)
        f = q - i
        f = f * f * (3 - 2 * f)
        i0, i1 = i % 32, (i + 1) % 32
        acc = 0.0
        for dx, wx in ((0, 1 - f[..., 0]), (1, f[..., 0])):
            ix = (i1 if dx else i0)[..., 0]
            for dy, wy in ((0, 1 - f[..., 1]), (1, f[..., 1])):
                iy = (i1 if dy else i0)[..., 1]
                acc = acc + wx * wy * (g[ix, iy, i0[..., 2]] * (1 - f[..., 2]) + g[ix, iy, i1[..., 2]] * f[..., 2])
        out += acc * amp
        total += amp
        amp *= 0.5
        freq *= 2.0
    return out / total


def rotation(seed):
    """A fixed turn of space (so that what is drawn does not line up with the texture's poles)."""
    q, _ = np.linalg.qr(np.random.default_rng(seed).normal(size=(3, 3)))
    return q


def relief(height, strength=1.0):
    """The normal map of a height (in the ball's units) over the sphere's texture: its slope east and down in real
    lengths, so the relief is as deep at the poles as at the equator."""
    h, w = height.shape
    st = np.sin(np.pi * (np.arange(h) + 0.5) / h)[:, None]
    east = (np.roll(height, -1, 1) - np.roll(height, 1, 1)) / (2 * 2 * np.pi * R * np.maximum(st, 0.05) / w)
    down = np.gradient(height, axis=0) / (np.pi * R / h)
    n = np.stack([-east * strength, down * strength, np.ones_like(east)], -1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    return n * 0.5 + 0.5


def fibonacci(n, seed, jitter=0.25):
    rng = np.random.default_rng(seed)
    out = []
    for i in range(n):
        y = 1 - 2 * (i + 0.5) / n
        r = math.sqrt(1 - y * y)
        a = i * math.pi * (3 - math.sqrt(5))
        v = np.array([r * math.cos(a), y, r * math.sin(a)]) + rng.normal(size=3) * jitter
        out.append(v / np.linalg.norm(v))
    return np.array(out)


def _hash(ids, seed, n=1):
    """n numbers in 0..1 for each integer triple (the same triple, the same numbers)."""
    a = ids[..., 0] * 73856093 ^ ids[..., 1] * 19349663 ^ ids[..., 2] * 83492791 ^ (seed * 2654435761)
    out = []
    for k in range(n):
        a = (a ^ (a >> 13)) * 1274126177 + 12345 + k * 668265263
        a = a ^ (a >> 16)
        out.append((a & 0xFFFFF) / float(0x100000))
    return np.stack(out, -1)


# --- the wooden ball ---------------------------------------------------------------------------------------------


def wood(w=1024, h=512, seed=18):
    """The wooden ball: turned from a blank glued up of blocks, as a turner's laminated ball is. Layers a unit thick,
    each of staves side by side, the staves of one layer lying across those of the next, each stave cut into blocks:
    every block its own piece of wood (its grain along its stave, a little askew; its year rings round its own pith;
    its own tone), dark glue joints between them, the lathe's fine rings round the turning axis, small dents and
    scratches of use. Returns the colour and the height (units) of its relief."""
    d = directions(w, h)
    p = (d * R) @ rotation(seed).T
    # the block each point lies in: layer along y, staves along x in even layers and z in odd ones
    layer = np.floor(p[..., 1] / 1.0 + 0.5).astype(np.int64)
    odd = (layer % 2) == 1
    along = np.where(odd, p[..., 2], p[..., 0])
    across = np.where(odd, p[..., 0], p[..., 2])
    shift = _hash(np.stack([layer, layer * 0, layer * 0], -1), seed, 2)
    sw = 1.25  # a stave's width
    stave = np.floor(across / sw + shift[..., 0]).astype(np.int64)
    off = _hash(np.stack([layer, stave, stave * 0], -1), seed + 1, 1)[..., 0]
    bl = 2.7  # a block's length
    block = np.floor(along / bl + off).astype(np.int64)
    ids = np.stack([layer, stave, block], -1)
    rnd = _hash(ids, seed + 2, 8)
    # how near the nearest joint is (units)
    fy = np.abs(p[..., 1] / 1.0 + 0.5 - layer - 0.5) * 1.0
    fs = np.abs(across / sw + shift[..., 0] - stave - 0.5) * sw
    fb = np.abs(along / bl + off - block - 0.5) * bl
    joint = np.minimum(np.minimum(0.5 - fy, sw / 2 - fs), bl / 2 - fb)
    # the block's own wood: its grain along the stave, askew by up to a sixth; its rings round a pith beside it
    up = np.zeros_like(p)
    up[..., 1] = 1.0
    g_along = np.where(odd[..., None], np.array([0, 0, 1.0]), np.array([1.0, 0, 0]))
    g_side = np.where(odd[..., None], np.array([1.0, 0, 0]), np.array([0, 0, 1.0]))
    grain = g_along + (rnd[..., 0:1] - 0.5) * 0.34 * g_side + (rnd[..., 1:2] - 0.5) * 0.34 * up
    grain /= np.linalg.norm(grain, axis=-1, keepdims=True)
    ang = rnd[..., 2] * 2 * np.pi
    pith = (np.cos(ang)[..., None] * g_side + np.sin(ang)[..., None] * up) * (1.2 + 2.5 * rnd[..., 3:4])
    rel = p - pith
    rel = rel - grain * (rel * grain).sum(-1, keepdims=True)
    warp = noise3(p + rnd[..., 4:7] * 9.0, 0.7, seed + 3, 3) - 0.5
    rings = np.linalg.norm(rel, axis=-1) * (5.0 + 4.0 * rnd[..., 7]) + 1.8 * warp
    t = rings - np.floor(rings)
    late = np.clip((t - 0.45) / 0.5, 0, 1) ** 2 * np.clip((1.0 - t) / 0.05, 0, 1)  # dark late wood, a sharp end
    # the fibres: fine streaks along the grain
    s = (p * grain).sum(-1, keepdims=True)
    q = (p - grain * s) * 1.0 + grain * s * 0.045
    fibre = noise3(q + rnd[..., 4:7] * 5.0, 16.0, seed + 4, 3) - 0.5
    pores = np.clip(noise3(q + rnd[..., 4:7] * 3.0, 34.0, seed + 5, 2) - 0.66, 0, 1) * 5.0
    # each block's tone: honey to brown, some redder, a few dark
    tone = 0.74 + 0.42 * rnd[..., 5]
    tone = np.where(rnd[..., 6] > 0.9, tone * 0.8, tone)
    warm = np.stack([np.ones_like(tone), 0.94 + 0.08 * rnd[..., 3], 0.84 + 0.20 * rnd[..., 4]], -1)
    early, dark = rgb(176, 116, 64), rgb(104, 60, 30)
    img = early + (dark - early) * (0.34 * late + 0.25 * np.clip(pores, 0, 1))[..., None]
    img = img * (tone * (1 + 0.34 * fibre))[..., None] * warm
    # the lathe's marks round the turning axis, the dirt of use, dents and scratches
    axis = rotation(seed + 6)[0]
    lat = np.arccos(np.clip(d @ axis, -1, 1))
    lathe = np.sin(lat * 150 + 4 * noise3(d, 1.5, seed + 7, 2)) * 0.5 + (noise3(d * [1, 1, 1], 40.0, seed + 8, 1) - 0.5)
    lathe = lathe * (0.4 + 0.6 * noise3(d, 2.0, seed + 9, 2))
    dirt = noise3(d, 1.6, seed + 10, 4) - 0.5
    img = img * (1 + 0.035 * lathe + 0.10 * dirt)[..., None]
    rng = np.random.default_rng(seed + 11)
    dent = np.zeros(p.shape[:-1])
    for c in fibonacci(46, seed + 12, 0.6):
        r = rng.uniform(0.012, 0.03)
        dent = np.maximum(dent, np.clip(1 - np.arccos(np.clip(d @ c, -1, 1)) / r, 0, 1))
    scratch = np.zeros(p.shape[:-1])
    for _ in range(14):
        n = rng.normal(size=3)
        n /= np.linalg.norm(n)
        c = rng.normal(size=3)
        c /= np.linalg.norm(c)
        line = np.clip(1 - np.abs(d @ n) / 0.006, 0, 1) * np.clip(1 - np.arccos(np.clip(d @ c, -1, 1)) / rng.uniform(0.25, 0.6), 0, 1)
        scratch = np.maximum(scratch, line)
    img = img * (1 - 0.30 * dent ** 0.5)[..., None] + 0.10 * scratch[..., None]
    # the glue joints: a thin dark line, the wood a little dirtier beside it
    line = np.clip(1.5 - joint / 0.006, 0, 1)
    halo = np.clip(1 - joint / 0.05, 0, 1)
    img = img * (1 - 0.34 * line - 0.05 * halo * (1 - line))[..., None]
    height = (0.012 * late - 0.010 * np.clip(pores, 0, 1) + 0.004 * fibre + 0.004 * lathe - 0.03 * dent
              - 0.012 * np.clip(1 - joint / 0.02, 0, 1) ** 2 - 0.008 * scratch)
    return np.clip(img, 0, 1), height


# --- the stone ball ----------------------------------------------------------------------------------------------


def marble(w=1024, h=512, seed=61):
    """The stone ball: a ball of grey-white marble with a matte polish. A light, faintly clouded ground (no large
    blotches: the clouding is weak and fine), dark grey veins of several widths wandering through it, branching, a few
    of them faintly ochre, a fine crystalline grain, and chips knocked out of it: small rough hollows, lighter inside.
    Returns the colour and the height (units) of its relief."""
    d = directions(w, h)
    p = (d * R) @ rotation(seed).T
    wv = np.stack([noise3(p + 11.0 * k, 0.55, seed + k, 4) for k in range(3)], -1) - 0.5

    # the veins: fractures through the block, each a wavering sheet that the ball's surface cuts in a wandering line,
    # of its own width, fading in and out along its run; fine side veins fork off the strong ones
    rng = np.random.default_rng(seed + 5)
    vein = np.zeros(p.shape[:-1])
    bloom = np.zeros(p.shape[:-1])
    for k in range(11):
        n = rng.normal(size=3)
        n /= np.linalg.norm(n)
        bend = noise3(p + 7.0 * k, 0.5, seed + 20 + k, 4) - 0.5
        t = np.abs(p @ n - rng.uniform(-1.5, 1.5) + 1.5 * bend + 0.5 * (wv @ n))
        width = rng.uniform(0.008, 0.03) if k < 7 else rng.uniform(0.004, 0.008)
        fade = np.clip((noise3(p + 3.0 * k, 0.6, seed + 40 + k, 3) - 0.38) * 5.0, 0, 1)
        strength = (rng.uniform(0.5, 1.0) if k < 7 else 0.45) * fade
        # (its width swells and thins along its run)
        width = width * (0.4 + 1.6 * noise3(p + 5.0 * k, 1.1, seed + 60 + k, 2))
        vein = np.maximum(vein, strength * np.clip(1.3 - t / width, 0, 1))
        bloom = np.maximum(bloom, strength * np.exp(-(t / (width * 7 + 0.07)) ** 2))
    run = bloom
    # the clouding: streaked along the stone's bedding, fine, never a large patch
    cloud = noise3((p + wv * 1.2) * [2.6, 0.5, 1.1], 1.3, seed + 10, 4) - 0.5
    grain = noise3(p, 30.0, seed + 11, 2) - 0.5
    ground, shade = rgb(208, 208, 204), rgb(150, 155, 163)
    dark, ochre = rgb(92, 96, 104), rgb(150, 124, 92)
    img = ground + (shade - ground) * np.clip(0.25 + 2.4 * cloud + 0.5 * run, 0, 1)[..., None] * 0.6
    img = img * (1 + 0.05 * grain)[..., None]
    tint = np.clip((noise3(p, 0.5, seed + 12, 2) - 0.55) * 6, 0, 1)[..., None]
    img = img + ((dark + (ochre - dark) * tint) - img) * (0.80 * vein)[..., None]
    # a soft grey bloom beside the veins
    img = img * (1 - 0.20 * bloom)[..., None]
    # chips: rough hollows with a ragged outline
    rng = np.random.default_rng(seed + 13)
    rag = noise3(d, 14.0, seed + 14, 2) - 0.5
    chip = np.zeros(p.shape[:-1])
    for c in fibonacci(12, seed + 15, 0.7):
        r = rng.uniform(0.03, 0.08)
        a = np.arccos(np.clip(d @ c, -1, 1)) / r
        chip = np.maximum(chip, np.clip((1 - a + 1.1 * rag) * 2.5, 0, 1))
    rough = noise3(d, 60.0, seed + 16, 2) - 0.5
    img = img * (1 - 0.07 * chip + 0.22 * chip * rough)[..., None]
    height = -0.016 * chip + 0.012 * chip * rough - 0.004 * vein + 0.0015 * grain
    return np.clip(img, 0, 1), height


# --- the paper ball ----------------------------------------------------------------------------------------------

# The seven strokes of a letter's cell (top, middle and bottom bars, the four half stems) and the sets of them that
# read as capitals: the print is letter shapes in no order, no word of any language.
_LETTERS = ["1110111", "1001010", "1111010", "1101010", "0101111", "0011010", "1011111", "1101111", "1011101",
            "0011111", "1011011", "1110101", "0111101", "1000101", "0111010", "1111111"]


def _letter(ink, x, y, cw, ch, s, pattern):
    """One capital of strokes s thick in the cell cw × ch at x, y."""
    H, W = ink.shape
    top, mid, bot, ul, ur, ll, lr = (c == "1" for c in pattern)

    def bar(x0, y0, x1, y1):
        ys = np.arange(int(y0), int(y1)) % H
        xs = np.arange(int(x0), int(x1)) % W
        ink[np.ix_(ys, xs)] = 1.0

    half = ch // 2
    if top:
        bar(x, y, x + cw, y + s)
    if mid:
        bar(x, y + half - s // 2, x + cw, y + half - s // 2 + s)
    if bot:
        bar(x, y + ch - s, x + cw, y + ch)
    if ul:
        bar(x, y, x + s, y + half)
    if ur:
        bar(x + cw - s, y, x + cw, y + half)
    if ll:
        bar(x, y + half, x + s, y + ch)
    if lr:
        bar(x + cw - s, y + half, x + cw, y + ch)


def _headline(ink, rng, x0, x1, y, ch, bold=0.22):
    """A line of capitals ch high from x0 to at most x1, in words."""
    cw, s = int(ch * 0.62), max(2, int(ch * bold))
    x = x0
    while True:
        n = int(rng.integers(2, 8))
        if x + n * (cw + s) > x1:
            break
        for _ in range(n):
            _letter(ink, x, y, cw, ch, s, _LETTERS[int(rng.integers(len(_LETTERS)))])
            x += cw + s
        x += cw


def _text(ink, rng, x0, x1, y0, y1, pitch=15, thick=5, tone=0.62):
    """A column of text: lines of words (bars broken into letters' widths), justified, in paragraphs."""
    H, W = ink.shape
    y = y0
    first = True
    while y + thick <= y1:
        last = rng.random() < 0.14
        x = x0 + (pitch if first else 0)
        end = x1 - (int(rng.integers(2, 6)) * pitch if last else 0)
        while x < end - 6:
            n = min(int(rng.integers(8, 34)), end - x)
            xs = np.arange(x, x + n) % W
            ink[np.ix_(np.arange(y, y + thick) % H, xs)] = tone * (0.75 + 0.25 * rng.random(n))[None, :]
            # the gaps between letters, and their ascenders
            for gx in range(x + int(rng.integers(3, 6)), x + n, int(rng.integers(4, 7))):
                ink[np.arange(y, y + thick) % H, gx % W] *= 0.45
                if rng.random() < 0.3:
                    ink[np.arange(y - 3, y) % H, (gx + 1) % W] = tone
            x += n + int(rng.integers(4, 8))
        first = last
        y += pitch + (pitch // 2 if last else 0)


def _photo(ink, rng, x0, x1, y0, y1):
    """A printed photograph: a landscape in halftone (a sky, a low sun, hills, a dark foreground) in a thin frame."""
    H, W = ink.shape
    hh, ww = y1 - y0, x1 - x0
    yy, xx = np.mgrid[0:hh, 0:ww] / float(hh)
    sky = 0.18 + 0.30 * yy
    sun = np.hypot(xx - rng.uniform(0.3, ww / hh - 0.3), yy - 0.3) < 0.13
    hill = yy > 0.62 - 0.16 * np.sin(xx * rng.uniform(2.5, 4.5) + rng.uniform(0, 6)) - 0.06 * np.sin(xx * 9 + 1)
    front = yy > 0.86 - 0.05 * np.sin(xx * 5 + rng.uniform(0, 6))
    grey = np.where(front, 0.85, np.where(hill, 0.5, np.where(sun, 0.02, sky * 0.8)))
    py, px = np.mgrid[0:hh, 0:ww]
    dots = 0.5 + 0.25 * (np.cos((px + py) * np.pi / 3) + np.cos((px - py) * np.pi / 3))
    pic = np.clip((grey - dots) * 3.0 + 0.5, 0, 1) * 0.6 + grey * 0.4
    frame = (py < 3) | (py >= hh - 3) | (px < 3) | (px >= ww - 3)
    ink[np.ix_(np.arange(y0, y1) % H, np.arange(x0, x1) % W)] = np.where(frame, 1.0, pic)


def page(n=1024, seed=51):
    """A newspaper's page that repeats both ways, as ink (0 paper, 1 black): the masthead in big capitals between
    rules with a date line, then stories in five columns under headlines of two sizes, two photographs with their
    captions, column rules. Letter shapes only, in no language."""
    rng = np.random.default_rng(seed)
    ink = np.zeros((n, n))
    m, cols = 14, 5
    cw = (n - m) // cols

    def col(c):
        return m + c * cw, m + (c + 1) * cw - m

    def rule(y, thick=3, x0=m, x1=n - m):
        ink[y:y + thick, x0:x1] = 1.0

    # the masthead
    rule(8, 5)
    _headline(ink, rng, m + 40, n - m, 26, 92, 0.26)
    rule(130, 2)
    _text(ink, rng, m, n - m, 138, 150, pitch=12, thick=5, tone=0.6)
    rule(154, 5)
    # the first story: a headline over three columns with a photograph, two columns beside it
    _headline(ink, rng, m, col(2)[1], 172, 50)
    _headline(ink, rng, m, col(2)[1], 232, 50)
    _photo(ink, rng, col(0)[0], col(1)[1], 296, 480)
    _text(ink, rng, col(0)[0], col(1)[1], 488, 512, tone=0.5)
    _text(ink, rng, col(2)[0], col(2)[1], 296, 560)
    _text(ink, rng, col(0)[0], col(0)[1], 524, 560)
    _text(ink, rng, col(1)[0], col(1)[1], 524, 560)
    for c in (3, 4):
        _headline(ink, rng, col(c)[0], col(c)[1], 172, 24, 0.2)
        _text(ink, rng, col(c)[0], col(c)[1], 206, 380)
        _headline(ink, rng, col(c)[0], col(c)[1], 392, 20, 0.2)
        _text(ink, rng, col(c)[0], col(c)[1], 422, 560)
    for c in range(3, 5):
        ink[172:560, col(c)[0] - m // 2 - 1:col(c)[0] - m // 2 + 1] = 0.8
    # the second: a headline right across, five columns, a photograph in the last three
    rule(572, 3)
    _headline(ink, rng, m, n - m, 590, 66, 0.24)
    rule(670, 2)
    _photo(ink, rng, col(2)[0], col(4)[1], 688, 880)
    _text(ink, rng, col(2)[0], col(4)[1], 888, 912, tone=0.5)
    for c in range(5):
        y = 688
        if c < 2:
            _headline(ink, rng, col(c)[0], col(c)[1], y, 22, 0.2)
            y += 34
            _text(ink, rng, col(c)[0], col(c)[1], y, n - 8)
        else:
            _text(ink, rng, col(c)[0], col(c)[1], 926, n - 8)
        if c:
            ink[688:n - 8, col(c)[0] - m // 2 - 1:col(c)[0] - m // 2 + 1] = 0.8
    return ink


PAPER_FACES = 30


def paper_shape(d, seed=152):
    """The crumpled sheet's shape along directions d (... × 3): the ball pressed into flat faces (each the plane of
    one of PAPER_FACES directions, a little nearer or further and askew), with a few dents. Returns the radius against
    the round ball's (1 where it is as round), the face each direction lies on, and how far (0..) it is from the
    nearest fold between two faces."""
    rng = np.random.default_rng(seed)
    normals = fibonacci(PAPER_FACES, seed, 0.42)
    dist = 1.0 + rng.uniform(0.0, 0.11, PAPER_FACES)
    dots = d @ normals.T
    r = dist / np.maximum(dots, 0.2)
    order = np.argsort(r, -1)
    face = order[..., 0]
    r0 = np.take_along_axis(r, order[..., 0:1], -1)[..., 0]
    r1 = np.take_along_axis(r, order[..., 1:2], -1)[..., 0]
    radius = 1 + 0.55 * (r0 - 1)
    for c in fibonacci(7, seed + 1, 0.5):
        radius = radius - 0.045 * np.clip(1 - np.arccos(np.clip(d @ c, -1, 1)) / 0.5, 0, 1) ** 2
    return radius, face, r1 - r0


def newspaper(w=1024, h=512, seed=152):
    """The paper ball: a newspaper's sheet crumpled into a ball. Each flat face of the crumpled shape shows its own
    part of the page (page(): masthead, headlines, columns, photographs) at its own angle, lit as its own plane; the
    folds between the faces are sharp creases, worn light along the ridge and shaded beside it, fine wrinkles cross the
    faces, and the newsprint is greyish and uneven. Returns the colour and the height (units) of its relief."""
    d = directions(w, h)
    radius, face, edge = paper_shape(d, seed)
    rng = np.random.default_rng(seed + 2)
    normals = fibonacci(PAPER_FACES, seed, 0.42)
    sheet = page()
    n = sheet.shape[0]
    ink = np.zeros((h, w))
    light = np.zeros((h, w))
    scale = 215.0  # the page's texels a unit of the ball: a face shows about a third of the page's width
    for i, nrm in enumerate(normals):
        mine = face == i
        if not mine.any():
            continue
        ref = np.array([0.0, 1.0, 0.0]) if abs(nrm[1]) < 0.9 else np.array([1.0, 0.0, 0.0])
        e1 = np.cross(ref, nrm)
        e1 /= np.linalg.norm(e1)
        e2 = np.cross(nrm, e1)
        # the face's plane, the page lying on it turned and shifted
        q = d[mine] / np.maximum(d[mine] @ nrm, 0.2)[:, None] * R
        a = rng.uniform(-0.9, 0.9) + (math.pi if rng.random() < 0.25 else 0.0)
        x, y = -(q @ e1), q @ e2
        px = (math.cos(a) * x - math.sin(a) * y) * scale + rng.uniform(0, n)
        py = (math.sin(a) * x + math.cos(a) * y) * scale + rng.uniform(0, n)
        x0, y0 = np.floor(px).astype(np.int64), np.floor(py).astype(np.int64)
        fx, fy = px - x0, py - y0
        s = lambda yy, xx: sheet[yy % n, xx % n]
        ink[mine] = (s(y0, x0) * (1 - fx) + s(y0, x0 + 1) * fx) * (1 - fy) + (s(y0 + 1, x0) * (1 - fx)
                                                                             + s(y0 + 1, x0 + 1) * fx) * fy
        # each face its own brightness, falling off across it: a plane turned its own way to the light
        tilt = rng.normal(size=2)
        light[mine] = rng.uniform(0.88, 1.0) + 0.05 * (tilt[0] * x + tilt[1] * y)
    p = d * R
    wr = noise3(p, 1.7, seed + 3, 3)
    wrinkle = np.clip(1 - np.abs(wr - 0.5) * 22, 0, 1) ** 2 * np.clip((noise3(p, 0.9, seed + 4, 2) - 0.35) * 4, 0, 1)
    wr2 = noise3(p, 3.4, seed + 5, 2)
    wrinkle = np.maximum(wrinkle, 0.6 * np.clip(1 - np.abs(wr2 - 0.5) * 26, 0, 1) ** 2)
    fibre = noise3(p, 45.0, seed + 6, 2) - 0.5
    stain = noise3(p, 0.9, seed + 7, 3) - 0.5
    paper = rgb(236, 232, 218) * (1 + 0.05 * fibre + 0.07 * stain)[..., None]
    paper = paper * np.stack([np.ones_like(stain), 1 - 0.03 * (stain + 0.5), 1 - 0.09 * (stain + 0.5)], -1)
    black = rgb(34, 33, 38)
    # the ink is thinner where the paper is creased
    crease = np.clip(1 - edge / 0.022, 0, 1)
    beside = np.clip(1 - edge / 0.12, 0, 1)
    k = (np.clip(ink, 0, 1) * (1 - 0.5 * crease) * (1 - 0.35 * wrinkle))[..., None]
    img = paper + (black - paper) * 0.86 * k
    img = img * np.clip(light, 0.6, 1.08)[..., None]
    img = img * (1 - 0.20 * beside * (1 - crease) - 0.12 * wrinkle)[..., None] + 0.10 * crease[..., None]
    # the relief: the fine folds (the faces themselves are in the drawn mesh, and softly here for a plain sphere)
    height = 0.35 * (radius - 1) * R + 0.022 * crease + 0.016 * wrinkle + 0.002 * fibre
    return np.clip(img, 0, 1), height


SKINS = {"wood": wood, "stone": marble, "paper": newspaper}
RELIEF = {"wood": 0.22, "stone": 0.6, "paper": 1.0}  # how strongly each height is drawn
_made = {}


def skin(kind):
    """A ball's colour and normal map (made once)."""
    if kind not in _made:
        img, height = SKINS[kind]()
        _made[kind] = (img, relief(height, RELIEF[kind]))
    return _made[kind]


if __name__ == "__main__":
    import os
    import sys

    from PIL import Image

    out = sys.argv[1] if len(sys.argv) > 1 else "."
    for kind in SKINS:
        img, nrm = skin(kind)
        Image.fromarray((img * 255 + 0.5).astype(np.uint8)).save(os.path.join(out, f"ball_{kind}.png"))
        Image.fromarray((nrm * 255 + 0.5).astype(np.uint8)).save(os.path.join(out, f"ball_{kind}_n.png"))
