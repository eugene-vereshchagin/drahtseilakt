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
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ownkit as K  # noqa: E402

TEX = 256
# The kit's version, written into each level it makes (asset.generator): 2 since the floors are seamless (one mesh
# along the way, the cross-sections upright across the cursor's heading, the start's and checkpoints' pieces sharing
# the floors' vertices). crates/game/src/ownlevels.rs checks the floors of the levels made by version 2 on for seams.
KIT_VERSION = 2
OWN = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# The game's own texture set (CC0), by the original's texture names.
TEXTURE_SET = os.path.join(OWN, "Textures")
# The track's own textures and the menu scene's bricks, which the borders share (make_textures.py, step track).
TRACK_SET = os.path.join(OWN, "Track")
# The textures every level shares, named as the game's (`game:<name>`) instead of copied into each level.
SHARED = ("Track_Side", "Track_floor_border", "Track_floor_border_n", "Wood_Plain", "Track_Rail", "Track_Rail_n",
          "Track_Column", "Laterne_Glas", "Floor_Top_Checkpoint")
# One picture, one look (open-format.md, a material's `original`): what each of our levels' own pictures stands for
# in the original, by the picture's name (a set's letter left off), and how the original draws it. With a texture pack
# other than ours (the copy's, the remaster's, a player's) the game draws that texture instead, as it draws the parts
# and balls, so floors, rails and grilles never keep our look among the copy's. `game:` names that are the original's
# own texture names (wood, plates, lamps, the ball tiles) need no entry. Every other picture must be own-only below:
# a level is not written with a picture that is neither.
ORIGINAL_OF = {
    "Track_Top": ("Floor_Top_Borderless", None),
    "Track_Shade": ("Floor_Top_Borderless", None),
    # the kerb, the gutter's slope and the side carry the brick strip along their outer edge, as Floor_Side does
    "Track_Kerb": ("Floor_Side", None),
    "Track_Slope": ("Floor_Side", None),
    "Track_Side": ("Floor_Side", None),
    "Track_floor_border": ("Brick", None),
    "Track_Rail": ("Rail_Environment", "chrome"),
    "Track_Column": ("Column_beige", None),
    "Fan_Grille": ("Modul18_Gitter", None),
    "SkyLayer": ("SkyLayer", None),
}
# Pictures that stand for nothing of the original: the signs, the community part's (a community map's own textures)
# and the sticky ball's skin (the original has no sticky ball). They stay ours in every look.
OWN_ONLY = ("Signs_", "Comm_", "Ball_Sticky")


def original_of(texture):
    """What a material's picture stands for: ("original", texture, texgen), ("own",) or ("game",) for a `game:` name
    of the original's own texture set; ValueError for a picture that is none of them."""
    game = texture.startswith(K.GAME)
    stem = texture[len(K.GAME):] if game else os.path.splitext(texture)[0]
    if game and os.path.isfile(os.path.join(TEXTURE_SET, stem + ".png")):
        return ("game",)
    base = stem[:-2] if len(stem) > 2 and stem[-2] == "_" and stem[-1] in "ABC" else stem
    if base in ORIGINAL_OF:
        return ("original",) + ORIGINAL_OF[base]
    if stem.startswith(OWN_ONLY):
        return ("own",)
    raise ValueError(f"{texture}: neither one of the original's textures nor own-only (levelkit.ORIGINAL_OF)")


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
# Where every profile's flat top is cut (section), in from each edge: the kerb's inner edge and the gutter's foot (and
# the border's shade's); the foot's cut follows a narrow gutter's foot (at least GUTTER_FLAT out from the middle).
FLAT_CUTS = (1.0, 1.25, 2.5)
GUTTER_FLAT = 1.5
# The shade beside a border, as wide as a gutter's slope, so the flat between them is the same.
BORDER_SHADE = 1.5
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
# Rails, measured on the original's: two tubes 0.6 thick, their middles 3.9 apart (the ball, 4 across, sits deep
# between them, its middle 1.22 over theirs). Ours lie on the floor they leave (the tubes' undersides level with its
# top: the ball drops 0.48 onto them, not 1.08 as onto the original's, sunk 0.6 deeper), and where a floor runs on from
# an end each tube bends out over it, flat on its top (RAIL_FLARE: along, out), a mouth that leads a ball coming off
# the middle onto them, kept on the floor's flat, clear of its borders and railings (on a gutter, which leads the ball
# to its middle anyway, the mouth is narrower). No ties across between the tubes, as on the
# original's first levels (a few saddles on columns hold them, RAIL_SADDLE); `ties` gives them (a later level's nerves).
RAIL_GAUGE, RAIL_TUBE = 3.9, 0.3
RAIL_FLARE = (3.0, 1.4)
RAIL_POSTS = 8.0
# The ball's radius (every kind's) and the least room a way leaves it at the height of its middle, between borders or
# railings: a way narrower than this wedges the ball (the kit refuses to build it).
BALL_R = 2.0
MIN_FREE = 5.5
# A loose ball's reach to the physics: the wooden and stone ones are spheres of BALL_R, the paper one the hull of its
# part's P_Ball_Paper_MF (2.04 from its middle at most, 2.03 below it), so it is rested as a sphere of this radius.
LOOSE_R = {"Wood": BALL_R, "Stone": BALL_R, "Paper": 2.04}
# The tile in the ball's texture under a transformer lies this much over the floor's top (drawn over it, not in it).
TRAFO_TILE_LIFT = 0.01
# How deep a loose body may start in the floors: the original puts its paper balls 0.031 into the floor (level 10; its
# crumpled hull reaches lower than the stand-in's sphere), which does no harm; 1 and more deep, the game's physics
# shoves the body out as its sector begins, and the player's ball touching it then is thrown away at hundreds of units
# a second (check_clear).
SUNK_TOLERANCE = 0.05
# The cloud layer under a level (the original's SkyLayer, which the game draws added over the sky): a plane CLOUD_BELOW
# under the lowest floor, reaching CLOUD_MARGIN past the level on every side, its texture repeating every CLOUD_TILE
# (the original's: 160 or so below, 400 to 600 past, 675 a repeat; numbers only).
CLOUD_BELOW, CLOUD_MARGIN, CLOUD_TILE = 165.0, 500.0, 675.0
# How much of the sky the cloud layer covers, by sky (1 a full sea of cloud; the photographic skies with a land or a
# sea of their own below get scattered banks over it, drifting between the track and the world below).
CLOUD_COVER = {"A": 1.0, "B": 1.0, "C": 1.0, "Sunset": 1.0, "Night": 0.0}
CLOUD_SCATTERED = 0.45
# The fog our levels have (level settings `fog`; the renderer's): the sky's haze just under its horizon is its colour;
# FOG_DENSITY of it a unit of distance; under the lowest floor less FOG_BELOW it thickens, whole FOG_DEPTH further down,
# so the columns and the supports, which reach far down (COLUMN_DEPTH), fade out of sight instead of ending.
FOG_DENSITY, FOG_BELOW, FOG_DEPTH = 0.0011, 12.0, 150.0
COLUMN_DEPTH = 240.0
# A shaft under a cornice (a column, a tower's) stops this short of the cornice's top, inside it: two faces in one plane
# flicker where they overlap (levelcheck.py, check_coplanar).
TUCK = 0.5
SKY_DIR = os.path.join(OWN, "Textures", "Sky")
# Rails' supports: a saddle under both tubes (its bar RAIL_SADDLE under their middles, below the ball riding between
# them) on a column down into the haze, about every RAIL_SUPPORT of rails (the DSL picks where: not near their ends,
# where the floors hold them, and not over another floor or rails).
RAIL_SADDLE, RAIL_SUPPORT, RAIL_COLUMN = 1.3, 40.0, 240.0
# a clamp round each tube every this far along the rails (the concept's: about two and a half balls apart)
RAIL_CLAMP = 9.0
# The room a transformer (its cage rises 5.5 round the ball, held and lifted in it), a fan or the balloon needs over it.
HEADROOM = 12.0
# A way under another floor or rails leaves the ball either no way in (the floor above at most OVERPASS_WALL over it:
# it meets the side as a wall) or room to pass (OVERPASS_CLEAR or more); between, it wedges.
OVERPASS_WALL, OVERPASS_CLEAR = 3.6, 6.0
# Where the ball comes back after a fall, above the floor round the start or a checkpoint.
RESET_UP = START_RESET - START_BOWL[-1][1]

# The sea of cloud under a level (settings `clouds`, drawn by the game: crates/game/src/clouds.wgsl). By sky: how much
# of it is cloud (1 a closed sea; under the skies with a land below, banks with gaps the land shows through, `under`
# 0), how much of the world below the haze under it hides and (`sun`) how much of CLOUD_SUN its sun gives (a high
# sun lights the whole sea). The skies' light (make_skies.py writes sky_light.json)
# lights it: the sun's direction and colour (half way to white: a photograph's low sun is redder than the light on
# the clouds it shines on) and the sky's, the shade bluer than the sky. A sky without its light there has no sea.
CLOUD_SEA = {"Sunset": dict(cover=0.75, under=1.0, warm=0.7), "Clouds": dict(cover=0.8, under=1.0, sun=0.65),
             "Mountains": dict(cover=0.42, under=0.0), "Forest": dict(cover=0.38, under=0.0),
             "Mist": dict(cover=0.5, under=0.4), "Desert": dict(cover=0.3, under=0.0)}
# its tops CLOUD_TOP under the lowest floor, CLOUD_THICK deep, a cloud about CLOUD_SIZE across, CLOUD_HAZE of the
# horizon's haze a unit of distance
CLOUD_TOP, CLOUD_THICK, CLOUD_SIZE, CLOUD_HAZE = 35.0, 110.0, 85.0, 0.0005
# the light's strength on the clouds: the sun's, the sky's from above, the shade's
CLOUD_SUN, CLOUD_SKY, CLOUD_SHADE = 2.6, (1.15, 1.2, 1.35), (1.05, 0.75, 1.0)
SKY_LIGHT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sky_light.json")


def cloud_sea(sky, lowest, cover=None, below=None, thickness=None, scale=None):
    """The level's `clouds` settings under `sky` with its lowest floor's top at `lowest` (None: the sky has none); the
    level's own cover, depth under the lowest floor, thickness and size of a cloud override the sky's."""
    import json
    if cover is not None and cover <= 0:
        return None
    if not os.path.exists(SKY_LIGHT):
        return None
    with open(SKY_LIGHT) as f:
        light = json.load(f).get(sky)
    sea = CLOUD_SEA.get(sky)
    if light is None or (sea is None and cover is None):
        return None
    sea = sea or dict(cover=cover, under=1.0)
    sun = np.asarray(light["sun_color"], np.float64)
    chroma = sun / max(float(sun.max()), 1e-9)
    warm = sea.get("warm", 0.5)
    sun_col = (1 - warm + warm * chroma) * CLOUD_SUN * sea.get("sun", 1.0)
    sky_col = np.asarray(light["sky"], np.float64)
    r = lambda v: [round(float(x), 4) for x in v]  # noqa: E731
    return {"top": round(float(lowest) - (CLOUD_TOP if below is None else below), 2),
            "thickness": CLOUD_THICK if thickness is None else thickness,
            "cover": sea["cover"] if cover is None else cover,
            "scale": CLOUD_SIZE if scale is None else scale,
            "sun": r(light["sun"]), "sun_color": r(sun_col),
            "sky": r(sky_col * np.array(CLOUD_SKY)), "shadow": r(sky_col * np.array(CLOUD_SHADE)),
            "haze": CLOUD_HAZE, "under": sea["under"]}


