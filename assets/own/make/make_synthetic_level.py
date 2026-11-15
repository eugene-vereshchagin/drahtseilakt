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
  sticky), the sack (P_Modul_26), the turning arms (P_Modul_17), the flap wheel (P_Modul_19);
- sector 4, transformers and the mechanics on the way: paper and the fan (P_Modul_18) that lifts it, wood, the
  seesaw (P_Modul_30), the plate bridge (P_Modul_29), the swing (P_Modul_08), stone shoving boxes, sticky, wood;
- sector 5, the puzzle modules in bays beside a wide floor (none blocks the way): the pusher (P_Modul_01), the crate
  under its stone (P_Modul_34), the tipping plates (P_Modul_41), a plate bridge and the drawbridge (P_Modul_25) to
  bays over the void, the flap bridge (P_Modul_37), the falling room (P_Modul_03).
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
import levelkit as L  # noqa: E402
import ownkit as K  # noqa: E402
import synthetic_community as C  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ASSETS = os.path.dirname(os.path.dirname(HERE))
NAME = "00_Синтетический_уровень"
OUT = os.path.join(ASSETS, "levels", "drahtseilakt", NAME)
FONT = os.path.join(ASSETS, "fonts", "NotoSans-Bold.ttf")

POSTED = "2026-11-15"
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
sign "P_Modul_26: swinging sack"
sack side 9 ahead 6
floor 18
sign "P_Modul_17: turning arms"
arms side 8 ahead 6 nopilot
floor 20
sign "P_Modul_19: flap wheel"
flaps side 8 ahead 6
floor 20
width 12
floor 16
floor 8
checkpoint

# ---- sector 4: transformers and the mechanics on the way ---------------------------------------------------------
floor 12
sign "CLASSIC 4: transformers, mechanics" ahead -6
sign "P_Trafo_Paper"
trafo paper
floor 18
sign "P_Modul_18: fan lifts paper"
fan ahead -5
gap 4 up 12 nopilot
floor 22
sign "P_Trafo_Wood"
trafo wood
floor 18
sign "P_Modul_30: seesaw"
seesaw
floor 18
sign "P_Modul_29: plate bridge"
platebridge
floor 18
sign "P_Modul_08: swing"
pilot safe,collector : goto -8 0 0 ; wait 50 ; via 0 0 0 ; via 9 0 0
pilot speed : via 0 0 0 ; via 9 0 0
swingbridge nopilot
floor 20
sign "P_Trafo_Stone: stone shoves boxes"
trafo stone
floor 16
box ahead 4 side -2.4 shove
box ahead 10 side 2.4 shove
floor 20
sign "P_Trafo_Sticky (our fourth ball)"
trafo sticky
floor 20
sign "P_Trafo_Wood"
trafo wood
floor 18
checkpoint

# ---- sector 5: the puzzle modules, in bays beside a wide floor ---------------------------------------------------
floor 12
sign "CLASSIC 5: puzzle modules in bays" ahead -6
# no borders here: the bridges leave from the floor's edges
safety none
width 60
floor 20
mark bays
plaza 13.2 14 ahead 18 side 51.4
plaza 10.5 10 ahead 30 side -44.75
plaza 11.5 12 ahead 59 side -52.25
back bays keepwidth
sign "P_Modul_01: pusher" side -34 ahead 0
module P_Modul_01 ahead 6 side -14 up 2.4
sign "P_Modul_34: crate under stone" side 34 ahead 0
module P_Modul_34 ahead 6 side 14 up 4.6
sign "P_Modul_29: plates to a bay" side 34 ahead 16
module P_Modul_29 ahead 25 side 36.6 turn 90
sign "P_Modul_25: drawbridge" side -34 ahead 26
module P_Modul_25 ahead 35 side -29.34 turn 90
sign "P_Modul_41: tipping plates" side 34 ahead 46
module P_Modul_41 ahead 57 side 15 up 2
sign "P_Modul_37: flap bridge" side -34 ahead 56
module P_Modul_37 ahead 65 side -27.62 turn 90
sign "P_Modul_03: falling room" side 34 ahead 76
module P_Modul_03 ahead 87 side 40.5 up 8.6 turn -90
point ahead 75
floor 110
width 12
floor 20
safety border
floor 10
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

    def run_command(self, cmd, args, opts, flags, line):
        if cmd == "sign":
            self.sign(D.word(args, 0, line), opts)
        elif cmd == "gate":
            self.gate(D.word(args, 0, line))
        elif self.community_command(cmd, args, opts, line):
            pass
        else:
            if cmd == "lighthouse":
                # its trim stands on its own shaft (a floor to the checks): no obstacle of the way's
                self.b.intended.add(f"Lighthouse_{self.count.get('lighthouse', 0) + 1:02d}_Trim")
            super().run_command(cmd, args, opts, flags, line)

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
