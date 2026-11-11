// The behaviour graphs of a community map as data: `Graph` is what the file says (blocks, links, parameters,
// operations, arrays; docs/drahtseilakt/scripts.md), `Runtime` (exec.rs) drives it a tick at a time through a
// `Host`, the engine's side (objects, physics, sounds, text). Nothing in a graph is run as code: a block is a
// prototype GUID our runtime either knows (blocks.rs) or reports. Every count is capped; indices are checked.

pub mod blocks;
pub mod exec;
#[cfg(test)]
mod runtime_tests;

pub use exec::{Ent, Host, Runtime};
pub use nmo::behavior::Guid;

use nmo::behavior::{self, Saved};
use nmo::{Document, class};
use std::collections::{HashMap, HashSet};

/// An index into `Graph::nodes` and `Graph::objects`: the object's index in the file.
pub type Id = usize;

/// Caps on a map's graphs (a hostile file may claim any count).
pub const MAX_OBJECTS: usize = 1 << 20;
pub const MAX_ARRAY_ROWS: usize = 1 << 16;
/// The most nodes one script may have to be copied (Object Copy copies an object's scripts with it).
pub const MAX_SCRIPT_NODES: usize = 1 << 14;

/// What an object of the file is to the engine: the level's entity, group, mesh, material or texture by index,
/// one of the graphs' own arrays, or something the engine has no handle on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjKind {
    Entity(usize),
    Group(usize),
    Mesh(usize),
    Material(usize),
    Texture(usize),
    Array(usize),
    Behavior,
    /// The game's ball frame (`Ball_Pos_Frame`), a pseudo-object the engine adds.
    Ball,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjInfo {
    pub name: String,
    pub class: u32,
    pub kind: ObjKind,
}

/// A parameter's value.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    None,
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(String),
    Obj(Option<Id>),
    Vec3([f32; 3]),
    Vec2([f32; 2]),
    Color([f32; 4]),
    Matrix([[f32; 4]; 4]),
    /// Bytes of a type the runtime does not decode; read as a number when a block wants one.
    Raw(Vec<u8>),
    /// A script's "This": the object the script is attached to, which the runtime puts in its place when the
    /// script starts (a copy of the script gets the copy of the object).
    Owner,
}

pub mod types {
    use super::Guid;
    pub const NONE: Guid = Guid(0x1cb1_0760, 0x419f_50c5);
    pub const INT: Guid = Guid(0x5a57_16fd, 0x44e2_76d7);
    pub const FLOAT: Guid = Guid(0x4788_4c3f, 0x432c_2c20);
    pub const BOOL: Guid = Guid(0x1ad5_2a8e, 0x5e74_1920);
    pub const STRING: Guid = Guid(0x6bd0_10e2, 0x1156_17ea);
    pub const VECTOR: Guid = Guid(0x4882_4eae, 0x2fe4_7960);
    pub const VECTOR2: Guid = Guid(0x4efc_b34a, 0x6079_e42f);
    pub const MATRIX: Guid = Guid(0x643f_046e, 0x6521_1b71);
    pub const COLOR: Guid = Guid(0x57d4_2fee, 0x7cbb_3b91);
    pub const TIME: Guid = Guid(0x54b4_422b, 0x730f_0f4f);
    pub const MESSAGE: Guid = Guid(0x0388_1e12, 0x5ba3_4e2b);
    pub const OBJECT: Guid = Guid(0x30ec_20ab, 0x6df6_517d);
    pub const BEOBJECT: Guid = Guid(0x71d8_0779, 0x402f_42f3);
    pub const ENTITY: Guid = Guid(0x5b8a_05d5, 0x31ea_28d4);
    pub const GROUP: Guid = Guid(0x5c0f_151b, 0x6f05_47fd);
    pub const SCRIPT: Guid = Guid(0x7ea4_176d, 0x1b40_5d30);
    pub const ARRAY: Guid = Guid(0x024d_52f1, 0x6782_23b2);
    pub const MESH: Guid = Guid(0x2453_5345, 0x65d1_5229);
    pub const MATERIAL: Guid = Guid(0x55ab_12cd, 0x22ae_6a8b);
}

fn le_f32(b: &[u8], at: usize) -> Option<f32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(f32::from_le_bytes([
        *s.first()?,
        *s.get(1)?,
        *s.get(2)?,
        *s.get(3)?,
    ]))
}

