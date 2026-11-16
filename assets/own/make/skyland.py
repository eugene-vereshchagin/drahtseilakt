#!/usr/bin/env python3
"""The green land far below the photographic skies (make_skies.py: `Mountains`, `Forest`, `Mist`), drawn point by point
where each pixel of a sky face looks, rather than looked up in a repeating map.

A map of the land magnified under the camera turns into blocks and a smear (the owner, 2026-11-16: "a rotten swamp
behind the clouds"); a function of the plane has the detail of any distance. Every field here is fractal value noise
evaluated at the world point itself, its octaves finer than the pixel's footprint faded to their mean (so nothing
aliases and far off the land turns smooth by itself, as the haze takes it). Light and shade come from the slopes,
taken by finite differences one footprint apart (so they are filtered at the pixel's own scale too): the relief at
the scale of mountains and valleys, and straight down the bumps of the tree crowns.

The land, in camera altitudes (the camera flies 1 over it): ridged mountains rising out of rolling hills, forest on
the slopes below the tree line (conifers higher up, broadleaf lower, stands of their own shades, the crowns' texture
straight down), clearings and meadows, rivers in the valleys and lakes in the hollows (mirroring the sky), bare rock
on the steep and high ground, snow on the highest peaks; for `Mist`, fog lying in the valleys.

Only numpy; nothing is read or downloaded.
"""

import numpy as np

LUM = np.array([0.2126, 0.7152, 0.0722])


def smooth(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0, 1)
    return t * t * (3 - 2 * t)


class Field:
    """Fractal value noise of the whole plane (each octave a 256 x 256 random lattice, turned and shifted its own way,
    so neither its lattice nor its repeat lines up), in 0..1. A ridged field folds its first `ridge` octaves (about
    (1 - |2n - 1|)^2): sharp crests (the finer ones left round, or their creases would line up into a lattice of
    straight furrows). An octave finer than the footprint is faded to its own mean, never thresholded or folded
    afterwards (a folded fbm whose octaves fade would climb to its crest far off: a plateau with a straight edge)."""

    def __init__(self, seed, freq, octaves, gain=0.5, lacunarity=2.03, ridge=0):
        rng = np.random.default_rng(seed)
        self.freq, self.gain, self.lac, self.ridge = freq, gain, lacunarity, ridge
        self.tabs = rng.random((octaves, 256, 256)).astype(np.float32)
        ang = rng.random(octaves) * 2 * np.pi
        self.rot = np.stack([np.cos(ang), np.sin(ang)], -1).astype(np.float32)
        self.off = (rng.random((octaves, 2)) * 256).astype(np.float32)
        probe = (rng.random((2, 4096)) * 256).astype(np.float32)
        self.mean = [float(self.fold(vnoise(t, probe[0], probe[1]), o).mean()) for o, t in enumerate(self.tabs)]

    def fold(self, n, octave):
        # the crest rounded a little (a sharp fold is a crease the light flips across: seen from above, a pencil
        # line round every hill)
        return (1.2 - np.sqrt((2 * n - 1) ** 2 + 0.04)) ** 2 if octave < self.ridge else n

    def __call__(self, x, z, fp):
        """At world points x, z with a footprint of fp (world units a sample spans)."""
        out = np.zeros(x.shape, np.float32)
        f, a, total = self.freq, 1.0, 0.0
        for o in range(len(self.tabs)):
            # full strength up to a quarter of a period a sample, none from half
            w = np.clip(2.0 - 4.0 * fp * f, 0.0, 1.0)
            if w.max() > 0:
                c, s = self.rot[o]
                u = (c * x - s * z) * f + self.off[o, 0]
                v = (s * x + c * z) * f + self.off[o, 1]
                n = self.fold(vnoise(self.tabs[o], u, v), o)
                out += a * (self.mean[o] + (n - self.mean[o]) * w)
            else:
                out += a * self.mean[o]
            total += a
            f *= self.lac
            a *= self.gain
        return out / total


def vnoise(tab, u, v):
    iu, iv = np.floor(u), np.floor(v)
    fu, fv = u - iu, v - iv
    fu = fu * fu * (3 - 2 * fu)
    fv = fv * fv * (3 - 2 * fv)
    iu = iu.astype(np.int64) & 255
    iv = iv.astype(np.int64) & 255
    iu1, iv1 = (iu + 1) & 255, (iv + 1) & 255
    a = tab[iv, iu] + (tab[iv, iu1] - tab[iv, iu]) * fu
    b = tab[iv1, iu] + (tab[iv1, iu1] - tab[iv1, iu]) * fu
    return a + (b - a) * fv


