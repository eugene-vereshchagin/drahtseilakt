#!/usr/bin/env python3
"""Builds the shipped showcase level `assets/levels/drahtseilakt/00_Синтетический_уровень/` ("Synthetic level"): one
level with every element the game has, to be rolled through in a few minutes and checked by eye: does everything
draw, with the right textures, sound and behave as it should. Our own content only (the kit's track, the game's own
parts, signs drawn here from the game's free fonts); in the Ballance look the game swaps in the copy's textures.

The way, in order (each element has a sign beside it; the classic part first, then a gate, then what community maps
bring):

CLASSIC
- sector 1, floors and surfaces: the gutter profile, the brick border, railings, the bare floor, wood (its own
  sound), a way narrowing and widening, a ramp up and down, two round turns, two hard corners;
- sector 2, rails and scenery: straight rails, rails turning, rails rising and falling; a wooden deck on
  columns; the lamp tower, mast, plinth, pillar, wall block with its lantern, garland and lighthouse;
- sector 3, the parts: extra point, extra life, boxes (one stacked), the dome, the loose balls (wood, stone, paper,
  sticky);
- sector 4, paper and wood on the way: the fan steps (P_Modul_18), the troughs (P_Modul_41), the seesaw (P_Modul_30),
  the plate bridge (P_Modul_29), the swing (P_Modul_08);
- sector 5, stone and the puzzle modules on the way, each as the original's levels set it: the flap wheel
  (P_Modul_19), the board bridge (P_Modul_25), the drawbridge (P_Modul_37), the pushers (P_Modul_01), the crate under
  its stone (P_Modul_34), the lift (P_Modul_03), a row of sacks (P_Modul_26), the turning arms (P_Modul_17).
Each element has `check` lines: checks.txt, run by crates/game/src/showcase.rs.
THE GATE: an arch of brick over the way with "COMMUNITY" on it.
COMMUNITY (synthetic_community.py: its description, commands and scripts): the materials community maps use, and our
own behaviour graphs (the level's scripts, open-format.md) for what their Virtools scripts do; then the balloon.

Run: python3 -I assets/own/make/make_synthetic_level.py [OUT_FOLDER]   (numpy, Pillow, scipy). Deterministic.
"""

import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import leveldsl as D  # noqa: E402
import levelparts as P  # noqa: E402
import levelkit as L  # noqa: E402
import ownkit as K  # noqa: E402
import synthetic_community as C  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ASSETS = os.path.dirname(os.path.dirname(HERE))
NAME = "00_Синтетический_уровень"
OUT = os.path.join(ASSETS, "levels", "drahtseilakt", NAME)
FONT = os.path.join(ASSETS, "fonts", "NotoSans-Bold.ttf")

POSTED = "2026-11-15"
CHECKS_HEADER = """# The showcase level's checks: each element does its job in the game's physics (crates/game/src/showcase.rs runs
# them). Made by assets/own/make/make_synthetic_level.py from its `check` lines; points are world points.

"""
HEADER = """# The showcase level: every element of the game on one way, to be checked by eye. First everything of classic
# Ballance (floors, rails, parts, transformers, modules), then, past the COMMUNITY gate, what community maps bring
# (materials, our own behaviour graphs). Made by assets/own/make/make_synthetic_level.py; our own content only.
"""

# The signs: boards beside the way, their texts drawn into atlases of CELL-sized cells.
ATLAS, CELL = 2048, (512, 128)
SIGN_HALF = (4.4, 1.1)

