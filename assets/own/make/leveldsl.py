#!/usr/bin/env python3
"""Compiles a level description (`level.lvl`, a small text language) into a level of the open format beside it
(docs/drahtseilakt/levels.md, "The level kit"), with levelkit.py, and writes the test player's hint
files for it (one a style: `pilot/safe.txt`, `pilot/speed.txt`, `pilot/collector.txt`; docs/drahtseilakt/autopilot.md).

A description lays a track like a turtle: a cursor (a point on the floor's top and a heading) moves on with each
piece, and parts are put where the cursor is. The heading starts along +X (the camera's view at the start); "left" is
+Z there. A line is a command and its words; `#` begins a comment.

    title NAME...            the level's name (also title.ru, title.de, title.zh: its name in another language)
    order N                  its place in the list of the shipped levels
    difficulty N             how hard it is, 1 (easy) to 10 (level.txt's difficulty; the menu sorts by it)
    author NAME... / sky NAME / music 1..5 / bonus N / ball wood|stone|paper|sticky / light R G B
                             (sky: A, B, C (above the clouds), Mountains, Forest, Sea, Storm, Sunset, Night, Desert,
                             Mist, Clouds; any Sky_<NAME>_* in the textures)
    drop [R]                 an intended landing at the cursor: floors within R (20) of it may be reached by a fall or a
                             flight from earlier in the route (levelcheck.py, check_shortcuts)
    obstacle                 (an option of lamptower, mast, plinth, wallblock, lighthouse, garland) the object stands in
                             the ball's way on purpose (levelcheck.py, check_scenery)
    clouds off | clouds [cover C] [below B] [thickness T] [size S]   the sea of cloud under the level (the game
                             draws it): none, or the sky's (levelkit.py, CLOUD_SEA) with its cover (0..1), its tops B
                             under the lowest floor, T deep, a cloud S across
    start [X Y Z]            the start's platform, a bowl the ball sits in (its neck as wide as the first floor); the
                             cursor at its front edge (10 ahead of its middle)
    width W                  the floor's width from here on (with its borders); a change tapers over the next floor
                             that runs on from the last (one after rails, a gap or a move begins at W); a way must
                             leave the ball 5.5 across at its middle's height (levelkit.MIN_FREE), or nothing is made
    columns D                columns under the floors every D units of way (0: none), for the look
    safety railings|gutter|border|none   how the floors keep the ball on from here on (levelkit.py, SAFETY); the
                             first in a file is the level's
    borders off|on           bare floors from here on / back to the level's safety
    surface stone|wood       the floor's top and its sound
    floor L [up H|down H]    a straight floor L long, rising or falling H over it
    left A R [up H|down H]   a floor turning A degrees to the left on a radius R (right A R: to the right)
    rails L [up H|down H] [ties]   a pair of rails L long (rails-left A R, rails-right A R: turning); with `ties`,
                             ties across between the tubes (a harder ride, for later levels)
    gap L [up H|down H] [runup R]  the cursor jumps L ahead (and H up or down): nothing built; the test player
                             takes it at speed (the careful styles with a run-up of R, 14 if not given)
    rebate L D               a floor L long, D lower than the cursor (which stays at its height): the ledge a deck
                             lies in flush with the way, either side of the gap it falls into (the drawbridge's lane,
                             level 5: 1 long, 0.5 deep, the deck's thickness)
    move F S U               the cursor moves F ahead, S to the left, U up (nothing built)
    turn A                   the heading turns A degrees to the left (negative: right), the cursor stays
    mark NAME / back NAME    remember the cursor / go back to it
    checkpoint               a checkpoint at the cursor (a piece of its own 11 long with its bowl and flame cups, taking
                             5.5 of the floors before and after it; the way into it at most 14 wide): the next sector begins
    finish                   the balloon, 22 ahead of the cursor (the floor must end at the cursor)
    point, life, box, dome, trafo KIND, loose KIND   a part at the cursor;  options: side S (to the left), ahead A
                             (a box also up U: on another box, `up 5`);
                             a box or a loose ball must leave room for the ball beside it, or be marked `shove`
                             (the ball the way has there, no lighter than it, pushes it away); a loose ball is
                             rested on the floor under it (levelkit.rest_loose), and refused with nothing under it
    column [side S] [ahead A] [depth D]   a stone column under the floor, for the look
    railing left|right|both|off   railings along the floors' edges from here on, on any profile (safety railings
                             gives both on the bare floor; another safety none); a railing on a turn's inner side
                             alone is refused
    railing turns|noturns    a railing on the outer side of every turn from here on, or no more
    crateblock               the stone on the crate (P_Modul_34) in a slot of 6 across the way: the stone blocks
                             the way until a ball on a floor 5 lower (laid by the description, crossing under the
                             slot) pushes the crate out from under it; then it lies level with the way
    pusher left|right        a pusher (P_Modul_01) in a slot of 1.6 across the way: its block fills the way (at
                             most 6 wide), its target plate stands 5 out past the way's edge on that side, over a
                             floor 2.5 lower (laid by the description) from where a ball pushes the block out
    smooth                   the careful styles' hints roll through the way's points (`pass`, no stop at each; a
                             hard corner at a set speed)
    corner left|right [rail] [gap G]   a hard 90 degree corner at the end of a floor (a mitred ring: the outer corner
                             filled, the profile unbroken); `rail`: a railing on its outer side 8 either side, open
                             for G along the way straight on
    plaza W L [brick|kerb] [stone|wood] [gap SIDE C W]... [fence SIDE...]   a flat square from the middle of its back
                             edge (side/ahead/up move it), a rim round it but at its gaps, railings on the fence sides
    disc R [rim] [ring [gap W]]   a round floor, a railing ring open at both ends;  pit L [depth D] [wall LEN] [side S]
                             a pit in the way with a stop wall;  mark NAME / back NAME also begin and end a branch
    lamptower, mast, plinth, wallblock, wlamp, lighthouse, garland   free standing scenery (options side, ahead, up, r,
                             height, long, thick; garland: ahead2 side2 up2 sag);  scatter D [pillars]: distant scenery
                             about every D units of way (pillars: plain stone pillars and plinths only)
    loose KIND lead|plug     a ball the way's ball leads (rolls on ahead), a body that plugs the way: no room check
    seesaw                   a seesaw over a gap of its length (14.8): the cursor goes over it
    platebridge              a plate bridge over a gap of 14.8 (a stone ball breaks it)
    swingbridge [ahead G]    a swing hanging in a gap of G (5) that it bridges as it swings across the way
    fan [ahead A] [side S]   a fan's updraft (a paper ball rises on it to about 17 over its floor)
    sack, arms  [ahead A] [side S]   a swinging sack, turning arms on the floor
    flaps                    the flap wheel (P_Modul_19) over a gap of a flap's length (6.4) in a way about 6 wide:
                             one flap lies flat over the gap at the floor's top (the way goes on over it, the cursor
                             too), the other stands up beside the way, 15 to the left
    module NAME [side S] [ahead A] [up U] [turn A] [tilt T]  any module, its frame at the cursor (x along the
                             heading, turned A to the left and tilted T up)
    pilot STYLES : STEP ; STEP    the test player's steps here (STYLES: safe,speed,collector or all), points as
                             `F S U` about the cursor: goto F S U [T] | via F S U [T] | push F S T | air F S T |
                             wait T | become KIND | turn N | shove F S U F2 S2 U2 T | lead F S U D T |
                             press F S U OF OS V T | pass F S U V [T] | hold F S U T
    nopilot STYLES           no automatic steps from here (until `autopilot STYLES`)

Run: python3 -I assets/own/make/leveldsl.py DIR/level.lvl ...
(The showcase level is made through compile_level by make_synthetic_level.py; the ten levels of our own once written in
this language were removed on 2026-11-16, decisions.md.)
Deterministic; needs Python 3 with numpy and Pillow (levelkit.py, ownkit.py beside it).
"""

import math
import os
import re
import shlex
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import levelkit as L  # noqa: E402
import levelparts as P  # noqa: E402
import ownkit as K  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
STYLES = ("safe", "speed", "collector")
LANGS = ("ru", "de", "zh", "fr", "es", "it")
BALLS = {"wood": "Wood", "stone": "Stone", "paper": "Paper", "sticky": "Sticky"}
# The masses the game gives the balls, the loose balls and a box (crates/game/src/sim.rs): who can shove whom.
BALL_MASS = {"wood": 1.9, "stone": 10.0, "paper": 0.2, "sticky": 4.0}
LOOSE_MASS = {"wood": 2.0, "stone": 10.0, "paper": 0.2, "sticky": 4.0}
BOX_MASS = 1.0
# A dome's radius (13.3 across, measured on the original's).
DOME_R = 6.65
# No loose ball nearer than this to the end of rails.
RAIL_KEEP = 14.0
# The test player's automatic steps: a point every this many units along the way, by style.
SPACING = {"safe": 9.0, "speed": 14.0, "collector": 9.0}
# the speeds the careful styles roll through the way's points at, and through a hard corner (hint step `pass`)
PASS_V = {"safe": 8.0, "collector": 8.0}
CORNER_V = 5.0
# The slots the stone on the crate and a pusher stand in (crateblock, pusher): the stone is 5 on a side (its slot 5.3, as
# level 1 has it), the pusher's beam 1.2 wide in its channel.
# How far a floor's end may be from a checkpoint piece's edge and still join it (the path drifts by a few thousandths).
JOIN_TOL = 0.02
CORNER_RAIL = 8.0
# Commands between a stone corner and the floor out of it that leave its railing to that floor (they lay nothing).
KEEPS_CORNER_RAIL = ("floor", "width", "sign", "check", "pilot", "nopilot", "autopilot", "smooth", "columns", "drop",
                     "surface", "mark")