# The kinds: how much of the land is mountains (0: hills only), the hills' and the mountains' height (camera
# altitudes), the tree line and the snow line (as shares of the land below them), lakes (the share of the land under
# water), rivers (their half width in the river field), how much of the forest is cleared, and fog in the valleys.
KINDS = {
    "mountains": dict(mountains=0.55, hills=0.10, peaks=0.55, treeline=0.80, snow=0.975, lakes=0.03, river=0.010,
                      clearings=0.30, fog=0.0),
    "forest": dict(mountains=0.0, hills=0.12, peaks=0.0, treeline=1.01, snow=1.01, lakes=0.06, river=0.012,
                   clearings=0.22, fog=0.0),
    "mist": dict(mountains=0.0, hills=0.30, peaks=0.0, treeline=1.01, snow=1.01, lakes=0.0, river=0.0,
                 clearings=0.15, fog=0.35),
}

# The albedos (linear): healthy summer greens, a cool grey rock, snow.
CONIFER = np.array([0.030, 0.062, 0.036], np.float32)
BROADLEAF = np.array([0.054, 0.104, 0.034], np.float32)
MEADOW = np.array([0.075, 0.130, 0.042], np.float32)
MEADOW_LUSH = np.array([0.055, 0.115, 0.038], np.float32)
ALPINE = np.array([0.080, 0.100, 0.055], np.float32)
ROCK = np.array([0.175, 0.180, 0.185], np.float32)
SCREE = np.array([0.24, 0.235, 0.225], np.float32)
SNOW = np.array([0.80, 0.82, 0.86], np.float32)
SHORE = np.array([0.15, 0.15, 0.12], np.float32)
WATER = np.array([0.010, 0.024, 0.030], np.float32)

CROWNS = 160.0  # the tree crowns' frequency (a crown about 1/300 of the camera's altitude across)


