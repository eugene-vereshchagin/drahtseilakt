#!/usr/bin/env python3
"""How close the own interface looks to the original's: a similarity score per element and per screen, the way the
generated sounds were judged (measure the original, make our own, score, iterate). The original is only read and
measured, from the player's copy; nothing of it is written anywhere but the comparison sheets for looking at, which
stay outside the repository.

A score is 0..100 from three parts, each comparing the same region of the original and of ours at the same size:
- 40 % colour: the overlap (Bhattacharyya coefficient) of the two Lab colour histograms (8 x 8 x 8 bins, smoothed);
- 30 % shape: SSIM of the greyscale lightness, heavily blurred (the masses: where it is light and dark);
- 30 % edges: SSIM of the edge maps (Sobel magnitude), lightly blurred (the outlines: frames, tubes, balls).
Textures are laid over two backdrops (dark and warm) first, so their transparency counts as it does on screen.

  python3 -I metric.py textures COPY OUT     # every interface element of Button01_* (the copy's Textures folder)
  python3 -I metric.py screens ORIG OWN OUT  # screenshot pairs NAME.png in ORIG and OWN, whole and by region
  python3 -I metric.py report BEFORE AFTER [ORIG]  # scores.md here from two runs' OUT folders (textures/ and
                                                  # screens/); with ORIG, the original against itself as a yardstick

Each writes OUT/scores.json and side-by-side sheets (original | ours) as OUT/<name>.png. Needs numpy, scipy, Pillow.
"""

import json
import os
import sys

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

HERE = os.path.dirname(os.path.abspath(__file__))
OWN = os.path.dirname(HERE)

# --- colour --------------------------------------------------------------------------------------------------------


def lab(rgb):
    """sRGB 0..1 (H x W x 3) to CIE Lab (D65)."""
    c = np.where(rgb <= 0.04045, rgb / 12.92, ((rgb + 0.055) / 1.055) ** 2.4)
    m = np.array([[0.4124, 0.3576, 0.1805], [0.2126, 0.7152, 0.0722], [0.0193, 0.1192, 0.9505]])
    xyz = c @ m.T / np.array([0.95047, 1.0, 1.08883])
    f = np.where(xyz > 0.008856, np.cbrt(xyz), 7.787 * xyz + 16 / 116)
    return np.stack([116 * f[..., 1] - 16, 500 * (f[..., 0] - f[..., 1]), 200 * (f[..., 1] - f[..., 2])], -1)


def histogram(l):
    h, _ = np.histogramdd(
        l.reshape(-1, 3), bins=(8, 8, 8), range=((0, 100), (-60, 60), (-60, 60))
    )
    h = ndimage.gaussian_filter(h, 0.7, mode="constant")
    return h / max(h.sum(), 1e-9)


def colour_overlap(la, lb):
    return float(np.sqrt(histogram(la) * histogram(lb)).sum())


# --- structure -----------------------------------------------------------------------------------------------------


def ssim(a, b, sigma):
    """Mean SSIM of two greyscale images (0..1 values), Gaussian windows of `sigma`."""
    c1, c2 = 0.01**2, 0.03**2
    g = lambda x: ndimage.gaussian_filter(x, sigma)
    ma, mb = g(a), g(b)
    va, vb, cov = g(a * a) - ma * ma, g(b * b) - mb * mb, g(a * b) - ma * mb
    s = ((2 * ma * mb + c1) * (2 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2))
    return float(np.clip(s.mean(), 0, 1))


def edges(grey):
    g = ndimage.gaussian_filter(grey, 0.8)
    m = np.hypot(ndimage.sobel(g, 0), ndimage.sobel(g, 1))
    return np.clip(m / 2.0, 0, 1)


def score(a, b):
    """The parts and the total for two RGB images (0..1) of the same size."""
    la, lb = lab(a), lab(b)
    ga, gb = la[..., 0] / 100, lb[..., 0] / 100
    side = min(a.shape[:2])
    colour = colour_overlap(la, lb)
    shape = ssim(ndimage.gaussian_filter(ga, side / 24), ndimage.gaussian_filter(gb, side / 24), max(side / 12, 1.5))
    edge = ssim(ndimage.gaussian_filter(edges(ga), 1.0), ndimage.gaussian_filter(edges(gb), 1.0), max(side / 16, 1.5))
    delta_e = float(np.linalg.norm(la.reshape(-1, 3).mean(0) - lb.reshape(-1, 3).mean(0)))
    total = 100 * (0.4 * colour + 0.3 * shape + 0.3 * edge)
    return {"total": round(total, 1), "colour": round(colour, 3), "shape": round(shape, 3), "edges": round(edge, 3),
            "mean_dE": round(delta_e, 1)}


# --- the elements of the interface textures --------------------------------------------------------------------------

