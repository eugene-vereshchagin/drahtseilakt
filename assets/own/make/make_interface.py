#!/usr/bin/env python3
"""Draws the own interface's textures (docs/drahtseilakt/resources.md, "own"), from nothing but this script and the
shipped Tektur font: the buttons (Button01_select / Button01_deselect), the special parts (Button01_special: the
highscore places, the disabled level button, the life ball, the points box, the end screen's bar, the black band)
and the picture behind the menus (Menu_Background). Each part sits where the game's code takes it from the texture
(the UV rectangles in crates/game/src/menu.rs, hud.rs and endscreen.rs), so the own files stand in for the copy's
by name. The look follows the original's style (violet glass panels in a steel tube with a polished ball, warm when
chosen) from its colours and proportions only; nothing of the original is copied, traced or resampled.

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


# --- the parts ---------------------------------------------------------------------------------------------------

# Two moods: chosen (warm rose glass, a gold ball) and not chosen (cool violet glass, a steel ball); the disabled
# level button is grey.
MOODS = {
    "select": dict(
        top=rgb(176, 64, 140), bottom=rgb(92, 22, 84), alpha=0.86, glow=rgb(255, 170, 210),
        rim_light=rgb(255, 236, 200), rim_dark=rgb(150, 96, 60), ball=rgb(236, 190, 84),
    ),
    "deselect": dict(
        top=rgb(92, 62, 150), bottom=rgb(38, 22, 82), alpha=0.80, glow=None,
        rim_light=rgb(228, 232, 244), rim_dark=rgb(86, 90, 112), ball=rgb(196, 202, 216),
    ),
    "disabled": dict(
        top=rgb(66, 62, 78), bottom=rgb(34, 32, 42), alpha=0.78, glow=None,
        rim_light=rgb(170, 170, 176), rim_dark=rgb(70, 70, 78), ball=rgb(120, 120, 128),
    ),
    "place": dict(
        top=rgb(64, 44, 112), bottom=rgb(28, 16, 60), alpha=0.82, glow=None,
        rim_light=rgb(214, 218, 232), rim_dark=rgb(80, 84, 104), ball=rgb(196, 202, 216),
    ),
}


def panel(c, box, r, mood, rim=2.4):
    """A glass panel in a steel tube: the glass's gradient and sheen, a glow along the inside when chosen, the tube
    shaded as lit from above."""
    x0, y0, x1, y1 = box
    d = rounded_box(c, x0, y0, x1, y1, r)
    t = (c.y - y0) / max(y1 - y0, 1)
    glass = mix(mood["top"], mood["bottom"], t)
    c.over(glass, inside(d) * mood["alpha"])
    # a sheen over the upper third
    sheen = inside(rounded_box(c, x0 + rim + 1, y0 + rim + 0.6, x1 - rim - 1, y0 + (y1 - y0) * 0.42, r * 0.7))
    c.over((1, 1, 1), sheen * 0.10 * (1 - t * 2.2).clip(0, 1))
    if mood["glow"] is not None:
        g = np.exp(-np.maximum(-d - rim, 0) / 3.0) * inside(d)
        c.over(mood["glow"], g * 0.55)
    # the tube: a band of width `rim` on the outline, light on top, dark below
    band = (np.abs(d + rim / 2) <= rim / 2).astype(np.float32)
    shade = mix(mood["rim_light"], mood["rim_dark"], t * 0.9 + 0.05)
    c.over(shade, band)
    # a thin dark line outside it, so the panel reads on a light sky
    c.over((0.05, 0.03, 0.10), ((d > 0) & (d < 0.8)).astype(np.float32) * 0.55)


def ball(c, cx, cy, r, color, socket=True):
    """A polished ball (light from the upper left) in a dark socket ring."""
    if socket:
        c.over((0.06, 0.04, 0.12), inside(circle(c, cx, cy, r + 1.6)) * 0.85)
    d = circle(c, cx, cy, r)
    nx, ny = (c.x - cx) / r, (c.y - cy) / r
    nz = np.sqrt(np.clip(1 - nx * nx - ny * ny, 0, 1))
    light = np.array([-0.45, -0.6, 0.66])
    light = light / np.linalg.norm(light)
    lam = np.clip(nx * light[0] + ny * light[1] + nz * light[2], 0, 1)
    base = np.asarray(color, np.float32)
    col = base[None, None, :] * (0.25 + 0.85 * lam[..., None])
    # the sky reflected in the lower half, and the highlight
    col += (0.18 * np.clip(ny, 0, 1) * nz)[..., None] * np.array([0.7, 0.75, 1.0])
    spec = np.clip(nx * light[0] + ny * light[1] + nz * light[2], 0, 1) ** 40
    col += spec[..., None] * 1.1
    c.over(np.clip(col, 0, 1), inside(d))


def stroke(c, d, width, light, dark, cy, half):
    """A tube along the zero line of `d`, `width` wide, shaded from `light` (at cy - half) to `dark`."""
    band = (np.abs(d) <= width / 2).astype(np.float32)
    t = (c.y - (cy - half)) / max(2 * half, 1)
    c.over(mix(light, dark, t), band)


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


def arrow(c, x0, y0, x1, y1, pointing, mood):
    """A triangle pointing left or right in a steel outline, a small ball in it."""
    mx = 2.5
    if pointing < 0:
        pts = [(x0 + mx, (y0 + y1) / 2), (x1 - mx, y0 + mx), (x1 - mx, y1 - mx)]
    else:
        pts = [(x1 - mx, (y0 + y1) / 2), (x0 + mx, y1 - mx), (x0 + mx, y0 + mx)]
    d = triangle(c, pts) - 1.5
    t = (c.y - y0) / (y1 - y0)
    c.over(mix(mood["top"], mood["bottom"], t), inside(d) * mood["alpha"])
    stroke(c, d + 1.2, 2.4, mood["rim_light"], mood["rim_dark"], (y0 + y1) / 2, (y1 - y0) / 2)
    cx = sum(p[0] for p in pts) / 3
    ball(c, cx, (y0 + y1) / 2, 5.5, mood["ball"])


def buttons(name):
    """Button01_select / Button01_deselect: the parts menu.rs takes (MAIN_UV, LEVEL_UV, BACK_UV, FIELD_UV, KEY_UV,
    YES_NO_UV, the arrows), each inside its rectangle of the texture."""
    m = MOODS[name]
    c = Canvas(SIZE, SIZE)
    # FIELD_UV (0, 0)-(256, 63): an option's field, its name above its value
    panel(c, (3, 3, 253, 60), 18, m, rim=2.6)
    ball(c, 24, 31.5, 13, m["ball"])
    # LEVEL_UV (0, 63)-(165, 94): a level's button
    panel(c, (2, 65.5, 161, 92), 13, m, rim=2.0)
    ball(c, 15.5, 78.7, 9.0, m["ball"])
    # ARROW_LEFT_UV (164, 63)-(201, 103) and ARROW_RIGHT_UV (203, 63)-(241, 103)
    arrow(c, 166, 65, 201, 102, -1, m)
    arrow(c, 204, 65, 240, 102, 1, m)
    # KEY_UV (0, 104)-(256, 130): a key's row, a divider where the key's name begins
    panel(c, (2, 106, 254, 129), 11.5, m, rim=1.8)
    ball(c, 13.5, 117.5, 7.6, m["ball"])
    c.over(m["rim_light"], inside(rounded_box(c, 131.3, 109.5, 132.7, 125.5, 0.6)) * 0.55)
    # MAIN_UV (0, 131)-(256, 191): a main menu's button
    panel(c, (3, 134.5, 253, 188), 26, m, rim=2.8)
    ball(c, 29.5, 161.2, 17.5, m["ball"])
    # BACK_UV (61, 193)-(222, 252): Back
    panel(c, (64, 196, 219, 249), 26, m, rim=2.8)
    ball(c, 89, 222.5, 16.5, m["ball"])
    # YES_NO_UV (0, 211)-(58, 233): yes and no
    panel(c, (2, 212.5, 56, 231.5), 9.5, m, rim=1.8)
    return c.image()


def special():
    """Button01_special: the parts hud.rs, endscreen.rs and menu.rs take from it."""
    c = Canvas(SIZE, SIZE)
    # PLACE_UV (0, 16)-(256, 43): a highscore place, its number over the socket at its left
    panel(c, (2, 18, 254, 41.5), 11.5, MOODS["place"], rim=1.8)
    c.over((0.06, 0.04, 0.12), inside(circle(c, 13.6, 29.7, 9.6)) * 0.9)
    stroke(c, circle(c, 13.6, 29.7, 9.6), 1.6, rgb(230, 232, 244), rgb(90, 94, 116), 29.7, 9.6)
    # LEVEL_UV (0, 63)-(165, 94): a level not open yet
    panel(c, (2, 65.5, 161, 92), 13, MOODS["disabled"], rim=2.0)
    ball(c, 15.5, 78.7, 9.0, MOODS["disabled"]["ball"])
    # the life ball (17, 135)-(45, 164) and the end piece (1, 134)-(15, 164): polished steel with a blue tint
    ball(c, 31.15, 149.6, 13.2, rgb(206, 214, 232), socket=False)
    steel_l, steel_d = rgb(232, 236, 248), rgb(84, 88, 110)
    end = np.hypot(c.x - 15.5, c.y - 149.3) - 11.5
    end = np.where(c.x < 15.5, end, 99)
    stroke(c, end, 3.0, steel_l, steel_d, 149.3, 12)
    # the bracket (47, 120)-(104, 180) round the first life: a tube open to the left, a stem going down
    cx, cy, r = 73.5, 150.0, 18.5
    ring = np.hypot(c.x - cx, c.y - cy) - r
    ring = np.where(c.x >= cx, ring, 99)
    top = np.where((c.x >= 49) & (c.x < cx), np.abs(c.y - (cy - r)), 99)
    bottom = np.where((c.x >= 49) & (c.x < cx), np.abs(c.y - (cy + r)), 99)
    tube = np.minimum(np.minimum(np.abs(ring), top), bottom)
    stroke(c, tube, 3.4, steel_l, steel_d, cy, r + 2)
    stem = rounded_box(c, 86, 165, 90.5, 179, 1.5)
    c.over(mix(steel_l, steel_d, (c.y - 165) / 14), inside(stem))
    for x in (49.5, 49.5):
        c.over(steel_l, inside(circle(c, x, cy - r, 2.3)) + inside(circle(c, x, cy + r, 2.3)))
    # the points box (83, 187)-(256, 251): a display for the digits in the upper middle, a tube curling under it
    panel(c, (102, 190, 238, 226.5), 12, MOODS["place"], rim=2.2)
    c.over((0.02, 0.01, 0.05), inside(rounded_box(c, 108, 194.5, 232, 222, 8)) * 0.35)
    rail_y = 240.0
    rail = np.where((c.x >= 101) & (c.x <= 248), np.abs(c.y - rail_y), 99)
    curl = np.hypot(c.x - 101, c.y - (rail_y - 13)) - 13
    curl = np.where((c.x < 101) & (c.y > rail_y - 13), np.abs(curl), 99)
    hook = np.hypot(c.x - 248, c.y - (rail_y - 5)) - 5
    hook = np.where(c.x > 248, np.abs(hook), 99)
    stroke(c, np.minimum(np.minimum(rail, curl), hook), 3.2, steel_l, steel_d, rail_y - 8, 12)
    # the end screen's bar (154, 133)-(230, 164): light, tinted yellow by the game
    bar = rounded_box(c, 157, 137, 227, 160, 6)
    soft = np.clip(-bar / 4.0, 0, 1)
    c.over(mix(rgb(255, 255, 255), rgb(255, 214, 120), np.abs(c.y - 148.5) / 12), soft)
    # the black band (the last rows: the menus' dark strip, the end screen's field)
    c.clear(c.y > 252)
    c.over((0, 0, 0), (c.y > 252).astype(np.float32))
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


# --- the picture behind the menus --------------------------------------------------------------------------------


def noise(shape, scale, seed, octaves=5):
    """Fractal value noise in 0..1, tiling-free, deterministic."""
    rng = np.random.default_rng(seed)
    h, w = shape
    out = np.zeros(shape, np.float32)
    amp, total = 1.0, 0.0
    for o in range(octaves):
        cells = (max(2, int(h / scale * 2**o)) + 2, max(2, int(w / scale * 2**o)) + 2)
        grid = rng.random(cells).astype(np.float32)
        img = Image.fromarray(grid).resize((w, h), Image.BICUBIC)
        out += np.asarray(img) * amp
        total += amp
        amp *= 0.5
    return np.clip(out / total, 0, 1)


def background(w=1600, h=900):
    """Dusk over a sea of clouds: a violet sky warming to the horizon, the low sun's glow, drifting cloud banks, and
    floating rails with a ball on the left and a pillar with a ball on the right (the middle is under the menus)."""
    ys, xs = np.mgrid[0:h, 0:w].astype(np.float32)
    v = ys / h
    horizon = 0.56
    sky = mix(rgb(16, 22, 58), rgb(92, 64, 140), np.clip(v / horizon, 0, 1) ** 1.3)
    sky = mix(sky, rgb(246, 168, 112), np.clip((v - horizon * 0.62) / (horizon * 0.38), 0, 1) ** 2.2)
    # the sun's glow, low and a little right of the middle
    sx, sy = w * 0.56, h * horizon
    dist = np.hypot((xs - sx) / w, (ys - sy) / h * 1.6)
    sky = sky + (np.exp(-dist * 7.0) * 0.55)[..., None] * np.array(rgb(255, 214, 160))
    sky = sky + (np.exp(-dist * 30.0) * 0.5)[..., None] * np.array(rgb(255, 245, 220))
    # high thin clouds
    wisp = noise((h, w), 260, 7)
    wisp = np.clip((wisp - 0.52) * 3.0, 0, 1) * np.clip(1 - v / horizon, 0, 1) * 0.35
    sky = mix(sky, rgb(214, 170, 210), wisp)
    # the cloud sea below the horizon: banks lit warm from the sun, darker towards the viewer
    sea = noise((h, w), 140, 11, octaves=6)
    below = np.clip((v - horizon) / (1 - horizon), 0, 1)
    tops = np.clip((sea - 0.42 + (1 - below) * 0.25) * 2.4, 0, 1) * (v > horizon - 0.02)
    lit = mix(rgb(250, 196, 160), rgb(120, 92, 150), below ** 0.8)
    lit = lit + (np.exp(-dist * 4.0) * 0.25)[..., None]
    sky = mix(sky, lit, tops * np.clip((v - horizon + 0.02) * 30, 0, 1))
    shade = noise((h, w), 60, 13, octaves=4)
    sky = sky * (0.92 + 0.16 * shade[..., None])
    img = Image.fromarray(np.clip(sky * 255, 0, 255).astype(np.uint8), "RGB").convert("RGBA")
    scenery(img)
    return img.convert("RGB")


def scenery(img):
    """The floating parts, drawn at four times the size and scaled down (smooth edges)."""
    w, h = img.size
    k = 2
    layer = Image.new("RGBA", (w * k, h * k), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    # the track on the left: a wooden deck running into the distance between two steel rails, a ball on it
    near_y, far_y = 0.97, 0.60
    def at(t, side):
        """A point of the deck's edge (side -1 left, 1 right, 0 the middle) at depth t (0 near, 1 far)."""
        p = t ** 0.55
        y = near_y + (far_y - near_y) * p
        half = 0.15 * (1 - p) + 0.010 * p
        x = 0.16 + 0.06 * p
        return ((x + side * half) * w * k, y * h * k)
    steps = 60
    for i in range(steps):
        t0, t1 = i / steps, (i + 1) / steps
        lum = 1.0 - 0.45 * t0
        # the deck's side, under its right edge (lit less), then its top
        thick = (1 - t0 ** 0.55) * 0.022 * h * k + 1
        a, b = at(t0, 1), at(t1, 1)
        d.polygon([a, b, (b[0], b[1] + thick * 0.95), (a[0], a[1] + thick)], fill=(int(70 * lum), int(44 * lum),
                  int(28 * lum), 255))
        a, b = at(t0, -1), at(t1, -1)
        d.polygon([a, b, (b[0], b[1] + thick * 0.95), (a[0], a[1] + thick)], fill=(int(96 * lum), int(60 * lum),
                  int(36 * lum), 255))
        top = [at(t0, -1), at(t0, 1), at(t1, 1), at(t1, -1)]
        seam = i % 6 == 5
        f = 0.82 if seam else 1.0
        d.polygon(top, fill=(int(178 * lum * f), int(122 * lum * f), int(72 * lum * f), 255))
    # the planks' long seams
    for side in (-0.6, -0.2, 0.2, 0.6):
        pts = [at(j / 40, side) for j in range(41)]
        for j in range(40):
            d.line([pts[j], pts[j + 1]], fill=(110, 70, 40, 255), width=max(1, int(3 * (1 - j / 40))))
    # the rails, a little above the deck's edges
    for side in (-0.97, 0.97):
        pts = [at(j / 60, side) for j in range(61)]
        for j in range(60):
            wd = max(2, int(14 * (1 - (j / 60) ** 0.55)))
            lift = wd * 0.9
            p0, p1 = (pts[j][0], pts[j][1] - lift), (pts[j + 1][0], pts[j + 1][1] - lift)
            d.line([p0, p1], fill=(212, 216, 230, 255), width=wd)
            d.line([(p0[0], p0[1] + wd * 0.35), (p1[0], p1[1] + wd * 0.35)], fill=(96, 98, 118, 255),
                   width=max(1, wd // 3))
    # a wooden ball on the deck, its shadow under it
    t = 0.18
    cx, cy = at(t, 0.1)
    p = t ** 0.55
    r = (0.15 * (1 - p) + 0.010 * p) * w * k * 0.42
    d.ellipse([cx - r * 0.9, cy - r * 0.18, cx + r * 0.9, cy + r * 0.18], fill=(60, 36, 20, 140))
    ball_on(layer, (cx, cy - r * 0.96), r, (172, 108, 56))
    # the pillar on the right with a stone ball on it
    px, top, bottom, pw = 0.84 * w * k, 0.50 * h * k, 1.02 * h * k, 0.055 * w * k
    for i in range(int(pw)):
        t = i / pw
        lum = 0.35 + 0.65 * np.clip(np.cos((t - 0.35) * np.pi), 0, 1)
        col = tuple(int(c * lum) for c in (186, 170, 150))
        d.line([(px - pw / 2 + i, top), (px - pw / 2 + i, bottom)], fill=col + (255,))
    cap = 0.022 * h * k
    d.rounded_rectangle([px - pw * 0.68, top - cap, px + pw * 0.68, top + cap * 0.4], radius=cap * 0.4,
                        fill=(150, 138, 124, 255))
    ball_on(layer, (px, top - cap - 40 * k), 40 * k, (150, 152, 160))
    small = layer.resize((w, h), Image.LANCZOS)
    img.alpha_composite(small)


def ball_on(layer, centre, r, color):
    """A shaded ball pasted into `layer`."""
    size = int(2 * r) + 4
    ys, xs = np.mgrid[0:size, 0:size].astype(np.float32)
    nx, ny = (xs - size / 2) / r, (ys - size / 2) / r
    rr = nx * nx + ny * ny
    nz = np.sqrt(np.clip(1 - rr, 0, 1))
    light = np.array([0.35, -0.55, 0.76])
    light /= np.linalg.norm(light)
    lam = np.clip(nx * light[0] + ny * light[1] + nz * light[2], 0, 1)
    base = np.array(color, np.float32) / 255
    col = base * (0.22 + 0.9 * lam[..., None]) + (lam ** 30)[..., None] * 0.7
    col += (np.clip(ny, 0, 1) * 0.25)[..., None] * np.array([1.0, 0.75, 0.6])
    edge = np.clip((1 - np.sqrt(rr)) * r, 0, 1)
    rgba = np.concatenate([np.clip(col, 0, 1), edge[..., None]], axis=-1)
    tile = Image.fromarray((rgba * 255).astype(np.uint8), "RGBA")
    layer.alpha_composite(tile, (int(centre[0] - size / 2), int(centre[1] - size / 2)))


def main():
    os.makedirs(TEXTURES, exist_ok=True)
    for name in ("select", "deselect"):
        buttons(name).save(os.path.join(TEXTURES, f"Button01_{name}.tga"), rle=False)
    special().save(os.path.join(TEXTURES, "Button01_special.tga"), rle=False)
    background().save(os.path.join(TEXTURES, "Menu_Background.png"), optimize=True)
    print("written to", TEXTURES)


if __name__ == "__main__":
    sys.exit(main())