class Terrain:
    def __init__(self, kind, seed, relief=1.0):
        self.k = dict(KINDS[kind])
        self.relief = relief
        s = seed * 97
        self.warp_x = Field(s + 1, 0.35, 4)
        self.warp_z = Field(s + 2, 0.35, 4)
        self.base = Field(s + 3, 0.20, 4)
        self.ridge = Field(s + 4, 0.60, 8, ridge=3)
        self.hill = Field(s + 5, 1.1, 7)
        self.moist = Field(s + 6, 1.3, 6)
        self.stand = Field(s + 7, 9.0, 4)
        self.mix = Field(s + 8, 2.2, 5)
        self.river = Field(s + 9, 0.30, 5)
        self.crowns = Field(s + 10, CROWNS, 3, gain=0.55)
        self.drift = Field(s + 11, 0.9, 5)
        # the heights' shares, from a wide sample of the land (the tree line, the snow and the lakes are by share)
        g = (np.random.default_rng(s).random((2, 65536)) * 60.0).astype(np.float32)
        fine = np.full(g[0].shape, 1e-4, np.float32)
        hs = self.relief_height(g[1], g[0], fine)[0]
        self.q = {n: float(np.quantile(hs, min(self.k[n], 1.0))) if self.k[n] <= 1 else float(hs.max()) + 1.0
                  for n in ("treeline", "snow", "lakes")}
        self.q["lo"], self.q["hi"] = float(np.quantile(hs, 0.02)), float(np.quantile(hs, 0.98))
        ms = self.moist(g[1], g[0], fine)
        self.q["clear"] = float(np.quantile(ms, self.k["clearings"]))

    def relief_height(self, x, z, fp):
        """The land's height without the crowns, and what the cover needs of it."""
        k = self.k
        wx = x + 0.55 * (self.warp_x(x, z, fp) - 0.5)
        wz = z + 0.55 * (self.warp_z(x, z, fp) - 0.5)
        base = self.base(wx, wz, fp)
        hills = self.hill(wx, wz, fp)
        h = 0.6 * base + k["hills"] * hills
        mtn = None
        if k["mountains"] > 0:
            mtn = smooth(0.47, 0.66, base) * k["mountains"] / 0.55
            h = h + k["peaks"] * mtn * self.ridge(wx, wz, fp)
        return h, wx, wz, base, mtn

    def height(self, x, z, fp, forest=None):
        h, wx, wz, base, mtn = self.relief_height(x, z, fp)
        if forest is None:
            forest = self.forest(x, z, fp, h)
        crowns = self.crowns(x, z, fp)
        return h + 0.0022 * forest * (crowns - 0.5), crowns, h

    def forest(self, x, z, fp, h):
        """How much of the ground is under trees (0..1)."""
        k = self.k
        m = self.moist(x, z, fp)
        tree = smooth(self.q["clear"] - 0.015, self.q["clear"] + 0.015, m)
        tl = self.q["treeline"]
        span = max(self.q["hi"] - self.q["lo"], 1e-6)
        ragged = 0.06 * span * (self.mix(x, z, fp) - 0.5)
        tree = tree * smooth(tl + 0.02 * span, tl - 0.03 * span, h + ragged)
        return tree

    def surface(self, x, z, fp):
        """At world points: the albedo, the normal, how much is water, how much is fog."""
        k, q = self.k, self.q
        h, wx, wz, base, mtn = self.relief_height(x, z, fp)
        forest = self.forest(x, z, fp, h)
        hc, crowns, _ = self.height(x, z, fp, forest)
        e = np.maximum(fp, 1e-5)
        hx, _, rx = self.height(x + e, z, fp, forest)
        hz, _, rz = self.height(x, z + e, fp, forest)
        rl = self.relief
        nrm = np.stack([-(hx - hc) / e * rl, np.ones_like(hc), -(hz - hc) / e * rl], -1)
        nrm /= np.linalg.norm(nrm, axis=-1, keepdims=True)
        span = max(q["hi"] - q["lo"], 1e-6)
        h01 = np.clip((h - q["lo"]) / span, 0, 1)
        # the ground's own steepness (not the crowns'): where rock shows
        steep = 1 - 1 / np.sqrt(1 + ((rx - h) / e) ** 2 + ((rz - h) / e) ** 2)

        # the open ground: meadows, alpine grass higher up, rock where it is steep and high, snow on the peaks
        mix = self.mix(x, z, fp)
        alb = MEADOW + (MEADOW_LUSH - MEADOW) * smooth(0.35, 0.65, mix)[..., None]
        alb = alb + (ALPINE - alb) * smooth(q["treeline"] - 0.05 * span, q["treeline"] + 0.1 * span, h)[..., None]
        if mtn is not None:
            rock = smooth(0.25, 0.5, steep + 0.4 * smooth(q["treeline"], q["snow"], h)) * smooth(0.1, 0.5, mtn)
            alb = alb + (ROCK + (SCREE - ROCK) * mix[..., None] - alb) * rock[..., None]
            snow = smooth(q["snow"] - 0.02 * span, q["snow"] + 0.03 * span, h + 0.08 * span * (mix - 0.5))
            snow = snow * smooth(0.55, 0.3, steep)
            alb = alb + (SNOW - alb) * snow[..., None]
            forest = forest * (1 - rock)
        # the forest: conifers higher up, broadleaf lower, each stand its own shade, the crowns' gaps dark
        conifer = smooth(0.35, 0.75, h01 + 0.5 * (self.mix(x * 0.7, z * 0.7, fp) - 0.5))
        trees = BROADLEAF + (CONIFER - BROADLEAF) * conifer[..., None]
        stand = self.stand(x, z, fp)
        trees = trees * (0.75 + 0.5 * stand)[..., None] * (0.55 + 0.9 * crowns)[..., None]
        alb = alb + (trees - alb) * forest[..., None]

        # water: lakes in the hollows, rivers along the valleys (their width antialiased by the footprint)
        water = np.zeros(h.shape, np.float32)
        if k["lakes"] > 0:
            water = smooth(q["lakes"] + 0.004 * span, q["lakes"] - 0.004 * span, h)
        if k["river"] > 0:
            rv = self.river(wx, wz, fp)
            d = np.abs(rv - 0.5)
            fpf = np.maximum(fp * self.river.freq * 3.0, 1e-6)
            width = k["river"] * smooth(q["treeline"], q["lakes"], h)
            water = np.maximum(water, np.clip(0.5 + (width - d) / fpf, 0, 1))
            shore = np.clip(0.5 + (width * 1.8 - d) / fpf, 0, 1) * smooth(0.0, 1.0, width / max(k["river"], 1e-6))
            alb = alb + (SHORE - alb) * (shore * (1 - forest) * 0.6)[..., None]

        fog = np.zeros(h.shape, np.float32)
        if k["fog"] > 0:
            drift = self.drift(x, z, fp)
            fog = 0.95 * smooth(k["fog"] + 0.15, k["fog"] - 0.1, h01 + 0.35 * (drift - 0.5)) ** 0.6
        return alb, nrm, water, fog


def preview(kind, seed=1, size=512, extent=2.0):
    """A top-down albedo x light picture of a patch (for tuning by eye)."""
    g = (np.mgrid[0:size, 0:size].astype(np.float32) + 0.5) * (extent / size)
    t = Terrain(kind, seed)
    alb, nrm, water, fog = t.surface(g[1], g[0], np.full(g[0].shape, extent / size, np.float32))
    sun = np.array([0.74, 0.22, 0.63])
    lam = np.clip(nrm @ sun, 0, 1)[..., None]
    col = alb * (lam * 3 + 0.6 * (0.5 + 0.5 * nrm[..., 1:2]))
    col = col * (1 - water[..., None]) + np.array([0.05, 0.07, 0.1]) * water[..., None]
    return col + (0.5 - col) * fog[..., None]