CLASSIC = """
title Synthetic level
title.ru Синтетический уровень
title.de Synthetischer Level
title.zh 合成关卡
title.fr Niveau synthétique
title.es Nivel sintético
title.it Livello sintetico
order 0
difficulty 1
sky Mountains
music 1
bonus 100
ball wood
columns 60

# ---- sector 1: floors and surfaces -------------------------------------------------------------------------------
safety gutter
start
width 12
sign "CLASSIC 1: floors and surfaces" ahead 4
floor 10
sign "Floor: gutter profile, stone"
floor 26
safety border
sign "Floor: brick border"
floor 26
safety railings
sign "Floor: steel railings"
floor 26
safety none
sign "Floor: bare, kerbs only"
floor 26
surface wood
sign "Floor: wood (wood sound)"
floor 26
surface stone
sign "Width: narrow 8, wide 16"
width 8
floor 14
floor 10
width 16
floor 14
floor 10
width 12
floor 12
sign "Ramp: up 4, down 4"
floor 22 up 4
floor 8
floor 22 down 4
floor 8
railing turns
sign "Round turns, railing outside"
left 90 22
floor 8
right 90 22
railing noturns
floor 14
sign "Hard corners 90"
floor 16
corner right rail
floor 16
corner left rail
floor 16
checkpoint

# ---- sector 2: rails and scenery ---------------------------------------------------------------------------------
safety border
floor 16
sign "CLASSIC 2: rails and scenery" ahead -8
sign "Rails: straight (metal sound)"
rails 30
floor 14
sign "Rails: turning left, right"
rails-left 45 26
rails-right 45 26
floor 14
sign "Rails: rising 3"
rails 24 up 3
floor 14
sign "Rails: falling 3"
rails 24 down 3
floor 14
sign "Rails: spiral down, two turns (level 13)"
check spiral : ball wood -6 0 0 ; go 9 0 0 4 300 ; arc 10 0 0 10 720 20 48 ; go 40 0 -20 4 400 ; reach 40 0 -18 2.5 ; kept
rails 10
rails-left 720 10 down 20
rails 36
floor 12
surface wood
safety none
sign "Wooden deck on columns"
floor 30
surface stone
safety border
floor 10
sign "Lamp tower"
lamptower side 12 ahead 6
floor 16
sign "Mast"
mast side 11 ahead 6
floor 16
sign "Plinth with lantern"
plinth side 13 ahead 6 up -1
floor 16
sign "Pillar"
pillar side 12 ahead 6
floor 16
sign "Wall block"
wallblock side 9 ahead 0 long 22 height 12 obstacle
floor 26
sign "Garland of lamps"
garland side 8 up 9 ahead 0 side2 -8 up2 9 ahead2 0 sag 2.5
mast side 8 ahead 0 height 9.5 obstacle
mast side -8 ahead 0 height 9.5 obstacle
floor 16
sign "Lighthouse"
lighthouse side 34 ahead 8 r 9 height 34 obstacle
floor 20
checkpoint

# ---- sector 3: the parts -----------------------------------------------------------------------------------------
floor 12
sign "CLASSIC 3: parts" ahead -6
width 30
floor 16
sign "P_Extra_Point"
point ahead 6
floor 18
sign "P_Extra_Life"
life ahead 6
floor 18
sign "P_Box: boxes, one stacked"
box side 9 ahead 4
box side 9 ahead 10
box side 9 ahead 7 up 5
box side -9 ahead 7
floor 20
sign "P_Dome: bumper"
dome side 0 ahead 10
pilot all : goto 0 9 0 ; goto 10 10.5 0 ; goto 20 9 0 ; goto 26 0 0
nopilot all
floor 26
autopilot all
sign "P_Ball_Wood / P_Ball_Stone"
loose wood side 9 ahead 6
loose stone side -9 ahead 6
floor 18
sign "P_Ball_Paper / P_Ball_Sticky"
loose paper side 9 ahead 6
loose sticky side -9 ahead 6
floor 18
floor 20
checkpoint

# ---- sector 4: paper (fans, troughs), then wood (seesaw, plates, swing) -------------------------------------------
# each element as the original's levels set it, the way going through it (no way round)
floor 12
sign "CLASSIC 4: paper, then wood" ahead -6
sign "P_Trafo_Paper"
check fans : ball wood -8 0 0 ; go 0 0 0 3 400 ; wait 150 ; kind paper ; go 41.5 0 0 4 900 ; wait 40 ; go 62 0 11 5 900 ; go 66.5 0 11 4 900 ; wait 10 ; go 82 0 25 6 900 ; reach 82 0 25 3 ; kept
check fans-wood : ball wood 20 0 0 ; go 41.5 0 0 4 900 ; wait 120 ; go 62 0 11 5 900 ; away 62 0 11 4
trafo paper
floor 16
safety none
width 6
floor 12
sign "P_Modul_18: fans lift paper up 11, then 14 (level 3)"
floor 6
columns 0
fanpad 15
# level 3: rails 11 up from just past the fan (3 there, 5 here: our headroom rule) over the 10 gap to the next platform
move -2.5 0 11
rails 12.5
fanpad 15
# level 3: the next floor stands right at the platform's far edge, 14 up (a wall the updraft lifts the ball over)
move 0 0 14
block 8 14
floor 16
columns 60
sign "P_Modul_41: troughs, paper only (level 7)"
check troughs-paper : ball paper 0 0 0 ; go 52 0 0 4 900 ; reach 52 0 2 3 ; kept
check troughs-wood : ball wood 0 0 0 ; go 52 0 0 4 600 ; away 52 0 2 6 ; turned P_Modul_41_01 P_Modul_41 20
floor 6
troughs 6
floor 14
width 12
floor 10
safety border
floor 6
sign "P_Trafo_Wood"
trafo wood
floor 18
sign "P_Modul_30: seesaw over a 12.9 gap (levels 2-9)"
check seesaw : ball wood -12 0 0 ; go 28 0 0 6 900 ; reach 28 0 0 3 ; kept
seesaw
floor 18
sign "P_Modul_29: plate bridge over 15.4 (levels 2-10)"
check plates : ball wood -12 0 0 ; go 30 0 0 5 900 ; reach 30 0 0 3 ; kept
check plates-stone : ball stone -12 0 0 ; go 30 0 0 5 900 ; lost
platebridge
floor 18
sign "P_Modul_08: swing over a trench (level 8)"
check swing : ball wood -8 0 0 ; wait 120 ; go 32 0 0 5 900 ; reach 32 0 0 3 ; kept
# level 8: the swing hangs over a trench 6.2 deep under the gap, its floor about as wide as the gap is long
plaza 18 15 up -6.2
block 15 8.7 width 3 side 10.5 top 0
block 15 8.7 width 3 side -10.5 top 0
pilot safe,collector : goto -8 0 0 ; wait 50 ; via 0 0 0 ; via 9 0 0
pilot speed : via 0 0 0 ; via 9 0 0
swingbridge nopilot
floor 20
checkpoint

# ---- sector 5: stone and the puzzle modules, each in its own setting on the way -----------------------------------
floor 12
sign "CLASSIC 5: stone and the puzzles" ahead -6
sign "P_Trafo_Stone: stone shoves boxes"
trafo stone
floor 16
box ahead 4 side -2.4 shove
box ahead 10 side 2.4 shove
floor 20
nopilot all
columns 0
safety none
width 6
floor 10
sign "P_Modul_19: flap wheel, stone tips it (levels 9-11)"
# (levels 9-11: the stone ball comes at the standing flap rolling, 7 or more; from rest against it, it holds)
check flaps-stone : ball stone 0 0 0 ; go 24 0 0 4 900 ; go 34.8 0 0 4 600 ; go 34.8 15 0 4 600 ; go 0 15 0 10 900 ; reach 0 15 2 3 ; turned P_Modul_19_01 P_Modul_19_Flaps 60 ; kept
check flaps-wood : ball wood 0 0 0 ; go 24 0 0 4 900 ; go 34.8 0 0 4 600 ; go 34.8 15 0 4 600 ; go 0 15 0 10 900 ; away 0 15 2 6 ; still P_Modul_19_01 P_Modul_19_Flaps 30
floor 10
flaps
floor 18.4
corner left
floor 15
corner left
floor 24.49
flapslot
floor 8
drop 10
corner right
floor 20
corner right
width 6
sign "P_Modul_25: board over a 9.1 gap (level 3)"
check board-stone : ball stone 0 0 0 ; go 32 0 0 4 900 ; reach 32 0 2 3 ; turned P_Modul_25_01 P_Modul_25_Bridge 60 ; kept
check board-wood : ball wood 0 0 0 ; go 32 0 0 4 900 ; reach 32 0 2 3 ; turned P_Modul_25_01 P_Modul_25_Bridge 60 ; kept
floor 14
boardbridge
floor 14
sign "P_Modul_37: drawbridge into a lane (level 5)"
check drawbridge-stone : ball stone 0 0 0 ; go 21 0 0 6 600 ; wait 120 ; turned P_Modul_37_01 P_Modul_37_Bridge 60 ; go 8 -2 0 4 600 ; go 8 -12 0 4 600 ; go 36.95 -12 0 4 900 ; go 36.95 12 0 7 900 ; reach 36.95 12 2 3 ; kept
check drawbridge-wood : ball wood 0 0 0 ; go 21 0 0 6 600 ; wait 120 ; still P_Modul_37_01 P_Modul_37_Bridge 30
floor 8
mark spur
floor 14
hingebridge
back spur
move 0 -2.5 0
turn -90
floor 9.5
corner left
floor 28.95
corner left
floor 8.2
gap 7.6 nopilot
floor 8.2
corner right
floor 14
sign "P_Modul_01: two pushers, from the low floor (level 1)"
check pushers : ball stone 0 0 0 ; go 8 -8 0 4 400 ; go 24.8 -15 0 4 600 ; go 24.8 -2 0 5 300 ; go 24.8 -15 0 4 400 ; go 34.8 -15 0 4 400 ; go 34.8 -2 0 5 300 ; go 34.8 -15 0 4 400 ; go 6 -8 0 4 600 ; go 6 0 0 4 300 ; go 50 0 2.5 4 900 ; reach 50 0 4.5 3 ; moved P_Modul_01_01 P_Modul_01_Pusher 3 ; moved P_Modul_01_02 P_Modul_01_Pusher 3 ; kept
check pushers-block : ball stone 0 0 0 ; go 50 0 2.5 4 600 ; away 50 0 4.5 8
floor 10
plaza 14 42 ahead -6 side -10
move 0 -10 0
drop 25
move 0 10 0
floor 10 up 2.5
floor 4
pusher right
floor 8.4
pusher right
floor 16
sign "P_Modul_34: crate under the stone, from below (level 1)"
# level 1: a lane (5 wide there, 6 here: our narrowest way) over a broad floor 5 lower (34 along the lane, 42 across);
# the stone stands in a slot 5.3 long, on its crate on the floor below; a stone ball rolling across that floor under
# the slot knocks the crate out (it stays on the broad floor), the stone sinks level with the lane and shuts the
# way back under it, and the ball goes up a ramp on the floor's far side
check crate : ball stone 0 0 0 ; go 6 -2 0 4 300 ; go 6 -12 0 4 400 ; go 26.65 -12 -5 4 600 ; go 26.65 7 -5 7 900 ; wait 60 ; go 36 12 -5 4 600 ; go 26.65 22 -5 4 600 ; go 6 22 0 4 600 ; go 6 0 0 4 300 ; go 42 0 0 4 900 ; reach 42 0 2 3 ; moved P_Modul_34_01 P_Modul_34_Kiste 4 ; lies P_Modul_34_01 P_Modul_34_Kiste 26.65 8 -5 18 ; kept
check crate-block : ball stone 0 0 0 ; go 42 0 0 4 600 ; away 42 0 2 6
floor 6
mark crate
floor 18
crateblock
floor 16
mark past
back crate
move 0 -2.5 0
turn -90
floor 9.5
corner left
width 6
floor 20.65 down 5
corner left
floor 2
plaza 34 30
back crate
move 0 2.5 0
turn 90
floor 19.5
corner right
width 6
floor 20.65 down 5
corner right
floor 2
back past
width 6
floor 10
sign "P_Trafo_Wood"
trafo wood
sign "P_Modul_03: lift, wood rams its walls off (levels 7-12)" ahead 8
# levels 7, 9, 10, 12: a wood ball comes on a low floor 11.3 under the frame (its transformer 23 before the frame) into
# the cage's gate, 11 from its middle; rammed off, the panels fall and the disc rises with the ball to the way on, 8.9
# higher on the far side (level 9)
check lift : ball wood 4 0 0 ; go 23 0 0 3 600 ; wait 60 ; go 34 0 0 6 60 ; go 23 0 0 3 300 ; wait 30 ; go 30 8 0 6 60 ; go 23 0 0 3 300 ; wait 30 ; go 30 -8 0 6 60 ; go 23 0 0 3 300 ; wait 30 ; go 23 11 0 6 60 ; go 23 0 0 3 300 ; wait 30 ; go 23 -11 0 6 60 ; go 23 0 0 3 300 ; wait 30 ; go 16 8 0 6 60 ; go 23 0 0 3 300 ; wait 30 ; go 16 -8 0 6 60 ; go 23 0 0 3 300 ; wait 30 ; wait 300 ; moved P_Modul_03_01 P_Modul_03_Wall01 20 ; moved P_Modul_03_01 P_Modul_03_Wall02 20 ; moved P_Modul_03_01 P_Modul_03_Wall03 20 ; moved P_Modul_03_01 P_Modul_03_Wall04 20 ; moved P_Modul_03_01 P_Modul_03_Wall05 20 ; moved P_Modul_03_01 P_Modul_03_Wall06 20 ; moved P_Modul_03_01 P_Modul_03_Wall07 20 ; moved P_Modul_03_01 P_Modul_03_Floor 5 ; go 46 0 8.9 4 600 ; reach 46 0 8.9 3 ; kept
floor 12
module P_Modul_03 ahead 11 up 11.3 turn 180
width 11
move 22 0 8.9
floor 6
width 6
sign "P_Modul_26: sacks swing across the lane (levels 9-13)"
# level 9 (on from its lift), 10, 12, 13: sacks in a row along a lane 5 wide (or rails), 10 apart, hung 2.8 over it
check sacks : ball wood 0 0 0 ; go 54 0 0 4 900 ; reach 54 0 0 3 ; turned P_Modul_26_01 P_Modul_26_Sack 10 ; kept
check sacks-paper : ball paper 19 0 0 ; go 21 0 0 2 100 ; wait 400 ; lost
sack ahead 21
sack ahead 31
sack ahead 41
floor 54
sign "P_Modul_17: turning arms round a hub (levels 3, 7, 9, 13)"
# level 9: the hub stands in a square loop of lanes 5 wide, 10 from it (the arms reach 11.6), the way going round
# three of its sides; at rest the arms lie across the lanes either side of the hub
check arms : ball wood 0 0 0 ; go 30 0 0 5 600 ; go 30 -20 0 4 600 ; go 18 -20 0 4 600 ; go 18 -34 0 4 600 ; reach 18 -34 0 3 ; turned P_Modul_17_01 Modul17_Dreharme 30 ; kept
plaza 5 5 ahead 17.5 side -10 up 0.67
module P_Modul_17 ahead 20 side -10 up 5 turn 90
floor 30
corner right
floor 20
corner right
floor 12
corner left
floor 14
corner left
width 12
floor 8
safety border
floor 10
sign "P_Trafo_Sticky (our fourth ball)"
trafo sticky
floor 20
sign "P_Trafo_Wood"
trafo wood
floor 18
autopilot all
columns 60
checkpoint
"""

