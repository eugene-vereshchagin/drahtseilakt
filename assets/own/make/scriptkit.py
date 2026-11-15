#!/usr/bin/env python3
"""Builds a level's behaviour graphs for the open format (docs/drahtseilakt/open-format.md, "Scripts";
docs/drahtseilakt/scripts.md): the same data a converted community map carries from its Virtools scripts, made here
node by node for our own levels, and run by the same runtime (crates/script). It mirrors the test helper `Build` of
crates/script/src/runtime_tests.rs, and writes what crates/level/src/script_json.rs reads.

A graph is a list of objects; an object is one of the level's things (an entity = a node of the level, a material, a
texture, the ball) or a node of the graph (a block, a link, an in/out, a parameter). The level's things are named
here by NAME (`entity("Lift_01")`); ownkit.write_open turns the names into the indices the reader counts.

    g = Graph()
    wheel = g.entity("Wheel_01")
    rot = g.bb("Rotate", ROTATE, ["In"], ["Out"],
               [g.const("Axis of Rotation", VECTOR, v3(0, 1, 0)), g.const("Angle of Rotation", ANGLE, f(0.02)),
                g.const("Referential of the Axis", ENTITY, obj(None))], target=g.target(wheel))
    g.script("Turn the wheel", wheel, [rot], [(START, rot.i[0]), (rot.o[0], rot.i[0], 1)])
    builder.scripts = g.json()

Every script listed starts active and its one input (START in the links) fires at the start; a block is repeated every
tick by a link from its output back to its input with a delay of 1. Times are milliseconds, angles radians, positions
the game's coordinates (as the level's matrices). The pins of each block, in order, are in crates/script/src/blocks/.
"""

# --- the blocks' prototypes (crates/script/src/blocks/mod.rs, `proto`) and those of environment.rs ------------------
ROTATE = (0xffffffee, 0xeeffffff)
SET_POSITION = (0xe456e78a, 0x456789aa)
LINEAR_PROGRESSION = (0xfff45680, 0xaa512a39)
BEZIER_PROGRESSION = (0x6bb8699a, 0x29fb6a4b)
INTERPOLATOR = (0x35503950, 0x2dde7a65)
TIMER = (0xa2a5a63a, 0xe4e7e8e5)
DELAYER = (0x15d472a5, 0x3bea409f)
COUNTER = (0x998f000f, 0xf000f899)
PARAMETER_SELECTOR = (0x63eb2b0c, 0x27e2767e)
BINARY_SWITCH = (0xeb506901, 0x984afccc)
TEST = (0x17d66d26, 0x726b7dec)
SEQUENCER = (0x42530844, 0x257b6053)
TT_SCALEABLE_PROXIMITY = (0x2a2a63ca, 0x00826175)
PROXIMITY = (0x5321cacb, 0xdcdc5213)
BOX_BOX_INTERSECTION = (0x64154401, 0x76cf37af)
HIDE = (0x31d97d82, 0x78d54d98)
SHOW = (0xa85a213a, 0xef78d52a)
TEXT_DISPLAY = (0xf22d010a, 0xf2cd010a)
TEXT_3D = (0x1f5b2fe7, 0x6d355175)
CREATE_SYSTEM_FONT = (0x936334fc, 0xf243684f)
SET_FONT_PROPERTIES = (0xdacfbd61, 0x7a6e65e7)
MOVIE_PLAYER = (0x778d16d4, 0x1dd60060)
SET_TEXTURE = (0xeb123eb5, 0x5be321be)
SET_DIFFUSE = (0xe1e1e1e1, 0x1e1e1e1e)
SET_EMISSIVE = (0xd1d1d1d1, 0x30303030)
SET_CURRENT_SLOT = (0xaaaa213a, 0xeaa8d52a)
PLANAR_PARTICLE_SYSTEM = (0x49957bfe, 0x0ff27ffc)
SPHERICAL_PARTICLE_SYSTEM = (0x67c88f47, 0x8880721e)
PHYSICALIZE = (0x7522370e, 0x37ec15ec)
SET_PHYSICS_FORCE = (0x56e20c57, 0x0b926068)
PHYSICS_IMPULSE = (0x0c7e39bb, 0x16db20d5)
SET_LIGHT_COLOR = (0x32115590, 0x78951230)
SET_FOG = (0x151aaaaa, 0xffd5bdbc)
TT_SKY = (0x36691920, 0x3b261630)

# --- parameter types (crates/script/src/types.rs) --------------------------------------------------------------------
NONE = (0x1cb10760, 0x419f50c5)
INT = (0x5a5716fd, 0x44e276d7)
FLOAT = (0x47884c3f, 0x432c2c20)
BOOL = (0x1ad52a8e, 0x5e741920)
STRING = (0x6bd010e2, 0x115617ea)
VECTOR = (0x48824eae, 0x2fe47960)
VECTOR2 = (0x4efcb34a, 0x6079e42f)
COLOR = (0x57d42fee, 0x7cbb3b91)
TIME = (0x54b4422b, 0x730f0f4f)
ANGLE = (0x11262cf5, 0x30b0233a)
RECT = (0x7ab20d20, 0x693044a9)
OBJECT = (0x30ec20ab, 0x6df6517d)
ENTITY = (0x5b8a05d5, 0x31ea28d4)
MATERIAL = (0x55ab12cd, 0x22ae6a8b)
TEXTURE = (0x155b2870, 0x183679f8)

# A script's own input in its links (Graph.script).
START = "start"


# --- values, as script_json.rs reads them -----------------------------------------------------------------------------

