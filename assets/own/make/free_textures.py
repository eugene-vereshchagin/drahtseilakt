#!/usr/bin/env python3
"""Freely licensed photo textures (CC0, ambientCG) as raw material for our own textures: where each comes from, its
pinned checksum, and the grading that turns it into ours (tinted to a measured palette, contrast set to a measured
grain, tiled, resized). The downloads are never shipped; what the scripts make from them is, and is committed, so
the game's build needs neither these files nor the network.

  python3 -I free_textures.py fetch [DIR]   # download the pinned archives once into DIR/<id>/ and check them
  python3 -I free_textures.py check [DIR]   # check the archives already there against their checksums

DIR defaults to $DRAHTSEILAKT_TEXTURE_SOURCES, else `texture-sources` beside the repository. The archives are read in
place (zipfile, only the one image asked for); nothing in them is extracted or run. Every source is checked against its
SHA-256 before use and refused when it differs.

Used by make_menu_scene.py (and make_textures.py for the levels' own textures) as a module.
"""

import hashlib
import io
import os
import sys
import urllib.request
import zipfile

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))

AMBIENTCG = {"author": "Lennart Demes (ambientCG)", "licence": "CC0 1.0 Universal",
             "licence_url": "https://docs.ambientcg.com/license/"}

# id: (archive, its SHA-256, its size, the asset's release date). The archive is the asset's 1K JPG set from
# https://ambientcg.com/get?file=<archive>, its page https://ambientcg.com/a/<id>.
SOURCES = {
    "Concrete047A": ("Concrete047A_1K-JPG.zip", "94e04bdf80c4fa377f8c736183963cdd911131a26673e2462495e45bf679102f",
                     8614118, "2025-10-09"),
    "Concrete012": ("Concrete012_1K-JPG.zip", "3424d5cbbab7191d5180f362de0792c99e6a1abd81b6eb3a431c4dfc52fefe7e",
                    8171336, "2018-10-28"),
    "Bricks005": ("Bricks005_1K-JPG.zip", "5d7b5584245ffa94eb74b9173c412e33a7cb012b75632a5125b426f9ff9fa0f6",
                  7688196, "2018-03-06"),
    "WoodFloor040": ("WoodFloor040_1K-JPG.zip", "c83a01f1733f2a1f6e95ad6703689eb12fe259e77245d4c0a5652728d07c87cb",
                     5913872, "2020-06-14"),
    "Metal063": ("Metal063_1K-JPG.zip", "cc0a7f83d4bfd6c5d38638d17a0c214a004ccd295b8c554c959537bc278167ed",
                 5620613, "2026-03-30"),
    "Metal009": ("Metal009_1K-JPG.zip", "1b1970219b40fe707e7194b28702f1a65b0d227bbea56da69dfbc8d8b5cfcb5e",
                 4767000, "2018-08-16"),
}
MAX_ARCHIVE = 64 << 20


def sources_dir(arg=None):
    return arg or os.environ.get("DRAHTSEILAKT_TEXTURE_SOURCES") or os.path.join(os.path.dirname(REPO),
                                                                                  "texture-sources")


def archive_path(sid, root=None):
    return os.path.join(sources_dir(root), sid, SOURCES[sid][0])


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


_checked = set()


def verified(sid, root=None):
    """The archive's path, after its checksum matched the pinned one (else an error saying how to get it)."""
    path = archive_path(sid, root)
    if path in _checked:
        return path
    if not os.path.exists(path):
        raise SystemExit(f"{path} is missing: python3 -I {os.path.join(HERE, 'free_textures.py')} fetch")
    got = sha256(path)
    if got != SOURCES[sid][1]:
        raise SystemExit(f"{path}: SHA-256 {got}, expected {SOURCES[sid][1]}; refusing it")
    _checked.add(path)
    return path


def fetch(root=None):
    for sid, (name, digest, size, _date) in SOURCES.items():
        path = archive_path(sid, root)
        if os.path.exists(path) and sha256(path) == digest:
            continue
        os.makedirs(os.path.dirname(path), exist_ok=True)
        url = f"https://ambientcg.com/get?file={name}"
        print("fetching", url)
        with urllib.request.urlopen(url, timeout=120) as r:
            data = r.read(MAX_ARCHIVE + 1)
        if len(data) > MAX_ARCHIVE or hashlib.sha256(data).hexdigest() != digest:
            raise SystemExit(f"{url}: not the pinned archive (size {len(data)}, expected {size}); refusing it")
        with open(path, "wb") as f:
            f.write(data)
    check(root)


def check(root=None):
    for sid in SOURCES:
        print(sid, "ok", verified(sid, root))


def load(sid, kind="Color", root=None):
    """One map (Color, NormalGL, Roughness, Displacement, ...) of a source as floats 0..1 (H x W x 3)."""
    member = f"{sid}_1K-JPG_{kind}.jpg"
    with zipfile.ZipFile(verified(sid, root)) as z:
        info = z.getinfo(member)
        if info.file_size > 32 << 20:
            raise SystemExit(f"{member}: too large")
        data = z.read(member)
    im = Image.open(io.BytesIO(data))
    if im.width > 4096 or im.height > 4096:
        raise SystemExit(f"{member}: too large")
    return np.asarray(im.convert("RGB"), np.float64) / 255.0


