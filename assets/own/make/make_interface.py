#!/usr/bin/env python3
"""Draws the own interface's textures (docs/drahtseilakt/resources.md, "own"), from nothing but this script and the
shipped Tektur font: the buttons (Button01_select / Button01_deselect) and the special parts (Button01_special: the
highscore places, the disabled level button, the life ball and its frame, the points box, the end screen's bar, the
black band). Each part sits where the game's code takes it from the texture (the UV rectangles in
crates/game/src/menu.rs, hud.rs and endscreen.rs), so the own files stand in for the copy's by name.

The look is the original's kind, made our own way from its measured numbers (metric.py scores how close each part
comes): see-through glass, neutral grey or (chosen) dark red; steel tubes 2.6 texels wide, shaded as lit cylinders
with a brushed grain; channels of three tubes with a tinted fill (lilac grey, gold when chosen) curling round a
chrome ball that reflects a sky. Nothing of the original is copied, traced or resampled: every shape is drawn here
from a few measured positions, widths and colours.

Run from anywhere: python3 -I assets/own/make/make_interface.py
Needs Python 3 with numpy and Pillow. The output is deterministic (fixed noise seeds).
"""

import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
OWN = os.path.dirname(HERE)
TEXTURES = os.path.join(OWN, "Textures")
FONT = os.path.join(OWN, "..", "fonts", "Tektur-Variable.ttf")

SIZE = 256  # the button textures' side
SS = 4  # supersampling: shapes are drawn at SS times the size and averaged down


# --- a small RGBA canvas with signed-distance shapes -----------------------------------------------------------


class Canvas:
    """Premultiplied RGBA in floats at SS times the texture's size; coordinates are texture pixels."""

    def __init__(self, w, h):
        self.w, self.h = w, h
        self.rgba = np.zeros((h * SS, w * SS, 4), np.float32)
        ys, xs = np.mgrid[0 : h * SS, 0 : w * SS].astype(np.float32)
        self.x = (xs + 0.5) / SS
        self.y = (ys + 0.5) / SS

    def over(self, color, alpha):
        """Lays `color` (H×W×3 or a triple, 0..1) with coverage `alpha` (H×W, 0..1) over the canvas."""
        a = np.clip(alpha, 0.0, 1.0)[..., None]
        c = np.broadcast_to(np.asarray(color, np.float32), self.rgba.shape[:2] + (3,))
        src = np.concatenate([c * a, a], axis=-1)
        self.rgba = src + self.rgba * (1.0 - a)

    def clear(self, mask):
        self.rgba[mask] = 0.0

    def image(self):
        h, w = self.h, self.w
        small = self.rgba.reshape(h, SS, w, SS, 4).mean(axis=(1, 3))
        a = small[..., 3:4]
        rgb = np.where(a > 1e-6, small[..., :3] / np.maximum(a, 1e-6), 0.0)
        out = np.concatenate([rgb, a], axis=-1)
        return Image.fromarray(np.clip(out * 255.0 + 0.5, 0, 255).astype(np.uint8), "RGBA")


def rounded_box(c, x0, y0, x1, y1, r):
    """Signed distance (texture pixels) to a rounded rectangle."""
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    hx, hy = (x1 - x0) / 2 - r, (y1 - y0) / 2 - r
    qx = np.abs(c.x - cx) - hx
    qy = np.abs(c.y - cy) - hy
    outside = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0))
    inside = np.minimum(np.maximum(qx, qy), 0)
    return outside + inside - r


def circle(c, cx, cy, r):
    return np.hypot(c.x - cx, c.y - cy) - r


def inside(d):
    return (d <= 0).astype(np.float32)


def mix(a, b, t):
    t = np.clip(t, 0.0, 1.0)[..., None]
    return np.asarray(a, np.float32) * (1 - t) + np.asarray(b, np.float32) * t


def rgb(*v):
    return tuple(x / 255.0 for x in v)


# --- materials: steel tubes, chrome balls, tinted glass ----------------------------------------------------------

