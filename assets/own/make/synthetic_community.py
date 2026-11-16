#!/usr/bin/env python3
"""The community part of the showcase level (make_synthetic_level.py): what community maps bring beyond the original's
levels, each built for real with our own content and shown beside a plain floor the ball rolls straight along.

- sector 6, materials (no scripts): alpha blending, alpha test, additive glow, texgen chrome and reflect, a
  double-sided sheet, an unlit colour, a normal map with parallax relief, a mesh with a material channel (a second
  pass), a 3D sprite, a glass floor in the way;
- sector 7, behaviour graphs (the level's own scripts, written with scriptkit.py and run by the game's script runtime
  as a converted map's are): Rotate, a progression driving Set Position (a shuttle, and a lift that is a real floor),
  Text Display by proximity, 3D Text, a portal (Box Box Intersection moving the ball), a particle fountain, an
  animated texture (Set Texture on a timer), Show/Hide and Set Diffuse blinking, a crate made a body by Physicalize;
  then the sticky ball of the BallSticky mod (our `sticky` kind: its transformer, a slope it climbs and the wooden
  ball does not, its loose ball);
- on the last stretch, from its sign to the balloon (ENV_LEN): another sky, fog and light (TT Sky, Set Fog, Set Light
  Color), switched by the ball's distance; the rest of the community part is lit as the classic one, so its floors and
  exhibits look as they do there (owner, 2026-11-16: over the whole part the warm light and the sunset sky read as
  darker textures).

`LINES` is its description (leveldsl.py's language with the commands of `Community` below). Of the 13 converted
community maps, only a glass floor (map 10), the fan's grate (the classic fan's), portals (map 05) and crates (the
classic P_Box) stand on their ways; none uses texgen, an unlit or additive material, a second pass, sprites, or
Rotate, progressions, lifts, Text Display, Set Texture, blinking or the sky, fog and light blocks. So the glass floor,
the scripted lift and the portal are on the way, each with its check (a ball rides the lift, the portal sends it over
a gap); the rest stand beside it, shown, not used.
"""

import math
import os
import shutil

import numpy as np

import levelkit as L
import ownkit as K
import scriptkit as S

# The stretch with another sky, fog and light (environment): the last of the way, the showcase's `floor 30` after
# LINES (make_synthetic_level.FINISH) and the 22 to the balloon. At the end, so nothing after it is seen without the level's own fog:
# leaving it, Set Fog's mode 0 switches all fog off, the level's own too (scripts.md), as Virtools' would.
ENV_LEN = 30.0 + 22.0

LINES = """
# ---- sector 6: materials ---------------------------------------------------------------------------------------------
floor 12
sign "COMMUNITY 6: materials" ahead -6
floor 14
sign "Material: alpha blend (glass)"
exhibit glass
floor 12
sign "Material: alpha test (grate)"
exhibit grate
floor 12
sign "Material: additive glow, unlit"
exhibit glow
floor 12
sign "Material: texgen chrome"
exhibit chrome
floor 12
sign "Material: texgen reflect"
exhibit reflect
floor 12
sign "Material: double-sided sheet"
exhibit sheet
floor 12
sign "Material: unlit colour"
exhibit unlit
floor 12
sign "Material: normal map + parallax"
exhibit relief
floor 12
sign "Mesh: second pass (channel)"
exhibit channel
floor 12
sign "3D sprite (billboard)"
exhibit sprite
floor 14
sign "Glass floor (alpha blend)" ahead 6
check comm-glass : ball wood -6 0 0 ; go 18 0 0 4 600 ; reach 18 0 0 3 ; kept
glassfloor 12
floor 16
checkpoint

# ---- sector 7: behaviour graphs --------------------------------------------------------------------------------------
floor 12
sign "COMMUNITY 7: scripts" ahead -6
sign "Script: Rotate"
exhibit rotate
floor 14
sign "Script: progression, Set Position"
exhibit shuttle
floor 16
sign "Script: lift in the way (a moving floor)"
check comm-lift : ball wood 9 0 0 ; wait 200 ; go 20 0 7 4 300 ; reach 20 0 7 3 ; kept
check comm-lift-down : ball wood 9 0 0 ; wait 470 ; go 20 0 7 4 300 ; away 20 0 7 4
safety none
width 6
floor 6
exhibit lift
floor 8
width 12
floor 10
safety border
sign "Script: Text Display by proximity"
exhibit text2d
floor 16
sign "Script: 3D Text"
exhibit text3d
floor 14
sign "Script: particle fountain"
exhibit fountain
floor 14
sign "Script: Set Texture animation"
exhibit movie
floor 14
sign "Script: Show / Hide, Set Diffuse"
exhibit blink
floor 14
width 12
floor 12
sign "Script: portal over a gap (moves the ball)"
check comm-portal : ball wood -8 0 0 ; go 32 0 0 4 900 ; reach 32 0 0 3 ; kept
safety none
width 6
floor 8
exhibit portal
gap 18
floor 10
width 30
floor 12
safety border
sign "Script: Physicalize (push it)"
exhibit crate
floor 20
width 12
floor 16
# the sticky ball: no level of the original has it, nor any of the converted community maps; it is BallSticky's (a
# BML mod: its ball and transformer, our `sticky` kind, docs/drahtseilakt/mods.md). What it does that no other ball
# does: pushed from rest it climbs a slope of 40 degrees (up to 43), which the wooden ball rolls back down
sign "BallSticky (mod): P_Trafo_Sticky"
trafo sticky
floor 16
sign "Sticky ball: climbs a slope of 40 (wood rolls back)"
check comm-sticky : ball sticky 8 0 0 ; go 30 0 8.4 3 1200 ; reach 30 0 8.4 3 ; kind sticky ; kept
check comm-sticky-wood : ball wood 8 0 0 ; go 30 0 8.4 3 1200 ; away 30 0 8.4 4
floor 10
floor 10 up 8.4
width 16
floor 12
sign "P_Ball_Sticky (BallSticky's loose ball)"
loose sticky side 3 ahead 10
floor 14
width 12
floor 6
sign "P_Trafo_Wood"
trafo wood
floor 16
# the finale: another sky, fog and light from here to the balloon (environment)
sign "TT Sky, Set Fog, Set Light Color: to the end" ahead 4
environment
"""

