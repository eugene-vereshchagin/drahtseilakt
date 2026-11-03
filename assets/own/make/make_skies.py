#!/usr/bin/env python3
"""The game's photographic skies: freely licensed (CC0) HDR panoramas from Poly Haven, made into the sky box the game
draws (crates/game/src/sky.rs: six faces round the camera, `Textures/Sky/Sky_<Name>_<Face>.jpg`), each with a world
far below the track that this script draws itself.

The panoramas are photographs taken on the ground, and Ballance's camera looks down a lot: what lies under the horizon
in a photograph (the grass at the photographer's feet, a road, a tripod) would read as a floor a few metres under the
track. So only the sky and the horizon are taken from the photograph; everything below is made here, lit by the same
photograph and seen as from high above: a sea that mirrors the sky with waves, a forest canopy, hazy mountains, a
canyon, a sea of cloud, a valley full of mist. The aerial haze thickens towards the horizon in the photograph's own
horizon colour, so the two meet without a seam.

  python3 -I make_skies.py fetch [DIR]   # download the pinned panoramas once into DIR/skies/<id>/ and check them
  python3 -I make_skies.py check [DIR]   # check the panoramas already there against their checksums
  python3 -I make_skies.py make [DIR] [NAME...]  # make the skies (all, or the named) into Textures/Sky/

DIR defaults to $DRAHTSEILAKT_TEXTURE_SOURCES, else `texture-sources` beside the repository (as free_textures.py).
The panoramas are never shipped; the faces made from them are, and are committed, so the game's build needs neither
the downloads nor the network. A panorama is parsed here with every length checked (a Radiance .hdr is plain data);
nothing in the downloads is run.
"""

import hashlib
import math
import os
import sys
import urllib.request

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import free_textures as ft  # noqa: E402  (the sources' folder)
import ownkit as k  # noqa: E402

OUT = os.path.join(os.path.dirname(HERE), "Textures", "Sky")
POLYHAVEN = {"licence": "CC0 1.0 Universal", "licence_url": "https://polyhaven.com/license"}

# id: (the 4K Radiance file's SHA-256, its size, its authors, published). The file is
# https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/4k/<id>_4k.hdr, its page https://polyhaven.com/a/<id> (CC0, checked
# on each page 2026-11-03).
SOURCES = {
    "table_mountain_1_puresky": ("1b1c70d89fdd257b242c6790d1a3e92bcaa4e72ea5f6644641df29e9efbb8617", 19336378,
                                 "Greg Zaal, Jarod Guest", "2022-12-03"),
    "kloofendal_48d_partly_cloudy_puresky": ("3061c00a16ecae748e84fd6c44c04804f539c118bb66325db858bad87b11bf88",
                                             20674857, "Greg Zaal, Jarod Guest", "2022-11-19"),
    "kloppenheim_06_puresky": ("1bebd7cbbfb5c48a642f03d93bffbabb2c42d0b109061f37a72259d1cbdb3053", 17385661,
                               "Greg Zaal, Jarod Guest", "2022-11-26"),
    "kloppenheim_07_puresky": ("98a94dc8c290a89de0147642750beb9dae4c791ee33c1689bebbf58126e97f22", 19356904,
                               "Greg Zaal, Jarod Guest", "2022-12-18"),
    "rocky_ridge_puresky": ("82efe60eea01151530c16f841270bfbdb5dd5eafdc3f285fb3cc368d8c0a7501", 18336135,
                            "Greg Zaal, Jarod Guest", "2022-12-04"),
    "rogland_clear_night": ("133d77ddf6a69926a138574169e5cdf4c04b785e8d22545274aa285f1ec3d879", 27915523,
                            "Greg Zaal", "2024-10-08"),
    "qwantani_late_afternoon_puresky": ("d820fa7a84732f69641655dcbea33b01ff78e6a4833ee3ca6f3bc938a0c09bb7", 17745266,
                                        "Greg Zaal, Jarod Guest", "2025-07-18"),
    "kloppenheim_01_puresky": ("e667700214b1141dd0890c6e45decf9c84964b3c6596b717a48c56e3d893bde3", 12968721,
                               "Greg Zaal, Jarod Guest", "2022-11-20"),
}
MAX_FILE = 48 << 20