# The light comes from the upper left, in front (texture y grows downwards); the view looks straight in.
LIGHT = np.array([-0.35, -0.8, 0.55]) / np.linalg.norm([-0.35, -0.8, 0.55])
HALF = (LIGHT + np.array([0.0, 0.0, 1.0])) / np.linalg.norm(LIGHT + np.array([0.0, 0.0, 1.0]))

# Measured on the original (its numbers only): steel lines about 2.6 texels wide, bluish grey (a lit side near
# (195, 195, 200), the middle near (115, 115, 125), the shaded side near (40, 40, 45)); the glass flat and see-through,
# neutral grey (39, 39, 39) at 0.6 when not chosen, a dark red (104, 40, 40) at 0.36 when chosen; the channels' fill
# lilac grey (105, 100, 134) at 0.7, warm gold (244, 157, 64) at 0.64 when chosen; the small rows' glass when chosen
# (117, 51, 42) at 0.67.
STEEL = np.array(rgb(124, 126, 138))
LINE = 2.6

LOOKS = {
    "deselect": dict(glass=(rgb(39, 39, 39), 0.60), row=(rgb(39, 39, 39), 0.60), fill=(rgb(105, 100, 134), 0.70),
                     ball="steel", yes=(rgb(150, 142, 191), 0.60), dot=rgb(40, 200, 60)),
    "select": dict(glass=(rgb(104, 40, 40), 0.36), row=(rgb(117, 51, 42), 0.67), fill=(rgb(244, 157, 64), 0.64),
                   ball="gold", yes=(rgb(244, 157, 64), 0.36), dot=rgb(40, 200, 60)),
    "special": dict(glass=(rgb(39, 39, 39), 0.60), row=(rgb(39, 39, 39), 0.60), fill=(rgb(105, 100, 134), 0.70),
                    ball="steel", yes=(rgb(150, 142, 191), 0.60), dot=rgb(200, 30, 30)),
}


def unit_gradient(c, d):
    gy, gx = np.gradient(d, 1.0 / SS)
    n = np.hypot(gx, gy) + 1e-6
    return gx / n, gy / n


GRAIN = {}


def blur(a, sigma):
    """A separable Gaussian blur of a 2D array (edges wrap)."""
    r = int(3 * sigma) + 1
    k = np.exp(-0.5 * (np.arange(-r, r + 1) / sigma) ** 2)
    k /= k.sum()
    out = sum(np.roll(a, i, axis=0) * w for i, w in zip(range(-r, r + 1), k))
    return sum(np.roll(out, i, axis=1) * w for i, w in zip(range(-r, r + 1), k))


def grain(c, seed):
    """Fine brushed grain, 1 +- 10 %: the steel is not polished smooth."""
    key = (c.rgba.shape, seed)
    if key not in GRAIN:
        rng = np.random.default_rng(seed)
        g = rng.standard_normal(c.rgba.shape[:2]).astype(np.float32)
        g = blur(g, 1.2)
        GRAIN[key] = 1.0 + 0.1 * g / max(float(np.std(g)), 1e-6)
    return GRAIN[key]


def tube(c, axis, width=LINE, color=STEEL, seed=3):
    """A steel tube `width` wide along the line where `axis` (a distance to that line, >= 0) is zero, shaded as a
    lit cylinder: its normal leans out from the axis across the tube."""
    hw = width / 2
    s = np.clip(axis / hw, 0, 1)
    gx, gy = unit_gradient(c, axis)
    nz = np.sqrt(np.clip(1 - s * s, 0, 1))
    nx, ny = gx * s, gy * s
    diffuse = np.clip(nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2], 0, 1)
    spec = np.clip(nx * HALF[0] + ny * HALF[1] + nz * HALF[2], 0, 1) ** 18
    col = np.asarray(color)[None, None, :] * (0.3 + 0.95 * diffuse)[..., None] + (0.55 * spec)[..., None]
    col = col * grain(c, seed)[..., None]
    c.over(np.clip(col, 0, 1), inside(axis - hw))


