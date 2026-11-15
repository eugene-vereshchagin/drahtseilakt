#!/usr/bin/env python3
# The converter audit's table (scripts/convert-audit.sh) as markdown: totals before and after, the causes of the
# differences by the maps they hit, and one row a map.
# Usage: python3 -I scripts/convert-audit-report.py BEFORE.tsv AFTER.tsv ROOT... > convert-audit.md
# (ROOT: folder prefixes cut from the map paths in the rows.)
import re
import sys
from collections import Counter


def rows(path):
    out = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        f = line.rstrip("\n").split("\t")
        if len(f) < 7 or f[0] != "convert-audit":
            continue
        out[f[1]] = {"converts": f[2], "loads": f[3], "content": f[4], "behaviour": f[5], "time": f[6]}
    return out


def ok(r):
    return {
        "convert": r["converts"] == "yes",
        "load": r["loads"] == "yes",
        "content": r["content"] == "same",
        "behaviour": r["behaviour"].startswith("same"),
    }


def cause(item):
    """A difference reduced to its kind (names and numbers left out)."""
    s = item.strip()
    s = re.sub(r'"[^"]*"', "X", s)
    s = re.sub(r"Some\(\d+\)|None|\b\d+\b", "", s)
    s = re.sub(r"\(.*\)", "", s)
    s = re.sub(r"\s+", " ", s).replace("->", "").strip(" :")
    return s


def causes(r):
    out = set()
    if r["converts"] != "yes":
        out.add("does not convert: " + cause(r["converts"][4:])[:80])
    elif r["loads"] != "yes":
        out.add("export does not load: " + cause(r["loads"][4:])[:80])
    else:
        if r["content"] != "same":
            for item in r["content"].split(";"):
                if not item.strip().startswith("..."):
                    out.add("content: " + cause(item))
        b = r["behaviour"]
        if b.startswith("differs"):
            out.add("plays differently: " + b.split(":", 1)[1].strip())
        elif not b.startswith("same"):
            out.add("not played: " + cause(b)[:80])
    return out


def totals(rs):
    t = Counter()
    for r in rs.values():
        for k, v in ok(r).items():
            t[k] += v
        t["identical"] += all(ok(r).values())
    return t


def main():
    before, after, roots = rows(sys.argv[1]), rows(sys.argv[2]), sys.argv[3:]
    short = lambda p: next((p[len(r):].lstrip("/") for r in roots if p.startswith(r)), p)
    tb, ta = totals(before), totals(after)
    print("| | before | after |\n|---|---|---|")
    print(f"| maps | {len(before)} | {len(after)} |")
    for k, name in [("convert", "convert"), ("load", "export loads"), ("content", "same contents"),
                    ("behaviour", "play the same"), ("identical", "identical (all four)")]:
        print(f"| {name} | {tb[k]} | {ta[k]} |")
    for label, rs in [("Before", before), ("After", after)]:
        c = Counter()
        for r in rs.values():
            c.update(causes(r))
        print(f"\n### Causes {label.lower()} (maps hit)\n")
        if not c:
            print("none")
        for k, n in c.most_common():
            print(f"- {n} — {k}")
    print("\n### Per map (after)\n")
    print("| map | converts | loads | contents | play (fixed keys) |\n|---|---|---|---|---|")
    esc = lambda s: s.replace("|", "\\|")
    for p in sorted(after):
        r = after[p]
        print(f"| {esc(short(p))} | {esc(r['converts'])} | {esc(r['loads'])} | {esc(r['content'])} | {esc(r['behaviour'])} |")


main()