# The skies by the name a level gives (`sky Mountains`): the panorama, its exposure (stops over the one that sets the
# sky's median luminance from 5 to 30 degrees to a middle grey), how much of the photograph's horizon is kept and the
# world drawn below it.
#   cut: the lowest elevation (degrees) of the photograph kept; reach: how far it is stretched down (a band of distant
#        land a few degrees deep, seen from higher up, fills more of the view below the horizon)
#   below: "sea" (water mirroring the sky), "land" (lit relief with the kind's colours), "clouds" (a sea of cloud),
#          "haze" (only the haze: a level under this sky has its sea of cloud drawn by the game, clouds.wgsl)
#   grade: a golden-hour grade, (colour at the horizon, colour from 40 degrees up[, colour at the horizon away from
#          the sun]) multiplying the photograph and the world below, blended by elevation (and round from the sun)
#   lift: (gain, degrees, amount) a dusty band over the horizon cleared: towards the sky's colour above it (9 to 15
#         degrees up, smoothed round the circle) times gain, `amount` of it at the horizon, none `degrees` up
#   haze: the aerial haze's thickness (per altitude of the camera over the world below; 0.1 is clear air)
SKIES = {
    "Mountains": dict(src="table_mountain_1_puresky", ev=0.0, below="land", kind="mountains", haze=0.10, seed=101),
    "Forest": dict(src="kloofendal_48d_partly_cloudy_puresky", ev=-0.2, below="land", kind="forest", haze=0.13,
                   seed=202),
    "Sea": dict(src="kloppenheim_06_puresky", ev=0.0, below="sea", waves=0.10, water=(0.012, 0.030, 0.040),
                haze=0.06, seed=303),
    "Storm": dict(src="kloppenheim_07_puresky", ev=-0.5, below="sea", waves=0.08, wave_tile=2.0, wave_cells=24, water=(0.014, 0.022, 0.022),
                  foam=0.6, haze=0.12, tint=(0.86, 0.92, 1.0), seed=404),
    "Sunset": dict(src="rocky_ridge_puresky", ev=0.3, below="haze", haze=0.10, cut=1.0, lift=(1.1, 12.0, 0.9),
                   grade=((1.45, 1.02, 0.68), (0.97, 1.0, 1.06), (1.08, 0.97, 1.05)), seed=505),
    "Night": dict(src="rogland_clear_night", ev=-0.6, below="sea", waves=0.035, water=(0.004, 0.006, 0.010),
                  haze=0.05, seed=606),
    "Desert": dict(src="qwantani_late_afternoon_puresky", ev=-0.2, below="land", kind="canyon", haze=0.09, seed=707),
    "Mist": dict(src="kloppenheim_01_puresky", ev=0.1, below="land", kind="mist", haze=0.10, key=1.2, seed=808),
    "Clouds": dict(src="kloofendal_48d_partly_cloudy_puresky", ev=-0.2, below="haze", haze=0.10, cut=1.0,
                   lift=(1.2, 10.0, 0.85), seed=909),
}
# What the skies cast on the game's sea of cloud (levelkit.py makes a level's `clouds` settings from it): written by
# `make` beside this script, the sun's direction (towards it, the game's axes) and colours in the faces' exposure.
SKY_LIGHT = os.path.join(HERE, "sky_light.json")

FACE = 1024  # the faces' size as shipped
SS = 2  # drawn at twice that and averaged down
JPEG_QUALITY = 88

# The six faces as the game draws them (sky.rs): corners on the unit cube from the bottom left, the texture's rows from
# the top; the texture coordinates are inset by 0.002 there, so the corners fall on these points.
FACES = {
    "Front": [(-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)],
    "Right": [(1, -1, 1), (1, -1, -1), (1, 1, -1), (1, 1, 1)],
    "Back": [(1, -1, -1), (-1, -1, -1), (-1, 1, -1), (1, 1, -1)],
    "Left": [(-1, -1, -1), (-1, -1, 1), (-1, 1, 1), (-1, 1, -1)],
    "Down": [(-1, -1, -1), (1, -1, -1), (1, -1, 1), (-1, -1, 1)],
    "Top": [(-1, 1, 1), (1, 1, 1), (1, 1, -1), (-1, 1, -1)],
}
UV_INSET = 0.002


# --- the sources -----------------------------------------------------------------------------------------------------


def source_path(sid, root=None):
    return os.path.join(ft.sources_dir(root), "skies", sid, f"{sid}_4k.hdr")


def verified(sid, root=None):
    path = source_path(sid, root)
    if not os.path.exists(path):
        raise SystemExit(f"{path} is missing: python3 -I {os.path.join(HERE, 'make_skies.py')} fetch")
    got = ft.sha256(path)
    if got != SOURCES[sid][0]:
        raise SystemExit(f"{path}: SHA-256 {got}, expected {SOURCES[sid][0]}; refusing it")
    return path


def fetch(root=None):
    for sid, (digest, size, _a, _d) in SOURCES.items():
        path = source_path(sid, root)
        if os.path.exists(path) and ft.sha256(path) == digest:
            continue
        os.makedirs(os.path.dirname(path), exist_ok=True)
        url = f"https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/4k/{sid}_4k.hdr"
        print("fetching", url)
        with urllib.request.urlopen(url, timeout=300) as r:
            data = r.read(MAX_FILE + 1)
        if len(data) > MAX_FILE or hashlib.sha256(data).hexdigest() != digest:
            raise SystemExit(f"{url}: not the pinned file (size {len(data)}, expected {size}); refusing it")
        with open(path, "wb") as f:
            f.write(data)
    check(root)


def check(root=None):
    for sid in SOURCES:
        print(sid, "ok", verified(sid, root))