def section(width, safety, surface="stone"):
    """A track's cross-section (_profile), its flat top cut at the same places across for every profile (FLAT_CUTS in
    from each edge), so two stretches of another profile meeting end to end share the vertices of the flat they have
    in common. Which edges are cut, and how often, is decided at a wide way, so every width of a profile has the same
    points (a stretch narrowing keeps them; on a narrow way a cut runs into its edge's end)."""
    ref = max(width, 12.0)
    pts_r, _ = _profile(ref, safety, surface)
    pts, edges = _profile(width, safety, surface)
    w, wr = width / 2, ref / 2
    n = len(pts)
    out_p, out_e = [pts[0]], []
    for k in range(n):
        a, b = pts[k], pts[(k + 1) % n]
        ar, br = pts_r[k], pts_r[(k + 1) % n]
        mat, kind = edges[k]
        inner = []
        if abs(ar[1]) < 1e-9 and abs(br[1]) < 1e-9 and abs(br[0] - ar[0]) > 1e-9:
            lo_r, hi_r = min(ar[0], br[0]), max(ar[0], br[0])
            lo, hi = min(a[0], b[0]), max(a[0], b[0])
            for c in FLAT_CUTS:
                for side in (-1, 1):
                    xr = side * (wr - c)
                    x = side * (max(w - c, GUTTER_FLAT) if c == GUTTER[-1][0] else w - c)
                    if lo_r + 1e-6 < xr < hi_r - 1e-6:
                        x = min(max(x, lo), hi)
                        x = min(x, 0.0) if side < 0 else max(x, 0.0)
                        inner.append((xr, min(max(x, lo), hi)))
            inner.sort(reverse=br[0] < ar[0])
            # in the order the edge runs, never back (on a narrow way two cuts may meet; they must not cross)
            up = b[0] >= a[0]
            prev, kept = a[0], []
            for xr, x in inner:
                x = max(x, prev) if up else min(x, prev)
                kept.append((xr, x))
                prev = x
            inner = kept
        seq = [a] + [(x, 0.0) for _, x in inner] + [b]
        for q0, q1 in zip(seq, seq[1:]):
            if kind[0] == "band" and abs(b[0] - a[0]) > 1e-12:
                t0 = (q0[0] - a[0]) / (b[0] - a[0])
                t1 = (q1[0] - a[0]) / (b[0] - a[0])
                d0, d1 = kind[1], kind[2]
                sub = ("band", d0 + (d1 - d0) * t0, d0 + (d1 - d0) * t1, kind[3])
            else:
                sub = kind
            out_e.append((mat, sub))
        out_p += [(x, 0.0) for _, x in inner]
        if k < n - 1:
            out_p.append(b)
    return out_p, out_e


def _profile(width, safety, surface="stone"):
    """A track's cross-section for its width and safety: its outline (across, up), from the bottom of its left side up,
    over the top and down the right side (the bottom closes it), and for each edge its material and how its texture
    runs across it: ("down", top) the side's, by the depth under `top`; ("band", d0, d1, w) a band's, by the distance
    in from its outer edge (d0 at the edge's first point, d1 at its second) over the band's width; ("slab",) the
    top's, by the distance across; ("brick",) the border's, by the run round its outline."""
    w = width / 2
    top = "wood" if surface == "wood" else "top"
    if safety == "gutter":
        g = min(1.0, (w - GUTTER_FLAT) / GUTTER[-1][0]) if w - GUTTER_FLAT < GUTTER[-1][0] else 1.0
        slope = [(d * g, h) for d, h in GUTTER]
        h0 = slope[0][1]
        left = [(-w + d, h) for d, h in slope]
        pts = [(-w, -DEPTH), (-w, 0.0)] + left + [(w - d, h) for d, h in reversed(slope)] + [(w, 0.0), (w, -DEPTH)]
        sw = slope[-1][0]
        edges = [("side", ("down", h0)), ("side", ("down", h0))]
        edges += [("slope", ("band", slope[k][0], slope[k + 1][0], sw)) for k in range(len(slope) - 1)]
        edges += [(top, ("slab",))]
        edges += [("slope", ("band", slope[k + 1][0], slope[k][0], sw)) for k in reversed(range(len(slope) - 1))]
        edges += [("side", ("down", h0)), ("side", ("down", h0)), ("side", ("slab",))]
        return pts, edges
    if safety == "border":
        b, h = BORDER_W, BORDER_H
        pts = [(-w, -DEPTH), (-w, 0.0), (-w, h), (-w + b, h), (-w + b, 0.0), (-w + b + BORDER_SHADE, 0.0),
               (w - b - BORDER_SHADE, 0.0), (w - b, 0.0), (w - b, h), (w, h), (w, 0.0), (w, -DEPTH)]
        edges = [("side", ("down", h)), ("side", ("down", h)), ("border", ("brick",)), ("border", ("brick",)),
                 ("shade", ("band", 0.0, BORDER_SHADE, BORDER_SHADE)), (top, ("slab",)),
                 ("shade", ("band", BORDER_SHADE, 0.0, BORDER_SHADE)),
                 ("border", ("brick",)), ("border", ("brick",)), ("side", ("down", h)), ("side", ("down", h)),
                 ("side", ("slab",))]
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


def join_points(width, safety, surface="stone"):
    """Where a stretch's end meets a flat piece (the start's or a checkpoint's): the points of the flat middle of its
    cross-section at the floor's top level (0), the run of them that holds the middle, by their distance across (to
    the right); the piece's edge has its corners there, so the two share every vertex along the join. The profile's
    slopes, kerbs and borders beside the flat are not joined (a flat piece's edge there would be a step under them):
    the piece's outline leaves them and its own sides go on from theirs."""
    pts, _ = section(width, safety, surface)
    run, best = [], None
    for x, y in pts + [(0.0, 1.0)]:
        if abs(y) < 1e-9:
            run.append(round(x, 9))
            continue
        if run and min(run) <= 1e-9 and max(run) >= -1e-9:
            best = sorted(set(run))
        run = []
    return best if best else sorted({round(x, 9) for x, y in pts if abs(y) < 1e-9})


def orient(m):
    """Turns every triangle of a mesh to face the way its vertices' normals say (its points counter-clockwise seen
    from that side), as the original's are: a face built in the other order would show its back to a tool that culls
    or reads the winding."""
    P = np.asarray(m.p, np.float64)
    N = np.asarray(m.n, np.float64)
    for mat, idx in m.parts.items():
        for t in range(0, len(idx) - 2, 3):
            a, b, c = idx[t], idx[t + 1], idx[t + 2]
            g = np.cross(P[b] - P[a], P[c] - P[a])
            if np.dot(g, N[a] + N[b] + N[c]) < 0:
                idx[t + 1], idx[t + 2] = c, b


def check_bowl(width):
    """A checkpoint's bowl on a way `width` wide: the measured one, smaller on a narrow way (its rim half a unit in
    from the edges) and its depth with it, so its slopes stay the measured ones (a stone ball resting in a steeper
    bowl does not roll out of it)."""
    k = min(CHECK_BOWL[-1][0], width / 2 - 0.5) / CHECK_BOWL[-1][0]
    return [(r * k, h * k) for r, h in CHECK_BOWL]


def ring_frames(path, headings=None, scales=None):
    """Each point of a track's path with its heading (horizontal, along the way), the direction to its right and up
    (the world's), and its direction along the path (with the slope). A cross-section stands upright across the
    heading, so two pieces meeting at a point with the same heading (a straight into a turn, a ramp into a flat) have
    the very same cross-section there: no wedge between them. The headings are the cursor's (exact on a turn);
    without them they are taken from the path."""
    pts = [np.asarray(p, np.float64) for p in path]
    up = np.array([0.0, 1.0, 0.0])
    out = []
    for i, p in enumerate(pts):
        t = pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]
        if headings is not None:
            f = np.asarray(headings[i], np.float64).copy()
        else:
            f = t.copy()
        f[1] = 0.0
        f /= np.linalg.norm(f) + 1e-12
        if np.linalg.norm(t) < 1e-12:
            t = f
        t = t / np.linalg.norm(t)
        side = np.array([f[2], 0.0, -f[0]])
        if scales is not None:
            # a mitred corner: the ring stands on the bisector and reaches 1/cos 45 degrees further across
            side = side * scales[i]
        out.append((p, f, side, up, t))
    return out


def track_sweep(m, mats, path, widths, along, safety, surface, caps=(True, True), headings=None, scales=None):
    """A stretch of track: its cross-section (section) carried along `path`, each point with its width (a width
    changing between two points tapers), its heading and its distance along the whole track (`along`), by which every
    texture runs on from one piece into the next with no seam. One strip of quads for each edge of the cross-section,
    the points of each ring shared by the quads before and after it (no cracks, no T-junctions along the way). Ends
    closed where `caps` says (a stretch running on into another piece is not)."""
    fr = ring_frames(path, headings, scales)
    secs = [section(w, safety, surface) for w in widths]
    edges = secs[0][1]
    n = len(secs[0][0])
    # the rings' points, computed once, so neighbouring strips share them exactly
    rings = [[p + s_ * q[0] + u_ * q[1] for q in sec[0]] for (p, f, s_, u_, t), sec in zip(fr, secs)]
    runs = []
    for sec in secs:
        run = [0.0]
        for k in range(n):
            run.append(run[-1] + float(np.linalg.norm(np.subtract(sec[0][(k + 1) % n], sec[0][k]))))
        runs.append(run)
    for e in range(n):
        mat, kind = edges[e]
        vscale = BRICK if kind[0] == "brick" else (SLAB if mat == "wood" else REPEAT)
        ids = []
        for i, (p, f, s_, u_, t) in enumerate(fr):
            a2, b2 = np.array(secs[i][0][e]), np.array(secs[i][0][(e + 1) % n])
            d2 = b2 - a2
            # the outline runs clockwise in (across, up): its outward side is to the left of each edge
            out = -s_ * d2[1] + u_ * d2[0]
            d = rings[i][(e + 1) % n] - rings[i][e]
            nm = np.cross(t, d)
            if np.linalg.norm(nm) < 1e-12:
                nm = out
            if np.dot(nm, out) < 0:
                nm = -nm
            nm = nm / (np.linalg.norm(nm) + 1e-12)
            ua, ub = _edge_uv(kind, a2, b2, runs[i][e], runs[i][e + 1])
            v = along[i] / vscale
            ids.append((m.vertex(rings[i][e], nm, (ua, v)), m.vertex(rings[i][(e + 1) % n], nm, (ub, v))))
        for i in range(len(fr) - 1):
            (a0, b0), (a1, b1) = ids[i], ids[i + 1]
            for tri in ((a0, b0, b1), (a0, b1, a1)):
                q = [np.asarray(m.p[k]) for k in tri]
                if np.linalg.norm(np.cross(q[1] - q[0], q[2] - q[0])) > 1e-10:
                    m.tri(mats[mat], *tri)
    for i, sgn, cap in ((0, -1, caps[0]), (len(fr) - 1, 1, caps[1])):
        if not cap:
            continue
        p, f, s_, u_, t = fr[i]
        sec = secs[i][0]
        # the outline runs clockwise seen from ahead (+f): reversed it is counter-clockwise in (across, up)
        poly = sec[::-1]
        tris = ear_clip(poly)
        ids = [m.vertex(p + s_ * q[0] + u_ * q[1], f * sgn, (q[0] / SLAB, -q[1] / SLAB)) for q in poly]
        for t3 in tris:
            if sgn > 0:
                m.tri(mats["side"], ids[t3[0]], ids[t3[1]], ids[t3[2]])
            else:
                m.tri(mats["side"], ids[t3[0]], ids[t3[2]], ids[t3[1]])


