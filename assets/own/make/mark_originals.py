#!/usr/bin/env python3
"""Marks what each material of an own-built open-format level stands for in the original (levelkit.ORIGINAL_OF,
open-format.md, a material's `original`), in place, for a level made before levels carried it whose generator is not
run again (Parts_Test). A level levelkit writes now carries it already. Fails on a picture that is neither one of the
original's textures nor own-only, as levelkit does.

usage: mark_originals.py LEVEL_DIR...
"""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import levelkit as L  # noqa: E402


def mark(level_dir):
    path = os.path.join(level_dir, "level.gltf")
    with open(path, encoding="utf-8") as f:
        doc = json.load(f)
    images = [img["uri"] for img in doc.get("images", [])]
    sources = [t["source"] for t in doc.get("textures", [])]
    changed = 0
    for m in doc.get("materials", []):
        index = m.get("pbrMetallicRoughness", {}).get("baseColorTexture", {}).get("index")
        if index is None:
            continue
        uri = images[sources[index]]
        picture = uri[len("textures/"):] if uri.startswith("textures/") else uri
        kind = L.original_of(picture)
        x = m.setdefault("extras", {}).setdefault("drahtseilakt", {})
        x.pop("original", None)
        x.pop("own_only", None)
        if kind[0] == "original":
            x["original"] = {"texture": kind[1], **({"texgen": kind[2]} if kind[2] else {})}
            changed += 1
        elif kind[0] == "own":
            x["own_only"] = True
            changed += 1
    # written to a new file and moved over the old, so a hard link to it elsewhere keeps its own contents
    tmp = path + ".new"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(doc, f, indent=1)
    os.replace(tmp, path)
    print(f"{level_dir}: {changed} materials marked")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    for d in sys.argv[1:]:
        mark(d)