GATE = """
# ---- the gate ------------------------------------------------------------------------------------------------------
floor 14
gate "COMMUNITY"
floor 14
"""

FINISH = """
floor 30
finish
"""


class Show(C.Community, D.Track):
    """The description's cursor with the showcase's own commands: `sign "TEXT"` (a board beside the way), `gate "TEXT"`
    (an arch over it)."""

    def __init__(self, out_dir):
        super().__init__(out_dir)
        self.signs = []
        self.checks = []

    def run_command(self, cmd, args, opts, flags, line):
        if cmd == "check":
            self.check(args, line)
        elif cmd == "sign":
            self.sign(D.word(args, 0, line), opts)
        elif cmd == "gate":
            self.gate(D.word(args, 0, line))
        elif cmd == "block":
            self.block(D.num(args, 0, line), D.num(args, 1, line), opts)
        elif self.community_command(cmd, args, opts, line):
            pass
        else:
            if cmd == "lighthouse":
                # its trim stands on its own shaft (a floor to the checks): no obstacle of the way's
                self.b.intended.add(f"Lighthouse_{self.count.get('lighthouse', 0) + 1:02d}_Trim")
            super().run_command(cmd, args, opts, flags, line)

    # --- checks ---------------------------------------------------------------------------------------------------

    def check(self, args, line):
        """`check NAME : STEP ; STEP ...`: a check that an element does its job in the game's physics, run by
        crates/game/src/showcase.rs from the level's checks.txt (points about the cursor, as `F S U`, written as world
        points): `ball KIND F S U` (begin there at rest), `go F S U V T` (roll to the point at up to V, at most T
        ticks), `arc F S U R A H N` (N points along a turn of radius R, A degrees to the left, H down, from F S U on
        the heading, rolled through at 4), `wait T`, and what must hold then: `reach F S U R` (the ball within R),
        `away F S U R` (not within R), `kind KIND`, `turned OBJECT PART DEG` (the module's part turned at least DEG
        from its start), `still OBJECT PART DEG` (less), `moved OBJECT PART D`, `lies OBJECT PART F S U R` (the part within R of the point), `lost` (a ball was lost), `kept` (none
        was)."""
        text = " ".join(args)
        if ":" not in text:
            raise D.LevelError(f"line {line}: check NAME : STEPS")
        name, body = (x.strip() for x in text.split(":", 1))
        out = [f"check {name}"]
        P = lambda w: self.at(float(w[0]), float(w[1]), float(w[2]))  # noqa: E731
        for st in (x.split() for x in body.split(";")):
            if not st:
                continue
            v, w = st[0], st[1:]
            if v == "ball":
                out.append(f"ball {w[0]} {D.fmt(P(w[1:4]))}")
            elif v in ("go",):
                out.append(f"go {D.fmt(P(w[0:3]))} {float(w[3]):g} {int(w[4])}")
            elif v in ("reach", "away"):
                out.append(f"{v} {D.fmt(P(w[0:3]))} {float(w[3]):g}")
            elif v == "arc":
                f0, s0, u0, r, a, h, n = (float(x) for x in w)
                f, lft = self.fwd(), self.left()
                start = self.at(f0, s0, u0)
                centre = start + lft * r * (1 if a > 0 else -1)
                for k in range(1, int(n) + 1):
                    t = math.radians(a) * k / n
                    # the point turned t about the centre, dropping h * k / n
                    if a > 0:
                        p = centre - lft * r * math.cos(t) + f * r * math.sin(t)
                    else:
                        p = centre + lft * r * math.cos(t) + f * r * math.sin(-t)
                    p = p - np.array([0.0, h * k / n, 0.0])
                    out.append(f"go {D.fmt(p)} 4 600")
            elif v in ("wait",):
                out.append(f"wait {int(w[0])}")
            elif v in ("kind", "lost", "kept"):
                out.append(" ".join([v] + w))
            elif v in ("turned", "still", "moved"):
                out.append(f"{v} {w[0]} {w[1]} {float(w[2]):g}")
            elif v == "lies":
                out.append(f"lies {w[0]} {w[1]} {D.fmt(P(w[2:5]))} {float(w[5]):g}")
            else:
                raise D.LevelError(f"line {line}: unknown check step {v}")
        self.checks.append("\n".join(out))

    # --- blocks ---------------------------------------------------------------------------------------------------

    def block(self, length, height, opts):
        """`block L H [width W] [ahead A] [side S] [top T]`: a solid of the floors' stone from the cursor (or A on, S to
        the left) L along the heading, the way's width (or W) across, from under the floor's slab (or from T over the
        cursor) H down: the wall a floor stands on where it rises straight from a lower one (level 3's floor 14 over
        its second fan's platform), a trench's side."""
        up = np.array([0.0, 1.0, 0.0])
        f, lft = self.fwd(), self.left()
        w = float(opts.get("width", self.width))
        a = float(opts.get("ahead", 0.0))
        top = float(opts["top"]) if "top" in opts else -L.DEPTH
        c = self.at(a + length / 2, float(opts.get("side", 0.0)), top - height / 2)
        m = K.Mesh("block")
        mats = self.b.mat_ids
        P.solid(m, mats["top"], mats["side"], c, (f, up, lft), (length / 2, height / 2, w / 2))
        self.b.add(f"Block_{self.n('block'):02d}", m, groups=self.b._floor_groups("stone") + self.b.sector(self.sector))

    # --- signs ----------------------------------------------------------------------------------------------------

    def sign_uv(self, text):
        """The text's cell in the signs' atlases: (material, (u0, v0, u1, v1))."""
        if text not in self.signs:
            self.signs.append(text)
        i = self.signs.index(text)
        cols, rows = ATLAS // CELL[0], ATLAS // CELL[1]
        page, k = divmod(i, cols * rows)
        r, c = divmod(k, cols)
        mat = self.b.mat(f"signs{page}", f"Signs_{page}.png", diffuse=0.25, emissive=0.9)
        return mat, (c / cols, r / rows, (c + 1) / cols, (r + 1) / rows)

    def board(self, m, text, centre, right, up, half):
        """A board with `text` on its face (seen with `right` to the reader's right), plain wood behind."""
        mat, (u0, v0, u1, v1) = self.sign_uv(text)
        c, r, u = (np.asarray(v, np.float64) for v in (centre, right, up))
        n = np.cross(u, r)  # toward the reader (left-handed: up x right faces back along the view)
        hw, hh = half
        q = [c - r * hw - u * hh, c + r * hw - u * hh, c + r * hw + u * hh, c - r * hw + u * hh]
        m.face(mat, [p + n * 0.13 for p in q], [(u0, v1), (u1, v1), (u1, v0), (u0, v0)], normal=n)
        K.obox(m, self.b.mat_ids["wood"], tuple(c), (tuple(r), tuple(u), tuple(n)), (hw + 0.25, hh + 0.25, 0.12))

    def sign(self, text, opts):
        """A sign on a post beside the way at the cursor: on the right by default (`side`: its place across the way),
        facing the ball that comes along the way, turned a little toward the way."""
        side = float(opts["side"]) if "side" in opts else -(self.width / 2 + 5.2)
        base = self.at(float(opts.get("ahead", 0.0)), side, float(opts.get("up", 0.0)))
        sgn = 1.0 if side > 0 else -1.0
        # the reader looks along the heading: the board's right is the way's right, turned toward the way
        a = math.radians(self.head + 22.0 * sgn)
        fwd = np.array([math.cos(a), 0.0, math.sin(a)])
        right = -np.array([-fwd[2], 0.0, fwd[0]])
        up = np.array([0.0, 1.0, 0.0])
        m = K.Mesh("sign")
        K.cylinder(m, self.b.mat_ids["wood"], tuple(base - up * 3.0), up, 0.2, 7.0, seg=8)
        self.board(m, text, base + up * 5.0 - fwd * 0.3, right, up, SIGN_HALF)
        self.b.add(f"Sign_{self.n('sign'):03d}", m)

    def gate(self, text):
        """An arch of brick over the way at the cursor, the text on both of its faces: only drawn (its posts stand
        outside the way)."""
        b = self.b
        up = np.array([0.0, 1.0, 0.0])
        f, lft = self.fwd(), self.left()
        w = self.width / 2 + 2.0
        m = K.Mesh("gate")
        for sgn in (-1.0, 1.0):
            K.obox(m, b.mat_ids["border"], tuple(self.at(0, sgn * w, 3.5)), (tuple(f), tuple(up), tuple(lft)),
                   (1.5, 6.5, 1.5), scale=L.BRICK)
        K.obox(m, b.mat_ids["border"], tuple(self.at(0, 0, 11.5)), (tuple(f), tuple(up), tuple(lft)),
               (1.5, 1.5, w + 1.5), scale=L.BRICK)
        self.board(m, text, self.at(-1.6, 0, 11.5), -lft, up, (6.0, 1.5))
        self.board(m, text, self.at(1.6, 0, 11.5), lft, up, (6.0, 1.5))
        b.add(f"Gate_{self.n('gate'):02d}", m)


