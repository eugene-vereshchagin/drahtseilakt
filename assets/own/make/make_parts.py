#!/usr/bin/env python3
"""Builds the game's own parts in the open level format (decisions.md, "The game plays out of the box";
docs/drahtseilakt/resources.md): every module a level can place (`assets/own/3D Entities/PH/<Module>/`), the balls
with their pieces (`Balls/`), the transformer's animation (`AnimTrafo/`), and the textures a run needs besides
(`assets/own/Textures/`: the ball's shadow, the particles, three skies).

Nothing of the original is copied, traced or resampled. What is taken from it are numbers measured on the player's
copy (never stored in the repository): the names the game's code looks objects up by, each object's pivot (its
place and axes, rounded), the overall sizes, and the dimensions of the collision shapes, because the physics must
behave as the original's does (the masses, frictions, joints and forces are the sim's own tables in
crates/game/src/modules.rs and stay as they are). The shapes are our own primitives at those dimensions: boxes where
the original's collision mesh is a box, and where it is not, a frustum, wedge or prism characterised by its widths,
heights and depth. The drawn meshes are our own designs of the same kind of thing (a pusher on a beam, a swing,
a sack on a rope, a plate bridge, flaps on an axle, a balloon platform...), mostly built on the collision shapes so
that what the ball touches is what the player sees.

Collision meshes (measured size → ours; the shape used):

    P_Modul_01  Rinne_01/02/03, Filler, Col01, Col02   boxes, exact measured size and centre
                Col03                     trapezoid prism: 4.92 wide at the bottom, 7.04 at the top, 4.35 high, 0.63 deep
    P_Modul_03  Floor                     20-sided slab, radius 10, 1.04 thick (the original's too)
                Wall_Coll                 trapezoid prism: 7.24 wide in front, 6.51 behind, 0.88 deep, 7.28 high
                Gate_Coll01/03            posts 1.01 wide, 1.32 deep at the foot, 0.86 at the head, 6.05 high
                Gate_Coll02               lintel 7.18 wide in front, 6.46 behind, 1.15/0.80 high, 0.89 deep
    P_Modul_08  Col1/Col6                 rods, boxes 0.45 x 9.94 x 0.51
                Col2/Col5                 boat-shaped side panels: hexagonal section 1.89 high, 6.70 long, 0.52 thick
                Col3/Col4                 seat halves: wedges 3.57 wide, 3.33 long, 0.40 to 0.54 high
    P_Modul_17  Col01/Col03               paddles, boxes 0.31 x 2.80 x 3.05
                Col02                     arm: 1.07 square at the hub (±0.73), tapering to 0.36 x 0.50 at ±7.74
    P_Modul_18  Kollisionsquader          the updraft's box, 2 x 16 x 2
    P_Modul_19  Col1/Col2                 flaps, boxes 6.39 x 0.50 x 5.00
                Col3                      axle, box 0.41 x 0.41 x 7.83
                Col4                      hub, regular hexagon radius 2.76, 0.57 thick
    P_Modul_25  Col01                     board, box 9.57 x 0.50 x 5.00
                Col02                     stop, right triangle 0.82 x 0.62, 0.50 thick
    P_Modul_26  Sack                      hull of our sack's points: octagonal body 2.79 x 3.64, seams to 3.89, neck,
                                          the tie's ends leaning over (heights and reaches to 0.05)
                Rope                      decagonal rod, 0.15 radius, 12.5 long
    P_Modul_29  Platte01..09              planks: each a board of its measured size with its ends' wear as a few kinds of cut (PLANKS)
    P_Modul_30  Col1                      board, box 14.80 x 0.50 x 5.00
                Col2                      axle, square turned 45 degrees, 0.73 across, 9.71 long
    P_Modul_34  Kiste, Schiebestein       boxes 4.1 x 4.1 x 5 and 5 x 5 x 5
    P_Modul_37  Col1, Col2                deck and beam, boxes 4.93 x 0.50 x 9.03 and 4.70 x 0.45 x 1.03
                Col3                      counterweight, pentagonal prism 3.60 x 3.47, 2.83 thick
    P_Modul_41  Col1                      trough: 3.97 wide at the foot, 5.00 at the top, 1.71 high, 5.00 deep
                Col2                      axle, pentagonal prism 0.87 x 0.83, 10.15 long
    PE_Balloon  Col_01/03                 side rails, quadrilateral prisms 9.3 x 1.8 (plan), 2.5 high
                Col_02                    the ramp, a parallelogram 3.6 long, 0.5 thick, rising 1.1, 4.27 wide
                Col_04                    floor plate, hexagonal prism 3.62 x 3.80, 0.5 thick
                Col_05                    back rail, 2.2 x 7.35 (plan), 2.5 high
                Col_06                    entry plate, box 2.35 x 0.50 x 4.17
                Platte01..08              planks as in P_Modul_29
                Ballon01..04              balloons (12-sided lathe, 3.48 x 4.35, not colliding)
                Seil01..04                ropes (not colliding): a fan fitted to the hull's volume, centre, inertia
                Box_slide                 unit box

How close each is (hull volume, centre, inertia, box against the original's) is checked by a script outside the
repository (it needs the player's copy); hull volumes are within 0.99..1.03 and inertias within 0.92..1.07 of the
original's. How the parts behave is checked by crates/game/src/partcheck.rs (copy's part against ours).

Run from anywhere: python3 -I assets/own/make/make_parts.py
Needs Python 3 with numpy and Pillow. The output is deterministic (fixed seeds).
"""

import math
import os
import shutil
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ballskins  # noqa: E402
import ownkit as k  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OWN = os.path.dirname(HERE)
ENT = os.path.join(OWN, "3D Entities")
TEXTURES = os.path.join(OWN, "Textures")
TEX = 256

# --- textures ----------------------------------------------------------------------------------------------------