MAX_SIDE = 16384


def read_hdr(data):
    """A Radiance .hdr (RGBE; run-length or flat scanlines, rows top down) as float32 H x W x 3. Every count and length
    is checked against the data; anything else is refused."""
    end = data.find(b"\n\n")
    if not data.startswith(b"#?") or end < 0 or end > 4096:
        raise ValueError("not a Radiance file")
    if b"FORMAT=32-bit_rle_rgbe" not in data[:end]:
        raise ValueError("not RGBE")
    nl = data.find(b"\n", end + 2)
    res = data[end + 2:nl].split() if 0 < nl - end < 64 else []
    if len(res) != 4 or res[0] != b"-Y" or res[2] != b"+X" or not (res[1].isdigit() and res[3].isdigit()):
        raise ValueError("unsupported resolution line")
    h, w = int(res[1]), int(res[3])
    if not (8 <= w <= MAX_SIDE and 0 < h <= MAX_SIDE):
        raise ValueError("bad size")
    buf = np.frombuffer(data, np.uint8)
    n, pos = len(buf), nl + 1
    out = np.empty((h, w, 4), np.uint8)
    for y in range(h):
        if pos + 4 > n:
            raise ValueError("truncated")
        if buf[pos] == 2 and buf[pos + 1] == 2 and not buf[pos + 2] & 0x80:
            if (int(buf[pos + 2]) << 8 | int(buf[pos + 3])) != w:
                raise ValueError("bad scanline")
            pos += 4
            for c in range(4):
                row, x = out[y, :, c], 0
                while x < w:
                    if pos >= n:
                        raise ValueError("truncated")
                    count = int(buf[pos])
                    if count > 128:
                        count -= 128
                        if x + count > w or pos + 1 >= n:
                            raise ValueError("bad run")
                        row[x:x + count] = buf[pos + 1]
                        pos += 2
                    else:
                        if count == 0 or x + count > w or pos + 1 + count > n:
                            raise ValueError("bad run")
                        row[x:x + count] = buf[pos + 1:pos + 1 + count]
                        pos += 1 + count
                    x += count
        else:
            if pos + 4 * w > n:
                raise ValueError("truncated")
            out[y] = buf[pos:pos + 4 * w].reshape(w, 4)
            pos += 4 * w
    e = out[..., 3].astype(np.int32)
    scale = np.where(e > 0, np.ldexp(1.0, e - 136), 0.0).astype(np.float32)
    return out[..., :3].astype(np.float32) * scale[..., None]


def load(sid, root=None):
    path = verified(sid, root)
    if os.path.getsize(path) > MAX_FILE:
        raise SystemExit(f"{path}: too large")
    with open(path, "rb") as f:
        return read_hdr(f.read())


# --- sampling --------------------------------------------------------------------------------------------------------

LUM = np.array([0.2126, 0.7152, 0.0722])


def elevation(d):
    return np.arcsin(np.clip(d[..., 1], -1, 1))


def azimuth_u(d):
    """0..1 round the horizon (the panorama's columns)."""
    return (np.arctan2(d[..., 0], d[..., 2]) / (2 * np.pi) + 0.5) % 1.0


def bilinear(img, x, y, wrap_y):
    """img (H x W x C) at texel coordinates x, y (texel centres at +0.5), wrapping in x, wrapping or clamped in y."""
    h, w = img.shape[:2]
    x = x - 0.5
    y = y - 0.5
    x0 = np.floor(x).astype(np.int64)
    y0 = np.floor(y).astype(np.int64)
    fx = (x - x0)[..., None]
    fy = (y - y0)[..., None]
    x1 = (x0 + 1) % w
    x0 = x0 % w
    if wrap_y:
        y1 = (y0 + 1) % h
        y0 = y0 % h
    else:
        y1 = np.clip(y0 + 1, 0, h - 1)
        y0 = np.clip(y0, 0, h - 1)
    a = img[y0, x0] * (1 - fx) + img[y0, x1] * fx
    b = img[y1, x0] * (1 - fx) + img[y1, x1] * fx
    return a * (1 - fy) + b * fy


def panorama(pano, u, el):
    """The panorama's radiance at azimuth u (0..1) and elevation el (radians)."""
    h, w = pano.shape[:2]
    return bilinear(pano, u * w, (0.5 - el / np.pi) * h, wrap_y=False)