def outline(c, d, width=LINE, color=STEEL):
    """A tube just inside the edge of the shape `d` (a signed distance)."""
    tube(c, np.abs(d + width / 2), width, color)


def glass(c, d, look):
    col, a = look
    c.over(col, inside(d) * a)


def env(ry, rx, dark=0.55):
    """The chrome's surroundings by the reflected ray (ry -1 up .. 1 down): a pale cloudy sky, a soft dark band at
    the horizon, a bright floor; a little cloud in the sky so the ball does not look painted."""
    sky = 0.9 - 0.15 * np.clip(ry + 1, 0, 1) + 0.06 * np.sin(5.0 * rx + 2.0 * ry)
    floor = 0.98
    t = np.clip((ry + 0.15) / 0.3, 0, 1)
    u = np.clip((ry - 0.25) / 0.4, 0, 1)
    band = sky * (1 - t) + dark * t
    return band * (1 - u) + floor * u


def chrome(c, cx, cy, r, kind="steel", ring=False):
    """A polished ball reflecting its surroundings, steel or gold; with a steel ring round it when it sits in a
    socket."""
    nx, ny = (c.x - cx) / r, (c.y - cy) / r
    nz = np.sqrt(np.clip(1 - nx * nx - ny * ny, 0, 1))
    ry = 2 * nz * ny
    rx = 2 * nz * nx
    ry = 0.75 * ry + 0.3 * rx  # the floor's light comes from the lower right
    # the life ball (the HUD's, seen small) has a deeper horizon in it, as the measured one does
    v = env(ry, rx, 0.3 if kind == "silver" else 0.55)
    if kind == "pearl":
        v = 0.78 + 0.22 * nz
    spec = np.clip(nx * HALF[0] + ny * HALF[1] + nz * HALF[2], 0, 1) ** 60
    tint = {"steel": np.array([0.93, 0.95, 1.0]), "gold": np.array([1.0, 0.78, 0.38]),
            "pearl": np.array([1.0, 0.97, 0.9]), "silver": np.array([0.96, 0.96, 0.97])}[kind]
    col = v[..., None] * tint + spec[..., None] * 0.9
    col = col * (0.7 + 0.3 * nz ** 0.3)[..., None]
    c.over(np.clip(col, 0, 1), inside(circle(c, cx, cy, r)))
    if ring:
        tube(c, np.abs(circle(c, cx, cy, r + 1.4)), 2.2)


def triangle(c, pts):
    """Signed distance to a triangle (negative inside)."""
    (ax, ay), (bx, by), (qx, qy) = pts

    def edge(px, py, ux, uy):
        ex, ey = ux - px, uy - py
        wx, wy = c.x - px, c.y - py
        h = np.clip((wx * ex + wy * ey) / (ex * ex + ey * ey), 0, 1)
        return np.hypot(wx - ex * h, wy - ey * h), ex * wy - ey * wx

    d1, s1 = edge(ax, ay, bx, by)
    d2, s2 = edge(bx, by, qx, qy)
    d3, s3 = edge(qx, qy, ax, ay)
    d = np.minimum(np.minimum(d1, d2), d3)
    ins = ((s1 >= 0) & (s2 >= 0) & (s3 >= 0)) | ((s1 <= 0) & (s2 <= 0) & (s3 <= 0))
    return np.where(ins, -d, d)


def seg(c, x0, y0, x1, y1):
    """Distance to a line segment."""
    ex, ey = x1 - x0, y1 - y0
    wx, wy = c.x - x0, c.y - y0
    h = np.clip((wx * ex + wy * ey) / (ex * ex + ey * ey), 0, 1)
    return np.hypot(wx - ex * h, wy - ey * h)