CRATE_SLOT = 5.3
PUSHER_SLOT = 1.6
# The flap wheel (P_Modul_19, flaps): the gap its flat flap lies over; its frame this far on from the gap's near edge
# and this far to the left of the way's middle, so that the flat flap fills the gap (the part's measured pivots).
FLAP_LEN = 6.4
FLAP_BACK = 3.24
FLAP_SIDE = 10.0
# `flaps stand`: the standing flap on the way (the frame 5 to the right), the flat one on a lane 15 to the right
FLAP_STAND_SIDE = -5.0
# the fan's own block (levelkit.fan_block)
FAN_BLOCK = 5.0
# the troughs' row (P_Modul_41; the original's level 7: the first 4 past the floor's end, 6 apart, the floor on 3 past)
TROUGH_FIRST, TROUGH_STEP, TROUGH_LAST = 4.0, 6.0, 3.0
# the board bridge's gap (P_Modul_25; the far floor ends 9.1 from the frame in all of the original's levels)
BOARD_GAP = 9.1
# the drawbridge (P_Modul_37; the original's level 5): its frame 1.2 past the spur's end, the crossing lane's middle
# 13.75 past the frame, its gap 7.6 long
HINGE_BACK, HINGE_LANE, HINGE_GAP = 1.2, 13.75, 7.6
# A hint's steps at most (hints.rs takes 64).
HINT_STEPS = 60


class LevelError(Exception):
    pass


class Pilot:
    """The test player's steps by style, in the order the way goes, cut into sectors."""

    def __init__(self):
        self.sectors = []  # [(trigger point, sector number, {style: [steps]})]
        self.auto = {s: True for s in STYLES}
        # `smooth`: the careful styles roll through the way's points (hint step `pass`) instead of stopping at each
        self.smooth = False
        self.last = {s: None for s in STYLES}

    def begin(self, at, sector):
        self.sectors.append((at, sector, {s: [] for s in STYLES}))
        for s in STYLES:
            self.last[s] = np.asarray(at, np.float64)

    def step(self, styles, text, at=None):
        for s in styles:
            if self.sectors:
                self.sectors[-1][2][s].append(text)
                if at is not None:
                    self.last[s] = np.asarray(at, np.float64)

    def way(self, p, force=False):
        """A point of the way: a goto (safe, collector) or via (speed) when far enough on from the last."""
        for s in STYLES:
            if not self.auto[s] or not self.sectors:
                continue
            if force or np.linalg.norm(p - self.last[s]) >= SPACING[s]:
                if s == "speed":
                    self.step([s], f"via {fmt(p)}", p)
                elif not self.smooth:
                    self.step([s], f"goto {fmt(p)}", p)
                else:
                    # rolling on through the point, no faster than PASS_V: a smooth ride (no stop at each point)
                    self.step([s], f"pass {fmt(p)} {PASS_V[s]:g}", p)

    def corner(self, p, v):
        """A hard corner at `p`: every style takes it through (the last point of the floor before it, when it is this
        one, is replaced), no faster than v."""
        if not self.smooth:
            return
        for s in STYLES:
            if not self.auto[s] or not self.sectors:
                continue
            steps = self.sectors[-1][2][s]
            if steps and steps[-1].split()[0] in ("pass", "via", "goto") and \
                    np.linalg.norm(np.array(steps[-1].split()[1:4], float) - p) < 0.02:
                steps.pop()
            self.step([s], f"pass {fmt(p)} {v if s != 'speed' else v + 3:g}", p)


def fmt(p):
    return " ".join(f"{c:.2f}" for c in p)


def trim_start(pts, heads, cut):
    """A piece's points and headings with its first `cut` units left off."""
    out, hs = list(pts), list(heads)
    while len(out) >= 2:
        seg = float(np.linalg.norm(out[1] - out[0]))
        if seg > cut:
            out[0] = out[0] + (out[1] - out[0]) * (cut / seg)
            return out, hs
        cut -= seg
        out.pop(0)
        hs.pop(0)
    return out, hs


def trim_end(st, cut):
    """A stretch (its points, widths, distances, headings, railings) with its last `cut` units left off."""
    st = {k: (list(v) if isinstance(v, list) else v) for k, v in st.items()}
    path, widths, along, heads = st["path"], st["widths"], st["along"], st["heads"]
    while len(path) >= 2:
        seg = along[-1] - along[-2]
        if seg > cut:
            t = 1 - cut / seg
            path[-1] = path[-2] + (path[-1] - path[-2]) * t
            widths[-1] = widths[-2] + (widths[-1] - widths[-2]) * t
            along[-1] = along[-2] + seg * t
            return st
        cut -= seg
        for k in ("path", "widths", "along", "heads", "rails", "scales"):
            st[k].pop()
    return st