class Pyramid:
    """A tiling map with its averaged halvings, sampled by footprint (texels of the full map per pixel)."""

    def __init__(self, img):
        self.levels = [img]
        while min(self.levels[-1].shape[:2]) > 8:
            a = self.levels[-1]
            self.levels.append(0.25 * (a[0::2, 0::2] + a[1::2, 0::2] + a[0::2, 1::2] + a[1::2, 1::2]))

    def sample(self, s, t, footprint):
        """At s, t (in tiles, any value: it wraps) for a footprint in full-map texels."""
        lod = np.clip(np.log2(np.maximum(footprint, 1.0)), 0, len(self.levels) - 1.001)
        i0 = np.floor(lod).astype(int)
        f = (lod - i0)[..., None]
        out = np.zeros(s.shape + self.levels[0].shape[2:], np.float64)
        for i in np.unique(i0):
            m = i0 == i
            lv0, lv1 = self.levels[i], self.levels[i + 1]
            a = bilinear(lv0, s[m] * lv0.shape[1], t[m] * lv0.shape[0], wrap_y=True)
            b = bilinear(lv1, s[m] * lv1.shape[1], t[m] * lv1.shape[0], wrap_y=True)
            out[m] = a * (1 - f[m]) + b * f[m]
        return out


# --- light from the photograph ---------------------------------------------------------------------------------------