def rail_in(width):
    """How far in from a way's edge its railing stands: RAIL_IN, nearer the edge on a way too narrow for it to leave
    MIN_FREE beside a railing on one side (a stone lane 6 wide with a corner's railing: 0.325)."""
    return max(RAIL_R, min(RAIL_IN, width - MIN_FREE - RAIL_R - 0.005))


def free_width(width, safety, railings=(False, False), surface="stone"):
    """The room a way leaves the ball across at the height of its middle (BALL_R over the floor): between its
    borders, between its railings' rails, or its whole width."""
    pts, _ = section(width, safety, surface)
    xs = [x for x, y in pts if y > -1e-9]
    lo, hi = min(xs), max(xs)
    if safety == "border":
        lo, hi = lo + BORDER_W, hi - BORDER_W
    inset = rail_in(width) + RAIL_R
    if railings[0]:
        lo = max(lo, -width / 2 + inset)
    if railings[1]:
        hi = min(hi, width / 2 - inset)
    return hi - lo


def top_height(width, safety, x, surface="stone"):
    """The height of a cross-section's top at `x` across (the highest of its edges over it): where a railing stands."""
    pts, _ = section(width, safety, surface)
    best = None
    for k in range(1, len(pts) - 2):
        (x0, y0), (x1, y1) = pts[k], pts[k + 1]
        if abs(x1 - x0) < 1e-9:
            continue
        c = (x - x0) / (x1 - x0)
        if -1e-9 <= c <= 1 + 1e-9:
            y = y0 + (y1 - y0) * c
            best = y if best is None else max(best, y)
    return best if best is not None else 0.0


def zipper(inner, outer, feet=None):
    """Triangles between two closed loops round one middle, each given as (angle, vertex) sorted by angle from 0 on
    (the inner one starting at angle 0): every point of both is used, so an edge of the outer loop is never split by
    a vertex the other side does not have. `feet` (one an outer vertex: the angle of the point of the edge from it to
    the next nearest the middle) makes the fan hand over from one outer vertex to the next there: every inner vertex
    is then joined to an outer one that sees it past the inner loop (a convex outline round a circle), so no triangle
    reaches over the inner loop. Without it the hand-over is at the next outer vertex's angle, and a long outer edge
    (a checkpoint's front, as wide as the way) fans from its first corner over the bowl inside it."""
    two = 2 * math.pi
    j0 = min(range(len(outer)), key=lambda j: min(outer[j][0] % two, two - outer[j][0] % two))
    o = outer[j0:] + outer[:j0]
    base = o[0][0] % two
    if base > math.pi:
        base -= two
    oa = []
    for k, (a, v) in enumerate(o):
        a = (a - o[0][0]) % two + base
        if k and a <= oa[-1][0]:
            a += two
        oa.append((a, v))
    oa.append((oa[0][0] + two, oa[0][1]))
    if feet is None:
        hand = [oa[k + 1][0] for k in range(len(o))]
    else:
        f = feet[j0:] + feet[:j0]
        hand = [min(oa[k][0] + (f[k] - oa[k][0]) % two, oa[k + 1][0]) for k in range(len(o))]
    ia = inner + [(inner[0][0] + two, inner[0][1])]
    i = j = 0
    tris = []
    while i < len(inner) or j < len(o):
        adv_inner = j >= len(o) or (i < len(inner) and ia[i + 1][0] <= hand[j])
        if adv_inner:
            tris.append((ia[i][1], oa[j][1], ia[i + 1][1]))
            i += 1
        else:
            tris.append((ia[i][1], oa[j][1], oa[j + 1][1]))
            j += 1
    return tris


def _tris_near(tris, c, reach):
    """Which of the triangles (n, 3, 3) come within `reach` of point `c` (by their bounding boxes)."""
    if not len(tris):
        return np.zeros(0, bool)
    lo, hi = tris.min(1), tris.max(1)
    return np.all((lo < c + reach) & (hi > c - reach), axis=1)


def _closest_points(tris, p):
    """The point of each triangle (n, 3, 3) nearest to `p`, and whether `p` lies over the triangle's face (its
    projection inside it)."""
    a, b, c = tris[:, 0], tris[:, 1], tris[:, 2]
    n = np.cross(b - a, c - a)
    nn = np.linalg.norm(n, axis=1)
    ok = nn > 1e-12
    n = n / np.where(ok, nn, 1.0)[:, None]
    d = np.einsum("ij,ij->i", p - a, n)
    q = p - d[:, None] * n
    # barycentric test of the projection
    v0, v1, v2 = b - a, c - a, q - a
    d00, d01, d11 = (np.einsum("ij,ij->i", v0, v0), np.einsum("ij,ij->i", v0, v1), np.einsum("ij,ij->i", v1, v1))
    d20, d21 = np.einsum("ij,ij->i", v2, v0), np.einsum("ij,ij->i", v2, v1)
    den = d00 * d11 - d01 * d01
    den = np.where(np.abs(den) > 1e-18, den, 1.0)
    v = (d11 * d20 - d01 * d21) / den
    w = (d00 * d21 - d01 * d20) / den
    inside = ok & (v >= 0) & (w >= 0) & (v + w <= 1)
    best = np.where(inside[:, None], q, np.inf)
    for e0, e1 in ((a, b), (b, c), (c, a)):
        e = e1 - e0
        t = np.clip(np.einsum("ij,ij->i", p - e0, e) / np.maximum(np.einsum("ij,ij->i", e, e), 1e-18), 0, 1)
        on = e0 + t[:, None] * e
        closer = ~inside & (np.linalg.norm(p - on, axis=1) < np.linalg.norm(p - best, axis=1))
        best[closer] = on[closer]
    return best, inside, d


def _sphere_depth(tris, c, r):
    """How deep a sphere reaches into the triangles (0 when clear): the most of r less its distance to one, or r and
    more with its centre behind a face it lies over (inside the solid). As the game measures it (collide.rs)."""
    if not len(tris):
        return 0.0
    q, inside, d = _closest_points(tris, c)
    dist = np.linalg.norm(c - q, axis=1)
    near = dist < r
    depth = np.where(inside & (d < 0), r + dist, r - dist)
    return float(max(0.0, depth[near].max())) if near.any() else 0.0


def _sphere_gap(tris, c, r):
    """How far a sphere is from touching the triangles (its least distance to them less r; inf with none)."""
    if not len(tris):
        return math.inf
    q, _, _ = _closest_points(tris, c)
    return float(np.linalg.norm(c - q, axis=1).min() - r)


def _contact_offset(tris, c):
    """How far beside the centre (across, not below) the triangles' nearest point lies: 0 on a level floor."""
    q, _, _ = _closest_points(tris, c)
    k = int(np.argmin(np.linalg.norm(c - q, axis=1)))
    off = q[k] - c
    return float(math.hypot(off[0], off[2]))


def _rest_height(tris, c, r):
    """How far to move a sphere up (or down, negative) from `c` to rest on the triangles: out of them when it is in
    them (up to 4 r), down onto them when it is over them (up to 8); None when neither finds a resting place."""
    def depth(dy):
        return _sphere_depth(tris, c + np.array([0.0, dy, 0.0]), r)

    step = 0.05
    if depth(0.0) > 0.0:
        below, dy = 0.0, step
        while dy <= 4 * r and depth(dy) > 0.0:
            below, dy = dy, dy + step
        if dy > 4 * r:
            return None
        hi = dy
        lo = below
    else:
        dy = -step
        while dy >= -8.0 and depth(dy) <= 0.0:
            dy -= step
        if dy < -8.0:
            return None
        lo, hi = dy, dy + step
    # lo is in the floor, hi clear: halve down to where it just touches
    for _ in range(30):
        mid = (lo + hi) / 2
        if depth(mid) > 0.0:
            lo = mid
        else:
            hi = mid
    return hi + 0.005