# (texture, name, rectangle in texels): the parts the game's code takes (menu.rs, hud.rs, endscreen.rs)
ELEMENTS = [
    ("select", "main button, chosen", (0, 131, 256, 191)),
    ("deselect", "main button", (0, 131, 256, 191)),
    ("select", "field, chosen", (0, 0, 256, 63)),
    ("deselect", "field", (0, 0, 256, 63)),
    ("select", "level button, chosen", (0, 63, 165, 94)),
    ("deselect", "level button", (0, 63, 165, 94)),
    ("deselect", "arrow left", (164, 63, 201, 103)),
    ("deselect", "arrow right", (203, 63, 241, 103)),
    ("select", "key row, chosen", (0, 104, 256, 130)),
    ("deselect", "key row", (0, 104, 256, 130)),
    ("select", "back button, chosen", (61, 193, 222, 252)),
    ("deselect", "back button", (61, 193, 222, 252)),
    ("deselect", "yes / no", (0, 211, 58, 233)),
    ("special", "highscore place", (0, 16, 256, 43)),
    ("special", "level not open", (0, 63, 165, 94)),
    ("special", "life ball", (17, 135, 45, 164)),
    ("special", "lives end piece", (1, 134, 15, 164)),
    ("special", "lives frame", (47, 119, 104, 180)),
    ("special", "points box", (83, 187, 256, 251)),
    ("special", "end screen bar", (154, 133, 230, 164)),
]
BACKDROPS = [(0.16, 0.15, 0.17), (0.86, 0.62, 0.42)]


def over(rgba, backdrop):
    a = rgba[..., 3:4]
    return rgba[..., :3] * a + np.asarray(backdrop) * (1 - a)


def load(path):
    return np.asarray(Image.open(path).convert("RGBA")).astype(np.float64) / 255


def to_image(rgb, scale):
    img = Image.fromarray((np.clip(rgb, 0, 1) * 255 + 0.5).astype(np.uint8))
    return img.resize((img.width * scale, img.height * scale), Image.NEAREST)


def sheet(rows, path, label_width=230):
    """Rows of (label, original image, our image) side by side."""
    w = label_width + 2 * max(max(r[1].width, r[2].width) for r in rows) + 30
    h = sum(max(r[1].height, r[2].height) + 10 for r in rows) + 30
    out = Image.new("RGB", (w, h), (24, 24, 30))
    d = ImageDraw.Draw(out)
    col = (w - label_width - 30) // 2
    d.text((label_width, 6), "original (player's copy)", fill=(220, 220, 220))
    d.text((label_width + col + 20, 6), "ours", fill=(220, 220, 220))
    y = 26
    for label, a, b in rows:
        d.text((8, y + 4), label, fill=(230, 230, 230))
        out.paste(a, (label_width, y))
        out.paste(b, (label_width + col + 20, y))
        y += max(a.height, b.height) + 10
    out.save(path)


def textures(copy, out):
    os.makedirs(out, exist_ok=True)
    ours = {n: load(os.path.join(OWN, "Textures", f"Button01_{n}.tga")) for n in ("select", "deselect", "special")}
    theirs = {n: load(os.path.join(copy, f"Button01_{n}.tga")) for n in ("select", "deselect", "special")}
    scores, rows = {}, []
    for tex, name, (x0, y0, x1, y1) in ELEMENTS:
        a, b = theirs[tex][y0:y1, x0:x1], ours[tex][y0:y1, x0:x1]
        parts = [score(over(a, bg), over(b, bg)) for bg in BACKDROPS]
        s = {k: round(float(np.mean([p[k] for p in parts])), 3 if k not in ("total", "mean_dE") else 1) for k in parts[0]}
        scores[name] = s
        k = 3 if (x1 - x0) > 100 else 5
        rows.append((f"{name}: {s['total']}", to_image(over(a, BACKDROPS[1]), k), to_image(over(b, BACKDROPS[1]), k)))
        print(f"{name:24s} {s['total']:5.1f}  colour {s['colour']:.2f} shape {s['shape']:.2f} edges {s['edges']:.2f}"
              f"  dE {s['mean_dE']}")
    print(f"{'mean':24s} {np.mean([s['total'] for s in scores.values()]):5.1f}")
    for i in range(0, len(rows), 10):
        sheet(rows[i : i + 10], os.path.join(out, f"textures_{i // 10 + 1}.png"))
    with open(os.path.join(out, "scores.json"), "w") as f:
        json.dump(scores, f, indent=1)
    return scores


# --- whole screens -------------------------------------------------------------------------------------------------

# regions of a 16:9 screen (fractions): the menu's column, the scene either side, the bottom corners of the HUD
REGIONS = {
    "whole": (0, 0, 1, 1),
    "menu column": (0.3, 0, 0.7, 1),
    "scene left": (0, 0, 0.3, 1),
    "scene right": (0.7, 0, 1, 1),
    "points box": (0, 0.82, 0.25, 1),
    "lives": (0.75, 0.82, 1, 1),
    "end panel": (0.33, 0.25, 0.67, 0.6),
}
SCREEN_REGIONS = {"hud": ["points box", "lives"], "end": ["end panel", "points box", "lives"]}