class Light:
    """What the photograph's sky casts on a level world below: the sun (direction, irradiance) and the rest of the sky
    (irradiance on a level surface), and the haze colour round the horizon."""

    def __init__(self, pano):
        # averaged in blocks of 8 x 8 (not every 8th texel: the sun's disc is a few texels and would be missed)
        hh, ww = pano.shape[0] // 8 * 8, pano.shape[1] // 8 * 8
        small = pano[:hh, :ww].reshape(hh // 8, 8, ww // 8, 8, 3).mean((1, 3), dtype=np.float64)
        h, w = small.shape[:2]
        el = (0.5 - (np.arange(h) + 0.5) / h) * np.pi
        az = ((np.arange(w) + 0.5) / w - 0.5) * 2 * np.pi
        up = el > 0
        lum = small @ LUM
        domega = (np.cos(el) * (np.pi / h) * (2 * np.pi / w))[:, None]
        cos = np.sin(el)[:, None]
        med = np.median(lum[up])
        sun = (lum > 40 * med) & up[:, None]
        dirs = np.stack([np.cos(el)[:, None] * np.sin(az)[None], np.repeat(np.sin(el)[:, None], w, 1),
                         np.cos(el)[:, None] * np.cos(az)[None]], -1)
        wgt = (lum * domega) * sun
        if sun.sum() >= 1 and wgt.sum() > 0:
            s = (dirs * wgt[..., None]).sum((0, 1))
            self.sun_dir = s / np.linalg.norm(s)
            # the sun's irradiance on a surface facing it (clipped in the file perhaps; enough for shading)
            self.sun = (small * (domega * sun)[..., None]).sum((0, 1))
        else:
            # a sun behind cloud: the brightest part of the sky (its brightest few hundredths of a percent) is where
            # it is, and its light what comes from there
            near = up[:, None] & (lum >= np.quantile(lum[up], 0.9995))
            wgt = lum * domega * near
            s = (dirs * wgt[..., None]).sum((0, 1))
            self.sun_dir = s / max(np.linalg.norm(s), 1e-12)
            self.sun = (small * (domega * near)[..., None]).sum((0, 1))
            sun = near
        sky = ~sun & up[:, None]
        self.sky = (small * (domega * cos * sky)[..., None]).sum((0, 1))
        # the horizon's colour by azimuth (just above it), smoothed round the circle: the haze's colour
        self.horizon = self.ring(small, (el > np.radians(0.5)) & (el < np.radians(5.0)))
        # the same higher up, over a dusty band at the horizon (a sky's `lift`)
        self.above = self.ring(small, (el > np.radians(9.0)) & (el < np.radians(15.0)), sigma=40.0)
        # the exposure's reference: the sky's median luminance from 5 to 30 degrees
        mid = (el > np.radians(5)) & (el < np.radians(30))
        self.reference = float(np.median(lum[mid]))

    @staticmethod
    def ring(small, band, sigma=8.0):
        """The colour of a band of elevations by azimuth, smoothed round the circle."""
        w = small.shape[1]
        hz = small[band].mean(0)
        # the sun on the horizon is no haze: a column far brighter than the horizon's median is capped (else the sun's
        # thousandfold radiance would streak the haze all the way down)
        hl = hz @ LUM
        cap = 4.0 * np.median(hl)
        hz = hz * np.minimum(1.0, cap / np.maximum(hl, 1e-12))[:, None]
        hz = np.concatenate([hz[-w // 4:], hz, hz[:w // 4]])
        r = int(3 * sigma)
        ker = np.exp(-0.5 * (np.arange(-r, r + 1) / sigma) ** 2)
        ker /= ker.sum()
        return np.stack([np.convolve(hz[:, c], ker, "same") for c in range(3)], -1)[w // 4:w // 4 + w]

    def haze(self, u, ring=None):
        ring = self.horizon if ring is None else ring
        w = len(ring)
        x = u * w - 0.5
        x0 = np.floor(x).astype(int)
        f = (x - x0)[..., None]
        return ring[x0 % w] * (1 - f) + ring[(x0 + 1) % w] * f


# --- the worlds below ------------------------------------------------------------------------------------------------

TILE = 12.0  # the land's map covers 12 x 12 camera altitudes (it repeats; far off, the haze hides it)
MAP = 2048


def ridged(n):
    return 1 - np.abs(2 * n - 1)


def smooth(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0, 1)
    return t * t * (3 - 2 * t)


def land_maps(kind, seed):
    """The land's height (in camera altitudes; only its slopes are drawn, as light and shade) and colour (albedo) on
    its map, and where water lies, by kind."""
    n = lambda cells, s, o=6: k.value_noise(MAP, MAP, (cells, cells), seed + s, octaves=o)  # noqa: E731
    none = np.zeros((MAP, MAP), bool)
    if kind == "mountains":
        big = n(3, 1)
        r = ridged(n(8, 2, 7))
        hgt = 1.2 * (0.45 * big + 0.55 * r ** 1.5) + 0.04 * n(64, 3, 4)
        h01 = (hgt - hgt.min()) / (hgt.max() - hgt.min())
        meadow = np.array([0.16, 0.22, 0.09])
        forest = np.array([0.05, 0.08, 0.04])
        rock = np.array([0.28, 0.26, 0.23])
        snow = np.array([0.75, 0.77, 0.80])
        wood = smooth(0.42, 0.55, n(24, 4, 5)) * smooth(0.65, 0.4, h01)
        alb = meadow * (1 - wood)[..., None] + forest * wood[..., None]
        alb = alb + (rock - alb) * smooth(0.42, 0.62, h01)[..., None]
        alb = alb + (snow - alb) * smooth(0.74, 0.86, h01 + 0.08 * n(32, 5, 4))[..., None]
        return hgt, alb, h01 < 0.12
    if kind == "forest":
        hills = n(5, 1)
        hgt = 0.35 * hills + 0.05 * n(40, 2, 5)
        tone = n(28, 3, 6)
        dark = np.array([0.03, 0.06, 0.025])
        light = np.array([0.09, 0.13, 0.045])
        autumn = np.array([0.17, 0.11, 0.035])
        alb = dark + (light - dark) * smooth(0.3, 0.7, tone)[..., None]
        alb = alb + (autumn - alb) * (0.7 * smooth(0.66, 0.8, n(14, 4, 5)))[..., None]
        glade = smooth(0.71, 0.77, n(12, 5, 5))
        alb = alb + (np.array([0.20, 0.24, 0.10]) - alb) * glade[..., None]
        h01 = (hills - hills.min()) / (hills.max() - hills.min())
        river = smooth(0.965, 0.99, ridged(n(4, 6, 6))) > 0.5
        return hgt, alb, (h01 < 0.06) | river
    if kind == "canyon":
        plateau = n(4, 1, 6)
        cut = ridged(n(5, 2, 7))
        gorge = smooth(0.62, 0.95, cut)
        steps = np.floor(gorge * 4) / 4 + smooth(0.35, 1.0, (gorge * 4) % 1) / 4  # stepped walls
        hgt = 0.2 * plateau - 0.25 * steps + 0.015 * n(64, 3, 4)
        h01 = (hgt - hgt.min()) / (hgt.max() - hgt.min())
        band = 0.5 + 0.5 * np.sin(h01 * 55.0 + 2.0 * n(10, 4, 4))
        red = np.array([0.40, 0.17, 0.08])
        ochre = np.array([0.56, 0.34, 0.17])
        sand = np.array([0.60, 0.45, 0.30])
        alb = sand + (ochre - sand) * smooth(0.3, 0.7, n(16, 5, 5))[..., None]
        wall = smooth(0.05, 0.3, gorge)
        alb = alb + ((red + (ochre - red) * band[..., None]) - alb) * wall[..., None]
        alb = alb + (np.array([0.45, 0.30, 0.20]) - alb) * smooth(0.9, 1.0, gorge)[..., None]
        return hgt, alb, none
    if kind == "mist":
        hills = n(6, 1, 7)
        hgt = 0.7 * hills + 0.03 * n(48, 2, 5)
        tone = n(20, 3, 5)
        alb = np.array([0.06, 0.10, 0.05]) + np.array([0.06, 0.07, 0.03]) * tone[..., None]
        return hgt, alb, none
    raise ValueError(kind)


def shade(hgt, alb, water, light, kind, seed):
    """The land's radiance on its map: lit by the sun by its slope and by the sky, lakes and rivers mirroring the sky
    near the horizon; for mist, the valleys filled with lit fog."""
    texel = TILE / MAP
    gy, gx = np.gradient(hgt, texel)
    nrm = np.stack([-gx, np.ones_like(hgt), -gy], -1)
    nrm /= np.linalg.norm(nrm, axis=-1, keepdims=True)
    # the map's s runs along x, t along z
    sd = light.sun_dir
    lam = np.clip(nrm[..., 0] * sd[0] + nrm[..., 1] * sd[1] + nrm[..., 2] * sd[2], 0, 1)
    sky = light.sky * (0.5 + 0.5 * nrm[..., 1])[..., None]
    rad = alb / np.pi * (light.sun[None, None] * lam[..., None] + sky)
    if water.any():
        rad[water] = light.horizon.mean(0) * 0.16 + light.sky / np.pi * 0.04
    if kind == "mist":
        h01 = (hgt - hgt.min()) / (hgt.max() - hgt.min())
        fog_lit = (light.sky + light.sun * max(light.sun_dir[1], 0.05)) / np.pi * 0.8
        drift = k.value_noise(MAP, MAP, (8, 8), seed + 9, octaves=6)
        fog = 0.95 * smooth(0.6, 0.4, h01 + 0.3 * (drift - 0.5)) ** 0.6
        rad = rad + (fog_lit - rad) * fog[..., None]
        # the fog's share rides along as a fourth channel: the land's fine detail is not drawn on the fog
        return np.concatenate([rad, fog[..., None]], -1)
    return rad


def cloud_maps(light, seed):
    """A sea of cloud seen from above: billows lit by the low sun on one side, in shade on the other, the haze's
    colour deep in the gaps."""
    puffs = k.value_noise(MAP, MAP, (12, 12), seed + 1, octaves=7)
    billow = k.value_noise(MAP, MAP, (96, 96), seed + 2, octaves=4)
    body = np.clip((puffs - 0.38) * 2.2, 0, 1)
    hgt = 1.6 * np.sqrt(body) + 0.15 * billow * body
    alb = np.ones(puffs.shape + (3,)) * 0.9
    rad = shade(hgt, alb, np.zeros_like(hgt, bool), light, "clouds", seed)
    gap = smooth(0.45, 0.33, puffs)
    deep = light.sky / np.pi * 0.45
    return rad + (deep - rad) * gap[..., None]


# Fine detail laid over the land at a much smaller scale than its map (tree crowns, rock grain, billows): a map of its
# own over one camera altitude, multiplying the land's radiance.
DETAIL = {"forest": (0.016, 0.55), "mist": (0.016, 0.4), "mountains": (0.03, 0.25), "canyon": (0.02, 0.2),
          "clouds": (0.05, 0.12)}


def detail_map(kind, seed):
    size = 1024
    cells = max(4, int(round(1.0 / DETAIL[kind][0])))
    if kind in ("forest", "mist"):
        # tree crowns: round, lit towards the middle, dark between them (the distance to the nearest of points
        # scattered one a cell)
        rng = np.random.default_rng(seed + 31)
        pts = rng.random((cells, cells, 2))
        y, x = (np.mgrid[0:size, 0:size] + 0.5) * (cells / size)
        cy, cx = np.floor(y).astype(int), np.floor(x).astype(int)
        best = np.full((size, size), 9.0)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                gy, gx = cy + dy, cx + dx
                p = pts[gy % cells, gx % cells]
                dist = np.hypot(gy + p[..., 0] - y, gx + p[..., 1] - x)
                best = np.minimum(best, dist)
        c = np.clip(1 - best / 0.75, 0, 1) ** 0.6 * (0.7 + 0.6 * k.value_noise(size, size, (cells, cells), seed + 32, 2))
        return np.clip(c, 0, 1)[..., None]
    c = k.value_noise(size, size, (cells, cells), seed + 31, octaves=3)
    return c[..., None]


class Below:
    """The world under the horizon for one sky: its radiance in a direction (unit vectors with y < 0)."""

    def __init__(self, cfg, light, pano):
        self.cfg, self.light, self.pano = cfg, light, pano
        seed = cfg["seed"]
        if cfg["below"] == "land":
            hgt, alb, water = land_maps(cfg["kind"], seed)
            self.ground = Pyramid(shade(hgt, alb, water, light, cfg["kind"], seed))
        elif cfg["below"] == "clouds":
            self.ground = Pyramid(cloud_maps(light, seed))
        if cfg["below"] in ("land", "clouds"):
            self.kind = cfg.get("kind", "clouds")
            self.detail = Pyramid(detail_map(self.kind, seed))
        elif cfg["below"] == "haze":
            pass
        else:
            self.wave_tile = cfg.get("wave_tile", 1.6)
            c = cfg.get("wave_cells", 6)
            w = k.value_noise(1024, 1024, (c, c), seed + 1, octaves=7)
            w = w + 0.6 * k.value_noise(1024, 1024, (c // 2, c * 7 // 3), seed + 2, octaves=6)  # long swell one way
            w = (w - w.min()) / (w.max() - w.min())
            gy, gx = np.gradient(w, 1.0 / 1024)
            self.slopes = Pyramid(np.stack([gx, gy], -1) * (cfg["waves"] / 6.0))
            # foam on the crests, broken into streaks by a finer pattern
            streak = k.value_noise(1024, 1024, (2 * c, 7 * c), seed + 3, octaves=5)
            self.crest = Pyramid((w * (0.6 + 0.8 * streak))[..., None])

    def radiance(self, d, footprint_angle, sky_fn):
        cfg, light = self.cfg, self.light
        down = np.maximum(-d[..., 1], 1e-4)
        dist = 1.0 / down  # slant distance in camera altitudes
        px, pz = d[..., 0] * dist, d[..., 2] * dist
        u = azimuth_u(d)
        # the haze's colour: the horizon's by azimuth when looking out, its average when looking straight down (where
        # every azimuth meets; by azimuth there it would draw a wedge pointing at the sun)
        side = ((1 - down) ** 1.5)[..., None]
        haze_col = light.horizon.mean(0) + (light.haze(u) - light.horizon.mean(0)) * side
        if cfg["below"] == "haze":
            # the deep air under a sea of cloud: the haze, a little of the sky's shade straight down
            col = haze_col * 0.8 + light.sky / np.pi * 0.25 * np.ones_like(haze_col)
        elif cfg["below"] in ("land", "clouds"):
            fp = footprint_angle / down ** 2 * (MAP / TILE)
            col = self.ground.sample(px / TILE, pz / TILE, fp)
            amount = DETAIL[self.kind][1]
            if col.shape[-1] == 4:
                amount = amount * (1 - col[..., 3])
                col = col[..., :3]
            dfp = footprint_angle / down ** 2 * 1024
            det = self.detail.sample(px, pz, dfp)[..., 0]
            col = col * (1 - amount + 2 * amount * det)[..., None]
        else:
            fp = footprint_angle / down ** 2 * (1024 / self.wave_tile)
            sl = self.slopes.sample(px / self.wave_tile, pz / self.wave_tile, fp)
            nrm = np.stack([-sl[..., 0], np.ones(down.shape), -sl[..., 1]], -1)
            nrm /= np.linalg.norm(nrm, axis=-1, keepdims=True)
            dn = (d * nrm).sum(-1, keepdims=True)
            refl = d - 2 * dn * nrm
            refl[..., 1] = np.maximum(refl[..., 1], 0.004)
            refl /= np.linalg.norm(refl, axis=-1, keepdims=True)
            cos = np.clip(-dn[..., 0], 0, 1)
            fres = 0.02 + 0.98 * (1 - cos) ** 5
            # the water's own colour, lit by the sky (what comes up from under the surface)
            water = np.array(cfg["water"]) * float(light.sky @ LUM) / np.pi * 8.0
            col = sky_fn(refl) * fres[..., None] + water * (1 - fres[..., None])
            if cfg.get("foam"):
                # turned against the waves' map and the view, so the noise's lattice does not line up into rows
                ct = self.wave_tile * 0.7
                crest = self.crest.sample((0.866 * px - 0.5 * pz) / ct, (0.5 * px + 0.866 * pz) / ct,
                                          fp * self.wave_tile / ct)[..., 0]
                foam = smooth(cfg["foam"], cfg["foam"] + 0.3, crest) * 0.45
                foam_col = (light.sky + light.sun * 0.3) / np.pi * 0.8
                col = col + (foam_col - col) * foam[..., None]
        f = 1 - np.exp(-cfg["haze"] * dist)
        return col * (1 - f[..., None]) + haze_col * f[..., None]


# --- one sky ---------------------------------------------------------------------------------------------------------


def aces(x):
    return np.clip((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0, 1)


def srgb(x):
    return np.where(x <= 0.0031308, 12.92 * x, 1.055 * np.power(np.maximum(x, 1e-9), 1 / 2.4) - 0.055)


def face_dirs(corners, rows, size):
    """Unit directions of a face's pixels (rows `rows` of `size`), as sky.rs maps the face's texture."""
    p0, p1, p2, p3 = (np.array(c, np.float64) for c in corners)
    j = rows[:, None] + 0.5
    i = np.arange(size)[None, :] + 0.5
    u = (i / size - UV_INSET) / (1 - 2 * UV_INSET)
    v = (j / size - UV_INSET) / (1 - 2 * UV_INSET)
    s, t = np.broadcast_arrays(u, 1 - v)
    q = (p0 * ((1 - s) * (1 - t))[..., None] + p1 * (s * (1 - t))[..., None] + p2 * (s * t)[..., None]
         + p3 * ((1 - s) * t)[..., None])
    return q / np.linalg.norm(q, axis=-1, keepdims=True)


def make_sky(name, root=None):
    cfg = SKIES[name]
    pano = load(cfg["src"], root)
    light = Light(pano)
    if cfg.get("key"):
        # a sun behind haze casts too little to model the land: a soft key light from its direction, a share of the sky's
        light.sun = light.sun + light.sky * cfg["key"]
    below = Below(cfg, light, pano)
    exposure = 0.18 / max(light.reference, 1e-9) * 2.0 ** cfg["ev"]
    cut = math.radians(cfg.get("cut", 0.0))
    reach = cfg.get("reach", 1.0)
    tint = np.array(cfg.get("tint", (1.0, 1.0, 1.0)))

    def sky_fn(d):
        """The photograph above its cut, its kept band stretched down by `reach`."""
        el = elevation(d)
        src = np.where(el < 0, np.arctan(np.tan(el) / reach), el)
        return panorama(pano, azimuth_u(d), np.maximum(src, cut)).astype(np.float64)

    def radiance(d):
        el = elevation(d)
        out = sky_fn(d)
        low = el < 0.0
        if low.any():
            src = np.arctan(np.tan(el[low]) / reach)
            # the photograph down to its cut, the world below from there (a degree and a half of blend)
            w = smooth(cut - math.radians(0.4), cut + math.radians(1.1), src)
            mix = w < 1
            if mix.any():
                idx = np.flatnonzero(low)[mix]
                dd = d.reshape(-1, 3)[idx]
                angle = (np.pi / 2) / (FACE * SS)
                g = below.radiance(dd, angle, sky_fn)
                o = out.reshape(-1, 3)
                o[idx] = o[idx] * w[mix][:, None] + g * (1 - w[mix][:, None])
        lift = cfg.get("lift")
        if lift:
            gain, top, amount = lift
            w = (smooth(math.radians(top), 0.0, np.maximum(el, 0.0)) * amount)[..., None]
            out = out + (light.haze(azimuth_u(d), light.above) * gain - out) * w
        grade = cfg.get("grade")
        if grade:
            k = smooth(0.0, math.radians(40.0), np.maximum(el, 0.0))[..., None]
            low_col = np.array(grade[0])
            if len(grade) > 2:
                # the horizon away from the sun: its own colour, round the circle from the sun's
                sd = light.sun_dir[[0, 2]] / max(np.linalg.norm(light.sun_dir[[0, 2]]), 1e-9)
                hd = d[..., [0, 2]] / np.maximum(np.linalg.norm(d[..., [0, 2]], axis=-1, keepdims=True), 1e-9)
                toward = ((hd @ sd + 1) / 2)[..., None] ** 1.5
                low_col = np.array(grade[2]) + (low_col - np.array(grade[2])) * toward
            out = out * (low_col * (1 - k) + np.array(grade[1]) * k)
        return out * tint

    size = FACE * SS
    os.makedirs(OUT, exist_ok=True)
    made = []
    for face, corners in FACES.items():
        img = np.empty((size, size, 3), np.float32)
        for r0 in range(0, size, 256):
            rows = np.arange(r0, min(size, r0 + 256))
            img[r0:r0 + len(rows)] = radiance(face_dirs(corners, rows, size))
        img = img.reshape(FACE, SS, FACE, SS, 3).mean((1, 3))
        ldr = srgb(aces(img * exposure))
        a = np.clip(ldr * 255 + 0.5, 0, 255).astype(np.uint8)
        path = os.path.join(OUT, f"Sky_{name}_{face}.jpg")
        Image.fromarray(a, "RGB").save(path, quality=JPEG_QUALITY, optimize=True)
        made.append(path)
    print(name, sum(os.path.getsize(p) for p in made), "bytes")
    save_light(name, light, exposure * tint)
    return made


def save_light(name, light, exposure):
    """What the sky casts, in the faces' exposure (linear; 1 white): the sun (direction towards it, its light on a
    surface facing it over pi, as a white cloud returns it), the sky's light from above, the horizon's haze."""
    import json
    table = {}
    if os.path.exists(SKY_LIGHT):
        with open(SKY_LIGHT) as f:
            table = json.load(f)
    r = lambda v: [round(float(x), 4) for x in v]  # noqa: E731
    table[name] = {"sun": r(light.sun_dir), "sun_color": r(light.sun * exposure / np.pi),
                   "sky": r(light.sky * exposure / np.pi), "horizon": r(light.horizon.mean(0) * exposure)}
    with open(SKY_LIGHT, "w") as f:
        json.dump(dict(sorted(table.items())), f, indent=1)
        f.write("\n")


def credits():
    out = []
    for name, cfg in SKIES.items():
        sid = cfg["src"]
        digest, _size, authors, date = SOURCES[sid]
        out.append(f"| `Sky_{name}` | [{sid}](https://polyhaven.com/a/{sid}) | {authors}, Poly Haven | CC0 1.0 | {date} "
                   f"| `{sid}_4k.hdr` | `{digest}` |")
    return out


def main(argv):
    if len(argv) >= 2 and argv[1] in ("fetch", "check") and len(argv) <= 3:
        (fetch if argv[1] == "fetch" else check)(argv[2] if len(argv) == 3 else None)
        return 0
    if len(argv) >= 2 and argv[1] == "make":
        rest = argv[2:]
        root = rest.pop(0) if rest and rest[0] not in SKIES else None
        for name in rest or list(SKIES):
            if name not in SKIES:
                raise SystemExit(f"no sky {name}: {', '.join(SKIES)}")
            make_sky(name, root)
        return 0
    if len(argv) == 2 and argv[1] == "credits":
        print("\n".join(credits()))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