def _hull_depth(tris, pts, axes):
    """How deep a convex body (its corner points, its own axes as columns) reaches into the triangles: for each one
    it overlaps, the least overlap over the separating axes (the triangle's normal, the body's axes, their edges
    crossed); the most of those (0 when clear)."""
    worst = 0.0
    for t in tris:
        cand = [np.cross(t[1] - t[0], t[2] - t[0])] + [axes[:, k] for k in range(3)]
        for e in (t[1] - t[0], t[2] - t[1], t[0] - t[2]):
            cand += [np.cross(e, axes[:, k]) for k in range(3)]
        least = math.inf
        for ax in cand:
            n = np.linalg.norm(ax)
            if n < 1e-9:
                continue
            ax = ax / n
            a, b = pts @ ax, t @ ax
            least = min(least, a.max() - b.min(), b.max() - a.min())
            if least <= 0:
                break
        if least > 0 and least != math.inf:
            worst = max(worst, least)
    return worst


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
        # the places where a way leaves the ball too little room (track): write() refuses the level while any are
        self.narrow = []
        # the parts standing on the floors (node, half the side of their footprint, what): write() checks that the
        # floor under each is flat, none of it through the part
        self.standing = []
        # parts that lift or hold the ball, not standing on a floor's top (a fan, the balloon): checked for headroom
        self.lifting = []
        # the loose balls (node, radius, what): write() rests each on the floor under it (rest_loose), and the loose
        # boxes (node, what): neither may start inside the floors (check_clear)
        self.loose = []
        self.boxes = []
        # the route (levelcheck.py): floor node -> (from, to) distance along the way; scenery marked `obstacle`
        # (by name); landings marked `drop` ((point, radius))
        self.route = {}
        self.intended = set()
        self.drops = []
        self.seed = seed
        # the level's behaviour graphs (scriptkit.Graph.json(); open-format.md, "Scripts"); None: no scripts
        self.scripts = None
        self._standard_materials()

    # --- materials ---------------------------------------------------------------------------------------------

    def _standard_materials(self):
        """The track's materials: the slabs, kerbs (with the brick strip), shade and gutter slopes of one of three sets
        by the level's seed (assets/own/Track, made by make_textures.py), copied into the level as its own; and the
        textures every level shares named as the game's (`game:<name>`, open-format.md): the track's sides and the
        borders' bricks (assets/own/Track) and, by the original's texture names, the wood, rails, posts and
        checkpoint plates of the game's own texture set (assets/own/Textures, CC0). The game draws those with the
        player's texture pack (ours by default), so a pack changes our levels too."""
        s = self.seed
        variant = "ABC"[s % 3]
        os.makedirs(self.tex, exist_ok=True)
        for name in (f"Track_Top_{variant}", f"Track_Kerb_{variant}", f"Track_Shade_{variant}",
                     f"Track_Slope_{variant}"):
            shutil.copyfile(os.path.join(TRACK_SET, name + ".png"), os.path.join(self.tex, name + ".png"))
        for name in SHARED:
            if not any(os.path.isfile(os.path.join(d, name + ".png")) for d in (TRACK_SET, TEXTURE_SET)):
                raise FileNotFoundError(f"{name}.png: not among the game's own textures (run make_textures.py)")
        g = K.GAME
        self.mat("top", f"Track_Top_{variant}.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("kerb", f"Track_Kerb_{variant}.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("shade", f"Track_Shade_{variant}.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("slope", f"Track_Slope_{variant}.png", diffuse=1.0, emissive=0.12, specular=0.3, power=100)
        self.mat("side", g + "Track_Side", diffuse=0.6, emissive=0.42)
        # the borders: the menu scene's brick, with its relief
        self.mat("border", g + "Track_floor_border", diffuse=1.0, emissive=0.24, normal=g + "Track_floor_border_n")
        self.mat("wood", g + "Wood_Plain", diffuse=1.0, emissive=0.25, specular=0.39, power=50)
        self.mat("rail", g + "Track_Rail", diffuse=1.0, emissive=0.22, specular=0.85, power=40, normal=g + "Track_Rail_n")
        self.mat("post", g + "Track_Column", diffuse=0.8, emissive=0.3)
        self.mat("lamp", g + "Laterne_Glas", diffuse=1.0, emissive=1.0)
        self.mat("plate", g + "Floor_Top_Checkpoint", diffuse=1.0, emissive=0.2, specular=0.3, power=60)

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

    def track(self, name, path, widths, along, safety="border", surface="stone", caps=(True, True), headings=None,
              railings=None, scales=None):
        """A stretch of track along a path of points (its top's centre line), each with its width, its heading and its
        distance along the whole track (track_sweep). `railings`: for each point, whether a railing runs on its left
        and on its right from the point before to it ((left, right) pairs); none given, both sides have them when the
        safety is railings."""
        if safety not in SAFETY:
            raise ValueError(f"safety is one of {', '.join(SAFETY)}")
        if railings is None:
            railings = [(safety == "railings",) * 2] * len(path)
        narrow = [(p, w, free_width(w, safety, r, surface)) for p, w, r in zip(path, widths, railings)
                  if free_width(w, safety, r, surface) < MIN_FREE - 1e-6]
        for p, w, f in narrow:
            self.narrow.append(f"{name} at ({p[0]:.1f}, {p[1]:.1f}, {p[2]:.1f}): {w:.1f} wide, {f:.2f} free ({safety})")
        m = K.Mesh(name)
        track_sweep(m, self.mat_ids, path, widths, along, safety, surface, caps, headings, scales)
        i = self.add(name, m, groups=self._floor_groups("wood" if surface == "wood" else "stone"))
        for k, side in ((0, "Left"), (1, "Right")):
            runs, cur = [], None
            for j in range(1, len(path)):
                if railings[j][k]:
                    cur = cur if cur is not None else j - 1
                elif cur is not None:
                    runs.append((cur, j - 1))
                    cur = None
            if cur is not None:
                runs.append((cur, len(path) - 1))
            for r, (j0, j1) in enumerate(runs):
                sl = slice(j0, j1 + 1)
                self.railings(f"{name}_Railing_{side}{r + 1}", path[sl], widths[sl],
                              None if headings is None else headings[sl], safety, surface, sides=(k,),
                              scales=None if scales is None else scales[sl])
        return i

    def _rail_runs(self, base):
        """The railing's base points as runs left after the openings `rail_cuts` ((x, z, r): a side opening, a pocket's
        or a balcony's mouth, where a railing must leave a gap)."""
        cuts = getattr(self, "rail_cuts", None)
        if not cuts:
            return [base]
        pts = [np.asarray(base[0], np.float64)]
        for a, c in zip(base[:-1], base[1:]):
            a, c = np.asarray(a, np.float64), np.asarray(c, np.float64)
            n = max(1, int(math.ceil(np.linalg.norm(c - a) / 0.5)))
            pts += [a + (c - a) * (k / n) for k in range(1, n + 1)]
        runs, cur = [], []
        for p in pts:
            inside = any(math.hypot(p[0] - x, p[2] - z) < r for x, z, r in cuts)
            if inside:
                if len(cur) > 1:
                    runs.append(cur)
                cur = []
            else:
                cur.append(p)
        if len(cur) > 1:
            runs.append(cur)
        return runs

    def _rail_run(self, m, rail, base, up):
        for h in RAIL_HEIGHTS:
            K.tube(m, rail, [b + up * h for b in base], RAIL_R, caps=True)
        # posts at the ends and about every POST_EVERY between
        seg = [float(np.linalg.norm(base[j + 1] - base[j])) for j in range(len(base) - 1)]
        total = sum(seg)
        count = max(1, int(round(total / POST_EVERY)))
        for j in range(count + 1):
            t = total * j / count
            q = 0
            while q < len(seg) - 1 and t > seg[q]:
                t -= seg[q]
                q += 1
            c = t / seg[q] if seg[q] > 0 else 0.0
            p = base[q] * (1 - c) + base[q + 1] * c
            K.tube(m, rail, [p, p + up * (RAIL_HEIGHTS[-1] + RAIL_R)], RAIL_R * 0.8, caps=True)

    def railings(self, name, path, widths, headings=None, safety="railings", surface="stone", sides=(0, 1),
                 scales=None):
        """Steel railings along the edges of a stretch (`sides`: 0 its left, 1 its right): two rails on posts
        (RAIL_HEIGHTS over the floor's top where they stand), solid to the ball (the original's railings are in the
        rails' physics group too)."""
        m = K.Mesh(name)
        fr = ring_frames(path, headings, scales)
        rail = self.mat_ids["rail"]
        for k in sides:
            sgn = -1.0 if k == 0 else 1.0
            off = [sgn * (w / 2 - rail_in(w)) for w in widths]
            base = [p + s_ * o + u * top_height(w, safety, o, surface)
                    for (p, f, s_, u, t), o, w in zip(fr, off, widths)]
            for run in self._rail_runs(base):
                self._rail_run(m, rail, run, fr[0][3])
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

    def fan_block(self, name, at, fwd, sector=0, half=2.5, hole=1.8, depth=3.1, sides=True):
        """The fan's own block, as the original's levels set every fan (their floor round a fan is a 5 x 5 tile of
        material Modul18_Top with a round hole of radius 1.8 and a grille, Modul18_Gitter, over it at the floor's
        top, the fan sunk 3 under it): a square tile with a round hole whose top is `at` (the middle), a steel
        collar round the hole, a shaft down to a floor, and the grille over the hole, which the ball rolls over (it
        is a floor) and the updraft blows through. Its sides are drawn (a block standing alone shows them; `sides`
        False where a floor stands round it, whose own sides would lie in their planes). `fwd`: the way's heading (the
        block's sides along it)."""
        f = np.asarray(fwd, np.float64)
        f = f / np.linalg.norm(f)
        lft = np.array([-f[2], 0.0, f[0]])
        c = np.asarray(at, np.float64)
        up = np.array([0.0, 1.0, 0.0])
        if "fan_grille" not in self.mat_ids:
            n = 256
            v, u = np.mgrid[0:n, 0:n] / n
            # as the original's grille reads (measured on its picture): a square grid of flat grey bars, seven each
            # way, a quarter of their pitch wide (8 to the hole's width), a dark solid hub over the fan's axle, 0.27 of
            # the radius; the rest open. No rings, no shine: a dull steel grate over the dark shaft.
            r = np.hypot(u - 0.5, v - 0.5) * 2
            bars = (np.abs(((u * 8) % 1) - 0.5) > 0.375) | (np.abs(((v * 8) % 1) - 0.5) > 0.375)
            hub = r < 0.27
            solid = bars | hub
            shade = np.where(hub, 0.32, 0.52) * (0.96 + 0.08 * K.value_noise(n, n, (16, 16), 11))
            rgb = np.stack([shade, shade, shade * 1.02], -1)
            K.save_png(self.tex, "Fan_Grille.png", rgb, solid.astype(np.float64))
            self.mat("fan_grille", "Fan_Grille.png", diffuse=1.0, emissive=0.3, specular=0.15, power=10,
                     alpha="MASK", cutoff=0.5, double_sided=True)
        P = lambda a, s, h: c + f * a + lft * s + up * h  # noqa: E731
        seg = 24
        top = K.Mesh(name)
        # the tile: from the hole's rim out to the square's edge, a quad a step (winding as the floors': up)
        for k in range(seg):
            a0, a1 = 2 * math.pi * k / seg, 2 * math.pi * (k + 1) / seg
            pts = []
            for a in (a0, a1):
                d = np.array([math.cos(a), math.sin(a)])
                pts.append((d * hole, d * (half / max(abs(d[0]), abs(d[1])))))
            q = [P(*pts[0][0], 0), P(*pts[0][1], 0), P(*pts[1][1], 0), P(*pts[1][0], 0)]
            uv = [((p - c) @ f / (2 * half) + 0.5, (p - c) @ lft / (2 * half) + 0.5) for p in q]
            top.face(self.mat_ids["top"], q[::-1], uv[::-1], normal=(0, 1, 0))
        # the grille over the hole: a floor too
        for k in range(seg):
            a0, a1 = 2 * math.pi * k / seg, 2 * math.pi * (k + 1) / seg
            q = [c.copy(), P(math.cos(a1) * hole, math.sin(a1) * hole, 0), P(math.cos(a0) * hole, math.sin(a0) * hole, 0)]
            uv = [(0.5 + (p - c) @ f / (2 * hole), 0.5 + (p - c) @ lft / (2 * hole)) for p in q]
            top.face(self.mat_ids["fan_grille"], q, uv, normal=(0, 1, 0))
        self.add(name, top, groups=self._floor_groups("metal") + self.sector(sector))
        # only drawn: the collar, the shaft and its floor, the block's sides
        m = K.Mesh(name + "_Shaft")
        # the collar: its outside and a flat ring on top round the hole, open over the hole (with the frustum's caps
        # a steel disc 0.18 over the grille covered it: the grille was never seen)
        K.cylinder(m, self.mat_ids["rail"], tuple(c - up * 0.02), up, hole + 0.35, 0.2, seg=seg, r_top=hole + 0.2,
                   caps=False)
        for k in range(seg):
            a0, a1 = 2 * math.pi * k / seg, 2 * math.pi * (k + 1) / seg
            ring = [P(math.cos(a) * r, math.sin(a) * r, 0.18) for a, r in
                    ((a0, hole), (a0, hole + 0.2), (a1, hole + 0.2), (a1, hole))]
            m.face(self.mat_ids["rail"], ring, normal=(0, 1, 0))
        for k in range(seg):
            a0, a1 = 2 * math.pi * k / seg, 2 * math.pi * (k + 1) / seg
            p0 = P(math.cos(a0) * hole, math.sin(a0) * hole, 0)
            p1 = P(math.cos(a1) * hole, math.sin(a1) * hole, 0)
            m.face(self.mat_ids["side"], [p0, p1, p1 - up * depth, p0 - up * depth])
        m.face(self.mat_ids["side"], [P(-hole, -hole, -depth), P(-hole, hole, -depth), P(hole, hole, -depth),
                                      P(hole, -hole, -depth)], normal=(0, 1, 0))
        for s0, s1 in (((-1, -1), (1, -1)), ((1, -1), (1, 1)), ((1, 1), (-1, 1)), ((-1, 1), (-1, -1))) if sides else ():
            a, b = P(s0[0] * half, s0[1] * half, 0), P(s1[0] * half, s1[1] * half, 0)
            m.face(self.mat_ids["side"], [b, a, a - up * (depth + 0.4), b - up * (depth + 0.4)])
        self.add(name + "_Shaft", m)

    def stopper(self, name, lo, hi):
        """An unseen floor stopper (Phys_FloorStopper): it holds boxes and modules' parts, the ball passes it."""
        m = K.Mesh(name)
        K.box(m, None, lo, hi)
        return self.add(name, m, visible=False, groups=["Phys_FloorStopper"])

    def _floor_groups(self, sound):
        sid = {"stone": "01", "wood": "02", "metal": "03"}[sound]
        phys = "Phys_FloorRails" if sound == "metal" else "Phys_Floors"
        return [phys, f"Sound_HitID_{sid}", f"Sound_RollID_{sid}", "Shadow"]

    def rails(self, name, path, gauge=RAIL_GAUGE, r=RAIL_TUBE, posts=RAIL_POSTS, sector=0, headings=None,
              ends=(None, None), ties=False, supports=()):
        """Two steel tubes the ball rolls on, `gauge` apart (across the cursor's heading at each point, so rails laid
        piece after piece run on without a kink), lying on the path (the floor's top where they begin and end). `ends`:
        for each end, the floor running on from it ((width, safety, surface, (railing left, right)), its middle on the
        rails' line) or None;
        over such a floor each tube bends out flat on its top (RAIL_FLARE). At the path's points `supports`, a saddle
        under the tubes on a column down into the haze; with `ties`, ties across between the tubes every `posts`."""
        m = K.Mesh(name)
        fr = ring_frames(path, headings)
        lift = np.array([0.0, r, 0.0])
        ahead, out = RAIL_FLARE
        for sgn in (-1.0, 1.0):
            line = [p + s * (sgn * gauge / 2) + lift for p, f, s, u, t in fr]
            for end, k, floor in ((0, -1.0, ends[0]), (-1, 1.0, ends[1])):
                if floor is None:
                    continue
                width, safety, surface, railed = floor
                # the mouth's tips stay inside the room the floor leaves (its borders, its railings)
                # and on the floor's flat (a gutter's slope would bend them up; the gutter is a mouth of its own)
                flat = 0.0
                while flat < width / 2 and top_height(width, safety, flat + 0.05, surface) < 1e-6:
                    flat += 0.05
                room = min(free_width(width, safety, railed, surface) / 2, flat)
                o = max(0.0, min(out, room - r - 0.1 - gauge / 2))
                p, f, s, u, t = fr[end]
                t3 = f * k
                bend = []
                for c in np.linspace(0.25, 1.0, 4):
                    across = sgn * (gauge / 2 + o * c * c)
                    q = p + t3 * (ahead * c) + s * across
                    q[1] = p[1] + r
                    bend.append(q)
                line = (bend[::-1] + line) if end == 0 else (line + bend)
            K.tube(m, self.mat_ids["rail"], line, r, caps=True)
        if ties:
            run = 0.0
            for i in range(1, len(fr) - 1):
                run += float(np.linalg.norm(fr[i][0] - fr[i - 1][0]))
                if run >= posts:
                    run = 0.0
                    p, f, s, u, t = fr[i]
                    K.tube(m, self.mat_ids["rail"], [p - s * gauge / 2 + lift, p + s * gauge / 2 + lift], r * 0.7)
        rail_id = self.add(name, m, groups=self._floor_groups("metal") + self.sector(sector))
        # the fittings, only drawn (never in the ball's way): a clamp round each tube every RAIL_CLAMP, its strap
        # bent down under the ball's path to the other tube's clamp; at the supports a thin rod from the strap down
        # into the haze (after the owner's concept of the rails)
        fit = K.Mesh(f"{name}_Fittings")
        steel = self.mat_ids["rail"]
        run = [0.0]
        for i in range(1, len(fr)):
            run.append(run[-1] + float(np.linalg.norm(fr[i][0] - fr[i - 1][0])))
        total = run[-1]
        count = int(total // RAIL_CLAMP)
        spots = [RAIL_CLAMP * (k + 0.5) + (total - RAIL_CLAMP * count) / 2 for k in range(count)]
        spots = [d for d in spots if RAIL_CLAMP / 2 <= d <= total - RAIL_CLAMP / 2]
        # the rods hang from the straps: each support moves to the clamp nearest it
        held = {min(range(len(spots)), key=lambda k: abs(spots[k] - run[i])) for i in supports} if spots else set()
        for k, d in enumerate(spots):
            j = int(np.searchsorted(run, d))
            j = min(max(j, 1), len(fr) - 1)
            c0 = (d - run[j - 1]) / max(run[j] - run[j - 1], 1e-9)
            # the very point and frame the tubes have there (the tubes run straight between the path's points)
            p = fr[j - 1][0] * (1 - c0) + fr[j][0] * c0
            # (the side's blend unnormalised: the tubes' points between two of the path's are exactly that blend)
            s_lin = fr[j - 1][2] * (1 - c0) + fr[j][2] * c0
            s_ = s_lin / np.linalg.norm(s_lin)
            t = fr[j][0] - fr[j - 1][0]
            t = t / (np.linalg.norm(t) + 1e-12)
            u = np.array([0.0, 1.0, 0.0])
            # the clamp's plane square to the tube (on a slope too)
            up_t = u - t * np.dot(u, t)
            up_t = up_t / np.linalg.norm(up_t)
            c = p + lift
            for sgn in (-1.0, 1.0):
                tube_c = c + s_lin * (sgn * gauge / 2)
                inward = -s_ * sgn
                inward = inward - t * np.dot(inward, t)
                inward = inward / np.linalg.norm(inward)
                # round the tube's outer side and underneath, clear of where the ball rests on it (above, inward),
                # its band touching the tube all round
                for off in (-0.12, 0.12):
                    arc = [tube_c + t * off + (inward * math.cos(a) + up_t * math.sin(a)) * (r + 0.05)
                           for a in np.radians(np.linspace(110, 330, 12))]
                    K.tube(fit, steel, arc, 0.05, seg=6)
                K.sphere(fit, steel, tuple(tube_c + (inward * math.cos(math.radians(110))
                                                     + up_t * math.sin(math.radians(110))) * (r + 0.1)), 0.07, seg=6)
            # the strap from one clamp's foot to the other's, bent down under the ball's path
            foot = {sgn: c + s_lin * (sgn * gauge / 2) - up_t * (r + 0.05) for sgn in (-1.0, 1.0)}
            bar = c - u * RAIL_SADDLE
            strap = [foot[-1.0], bar - s_ * 0.8, bar + s_ * 0.8, foot[1.0]]
            K.tube(fit, steel, strap, 0.08, seg=6, caps=True)
            if k in held:
                # a thin rod from the strap's middle down into the haze, a collar where it meets the strap
                K.cylinder(fit, steel, bar - u * 0.25, u, 0.16, 0.3, seg=8)
                K.cylinder(fit, steel, bar - u * RAIL_COLUMN, u, 0.12, RAIL_COLUMN, seg=8)
        if fit.p:
            self.add(f"{name}_Fittings", fit)
        return rail_id


    def bowl_piece(self, name, centre, fwd, bowl, outer, joins, cups, along=0.0):
        """A platform with a bowl in its middle (the start, a checkpoint): its top at `centre` (the floor's height round
        the bowl), the bowl's profile `bowl` ((radius, height over its bottom) from its middle out), the platform's
        outline `outer` ((ahead, left) about the middle, counter-clockwise from ahead towards the left, seen round
        the middle), flame cups at `cups` ((ahead, left)). The edges in `joins` (by index: outer[k] to outer[k+1])
        are where a stretch of track runs on from the platform: there the platform has no side down to the track's
        depth (the track's own sides go on from its sides), and its corners are the track's (join_points), so the
        two share every vertex. The bowl is drawn with the checkpoint's floor (its rings round a grate at its middle,
        the outer ring at its rim), the platform round it with the track's slabs, mapped as the track's (across, and
        along by `along`, the distance along the track at the middle)."""
        c = np.asarray(centre, np.float64)
        f = np.asarray(fwd, np.float64)
        f = f / np.linalg.norm(f)
        left = np.array([-f[2], 0.0, f[0]])
        up = np.array([0.0, 1.0, 0.0])
        rb, deep = bowl[-1][0], bowl[-1][1]
        bottom = c - up * deep
        tex = rb * 64 / 62  # the texture's half side, its outer ring's edge (62 of 64) at the rim
        m = K.Mesh(name)
        plate, top, side = self.mat_ids["plate"], self.mat_ids["top"], self.mat_ids["side"]
        angles = [2 * math.pi * k / 64 for k in range(64)]

        def at(r, a, h):
            return bottom + f * (r * math.cos(a)) + left * (r * math.sin(a)) + up * h

        def uv_bowl(r, a):
            return (0.5 + r * math.cos(a) / (2 * tex), 0.5 + r * math.sin(a) / (2 * tex))

        def uv_top(ahead, lft):
            return (-lft / SLAB + 0.5, (along + ahead) / REPEAT)

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
        # the top round the bowl: from the rim out to the outline, one triangle fan zipped between the two loops
        inner = [(a, m.vertex(at(rb, a, deep), up, uv_top(rb * math.cos(a), rb * math.sin(a)))) for a in angles]
        world = [c + f * x + left * z for x, z in outer]
        outer_v = [(math.atan2(z, x) % (2 * math.pi), m.vertex(w, up, uv_top(x, z))) for (x, z), w in zip(outer, world)]
        # each outline edge's point nearest the middle: where the fan hands over to its next corner
        feet = []
        for k, (x0, z0) in enumerate(outer):
            x1, z1 = outer[(k + 1) % len(outer)]
            dx, dz = x1 - x0, z1 - z0
            t = min(max(-(x0 * dx + z0 * dz) / max(dx * dx + dz * dz, 1e-12), 0.0), 1.0)
            feet.append(math.atan2(z0 + t * dz, x0 + t * dx) % (2 * math.pi))
        for t in zipper(inner, outer_v, feet):
            m.tri(top, *t)
        # the sides round the outline: down to the track's depth (none where a track runs on), then to the platform's
        run = 0.0
        n = len(outer)
        for k in range(n):
            p0, p1 = world[k], world[(k + 1) % n]
            L = float(np.linalg.norm(p1 - p0))
            if L < 1e-9:
                continue
            d = (p1 - p0) / L
            nm = np.array([d[2], 0.0, -d[0]])
            for y0, y1 in ((0.0, DEPTH), (DEPTH, START_DEEP)):
                if y0 == 0.0 and k in joins:
                    continue
                q = [p0 - up * y0, p1 - up * y0, p1 - up * y1, p0 - up * y1]
                uv = [(y0 / SLAB, run / REPEAT), (y0 / SLAB, (run + L) / REPEAT), (y1 / SLAB, (run + L) / REPEAT),
                      (y1 / SLAB, run / REPEAT)]
                m.face(side, q, uv, normal=nm)
            run += L
        pts = [w - up * START_DEEP for w in world]
        ids = [m.vertex(q, -up, (q[0] / SLAB, q[2] / SLAB)) for q in pts]
        for t in ear_clip([(x, z) for x, z in outer]):
            m.tri(side, ids[t[0]], ids[t[2]], ids[t[1]])
        # the flame cups: a column each, its top a little over the floor with the checkpoint's rings on it
        for ahead, lft in cups:
            p = c + f * ahead + left * lft
            K.cylinder(m, side, p - up * (START_DEEP - 1.0), up, CUP_R, START_DEEP - 1.0 + CUP_UP,
                       seg=24, cap_mat=plate, vscale=SLAB)
        self.add(name, m, groups=self._floor_groups("stone"))
        return bottom

    def start(self, at, width=14.0, join=None, along=-START_RADIUS):
        """The start: a round platform with a bowl in its middle the ball sits in, four flame cups round it and a neck
        as wide as the track leaving along +X (to 10 ahead), the stand-in of PS_FourFlames in the bowl's bottom and
        the first reset point over it. `at` is the platform's top centre (the floor's height round the bowl); `join`
        the first stretch's join_points when it runs on from the neck's front edge. Returns the reset point."""
        R = START_RADIUS
        hw = min(width, 2 * R) / 2
        neck = math.sqrt(max(R * R - hw * hw, 0.0))
        front = [-x for x in (join or [-hw, hw])]
        outer = [(R, lft) for lft in sorted(front)]
        outer.append((neck, hw))
        meet = math.atan2(hw, neck)
        steps = 48
        for k in range(1, steps):
            a = meet + (2 * math.pi - 2 * meet) * k / steps
            outer.append((R * math.cos(a), R * math.sin(a)))
        outer.append((neck, -hw))
        joins = set(range(len(front) - 1)) if join else set()
        bottom = self.bowl_piece("Start_Platform", at, (1, 0, 0), START_BOWL, outer, joins,
                                 [(sa * 6.1, sl * 7.3) for sa in (-1, 1) for sl in (-1, 1)], along)
        # the start stands on a stone tower going down into the haze (only drawn), a cornice of brick under its edge
        tower = K.Mesh("Start_Tower")
        x, y, z = at
        K.cylinder(tower, self.mat_ids["post"], (x, y - START_DEEP - COLUMN_DEPTH, z), (0, 1, 0), START_RADIUS - 0.8,
                   COLUMN_DEPTH - TUCK, seg=32, vscale=SLAB)
        K.cylinder(tower, self.mat_ids["border"], (x, y - START_DEEP - 1.4, z), (0, 1, 0), START_RADIUS + 0.3, 1.4,
                   seg=32, vscale=BRICK)
        self.add("Start_Tower", tower)
        for sa in (-1, 1):
            for sl in (-1, 1):
                self.lantern((x + sa * 9.0, y, z + sl * 9.0), (sa * 0.7071, 0.0, sl * 0.7071))
        # its z axis points back at the camera (-X): x axis (0, 0, 1)
        self.placeholder("PS_FourFlames_01", ["PS_Levelstart"], K.pose(tuple(bottom), (0, 0, 1)), (-8, 0, -7),
                         (8, 6, 7))
        reset = tuple(bottom + np.array([0.0, START_RESET, 0.0]))
        self.reset(1, reset)
        return reset

    def lantern(self, at, out, name="Fixture"):
        """A lantern on a wooden post standing at `at` (on the floor's level, just outside its edge, on a stone corbel
        set into the floor's side), its arm reaching over `out`'s way, its glass glowing: only drawn."""
        m = K.Mesh("lantern")
        p = np.asarray(at, np.float64)
        o = np.asarray(out, np.float64)
        o = o / (np.linalg.norm(o) + 1e-9)
        up = np.array([0.0, 1.0, 0.0])
        K.obox(m, self.mat_ids["side"], tuple(p - up * 0.9), (tuple(o), tuple(up), tuple(np.cross(o, up))),
               (0.55, 0.9, 0.55), scale=SLAB)
        K.cylinder(m, self.mat_ids["wood"], p, up, 0.22, 6.2, seg=8)
        arm = p + up * 5.9
        K.cylinder(m, self.mat_ids["wood"], arm, -o, 0.12, 1.2, seg=6)
        lamp = arm - o * 1.2 - up * 0.75
        K.cylinder(m, self.mat_ids["rail"], lamp + up * 0.62, up, 0.05, 0.18, seg=6)
        K.cylinder(m, self.mat_ids["lamp"], lamp - up * 0.3, up, 0.28, 0.6, seg=8, cap_mat=self.mat_ids["rail"])
        K.cylinder(m, self.mat_ids["rail"], lamp + up * 0.3, up, 0.36, 0.08, seg=8, r_top=0.12)
        self.add(f"{name}_{len(self.nodes):03d}", m)

    def reset(self, n, point, xaxis=(-1, 0, 0)):
        """Reset point n at `point` (where the ball comes back). As in the original levels, the way on is along the
        point's -x axis: the camera looks that way after a respawn (give the track's direction negated)."""
        self.placeholder(f"PR_Resetpoint_{n:02d}", ["PR_Resetpoints"], K.pose(point, xaxis), (-2, -2, -2), (2, 2, 2))

    def column(self, name, top, depth=40.0, r=2.2):
        """A stone column hanging under the floor from `top` down `depth`: only for the look (no physics)."""
        x, y, z = top
        m = K.Mesh(name)
        K.cylinder(m, self.mat_ids["post"], (x, y - DEPTH - depth, z), (0, 1, 0), r, depth - TUCK, seg=12, vscale=SLAB)
        K.cylinder(m, self.mat_ids["border"], (x, y - DEPTH - 1.2, z), (0, 1, 0), r * 1.35, 1.2, seg=12)
        return self.add(name, m)

    def checkpoint(self, n, at, width=14.0, fwd=(1, 0, 0), back=None, front=None, along=0.0):
        """Checkpoint n (1, 2...) at `at` (the floor's top, the track along `fwd`, a multiple of 90 degrees): a piece
        of floor CHECK_LEN long and as wide as the track with the checkpoint's bowl in its middle, two flame cups
        across the way (at ±7 to the sides, as the original's flames), the stand-in of PC_TwoFlames in the bowl's
        bottom (its z axis across the way, where its flames stand) and the next sector's reset point over it. The
        floors before and after end at the piece; `back` and `front` are their join_points where they run on from
        it. Returns the reset point."""
        f = np.asarray(fwd, np.float64)
        f /= np.linalg.norm(f)
        left = np.array([-f[2], 0.0, f[0]])
        hw, hl = width / 2, CHECK_LEN / 2
        bowl = check_bowl(width)
        # counter-clockwise from the front edge's right corner: the front edge, the left side, the back edge, the
        # right side (the sides cut every 2.5 for the triangles' shape)
        fr_l = sorted(-x for x in (front or [-hw, hw]))
        bk_l = sorted((-x for x in (back or [-hw, hw])), reverse=True)
        cuts = max(1, int(round(CHECK_LEN / 2.5)))
        outer = [(hl, lft) for lft in fr_l]
        outer += [(hl - CHECK_LEN * k / cuts, hw) for k in range(1, cuts)]
        outer += [(-hl, lft) for lft in bk_l]
        outer += [(-hl + CHECK_LEN * k / cuts, -hw) for k in range(1, cuts)]
        joins = set()
        if front:
            joins |= set(range(len(fr_l) - 1))
        if back:
            b0 = len(fr_l) + cuts - 1
            joins |= set(range(b0, b0 + len(bk_l) - 1))
        bottom = self.bowl_piece(f"Checkpoint_{n:02d}", at, f, bowl, outer, joins,
                                 [(0.0, -CHECK_FLAMES), (0.0, CHECK_FLAMES)], along)
        c = np.asarray(at, np.float64)
        for sl in (-1.0, 1.0):
            self.lantern(tuple(c + f * (hl - 1.0) + left * sl * (hw + 0.6)), tuple(left * sl))
        self.placeholder(f"PC_TwoFlames_{n:02d}", ["PC_Checkpoints"], K.pose(tuple(bottom), tuple(f)), (-4, 0, -8),
                         (4, 6, 8))
        reset = tuple(bottom + np.array([0.0, CHECK_RESET, 0.0]))
        self.reset(n + 1, reset, tuple(-f))
        return reset

    def finish(self, at, sector, xaxis=(-1, 0, 0)):
        """The finish: the balloon's stand-in (PE_Balloon, group PE_Levelende) with its platform at `at`; its bridge
        of plates reaches back along -X from 9.5 to 21 (the part's measured plates), so the track ends 22 before."""
        i = self.placeholder("PE_Balloon_01", ["PE_Levelende"] + self.sector(sector), K.pose(at, xaxis), (-4, -1, -4),
                             (4, 1, 4))
        self.lifting.append((i, 9.0, 0.0, "PE_Balloon_01"))

    # --- parts -------------------------------------------------------------------------------------------------

    def box(self, n, at, sector, on_floor=True):
        """A wooden box (P_Box): its body is its stand-in's mesh, a cube of 5 standing on `at`."""
        x, y, z = at
        m = K.Mesh("box")
        K.box(m, None, (-2.5, -2.5, -2.5), (2.5, 2.5, 2.5))
        i = self.placeholder(f"P_Box_{n:02d}", ["P_Box"] + self.sector(sector), K.pose((x, y + 2.5, z)), mesh=m)
        if on_floor:
            self.standing.append((i, 2.5, 2.5, f"P_Box_{n:02d}"))
        self.boxes.append((i, f"P_Box_{n:02d}"))
        return i

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
        i = self.placeholder(f"P_Dome_{n:02d}", ["P_Dome"] + self.sector(sector), K.pose(at), mesh=m)
        self.standing.append((i, 4.7, 0.0, f"P_Dome_{n:02d}"))
        return i

    def loose_ball(self, kind, n, at, sector):
        """A loose ball (P_Ball_Wood, _Stone, _Paper) lying on `at`; the wooden and stone ones are spheres of radius
        2 to the physics, the paper one the hull of its stand-in, a round ball of the same size."""
        x, y, z = at
        m = K.Mesh("ball")
        K.sphere(m, None, (0, 0, 0), 2.0, seg=12)
        name = f"P_Ball_{kind}"
        i = self.placeholder(f"{name}_{n:02d}", [name] + self.sector(sector), K.pose((x, y + 2.0, z)), mesh=m)
        # `at` is a point on the cursor's way, which need not be on the floor there (behind a slope or a bend, at a
        # gutter's side): write() rests the ball on what is under it
        self.loose.append((i, LOOSE_R.get(kind, BALL_R), f"{name}_{n:02d}"))
        return i

    def trafo(self, kind, n, at, sector):
        """A transformer (P_Trafo_Wood, _Stone, _Paper, _Sticky): its pad lies on `at`, on a tile of the floor in the
        ball's own texture, as the original's levels have it (their floor under a transformer is a 5 x 5 tile of
        material Floor_Trafo_<Kind>, Ball_<Kind>'s texture): the part is a ring, and what shows through it is the
        floor (trafo_tile)."""
        x, y, z = at
        name = f"P_Trafo_{kind}"
        i = self.placeholder(f"{name}_{n:02d}", [name] + self.sector(sector), K.pose((x, y + 0.1, z)),
                             (-2.5, -0.1, -2.5), (2.5, 2.4, 2.5))
        self.standing.append((i, 2.47, 0.1, f"{name}_{n:02d}"))
        self.trafo_tile(kind, n, at)
        return i

    def trafo_tile(self, kind, n, at):
        """The tile in the ball's texture under a transformer: a square of 5 a hair over the floor's top, only drawn
        (the floor under it is the physics'). Wood, stone and paper name the game's Ball_<Kind> texture (`game:`), so
        a texture pack or the original's look draws the original's, as on its levels; the sticky ball is ours alone,
        its skin copied into the level."""
        mat = f"Floor_Trafo_{kind}"
        if mat not in self.mat_ids:
            if kind == "Sticky":
                shutil.copyfile(os.path.join(OWN, "3D Entities", "Balls", "textures", "ball_sticky.png"),
                                os.path.join(self.tex, "Ball_Sticky.png"))
                tex = "Ball_Sticky.png"
            else:
                tex = K.GAME + f"Ball_{kind}"
            self.mat(mat, tex, diffuse=1.0, emissive=0.2, specular=0.2, power=40)
        x, y, z = at
        h = y + TRAFO_TILE_LIFT
        m = K.Mesh(f"Trafo_Tile_{kind}_{n:02d}")
        m.face(self.mat_ids[mat], [(x - 2.5, h, z - 2.5), (x - 2.5, h, z + 2.5), (x + 2.5, h, z + 2.5),
                                   (x + 2.5, h, z - 2.5)], uvs=[(0, 0), (0, 1), (1, 1), (1, 0)], normal=(0, 1, 0))
        return self.add(f"Trafo_Tile_{kind}_{n:02d}", m, groups=["Shadow"])

    def extra_point(self, n, at, sector):
        x, y, z = at
        return self.placeholder(f"P_Extra_Point_{n:02d}", ["P_Extra_Point"] + self.sector(sector),
                                K.pose((x, y + 2.5, z)), (-2.5, -2.5, -2.5), (2.5, 2.5, 2.5))

    def extra_life(self, n, at, sector):
        x, y, z = at
        return self.placeholder(f"P_Extra_Life_{n:02d}", ["P_Extra_Life"] + self.sector(sector),
                                K.pose((x, y + 2.56, z)), (-2.5, -2.56, -2.5), (2.5, 1.2, 2.5))

    def module(self, name, n, matrix, sector, lo=(-2, -2, -2), hi=(2, 2, 2), lift=0.6):
        """A module (P_Modul_NN) at `matrix`, the frame its part is made in (a fan's frame `lift` over the floor its
        updraft starts from: 0 for one in its own block, fan_block)."""
        i = self.placeholder(f"{name}_{n:02d}", [name] + self.sector(sector), matrix, lo, hi)
        if name == "P_Modul_18":
            self.lifting.append((i, 1.0, lift, f"{name}_{n:02d}"))
        return i

    # --- writing -----------------------------------------------------------------------------------------------

    def lowest_floor(self):
        """The lowest point of the floors' meshes (Phys_Floors), or None without floors."""
        ys = []
        for i in self.groups.get("Phys_Floors", []):
            node = self.nodes[i]
            if node.get("mesh") is not None and self.meshes[node["mesh"]].p:
                pts = np.asarray(self.meshes[node["mesh"]].p, np.float64)
                ys.append(float((pts @ node["matrix"][:3, :3].T + node["matrix"][:3, 3])[:, 1].min()))
        return min(ys) if ys else None

    def floor_tops(self):
        """The floors' tops (triangles facing up) by cells of 4 across the level, for heights under a point."""
        cells = {}
        tris = []
        for i in self.groups.get("Phys_Floors", []):
            m = self.meshes[self.nodes[i]["mesh"]]
            P = np.asarray(m.p)
            N = np.asarray(m.n)
            for idx in m.parts.values():
                for t in np.reshape(idx, (-1, 3)):
                    q = P[list(t)]
                    nrm = np.cross(q[1] - q[0], q[2] - q[0])
                    # a top: facing up by its normals (its winding is set later, by orient)
                    if np.linalg.norm(nrm) < 1e-9 or N[list(t)][:, 1].mean() < 0.3:
                        continue
                    tris.append(q)
                    lo = np.floor(q[:, [0, 2]].min(0) / 4).astype(int)
                    hi = np.floor(q[:, [0, 2]].max(0) / 4).astype(int)
                    for a in range(lo[0], hi[0] + 1):
                        for b in range(lo[1], hi[1] + 1):
                            cells.setdefault((a, b), []).append(len(tris) - 1)
        return tris, cells

    def check_standing(self):
        """Every part standing on the floors (a transformer's pad, a box, a dome) stands on a flat, level patch at its
        own foot: sampled over its footprint, every floor top under it lies within 0.05 of the foot's height (no
        gutter's slope, kerb, border or a floor's end through it, and nothing missing under it). A level with one
        that does not is refused, and every such part is named."""
        tris, cells = self.floor_tops()

        def heights(x, z):
            out = []
            for k in cells.get((int(np.floor(x / 4)), int(np.floor(z / 4))), []):
                a, b, c = tris[k]
                d = (b[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (b[2] - a[2])
                if abs(d) < 1e-12:
                    continue
                u = ((x - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (z - a[2])) / d
                v = ((b[0] - a[0]) * (z - a[2]) - (x - a[0]) * (b[2] - a[2])) / d
                if u >= -1e-6 and v >= -1e-6 and u + v <= 1 + 1e-6:
                    out.append(a[1] + u * (b[1] - a[1]) + v * (c[1] - a[1]))
            return out

        bad = []
        for i, half, lift, what in self.standing:
            M = self.nodes[i]["matrix"]
            x, y, z = M[0, 3], M[1, 3] - lift, M[2, 3]
            worst = 0.0
            for sx in np.linspace(-half, half, 7):
                for sz in np.linspace(-half, half, 7):
                    hs = [h for h in heights(x + sx, z + sz) if abs(h - y) < 3.0]
                    off = max((abs(h - y) for h in hs), default=9.0)
                    worst = max(worst, off)
            if worst > 0.05:
                bad.append(f"{what} at ({x:.1f}, {y:.1f}, {z:.1f}): the floor under it is {worst:.2f} off its foot"
                           + (" (or missing)" if worst >= 9.0 else ""))
        # headroom: over a transformer the ball is held and lifted (its cage rises 5.5) and over a fan it rises: nothing
        # of the floors or rails may hang in the column over it, HEADROOM high and the pad's width and the ball's wide
        geo = []
        for g in ("Phys_Floors", "Phys_FloorRails"):
            for i in self.groups.get(g, []):
                # (a floor's own railings stand beside the way, not over it)
                if "_Railing" in self.nodes[i]["name"]:
                    continue
                m = self.meshes[self.nodes[i]["mesh"]]
                if m.p:
                    geo.append(np.asarray(m.p))
        geo = np.vstack(geo) if geo else np.zeros((0, 3))
        for i, half, lift, what in self.standing + self.lifting:
            if not (what.startswith("P_Trafo") or what.startswith("P_Modul_18") or what.startswith("PE_")):
                continue
            M = self.nodes[i]["matrix"]
            x, y, z = M[0, 3], M[1, 3] - lift, M[2, 3]
            reach = half + BALL_R + 0.5
            near = geo[(np.abs(geo[:, 0] - x) < reach) & (np.abs(geo[:, 2] - z) < reach)
                       & (geo[:, 1] > y + 0.6) & (geo[:, 1] < y + HEADROOM)]
            if len(near):
                over = float(near[:, 1].min() - y)
                bad.append(f"{what} at ({x:.1f}, {y:.1f}, {z:.1f}): a floor or rails {over:.1f} over "
                           f"it (at least {HEADROOM} clear over it)")
        if bad:
            raise ValueError(f"{len(bad)} parts do not stand on a flat patch of floor with room over it:\n  "
                             + "\n  ".join(bad))

    def solid_triangles(self):
        """The triangles a loose body collides with in the game (its static geometry: floors, rails, the floors'
        stoppers), in world space, as an array (n, 3, 3)."""
        tris = []
        for g in ("Phys_Floors", "Phys_FloorRails", "Phys_FloorStopper"):
            for i in self.groups.get(g, []):
                node = self.nodes[i]
                if node.get("mesh") is None:
                    continue
                m = self.meshes[node["mesh"]]
                if not m.p:
                    continue
                M = node["matrix"]
                P = np.asarray(m.p, np.float64) @ M[:3, :3].T + M[:3, 3]
                for idx in m.parts.values():
                    tris.append(P[np.reshape(idx, (-1, 3))])
        return np.concatenate(tris) if tris else np.zeros((0, 3, 3))

    def rest_loose(self):
        """Rests each loose ball on the floor under it: one put into a floor (a slope or a gutter's side under it, a
        bend's floor beside the cursor's straight line) is lifted out of it, one over the floor is let down onto it;
        one that already touches stays as it is. A ball with nothing under it, or that no lift frees, is refused."""
        if not self.loose:
            return
        tris = self.solid_triangles()
        bad = []
        for i, r, what in self.loose:
            M = self.nodes[i]["matrix"]
            c = M[:3, 3].astype(np.float64)
            near = tris[_tris_near(tris, c, r + max(4 * r, 8.0))]
            depth = _sphere_depth(near, c, r)
            gap = _sphere_gap(near, c, r)
            if depth <= 0.005 and gap <= 0.005:
                continue
            lift = _rest_height(near, c, r)
            if lift is None:
                bad.append(f"{what} at ({c[0]:.1f}, {c[1]:.1f}, {c[2]:.1f}): "
                           + ("no floor under it" if depth <= 0.005 else f"{depth:.2f} deep in a floor"))
                continue
            M[1, 3] = c[1] + lift
        if bad:
            raise ValueError(f"{len(bad)} loose balls cannot be rested on a floor:\n  " + "\n  ".join(bad))

    def check_clear(self):
        """No loose body (a loose ball or a box) starts in the floors, rails or stoppers deeper than SUNK_TOLERANCE:
        the game's physics would shove it out as the run begins, and throw the player's ball touching it far away;
        and every loose ball lies on something. A level with one that does not is refused, and each is named."""
        tris = self.solid_triangles()
        bad = []
        for i, r, what in self.loose:
            c = self.nodes[i]["matrix"][:3, 3].astype(np.float64)
            d = _sphere_depth(tris[_tris_near(tris, c, r)], c, r)
            gap = _sphere_gap(tris[_tris_near(tris, c, r + 0.1)], c, r)
            if d > SUNK_TOLERANCE:
                bad.append(f"{what} at ({c[0]:.1f}, {c[1]:.1f}, {c[2]:.1f}): {d:.2f} deep")
            elif gap > 0.05:
                bad.append(f"{what} at ({c[0]:.1f}, {c[1]:.1f}, {c[2]:.1f}): on nothing (it falls as its sector "
                           "begins)")
            else:
                # resting: on a slope it rolls away as the sector begins (allowed, but said)
                off = _contact_offset(tris[_tris_near(tris, c, r + 0.1)], c)
                if off > 0.1:
                    print(f"note: {what} at ({c[0]:.1f}, {c[1]:.1f}, {c[2]:.1f}) lies on a slope "
                          f"({math.degrees(math.asin(min(1.0, off / r))):.0f} degrees): it rolls away", file=sys.stderr)
        for i, what in self.boxes:
            M = self.nodes[i]["matrix"]
            corners = np.array([[x, y, z] for x in (-2.5, 2.5) for y in (-2.5, 2.5) for z in (-2.5, 2.5)])
            pts = corners @ M[:3, :3].T + M[:3, 3]
            c = M[:3, 3]
            d = _hull_depth(tris[_tris_near(tris, c, 4.4)], pts, M[:3, :3])
            if d > SUNK_TOLERANCE:
                bad.append(f"{what} at ({c[0]:.1f}, {c[1]:.1f}, {c[2]:.1f}): {d:.2f} deep")
        if bad:
            raise ValueError(f"{len(bad)} loose bodies start inside the floors (at most {SUNK_TOLERANCE}):\n  "
                             + "\n  ".join(bad))

    def mark_route(self, first, s0, s1):
        """Gives the floors made since node `first` that have no place on the route yet the stretch s0..s1."""
        phys = set(self.groups.get("Phys_Floors", [])) | set(self.groups.get("Phys_FloorRails", []))
        for i in range(first, len(self.nodes)):
            if i in phys and i not in self.route and "_Railing" not in self.nodes[i]["name"] \
                    and "_Fence" not in self.nodes[i]["name"]:
                self.route[i] = (s0, s1)

    def floor_bounds(self):
        floors = set(self.groups.get("Phys_Floors", []))
        pts = np.array([q for i in floors for q in self.meshes[self.nodes[i]["mesh"]].p]) if floors else None
        if pts is None or not len(pts):
            return None
        return pts.min(0), pts.max(0)

    def cloud_layer(self, cover):
        """The level's cloud layer (SkyLayer, hidden in the file as the original's: the game shows it when the
        options have clouds, drawn added over the sky): two decks under the lowest floor, past the level on every side,
        with our own clouds, grey on black; `cover` how much of the sky they cover (1 a sea of cloud, less scattered
        banks)."""
        bounds = self.floor_bounds()
        if bounds is None or cover <= 0:
            return
        lo, hi = bounds
        x0, z0, x1, z1 = lo[0] - CLOUD_MARGIN, lo[2] - CLOUD_MARGIN, hi[0] + CLOUD_MARGIN, hi[2] + CLOUD_MARGIN
        size = 256
        wisps = K.value_noise(size, size, (4, 4), self.seed + 9, octaves=6)
        streak = K.value_noise(size, size, (2, 8), self.seed + 10, octaves=4)
        cut = 0.47 + (1.0 - cover) * 0.16
        v = np.clip((wisps * 0.7 + streak * 0.3 - cut) * 3.2, 0, 1) ** 1.6 * 0.72
        K.save_png(self.tex, "SkyLayer.png", np.repeat(v[..., None], 3, axis=2))
        mat = self.mat("SkyLayer", "SkyLayer.png", diffuse=1.0, emissive=1.0)
        m = K.Mesh("SkyLayer")
        # a nearer deck and a farther one, the farther larger and turned, so the banks drift over each other
        for below, tile, shift in ((CLOUD_BELOW * 0.7, CLOUD_TILE, 0.0), (CLOUD_BELOW * 1.4, CLOUD_TILE * 1.6, 0.37)):
            y = lo[1] - below
            q = [(x0, y, z0), (x0, y, z1), (x1, y, z1), (x1, y, z0)]
            uv = [((z - z0) / tile + shift, (x - x0) / tile + shift) for x, _, z in q]
            m.face(mat, q, uv, normal=(0, 1, 0))
        self.add("SkyLayer", m, visible=False)

    def fog(self, sky):
        """The level's fog (settings `fog`): the colour of the sky's haze just under its horizon (the side faces'
        rows from 52 to 60 of a hundred down) and of the world straight below (the down face's middle), FOG_DENSITY,
        thickening under the lowest floor (FOG_BELOW, FOG_DEPTH)."""
        from PIL import Image
        rows = []
        for face in ("Front", "Right", "Back", "Left"):
            for ext in ("jpg", "png"):
                f = os.path.join(SKY_DIR, f"Sky_{sky}_{face}.{ext}")
                if os.path.isfile(f):
                    a = np.asarray(Image.open(f).convert("RGB"), np.float64) / 255.0
                    h = a.shape[0]
                    rows.append(a[int(h * 0.52):int(h * 0.60)].reshape(-1, 3))
                    break
        bounds = self.floor_bounds()
        if not rows or bounds is None:
            return None
        colour = np.concatenate(rows).mean(0)
        # seen straight down: the middle of the down face (the world below, or the sea of cloud)
        down = colour
        for ext in ("jpg", "png"):
            f = os.path.join(SKY_DIR, f"Sky_{sky}_Down.{ext}")
            if os.path.isfile(f):
                a = np.asarray(Image.open(f).convert("RGB"), np.float64) / 255.0
                h, w = a.shape[:2]
                down = a[h // 4:3 * h // 4, w // 4:3 * w // 4].reshape(-1, 3).mean(0)
                break
        return {"color": [round(float(c), 4) for c in colour], "color_down": [round(float(c), 4) for c in down],
                "density": FOG_DENSITY,
                "below": round(float(bounds[0][1]) - FOG_BELOW, 2), "depth": FOG_DEPTH}

    def write(self, title, author, sky, start_ball, music, light=(1.0, 1.0, 1.0), bonus=100, licence=None,
              extra=None, clouds=None, sector_balls=None):
        """Writes the level and its level.txt (with `extra` keys besides title, author, version, licence). `clouds`:
        the level's own choices for its sea of cloud (cloud_sea's cover, below, thickness, scale; cover 0 none)."""
        if self.narrow:
            raise ValueError(f"{len(self.narrow)} places leave the ball less than {MIN_FREE} across at the height of "
                             "its middle:\n  " + "\n  ".join(self.narrow))
        self.rest_loose()
        self.check_standing()
        self.check_clear()
        import levelcheck
        found = levelcheck.check_all(self)
        if found:
            raise ValueError("\n  ".join([f"{len(found)} geometry checks fail:"] + found))
        lowest = self.lowest_floor()
        sea = cloud_sea(sky, lowest, **(clouds or {})) if lowest is not None else None
        # the flat cloud layer only where no sea of cloud is drawn as a volume
        if sea is None:
            self.cloud_layer(CLOUD_COVER.get(sky, CLOUD_SCATTERED))
        fog = self.fog(sky)
        for m in self.meshes:
            orient(m)
        groups = sorted(self.groups.items())
        for mt in self.materials:
            if mt["texture"]:
                kind = original_of(mt["texture"])
                if kind[0] == "original":
                    mt["original"] = {"texture": kind[1], **({"texgen": kind[2]} if kind[2] else {})}
                elif kind[0] == "own":
                    mt["own_only"] = True
        K.write_open(self.out, self.meshes, self.nodes, self.materials, groups, sky=sky, start_ball=start_ball,
                     settings={"music": music, "light": list(light), "level_bonus": bonus, "tutorial": False,
                               "last_level": False, "parts": "own", **({"fog": fog} if fog else {}),
                               **({"clouds": sea} if sea else {}),
                               **({"sector_balls": list(sector_balls)} if sector_balls and any(sector_balls) else {})},
                     generator=f"drahtseilakt levelkit.py {KIT_VERSION}", scripts=self.scripts)
        with open(os.path.join(self.out, "level.txt"), "w") as f:
            f.write(f"title = {title}\nauthor = {author}\nversion = 1\n")
            if licence:
                f.write(f"licence = {licence}\n")
            for k, v in (extra or {}).items():
                f.write(f"{k} = {v}\n")


# --- checking a level already made ---------------------------------------------------------------------------------


class _Made:
    """A level folder already written (level.gltf and level.bin), read back into the builder's shapes (nodes with
    their matrices and meshes, meshes with their points and triangles, the groups) and its loose bodies, for
    check_clear."""

    def __init__(self, folder):
        import json

        with open(os.path.join(folder, "level.gltf")) as f:
            doc = json.load(f)
        with open(os.path.join(folder, "level.bin"), "rb") as f:
            blob = f.read()

        def accessor(i):
            a = doc["accessors"][i]
            v = doc["bufferViews"][a["bufferView"]]
            width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3}[a["type"]]
            kind = {5126: np.float32, 5125: np.uint32, 5123: np.uint16}[a["componentType"]]
            arr = np.frombuffer(blob, kind, a["count"] * width, v.get("byteOffset", 0) + a.get("byteOffset", 0))
            return arr.reshape(-1, width) if width > 1 else arr

        mirror = np.diag([1.0, 1.0, -1.0, 1.0])
        self.meshes = []
        for m in doc["meshes"]:
            mesh = K.Mesh(m["name"])
            pts, parts = [], {}
            for k, prim in enumerate(m["primitives"]):
                p = accessor(prim["attributes"]["POSITION"]).astype(np.float64) * (1.0, 1.0, -1.0)
                base = len(pts)
                pts.extend(p.tolist())
                tri = np.asarray(accessor(prim["indices"]), np.int64).reshape(-1, 3)[:, [0, 2, 1]] + base
                parts[k] = tri.flatten().tolist()
            mesh.p, mesh.parts = pts, parts
            self.meshes.append(mesh)
        self.nodes = []
        for nd in doc["nodes"]:
            mg = np.asarray(nd["matrix"], np.float64).reshape(4, 4).T
            self.nodes.append({"name": nd["name"], "mesh": nd.get("mesh"), "matrix": mirror @ mg @ mirror})
        scene = doc["scenes"][0]["extras"]["drahtseilakt"]
        self.groups = {g["name"]: list(g["members"]) for g in scene["groups"]}
        self.loose, self.boxes = [], []
        for i, nd in enumerate(self.nodes):
            parts = nd["name"].split("_")
            if nd["name"].startswith("P_Ball_") and len(parts) >= 3 and parts[2] in LOOSE_R:
                self.loose.append((i, LOOSE_R[parts[2]], nd["name"]))
            elif nd["name"].startswith("P_Box_"):
                self.boxes.append((i, nd["name"]))

    solid_triangles = LevelBuilder.solid_triangles
    check_clear = LevelBuilder.check_clear


def main(argv):
    """`levelkit.py --check FOLDER...`: check_clear on levels already made (their loose bodies against their
    floors); 1 when one is refused."""
    if len(argv) < 3 or argv[1] != "--check":
        print("usage: levelkit.py --check LEVEL_FOLDER...", file=sys.stderr)
        return 2
    status = 0
    for folder in argv[2:]:
        try:
            made = _Made(folder)
            made.check_clear()
            print(f"{folder}: {len(made.loose)} loose balls and {len(made.boxes)} boxes clear of the floors")
        except ValueError as e:
            print(f"{folder}: {e}", file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv))