def _target():
    """Red and white chevrons: the original marks what pushes or hits the ball this way; ours are diagonal bands."""
    y, x = np.mgrid[0:TEX, 0:TEX]
    band = ((x + y) // 32) % 2
    red, white = k.rgb(178, 38, 34), k.rgb(232, 226, 214)
    img = np.where(band[..., None] == 1, red, white)
    return np.clip(img * (0.9 + 0.12 * k.value_noise(TEX, TEX, (6, 6), 71))[..., None], 0, 1)


def _crate():
    """A wooden crate face: boards with a frame and a diagonal brace."""
    img = k.planks(TEX, k.rgb(170, 122, 72), 81, boards=5)
    y, x = np.mgrid[0:TEX, 0:TEX]
    frame = (np.minimum(np.minimum(x, TEX - 1 - x), np.minimum(y, TEX - 1 - y)) < 22)
    brace = np.abs(x - y) < 14
    dark = k.planks(TEX, k.rgb(120, 80, 44), 82, boards=8).transpose(1, 0, 2)
    img = np.where((frame | brace)[..., None], dark, img)
    edge = (np.minimum(np.minimum(x, TEX - 1 - x), np.minimum(y, TEX - 1 - y)) < 3)
    return np.where(edge[..., None], img * 0.5, img)


def _rope():
    y, x = np.mgrid[0:TEX, 0:TEX]
    twist = 0.5 + 0.5 * np.sin((x * 0.25 + y * 0.5))
    base = k.rgb(170, 140, 96)
    return np.clip(base[None, None] * (0.7 + 0.4 * twist)[..., None], 0, 1)


def _burlap():
    y, x = np.mgrid[0:TEX, 0:TEX]
    weave = 0.5 + 0.25 * np.sin(x * 1.2) + 0.25 * np.sin(y * 1.2)
    n = k.value_noise(TEX, TEX, (5, 5), 91)
    return np.clip(k.rgb(150, 120, 80)[None, None] * (0.7 + 0.25 * weave + 0.2 * n)[..., None], 0, 1)


def _balloon():
    """Gores of a balloon: red and cream stripes round it (u goes round)."""
    y, x = np.mgrid[0:TEX, 0:TEX]
    gore = (x // (TEX // 8)) % 2
    img = np.where(gore[..., None] == 1, k.rgb(196, 50, 40), k.rgb(236, 222, 190))
    return np.clip(img * (0.92 + 0.1 * k.value_noise(TEX, TEX, (3, 3), 93))[..., None], 0, 1)


def _ball_sticky():
    """Our sticky ball (no original's to stand in for): tacky crimson rubber in a honeycomb of shallow cells, each cell
    a little darker at its rim, a soft sheen in the middle."""
    y, x = np.mgrid[0:TEX, 0:TEX].astype(np.float64)
    # a hexagonal grid that tiles across u (12 cells round) and v (8 rows down)
    w = TEX / 12.0
    h = w * math.sqrt(3) / 2
    row = np.floor(y / h)
    xs = x / w - 0.5 * (row % 2)
    cx = (np.floor(xs) + 0.5 + 0.5 * (row % 2)) * w
    cy = (row + 0.5) * h
    best = np.full(x.shape, np.inf)
    for dr in (-1, 0, 1):
        r2 = row + dr
        xs2 = x / w - 0.5 * (r2 % 2)
        for dc in (-1, 0, 1):
            ccx = (np.floor(xs2) + dc + 0.5 + 0.5 * (r2 % 2)) * w
            ccy = (r2 + 0.5) * h
            d = np.hypot(x - ccx, y - ccy)
            best = np.minimum(best, d)
    rim = np.clip(best / (w * 0.55), 0, 1) ** 3
    n = k.value_noise(TEX, TEX, (8, 8), 125)
    base = k.rgb(178, 28, 64)[None, None] * (0.9 + 0.15 * n)[..., None]
    img = base * (1.05 - 0.55 * rim)[..., None] + (0.18 * (1 - rim) ** 4)[..., None] * k.rgb(255, 170, 190)[None, None]
    return np.clip(img, 0, 1)


def _dome():
    """The dome's enamel, one panel a picture: deep blue glass fused on metal, clouded as fired enamel is, a little
    lighter in the panel's middle, darker where it runs under the ribs, with a fine crackle."""
    y, x = np.mgrid[0:TEX, 0:TEX] / float(TEX)
    cloud = k.value_noise(TEX, TEX, (3, 3), 131, octaves=4) - 0.5
    fine = k.value_noise(TEX, TEX, (40, 40), 132, octaves=2) - 0.5
    edge = np.minimum(np.minimum(x, 1 - x), np.minimum(y, 1 - y))
    middle = np.clip(edge / 0.35, 0, 1) ** 0.6
    crack = np.abs(k.value_noise(TEX, TEX, (9, 9), 133, octaves=3) - 0.5) < 0.006
    deep, light = k.rgb(16, 52, 128), k.rgb(52, 112, 190)
    img = deep[None, None] + (light - deep)[None, None] * np.clip(0.15 + 0.55 * middle + 0.9 * cloud, 0, 1)[..., None]
    img = img * (1 + 0.06 * fine)[..., None] * (0.55 + 0.45 * np.clip(edge / 0.05, 0, 1))[..., None]
    return np.clip(np.where(crack[..., None], img * 0.8, img), 0, 1)


def _radial(power=1.5, inner=0.0):
    """1 in the middle falling to 0 at the edge."""
    y, x = np.mgrid[0:TEX, 0:TEX]
    r = np.hypot(x - TEX / 2 + 0.5, y - TEX / 2 + 0.5) / (TEX / 2)
    return np.clip(1 - np.clip((r - inner) / (1 - inner), 0, 1), 0, 1) ** power


def _flash():
    """The transformer's flash field: bright streaks that the game scrolls."""
    y, x = np.mgrid[0:TEX, 0:TEX]
    s = np.abs(np.sin(x * 0.08 + 3 * k.value_noise(TEX, TEX, (2, 6), 141))) ** 6
    img = np.stack([0.6 * s, 0.85 * s, s], -1) * _radial(0.6)[..., None]
    return np.clip(img, 0, 1)


def _silver():
    """A small silver ball for the sprites (alpha outside the disc 0)."""
    y, x = np.mgrid[0:TEX, 0:TEX]
    c = TEX / 2 - 0.5
    dx, dy = (x - c) / c, (y - c) / c
    r2 = dx * dx + dy * dy
    z = np.sqrt(np.clip(1 - r2, 0, 1))
    light = np.clip(-0.4 * dx - 0.5 * dy + 0.75 * z, 0, 1)
    img = np.stack([0.55 + 0.45 * light ** 2] * 3, -1) * np.array([0.95, 0.97, 1.0])
    return np.clip(img, 0, 1), (r2 < 1.0).astype(np.float64)


def _oil():
    """An oily film for the extra life's bubble, added to the frame: dark, so that only its colours show."""
    n = k.value_noise(TEX, TEX, (3, 3), 151)
    h = (n * 3.0) % 1.0
    img = np.stack([0.5 + 0.5 * np.cos(2 * np.pi * (h + o)) for o in (0.0, 0.33, 0.67)], -1)
    return np.clip(img * 0.32, 0, 1)


def _lift_deck():
    """The lift's deck: oak boards along x, butt joints staggered from board to board with two nails either side,
    worn lighter along the boards' middles."""
    n = 2 * TEX
    img = k.planks(n, k.rgb(142, 100, 60), 31, boards=4, gaps=0.75)
    y, x = np.mgrid[0:n, 0:n]
    rng = np.random.default_rng(32)
    board = y // (n // 4)
    wear = k.value_noise(n, n, (3, 6), 33)
    img = img * (0.86 + 0.3 * wear * np.sin(np.pi * (y % (n // 4)) / (n // 4)) ** 2)[..., None]
    for b in range(4):
        jx = int(rng.integers(n // 8, n - n // 8))
        on = board == b
        img = np.where((on & (np.abs(x - jx) < 2))[..., None], img * 0.35, img)
        for dx in (-14, 14):
            for fy in (0.25, 0.75):
                cy = (b + fy) * (n // 4)
                d = np.hypot(x - (jx + dx), y - cy)
                img = np.where((d < 4.5)[..., None], k.rgb(58, 54, 52)[None, None] * (1.5 - d / 6)[..., None], img)
    return np.clip(img, 0, 1)


def _lift_paint():
    """Red-oxide paint on iron, as the lift's loose panels have it: brushed, worn to the dark metal in a few
    patches, a little soot in the mottle."""
    img = k.metal(TEX, k.rgb(150, 60, 45), 34, brushed=0.05)
    chips = k.value_noise(TEX, TEX, (10, 10), 35, octaves=3)
    soot = k.value_noise(TEX, TEX, (5, 5), 36)
    img = img * (0.84 + 0.24 * soot)[..., None]
    bare = np.clip((chips - 0.72) * 9, 0, 0.7)[..., None]
    return np.clip(img * (1 - bare) + k.rgb(84, 70, 64)[None, None] * bare, 0, 1)


def _lift_rim():
    """Turned gunmetal for the lift's disc (u goes round it): fine rings and a few darker ones."""
    img = k.metal(TEX, k.rgb(112, 118, 128), 37, brushed=0.1).transpose(1, 0, 2)
    y = np.arange(TEX)[:, None]
    groove = 1 - 0.3 * np.exp(-((y % 64) - 32.0) ** 2 / 6.0)
    return np.clip(img * groove[..., None], 0, 1)


def _paint_green():
    """Bottle-green machine paint on iron (the spinner's arm, the pusher's beam): as the lift's red, brushed and
    chipped to the dark metal in a few patches."""
    img = k.metal(TEX, k.rgb(58, 98, 80), 44, brushed=0.05)
    chips = k.value_noise(TEX, TEX, (10, 10), 45, octaves=3)
    soot = k.value_noise(TEX, TEX, (5, 5), 46)
    img = img * (0.84 + 0.24 * soot)[..., None]
    bare = np.clip((chips - 0.72) * 9, 0, 0.7)[..., None]
    return np.clip(img * (1 - bare) + k.rgb(84, 70, 64)[None, None] * bare, 0, 1)


def _bullseye():
    """A target to be put whole on a face: red rings round a red bull's eye on cream paint, a dark line round its
    edge, a little worn."""
    y, x = np.mgrid[0:TEX, 0:TEX]
    c = (TEX - 1) / 2
    r = np.hypot(x - c, y - c) / (TEX / 2)
    red, white = k.rgb(178, 38, 34), k.rgb(232, 226, 214)
    ring = (r < 0.2) | ((r > 0.42) & (r < 0.62)) | ((r > 0.82) & (r < 0.94))
    img = np.where(ring[..., None], red, white)
    edge = np.minimum(np.minimum(x, TEX - 1 - x), np.minimum(y, TEX - 1 - y)) < 5
    img = np.where(edge[..., None], k.rgb(70, 62, 58), img)
    wear = k.value_noise(TEX, TEX, (6, 6), 47)
    chips = np.clip((k.value_noise(TEX, TEX, (14, 14), 48, octaves=3) - 0.76) * 8, 0, 0.5)[..., None]
    img = img * (0.88 + 0.14 * wear)[..., None]
    return np.clip(img * (1 - chips) + k.rgb(96, 88, 82)[None, None] * chips, 0, 1)


def _sack_cloth():
    """Canvas for the sack (u goes twice round it): a plain weave, mottled, and a grain sack's woven stripes, one
    broad and two narrow, down the middle of each broad side."""
    n = 2 * TEX
    y, x = np.mgrid[0:n, 0:n]
    warp, weft = 0.5 + 0.5 * np.sin(x * np.pi / 2), 0.5 + 0.5 * np.sin(y * np.pi / 2)
    weave = np.where(((x // 4 + y // 4) % 2) == 0, warp, weft)
    img = k.rgb(180, 154, 108)[None, None] * (0.76 + 0.16 * weave + 0.18 * k.value_noise(n, n, (6, 6), 95)
                                               + 0.06 * k.value_noise(n, n, (48, 48), 96, octaves=2))[..., None]
    fx = x / n
    d = np.minimum(fx, 1 - fx)
    stripe = (d < 0.02) | ((d > 0.036) & (d < 0.047))
    blue = k.rgb(62, 80, 122)[None, None] * (0.8 + 0.3 * weave)[..., None]
    return np.clip(np.where(stripe[..., None], 0.75 * blue + 0.25 * img, img), 0, 1)


def _shadow_soft():
    return np.zeros((TEX, TEX, 3)), _radial(1.2) * 0.85


def track_slab(name, rows=256):
    """One slab (5 units) of our track's texture (assets/own/Track, make_textures.py), for a part that becomes part
    of the track (the crate puzzle's stone, which drops level with the way)."""
    from PIL import Image
    img = np.asarray(Image.open(os.path.join(OWN, "Track", name)).convert("RGB"), np.float64) / 255.0
    return img[:rows]


def _wicker():
    """A woven basket: willow rods under and over the stakes, each rod round in shade, the gaps dark."""
    n = TEX
    y, x = np.mgrid[0:n, 0:n] / n
    rows, cols = 16, 8
    ry = (y * rows) % 1.0
    cx = (x * cols) % 1.0
    over = ((np.floor(y * rows) + np.floor(x * cols)) % 2) == 0
    # a rod is lit along its middle, dark at its edges; where it passes under a stake the stake's side shades it
    rod = np.sin(ry * np.pi) ** 0.6
    under = np.where(over, 1.0, 0.72 + 0.28 * np.abs(np.cos(cx * np.pi)))
    tone = 0.85 + 0.15 * k.value_noise(n, n, (24, 6), 71, octaves=3)
    base = np.array([0.62, 0.47, 0.28])
    img = base[None, None] * (rod * under * tone)[..., None]
    return np.clip(img + 0.04, 0, 1)


TEXTURE_MAKERS = {
    "wicker.png": _wicker,
    "track_top.png": lambda: track_slab("Track_Top_A.png"),
    "track_side.png": lambda: track_slab("Track_Side.png")[:224, 32:],  # below its brick strip
    "wood.png": lambda: k.planks(TEX, k.rgb(176, 128, 78), 11),
    "wood_dark.png": lambda: k.planks(TEX, k.rgb(120, 82, 50), 12, boards=3),
    "stone.png": lambda: k.stone(TEX, k.rgb(150, 144, 132), 13, slabs=1),
    "stone_light.png": lambda: k.stone(TEX, k.rgb(196, 182, 152), 14, slabs=2, seams=0.2),
    "stone_side.png": lambda: k.stone(TEX, k.rgb(160, 136, 110), 15, slabs=0),
    "steel.png": lambda: k.metal(TEX, k.rgb(150, 156, 168), 16),
    "iron.png": lambda: k.metal(TEX, k.rgb(84, 88, 96), 17, brushed=0.12),
    "brass.png": lambda: k.metal(TEX, k.rgb(186, 150, 82), 18),
    "target.png": _target,
    "crate.png": _crate,
    "lift_deck.png": _lift_deck,
    "lift_paint.png": _lift_paint,
    "lift_rim.png": _lift_rim,
    "paint_green.png": _paint_green,
    "bullseye.png": _bullseye,
    "sack_cloth.png": _sack_cloth,
    "rope.png": _rope,
    "burlap.png": _burlap,
    "balloon.png": _balloon,
    # the balls' skins and their reliefs (ballskins.py; the pack's Ball_Wood, Ball_Stone and Ball_Paper are the same)
    "ball_wood.png": lambda: ballskins.skin("wood")[0], "ball_wood_n.png": lambda: ballskins.skin("wood")[1],
    "ball_stone.png": lambda: ballskins.skin("stone")[0], "ball_stone_n.png": lambda: ballskins.skin("stone")[1],
    "ball_paper.png": lambda: ballskins.skin("paper")[0], "ball_paper_n.png": lambda: ballskins.skin("paper")[1],
    "ball_sticky.png": _ball_sticky,
    "dome_enamel.png": _dome,
    "flash.png": _flash,
    "chrome_env.png": lambda: k.env_map(TEX, k.rgb(150, 170, 220), k.rgb(250, 250, 255), k.rgb(70, 60, 60), 19),
    "oil.png": _oil,
    "glow.png": lambda: np.stack([_radial(2.0)] * 3, -1),
}
ALPHA_MAKERS = {"silver.png": _silver, "shadow.png": _shadow_soft}
_cache = {}


def texture(folder, name, as_name=None):
    """Saves texture `name` into the part's textures/ (made once, kept in memory), as `as_name` when given."""
    if name not in _cache:
        _cache[name] = ALPHA_MAKERS[name]() if name in ALPHA_MAKERS else (TEXTURE_MAKERS[name](), None)
    img, alpha = _cache[name]
    k.save_png(os.path.join(folder, "textures"), as_name or name, img, alpha)


# The texture each material's role has in the original (its texture's name there), where the game's own texture pack
# (assets/own/Textures, TEXTURES.md) has a picture of that name: the game draws our parts with the pack's picture by
# that name (the default, `textures own`), and with our placeholder kept in the part's folder otherwise. Materials
# not listed keep their placeholder's name (the pack has nothing for their role).
PACK_NAMES = {
    "AnimTrafo_Bars": "Trafo_Anim_Environment", "AnimTrafo_RingParts": "Trafo_Anim_Environment",
    "AnimTrafo_RingParts_Color": "Trafo_Anim_Environment", "P_Trafo_Base": "Metal_stained",
    "Ball_Wood": "Ball_Wood", "Ball_Wood_Pieces": "Ball_Wood", "Ball_Stone": "Ball_Stone",
    "Ball_Stone_Pieces": "Ball_Stone", "Ball_Paper": "Ball_Paper", "Ball_Paper_Pieces": "Ball_Paper",
    "P_Ball_Wood": "P_Ball_Wood", "P_Ball_Stone": "Ball_Stone", "P_Ball_Paper": "Ball_Paper",
    "P_Box": "E_Holzbeschlag", "P_Dome": "Dome",
    "PE_Balloonl_Halter": "Wood_Metal", "PE_Balloon_SuspBridge": "Wood_Old",
    "P_Modul_01_Stone": "Column_beige", "P_Modul_01_Target": "Target", "P_Modul_01_Wood": "Wood_Metal",
    "P_Modul_08_Stone": "Ball_Stone", "P_Modul_08_Schelle": "Metal_stained", "P_Modul_08_Wood": "Wood_Nailed",
    "P_Modul_17_Target": "Target", "P_Modul_17_Cube": "E_Holzbeschlag",
    "P_Modul_18_Rotor": "Rail_Environment",
    "P_Modul_19_Axis": "Rail_Environment", "P_Modul_19_Flaps": "Wood_MetalStripes",
    "P_Modul_19_FlapsBackboards": "Wood_Nailed",
    "P_Modul_25_Wood": "Wood_Raft", "P_Modul_25_HingeSide": "Wood_Misc", "P_Modul_25_Hinge": "Wood_Metal",
    "P_Modul_26_Sandsack": "sandsack", "P_Modul_26_Halter": "E_Holzbeschlag", "P_Modul_26_Seil": "sandsack",
    "P_Modul_29": "Wood_Old", "P_Modul_30_Board": "Modul30_d_Seiten",
    "P_Modul_34_Box": "E_Holzbeschlag",
    "P_Modul_37_WoodMisc": "Wood_Misc", "P_Modul_37_WoodAxis": "Wood_Metal", "P_Modul_37_Wood01": "Ball_Wood",
    "P_Modul_37_Target": "Target", "P_Modul_37_Metall_1": "Metal_stained",
    "P_Modul37_Misc_MetalButtons": "Metal_stained",
    "P_Modul_41_BoxMetal": "E_Holzbeschlag", "P_Modul_41_BoxWood": "E_Holzbeschlag",
    "P_Trafo_Wood": "Trafo_Environment", "P_Trafo_Wood_Color": "Trafo_Environment",
    "P_Trafo_Stone": "Trafo_Environment", "P_Trafo_Stone_Color": "Trafo_Environment",
    "P_Trafo_Paper": "Trafo_Environment", "P_Trafo_Paper_Color": "Trafo_Environment",
    "P_Trafo_Sticky": "Trafo_Environment", "P_Trafo_Sticky_Color": "Trafo_Environment",
}


# --- a part being built ------------------------------------------------------------------------------------------


class Part:
    """One part folder: its meshes, nodes (by name), materials (by name) and groups."""

    def __init__(self, folder):
        self.folder = os.path.join(ENT, folder)
        self.meshes, self.nodes, self.mats = [], [], []
        self.mat_ids, self.groups = {}, []
        self.files = []
        if os.path.isdir(self.folder):
            shutil.rmtree(self.folder)

    def mat(self, name, tex=None, **kw):
        """A material by name, made once; its texture is written into the part's folder."""
        if name not in self.mat_ids:
            # (a `game:` texture is the game's own by name, as the levels' scenery has them: no file here)
            if tex and not tex.startswith(k.GAME):
                # named after the role's texture in the pack; two roles of one name in a part share the first's file
                file = PACK_NAMES[name] + ".png" if name in PACK_NAMES else tex
                if file not in self.files:
                    texture(self.folder, tex, file)
                    self.files.append(file)
                tex = file
            # its relief, a normal map on the same coordinates, beside it
            if kw.get("normal") and not kw["normal"].startswith(k.GAME) and kw["normal"] not in self.files:
                texture(self.folder, kw["normal"])
                self.files.append(kw["normal"])
            self.mats.append(k.material(name, tex, **kw))
            self.mat_ids[name] = len(self.mats) - 1
        return self.mat_ids[name]

    def mesh(self, m):
        self.meshes.append(m)
        return len(self.meshes) - 1

    def node(self, name, mesh=None, pos=(0, 0, 0), x=(1, 0, 0), y=(0, 1, 0), s=1.0, visible=False, sprite=None):
        self.nodes.append(k.node(name, mesh, k.pose(pos, x, y, (s, s, s) if np.isscalar(s) else s), visible, sprite))
        return len(self.nodes) - 1

    def group(self, name, members):
        ids = [i for i, n in enumerate(self.nodes) if n["name"] in members]
        missing = set(members) - {self.nodes[i]["name"] for i in ids}
        assert not missing, (name, missing)
        self.groups.append((name, sorted(ids, key=lambda i: members.index(self.nodes[i]["name"]))))

    def write(self):
        size = k.write_open(self.folder, self.meshes, self.nodes, self.mats, self.groups,
                            generator="drahtseilakt make_parts.py")
        print(f"{os.path.relpath(self.folder, OWN)}: {len(self.nodes)} objects, {len(self.meshes)} meshes, {size} bytes")


def col(name, build):
    """A collision mesh (no material: the game never draws it) built by `build(mesh)`."""
    m = k.Mesh(name)
    build(m)
    return m


def prism_xy(m, mat, pts, z0, z1):
    """A prism of a convex section in x-y (points in order round it) from z0 to z1."""
    bottom = [(x, y, z0) for x, y in pts]
    top = [(x, y, z1) for x, y in pts]
    _prism(m, mat, bottom, top)


def prism_yz(m, mat, pts, x0, x1):
    """A prism of a convex section in y-z (points (y, z)) from x0 to x1."""
    _prism(m, mat, [(x0, y, z) for y, z in pts], [(x1, y, z) for y, z in pts])


def prism_xz(m, mat, pts, y0, y1):
    """A prism of a convex section in x-z (points (x, z)) from y0 to y1."""
    _prism(m, mat, [(x, y0, z) for x, z in pts], [(x, y1, z) for x, z in pts])


def _prism(m, mat, a, b):
    a, b = [np.array(p, np.float64) for p in a], [np.array(p, np.float64) for p in b]
    centre = (sum(a) + sum(b)) / (2 * len(a))
    n = len(a)

    def face(pts):
        nrm = np.cross(pts[1] - pts[0], pts[2] - pts[0])
        if np.dot(nrm, sum(pts) / len(pts) - centre) < 0:
            pts = pts[::-1]
        m.face(mat, pts)

    face(list(a))
    face(list(b))
    for i in range(n):
        face([a[i], a[(i + 1) % n], b[(i + 1) % n], b[i]])


def regular(n, r, phase=0.0, cx=0.0, cy=0.0):
    return [(cx + r * math.cos(phase + 2 * math.pi * i / n), cy + r * math.sin(phase + 2 * math.pi * i / n))
            for i in range(n)]


def lathe(m, mat, profile, seg=16, sx=1.0, sz=1.0, centre=(0, 0, 0)):
    """A surface of revolution about y from (y, r) points (bottom to top), its section scaled sx, sz."""
    cx, cy, cz = centre
    rows = []
    for i, (y, r) in enumerate(profile):
        y0, r0 = profile[max(i - 1, 0)]
        y1, r1 = profile[min(i + 1, len(profile) - 1)]
        dy, dr = y1 - y0, r1 - r0
        row = []
        for j in range(seg + 1):
            a = 2 * math.pi * j / seg
            ca, sa = math.cos(a), math.sin(a)
            nrm = np.array([dy * ca / sx, -dr, dy * sa / sz])
            nrm /= np.linalg.norm(nrm) + 1e-12
            row.append(m.vertex((cx + r * ca * sx, cy + y, cz + r * sa * sz), nrm, (j / seg, i / (len(profile) - 1))))
        rows.append(row)
    for i in range(len(rows) - 1):
        for j in range(seg):
            a, b, c, d = rows[i][j], rows[i][j + 1], rows[i + 1][j + 1], rows[i + 1][j]
            m.tri(mat, a, c, b)
            m.tri(mat, a, d, c)


def quad_xz(m, mat, half, y=0.0, uv=1.0):
    """A flat square facing up (shadows, floors of light)."""
    m.face(mat, [(-half, y, -half), (-half, y, half), (half, y, half), (half, y, -half)],
           [(0, 0), (0, uv), (uv, uv), (uv, 0)], normal=(0, 1, 0))


def sprite_quad(m, mat):
    """The unit quad a 3D sprite carries (the game sizes and turns it itself)."""
    m.face(mat, [(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)], [(0, 1), (1, 1), (1, 0), (0, 0)], normal=(0, 0, -1))


# --- the balls ---------------------------------------------------------------------------------------------------

# The balls' boxes, measured: the physics takes each ball's box from its mesh (sim.rs, ball_boxes: the updraft's test).
WOOD_BOX = ((-2.0, -2.0, -2.0), (2.0, 2.0, 2.0))
STONE_BOX = ((-1.988, -2.0, -2.0), (1.988, 2.0, 2.0))
PAPER_BOX = ((-2.039, -2.030, -2.036), (1.970, 1.968, 1.960))


def fit_box(m, box):
    """Moves and scales a mesh along each axis so that its box is exactly `box`."""
    lo, hi = m.bounds()
    want_lo, want_hi = np.array(box[0]), np.array(box[1])
    s = (want_hi - want_lo) / (hi - lo)
    t = want_lo - lo * s
    mat = np.eye(4)
    mat[:3, :3] = np.diag(s)
    mat[:3, 3] = t
    m.transformed(mat)


def round_look(name, mat, kind):
    """The wooden or stone ball as drawn: its sphere with many more faces and smooth normals (the mesh above, as
    coarse as the original's, stays the physics')."""
    m = k.Mesh(name)
    k.sphere(m, mat, (0, 0, 0), 2.0, seg=48)
    fit_box(m, STONE_BOX if kind == "stone" else WOOD_BOX)
    return m


def paper_look(name, mat):
    """The paper ball as drawn: a sheet crumpled into a ball, pressed into the flat faces and dents its skin is printed
    for (ballskins.paper_shape: each face shows its own part of the page), many faces, smooth normals; the sharp
    creases are in the skin's relief."""
    m = k.Mesh(name)
    k.sphere(m, mat, (0, 0, 0), 2.0, seg=64)
    d = np.asarray(m.p) / 2.0
    radius, _, _ = ballskins.paper_shape(d)
    # as big as the physics' sphere (radius 2 round the middle): most of it on that sphere, only the folds' ridges
    # a little out of it (no gap under it where it rests, and it does not sink in)
    top = np.sort(radius)[int(0.9 * (len(radius) - 1))]
    m.p = [tuple(q) for q in d * (radius / top * 2.0)[:, None]]
    smooth_normals(m)
    return m


def ball_mesh(name, mat, kind):
    m = k.Mesh(name)
    if kind == "paper":
        # a crumpled sheet: a sphere pushed in and out by noise, then fitted to the measured box
        rng = np.random.default_rng(151)
        bumps = [(rng.normal(size=3), rng.uniform(0.04, 0.14)) for _ in range(40)]
        tmp = k.Mesh("tmp")
        k.sphere(tmp, mat, (0, 0, 0), 2.0, seg=20)
        pts = []
        for p in tmp.p:
            d = np.array(p) / 2.0
            r = 2.0 + sum(a * math.cos(6 * float(np.dot(d, v / np.linalg.norm(v)))) for v, a in bumps) * 0.25
            pts.append(tuple(d * r))
        tmp.p = pts
        # faceted like folded paper: each triangle its own normal
        for mt, idx in tmp.parts.items():
            for t in range(0, len(idx), 3):
                a, b, c = (np.array(tmp.p[i]) for i in idx[t:t + 3])
                uv = [tmp.uv[i] for i in idx[t:t + 3]]
                n = np.cross(b - a, c - a)
                ids = [m.vertex(q, n / (np.linalg.norm(n) + 1e-12), u) for q, u in zip((a, b, c), uv)]
                m.tri(mt, *ids)
        fit_box(m, PAPER_BOX)
    elif kind == "stone":
        k.sphere(m, mat, (0, 0, 0), 2.0, seg=24)
        fit_box(m, STONE_BOX)
    elif kind == "sticky":
        # ours: a true sphere of the physics' radius 2, smooth (the cells are in its texture)
        k.sphere(m, mat, (0, 0, 0), 2.0, seg=24)
    else:
        k.sphere(m, mat, (0, 0, 0), 2.0, seg=20)
        fit_box(m, WOOD_BOX)
    return m


def fibonacci(n, seed):
    """n directions spread over the sphere, turned by a seed."""
    rng = np.random.default_rng(seed)
    out = []
    for i in range(n):
        y = 1 - 2 * (i + 0.5) / n
        r = math.sqrt(1 - y * y)
        a = i * math.pi * (3 - math.sqrt(5)) + rng.uniform(-0.2, 0.2)
        out.append(np.array([r * math.cos(a), y, r * math.sin(a)]))
    return out


def pieces(part, kind, mat, n, thick, seed, stretch=1.0):
    """A ball's pieces: the ball's own skin peeled into n curved shell fragments that together cover the sphere (each
    the part of the sphere nearest one of n directions; `stretch` draws the cells out along the poles, as staves),
    `thick` deep, the ball's texture on the outside as the ball has it, a thin inner side and edges. Each is its own
    node, at its centre with its y along its direction (the burst pushes along the piece's own up, pieces.rs), in
    the group Ball_<Kind>_Pieces. Its mesh, whose points the physics takes as its hull, is cut from a coarse sphere
    (a few dozen points); what is drawn, its look, from a fine one."""
    names = []
    dirs = fibonacci(n, seed)
    squash = np.array([1.0, 1.0 / stretch, 1.0])
    keys = [d * squash / np.linalg.norm(d * squash) for d in dirs]

    def owner(q):
        v = q / np.linalg.norm(q) * squash
        v /= np.linalg.norm(v)
        return int(np.argmax([np.dot(v, kk) for kk in keys]))

    def shell(seg):
        sph = k.Mesh("tmp")
        k.sphere(sph, mat, (0, 0, 0), 2.0, seg=seg)
        P = np.asarray(sph.p)
        tris = [tuple(t) for idx in sph.parts.values() for t in np.reshape(idx, (-1, 3))]
        own = [owner(P[list(t)].mean(0)) for t in tris]
        return P, sph.uv, tris, own

    coarse, fine = shell(10), shell(40)
    inner = (2.0 - thick) / 2.0

    def build(name, shell_, i, centre, rot):
        P, UV, tris, own = shell_
        m = k.Mesh(name)
        local = lambda q: rot.T @ (q - centre)
        edges = {}
        for t, o in zip(tris, own):
            if o != i:
                continue
            for x, y in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                key = (min(x, y), max(x, y))
                edges[key] = None if key in edges else (x, y)
            m.tri(mat, *[m.vertex(local(P[j]), rot.T @ (P[j] / 2.0), UV[j]) for j in t])
            inn = [m.vertex(local(P[j] * inner), -(rot.T @ (P[j] / 2.0)), UV[j]) for j in t]
            m.tri(mat, inn[0], inn[2], inn[1])
        # the edges: where the piece's skin ends (an edge only one of its triangles has), a strip down to its inside
        for e in edges.values():
            if e is None:
                continue
            x, y = e
            q = [P[x], P[y], P[y] * inner, P[x] * inner]
            m.face(mat, [local(v) for v in q], [UV[x], UV[y], UV[y], UV[x]], normal=rot.T @ np.cross(P[y] - P[x], P[x]))
        return m

    for i, d in enumerate(dirs):
        P, _, tris, own = coarse
        mine = [t for t, o in zip(tris, own) if o == i]
        if not mine:
            continue
        pts = np.array([P[j] for t in mine for j in t])
        centre = pts.mean(0) * (1 + inner) / 2
        ref = np.array([0.0, 1.0, 0.0]) if abs(d[1]) < 0.9 else np.array([1.0, 0.0, 0.0])
        t1 = np.cross(ref, d)
        t1 /= np.linalg.norm(t1)
        rot = np.stack([t1, d, np.cross(t1, d)], 1)
        m = build(f"Ball_{kind}_piece{i + 1:02d}_Mesh", coarse, i, centre, rot)
        look = build(f"Ball_{kind}_piece{i + 1:02d}_Look", fine, i, centre, rot)
        if look.p:
            m.look = look
        name = f"Ball_{kind}_piece{i + 1:02d}"
        part.node(name, part.mesh(m), centre, t1, d)
        names.append(name)
    part.group(f"Ball_{kind}_Pieces", names)
    return names


def make_balls():
    p = Part("Balls")
    piece_mat = dict(alpha="BLEND", blend=("src_alpha", "inv_src_alpha"), z_write=True)
    # (the pieces are the balls' own skin, so they carry the same picture and relief)
    wood = p.mat("Ball_Wood", "ball_wood.png", emissive=0.24, specular=0.25, power=24, normal="ball_wood_n.png")
    stone = p.mat("Ball_Stone", "ball_stone.png", emissive=0.27, specular=0.22, power=14, normal="ball_stone_n.png")
    paper = p.mat("Ball_Paper", "ball_paper.png", emissive=0.2, normal="ball_paper_n.png")
    wp = p.mat("Ball_Wood_Pieces", "ball_wood.png", emissive=0.24, normal="ball_wood_n.png", **piece_mat)
    sp = p.mat("Ball_Stone_Pieces", "ball_stone.png", emissive=0.27, normal="ball_stone_n.png", **piece_mat)
    pp = p.mat("Ball_Paper_Pieces", "ball_paper.png", emissive=0.2, double_sided=True, normal="ball_paper_n.png",
               **piece_mat)
    sticky = p.mat("Ball_Sticky", "ball_sticky.png", emissive=0.24, specular=0.45, power=30)
    tp = p.mat("Ball_Sticky_Pieces", "ball_sticky.png", emissive=0.24, **piece_mat)
    light = p.mat("Ball_LightningSphere", "glow.png", diffuse=[0, 0, 0, 1], emissive=1.0, alpha="BLEND",
                  blend=("one", "one"), double_sided=True)
    # the skin peeled: sixteen staves of wood, seventeen shards of stone, eighteen scraps of paper, as many as the
    # original's
    wood_names = pieces(p, "Wood", wp, 16, 0.28, 161, stretch=2.2)
    stone_names = pieces(p, "Stone", sp, 17, 0.4, 162)
    paper_names = pieces(p, "Paper", pp, 18, 0.07, 163)
    # ours: fourteen thick rubber chunks
    sticky_names = pieces(p, "Sticky", tp, 14, 0.3, 164)
    balls = []
    for name, mat, kind in (("Ball_Wood", wood, "wood"), ("Ball_Stone", stone, "stone"), ("Ball_Paper", paper, "paper"),
                            ("Ball_Sticky", sticky, "sticky")):
        mesh_name = "Ball_Stone_HighRes_Mesh" if kind == "stone" else f"{name}_Mesh"
        bm = ball_mesh(mesh_name, mat, kind)
        bm.look = paper_look(f"{name}_Look", mat) if kind == "paper" else round_look(f"{name}_Look", mat, kind)
        p.node(name, p.mesh(bm))
        balls.append(name)
    # the new ball's flash: a glowing sphere, cut flat a little below the ball's middle where it meets the floor
    m = k.Mesh("Ball_LightningSphere_Mesh")
    prof = [(y, math.sqrt(max(2.25 ** 2 - y * y, 0.0))) for y in np.linspace(-1.59, 2.25, 10)]
    prof = [(-1.59, 0.0)] + prof
    lathe_mesh(m, light, prof, 16)
    p.node("Ball_Lightning_PointLight", pos=(0, 9.0, 0), x=(0.001, 0, 1), y=(0.952, 0.306, -0.001))
    p.node("Ball_Particle_Frame", s=1.5, visible=True)
    p.node("Ball_LightningSphere", p.mesh(m))
    for f in ("Ball_Pos_Frame", "Ball_WoodPieces_Frame", "Ball_StonePieces_Frame", "Ball_PaperPieces_Frame",
              "Ball_StickyPieces_Frame", "Balls_MF"):
        p.node(f)
    p.group("All_Balls", wood_names + stone_names + paper_names + sticky_names + balls +
            ["Ball_Lightning_PointLight", "Ball_Particle_Frame", "Ball_LightningSphere", "Ball_Pos_Frame",
             "Ball_WoodPieces_Frame", "Ball_StonePieces_Frame", "Ball_PaperPieces_Frame", "Ball_StickyPieces_Frame",
             "Balls_MF"])
    p.write()


def lathe_mesh(m, mat, profile, seg=16, **kw):
    lathe(m, mat, profile, seg, **kw)


def make_loose_balls():
    """The loose balls a level places (P_Ball_*): their physics comes from the level's stand-in; this is their look."""
    for kind, tex, em in (("Wood", "ball_wood.png", 0.31), ("Stone", "ball_stone.png", 0.31),
                          ("Paper", "ball_paper.png", 0.31), ("Sticky", "ball_sticky.png", 0.31)):
        p = Part(f"PH/P_Ball_{kind}")
        mat = p.mat(f"P_Ball_{kind}", tex, emissive=em, specular=0.0 if kind == "Paper" else 0.2, power=15,
                    normal=None if kind == "Sticky" else tex.replace(".png", "_n.png"))
        bm = ball_mesh(f"P_Ball_{kind}_Mesh", mat, kind.lower())
        bm.look = (paper_look(f"P_Ball_{kind}_Look", mat) if kind == "Paper"
                   else round_look(f"P_Ball_{kind}_Look", mat, kind.lower()))
        p.node(f"P_Ball_{kind}_MF", p.mesh(bm))
        p.group(f"All_P_Ball_{kind}", [f"P_Ball_{kind}_MF"])
        p.write()


# --- simple parts ------------------------------------------------------------------------------------------------


def make_box():
    p = Part("PH/P_Box")
    mat = p.mat("P_Box", "crate.png", emissive=0.31)
    m = k.Mesh("P_Box_Mesh")
    # each face its own crate face: uv 0..1 over 5 units
    k.box(m, mat, (-2.5, -2.5, -2.5), (2.5, 2.5, 2.5), scale=5.0)
    for i, (u, v) in enumerate(m.uv):
        m.uv[i] = (u + 0.5, v + 0.5)
    p.node("P_Box_MF", p.mesh(m))
    p.group("All_P_Box", ["P_Box_MF"])
    p.write()


def shadow(p, name, mat_name, half=6.05):
    m = k.Mesh(f"{name}_Mesh")
    mat = p.mat(mat_name, "shadow.png", diffuse=[0, 0, 0, 1], emissive=1.0, alpha="BLEND",
                blend=("src_alpha", "inv_src_alpha"), z_write=False)
    quad_xz(m, mat, half)
    return p.mesh(m)


def make_dome():
    """The dome bumper. What the ball meets is the plain cap (its points are the body's hull); what is drawn, its look,
    lies just inside that cap: twelve brass ribs riveted from a base ring up to a hub, a hoop round the middle, and
    between them panels of blue enamel set a little deeper."""
    p = Part("PH/P_Dome")
    sh = shadow(p, "P_DomeShadow", "P_DomeShadow")
    # polished steel that mirrors the sky (the reflection map: sky above, a bright band, a dark ground)
    mat = p.mat("P_Dome", "chrome_env.png", diffuse=[0.74, 0.84, 1.0, 1], emissive=0.6, specular=0.9, power=60,
                texgen="reflect")
    m = k.Mesh("P_Dome_MF_Mesh")
    # a low cap 13.3 across rising to 3.7, on a thin rim
    r = 6.644
    prof = [(0.123, r), (0.45, r * 0.99)]
    for t in np.linspace(0.05, 1.0, 9):
        a = t * math.pi / 2
        prof.append((0.45 + (3.676 - 0.45) * math.sin(a), r * 0.99 * math.cos(a)))
    lathe(m, mat, prof, 24)
    p.node("P_Dome_Shadow", sh, pos=(0.248, 0.11, 0.248), s=1.3)
    p.node("P_Dome_MF", p.mesh(m))
    p.group("All_P_Dome", ["P_Dome_Shadow", "P_Dome_MF"])
    p.write()


def dome_look(p, r0, foot, y0, rise, ribs=12):
    """The dome as drawn: the cap of radius r0 rising `rise` from y0 (a quarter ellipse), its rim down to `foot`."""
    enamel = p.mat("P_Dome_Enamel", "dome_enamel.png", emissive=0.34, specular=0.85, power=48)
    brass = p.mat("P_Dome_Ribs", "brass.png", emissive=0.3, specular=0.7, power=28)
    m = k.Mesh("P_Dome_MF_Look")
    deep, top = -0.11, -0.012  # the panels and the metal's face, measured out of the cap along its normal

    def at(a, ph, off=0.0):
        nr, ny = rise * math.cos(a), r0 * math.sin(a)
        ln = math.hypot(nr, ny)
        nr, ny = nr / ln, ny / ln
        rad, y = r0 * math.cos(a) + nr * off, y0 + rise * math.sin(a) + ny * off
        return (np.array([rad * math.cos(ph), y, rad * math.sin(ph)]),
                np.array([nr * math.cos(ph), ny, nr * math.sin(ph)]))

    def grid(mt, rows):
        """Rows of (point, normal, uv) from the bottom up, each round with the angle: the faces between them."""
        ids = [[m.vertex(q, n, uv) for q, n, uv in row] for row in rows]
        for i in range(len(ids) - 1):
            for j in range(len(ids[i]) - 1):
                a, b, c, d = ids[i][j], ids[i][j + 1], ids[i + 1][j + 1], ids[i + 1][j]
                m.tri(mt, a, c, b)
                m.tri(mt, a, d, c)

    def patch(mt, a0, a1, ph0, ph1, off, na, nph, v0=0.0, v1=1.0, u1=1.0):
        grid(mt, [[at(a, ph, off) + ((u1 * s, v1 + (v0 - v1) * t),) for s, ph in zip(np.linspace(0, 1, nph + 1),
                                                                                    np.linspace(ph0, ph1, nph + 1))]
                  for t, a in zip(np.linspace(0, 1, na + 1), np.linspace(a0, a1, na + 1))])

    def step(mt, a, ph0, ph1, nph, up):
        """The metal's edge at angle a: a wall from its face down to the panels, facing up the dome or down it."""
        rows = []
        for off in ((deep, top) if up else (top, deep)):
            row = []
            for s, ph in zip(np.linspace(0, 1, nph + 1), np.linspace(ph0, ph1, nph + 1)):
                q, n = at(a, ph, off)
                t = at(a + 0.01, ph)[0] - at(a - 0.01, ph)[0]
                t /= np.linalg.norm(t)
                row.append((q, t if up else -t, (s * 8, 0.5 + off)))
            rows.append(row)
        grid(mt, rows)

    def rivet(a, ph, side=0.0, size=0.085):
        q, n = at(a, ph, top - size * 0.45)
        e = np.array([-math.sin(ph), 0.0, math.cos(ph)])
        k.sphere(m, brass, tuple(q + e * side), size, seg=6)

    full = 2 * math.pi
    a_ring, a_hoop, a_hub = 0.13, 0.66, math.acos(1.35 / r0)
    hoop = 0.032  # the hoop's half width, as an angle up the dome
    # the enamel: a panel a picture, two rows of them between the ribs
    for i in range(ribs):
        ph0, ph1 = full * i / ribs, full * (i + 1) / ribs
        patch(enamel, a_ring, a_hoop, ph0, ph1, deep, 6, 4)
        patch(enamel, a_hoop, a_hub, ph0, ph1, deep, 7, 4)
    # the base ring: the rim's wall, the ring's face up to its edge, riveted
    grid(brass, [[(np.array([r0 * math.cos(ph), y, r0 * math.sin(ph)]), np.array([math.cos(ph), 0, math.sin(ph)]),
                   (ph * 6, y)) for ph in np.linspace(0, full, 49)] for y in (foot, y0)])
    patch(brass, 0.0, a_ring, 0, full, top, 2, 48, u1=40.0, v1=0.4)
    step(brass, a_ring, 0, full, 48, True)
    for i in range(ribs * 3):
        rivet(a_ring * 0.5, full * (i + 0.5) / (ribs * 3))
    # the hoop and the hub
    patch(brass, a_hoop - hoop, a_hoop + hoop, 0, full, top, 1, 48, u1=40.0, v1=0.3)
    step(brass, a_hoop - hoop, 0, full, 48, False)
    step(brass, a_hoop + hoop, 0, full, 48, True)
    patch(brass, a_hub, math.pi / 2, 0, full, top, 4, 24, u1=6.0)
    step(brass, a_hub, 0, full, 24, False)
    for i in range(8):
        rivet(a_hub + 0.07, full * i / 8)
    k.sphere(m, brass, tuple(at(math.pi / 2, 0, top - 0.16)[0]), 0.3, seg=10)
    # the ribs: a strip 0.44 wide up the dome on each meridian, its two sides down to the enamel, rivets along it
    half = 0.22
    for i in range(ribs):
        ph = full * i / ribs
        e = np.array([-math.sin(ph), 0.0, math.cos(ph)])
        steps = np.linspace(a_ring, a_hub, 15)
        face, left, right = [], [], []
        length = 0.0
        for j, a in enumerate(steps):
            (q, n), (low, _) = at(a, ph, top), at(a, ph, deep)
            if j:
                length += float(np.linalg.norm(q - at(steps[j - 1], ph, top)[0]))
            v = length / 2.0
            face.append([(q - e * half, n, (0.0, v)), (q + e * half, n, (0.25, v))])
            left.append([(low - e * half, -e, (0.3, v)), (q - e * half, -e, (0.35, v))])
            right.append([(q + e * half, e, (0.3, v)), (low + e * half, e, (0.35, v))])
        for rows in (face, left, right):
            grid(brass, rows)
        for a in np.linspace(a_ring + 0.09, a_hub - 0.09, 9):
            if abs(a - a_hoop) > 0.05:
                rivet(a, ph)
        # and one on the hoop each side of the rib
        for side in (-1, 1):
            rivet(a_hoop, ph + side * full / ribs / 4)
    return m


def make_trafos():
    """The transformer pads: a steel ring 4.93 across and 0.48 high with the ball kind's colour inside (wood amber,
    stone green, paper blue, and our sticky ball's crimson for P_Trafo_Sticky), and its shadow."""
    colours = {"Wood": (0.92, 0.58, 0.09), "Stone": (0.09, 0.92, 0.58), "Paper": (0.09, 0.46, 0.92),
               "Sticky": (0.92, 0.09, 0.32)}
    for kind, c in colours.items():
        p = Part(f"PH/P_Trafo_{kind}")
        sh = shadow(p, f"P_Trafo_{kind}_Shadow", f"P_Trafo_{kind}_Shadow")
        steel = p.mat(f"P_Trafo_{kind}", "chrome_env.png", diffuse=[0.6, 0.65, 0.74, 1], emissive=0.29,
                      texgen="reflect")
        colour = p.mat(f"P_Trafo_{kind}_Color", "chrome_env.png", diffuse=list(c) + [1.0], emissive=0.2,
                       texgen="reflect")
        m = k.Mesh(f"P_Trafo_{kind}_Mesh")
        trafo_ring(m, steel, colour)
        # what is drawn: the original's kind of pad (measured: 4.94 across, 0.48 high), a rounded square frame of
        # chrome tube round a disc of the ball's own material, a coloured stud on each corner, on a riveted plate
        core = p.mat(f"Ball_{kind}", f"ball_{kind.lower()}.png", emissive=0.35, specular=0.3, power=20)
        base = p.mat("P_Trafo_Base", "steel.png", diffuse=0.75, emissive=0.3, specular=0.4, power=25)
        m.look = trafo_look(f"P_Trafo_{kind}_Look", steel, colour, core, base)
        p.node(f"P_Trafo_{kind}_Shadow", sh, pos=(0.08, -0.029, 0.219), s=0.453)
        p.node(f"P_Trafo_{kind}_MF", p.mesh(m))
        p.group(f"All_P_Trafo_{kind}", [f"P_Trafo_{kind}_Shadow", f"P_Trafo_{kind}_MF"])
        p.write()


def smooth_normals(m):
    """Gives every vertex the mean normal of the faces round its place (a smooth surface; the mesh's own seams, where
    a texture wraps, stay invisible)."""
    P = np.asarray(m.p)
    acc = {}
    for idx in m.parts.values():
        for t in range(0, len(idx), 3):
            a, b, c = idx[t:t + 3]
            n = np.cross(P[b] - P[a], P[c] - P[a])
            for i in (a, b, c):
                key = tuple(np.round(P[i], 4))
                acc[key] = acc.get(key, 0) + n
    m.n = [tuple(v / (np.linalg.norm(v) + 1e-12)) for v in (acc.get(tuple(np.round(q, 4)), np.array([0, 1.0, 0]))
                                                          for q in P)]


def rounded_square(half, corner, n=8):
    """A rounded square's outline (x, z), counter-clockwise from above."""
    out = []
    for cx, cz, a0 in ((half - corner, half - corner, 0), (-(half - corner), half - corner, 90),
                       (-(half - corner), -(half - corner), 180), (half - corner, -(half - corner), 270)):
        for i in range(n + 1):
            a = math.radians(a0 + 90 * i / n)
            out.append((cx + corner * math.cos(a), cz + corner * math.sin(a)))
    return out


def trafo_look(name, steel, colour, core, base):
    m = k.Mesh(name)
    # the plate: 4.9 square, 0.1 thick, its corners cut round, riveted along the edges
    plate = rounded_square(2.45, 0.5, 4)
    k.hull(m, base, [(x, y, z) for x, z in plate for y in (0.0, 0.1)])
    for x, z in plate[::3]:
        k.cylinder(m, steel, (x * 0.9, 0.1, z * 0.9), (0, 1, 0), 0.07, 0.04, seg=6)
    # the frame: a chrome tube round a rounded square, standing on the plate
    path = rounded_square(1.95, 0.7, 6)
    k.tube(m, steel, [(x, 0.3, z) for x, z in path + path[:1]], 0.2, seg=10)
    # the disc of the ball's material inside it, a little below the frame's top
    k.cylinder(m, core, (0, 0.1, 0), (0, 1, 0), 1.62, 0.12, seg=32, cap_mat=core)
    # a coloured stud on each corner, domed
    for x, z in ((1.6, 1.6), (-1.6, 1.6), (-1.6, -1.6), (1.6, -1.6)):
        k.cylinder(m, steel, (x, 0.1, z), (0, 1, 0), 0.34, 0.16, seg=16)
        k.ellipsoid(m, colour, (x, 0.26, z), (0.3, 0.18, 0.3), seg=16)
    smooth_normals(m)
    return m


def trafo_ring(m, steel, colour, a0=0.0, a1=360.0, seg=32):
    """A flat ring: outer steel lip, coloured groove, inner steel lip (or a part of it, from a0 to a1 degrees)."""
    rings = [(2.466, -0.008, 0.475, steel), (2.0, 0.05, 0.36, colour), (1.55, -0.008, 0.42, steel)]
    for (ro, y0, y1, mat), (ri, *_rest) in zip(rings, rings[1:] + [(1.2, 0, 0, None)]):
        n = max(2, int(seg * (a1 - a0) / 360))
        for i in range(n):
            t0, t1 = math.radians(a0 + (a1 - a0) * i / n), math.radians(a0 + (a1 - a0) * (i + 1) / n)
            p = lambda r, t, y: (r * math.cos(t), y, r * math.sin(t))
            m.face(mat, [p(ro, t0, y1), p(ro, t1, y1), p(ri, t1, y1), p(ri, t0, y1)], normal=(0, 1, 0))
            m.face(mat, [p(ro, t0, y0), p(ro, t0, y1), p(ro, t1, y1), p(ro, t1, y0)],
                   normal=(math.cos((t0 + t1) / 2), 0, math.sin((t0 + t1) / 2)))


def make_animtrafo():
    p = Part("AnimTrafo")
    bars = p.mat("AnimTrafo_Bars", "steel.png", diffuse=[0.6, 0.65, 0.74, 1], emissive=0.37, specular=0.5, power=20)
    ring = p.mat("AnimTrafo_RingParts", "chrome_env.png", diffuse=[0.77, 0.8, 0.85, 1], emissive=0.15,
                 texgen="reflect")
    ringc = p.mat("AnimTrafo_RingParts_Color", "chrome_env.png", diffuse=[0.09, 0.92, 0.58, 1], texgen="reflect")
    flash = p.mat("AnimTrafo_Flashfield", "flash.png", diffuse=[1, 1, 1, 0.65], emissive=1.0, alpha="BLEND",
                  blend=("one", "one"), double_sided=True)
    # the cage that rises out of the floor (5.5 tall), in the pad's style: a chrome post on each rounded corner, a
    # thinner bar in the middle of each side, a chrome frame at its foot and head as the pad's, and a band of the
    # kind's colour round its middle
    mb = k.Mesh("AnimTrafo_Bars_Mesh")
    frame = rounded_square(2.1, 0.75, 6)
    for x, z in ((1.8, 1.8), (-1.8, 1.8), (-1.8, -1.8), (1.8, -1.8)):
        k.cylinder(mb, ring, (x, -5.5, z), (0, 1, 0), 0.17, 5.49, seg=12)
        k.ellipsoid(mb, ring, (x, -0.02, z), (0.22, 0.12, 0.22), seg=12)
    for x, z in ((2.25, 0), (-2.25, 0), (0, 2.25), (0, -2.25)):
        k.cylinder(mb, bars, (x, -5.3, z), (0, 1, 0), 0.08, 5.1, seg=8)
    for y, r, mat in ((-5.25, 0.14, ring), (-0.3, 0.14, ring), (-2.8, 0.09, ringc)):
        k.tube(mb, mat, [(x, y, z) for x, z in frame + frame[:1]], r, seg=10)
    smooth_normals(mb)
    mf = k.Mesh("AnimTrafo_Flashfield_Mesh")
    quad_xz(mf, flash, 2.46, y=-0.024)
    # four quarters of the pad's frame, each in its frame's own turn (they open outwards along their frame's
    # (-1, 0, -1)): a quarter of the chrome tube round its rounded corner and that corner's coloured stud
    ring_mesh = k.Mesh("AnimTrafo_Ringpart1_Mesh")
    quarter = [(x, z) for x, z in rounded_square(1.95, 0.7, 12) if x <= 1e-6 and z <= 1e-6]
    quarter.sort(key=lambda q: math.atan2(q[1], q[0]))
    k.tube(ring_mesh, ring, [(x, 0.3, z) for x, z in [(-1.95, 0.0)] + quarter + [(0.0, -1.95)]], 0.2, seg=10,
           caps=True)
    k.cylinder(ring_mesh, ring, (-1.6, 0.1, -1.6), (0, 1, 0), 0.34, 0.16, seg=16)
    k.ellipsoid(ring_mesh, ringc, (-1.6, 0.26, -1.6), (0.3, 0.18, 0.3), seg=16)
    smooth_normals(ring_mesh)
    axes = [((1, 0, 0), 1), ((0, 0, 1), 2), ((-1, 0, 0), 3), ((0, 0, -1), 4)]
    for x, i in reversed(axes):
        m = k.Mesh(f"AnimTrafo_Ringpart{i}_Mesh")
        k.add(m, ring_mesh)
        p.node(f"AnimTrafo_Ringpart{i}", p.mesh(m), x=x)
    for x, i in reversed(axes):
        p.node(f"AnimTrafo_Frame0{i}", x=x)
    p.node("AnimTrafo_Flashfield", p.mesh(mf), x=(0, 0, 1))
    p.node("AnimTrafo_Bars", p.mesh(mb))
    p.node("AnimTrafo_MF")
    p.group("All_AnimTrafo", ["AnimTrafo_MF"] + [f"AnimTrafo_Frame0{i}" for i in (4, 3, 2, 1)] +
            [f"AnimTrafo_Ringpart{i}" for i in (4, 3, 2, 1)] + ["AnimTrafo_Flashfield", "AnimTrafo_Bars"])
    p.write()


def make_flames():
    """The start's four flames and a checkpoint's two: frames only (the flames are particles, the posts the
    level's own), at the measured places, each frame's y pointing along the flame's way out."""
    p = Part("PH/PS_FourFlames")
    for name, pos in (("D", (7.334, 2.053, -6.132)), ("A", (7.334, 2.053, 6.145)), ("B", (-7.221, 2.053, 6.098)),
                      ("C", (-7.221, 2.053, -6.132))):
        p.node(f"PS_FourFlames_Flame_{name}", pos=pos, x=(-1, 0, 0), y=(0, -0.007, 1), visible=True)
    p.node("PS_FourFlames_MF")
    p.group("All_PS_FourFlames", ["PS_FourFlames_MF"] + [f"PS_FourFlames_Flame_{c}" for c in "DCBA"])
    p.write()
    p = Part("PH/PC_TwoFlames")
    p.node("PC_TwoFlames_Flame_SmallB", pos=(0.039, 2.053, 7.06), x=(0, 0, -1), y=(-1, -0.007, 0), visible=True)
    p.node("PC_TwoFlames_Flame_SmallA", pos=(0.04, 2.053, -6.942), x=(0, 0, -1), y=(-1, -0.007, 0), visible=True)
    p.node("PC_TwoFlames_Flame_Big", pos=(0.0, 1.495, 0.0), x=(0, 0, -1), y=(-1, 0, 0), visible=True)
    p.node("PC_TwoFlames_MF")
    p.group("All_PC_TwoFlames", ["PC_TwoFlames_Flame_SmallB", "PC_TwoFlames_Flame_SmallA", "PC_TwoFlames_MF",
                                 "PC_TwoFlames_Flame_Big"])
    p.write()


def make_extra_life():
    p = Part("PH/P_Extra_Life")
    glow = p.mat("P_Extra_Life_Schatten", "glow.png", diffuse=[0, 0, 0, 0.17], emissive=[0.49, 0.3, 0.6],
                 alpha="BLEND", blend=("one", "one"), z_write=False)
    silver = p.mat("P_Extra_Life_SilverBall", "silver.png", emissive=0.0, alpha="BLEND",
                   blend=("src_alpha", "inv_src_alpha"), z_write=False)
    glass = p.mat("P_Extra_Life_Sphere", "oil.png", diffuse=[1, 1, 1, 1], emissive=0.0, alpha="BLEND",
                  blend=("one", "src_alpha"), z_write=False, texgen="reflect")
    ms = k.Mesh("P_Extra_Life_Shadow_Mesh")
    quad_xz(ms, glow, 1.17)
    mb = k.Mesh("P_Extra_Life_Sphere_Mesh")
    k.sphere(mb, glass, (0, 0, 0), 2.0, seg=16)
    sp = k.Mesh("P_Extra_Life_SilverBall_Sprite")
    sprite_quad(sp, silver)
    p.node("P_Extra_Life_Shadow", p.mesh(ms), pos=(0, -2.117, 0), s=1.4)
    p.node("P_Extra_Life_SilverBall", p.mesh(sp), x=(-0.324, 0, -0.946), y=(0.526, 0.831, -0.18),
           s=(0.997, 0.996, 0.996), sprite=(0, (1.0, 1.0), (0.0, 0.0)))
    p.node("P_Extra_Life_Sphere", p.mesh(mb))
    p.node("P_Extra_Life_Particle_Blob", s=2.0, visible=True)
    p.node("P_Extra_Life_Particle_Fizz", s=2.0, visible=True)
    p.node("P_Extra_Life_MF")
    p.group("All_P_Extra_Life", ["P_Extra_Life_MF", "P_Extra_Life_Sphere", "P_Extra_Life_Shadow",
                                 "P_Extra_Life_SilverBall", "P_Extra_Life_Particle_Blob",
                                 "P_Extra_Life_Particle_Fizz"])
    p.write()


def make_extra_point():
    """The extra points: a big silver ball in the middle and six small ones round it (sprites at the measured
    frames), over a soft glowing floor."""
    p = Part("PH/P_Extra_Point")
    floor = p.mat("ShadowPolyBig", "glow.png", diffuse=[0, 0, 0, 1], emissive=[0.45, 0.5, 0.6], alpha="BLEND",
                  blend=("one", "one"))
    silver = p.mat("P_Extra_Point_SilverBall", "silver.png", emissive=0.0, alpha="MASK", cutoff=0.5)
    frames = {0: (0.0, 0.0, 0.0), 1: (0.0, 0.0, 2.05), 2: (0.0, 2.0, 0.05), 3: (0.0, 0.0, -1.95),
              4: (-1.005, -0.967, 1.48), 5: (1.003, 0.998, -1.36), 6: (-1.345, 1.366, 0.0)}
    balls = {0: (0.0, 0.0, 0.0), 1: (0.0, 0.0, 2.0), 2: (0.0, 2.0, 0.0), 3: (0.0, 0.0, -2.0),
             4: (-1.005, -0.967, 1.433), 5: (1.003, 0.998, -1.414), 6: (-1.345, 1.366, -0.049)}
    frame_axes = {4: ((0.993, 0, -0.121), (0.082, 0.738, 0.669)), 5: ((1, 0, 0), (0, 0.981, 0.196)),
                  6: ((0.983, 0, 0.182), (-0.114, 0.78, 0.616))}
    for i in (4, 6):
        p.node(f"P_Extra_Point_Frame{i}", pos=frames[i], x=frame_axes[i][0], y=frame_axes[i][1], visible=True)
        sp = k.Mesh(f"P_Extra_Point_Ball{i}_Sprite")
        sprite_quad(sp, silver)
        p.node(f"P_Extra_Point_Ball{i}", p.mesh(sp), pos=balls[i], y=(0, 0.981, 0.196),
               sprite=(0, (0.5, 0.5), (0.0, 0.0)))
    mf = k.Mesh("P_Extra_Point_Floor_Mesh")
    quad_xz(mf, floor, 2.288)
    p.node("P_Extra_Point_Floor", p.mesh(mf), pos=(0, -2.1, 0))
    for i in (3, 0, 1, 5, 2):
        x, y = frame_axes.get(i, ((1, 0, 0), (0, 1, 0)))
        p.node(f"P_Extra_Point_Frame{i}", pos=frames[i], x=x, y=y, visible=True)
        sp = k.Mesh(f"P_Extra_Point_Ball{i}_Sprite")
        sprite_quad(sp, silver)
        size = 1.0 if i == 0 else 0.5
        p.node(f"P_Extra_Point_Ball{i}", p.mesh(sp), pos=balls[i], y=(0, 0.981, 0.196),
               sprite=(0, (size, size), (0.0, 0.0)))
    p.node("P_Extra_Point_MF", visible=True)
    hits = [f"P_Extra_Point_Ball_HitFrame0{i}" for i in (1, 2, 4, 3, 5, 6)]
    for h in hits:
        p.node(h, visible=True)
    p.group("P_Extra_Point_Hitframes", hits)
    p.group("All_P_Extra_Point", ["P_Extra_Point_MF"] + hits[::-1] +
            [n["name"] for n in p.nodes if n["name"].startswith(("P_Extra_Point_Frame", "P_Extra_Point_Ball"))
             and "Hit" not in n["name"]] + ["P_Extra_Point_Floor"])
    p.write()


# --- the modules with physics ------------------------------------------------------------------------------------


# --- the second round of redrawn parts (2026-11-12): looks only, the meshes the physics reads stay as they were ----


def annulus(m, mat, base, axis, r0, r1, length, seg=32, turns=8.0, face_mat=None):
    """A round ring with a hole, from `base` along `axis`: outer wall r1, inner wall r0, a flat face at each end
    (of `face_mat` when given); shaded round."""
    a, u, v = k._basis(axis)
    b = np.asarray(base, np.float64)
    fm = mat if face_mat is None else face_mat
    top = a * length

    def quad(mt, pts, nrms, uvs):
        ids = [m.vertex(q, n, t) for q, n, t in zip(pts, nrms, uvs)]
        if np.dot(np.cross(pts[1] - pts[0], pts[2] - pts[0]), nrms[0] + nrms[2]) < 0:
            ids = ids[::-1]
        m.tri(mt, ids[0], ids[1], ids[2])
        m.tri(mt, ids[0], ids[2], ids[3])

    for j in range(seg):
        t0, t1 = 2 * math.pi * j / seg, 2 * math.pi * (j + 1) / seg
        d0, d1 = u * math.cos(t0) + v * math.sin(t0), u * math.cos(t1) + v * math.sin(t1)
        u0, u1 = j / seg * turns, (j + 1) / seg * turns
        wall = [(u0, 0), (u1, 0), (u1, length / 4), (u0, length / 4)]
        quad(mat, [b + d0 * r1, b + d1 * r1, b + d1 * r1 + top, b + d0 * r1 + top], [d0, d1, d1, d0], wall)
        if r0 > 0:
            quad(mat, [b + d0 * r0, b + d1 * r0, b + d1 * r0 + top, b + d0 * r0 + top], [-d0, -d1, -d1, -d0], wall)
        for off, n in ((top * 0, -a), (top, a)):
            quad(fm, [b + off + d0 * r0, b + off + d1 * r0, b + off + d1 * r1, b + off + d0 * r1], [n] * 4,
                 [(u0, 0), (u1, 0), (u1, (r1 - r0) / 4), (u0, (r1 - r0) / 4)])


def bolt_ring(m, mat, centre, axis, r, n, size=0.11, proud=0.05, phase=0.0):
    """`n` hexagonal bolt heads on a circle of radius r about `axis`, standing `proud` of the face at `centre`."""
    a, u, v = k._basis(axis)
    c = np.asarray(centre, np.float64)
    for i in range(n):
        t = phase + 2 * math.pi * i / n
        k.prism(m, mat, c + (u * math.cos(t) + v * math.sin(t)) * r, a, size, proud, 6)


def target_face(m, mat, corner, du, dv):
    """A flat face carrying the whole target picture once (never stretched over the faces round it): from `corner`
    along `du` and `dv`, facing du x dv."""
    c, du, dv = (np.asarray(q, np.float64) for q in (corner, du, dv))
    m.face(mat, [c, c + du, c + du + dv, c + dv], [(0, 1), (1, 1), (1, 0), (0, 0)], normal=np.cross(du, dv))


# The sack as drawn: rings (height, half width in x, half width in z, how deep its pleats are) from its sewn bottom
# up to the gathered cloth above the tie. They keep within the physics' hull (sack_points) but for the pleats.
SACK_RINGS = [(-2.93, 0.04, 1.9, 0.0), (-2.62, 0.4, 1.84, 0.0), (-2.3, 0.78, 1.74, 0.0), (-2.03, 1.05, 1.66, 0.0),
              (-1.5, 1.27, 1.75, 0.0), (-1.03, 1.36, 1.79, 0.0), (-0.03, 1.36, 1.79, 0.0), (0.5, 1.29, 1.7, 0.015),
              (0.97, 1.14, 1.55, 0.03), (1.3, 0.95, 1.27, 0.06), (1.58, 0.71, 0.95, 0.09), (1.78, 0.5, 0.58, 0.12),
              (1.92, 0.36, 0.38, 0.1), (2.02, 0.4, 0.44, 0.14), (2.2, 0.5, 0.58, 0.2), (2.34, 0.56, 0.66, 0.24)]


def sack_at(i, a, out=1.0):
    """A point of the drawn sack: ring i at angle a (the bulges of the sand low down, the pleats towards the neck)."""
    y, rx, rz, pleat = SACK_RINGS[i]
    f = (1 + pleat * math.cos(10 * a) + 0.022 * math.sin(3 * a + 1.7 * y)) * out
    return np.array([rx * f * math.cos(a), y, rz * f * math.sin(a)])


def sack_look(name, cloth, cord):
    """The sack as drawn, inside the physics' hull: a sewn canvas sack heavy with sand, wedge-bottomed along its
    bottom seam, widest low down, its side seams running up to a pleated neck tied with three turns of cord, the
    cloth gathered above the tie, the cord's ends hanging over to one side (where the hull's tie is)."""
    m = k.Mesh(name)
    seg = 40
    rows = []
    for i, (y, rx, rz, pleat) in enumerate(SACK_RINGS):
        rows.append([m.vertex(sack_at(i, 2 * math.pi * j / seg), (0, 1, 0), (j / seg * 2, (y + 3.0) / 5.4))
                     for j in range(seg + 1)])
    P = lambda i: np.array(m.p[i])

    def tri(a, b, c, ref):
        if np.dot(np.cross(P(b) - P(a), P(c) - P(a)), ref) < 0:
            b, c = c, b
        m.tri(cloth, a, b, c)

    for i in range(len(rows) - 1):
        for j in range(seg):
            a, b, c, d = rows[i][j], rows[i][j + 1], rows[i + 1][j + 1], rows[i + 1][j]
            ref = (P(a) + P(c)) * np.array([1, 0, 1]) + np.array([0, -0.4 if i < 3 else 0.0, 0])
            tri(a, c, b, ref)
            tri(a, d, c, ref)
    lo = m.vertex((0, -2.975, 0), (0, -1, 0), (0.5, 0))
    hi = m.vertex((0, 2.16, 0), (0, 1, 0), (0.5, 1))
    for j in range(seg):
        tri(rows[0][j], rows[0][j + 1], lo, (0, -1, 0))
        tri(rows[-1][j], rows[-1][j + 1], hi, (0, 1, 0))
    smooth_normals(m)
    # the seam: down one side, along the bottom, up the other, and an ear at each bottom corner
    top = 11
    path = [sack_at(i, math.pi / 2, 1.012) for i in range(top, -1, -1)]
    path += [np.array([0, -2.99, 0.0])] + [sack_at(i, 3 * math.pi / 2, 1.012) for i in range(0, top + 1)]
    k.tube(m, cord, path, 0.05, seg=6)
    for s in (-1, 1):
        k.cylinder(m, cloth, (0, -2.84, s * 1.8), (0, -0.25, s), 0.17, 0.2, seg=8, r_top=0.03)
    # the tie: three turns, a knot, two ends leaning over to +z
    for y, r in ((1.83, 0.5), (1.92, 0.44), (2.01, 0.47)):
        k.torus(m, cord, (0, y, 0.0), (0, 1, 0), r, 0.055, seg=20, tube_seg=6)
    k.ellipsoid(m, cord, (0.0, 1.93, 0.52), (0.15, 0.13, 0.11), seg=8)
    k.tube(m, cord, [(0.06, 1.95, 0.55), (0.1, 2.2, 0.95), (0.1, 2.22, 1.3), (0.1, 2.02, 1.55)], 0.05, seg=6, caps=True)
    k.tube(m, cord, [(-0.06, 1.95, 0.55), (-0.3, 2.3, 0.85), (-0.35, 2.3, 1.15), (-0.35, 2.05, 1.42)], 0.05, seg=6,
           caps=True)
    return m


def sack_rope_look(name, rope, iron, brass):
    """The sack's rope as drawn (the physics' is a plain rod): a twisted rope, a brass ferrule and an iron eye at
    its head (the eye hangs in the holder's ring), a stopper knot where the sack's neck is tied round it."""
    m = k.Mesh(name)
    k.cylinder(m, rope, (0, -6.25, 0), (0, 1, 0), 0.13, 11.72, seg=10)
    k.cylinder(m, brass, (0, 5.08, 0), (0, 1, 0), 0.175, 0.4, seg=10)
    k.torus(m, iron, (0, 5.72, 0), (1, 0, 0), 0.26, 0.07, seg=16, tube_seg=6)
    k.ellipsoid(m, rope, (0, -5.93, 0), (0.25, 0.2, 0.25), seg=10)
    return m


def sack_holder_look(name, plate, iron, brass):
    """The sack's holder as drawn: its plate (the measured box), a boss under it with the ring the rope's eye hangs
    in, and four bolts through the plate."""
    m = k.Mesh(name)
    k.box(m, plate, (-0.733, 0.0, -1.427), (0.733, 0.255, 1.426))
    k.cylinder(m, iron, (0, -0.07, 0), (0, 1, 0), 0.4, 0.07, seg=16)
    k.torus(m, iron, (0, -0.29, 0), (0, 0, 1), 0.26, 0.07, seg=16, tube_seg=6)
    for x in (-0.47, 0.47):
        for z in (-1.12, 1.12):
            k.prism(m, brass, (x, 0.255, z), (0, 1, 0), 0.13, 0.06, 6)
            k.prism(m, brass, (x, 0.0, z), (0, -1, 0), 0.13, 0.06, 6)
    return m


def flap_panel(paint, wood, bolts):
    """One flap as drawn, the physics' board (6.39 x 0.50 x 5.00, the axle through its root): a painted iron frame,
    its root rail round the axle, a rail across the middle, oak boards set back in it, a bolt at each corner."""
    m = k.Mesh("flap")
    x0, x1, y0, y1, z0, z1 = -0.308, 6.085, -0.254, 0.246, -10.0, -5.0
    w = 0.42
    k.box(m, paint, (x0, y0, z0), (x1, y1, z0 + w))
    k.box(m, paint, (x0, y0, z1 - w), (x1, y1, z1))
    k.box(m, paint, (x0, y0, z0 + w), (x0 + 0.62, y1, z1 - w))
    k.box(m, paint, (x1 - w, y0, z0 + w), (x1, y1, z1 - w))
    k.box(m, paint, (2.95, y0 + 0.04, z0 + w), (3.27, y1 - 0.04, z1 - w))
    k.box(m, wood, (x0 + 0.62, y0 + 0.1, z0 + w), (x1 - w, y1 - 0.1, z1 - w), scale=2.0)
    for x in (x0 + 0.31, 3.11, x1 - 0.21):
        for z in (z0 + 0.21, z1 - 0.21):
            k.prism(m, bolts, (x, y1, z), (0, 1, 0), 0.11, 0.05, 6)
            k.prism(m, bolts, (x, y0, z), (0, -1, 0), 0.11, 0.05, 6)
    return m


def flaps_look(name, paint, wood, bolts):
    """Both flaps as drawn: the second is the first turned a quarter about the axle at the other end."""
    m = k.Mesh(name)
    one = flap_panel(paint, wood, bolts)
    k.add(m, one)
    turn = np.array([[0.0, -1, 0, 0], [1, 0, 0, 0], [0, 0, 1, 15.0], [0, 0, 0, 1]])
    k.add(m, one, turn)
    return m


def flap_axis_look(name, shaft, iron, hub, paint, bolts):
    """The flap wheel's axle as drawn: a steel shaft in a sleeve, a spoked wheel in the middle (inside the physics'
    hexagon: painted rim, six iron spokes, a bolted boss), a bearing collar either side of the wheel and a bolted
    bearing flange at each end."""
    m = k.Mesh(name)
    k.cylinder(m, shaft, (0, 0, -10.0), (0, 0, 1), 0.2, 20.0, seg=12)
    k.cylinder(m, iron, (0, 0, -4.064), (0, 0, 1), 0.29, 7.826, seg=12)
    annulus(m, paint, (0.001, 0, -0.273), (0, 0, 1), 2.2, 2.6, 0.567, seg=36)
    for i in range(6):
        a = math.radians(60 * i)
        d, t = (math.cos(a), math.sin(a), 0), (-math.sin(a), math.cos(a), 0)
        k.obox(m, iron, (1.4 * d[0], 1.4 * d[1], 0.01), [d, (0, 0, 1), t], (0.86, 0.2, 0.16))
    k.cylinder(m, hub, (0, 0, -0.36), (0, 0, 1), 0.66, 0.74, seg=20)
    for z, ax in ((-0.36, (0, 0, -1)), (0.38, (0, 0, 1))):
        bolt_ring(m, bolts, (0, 0, z), ax, 0.46, 6, size=0.09, proud=0.04)
        k.cylinder(m, bolts, (0, 0, z), ax, 0.27, 0.07, seg=12)
    for z in (-4.75, 4.25):
        k.cylinder(m, hub, (0, 0, z), (0, 0, 1), 0.44, 0.5, seg=14)
        k.cylinder(m, bolts, (0, 0, z + 0.19), (0, 0, 1), 0.47, 0.12, seg=14)
    for z, ax in ((-9.55, (0, 0, -1)), (9.55, (0, 0, 1))):
        k.cylinder(m, hub, (0, 0, z), ax, 1.22, 0.49, seg=24)
        bolt_ring(m, bolts, (0, 0, z + ax[2] * 0.49), ax, 0.88, 6, size=0.11, proud=0.05)
        k.cylinder(m, iron, (0, 0, z + ax[2] * 0.49), ax, 0.4, 0.08, seg=12)
    return m


def spinner_look(name, arm, paddle, paint, iron, hub, steel, bolts, target):
    """The turning arms as drawn: the physics' tapered arm painted, banded twice each side; a turned drum on a
    spindle for the hub (the arm turns about the upright); each paddle a framed iron plate held by a fork and two
    stays, with the whole target, rings round a bull's eye, once on each face."""
    m = k.Mesh(name)
    arm(m, paint)
    section = lambda z: (1.068 - 0.708 * (abs(z) - 0.73) / 7.006, 1.069 - 0.569 * (abs(z) - 0.73) / 7.006)
    for z in (-5.4, -2.9, 2.9, 5.4):
        w, h = section(z)
        k.box(m, iron, (-w / 2 - 0.035, -h / 2 - 0.035, z - 0.15), (w / 2 + 0.035, h / 2 + 0.035, z + 0.15))
        for s in (-1, 1):
            k.prism(m, bolts, (s * 0.0, h / 2 + 0.035, z), (0, 1, 0), 0.09, 0.04, 6)
            k.prism(m, bolts, (s * (w / 2 + 0.035), 0, z), (s, 0, 0), 0.09, 0.04, 6)
    k.cylinder(m, hub, (0, -0.62, 0), (0, 1, 0), 0.86, 1.24, seg=24)
    bolt_ring(m, bolts, (0, 0.62, 0), (0, 1, 0), 0.62, 6, size=0.1, proud=0.05)
    k.prism(m, iron, (0, 0.62, 0), (0, 1, 0), 0.3, 0.16, 6)
    k.cylinder(m, steel, (0, -1.2, 0), (0, 1, 0), 0.3, 0.58, seg=12)
    k.cylinder(m, iron, (0, -1.3, 0), (0, 1, 0), 0.6, 0.16, seg=16)
    for s in (-1, 1):
        z0, z1 = (8.573, 11.621) if s > 0 else (-11.621, -8.573)
        w = 0.24
        k.box(m, iron, (-0.155, -1.4, z0), (0.155, -1.4 + w, z1))
        k.box(m, iron, (-0.155, 1.4 - w, z0), (0.155, 1.4, z1))
        k.box(m, iron, (-0.155, -1.4 + w, z0), (0.155, 1.4 - w, z0 + w))
        k.box(m, iron, (-0.155, -1.4 + w, z1 - w), (0.155, 1.4 - w, z1))
        k.box(m, paddle, (-0.11, -1.4 + w, z0 + w), (0.11, 1.4 - w, z1 - w))
        span_y, span_z = 2.8 - 2 * w, z1 - z0 - 2 * w
        target_face(m, target, (0.112, -1.4 + w, z0 + w), (0, 0, span_z), (0, span_y, 0))
        target_face(m, target, (-0.112, -1.4 + w, z1 - w), (0, 0, -span_z), (0, span_y, 0))
        for x in (-0.155, 0.155):
            for y in (-1.28, 1.28):
                for z in (z0 + 0.12, z1 - 0.12):
                    k.prism(m, bolts, (x, y, z), (math.copysign(1, x), 0, 0), 0.08, 0.04, 6)
        # the fork from the arm's end to the paddle's inner edge, and a stay above and below
        inner = z0 if s > 0 else z1
        k.box(m, iron, (-0.12, -0.3, min(s * 7.5, inner)), (0.12, 0.3, max(s * 7.5, inner)))
        for y in (-1, 1):
            k.tube(m, steel, [(0, y * 0.33, s * 6.1), (0, y * 1.22, inner + s * 0.12)], 0.06, seg=6)
    return m


def fan_look(name, hub, blade, tip, steel, brass):
    """The fan as drawn (everything turns with the rotor, so everything but the blades is round about the axle): a
    hub with a brass nose, four broad twisted blades of painted sheet with brass tips and a turned shroud ring round
    them. No guard of its own: the level's grille over the shaft is the guard (as the original's fan has it), and a
    guard of wire rings turning just under the grille, at its height, drew a second grid through the grille."""
    m = k.Mesh(name)
    k.cylinder(m, hub, (0, 0.08, 0), (0, 1, 0), 0.4, 0.8, seg=16)
    k.cylinder(m, brass, (0, 0.88, 0), (0, 1, 0), 0.4, 0.2, seg=16, r_top=0.14)
    stations = [(0.36, 0.2, 0.3), (0.8, 0.44, 0.3), (1.22, 0.54, 0.25), (1.52, 0.44, 0.2)]
    up = lambda h: np.array([0, h, 0.0])
    for b in range(4):
        t = math.pi / 2 * b
        d, s = np.array([math.cos(t), 0, math.sin(t)]), np.array([-math.sin(t), 0, math.cos(t)])
        for (r0, c0, p0), (r1, c1, p1) in zip(stations, stations[1:]):
            pts = [d * r0 + s * c0 + up(0.5 - p0), d * r1 + s * c1 + up(0.5 - p1),
                   d * r1 - s * c1 + up(0.5 + p1), d * r0 - s * c0 + up(0.5 + p0)]
            m.face(tip if r1 > 1.4 else blade, pts)
    annulus(m, hub, (0, 0.02, 0), (0, 1, 0), 1.68, 1.8, 1.0, seg=36, turns=10.0)
    return m


def make_modul_01():
    """The pusher: a stone block on a long beam with an end plate, sliding in its channel (the channel's floor,
    sides and filler are collision only: the level draws the channel)."""
    p = Part("PH/P_Modul_01")
    stone = p.mat("P_Modul_01_Stone", "stone.png", diffuse=0.59, emissive=0.38, specular=0.3, power=10)
    target = p.mat("P_Modul_01_Target", "target.png", emissive=0.27)
    wood = p.mat("P_Modul_01_Wood", "wood.png", diffuse=0.76, emissive=0.43, specular=0.25, power=10)
    plate = lambda m, mt: k.box(m, mt, (-8.008, -0.71, -1.98), (-7.508, 2.89, 1.98))
    beam = lambda m, mt: k.box(m, mt, (-7.276, -0.91, -0.6), (2.592, 1.099, 0.6))
    block = lambda m, mt: prism_xy(m, mt, [(-2.47, 1.59), (2.45, 1.59), (3.457, 5.938), (-3.578, 5.938)],
                                   -0.313, 0.313)
    m = k.Mesh("P_Modul_01_Pusher_Mesh")
    # the look stays inside the collision shapes: a round target on the end plate, a wooden rod along the beam into
    # a stone block, and three wooden pegs fanning up from the block (where the trapezoid's hull is)
    k.cylinder(m, target, (-8.008, 1.09, 0), (1, 0, 0), 1.8, 0.5, seg=20)
    k.cylinder(m, wood, (-7.508, 0.1, 0), (1, 0, 0), 0.45, 5.0, seg=10)
    k.box(m, stone, (-2.6, -0.91, -0.6), (2.592, 1.099, 0.6))
    for x0, x1 in ((-2.2, -3.2), (0.0, 0.0), (2.1, 3.1)):
        k.tube(m, wood, [(x0, 1.0, 0), (x1, 5.75, 0)], 0.28, seg=8, caps=True)
    p.node("P_Modul_01_Pusher", p.mesh(m), pos=(2.508, -1.485, 0.65))
    rinne = p.mesh(col("P_Modul_01_Rinne_01_Mesh", lambda m: k.box(m, None, (-8.979, -1.582, -0.65),
                                                                       (4.752, -1.177, 0.65))))
    p.node("P_Modul_01_Rinne", rinne, pos=(4.752, -1.323, 0.65))
    filler = p.mesh(col("P_Modul_01_Filler_Mesh", lambda m: k.box(m, None, (-2.5, -0.203, -0.313),
                                                                      (2.5, 0.203, 0.313))))
    p.node("P_Modul_01_Filler", filler, pos=(2.5, -0.251, 0.65))
    p.node("P_Modul_01_MF")
    p.mesh(col("P_Modul_01_Col03_Mesh", lambda m: block(m, None)))
    p.mesh(col("P_Modul_01_Col02_Mesh", lambda m: beam(m, None)))
    p.mesh(col("P_Modul_01_Col01_Mesh", lambda m: plate(m, None)))
    p.mesh(col("P_Modul_01_Rinne_03_Mesh", lambda m: k.box(m, None, (-4.752, -1.177, 0.67), (3.747, 1.323, 1.509))))
    p.mesh(col("P_Modul_01_Rinne_02_Mesh", lambda m: k.box(m, None, (-4.752, -1.177, -1.51), (3.747, 1.323, -0.67))))
    p.group("All_P_Modul_01", ["P_Modul_01_MF", "P_Modul_01_Pusher", "P_Modul_01_Rinne", "P_Modul_01_Filler"])
    p.write()


def flat_xz(m, mat, pts, y, up=True, uv=None):
    """A flat convex polygon at height y from (x, z) points, facing up or down; `uv` a function of (x, z)."""
    q = [np.array((x, y, z), np.float64) for x, z in pts]
    n = np.cross(q[1] - q[0], q[2] - q[0])
    if (n[1] > 0) != up:
        q = q[::-1]
    uv = uv or (lambda x, z: (x / 4.0, z / 4.0))
    m.face(mat, q, [uv(v[0], v[2]) for v in q], normal=(0, 1 if up else -1, 0))


def ring_xz(m, mat, r0, r1, y, seg=40, turns=10.0, vspan=1.0):
    """A flat ring facing up, from radius r0 to r1 at height y; the texture's u goes round it `turns` times."""
    for j in range(seg):
        a0, a1 = 2 * math.pi * j / seg, 2 * math.pi * (j + 1) / seg
        pts = [(r0 * math.cos(a0), r0 * math.sin(a0)), (r1 * math.cos(a0), r1 * math.sin(a0)),
               (r1 * math.cos(a1), r1 * math.sin(a1)), (r0 * math.cos(a1), r0 * math.sin(a1))]
        uvs = [(j / seg * turns, 0), (j / seg * turns, vspan), ((j + 1) / seg * turns, vspan),
               ((j + 1) / seg * turns, 0)]
        q = [np.array((x, y, z), np.float64) for x, z in pts]
        if np.cross(q[1] - q[0], q[2] - q[0])[1] < 0:
            q, uvs = q[::-1], uvs[::-1]
        m.face(mat, q, uvs, normal=(0, 1, 0))


def lift_disc(name, metal, dark, deck, bolts):
    """The lift's disc as drawn, inside the physics' slab (radius 10, twenty sides, 1.04 thick): a turned metal
    body with chamfered edges, on top a wide rim the cage stands on, bolted between the panels' places, a brass
    line, an oak deck within it and a bolted boss plate in the middle (the plate and the bolts' heads stand a few
    hundredths proud of the slab)."""
    m = k.Mesh(name)
    top = 0.52
    lathe(m, metal, [(-0.52, 9.5), (-0.36, 9.87), (0.38, 9.87), (top, 9.74)], seg=40)
    flat_xz(m, dark, regular(40, 9.5), -0.52, up=False)
    ring_xz(m, metal, 7.6, 9.74, top, turns=12.0, vspan=0.55)
    ring_xz(m, bolts, 7.42, 7.6, top, turns=40.0, vspan=0.05)
    flat_xz(m, deck, regular(40, 7.42), top)
    # sixteen bolts round the rim: a pair outside each panel, none under the cage's corners
    for i in range(16):
        a = math.radians(11.25 + 22.5 * i)
        k.prism(m, bolts, (9.42 * math.cos(a), top, 9.42 * math.sin(a)), (0, 1, 0), 0.17, 0.07, 6, phase=a)
    k.cylinder(m, metal, (0, top, 0), (0, 1, 0), 1.45, 0.03, seg=24)
    k.cylinder(m, dark, (0, top + 0.03, 0), (0, 1, 0), 0.5, 0.03, seg=16)
    for i in range(6):
        a = math.radians(60 * i)
        k.prism(m, bolts, (1.08 * math.cos(a), top + 0.03, 1.08 * math.sin(a)), (0, 1, 0), 0.13, 0.05, 6, phase=a)
    return m


def lift_panel(m, paint, wood, bars, bolts):
    """One of the lift's loose panels as drawn, inside the physics' trapezoid prism (7.24 wide outside, 6.51
    inside, 0.88 deep, 7.28 high): two stiles with the prism's slanted sides, three rails set back between them,
    boards filling the foot and six round bars the upper part (the ball is seen through them), brass bolts where
    the rails meet the stiles."""
    z0, z1, y0, y1 = -0.438, 0.438, -2.72, 4.565
    xi = 2.705
    for s in (-1, 1):
        k.hexahedron(m, paint, [(s * 3.618, y0, z0), (s * xi, y0, z0), (s * xi, y0, z1), (s * 3.255, y0, z1),
                                (s * 3.618, y1, z0), (s * xi, y1, z0), (s * xi, y1, z1), (s * 3.255, y1, z1)])
    rails = ((y0, -2.22, 0.36), (-0.3, 0.14, 0.3), (4.02, y1, 0.36))
    for lo, hi, d in rails:
        k.box(m, paint, (-xi, lo, -d), (xi, hi, d))
    k.box(m, wood, (-xi, -2.22, -0.17), (xi, -0.3, 0.17), scale=2.0)
    for i in range(6):
        k.cylinder(m, bars, ((i - 2.5) * 0.78, 0.14, 0.0), (0, 1, 0), 0.125, 3.88, seg=10, caps=False)
    for z, axis, x in ((z0, (0, 0, -1), 3.16), (z1, (0, 0, 1), 2.98)):
        for s in (-1, 1):
            for y in (-2.47, -0.08, 4.29):
                k.prism(m, bolts, (s * x, y, z), axis, 0.13, 0.05, 6)


def lift_gate_trim(m, dark, bolts):
    """What the lift's gate has besides its posts and lintel: a dark plate on the lintel's outer face with two
    brass bolts, and a foot band on each post's inner side."""
    k.box(m, dark, (-1.6, 4.47, -0.52), (1.6, 4.95, -0.491))
    for x in (-1.25, 1.25):
        k.prism(m, bolts, (x, 4.71, -0.52), (0, 0, -1), 0.13, 0.05, 6)


def make_modul_03():
    """The lift: a disc on a spring, held down by the seven loose panels and the gate of its cage; pushed off, they
    fall away and it rises. The physics' shapes are the plain ones below (a twenty-sided slab, trapezoid panels, the
    gate's posts and lintel), and they stay the objects' meshes; what is drawn, their looks, is built into them: a
    turned metal disc with a bolted rim and an oak deck, and red-painted iron panels, each a frame with a boarded
    foot and bars above it (lift_disc, lift_panel, lift_gate_trim)."""
    p = Part("PH/P_Modul_03")
    floor = p.mat("P_Modul_03_Floor", "lift_rim.png", diffuse=0.85, emissive=0.3, specular=0.45, power=24)
    walls = p.mat("P_Modul_03_Walls", "lift_paint.png", diffuse=0.85, emissive=0.3, specular=0.2, power=16)
    trim = p.mat("P_Modul_03_Trim", "iron.png", emissive=0.3, specular=0.4, power=20)
    deck = p.mat("P_Modul_03_Deck", "lift_deck.png", diffuse=0.85, emissive=0.34, specular=0.1, power=10)
    bars = p.mat("P_Modul_03_Bars", "steel.png", diffuse=0.8, emissive=0.3, specular=0.6, power=30)
    bolts = p.mat("P_Modul_03_Bolts", "brass.png", diffuse=0.85, emissive=0.32, specular=0.6, power=30)
    slab = lambda m, mt: prism_xz(m, mt, regular(20, 10.0), -0.52, 0.52)
    # 7.24 wide in front, 6.51 behind, 0.88 deep, 7.28 high
    wall = lambda m, mt: k.hexahedron(
        m, mt, [(-3.618, -2.72, -0.438), (3.618, -2.72, -0.438), (3.255, -2.72, 0.438), (-3.255, -2.72, 0.438),
                (-3.618, 4.565, -0.438), (3.618, 4.565, -0.438), (3.255, 4.565, 0.438), (-3.255, 4.565, 0.438)])

    def post(m, mt, x_out, x_in, head=-0.46):
        # 1.01 wide, 1.32 deep at its foot and 0.86 (0.89 the right one) at its head, its outer back corner bevelled
        s = 1 if x_in > x_out else -1
        k.hexahedron(m, mt, [(x_out, -1.92, -0.92), (x_in, -1.92, -0.92), (x_in, -1.92, 0.395),
                             (x_out + s * 0.36, -1.92, 0.395),
                             (x_out, 4.128, head), (x_in, 4.128, head), (x_in, 4.128, 0.395),
                             (x_out + s * 0.36, 4.128, 0.395)])

    lintel = lambda m, mt: k.hexahedron(m, mt, [
        (-3.582, 4.287, -0.491), (3.601, 4.287, -0.491), (3.25, 4.287, 0.395), (-3.21, 4.287, 0.395),
        (-3.582, 5.438, -0.491), (3.601, 5.28, -0.491), (3.25, 5.09, 0.395), (-3.21, 5.09, 0.395)])
    # The plain shapes stay the objects' meshes, as they were: the slab is the physics' shape, and the test player
    # reads the modules' meshes as floors and walls (autopilot.rs, floor_triangles), so a panel must stay one closed
    # wall to it, not rails and bars. What is drawn is each mesh's look.
    mf = k.Mesh("P_Modul_03_Floor_Mesh")
    slab(mf, floor)
    mf.look = lift_disc("P_Modul_03_Floor_Look", floor, trim, deck, bolts)
    mw = k.Mesh("P_Modul_03_Wall01_Mesh")
    wall(mw, walls)
    k.box(mw, trim, (-3.5, 4.565, -0.35), (3.5, 4.75, 0.35))
    mw.look = k.Mesh("P_Modul_03_Wall01_Look")
    lift_panel(mw.look, walls, deck, bars, bolts)
    mg = k.Mesh("P_Modul_03_Gate_Mesh")
    mg.look = k.Mesh("P_Modul_03_Gate_Look")
    for m in (mg, mg.look):
        post(m, walls, -3.612, -2.6)
        post(m, walls, 3.601, 2.6, -0.49)
        lintel(m, walls)
    lift_gate_trim(mg.look, trim, bolts)
    p.node("P_Modul_03_frame_high", pos=(0, 17.079, 0), x=(0, 0, 1), s=0.313)
    p.node("P_Modul_03_frame_low", pos=(0, -6.117, 0), x=(0, 0, 1), s=0.313)
    p.node("P_Modul_03_Floor", p.mesh(mf), pos=(0, -9.118, 0), x=(-0.707, 0, 0.707))
    wi = p.mesh(mw)
    walls_at = [("07", (5.998, -5.75, -5.998), (0.707, 0, 0.707)), ("06", (0, -5.75, -8.497), (1, 0, 0)),
                ("05", (-5.998, -5.75, -5.998), (0.707, 0, -0.707)), ("04", (-8.497, -5.75, 0), (0, 0, -1)),
                ("03", (-5.998, -5.75, 5.998), (-0.707, 0, -0.707)), ("02", (0, -5.75, 8.497), (-1, 0, 0)),
                ("01", (5.998, -5.75, 5.998), (-0.707, 0, 0.707))]
    for n, pos, x in walls_at:
        p.node(f"P_Modul_03_Wall{n}", wi, pos=pos, x=x, s=(1, 0.999, 1))
    p.node("P_Modul_03_Gate", p.mesh(mg), pos=(8.497, -6.59, 0), x=(0, 0, 1), s=(1, 0.999, 1))
    p.node("P_Modul_03_MF")
    p.mesh(col("P_Modul_03_Wall_Coll_Mesh", lambda m: wall(m, None)))
    p.mesh(col("P_Modul_03_Gate_Coll03_Mesh", lambda m: post(m, None, 3.601, 2.6, -0.49)))
    p.mesh(col("P_Modul_03_Gate_Coll02_Mesh", lambda m: lintel(m, None)))
    p.mesh(col("P_Modul_03_Gate_Coll01_Mesh", lambda m: post(m, None, -3.612, -2.6)))
    p.group("P_Modul_03_FallingParts", ["P_Modul_03_Gate"] + [f"P_Modul_03_Wall0{i}" for i in range(1, 8)])
    p.group("All_P_Modul_03", ["P_Modul_03_MF", "P_Modul_03_frame_high", "P_Modul_03_frame_low", "P_Modul_03_Floor",
                               "P_Modul_03_Gate"] + [f"P_Modul_03_Wall0{i}" for i in range(7, 0, -1)])
    p.write()


def make_modul_08():
    """The swing: a boat-shaped wooden seat on two iron rods, hanging from a stone beam with iron clamps."""
    p = Part("PH/P_Modul_08")
    stone = p.mat("P_Modul_08_Stone", "stone.png", diffuse=0.67, emissive=0.37)
    clamp = p.mat("P_Modul_08_Schelle", "iron.png", diffuse=0.59, emissive=0.36, specular=0.5, power=20)
    wood = p.mat("P_Modul_08_Wood", "wood.png", diffuse=0.6, emissive=0.44)
    metal = p.mat("P_Modul_08_Metal", "steel.png", diffuse=[0.29, 0.33, 0.38, 1], emissive=0.3, specular=0.6,
                  power=30)
    rod = lambda m, mt, x0, x1: k.hexahedron(m, mt, [
        (x0, 1.664, -0.25), (x1, 1.664, -0.25), (x1, 1.664, 0.25), (x0, 1.664, 0.25),
        (x0, 11.604, -0.256), (x1, 11.604, -0.256), (x1, 11.604, 0.256), (x0, 11.604, 0.256)])
    side_section = [(-0.337, 0.0), (0.48, -3.35), (0.88, -3.35), (1.551, 0.0), (0.88, 3.35), (0.48, 3.35)]
    side = lambda m, mt, x0, x1: prism_yz(m, mt, side_section, x0, x1)

    def seat(m, mt, sgn):
        # half of the seat from the middle (z = ±0.025) out to the end (±3.35): its top rises from 0.34 to 0.88
        z_mid, z_end, z_lip = 0.025 * sgn, 3.35 * sgn, 3.0 * sgn
        prism_yz(m, mt, [(0.336, z_mid), (0.875, z_lip), (0.875, z_end), (0.48, z_end)] if sgn > 0 else
                 [(0.336, z_mid), (0.48, z_end), (0.875, z_end), (0.875, z_lip)], -1.783, 1.783)

    ms = k.Mesh("P_Modul_08_Schaukel_Mesh")
    rod(ms, metal, -2.981, -2.527)
    rod(ms, metal, 2.509, 2.962)
    side(ms, wood, -3.0, -2.483)
    side(ms, wood, 2.483, 3.0)
    seat(ms, wood, 1)
    seat(ms, wood, -1)
    # the rods' eyes round the hinge
    for x in (-2.75, 2.73):
        k.torus(ms, metal, (x, 12.15, 0), (1, 0, 0), 0.4, 0.12, seg=12, tube_seg=6)
    mf = k.Mesh("P_Modul_08_Fix_Mesh")
    k.box(mf, stone, (-4.183, -0.6, -1.5), (4.183, 0.6, 1.5))
    for zz in (-2.75, 2.75):
        # clamps where the rods hang, across the beam (the beam's frame is turned: its x is the swing's z)
        k.box(mf, clamp, (zz - 0.35, -0.771, -1.5), (zz + 0.35, 0.771, 1.5))
    p.node("P_Modul_08_Fix", p.mesh(mf), pos=(0, 10.951, 0.017), x=(0, 0, -1))
    p.node("P_Modul_08_Schaukel", p.mesh(ms), pos=(0, -0.775, 0), x=(0, 0, -1))
    p.node("P_Modul_08_HingeFrame", pos=(0, 11.379, 0), x=(-1, 0, 0))
    p.node("P_Modul_08_MF")
    p.mesh(col("P_Modul_08_Col6_Mesh", lambda m: rod(m, None, 2.509, 2.962)))
    p.mesh(col("P_Modul_08_Col5_Mesh", lambda m: side(m, None, 2.483, 3.0)))
    p.mesh(col("P_Modul_08_Col4_Mesh", lambda m: seat(m, None, 1)))
    p.mesh(col("P_Modul_08_Col3_Mesh", lambda m: seat(m, None, -1)))
    p.mesh(col("P_Modul_08_Col2_Mesh", lambda m: side(m, None, -3.0, -2.483)))
    p.mesh(col("P_Modul_08_Col1_Mesh", lambda m: rod(m, None, -2.981, -2.527)))
    p.group("All_P_Modul_08", ["P_Modul_08_MF", "P_Modul_08_HingeFrame", "P_Modul_08_Schaukel", "P_Modul_08_Fix"])
    p.write()


def make_modul_17():
    """The turning arms: a tapered arm on a hub, a paddle at each end (drawn by spinner_look: a painted, banded
    arm on a turned drum, framed paddles with a whole target on each face)."""
    p = Part("PH/P_Modul_17")
    target = p.mat("P_Modul_17_Target", "target.png", emissive=0.27)
    metal = p.mat("P_Modul_17_Metal", "steel.png", diffuse=0.31, emissive=0.22, specular=0.6, power=30)
    cube = p.mat("P_Modul_17_Cube", "iron.png", emissive=0.27)
    paddle = lambda m, mt, z0, z1: k.box(m, mt, (-0.155, -1.4, z0), (0.155, 1.4, z1))

    def arm(m, mt):
        # one convex body: the hub square at ±0.73, the tapered ends at ±7.74
        pts = []
        for z, w, h in ((-7.736, 0.36, 0.5), (-0.73, 1.068, 1.069), (0.73, 1.068, 1.069), (7.736, 0.36, 0.5)):
            pts.append([(-w / 2, -h / 2, z), (w / 2, -h / 2, z), (w / 2, h / 2, z), (-w / 2, h / 2, z)])
        for a, b in zip(pts, pts[1:]):
            k.hexahedron(m, mt, [a[0], a[1], b[1], b[0], a[3], a[2], b[2], b[3]])

    md = k.Mesh("P_Modul_17_Dreharme_Mesh")
    arm(md, metal)
    paddle(md, target, -11.621, -8.573)
    paddle(md, target, 8.573, 11.621)
    k.cylinder(md, cube, (-0.729, 0, 0), (1, 0, 0), 0.9, 1.458, seg=12)
    l_green = p.mat("P_Modul_17_Green", "paint_green.png", diffuse=0.85, emissive=0.3, specular=0.2, power=16)
    l_iron = p.mat("P_Modul_17_Iron", "iron.png", emissive=0.3, specular=0.4, power=20)
    l_hub = p.mat("P_Modul_17_Hub", "lift_rim.png", diffuse=0.85, emissive=0.3, specular=0.45, power=24)
    l_steel = p.mat("P_Modul_17_Steel", "steel.png", diffuse=0.8, emissive=0.3, specular=0.6, power=30)
    l_bolts = p.mat("P_Modul_17_Bolts", "brass.png", diffuse=0.85, emissive=0.32, specular=0.6, power=30)
    l_bullseye = p.mat("P_Modul_17_Bullseye", "bullseye.png", diffuse=0.9, emissive=0.3)
    md.look = spinner_look("P_Modul_17_Dreharme_Look", arm, l_iron, l_green, l_iron, l_hub, l_steel, l_bolts,
                           l_bullseye)
    p.node("P_Modul_17_HingeFrame", pos=(0, -3.749, 0), x=(0, 0, -1), y=(-1, 0, 0), s=2.0)
    p.node("Modul17_Dreharme", p.mesh(md), pos=(0, -3.0, 0), x=(0, 0, -1))
    p.node("P_Modul_17_MF")
    p.mesh(col("P_Modul_17_Col03_Mesh", lambda m: paddle(m, None, -11.621, -8.573)))
    p.mesh(col("P_Modul_17_Col02_Mesh", lambda m: arm(m, None)))
    p.mesh(col("P_Modul_17_Col01_Mesh", lambda m: paddle(m, None, 8.573, 11.621)))
    p.group("All_P_Modul_17", ["P_Modul_17_MF", "P_Modul_17_HingeFrame"])
    p.write()


def make_modul_18():
    """The fan: a four-bladed rotor (the frame round it is the level's), and the updraft's box. Drawn (fan_look)
    with a shroud ring and a wire guard, which turn with it."""
    p = Part("PH/P_Modul_18")
    rotor = p.mat("P_Modul_18_Rotor", "steel.png", diffuse=0.62, emissive=0.18, specular=0.5, power=20,
                  double_sided=True)
    m = k.Mesh("Modul_18_Rotor_Mesh")
    k.cylinder(m, rotor, (0, 0.076, 0), (0, 1, 0), 0.38, 0.968, seg=12)
    for b in range(4):
        t = math.pi / 2 * b
        d, s = np.array([math.cos(t), 0, math.sin(t)]), np.array([-math.sin(t), 0, math.cos(t)])
        # a blade twisted 30 degrees, 1.4 long from the hub, 0.95 wide
        pts = [d * 0.3 + s * 0.48 + np.array([0, 0.3, 0]), d * 1.767 + s * 0.48 + np.array([0, 0.25, 0]),
               d * 1.767 - s * 0.48 + np.array([0, 0.85, 0]), d * 0.3 - s * 0.48 + np.array([0, 0.95, 0])]
        m.face(rotor, pts)
    l_hub = p.mat("P_Modul_18_Hub", "lift_rim.png", diffuse=0.85, emissive=0.3, specular=0.45, power=24)
    l_blade = p.mat("P_Modul_18_Blade", "lift_paint.png", diffuse=0.85, emissive=0.3, specular=0.2, power=16, double_sided=True)
    l_tip = p.mat("P_Modul_18_Tip", "brass.png", diffuse=0.85, emissive=0.32, specular=0.6, power=30, double_sided=True)
    l_steel = p.mat("P_Modul_18_Steel", "steel.png", diffuse=0.8, emissive=0.3, specular=0.6, power=30)
    l_bolts = p.mat("P_Modul_18_Bolts", "brass.png", diffuse=0.85, emissive=0.32, specular=0.6, power=30)
    m.look = fan_look("Modul_18_Rotor_Look", l_hub, l_blade, l_tip, l_steel, l_bolts)
    p.node("P_Modul_18_Particle", y=(0, 0, -1))
    p.node("P_Modul_18_Rotor", p.mesh(m), pos=(0, -0.55, 0.002), x=(0.336, 0.007, 0.942), y=(0, 1, -0.004),
           s=(1, 0.5, 1))
    vol = p.mesh(col("Modul_18_Kollisionsquader_Mesh", lambda m: k.box(m, None, (-1, 0, -1), (1, 16, 1))))
    p.node("P_Modul_18_Kollisionsquader", vol, pos=(0, 1.14, -0.001), y=(0, 1, -0.004))
    p.node("P_Modul_18_MF")
    p.group("All_P_Modul_18", ["P_Modul_18_Particle", "P_Modul_18_Rotor", "P_Modul_18_Kollisionsquader",
                               "P_Modul_18_MF"])
    p.write()


def make_modul_19():
    """The flap wheel: two boards at right angles on a hexagonal hub and an axle (drawn by flaps_look and
    flap_axis_look: framed panels of oak boards, a spoked wheel, bearing collars and flanges)."""
    p = Part("PH/P_Modul_19")
    axis = p.mat("P_Modul_19_Axis", "steel.png", diffuse=[0.72, 0.76, 0.84, 1], emissive=0.43, specular=0.5,
                 power=20)
    flaps = p.mat("P_Modul_19_Flaps", "wood.png", diffuse=0.56, emissive=0.42)
    backs = p.mat("P_Modul_19_FlapsBackboards", "wood_dark.png", diffuse=0.77, emissive=0.44)
    flap1 = lambda m, mt: k.box(m, mt, (-0.308, -0.254, -10.0), (6.085, 0.246, -5.0))
    flap2 = lambda m, mt: k.box(m, mt, (-0.242, -0.308, 5.0), (0.258, 6.059, 10.0))
    axle = lambda m, mt: k.box(m, mt, (-0.2, -0.202, -4.064), (0.207, 0.206, 3.762))
    hub = lambda m, mt: prism_xy(m, mt, regular(6, 2.756, 0.0, 0.001, 0.0), -0.273, 0.294)
    mfl = k.Mesh("P_Modul_19_Flaps_Mesh")
    flap1(mfl, flaps)
    flap2(mfl, flaps)
    # battens across the boards
    for z in (-9.0, -6.0):
        k.box(mfl, backs, (0.3, 0.246, z - 0.2), (5.8, 0.4, z + 0.2))
    for z in (6.0, 9.0):
        k.box(mfl, backs, (-0.392, 0.3, z - 0.2), (-0.242, 5.8, z + 0.2))
    max_ = k.Mesh("P_Modul_19_Axis_Mesh")
    axle(max_, axis)
    hub(max_, axis)
    k.cylinder(max_, axis, (0, 0, -10.0), (0, 0, 1), 0.3, 20.0, seg=10)
    for z in (-10.0, 9.6):
        k.cylinder(max_, axis, (0, 0, z), (0, 0, 1), 1.27, 0.4, seg=16)
    l_paint = p.mat("P_Modul_19_Paint", "lift_paint.png", diffuse=0.85, emissive=0.3, specular=0.2, power=16)
    l_boards = p.mat("P_Modul_19_Boards", "lift_deck.png", diffuse=0.85, emissive=0.34, specular=0.1, power=10)
    l_iron = p.mat("P_Modul_19_Iron", "iron.png", emissive=0.3, specular=0.4, power=20)
    l_hub = p.mat("P_Modul_19_Hub", "lift_rim.png", diffuse=0.85, emissive=0.3, specular=0.45, power=24)
    l_steel = p.mat("P_Modul_19_Steel", "steel.png", diffuse=0.8, emissive=0.3, specular=0.6, power=30)
    l_bolts = p.mat("P_Modul_19_Bolts", "brass.png", diffuse=0.85, emissive=0.32, specular=0.6, power=30)
    mfl.look = flaps_look("P_Modul_19_Flaps_Look", l_paint, l_boards, l_bolts)
    max_.look = flap_axis_look("P_Modul_19_Axis_Look", l_steel, l_iron, l_hub, l_paint, l_bolts)
    rot = dict(x=(0.996, 0.087, 0), y=(-0.087, 0.996, 0))
    p.node("P_Modul_19_Axis", p.mesh(max_), pos=(-2.929, -0.246, -2.5), **rot)
    p.node("P_Modul_19_Flaps", p.mesh(mfl), pos=(-2.929, -0.246, -2.5), **rot)
    p.node("P_Modul_19_HingeFrame", pos=(-2.926, -0.246, -2.499))
    p.node("P_Modul_19_MF")
    p.mesh(col("P_Modul_19_Col4_Mesh", lambda m: hub(m, None)))
    p.mesh(col("P_Modul_19_Col3_Mesh", lambda m: axle(m, None)))
    p.mesh(col("P_Modul_19_Col2_Mesh", lambda m: flap2(m, None)))
    p.mesh(col("P_Modul_19_Col1_Mesh", lambda m: flap1(m, None)))
    p.group("All_P_Modul_19", ["P_Modul_19_MF", "P_Modul_19_HingeFrame", "P_Modul_19_Flaps", "P_Modul_19_Axis"])
    p.write()


def make_modul_25():
    """The flap bridge: a board on a hinge with a small stop at its root."""
    p = Part("PH/P_Modul_25")
    wood = p.mat("P_Modul_25_Wood", "wood.png", diffuse=0.77, emissive=0.37)
    hinge = p.mat("P_Modul_25_Hinge", "iron.png", diffuse=0.73, emissive=0.29, specular=0.5, power=20)
    side = p.mat("P_Modul_25_HingeSide", "stone.png", diffuse=0.73, emissive=0.34)
    board = lambda m, mt: k.box(m, mt, (-9.214, -0.279, -2.523), (0.359, 0.221, 2.477))
    stop = lambda m, mt: prism_xy(m, mt, [(-1.33, 0.221), (-0.506, 0.221), (-0.506, 0.836)], 1.973, 2.477)
    mb = k.Mesh("P_Modul_25_Bridge_Mesh")
    board(mb, wood)
    for x in (-8.0, -4.6, -1.2):
        k.box(mb, hinge, (x - 0.15, -0.3, -2.5), (x + 0.15, 0.23, 2.45))
    mh = k.Mesh("P_Modul_25_Hinge_Mesh")
    k.cylinder(mh, hinge, (0, 0, -4.99), (0, 0, 1), 0.35, 5.7, seg=12)
    k.box(mh, side, (-0.608, -0.989, -0.6), (1.274, 0.595, 0.747))
    p.node("P_Modul_25_HingeFrame", pos=(-0.669, -0.234, 0))
    p.node("P_Modul_25_Bridge", p.mesh(mb), pos=(-0.656, -0.245, 0.023), x=(0, -1, 0), y=(1, 0, 0))
    p.node("P_Modul_25_Hinge", p.mesh(mh), pos=(-0.669, -0.227, 2.118))
    p.node("P_Modul_25_MF")
    p.mesh(col("P_Modul_25_Col01_Mesh", lambda m: board(m, None)))
    p.mesh(col("P_Modul_25_Col02_Mesh", lambda m: stop(m, None)))
    p.group("All_P_Modul_25", ["P_Modul_25_HingeFrame", "P_Modul_25_MF", "P_Modul_25_Bridge", "P_Modul_25_Hinge"])
    p.write()


def sack_points():
    """The sack as the original's is built (measured: its rings' heights and widths, its seams, its tie): an
    octagonal body, oval across (1.39 wide in x, 1.82 in z), slung low; two seam corners below reaching 1.95; a
    shoulder and a tied neck; and the tie's frayed end flapping over to one side (+z), 1.2 wide, from 2.16 up to 3.09.
    The body (it is a body of its own hull) is the hull of these points."""
    pts = [(0.0, -2.975, -0.09)]
    rings = [(-2.03, 1.1, 1.6, 6), (-1.03, 1.394, 1.82, 8), (-0.03, 1.394, 1.82, 8), (0.97, 1.18, 1.59, 8),
             (1.58, 0.75, 1.0, 8), (1.92, 0.42, 0.44, 8)]
    for y, rx, rz, n in rings:
        for i in range(n):
            a = 2 * math.pi * i / n
            pts.append((rx * math.cos(a), y, rz * math.sin(a) - (0.09 if abs(math.cos(a)) > 0.9 else 0.0)))
    pts += [(0.0, -2.86, -1.947), (0.0, -2.86, 1.947)]
    # the tie's frayed ends, each (height, reach towards +z, offset across), to 0.05
    for y, z, x in ((2.15, 1.6, 0.1), (2.2, 1.5, -0.35), (2.5, 1.25, 0.5), (2.65, 1.0, -0.6), (3.0, 0.9, 0.1),
                    (3.092, 0.8, -0.35)):
        pts.append((x, y, z))
    return pts


def make_modul_26():
    """The sandbag on a rope from a holder; the sack and the rope are bodies of their own meshes (the hull of
    sack_points and a plain rod), which stay as they are. Drawn (sack_look, sack_rope_look, sack_holder_look) as a
    sewn canvas sack tied at the neck, a twisted rope with an iron eye, and a bolted plate with a ring."""
    p = Part("PH/P_Modul_26")
    sack = p.mat("P_Modul_26_Sandsack", "burlap.png", diffuse=0.7, emissive=0.46)
    holder = p.mat("P_Modul_26_Halter", "iron.png", diffuse=0.62, emissive=0.4, specular=0.5, power=20)
    rope = p.mat("P_Modul_26_Seil", "rope.png", diffuse=0.73, emissive=0.41)
    ms = k.Mesh("P_Modul_26_Sack_Mesh")
    k.hull(ms, sack, sack_points())
    mh = k.Mesh("P_Modul_26_Halter_Mesh")
    k.box(mh, holder, (-0.733, 0.0, -1.427), (0.733, 0.255, 1.426))
    mr = k.Mesh("P_Modul_26_Rope_Mesh")
    k.prism(mr, rope, (0, -6.25, 0), (0, 1, 0), 0.15, 12.5, 10)
    l_cloth = p.mat("P_Modul_26_Cloth", "sack_cloth.png", diffuse=0.8, emissive=0.42)
    l_cord = p.mat("P_Modul_26_Cord", "rope.png", diffuse=0.73, emissive=0.41)
    l_iron = p.mat("P_Modul_26_Iron", "iron.png", emissive=0.3, specular=0.4, power=20)
    l_bolts = p.mat("P_Modul_26_Bolts", "brass.png", diffuse=0.85, emissive=0.32, specular=0.6, power=30)
    ms.look = sack_look("P_Modul_26_Sack_Look", l_cloth, l_cord)
    mh.look = sack_holder_look("P_Modul_26_Halter_Look", holder, l_iron, l_bolts)
    mr.look = sack_rope_look("P_Modul_26_Rope_Look", rope, l_iron, l_bolts)
    p.node("P_Modul_26_Sack", p.mesh(ms), pos=(0, 0.517, 0))
    p.node("P_Modul_26_Halter", p.mesh(mh), pos=(-0.001, 14.964, 0))
    p.node("P_Modul_26_Rope", p.mesh(mr), pos=(0, 8.714, 0))
    p.node("P_Modul_26_Balljoint_oben", pos=(0, 14.964, 0))
    p.node("P_Modul_26_Balljoint_unten", pos=(0, 2.464, 0))
    p.node("P_Modul_26_MF")
    p.group("All_P_Modul_26", ["P_Modul_26_MF", "P_Modul_26_Balljoint_unten", "P_Modul_26_Balljoint_oben",
                               "P_Modul_26_Rope", "P_Modul_26_Sack", "P_Modul_26_Halter"])
    sack_supports(p)
    p.write()


# How far down a support's post reaches: as the levels' columns do, out of sight in the haze (levelkit.COLUMN_DEPTH).
SUPPORT_DEPTH = 240.0


def sack_supports(p):
    """What the sack's holder hangs on (the original's hangs in the air): pieces the game puts together for each
    sack where nothing of them comes near the ball's way (crates/game/src/supports.rs, the `sack-supports` mod).
    Drawn only, never in the `All_` group. Every piece is in the holder's frame, +x away from the sack. Of each of
    the three materials (timber, iron, rail tube) there is a beam, one unit long from the holder and drawn stretched
    to its length, and a mount that holds the holder's plate to it; the posts stand at the beam's far end, each with
    the joint that takes its beam (a band round a column or a mast, a bolted shoe on a timber post, a bend of the
    tube itself). Every member is level or upright: nothing leans but the rail tube's quarter bend."""
    g = k.GAME
    wood = p.mat("P_Modul_26_Balken", "wood_dark.png", diffuse=0.75, emissive=0.38)
    iron = p.mat("P_Modul_26_Halter", "iron.png", diffuse=0.62, emissive=0.4, specular=0.5, power=20)
    beige = p.mat("P_Modul_26_Saeule", g + "Column_beige", diffuse=1.0, emissive=0.32)
    brick = p.mat("P_Modul_26_Sims", g + "Brick", diffuse=1.0, emissive=0.3)
    stripes = p.mat("P_Modul_26_Mast", g + "Stick_Stripes", diffuse=1.0, emissive=0.35, specular=0.2, power=40)
    # the track's own rail tube (levelkit's "rail")
    rail = p.mat("P_Modul_26_Rohr", g + "Track_Rail", diffuse=1.0, emissive=0.22, specular=0.85, power=40,
                 normal=g + "Track_Rail_n")
    d = SUPPORT_DEPTH
    X, Y = (1, 0, 0), (0, 1, 0)
    # the heights of the beams' middles over the holder's plate (its top is at 0.255), and the rail tube's radius
    # (levelkit.RAIL_TUBE) and bend
    arm, tube_y, tube_r, bend = 0.5, 0.6, 0.3, 0.9

    def bolts(m, pts, axis, r=0.07, out=0.06):
        for q in pts:
            k.cylinder(m, iron, q, axis, r, out, seg=6)

    # the beams: a timber, an iron pipe, a rail tube
    beam_wood = k.Mesh("P_Modul_26_Support_Beam_Wood_Mesh")
    k.box(beam_wood, wood, (0.0, 0.27, -0.2), (1.0, 0.73, 0.2))
    beam_iron = k.Mesh("P_Modul_26_Support_Beam_Iron_Mesh")
    k.cylinder(beam_iron, iron, (0.0, arm, 0.0), X, 0.16, 1.0, seg=10, caps=False)
    beam_rail = k.Mesh("P_Modul_26_Support_Beam_Rail_Mesh")
    k.cylinder(beam_rail, rail, (0.0, tube_y, 0.0), X, tube_r, 1.0, seg=12, caps=False)
    # the mounts. Timber: a head block over the plate, two iron straps round both, bolted through
    mount_wood = k.Mesh("P_Modul_26_Support_Mount_Wood_Mesh")
    k.box(mount_wood, wood, (-0.85, 0.255, -1.55), (0.85, 0.82, 1.55))
    for z in (-1.0, 1.0):
        k.box(mount_wood, iron, (-0.9, -0.05, z - 0.13), (0.9, 0.87, z + 0.13))
        bolts(mount_wood, [(-0.5, 0.87, z), (0.5, 0.87, z)], Y)
    # iron: a flange bolted on the plate and a hub the pipe is screwed into from any side
    mount_iron = k.Mesh("P_Modul_26_Support_Mount_Iron_Mesh")
    k.cylinder(mount_iron, iron, (0.0, 0.255, 0.0), Y, 0.66, 0.09, seg=12)
    k.cylinder(mount_iron, iron, (0.0, 0.34, 0.0), Y, 0.3, 0.46, seg=10)
    k.sphere(mount_iron, iron, (0.0, 0.8, 0.0), 0.3, seg=10)
    bolts(mount_iron, [(0.5 * math.cos(a), 0.345, 0.5 * math.sin(a)) for a in (0.8, 2.4, 3.9, 5.5)], Y)
    # rail tube: it runs on over the plate in two clamps on a foot plate, as the track's rails in theirs
    mount_rail = k.Mesh("P_Modul_26_Support_Mount_Rail_Mesh")
    k.cylinder(mount_rail, rail, (-0.62, tube_y, 0.0), X, tube_r, 0.62, seg=12)
    k.box(mount_rail, iron, (-0.66, 0.255, -0.55), (0.66, 0.32, 0.55))
    for x in (-0.4, 0.26):
        k.cylinder(mount_rail, iron, (x, tube_y, 0.0), X, tube_r + 0.07, 0.14, seg=12)
        k.box(mount_rail, iron, (x, 0.32, -0.45), (x + 0.14, 0.44, 0.45))
        bolts(mount_rail, [(x + 0.07, 0.44, -0.36), (x + 0.07, 0.44, 0.36)], Y, r=0.05)
    # a stone column as the track's own, a brick cornice and a flat cap; an iron band round it takes the pipe
    col = k.Mesh("P_Modul_26_Support_Column_Mesh")
    k.box(col, beige, (-0.1, -d, -1.0), (1.9, 2.1, 1.0))
    k.box(col, brick, (-0.3, 1.3, -1.2), (2.1, 2.1, 1.2))
    k.box(col, beige, (-0.2, 2.1, -1.1), (2.0, 2.4, 1.1))
    k.box(col, iron, (-0.17, arm - 0.3, -1.07), (1.97, arm + 0.3, 1.07))
    k.cylinder(col, iron, (-0.5, arm, 0.0), X, 0.24, 0.34, seg=10)
    bolts(col, [(-0.17, arm, z) for z in (-0.75, 0.75)], (-1, 0, 0))
    bolts(col, [(x, arm, s * 1.07) for x in (0.3, 1.5) for s in (-1, 1)], (0, 0, 1))
    # a striped mast with a ball on its top; a collar round it takes the pipe
    mast = k.Mesh("P_Modul_26_Support_Mast_Mesh")
    k.cylinder(mast, stripes, (0.45, -d, 0.0), Y, 0.42, d + 2.0, seg=10, vscale=5.0)
    k.cylinder(mast, iron, (0.45, arm - 0.32, 0.0), Y, 0.55, 0.64, seg=10)
    k.cylinder(mast, iron, (-0.38, arm, 0.0), X, 0.24, 0.4, seg=10)
    k.cylinder(mast, iron, (0.45, 1.8, 0.0), Y, 0.52, 0.3, seg=10)
    k.sphere(mast, iron, (0.45, 2.5, 0.0), 0.5, seg=10)
    # a timber post (two of them make a gate): the beam sits in an iron shoe bolted to its head,
    # under an iron cap
    post = k.Mesh("P_Modul_26_Support_Post_Mesh")
    k.box(post, wood, (0.0, -d, -0.34), (0.68, 1.15, 0.34))
    k.box(post, iron, (-0.06, 1.15, -0.4), (0.74, 1.27, 0.4))
    k.box(post, iron, (-0.06, 0.05, -0.4), (0.0, 0.95, 0.4))
    k.box(post, iron, (-0.55, 0.2, -0.26), (-0.06, 0.8, 0.26))
    bolts(post, [(-0.06, y, z) for y in (0.17, 0.83) for z in (-0.32, 0.32)], (-1, 0, 0))
    bolts(post, [(-0.3, 0.5, s * 0.26) for s in (-1, 1)], (0, 0, 1), out=0.05)
    # a tube of the track's rails coming up beside the track and bending over into the beam, a sleeve at the joint
    rail_post = k.Mesh("P_Modul_26_Support_Rail_Mesh")
    k.cylinder(rail_post, rail, (bend, -d, 0.0), Y, tube_r, d + tube_y - bend, seg=12, caps=False)
    arc = [(bend * math.sin(a), tube_y - bend + bend * math.cos(a), 0.0)
           for a in (math.pi / 2 * i / 8 for i in range(9))]
    k.tube(rail_post, rail, arc, tube_r, seg=12)
    k.cylinder(rail_post, iron, (-0.12, tube_y, 0.0), X, tube_r + 0.07, 0.24, seg=12)
    k.cylinder(rail_post, iron, (bend, tube_y - bend - 0.24, 0.0), Y, tube_r + 0.07, 0.24, seg=12)
    # an iron plate bolted on a wall or a pillar of the level, a socket on it for the pipe
    plate = k.Mesh("P_Modul_26_Support_Wall_Mesh")
    k.box(plate, iron, (-0.12, arm - 0.75, -0.6), (0.0, arm + 0.75, 0.6))
    k.cylinder(plate, iron, (-0.5, arm, 0.0), X, 0.24, 0.38, seg=10)
    bolts(plate, [(-0.12, arm + y, z) for y in (-0.55, 0.55) for z in (-0.42, 0.42)], (-1, 0, 0))
    # each post's plain shaft, which the game puts under it as often as a level is deep
    col_shaft = k.Mesh("P_Modul_26_Support_Column_Shaft_Mesh")
    k.box(col_shaft, beige, (-0.1, -d, -1.0), (1.9, 0.0, 1.0))
    mast_shaft = k.Mesh("P_Modul_26_Support_Mast_Shaft_Mesh")
    k.cylinder(mast_shaft, stripes, (0.45, -d, 0.0), Y, 0.42, d, seg=10, vscale=5.0)
    post_shaft = k.Mesh("P_Modul_26_Support_Post_Shaft_Mesh")
    k.box(post_shaft, wood, (0.0, -d, -0.34), (0.68, 0.0, 0.34))
    rail_shaft = k.Mesh("P_Modul_26_Support_Rail_Shaft_Mesh")
    k.cylinder(rail_shaft, rail, (bend, -d, 0.0), Y, tube_r, d, seg=12, caps=False)
    names = []
    for name, mesh in (("Beam_Wood", beam_wood), ("Beam_Iron", beam_iron), ("Beam_Rail", beam_rail),
                       ("Mount_Wood", mount_wood), ("Mount_Iron", mount_iron), ("Mount_Rail", mount_rail),
                       ("Column", col), ("Mast", mast), ("Post", post), ("Rail", rail_post), ("Wall", plate),
                       ("Column_Shaft", col_shaft), ("Mast_Shaft", mast_shaft), ("Post_Shaft", post_shaft),
                       ("Rail_Shaft", rail_shaft)):
        names.append("P_Modul_26_Support_" + name)
        p.node(names[-1], p.mesh(mesh), pos=(0, 14.964, 0))
    p.group("Supports_P_Modul_26", names)


# The planks of the plate bridges (P_Modul_29 and the balloon's), by measured dimensions, rounded to 0.05: each a
# board of a width (x), thickness (y, from its bottom at 0) and length (z, from z0 to z1), with the wear of its ends
# as our own few kinds of cut, each given by one length:
#   ("lean", end, d)    the end's top edge stands d further out (+) or back (-) than its bottom edge;
#   ("skew", end, d)    the end is cut on the slant across the board: its x1 corner d further out than its x0 one;
#   ("corner", end, side, d)  the top corner of that end and side is cut back by d across the board.
# `end` is 0 (the z0 end) or 1 (the z1 end), `side` 0 (x0) or 1 (x1). The ball rolls over their gaps, so the ends
# matter to the physics; the boards' bodies are the hulls of these.
PLANKS = {
    1: ((1.5, 0.6, -2.5, 2.5), ()),
    2: ((1.45, 0.6, -2.4, 2.5), (("skew", 0, 0.15),)),
    3: ((1.5, 0.6, -2.4, 2.3), ()),
    4: ((1.5, 0.6, -2.5, 2.5), (("lean", 1, -0.2),)),
    5: ((1.5, 0.6, -2.4, 2.5), (("corner", 0, 0, 0.2),)),
    6: ((1.5, 0.6, -2.6, 2.4), (("skew", 1, 0.1),)),
    7: ((1.4, 0.6, -2.5, 2.5), (("lean", 0, -0.2),)),
    8: ((1.4, 0.55, -2.4, 2.4), ()),
    9: ((1.5, 0.6, -2.5, 2.5), (("corner", 1, 0, 0.25),)),
}


def plank(m, mt, i):
    """Plank i of PLANKS: its board, its ends cut as listed."""
    (w, t, z0, z1), cuts = PLANKS[i]
    x0, x1 = -0.75, -0.75 + w
    # corners: bottom x0z0, x1z0, x1z1, x0z1, then the top the same
    c = [np.array(q, np.float64) for q in ((x0, 0, z0), (x1, 0, z0), (x1, 0, z1), (x0, 0, z1),
                                           (x0, t, z0), (x1, t, z0), (x1, t, z1), (x0, t, z1))]
    out = {0: -1.0, 1: 1.0}
    ends = {0: (0, 1, 4, 5), 1: (3, 2, 7, 6)}  # (x0 bottom, x1 bottom, x0 top, x1 top) of each end
    for cut in cuts:
        kind, end = cut[0], cut[1]
        b0, b1, t0, t1 = ends[end]
        if kind == "lean":
            for j in (t0, t1):
                c[j][2] += out[end] * cut[2]
        elif kind == "skew":
            for j in (b1, t1):
                c[j][2] += out[end] * cut[2] / 2
            for j in (b0, t0):
                c[j][2] -= out[end] * cut[2] / 2
        elif kind == "corner":
            j = t0 if cut[2] == 0 else t1
            c[j][0] += cut[3] if cut[2] == 0 else -cut[3]
    k.hull(m, mt, c)


def make_modul_29():
    """The plate bridge: nine planks hinged in a row."""
    p = Part("PH/P_Modul_29")
    wood = p.mat("P_Modul_29", "wood_dark.png", emissive=0.25)
    for i in (7, 8, 9):
        m = k.Mesh(f"P_Modul_29_Platte0{i}_Mesh")
        plank(m, wood, i)
        p.node(f"P_Modul_29_Platte0{i}", p.mesh(m), pos=(-5.736 + 1.639 * (i - 1), -0.6, 0.211))
    for i, x in enumerate((-6.584, -4.916, -3.291, -1.639, 0.033, 1.639, 3.327, 4.936, 6.592, 8.224), 1):
        p.node(f"P_Modul_29_HingeFrame{i:02d}", pos=(x, 0, 0))
    for i in range(1, 7):
        m = k.Mesh(f"P_Modul_29_Platte0{i}_Mesh")
        plank(m, wood, i)
        p.node(f"P_Modul_29_Platte0{i}", p.mesh(m), pos=(-5.736 + 1.639 * (i - 1), -0.6, 0.211))
    p.node("P_Modul_29_MF")
    p.group("All_P_Modul_29", ["P_Modul_29_MF"] + [f"P_Modul_29_HingeFrame{i:02d}" for i in range(10, 0, -1)] +
            [f"P_Modul_29_Platte0{i}" for i in range(9, 0, -1)])
    p.write()


def make_modul_30():
    """The seesaw: a board on a square axle turned on its edge."""
    p = Part("PH/P_Modul_30")
    board = p.mat("P_Modul_30_Board", "wood.png", diffuse=0.54, emissive=0.43)
    axis = p.mat("P_Modul_30_Axis", "iron.png", diffuse=[0.16, 0.19, 0.22, 1], emissive=0.27, specular=0.5,
                 power=20)
    plate = lambda m, mt: k.box(m, mt, (-7.402, -2.095, -2.604), (7.398, -1.6, 2.396))
    axle = lambda m, mt: prism_xy(m, mt, [(0.364, -2.022), (0.0, -1.658), (-0.364, -2.022), (0.0, -2.386)],
                                  -4.9, 4.814)
    mw = k.Mesh("P_Modul_30_Wippe_Mesh")
    plate(mw, board)
    axle(mw, axis)
    for x in (-6.8, 6.8):
        k.box(mw, axis, (x - 0.3, -3.1, -2.6), (x + 0.3, -2.095, 2.4))
    p.node("P_Modul_30_HingeFrame", pos=(0, 2.562, 0))
    p.node("P_Modul_30_Wippe", p.mesh(mw), pos=(-0.351, 4.556, 0.15), x=(0.985, 0.174, 0), y=(-0.174, 0.985, 0))
    p.node("P_Modul_30_MF")
    p.mesh(col("P_Modul_30_Col2_Mesh", lambda m: axle(m, None)))
    p.mesh(col("P_Modul_30_Col1_Mesh", lambda m: plate(m, None)))
    p.group("All_P_Modul_30", ["P_Modul_30_MF", "P_Modul_30_HingeFrame", "P_Modul_30_Wippe"])
    p.write()


def make_modul_34():
    """The stone-ball puzzle's block and crate: a stone cube that slides down onto a crate; both are bodies of
    their own meshes."""
    p = Part("PH/P_Modul_34")
    # the stone drops level with the way and becomes part of it: the track's own concrete on top, its side below
    side = p.mat("P_Modul_34_Floor_Side", "track_side.png", diffuse=0.6, emissive=0.42)
    top = p.mat("P_Modul34_Floor_Top_Flat", "track_top.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
    box = p.mat("P_Modul_34_Box", "crate.png", diffuse=0.88, emissive=0.37)
    p.node("P_Modul_34_Slider_Frame01", pos=(-7.7, 5.0, 0), x=(-1, 0, 0))
    p.node("P_Modul_34_Slider_Frame02", pos=(-7.7, -10.0, 0), x=(-1, 0, 0))
    ms = k.Mesh("P_Modul_34_Schiebestein_Mesh")
    lo, hi = (-2.5, -2.5, -2.5), (2.5, 2.5, 2.5)
    tmp = k.Mesh("t")
    k.box(tmp, side, lo, hi, scale=5.0)
    # the top face in the floor's top texture, the others in its side
    for mt, idx in tmp.parts.items():
        for t in range(0, len(idx), 3):
            ids = idx[t:t + 3]
            up = all(abs(tmp.p[i][1] - 2.5) < 1e-9 for i in ids)
            new = [ms.vertex(tmp.p[i], tmp.n[i], tmp.uv[i]) for i in ids]
            ms.tri(top if up else side, *new)
    p.node("P_Modul_34_Schiebestein", p.mesh(ms), pos=(-7.7, 2.296, 0), x=(-1, 0, 0))
    mk = k.Mesh("P_Modul_34_Kiste_Mesh")
    k.box(mk, box, (-2.05, -2.05, -2.5), (2.05, 2.05, 2.5), scale=4.1)
    for i, (u, v) in enumerate(mk.uv):
        mk.uv[i] = (u + 0.5, v + 0.5)
    p.node("P_Modul_34_Kiste", p.mesh(mk), pos=(-7.7, -2.48, 0), x=(-1, 0, 0))
    p.node("P_Modul_34_MF")
    p.group("All_P_Modul_34", ["P_Modul_34_MF", "P_Modul_34_Slider_Frame02", "P_Modul_34_Slider_Frame01"])
    p.write()


def make_modul_37():
    """The drawbridge: a deck on a long beam with a counterweight, turning on a hinge."""
    p = Part("PH/P_Modul_37")
    misc = p.mat("P_Modul_37_WoodMisc", "wood_dark.png", diffuse=0.74, emissive=0.31)
    axis = p.mat("P_Modul_37_WoodAxis", "wood.png", diffuse=0.77, emissive=0.32)
    deckm = p.mat("P_Modul_37_Wood01", "wood.png", diffuse=0.76, emissive=0.29)
    target = p.mat("P_Modul_37_Target", "target.png", diffuse=0.76, emissive=0.35)
    metal = p.mat("P_Modul_37_Metall_1", "steel.png", diffuse=0.68, emissive=0.3, specular=0.5, power=20)
    buttons = p.mat("P_Modul37_Misc_MetalButtons", "brass.png", diffuse=0.89, emissive=0.21)
    deck = lambda m, mt: k.box(m, mt, (-13.856, 0.125, -4.513), (-8.926, 0.623, 4.514))
    beam = lambda m, mt: k.box(m, mt, (-8.522, -0.329, -0.511), (-3.818, 0.125, 0.518))
    weight = lambda m, mt: prism_xy(m, mt, [(-3.39, 0.125), (-1.053, 0.125), (-1.053, 2.09), (-1.68, 3.597),
                                            (-4.656, 3.597)], -1.388, 1.442)
    mb = k.Mesh("P_Modul_37_Bridge_Mesh")
    deck(mb, deckm)
    beam(mb, axis)
    weight(mb, target)
    k.cylinder(mb, metal, (0, 0, -4.5), (0, 0, 1), 0.5, 9.0, seg=12)
    k.box(mb, misc, (-3.818, -0.98, -0.6), (0.693, 0.125, 0.6))
    for z in (-3.5, 0.0, 3.5):
        k.box(mb, buttons, (-13.6, 0.623, z - 0.2), (-9.2, 0.7, z + 0.2))
    mh = k.Mesh("P_Modul_37_Hinge_Mesh")
    k.box(mh, misc, (-1.104, -3.386, -1.767), (2.384, 0.61, 1.244))
    k.cylinder(mh, buttons, (0.64, 0.0, -1.9), (0, 0, 1), 0.4, 3.3, seg=10)
    p.node("P_Modul_37_Bridge", p.mesh(mb), pos=(-2.381, -0.625, 0), x=(-0.087, -0.996, 0), y=(0.996, -0.087, 0))
    p.node("P_Modul_37_Hinge", p.mesh(mh), pos=(-2.384, -0.712, -1.326), y=(0, 0, -1))
    p.node("P_Modul_37_HingeFrame", pos=(-2.381, -0.625, 0))
    p.node("P_Modul_37_MF")
    p.mesh(col("P_Modul_37_Col3_Mesh", lambda m: weight(m, None)))
    p.mesh(col("P_Modul_37_Col2_Mesh", lambda m: beam(m, None)))
    p.mesh(col("P_Modul_37_Col1_Mesh", lambda m: deck(m, None)))
    p.group("All_P_Modul_37", ["P_Modul_37_MF", "P_Modul_37_HingeFrame", "P_Modul_37_Hinge", "P_Modul_37_Bridge"])
    p.write()


def make_modul_41():
    """The tipping trough: a box wider at its top on a pentagonal axle."""
    p = Part("PH/P_Modul_41")
    metal = p.mat("P_Modul_41_BoxMetal", "iron.png", diffuse=0.69, emissive=0.39, specular=0.4, power=20)
    wood = p.mat("P_Modul_41_BoxWood", "wood.png", diffuse=0.91, emissive=0.21)
    axis = p.mat("P_Modul_41_Axis", "steel.png", diffuse=[0.38, 0.44, 0.49, 1], emissive=0.22)
    trough = lambda m, mt: k.hexahedron(m, mt, [
        (-1.96, -0.359, -2.515), (2.01, -0.359, -2.515), (2.01, -0.359, 2.485), (-1.96, -0.359, 2.485),
        (-2.477, 1.351, -2.515), (2.525, 1.351, -2.515), (2.525, 1.351, 2.485), (-2.477, 1.351, 2.485)])
    axle = lambda m, mt: prism_xy(m, mt, [(-0.3, 0.178), (0.28, 0.178), (0.426, 0.78), (-0.01, 1.01),
                                          (-0.446, 0.78)], -5.029, 5.121)
    m = k.Mesh("P_Modul_41_Mesh")
    trough(m, wood)
    axle(m, axis)
    for x in (-2.2, 2.25):
        k.box(m, metal, (x - 0.1, 0.9, -2.55), (x + 0.1, 1.38, 2.52))
    p.node("P_Modul_41", p.mesh(m), pos=(0, -1.351, 0))
    p.node("P_Modul_41_HingeFrame", pos=(0, -0.758, 0))
    p.node("P_Modul_41_MF")
    p.mesh(col("P_Modul_41_Col2_Mesh", lambda m: axle(m, None)))
    p.mesh(col("P_Modul_41_Col1_Mesh", lambda m: trough(m, None)))
    p.group("All_P_Modul_41", ["P_Modul_41_MF", "P_Modul_41_HingeFrame", "P_Modul_41"])
    p.write()


# A balloon: twelve gores round, 3.48 across and 4.35 high, its knot at the bottom.
BALLOON = [(-2.174, 0.0), (-2.174, 0.15), (-0.65, 1.51), (0.43, 1.74), (1.3, 1.51), (1.94, 0.87), (2.174, 0.0)]


# A balloon's ropes, measured as a whole: from the knot under the balloon five ropes fan down in one upright plane
# towards the platform's rim, ending at these fractions of the way (along, across the rim's way) and heights below
# the rim's hinge; ROPE_SPREAD sideways and ROPE_R thick, so that the hull (the ropes are one body) has the
# original's volume and inertia.
ROPE_ENDS = [(0.73, -0.51), (1.0, -0.61), (0.39, -0.36), (0.32, 0.15), (0.17, -0.14)]
ROPE_SPREAD, ROPE_R = 0.05, 0.155


def balloon_ropes(m, mat, top, rim):
    flat = np.array([rim[0] - top[0], 0.0, rim[2] - top[2]])
    side = np.cross(np.array([0.0, 1.0, 0.0]), flat / np.linalg.norm(flat))
    for j, (f, dy) in enumerate(ROPE_ENDS):
        end = top + flat * f + np.array([0.0, rim[1] - top[1] + dy, 0.0]) + side * ROPE_SPREAD * (1 if j % 2 else -1)
        k.tube(m, mat, [top, end], ROPE_R, seg=6)


def rope_look(name, mat, knot, post, frame):
    """A balloon's ropes as drawn: from the knot under it to the top of its post and to the basket's ring (radius 8,
    1.9 up) a little either side of the post, each rope sagging a little; in the rope object's frame (`frame`)."""
    m = k.Mesh(name)
    a = math.atan2(post[2], post[0])
    ends = [np.array([post[0], 2.15, post[2]])]
    for d in (-0.12, 0.12):
        ends.append(np.array([8.0 * math.cos(a + d), 1.9, 8.0 * math.sin(a + d)]))
    for e in ends:
        pts = []
        for t in np.linspace(0.0, 1.0, 9):
            q = knot + (e - knot) * t
            q[1] -= 0.35 * math.sin(math.pi * t)
            pts.append(q - frame)
        k.tube(m, mat, pts, 0.07, seg=6, caps=True)
    return m


def make_balloon():
    """The finish: a platform hung under four balloons, reached by a plank bridge; when the ball is in, the
    bridge's hinges break and the balloons carry it off (the sim's PE_Balloon entry)."""
    p = Part("PH/PE_Balloon")
    plat = p.mat("PE_Balloon_Platform", "stone_light.png", emissive=0.2)
    holder = p.mat("PE_Balloonl_Halter", "wood_dark.png")
    metal = p.mat("PE_Balloon_Metall", "steel.png", specular=0.5, power=20)
    bridge = p.mat("PE_Balloon_SuspBridge", "wood_dark.png", emissive=0.15)
    gores = p.mat("PE_Balloon_Ballon", "balloon.png", emissive=0.39, specular=0.4, power=30)
    rope = p.mat("PE_Balloon_Rope", "rope.png", emissive=0.3)
    rail1 = lambda m, mt: prism_xz(m, mt, [(-1.939, 2.494), (4.06, 2.56), (7.362, 4.329), (-1.939, 4.26)],
                                   -0.621, 1.879)
    rail3 = lambda m, mt: prism_xz(m, mt, [(-1.939, -2.5), (3.84, -2.44), (7.299, -4.31), (-1.92, -4.376)],
                                   -0.621, 1.879)
    back = lambda m, mt: prism_xz(m, mt, [(-4.38, 3.68), (-4.32, -3.657), (-2.164, -3.64), (-2.23, 3.698)],
                                  -0.621, 1.879)
    floor = lambda m, mt: prism_xz(m, mt, [(-1.618, -1.18), (0.62, -1.902), (2.0, 0.0), (0.62, 1.902), (-1.618, 1.18)],
                                   -2.341, -1.841)
    entry = lambda m, mt: k.box(m, mt, (6.149, -0.386, -2.048), (8.497, 0.114, 2.122))
    ramp = lambda m, mt: prism_xy(m, mt, [(2.336, -1.02), (2.49, -1.495), (5.955, -0.37), (5.8, 0.106)],
                                  -2.154, 2.112)
    mp = k.Mesh("PE_Balloon_Mesh")
    for f in (rail1, rail3, back):
        f(mp, plat)
    floor(mp, plat)
    # the basket's deck round the floor plate, under the rails (the ball cannot reach beyond them)
    k.cylinder(mp, plat, (0, -2.341, 0), (0, 1, 0), 4.3, 0.5, seg=24)
    entry(mp, bridge)
    ramp(mp, plat)
    # the basket's ring and the four posts the ropes are tied to
    k.torus(mp, metal, (0, 1.9, 0), (0, 1, 0), 8.0, 0.18, seg=32, tube_seg=6)
    for x, z in ((2.13, -7.99), (-5.84, -5.85), (-5.84, 5.85), (2.16, 7.98)):
        k.cylinder(mp, holder, (x, -1.8, z), (0, 1, 0), 0.3, 3.99, seg=8)
    # what is drawn: a round woven basket (open where the bridge comes in), its rim, a planked deck inside, and the
    # rails and the ramp the ball touches as wooden benches (the mesh above stays the physics')
    wicker = p.mat("PE_Balloon_Wicker", "wicker.png", diffuse=1.0, emissive=0.3)
    deckm = p.mat("PE_Balloon_Deck", "wood.png", diffuse=0.9, emissive=0.3)
    look = k.Mesh("PE_Balloon_Look")
    gap = math.radians(24)
    for r_out, r_in in ((8.3, 7.95),):
        rows = [(-2.6, 7.6), (-2.2, 8.1), (0.0, 8.3), (1.75, r_out)]
        for i in range(len(rows) - 1):
            (y0, a0), (y1, a1) = rows[i], rows[i + 1]
            n = 40
            for j in range(n):
                t0 = gap + (2 * math.pi - 2 * gap) * j / n
                t1 = gap + (2 * math.pi - 2 * gap) * (j + 1) / n
                for rr0, rr1, inward in ((a0, a1, False), (a0 - 0.35, a1 - 0.35, True)):
                    q = [(rr0 * math.cos(t0), y0, rr0 * math.sin(t0)), (rr0 * math.cos(t1), y0, rr0 * math.sin(t1)),
                         (rr1 * math.cos(t1), y1, rr1 * math.sin(t1)), (rr1 * math.cos(t0), y1, rr1 * math.sin(t0))]
                    uv = [(t0 * 2, y0 / 4), (t1 * 2, y0 / 4), (t1 * 2, y1 / 4), (t0 * 2, y1 / 4)]
                    tm = (t0 + t1) / 2
                    nrm = (math.cos(tm), 0, math.sin(tm)) if not inward else (-math.cos(tm), 0, -math.sin(tm))
                    look.face(wicker, q if not inward else q[::-1], uv if not inward else uv[::-1], normal=nrm)
    k.cylinder(look, deckm, (0, -2.6, 0), (0, 1, 0), 7.6, 0.26, seg=40)
    k.torus(look, metal, (0, 1.9, 0), (0, 1, 0), 8.15, 0.2, seg=48, tube_seg=8)
    for f in (rail1, rail3, back):
        f(look, deckm)
    floor(look, deckm)
    entry(look, bridge)
    ramp(look, deckm)
    for x, z in ((2.13, -7.99), (-5.84, -5.85), (-5.84, 5.85), (2.16, 7.98)):
        k.cylinder(look, holder, (x, -1.8, z), (0, 1, 0), 0.3, 3.99, seg=12)
        k.ellipsoid(look, metal, (x, 2.2, z), (0.36, 0.22, 0.36), seg=12)
    mp.look = look
    pi = p.mesh(mp)
    p.node("PE_Balloon_Platform", pi)
    hinge_a = [((2.132, 0.252, -7.989), (0.267, 0, -0.964)), ((-5.842, 0.252, -5.849), (-0.707, 0, -0.707)),
               ((-5.842, 0.252, 5.849), (0.707, 0, -0.707)), ((2.157, 0.252, 7.977), (-0.267, 0, -0.964))]
    hinge_b = [((4.013, 4.506, -14.926), (-0.25, 0, 0.968)), ((-10.877, 4.506, -10.872), (0.701, 0, 0.713)),
               ((-10.877, 4.506, 10.91), (0.701, 0, -0.713)), ((4.013, 4.506, 14.882), (-0.259, 0, -0.966))]
    for i in range(4):
        p.node(f"PE_Balloon_Ballon0{i + 1}_hinge_a", pos=hinge_a[i][0], x=hinge_a[i][1])
        p.node(f"PE_Balloon_Ballon0{i + 1}_hinge_b", pos=hinge_b[i][0], x=hinge_b[i][1])
    for i, x in enumerate((8.687, 10.312, 11.964, 13.637, 15.242, 16.93, 18.539, 20.195, 21.827), 1):
        p.node(f"PE_Balloon_Hinge0{i}", pos=(x, 0.007, -0.044))
    for name, pos in (("close", (0, 20, 0)), ("far", (-20, 20, 0)), ("high", (0, 7.002, 0)), ("low", (0, -6.421, 0))):
        p.node(f"PE_Balloon_Slideframe_{name}", pos=pos)
    mbox = k.Mesh("Box_slide_Mesh")
    k.box(mbox, None, (-0.5, 0, -0.5), (0.5, 1, 0.5))
    p.node("PE_Box_slide", p.mesh(mbox), pos=(0, 20, 0))
    balloons = [(4.013, 6.683, -14.926), (-10.877, 6.683, -10.872), (-10.877, 6.683, 10.91), (4.013, 6.683, 14.882)]
    for i, pos in enumerate(balloons, 1):
        mb = k.Mesh(f"PE_Balloon_Ballon0{i}_Mesh")
        lathe(mb, gores, BALLOON, 12)
        # drawn smooth: the same envelope with many more gores and rows
        fine = [(BALLOON[0][0], 0.0)]
        for (y0, r0), (y1, r1) in zip(BALLOON, BALLOON[1:]):
            for t in np.linspace(0, 1, 5)[1:]:
                fine.append((y0 + (y1 - y0) * t, r0 + (r1 - r0) * t))
        mbl = k.Mesh(f"PE_Balloon_Ballon0{i}_Look")
        lathe(mbl, gores, fine, 36)
        smooth_normals(mbl)
        mb.look = mbl
        p.node(f"PE_Balloon_Ballon0{i}", p.mesh(mb), pos=pos)
    # each balloon's ropes: from the balloon's hinge under it down to the platform's rim (the joints' frames), in
    # the rope object's own frame half way between; three strands from one knot fanning out to the rim
    seil = [(3.12, 1.856, -11.621), (-8.455, 1.856, -8.435), (-8.44, 1.856, 8.488), (3.119, 1.856, 11.577)]
    for i in range(4):
        ms = k.Mesh(f"PE_Balloon_Ballon_Seil0{i + 1}_Mesh")
        balloon_ropes(ms, rope, np.array(hinge_b[i][0]) - np.array(seil[i]), np.array(hinge_a[i][0]) - np.array(seil[i]))
        # what is drawn: the ropes tied where the balloon pulls, from its knot down to the top of its post and to the
        # basket's ring on either side of it (the hull above, measured as a whole, stays the physics')
        ms.look = rope_look(f"PE_Balloon_Ballon_Seil0{i + 1}_Look", rope, np.array(hinge_b[i][0]),
                            np.array(hinge_a[i][0]), np.array(seil[i]))
        p.node(f"PE_Balloon_Ballon_Seil0{i + 1}", p.mesh(ms), pos=seil[i])
    for i in range(1, 9):
        m = k.Mesh(f"PE_Balloon_Platte0{i}_Mesh")
        # the bridge's planks have the plate bridge's measured sizes, in the balloon's order
        plank(m, bridge, i + 1)
        p.node(f"PE_Balloon_Platte0{i}", p.mesh(m), pos=(9.506 + 1.639 * (i - 1), -0.593, 0.167))
    p.node("PE_Balloon_MF")
    for i, f in ((1, rail1), (2, ramp), (3, rail3), (4, floor), (5, back), (6, entry)):
        p.mesh(col(f"PE_Balloon_Col_0{i}_Mesh", lambda m, f=f: f(m, None)))
    p.group("All_PE_Balloon", [n["name"] for n in p.nodes])
    p.write()


# --- textures a run needs ----------------------------------------------------------------------------------------


def make_run_textures():
    """The ball's shadow (multiplied into the floor: white leaves it, the dark middle shades it), the particles'
    textures (added to the frame: black adds nothing), and three skies."""
    y, x = np.mgrid[0:64, 0:64]
    r = np.hypot(x - 31.5, y - 31.5) / 32
    shade = 1 - 0.78 * np.clip(1 - r, 0, 1) ** 0.9
    k.save_png(TEXTURES, "HardShadow.png", np.stack([shade] * 3, -1))
    part = os.path.join(TEXTURES, "Particles")

    def blob(size, colour, power, seed, wisps=0.0):
        yy, xx = np.mgrid[0:size, 0:size]
        rr = np.hypot(xx - size / 2 + 0.5, yy - size / 2 + 0.5) / (size / 2)
        a = np.clip(1 - rr, 0, 1) ** power
        if wisps:
            a *= 1 - wisps + wisps * k.value_noise(size, size, (4, 4), seed)
        return np.clip(np.array(colour)[None, None] * a[..., None], 0, 1)

    # the flame: a hot pink-white core (the game tints it with the flame's red), soft and a little ragged
    k.save_png(part, "Particle_Flames.png", blob(128, (0.95, 0.35, 0.85), 1.6, 201, 0.35))
    k.save_png(part, "Particle_Smoke.png", blob(64, (0.92, 0.92, 0.92), 1.2, 202, 0.5))
    k.save_png(part, "ExtraParticle.png", blob(64, (0.92, 0.18, 0.16), 1.8, 203))
    ball = blob(64, (1.0, 1.0, 1.0), 0.6, 204)
    k.save_png(part, "ExtraBall.png", ball)
    k.save_png(part, "P_Extra_Life_Particle.png", blob(64, (0.3, 0.07, 0.36), 1.0, 205, 0.4))
    make_skies()


# The sky box's faces as the game draws them (sky.rs): corners on the unit cube from the bottom left as seen from
# inside, the texture's rows from the top.
FACES = {
    "Front": [(-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)],
    "Right": [(1, -1, 1), (1, -1, -1), (1, 1, -1), (1, 1, 1)],
    "Back": [(1, -1, -1), (-1, -1, -1), (-1, 1, -1), (1, 1, -1)],
    "Left": [(-1, -1, -1), (-1, -1, 1), (-1, 1, 1), (-1, 1, -1)],
    "Down": [(-1, -1, -1), (1, -1, -1), (1, -1, 1), (-1, -1, 1)],
}
SKY = 512
# Three moods, each a colour by elevation (degrees) and its clouds: A dusk (violet above, rose below, as the menu's),
# B a clear day, C a storm at night.
SKIES = {
    "A": dict(profile=[(-90, (212, 176, 160)), (-30, (195, 166, 156)), (-12, (142, 136, 168)), (0, (122, 122, 172)),
                       (90, (96, 98, 160))], cloud=(0.9, 0.78, 0.72), shade=(0.6, 0.55, 0.62), seed=31),
    "B": dict(profile=[(-90, (200, 214, 226)), (-25, (186, 206, 226)), (-5, (170, 200, 236)), (5, (140, 182, 232)),
                       (90, (60, 110, 196))], cloud=(0.98, 0.98, 1.0), shade=(0.7, 0.76, 0.86), seed=41),
    "C": dict(profile=[(-90, (40, 42, 58)), (-25, (52, 54, 72)), (-5, (66, 68, 92)), (5, (44, 48, 76)),
                       (90, (14, 16, 34))], cloud=(0.42, 0.42, 0.5), shade=(0.12, 0.12, 0.18), seed=51),
}


def sky_colour(d, mood):
    e = np.degrees(np.arcsin(np.clip(d[..., 1], -1, 1)))
    prof = mood["profile"]
    xs = [q[0] for q in prof]
    c = np.stack([np.interp(e, xs, [q[1][i] / 255.0 for q in prof]) for i in range(3)], -1)
    az = np.arctan2(d[..., 0], d[..., 2])
    u = (az / (2 * np.pi) + 0.5) % 1.0
    w = 1024
    sx = np.clip(u * (w - 1), 0, w - 1).astype(int)
    # clouds: streaks near the horizon and a sea of puffs below
    streaks = k.value_noise(128, w, (3, 5), mood["seed"], octaves=5)
    sy = np.clip((-e + 10) / 40 * 127, 0, 127).astype(int)
    band = np.exp(-(((e - 2) / 12) ** 2))
    s = np.clip((streaks[sy, sx] - 0.5) * 2.5, 0, 1) * band
    c = c + (np.array(mood["cloud"]) - c) * (0.5 * s)[..., None]
    # the sea of cloud below is a layer seen in perspective: its noise is laid on a plane under the camera (by
    # azimuth and elevation it would swirl into the point straight down)
    sea = k.value_noise(512, 512, (8, 8), mood["seed"] + 1, octaves=6)
    depth = np.maximum(-d[..., 1], 0.12)
    px = ((d[..., 0] / depth * 0.18 + 0.5) % 1.0 * 511).astype(int)
    pz = ((d[..., 2] / depth * 0.18 + 0.5) % 1.0 * 511).astype(int)
    puff = np.clip((sea[pz, px] - 0.45) * 2.2, -1, 1)
    deep = np.clip((-e - 10) / 16, 0, 1)
    lit, sh = np.array(mood["cloud"]), np.array(mood["shade"])
    c = c + np.where((puff > 0)[..., None], (lit - c) * (0.6 * puff)[..., None], (sh - c) * (-0.4 * puff)[..., None]) \
        * deep[..., None]
    return np.clip(c, 0, 1)


def make_skies():
    j, i = np.mgrid[0:SKY, 0:SKY]
    s = (i + 0.5) / SKY
    t = 1 - (j + 0.5) / SKY
    folder = os.path.join(TEXTURES, "Sky")
    for letter, mood in SKIES.items():
        for face, corners in FACES.items():
            p0, p1, p2, p3 = (np.array(c, np.float64) for c in corners)
            q = (p0[None, None] * ((1 - s) * (1 - t))[..., None] + p1[None, None] * (s * (1 - t))[..., None]
                 + p2[None, None] * (s * t)[..., None] + p3[None, None] * ((1 - s) * t)[..., None])
            d = q / np.linalg.norm(q, axis=-1, keepdims=True)
            k.save_png(folder, f"Sky_{letter}_{face}.png", sky_colour(d, mood))


def main():
    make_balls()
    make_loose_balls()
    make_box()
    make_dome()
    make_trafos()
    make_animtrafo()
    make_flames()
    make_extra_life()
    make_extra_point()
    make_modul_01()
    make_modul_03()
    make_modul_08()
    make_modul_17()
    make_modul_18()
    make_modul_19()
    make_modul_25()
    make_modul_26()
    make_modul_29()
    make_modul_30()
    make_modul_34()
    make_modul_37()
    make_modul_41()
    make_balloon()
    if "--no-textures" not in sys.argv:
        make_run_textures()


if __name__ == "__main__":
    sys.exit(main())