fn le_i32(b: &[u8], at: usize) -> Option<i32> {
    le_f32(b, at).map(|f| f.to_bits() as i32)
}

impl Value {
    /// A saved value by its type; unknown types are kept raw, decoded by their size when a block needs a number.
    pub fn decode(guid: Guid, saved: &Saved) -> Value {
        match saved {
            Saved::None | Saved::SubChunk | Saved::Type(_) => Value::None,
            Saved::Object(o) => Value::Obj(*o),
            Saved::Name(n) => Value::Str(String::from_utf8_lossy(n).into_owned()),
            Saved::Bytes(b) => {
                let f = |i: usize| le_f32(b, i.saturating_mul(4)).unwrap_or(0.0);
                match guid {
                    types::INT => Value::Int(le_i32(b, 0).unwrap_or(0)),
                    types::FLOAT | types::TIME => Value::Float(f(0)),
                    types::BOOL => Value::Bool(le_i32(b, 0).unwrap_or(0) != 0),
                    types::STRING => {
                        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
                        Value::Str(
                            String::from_utf8_lossy(b.get(..end).unwrap_or_default()).into_owned(),
                        )
                    }
                    types::VECTOR if b.len() >= 12 => Value::Vec3([f(0), f(1), f(2)]),
                    types::VECTOR2 if b.len() >= 8 => Value::Vec2([f(0), f(1)]),
                    types::COLOR if b.len() >= 16 => Value::Color([f(0), f(1), f(2), f(3)]),
                    types::MATRIX if b.len() >= 64 => Value::Matrix(std::array::from_fn(|r| {
                        std::array::from_fn(|c| f(r.saturating_mul(4).saturating_add(c)))
                    })),
                    _ => Value::Raw(b.clone()),
                }
            }
        }
    }

    pub fn as_int(&self) -> i32 {
        match self {
            Value::Int(i) => *i,
            Value::Float(f) => *f as i32,
            Value::Bool(b) => i32::from(*b),
            Value::Str(s) => s.trim().parse().unwrap_or(0),
            Value::Raw(b) => le_i32(b, 0).unwrap_or(0),
            _ => 0,
        }
    }

    pub fn as_float(&self) -> f32 {
        match self {
            Value::Int(i) => *i as f32,
            Value::Float(f) => *f,
            Value::Bool(b) => f32::from(u8::from(*b)),
            Value::Str(s) => s.trim().parse().unwrap_or(0.0),
            Value::Raw(b) => le_f32(b, 0).unwrap_or(0.0),
            _ => 0.0,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Obj(o) => o.is_some(),
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::None => false,
            v => v.as_int() != 0,
        }
    }

    pub fn as_obj(&self) -> Option<Id> {
        match self {
            Value::Obj(o) => *o,
            _ => None,
        }
    }

    pub fn as_vec3(&self) -> [f32; 3] {
        match self {
            Value::Vec3(v) => *v,
            Value::Raw(b) if b.len() >= 12 => {
                [le_f32(b, 0), le_f32(b, 4), le_f32(b, 8)].map(|x| x.unwrap_or(0.0))
            }
            Value::Matrix(m) => [m[3][0], m[3][1], m[3][2]],
            _ => [0.0; 3],
        }
    }

    pub fn as_matrix(&self) -> Option<[[f32; 4]; 4]> {
        match self {
            Value::Matrix(m) => Some(*m),
            _ => None,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format!("{f}"),
            Value::Bool(b) => i32::from(*b).to_string(),
            _ => String::new(),
        }
    }