def arc(c, cx, cy, r, a0, a1):
    """Distance to an arc of a circle from angle a0 to a1 (degrees, clockwise on screen from the +x axis)."""
    ang = np.degrees(np.arctan2(c.y - cy, c.x - cx)) % 360
    span = (a1 - a0) % 360
    on = ((ang - a0) % 360) <= span
    p0 = (cx + r * np.cos(np.radians(a0)), cy + r * np.sin(np.radians(a0)))
    p1 = (cx + r * np.cos(np.radians(a1)), cy + r * np.sin(np.radians(a1)))
    ends = np.minimum(np.hypot(c.x - p0[0], c.y - p0[1]), np.hypot(c.x - p1[0], c.y - p1[1]))
    return np.where(on, np.abs(np.hypot(c.x - cx, c.y - cy) - r), ends)


def channel(c, path, half, look, lines=3):
    """A channel along `path` (a distance to its middle line), 2 * half wide: the tinted fill between steel rails
    at its edges and, with three lines, one down its middle."""
    col, a = look
    c.over(col, inside(path - half) * a)
    tube(c, np.abs(path - (half - LINE / 2)))
    if lines == 3:
        tube(c, path, LINE * 0.9)


def pill(c, x0, y0, x1, y1, r=None):
    return rounded_box(c, x0, y0, x1, y1, (y1 - y0) / 2 if r is None else r)


# --- the parts ---------------------------------------------------------------------------------------------------


def c_hook(c, cx, cy, ro, top_end, bottom_end, look):
    """The hook round a button's ball: a channel 12.5 texels wide curling round the left from the top (ending at
    x = top_end) to the bottom (running on to x = bottom_end)."""
    half = 6.25
    rm = ro - half
    path = np.minimum(np.minimum(arc(c, cx, cy, rm, 90, 270), seg(c, cx, cy - rm, top_end, cy - rm)),
                      seg(c, cx, cy + rm, bottom_end, cy + rm))
    channel(c, path, half, look)


def main_button(c, y0, look, kind):
    """MAIN_UV: the main menu's button. Measured: glass (20, 138)-(247, 183), the hook round a ball of radius 11 at
    (33, 160.5) reaching 29 out, its top ending at x 42, its bottom running on under the glass to x 222."""
    d = pill(c, 18, y0 + 7, 247, y0 + 52.5)
    glass(c, d, look["glass"])
    outline(c, d)
    c_hook(c, 33, y0 + 29.5, 29, 40, 220, look["fill"])
    chrome(c, 33, y0 + 29.5, 11, look["ball"])


def field(c, look):
    """FIELD_UV: an option's field. Measured: glass (12, 9)-(250, 55), the hook round a ball at (33, 32) with both
    ends short, a small bracket at the right end."""
    d = pill(c, 14, 9, 250, 55, 17)
    glass(c, d, look["glass"])
    outline(c, d)
    c_hook(c, 33, 32, 29, 40, 55, look["fill"])
    chrome(c, 33, 32, 11, look["ball"])
    channel(c, arc(c, 226, 32, 20, -32, 32), 4.6, look["fill"], lines=2)


def small_row(c, x0, y0, x1, y1, look, ball, dot=True, rr=None):
    """LEVEL_UV, KEY_UV, PLACE_UV: a slim row of glass with a ball in a socket at its left."""
    d = pill(c, x0, y0, x1, y1, rr)
    glass(c, d, look["row"])
    outline(c, d)
    r = (y1 - y0) / 2
    if ball == "socket":
        tube(c, np.abs(circle(c, x0 + r, (y0 + y1) / 2, r - 1.2)), 2.4)
    elif ball:
        chrome(c, x0 + r, (y0 + y1) / 2, r - 2.2, ball, ring=True)
    if dot:
        x, y = x1 - 3.5, y1 - 3.5
        tube(c, np.abs(circle(c, x, y, 2.6)), 1.6)
        c.over(look["dot"], inside(circle(c, x, y, 1.6)))


