#!/usr/bin/env python3
"""Builds the shipped test level `assets/levels/Parts_Test/`: a few hundred units of track with every one of our parts
on it or beside it, made only of our own content, so the game can be seen and tested with no copy of the original
(decisions.md, "The game plays out of the box"; crates/game/src/partcheck.rs loads and runs it headless).

The way to the finish runs straight along +X over three sectors; the parts stand on it or in bays beside it:
- sector 1: an extra point and an extra life on the way, boxes, loose balls, a dome, the pusher (P_Modul_01), the
  crate under its stone (P_Modul_34), the stone transformer;
- sector 2: the paper transformer, a fan's updraft (P_Modul_18), the turnstile (P_Modul_17), the flaps (P_Modul_19),
  the sack (P_Modul_26) and the swing (P_Modul_08);
- sector 3: the wooden transformer, the seesaw (P_Modul_30), the tipping plate (P_Modul_41), and in bays over the
  void the plate bridge (P_Modul_29), the drawbridge (P_Modul_25), the flap bridge (P_Modul_37) and the falling room
  (P_Modul_03); then the balloon (PE_Balloon).

Run: python3 -I assets/own/make/make_test_level.py (numpy, Pillow). Deterministic.
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import levelkit as L  # noqa: E402
import ownkit as K  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(os.path.dirname(os.path.dirname(HERE)), "levels", "Parts_Test")


def build():
    b = L.LevelBuilder(OUT, seed=101)
    b.start((0.0, 0.0, 0.0))
    # sector 1: x 10..130, 40 wide
    b.slab("S1_Floor", (10, -L.DEPTH, -20), (130, 0, 20), sector=0)
    b.extra_point(1, (25, 0, 0), 1)
    b.box(1, (40, 0, -12), 1)
    b.box(2, (46, 0, -12), 1)
    b.loose_ball("Wood", 1, (40, 0, 12), 1)
    b.extra_life(1, (52, 0, 0), 1)
    b.loose_ball("Stone", 1, (58, 0, -12), 1)
    b.dome(1, (68, 0, 12), 1)
    b.loose_ball("Paper", 1, (80, 0, -12), 1)
    # the pusher's parts rest on the floor 2.4 under its frame
    b.module("P_Modul_01", 1, K.pose((95, 2.4, -14)), 1)
    # the crate stands on the floor 4.6 under its frame, the stone on it
    b.module("P_Modul_34", 1, K.pose((112, 4.6, 14)), 1)
    b.trafo("Stone", 1, (120, 0, 0), 1)
    b.checkpoint(1, (130, 0, 0))
    # sector 2: x 130..262, 60 wide
    b.slab("S2_Floor", (130, -L.DEPTH, -30), (262, 0, 30), sector=0)
    b.trafo("Paper", 1, (142, 0, 0), 2)
    b.module("P_Modul_18", 1, K.pose((165, 0.6, 12)), 2)
    b.module("P_Modul_17", 1, K.pose((185, 4.5, -18)), 2)
    b.module("P_Modul_19", 1, K.pose((205, 0.5, 15)), 2)
    b.module("P_Modul_26", 1, K.pose((222, 3.0, -15)), 2)
    b.module("P_Modul_08", 1, K.pose((242, 1.2, 15)), 2)
    b.extra_point(2, (200, 0, 0), 2)
    b.checkpoint(2, (260, 0, 0))
    # sector 3: x 262..390, 60 wide, bays over the void on either side
    b.slab("S3_Floor", (262, -L.DEPTH, -30), (390, 0, 30), sector=0)
    b.trafo("Wood", 1, (274, 0, 0), 3)
    # the plate bridge from the floor's edge (z 30) to a pad 14.8 further, its x along +z
    b.module("P_Modul_29", 1, K.pose((290, 0, 36.6), (0, 0, 1)), 3)
    b.slab("Bay_Plates", (283, -L.DEPTH, 44.8), (297, 0, 58), sector=0)
    # the drawbridge stands at the floor's edge (z -30) and falls outward, its x along +z
    b.module("P_Modul_25", 1, K.pose((300, 0, -29.34), (0, 0, 1)), 3)
    b.slab("Bay_Draw", (295, -L.DEPTH, -50), (305, 0, -39.5), sector=0)
    b.module("P_Modul_37", 1, K.pose((330, 0, -27.62), (0, 0, 1)), 3)
    b.slab("Bay_Flap", (324, -L.DEPTH, -58), (336, 0, -46.5), sector=0)
    b.module("P_Modul_30", 1, K.pose((312, 0, -15)), 3)
    b.module("P_Modul_41", 1, K.pose((322, 2.0, 15)), 3)
    # the falling room beside the floor, its gate (its +x) facing the way (-z)
    b.module("P_Modul_03", 1, K.pose((352, 8.6, 40.5), (0, 0, -1)), 3)
    b.extra_point(3, (340, 0, 0), 3)
    b.finish((412, 0, 0), 3)
    b.write("Parts test", "Drahtseilakt", sky="A", start_ball="Ball_Wood", music=1, licence="GPL-3.0-only")
    print(f"{OUT}: {len(b.nodes)} objects, {len(b.meshes)} meshes")


if __name__ == "__main__":
    build()