# How far beside the way's middle the exhibits stand (the way is 12 wide there), and their columns' radius.
SIDE = 10.0
COLUMN_R = 2.2
# The scripted lift's rise and the gap the portal sends the ball over.
LIFT_RISE = 7.0
PORTAL_GAP = 18.0
TEX = 256


def _grid(n=TEX):
    """Texture coordinates u, v in 0..1 over an n × n picture."""
    t = (np.arange(n) + 0.5) / n
    return np.meshgrid(t, t)


class Community:
    """The community part's commands: `environment` (another sky, fog and light on the next ENV_LEN of the way),
    `exhibit KIND` (an exhibit beside the way at the cursor), `glassfloor L` (a translucent floor in the way)."""

    def community_command(self, cmd, args, opts, line):
        if cmd == "environment":
            # its script is written with the others (community_finish)
            self.env_zone = (self.at(), self.at(ENV_LEN))
        elif cmd == "exhibit":
            getattr(self, "ex_" + args[0])()
        elif cmd == "glassfloor":
            self.glassfloor(float(args[0]))
        else:
            return False
        return True

    # --- shared ---------------------------------------------------------------------------------------------------

    @property
    def graph(self):
        if not hasattr(self, "_graph"):
            self._graph = S.Graph()
        return self._graph

    def picture(self, name, make):
        """A texture of the community part, drawn once: `make()` gives (rgb, alpha or None)."""
        done = self.__dict__.setdefault("_pictures", set())
        if name not in done:
            rgb, alpha = make()
            K.save_png(self.b.tex, name, rgb, alpha)
            done.add(name)
        return name

    def stand(self, side=SIDE, ahead=0.0):
        """A column beside the way to put an exhibit on; returns the middle of its top."""
        top = self.at(ahead, side, 0.0)
        # levelkit's column hangs from under a floor: lifted by the floor's depth its top is level with the way
        self.b.column(f"Comm_Column_{self.n('comm_column'):02d}", tuple(top + np.array([0.0, L.DEPTH, 0.0])),
                      depth=40.0, r=COLUMN_R)
        return top

    def axes(self):
        return self.fwd(), np.array([0.0, 1.0, 0.0]), self.left()

    def tiles(self):
        """A plain own tile texture for the exhibits' solid things."""
        def make():
            u, v = _grid()
            seam = np.minimum(np.abs((u * 4) % 1 - 0.5), np.abs((v * 4) % 1 - 0.5)) > 0.46
            img = np.array([0.62, 0.6, 0.56]) * (0.9 + 0.2 * K.value_noise(TEX, TEX, (8, 8), 11))[..., None]
            img[seam] *= 0.55
            return img, None
        return self.b.mat("comm_tiles", self.picture("Comm_Tiles.png", make), diffuse=1.0, emissive=0.2)

    def glow_picture(self):
        def make():
            u, v = _grid()
            r = np.hypot(u - 0.5, v - 0.5) * 2
            a = np.clip(1 - r, 0, 1)
            return (a ** 0.8)[..., None] * np.array([1.0, 0.7, 0.25]), a
        return self.picture("Comm_Glow.png", make)

    def panel(self, name, mat, centre, half=(2.2, 2.2), both=True):
        """An upright panel across the view of the ball coming along the way (its face toward the ball)."""
        f, up, left = self.axes()
        c = np.asarray(centre)
        hw, hh = half
        q = [c - left * hw - up * hh, c + left * hw - up * hh, c + left * hw + up * hh, c - left * hw + up * hh]
        m = K.Mesh(name)
        m.face(mat, q, [(0, 1), (1, 1), (1, 0), (0, 0)], normal=-f)
        if both:
            m.face(mat, q[::-1], [(0, 0), (1, 0), (1, 1), (0, 1)], normal=f)
        return self.b.add(name, m)

    def frame(self, name, centre, half=(2.2, 2.2)):
        """A wooden frame round a panel, standing on the column under it."""
        f, up, left = self.axes()
        c = np.asarray(centre)
        hw, hh = half
        m = K.Mesh(name)
        ax = (tuple(f), tuple(up), tuple(left))
        for s in (-1.0, 1.0):
            K.obox(m, self.b.mat_ids["wood"], tuple(c + left * s * (hw + 0.15)), ax, (0.15, hh + 0.3, 0.15))
            K.obox(m, self.b.mat_ids["wood"], tuple(c + up * s * (hh + 0.15)), ax, (0.15, 0.15, hw + 0.3))
        self.b.add(name, m)

    def loop_steps(self, name, owner, steps, ms):
        """A script running `steps` (blocks with one input and one output) one after another for ever, `ms` apart."""
        g = self.graph
        links, blocks = [(S.START, steps[0].i[0])], []
        for k, step in enumerate(steps):
            wait = g.bb("Delayer", S.DELAYER, ["In"], ["Out"], [g.const("Time to Wait", S.TIME, S.f(ms))],
                        [g.pout("Elapsed Time", S.TIME)])
            links += [(step.o[0], wait.i[0]), (wait.o[0], steps[(k + 1) % len(steps)].i[0])]
            blocks += [step, wait]
        g.script(name, owner, blocks, links)

    def ping_pong(self, name, ent, p0, p1, ms):
        """A script moving entity `ent` from p0 to p1 and back for ever, `ms` each way: a Linear Progression's value
        mixes the two places (Interpolator) for Set Position, tick by tick."""
        g = self.graph
        a = g.local("A", S.VECTOR, S.v3(*p0))
        c = g.local("B", S.VECTOR, S.v3(*p1))
        halves = []
        for frm, to in ((0.0, 1.0), (1.0, 0.0)):
            value = g.pout("Value", S.FLOAT)
            lp = g.bb("Linear Progression", S.LINEAR_PROGRESSION, ["In", "Loop In"], ["Out", "Loop Out"],
                      [g.const("Duration", S.TIME, S.f(ms)), g.const("A", S.FLOAT, S.f(frm)),
                       g.const("B", S.FLOAT, S.f(to))],
                      [g.pout("Elapsed Time", S.TIME), value, g.pout("Delta", S.FLOAT),
                       g.pout("Progression", S.FLOAT)])
            mixed = g.pout("C", S.VECTOR)
            mix = g.bb("Interpolator", S.INTERPOLATOR, ["In"], ["Out"],
                       [g.pin("A", S.VECTOR, a), g.pin("B", S.VECTOR, c), g.pin("Value", S.FLOAT, value)], [mixed])
            put = g.bb("Set Position", S.SET_POSITION, ["In"], ["Out"],
                       [g.pin("Position", S.VECTOR, mixed), g.const("Referential", S.ENTITY, S.obj(None))],
                       target=g.target(ent))
            halves.append((lp, mix, put))
        links = [(S.START, halves[0][0].i[0])]
        for k, (lp, mix, put) in enumerate(halves):
            other = halves[1 - k][0]
            links += [(lp.o[1], mix.i[0]), (mix.o[0], put.i[0]), (put.o[0], lp.i[1], 1), (lp.o[0], other.i[0])]
        g.script(name, ent, [x for h in halves for x in h], links)

    def near_ball(self, marker, distance):
        """A proximity test of the ball against entity `marker`, running from the start: outputs EnterRange,
        ExitRange. Returns (block, the links that start it)."""
        g = self.graph
        one = g.local("one", S.FLOAT, S.f(1.0))
        prox = g.bb("TT Scaleable Proximity", S.TT_SCALEABLE_PROXIMITY, ["In", "Off"], ["EnterRange", "ExitRange"],
                    [g.const("Distance", S.FLOAT, S.f(distance)), g.const("ObjectA", S.ENTITY, S.obj(g.ball())),
                     g.const("ObjectB", S.ENTITY, S.obj(marker)), g.const("Barycenter?", S.BOOL, S.b(False)),
                     g.pin("Exactness min. Distance", S.FLOAT, one), g.pin("Exactness max. Distance", S.FLOAT, one),
                     g.pin("Minimum Framedelay", S.INT, one), g.pin("Maximum Framedelay", S.INT, one)],
                    [g.pout("Current Distance", S.FLOAT)])
        return prox, [(S.START, prox.i[0])]

    # --- sector 6: materials --------------------------------------------------------------------------------------

    def ex_glass(self):
        def make():
            u, v = _grid()
            edge = np.minimum(np.minimum(u, 1 - u), np.minimum(v, 1 - v))
            streak = np.clip(1 - np.abs((u + v) - 0.8) * 6, 0, 1) * 0.35
            img = np.array([0.55, 0.8, 0.95]) + streak[..., None]
            return np.clip(img, 0, 1), np.where(edge < 0.03, 0.9, 0.32 + streak)
        mat = self.b.mat("comm_glass", self.picture("Comm_Glass.png", make), diffuse=1.0, emissive=0.5, specular=0.6,
                         power=60, alpha="BLEND", double_sided=True)
        top = self.stand()
        c = top + np.array([0.0, 2.8, 0.0])
        self.frame(f"Comm_Glass_Frame_{self.n('comm_glass')}", c)
        self.panel("Comm_Glass_Panel", mat, c, both=False)

    def ex_grate(self):
        def make():
            u, v = _grid()
            bars = (np.abs((u * 6) % 1 - 0.5) > 0.3) | (np.abs((v * 6) % 1 - 0.5) > 0.3)
            img = np.array([0.5, 0.52, 0.56]) * (0.85 + 0.3 * K.value_noise(TEX, TEX, (16, 16), 5))[..., None]
            return img, bars.astype(np.float64)
        mat = self.b.mat("comm_grate", self.picture("Comm_Grate.png", make), diffuse=1.0, emissive=0.3, specular=0.5,
                         power=40, alpha="MASK", cutoff=0.5, double_sided=True)
        top = self.stand()
        c = top + np.array([0.0, 2.8, 0.0])
        self.frame("Comm_Grate_Frame", c)
        self.panel("Comm_Grate_Panel", mat, c, both=False)

    def ex_glow(self):
        mat = self.b.mat("comm_glow", self.glow_picture(), diffuse=1.0, emissive=1.0, alpha="BLEND",
                         blend=("one", "one"), unlit=True, double_sided=True)
        top = self.stand()
        m = K.Mesh("lamp")
        K.cylinder(m, self.b.mat_ids["rail"], tuple(top), (0, 1, 0), 0.25, 2.0, seg=10)
        self.b.add("Comm_Glow_Post", m)
        # three crossed sheets: a glow that adds its light from any side
        f, up, left = self.axes()
        c = top + up * 3.6
        g = K.Mesh("glow")
        for k in range(3):
            a = math.pi * k / 3
            r = f * math.cos(a) + left * math.sin(a)
            q = [c - r * 2 - up * 2, c + r * 2 - up * 2, c + r * 2 + up * 2, c - r * 2 + up * 2]
            g.face(mat, q, [(0, 1), (1, 1), (1, 0), (0, 0)])
        self.b.add("Comm_Glow", g)
        # a dark board behind it: light that is only added shows best over the dark
        back = self.b.mat("comm_dark", None, diffuse=(0.03, 0.03, 0.05), emissive=(0.03, 0.03, 0.05), unlit=True)
        d = K.Mesh("board")
        K.obox(d, back, tuple(c + left * 2.4), (tuple(f), tuple(up), tuple(left)), (2.6, 2.6, 0.1))
        self.b.add("Comm_Glow_Board", d)

    def env_picture(self):
        def make():
            return K.env_map(TEX, np.array([0.45, 0.62, 0.95]), np.array([1.0, 0.95, 0.85]), np.array([0.22, 0.18, 0.16]), 3), None
        return self.picture("Comm_Env.png", make)

    def ball_on_stand(self, name, mat):
        top = self.stand()
        m = K.Mesh(name)
        K.sphere(m, mat, tuple(top + np.array([0.0, 2.0, 0.0])), 2.0, seg=28)
        self.b.add(name, m)

    def ex_chrome(self):
        self.ball_on_stand("Comm_Chrome", self.b.mat("comm_chrome", self.env_picture(), diffuse=1.0, emissive=0.6,
                                                     specular=0.9, power=80, texgen="chrome"))

    def ex_reflect(self):
        self.ball_on_stand("Comm_Reflect", self.b.mat("comm_reflect", self.env_picture(), diffuse=1.0, emissive=0.6,
                                                      specular=0.9, power=80, texgen="reflect"))

    def ex_sheet(self):
        def make():
            u, v = _grid()
            check = ((u * 6).astype(int) + (v * 6).astype(int)) % 2
            return np.where(check[..., None] > 0, np.array([0.85, 0.2, 0.2]), np.array([0.95, 0.9, 0.8])), None
        mat = self.b.mat("comm_sheet", self.picture("Comm_Sheet.png", make), diffuse=1.0, emissive=0.3,
                         double_sided=True)
        top = self.stand()
        f, up, left = self.axes()
        m = K.Mesh("sheet")
        K.cylinder(m, self.b.mat_ids["rail"], tuple(top + left * 2.0), (0, 1, 0), 0.15, 5.4, seg=8)
        # one wavy sheet of single faces, a flag on its pole: seen from both sides because the material says so
        n = 12
        for k in range(n):
            x0, x1 = 4.0 * k / n, 4.0 * (k + 1) / n
            w0, w1 = 0.35 * math.sin(x0 * 2.2), 0.35 * math.sin(x1 * 2.2)
            base = top + left * 2.0 + up * 2.4
            q = [base - left * x0 + f * w0, base - left * x1 + f * w1, base - left * x1 + f * w1 + up * 2.8,
                 base - left * x0 + f * w0 + up * 2.8]
            m.face(mat, q, [(k / n, 1), ((k + 1) / n, 1), ((k + 1) / n, 0), (k / n, 0)])
        self.b.add("Comm_Sheet", m)

    def ex_unlit(self):
        mat = self.b.mat("comm_unlit", None, diffuse=(0.1, 0.9, 0.55), emissive=(0.1, 0.9, 0.55), unlit=True)
        top = self.stand()
        m = K.Mesh("unlit")
        K.obox(m, mat, tuple(top + np.array([0.0, 1.6, 0.0])), ((1, 0, 0), (0, 1, 0), (0, 0, 1)), (1.6, 1.6, 1.6))
        self.b.add("Comm_Unlit", m)

    def ex_relief(self):
        def height():
            u, v = _grid()
            du, dv = np.abs((u * 4) % 1 - 0.5), np.abs((v * 4) % 1 - 0.5)
            return np.clip((0.46 - np.maximum(du, dv)) * 12, 0, 1)

        def colour():
            h = height()
            img = np.array([0.72, 0.5, 0.36]) * (0.8 + 0.25 * K.value_noise(TEX, TEX, (12, 12), 9))[..., None]
            return img * (0.55 + 0.45 * h)[..., None], None

        def normal():
            h = height()
            gx = np.roll(h, -1, 1) - np.roll(h, 1, 1)
            gy = np.roll(h, -1, 0) - np.roll(h, 1, 0)
            # tangent space, green up (the picture's rows run down)
            n = np.stack([-gx * 3, gy * 3, np.ones_like(h)], -1)
            n /= np.linalg.norm(n, axis=-1, keepdims=True)
            return n * 0.5 + 0.5, h
        mat = self.b.mat("comm_relief", self.picture("Comm_Relief.png", colour), diffuse=1.0, emissive=0.2,
                         specular=0.3, power=30, normal=self.picture("Comm_Relief_n.png", normal), parallax=0.08)
        top = self.stand()
        m = K.Mesh("relief")
        f, up, left = self.axes()
        # a slab leaning back, so the relief is seen at a glancing angle as the ball passes
        tilt = up * math.cos(0.5) + left * math.sin(0.5)
        K.obox(m, mat, tuple(top + up * 2.2 + left * 0.8), (tuple(f), tuple(tilt), tuple(np.cross(f, tilt))),
               (2.4, 2.4, 0.3), scale=4.8)
        self.b.add("Comm_Relief", m)

    def ex_channel(self):
        def make():
            u, v = _grid()
            band = np.clip(1 - np.abs(((u + v) * 3) % 1 - 0.5) * 5, 0, 1)
            return band[..., None] * np.array([0.2, 0.75, 1.0]), None
        over = self.b.mat("comm_channel", self.picture("Comm_Bands.png", make), diffuse=1.0, emissive=1.0, unlit=True)
        top = self.stand()
        m = K.Mesh("channel")
        K.obox(m, self.tiles(), tuple(top + np.array([0.0, 1.8, 0.0])), ((1, 0, 0), (0, 1, 0), (0, 0, 1)),
               (1.8, 1.8, 1.8))
        # the second pass: light bands added over the tiles, on coordinates of their own (running across the faces)
        m.channel(over, ("one", "one"), lambda p, n, uv: (uv[0] * 0.5 + uv[1] * 0.5, uv[1] * 0.5 - uv[0] * 0.5))
        self.b.add("Comm_Channel", m)

    def ex_sprite(self):
        def make():
            u, v = _grid()
            r = np.hypot(u - 0.5, v - 0.5) * 2
            a = np.arctan2(v - 0.5, u - 0.5)
            star = r < 0.55 + 0.35 * np.cos(a * 5)
            img = np.where(star[..., None], np.array([1.0, 0.85, 0.2]), np.array([0.15, 0.3, 0.7]))
            return img, np.clip((0.97 - r) * 30, 0, 1)
        mat = self.b.mat("comm_sprite", self.picture("Comm_Sprite.png", make), diffuse=1.0, emissive=1.0, alpha="BLEND", unlit=True,
                         double_sided=True)
        top = self.stand()
        m = K.Mesh("post")
        K.cylinder(m, self.b.mat_ids["rail"], tuple(top), (0, 1, 0), 0.2, 1.6, seg=8)
        self.b.add("Comm_Sprite_Post", m)
        q = K.Mesh("sprite")
        # a sprite's mesh is the unit quad (0..1 in x and y) the game sizes and turns each frame (scene.rs)
        q.face(mat, [(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)], [(0, 1), (1, 1), (1, 0), (0, 0)])
        i = self.b.add("Comm_Sprite", q, K.pose(tuple(top + np.array([0.0, 3.4, 0.0]))))
        # mode 0: the quad always faces the view (open-format.md, a node's sprite)
        self.b.nodes[i]["sprite"] = (0, (3.6, 3.6), (0.0, 0.0))

    def glassfloor(self, length):
        """A translucent slab of floor in the way for the next `length`: the ball rolls over glass, the void seen
        through it. The stretch laid so far ends; the cursor goes on as over a slot (leveldsl.Track.slot)."""
        def make():
            u, v = _grid()
            line = np.minimum(np.abs((u * 3) % 1 - 0.5), np.abs((v * 3) % 1 - 0.5)) > 0.47
            img = np.where(line[..., None], np.array([0.9, 0.97, 1.0]), np.array([0.5, 0.78, 0.92]))
            return img, np.where(line, 0.85, 0.42)
        self.b.mat("comm_glassfloor", self.picture("Comm_GlassFloor.png", make), diffuse=1.0, emissive=0.45,
                   specular=0.7, power=80, alpha="BLEND")
        if abs(self.fwd()[0]) < 0.999:
            raise ValueError("glassfloor: the way must run along x (a slab is laid by its corners)")
        a, c = self.at(0.0, -self.width / 2, -0.6), self.at(length, self.width / 2, 0.0)
        self.b.slab(f"Comm_GlassFloor_{self.n('comm_glassfloor'):02d}", tuple(np.minimum(a, c)),
                    tuple(np.maximum(a, c)), mat="comm_glassfloor", sector=self.sector)
        self.slot(length)

    # --- sector 7: scripts ----------------------------------------------------------------------------------------

    def ex_rotate(self):
        top = self.stand()
        m = K.Mesh("wheel")
        K.torus(m, self.b.mat_ids["rail"], (0, 0, 0), (0, 1, 0), 2.0, 0.3)
        for k in range(4):
            a = math.pi * k / 4
            K.obox(m, self.tiles(), (0, 0, 0), ((math.cos(a), 0, math.sin(a)), (0, 1, 0),
                                                 (-math.sin(a), 0, math.cos(a))), (2.0, 0.18, 0.18))
        K.cylinder(m, self.b.mat_ids["rail"], (0, -1.5, 0), (0, 1, 0), 0.3, 1.5, seg=10)
        self.b.add("Comm_Wheel", m, K.pose(tuple(top + np.array([0.0, 1.5, 0.0]))))
        g = self.graph
        wheel = g.entity("Comm_Wheel")
        rot = g.bb("Rotate", S.ROTATE, ["In"], ["Out"],
                   [g.const("Axis of Rotation", S.VECTOR, S.v3(0, 1, 0)),
                    g.const("Angle of Rotation", S.ANGLE, S.f(0.03)),
                    g.const("Referential of the Axis", S.ENTITY, S.obj(None))], target=g.target(wheel))
        g.script("Comm rotate", wheel, [rot], [(S.START, rot.i[0]), (rot.o[0], rot.i[0], 1)])

    def ex_shuttle(self):
        f, up, left = self.axes()
        a, c = self.stand(ahead=-4.0), self.stand(ahead=4.0)
        m = K.Mesh("rail")
        K.tube(m, self.b.mat_ids["rail"], [tuple(a + up * 1.0), tuple(c + up * 1.0)], 0.2, caps=True)
        for p in (a, c):
            K.cylinder(m, self.b.mat_ids["rail"], tuple(p), (0, 1, 0), 0.25, 1.0, seg=8)
        self.b.add("Comm_Shuttle_Rail", m)
        car = K.Mesh("car")
        K.obox(car, self.tiles(), (0, 0, 0), ((1, 0, 0), (0, 1, 0), (0, 0, 1)), (1.0, 0.7, 1.0))
        p0, p1 = a + up * 1.9, c + up * 1.9
        self.b.add("Comm_Shuttle", car, K.pose(tuple(p0)))
        self.ping_pong("Comm shuttle", self.graph.entity("Comm_Shuttle"), p0, p1, 2500.0)

    def ex_lift(self):
        """A platform in the way that is a real floor (Phys_Floors), moved up 7 and down again by a script: the way ends
        at its low place and goes on from its high one, so the ball rides it (none of the converted community maps
        has a scripted lift; ours is put where it has a job). The game moves its body with it. The cursor goes over
        it and up."""
        f, up, left = self.axes()
        c = self.at(3.0)
        m = K.Mesh("guides")
        for s in (-1.0, 1.0):
            K.cylinder(m, self.b.mat_ids["post"], tuple(c + left * s * 3.6 - up * 40.0), (0, 1, 0), 0.5, 49.0, seg=10)
        self.b.add("Comm_Lift_Guides", m)
        # the slab's mesh is in the level's coordinates (the kit's checks read floors so); Set Position moves the
        # object's origin, so the script's places are offsets
        a, b = self.at(0.0, -3.0, -0.6), self.at(6.0, 3.0, 0.0)
        self.b.slab("Comm_Lift", tuple(np.minimum(a, b)), tuple(np.maximum(a, b)), mat="plate", sector=self.sector)
        self.ping_pong("Comm lift", self.graph.entity("Comm_Lift"), (0, 0, 0), (0, LIFT_RISE, 0), 4000.0)
        self.slot(6.0)
        self.pos = self.pos + up * LIFT_RISE

    def ex_text2d(self):
        g = self.graph
        self.b.add("Comm_Text_Spot", None, K.pose(tuple(self.at(0.0, 0.0, 2.0))))
        # a ring on the floor's side shows where the text is switched
        top = self.stand()
        m = K.Mesh("ring")
        K.torus(m, self.b.mat_ids["lamp"], tuple(top + np.array([0.0, 2.2, 0.0])), tuple(self.fwd()), 1.6, 0.25)
        K.cylinder(m, self.b.mat_ids["rail"], tuple(top), (0, 1, 0), 0.2, 0.6, seg=8)
        self.b.add("Comm_Text_Ring", m)
        spot = g.entity("Comm_Text_Spot")
        prox, links = self.near_ball(spot, 9.0)
        text = g.bb("Text Display", S.TEXT_DISPLAY, ["On", "Off"], ["Exit On", "Exit Off"],
                    [g.const("Offset", S.VECTOR2, S.v2(20, 60)), g.const("Color", S.COLOR, S.colour(1, 0.9, 0.4)),
                     g.const("Alignment", S.INT, S.i(1)), g.const("Size", S.INT, S.i(20)),
                     g.const("Text", S.STRING, S.s("Text Display: shown while the ball is near"))],
                    locals_=[g.local("setting", S.NONE) for _ in range(4)]
                    + [g.local("Sprite Size", S.VECTOR2, S.v2(600, 40))])
        g.script("Comm text display", spot, [prox, text],
                 links + [(prox.o[0], text.i[0]), (prox.o[1], text.i[1])])

    def ex_text3d(self):
        g = self.graph
        top = self.stand()
        f, up, left = self.axes()
        m = K.Mesh("board")
        K.cylinder(m, self.b.mat_ids["wood"], tuple(top), (0, 1, 0), 0.2, 2.0, seg=8)
        K.obox(m, self.b.mat_ids["wood"], tuple(top + up * 3.6), (tuple(f), tuple(up), tuple(left)), (0.1, 1.7, 5.2))
        self.b.add("Comm_Text3D_Board", m)
        # the text lies on its frame's local XY plane: x along the reader's right, the frame scaled to the letters
        right = -left
        self.b.add("Comm_Text3D", None, K.pose(tuple(top + up * 3.6 - f * 0.15), tuple(right), tuple(up),
                                                 scale=(4.0, 4.0, 4.0)))
        frame = g.entity("Comm_Text3D")
        text = g.bb("3D Text", S.TEXT_3D, ["On", "Off"], ["Exit On", "Exit Off"],
                    [g.const("Font", S.INT, S.i(0)), g.const("Text", S.STRING, S.s("3D Text\nin the world")),
                     g.const("Alignment", S.INT, S.i(0)), g.const("Margins", S.RECT, S.rect(0, 0, 0, 0)),
                     g.const("Offset", S.VECTOR2, S.v2(0, 0))],
                    [g.pout("Text Extents", S.RECT), g.pout("Line Count", S.INT)], target=g.target(frame))
        g.script("Comm 3D text", frame, [text], [(S.START, text.i[0])])

    def ex_fountain(self):
        g = self.graph
        top = self.stand()
        m = K.Mesh("basin")
        K.cylinder(m, self.tiles(), tuple(top), (0, 1, 0), 1.9, 0.7, seg=20)
        K.cylinder(m, self.b.mat_ids["rail"], tuple(top + np.array([0.0, 0.7, 0.0])), (0, 1, 0), 0.3, 0.8, seg=10)
        self.b.add("Comm_Fountain_Basin", m)
        # the emitter throws along its frame's z: up here; its x and y (the plane the particles start on) kept small
        self.b.add("Comm_Fountain", None, K.pose(tuple(top + np.array([0.0, 1.6, 0.0])), (1, 0, 0), (0, 0, -1),
                                                   scale=(0.3, 0.3, 1.0)))
        frame = g.entity("Comm_Fountain")
        ff = lambda n, v: g.const(n, S.FLOAT, S.f(v))
        ii = lambda n, v: g.const(n, S.INT, S.i(v))
        pins = [ff("Emission Delay", 30), ff("Emission Delay Variance", 0), ff("Yaw Variance", 0.22),
                ff("Pitch Variance", 0.22), ff("Speed", 0.008), ff("Speed Variance", 0.002), ff("p6", 0), ff("p7", 0),
                ff("Lifespan", 1100), ff("Lifespan Variance", 200), ii("Maximum Number", 160), ii("Emission", 3),
                ii("Emission Variance", 1), ff("Initial Size", 0.5), ff("Initial Size Variance", 0.1),
                ff("Ending Size", 1.6), ff("Ending Size Variance", 0.2)]
        pins += [ff(f"p{k}", 0) for k in range(17, 23)]
        pins += [g.const("Initial Color", S.COLOR, S.colour(0.5, 0.8, 1.0, 1.0)), ff("p24", 0),
                 g.const("Ending Color", S.COLOR, S.colour(0.2, 0.4, 1.0, 0.0)), ff("p26", 0),
                 g.const("Texture", S.TEXTURE, S.obj(g.named("Particle_Smoke")))]
        # the settings by place (tt_particles.rs, read_spec): 3 the pool, 4 sprites, 5/6 the blend (alpha, one),
        # 8 size and colour change over the life, 9 which variances apply (speed, life, emission, sizes)
        settings = [None, S.i(1), S.f(0), S.i(160), S.i(3), S.i(5), S.i(2), S.obj(None), S.i(3), S.i(1 | 4 | 8 | 48)]
        emit = g.bb("PlanarParticleSystem", S.PLANAR_PARTICLE_SYSTEM, ["On", "Off", "Freeze"],
                    ["Exit On", "Exit Off", "Exit Freeze"], pins, locals_=[g.local("setting", S.NONE, v) for v in settings])
        g.script("Comm fountain", frame, [emit], [(S.START, emit.i[0])])

    def ex_movie(self):
        """A screen whose picture changes four times a second: Set Texture on a timer (the open format carries no
        texture slots for Movie Player)."""
        g = self.graph
        frames = []
        for k in range(4):
            def make(k=k):
                u, v = _grid()
                a = np.arctan2(v - 0.5, u - 0.5) - k * math.pi / 2
                r = np.hypot(u - 0.5, v - 0.5)
                hand = (np.abs(np.angle(np.exp(1j * a))) < 0.5) & (r < 0.42)
                img = np.where(hand[..., None], np.array([1.0, 0.8, 0.2]), np.array([0.08, 0.1, 0.16]))
                img[(r > 0.42) & (r < 0.46)] = (0.9, 0.9, 0.9)
                return img, None
            frames.append(self.picture(f"Comm_Movie_{k}.png", make))
        mat = self.b.mat("comm_movie", frames[0], diffuse=1.0, emissive=0.9, unlit=True)
        # the other frames are textures of the level only through a material that names them
        for k, name in enumerate(frames[1:], 1):
            self.b.mat(f"comm_movie_frame{k}", name, diffuse=1.0, emissive=0.9, unlit=True)
        top = self.stand()
        c = top + np.array([0.0, 2.8, 0.0])
        self.frame("Comm_Movie_Frame", c)
        screen = self.panel("Comm_Movie_Screen", mat, c)
        steps = [g.bb("Set Texture", S.SET_TEXTURE, ["In"], ["Out"],
                      [g.const("Texture", S.TEXTURE, S.obj(g.texture(name)))], target=g.target(g.material("comm_movie")))
                 for name in frames]
        del screen
        self.loop_steps("Comm movie", g.entity("Comm_Movie_Screen"), steps, 250.0)

    def ex_blink(self):
        g = self.graph
        f, up, left = self.axes()
        top = self.stand()
        m = K.Mesh("posts")
        for s in (-1.0, 1.0):
            K.cylinder(m, self.b.mat_ids["rail"], tuple(top + f * s * 1.2), (0, 1, 0), 0.2, 1.6, seg=8)
        self.b.add("Comm_Blink_Posts", m)
        lamp = K.Mesh("lamp")
        K.sphere(lamp, self.b.mat_ids["lamp"], tuple(top - f * 1.2 + up * 2.4), 0.9, seg=16)
        self.b.add("Comm_Blink_Lamp", lamp)
        mat = self.b.mat("comm_colour", None, diffuse=(1.0, 0.2, 0.2), emissive=0.25, specular=0.5, power=40)
        ball = K.Mesh("colour")
        K.sphere(ball, mat, tuple(top + f * 1.2 + up * 2.4), 0.9, seg=16)
        self.b.add("Comm_Colour_Ball", ball)
        ent = g.entity("Comm_Blink_Lamp")
        vis = [g.bb(n, p, ["In"], ["Out"], [g.const("Hierarchy", S.BOOL, S.b(False))], target=g.target(ent))
               for n, p in (("Hide", S.HIDE), ("Show", S.SHOW))]
        self.loop_steps("Comm blink", ent, vis, 500.0)
        tints = [g.bb("Set Diffuse", S.SET_DIFFUSE, ["In"], ["Out"],
                      [g.const("Diffuse Color", S.COLOR, S.colour(*c)),
                       g.const("Keep Alpha Component", S.BOOL, S.b(True))], target=g.target(g.material("comm_colour")))
                 for c in ((0.2, 1.0, 0.3), (0.2, 0.4, 1.0), (1.0, 0.2, 0.2))]
        self.loop_steps("Comm colours", g.entity("Comm_Colour_Ball"), tints, 700.0)

    def arch(self, name, at, mat):
        f, up, left = self.axes()
        ax = (tuple(f), tuple(up), tuple(left))
        m = K.Mesh(name)
        for s in (-1.0, 1.0):
            K.obox(m, mat, tuple(at + left * s * 2.9 + up * 2.6), ax, (0.3, 2.6, 0.3))
        K.obox(m, mat, tuple(at + up * 5.5), ax, (0.3, 0.3, 3.2))
        self.b.add(name, m)

    def ex_portal(self):
        """A portal at the end of the way: the ball rolling through the arch 3 before the floor's end is set down under
        a second arch on the floor beyond a gap of 18 (map 05 See You Tomorrow, the one converted map with portals:
        its trigger boxes stand on the way's floor, the ball set down on a floor far on). Box Box Intersection of the
        ball with an unseen box in the arch, tested every tick."""
        g = self.graph
        f, up, left = self.axes()
        a, c = self.at(-3.0), self.at(PORTAL_GAP + 3.0)
        def make():
            u, v = _grid()
            r = np.hypot(u - 0.5, v - 0.5) * 2
            rings = 0.5 + 0.5 * np.cos(r * 18 + np.arctan2(v - 0.5, u - 0.5) * 2)
            img = np.array([0.25, 0.45, 1.0]) + rings[..., None] * np.array([0.5, 0.45, 0.0])
            return np.clip(img, 0, 1), 0.35 + 0.4 * rings
        veil = self.b.mat("comm_portal", self.picture("Comm_Portal.png", make), diffuse=1.0, emissive=1.0,
                          alpha="BLEND", unlit=True, double_sided=True)
        for k, p in enumerate((a, c), 1):
            self.arch(f"Comm_Portal_Arch_{k}", p, self.b.mat_ids["rail"])
            self.panel(f"Comm_Portal_Veil_{k}", veil, p + up * 2.6, half=(2.6, 2.6), both=False)
        box = K.Mesh("trigger")
        K.box(box, None, (-0.3, -1.5, -1.6), (0.3, 1.5, 1.6))
        self.b.add("Comm_Portal_Trigger", box, K.pose(tuple(a + up * 2.0)), visible=False)
        trig = g.entity("Comm_Portal_Trigger")
        hit = g.bb("Box Box Intersection", S.BOX_BOX_INTERSECTION, ["In"], ["True", "False"],
                   [g.const("Entity 0", S.ENTITY, S.obj(g.ball())), g.const("Entity 1", S.ENTITY, S.obj(trig))])
        out = c + f * 3.0 + up * 2.3
        put = g.bb("Set Position", S.SET_POSITION, ["In"], ["Out"],
                   [g.const("Position", S.VECTOR, S.v3(*out)), g.const("Referential", S.ENTITY, S.obj(None))],
                   target=g.target(g.ball()))
        g.script("Comm portal", trig, [hit, put],
                 [(S.START, hit.i[0]), (hit.o[1], hit.i[0], 1), (hit.o[0], put.i[0]), (put.o[0], hit.i[0], 1)])

    def ex_crate(self):
        """A crate on the wide floor's right side that only a script makes a body (Physicalize): the ball pushes it. It
        begins a unit over the floor and drops as the level starts: a body from its first tick."""
        g = self.graph
        m = K.Mesh("crate")
        K.box(m, self.b.mat_ids["wood"], (-1.5, -1.5, -1.5), (1.5, 1.5, 1.5), scale=3.0)
        self.b.add("Comm_Crate", m, K.pose(tuple(self.at(4.0, -9.0, 2.6))))
        crate = g.entity("Comm_Crate")
        pins = [g.const("Fixed?", S.BOOL, S.b(False)), g.const("Friction", S.FLOAT, S.f(0.7)),
                g.const("Elasticity", S.FLOAT, S.f(0.3)), g.const("Mass", S.FLOAT, S.f(0.8)),
                g.const("Collision Group", S.STRING, S.s("")), g.const("Start Frozen", S.BOOL, S.b(False)),
                g.const("Enable Collision", S.BOOL, S.b(True)), g.const("Automatic Calculate Mass Center", S.BOOL,
                                                                        S.b(True)),
                g.const("Linear Speed Dampening", S.FLOAT, S.f(0.1)), g.const("Rot Speed Dampening", S.FLOAT, S.f(0.1)),
                g.const("Collision Surface", S.STRING, S.s("Comm_Crate"))]
        # the settings: one convex shape (the object's own mesh), no balls, no concave ones, no shift of the centre
        settings = [S.i(1), S.i(0), S.i(0), S.v3(0, 0, 0)]
        phys = g.bb("Physicalize", S.PHYSICALIZE, ["Physicalize", "Unphysicalize"], ["Out", "Out 2"], pins,
                    locals_=[g.local("setting", S.NONE, v) for v in settings], target=g.target(crate))
        g.script("Comm crate", crate, [phys], [(S.START, phys.i[0])])

    # --- the environment and the end ------------------------------------------------------------------------------

    def environment(self):
        """Another sky, fog and light while the ball is on the environment's stretch: a proximity test of the ball
        against a frame in its middle, as far as the stretch reaches."""
        g = self.graph
        start, end = self.env_zone
        centre = (start + end) / 2
        reach = float(np.linalg.norm(end - start)) / 2
        self.b.add("Comm_Environment", None, K.pose(tuple(centre)))
        marker = g.entity("Comm_Environment")
        prox, links = self.near_ball(marker, reach)
        # the sky's walls, lid and floor: our own Sunset sky
        # (copies of our own pictures in the level: the game's skies are not among the textures a level can name)
        mats = []
        for w in ("Front", "Right", "Back", "Left", "Top", "Down"):
            shutil.copyfile(os.path.join(L.SKY_DIR, f"Sky_Sunset_{w}.jpg"), os.path.join(self.b.tex, f"Comm_Sky_{w}.jpg"))
            self.b.mat(f"comm_sky_{w.lower()}", f"Comm_Sky_{w}.jpg", diffuse=1.0, emissive=1.0, unlit=True)
            mats.append(g.material(f"comm_sky_{w.lower()}"))
        sky = g.bb("TT Sky", S.TT_SKY, ["On", "Off"], ["Exit On", "Exit Off"],
                   [g.const("Distortion", S.FLOAT, S.f(0)), g.const("Vertex Color", S.COLOR, S.colour(1, 1, 1)),
                    g.const("Orientation Object", S.ENTITY, S.obj(None)), g.const("Radius", S.FLOAT, S.f(50)),
                    g.const("Quadratic SideFaces?", S.BOOL, S.b(True)), g.const("or SideFace-Heigth", S.FLOAT, S.f(100)),
                    g.const("Y-Position of Sky", S.FLOAT, S.f(0))]
                   + [g.const(f"Material {k}", S.MATERIAL, S.obj(m)) for k, m in enumerate(mats)],
                   locals_=[g.local("Sides", S.INT, S.i(4)), g.local("Top", S.BOOL, S.b(True)),
                            g.local("Bottom", S.BOOL, S.b(True))])
        light = g.named("Light_Ingame")

        def env(fog_mode, fog, tint):
            f_ = g.bb("Set Fog", S.SET_FOG, ["In"], ["Out"],
                      [g.const("Fog Mode", S.INT, S.i(fog_mode)), g.const("Fog Start", S.FLOAT, S.f(120)),
                       g.const("Fog End", S.FLOAT, S.f(900)), g.const("Fog Density", S.FLOAT, S.f(0.001)),
                       g.const("Fog Color", S.COLOR, S.colour(*fog))])
            l_ = g.bb("Set Light Color", S.SET_LIGHT_COLOR, ["In"], ["Out"], [g.const("Color", S.COLOR, S.colour(*tint))],
                      target=g.target(light))
            return f_, l_
        fog_on, light_on = env(3, (0.95, 0.6, 0.4), (1.0, 0.82, 0.66))
        fog_off, light_off = env(0, (1, 1, 1), (1, 1, 1))
        g.script("Comm environment", marker, [prox, sky, fog_on, light_on, fog_off, light_off],
                 links + [(prox.o[0], sky.i[0]), (prox.o[0], fog_on.i[0]), (prox.o[0], light_on.i[0]),
                          (prox.o[1], sky.i[1]), (prox.o[1], fog_off.i[0]), (prox.o[1], light_off.i[0])])

    def community_finish(self):
        self.environment()
        self.b.scripts = self.graph.json()
