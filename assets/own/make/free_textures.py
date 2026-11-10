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
    "Marble012": ("Marble012_1K-JPG.zip", "b04384c6d49ce73baeb70db7ea4abfe9948fc9b74b9fd92c3766970662bf2c43",
                    4672904, "2020-01-26"),
    "RoofingTiles015A": ("RoofingTiles015A_1K-JPG.zip", "5bb040c4c08592b607bba47cbf7384c2769bea1b49d088c463fa296708c0f743",
                           4222671, "2023-12-15"),
    "WoodFloor041": ("WoodFloor041_1K-JPG.zip", "e7364ef32a8db269b2475e76a7e1832eca82bdf6574191cd96b1cdb2b362cdc6",
                       6395278, "2020-06-14"),
    "Bricks053": ("Bricks053_1K-JPG.zip", "8fdff3ad68df3c9aadc47e118c72af639357076af5d835f7986f2999740d5c0f",
                    8202202, "2021-02-14"),
    "Fabric066": ("Fabric066_1K-JPG.zip", "bcd274190979010b2056cab1b10d0420da6e2f58a8e1cf37bc527d8a67772684",
                    9676056, "2021-12-12"),
}
# A palette for levels to come (themes: old stone blocks, worn planks, mossy rock, rope, rusty iron, lava;
# docs/drahtseilakt/decisions.md "Own content by default"): pinned and fetched like the sources above, used by no
# script yet, so nothing is made from them or shipped (and CREDITS.md lists them only once something is).
PALETTE = {
    "Bricks075B": ("Bricks075B_1K-JPG.zip", "d3ca2f8541b94626174829b82b3afba7d102d0a905f28fa084b633a10a827052",
                   5513135, "2021-11-28"),
    "Bricks106A": ("Bricks106A_1K-JPG.zip", "839ca840dd2fbb107e93cc5188c3b94bbd641a639af0c6b002212b28e9ebe898",
                   8178310, "2026-10-20"),
    "PavingStones150": ("PavingStones150_1K-JPG.zip",
                        "bcaffad6b5ae522394a740b698dbf1b478ff9daca465a3acca20ddbc45e1442f", 5336973, "2025-11-03"),
    "Rock064": ("Rock064_1K-JPG.zip", "560f23fdd24f4abe27f5c272b86e69fd026c3d111e22f60d7fb647306830367b",
                8205462, "2026-04-14"),
    "Moss002": ("Moss002_1K-JPG.zip", "727d4e3134f604837c2e11aa3bf12417a022d3c668a6d7339efa572fdbb056bb",
                10391949, "2020-09-13"),
    "Planks037A": ("Planks037A_1K-JPG.zip", "df255df16475bad251f194eb173017c46d5354d572348d435808717c478cf144",
                   5823702, "2023-06-08"),
    "Planks023A": ("Planks023A_1K-JPG.zip", "0d3e5649dcfd41239c9d34c3214c410faec32175d7d77e7de53fba450e33f669",
                   6793089, "2022-01-16"),
    "WoodFloor064": ("WoodFloor064_1K-JPG.zip", "edd177798c9fa641531ef32970418ff9781e3bf3ceb7808257d2b6c09d7d3cc1",
                     4002335, "2025-04-18"),
    "Rope001": ("Rope001_1K-JPG.zip", "0f4daf2c2720b9bd910369cab753090b0ad6560435526ec5aaab293bae5fdbfc",
                4226862, "2018-03-11"),
    "Rope002": ("Rope002_1K-JPG.zip", "872be328ca3cab1fd44b026b4d42a16eaf182c4eb99c0375d0cbfede85fe666e",
                4545886, "2018-03-21"),
    "Rust009": ("Rust009_1K-JPG.zip", "15b3189498576f145c6aed9b0e153e192ff8147f822be26a1f009f2afcdd6938",
                7743455, "2024-08-05"),
    "MetalPlates013": ("MetalPlates013_1K-JPG.zip", "fa2e438b7512470264ebd98c872ca2ea321a0cce4c0ac43264891beec6a21b7d",
                       7022039, "2021-11-21"),
    "Lava004": ("Lava004_1K-JPG.zip", "fee9256befd26b59a7ff452ab6379dab4cdace5d7bee7f78cd9ad8176dd67c28",
                8986332, "2019-12-12"),
}
# (Rock030 and Paper003 made the stone and the paper ball until 2026-11-11; the balls are now drawn, ballskins.py)
PALETTE.update({
    "Rock030": ("Rock030_1K-JPG.zip", "2fd0bc2d214e2487c19062ed0798a85dd135b41cfc12bbaed42d5acae238f989",
                  9677446, "2020-03-29"),
    "Paper003": ("Paper003_1K-JPG.zip", "90b44da4c960ba044b37c26713c50ef1986687a522f1bbe847aeb9b471409040",
                   2545301, "2018-08-30"),
})
ALL = {**SOURCES, **PALETTE}
MAX_ARCHIVE = 64 << 20


def sources_dir(arg=None):
    return arg or os.environ.get("DRAHTSEILAKT_TEXTURE_SOURCES") or os.path.join(os.path.dirname(REPO),
                                                                                  "texture-sources")


def archive_path(sid, root=None):
    return os.path.join(sources_dir(root), sid, ALL[sid][0])


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
    if got != ALL[sid][1]:
        raise SystemExit(f"{path}: SHA-256 {got}, expected {ALL[sid][1]}; refusing it")
    _checked.add(path)
    return path


def fetch(root=None):
    for sid, (name, digest, size, _date) in ALL.items():
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
    for sid in ALL:
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