def render_signs(t):
    """Draws the signs' atlases (light letters on a dark board) into the level's textures."""
    from PIL import Image, ImageDraw, ImageFont
    cols, rows = ATLAS // CELL[0], ATLAS // CELL[1]
    per = cols * rows
    for page in range((len(t.signs) + per - 1) // per):
        img = Image.new("RGB", (ATLAS, ATLAS), (24, 30, 38))
        d = ImageDraw.Draw(img)
        for k, text in enumerate(t.signs[page * per:(page + 1) * per]):
            r, c = divmod(k, cols)
            x0, y0 = c * CELL[0], r * CELL[1]
            d.rectangle([x0 + 4, y0 + 4, x0 + CELL[0] - 5, y0 + CELL[1] - 5], outline=(236, 196, 96), width=3)
            size = 56
            while size > 14:
                font = ImageFont.truetype(FONT, size)
                box = d.textbbox((0, 0), text, font=font)
                if box[2] - box[0] <= CELL[0] - 36:
                    break
                size -= 2
            d.text((x0 + CELL[0] / 2, y0 + CELL[1] / 2), text, font=font, fill=(244, 240, 228), anchor="mm")
        img.save(os.path.join(t.b.tex, f"Signs_{page}.png"), optimize=True)


def hook(t):
    t.community_finish()
    render_signs(t)
    # the elements' checks (crates/game/src/showcase.rs), world points
    with open(os.path.join(t.b.out, "checks.txt"), "w", encoding="utf-8") as f:
        f.write(CHECKS_HEADER + "\n\n".join(t.checks) + "\n")


def build(out=OUT):
    os.makedirs(out, exist_ok=True)
    lines = (CLASSIC + GATE + C.LINES + FINISH).splitlines()
    D.compile_level(os.path.join(out, "level.lvl"), lines=lines, track=Show, hook=hook,
                    extra={"posted": POSTED})
    # what the level is, for whoever opens its folder
    path = os.path.join(out, "level.txt")
    with open(path, encoding="utf-8") as f:
        text = f.read()
    with open(path, "w", encoding="utf-8") as f:
        f.write(HEADER + text)


if __name__ == "__main__":
    try:
        build(*sys.argv[1:2])
    except D.LevelError as e:
        print(e, file=sys.stderr)
        sys.exit(1)