    /// Equality as a Test or Switch On Parameter compares: numbers by value, strings exactly, objects by identity.
    pub fn same(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Obj(a), Value::Obj(b)) => a == b,
            (Value::Obj(_), _) | (_, Value::Obj(_)) => false,
            (Value::Float(_), _) | (_, Value::Float(_)) => {
                (self.as_float() - other.as_float()).abs() < 1e-6
            }
            _ => self.as_int() == other.as_int(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub name: String,
    /// The building block's prototype; a graph (a box of other blocks) has none.
    pub proto: Option<Guid>,
    pub flags: u32,
    pub priority: i32,
    /// The parameter holding the block's target, when it has its own; else the target is the script's owner.
    pub target: Option<Id>,
    pub inputs: Vec<Id>,
    pub outputs: Vec<Id>,
    pub pins: Vec<Id>,
    pub pouts: Vec<Id>,
    pub locals: Vec<Id>,
    pub sub: Vec<Id>,
    pub links: Vec<Id>,
    pub ops: Vec<Id>,
    pub parent: Option<Id>,
    /// The object the script is attached to (the default target of its blocks).
    pub owner: Option<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub delay: u16,
    pub src: Id,
    pub dst: Id,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Io {
    pub owner: Option<Id>,
    pub is_input: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Param {
    pub name: String,
    pub guid: Guid,
    /// Its saved value (outputs and locals); an input's is read from its source.
    pub value: Value,
    /// An input parameter's source: the parameter it reads (direct) or the input it shares (shared).
    pub source: Option<Id>,
    pub shared: bool,
    /// Parameters this one is copied to when it changes.
    pub destinations: Vec<Id>,
    /// The operation whose output this is, run before the value is read.
    pub op: Option<Id>,
    /// The block this parameter belongs to.
    pub block: Option<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Op {
    pub guid: Guid,
    pub inputs: [Option<Id>; 2],
    pub output: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Array {
    pub name: String,
    /// Column names and kinds (1 int, 2 float, 3 string, 4 object, 5 parameter).
    pub columns: Vec<(String, u32)>,
    pub rows: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Node {
    #[default]
    None,
    Block(Block),
    Link(Link),
    Io(Io),
    Param(Param),
    Op(Op),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Graph {
    pub objects: Vec<ObjInfo>,
    pub nodes: Vec<Node>,
    pub arrays: Vec<Array>,
    /// The scripts (top-level graphs), in file order.
    pub scripts: Vec<Id>,
    /// For each IO, the links leaving it.
    pub links_from: Vec<Vec<Id>>,
}

impl Graph {
    /// The graphs of a file. `kind` says what each object of the file is to the engine (by its index).
    pub fn from_document(doc: &Document, kind: &dyn Fn(usize) -> ObjKind) -> Graph {
        let n = doc.objects.len().min(MAX_OBJECTS);
        let mut g = Graph {
            objects: Vec::with_capacity(n),
            nodes: vec![Node::None; n],
            arrays: Vec::new(),
            scripts: Vec::new(),
            links_from: vec![Vec::new(); n],
        };
        for (i, o) in doc.objects.iter().enumerate().take(n) {
            let mut k = kind(i);
            if o.class_id == class::DATA_ARRAY
                && let Some(chunk) = &o.chunk
                && let Ok(a) = nmo::scene::data_array(chunk)
            {
                k = ObjKind::Array(g.arrays.len());
                g.arrays.push(Array {
                    name: o.name_lossy(),
                    columns: a
                        .columns
                        .iter()
                        .map(|c| String::from_utf8_lossy(c).into_owned())
                        .collect::<Vec<_>>()
                        .into_iter()
                        .zip(
                            // the kinds are not kept by the decoder: infer each from the cells, else object
                            (0..a.columns.len()).map(|c| {
                                a.rows.first().and_then(|r| r.get(c)).map_or(4, cell_kind)
                            }),
                        )
                        .collect(),
                    rows: a
                        .rows
                        .iter()
                        .take(MAX_ARRAY_ROWS)
                        .map(|r| r.iter().map(cell_value).collect())
                        .collect(),
                });
            }
            if o.class_id == behavior::class::BEHAVIOR {
                k = ObjKind::Behavior;
            }
            g.objects.push(ObjInfo {
                name: o.name_lossy(),
                class: o.class_id,
                kind: k,
            });
            let Some(chunk) = &o.chunk else { continue };
            let node = match o.class_id {
                behavior::class::BEHAVIOR => behavior::behavior(chunk).ok().map(|b| {
                    Node::Block(Block {
                        name: o.name_lossy(),
                        proto: b.proto.map(|(guid, _)| guid),
                        flags: b.flags,
                        priority: b.priority,
                        target: b.target,
                        inputs: b.inputs,
                        outputs: b.outputs,
                        pins: b.inputs_params,
                        pouts: b.output_params,
                        locals: b.local_params,
                        sub: b.sub_behaviors,
                        links: b.links,
                        ops: b.operations,
                        parent: None,
                        owner: None,
                    })
                }),
                behavior::class::BEHAVIOR_LINK => behavior::link(chunk).ok().and_then(|l| {
                    Some(Node::Link(Link {
                        delay: l.delay,
                        src: l.source?,
                        dst: l.destination?,
                    }))
                }),
                behavior::class::BEHAVIOR_IO => Some(Node::Io(Io {
                    owner: None,
                    is_input: behavior::io_flags(chunk) & 1 != 0,
                })),
                behavior::class::PARAMETER_IN => behavior::parameter_in(chunk).ok().map(|p| {
                    Node::Param(Param {
                        name: o.name_lossy(),
                        guid: p.guid,
                        source: p.source,
                        shared: p.shared,
                        ..Param::default()
                    })
                }),
                behavior::class::PARAMETER_LOCAL if behavior::parameter_is_owner(chunk) => {
                    Some(Node::Param(Param {
                        name: o.name_lossy(),
                        guid: types::BEOBJECT,
                        value: Value::Owner,
                        ..Param::default()
                    }))
                }
                behavior::class::PARAMETER_OUT | behavior::class::PARAMETER_LOCAL => {
                    behavior::parameter(chunk).ok().map(|p| {
                        Node::Param(Param {
                            name: o.name_lossy(),
                            guid: p.guid,
                            value: Value::decode(p.guid, &p.value),
                            destinations: p.destinations.into_iter().filter(|&d| d < n).collect(),
                            ..Param::default()
                        })
                    })
                }
                behavior::class::PARAMETER_OPERATION => behavior::operation(chunk).ok().map(|op| {
                    Node::Op(Op {
                        guid: op.guid,
                        inputs: op.inputs,
                        output: op.output,
                    })
                }),
                _ => None,
            };
            if let (Some(node), Some(slot)) = (node, g.nodes.get_mut(i)) {
                *slot = node;
            }
        }
        g.wire();
        // Scripts: behaviours no other behaviour lists as a sub-behaviour, owned by the objects that list them.
        let mut owners: Vec<Option<Id>> = vec![None; n];
        for (i, o) in doc.objects.iter().enumerate().take(n) {
            let Some(chunk) = &o.chunk else { continue };
            if o.class_id == behavior::class::BEHAVIOR
                || o.class_id == behavior::class::BEHAVIOR_LINK
            {
                continue;
            }
            for s in behavior::scripts(chunk) {
                if let Some(slot) = owners.get_mut(s) {
                    *slot = Some(i);
                }
            }
        }
        for i in 0..n {
            let is_root = matches!(g.nodes.get(i), Some(Node::Block(b)) if b.parent.is_none());
            if is_root {
                g.scripts.push(i);
                let owner = owners.get(i).copied().flatten();
                g.set_owner(i, owner, 0);
            }
        }
        // The game's objects scripts look up by name, which the map does not hold: the ball's frame, and the
        // game's tables, which the host answers (`Host::game_cell`).
        if g.find("Ball_Pos_Frame").is_none() {
            g.add_object("Ball_Pos_Frame", ObjKind::Ball);
        }
        for table in ["CurrentLevel", "IngameParameter", "Energy"] {
            if g.find(table).is_none() {
                g.add_object(table, ObjKind::Other);
            }
        }
        g
    }

    /// Fills the back references: each IO's and parameter's block, each sub-behaviour's parent, each operation's
    /// output, and the links leaving each IO.
    fn wire(&mut self) {
        let ids: Vec<Id> = (0..self.nodes.len()).collect();
        for &i in &ids {
            let Some(Node::Block(b)) = self.nodes.get(i) else {
                continue;
            };
            let b = b.clone();
            for (io, is_input) in b
                .inputs
                .iter()
                .map(|&x| (x, true))
                .chain(b.outputs.iter().map(|&x| (x, false)))
            {
                match self.nodes.get_mut(io) {
                    Some(Node::Io(n)) => {
                        n.owner = Some(i);
                        n.is_input = is_input;
                    }
                    Some(slot @ Node::None) => {
                        *slot = Node::Io(Io {
                            owner: Some(i),
                            is_input,
                        })
                    }
                    _ => {}
                }
            }
            for &p in b.pins.iter().chain(&b.pouts).chain(&b.locals) {
                if let Some(Node::Param(n)) = self.nodes.get_mut(p) {
                    n.block = Some(i);
                }
            }
            for &s in &b.sub {
                if let Some(Node::Block(n)) = self.nodes.get_mut(s) {
                    n.parent = Some(i);
                }
            }
            for &l in &b.links {
                if let Some(Node::Link(link)) = self.nodes.get(l).cloned()
                    && let Some(from) = self.links_from.get_mut(link.src)
                {
                    from.push(l);
                }
            }
        }
        for &i in &ids {
            if let Some(Node::Op(op)) = self.nodes.get(i).cloned()
                && let Some(out) = op.output
                && let Some(Node::Param(p)) = self.nodes.get_mut(out)
            {
                p.op = Some(i);
            }
        }
    }

    fn set_owner(&mut self, block: Id, owner: Option<Id>, depth: usize) {
        if depth > 64 {
            return;
        }
        let Some(Node::Block(b)) = self.nodes.get_mut(block) else {
            return;
        };
        b.owner = owner;
        let subs = b.sub.clone();
        for s in subs {
            self.set_owner(s, owner, depth.saturating_add(1));
        }
    }

    pub fn block(&self, id: Id) -> Option<&Block> {
        match self.nodes.get(id) {
            Some(Node::Block(b)) => Some(b),
            _ => None,
        }
    }

    pub fn param(&self, id: Id) -> Option<&Param> {
        match self.nodes.get(id) {
            Some(Node::Param(p)) => Some(p),
            _ => None,
        }
    }

    pub fn link(&self, id: Id) -> Option<Link> {
        match self.nodes.get(id) {
            Some(Node::Link(l)) => Some(*l),
            _ => None,
        }
    }

    pub fn io(&self, id: Id) -> Option<Io> {
        match self.nodes.get(id) {
            Some(Node::Io(io)) => Some(*io),
            _ => None,
        }
    }

    pub fn name(&self, id: Id) -> &str {
        self.objects.get(id).map_or("", |o| o.name.as_str())
    }

    /// The first object of this name (case-insensitively), as the original's `GetObjectByName`.
    pub fn find(&self, name: &str) -> Option<Id> {
        self.objects
            .iter()
            .position(|o| o.name.eq_ignore_ascii_case(name))
    }

    /// Every building block prototype used, with its name and how many blocks use it, most used first.
    pub fn prototypes(&self) -> Vec<(Guid, String, usize)> {
        let mut out: Vec<(Guid, String, usize)> = Vec::new();
        for n in &self.nodes {
            let Node::Block(b) = n else { continue };
            let Some(guid) = b.proto else { continue };
            match out.iter_mut().find(|(g, _, _)| *g == guid) {
                Some(e) => e.2 = e.2.saturating_add(1),
                None => out.push((guid, b.name.clone(), 1)),
            }
        }
        out.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
        out
    }

    /// A graph from its primary data, as the open format carries it (the back references are derived again, as
    /// `from_document` derives them): `nodes` are `(object index, node)` pairs, `scripts` the top-level graphs with
    /// the object each is attached to. Every index is checked against the objects, the arrays and the caps, so a
    /// stranger's level can name anything and gets an error, not a panic or a stall.
    pub fn assemble(
        objects: Vec<ObjInfo>,
        nodes: Vec<(Id, Node)>,
        arrays: Vec<Array>,
        scripts: Vec<(Id, Option<Id>)>,
    ) -> Result<Graph, String> {
        let n = objects.len();
        if n > MAX_OBJECTS || arrays.len() > MAX_OBJECTS {
            return Err("too many objects in the scripts".into());
        }
        let id = |i: Id| -> Result<Id, String> {
            if i < n {
                Ok(i)
            } else {
                Err(format!("script object {i} does not exist"))
            }
        };
        let opt = |i: Option<Id>| i.map_or(Ok(()), |i| id(i).map(|_| ()));
        let all = |v: &[Id]| v.iter().try_for_each(|&i| id(i).map(|_| ()));
        for o in &objects {
            if let ObjKind::Array(a) = o.kind
                && a >= arrays.len()
            {
                return Err("script object names an array that does not exist".into());
            }
        }
        for a in &arrays {
            if a.rows.len() > MAX_ARRAY_ROWS {
                return Err("script array has too many rows".into());
            }
            for r in &a.rows {
                r.iter().try_for_each(|v| match v {
                    Value::Obj(o) => opt(*o),
                    _ => Ok(()),
                })?;
            }
        }
        let mut g = Graph {
            objects,
            nodes: vec![Node::None; n],
            arrays,
            scripts: Vec::new(),
            links_from: vec![Vec::new(); n],
        };
        for (i, node) in nodes {
            match &node {
                Node::None => {}
                Node::Block(b) => {
                    opt(b.target)?;
                    for v in [
                        &b.inputs, &b.outputs, &b.pins, &b.pouts, &b.locals, &b.sub, &b.links,
                        &b.ops,
                    ] {
                        all(v)?;
                    }
                }
                Node::Link(l) => {
                    id(l.src)?;
                    id(l.dst)?;
                }
                Node::Io(_) => {}
                Node::Param(p) => {
                    opt(p.source)?;
                    all(&p.destinations)?;
                    if let Value::Obj(o) = &p.value {
                        opt(*o)?;
                    }
                }
                Node::Op(o) => {
                    opt(o.inputs[0])?;
                    opt(o.inputs[1])?;
                    opt(o.output)?;
                }
            }
            *g.nodes.get_mut(id(i)?).ok_or("script node out of range")? = node;
        }
        g.check_shape()?;
        g.wire();
        for (s, owner) in scripts {
            opt(owner)?;
            let Some(Node::Block(b)) = g.nodes.get(id(s)?) else {
                return Err("a script is not a block".into());
            };
            if b.parent.is_some() {
                return Err("a script is inside another graph".into());
            }
            if g.scripts.contains(&s) {
                return Err("a script is listed twice".into());
            }
            g.scripts.push(s);
            // graphs are trees (checked above), so this visits each block once
            g.set_owner(s, owner, 0);
        }
        Ok(g)
    }

    /// The wiring a stranger's graph may have: a block is the sub-behaviour of at most one block, once and not of
    /// itself (graphs are trees, so walking them is linear); no parameter copies to itself, twice, or round a loop,
    /// and none reads its own value through its source or operation. (A graph from a `.nmo` is not held to this; the
    /// runtime's work budget bounds what it does.)
    fn check_shape(&self) -> Result<(), String> {
        let n = self.nodes.len();
        let mut parent = vec![false; n];
        for (i, node) in self.nodes.iter().enumerate() {
            let Node::Block(b) = node else { continue };
            for &s in &b.sub {
                if s == i || parent.get(s).copied().unwrap_or(true) {
                    return Err("a block is a sub-behaviour twice, or of itself".into());
                }
                if let Some(x) = parent.get_mut(s) {
                    *x = true;
                }
            }
            for &l in &b.links {
                if !matches!(self.nodes.get(l), Some(Node::Link(_))) {
                    return Err("a block lists a link that is not one".into());
                }
            }
        }
        for node in &self.nodes {
            if let Node::Param(p) = node {
                let mut seen: Vec<Id> = p.destinations.clone();
                seen.sort_unstable();
                seen.dedup();
                if seen.len() != p.destinations.len() {
                    return Err("a parameter copies to another twice".into());
                }
            }
        }
        // no cycle through destinations, nor through what a parameter reads (its source, else its operation's inputs)
        let dests = |i: Id| -> Vec<Id> {
            match self.nodes.get(i) {
                Some(Node::Param(p)) => p.destinations.clone(),
                _ => Vec::new(),
            }
        };
        let mut op_of = vec![None; n];
        for (i, node) in self.nodes.iter().enumerate() {
            if let Node::Op(o) = node
                && let Some(out) = o.output
                && let Some(slot) = op_of.get_mut(out)
            {
                *slot = Some(i);
            }
        }
        let reads = |i: Id| -> Vec<Id> {
            let Some(Node::Param(p)) = self.nodes.get(i) else {
                return Vec::new();
            };
            if let Some(s) = p.source {
                return vec![s];
            }
            match op_of
                .get(i)
                .copied()
                .flatten()
                .and_then(|o| self.nodes.get(o))
            {
                Some(Node::Op(o)) => o.inputs.iter().flatten().copied().collect(),
                _ => Vec::new(),
            }
        };
        if has_cycle(n, &dests) || has_cycle(n, &reads) {
            return Err("parameters copy or read round a loop".into());
        }
        Ok(())
    }

    /// Every node of a script: its blocks down the tree, their inputs, outputs, parameters, links and
    /// operations (with the operations' own parameters), and the script's "This" parameters. None when the
    /// script is larger than a copy may be.
    pub fn script_nodes(&self, root: Id) -> Option<Vec<Id>> {
        // membership only (the order is `out`'s): sized to the script, not to the whole graph, so walking
        // every script of a map is linear in the map
        let mut seen: HashSet<Id> = HashSet::new();
        let mut out: Vec<Id> = Vec::new();
        let n = self.nodes.len();
        let mut add = |i: Id, out: &mut Vec<Id>| -> bool {
            if i < n && seen.insert(i) {
                out.push(i);
                true
            } else {
                false
            }
        };
        let mut todo = vec![root];
        while let Some(id) = todo.pop() {
            if out.len() > MAX_SCRIPT_NODES {
                return None;
            }
            let Some(b) = self.block(id) else { continue };
            if !add(id, &mut out) {
                continue;
            }
            for &i in b
                .inputs
                .iter()
                .chain(&b.outputs)
                .chain(&b.pins)
                .chain(&b.pouts)
                .chain(&b.locals)
                .chain(&b.links)
                .chain(&b.target)
            {
                add(i, &mut out);
            }
            for &o in &b.ops {
                if let Some(Node::Op(op)) = self.nodes.get(o) {
                    add(o, &mut out);
                    for &i in op.inputs.iter().flatten().chain(&op.output) {
                        add(i, &mut out);
                    }
                }
            }
            todo.extend(b.sub.iter().rev());
        }
        // "This" may be listed by no block: it is found from the inputs that read it
        let mut k = 0usize;
        while let Some(&id) = out.get(k) {
            k = k.saturating_add(1);
            if out.len() > MAX_SCRIPT_NODES {
                return None;
            }
            if let Some(src) = self.param(id).and_then(|p| p.source)
                && matches!(self.param(src), Some(p) if p.value == Value::Owner)
            {
                add(src, &mut out);
            }
        }
        Some(out)
    }

    /// Copies a script for another owner (the copy of the object it is attached to): every node of it anew, the
    /// references among them kept, those to anything else shared. Returns the copy's root and, for each node
    /// copied, (the original, its copy).
    pub fn clone_script(&mut self, root: Id, owner: Id) -> Option<(Id, Vec<(Id, Id)>)> {
        let is_root = matches!(self.block(root), Some(b) if b.parent.is_none());
        if !is_root || owner >= self.objects.len() {
            return None;
        }
        let ids = self.script_nodes(root)?;
        let base = self.nodes.len();
        if base.checked_add(ids.len())? > MAX_OBJECTS {
            return None;
        }
        let mut map: HashMap<Id, Id> = HashMap::with_capacity(ids.len());
        let mut pairs = Vec::with_capacity(ids.len());
        for (k, &old) in ids.iter().enumerate() {
            let new = base.checked_add(k)?;
            map.insert(old, new);
            pairs.push((old, new));
        }
        let r = |i: Id| map.get(&i).copied().unwrap_or(i);
        let ro = |i: Option<Id>| i.map(r);
        let rv = |v: &[Id]| v.iter().map(|&i| r(i)).collect::<Vec<Id>>();
        for &(old, _) in &pairs {
            let node = match self.nodes.get(old)? {
                Node::None => Node::None,
                Node::Block(b) => Node::Block(Block {
                    name: b.name.clone(),
                    proto: b.proto,
                    flags: b.flags,
                    priority: b.priority,
                    target: ro(b.target),
                    inputs: rv(&b.inputs),
                    outputs: rv(&b.outputs),
                    pins: rv(&b.pins),
                    pouts: rv(&b.pouts),
                    locals: rv(&b.locals),
                    sub: rv(&b.sub),
                    links: rv(&b.links),
                    ops: rv(&b.ops),
                    parent: if old == root { None } else { ro(b.parent) },
                    owner: Some(owner),
                }),
                Node::Link(l) => Node::Link(Link {
                    delay: l.delay,
                    src: r(l.src),
                    dst: r(l.dst),
                }),
                Node::Io(io) => Node::Io(Io {
                    owner: ro(io.owner),
                    is_input: io.is_input,
                }),
                Node::Param(p) => Node::Param(Param {
                    name: p.name.clone(),
                    guid: p.guid,
                    value: p.value.clone(),
                    source: ro(p.source),
                    shared: p.shared,
                    destinations: rv(&p.destinations),
                    op: ro(p.op),
                    block: ro(p.block),
                }),
                Node::Op(o) => Node::Op(Op {
                    guid: o.guid,
                    inputs: [ro(o.inputs[0]), ro(o.inputs[1])],
                    output: ro(o.output),
                }),
            };
            let info = self.objects.get(old)?.clone();
            self.objects.push(info);
            self.nodes.push(node);
            self.links_from.push(Vec::new());
        }
        for &(_, new) in &pairs {
            let links = match self.nodes.get(new) {
                Some(Node::Block(b)) => b.links.clone(),
                _ => continue,
            };
            for l in links {
                if let Some(link) = self.link(l)
                    && let Some(from) = self.links_from.get_mut(link.src)
                {
                    from.push(l);
                }
            }
        }
        let new_root = r(root);
        self.scripts.push(new_root);
        Some((new_root, pairs))
    }

    /// Adds a pseudo-object of the engine's (the ball frame, the game's arrays) and returns its id.
    pub fn add_object(&mut self, name: &str, kind: ObjKind) -> Id {
        let id = self.objects.len();
        self.objects.push(ObjInfo {
            name: name.to_string(),
            class: 0,
            kind,
        });
        self.nodes.push(Node::None);
        self.links_from.push(Vec::new());
        id
    }
}

fn cell_kind(c: &nmo::scene::Cell) -> u32 {
    match c {
        nmo::scene::Cell::Int(_) => 1,
        nmo::scene::Cell::Float(_) => 2,
        nmo::scene::Cell::String(_) => 3,
        nmo::scene::Cell::Object(_) => 4,
    }
}

fn cell_value(c: &nmo::scene::Cell) -> Value {
    match c {
        nmo::scene::Cell::Int(i) => Value::Int(*i),
        nmo::scene::Cell::Float(f) => Value::Float(*f),
        nmo::scene::Cell::String(s) => Value::Str(String::from_utf8_lossy(s).into_owned()),
        nmo::scene::Cell::Object(o) => Value::Obj(*o),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_decode_by_type_and_convert() {
        let int = Value::decode(types::INT, &Saved::Bytes(7i32.to_le_bytes().to_vec()));
        assert_eq!(int, Value::Int(7));
        assert_eq!(int.as_float(), 7.0);
        let f = Value::decode(types::FLOAT, &Saved::Bytes(2.5f32.to_le_bytes().to_vec()));
        assert_eq!(f, Value::Float(2.5));
        assert_eq!(f.as_int(), 2);
        assert_eq!(
            Value::decode(types::BOOL, &Saved::Bytes(vec![1, 0, 0, 0])),
            Value::Bool(true)
        );
        assert_eq!(
            Value::decode(types::STRING, &Saved::Bytes(b"Ball\0junk".to_vec())),
            Value::Str("Ball".into())
        );
        let v: Vec<u8> = [1.0f32, 2.0, 3.0]
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect();
        assert_eq!(
            Value::decode(types::VECTOR, &Saved::Bytes(v.clone())).as_vec3(),
            [1.0, 2.0, 3.0]
        );
        assert_eq!(
            Value::Raw(v).as_vec3(),
            [1.0, 2.0, 3.0],
            "an untyped 12-byte value still reads as a vector"
        );
        assert_eq!(
            Value::decode(Guid(1, 2), &Saved::Bytes(3i32.to_le_bytes().to_vec())).as_int(),
            3
        );
        assert!(Value::Obj(Some(3)).as_bool() && !Value::Obj(None).as_bool());
        assert!(
            Value::Int(2).same(&Value::Float(2.0))
                && !Value::Str("a".into()).same(&Value::Str("b".into()))
        );
    }
}

/// Whether the directed graph on `0..n` has a cycle (iterative, three colours: linear in nodes and edges).
fn has_cycle(n: usize, edges: &dyn Fn(Id) -> Vec<Id>) -> bool {
    // 0 unseen, 1 on the path, 2 done
    let mut state = vec![0u8; n];
    for start in 0..n {
        if state.get(start).copied() != Some(0) {
            continue;
        }
        let mut path: Vec<(Id, Vec<Id>)> = vec![(start, edges(start))];
        if let Some(x) = state.get_mut(start) {
            *x = 1;
        }
        while let Some((node, todo)) = path.last_mut() {
            let node = *node;
            match todo.pop() {
                Some(next) => match state.get(next).copied() {
                    Some(1) => return true,
                    Some(0) => {
                        if let Some(x) = state.get_mut(next) {
                            *x = 1;
                        }
                        path.push((next, edges(next)));
                    }
                    _ => {}
                },
                None => {
                    if let Some(x) = state.get_mut(node) {
                        *x = 2;
                    }
                    path.pop();
                }
            }
        }
    }
    false
}