def arrow(c, x0, y0, x1, y1, pointing, look):
    """A rounded triangle outlined in steel, pointing left or right, a ball in it."""
    m = 4.8
    mid = (y0 + y1) / 2
    if pointing < 0:
        pts = [(x0 + m, mid), (x1 - m, y0 + m), (x1 - m, y1 - m)]
    else:
        pts = [(x1 - m, mid), (x0 + m, y1 - m), (x0 + m, y0 + m)]
    d = triangle(c, pts) - 4.5
    outline(c, d)
    cx = sum(p[0] for p in pts) / 3
    chrome(c, cx + 1.5 * pointing, mid, 8.5, look["ball"])


def back_button(c, look):
    """BACK_UV: Back. Measured: glass (66, 199)-(220, 245), a ball at (88, 222) between two short arcs of channel,
    a bracket round the right end running on under the glass."""
    d = pill(c, 68, 199, 213, 245.5)
    glass(c, d, look["glass"])
    outline(c, d)
    half, cx, cy = 5.6, 88.5, 222.0
    rm = 23 - half
    left = np.minimum(arc(c, cx, cy, rm, 200, 265), arc(c, cx, cy, rm, 95, 160))
    left = np.minimum(left, np.minimum(seg(c, cx + 0.5, cy - rm, 94, cy - rm), seg(c, cx + 0.5, cy + rm, 94, cy + rm)))
    channel(c, left, half, look["fill"])
    rx = 199.5
    right = np.minimum(arc(c, rx, cy, rm, -75, 90), seg(c, 180, cy + rm, rx, cy + rm))
    channel(c, right, half, look["fill"])
    chrome(c, cx, cy, 11, look["ball"])


def buttons(name):
    """Button01_select / Button01_deselect: the parts menu.rs takes (MAIN_UV, LEVEL_UV, BACK_UV, FIELD_UV, KEY_UV,
    YES_NO_UV, the arrows), each inside its rectangle of the texture."""
    look = LOOKS[name]
    c = Canvas(SIZE, SIZE)
    field(c, look)
    # LEVEL_UV (0, 63)-(165, 94), measured (5, 66)-(160, 91)
    small_row(c, 5.5, 66.5, 159.5, 90.5, look, look["ball"], rr=11)
    # ARROW_LEFT_UV (164, 63)-(201, 103) and ARROW_RIGHT_UV (203, 63)-(241, 103)
    arrow(c, 165, 65, 200, 103, -1, look)
    arrow(c, 204, 65, 239, 103, 1, look)
    # KEY_UV (0, 104)-(256, 130): a key's row, three short rails where the key's name begins
    small_row(c, 1.5, 105.5, 254.5, 128.5, look, look["ball"], dot=False)
    for y in (110.5, 117, 123.5):
        tube(c, seg(c, 127, y, 141, y), 1.8)
    main_button(c, 131, look, name)
    back_button(c, look)
    # YES_NO_UV (0, 211)-(58, 233), measured (0, 212)-(57, 232): a small tinted plate
    d = rounded_box(c, 1, 212.5, 56.5, 231.5, 7)
    glass(c, d, look["yes"])
    outline(c, d, 2.2)
    # the - and + of the volume rows, small balls at (236, 210) and (236, 238)
    for y, plus in ((210.5, False), (238, True)):
        chrome(c, 236.5, y, 7.5, look["ball"])
        mark = rounded_box(c, 232, y - 0.8, 241, y + 0.8, 0.5)
        if plus:
            mark = np.minimum(mark, rounded_box(c, 235.7, y - 4.5, 237.3, y + 4.5, 0.5))
        c.over((0.12, 0.12, 0.14), inside(mark))
    return c.image()