class Track:
    """The cursor and what the description has built so far."""

    def __init__(self, out_dir):
        self.b = L.LevelBuilder(out_dir, seed=sum(map(ord, os.path.basename(out_dir))))
        self.pos = np.zeros(3)
        self.head = 0.0
        self.new_cuts = []
        self.width = 10.0
        # the level's safety (its first `safety`), and the current stretch's
        self.level_safety = None
        self.safety = "border"  # the floors' profile (levelkit.section): gutter, border or none
        # the stretch of track being laid: its points, widths and distances along the track; the distance laid so far
        # (gaps and all, so every texture runs on); how much of the next floor a checkpoint's piece takes
        self.stretch = None
        self.along = 0.0
        self.skip = 0.0
        self.laid_width = None
        # where the last floor built ends (a floor beginning there runs on from its width)
        self.floor_end = None
        self.floor_end_info = None
        # railings along the floors' left and right edges from here on (safety railings: both)
        self.rail_sides = (False, False)
        # railings on the outer side of every turn from here on (railing turns), and the current turn's
        self.rail_turns = False
        self.turn_rail = (False, False)
        # rails being laid piece after piece (one object), and a checkpoint waiting for the floor after it
        self.rail_run = None
        self.pending = None
        self.surface = "stone"
        self.sector = 1
        self.marks = {}
        self.count = {}
        self.pilot = Pilot()
        self.info = {"author": "Drahtseilakt", "sky": "A", "music": 1, "bonus": 100, "ball": "wood",
                     "light": (1.0, 1.0, 1.0), "titles": {}}
        self.started = self.finished = False
        # the ball the way has from here on (the start's, then each transformer's)
        self.ball_kind = "wood"
        # the ball each sector begins with: the ball the description has at its checkpoint (sector 1: none, the level's
        # start ball); written as the level's settings `sector_balls`, for a run begun at a sector (--sector)
        self.sector_balls = [None]
        self.bodies = []
        self.rail_ends = []
        self.start_at = None
        self.checkpoints = 0
        # the distance along the way where each sector ends (its checkpoint, the finish)
        self.sector_ends = []
        self.columns = 0.0
        self.scatter_every = 0.0
        self.scatter_pillars = False
        self.lighthouse_at = None
        self.column_run = 0.0
        self.column_spots = []
        # the length of floor laid since the last gap: the most a run-up to the next can take
        self.since_gap = 0.0
        self.floor_points = []
        # each point's reach across (half the floor's width, or the rails'), for keeping columns clear of it
        self.floor_reach = []
        self.floor_along = []
        self.floor_rails = []

    # --- the cursor ---------------------------------------------------------------------------------------------

    def fwd(self, h=None):
        a = math.radians(self.head if h is None else h)
        return np.array([math.cos(a), 0.0, math.sin(a)])

    def left(self, h=None):
        a = math.radians(self.head if h is None else h)
        return np.array([-math.sin(a), 0.0, math.cos(a)])

    def at(self, ahead=0.0, side=0.0, up=0.0):
        return self.pos + self.fwd() * ahead + self.left() * side + np.array([0.0, up, 0.0])

    def n(self, kind):
        self.count[kind] = self.count.get(kind, 0) + 1
        return self.count[kind]

    # --- pieces -------------------------------------------------------------------------------------------------

    def straight(self, length, rise):
        steps = max(2, int(math.ceil(length / 4.0)) + 1)
        pts = [self.pos + self.fwd() * length * t + np.array([0, rise * t, 0]) for t in np.linspace(0, 1, steps)]
        return pts, [self.fwd()] * len(pts)

    def arc(self, angle, radius, rise):
        """Points of a turn `angle` degrees to the left (negative: right) on `radius`."""
        sgn = 1.0 if angle > 0 else -1.0
        centre = self.pos + self.left() * radius * sgn
        n = max(3, int(abs(angle) / 6) + 1)
        pts, heads = [], []
        for k in range(n + 1):
            t = k / n
            h = self.head + angle * t
            p = centre - self.left(h) * radius * sgn
            pts.append(np.array([p[0], self.pos[1] + rise * t, p[2]]))
            heads.append(self.fwd(h))
        # the turn ends exactly where its last point says (the cursor goes on from there)
        return pts, heads

    def lay(self, pts, heads, kind, straight=False, ties=False):
        self.road_head = self.head
        if kind == "Rails":
            self.end_stretch()
            self.resolve_pending()
            self.extend_rails(pts, heads, ties)
        dense = pts
        if kind != "Rails":
            self.end_rails(pts[0])
            if straight:
                pts, heads = [pts[0], pts[-1]], [heads[0], heads[-1]]
            self.extend_stretch(pts, heads)
        # a straight piece has only its two ends; a hint's step takes at most 600 ticks, so a long one gets points on it
        way = [pts[0]]
        for q in pts[1:]:
            d = float(np.linalg.norm(q - way[-1]))
            k = int(d // 30.0) if self.pilot.smooth else 0
            way += [way[-1] + (q - way[-1]) * (j / (k + 1)) for j in range(1, k + 1)]
            way.append(q)
        for p in way[1:]:
            self.pilot.way(p)
        # every few units of it, for keeping columns clear of it
        self.floor_points.extend(dense)
        self.floor_reach.extend([L.RAIL_GAUGE / 2 + 1.0 if kind == "Rails" else self.width / 2] * len(dense))
        # where along the way each lies (an overpass is a floor far on along the way, not the one before or after)
        self.floor_along.extend([self.along] * len(dense))
        self.floor_rails.extend([kind == "Rails"] * len(dense))
        self.since_gap += sum(float(np.linalg.norm(b - a)) for a, b in zip(pts, pts[1:]))
        if self.columns and kind == "Floor":
            for a, c in zip(pts, pts[1:]):
                self.column_run += float(np.linalg.norm(c - a))
                if self.column_run >= self.columns:
                    self.column_run = 0.0
                    self.column_spots.append(c)

    # --- stretches of floor: the pieces laid one after another as one mesh ----------------------------------------

    def extend_stretch(self, pts, heads):
        """Adds a floor's points to the stretch being laid when it runs on from its end (same place, heading, safety
        and surface), else begins a new one. A width changed since the last floor tapers over this one."""
        pts = [np.asarray(p, np.float64) for p in pts]
        heads = [np.asarray(h, np.float64) for h in heads]
        if self.skip > 0:
            self.end_stretch()
            whole = sum(float(np.linalg.norm(b - a)) for a, b in zip(pts, pts[1:]))
            pts, heads = trim_start(pts, heads, self.skip)
            self.along += min(self.skip, whole)
            self.skip = 0.0
            if len(pts) < 2:
                return
        st = self.stretch
        runs_on = (st is not None and st["safety"] == self.safety and st["surface"] == self.surface
                   and np.linalg.norm(st["path"][-1] - pts[0]) < 1e-6
                   and np.linalg.norm(st.get("tail", st["heads"][-1]) - heads[0]) < 1e-6)
        if not runs_on:
            self.end_stretch()
            from_start = self.build_start(pts[0])
            # a floor running on from a checkpoint begins as wide as its piece; any other new stretch (after rails,
            # a gap, a move) at the width set now
            if self.pending is None and (self.floor_end is None or np.linalg.norm(self.floor_end - pts[0]) > 1e-6):
                self.laid_width = self.width
            st = self.stretch = {"path": [pts[0]], "widths": [self.laid_width or self.width], "along": [self.along],
                                 "heads": [heads[0]], "rails": [self.rail_sides], "scales": [1.0], "safety": self.safety,
                                 "surface": self.surface, "from_start": from_start}
            self.resolve_pending(st)
        w0 = self.laid_width if self.laid_width is not None else self.width
        length = sum(float(np.linalg.norm(b - a)) for a, b in zip(pts, pts[1:]))
        run = 0.0
        for a, b, h in zip(pts, pts[1:], heads[1:]):
            run += float(np.linalg.norm(b - a))
            st["path"].append(b)
            st["heads"].append(h)
            st["scales"].append(1.0)
            st["rails"].append((self.rail_sides[0] or self.turn_rail[0], self.rail_sides[1] or self.turn_rail[1]))
            st["widths"].append(w0 + (self.width - w0) * (run / length if length > 0 else 1.0))
            st["along"].append(st["along"][-1] + float(np.linalg.norm(b - a)))
        self.along = st["along"][-1]
        self.laid_width = self.width
        st.pop("tail", None)

    def resolve_pending(self, st=None):
        """Builds the checkpoint waiting for the floor after it: joined to that floor when it runs on from the
        piece's front edge (`st`, the stretch just begun), else with a side there."""
        cp, self.pending = self.pending, None
        if cp is None:
            return
        front = None
        if st is not None:
            edge = np.asarray(cp["at"]) + np.asarray(cp["fwd"]) * L.CHECK_LEN / 2
            if (np.linalg.norm(st["path"][0] - edge) < JOIN_TOL and np.linalg.norm(st["heads"][0] - cp["fwd"]) < 1e-6
                    and abs(st["widths"][0] - cp["width"]) < 1e-6):
                st["path"][0] = edge
                front = L.join_points(st["widths"][0], st["safety"], st["surface"])
                st["joined"] = "start"
        n0 = len(self.b.nodes)
        self.b.checkpoint(cp["n"], tuple(cp["at"]), width=cp["width"], fwd=tuple(cp["fwd"]), back=cp["back"],
                          front=front, along=cp["along"])
        self.b.mark_route(n0, cp["along"] - L.CHECK_LEN / 2, cp["along"] + L.CHECK_LEN / 2)

    def extend_rails(self, pts, heads, ties=False):
        """Rails laid piece after piece are one pair of tubes (no kink or gap where two pieces meet); the floor they
        leave, if one ends where they begin, gets their mouth."""
        r = self.rail_run
        if r is not None and (np.linalg.norm(r["path"][-1] - pts[0]) > 1e-6
                              or np.linalg.norm(r["heads"][-1] - heads[0]) > 1e-6):
            self.end_rails()
            r = None
        if r is None:
            start = None
            if self.floor_end is not None and np.linalg.norm(self.floor_end - pts[0]) < 1e-6:
                start = self.floor_end_info
            r = self.rail_run = {"path": [np.asarray(pts[0], np.float64)], "heads": [heads[0]], "start": start,
                                 "along": self.along,
                                 "ties": False}
        r["path"].extend(np.asarray(p, np.float64) for p in pts[1:])
        r["heads"].extend(heads[1:])
        r["ties"] |= ties

    def end_rails(self, next_floor=None):
        """Builds the rails being laid; `next_floor`: where the floor laid next begins (its mouth there when it runs
        on from their end)."""
        r, self.rail_run = self.rail_run, None
        if r is None:
            return
        end = None
        if next_floor is not None and np.linalg.norm(np.asarray(next_floor) - r["path"][-1]) < 1e-6:
            end = (self.width, self.safety, self.surface, self.rail_sides)
        self.rail_ends += [r["path"][0], r["path"][-1]]
        n0 = len(self.b.nodes)
        self.b.rails(f"Rails_{self.n('Rails'):03d}", [tuple(p) for p in r["path"]], headings=r["heads"],
                     ends=(r["start"], end), ties=r["ties"], supports=self.rail_supports(r["path"]))
        self.b.mark_route(n0, r.get("along", self.along), self.along)

    def rail_supports(self, path):
        """Where rails get a saddle on a column: about every RAIL_SUPPORT along them, none within RAIL_SUPPORT / 4 of
        their ends (the floors there hold them), none over another floor or rails within the column's reach."""
        run = [0.0]
        for a, b in zip(path, path[1:]):
            run.append(run[-1] + float(np.linalg.norm(b - a)))
        total = run[-1]
        keep = L.RAIL_SUPPORT / 4
        if total < 2 * keep:
            return []
        count = max(1, int(round((total - 2 * keep) / L.RAIL_SUPPORT)))
        want = [keep + (total - 2 * keep) * (k + 0.5) / count for k in range(count)]
        below = np.array(self.floor_points) if self.floor_points else np.zeros((0, 3))
        reach = np.array(self.floor_reach) if self.floor_reach else np.zeros(0)
        out = []
        for d in want:
            i = min(range(len(run)), key=lambda j: abs(run[j] - d))
            c = path[i]
            if len(below):
                flat = np.hypot(below[:, 0] - c[0], below[:, 2] - c[2])
                if np.any((flat < reach + 2.0) & (below[:, 1] < c[1] - 1.0)
                          & (below[:, 1] > c[1] - L.RAIL_COLUMN - 3.0)):
                    continue
            out.append(i)
        return out

    def build_start(self, first=None):
        """The start, built with the first floor: joined to it when the floor runs on from the neck's front edge."""
        if self.start_at is not None:
            join = None
            if first is not None and np.linalg.norm(first - (self.start_at + np.array([L.START_RADIUS, 0, 0]))) < 1e-6:
                join = L.join_points(self.width, self.safety, self.surface)
            n0 = len(self.b.nodes)
            self.b.start(tuple(self.start_at), width=self.width, join=join, along=self.along - L.START_RADIUS)
            self.b.mark_route(n0, self.along - 2 * L.START_RADIUS, self.along)
            self.start_at = None
            return join is not None
        return False

    def end_stretch(self):
        """Builds the stretch being laid; returns it (None when nothing was built)."""
        st, self.stretch = self.stretch, None
        return self.build_stretch(st)

    def build_stretch(self, st):
        if st is None or len(st["path"]) < 2:
            return None
        self.floor_end = np.asarray(st["path"][-1], np.float64)
        self.floor_end_info = (st["widths"][-1], st["safety"], st["surface"], st["rails"][-1])
        begins_joined = st.get("joined") == "start" or st.get("from_start", False)
        try:
            node = self.b.track(f"Floor_{self.n('Floor'):03d}", [tuple(p) for p in st["path"]], st["widths"],
                                st["along"], st["safety"], st["surface"],
                                caps=(not begins_joined, not st.get("ends_joined", False)),
                                headings=st["heads"], railings=st["rails"], scales=st["scales"])
            self.b.route[node] = (min(st["along"]), max(st["along"]))
        except ValueError as e:
            raise LevelError(str(e)) from None
        return st

    def move_along(self, pts, head_turn=0.0):
        end = np.array(pts[-1], np.float64)
        st = self.stretch
        if st is None or np.linalg.norm(st["path"][-1] - end) > 1e-6:
            # nothing laid to here (a gap, rails, a module): the distance along still runs on
            self.along += sum(float(np.linalg.norm(np.subtract(b, a))) for a, b in zip(pts, pts[1:]))
        self.pos = end
        self.head += head_turn

    # --- the description's commands ----------------------------------------------------------------------------

    def run(self, cmd, args, opts, flags, line):
        n0, a0 = len(self.b.nodes), self.along
        if cmd not in KEEPS_CORNER_RAIL:
            # a stone corner's railing goes on along the floor laid next from the corner, nothing else
            self.corner_rail_next = None
        self.run_command(cmd, args, opts, flags, line)
        self.b.mark_route(n0, a0, max(self.along, a0))

    def run_command(self, cmd, args, opts, flags, line):
        if cmd == "floor" and getattr(self, "corner_rail_next", None):
            # the floor out of a stone corner: its first CORNER_RAIL with the railing on the corner's outer side
            outer, self.corner_rail_next = self.corner_rail_next, None
            length = num(args, 0, line)
            rise = self.rise(opts, line)
            r = min(CORNER_RAIL, length)
            for part, turn_rail in ((r, outer), (length - r, (False, False))):
                if part <= 1e-6:
                    continue
                pts, heads = self.straight(part, rise * part / length)
                self.turn_rail = turn_rail
                self.lay(pts, heads, "Floor", straight=True)
                self.turn_rail = (False, False)
                self.move_along(pts)
        elif cmd in ("floor", "rails", "gap"):
            length = num(args, 0, line)
            rise = self.rise(opts, line)
            pts, heads = self.straight(length, rise)
            if cmd == "gap":
                self.jump(pts[-1], opts)
            else:
                self.lay(pts, heads, "Rails" if cmd == "rails" else "Floor", straight=True, ties="ties" in opts)
            self.move_along(pts)
        elif cmd in ("left", "right", "rails-left", "rails-right"):
            angle, radius = num(args, 0, line), num(args, 1, line)
            angle = angle if cmd.endswith("left") else -angle
            pts, heads = self.arc(angle, radius, self.rise(opts, line))
            # a turn's railing belongs on its outer side (a left turn's right)
            inner = 0 if angle > 0 else 1
            if not cmd.startswith("rails") and self.rail_sides[inner] and not self.rail_sides[1 - inner]:
                raise LevelError(f"line {line}: a railing on the inner side of the turn (`railing turns` puts one on "
                                 "the outer side of every turn)")
            self.turn_rail = (angle < 0, angle > 0) if self.rail_turns else (False, False)
            self.lay(pts, heads, "Rails" if cmd.startswith("rails") else "Floor", ties="ties" in opts)
            self.turn_rail = (False, False)
            self.move_along(pts, angle)
        elif cmd == "rebate":
            # a ledge: the floor laid D lower, the cursor back up at its end (both steps are the floors' own ends)
            length, deep = num(args, 0, line), num(args, 1, line)
            down = np.array([0.0, deep, 0.0])
            self.pos = self.pos - down
            self.run_command("floor", [f"{length:g}"], {}, flags, line)
            self.pos = self.pos + down
        elif cmd == "move":
            to = self.at(num(args, 0, line), num(args, 1, line), num(args, 2, line))
            self.along += float(np.linalg.norm(to - self.pos))
            self.pos = to
        elif cmd == "turn":
            self.head += num(args, 0, line)
        elif cmd == "drop":
            # an intended landing here: floors within R (20) of the cursor may be reached by falling or flying
            self.b.drops.append((self.pos.copy(), float(args[0]) if args else 20.0))
        elif cmd == "columns":
            self.columns = num(args, 0, line)
        elif cmd == "width":
            self.width = num(args, 0, line)
        elif cmd == "safety":
            kind = word(args, 0, line)
            if kind not in L.SAFETY:
                raise LevelError(f"line {line}: safety is {', '.join(L.SAFETY)}")
            if self.level_safety is None:
                self.level_safety = kind
            self.set_safety(kind)
        elif cmd == "borders":
            # off: the bare floor; on: back to the level's safety
            self.set_safety((self.level_safety or "border") if word(args, 0, line) == "on" else "none")
        elif cmd == "railing":
            which = word(args, 0, line)
            sides = {"left": (True, False), "right": (False, True), "both": (True, True), "off": (False, False)}
            if which == "turns":
                self.rail_turns = True
            elif which == "noturns":
                self.rail_turns = False
            elif which not in sides:
                raise LevelError(f"line {line}: railing left|right|both|off|turns|noturns")
            else:
                self.rail_sides = sides[which]
        elif cmd == "surface":
            self.surface = word(args, 0, line)
            if self.surface not in ("stone", "wood"):
                raise LevelError(f"line {line}: surface is stone or wood")
        elif cmd == "mark":
            self.marks[word(args, 0, line)] = (self.pos.copy(), self.head, self.along, self.width)
        elif cmd == "back":
            # a branch ends here: nothing laid after runs on from what was before
            name = word(args, 0, line)
            if name not in self.marks:
                raise LevelError(f"line {line}: no mark {name}")
            self.end_stretch()
            self.end_rails()
            self.pos, self.head = self.marks[name][0].copy(), self.marks[name][1]
            self.along = self.marks[name][2]
            if "keepwidth" not in args:
                self.width = self.marks[name][3]
            self.floor_end = None
        elif cmd == "corner":
            self.corner(args, opts, line)
        elif cmd == "smooth":
            self.pilot.smooth = True
        elif cmd == "plaza":
            self.plaza(args, opts, line)
        elif cmd == "disc":
            self.end_stretch()
            c = self.at(float(opts.get("ahead", 0)), float(opts.get("side", 0)), float(opts.get("up", 0)))
            self.b.disc_node = P.disc(self.b, f"Disc_{self.n('disc'):02d}", tuple(c), num(args, 0, line), self.sector,
                                      surface=self.surface, rim="rim" in args)
            if "ring" in args:
                # a railing ring with two openings, toward the way in (behind) and the way on (ahead)
                w = float(opts.get("gap", 8.0))
                h = self.head
                P.rail_ring(self.b, f"Ring_{self.n('ring'):02d}", tuple(c), num(args, 0, line),
                            [(h + 180.0, w), (h, w)])
        elif cmd == "pit":
            self.pit(args, opts, line)
        elif cmd in ("lamptower", "mast", "plinth", "pillar", "wallblock", "lighthouse", "garland", "wlamp"):
            self.scenery(cmd, args, opts, line)
        elif cmd == "scatter":
            self.scatter_every = num(args, 0, line)
            self.scatter_pillars = "pillars" in args
        elif cmd == "start":
            if args:
                self.pos = np.array([num(args, i, line) for i in range(3)])
            # built with the first floor, as wide as it
            self.start_at = self.pos.copy()
            self.pilot.begin(self.pos + np.array([0, L.RESET_UP, 0]), 1)
            self.pos = self.at(10.0)
            self.started = True
        elif cmd == "checkpoint":
            if abs(self.head % 90) > 1e-6:
                raise LevelError(f"line {line}: a checkpoint's heading must be a multiple of 90 (the camera's)")
            self.checkpoints += 1
            self.sector_balls.append(self.ball_kind)
            self.sector_ends.append(self.along)
            self.pilot.way(self.pos, force=True)
            self.end_rails()
            self.resolve_pending()
            # the piece takes the last half of its length of the floor before it and the first of the floor after;
            # it is built with the floor after it (resolve_pending), joined to both where they run on from its edges
            back_edge = self.pos - self.fwd() * L.CHECK_LEN / 2
            st, self.stretch = self.stretch, None
            back = None
            if st is not None:
                st = trim_end(st, L.CHECK_LEN / 2)
                if (len(st["path"]) >= 2 and np.linalg.norm(st["path"][-1] - back_edge) < JOIN_TOL
                        and np.linalg.norm(st["heads"][-1] - self.fwd()) < 1e-6
                        and abs(st["widths"][-1] - self.width) < 1e-6):
                    back = L.join_points(st["widths"][-1], st["safety"], st["surface"])
                    st["ends_joined"] = True
                    st["path"][-1] = back_edge
                self.build_stretch(st)
            self.skip = L.CHECK_LEN / 2
            self.pending = {"n": self.checkpoints, "at": self.pos.copy(), "width": self.width,
                            "fwd": self.fwd(), "back": back, "along": self.along}
            bottom_reset = self.pos + np.array([0.0, L.CHECK_RESET - L.check_bowl()[-1][1], 0.0])
            self.sector += 1
            self.pilot.begin(bottom_reset, self.sector)
        elif cmd == "finish":
            at = self.at(22.0)
            self.sector_ends.append(self.along + 22.0)
            self.b.finish(tuple(at), self.sector, xaxis=tuple(-self.fwd()))
            self.pilot.step(STYLES, f"via {fmt(self.at(12.0))}")
            self.pilot.step(STYLES, f"goto {fmt(at)} 1200")
            self.finished = True
        elif cmd == "seesaw":
            self.seesaw(line)
        elif cmd == "platebridge":
            # as every original level has it (levels 2-10): a gap of 15.4, the frame 6.9 on from its near edge
            self.module_at("P_Modul_29", 6.9, 0.0, 0.0)
            pts, _ = self.straight(15.4, 0.0)
            for p in pts[1:]:
                self.pilot.way(p)
            self.move_along(pts)
        elif cmd == "swingbridge":
            # as levels 8-10 have it: over a gap of 15, its frame in the middle, 1.2 under the floors' top
            gap = float(opts.get("ahead", 15.0))
            self.module_at("P_Modul_08", gap / 2, -1.2, 0.0)
            pts, _ = self.straight(gap, 0.0)
            if "nopilot" not in opts:
                self.pilot.way(pts[-1], force=True)
            self.move_along(pts)
        elif cmd == "crateblock":
            self.crateblock()
        elif cmd == "pusher":
            self.pusher(word(args, 0, line), line)
        elif cmd == "fan":
            q = self.module_at("P_Modul_18", float(opts.get("ahead", 0)), 0.6, 0.0, float(opts.get("side", 0)))
            self.b.fan_housing(f"Fan_Housing_{self.n('housing'):02d}", tuple(q - np.array([0, 0.6, 0])))
            if "nopilot" not in opts:
                self.pilot.step(STYLES, f"goto {fmt(q)}", q)
                self.pilot.step(STYLES, "wait 240")
        elif cmd == "sack":
            self.module_at("P_Modul_26", float(opts.get("ahead", 0)), 3.0, 0.0, float(opts.get("side", 0)))
        elif cmd == "arms":
            ahead = float(opts.get("ahead", 0))
            self.module_at("P_Modul_17", ahead, 4.5, 0.0, float(opts.get("side", 0)))
            if "nopilot" not in opts:
                # through the arms at some speed: a ball rolling up slowly does not turn them
                for s in ("safe", "collector"):
                    self.pilot.step([s], f"goto {fmt(self.at(ahead - 14))}", self.at(ahead - 14))
                self.pilot.step(STYLES, f"via {fmt(self.at(ahead + 8))}", self.at(ahead + 8))
        elif cmd == "flaps":
            self.flaps("stand" in args)
        elif cmd == "flapslot":
            # the flap wheel's second lane: its gap before the axle, a stopper under it for the flap that falls back
            corners = [self.at(0.0, -2.5, -1.03), self.at(FLAP_LEN, 2.5, -0.53)]
            lo, hi = np.minimum(*corners), np.maximum(*corners)
            self.b.stopper(f"Stopper_{self.n('stopper'):03d}", tuple(lo), tuple(hi))
            self.slot(FLAP_LEN)
        elif cmd == "fanpad":
            self.fanpad(num(args, 0, line))
        elif cmd == "troughs":
            self.troughs(int(num(args, 0, line)))
        elif cmd == "boardbridge":
            self.boardbridge()
        elif cmd == "hingebridge":
            self.hingebridge()
        elif cmd in ("point", "life", "box", "dome", "trafo", "loose", "column", "module"):
            self.part(cmd, args, opts, line)
        elif cmd == "pilot":
            self.pilot_steps(args, line)
        elif cmd in ("nopilot", "autopilot"):
            for s in styles_of(word(args, 0, line), line):
                self.pilot.auto[s] = cmd == "autopilot"
        else:
            raise LevelError(f"line {line}: unknown command {cmd}")

    def corner(self, args, opts, line):
        """A hard 90 degree corner: the last floor's end becomes a mitre ring (the cross-section standing on the
        bisector, 1/cos 45 wider across) shared with the floor that follows, whose heading is turned 90: the outer
        corner filled, the profile continuous. `rail`: a railing 8 either side of it on the outer side."""
        d = word(args, 0, line)
        if d not in ("left", "right"):
            raise LevelError(f"line {line}: corner left|right")
        sgn = 1.0 if d == "left" else -1.0
        st = self.stretch
        if st is None or len(st["path"]) < 2 or np.linalg.norm(st["path"][-1] - self.pos) > 1e-6:
            raise LevelError(f"line {line}: a corner must come at the end of a floor")
        outer = (False, True) if sgn > 0 else (True, False)
        reach = CORNER_RAIL
        # where the stone ball goes on a bare floor (no border, no gutter), every hard corner has the railing on its
        # outer side: so heavy and slow a ball, steered round, runs on straight into the corner's outer edge and
        # falls (the owner's notes, 2026-11-16; the original's narrow stone lanes are troughs instead, 0.68 deep in
        # the middle of 5, level 5). Along the floor out of it as far as the floor laid next reaches (no more floor
        # laid for it: the description stays as it is).
        stone_rail = self.ball_kind == "stone" and self.safety == "none" and "rail" not in args
        if "rail" in args or stone_rail:
            seg = st["along"][-1] - st["along"][-2]
            r = min(reach, seg * 0.5)
            t = 1 - r / seg
            P0, P1 = st["path"][-2], st["path"][-1]
            st["path"].insert(-1, P0 + (P1 - P0) * t)
            st["widths"].insert(-1, st["widths"][-2] + (st["widths"][-1] - st["widths"][-2]) * t)
            st["along"].insert(-1, st["along"][-1] - r)
            st["heads"].insert(-1, st["heads"][-2])
            st["scales"].insert(-1, 1.0)
            st["rails"].insert(-1, st["rails"][-1])
            old = st["rails"][-1]
            st["rails"][-1] = (old[0] or outer[0], old[1] or outer[1])
        self.pilot.corner(self.pos.copy(), CORNER_V)
        st["heads"][-1] = self.fwd(self.head + 45.0 * sgn)
        st["scales"][-1] = math.sqrt(2.0)
        self.head += 90.0 * sgn
        st["tail"] = self.fwd()
        if stone_rail:
            self.corner_rail_next = outer
        elif "rail" in args:
            gap = float(opts.get("gap", 0))
            if gap > 0:
                # an opening in the outer railing along the first `gap` of the way out: the way straight on
                pts, heads = self.straight(gap, 0.0)
                self.lay(pts, heads, "Floor", straight=True)
                self.move_along(pts)
            self.turn_rail = outer
            pts, heads = self.straight(reach, 0.0)
            self.lay(pts, heads, "Floor", straight=True)
            self.turn_rail = (False, False)
            self.move_along(pts)

    def plaza(self, args, opts, line):
        """plaza W L [brick|kerb] [stone|wood] [gap SIDE C W]... [fence SIDE...]: a flat square of floor from the middle
        of its back edge (at the cursor, moved by side/ahead/up), the cursor staying."""
        self.end_stretch()
        width, length = num(args, 0, line), num(args, 1, line)
        rim, gaps, fence, surf = None, {}, [], self.surface
        i = 2
        while i < len(args):
            a = args[i]
            if a in ("brick", "kerb"):
                rim = a
            elif a in ("stone", "wood"):
                surf = a
            elif a == "gap":
                side = word(args, i + 1, line)
                if side not in P.SIDES:
                    raise LevelError(f"line {line}: plaza gap back|front|left|right C W")
                gaps.setdefault(side, []).append((num(args, i + 2, line), num(args, i + 3, line)))
                i += 3
            elif a == "fence":
                while i + 1 < len(args) and args[i + 1] in P.SIDES:
                    fence.append(args[i + 1])
                    i += 1
            else:
                raise LevelError(f"line {line}: plaza: what is {a!r}")
            i += 1
        # a plaza that opens off the way's side (its back gap, at right angles to the way): the way's railings leave a
        # gap at its mouth (found on a first pass, applied on the second: compile_level)
        rh = getattr(self, "road_head", None)
        if rh is not None and "back" in gaps and abs(((self.head - rh) % 180.0) - 90.0) < 1.0:
            mouth = self.pos + self.fwd() * (self.width / 2)
            self.new_cuts.append((float(mouth[0]), float(mouth[2]), width / 2 + 0.8))
        at = self.at(float(opts.get("ahead", 0)), float(opts.get("side", 0)), float(opts.get("up", 0)))
        P.plaza(self.b, f"Plaza_{self.n('plaza'):02d}", at, self.fwd(), width, length, rim, gaps, fence, self.sector,
                surf)
        # the floor under it counts for columns and overpasses
        for a in np.linspace(0, length, max(2, int(length / 6) + 1)):
            self.floor_points.append(at + self.fwd() * a)
            self.floor_reach.append(max(width, 2.0) / 2 + 0.0)
            self.floor_along.append(self.along + a)
            self.floor_rails.append(False)

    def pit(self, args, opts, line):
        """pit L [depth D] [wall LEN] [side S]: the way stops for L; a flat bottom D (4.3) under it, its walls, and a
        stop wall LEN long, 2 high, on side S (1 left, -1 right) of the way beyond its kerb."""
        length = num(args, 0, line)
        depth = float(opts.get("depth", 4.3))
        P.pit(self.b, f"Pit_{self.n('pit'):02d}", self.pos.copy(), self.fwd(), self.width, length, depth,
              float(opts.get("wall", 0)), 2.0, float(opts.get("side", 1)), self.sector)
        self.slot(length)

    def scenery(self, cmd, args, opts, line):
        p = self.at(float(opts.get("ahead", 0)), float(opts.get("side", 0)), float(opts.get("up", 0)))
        n = f"{cmd.capitalize()}_{self.n(cmd):02d}"
        if "obstacle" in opts:
            self.b.intended.add(n)
        if cmd == "lamptower":
            P.lamp_tower(self.b, n, tuple(p), float(opts.get("r", 2.6)), float(opts.get("height", 20)))
        elif cmd == "mast":
            P.mast(self.b, n, tuple(p), float(opts.get("height", 22)))
        elif cmd == "plinth":
            P.plinth(self.b, n, tuple(p), float(opts.get("r", 4.5)))
        elif cmd == "pillar":
            P.pillar(self.b, n, tuple(p), float(opts.get("r", 3.0)), float(opts.get("height", 10)))
        elif cmd == "wallblock":
            P.wall_block(self.b, n, tuple(p), self.fwd(), float(opts.get("long", 40)), float(opts.get("height", 14)),
                         float(opts.get("thick", 6)), self.sector)
        elif cmd == "lighthouse":
            P.lighthouse(self.b, n, tuple(p), float(opts.get("r", 13)), float(opts.get("height", 40)), self.sector)
            self.lighthouse_at = p
        elif cmd == "garland":
            q = self.at(float(opts.get("ahead2", 0)), float(opts.get("side2", 0)), float(opts.get("up2", 0)))
            P.garland(self.b, n, tuple(p), tuple(q), float(opts.get("sag", 2.0)), lamps="nolamps" not in args)
        elif cmd == "wlamp":
            # a lantern on a wall: `side` is out from the wall toward the lamp (turn: the direction it faces)
            out = self.fwd(self.head + float(opts.get("turn", 90)))
            self.b.lantern(tuple(p), tuple(out), "Lantern")

    def set_safety(self, kind):
        """The floors' profile from here on, and their railings: both sides for railings (on the bare floor), none
        for the others (`railing` adds them on any profile)."""
        self.safety = "none" if kind == "railings" else kind
        self.rail_sides = (True, True) if kind == "railings" else (False, False)

    def slot(self, length):
        """The way goes on `length` further with nothing built (a slot a module's part stands or slides in); the test
        player rolls on over it."""
        pts, _ = self.straight(length, 0.0)
        self.end_stretch()
        for p in pts[1:]:
            self.pilot.way(p)
        self.move_along(pts)

    def crateblock(self):
        """The stone on the crate (P_Modul_34): a slot of CRATE_SLOT in the way, the module's stone standing in it on
        its crate, which stands on a floor 5 under the way (laid by the description, crossing under the slot): the
        stone rises out of the way and blocks it until a ball on the floor below pushes the crate out from under it,
        and it slides down, level with the way. The module's frame is at the way's top, its x across the way (the
        crate is pushed along it); the stone stands 7.7 back along that x (the part's measured pivots)."""
        x = self.fwd()
        stone = self.at(CRATE_SLOT / 2)
        frame = stone + x * 7.7
        self.b.module("P_Modul_34", self.n("P_Modul_34"), K.pose(tuple(frame), tuple(x)), self.sector)
        self.slot(CRATE_SLOT)

    def pusher(self, side, line):
        """A pusher (P_Modul_01) across the way: its stone block stands in the way, its beam in a slot of PUSHER_SLOT
        across the way, its target plate standing out PUSHER_REACH past the way's edge on `side` (left or right),
        where a ball below the way (on a floor 2.5 lower, laid by the description) pushes it: the block slides across
        and out of the way (the way should be at most 6 wide there, so the block fills it). The module's frame is at
        that edge at the way's top, its x across the way; the module's beam runs 0.65 along its z from the frame (the
        part's measured pivots), so the frame stands that much back from the slot's middle."""
        if side not in ("left", "right"):
            raise LevelError(f"line {line}: pusher left|right")
        sgn = 1.0 if side == "left" else -1.0
        x = -self.left() * sgn
        up = np.array([0.0, 1.0, 0.0])
        z = np.cross(x, up)
        edge = self.at(PUSHER_SLOT / 2, sgn * self.width / 2)
        frame = edge - z * 0.65
        n = self.n("P_Modul_01")
        self.b.module("P_Modul_01", n, K.pose(tuple(frame), tuple(x)), self.sector)
        # the channel under the slot (only for the look: the part's own channel floor holds the beam)
        m = K.Mesh(f"Pusher_Channel_{n:02d}")
        mid = frame + z * 0.65 + x * (self.width / 2 + 2.0) - up * 2.75
        K.obox(m, self.b.mat_ids["side"], tuple(mid), (tuple(x), tuple(up), tuple(z)),
               (self.width / 2 + 2.0, 0.25, PUSHER_SLOT / 2 + 0.4), scale=L.SLAB)
        self.b.add(f"Pusher_Channel_{n:02d}", m)
        self.slot(PUSHER_SLOT)

    def module_at(self, name, ahead, up, turn, side=0.0):
        """A module with its frame `ahead` and `up` of the cursor, its x turned `turn` from the heading."""
        q = self.at(ahead, side, up)
        self.b.module(name, self.n(name), K.pose(tuple(q), tuple(self.fwd(self.head + turn))), self.sector)
        return q

    def jump(self, land, opts):
        """A gap: the styles that go carefully take a run-up first; all go over it at speed."""
        if "nopilot" in opts:
            return
        run = min(float(opts.get("runup", 14.0)), max(self.since_gap - 2.0, 0.0))
        self.since_gap = 0.0
        take_off = self.pos.copy()
        for s in ("safe", "collector"):
            if self.pilot.auto[s] and run > 0:
                self.pilot.step([s], f"goto {fmt(self.at(-run))}", self.at(-run))
        on = land + (land - take_off) / (np.linalg.norm(land - take_off) + 1e-9) * 4.0
        self.pilot.step(STYLES, f"via {fmt(take_off)}", take_off)
        self.pilot.step(STYLES, f"via {fmt(on)}", on)

    def seesaw(self, line):
        """A seesaw (P_Modul_30) over a gap of 12.9 (its board 14.8 long lying on both edges), its frame in the middle
        half a unit under the floor, as every original level sets it (levels 2-9), an unseen stopper under the board."""
        board = 12.9
        mid = self.at(board / 2, 0, -0.5)
        self.b.module("P_Modul_30", self.n("P_Modul_30"), K.pose(tuple(mid), tuple(self.fwd())), self.sector)
        corners = [self.at(0, -2.5, -1.0), self.at(board, 2.5, -0.5)]
        lo, hi = np.minimum(*corners), np.maximum(*corners)
        self.b.stopper(f"Stopper_{self.n('stopper'):03d}", tuple(lo), tuple(hi))
        pts, _ = self.straight(board, 0.0)
        # onto the board's raised end at some speed (a heavy ball from rest does not get up it)
        self.jump(pts[-1], {})
        self.move_along(pts)

    def fanpad(self, size):
        """A square platform `size` across from the cursor (the middle of its back edge), with a fan (P_Modul_18) in
        its own block in the middle, as the original's level 3 has its fans (a 15 x 15 platform each, the next 25 on
        and 11 higher): the fan's frame at the floor's top, sunk in the block under its grille (levelkit.fan_block).
        The cursor goes to the platform's far edge."""
        self.end_stretch()
        f, lft = self.fwd(), self.left()
        up = np.array([0.0, 1.0, 0.0])
        mid = self.at(size / 2)
        n = self.n("fanpad")
        m = K.Mesh(f"Fan_Pad_{n:02d}")
        top = mid - up * P.FLUSH
        hb, rest = FAN_BLOCK / 2, (size - FAN_BLOCK) / 2
        mats = self.b.mat_ids
        for a, s, ha, hs in ((-(hb + rest / 2), 0.0, rest / 2, size / 2), ((hb + rest / 2), 0.0, rest / 2, size / 2),
                             (0.0, hb + rest / 2, hb, rest / 2), (0.0, -(hb + rest / 2), hb, rest / 2)):
            c = top + f * a + lft * s - up * L.DEPTH / 2
            P.solid(m, mats["top"], mats["side"], c, (f, up, lft), (ha, L.DEPTH / 2, hs))
        self.b.add(f"Fan_Pad_{n:02d}", m, groups=self.b._floor_groups("stone") + self.b.sector(self.sector))
        self.b.fan_block(f"Fan_Block_{n:02d}", tuple(mid), tuple(f), self.sector, sides=False)
        k = self.n("P_Modul_18")
        self.b.module("P_Modul_18", k, K.pose(tuple(mid), tuple(f)), self.sector, lift=0.0)
        for a in np.linspace(0, size, max(2, int(size / 6) + 1)):
            self.floor_points.append(self.at(a))
            self.floor_reach.append(size / 2)
            self.floor_along.append(self.along + a)
            self.floor_rails.append(False)
        pts, _ = self.straight(size, 0.0)
        self.move_along(pts)

    def troughs(self, count):
        """A row of `count` tipping troughs (P_Modul_41) over the void where the way stops, as the original's levels
        7 to 11 have them (five to eight in a row, 6 apart, the first 4 past the floor's end, the floor going on 3
        past the last): each hangs level with the floor on its axle across the way and tips under a heavy ball; the
        paper ball rolls over them. The cursor goes to where the floor goes on."""
        self.end_stretch()
        for i in range(count):
            self.module_at("P_Modul_41", TROUGH_FIRST + TROUGH_STEP * i, 0.0, 0.0)
        self.slot(TROUGH_FIRST + TROUGH_STEP * (count - 1) + TROUGH_LAST)

    def boardbridge(self):
        """The flap bridge's board (P_Modul_25) standing at the far side of the way's end, as the original's levels 2,
        3, 5 and 8 set it: the way stops at the cursor, the board stands on its hinge at the cursor's edge of a gap
        of BOARD_GAP (9.1, every one of theirs), the floor going on past it. The ball pushes the board over: it falls
        across the gap (9.57 long from its hinge) onto the far floor. The module's x points back along the way; the
        cursor goes on past the gap."""
        self.end_stretch()
        self.module_at("P_Modul_25", 0.0, 0.0, 180.0)
        self.slot(BOARD_GAP)

    def hingebridge(self):
        """The drawbridge (P_Modul_37) at the end of a spur, as the original's levels 3, 5, 8 and 10 set it: the
        spur's floor stops at the cursor, the module's frame HINGE_BACK on, its x back along the spur; standing up,
        its deck (9 wide, 8.9 to 13.9 from its hinge) falls, when the ball tips it, into the gap of a lane that crosses
        the spur's line HINGE_LANE past the frame (laid by the description: a lane 5 wide, its gap 7.6 long). The
        cursor stays."""
        self.end_stretch()
        self.module_at("P_Modul_37", HINGE_BACK, 0.0, 180.0)

    def flaps(self, stand=False):
        """The flap wheel (P_Modul_19) as the original's levels set it (levels 9, 10 and 11): its frame at the floors'
        top, its axle across the way at the near edge of a gap of a flap's length, one flap lying flat over the gap
        (the way goes on over it) and the other standing up at the axle 15 to the left (in the original, over a second
        lane's gap: the stone ball tips it flat, and the first rises), with an unseen stopper under the flat flap's
        place, half a unit under the floor, that holds it level under the ball. The part's measured pivots: the flaps
        are 6.4 long from the axle (local x -3.24 to 3.16, the axle at -2.93), 5 wide, the flat one at local z -12.5
        to -7.5 (to the right of the frame), the standing one at 2.5 to 7.5."""
        side = FLAP_STAND_SIDE if stand else FLAP_SIDE
        self.module_at("P_Modul_19", FLAP_BACK, 0.0, 0.0, side)
        # the stoppers under the flat flaps' places: on the way, and (standing) under the flat one on the lane beside
        for s in ((0.0, FLAP_STAND_SIDE - FLAP_SIDE) if stand else (0.0,)):
            corners = [self.at(0.0, s - 2.5, -1.03), self.at(FLAP_LEN, s + 2.5, -0.53)]
            lo, hi = np.minimum(*corners), np.maximum(*corners)
            self.b.stopper(f"Stopper_{self.n('stopper'):03d}", tuple(lo), tuple(hi))
        self.slot(FLAP_LEN)

    def place_columns(self, depth=L.COLUMN_DEPTH):
        """The columns under the floors, but none that would pass through a floor or rails below it."""
        below = np.array(self.floor_points) if self.floor_points else np.zeros((0, 3))
        reach = np.array(self.floor_reach) if self.floor_reach else np.zeros(0)
        for c in self.column_spots:
            if len(below):
                flat = np.hypot(below[:, 0] - c[0], below[:, 2] - c[2])
                if np.any((flat < reach + 3.0) & (below[:, 1] < c[1] - 1.0) & (below[:, 1] > c[1] - depth - 3.0)):
                    continue
            self.b.column(f"Column_{self.n('column'):03d}", tuple(c), depth)

    def place_scatter(self):
        """The distant scenery (`scatter D`): one object every D units of the way, clear of every floor."""
        if not self.scatter_every:
            return
        rng = np.random.default_rng(sum(map(ord, os.path.basename(self.b.out))) + 7)
        avoid = list(zip(self.floor_points, self.floor_reach))
        if self.lighthouse_at is not None:
            avoid += [(self.lighthouse_at, 40.0)]
        # the way, in order, as a path
        order = sorted(range(len(self.floor_points)), key=lambda k: self.floor_along[k])
        path = [self.floor_points[k] for k in order]
        P.scatter(self.b, rng, path, None, None, self.scatter_every, avoid, pillars=self.scatter_pillars)

    def rise(self, opts, line):
        if "up" in opts:
            return float(opts["up"])
        if "down" in opts:
            return -float(opts["down"])
        return 0.0

    def width_at(self, ahead):
        """The floor's width `ahead` of the cursor (on the stretch being laid, a width tapering over its last floor),
        else the width set."""
        st = self.stretch
        if st is None or len(st["path"]) < 2:
            return self.width
        q = self.at(ahead)
        best = None
        for (a, b), (wa, wb) in zip(zip(st["path"], st["path"][1:]), zip(st["widths"], st["widths"][1:])):
            a, b = np.asarray(a), np.asarray(b)
            ab = b - a
            t = float(np.clip(np.dot(q - a, ab) / (np.dot(ab, ab) + 1e-12), 0.0, 1.0))
            d = float(np.linalg.norm(a + ab * t - q))
            if best is None or d < best[0]:
                best = (d, wa + (wb - wa) * t)
        return best[1] if best[0] < 6.0 else self.width

    def check_room(self, cmd, kind, side, ahead, opts, line):
        """Notes a box or a loose ball for check_bodies; one marked `shove` must be no heavier than the ball the way
        has there."""
        body_r = {"box": 2.5 * math.sqrt(2)}.get(cmd, 2.0)
        if "plug" in opts:
            # put there to block the way until the right ball comes: the room check does not apply
            return
        if "lead" in opts:
            # a ball the way's ball leads: it rolls on ahead of the player, never blocks
            opts = dict(opts, shove=True)
        elif "shove" in opts:
            mine = BALL_MASS[self.ball_kind]
            theirs = {"box": BOX_MASS}.get(cmd, LOOSE_MASS.get(kind, 0.0))
            if mine < theirs:
                raise LevelError(f"line {line}: a {self.ball_kind} ball ({mine}) cannot shove "
                                 f"a {kind or cmd} ({theirs})")
        self.bodies.append({"at": self.at(ahead, side), "centre": self.at(ahead), "across": self.left(), "r": body_r,
                            "free": L.free_width(self.width_at(ahead), self.safety, self.rail_sides, self.surface),
                            "shove": "shove" in opts, "rolls": cmd == "loose", "line": line, "what": kind or cmd})

    def check_overpasses(self):
        """No way runs under another floor or rails with the room between them where a ball wedges: the underside over
        a way's top is either at most OVERPASS_WALL over it (the ball meets its side as a wall) or OVERPASS_CLEAR or
        more (the ball passes)."""
        P = np.array(self.floor_points)
        if not len(P):
            return
        R, A, RL = np.array(self.floor_reach), np.array(self.floor_along), np.array(self.floor_rails)
        bad = []
        for i in range(len(P)):
            if RL[i]:
                continue
            flat = np.hypot(P[:, 0] - P[i, 0], P[:, 2] - P[i, 2])
            # the underside of what passes over: a floor's DEPTH under its top, rails' tubes under theirs
            under = P[:, 1] - np.where(RL, 0.6, L.DEPTH)
            gap = under - P[i, 1]
            hit = (flat < np.minimum(R, 3.0) + 1.0) & (np.abs(A - A[i]) > 30.0) & (gap > L.OVERPASS_WALL) \
                & (gap < L.OVERPASS_CLEAR)
            if np.any(hit):
                j = int(np.argmax(hit))
                bad.append(f"({P[i, 0]:.1f}, {P[i, 1]:.1f}, {P[i, 2]:.1f}): {gap[j]:.1f} under the way over it")
        if bad:
            raise LevelError(f"{len(bad)} places under an overpass where the ball wedges (under {L.OVERPASS_WALL} or "
                             f"{L.OVERPASS_CLEAR} and more over a way), e.g. " + "; ".join(bad[:3]))

    def check_bodies(self):
        """The way must stay open past the boxes and loose balls on it: across the way where each lies, with the
        others near it, a gap of the room of a way (levelkit.MIN_FREE), unless they are all there to be shoved; and
        no loose ball within RAIL_KEEP of the end of rails (a ball rolls, and in the rails' mouth it blocks them)."""
        for b in self.bodies:
            # what lies across the way with it: within a ball's width along the way, and on the same way
            fwd = np.array([b["across"][2], 0.0, -b["across"][0]])
            near = [c for c in self.bodies if abs(float(np.dot(c["at"] - b["at"], fwd))) < 4.0 + c["r"]
                    and np.linalg.norm(c["centre"] - b["centre"]) < 4.0 + b["free"]]
            if all(c["shove"] for c in near):
                continue
            spans = sorted((float(np.dot(c["at"] - b["centre"], b["across"])) - c["r"],
                            float(np.dot(c["at"] - b["centre"], b["across"])) + c["r"]) for c in near)
            edge, best = -b["free"] / 2, 0.0
            for lo, hi in spans:
                best = max(best, lo - edge)
                edge = max(edge, hi)
            best = max(best, b["free"] / 2 - edge)
            if best < L.MIN_FREE:
                raise LevelError(f"line {b['line']}: the {b['what']} (with what lies near it) leaves the way only "
                                 f"{best:.1f} open (at least {L.MIN_FREE}; or mark them `shove` where the ball there "
                                 "can push them away)")
            if b["rolls"] and not b["shove"]:
                for e in self.rail_ends:
                    if np.linalg.norm(e - b["at"]) < RAIL_KEEP:
                        far = np.linalg.norm(e - b["at"])
                        raise LevelError(f"line {b['line']}: a loose {b['what']} ball {far:.0f} "
                                         f"from the end of rails (at least {RAIL_KEEP}: it rolls into their mouth)")

    def part(self, cmd, args, opts, line):
        side, ahead = float(opts.get("side", 0)), float(opts.get("ahead", 0))
        if cmd in ("box", "loose"):
            self.check_room(cmd, word(args, 0, line) if cmd == "loose" else None, side, ahead, opts, line)
        p = self.at(ahead, side, float(opts.get("up", 0)) if cmd in ("point", "life", "trafo", "loose") else 0.0)
        s = self.sector
        if cmd == "point":
            self.b.extra_point(self.n("point"), tuple(p), s)
            self.pilot.step(["collector"], f"goto {fmt(p)}", p)
        elif cmd == "life":
            self.b.extra_life(self.n("life"), tuple(p), s)
            self.pilot.step(["collector"], f"goto {fmt(p)}", p)
        elif cmd == "box":
            # a box on a box (`up 5`): it stands on the one under it, not on the floor
            up = float(opts.get("up", 0))
            self.b.box(self.n("box"), tuple(p + np.array([0.0, up, 0.0])), s, on_floor=up == 0)
        elif cmd == "dome":
            self.b.dome(self.n("dome"), tuple(p), s)
            # a dome is gone round, never over: it must leave the room of a way on both sides
            free = L.free_width(self.width_at(ahead), self.safety, self.rail_sides, self.surface)
            for sgn in (-1.0, 1.0):
                room = free / 2 - sgn * side - DOME_R
                if room < L.MIN_FREE:
                    raise LevelError(f"line {line}: the dome leaves only {room:.1f} beside it on the "
                                     f"{'left' if sgn > 0 else 'right'} (at least {L.MIN_FREE} on both sides)")
        elif cmd == "trafo":
            kind = BALLS.get(word(args, 0, line))
            if kind is None:
                raise LevelError(f"line {line}: trafo wood|stone|paper|sticky")
            self.b.trafo(kind, self.n("trafo" + kind), tuple(p), s)
            self.ball_kind = kind.lower()
            if "nopilot" not in opts:
                self.pilot.step(STYLES, f"become {kind.lower()}", p)
        elif cmd == "loose":
            kind = BALLS.get(word(args, 0, line))
            if kind is None:
                raise LevelError(f"line {line}: loose wood|stone|paper|sticky")
            self.b.loose_ball(kind, self.n("loose" + kind), tuple(p), s)
        elif cmd == "column":
            self.b.column(f"Column_{self.n('column'):03d}", tuple(p), float(opts.get("depth", 40)),
                          float(opts.get("r", 2.2)))
        elif cmd == "module":
            name = word(args, 0, line)
            if not name.startswith("P_Modul_"):
                raise LevelError(f"line {line}: a module is P_Modul_NN")
            q = self.at(ahead, side, float(opts.get("up", 0)))
            h = self.head + float(opts.get("turn", 0))
            tilt = math.radians(float(opts.get("tilt", 0)))
            x = self.fwd(h) * math.cos(tilt) + np.array([0.0, math.sin(tilt), 0.0])
            self.b.module(name, self.n(name), K.pose(tuple(q), tuple(x)), s)

    def pilot_steps(self, args, line):
        """`pilot STYLES : STEP ; STEP`: the steps with their points about the cursor, turned into world ones."""
        text = " ".join(args)
        if ":" not in text:
            raise LevelError(f"line {line}: pilot STYLES : STEPS")
        head, body = text.split(":", 1)
        styles = styles_of(head.strip(), line)
        for st in (x.split() for x in body.split(";")):
            if not st:
                continue
            verb, w = st[0], [float(x) if is_num(x) else x for x in st[1:]]
            try:
                if verb in ("goto", "via"):
                    p = self.at(w[0], w[1], w[2])
                    t = f" {int(w[3])}" if len(w) > 3 else ""
                    self.pilot.step(styles, f"{verb} {fmt(p)}{t}", p)
                elif verb in ("push", "air"):
                    d = self.fwd() * w[0] + self.left() * w[1]
                    self.pilot.step(styles, f"{verb} {d[0]:.3f} {d[2]:.3f} {int(w[2])}")
                elif verb == "shove":
                    a, b = self.at(w[0], w[1], w[2]), self.at(w[3], w[4], w[5])
                    self.pilot.step(styles, f"shove {fmt(a)} {fmt(b)} {int(w[6])}")
                elif verb == "lead":
                    a = self.at(w[0], w[1], w[2])
                    self.pilot.step(styles, f"lead {fmt(a)} {w[3]:.2f} {int(w[4])}")
                elif verb == "press":
                    # press F S U OA OS V T: along the line to the point, kept to it (an offset OA OS across it, turned
                    # as push's direction), on while slower than V
                    p = self.at(w[0], w[1], w[2])
                    o = self.fwd() * w[3] + self.left() * w[4]
                    self.pilot.step(styles, f"press {fmt(p)} {o[0]:.3f} {o[2]:.3f} {w[5]:g} {int(w[6])}", p)
                elif verb == "pass":
                    p = self.at(w[0], w[1], w[2])
                    t = f" {int(w[4])}" if len(w) > 4 else ""
                    self.pilot.step(styles, f"pass {fmt(p)} {w[3]:g}{t}", p)
                elif verb == "hold":
                    p = self.at(w[0], w[1], w[2])
                    self.pilot.step(styles, f"hold {fmt(p)} {int(w[3])}", p)
                elif verb in ("wait", "turn"):
                    self.pilot.step(styles, f"{verb} {int(w[0])}")
                elif verb == "become":
                    self.pilot.step(styles, f"become {st[1]}")
                else:
                    raise LevelError(f"line {line}: unknown pilot step {verb}")
            except (IndexError, TypeError, ValueError):
                raise LevelError(f"line {line}: a malformed pilot step {' '.join(st)}") from None


def is_num(x):
    try:
        float(x)
        return True
    except ValueError:
        return False


def num(args, i, line):
    try:
        return float(args[i])
    except (IndexError, ValueError):
        raise LevelError(f"line {line}: a number expected as argument {i + 1}") from None


def word(args, i, line):
    if i >= len(args):
        raise LevelError(f"line {line}: argument {i + 1} missing")
    return args[i]


def styles_of(text, line):
    if text == "all":
        return list(STYLES)
    out = [s.strip() for s in text.split(",")]
    for s in out:
        if s not in STYLES:
            raise LevelError(f"line {line}: no style {s} (safe, speed, collector, all)")
    return out


OPTION_WORDS = ("up", "down", "side", "ahead", "depth", "turn", "tilt", "runup", "r", "height", "wall", "sag", "ahead2",
                "side2", "up2", "thick", "long", "hole", "gap")
FLAG_WORDS = ("obstacle", "nopilot", "ties", "shove", "lead", "nolamps", "plug")


def parse_line(text):
    words = shlex.split(text, comments=True, posix=True)
    if not words:
        return None
    cmd, rest = words[0], words[1:]
    if cmd == "pilot":
        return cmd, rest, {}, set()
    args, opts, flags = [], {}, set()
    i = 0
    while i < len(rest):
        w = rest[i]
        if w in OPTION_WORDS and i + 1 < len(rest) and is_num(rest[i + 1]):
            opts[w] = rest[i + 1]
            i += 2
            continue
        if w in FLAG_WORDS:
            opts[w] = True
            flags.add(w)
        else:
            args.append(w)
        i += 1
    return cmd, args, opts, flags


META = ("title", "order", "difficulty", "author", "sky", "music", "bonus", "ball", "light", "clouds") + tuple(f"title.{x}" for x in LANGS)


CLOUD_KEYS = {"cover": ("cover", 0.0, 1.0), "below": ("below", -1000.0, 10000.0),
              "thickness": ("thickness", 1.0, 10000.0), "size": ("scale", 1.0, 10000.0)}


def parse_clouds(value, line):
    """`clouds off` or `clouds [cover C] [below B] [thickness T] [size S]`: the level's choices for its sea of cloud."""
    words = value.split()
    if words == ["off"]:
        return {"cover": 0.0}
    if len(words) % 2:
        raise LevelError(f"line {line}: clouds: off, or pairs of cover, below, thickness, size and a number")
    out = {}
    for k, v in zip(words[::2], words[1::2]):
        if k not in CLOUD_KEYS:
            raise LevelError(f"line {line}: clouds: no {k!r} (cover, below, thickness, size)")
        name, lo, hi = CLOUD_KEYS[k]
        try:
            x = float(v)
        except ValueError:
            raise LevelError(f"line {line}: clouds: {k} {v!r} is not a number") from None
        if not lo <= x <= hi:
            raise LevelError(f"line {line}: clouds: {k} {v} is not in {lo:g}..{hi:g}")
        out[name] = x
    return out


def compile_level(path, cuts=None, **kw):
    """Compiles a level; a plaza opening off the way's side asks for a gap in the way's railing at its mouth, which
    is made on a second pass (the first one finds the openings). For a level made by a script instead of a
    `level.lvl` (make_synthetic_level.py): `lines` the description's lines (the file at `path` is then not read, only
    its folder used), `track` a Track with commands of the script's own, `hook(track)` called before the level is
    written (objects, materials and scripts added by hand), `extra` more level.txt lines (key: value)."""
    found = _compile_level(path, cuts or [], dry=True, **kw)
    _compile_level(path, found, **kw)


def _compile_level(path, cuts, dry=False, lines=None, track=None, hook=None, extra=None):
    folder = os.path.dirname(os.path.abspath(path))
    if lines is None:
        with open(path, encoding="utf-8") as f:
            lines = f.read().splitlines()
    t = (track or Track)(folder)
    t.b.rail_cuts = list(cuts)
    for i, raw in enumerate(lines, 1):
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        key = stripped.split(None, 1)[0]
        if key in META:
            value = stripped.split(None, 1)[1].strip() if " " in stripped else ""
            if key.startswith("title"):
                t.info["titles"][key[6:] or "en"] = value
            elif key == "light":
                t.info["light"] = tuple(float(x) for x in value.split())
            elif key == "clouds":
                t.info["clouds"] = parse_clouds(value, i)
            elif key in ("music", "bonus", "order"):
                t.info[key] = int(value)
            elif key == "difficulty":
                if not re.fullmatch(r"(10|[1-9])", value):
                    raise LevelError(f"line {i}: difficulty is a whole number 1 to 10")
                t.info[key] = int(value)
            elif key == "sky":
                # the game takes letters, digits, _ and - (sky.rs, valid_name): it names texture files
                if not re.fullmatch(r"[A-Za-z0-9_-]{1,32}", value):
                    raise LevelError(f"line {i}: sky {value!r}: a sky's name is letters, digits, _ and -")
                t.info[key] = value
            else:
                t.info[key] = value
                if key == "ball":
                    t.ball_kind = value
            continue
        try:
            parsed = parse_line(raw)
        except ValueError as e:
            raise LevelError(f"line {i}: {e}") from None
        if parsed:
            t.run(*parsed, line=i)
    if not t.started or not t.finished:
        raise LevelError(f"{path}: a level needs a start and a finish")
    t.build_start()
    t.end_stretch()
    t.end_rails()
    t.resolve_pending()
    t.check_bodies()
    t.check_overpasses()
    t.place_columns()
    t.place_scatter()
    if dry:
        return list(t.new_cuts)
    info = t.info
    titles = info["titles"]
    if "en" not in titles:
        raise LevelError(f"{path}: no title")
    if hook is not None:
        hook(t)
    try:
        t.b.write(titles["en"], info["author"], sky=info["sky"], start_ball=f"Ball_{BALLS[info['ball']]}",
                  music=info["music"], light=info["light"], bonus=info["bonus"], licence="GPL-3.0-only",
                  clouds=info.get("clouds"), sector_balls=t.sector_balls,
                  extra={**{f"title.{k}": v for k, v in titles.items() if k != "en"},
                         **({"order": info["order"]} if "order" in info else {}),
                         **({"difficulty": info["difficulty"]} if "difficulty" in info else {}),
                         **(extra or {})})
    except ValueError as e:
        raise LevelError(f"{path}: {e}") from None
    write_pilot(folder, t)
    lengths = [b - a for a, b in zip([0.0] + t.sector_ends, t.sector_ends)]
    print(f"{folder}: {len(t.b.nodes)} objects, {t.sector} sectors, the way {t.sector_ends[-1]:.0f} long "
          f"(sectors {', '.join(f'{x:.0f}' for x in lengths)}; moves and gaps included)")
    return list(t.new_cuts)


def write_pilot(folder, t):
    out = os.path.join(folder, "pilot")
    os.makedirs(out, exist_ok=True)
    for style in STYLES:
        lines = [f"# {style}: written by leveldsl.py from level.lvl (do not edit; edit level.lvl)"]
        for at, sector, steps in t.pilot.sectors:
            chunk_at = at
            todo = steps[style]
            while todo:
                part, todo = todo[:HINT_STEPS], todo[HINT_STEPS:]
                lines.append(f"at {fmt(chunk_at)} r 5 sector {sector} : " + " ; ".join(part))
                last = next((s for s in reversed(part) if s.split()[0] in ("goto", "via")), None)
                if last:
                    w = last.split()
                    chunk_at = np.array([float(w[1]), float(w[2]) + 2.0, float(w[3])])
        with open(os.path.join(out, f"{style}.txt"), "w") as f:
            f.write("\n".join(lines) + "\n")


def main(argv):
    paths = argv[1:]
    if not paths:
        print(__doc__)
        return 2
    for p in paths:
        try:
            compile_level(p)
        except LevelError as e:
            print(f"{p}: {e}", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