# --- grading -----------------------------------------------------------------------------------------------------

LUMA = np.array([0.299, 0.587, 0.114])


def blur_wrap(img, sigma):
    """A Gaussian blur that wraps round the edges (the textures tile)."""
    from scipy import ndimage

    return ndimage.gaussian_filter(img, (sigma, sigma, 0) if img.ndim == 3 else sigma, mode="wrap")


def grade(img, mean, rel_std, hue_keep=0.35, flatten=0.0):
    """The texture tinted to `mean` (RGB 0..255, a measured palette) with its lightness varying by `rel_std` (the
    standard deviation against the mean, measured on the original's texture of the same role): the source gives only
    the pattern, its own colour is kept for `hue_keep` of its variation. `flatten` (0..1) takes out that much of the
    large blotches (the light and dark patches wider than an eighth of the texture), so the grain is left."""
    img = np.clip(img, 1e-3, 1)
    ratio = img / img.mean((0, 1))
    r = ratio @ LUMA
    if flatten:
        low = blur_wrap(r, img.shape[0] / 16)
        r = r / (low ** flatten)
    chroma = ratio / r[..., None]
    d = r / r.mean() - 1
    d = d * (rel_std / max(d.std(), 1e-6))
    out = (np.asarray(mean, np.float64) / 255.0) * (1 + d)[..., None] * chroma ** hue_keep
    out = out * (np.asarray(mean, np.float64) / 255.0) / out.mean((0, 1))
    return np.clip(out, 0, 1)


def seam_error(img):
    """How much the texture jumps across its wrap against inside it (1: as much as between any neighbours)."""
    inner = np.abs(np.diff(img, axis=1)).mean() + np.abs(np.diff(img, axis=0)).mean()
    wrap = np.abs(img[:, 0] - img[:, -1]).mean() + np.abs(img[0] - img[-1]).mean()
    return wrap / max(inner, 1e-9)


def tile_fix(img, band=0.12):
    """Makes a texture tile without a visible seam when it has one: blends a band along its edges with the same
    texture shifted by half (whose middle is continuous there). A texture that already tiles is returned as it is."""
    if seam_error(img) < 1.5:
        return img
    h, w = img.shape[:2]
    shifted = np.roll(img, (h // 2, w // 2), (0, 1))
    y = np.minimum(np.arange(h), h - 1 - np.arange(h)) / (band * h)
    x = np.minimum(np.arange(w), w - 1 - np.arange(w)) / (band * w)
    a = np.clip(np.minimum(y[:, None], x[None, :]), 0, 1)
    a = a * a * (3 - 2 * a)
    return img * a[..., None] + shifted * (1 - a[..., None])


def resize(img, size):
    """Resized with Lanczos on a wrapped copy, so the edges still tile."""
    h, w = img.shape[:2]
    big = np.concatenate([img[-h // 8:], img, img[: h // 8]], 0)
    big = np.concatenate([big[:, -w // 8:], big, big[:, : w // 8]], 1)
    sw, sh = (size, size) if isinstance(size, int) else size
    fx, fy = sw / w, sh / h
    im = Image.fromarray(np.clip(big * 255 + 0.5, 0, 255).astype(np.uint8))
    im = im.resize((round(big.shape[1] * fx), round(big.shape[0] * fy)), Image.LANCZOS)
    a = np.asarray(im, np.float64) / 255.0
    oy, ox = round(h // 8 * fy), round(w // 8 * fx)
    return a[oy:oy + sh, ox:ox + sw]


def normal_map(sid, size, strength=1.0, rotate=0, root=None):
    """The source's OpenGL normal map (green up, as glTF has it) resized, its relief scaled by `strength`, as RGB."""
    n = load(sid, "NormalGL", root) * 2 - 1
    n = np.rot90(n, rotate)
    if rotate % 2:
        # turned a quarter: the x and y slopes swap, one of them changing sign
        n = np.stack([-n[..., 1], n[..., 0], n[..., 2]], -1) if rotate % 4 == 1 else np.stack(
            [n[..., 1], -n[..., 0], n[..., 2]], -1)
    n = resize(n * 0.5 + 0.5, size) * 2 - 1
    n[..., :2] *= strength
    n[..., 2] = np.maximum(n[..., 2], 0.05)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    return n * 0.5 + 0.5


def credits():
    """The lines naming every source, for CREDITS.md."""
    out = []
    for sid, (name, digest, _size, date) in SOURCES.items():
        out.append(f"- {sid} ({date}), https://ambientcg.com/a/{sid}, archive {name}, SHA-256 {digest}: "
                   f"{AMBIENTCG['author']}, {AMBIENTCG['licence']}")
    return out


def main(argv):
    if len(argv) in (2, 3) and argv[1] in ("fetch", "check"):
        (fetch if argv[1] == "fetch" else check)(argv[2] if len(argv) == 3 else None)
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