def special():
    """Button01_special: the parts hud.rs, endscreen.rs and menu.rs take from it."""
    look = LOOKS["special"]
    c = Canvas(SIZE, SIZE)
    # PLACE_UV (0, 16)-(256, 43), measured (1, 18)-(255, 42): a highscore place, a pearl in its socket
    small_row(c, 1.5, 18.5, 254.5, 41.5, look, None, dot=False)
    chrome(c, 13, 30, 9.5, "pearl")
    tube(c, np.abs(circle(c, 13, 30, 10.8)), 2.4)
    # LEVEL_UV: a level not open yet, an empty socket and a red light
    small_row(c, 5.5, 66.5, 159.5, 90.5, look, "socket", rr=11)
    # the life ball (17, 135)-(45, 164), measured (19, 137)-(45, 163)
    chrome(c, 31.5, 150, 12.6, "silver")
    # the end piece (1, 134)-(15, 164): a short bent tube, a channel's end
    channel(c, arc(c, 25, 150, 18, 145, 215), 4.2, (rgb(0, 0, 0), 0.0), lines=2)
    # the frame round the first life (47, 119)-(104, 180), measured (49, 121)-(104, 179): a channel open to the left,
    # its top ending in a curl, its bottom running out left
    half = 6.0
    path = np.minimum(arc(c, 75, 149.5, 21.5, -90, 90), seg(c, 56, 171, 75, 171))
    path = np.minimum(path, seg(c, 70, 128, 75, 128))
    channel(c, path, half, (rgb(0, 0, 0), 0.0), lines=2)
    # the points box (83, 187)-(256, 251), measured (84, 188)-(255, 250): a dark display, a channel curling under it
    d = rounded_box(c, 109, 189.5, 238, 225.5, 7)
    c.over((0.0, 0.0, 0.0), inside(d) * 0.4)
    outline(c, d, 2.8, color=np.array(rgb(170, 172, 182)))
    rail = np.minimum(seg(c, 92, 198, 92, 222), arc(c, 110, 222, 18, 90, 180))
    rail = np.minimum(rail, seg(c, 110, 240, 247, 240))
    channel(c, rail, 7.0, (rgb(0, 0, 0), 0.0), lines=2)
    # the end screen's bar (154, 133)-(230, 164), measured (154, 133)-(230, 160): a warm glowing plate, tinted yellow
    # by the game
    bar = rounded_box(c, 155, 133.5, 229.5, 159.5, 2)
    soft = np.clip(-bar / 1.5, 0, 1)
    c.over(mix(rgb(255, 212, 30), rgb(255, 160, 0), (np.abs(c.y - 146.5) / 13) ** 1.5), soft)
    # the black band (the last rows: the menus' dark strip, the end screen's field), at 0.78 as measured
    c.clear(c.y > 252)
    c.over((0, 0, 0), (c.y > 252).astype(np.float32) * 0.78)
    img = c.image()
    # the highscore places' numbers 1 to 10 (u = 0.0627 k, 15 texels wide, the top 15 rows)
    draw_numbers(img)
    return img


def tektur(size, weight):
    f = ImageFont.truetype(FONT, size)
    try:
        f.set_variation_by_axes([100, weight])
    except (OSError, ValueError):
        pass
    return f


def draw_numbers(img):
    big = 4
    layer = Image.new("RGBA", (SIZE * big, 16 * big), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for k in range(10):
        text = str(k + 1)
        size = 13 * big if k < 9 else 10 * big
        f = tektur(size, 800)
        x0 = round(0.0627 * k * SIZE * big)
        w = round(0.0588 * SIZE * big)
        box = d.textbbox((0, 0), text, font=f)
        tx = x0 + (w - (box[2] - box[0])) / 2 - box[0]
        ty = (15 * big - (box[3] - box[1])) / 2 - box[1]
        d.text((tx, ty), text, font=f, fill=(255, 238, 190, 255), stroke_width=big, stroke_fill=(30, 12, 40, 255))
    small = layer.resize((SIZE, 16), Image.LANCZOS)
    img.alpha_composite(small, (0, 0))


def main():
    os.makedirs(TEXTURES, exist_ok=True)
    for name in ("select", "deselect"):
        buttons(name).save(os.path.join(TEXTURES, f"Button01_{name}.tga"), rle=False)
    special().save(os.path.join(TEXTURES, "Button01_special.tga"), rle=False)
    print("written to", TEXTURES)


if __name__ == "__main__":
    sys.exit(main())