def f(x):
    return ["f", float(x)]


def i(x):
    return ["i", int(x)]


def b(x):
    return ["b", bool(x)]


def s(x):
    return ["s", str(x)]


def obj(x):
    """An object of the graph (its id), or none."""
    return ["o", None if x is None else int(x)]


def v2(x, y):
    return ["v2", [float(x), float(y)]]


def v3(x, y, z):
    return ["v3", [float(x), float(y), float(z)]]


def colour(r, g, b_, a=1.0):
    return ["c", [float(r), float(g), float(b_), float(a)]]


def rect(a, b_, c, d):
    return ["rect", [float(a), float(b_), float(c), float(d)]]


class Block:
    """A block made by Graph.bb: its id and its inputs' (`i`), outputs' (`o`), pins' and parameter outputs' ids."""

    def __init__(self, id_, inputs, outputs, pins, pouts):
        self.id, self.i, self.o, self.pins, self.pouts = id_, inputs, outputs, pins, pouts


class Graph:
    """The graphs of one level: objects and nodes by id (ids are handed out in order), and the scripts."""

    def __init__(self):
        self.objects = []  # [name, class, [kind, index or name]]
        self.nodes = {}  # id -> the node's body
        self.scripts = []  # [id, owner]
        self._things = {}

    # --- objects ------------------------------------------------------------------------------------------------

    def _object(self, name, kind, ref=0):
        self.objects.append([name, 0, [kind, ref]])
        return len(self.objects) - 1

    def _thing(self, name, kind, ref):
        key = (kind, ref)
        if key not in self._things:
            self._things[key] = self._object(name, kind, ref)
        return self._things[key]

    def entity(self, node_name):
        """The level's node of that name (an object the blocks move, show, test)."""
        return self._thing(node_name, "entity", node_name)

    def material(self, name):
        return self._thing(name, "material", name)

    def texture(self, file_name):
        """One of the level's textures, by its file's name as the materials give it (`Name.png`, `game:Name`)."""
        return self._thing(file_name, "texture", file_name)

    def ball(self):
        """The player's ball (its place, its box; Set Position on it moves the ball)."""
        return self._thing("Ball_Pos_Frame", "ball", 0)

    def named(self, name):
        """An object that is only a name to the engine (the game's light `Light_Ingame`, a particle texture)."""
        return self._thing(name, "other", name)

    def _node(self, name, body):
        id_ = self._object(name, "other")
        self.nodes[id_] = body
        return id_

    # --- parameters ---------------------------------------------------------------------------------------------

    def local(self, name, type_, value=None):
        """A parameter holding a value (a block's setting, a constant a pin reads, a parameter output)."""
        return self._node(name, {"t": "param", "name": name, "guid": list(type_), "value": value, "source": None,
                                 "shared": False, "dest": []})

    def pin(self, name, type_, source):
        """A block's input parameter reading `source` (a local or another block's parameter output)."""
        return self._node(name, {"t": "param", "name": name, "guid": list(type_), "value": None, "source": source,
                                 "shared": False, "dest": []})

    def const(self, name, type_, value):
        """A pin with a constant of its own."""
        return self.pin(name, type_, self.local(name, type_, value))

    def pout(self, name, type_):
        return self.local(name, type_, None)

    def target(self, thing):
        """A block's target parameter naming an object of the graph (entity(), material(), texture(), ball())."""
        return self.local("Target", OBJECT, obj(thing))

    # --- blocks, links, scripts ---------------------------------------------------------------------------------

    def bb(self, name, proto, ins, outs, pins=(), pouts=(), locals_=(), target=None):
        """A building block with inputs and outputs named as given; returns a Block."""
        io = lambda n, is_in: self._node(n, {"t": "io", "input": is_in})
        inputs = [io(n, True) for n in ins]
        outputs = [io(n, False) for n in outs]
        id_ = self._node(name, {"t": "block", "name": name, "proto": list(proto), "flags": 0, "priority": 0,
                                "target": target, "inputs": inputs, "outputs": outputs, "pins": list(pins),
                                "pouts": list(pouts), "locals": list(locals_), "sub": [], "links": [], "ops": []})
        return Block(id_, inputs, outputs, list(pins), list(pouts))

    def script(self, name, owner, blocks, links):
        """A script (a top-level graph) of the object `owner` (None: nobody's) holding `blocks` and `links`: each
        (from, to) or (from, to, delay in ticks), where from/to are in/out ids and START is the script's own input,
        which fires when the level starts."""
        start = self._node("Start", {"t": "io", "input": True})
        ids = []
        for link in links:
            src, dst = link[0], link[1]
            delay = link[2] if len(link) > 2 else 0
            ids.append(self._node("link", {"t": "link", "delay": int(delay), "src": start if src is START else src,
                                           "dst": start if dst is START else dst}))
        id_ = self._node(name, {"t": "block", "name": name, "proto": None, "flags": 2, "priority": 0, "target": None,
                                "inputs": [start], "outputs": [], "pins": [], "pouts": [], "locals": [],
                                "sub": [x.id if isinstance(x, Block) else x for x in blocks], "links": ids,
                                "ops": []})
        self.scripts.append([id_, owner])
        return id_

    def json(self):
        """The graphs as the scene's `scripts` (ownkit.write_open resolves the names of the level's things)."""
        objects = [[n, c, [k, 0 if k == "other" else r]] for n, c, (k, r) in self.objects]
        return {"objects": objects, "nodes": [[k, v] for k, v in sorted(self.nodes.items())], "arrays": [],
                "scripts": self.scripts}