def screens(orig, own, out):
    os.makedirs(out, exist_ok=True)
    scores = {}
    for name in sorted(os.listdir(orig)):
        if not name.endswith(".png") or not os.path.exists(os.path.join(own, name)):
            continue
        a = load(os.path.join(orig, name))[..., :3]
        b = load(os.path.join(own, name))[..., :3]
        if a.shape != b.shape:
            continue
        key = name[:-4]
        kind = key.split("_")[-1]
        regions = ["whole"] + SCREEN_REGIONS.get(kind, ["menu column", "scene left", "scene right"])
        s = {}
        h, w = a.shape[:2]
        for r in regions:
            x0, y0, x1, y1 = REGIONS[r]
            box = (slice(int(y0 * h), int(y1 * h)), slice(int(x0 * w), int(x1 * w)))
            s[r] = score(a[box], b[box])
        scores[key] = s
        print(f"{key:20s} " + "  ".join(f"{r} {v['total']:.1f}" for r, v in s.items()))
        ia, ib = to_image(a, 1), to_image(b, 1)
        ia, ib = ia.resize((w // 2, h // 2)), ib.resize((w // 2, h // 2))
        sheet([(f"{key}: {s['whole']['total']}", ia, ib)], os.path.join(out, f"{key}.png"), label_width=160)
    with open(os.path.join(out, "scores.json"), "w") as f:
        json.dump(scores, f, indent=1)
    return scores


def report(before, after, orig=None):
    """scores.md: each element's and each screen's score now against before (v1, the first own interface)."""
    def read(d, kind):
        try:
            with open(os.path.join(d, kind, "scores.json")) as f:
                return json.load(f)
        except OSError:
            return {}
    lines = ["# How close the own interface comes to the original", "",
             "Made by `metric.py` (its docstring says how a score is made: 40 % Lab colour histogram overlap, 30 %",
             "SSIM of the heavily blurred lightness, 30 % SSIM of the edge maps; 0 to 100). *Before* is the previous",
             "version (v2: the menu scene's textures drawn from noise; v1, the first own interface, scored 51.7 for the",
             "elements and 34 to 63 for the menu screens' regions), *now* this one (v3: its concrete, brick, wood and",
             "iron from CC0 photo textures, graded to the measured palette and grain). Screens are 1280x720",
             "renders of the same moment (`DRAHTSEILAKT_SHOT_MENU`, the menu scene `DRAHTSEILAKT_SHOT_MENU_MS` into",
             "its round), English, the original's from the player's copy. The original's pictures never enter the",
             "repository; only these numbers do.", "", "## Elements of the interface textures", "",
             "| element | before | now | colour | shape | edges |", "|---|---:|---:|---:|---:|---:|"]
    tb, ta = read(before, "textures"), read(after, "textures")
    for name, s in ta.items():
        b = tb.get(name, {}).get("total", "")
        lines.append(f"| {name} | {b} | **{s['total']}** | {s['colour']} | {s['shape']} | {s['edges']} |")
    mb = np.mean([v["total"] for v in tb.values()]) if tb else float("nan")
    lines.append(f"| *mean* | {mb:.1f} | **{np.mean([v['total'] for v in ta.values()]):.1f}** | | | |")
    lines += ["", "## Screens", "", "| screen | region | before | now |", "|---|---|---:|---:|"]
    sb, sa = read(before, "screens"), read(after, "screens")
    for key, regions in sa.items():
        for r, v in regions.items():
            b = sb.get(key, {}).get(r, {}).get("total", "")
            lines.append(f"| {key} | {r} | {b} | **{v['total']}** |")
    if orig:
        lines += ["", "## Yardstick: the original against itself", "",
                  "The original's main menu at one moment of its camera's round against another moment (the same scene",
                  "and buttons, seen from elsewhere): what a score means for a scene that is the same kind of place.",
                  "", "| screens | whole | menu column | scene left | scene right |", "|---|---:|---:|---:|---:|"]
        shots = sorted(n for n in os.listdir(orig) if n.startswith("main_") and n.endswith(".png"))
        for i, a_name in enumerate(shots):
            for b_name in shots[i + 1 :]:
                a = load(os.path.join(orig, a_name))[..., :3]
                b = load(os.path.join(orig, b_name))[..., :3]
                h, w = a.shape[:2]
                row = []
                for r in ("whole", "menu column", "scene left", "scene right"):
                    x0, y0, x1, y1 = REGIONS[r]
                    box = (slice(int(y0 * h), int(y1 * h)), slice(int(x0 * w), int(x1 * w)))
                    row.append(score(a[box], b[box])["total"])
                lines.append(f"| {a_name[:-4]} / {b_name[:-4]} | " + " | ".join(str(v) for v in row) + " |")
    with open(os.path.join(HERE, "scores.md"), "w") as f:
        f.write("\n".join(lines) + "\n")
    print("written", os.path.join(HERE, "scores.md"))


def main(argv):
    if len(argv) == 4 and argv[1] == "textures":
        textures(argv[2], argv[3])
    elif len(argv) == 5 and argv[1] == "screens":
        screens(argv[2], argv[3], argv[4])
    elif len(argv) in (4, 5) and argv[1] == "report":
        report(*argv[2:])
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
